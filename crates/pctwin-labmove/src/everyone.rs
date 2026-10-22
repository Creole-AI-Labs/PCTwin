//! Everyone in one move, for the test lab (Engineering Plan 5.1, "Several people in one move" and
//! "Nothing forgotten"; Task List, moving everyone, step 8). Run as the mover, a standard account,
//! after the lab's script (standing in for the system's administrator prompt, from outside the
//! product) has started the administrator helper for this move. With no helper, it is the move
//! with no administrator anywhere.
//!
//! The real sweep finds everything on this laptop; the helper scans each other person (a person
//! the administrator did not approve is refused, and nothing of theirs moves); the move runs
//! through the real engine to a new laptop on the same machine: the mover's things into a folder
//! that stands for their new home, each approved person's into a new account the helper makes,
//! through the holding area and handed over at the end.
//!
//! Two outputs: the mover's report (the file the mover can read: their own files by name, every
//! other person as counts only) and the lab's evidence on standard output, which the lab's script
//! keeps where only an administrator can read it, with the answer key.

use std::collections::{BTreeMap, HashMap};
use std::path::PathBuf;
use std::sync::atomic::AtomicBool;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use pctwin_gate::{Approved, Destinations};
use pctwin_helper::{Client, ClientError, Refusal, Scanned};
use pctwin_journal::Journal;
use pctwin_move::landing::LandingRules;
use pctwin_move::people::{HelperOpener, add_people, helper_source};
use pctwin_move::report::{FileResult, MoverReport, for_the_mover};
use pctwin_move::{Live, Steps, Stop};
use pctwin_record::{FolderRole, Item, LaptopId, Owner, PersonTarget, Record};
use pctwin_scan::{PersonKind, Sweep};
use pctwin_transfer::{Channel, ChannelError, LandingPlace, role_label, shown_path};
use serde_json::{Value, json};
use tokio::sync::mpsc;

use crate::err;

/// The new home's folders, one per role (Public is the shared one).
const NEW_ROLES: [FolderRole; 10] = [
    FolderRole::Desktop,
    FolderRole::Documents,
    FolderRole::Downloads,
    FolderRole::Pictures,
    FolderRole::Music,
    FolderRole::Videos,
    FolderRole::Home,
    FolderRole::AppData,
    FolderRole::Other,
    FolderRole::Public,
];

/// How long the helper may take to make its channel.
const HELPER_WAIT: Duration = Duration::from_secs(60);

/// The helper the lab started for this move.
pub struct HelperChannel {
    pub session: String,
    pub channel: String,
    /// Mac and Linux: the folder the socket is in.
    pub dir: Option<PathBuf>,
}

/// What to run.
pub struct Everyone {
    /// The old laptop's ID, the one the helper was started with.
    pub laptop: LaptopId,
    pub helper: Option<HelperChannel>,
    /// Stands for the mover's home on the new laptop.
    pub new_home: PathBuf,
    pub journal: PathBuf,
    /// New accounts are named this, then a number.
    pub new_names: String,
    /// A hostile mover: links to the system's own files planted in each holding folder before
    /// the hand-over, which must never follow them.
    pub hostile: bool,
}

/// What the run came to: the mover's report, and the lab's evidence.
pub struct Ran {
    pub report: Value,
    pub evidence: Value,
}

/// An in-memory connection between the two sides (both laptops are this machine).
struct Mem {
    tx: mpsc::UnboundedSender<Vec<u8>>,
    rx: mpsc::UnboundedReceiver<Vec<u8>>,
}

fn mem_pair() -> (Mem, Mem) {
    let (a_tx, b_rx) = mpsc::unbounded_channel();
    let (b_tx, a_rx) = mpsc::unbounded_channel();
    (Mem { tx: a_tx, rx: a_rx }, Mem { tx: b_tx, rx: b_rx })
}

impl Channel for Mem {
    async fn send(&mut self, data: &[u8]) -> Result<(), ChannelError> {
        tokio::task::yield_now().await;
        self.tx.send(data.to_vec()).map_err(|_| ChannelError)
    }
    async fn recv(&mut self) -> Result<Vec<u8>, ChannelError> {
        self.rx.recv().await.ok_or(ChannelError)
    }
}

