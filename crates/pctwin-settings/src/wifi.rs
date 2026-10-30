//! Saved Wi-Fi networks with their passwords (Task List 2.4; decided 20 October 2026).
//!
//! - **Offered:** ticked by default when the person moving is the laptop's administrator (one
//!   system prompt when they are read); left off, with "ask whoever looks after this laptop",
//!   when they are not ([`offer`]).
//! - **Read on Windows** with the system's own tool, `netsh wlan export profile key=clear`, which
//!   needs the administrator and writes each profile, password in plain text, to a file. The
//!   files go into a fresh folder in the person's own temporary folder (only they, the system and
//!   administrators can open it); each file is read into memory and deleted at once; the folder
//!   is removed when reading ends, also on failure; a folder a crash left behind is removed at the
//!   next start ([`sweep_leftovers`]).
//! - **Passwords** live only in [`Secret`]: wiped from memory when dropped, never printed
//!   (`Debug` shows nothing of it), never serialised except by [`Secret::to_session`] for the
//!   move's encrypted session. Never in reports, logs or AI.
//! - **Enterprise networks** (802.1X: certificates, accounts) are a separate step with the
//!   organisation; their profile moves without any secret ([`Security::Enterprise`]).
//! - **Applied on Windows** with `netsh wlan add profile user=current`, from a file in a fresh
//!   owner-only folder, deleted at once; then read back (the profile is there) before it counts.
//! - **Linux** (NetworkManager, 28 October 2026): read with `nmcli --show-secrets`, the password
//!   only on its standard output; added with `nmcli connection add` without the password, which
//!   then goes to `nmcli`'s own editor on its standard input ([`export_nm`], [`import_nm`]).
//!   NetworkManager's own permission check (polkit) decides who may see or add each one.
//! - **Mac** (30 October 2026): read with the system's own tools: the Wi-Fi device and its saved
//!   networks from `networksetup`, and each network's password from its item in the System
//!   keychain with `security find-generic-password -w`, which macOS itself guards with its own
//!   administrator prompt (PCTwin never sees the administrator's password and never bypasses the
//!   prompt; macOS asks once for each network, since each item has its own access list). The
//!   password comes back only on the tool's standard output, into wiped memory. A network with no
//!   keychain item may be open or an organisation's: it is not moved (never guessed).
//!   Added on a new Mac as a configuration profile (`com.apple.wifi.managed`, Apple's own
//!   format for saved networks), written into a fresh owner-only folder and handed to macOS with
//!   `open`: macOS shows it in System Settings, where the person installs it with the
//!   administrator's password (one approval for every network). The password is never on a
//!   command line (`networksetup -addpreferredwirelessnetworkatindex` would put it there, where
//!   other programs on the laptop can read it). The profile file is removed when PCTwin stops
//!   waiting ([`MacProfile`] dropped), and a folder a crash left behind at the next start
//!   ([`sweep_leftovers`]). A network counts as added only once `networksetup` lists it.
//!   Removing the profile in System Settings removes its networks again.

use std::path::{Path, PathBuf};

use zeroize::Zeroizing;

use crate::runner::{Runner, Tool};

/// A Wi-Fi password. Wiped when dropped; never printed.
pub struct Secret(Zeroizing<String>);

impl Secret {
    pub fn new(s: Zeroizing<String>) -> Self {
        Self(s)
    }

    /// The bytes sent over the move's encrypted session (and nowhere else).
    pub fn to_session(&self) -> Zeroizing<Vec<u8>> {
        Zeroizing::new(self.0.as_bytes().to_vec())
    }

    /// Back from the session.
    pub fn from_session(bytes: Zeroizing<Vec<u8>>) -> Option<Self> {
        let s = std::str::from_utf8(&bytes).ok()?;
        (s.len() <= 64 && !s.contains('\0')).then(|| Self(Zeroizing::new(s.to_string())))
    }

    fn expose(&self) -> &str {
        &self.0
    }

    /// The password itself, only for the move's sealed record (the full move's Wi-Fi record,
    /// built in a buffer wiped when dropped) and nowhere else.
    pub fn for_session(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Debug for Secret {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Secret(hidden)")
    }
}

/// How a network is protected.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Security {
    Open,
    /// A shared password (`WPA2PSK`, `WPA3SAE`, ...), with the cipher.
    Personal {
        auth: String,
        encryption: String,
    },
    /// 802.1X: certificates or accounts. A separate step with the organisation (T24-015).
    Enterprise,
    /// A kind PCTwin does not move.
    Other(String),
}

/// One saved network.
#[derive(Debug)]
pub struct Network {
    pub name: String,
    pub ssid: String,
    pub hidden: bool,
    pub auto_connect: bool,
    pub security: Security,
    pub secret: Option<Secret>,
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum WifiError {
    #[error("the saved network could not be read")]
    Unreadable,
    #[error("this kind of network is set up with your organisation")]
    Enterprise,
    #[error("the system's Wi-Fi tool did not do it")]
    Tool,
    #[error("the network was not there afterwards")]
    NotThere,
    #[error("this network's name cannot be used safely")]
    BadName,
}

/// Whether Wi-Fi networks are offered ticked.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Offer {
    pub ticked: bool,
    /// Shown when not ticked.
    pub note: Option<&'static str>,
}

pub const ASK_ADMIN: &str = "ask whoever looks after this laptop";

/// The 20 October 2026 decision.
pub fn offer(mover_is_admin: bool) -> Offer {
    if mover_is_admin {
        Offer {
            ticked: true,
            note: None,
        }
    } else {
        Offer {
            ticked: false,
            note: Some(ASK_ADMIN),
        }
    }
}

