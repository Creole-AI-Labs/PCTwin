//! Lab laptops with several people and things planted in odd places, and the answer key the
//! "nothing forgotten" test checks the sweep against (Engineering Plan 5.1 and 9.16).
//!
//! [`plan`] makes a seeded [`LaptopSpec`]: one to six people of every kind (the mover, others,
//! one whose home is locked, a leftover from a removed account) and things planted at the root of
//! the system drive, on another drive, in the mover's home outside the usual folders, inside
//! other people's homes, behind a link loop and in a folder the mover may not open. Every plant
//! carries what the sweep must say about it. [`plant`] puts one person's plants on disk (run as
//! that person on a real laptop, or for everyone in a temporary folder). [`check`] reads the
//! sweep's JSON and lists every way it fell short; an empty list is a pass.
//!
//! The planted names carry a token (`nf7x3`) so the leak check can look for them anywhere in the
//! sweep's output. Everything here is synthetic.

use std::collections::BTreeMap;
use std::io;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::Rng;

/// What kind of person a lab person is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Role {
    /// The person running the move.
    Mover,
    /// Another person whose home the mover can't open (the system's normal setting).
    Other,
    /// Another person whose home is locked down even further (no listing at all).
    Locked,
    /// An account that was removed, leaving its home folder behind.
    Leftover,
}

/// One person on the lab laptop.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LabPerson {
    /// A short label (`ada`); the real account name is chosen by the lab script.
    pub label: String,
    pub role: Role,
}

/// Where a plant goes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "spot", rename_all = "kebab-case")]
pub enum Spot {
    /// The root of the drive the system runs from (`C:\Projects`, `/srv`-like places).
    SystemRoot,
    /// The root of another drive (a virtual disk, a disk image, a loop device).
    OtherDrive,
    /// Someone's home folder, outside the usual folders (`~/code`).
    Home { person: String },
}

/// Who may open a planted folder.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Opens {
    /// Everyone on the laptop may read it.
    Everyone,
    /// Only its owner (and administrators).
    OwnerOnly,
    /// Not even its owner may list it (a folder the mover can't open).
    Nobody,
}

/// What the sweep must say about a plant.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "expect", rename_all = "kebab-case")]
pub enum Expect {
    /// In the move record, owned by the mover.
    Mover,
    /// In the move record, shared.
    Shared,
    /// In the record as part of one counted item (`node_modules`): the item is the folder named.
    InItem { item_folder: String },
    /// Never named anywhere in the sweep's output, and either counted as another person's or
    /// listed as a blind spot with one of these reasons.
    NotNamed { reasons: Vec<String> },
    /// Listed as left out because it can't be read, and as a blind spot with one of these
    /// reasons.
    Unreadable { reasons: Vec<String> },
}

/// One planted thing: a file in a folder, both named with the plant's token.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Plant {
    /// Unique in the spec, and part of every name the plant makes.
    pub token: String,
    /// Whose it is: who makes it on a real laptop.
    pub by: String,
    #[serde(flatten)]
    pub spot: Spot,
    /// The folder under the spot that the plant makes (its first part carries the token and the
    /// access is set on it), then the file inside, `/` between parts.
    pub folder: String,
    pub file: String,
    pub bytes: u64,
    pub opens: Opens,
    /// A folder link inside the plant's folder pointing back at it (a loop).
    pub loop_link: bool,
    #[serde(flatten)]
    pub expect: Expect,
}

impl Plant {
    /// The first folder the plant makes, under its spot.
    pub fn top(&self) -> &str {
        self.folder.split('/').next().unwrap_or(&self.folder)
    }
}

/// A seeded lab laptop.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LaptopSpec {
    pub seed: u64,
    pub people: Vec<LabPerson>,
    pub plants: Vec<Plant>,
}

impl LaptopSpec {
    pub fn several(&self) -> bool {
        self.people
            .iter()
            .filter(|p| p.role != Role::Leftover)
            .count()
            >= 2
    }

