//! A headless move for the test lab (Task List 0.7): the old laptop and the new laptop as two
//! processes, which CI runs under two different user accounts on one machine.
//!
//! Both sides use the real engine: the old laptop scans with `pctwin-scan`, hosts the pairing code
//! and sends with `pctwin-transfer`; the new laptop connects with the code, approves the plan,
//! and receives through the safety gate (`pctwin-gate`) with every write in the change journal.
//!
//! What a person does on screen is done through a shared "handoff" folder instead: the old laptop
//! writes its address and code there, the new laptop writes the number it shows, and the old
//! laptop picks it. The scanned record also travels through the handoff folder, because sending
//! it over the connection is still to be built (Phase 1); the new laptop checks it exactly as it
//! will check one that arrives over the connection.
//!
//! Each side ends by writing a report of every item it handled. A side never claims a file moved
//! unless the engine said so.

use std::collections::BTreeMap;
use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use pctwin_gate::{Approved, Destinations};
use pctwin_journal::Journal;
use pctwin_link::LinkConfig;
use pctwin_pairing::{PairingCode, RotatingSender};
use pctwin_record::{
    FolderRole, Inclusion, ItemKind, ItemName, LaptopId, PersonTarget, RawName, Record, Storage,
};
use pctwin_scan::{FolderLookup, FoundFolder, Person};
use pctwin_transfer::{
    Allowance, ReceiveOutcome, ReceiverSession, SendJob, SendOutcome, SenderSession, Tier,
    plan_order, role_label,
};
use serde::Serialize;

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
}

/// One item in a side's report.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Line {
    pub item: String,
    pub path: String,
    pub outcome: String,
}

/// A side's report: every item it handled, and anything that stopped it.
#[derive(Debug, Default, Serialize)]
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
            })
        })
        .collect()
}

/// The name on disk, exactly as the scan recorded it.
fn on_disk(name: &ItemName) -> OsString {
    match name.exact() {
        #[cfg(unix)]
        Some(RawName::Unix(bytes)) => {
            use std::os::unix::ffi::OsStringExt;
            OsString::from_vec(bytes.clone())
        }
        #[cfg(windows)]
        Some(RawName::Windows(units)) => {
            use std::os::windows::ffi::OsStringExt;
            OsString::from_wide(units)
        }
        _ => OsString::from(name.display()),
    }
}

fn compressible(path: &str) -> bool {
    let ext = path.rsplit('.').next().unwrap_or("").to_ascii_lowercase();
    !matches!(
        ext.as_str(),
        "jpg" | "jpeg" | "png" | "mp3" | "mp4" | "zip" | "docx" | "xlsx" | "gz" | "7z"
    )
}