type Helper = Arc<Mutex<Client<pctwin_helper::client::Channel>>>;

/// Links in `holding` to the system's own folder and files, as a hostile mover would plant them:
/// where each was planted.
fn plant_links(holding: &std::path::Path) -> Result<Vec<String>, String> {
    let mut planted = Vec::new();
    #[cfg(unix)]
    for (target, link) in [
        ("/etc", "Documents/etc-link"),
        ("/etc/passwd", "passwd-link"),
    ] {
        let at = holding.join(link);
        if let Some(parent) = at.parent() {
            std::fs::create_dir_all(parent).map_err(err)?;
        }
        std::os::unix::fs::symlink(target, &at).map_err(err)?;
        planted.push(at.to_string_lossy().into_owned());
    }
    #[cfg(windows)]
    {
        // A junction needs no special right, so any mover can make one.
        let root = std::env::var_os("SystemRoot").unwrap_or_else(|| r"C:\Windows".into());
        let target = std::path::Path::new(&root).join(r"System32\drivers\etc");
        let documents = holding.join("Documents");
        std::fs::create_dir_all(&documents).map_err(err)?;
        let at = documents.join("etc-junction");
        let made = std::process::Command::new("cmd")
            .arg("/c")
            .arg("mklink")
            .arg("/J")
            .arg(&at)
            .arg(&target)
            .output()
            .map_err(err)?;
        if !made.status.success() {
            return Err(format!(
                "mklink: {}",
                String::from_utf8_lossy(&made.stderr).trim()
            ));
        }
        planted.push(at.to_string_lossy().into_owned());
    }
    Ok(planted)
}

/// Calls the helper and says goodbye, nothing more: whether it served this account.
pub fn hello(token: &str, channel: &str, dir: Option<&std::path::Path>) -> Result<(), String> {
    let h = HelperChannel {
        session: token.to_string(),
        channel: channel.to_string(),
        dir: dir.map(PathBuf::from),
    };
    connect(&h)?.bye().map_err(err)
}

fn connect(h: &HelperChannel) -> Result<Client<pctwin_helper::client::Channel>, String> {
    let started = Instant::now();
    loop {
        match pctwin_helper::connect(&h.channel, h.dir.as_deref(), &h.session) {
            Ok(c) => return Ok(c),
            Err(ClientError::Unreachable(_)) if started.elapsed() < HELPER_WAIT => {
                std::thread::sleep(Duration::from_millis(200));
            }
            Err(e) => return Err(format!("the helper: {e}")),
        }
    }
}

/// The new laptop's choices: the mover stays the mover; each approved person goes to the account
/// made for them; the plan is approved as it is.
struct EveryoneSteps {
    targets: BTreeMap<String, PersonTarget>,
    rules: LandingRules,
}

impl Steps for EveryoneSteps {
    async fn person(&mut self, _: &Record, account_id: &str) -> Result<PersonTarget, Stop> {
        Ok(self
            .targets
            .get(account_id)
            .cloned()
            .unwrap_or(PersonTarget::Skip))
    }
    async fn approve(&mut self, _: &Record, _: &pctwin_move::Preview) -> Result<(), Stop> {
        Ok(())
    }
    fn landing(&self, item: &Item) -> Option<LandingPlace> {
        self.rules.landing(item)
    }
}

/// Where each of the mover's (and the shared) items is on this laptop, from the sweep.
fn sources(sweep: &Sweep) -> HashMap<pctwin_record::ItemId, PathBuf> {
    let place_of: HashMap<_, _> = sweep.placed_by.iter().copied().collect();
    sweep
        .record
        .items
        .iter()
        .filter_map(|i| {
            let mut path = sweep.item_roots.get(place_of.get(&i.id)?)?.clone();
            for part in i.path.parts() {
                path.push(part.on_disk());
            }
            Some((i.id, path))
        })
        .collect()
}

fn result_word(r: FileResult) -> &'static str {
    match r {
        FileResult::Copied => "copied",
        FileResult::NeedsAction => "needs-action",
        FileResult::Failed => "failed",
    }
}

