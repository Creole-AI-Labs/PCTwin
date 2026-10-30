//! One session of the helper: one app, one move, one connection. The app must say the session's
//! token first; the helper then scans only the people the administrator approved, keeps their
//! files in its own plan, and reads only from that plan, by handle. Nothing else is accepted, and
//! nothing a request says is ever taken as a path.

use std::collections::{BTreeMap, BTreeSet};
use std::io::{Read, Write};

use pctwin_gate::FileId;

use pctwin_record::Item;

use crate::protocol::{
    Answer, FrameError, Handle, ITEMS_PER_FRAME, MAX_HANDLES, Refusal, Request, read_request,
    send_answer, send_blocks, send_handles, send_items,
};
use crate::read::{Entry, read_range};

/// Finds an approved person's files (see [`crate::person::scan_person`]).
pub type Scanner<'a> = dyn FnMut(&str) -> Result<crate::person::PersonScan, Refusal> + 'a;

/// Makes an approved person's account (see [`crate::account::create_account`]).
pub type AccountMaker<'a> = dyn FnMut(&str, &str) -> Result<crate::account::Made, Refusal> + 'a;

/// Reads the saved Wi-Fi networks named (the ones the person ticked) with their passwords, and
/// no other (see [`crate::wifi`]).
pub type WifiReader<'a> =
    dyn FnMut(&[String]) -> Result<Vec<crate::protocol::Secret>, Refusal> + 'a;

/// What the helper was started for.
pub struct Session<'s> {
    /// The token the app was given when it started the helper.
    token: String,
    /// The people the administrator approved, by account ID (never the mover).
    approved: BTreeSet<String>,
    opened: bool,
    /// Every file handed out, by handle (its index), with its identity once first read.
    plan: Vec<(Entry, Option<FileId>)>,
    scanner: &'s mut Scanner<'s>,
    maker: Option<&'s mut AccountMaker<'s>>,
    new_laptop: Option<&'s mut dyn crate::newlaptop::NewLaptop>,
    /// Accounts made in this session: name to account ID.
    made: BTreeMap<String, String>,
    /// This move's holding area, once made, and each person's folder in it.
    area: Option<std::path::PathBuf>,
    held: BTreeMap<String, std::path::PathBuf>,
    /// The places the helper's own sweep found only the administrators and the system may open,
    /// outside every home: kept here, never sent; only yes or no about the mover's own places.
    system_only: Vec<std::path::PathBuf>,
    /// Places asked about so far (at most [`crate::MAX_SYSTEM_ASKED`]).
    asked: usize,
    /// Phase 2: the permission broker, when the helper was started with an approved plan.
    broker: Option<crate::broker::Broker<'s>>,
    /// The saved Wi-Fi networks, when the helper was started with `--wifi yes`; read at most
    /// once (then `None`).
    wifi: Option<&'s mut WifiReader<'s>>,
}

impl<'s> Session<'s> {
    pub fn new(token: String, approved: BTreeSet<String>, scanner: &'s mut Scanner<'s>) -> Self {
        Self {
            token,
            approved,
            opened: false,
            plan: Vec::new(),
            scanner,
            maker: None,
            new_laptop: None,
            made: BTreeMap::new(),
            area: None,
            held: BTreeMap::new(),
            system_only: Vec::new(),
            asked: 0,
            broker: None,
            wifi: None,
        }
    }

    /// With the saved Wi-Fi networks to read once, for a helper the administrator started for
    /// them (`--wifi yes`). Without, [`Request::WifiProfiles`] is refused.
    pub fn with_wifi(mut self, reader: &'s mut WifiReader<'s>) -> Self {
        self.wifi = Some(reader);
        self
    }

    /// With the permission broker for the plan it was started with (Phase 2.2).
    pub fn with_broker(mut self, broker: crate::broker::Broker<'s>) -> Self {
        self.broker = Some(broker);
        self
    }