/// The old laptop: scans, hosts pairing on `bind`, sends, and reports.
pub async fn send(folders: Folders, handoff: &Path, bind: &str) -> Result<Report, String> {
    let laptop = LaptopId::generate().map_err(err)?;
    let (scan, places) = match &folders {
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
            (scan, found)
        }
        Folders::Profile => {
            let scan = pctwin_scan::scan_this_laptop(laptop).map_err(err)?;
            let found = scan.folders.clone();
            (scan, found)
        }
    };
    let record = scan.record.clone();
    let roots: BTreeMap<FolderRole, PathBuf> = places
        .iter()
        .filter_map(|f| match f {
            FolderLookup::Found(f) => Some((f.role, f.path.clone())),
            _ => None,
        })
        .collect();

    let mut report = Report::default();
    let mut by_item = BTreeMap::new();
    let mut shown_of = BTreeMap::new();
    for item in &record.items {
        let path = item
            .path
            .parts()
            .iter()
            .map(ItemName::display)
            .collect::<Vec<_>>()
            .join("/");
        let shown = format!("{}/{path}", role_name(item.place.role));
        let line = |outcome: &str| Line {
            item: item.id.to_hex(),
            path: shown.clone(),
            outcome: outcome.into(),
        };
        if item.kind != ItemKind::File {
            continue;
        }
        if let Inclusion::LeftOut { reason } = item.inclusion {
            report.not_sent.push(line(&format!("left out: {reason:?}")));
            continue;
        }
        let Some(root) = roots.get(&item.place.role) else {
            report.not_sent.push(line("no folder for its role"));
            continue;
        };
        let mut source = root.clone();
        for part in item.path.parts() {
            source.push(on_disk(part));
        }
        shown_of.insert(item.id, shown.clone());
        by_item.insert(
            item.id,
            SendJob {
                item: item.id,
                source,
                destination: role_label(item.place.role).to_string(),
                path: path.clone(),
                compressible: compressible(&path),
                tier: Tier::Rest,
            },
        );
    }
    // The order the app moves things in.
    let order = plan_order(&record.items, &[], &BTreeMap::new(), 0);
    let mut jobs = Vec::new();
    for (id, tier) in order {
        if let Some(mut job) = by_item.remove(&id) {
            job.tier = tier;
            jobs.push(job);
        }
    }
    jobs.extend(by_item.into_values());

    hand(handoff, "record.json", &record.to_json())?;
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
    let rotating = Arc::new(Mutex::new(RotatingSender::new(Instant::now()).map_err(err)?));
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

    let mut sender = SenderSession::new(jobs.clone(), 4);
    sender.run(&mut link).await.map_err(err)?;
    for job in &jobs {
        let outcome = match sender.outcome(job.item) {
            Some(SendOutcome::Arrived) => "arrived".to_string(),
            Some(SendOutcome::AlreadyThere) => "already-there".to_string(),
            Some(SendOutcome::Failed(why)) => format!("failed: {why}"),
            None => "no outcome".to_string(),
        };
        report.lines.push(Line {
            item: job.item.to_hex(),
            path: shown_of.get(&job.item).cloned().unwrap_or_default(),
            outcome,
        });
    }
    Ok(report)
}

/// The new laptop: checks and approves the record, connects with the code, receives into
/// `folders`, and reports. `account` is this side's account ID.
pub async fn receive(folders: Folders, handoff: &Path, journal: &Path) -> Result<Report, String> {
    let (account, mut table) = (match &folders {
        Folders::Lab { account, .. } => account.clone(),
        Folders::Profile => pctwin_scan::list_people()
            .into_iter()
            .find(|p| p.is_me)
            .map(|p| p.account_id)
            .ok_or("no signed-in person")?,
    }, Destinations::new());
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
    }

    let mut record = Record::from_json(&take(handoff, "record.json").await?).map_err(err)?;
    // The person's choice on screen: the old laptop's person lands in this account.
    let people: Vec<String> = record
        .items
        .iter()
        .filter_map(|i| match &i.owner {
            pctwin_record::Owner::Person { account_id } => Some(account_id.clone()),
            _ => None,
        })
        .collect();
    for person in people {
        record.mapping.people.insert(
            person,
            PersonTarget::ExistingAccount {
                account_id: account.clone(),
            },
        );
    }
    let approval = record.approve().map_err(err)?;
    let allowance = Allowance::from_record(&record, &approval).map_err(err)?;

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
    hand(handoff, "number.txt", pending.match_number().to_string().as_bytes())?;
    let mut link = pending.approval().await.map_err(err)?;

    let journal = Journal::open(journal).map_err(err)?;
    let mut receiver = ReceiverSession::new(&table, allowance, &journal, &account);
    receiver.run(&mut link).await.map_err(err)?;

    let mut report = Report::default();
    for item in record.items.iter().filter(|i| i.kind == ItemKind::File) {
        if !matches!(item.inclusion, Inclusion::Included) {
            continue;
        }
        let folder = role_name(item.place.role);
        let outcome = match receiver.outcome(item.id) {
            Some(ReceiveOutcome::Finished(f)) => format!("landed: {folder}/{}", f.final_path),
            Some(ReceiveOutcome::AlreadyThere(p)) => format!("already-there: {p}"),
            Some(ReceiveOutcome::Failed(why)) => format!("failed: {why}"),
            None => "no outcome".to_string(),
        };
        report.lines.push(Line {
            item: item.id.to_hex(),
            path: format!(
                "{folder}/{}",
                item.path
                    .parts()
                    .iter()
                    .map(ItemName::display)
                    .collect::<Vec<_>>()
                    .join("/")
            ),
            outcome,
        });
    }
    Ok(report)
}