/* ---------- Whether the person moving is the laptop's administrator ---------- */

/// The administrators group of every Windows laptop, by its SID (the same in every language).
pub const ADMINISTRATORS_SID: &str = "S-1-5-32-544";
/// The administrators group of every Mac, by its number.
pub const MAC_ADMIN_GROUP: &str = "80";
/// The groups a Linux system's own prompt (polkit) asks a member of for administrator rights:
/// `sudo` (Debian, Ubuntu), `wheel` (Fedora, Arch and most others), `admin` (older Ubuntu).
pub const LINUX_ADMIN_GROUPS: [&str; 3] = ["sudo", "wheel", "admin"];

/// The fields of one line of `whoami /fo csv`: each in double quotes, a quote inside doubled.
fn csv_fields(line: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut field = String::new();
    let mut quoted = false;
    let mut chars = line.trim_end_matches('\r').chars().peekable();
    while let Some(c) = chars.next() {
        match (c, quoted) {
            ('"', true) if chars.peek() == Some(&'"') => {
                field.push('"');
                chars.next();
            }
            ('"', _) => quoted = !quoted,
            (',', false) => out.push(std::mem::take(&mut field)),
            (c, _) => field.push(c),
        }
    }
    out.push(field);
    out
}

/// Whether `whoami /groups /fo csv /nh` lists the administrators group (its third field, the
/// SID): the person is one of this laptop's administrators, whether or not this program runs
/// with those rights now (while it does not, Windows lists the group "for deny only").
pub fn admin_in_whoami(csv: &str) -> bool {
    csv.lines().any(|l| {
        csv_fields(l)
            .get(2)
            .is_some_and(|sid| sid == ADMINISTRATORS_SID)
    })
}

/// Whether `id -G` (a Mac: group numbers) or `id -Gn` (Linux: group names) lists an
/// administrators group of `system`.
pub fn admin_in_groups(groups: &str, mac: bool) -> bool {
    groups.split_whitespace().any(|g| {
        if mac {
            g == MAC_ADMIN_GROUP
        } else {
            LINUX_ADMIN_GROUPS.contains(&g)
        }
    })
}

/// Whether the person running PCTwin is one of this laptop's administrators, as the system's
/// own tools say (Windows `whoami`, Mac and Linux `id`). Not known: not one (the Wi-Fi networks
/// are then offered unticked, with "ask whoever looks after this laptop").
pub fn mover_is_admin(runner: &mut dyn Runner) -> bool {
    let out = if cfg!(windows) {
        runner.run(Tool::Whoami, &["/groups", "/fo", "csv", "/nh"])
    } else if cfg!(target_os = "macos") {
        runner.run(Tool::Id, &["-G"])
    } else {
        runner.run(Tool::Id, &["-Gn"])
    };
    match out {
        Ok(o) if o.ok && cfg!(windows) => admin_in_whoami(&o.stdout),
        Ok(o) if o.ok => admin_in_groups(&o.stdout, cfg!(target_os = "macos")),
        _ => false,
    }
}

const NS: &str = "http://www.microsoft.com/networking/WLAN/profile/v1";
use pctwin_types::limits::{WIFI_PROFILE_BYTES as MAX_PROFILE, WIFI_PROFILES as MAX_PROFILES};

fn child<'a, 'i>(n: roxmltree::Node<'a, 'i>, name: &str) -> Option<roxmltree::Node<'a, 'i>> {
    n.children()
        .find(|c| c.is_element() && c.tag_name().name() == name)
}

fn text_of(n: Option<roxmltree::Node<'_, '_>>) -> Option<String> {
    n.and_then(|n| n.text()).map(str::trim).map(str::to_string)
}

/// Reads one Windows WLAN profile (as `netsh` exports it).
pub fn parse_profile(xml: &str) -> Result<Network, WifiError> {
    pctwin_types::nesting::check(xml, pctwin_types::nesting::Syntax::Xml)
        .map_err(|_| WifiError::Unreadable)?;
    let doc = roxmltree::Document::parse(xml).map_err(|_| WifiError::Unreadable)?;
    let root = doc.root_element();
    if root.tag_name().name() != "WLANProfile" || root.tag_name().namespace() != Some(NS) {
        return Err(WifiError::Unreadable);
    }
    let name = text_of(child(root, "name")).ok_or(WifiError::Unreadable)?;
    let ssid_node = child(root, "SSIDConfig").and_then(|c| child(c, "SSID"));
    let ssid = ssid_node
        .and_then(|s| text_of(child(s, "name")))
        .ok_or(WifiError::Unreadable)?;
    let hidden = child(root, "SSIDConfig")
        .and_then(|c| text_of(child(c, "nonBroadcast")))
        .is_some_and(|t| t == "true");
    let auto_connect = text_of(child(root, "connectionMode")).is_none_or(|m| m == "auto");
    let security = child(root, "MSM")
        .and_then(|m| child(m, "security"))
        .ok_or(WifiError::Unreadable)?;
    let ae = child(security, "authEncryption").ok_or(WifiError::Unreadable)?;
    let auth = text_of(child(ae, "authentication")).ok_or(WifiError::Unreadable)?;
    let encryption = text_of(child(ae, "encryption")).unwrap_or_default();
    let one_x = text_of(child(ae, "useOneX")).is_some_and(|t| t == "true");
    let enterprise = one_x
        || matches!(
            auth.as_str(),
            "WPA" | "WPA2" | "WPA3" | "WPA3ENT" | "WPA3ENT192"
        );
    let kind = if enterprise {
        Security::Enterprise
    } else if auth == "open" && (encryption == "none" || encryption.is_empty()) {
        Security::Open
    } else if matches!(auth.as_str(), "WPAPSK" | "WPA2PSK" | "WPA3SAE" | "OWE") {
        Security::Personal {
            auth: auth.clone(),
            encryption,
        }
    } else {
        Security::Other(auth)
    };
    let secret = match (&kind, child(security, "sharedKey")) {
        (Security::Personal { .. }, Some(sk)) => {
            let protected = text_of(child(sk, "protected")).is_some_and(|t| t == "true");
            let material = child(sk, "keyMaterial").and_then(|k| k.text());
            match (protected, material) {
                (false, Some(m)) if !m.is_empty() && m.len() <= 64 => {
                    Some(Secret(Zeroizing::new(m.to_string())))
                }
                _ => None,
            }
        }
        _ => None,
    };
    Ok(Network {
        name,
        ssid,
        hidden,
        auto_connect,
        security: kind,
        secret,
    })
}

