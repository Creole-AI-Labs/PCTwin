//! Saved Wi-Fi networks in the full move (Task List 2.4, decided 20 October 2026; move protocol
//! 8, `pctwin_transfer::WIFI_SINCE`).
//!
//! - **The old laptop's choose page** lists the networks saved there, by name only (read
//!   without any prompt), each ticked by default when the person moving is one of the laptop's
//!   administrators, and none ticked, with "ask whoever looks after this laptop", when not
//!   (`pctwin_settings::wifi::offer`). Networks set up with an organisation (802.1X) and kinds
//!   PCTwin does not move are listed but cannot be ticked.
//! - **Reading** happens once the person has chosen, and only for the networks they ticked: the
//!   system's own prompt where it needs one (Windows: the administrator helper, one prompt, or
//!   none more when it is already running for this move; a Mac: macOS's own prompt for each
//!   network's keychain item; Linux: NetworkManager's own prompt).
//! - **The extras** carry [`WifiListed`]: each network's name, kind, whether it was ticked and
//!   whether its password was read. Never a password; the new laptop's plan shows them.
//! - **The record** ([`WifiHeld::record`]) is sent only after the new laptop's person approved
//!   the plan, inside the sealed link, holding only the ticked networks that were read: every
//!   buffer wiped when dropped, never logged, never in a report, a log or the AI.
//! - **The new laptop** reads the record strictly against the listed networks
//!   ([`decode_record`]) and adds each through the system's own tool ([`add_networks`]): every
//!   network ends copied (the system lists it afterwards), needs-action with why, or
//!   skipped-by-user. A network already saved there is left as it is (never overwritten). On a
//!   Mac the networks come as one configuration profile the person installs in System Settings.

use serde::{Deserialize, Serialize};
use zeroize::Zeroizing;

use pctwin_settings::wifi::{Network, Secret, Security};
use pctwin_types::{ItemResult, Reason};

/// The most networks listed or carried.
pub const MAX_NETWORKS: usize = pctwin_types::limits::WIFI_PROFILES;
/// The record format this release writes and reads.
pub const RECORD_FORMAT: u8 = 1;
/// The longest network name (a profile's name, in bytes).
pub const MAX_NAME: usize = 255;
/// The longest network ID (SSID, in bytes, as Wi-Fi allows).
pub const MAX_SSID: usize = 32;

/// How a saved network is protected, as the person is told.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum WifiKind {
    /// No password.
    Open,
    /// A shared password (WPA, WPA2, WPA3 personal).
    Password,
    /// Set up with an organisation (802.1X: certificates or accounts): a step with them.
    Organisation,
    /// A kind PCTwin does not move.
    Other,
}

impl WifiKind {
    pub fn of(s: &Security) -> Self {
        match s {
            Security::Open => WifiKind::Open,
            Security::Personal { auth, .. } if PASSWORD_AUTH.contains(&auth.as_str()) => {
                WifiKind::Password
            }
            Security::Personal { .. } | Security::Other(_) => WifiKind::Other,
            Security::Enterprise => WifiKind::Organisation,
        }
    }

    /// Whether PCTwin moves this kind (the person may tick it).
    pub fn movable(self) -> bool {
        matches!(self, WifiKind::Open | WifiKind::Password)
    }
}

/// The shared-password kinds moved, in the names Windows' profiles use (and the other systems'
/// readers translate into).
pub const PASSWORD_AUTH: [&str; 3] = ["WPAPSK", "WPA2PSK", "WPA3SAE"];

/// One saved network as the old laptop lists it beside its other extras. Never its password.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct WifiListed {
    /// The network's name (its SSID), shown exactly as it is.
    pub name: String,
    pub kind: WifiKind,
    /// The old laptop's person ticked it.
    pub ticked: bool,
    /// Its password was read (an open network needs none): it is in the record.
    pub read: bool,
}

fn name_ok(s: &str, most: usize) -> bool {
    !s.is_empty() && s.len() <= most && !s.chars().any(char::is_control)
}

