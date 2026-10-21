//! A headless move for the test lab (Task List 0.7): the old laptop and the new laptop as two
//! processes, which CI runs under two different user accounts on one machine.
//!
//! Both sides use the real engine: the old laptop scans with `pctwin-scan`, hosts the pairing code
//! and sends with `pctwin-transfer`; the new laptop connects with the code, approves the plan,
//! and receives through the safety gate (`pctwin-gate`) with every write in the change journal.
//!
//! What a person does on screen is done through a shared "handoff" folder instead: the old laptop
//! writes its address and code there, the new laptop writes the number it shows, and the old
//! laptop picks it. The scanned record then travels over the sealed connection, before any file,
//! and the new laptop checks it as coming from a stranger.
//!
//! Each side ends by writing a report of every item it handled. A side never claims a file moved
//! unless the engine said so.
//!
//! Everything after pairing is `pctwin-move`, the same move the app runs: this crate only finds
//! the folders, pairs through the handoff folder, answers the new laptop's choices by itself, and
//! writes the reports.

pub mod everyone;

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use pctwin_gate::{Approved, Destinations};
use pctwin_journal::Journal;
use pctwin_lab::{Entry, Snapshot, compare};
use pctwin_link::LinkConfig;
use pctwin_move::{NotSentWhy, Steps, Stop};
use pctwin_pairing::{PairingCode, RotatingSender};
use pctwin_record::{
    FolderRole, Inclusion, Item, ItemKind, LaptopId, PersonTarget, Record, Storage,
};
use pctwin_scan::{FolderLookup, FoundFolder, Person};
use pctwin_transfer::{LandingPlace, ReceiveOutcome, SendOutcome, role_label, shown_path};
use serde::{Deserialize, Serialize};

/// How long either side waits for the other before giving up.
pub const WAIT: Duration = Duration::from_secs(180);

/// The special folders a lab laptop has, by their plain names.
pub const ROLES: [(FolderRole, &str); 6] = [
    (FolderRole::Documents, "Documents"),
    (FolderRole::Pictures, "Pictures"),
    (FolderRole::Music, "Music"),
    (FolderRole::Videos, "Videos"),
    (FolderRole::Desktop, "Desktop"),
    (FolderRole::Downloads, "Downloads"),
];

/// Where a side's folders are.
#[derive(Debug, Clone)]
pub enum Folders {
    /// A lab folder holding `Documents`, `Pictures` and the rest, for an account named `account`.
    Lab { root: PathBuf, account: String },
    /// The signed-in person's real special folders, as the app finds them.
    Profile,
    /// A lab package made by [`pack`], possibly on another system: the old laptop's record as that
    /// system scanned it, and each file stored under its item ID.
    Package { dir: PathBuf },
}

/// One item in a side's report.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Line {
    pub item: String,
    pub path: String,
    pub outcome: String,
}

/// A side's report: every item it handled, and anything that stopped it.
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct Report {
    pub lines: Vec<Line>,
    /// Items in the record that were not sent, with why (left out, folders, no place).
    pub not_sent: Vec<Line>,
}

/// The plain folder name for a role (`Documents`), as reports name paths.
pub fn role_name(role: FolderRole) -> &'static str {
    ROLES
        .iter()
        .find(|(r, _)| *r == role)
        .map_or("Other", |(_, n)| n)
}

/// Where an item lands in the lab: its role's folder, at its path as shown. Both sides use this
/// one function (the old laptop to say where it sends each file, the new laptop to check it), so
/// they cannot disagree.
pub fn lab_landing(item: &Item) -> LandingPlace {
    LandingPlace::new(role_label(item.place.role), shown_path(&item.path))
}

fn err(e: impl std::fmt::Display) -> String {
    e.to_string()
}

/// Writes a handoff file in one step, so the other side never reads half of it.
fn hand(dir: &Path, name: &str, bytes: &[u8]) -> Result<(), String> {
    let tmp = dir.join(format!(".{name}.part"));
    std::fs::write(&tmp, bytes).map_err(err)?;
    std::fs::rename(&tmp, dir.join(name)).map_err(err)
}