fn escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&apos;"),
            c => out.push(c),
        }
    }
    out
}

/// The profile to add on the new laptop. Only open and password networks; the password is
/// written only into the returned buffer, which is wiped when dropped.
pub fn profile_xml(n: &Network) -> Result<Zeroizing<String>, WifiError> {
    let bad = |s: &str| s.is_empty() || s.len() > 255 || s.chars().any(char::is_control);
    if bad(&n.name) || bad(&n.ssid) || n.ssid.len() > 32 {
        return Err(WifiError::BadName);
    }
    let (auth, enc, key) = match &n.security {
        Security::Open => ("open".to_string(), "none".to_string(), None),
        Security::Personal { auth, encryption } => {
            let s = n.secret.as_ref().ok_or(WifiError::Unreadable)?;
            (auth.clone(), encryption.clone(), Some(s))
        }
        Security::Enterprise => return Err(WifiError::Enterprise),
        Security::Other(_) => return Err(WifiError::Unreadable),
    };
    let ok = |s: &str| s.bytes().all(|b| b.is_ascii_alphanumeric());
    if !ok(&auth) || !ok(&enc) {
        return Err(WifiError::Unreadable);
    }
    let mut x = Zeroizing::new(String::with_capacity(1024));
    x.push_str("<?xml version=\"1.0\"?>\n");
    x.push_str(&format!("<WLANProfile xmlns=\"{NS}\">"));
    x.push_str(&format!("<name>{}</name>", escape(&n.name)));
    x.push_str(&format!(
        "<SSIDConfig><SSID><name>{}</name></SSID>{}</SSIDConfig>",
        escape(&n.ssid),
        if n.hidden {
            "<nonBroadcast>true</nonBroadcast>"
        } else {
            ""
        }
    ));
    x.push_str("<connectionType>ESS</connectionType>");
    x.push_str(&format!(
        "<connectionMode>{}</connectionMode>",
        if n.auto_connect { "auto" } else { "manual" }
    ));
    x.push_str(&format!(
        "<MSM><security><authEncryption><authentication>{auth}</authentication><encryption>{enc}</encryption><useOneX>false</useOneX></authEncryption>"
    ));
    if let Some(s) = key {
        x.push_str(
            "<sharedKey><keyType>passPhrase</keyType><protected>false</protected><keyMaterial>",
        );
        x.push_str(&escape(s.expose()));
        x.push_str("</keyMaterial></sharedKey>");
    }
    x.push_str("</security></MSM></WLANProfile>\n");
    Ok(x)
}

/// The prefix of PCTwin's Wi-Fi folders in the temporary folder.
pub const FOLDER_PREFIX: &str = "pctwin-wifi-";

/// Why a private folder could not be made.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FolderError {
    /// Something already has that name: never used, another name is tried.
    NameTaken,
    Failed,
}

/// Makes the folder the Wi-Fi tool writes the password into (in plain text): new, and only
/// this account (and the system) may open it, from its first moment, nothing inherited from the
/// folder above (fourth review, finding F3). Handed in by the app, which makes it through the
/// helper's fenced system calls (`pctwin_helper_os::private_folder`); this crate never links
/// those (the same pattern as the maker check and the journal).
pub trait PrivateFolders {
    fn make(&self, path: &Path) -> Result<(), FolderError>;
    /// The folder's access list as Windows writes it (`D:P(A;OICI;FA;;;S-1-...)`), read back.
    fn access_list(&self, path: &Path) -> Option<String>;
    /// The person's own account as the access list names it (their SID on Windows), from the
    /// system, never from the access list itself (`pctwin_helper_os::current_account`).
    fn me(&self) -> Option<String>;
}

/// Private folders through the standard library: mode 0700 from the first moment (Mac and
/// Linux, where that is the whole of a folder's access).
#[cfg(unix)]
pub struct UnixPrivateFolders;

#[cfg(unix)]
impl PrivateFolders for UnixPrivateFolders {
    fn make(&self, path: &Path) -> Result<(), FolderError> {
        use std::os::unix::fs::DirBuilderExt;
        std::fs::DirBuilder::new()
            .mode(0o700)
            .create(path)
            .map_err(|e| {
                if e.kind() == std::io::ErrorKind::AlreadyExists {
                    FolderError::NameTaken
                } else {
                    FolderError::Failed
                }
            })
    }
    fn access_list(&self, _: &Path) -> Option<String> {
        None
    }
    fn me(&self) -> Option<String> {
        None
    }
}