/// The mover's report, as the mover could save it: nothing in it names another person's file.
fn report_json(sweep: &Sweep, report: &MoverReport, declined: &[String], lines: &[Value]) -> Value {
    json!({
        "top_line": sweep.top_line.words(),
        "who": sweep.who_is_moving().iter().map(|p| json!({
            "account_id": p.account_id,
            "name": p.suggested_name,
            "kind": format!("{:?}", p.kind),
            "bytes": p.bytes,
        })).collect::<Vec<_>>(),
        "blind_spots": sweep.blind_spots.iter().map(|b| json!({
            "volume": b.volume,
            "path": b.path,
            "reason": serde_json::to_value(&b.reason).unwrap_or(Value::Null),
            "count": b.count,
        })).collect::<Vec<_>>(),
        "own": report.own.iter().map(|l| json!({
            "path": l.path,
            "result": result_word(l.result),
            "why": l.why.as_ref().map(|(_, w)| w.clone()),
            "changed_since_scan": l.changed_since_scan,
        })).collect::<Vec<_>>(),
        "people": report.people.iter().map(|(id, p)| json!({
            "account_id": id,
            "copied": p.copied,
            "needs_action": p.needs_action,
            "failed": p.failed,
            "bytes_copied": p.bytes_copied,
        })).collect::<Vec<_>>(),
        "not_approved": declined,
        "hand_over": lines,
    })
}