    pub fn mover(&self) -> &str {
        self.people
            .iter()
            .find(|p| p.role == Role::Mover)
            .map_or("", |p| p.label.as_str())
    }
}

const LABELS: [&str; 6] = ["ada", "bea", "cal", "dee", "eko", "fay"];

/// Makes the spec for a laptop with `people` people (1 to 6). With two or more, the last is a
/// leftover from a removed account and, with three or more, one other person's home is locked.
pub fn plan(seed: u64, people: usize) -> LaptopSpec {
    let n = people.clamp(1, 6);
    let mut rng = Rng::new(seed);
    let mut list = vec![LabPerson {
        label: LABELS[0].into(),
        role: Role::Mover,
    }];
    for (i, label) in LABELS.iter().enumerate().take(n).skip(1) {
        let role = if n >= 2 && i == n - 1 && n >= 3 {
            Role::Leftover
        } else if i == 2 {
            Role::Locked
        } else {
            Role::Other
        };
        list.push(LabPerson {
            label: (*label).into(),
            role,
        });
    }
    let mut spec = LaptopSpec {
        seed,
        people: list,
        plants: Vec::new(),
    };
    let several = spec.several();
    let mut k = 0;
    let mut token = |rng: &mut Rng| {
        k += 1;
        format!("nf{}x{}r{}", seed, k, rng.below(1000))
    };
    let mover = spec.mover().to_string();
    let mut add = |spec: &mut LaptopSpec,
                   rng: &mut Rng,
                   by: &str,
                   spot: Spot,
                   shape: &str,
                   opens: Opens,
                   expect: &dyn Fn(&str) -> Expect| {
        let t = token(rng);
        let folder = shape.replace("{t}", &t);
        let bytes = 1 + rng.below(4096);
        let file = format!("thing-{t}.txt");
        let expect = expect(&folder);
        spec.plants.push(Plant {
            token: t,
            by: by.into(),
            spot,
            folder,
            file,
            bytes,
            opens,
            loop_link: false,
            expect,
        });
    };
    let shared_or_mover = |_: &str| {
        if several {
            Expect::Shared
        } else {
            Expect::Mover
        }
    };
    let mover_only = |_: &str| Expect::Mover;
    // The mover's things in odd places.
    add(
        &mut spec,
        &mut rng,
        &mover,
        Spot::SystemRoot,
        "Projects-{t}/app/src",
        Opens::OwnerOnly,
        &mover_only,
    );
    add(
        &mut spec,
        &mut rng,
        &mover,
        Spot::SystemRoot,
        "Shared-{t}",
        Opens::Everyone,
        &shared_or_mover,
    );
    add(
        &mut spec,
        &mut rng,
        &mover,
        Spot::OtherDrive,
        "Photos-{t}/2019/beach",
        Opens::OwnerOnly,
        &mover_only,
    );
    add(
        &mut spec,
        &mut rng,
        &mover,
        Spot::OtherDrive,
        "Music-{t}",
        Opens::Everyone,
        &shared_or_mover,
    );
    let home = Spot::Home {
        person: mover.clone(),
    };
    add(
        &mut spec,
        &mut rng,
        &mover,
        home.clone(),
        "code-{t}/site",
        Opens::OwnerOnly,
        &mover_only,
    );
    add(
        &mut spec,
        &mut rng,
        &mover,
        home.clone(),
        "code-{t}/site/node_modules/pkg",
        Opens::OwnerOnly,
        &|f: &str| Expect::InItem {
            item_folder: f.trim_end_matches("/pkg").to_string(),
        },
    );
    add(
        &mut spec,
        &mut rng,
        &mover,
        home.clone(),
        "Odd folder {t}/deep/deeper/deepest",
        Opens::OwnerOnly,
        &mover_only,
    );
    add(
        &mut spec,
        &mut rng,
        &mover,
        Spot::OtherDrive,
        "Locked-{t}",
        Opens::Nobody,
        &|_: &str| Expect::Unreadable {
            reasons: vec!["no-permission".into()],
        },
    );
    // A loop: a link inside a project that points back at it.
    add(
        &mut spec,
        &mut rng,
        &mover,
        home,
        "Loop-{t}/inner",
        Opens::OwnerOnly,
        &mover_only,
    );
    if let Some(p) = spec.plants.last_mut() {
        p.loop_link = true;
    }
    // Everyone else's.
    let others: Vec<LabPerson> = spec
        .people
        .iter()
        .filter(|p| p.role != Role::Mover)
        .cloned()
        .collect();
    for person in others {
        let by = person.label.clone();
        let not_named = |reasons: &[&str]| {
            let reasons: Vec<String> = reasons.iter().map(|r| (*r).to_string()).collect();
            move |_: &str| Expect::NotNamed {
                reasons: reasons.clone(),
            }
        };
        let home = Spot::Home { person: by.clone() };
        match person.role {
            Role::Leftover => {
                add(
                    &mut spec,
                    &mut rng,
                    &by,
                    home,
                    "Left-{t}",
                    Opens::OwnerOnly,
                    &not_named(&[
                        "from-old-account",
                        "another-person",
                        "no-permission",
                        "access-not-readable",
                    ]),
                );
            }
            _ => {
                add(
                    &mut spec,
                    &mut rng,
                    &by,
                    home.clone(),
                    "Private-{t}",
                    Opens::OwnerOnly,
                    &not_named(&["another-person"]),
                );
                // Readable by everyone, but inside another person's home: still never named.
                add(
                    &mut spec,
                    &mut rng,
                    &by,
                    home,
                    "Readable-{t}",
                    Opens::Everyone,
                    &not_named(&["another-person"]),
                );
                add(
                    &mut spec,
                    &mut rng,
                    &by,
                    Spot::OtherDrive,
                    "Theirs-{t}",
                    Opens::OwnerOnly,
                    &not_named(&["another-person", "access-not-readable", "no-permission"]),
                );
                add(
                    &mut spec,
                    &mut rng,
                    &by,
                    Spot::OtherDrive,
                    "Family-{t}",
                    Opens::Everyone,
                    &|_: &str| Expect::Shared,
                );
            }
        }
    }
    spec
}