/// A fresh folder for the Wi-Fi tool, inside `parent` (the person's own temporary folder).
fn fresh_folder(folders: &dyn PrivateFolders, parent: &Path) -> Result<TempFolder, WifiError> {
    for _ in 0..8 {
        let mut b = [0u8; 12];
        getrandom::fill(&mut b).map_err(|_| WifiError::Tool)?;
        let name: String = b.iter().map(|x| format!("{x:02x}")).collect();
        let path = parent.join(format!("{FOLDER_PREFIX}{name}"));
        // Owner-only and protected from its first moment (the password is written into it in
        // plain text): never a folder someone else prepared, never one that inherits its
        // parent's access. If that cannot be done, nothing goes on.
        match folders.make(&path) {
            Ok(()) => {
                let folder = TempFolder(path);
                check_private(folders, &folder.0)?;
                return Ok(folder);
            }
            Err(FolderError::NameTaken) => continue,
            Err(FolderError::Failed) => return Err(WifiError::Tool),
        }
    }
    Err(WifiError::Tool)
}

/// Reads the folder's access back: protected, nothing inherited, only this account and the
/// system (Windows); only this account (elsewhere).
fn check_private(folders: &dyn PrivateFolders, path: &Path) -> Result<(), WifiError> {
    #[cfg(windows)]
    {
        let sddl = folders.access_list(path).ok_or(WifiError::Tool)?;
        let me = folders.me().ok_or(WifiError::Tool)?;
        if !private_sddl(&sddl, &me) {
            return Err(WifiError::Tool);
        }
    }
    #[cfg(not(windows))]
    let _ = folders;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mode = std::fs::symlink_metadata(path)
            .map_err(|_| WifiError::Tool)?
            .permissions()
            .mode();
        if mode & 0o077 != 0 {
            return Err(WifiError::Tool);
        }
    }
    Ok(())
}

/// Whether an access list (its text form) is exactly what the private folder is made with
/// (`pctwin_helper_os::private_folder`): protected, nothing inherited, and full access for the
/// person `me` and the system, nobody else (fifth review, finding 8: a list for Everyone, all
/// users or any other one account is never private).
pub fn private_sddl(sddl: &str, me: &str) -> bool {
    let Some(dacl) = sddl.strip_prefix("D:") else {
        return false;
    };
    let Some(aces) = dacl.strip_prefix('P') else {
        return false;
    };
    // `AI` (inheritance computed) is the only other flag Windows writes back.
    let aces = aces.strip_prefix("AI").unwrap_or(aces);
    if !aces.starts_with('(') || !aces.ends_with(')') || me.is_empty() {
        return false;
    }
    let me = me.to_ascii_uppercase();
    let mut list: Vec<String> = Vec::new();
    for ace in aces[1..aces.len() - 1].split(")(") {
        let ace = ace.to_ascii_uppercase();
        // The trustee is the last field; Windows may write it as its short name.
        let (head, who) = ace.rsplit_once(';').unwrap_or(("", ace.as_str()));
        list.push(format!("{head};{}", written_as(who, &me)));
    }
    list.sort();
    let mut want = vec![format!("A;OICI;FA;;;{me}"), "A;OICI;FA;;;SY".to_string()];
    want.sort();
    list == want
}

/// The account an access list entry names, with the short names Windows writes for single local
/// accounts turned back into the person's own SID when, and only when, that is who they are.
///
/// Windows writes a few well-known accounts by a two-letter name instead of their SID: `LA` is
/// this computer's built-in Administrator (its local account with number 500), `LG` its built-in
/// Guest (501). So when the person IS that account (GitHub's Windows runners run as it, and some
/// people use it), the folder made for `S-1-5-21-...-500` reads back naming `LA`. Every other
/// short name (`BA`, `BU`, `WD`, `AU`, ...) is a group or many people and never the person.
///
/// The person's SID says the number (500 or 501) but not whether its domain is this computer's;
/// for a network (domain) account with number 500, `LA` would be this computer's own
/// Administrator instead, an administrator of this computer, who can already open every folder
/// on it. So reading `LA` as the person never lets the Wi-Fi password reach anyone who could not
/// already read it.
fn written_as<'a>(who: &'a str, me: &'a str) -> &'a str {
    let local_account = me.starts_with("S-1-5-21-");
    match who {
        "LA" if local_account && me.ends_with("-500") => me,
        "LG" if local_account && me.ends_with("-501") => me,
        other => other,
    }
}

/// A folder removed with everything in it when dropped (also on failure).
struct TempFolder(PathBuf);

