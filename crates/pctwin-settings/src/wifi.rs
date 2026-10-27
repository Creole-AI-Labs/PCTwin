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
//! - **Mac and Linux:** reading and adding are not built yet. Every system tool found takes the
//!   password on its command line (`networksetup`, `nmcli`), where other programs on the laptop
//!   can read it; a native route is needed first. Until then these networks are a guided step.

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
    let mut list: Vec<String> = aces[1..aces.len() - 1]
        .split(")(")
        .map(str::to_ascii_uppercase)
        .collect();
    list.sort();
    let mut want = vec![
        format!("A;OICI;FA;;;{}", me.to_ascii_uppercase()),
        "A;OICI;FA;;;SY".to_string(),
    ];
    want.sort();
    list == want
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