/// Waits for a handoff file and reads it.
async fn take(dir: &Path, name: &str) -> Result<Vec<u8>, String> {
    let until = Instant::now() + WAIT;
    loop {
        if let Ok(bytes) = std::fs::read(dir.join(name)) {
            return Ok(bytes);
        }
        if Instant::now() > until {
            return Err(format!("the other side never wrote {name}"));
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
}

fn lab_folders(root: &Path) -> Vec<FolderLookup> {
    ROLES
        .iter()
        .filter(|(_, name)| root.join(name).is_dir())
        .map(|(role, name)| {
            FolderLookup::Found(FoundFolder {
                role: *role,
                path: root.join(name),
                storage: Storage::SystemDrive,
                moved: false,
                managed_by: pctwin_record::ManagedBy::Personal,
            })
        })
        .collect()
}

/// Where an item is shown in a report: its role's folder, then its path as it lands
/// (`Documents/Letters/ada.txt`).
fn shown_in_report(item: &Item) -> String {
    format!("{}/{}", role_name(item.place.role), lab_landing(item).path)
}

/// The old laptop: scans, hosts pairing on `bind`, sends, and reports.
pub async fn send(folders: Folders, handoff: &Path, bind: &str) -> Result<Report, String> {
    let laptop = LaptopId::generate().map_err(err)?;
    let (record, places, blobs) = match &folders {
        Folders::Lab { root, account } => {
            let me = Person {
                account_id: account.clone(),
                suggested_name: account.clone(),
                home: root.clone(),
                is_me: true,
            };
            let found = lab_folders(root);
            let scan = pctwin_scan::build_scan(laptop, vec![me], found.clone(), Vec::new())
                .map_err(err)?;
            (scan.record, found, None)
        }
        Folders::Profile => {
            let scan = pctwin_scan::scan_this_laptop(laptop).map_err(err)?;
            let found = scan.folders.clone();
            (scan.record, found, None)
        }
        Folders::Package { dir } => {
            let bytes = std::fs::read(dir.join("record.json")).map_err(err)?;
            let record = Record::from_json(&bytes).map_err(err)?;
            (record, Vec::new(), Some(dir.join("blobs")))
        }
    };
    let roots: BTreeMap<FolderRole, PathBuf> = places
        .iter()
        .filter_map(|f| match f {
            FolderLookup::Found(f) => Some((f.role, f.path.clone())),
            _ => None,
        })
        .collect();

    let plan = pctwin_move::send_plan(
        &record,
        |item| {
            if let Some(blobs) = &blobs {
                return Some(blobs.join(item.id.to_hex()));
            }
            let mut source = roots.get(&item.place.role)?.clone();
            for part in item.path.parts() {
                source.push(part.on_disk());
            }
            Some(source)
        },
        lab_landing,
    );
    let shown_of: BTreeMap<_, _> = record
        .items
        .iter()
        .map(|item| (item.id, shown_in_report(item)))
        .collect();
    let shown = |id| shown_of.get(id).cloned().unwrap_or_default();

    let config = LinkConfig::default();
    let host = pctwin_link::Host::bind(bind.parse().map_err(err)?, config)
        .await
        .map_err(err)?;
    // Listening on every address is reached through loopback when both sides share a machine.
    let mut addr = host.local_addr().map_err(err)?;
    if addr.ip().is_unspecified() {
        addr.set_ip(std::net::Ipv4Addr::LOCALHOST.into());
    }
    hand(handoff, "addr.txt", addr.to_string().as_bytes())?;

    // The code changes every minute, as on screen; the current one is always in the handoff.
    let rotating = Arc::new(Mutex::new(
        RotatingSender::new(Instant::now()).map_err(err)?,
    ));
    let ticker = {
        let rotating = rotating.clone();
        let dir = handoff.to_path_buf();
        tokio::spawn(async move {
            let mut last = String::new();
            loop {
                let code = {
                    let Ok(mut r) = rotating.lock() else { return };
                    let _ = r.tick(Instant::now());
                    r.code()
                };
                if let Some(code) = code
                    && code != last
                {
                    let _ = hand(&dir, "code.txt", code.as_bytes());
                    last = code;
                }
                tokio::time::sleep(Duration::from_millis(500)).await;
            }
        })
    };
    let pending = tokio::time::timeout(WAIT, host.next_peer(&rotating))
        .await
        .map_err(|_| "the new laptop never connected".to_string())?
        .map_err(err)?;
    ticker.abort();
    let number = String::from_utf8(take(handoff, "number.txt").await?).map_err(err)?;
    let number: u8 = number.trim().parse().map_err(err)?;
    let mut link = pending.choose(number, Instant::now()).await.map_err(err)?;
    let sent = pctwin_move::send(&mut link, &record, plan, &pctwin_move::Live::default())
        .await
        .map_err(err)?;

    let mut report = Report::default();
    for file in &sent.files {
        let outcome = match &file.outcome {
            Some(SendOutcome::Arrived) => "arrived".to_string(),
            Some(SendOutcome::AlreadyThere) => "already-there".to_string(),
            Some(SendOutcome::Failed(why)) => format!("failed: {why}"),
            None => "no outcome".to_string(),
        };
        report.lines.push(Line {
            item: file.item.to_hex(),
            path: shown(&file.item),
            outcome,
        });
    }
    for not in &sent.not_sent {
        let outcome = match not.why {
            NotSentWhy::LeftOut { reason } => format!("left out: {reason:?}"),
            NotSentWhy::NoFolderForRole => "no folder for its role".to_string(),
        };
        report.not_sent.push(Line {
            item: not.item.to_hex(),
            path: shown(&not.item),
            outcome,
        });
    }
    Ok(report)
}

/// The new laptop's choices, made by the lab itself: every person on the old laptop lands in
/// this account, the plan is approved as it is, and each item lands where [`lab_landing`] says.
struct LabSteps {
    account: String,
}

impl Steps for LabSteps {
    async fn person(&mut self, _: &Record, _: &str) -> Result<PersonTarget, Stop> {
        Ok(PersonTarget::ExistingAccount {
            account_id: self.account.clone(),
        })
    }

    async fn approve(&mut self, _: &Record, _: &pctwin_move::Preview) -> Result<(), Stop> {
        Ok(())
    }

    fn landing(&self, item: &Item) -> Option<LandingPlace> {
        Some(lab_landing(item))
    }
}

/// The new laptop: checks and approves the record, connects with the code, receives into
/// `folders`, and reports. `account` is this side's account ID.
pub async fn receive(folders: Folders, handoff: &Path, journal: &Path) -> Result<Report, String> {
    let (account, mut table) = (
        match &folders {
            Folders::Lab { account, .. } => account.clone(),
            Folders::Package { .. } => {
                return Err("a package is only sent, never received into".into());
            }
            Folders::Profile => pctwin_scan::list_people()
                .into_iter()
                .find(|p| p.is_me)
                .map(|p| p.account_id)
                .ok_or("no signed-in person")?,
        },
        Destinations::new(),
    );
    match &folders {
        Folders::Lab { root, .. } => {
            for (role, name) in ROLES {
                let dir = root.join(name);
                std::fs::create_dir_all(&dir).map_err(err)?;
                table
                    .approve(role_label(role), Approved::MyFolders, &dir)
                    .map_err(err)?;
            }
        }
        Folders::Profile => {
            pctwin_transfer::approve_new_places(&mut table, &pctwin_scan::find_special_folders())
                .map_err(err)?;
        }
        // Refused above.
        Folders::Package { .. } => {}
    }

    let addr: std::net::SocketAddr = String::from_utf8(take(handoff, "addr.txt").await?)
        .map_err(err)?
        .trim()
        .parse()
        .map_err(err)?;
    let config = LinkConfig::default();
    // A code can change between reading and using it; then the next one is tried.
    let until = Instant::now() + WAIT;
    let pending = loop {
        let code = String::from_utf8(take(handoff, "code.txt").await?).map_err(err)?;
        let code = PairingCode::parse(code.trim()).map_err(err)?;
        match pctwin_link::connect(addr, &code, config).await {
            Ok(p) => break p,
            Err(e) if Instant::now() > until => return Err(err(e)),
            Err(_) => tokio::time::sleep(Duration::from_secs(1)).await,
        }
    };
    hand(
        handoff,
        "number.txt",
        pending.match_number().to_string().as_bytes(),
    )?;
    let mut link = pending.approval().await.map_err(err)?;
    let plan = pctwin_move::receive_plan(
        &mut link,
        &mut LabSteps {
            account: account.clone(),
        },
        &table,
    )
    .await
    .map_err(err)?;

    let journal = Journal::open(journal).map_err(err)?;
    let received = plan
        .receive(
            &mut link,
            &table,
            &journal,
            &account,
            &pctwin_move::Live::default(),
        )
        .await
        .map_err(err)?;

    let items: BTreeMap<_, _> = received.record.items.iter().map(|i| (i.id, i)).collect();
    let mut report = Report::default();
    for file in &received.files {
        let Some(item) = items.get(&file.item) else {
            continue;
        };
        let folder = role_name(item.place.role);
        let outcome = match &file.outcome {
            Some(ReceiveOutcome::Finished(f)) => format!("landed: {folder}/{}", f.final_path),
            Some(ReceiveOutcome::NeedsAction { finished, why }) => {
                format!("needs-action: {folder}/{}: {why}", finished.final_path)
            }
            Some(ReceiveOutcome::AlreadyThere(p)) => format!("already-there: {p}"),
            Some(ReceiveOutcome::Failed(why)) => format!("failed: {why}"),
            None => "no outcome".to_string(),
        };
        report.lines.push(Line {
            item: file.item.to_hex(),
            path: format!("{folder}/{}", shown_path(&item.path)),
            outcome,
        });
    }
    Ok(report)
}

/// What a checked move came to.
#[derive(Debug, Default, PartialEq, Eq, Serialize)]
pub struct Checked {
    pub sent: usize,
    pub landed: usize,
    /// Landed under another name the gate chose (a reserved or clashing name).
    pub renamed: usize,
    /// Could not be read on the old laptop (held open by another program) and honestly ended
    /// failed.
    pub unreadable: usize,
    /// Not the lab's own files (with [`check_move`]'s `only`): each has an outcome on both sides,
    /// whatever it is, and is not judged further.
    pub others: usize,
}

/// Checks a finished move from its evidence: the source folder's snapshot before and after, the
/// destination's snapshot, and both reports. Returns every problem found:
///
/// - the source changed (the old laptop is read-only);
/// - an item has no outcome, or only one side reports it;
/// - an item failed, unless the old laptop's snapshot could not read it either (a file another
///   program holds open must end failed, never landed);
/// - with `only` (the lab's own paths, when a real profile also holds the system's files), items
///   outside it need an outcome on both sides but are not judged further;
/// - a file that landed does not match its original byte for byte.
pub fn check_move(
    before: &Snapshot,
    source_after: &Snapshot,
    dest: &Snapshot,
    sent: &Report,
    received: &Report,
    only: Option<&std::collections::BTreeSet<String>>,
) -> Result<Checked, Vec<String>> {
    let judged = |path: &str| only.is_none_or(|o| o.contains(path));
    let mut problems: Vec<String> = compare(before, source_after)
        .into_iter()
        .map(|d| format!("the source changed: {d}"))
        .collect();
    if sent.lines.is_empty() {
        problems.push("nothing was sent".into());
    }
    let got: BTreeMap<&str, &Line> = received
        .lines
        .iter()
        .map(|l| (l.item.as_str(), l))
        .collect();
    let mut out = Checked {
        sent: sent.lines.len(),
        ..Checked::default()
    };
    let unreadable =
        |path: &str| matches!(before.get(path), Some(Entry::File { blake3: None, .. }));
    for line in &sent.lines {
        if !judged(&line.path) {
            out.others += 1;
            if line.outcome == "no outcome" {
                problems.push(format!("old laptop: {} has no outcome", line.path));
            }
            if !got.contains_key(line.item.as_str()) {
                problems.push(format!("new laptop has no outcome for {}", line.path));
            }
            continue;
        }
        let failed = line.outcome.starts_with("failed");
        if failed && unreadable(&line.path) {
            out.unreadable += 1;
        } else if line.outcome == "no outcome" || failed {
            problems.push(format!("old laptop: {} ended {}", line.path, line.outcome));
        } else if unreadable(&line.path) {
            problems.push(format!(
                "old laptop: {} could not be read, yet ended {}",
                line.path, line.outcome
            ));
        }
        if !got.contains_key(line.item.as_str()) {
            problems.push(format!("new laptop has no outcome for {}", line.path));
        }
    }
    if sent.lines.len() != received.lines.len() {
        problems.push(format!(
            "the old laptop reports {} files, the new laptop {}",
            sent.lines.len(),
            received.lines.len()
        ));
    }
    for line in &received.lines {
        if !judged(&line.path) {
            if line.outcome == "no outcome" {
                problems.push(format!("new laptop: {} has no outcome", line.path));
            }
            continue;
        }
        // An identical file already there is not copied again; it is checked like a landed one.
        let landed = line
            .outcome
            .strip_prefix("landed: ")
            .or_else(|| line.outcome.strip_prefix("already-there: "));
        let Some(final_path) = landed else {
            // A file the old laptop could not read is never offered, so the new laptop may have
            // no outcome for it; the old laptop's report carries its failure.
            let not_offered = line.outcome.starts_with("failed") || line.outcome == "no outcome";
            if !(not_offered && unreadable(&line.path)) {
                problems.push(format!("new laptop: {} ended {}", line.path, line.outcome));
            }
            continue;
        };
        out.landed += 1;
        if final_path != line.path {
            out.renamed += 1;
        }
        let copy = dest.get(final_path);
        // A name that is not valid text is shown with a replacement mark, so its original is
        // found by its contents instead of by name.
        let original = before.get(&line.path).or_else(|| {
            if line.path.contains('\u{FFFD}') {
                copy.and_then(|c| before.values().find(|v| *v == c))
            } else {
                None
            }
        });
        match (original, copy) {
            (
                Some(Entry::File {
                    size: a,
                    blake3: ha,
                    ..
                }),
                Some(Entry::File {
                    size: b,
                    blake3: hb,
                    ..
                }),
            ) if a == b && ha == hb && ha.is_some() => {}
            other => problems.push(format!(
                "{} does not match its original: {other:?}",
                line.path
            )),
        }
    }
    if problems.is_empty() {
        Ok(out)
    } else {
        Err(problems)
    }
}

/// The signed-in person's special folders that a lab laptop uses, by plain name (`Documents`),
/// wherever they really are (OneDrive, another drive, a translated name).
pub fn profile_places() -> BTreeMap<String, PathBuf> {
    pctwin_scan::find_special_folders()
        .into_iter()
        .filter_map(|f| match f {
            FolderLookup::Found(f) => ROLES
                .iter()
                .find(|(r, _)| *r == f.role)
                .map(|(_, name)| (name.to_string(), f.path)),
            FolderLookup::Missing { .. } => None,
        })
        .collect()
}

/// A snapshot of the signed-in person's special folders, named the way lab reports name them.
pub fn snapshot_profile() -> std::io::Result<Snapshot> {
    let named: Vec<(String, PathBuf)> = profile_places().into_iter().collect();
    pctwin_lab::snapshot_named(&named)
}

/// One special folder as the probe found it.
#[derive(Debug, Clone, Serialize)]
pub struct ProbedFolder {
    pub role: FolderRole,
    /// Where the system says it is.
    pub real: String,
    /// Where the plain guess (`home/Documents`) would put it.
    pub guess: String,
    pub storage: Storage,
    /// What the scan says: the folder is not where its plain name would put it.
    pub moved: bool,
}

/// The folder's plain name on this system: macOS keeps videos in `Movies`.
fn plain_name(role: FolderRole, name: &'static str) -> &'static str {
    if cfg!(target_os = "macos") && role == FolderRole::Videos {
        "Movies"
    } else {
        name
    }
}

/// What the probe found (Task List 0.7: "probe first").
#[derive(Debug, Clone, Serialize)]
pub struct Probe {
    pub home: String,
    pub folders: Vec<ProbedFolder>,
    pub missing: Vec<FolderRole>,
    /// Every real drive, its file system, and any other installed system found on it.
    pub drives: Vec<pctwin_scan::Drive>,
    /// Item IDs from two scans of the same laptop are the same.
    pub ids_stable: bool,
    pub items: usize,
}

/// Compares where the system says each special folder is with the plain guess, and scans the
/// laptop twice to check every item keeps its ID. Returns the findings and any problem: a folder
/// the scan calls moved that sits at the guess (or the other way round), or IDs that changed.
pub fn probe() -> Result<(Probe, Vec<String>), String> {
    let home = pctwin_scan::list_people()
        .into_iter()
        .find(|p| p.is_me)
        .map(|p| p.home)
        .ok_or("no signed-in person")?;
    let mut folders = Vec::new();
    let mut missing = Vec::new();
    let mut problems = Vec::new();
    for lookup in pctwin_scan::find_special_folders() {
        match lookup {
            FolderLookup::Found(f) => {
                let Some((_, name)) = ROLES.iter().find(|(r, _)| *r == f.role) else {
                    continue;
                };
                let guess = home.join(plain_name(f.role, name));
                let same = std::fs::canonicalize(&f.path).ok()
                    == std::fs::canonicalize(&guess).ok()
                    && guess.exists();
                if f.moved == same {
                    problems.push(format!(
                        "{name}: the scan says moved={} but the real folder {} is {}the guess {}",
                        f.moved,
                        f.path.display(),
                        if same { "" } else { "not " },
                        guess.display()
                    ));
                }
                folders.push(ProbedFolder {
                    role: f.role,
                    real: f.path.display().to_string(),
                    guess: guess.display().to_string(),
                    storage: f.storage,
                    moved: f.moved,
                });
            }
            FolderLookup::Missing { role } => missing.push(role),
        }
    }
    let laptop = LaptopId::generate().map_err(err)?;
    let first = pctwin_scan::scan_this_laptop(laptop).map_err(err)?;
    let second = pctwin_scan::scan_this_laptop(laptop).map_err(err)?;
    let ids = |r: &Record| {
        let mut v: Vec<String> = r.items.iter().map(|i| i.id.to_hex()).collect();
        v.sort();
        v
    };
    let ids_stable = ids(&first.record) == ids(&second.record);
    if !ids_stable {
        problems.push("item IDs changed between two scans of the same laptop".into());
    }
    Ok((
        Probe {
            home: home.display().to_string(),
            folders,
            missing,
            drives: pctwin_scan::list_drives(),
            ids_stable,
            items: first.record.items.len(),
        },
        problems,
    ))
}

/// Makes a lab package from a lab laptop at `root` (Task List 0.7, cross-system handoff): the
/// record as this system scans it (`record.json`), each included file copied under its item ID
/// (`blobs/<id>`, so no name of the old system touches the next one's disk before its safety gate
/// sees it), and a snapshot of the laptop (`before.json`) for checking. Another system then sends
/// it with [`Folders::Package`]. Not encrypted: the encrypted package for drives is the archive
/// format of Release 2, which will replace this one. Returns how many files it holds.
pub fn pack(root: &Path, account: &str, out: &Path) -> Result<usize, String> {
    let me = Person {
        account_id: account.to_string(),
        suggested_name: account.to_string(),
        home: root.to_path_buf(),
        is_me: true,
    };
    let found = lab_folders(root);
    let laptop = LaptopId::generate().map_err(err)?;
    let scan = pctwin_scan::build_scan(laptop, vec![me], found.clone(), Vec::new()).map_err(err)?;
    let roots: BTreeMap<FolderRole, PathBuf> = found
        .iter()
        .filter_map(|f| match f {
            FolderLookup::Found(f) => Some((f.role, f.path.clone())),
            _ => None,
        })
        .collect();
    let blobs = out.join("blobs");
    std::fs::create_dir_all(&blobs).map_err(err)?;
    let mut held = 0;
    for item in &scan.record.items {
        if item.kind != ItemKind::File || item.inclusion != Inclusion::Included {
            continue;
        }
        let Some(root) = roots.get(&item.place.role) else {
            continue;
        };
        let mut source = root.clone();
        for part in item.path.parts() {
            source.push(part.on_disk());
        }
        let source = pctwin_lab::long(&source).map_err(err)?;
        // A file the system will not let us read stays out of the package; the receiving side
        // then reports it failed, as a live move would.
        if std::fs::copy(&source, blobs.join(item.id.to_hex())).is_ok() {
            held += 1;
        }
    }
    // Written a piece at a time, never as one text in memory.
    let mut file =
        std::io::BufWriter::new(std::fs::File::create(out.join("record.json")).map_err(err)?);
    scan.record.write_json(&mut file).map_err(err)?;
    std::io::Write::flush(&mut file).map_err(err)?;
    let snap = pctwin_lab::snapshot(root).map_err(err)?;
    std::fs::write(
        out.join("before.json"),
        serde_json::to_vec_pretty(&snap).map_err(err)?,
    )
    .map_err(err)?;
    std::fs::write(out.join("origin.txt"), std::env::consts::OS).map_err(err)?;
    Ok(held)
}