/// Whether a list from the other laptop holds together: bounded, every name plain and once,
/// only movable networks ticked, only ticked ones read.
pub fn listed_ok(list: &[WifiListed]) -> bool {
    let mut names = std::collections::HashSet::new();
    list.len() <= MAX_NETWORKS
        && list.iter().all(|l| {
            name_ok(&l.name, MAX_SSID)
                && names.insert(l.name.as_str())
                && (!l.ticked || l.kind.movable())
                && (!l.read || l.ticked)
        })
}

/// The networks found on the old laptop, as its choose page offers them: each by name and kind,
/// in the order found, each name once, within [`MAX_NETWORKS`]. `networks` are read without
/// passwords.
pub fn offered(networks: &[Network]) -> Vec<(String, WifiKind)> {
    let mut seen = std::collections::HashSet::new();
    networks
        .iter()
        .filter(|n| name_ok(&n.ssid, MAX_SSID) && seen.insert(n.ssid.clone()))
        .map(|n| (n.ssid.clone(), WifiKind::of(&n.security)))
        .take(MAX_NETWORKS)
        .collect()
}

/// The old laptop, once its person chose: the networks offered, the ones ticked, and those read
/// with their passwords. Sent as [`WifiHeld::record`] only once the plan is approved.
pub struct WifiHeld {
    listed: Vec<WifiListed>,
    /// Each ticked network that was read, by its place in `listed`.
    read: Vec<(u16, Network)>,
}

impl std::fmt::Debug for WifiHeld {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // Names and counts only: never a password.
        f.debug_struct("WifiHeld")
            .field("listed", &self.listed)
            .field("read", &self.read.len())
            .finish()
    }
}

impl WifiHeld {
    /// From the networks `offered` on the choose page, the places the person `ticked` there
    /// (anything not offered, or not movable, counts for nothing), and the networks `read` with
    /// their passwords (any others read are dropped, wiped, here).
    pub fn new(offered: &[(String, WifiKind)], ticked: &[u32], read: Vec<Network>) -> Self {
        let mut listed: Vec<WifiListed> = offered
            .iter()
            .take(MAX_NETWORKS)
            .enumerate()
            .map(|(at, (name, kind))| WifiListed {
                name: name.clone(),
                kind: *kind,
                ticked: kind.movable() && ticked.contains(&(at as u32)),
                read: false,
            })
            .collect();
        let mut kept = Vec::new();
        for n in read {
            let Some(at) = listed
                .iter()
                .position(|l| l.ticked && !l.read && l.name == n.ssid)
            else {
                continue;
            };
            let kind = WifiKind::of(&n.security);
            // The kind of password and cipher as short plain words (what the record carries).
            let plain = |w: &str| {
                (1..=32).contains(&w.len()) && w.bytes().all(|b| b.is_ascii_alphanumeric())
            };
            let usable = kind == listed[at].kind
                && match (&n.security, kind) {
                    (_, WifiKind::Open) => true,
                    (Security::Personal { auth, encryption }, WifiKind::Password) => {
                        n.secret.as_ref().is_some_and(secret_ok) && plain(auth) && plain(encryption)
                    }
                    _ => false,
                }
                && name_ok(&n.name, MAX_NAME);
            if usable {
                listed[at].read = true;
                kept.push((at as u16, n));
            }
        }
        // An open network needs no password: ticked, it is read once found again here.
        Self { listed, read: kept }
    }

    /// What the extras say (never a password).
    pub fn listed(&self) -> &[WifiListed] {
        &self.listed
    }