impl Drop for TempFolder {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

/// Removes Wi-Fi folders a crash left in `parent` (called at start).
pub fn sweep_leftovers(parent: &Path) -> usize {
    let Ok(entries) = std::fs::read_dir(parent) else {
        return 0;
    };
    let mut n = 0;
    for e in entries.flatten() {
        let name = e.file_name();
        let is_ours = name.to_string_lossy().starts_with(FOLDER_PREFIX);
        let is_dir = e.file_type().is_ok_and(|t| t.is_dir() && !t.is_symlink());
        if is_ours && is_dir && std::fs::remove_dir_all(e.path()).is_ok() {
            n += 1;
        }
    }
    n
}

/// A path as one `netsh` argument value: quoted, never holding a quote.
fn netsh_quoted(p: &Path) -> Result<String, WifiError> {
    let s = p.to_str().ok_or(WifiError::BadName)?;
    if s.contains('"') || s.chars().any(char::is_control) {
        return Err(WifiError::BadName);
    }
    Ok(format!("\"{s}\""))
}

/// Every saved network with its password (Windows): exported into a fresh folder in `parent`,
/// each file read into memory and deleted at once.
pub fn export_all(
    runner: &mut dyn Runner,
    folders: &dyn PrivateFolders,
    parent: &Path,
    with_keys: bool,
) -> Result<Vec<Network>, WifiError> {
    let folder = fresh_folder(folders, parent)?;
    let arg = format!("folder={}", netsh_quoted(&folder.0)?);
    let mut args = vec!["wlan", "export", "profile"];
    if with_keys {
        args.push("key=clear");
    }
    args.push(&arg);
    let out = runner
        .run(Tool::Netsh, &args)
        .map_err(|_| WifiError::Tool)?;
    let mut nets = Vec::new();
    let entries = std::fs::read_dir(&folder.0).map_err(|_| WifiError::Unreadable)?;
    for e in entries.flatten().take(MAX_PROFILES) {
        let path = e.path();
        let meta = std::fs::symlink_metadata(&path).map_err(|_| WifiError::Unreadable)?;
        let bytes = if meta.is_file() && meta.len() <= MAX_PROFILE {
            Zeroizing::new(std::fs::read(&path).unwrap_or_default())
        } else {
            Zeroizing::new(Vec::new())
        };
        // Gone from the disk before anything else happens to it.
        let _ = std::fs::remove_file(&path);
        let Ok(text) = std::str::from_utf8(&bytes) else {
            continue;
        };
        if let Ok(n) = parse_profile(text.trim_start_matches('\u{feff}')) {
            nets.push(n);
        }
    }
    drop(folder);
    if !out.ok && nets.is_empty() {
        return Err(WifiError::Tool);
    }
    Ok(nets)
}

/// Adds one network for the person (Windows), then checks it is there.
pub fn import(
    runner: &mut dyn Runner,
    folders: &dyn PrivateFolders,
    parent: &Path,
    n: &Network,
) -> Result<(), WifiError> {
    let xml = profile_xml(n)?;
    {
        let folder = fresh_folder(folders, parent)?;
        let file = folder.0.join("profile.xml");
        {
            use std::io::Write;
            let mut f = std::fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&file)
                .map_err(|_| WifiError::Tool)?;
            f.write_all(xml.as_bytes()).map_err(|_| WifiError::Tool)?;
        }
        let arg = format!("filename={}", netsh_quoted(&file)?);
        let out = runner.run(
            Tool::Netsh,
            &["wlan", "add", "profile", &arg, "user=current"],
        );
        let _ = std::fs::remove_file(&file);
        drop(folder);
        if !out.map_err(|_| WifiError::Tool)?.ok {
            return Err(WifiError::Tool);
        }
    }
    drop(xml);
    // Read back, without any password.
    let now = export_all(runner, folders, parent, false)?;
    if now.iter().any(|x| x.name == n.name && x.ssid == n.ssid) {
        Ok(())
    } else {
        Err(WifiError::NotThere)
    }
}

/* ---------- Linux: NetworkManager ---------- */

/// One field of a profile as `nmcli --terse --escape no` prints it in its "field:value" lines.
fn nm_field<'a>(text: &'a str, field: &str) -> Option<&'a str> {
    text.lines()
        .find_map(|l| l.strip_prefix(field)?.strip_prefix(':'))
}

/// A name or network name that can go into an `nmcli` line safely: no line breaks, no NUL, not
/// read as one of its options, and no spaces at its ends (its editor trims them).
fn nm_plain(s: &str) -> bool {
    !s.is_empty()
        && s.len() <= 255
        && !s.starts_with('-')
        && s.trim() == s
        && !s.contains(['\n', '\r', '\0'])
}

/// The NetworkManager key management of a profile, in the names Windows' profiles use, so a
/// network read on one system is added on another.
fn nm_security(key_mgmt: &str) -> Security {
    match key_mgmt {
        "" => Security::Open,
        "wpa-psk" => Security::Personal {
            auth: "WPA2PSK".into(),
            encryption: "AES".into(),
        },
        "sae" => Security::Personal {
            auth: "WPA3SAE".into(),
            encryption: "AES".into(),
        },
        "wpa-eap" | "wpa-eap-suite-b-192" | "ieee8021x" => Security::Enterprise,
        other => Security::Other(other.into()),
    }
}

/// Every saved Wi-Fi network NetworkManager has (Linux), with its password when `with_keys`.
/// The password comes only on `nmcli`'s standard output (`--show-secrets`), held in memory that
/// is wiped; it is never on a command line. NetworkManager itself decides whether the person may
/// see each password (its system prompt, polkit); one it will not show moves without it.
pub fn export_nm(runner: &mut dyn Runner, with_keys: bool) -> Result<Vec<Network>, WifiError> {
    export_nm_where(runner, with_keys, &|_| true)
}