/// Where the spots are on one laptop.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Roots {
    pub system_root: PathBuf,
    pub other_drive: PathBuf,
    /// Each lab person's home folder, by label.
    pub homes: BTreeMap<String, PathBuf>,
    /// Each lab person's account ID, by label, when the lab knows it: the check then ties every
    /// other person's plant to that person's own count or blind spot.
    #[serde(default)]
    pub ids: BTreeMap<String, String>,
}

impl Roots {
    /// The plant's top folder on disk.
    pub fn top_of(&self, plant: &Plant) -> Option<PathBuf> {
        let base = match &plant.spot {
            Spot::SystemRoot => self.system_root.clone(),
            Spot::OtherDrive => self.other_drive.clone(),
            Spot::Home { person } => self.homes.get(person)?.clone(),
        };
        Some(base.join(plant.top()))
    }

    /// The planted file on disk.
    pub fn file_of(&self, plant: &Plant) -> Option<PathBuf> {
        let base = match &plant.spot {
            Spot::SystemRoot => self.system_root.clone(),
            Spot::OtherDrive => self.other_drive.clone(),
            Spot::Home { person } => self.homes.get(person)?.clone(),
        };
        let mut p = base;
        for part in plant.folder.split('/') {
            p.push(part);
        }
        Some(p.join(&plant.file))
    }
}