    /// The record sent once the plan is approved: only the ticked networks that were read.
    ///
    /// Its form: the format byte, the count (u16), then for each network its place in the list
    /// (u16), its profile name and SSID (u16 length, bytes), its flags (1: hidden, 2: joins by
    /// itself), its kind of password and cipher (u8 length, ASCII) and its password (u8 length,
    /// bytes; 0 for none). Big endian. Built in one buffer of exactly its size, wiped when
    /// dropped.
    pub fn record(&self) -> Zeroizing<Vec<u8>> {
        let parts: Vec<(u16, &Network, &str, &str, &str)> = self
            .read
            .iter()
            .map(|(at, n)| {
                let (auth, enc) = match &n.security {
                    Security::Personal { auth, encryption } => (auth.as_str(), encryption.as_str()),
                    _ => ("open", "none"),
                };
                let pw = n.secret.as_ref().map(Secret::for_session).unwrap_or("");
                (*at, n, auth, enc, pw)
            })
            .collect();
        let size = 3 + parts
            .iter()
            .map(|(_, n, a, e, p)| {
                2 + 2 + n.name.len() + 2 + n.ssid.len() + 1 + 3 + a.len() + e.len() + p.len()
            })
            .sum::<usize>();
        let mut out = Zeroizing::new(Vec::with_capacity(size));
        out.push(RECORD_FORMAT);
        out.extend_from_slice(&(parts.len() as u16).to_be_bytes());
        for (at, n, auth, enc, pw) in parts {
            out.extend_from_slice(&at.to_be_bytes());
            for s in [&n.name, &n.ssid] {
                out.extend_from_slice(&(s.len() as u16).to_be_bytes());
                out.extend_from_slice(s.as_bytes());
            }
            out.push(u8::from(n.hidden) | (u8::from(n.auto_connect) << 1));
            for s in [auth, enc, pw] {
                out.push(s.len() as u8);
                out.extend_from_slice(s.as_bytes());
            }
        }
        debug_assert_eq!(out.len(), size);
        out
    }
}

impl pctwin_move::wifi::WifiOut for WifiHeld {
    fn record(&self) -> Zeroizing<Vec<u8>> {
        WifiHeld::record(self)
    }
}

/// A password as Wi-Fi allows a shared one: 8 to 63 characters, or 64 hex digits; no control
/// character.
fn secret_ok(s: &Secret) -> bool {
    let p = s.for_session();
    let n = p.chars().count();
    !p.chars().any(char::is_control)
        && ((8..=63).contains(&n) || (p.len() == 64 && p.bytes().all(|b| b.is_ascii_hexdigit())))
}

/// Why the old laptop's Wi-Fi record was refused (all of it: none of its networks is added).
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum RecordError {
    #[error("the old laptop's Wi-Fi networks could not be read")]
    Unreadable,
    #[error("the old laptop sent a Wi-Fi network its person did not tick")]
    NotTicked,
}

/// Reads `bytes` from the other laptop, strictly, against what its extras `listed`: the format
/// this release knows; every network one that was listed as ticked and read, once, under the
/// same name, of the same kind; every text within its limits; nothing left over. Each network
/// comes back with its place in `listed`.
pub fn decode_record(
    bytes: &[u8],
    listed: &[WifiListed],
) -> Result<Vec<(usize, Network)>, RecordError> {
    use RecordError::{NotTicked, Unreadable};
    struct Reader<'a>(&'a [u8]);
    impl<'a> Reader<'a> {
        fn take(&mut self, n: usize) -> Result<&'a [u8], RecordError> {
            if self.0.len() < n {
                return Err(Unreadable);
            }
            let (a, b) = self.0.split_at(n);
            self.0 = b;
            Ok(a)
        }
        fn u8(&mut self) -> Result<u8, RecordError> {
            Ok(self.take(1)?[0])
        }
        fn u16(&mut self) -> Result<u16, RecordError> {
            let b = self.take(2)?;
            Ok(u16::from_be_bytes([b[0], b[1]]))
        }
        fn text16(&mut self, most: usize) -> Result<String, RecordError> {
            let n = self.u16()? as usize;
            let s = std::str::from_utf8(self.take(n)?).map_err(|_| Unreadable)?;
            name_ok(s, most).then(|| s.to_string()).ok_or(Unreadable)
        }
        fn ascii8(&mut self) -> Result<&'a str, RecordError> {
            let n = self.u8()? as usize;
            let b = self.take(n)?;
            b.iter()
                .all(u8::is_ascii_alphanumeric)
                .then(|| std::str::from_utf8(b).map_err(|_| Unreadable))
                .ok_or(Unreadable)?
        }
    }
    if bytes.is_empty() {
        return Ok(Vec::new());
    }
    let mut r = Reader(bytes);
    if r.u8()? != RECORD_FORMAT {
        return Err(Unreadable);
    }
    let count = r.u16()? as usize;
    if count > listed.len() {
        return Err(Unreadable);
    }
    let mut seen = std::collections::HashSet::new();
    let mut out = Vec::with_capacity(count);
    for _ in 0..count {
        let at = r.u16()? as usize;
        let l = listed.get(at).ok_or(NotTicked)?;
        if !l.ticked || !l.read || !seen.insert(at) {
            return Err(NotTicked);
        }
        let name = r.text16(MAX_NAME)?;
        let ssid = r.text16(MAX_SSID)?;
        let flags = r.u8()?;
        if flags & !3 != 0 || ssid != l.name {
            return Err(Unreadable);
        }
        let auth = r.ascii8()?.to_string();
        let encryption = r.ascii8()?.to_string();
        let n = r.u8()? as usize;
        let pw = r.take(n)?;
        let secret = match n {
            0 => None,
            _ => {
                let text = std::str::from_utf8(pw).map_err(|_| Unreadable)?;
                Some(Secret::new(Zeroizing::new(text.to_string())))
            }
        };
        let security = match (l.kind, auth.as_str(), &secret) {
            (WifiKind::Open, "open", None) if encryption == "none" => Security::Open,
            (WifiKind::Password, a, Some(s)) if PASSWORD_AUTH.contains(&a) && secret_ok(s) => {
                Security::Personal { auth, encryption }
            }
            _ => return Err(Unreadable),
        };
        out.push((
            at,
            Network {
                name,
                ssid,
                hidden: flags & 1 != 0,
                auto_connect: flags & 2 != 0,
                security,
                secret,
            },
        ));
    }
    if !r.0.is_empty() {
        return Err(Unreadable);
    }
    Ok(out)
}