/// [`export_nm`] for only the networks whose names `keep` says yes to: no other network's
/// password is asked for.
pub fn export_nm_where(
    runner: &mut dyn Runner,
    with_keys: bool,
    keep: &dyn Fn(&str) -> bool,
) -> Result<Vec<Network>, WifiError> {
    let list = runner
        .run(
            Tool::Nmcli,
            &[
                "--terse",
                "--escape",
                "no",
                "--fields",
                "TYPE,NAME",
                "connection",
                "show",
            ],
        )
        .map_err(|_| WifiError::Tool)?;
    if !list.ok {
        return Err(WifiError::Tool);
    }
    let mut nets = Vec::new();
    for line in list.stdout.lines().take(MAX_PROFILES) {
        let Some((kind, name)) = line.split_once(':') else {
            continue;
        };
        if kind != "802-11-wireless" || !nm_plain(name) || !keep(name) {
            continue;
        }
        let mut fields = vec![
            "802-11-wireless.ssid",
            "802-11-wireless.hidden",
            "connection.autoconnect",
            "802-11-wireless-security.key-mgmt",
        ];
        if with_keys {
            fields.push("802-11-wireless-security.psk");
        }
        let fields = fields.join(",");
        let mut args = vec!["--terse", "--escape", "no", "--fields", &fields];
        if with_keys {
            args.push("--show-secrets");
        }
        args.extend(["connection", "show", "id", name]);
        let Ok(out) = runner.run_secret(Tool::Nmcli, &args, None) else {
            continue;
        };
        if !out.ok {
            continue;
        }
        let text: &str = &out.stdout;
        let Some(ssid) = nm_field(text, "802-11-wireless.ssid").filter(|s| !s.is_empty()) else {
            continue;
        };
        let security =
            nm_security(nm_field(text, "802-11-wireless-security.key-mgmt").unwrap_or(""));
        let secret = match &security {
            Security::Personal { .. } if with_keys => {
                nm_field(text, "802-11-wireless-security.psk")
                    .filter(|p| !p.is_empty() && p.len() <= 64)
                    .map(|p| Secret::new(Zeroizing::new(p.to_string())))
            }
            _ => None,
        };
        nets.push(Network {
            name: name.to_string(),
            ssid: ssid.to_string(),
            hidden: nm_field(text, "802-11-wireless.hidden") == Some("yes"),
            auto_connect: nm_field(text, "connection.autoconnect") != Some("no"),
            security,
            secret,
        });
    }
    Ok(nets)
}

/// Adds one network to NetworkManager (Linux), then checks it is there. The profile is made
/// without its password (`nmcli connection add`), and the password is then given to `nmcli`'s
/// own editor on its standard input, never on a command line (the route Ansible's nmcli module
/// takes for the same reason). A profile whose password could not be set is removed again, so
/// nothing half made is left. An open network needs no password; an enterprise one is a step
/// with the organisation.
pub fn import_nm(runner: &mut dyn Runner, n: &Network) -> Result<(), WifiError> {
    if !nm_plain(&n.name) || !nm_plain(&n.ssid) {
        return Err(WifiError::BadName);
    }
    let key_mgmt = match &n.security {
        Security::Open => None,
        Security::Personal { auth, .. } => match auth.as_str() {
            "WPA2PSK" | "WPAPSK" => Some("wpa-psk"),
            "WPA3SAE" => Some("sae"),
            _ => return Err(WifiError::Tool),
        },
        Security::Enterprise => return Err(WifiError::Enterprise),
        Security::Other(_) => return Err(WifiError::Tool),
    };
    let secret = match (key_mgmt, &n.secret) {
        (None, _) => None,
        (Some(_), Some(s)) if nm_plain(s.expose()) => Some(s),
        (Some(_), _) => return Err(WifiError::Unreadable),
    };
    let yes = |b: bool| if b { "yes" } else { "no" };
    let mut args = vec![
        "connection",
        "add",
        "type",
        "wifi",
        "con-name",
        &n.name,
        "ssid",
        &n.ssid,
        "802-11-wireless.hidden",
        yes(n.hidden),
        // Off while the password is set: nmcli's editor asks before saving a profile that would
        // connect at once, and nothing would answer it. Turned on afterwards, as it was.
        "connection.autoconnect",
        "no",
    ];
    if let Some(k) = key_mgmt {
        args.extend(["802-11-wireless-security.key-mgmt", k]);
    }
    let added = runner
        .run(Tool::Nmcli, &args)
        .map_err(|_| WifiError::Tool)?;
    if !added.ok {
        return Err(WifiError::Tool);
    }
    if let Some(s) = secret {
        let script = Zeroizing::new(format!(
            "set 802-11-wireless-security.psk {}\nsave persistent\nquit\n",
            s.expose()
        ));
        let set = runner.run_secret(
            Tool::Nmcli,
            &["connection", "edit", "id", &n.name],
            Some(script.as_bytes()),
        );
        if !set.is_ok_and(|o| o.ok) {
            let _ = runner.run(Tool::Nmcli, &["connection", "delete", "id", &n.name]);
            return Err(WifiError::Tool);
        }
    }
    if n.auto_connect {
        let on = runner
            .run(
                Tool::Nmcli,
                &[
                    "connection",
                    "modify",
                    "id",
                    &n.name,
                    "connection.autoconnect",
                    "yes",
                ],
            )
            .map_err(|_| WifiError::Tool)?;
        if !on.ok {
            let _ = runner.run(Tool::Nmcli, &["connection", "delete", "id", &n.name]);
            return Err(WifiError::Tool);
        }
    }
    // Read back: the network is there, and NetworkManager holds its very password.
    let now = export_nm(runner, secret.is_some())?;
    let kept = |x: &Network| match (secret, &x.secret) {
        (None, _) => true,
        (Some(want), Some(got)) => want.expose() == got.expose(),
        (Some(_), None) => false,
    };
    if now
        .iter()
        .any(|x| x.name == n.name && x.ssid == n.ssid && kept(x))
    {
        Ok(())
    } else {
        Err(WifiError::NotThere)
    }
}

/* ---------- Mac: networksetup, the System keychain, a configuration profile ---------- */

/// The System keychain, where macOS keeps saved Wi-Fi passwords.
pub const SYSTEM_KEYCHAIN: &str = "/Library/Keychains/System.keychain";
/// The kind macOS gives a saved Wi-Fi password's keychain item.
pub const AIRPORT_KIND: &str = "AirPort network password";