/// Runs the move as the mover.
pub async fn everyone(args: Everyone) -> Result<Ran, String> {
    // Carrying other people's things: hardened before anything of theirs is read.
    if args.helper.is_some() {
        pctwin_helper::harden_engine()?;
    }
    let laptop = args.laptop;
    let sweep = tokio::task::spawn_blocking(move || {
        pctwin_scan::sweep_this_laptop(laptop, &AtomicBool::new(false), &mut |_| {}, None)
    })
    .await
    .map_err(err)?;
    let me = sweep.census.me.clone();
    let others: Vec<String> = sweep
        .census
        .residents
        .iter()
        .filter(|r| r.kind == PersonKind::Other)
        .filter_map(|r| r.account_id.clone())
        .collect();
    let mut record = sweep.record.clone();
    let by_path = sources(&sweep);

    // The helper: each other person scanned, if the administrator approved them.
    let helper: Option<Helper> = match &args.helper {
        Some(h) => {
            let h = h.clone_args();
            let client = tokio::task::spawn_blocking(move || connect(&h))
                .await
                .map_err(err)??;
            Some(Arc::new(Mutex::new(client)))
        }
        None => None,
    };
    let mut scanned: Vec<(String, Scanned)> = Vec::new();
    let mut declined = Vec::new();
    if let Some(helper) = &helper {
        for person in &others {
            let answer = helper.lock().map_err(err)?.scan_person(person);
            match answer {
                Ok(s) => scanned.push((person.clone(), s)),
                Err(ClientError::Refused(Refusal::NotApproved)) => declined.push(person.clone()),
                Err(e) => return Err(format!("scanning another person: {e}")),
            }
        }
    } else {
        declined.clone_from(&others);
    }
    let people: Vec<Scanned> = scanned.iter().map(|(_, s)| s.clone()).collect();
    let handles = add_people(&mut record, &people);
    record.validate().map_err(err)?;

    // The new laptop: the mover's new home, and for each approved person an account and the
    // place their things are held in until the hand-over.
    let mut table = Destinations::new();
    for role in NEW_ROLES.into_iter().filter(|r| *r != FolderRole::Public) {
        let folder = args.new_home.join(role_label(role));
        std::fs::create_dir_all(&folder).map_err(err)?;
        table
            .approve(role_label(role), Approved::MyFolders, &folder)
            .map_err(err)?;
    }
    let public = args.new_home.join(role_label(FolderRole::Public));
    std::fs::create_dir_all(&public).map_err(err)?;
    table
        .approve(
            role_label(FolderRole::Public),
            Approved::SharedFolder,
            &public,
        )
        .map_err(err)?;
    let mut targets = BTreeMap::from([(
        me.clone(),
        PersonTarget::ExistingAccount {
            account_id: me.clone(),
        },
    )]);
    let mut held = BTreeMap::new();
    let mut made = Vec::new();
    if let Some(helper) = &helper {
        for (n, (person, _)) in scanned.iter().enumerate() {
            let mut client = helper.lock().map_err(err)?;
            let name = format!("{}{}", args.new_names, n + 1);
            let account = client
                .create_account(&name, &format!("PCTwin lab {}", n + 1))
                .map_err(|e| format!("making an account: {e}"))?;
            let holding = client
                .hold_for(&account.account)
                .map_err(|e| format!("holding: {e}"))?;
            let label = format!("for-{}", n + 1);
            table
                .approve_holding(&label, &account.account, &holding)
                .map_err(err)?;
            held.insert(person.clone(), label);
            targets.insert(
                person.clone(),
                PersonTarget::ExistingAccount {
                    account_id: account.account.clone(),
                },
            );
            made.push(json!({
                "old_account": person,
                "name": name,
                "account": account.account,
                "secure_token": account.secure_token,
                "holding": holding,
            }));
        }
    }
    // The very rules the app's new laptop sends the old one: both laptops place every file the
    // same way, other people's under their holding place by place label, as the hand-over reads.
    let rules = LandingRules {
        mover: me.clone(),
        roles: NEW_ROLES
            .iter()
            .map(|r| (*r, pctwin_record::Storage::SystemDrive))
            .collect(),
        fallback: BTreeMap::new(),
        held,
        left_out: Default::default(),
        chosen: Default::default(),
    };
    let plan = pctwin_move::send_plan_where(
        &record,
        |i| match handles.get(&i.id) {
            Some(h) => Some(helper_source(*h)),
            None => by_path.get(&i.id).cloned(),
        },
        |i| rules.landing(i),
    );
    let opener: Arc<dyn pctwin_transfer::Opener> = match &helper {
        Some(h) => Arc::new(HelperOpener::new(Arc::clone(h), &people)),
        None => Arc::new(pctwin_transfer::FsOpener),
    };
    let journal = Journal::open(&args.journal).map_err(err)?;
    let mut steps = EveryoneSteps {
        targets,
        rules: rules.clone(),
    };
    let (mut a, mut b) = mem_pair();
    let (here, there) = (Live::default(), Live::default());
    let (sent, received) = tokio::join!(
        pctwin_move::send_with(&mut a, &record, plan, &here, opener),
        pctwin_move::receive(&mut b, &mut steps, &table, &journal, &me, &there),
    );
    let (sent, received) = (sent.map_err(err)?, received.map_err(err)?);

    let mut planted_links = Vec::new();
    if args.hostile {
        for m in &made {
            let holding = PathBuf::from(m["holding"].as_str().unwrap_or_default());
            planted_links.extend(plant_links(&holding)?);
        }
        if planted_links.is_empty() && !made.is_empty() {
            return Err("no link could be planted: the hostile check would prove nothing".into());
        }
    }

    // The hand-over, now, as the administrator (the CI runner has no one to sign in).
    let mut hand_over = Vec::new();
    if let Some(helper) = &helper {
        let mut client = helper.lock().map_err(err)?;
        for m in &made {
            let account = m["account"].as_str().unwrap_or_default();
            let done = client
                .hand_over_now(account)
                .map_err(|e| format!("handing over: {e}"))?;
            hand_over.push(json!({
                "account": account,
                "landed": done.landed,
                "kept": done.kept,
                "failed": done.failed,
            }));
        }
    }
    if let Some(helper) = helper {
        let client = Arc::try_unwrap(helper)
            .map_err(|_| "the helper is still in use".to_string())?
            .into_inner()
            .map_err(err)?;
        client.bye().map_err(err)?;
    }

    let mover = for_the_mover(&received, &me);
    let mut report = report_json(&sweep, &mover, &declined, &hand_over);
    // macOS: the helper was kept out of some of someone's places; the app shows how to give it
    // Full Disk Access, and those places are never counted as moved.
    report["helper_needs_full_disk_access"] =
        json!(scanned.iter().any(|(_, s)| s.privacy_blocked > 0));
    let items: BTreeMap<_, _> = received.record.items.iter().map(|i| (i.id, i)).collect();
    let outcomes: Vec<Value> = received
        .files
        .iter()
        .filter_map(|f| {
            let i = items.get(&f.item)?;
            Some(json!({
                "owner": match &i.owner {
                    Owner::Person { account_id } => account_id.clone(),
                    Owner::Shared => "shared".into(),
                    Owner::System => "system".into(),
                },
                "role": role_label(i.place.role),
                "path": shown_path(&i.path),
                "source": by_path.get(&i.id),
                "outcome": format!("{:?}", f.outcome),
            }))
        })
        .collect();
    let evidence = json!({
        "me": me,
        "others": others,
        "approved": scanned.iter().map(|(p, s)| json!({"account": p, "files": s.files, "bytes": s.bytes, "privacy_blocked": s.privacy_blocked})).collect::<Vec<_>>(),
        "declined": declined,
        "made": made,
        "places": serde_json::to_value(&sweep.places).map_err(err)?,
        "blind_spots": serde_json::to_value(&sweep.blind_spots).map_err(err)?,
        "finished": sweep.finished,
        "top_line": serde_json::to_value(sweep.top_line).map_err(err)?,
        "received": outcomes,
        "sent": sent.files.iter().map(|f| format!("{:?}", f.outcome)).collect::<Vec<_>>(),
        "not_sent": sent.not_sent.len(),
        "planted_links": planted_links,
    });
    Ok(Ran { report, evidence })
}