    /// The broker, if there is one (it is closed once the session ends).
    pub fn broker(&self) -> Option<&crate::broker::Broker<'s>> {
        self.broker.as_ref()
    }

    fn broker_answer(&mut self, request: Request) -> Answer {
        use crate::broker::{Answer as Broker, Denial};
        let Some(broker) = self.broker.as_mut() else {
            return Answer::Denied(Denial::NotAvailable);
        };
        match request {
            Request::SealPlan { plan } => match broker.seal(plan) {
                Ok(()) => Answer::Sealed,
                Err(why) => Answer::Denied(why),
            },
            Request::Act(request) => match broker.handle(&request) {
                Broker::Done(receipt) => Answer::Receipt(receipt),
                Broker::Denied(why) => Answer::Denied(why),
            },
            Request::Revoke => {
                broker.revoker().revoke();
                broker.close();
                Answer::Denied(Denial::Revoked)
            }
            Request::StagePackage {
                artifact,
                sha256,
                size,
            } => match broker.stage_begin(artifact, sha256, size) {
                Ok(()) => Answer::Staging,
                Err(why) => Answer::Denied(why),
            },
            Request::StageBytes { bytes } => {
                use base64::Engine as _;
                // Decoded only if its text could hold no more than one piece.
                let decoded = (bytes.len() <= crate::protocol::STAGE_PIECE.div_ceil(3) * 4)
                    .then(|| {
                        base64::engine::general_purpose::STANDARD
                            .decode(bytes.as_bytes())
                            .ok()
                    })
                    .flatten();
                match decoded {
                    Some(piece) => match broker.stage_bytes(&piece) {
                        Ok(None) => Answer::Staging,
                        Ok(Some(artifact)) => Answer::Staged { artifact },
                        Err(why) => Answer::Denied(why),
                    },
                    None => {
                        // A piece that cannot be read ends the package.
                        broker.stage_abort();
                        Answer::Denied(Denial::Unsafe)
                    }
                }
            }
            _ => Answer::Denied(Denial::NotAvailable),
        }
    }

    /// On the new laptop: holding areas and hand-overs, by `system`.
    pub fn with_new_laptop(mut self, system: &'s mut dyn crate::newlaptop::NewLaptop) -> Self {
        self.new_laptop = Some(system);
        self
    }

    /// An account made in this session, named by its name or by the ID the helper gave back for
    /// it (the app holds and hands over by that ID): its name and ID.
    fn made_account(&self, account: &str) -> Option<(&String, &String)> {
        self.made
            .get_key_value(account)
            .or_else(|| self.made.iter().find(|(_, id)| id.as_str() == account))
    }

    /// The one key a person's held folder is kept under, however the app names them: a made
    /// account's name, else the approved ID as given.
    fn held_key(&self, account: &str) -> String {
        self.made_account(account)
            .map_or_else(|| account.to_string(), |(name, _)| name.clone())
    }

    /// The account `account` names: one made in this session (by its name or ID), or an approved
    /// account ID; and its name, when the helper made it.
    fn account_of(&self, account: &str) -> Option<(pctwin_helper_os::Account, String)> {
        let (id, name) = match self.made_account(account) {
            Some((name, id)) => (id.clone(), name.clone()),
            None if self.approved.contains(account) => (account.to_string(), String::new()),
            None => return None,
        };
        let parsed = match id.parse::<u32>() {
            Ok(uid) => pctwin_helper_os::Account::Uid(uid),
            Err(_) => pctwin_helper_os::Account::Sid(pctwin_helper_os::Sid::parse(&id).ok()?),
        };
        Some((parsed, name))
    }

    fn hold_for(&mut self, account: &str) -> Answer {
        if self.account_of(account).is_none() {
            return Answer::Refused(Refusal::NotApproved);
        }
        let held_key = self.held_key(account);
        let Some(system) = self.new_laptop.as_mut() else {
            return Answer::Refused(Refusal::NotApproved);
        };
        if let Some(path) = self.held.get(&held_key) {
            return Answer::Holding {
                path: path.to_string_lossy().into_owned(),
            };
        }
        let area = match &self.area {
            Some(a) => a.clone(),
            None => match system.area() {
                Ok(a) => {
                    self.area = Some(a.clone());
                    a
                }
                Err(why) => return Answer::Refused(why),
            },
        };
        // Named by the helper, never by the app.
        let key = format!("p{}", self.held.len() + 1);
        let Some((person, _)) = self.account_of(account) else {
            return Answer::Refused(Refusal::NotApproved);
        };
        let Some(system) = self.new_laptop.as_mut() else {
            return Answer::Refused(Refusal::NotApproved);
        };
        match system.person_folder(&area, &key, &person) {
            Ok(path) => {
                self.held.insert(held_key, path.clone());
                Answer::Holding {
                    path: path.to_string_lossy().into_owned(),
                }
            }
            Err(why) => Answer::Refused(why),
        }
    }

    fn hand_over_at_sign_in(&mut self, account: &str) -> Answer {
        let Some((person, name)) = self.account_of(account) else {
            return Answer::Refused(Refusal::NotApproved);
        };
        let Some(folder) = self.held.get(&self.held_key(account)).cloned() else {
            return Answer::Refused(Refusal::NothingHeld);
        };
        let key = folder
            .file_name()
            .map(|k| k.to_string_lossy().into_owned())
            .unwrap_or_default();
        let entry = format!(
            "{}{}",
            self.area
                .as_ref()
                .and_then(|a| a.file_name())
                .map(|a| a.to_string_lossy().chars().take(16).collect::<String>())
                .unwrap_or_default(),
            key
        );
        let Some(system) = self.new_laptop.as_mut() else {
            return Answer::Refused(Refusal::NotApproved);
        };
        match system.hand_over_at_sign_in(&person, &name, &folder, &entry) {
            Ok(()) => Answer::AtSignIn,
            Err(why) => Answer::Refused(why),
        }
    }

    fn hand_over_now(&mut self, account: &str) -> Answer {
        let Some((parsed, name)) = self.account_of(account) else {
            return Answer::Refused(Refusal::NotApproved);
        };
        let Some(folder) = self.held.get(&self.held_key(account)).cloned() else {
            return Answer::Refused(Refusal::NothingHeld);
        };
        let Some(system) = self.new_laptop.as_mut() else {
            return Answer::Refused(Refusal::NotApproved);
        };
        match system.hand_over(&parsed, &name, &folder) {
            Ok(c) => Answer::HandedOver {
                landed: c.landed,
                kept: c.kept,
                failed: c.failed,
            },
            Err(why) => Answer::Refused(why),
        }
    }

    /// On the new laptop: the people named in `approved` may have their accounts made, by
    /// `maker`.
    pub fn with_account_maker(mut self, maker: &'s mut AccountMaker<'s>) -> Self {
        self.maker = Some(maker);
        self
    }

    fn create(&mut self, name: &str, full_name: &str) -> Answer {
        if !self.approved.contains(name) {
            return Answer::Refused(Refusal::NotApproved);
        }
        let Some(maker) = self.maker.as_mut() else {
            return Answer::Refused(Refusal::NotApproved);
        };
        match maker(name, full_name) {
            Ok(made) => {
                self.made.insert(name.to_string(), made.account.clone());
                Answer::AccountMade {
                    account: made.account,
                    password: crate::protocol::Secret::new(made.password),
                    secure_token: made.secure_token,
                }
            }
            Err(why) => Answer::Refused(why),
        }
    }

    /// Serves the app on `channel` until it says goodbye or the channel closes. However it ends
    /// (goodbye, the app gone, a broken channel), the broker is closed: nothing outlives the
    /// session.
    pub fn serve(&mut self, channel: &mut (impl Read + Write)) -> Result<(), FrameError> {
        let served = self.serve_requests(channel);
        if let Some(broker) = self.broker.as_mut() {
            broker.close();
        }
        served
    }

    fn serve_requests(&mut self, channel: &mut (impl Read + Write)) -> Result<(), FrameError> {
        loop {
            let request = match read_request(channel) {
                Ok(r) => r,
                // The app went away: the move is over.
                Err(FrameError::Io(e)) if e.kind() == std::io::ErrorKind::UnexpectedEof => {
                    return Ok(());
                }
                Err(e) => return Err(e),
            };
            match request {
                Request::Hello { session } => {
                    // Compared whole; the token is random and only ever said to this helper.
                    if session != self.token || self.opened {
                        send_answer(channel, &Answer::Refused(Refusal::WrongSession))?;
                        return Ok(());
                    }
                    self.opened = true;
                    send_answer(channel, &Answer::Ready)?;
                }
                _ if !self.opened => {
                    send_answer(channel, &Answer::Refused(Refusal::NoSession))?;
                    return Ok(());
                }
                // Only ever the first message to a launch daemon, never inside a session.
                Request::Open { .. } => {
                    send_answer(channel, &Answer::Refused(Refusal::WrongSession))?;
                    return Ok(());
                }
                Request::ScanPerson { account } => self.scan(channel, &account)?,
                Request::SystemOnly { paths } => {
                    let answer = self.system_only(&paths);
                    send_answer(channel, &answer)?;
                }
                Request::ReadBlocks {
                    handle,
                    offset,
                    len,
                } => {
                    let read = usize::try_from(handle)
                        .ok()
                        .and_then(|i| self.plan.get_mut(i))
                        .ok_or(Refusal::UnknownHandle)
                        .and_then(|(entry, pinned)| read_range(entry, pinned, offset, len));
                    match read {
                        Ok(bytes) => send_blocks(channel, &bytes)?,
                        Err(why) => send_answer(channel, &Answer::Refused(why))?,
                    }
                }
                Request::CreateAccount { name, full_name } => {
                    let answer = self.create(&name, &full_name);
                    send_answer(channel, &answer)?;
                }
                Request::HoldFor { account } => {
                    let answer = self.hold_for(&account);
                    send_answer(channel, &answer)?;
                }
                Request::HandOverNow { account } => {
                    let answer = self.hand_over_now(&account);
                    send_answer(channel, &answer)?;
                }
                Request::HandOverAtSignIn { account } => {
                    let answer = self.hand_over_at_sign_in(&account);
                    send_answer(channel, &answer)?;
                }
                request @ (Request::SealPlan { .. }
                | Request::Act(_)
                | Request::Revoke
                | Request::StagePackage { .. }
                | Request::StageBytes { .. }) => {
                    let answer = self.broker_answer(request);
                    send_answer(channel, &answer)?;
                }
                Request::WifiProfiles { names } => {
                    // Once, and only for a helper started for it; only the networks named, each
                    // plain text, never more than the bound.
                    let answer = match self.wifi.take() {
                        Some(_) if names.len() > crate::protocol::MAX_WIFI_PROFILES => {
                            Answer::Refused(Refusal::TooMuch)
                        }
                        Some(_) if !names.iter().all(|n| crate::wifi::name_ok(n)) => {
                            Answer::Refused(Refusal::NotPlain)
                        }
                        Some(read) => match read(&names) {
                            Ok(profiles) => Answer::WifiProfiles { profiles },
                            Err(why) => Answer::Refused(why),
                        },
                        None => Answer::Refused(Refusal::NotApproved),
                    };
                    send_answer(channel, &answer)?;
                }
                Request::Bye => {
                    send_answer(channel, &Answer::Goodbye)?;
                    return Ok(());
                }
            }
        }
    }

    /// Which of `paths` (the mover's own places it could not read) this helper's sweep found only
    /// the administrators and the system may open: yes or no for each, in order. Before any
    /// scan, every answer is no; too many places, or one too long, is refused.
    fn system_only(&mut self, paths: &[String]) -> Answer {
        if paths.len() > crate::MAX_SYSTEM_ASK
            || paths.iter().any(|p| p.len() > crate::MAX_SYSTEM_PATH)
            || self.asked.saturating_add(paths.len()) > crate::MAX_SYSTEM_ASKED
        {
            return Answer::Refused(Refusal::TooMuch);
        }
        self.asked += paths.len();
        let same = |a: &std::path::Path, b: &std::path::Path| {
            let text = |p: &std::path::Path| {
                let t = p.to_string_lossy();
                let t = t.trim_end_matches(['/', '\\']).to_string();
                if cfg!(windows) { t.to_lowercase() } else { t }
            };
            text(a) == text(b)
        };
        Answer::SystemOnly {
            answers: paths
                .iter()
                .map(|p| {
                    let p = std::path::Path::new(p);
                    self.system_only.iter().any(|s| same(s, p))
                })
                .collect(),
        }
    }

    fn scan(&mut self, channel: &mut impl Write, account: &str) -> Result<(), FrameError> {
        if !self.approved.contains(account) {
            return send_answer(channel, &Answer::Refused(Refusal::NotApproved));
        }
        let (found, privacy_blocked) = match (self.scanner)(account) {
            Ok(scan) => {
                // Every scan's places are kept together, each once: with several people, what one
                // scan found is never lost to the next.
                self.system_only.extend(scan.system_only);
                self.system_only.sort();
                self.system_only.dedup();
                (scan.found, scan.privacy_blocked)
            }
            Err(why) => return send_answer(channel, &Answer::Refused(why)),
        };
        if self.plan.len() + found.len() > MAX_HANDLES {
            return send_answer(channel, &Answer::Refused(Refusal::TooManyFiles));
        }
        let first = self.plan.len() as u64;
        let bytes = found
            .iter()
            .fold(0u64, |t, (_, e)| t.saturating_add(e.size));
        let handles: Vec<Handle> = found
            .iter()
            .enumerate()
            .map(|(n, (_, e))| Handle {
                handle: first + n as u64,
                size: e.size,
            })
            .collect();
        send_answer(
            channel,
            &Answer::Scanned {
                files: handles.len() as u64,
                bytes,
                privacy_blocked,
            },
        )?;
        send_handles(channel, &handles)?;
        // Then the items themselves, in the handles' order, a page at a time: they travel in the
        // move record, never shown to the mover (decided 21 October 2026).
        let items: Vec<&Item> = found.iter().map(|(i, _)| i).collect();
        for page in items.chunks(ITEMS_PER_FRAME) {
            send_items(channel, page)?;
        }
        self.plan.extend(found.into_iter().map(|(_, e)| (e, None)));
        Ok(())
    }
}