/// Plants everything `by` owns (or everyone's, with `None`), with the access each plant asks
/// for. On a real laptop each person runs this for their own plants, so the system records them
/// as the owner.
pub fn plant(spec: &LaptopSpec, roots: &Roots, by: Option<&str>) -> io::Result<Vec<PathBuf>> {
    let mut made = Vec::new();
    for p in spec.plants.iter().filter(|p| by.is_none_or(|b| b == p.by)) {
        let (Some(file), Some(top)) = (roots.file_of(p), roots.top_of(p)) else {
            return Err(io::Error::other(format!("no root for {}", p.token)));
        };
        if let Some(parent) = file.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let mut rng = Rng::new(spec.seed ^ p.bytes);
        let body: Vec<u8> = (0..p.bytes).map(|_| b'a' + (rng.below(26) as u8)).collect();
        std::fs::write(&file, body)?;
        if p.loop_link {
            let link = file
                .parent()
                .unwrap_or(&top)
                .join(format!("back-{}", p.token));
            dir_link(&top, &link)?;
        }
        set_access(&top, p.opens)?;
        made.push(file);
    }
    Ok(made)
}

fn dir_link(target: &Path, link: &Path) -> io::Result<()> {
    #[cfg(unix)]
    {
        std::os::unix::fs::symlink(target, link)
    }
    #[cfg(windows)]
    {
        let ok = std::process::Command::new("cmd")
            .args(["/c", "mklink", "/J"])
            .arg(link)
            .arg(target)
            .output()?
            .status
            .success();
        if ok {
            Ok(())
        } else {
            Err(io::Error::other("mklink /J failed"))
        }
    }
}

/// Sets who may open a planted folder (and everything in it).
pub fn set_access(top: &Path, opens: Opens) -> io::Result<()> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let (dir, file) = match opens {
            Opens::Everyone => (0o755, 0o644),
            Opens::OwnerOnly => (0o700, 0o600),
            Opens::Nobody => (0o000, 0o600),
        };
        let mut pending = vec![top.to_path_buf()];
        let mut dirs = Vec::new();
        while let Some(d) = pending.pop() {
            for e in std::fs::read_dir(&d)? {
                let e = e?;
                let t = e.file_type()?;
                if t.is_symlink() {
                    continue;
                }
                if t.is_dir() {
                    pending.push(e.path());
                } else {
                    std::fs::set_permissions(e.path(), std::fs::Permissions::from_mode(file))?;
                }
            }
            dirs.push(d);
        }
        // Folders last, deepest first, so the walk above could still enter them.
        for d in dirs.iter().rev() {
            let mode = if d == top { dir } else { dir.max(0o700) };
            std::fs::set_permissions(d, std::fs::Permissions::from_mode(mode))?;
        }
        Ok(())
    }
    #[cfg(windows)]
    {
        let me = windows_sid()?;
        let run = |args: &[String]| -> io::Result<()> {
            let ok = std::process::Command::new("icacls")
                .arg(top)
                .args(args)
                .output()?
                .status
                .success();
            if ok {
                Ok(())
            } else {
                Err(io::Error::other(format!("icacls {args:?} failed")))
            }
        };
        let owner_only = || {
            vec![
                "/inheritance:r".to_string(),
                "/grant:r".into(),
                format!("*{me}:(OI)(CI)F"),
                "*S-1-5-32-544:(OI)(CI)F".into(),
                "*S-1-5-18:(OI)(CI)F".into(),
            ]
        };
        match opens {
            Opens::Everyone => run(&["/grant".into(), "*S-1-5-32-545:(OI)(CI)RX".into()]),
            Opens::OwnerOnly => run(&owner_only()),
            Opens::Nobody => {
                run(&owner_only())?;
                run(&["/deny".into(), format!("*{me}:(OI)(CI)(RD)")])
            }
        }
    }
}

#[cfg(windows)]
fn windows_sid() -> io::Result<String> {
    let out = std::process::Command::new("whoami")
        .args(["/user", "/fo", "csv", "/nh"])
        .output()?;
    let text = String::from_utf8_lossy(&out.stdout);
    text.trim()
        .rsplit(',')
        .next()
        .map(|s| s.trim_matches('"').to_string())
        .filter(|s| s.starts_with("S-1-"))
        .ok_or_else(|| io::Error::other("whoami gave no SID"))
}