/// Why a network needs the person, said on its line.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum WifiWhy {
    /// Set up with an organisation: join it again with their details.
    Organisation,
    /// A kind PCTwin does not move: join it again here.
    NotThisKind,
    /// Its password was not read on the old laptop (the system did not give it, or an
    /// administrator did not approve): type it once when joining.
    NoPassword,
    /// It did not arrive (the connection was lost, or the old laptop's record was refused).
    DidNotArrive,
    /// A network of that name was already saved here: left as it is.
    AlreadyHere,
    /// The system did not add it, or did not list it afterwards.
    NotAdded,
    /// A Mac: install the profile "Wi-Fi networks from your old laptop" in System Settings.
    InstallProfile,
}

/// How one network ended.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WifiDone {
    pub name: String,
    pub result: ItemResult,
    pub why: Option<WifiWhy>,
}

/// Why the system did not add a network.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NotAdded {
    /// It did not do it, or did not list it afterwards.
    Failed,
    /// A Mac: the person did not install the profile (yet).
    InstallProfile,
}

/// What is saved here could not be seen (nothing is then added: it could be added over).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NotSeen;

/// This laptop's saved Wi-Fi networks, through the system's own tools.
pub trait WifiHere {
    /// The networks saved here now, by name (SSID).
    fn saved(&mut self) -> Result<Vec<String>, NotSeen>;
    /// Adds `nets`, one answer each, in order. Each `Ok` only once the system lists it.
    fn add(&mut self, nets: &[&Network]) -> Vec<Result<(), NotAdded>>;
}

fn needs(why: WifiWhy) -> (ItemResult, Option<WifiWhy>) {
    (ItemResult::because(Reason::FinishByHand), Some(why))
}