impl HelperChannel {
    fn clone_args(&self) -> HelperChannel {
        HelperChannel {
            session: self.session.clone(),
            channel: self.channel.clone(),
            dir: self.dir.clone(),
        }
    }
}

/// Reads the arguments after `everyone`:
/// `--laptop <id> --new-home <folder> --journal <file> --new-names <prefix>
///  [--session <token> --channel <name> [--dir <folder>]]`.
pub fn parse(args: &[String]) -> Result<Everyone, String> {
    let mut map: BTreeMap<&str, &str> = BTreeMap::new();
    let mut it = args.iter();
    while let Some(flag) = it.next() {
        let value = it.next().ok_or(format!("{flag} needs a value"))?;
        if map.insert(flag.as_str(), value.as_str()).is_some() {
            return Err(format!("{flag} given twice"));
        }
    }
    let need = |k: &str| map.get(k).copied().ok_or(format!("{k} is needed"));
    let helper = match (map.get("--session"), map.get("--channel")) {
        (Some(s), Some(c)) => Some(HelperChannel {
            session: (*s).to_string(),
            channel: (*c).to_string(),
            dir: map.get("--dir").map(PathBuf::from),
        }),
        (None, None) => None,
        _ => return Err("--session and --channel go together".into()),
    };
    for k in map.keys() {
        if ![
            "--laptop",
            "--new-home",
            "--journal",
            "--new-names",
            "--hostile",
            "--session",
            "--channel",
            "--dir",
        ]
        .contains(k)
        {
            return Err(format!("unknown {k}"));
        }
    }
    Ok(Everyone {
        laptop: LaptopId::from_hex(need("--laptop")?).map_err(err)?,
        helper,
        new_home: PathBuf::from(need("--new-home")?),
        journal: PathBuf::from(need("--journal")?),
        new_names: need("--new-names")?.to_string(),
        hostile: map.get("--hostile") == Some(&"yes"),
    })
}

/// Whether `text` holds any of `names` (the leak check): the planted names of other people's
/// things, which must never reach what the mover can read.
pub fn leaks<'a>(text: &str, names: &'a [String]) -> Vec<&'a str> {
    names
        .iter()
        .filter(|n| !n.is_empty() && text.contains(n.as_str()))
        .map(String::as_str)
        .collect()
}

/// The bytes the lab plants for a file named `name` of `size` bytes: the name and a line end,
/// over and over (the scripts make them the same way).
pub fn planted_bytes(name: &str, size: u64) -> Vec<u8> {
    let line: Vec<u8> = format!(
        "{name}
"
    )
    .into_bytes();
    line.iter().copied().cycle().take(size as usize).collect()
}

fn files_named(root: &std::path::Path, name: &str, out: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(root) else {
        return;
    };
    for e in entries.flatten() {
        let Ok(kind) = e.file_type() else { continue };
        if kind.is_symlink() {
            continue;
        }
        if kind.is_dir() {
            files_named(&e.path(), name, out);
        } else if e.file_name().to_string_lossy() == name {
            out.push(e.path());
        }
    }
}