/// The Wi-Fi device (`en0`) in `networksetup -listallhardwareports`: the device after the
/// `Hardware Port: Wi-Fi` line (`AirPort` on older systems).
pub fn mac_wifi_device(ports: &str) -> Option<String> {
    let mut wifi = false;
    for l in ports.lines().map(str::trim) {
        if let Some(port) = l.strip_prefix("Hardware Port:") {
            wifi = matches!(port.trim(), "Wi-Fi" | "AirPort");
        } else if let Some(dev) = l.strip_prefix("Device:")
            && wifi
        {
            let dev = dev.trim();
            let plain = !dev.is_empty()
                && dev.len() <= 15
                && dev.bytes().all(|b| b.is_ascii_alphanumeric());
            return plain.then(|| dev.to_string());
        }
    }
    None
}

/// The saved networks in `networksetup -listpreferredwirelessnetworks <device>`: one per line
/// after its heading, each indented by one tab (a name's own spaces kept).
pub fn mac_preferred(list: &str) -> Vec<String> {
    list.lines()
        .filter_map(|l| l.strip_prefix('\t'))
        .map(|l| l.trim_end_matches('\r').to_string())
        .filter(|s| !s.is_empty() && s.len() <= 32 && !s.chars().any(char::is_control))
        .take(MAX_PROFILES)
        .collect()
}

fn mac_device(runner: &mut dyn Runner) -> Result<String, WifiError> {
    let ports = runner
        .run(Tool::Networksetup, &["-listallhardwareports"])
        .map_err(|_| WifiError::Tool)?;
    if !ports.ok {
        return Err(WifiError::Tool);
    }
    mac_wifi_device(&ports.stdout).ok_or(WifiError::Tool)
}

/// The networks saved on this Mac, by name.
pub fn mac_saved(runner: &mut dyn Runner) -> Result<Vec<String>, WifiError> {
    let dev = mac_device(runner)?;
    let list = runner
        .run(
            Tool::Networksetup,
            &["-listpreferredwirelessnetworks", &dev],
        )
        .map_err(|_| WifiError::Tool)?;
    if !list.ok {
        return Err(WifiError::Tool);
    }
    Ok(mac_preferred(&list.stdout))
}

/// Every saved Wi-Fi network on this Mac with a password, with the password when `with_keys`
/// (see the module notes): macOS asks for the administrator for each one; a network whose
/// password it does not give moves without it.
pub fn export_mac(runner: &mut dyn Runner, with_keys: bool) -> Result<Vec<Network>, WifiError> {
    export_mac_where(runner, with_keys, &|_| true)
}

/// [`export_mac`] for only the networks whose names `keep` says yes to: macOS asks for no other
/// network's password.
pub fn export_mac_where(
    runner: &mut dyn Runner,
    with_keys: bool,
    keep: &dyn Fn(&str) -> bool,
) -> Result<Vec<Network>, WifiError> {
    let mut nets = Vec::new();
    for ssid in mac_saved(runner)?.into_iter().filter(|s| keep(s)) {
        // The item's attributes only: no password, no prompt.
        let item = runner.run(
            Tool::Security,
            &[
                "find-generic-password",
                "-D",
                AIRPORT_KIND,
                "-a",
                &ssid,
                SYSTEM_KEYCHAIN,
            ],
        );
        if !item.is_ok_and(|o| o.ok) {
            // No password item: open, or an organisation's network. Never guessed.
            continue;
        }
        let secret = if with_keys {
            runner
                .run_secret(
                    Tool::Security,
                    &[
                        "find-generic-password",
                        "-D",
                        AIRPORT_KIND,
                        "-a",
                        &ssid,
                        "-w",
                        SYSTEM_KEYCHAIN,
                    ],
                    None,
                )
                .ok()
                .filter(|o| o.ok)
                .and_then(|o| {
                    let p = o.stdout.strip_suffix('\n').unwrap_or(&o.stdout);
                    (!p.is_empty() && p.len() <= 64 && !p.chars().any(char::is_control))
                        .then(|| Secret::new(Zeroizing::new(p.to_string())))
                })
        } else {
            None
        };
        nets.push(Network {
            name: ssid.clone(),
            ssid,
            hidden: false,
            auto_connect: true,
            // A Mac's item does not say WPA2 or WPA3; WPA2's name is the one every system reads
            // (the Mac's own profile below asks for any WPA personal network).
            security: Security::Personal {
                auth: "WPA2PSK".into(),
                encryption: "AES".into(),
            },
            secret,
        });
    }
    Ok(nets)
}

fn uuid(mut b: [u8; 16]) -> String {
    b[6] = (b[6] & 0x0f) | 0x40;
    b[8] = (b[8] & 0x3f) | 0x80;
    let h: String = b.iter().map(|x| format!("{x:02X}")).collect();
    format!(
        "{}-{}-{}-{}-{}",
        &h[0..8],
        &h[8..12],
        &h[12..16],
        &h[16..20],
        &h[20..32]
    )
}