/// The new laptop: adds the networks that `arrived` (`None`: the record never came, or was
/// refused), as the old laptop `listed` them, through `here`; one result for every listed
/// network, in the listed order. A network already saved here is never added over.
pub fn add_networks(
    listed: &[WifiListed],
    arrived: Option<&[(usize, Network)]>,
    here: &mut dyn WifiHere,
) -> Vec<WifiDone> {
    let mut results: Vec<Option<(ItemResult, Option<WifiWhy>)>> = listed
        .iter()
        .map(|l| match (l.kind, l.ticked, l.read) {
            (WifiKind::Organisation, ..) => Some(needs(WifiWhy::Organisation)),
            (WifiKind::Other, ..) => Some(needs(WifiWhy::NotThisKind)),
            (_, false, _) => Some((ItemResult::skipped_by_user(), None)),
            (_, true, false) => Some(needs(WifiWhy::NoPassword)),
            (_, true, true) => None,
        })
        .collect();
    let arrived = arrived.unwrap_or(&[]);
    let mut to_add: Vec<(usize, &Network)> = Vec::new();
    match here.saved() {
        Ok(saved) => {
            for (at, n) in arrived {
                if results.get(*at).is_none_or(Option::is_some) {
                    continue;
                }
                if saved.contains(&n.ssid) {
                    results[*at] = Some(needs(WifiWhy::AlreadyHere));
                } else {
                    to_add.push((*at, n));
                }
            }
        }
        // What is here cannot be seen: nothing is added (it could be added over).
        Err(NotSeen) => {
            for (at, _) in arrived {
                if results.get(*at).is_some_and(Option::is_none) {
                    results[*at] = Some(needs(WifiWhy::NotAdded));
                }
            }
        }
    }
    if !to_add.is_empty() {
        let nets: Vec<&Network> = to_add.iter().map(|(_, n)| *n).collect();
        let added = here.add(&nets);
        for (i, (at, _)) in to_add.iter().enumerate() {
            results[*at] = Some(match added.get(i) {
                Some(Ok(())) => (
                    ItemResult::done(pctwin_types::ItemStatus::Copied)
                        .unwrap_or_else(|_| ItemResult::because(Reason::NotVerified)),
                    None,
                ),
                Some(Err(NotAdded::InstallProfile)) => needs(WifiWhy::InstallProfile),
                Some(Err(NotAdded::Failed)) | None => needs(WifiWhy::NotAdded),
            });
        }
    }
    listed
        .iter()
        .zip(results)
        .map(|(l, r)| {
            let (result, why) = r.unwrap_or_else(|| needs(WifiWhy::DidNotArrive));
            WifiDone {
                name: l.name.clone(),
                result,
                why,
            }
        })
        .collect()
}

/* ---------- This laptop, through the system's own tools ---------- */

use pctwin_settings::runner::SystemRunner;
use pctwin_settings::wifi::PrivateFolders;

/// The networks saved on this laptop, read without their passwords (no prompt on any system).
/// None when they cannot be read.
pub fn found_here(folders: &dyn PrivateFolders) -> Vec<Network> {
    let mut runner = SystemRunner;
    #[cfg(windows)]
    let found =
        pctwin_settings::wifi::export_all(&mut runner, folders, &std::env::temp_dir(), false);
    #[cfg(target_os = "macos")]
    let found = {
        let _ = folders;
        pctwin_settings::wifi::export_mac(&mut runner, false)
    };
    #[cfg(all(unix, not(target_os = "macos")))]
    let found = {
        let _ = folders;
        pctwin_settings::wifi::export_nm(&mut runner, false)
    };
    found.unwrap_or_default()
}

/// A Mac or Linux: the networks named `ticked` read with their passwords, through the system's
/// own prompt (macOS's for each network's keychain item; NetworkManager's). No other network's
/// password is asked for. Windows reads through the administrator helper instead
/// ([`parse_profiles`]).
pub fn read_ticked_here(ticked: &[String]) -> Vec<Network> {
    let keep = |n: &str| ticked.iter().any(|t| t == n);
    let mut runner = SystemRunner;
    #[cfg(target_os = "macos")]
    let read = pctwin_settings::wifi::export_mac_where(&mut runner, true, &keep);
    // NetworkManager names each saved connection apart from its network's name: the ticked
    // networks' connections are found first (no secret), then only theirs are read.
    #[cfg(all(unix, not(target_os = "macos")))]
    let read = {
        let names: Vec<String> = pctwin_settings::wifi::export_nm(&mut runner, false)
            .unwrap_or_default()
            .into_iter()
            .filter(|n| keep(&n.ssid))
            .map(|n| n.name)
            .collect();
        pctwin_settings::wifi::export_nm_where(&mut runner, true, &|name| {
            names.iter().any(|n| n == name)
        })
        .map(|nets| nets.into_iter().filter(|n| keep(&n.ssid)).collect())
    };
    #[cfg(windows)]
    let read: Result<Vec<Network>, pctwin_settings::wifi::WifiError> = {
        let _ = (&mut runner, &keep);
        Ok(Vec::new())
    };
    read.unwrap_or_default()
}