/// Checks a run against the lab's answer key (held by the administrator): every problem, in
/// words; none means the run passed. `visible` is everything the mover can read (their report
/// and their program's own output), each with a name for the problem it would cause.
///
/// The key: `mover`; `new_home`; `new_homes` (each new account the helper made, by its ID, to its
/// home folder); `planted`, each `{owner, path, name, size, expect}`, where `owner` is an account
/// ID or `shared` and `expect` is `moves` (it must arrive, in the right person's place, byte for
/// byte), `stays` (a person not approved: it must arrive nowhere) or `blind` (the sweep must name
/// it a blind spot, and never say everything is accounted for).
pub fn check(key: &Value, evidence: &Value, visible: &[(String, String)]) -> Vec<String> {
    let mut problems = Vec::new();
    let s = |v: &Value| v.as_str().unwrap_or_default().to_string();
    let mover = s(&key["mover"]);
    let new_home = PathBuf::from(s(&key["new_home"]));
    let new_of: BTreeMap<String, String> = evidence["made"]
        .as_array()
        .into_iter()
        .flatten()
        .map(|m| (s(&m["old_account"]), s(&m["account"])))
        .collect();
    let home_of = |old: &str| -> Option<PathBuf> {
        let new = new_of.get(old)?;
        key["new_homes"][new].as_str().map(PathBuf::from)
    };
    let mut everywhere = vec![new_home.clone()];
    everywhere.extend(
        key["new_homes"]
            .as_object()
            .into_iter()
            .flatten()
            .filter_map(|(_, h)| h.as_str().map(PathBuf::from)),
    );
    let blind: Vec<String> = evidence["blind_spots"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|b| b["path"].as_str().map(String::from))
        .collect();
    let mut any_blind = false;
    for p in key["planted"].as_array().into_iter().flatten() {
        let (owner, path, name, expect) = (
            s(&p["owner"]),
            s(&p["path"]),
            s(&p["name"]),
            s(&p["expect"]),
        );
        let size = p["size"].as_u64().unwrap_or(0);
        let want = planted_bytes(&name, size);
        // The old laptop is read-only: the planted file is exactly as it was made.
        if std::fs::read(&path).ok().as_deref() != Some(want.as_slice()) {
            problems.push(format!("{path}: changed or gone on the old laptop"));
        }
        // Another person's file is never named to the mover.
        if owner != mover && owner != "shared" {
            for (what, text) in visible {
                if text.contains(&name) {
                    problems.push(format!("{name} (another person's) is named in {what}"));
                }
            }
        }
        let mut found = Vec::new();
        match expect.as_str() {
            "moves" => {
                let place = if owner == mover || owner == "shared" {
                    Some(new_home.clone())
                } else {
                    home_of(&owner)
                };
                let Some(place) = place else {
                    problems.push(format!("{path}: no new account for its person"));
                    continue;
                };
                files_named(&place, &name, &mut found);
                if !found
                    .iter()
                    .any(|f| std::fs::read(f).ok().as_deref() == Some(want.as_slice()))
                {
                    problems.push(format!("{path}: did not arrive in {}", place.display()));
                }
                // Never in anyone else's place.
                for other in everywhere.iter().filter(|e| **e != place) {
                    let mut stray = Vec::new();
                    files_named(other, &name, &mut stray);
                    if !stray.is_empty() {
                        problems.push(format!("{path}: also arrived in {}", other.display()));
                    }
                }
            }
            "stays" => {
                for place in &everywhere {
                    files_named(place, &name, &mut found);
                }
                if !found.is_empty() {
                    problems.push(format!("{path}: moved, though its person was not approved"));
                }
            }
            "blind" => {
                any_blind = true;
                if !blind.iter().any(|b| path.starts_with(b.as_str())) {
                    problems.push(format!("{path}: not named as a blind spot"));
                }
            }
            other => problems.push(format!("{path}: unknown expectation {other}")),
        }
    }
    if any_blind && evidence["top_line"]["line"] == "everything" {
        problems.push("said everything is accounted for while a blind spot remains".into());
    }
    // No invented items: everything the mover's side moved of theirs is really there.
    for r in evidence["received"].as_array().into_iter().flatten() {
        if let Some(source) = r["source"].as_str()
            && !std::path::Path::new(source).exists()
        {
            problems.push(format!("{source}: moved, but there is no such file"));
        }
    }
    problems
}