/// Lets the lab clean up a plant it locked.
pub fn unlock(top: &Path) -> io::Result<()> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(top, std::fs::Permissions::from_mode(0o700))
    }
    #[cfg(windows)]
    {
        // icacls takes a denied right out of the person's own allowed rights too, so they are
        // given back after the denial is removed.
        let me = windows_sid()?;
        std::process::Command::new("icacls")
            .arg(top)
            .args(["/remove:d", &format!("*{me}")])
            .output()?;
        std::process::Command::new("icacls")
            .arg(top)
            .args(["/grant", &format!("*{me}:(OI)(CI)F")])
            .output()
            .map(|_| ())
    }
}

/// What the lab saw that the sweep can't know (each is recorded in the report).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Observed {
    /// The mover could list this folder although it should have been refused (a privacy
    /// setting the runner pre-approves): the check for it is "not reachable".
    pub not_reachable: Vec<String>,
}

// ------------------------------------------------------------------------------------------
// The answer key.

fn s<'a>(v: &'a Value, key: &str) -> &'a str {
    v.get(key).and_then(Value::as_str).unwrap_or("")
}

fn parts(item: &Value) -> Vec<String> {
    item.get("path")
        .and_then(Value::as_array)
        .map(|a| {
            a.iter()
                .map(|n| {
                    n.get("text")
                        .and_then(Value::as_str)
                        .unwrap_or("")
                        .to_string()
                })
                .collect()
        })
        .unwrap_or_default()
}

fn norm(p: &str) -> String {
    let t = p.replace('\\', "/");
    let t = t.trim_end_matches('/').to_string();
    if cfg!(any(windows, target_os = "macos")) {
        t.to_lowercase()
    } else {
        t
    }
}

/// The places that rely on blind spots, and the reasons those blind spots may give.
type Relying = (Vec<String>, Vec<String>);

/// The volume (as the sweep names it, normalised) a path is on: the longest mount that holds it.
fn volume_of(sweep: &Value, path: &str) -> String {
    sweep
        .get("volumes")
        .and_then(Value::as_array)
        .map_or(&[][..], Vec::as_slice)
        .iter()
        .map(|v| norm(s(v, "mount")))
        .filter(|m| path == m || path.starts_with(&format!("{m}/")) || m.is_empty())
        .max_by_key(String::len)
        .unwrap_or_default()
}

/// Where every record item really is, worked out from the sweep's own output: known folders from
/// their place, home items from the mover's home, other items from their drive.
pub fn item_paths(sweep: &Value) -> Vec<(String, &Value)> {
    let items = sweep
        .pointer("/record/items")
        .and_then(Value::as_array)
        .map_or(&[][..], Vec::as_slice);
    let places = sweep
        .get("places")
        .and_then(Value::as_array)
        .map_or(&[][..], Vec::as_slice);
    let placed: BTreeMap<String, usize> = sweep
        .get("placed_by")
        .and_then(Value::as_array)
        .map_or(&[][..], Vec::as_slice)
        .iter()
        .filter_map(|p| {
            let a = p.as_array()?;
            Some((
                a.first()?.as_str()?.to_string(),
                a.get(1)?.as_u64()? as usize,
            ))
        })
        .collect();
    let me = sweep
        .pointer("/census/me")
        .and_then(Value::as_str)
        .unwrap_or("");
    let my_home = sweep
        .pointer("/census/residents")
        .and_then(Value::as_array)
        .and_then(|r| {
            r.iter()
                .find(|x| x.get("account_id").and_then(Value::as_str) == Some(me))
        })
        .and_then(|r| r.get("home"))
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string();
    let volumes = sweep
        .get("volumes")
        .and_then(Value::as_array)
        .map_or(&[][..], Vec::as_slice);
    let system_mount = volumes
        .iter()
        .find(|v| v.get("is_system").and_then(Value::as_bool) == Some(true))
        .map(|v| s(v, "mount").to_string())
        .unwrap_or_default();
    let mut out = Vec::new();
    for item in items {
        let id = s(item, "id");
        let role = item
            .pointer("/place/role")
            .and_then(Value::as_str)
            .unwrap_or("");
        let storage = item
            .pointer("/place/storage")
            .cloned()
            .unwrap_or(Value::Null);
        let place = placed.get(id).and_then(|i| places.get(*i));
        let rule = place.map(|p| s(p, "rule")).unwrap_or("");
        let base = if rule == "known-folder" {
            place.map(|p| s(p, "path").to_string()).unwrap_or_default()
        } else if role == "home" {
            my_home.clone()
        } else if role == "other" {
            match storage.get("other-drive") {
                Some(d) => s(d, "drive").to_string(),
                None => system_mount.clone(),
            }
        } else {
            place.map(|p| s(p, "path").to_string()).unwrap_or_default()
        };
        let mut path = norm(&base);
        for part in parts(item) {
            path.push('/');
            path.push_str(&part);
        }
        out.push((norm(&path), item));
    }
    out
}