/// The configuration profile that adds `nets` on a Mac (Apple's `com.apple.wifi.managed`
/// payload, one for each network), its identifiers from `random` (16 bytes for the profile and
/// 16 for each network). Only open and password networks; the passwords are written only into
/// the returned buffer, which is wiped when dropped.
pub fn mac_profile(nets: &[&Network], random: &[u8]) -> Result<Zeroizing<String>, WifiError> {
    if nets.is_empty() || nets.len() > MAX_PROFILES || random.len() < 16 * (nets.len() + 1) {
        return Err(WifiError::Tool);
    }
    let id = |i: usize| -> String {
        let mut b = [0u8; 16];
        b.copy_from_slice(&random[16 * i..16 * (i + 1)]);
        uuid(b)
    };
    let bad = |s: &str| s.is_empty() || s.len() > 32 || s.chars().any(char::is_control);
    let tf = |b: bool| if b { "<true/>" } else { "<false/>" };
    let mut x = Zeroizing::new(String::with_capacity(1024 * (nets.len() + 1)));
    x.push_str("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<!DOCTYPE plist PUBLIC \"-//Apple//DTD PLIST 1.0//EN\" \"http://www.apple.com/DTDs/PropertyList-1.0.dtd\">\n<plist version=\"1.0\"><dict><key>PayloadContent</key><array>");
    for (i, n) in nets.iter().enumerate() {
        if bad(&n.ssid) {
            return Err(WifiError::BadName);
        }
        let (kind, key) = match &n.security {
            Security::Open => ("None", None),
            Security::Personal { .. } => {
                let s = n.secret.as_ref().ok_or(WifiError::Unreadable)?;
                let p = s.expose();
                if p.is_empty() || p.len() > 64 || p.chars().any(char::is_control) {
                    return Err(WifiError::Unreadable);
                }
                ("WPA", Some(p))
            }
            Security::Enterprise => return Err(WifiError::Enterprise),
            Security::Other(_) => return Err(WifiError::Unreadable),
        };
        let uid = id(i + 1);
        x.push_str(&format!(
            "<dict><key>PayloadType</key><string>com.apple.wifi.managed</string><key>PayloadVersion</key><integer>1</integer><key>PayloadIdentifier</key><string>com.creoleailabs.pctwin.wifi.{uid}</string><key>PayloadUUID</key><string>{uid}</string><key>PayloadDisplayName</key><string>{name}</string><key>SSID_STR</key><string>{name}</string><key>HIDDEN_NETWORK</key>{hidden}<key>AutoJoin</key>{auto}<key>EncryptionType</key><string>{kind}</string>",
            name = escape(&n.ssid),
            hidden = tf(n.hidden),
            auto = tf(n.auto_connect),
        ));
        if let Some(p) = key {
            x.push_str("<key>Password</key><string>");
            x.push_str(&escape(p));
            x.push_str("</string>");
        }
        x.push_str("</dict>");
    }
    let uid = id(0);
    x.push_str(&format!(
        "</array><key>PayloadType</key><string>Configuration</string><key>PayloadVersion</key><integer>1</integer><key>PayloadIdentifier</key><string>com.creoleailabs.pctwin.wifi.{uid}</string><key>PayloadUUID</key><string>{uid}</string><key>PayloadDisplayName</key><string>Wi-Fi networks from your old laptop (PCTwin)</string><key>PayloadOrganization</key><string>PCTwin</string><key>PayloadScope</key><string>System</string><key>PayloadRemovalDisallowed</key><false/></dict></plist>\n"
    ));
    Ok(x)
}

/// A profile handed to macOS, waiting for the person to install it in System Settings. Its file
/// (and folder) is removed when this is dropped.
pub struct MacProfile {
    folder: TempFolder,
    /// The networks it adds, by name.
    pub ssids: Vec<String>,
}

impl MacProfile {
    /// The profile file.
    pub fn file(&self) -> PathBuf {
        self.folder.0.join("PCTwin Wi-Fi.mobileconfig")
    }
}

/// Writes the profile for `nets` into a fresh owner-only folder in `parent` and hands it to
/// macOS (`open`), which shows it in System Settings for the person to install (see the module
/// notes). Nothing is added until they do; [`added_mac`] says which are there.
pub fn offer_mac(
    runner: &mut dyn Runner,
    folders: &dyn PrivateFolders,
    parent: &Path,
    nets: &[&Network],
) -> Result<MacProfile, WifiError> {
    let mut random = vec![0u8; 16 * (nets.len() + 1)];
    getrandom::fill(&mut random).map_err(|_| WifiError::Tool)?;
    let profile = mac_profile(nets, &random)?;
    let offered = MacProfile {
        folder: fresh_folder(folders, parent)?,
        ssids: nets.iter().map(|n| n.ssid.clone()).collect(),
    };
    let file = offered.file();
    {
        use std::io::Write;
        let mut f = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&file)
            .map_err(|_| WifiError::Tool)?;
        f.write_all(profile.as_bytes())
            .map_err(|_| WifiError::Tool)?;
    }
    drop(profile);
    let path = file.to_str().ok_or(WifiError::BadName)?;
    let opened = runner
        .run(Tool::Open, &[path])
        .map_err(|_| WifiError::Tool)?;
    if !opened.ok {
        return Err(WifiError::Tool);
    }
    Ok(offered)
}

/// The System Settings page where the person installs a downloaded profile.
pub const PROFILES_PAGE: &str = "x-apple.systempreferences:com.apple.Profiles-Settings.extension";

/// Opens System Settings at its profiles page.
pub fn open_profiles_page(runner: &mut dyn Runner) -> Result<(), WifiError> {
    match runner.run(Tool::Open, &[PROFILES_PAGE]) {
        Ok(o) if o.ok => Ok(()),
        _ => Err(WifiError::Tool),
    }
}

/// Which of the offered networks this Mac now lists as saved (the person installed the
/// profile): one answer for each, in order.
pub fn added_mac(runner: &mut dyn Runner, offered: &MacProfile) -> Result<Vec<bool>, WifiError> {
    let now = mac_saved(runner)?;
    Ok(offered
        .ssids
        .iter()
        .map(|s| now.iter().any(|n| n == s))
        .collect())
}