/// Windows: the profiles the administrator helper read, parsed; only the networks named
/// `ticked` are kept (every other profile, password and all, is wiped as it is dropped here).
pub fn parse_profiles(profiles: Vec<Zeroizing<String>>, ticked: &[String]) -> Vec<Network> {
    profiles
        .iter()
        .filter_map(|p| pctwin_settings::wifi::parse_profile(p).ok())
        .filter(|n| ticked.contains(&n.ssid))
        .collect()
}

/// A Mac: how long PCTwin waits for the person to install the profile. macOS removes a profile
/// that was not installed after about 8 minutes, so the wait ends a little before.
pub const MAC_PROFILE_WAIT: std::time::Duration = std::time::Duration::from_secs(7 * 60);
/// A Mac: how often PCTwin looks whether the networks are there yet.
pub const MAC_LOOK_EVERY: std::time::Duration = std::time::Duration::from_secs(5);

/// This laptop's saved networks through the system's own tools (Windows `netsh`, Linux
/// `nmcli`, a Mac's configuration profile).
pub struct SystemWifi<'a> {
    pub folders: &'a dyn PrivateFolders,
    /// The person's own temporary folder (where the owner-only folders go).
    pub parent: std::path::PathBuf,
    /// The person stopped the move: a Mac stops waiting for the profile.
    pub stopped: &'a dyn Fn() -> bool,
    /// A Mac: told the minutes left while waiting for the person to install the profile.
    pub waiting: &'a mut dyn FnMut(u64),
}

impl WifiHere for SystemWifi<'_> {
    fn saved(&mut self) -> Result<Vec<String>, NotSeen> {
        let mut runner = SystemRunner;
        #[cfg(windows)]
        let saved =
            pctwin_settings::wifi::export_all(&mut runner, self.folders, &self.parent, false)
                .map(|nets| nets.into_iter().map(|n| n.ssid).collect());
        #[cfg(target_os = "macos")]
        let saved = pctwin_settings::wifi::mac_saved(&mut runner);
        #[cfg(all(unix, not(target_os = "macos")))]
        let saved = pctwin_settings::wifi::export_nm(&mut runner, false)
            .map(|nets| nets.into_iter().map(|n| n.ssid).collect());
        saved.map_err(|_| NotSeen)
    }

    fn add(&mut self, nets: &[&Network]) -> Vec<Result<(), NotAdded>> {
        let mut runner = SystemRunner;
        #[cfg(windows)]
        {
            nets.iter()
                .map(|n| {
                    pctwin_settings::wifi::import(&mut runner, self.folders, &self.parent, n)
                        .map_err(|_| NotAdded::Failed)
                })
                .collect()
        }
        #[cfg(all(unix, not(target_os = "macos")))]
        {
            nets.iter()
                .map(|n| {
                    pctwin_settings::wifi::import_nm(&mut runner, n).map_err(|_| NotAdded::Failed)
                })
                .collect()
        }
        #[cfg(target_os = "macos")]
        {
            use pctwin_settings::wifi::{added_mac, offer_mac, open_profiles_page};
            let Ok(offered) = offer_mac(&mut runner, self.folders, &self.parent, nets) else {
                return nets.iter().map(|_| Err(NotAdded::Failed)).collect();
            };
            let _ = open_profiles_page(&mut runner);
            let until = std::time::Instant::now() + MAC_PROFILE_WAIT;
            let mut now = vec![false; nets.len()];
            loop {
                if let Ok(there) = added_mac(&mut runner, &offered) {
                    now = there;
                }
                let left = until.saturating_duration_since(std::time::Instant::now());
                if now.iter().all(|x| *x) || left.is_zero() || (self.stopped)() {
                    break;
                }
                (self.waiting)(left.as_secs().div_ceil(60));
                std::thread::sleep(MAC_LOOK_EVERY.min(left));
            }
            // The profile file goes when PCTwin stops waiting.
            drop(offered);
            now.into_iter()
                .map(|x| {
                    if x {
                        Ok(())
                    } else {
                        Err(NotAdded::InstallProfile)
                    }
                })
                .collect()
        }
    }
}