/// Checks a sweep's JSON against the spec. Each line is one way the sweep fell short; an empty
/// list is a pass. `exists` says whether a path is really on disk (the "no invented items"
/// check), run by someone who may look everywhere.
pub fn check(
    spec: &LaptopSpec,
    roots: &Roots,
    sweep: &Value,
    observed: &Observed,
    exists: &dyn Fn(&str) -> bool,
) -> Vec<String> {
    let mut problems = Vec::new();
    let mut notes = Vec::new();
    let mut relying: Vec<(String, Option<String>, Vec<String>, String)> = Vec::new();
    let text = sweep.to_string();
    let located = item_paths(sweep);
    let by_path: BTreeMap<&str, &Value> = located.iter().map(|(p, v)| (p.as_str(), *v)).collect();
    let me = sweep
        .pointer("/census/me")
        .and_then(Value::as_str)
        .unwrap_or("");
    let empty = Vec::new();
    let blind = sweep
        .get("blind_spots")
        .and_then(Value::as_array)
        .unwrap_or(&empty);
    for p in &spec.plants {
        let (Some(file), Some(top)) = (roots.file_of(p), roots.top_of(p)) else {
            problems.push(format!("{}: no root to check", p.token));
            continue;
        };
        let file_n = norm(&file.to_string_lossy());
        let top_n = norm(&top.to_string_lossy());
        if observed.not_reachable.contains(&p.token) {
            continue;
        }
        let owner_is = |item: &Value, want: &str| -> bool {
            match want {
                "mover" => {
                    item.pointer("/owner/person/account_id")
                        .and_then(Value::as_str)
                        == Some(me)
                }
                _ => item.get("owner").and_then(Value::as_str) == Some("shared"),
            }
        };
        match &p.expect {
            Expect::Mover | Expect::Shared => {
                let want = if p.expect == Expect::Mover {
                    "mover"
                } else {
                    "shared"
                };
                match by_path.get(file_n.as_str()) {
                    None => problems.push(format!("{}: {} not in the record", p.token, file_n)),
                    Some(item) => {
                        if !owner_is(item, want) {
                            problems.push(format!(
                                "{}: owner {} where {want} was expected",
                                p.token,
                                item.get("owner").map(Value::to_string).unwrap_or_default()
                            ));
                        }
                        if item.get("inclusion").and_then(Value::as_str) != Some("included") {
                            problems.push(format!("{}: not included", p.token));
                        }
                    }
                }
            }
            Expect::InItem { item_folder } => {
                let base = match &p.spot {
                    Spot::Home { person } => roots.homes.get(person).cloned(),
                    Spot::SystemRoot => Some(roots.system_root.clone()),
                    Spot::OtherDrive => Some(roots.other_drive.clone()),
                };
                let mut want = base.unwrap_or_default();
                for part in item_folder.split('/') {
                    want.push(part);
                }
                let want = norm(&want.to_string_lossy());
                match by_path.get(want.as_str()) {
                    None => {
                        problems.push(format!("{}: the counted item {want} is missing", p.token))
                    }
                    Some(item) => {
                        if item.get("size_bytes").and_then(Value::as_u64).unwrap_or(0) < p.bytes {
                            problems.push(format!(
                                "{}: the counted item is smaller than the plant",
                                p.token
                            ));
                        }
                    }
                }
                if by_path.contains_key(file_n.as_str()) {
                    problems.push(format!("{}: listed inside a counted item", p.token));
                }
            }
            Expect::NotNamed { reasons: allowed } => {
                if text.contains(&p.token) {
                    problems.push(format!(
                        "{}: another person's thing is named in the sweep",
                        p.token
                    ));
                }
                // This plant, not just any: a nameless count of its owner's on its drive, or a
                // blind spot with an allowed reason on its drive (for its owner, when the reason
                // names one).
                let vol = volume_of(sweep, &top_n);
                let id = roots.ids.get(&p.by).map(String::as_str);
                let is_theirs = |v: &Value| match v.get("account_id").and_then(Value::as_str) {
                    Some(a) => id.is_none_or(|id| id == a),
                    None => true,
                };
                let counted = sweep
                    .get("places")
                    .and_then(Value::as_array)
                    .is_some_and(|ps| {
                        ps.iter().any(|pl| {
                            let placed = pl.get("placement").unwrap_or(&Value::Null);
                            pl.get("path").is_none_or(Value::is_null)
                                && matches!(s(placed, "whose"), "person" | "from-old-account")
                                && norm(s(pl, "volume")) == vol
                                && is_theirs(placed)
                                && pl.get("files").and_then(Value::as_u64).unwrap_or(0) > 0
                        })
                    });
                let spotted = blind.iter().any(|b| {
                    allowed.iter().any(|a| a == s(b, "reason"))
                        && norm(s(b, "volume")) == vol
                        && b.get("path").is_none_or(Value::is_null)
                        && is_theirs(b)
                });
                if !counted && !spotted {
                    problems.push(format!(
                        "{}: neither counted as someone else's nor a blind spot ({allowed:?})",
                        p.token
                    ));
                } else if !counted {
                    // A refused home is one place, whatever is inside it; elsewhere each planted
                    // folder is its own place.
                    let unit = match &p.spot {
                        Spot::Home { person } => format!("home of {person}"),
                        _ => p.token.clone(),
                    };
                    relying.push((vol, id.map(str::to_string), allowed.clone(), unit));
                }
            }
            Expect::Unreadable { reasons: allowed } => {
                let spot = blind.iter().any(|b| {
                    norm(s(b, "path")) == top_n && allowed.iter().any(|a| a == s(b, "reason"))
                });
                if !spot {
                    problems.push(format!(
                        "{}: {top_n} is not a blind spot {allowed:?}",
                        p.token
                    ));
                }
                if let Some(item) = by_path.get(top_n.as_str())
                    && item
                        .pointer("/inclusion/left-out/reason")
                        .and_then(Value::as_str)
                        != Some("unreadable")
                {
                    problems.push(format!(
                        "{}: an unreadable folder is not marked unreadable",
                        p.token
                    ));
                }
                if by_path.contains_key(file_n.as_str()) {
                    problems.push(format!(
                        "{}: a file inside a refused folder was listed",
                        p.token
                    ));
                }
            }
        }
    }
    // Every one of another person's things that only a blind spot stands for is counted in it:
    // the blind spots on a drive for a person (or without one) count at least as many places
    // as rely on them.
    let mut groups: BTreeMap<(String, Option<String>), Relying> = BTreeMap::new();
    for (vol, id, allowed, unit) in &relying {
        let g = groups.entry((vol.clone(), id.clone())).or_default();
        if !g.0.contains(unit) {
            g.0.push(unit.clone());
        }
        for a in allowed {
            if !g.1.contains(a) {
                g.1.push(a.clone());
            }
        }
    }
    for ((vol, id), (tokens, allowed)) in &groups {
        let available: u64 = blind
            .iter()
            .filter(|b| {
                norm(s(b, "volume")) == *vol
                    && allowed.iter().any(|a| a == s(b, "reason"))
                    && b.get("path").is_none_or(Value::is_null)
                    && match (b.get("account_id").and_then(Value::as_str), id) {
                        (Some(a), Some(id)) => a == id,
                        _ => true,
                    }
            })
            .map(|b| b.get("count").and_then(Value::as_u64).unwrap_or(1))
            .sum();
        if available < tokens.len() as u64 {
            problems.push(format!(
                "{}: {} places on {vol} rely on blind spots that count only {available}",
                tokens.join(", "),
                tokens.len()
            ));
        }
    }
    // And on each drive, everyone's together: a blind spot that names nobody can stand for one
    // place only, whoever's it is.
    let mut by_drive: BTreeMap<String, Relying> = BTreeMap::new();
    for (vol, _, allowed, unit) in &relying {
        let g = by_drive.entry(vol.clone()).or_default();
        if !g.0.contains(unit) {
            g.0.push(unit.clone());
        }
        for a in allowed {
            if !g.1.contains(a) {
                g.1.push(a.clone());
            }
        }
    }
    for (vol, (units, allowed)) in &by_drive {
        let available: u64 = blind
            .iter()
            .filter(|b| {
                norm(s(b, "volume")) == *vol
                    && allowed.iter().any(|a| a == s(b, "reason"))
                    && b.get("path").is_none_or(Value::is_null)
            })
            .map(|b| b.get("count").and_then(Value::as_u64).unwrap_or(1))
            .sum();
        if available < units.len() as u64 {
            problems.push(format!(
                "{}: {} places on {vol} rely on blind spots that count only {available}",
                units.join(", "),
                units.len()
            ));
        }
    }
    // No invented items: everything in the record is really there. Inside the lab's own places
    // nothing changes between the sweep and this check, so a missing path there is invented.
    // Elsewhere the system itself may remove a file meanwhile (a rotated log), so a missing
    // path there is reported as a note, not judged.
    let mut lab_roots: Vec<String> = vec![norm(&roots.other_drive.to_string_lossy())];
    lab_roots.extend(roots.homes.values().map(|h| norm(&h.to_string_lossy())));
    lab_roots.extend(
        spec.plants
            .iter()
            .filter_map(|p| roots.top_of(p))
            .map(|t| norm(&t.to_string_lossy())),
    );
    let inside = |path: &str| {
        lab_roots
            .iter()
            .any(|r| path == r || path.starts_with(&format!("{r}/")))
    };
    for (path, _) in &located {
        if !exists(path) {
            if inside(path) {
                problems.push(format!("invented: {path}"));
            } else {
                notes.push(format!("gone since the sweep (not judged): {path}"));
            }
        }
    }
    // The top line follows the blind spots.
    let top = sweep
        .pointer("/top_line/line")
        .and_then(Value::as_str)
        .unwrap_or("");
    let finished = sweep.get("finished").and_then(Value::as_bool) == Some(true);
    match (top, finished, blind.is_empty()) {
        ("everything", true, true)
        | ("accounted-for-except", true, false)
        | ("did-not-finish", false, _) => {}
        _ => problems.push(format!(
            "top line {top} with finished={finished} and {} blind spots",
            blind.len()
        )),
    }
    for n in notes {
        eprintln!("note: {n}");
    }
    problems
}
