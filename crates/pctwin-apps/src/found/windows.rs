//! Windows: the lists behind "Installed apps", and each person's packaged apps.
//!
//! **Which entries.** Windows shows an entry of its Uninstall lists when it has a display name,
//! is not marked as a hidden component (`SystemComponent = 1`) and is not an update to another
//! entry (`ParentKeyName`, or a `ReleaseType` of an update or hotfix). PCTwin lists exactly
//! those, from the machine's 64-bit list, its 32-bit list, and each person's own list; the rule
//! is Windows' own, never a list of app names.
//!
//! **Who made it.** The maker is read from the old laptop's own copy by PCTwin's own signature
//! reader (`pctwin_maker::windows_old_copy`), never by running anything: first the app's own
//! program (the entry's icon, when it is a program outside Windows' own folder), then the
//! installer package Windows keeps for a Windows Installer product, then the app's own
//! uninstaller (outside Windows' folder: `msiexec` and `rundll32` are Microsoft's, not the
//! app's maker). The first copy that can be read decides. When none names a program, the
//! programs directly in the app's install folder are read, and they decide only when every
//! signed one has the same signer ([`folder_maker`]). A copy read and found unsigned is
//! [`AppMaker::Unsigned`]; none readable is [`AppMaker::NotRead`] with why.
//!
//! **Packaged apps** (MSIX/Appx, the Store's) are listed with their package family name, their
//! signing kind and their publisher as their package identity names it: Windows checks a
//! package's signature against that identity itself, so it is the maker.
//!
//! Other people's lists can be read only while their settings are open (they are signed in), and
//! their packaged apps only by an administrator: those are listed as not read, with why.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use pctwin_maker::{OldCopy, OldMaker};
use pctwin_record::apps::{MAX_APP_TEXT, MAX_APP_URL};
use pctwin_record::{
    AppArch, AppMaker, AppNote, AppSystem, FoundApp, FoundWhere, Owner, PackageSigning,
    RegistryView,
};
use serde::Deserialize;

use super::{app_arch, clean, signer_id};

/// One value of an Uninstall entry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RegValue {
    Text(String),
    Number(u32),
}

/// One entry of an Uninstall list, as read: its key name and its values (names in lower case).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RawEntry {
    pub view: RegistryView,
    pub key: String,
    pub values: BTreeMap<String, RegValue>,
}

impl RawEntry {
    fn text(&self, name: &str) -> Option<&str> {
        match self.values.get(name)? {
            RegValue::Text(t) => Some(t.as_str()).filter(|t| !t.trim().is_empty()),
            RegValue::Number(_) => None,
        }
    }

    fn number(&self, name: &str) -> Option<u32> {
        match self.values.get(name)? {
            RegValue::Number(n) => Some(*n),
            RegValue::Text(t) => t.trim().parse().ok(),
        }
    }

    /// Whether Windows shows it in "Installed apps".
    pub fn shown(&self) -> bool {
        self.text("displayname").is_some()
            && self.number("systemcomponent") != Some(1)
            && self.text("parentkeyname").is_none()
            && !matches!(
                self.text("releasetype")
                    .map(str::to_ascii_lowercase)
                    .as_deref(),
                Some("update" | "hotfix" | "security update" | "service pack")
            )
    }

    /// Whether it is a Windows Installer product.
    pub fn is_msi(&self) -> bool {
        self.number("windowsinstaller") == Some(1)
    }

    /// The Windows Installer product code: the key name, when it is a GUID and the entry is one.
    pub fn product_code(&self) -> Option<String> {
        (self.is_msi() && is_guid(&self.key)).then(|| self.key.to_ascii_uppercase())
    }
}

/// `{xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx}`.
pub fn is_guid(s: &str) -> bool {
    let b = s.as_bytes();
    b.len() == 38
        && b[0] == b'{'
        && b[37] == b'}'
        && b[1..37].iter().enumerate().all(|(i, c)| match i {
            8 | 13 | 18 | 23 => *c == b'-',
            _ => c.is_ascii_hexdigit(),
        })
}

/// Windows Installer's packed form of a product code, as its own registry keys use it.
pub fn packed_guid(guid: &str) -> Option<String> {
    if !is_guid(guid) {
        return None;
    }
    let hex: Vec<char> = guid
        .chars()
        .filter(char::is_ascii_hexdigit)
        .map(|c| c.to_ascii_uppercase())
        .collect();
    let mut out = String::with_capacity(32);
    out.extend(hex[0..8].iter().rev());
    out.extend(hex[8..12].iter().rev());
    out.extend(hex[12..16].iter().rev());
    for pair in hex[16..32].chunks(2) {
        out.push(pair[1]);
        out.push(pair[0]);
    }
    Some(out)
}

/// The program a value names: `"C:\App\app.exe",0`, `C:\App\app.exe /uninstall`. Only a full
/// path ending in `.exe`, never inside Windows' own folder.
pub fn program_in(value: &str, windows_folder: &Path) -> Option<PathBuf> {
    let v = value.trim();
    let path = if let Some(rest) = v.strip_prefix('"') {
        rest.split('"').next()?.to_string()
    } else {
        let lower = v.to_ascii_lowercase();
        let end = lower.find(".exe")? + 4;
        v[..end].to_string()
    };
    let path = path.trim();
    let lower = path.to_ascii_lowercase();
    // "app.exe,0": the icon index after a program named without quotes.
    if !lower.ends_with(".exe") {
        return None;
    }
    let p = PathBuf::from(path);
    if !p.is_absolute() || p.starts_with(windows_folder) {
        return None;
    }
    let win = windows_folder.to_string_lossy().to_ascii_lowercase();
    if !win.is_empty() && lower.starts_with(&win) {
        return None;
    }
    Some(p)
}

/// Where to read an entry's maker from, best first.
pub fn maker_copies(
    entry: &RawEntry,
    windows_folder: &Path,
    msi_cache: Option<PathBuf>,
) -> Vec<PathBuf> {
    let mut out = Vec::new();
    if let Some(p) = entry
        .text("displayicon")
        .and_then(|v| program_in(v, windows_folder))
    {
        out.push(p);
    }
    if let Some(p) = msi_cache {
        out.push(p);
    }
    for name in ["quietuninstallstring", "uninstallstring"] {
        if let Some(p) = entry.text(name).and_then(|v| program_in(v, windows_folder))
            && !out.contains(&p)
        {
            out.push(p);
        }
    }
    out
}

/// Most programs read from an install folder.
const MAX_FOLDER_PROGRAMS: usize = 32;

/// The maker of the programs directly in an install folder: decided only when every signed one
/// has the same signing certificate (a folder can hold a maker's own programs and a bundled
/// tool signed by someone else: then nothing is decided). `None` when no program could be read.
pub fn folder_maker(
    programs: &[PathBuf],
    read: &dyn Fn(&Path) -> OldCopy,
) -> Option<(AppMaker, AppArch)> {
    let mut signed: Option<(pctwin_maker::WindowsSigner, Option<pctwin_maker::Arch>)> = None;
    let mut any_unsigned = None;
    for p in programs.iter().take(MAX_FOLDER_PROGRAMS) {
        let copy = read(p);
        match copy.maker {
            OldMaker::Signed(s) => match &signed {
                Some((first, _)) if first.certificate != s.certificate => {
                    return Some((
                        AppMaker::NotRead {
                            why: "Its programs are signed by different makers.".into(),
                        },
                        AppArch::UNKNOWN,
                    ));
                }
                Some(_) => {}
                None => signed = Some((s, copy.arch)),
            },
            OldMaker::Unsigned => any_unsigned = any_unsigned.or(Some(copy.arch)),
            OldMaker::NotRead(_) => {}
        }
    }
    match (signed, any_unsigned) {
        (Some((s, arch)), _) => Some((AppMaker::Windows(Box::new(signer_id(&s))), app_arch(arch))),
        (None, Some(arch)) => Some((AppMaker::Unsigned, app_arch(arch))),
        (None, None) => None,
    }
}

/// The programs directly in `folder` (no links, no deeper folders), in name order.
pub fn programs_in_folder(folder: &Path, windows_folder: &Path) -> Vec<PathBuf> {
    if !folder.is_absolute() || folder.starts_with(windows_folder) {
        return vec![];
    }
    let Ok(rd) = std::fs::read_dir(folder) else {
        return vec![];
    };
    let mut out: Vec<PathBuf> = rd
        .flatten()
        .filter(|e| e.file_type().is_ok_and(|t| t.is_file()))
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|e| e.eq_ignore_ascii_case("exe")))
        .collect();
    out.sort();
    out.truncate(MAX_FOLDER_PROGRAMS);
    out
}

/// The maker and architecture from the first readable copy; then, when no copy is named or none
/// could be read, from the install folder's programs (`folder`).
pub fn maker_of(
    copies: &[PathBuf],
    folder: &[PathBuf],
    read: &dyn Fn(&Path) -> OldCopy,
) -> (AppMaker, AppArch) {
    let mut why = Vec::new();
    for path in copies {
        let copy = read(path);
        match copy.maker {
            OldMaker::Signed(s) => {
                return (
                    AppMaker::Windows(Box::new(signer_id(&s))),
                    app_arch(copy.arch),
                );
            }
            OldMaker::Unsigned => return (AppMaker::Unsigned, app_arch(copy.arch)),
            OldMaker::NotRead(w) => why.push(w),
        }
    }
    if let Some(found) = folder_maker(folder, read) {
        return found;
    }
    let why = if copies.is_empty() {
        "Windows' list does not say where the app's program is.".to_string()
    } else {
        format!(
            "Its program could not be read ({}).",
            why.first().map_or("", String::as_str)
        )
    };
    (
        AppMaker::NotRead {
            why: clean(&why, MAX_APP_TEXT).unwrap_or_default(),
        },
        AppArch::UNKNOWN,
    )
}

/// A web address the old laptop recorded, kept only when it is an ordinary web address.
pub fn web_address(s: &str) -> Option<String> {
    let s = s.trim();
    let lower = s.to_ascii_lowercase();
    if !(lower.starts_with("https://") || lower.starts_with("http://")) {
        return None;
    }
    let u = url::Url::parse(s).ok()?;
    if u.username() != "" || u.password().is_some() || u.host_str().is_none() {
        return None;
    }
    clean(u.as_str(), MAX_APP_URL).filter(|c| c.len() == u.as_str().len())
}

/// One shown entry as a found app.
pub fn app_of(
    entry: &RawEntry,
    owner: Owner,
    windows_folder: &Path,
    msi_cache: Option<PathBuf>,
    read: &dyn Fn(&Path) -> OldCopy,
) -> Option<FoundApp> {
    let name = clean(entry.text("displayname")?, MAX_APP_TEXT)?;
    let copies = maker_copies(entry, windows_folder, msi_cache);
    let folder = entry
        .text("installlocation")
        .map(|f| programs_in_folder(Path::new(f.trim().trim_matches('"')), windows_folder))
        .unwrap_or_default();
    let (maker, arch) = maker_of(&copies, &folder, read);
    let web = ["urlinfoabout", "urlupdateinfo", "helplink"]
        .iter()
        .find_map(|v| entry.text(v).and_then(web_address));
    let reg = |v| entry.text(v).and_then(|t| clean(t, MAX_APP_TEXT));
    let (o, c, p) = (reg("regowner"), reg("regcompany"), reg("productid"));
    let notes = if o.is_some() || c.is_some() || p.is_some() {
        vec![AppNote::Registration {
            owner: o,
            company: c,
            product_id: p,
        }]
    } else {
        vec![]
    };
    Some(FoundApp {
        name,
        version: entry
            .text("displayversion")
            .and_then(|v| clean(v, MAX_APP_TEXT)),
        publisher: entry.text("publisher").and_then(|v| clean(v, MAX_APP_TEXT)),
        owner,
        system: AppSystem::Windows,
        found: FoundWhere::WindowsEntry {
            view: entry.view,
            key: clean(&entry.key, MAX_APP_TEXT)?,
            product_code: entry.product_code(),
        },
        arch,
        maker,
        web,
        update_feed: None,
        installers: vec![],
        notes,
    })
}

/// The fixed script that lists the person's own packaged apps (not frameworks, not resource
/// packs), each field as text so Windows PowerShell 5.1 does not write numbers for its kinds.
pub const PACKAGES_SCRIPT: &str = "$ErrorActionPreference='Stop'; Get-AppxPackage | Where-Object { -not $_.IsFramework -and -not $_.IsResourcePackage } | ForEach-Object { [pscustomobject]@{ n=[string]$_.Name; p=[string]$_.Publisher; f=[string]$_.PackageFamilyName; s=[string]$_.SignatureKind; a=[string]$_.Architecture; v=[string]$_.Version } } | ConvertTo-Json -Compress";

#[derive(Deserialize)]
struct RawPackage {
    n: String,
    p: String,
    f: String,
    s: String,
    a: String,
    v: String,
}

/// Reads what [`PACKAGES_SCRIPT`] wrote (one object, or a list, or nothing at all).
pub fn packages_from(json: &str, owner: &Owner) -> Result<Vec<FoundApp>, String> {
    let json = json.trim();
    if json.is_empty() {
        return Ok(vec![]);
    }
    let raw: Vec<RawPackage> = if json.starts_with('[') {
        serde_json::from_str(json).map_err(|e| e.to_string())?
    } else {
        vec![serde_json::from_str(json).map_err(|e| e.to_string())?]
    };
    let mut out = Vec::new();
    for p in raw {
        let (Some(name), Some(family), Some(publisher)) = (
            clean(&p.n, MAX_APP_TEXT),
            clean(&p.f, MAX_APP_TEXT),
            clean(&p.p, MAX_APP_TEXT),
        ) else {
            continue;
        };
        let signing = match p.s.as_str() {
            "Store" => PackageSigning::Store,
            "Developer" => PackageSigning::Developer,
            "System" => PackageSigning::System,
            "Enterprise" => PackageSigning::Enterprise,
            _ => PackageSigning::None,
        };
        let arch = match p.a.as_str() {
            "X86" => app_arch(Some(pctwin_maker::Arch::X86)),
            "X64" => app_arch(Some(pctwin_maker::Arch::X64)),
            "Arm64" => app_arch(Some(pctwin_maker::Arch::Arm64)),
            // Neutral: runs on every processor.
            "Neutral" => AppArch {
                known: true,
                x86: true,
                x64: true,
                arm64: true,
            },
            _ => AppArch::UNKNOWN,
        };
        out.push(FoundApp {
            name,
            version: clean(&p.v, MAX_APP_TEXT),
            publisher: Some(publisher.clone()),
            owner: owner.clone(),
            system: AppSystem::Windows,
            found: FoundWhere::WindowsPackage {
                family_name: family,
                signing,
            },
            arch,
            maker: AppMaker::WindowsPackage { publisher },
            web: None,
            update_feed: None,
            installers: vec![],
            notes: vec![],
        });
    }
    Ok(out)
}

#[cfg(windows)]
pub use real::{machine_entries, read_windows_apps};

#[cfg(windows)]
mod real {
    use super::*;
    use crate::found::Collector;
    use pctwin_maker::run::{Call, Runner};
    use pctwin_record::AppPlace;
    use std::time::Duration;
    use winreg::RegKey;
    use winreg::enums::{
        HKEY_LOCAL_MACHINE, HKEY_USERS, KEY_READ, KEY_WOW64_32KEY, KEY_WOW64_64KEY,
    };

    const UNINSTALL: &str = r"SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall";
    /// Most entries read from one list.
    const MAX_ENTRIES: usize = 20_000;
    /// Most values read from one entry.
    const MAX_VALUES: usize = 256;

    fn entries(
        root: &RegKey,
        path: &str,
        flags: u32,
        view: RegistryView,
    ) -> Result<Vec<RawEntry>, String> {
        let list = root
            .open_subkey_with_flags(path, KEY_READ | flags)
            .map_err(|e| e.to_string())?;
        let mut out = Vec::new();
        for name in list.enum_keys().take(MAX_ENTRIES) {
            let Ok(name) = name else { continue };
            let Ok(key) = list.open_subkey_with_flags(&name, KEY_READ | flags) else {
                continue;
            };
            let mut values = BTreeMap::new();
            for v in key.enum_values().take(MAX_VALUES) {
                let Ok((vname, value)) = v else { continue };
                let value = match value.vtype {
                    winreg::enums::REG_DWORD => match u32::from_reg_value(&value) {
                        Ok(n) => RegValue::Number(n),
                        Err(_) => continue,
                    },
                    winreg::enums::REG_SZ | winreg::enums::REG_EXPAND_SZ => {
                        match String::from_reg_value(&value) {
                            Ok(t) => RegValue::Text(t),
                            Err(_) => continue,
                        }
                    }
                    _ => continue,
                };
                values.insert(vname.to_ascii_lowercase(), value);
            }
            out.push(RawEntry {
                view,
                key: name,
                values,
            });
        }
        Ok(out)
    }

    use winreg::types::FromRegValue;

    /// Every entry of Windows' list of apps installed for everyone, both views, as read now (what
    /// is really there after an install; 2.1 step 5). A list that cannot be read adds nothing.
    pub fn machine_entries() -> Vec<RawEntry> {
        let hklm = RegKey::predef(HKEY_LOCAL_MACHINE);
        let mut out = Vec::new();
        for (flags, view) in [
            (KEY_WOW64_64KEY, RegistryView::Machine),
            (KEY_WOW64_32KEY, RegistryView::Machine32),
        ] {
            if let Ok(list) = entries(&hklm, UNINSTALL, flags, view) {
                out.extend(list);
            }
        }
        out
    }

    /// The installer package Windows keeps for a Windows Installer product, for `sid` ("S-1-5-18"
    /// for products installed for everyone).
    fn msi_cache(code: &str, sid: &str) -> Option<PathBuf> {
        let packed = packed_guid(code)?;
        let path = format!(
            r"SOFTWARE\Microsoft\Windows\CurrentVersion\Installer\UserData\{sid}\Products\{packed}\InstallProperties"
        );
        let key = RegKey::predef(HKEY_LOCAL_MACHINE)
            .open_subkey_with_flags(path, KEY_READ | KEY_WOW64_64KEY)
            .ok()?;
        let p: String = key.get_value("LocalPackage").ok()?;
        let p = PathBuf::from(p);
        p.is_absolute().then_some(p)
    }

    fn windows_folder() -> PathBuf {
        std::env::var_os("SystemRoot")
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from(r"C:\Windows"))
    }

    fn add_list(
        c: &mut Collector,
        entries: Vec<RawEntry>,
        owner: &Owner,
        sid_for_msi: &str,
        win: &Path,
    ) {
        for e in entries.iter().filter(|e| e.shown()) {
            let cache = e
                .product_code()
                .and_then(|code| msi_cache(&code, sid_for_msi));
            if let Some(app) = app_of(
                e,
                owner.clone(),
                win,
                cache,
                &pctwin_maker::windows_old_copy,
            ) {
                c.add(app);
            }
        }
    }

    /// Every app Windows lists, for everyone and for each person in `people` (account IDs), and
    /// the packaged apps of `me`, the person PCTwin runs as. `runner` runs Windows PowerShell for
    /// the packaged apps.
    pub fn read_windows_apps(
        people: &[String],
        me: &str,
        runner: &dyn Runner,
    ) -> pctwin_record::AppList {
        let mut c = Collector::default();
        let win = windows_folder();
        let hklm = RegKey::predef(HKEY_LOCAL_MACHINE);
        for (flags, view) in [
            (KEY_WOW64_64KEY, RegistryView::Machine),
            (KEY_WOW64_32KEY, RegistryView::Machine32),
        ] {
            match entries(&hklm, UNINSTALL, flags, view) {
                Ok(list) => add_list(&mut c, list, &Owner::Shared, "S-1-5-18", &win),
                Err(e) => c.not_read(
                    Owner::Shared,
                    AppPlace::WindowsMachineList,
                    format!("Windows' list of apps for everyone could not be read ({e})."),
                ),
            }
        }
        let hku = RegKey::predef(HKEY_USERS);
        for sid in people {
            let owner = Owner::Person {
                account_id: sid.clone(),
            };
            if hku.open_subkey_with_flags(sid, KEY_READ).is_err() {
                c.not_read(
                    owner,
                    AppPlace::WindowsPersonList,
                    "This person's own apps can be read only while they are signed in. Sign in as them, then scan again.".into(),
                );
                continue;
            }
            // No list at all: this person installed nothing for themselves alone.
            if let Ok(list) = entries(
                &hku,
                &format!(r"{sid}\{UNINSTALL}"),
                KEY_WOW64_64KEY,
                RegistryView::Person,
            ) {
                add_list(&mut c, list, &owner, sid, &win);
            }
            if sid != me {
                c.not_read(
                    owner,
                    AppPlace::WindowsPackages,
                    "Another person's Store apps can be listed only while PCTwin runs as them."
                        .into(),
                );
            }
        }
        let me_owner = Owner::Person {
            account_id: me.to_string(),
        };
        let powershell = win.join(r"System32\WindowsPowerShell\v1.0\powershell.exe");
        let env: Vec<_> = [
            "SystemRoot",
            "windir",
            "USERPROFILE",
            "LOCALAPPDATA",
            "APPDATA",
            "TEMP",
            "TMP",
            "USERNAME",
            "USERDOMAIN",
        ]
        .iter()
        .filter_map(|k| std::env::var_os(k).map(|v| (k.into(), v)))
        .collect();
        let call = Call {
            program: powershell,
            args: vec![
                "-NoProfile".into(),
                "-NonInteractive".into(),
                "-Command".into(),
                PACKAGES_SCRIPT.into(),
            ],
            env,
            timeout: Duration::from_secs(120),
            stdin: None,
            cwd: Some(win.join("System32")),
        };
        match runner.run(&call) {
            Ok(ran) if ran.code == Some(0) => match packages_from(&ran.stdout, &me_owner) {
                Ok(apps) => apps.into_iter().for_each(|a| c.add(a)),
                Err(e) => c.not_read(
                    me_owner,
                    AppPlace::WindowsPackages,
                    format!("The list of Store apps could not be read ({e})."),
                ),
            },
            Ok(ran) => c.not_read(
                me_owner,
                AppPlace::WindowsPackages,
                format!("Windows did not list the Store apps (code {:?}).", ran.code),
            ),
            Err(e) => c.not_read(
                me_owner,
                AppPlace::WindowsPackages,
                format!("Windows did not list the Store apps ({e})."),
            ),
        }
        c.finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(values: &[(&str, RegValue)]) -> RawEntry {
        RawEntry {
            view: RegistryView::Machine,
            key: "{3B0346D3-155D-43E5-9561-B9BE51CF2EB6}".into(),
            values: values
                .iter()
                .map(|(k, v)| (k.to_string(), v.clone()))
                .collect(),
        }
    }

    fn t(s: &str) -> RegValue {
        RegValue::Text(s.into())
    }

    #[test]
    fn only_what_windows_shows_is_listed() {
        assert!(entry(&[("displayname", t("PuTTY"))]).shown());
        assert!(!entry(&[]).shown());
        assert!(!entry(&[("displayname", t("  "))]).shown());
        assert!(
            !entry(&[
                ("displayname", t("X")),
                ("systemcomponent", RegValue::Number(1))
            ])
            .shown()
        );
        assert!(
            entry(&[
                ("displayname", t("X")),
                ("systemcomponent", RegValue::Number(0))
            ])
            .shown()
        );
        assert!(!entry(&[("displayname", t("X")), ("parentkeyname", t("Office"))]).shown());
        assert!(
            !entry(&[
                ("displayname", t("X")),
                ("releasetype", t("Security Update"))
            ])
            .shown()
        );
    }

    #[test]
    fn product_codes_and_their_packed_form() {
        let e = entry(&[
            ("displayname", t("PuTTY")),
            ("windowsinstaller", RegValue::Number(1)),
        ]);
        assert_eq!(
            e.product_code().as_deref(),
            Some("{3B0346D3-155D-43E5-9561-B9BE51CF2EB6}")
        );
        assert!(
            entry(&[("displayname", t("PuTTY"))])
                .product_code()
                .is_none()
        );
        // Windows Installer's own documented example.
        assert_eq!(
            packed_guid("{12345678-ABCD-EF01-2345-6789ABCDEF01}").as_deref(),
            Some("87654321DCBA10FE32547698BADCFE10")
        );
        assert!(packed_guid("{nope}").is_none());
    }

    #[test]
    fn programs_named_in_the_list_are_read_only_outside_windows() {
        let win = Path::new(r"C:\Windows");
        assert_eq!(
            program_in(r#""C:\Program Files\App\app.exe",0"#, win),
            Some(PathBuf::from(r"C:\Program Files\App\app.exe"))
        );
        assert_eq!(
            program_in(r"C:\Program Files\App\unins000.exe /SILENT", win),
            Some(PathBuf::from(r"C:\Program Files\App\unins000.exe"))
        );
        assert_eq!(
            program_in(r"MsiExec.exe /X{3B0346D3-155D-43E5-9561-B9BE51CF2EB6}", win),
            None
        );
        assert_eq!(program_in(r"C:\Windows\System32\rundll32.exe x", win), None);
        assert_eq!(program_in(r"c:\windows\system32\msiexec.exe /x", win), None);
        assert_eq!(program_in(r"C:\App\app.ico", win), None);
        assert_eq!(program_in(r"app.exe", win), None);
    }

    #[test]
    fn the_first_readable_copy_names_the_maker() {
        let signer = pctwin_maker::WindowsSigner {
            subject: Default::default(),
            certificate: pctwin_maker::Sha256([5; 32]),
            issuer: "CA".into(),
            issuer_organization: None,
            public_key: None,
        };
        let copies = vec![PathBuf::from(r"C:\gone.exe"), PathBuf::from(r"C:\app.exe")];
        let s = signer.clone();
        let read = move |p: &Path| {
            if p.ends_with("gone.exe") {
                OldCopy {
                    maker: OldMaker::NotRead("not found".into()),
                    arch: None,
                }
            } else {
                OldCopy {
                    maker: OldMaker::Signed(s.clone()),
                    arch: Some(pctwin_maker::Arch::X64),
                }
            }
        };
        let (maker, arch) = maker_of(&copies, &[], &read);
        assert_eq!(
            maker,
            AppMaker::Windows(Box::new(crate::found::signer_id(&signer)))
        );
        assert!(arch.known && arch.x64 && !arch.arm64);
        let unsigned = |_: &Path| OldCopy {
            maker: OldMaker::Unsigned,
            arch: None,
        };
        assert_eq!(maker_of(&copies, &[], &unsigned).0, AppMaker::Unsigned);
        let none = |_: &Path| OldCopy {
            maker: OldMaker::NotRead("locked".into()),
            arch: None,
        };
        assert!(
            matches!(maker_of(&copies, &[], &none).0, AppMaker::NotRead { why } if why.contains("locked"))
        );
        assert!(
            matches!(maker_of(&[], &[], &none).0, AppMaker::NotRead { why } if why.contains("does not say"))
        );
    }

    #[test]
    fn an_install_folder_decides_only_when_its_programs_agree() {
        let signer = |c: u8| pctwin_maker::WindowsSigner {
            subject: Default::default(),
            certificate: pctwin_maker::Sha256([c; 32]),
            issuer: "CA".into(),
            issuer_organization: None,
            public_key: None,
        };
        let read = |p: &Path| {
            let name = p.file_name().unwrap().to_string_lossy().to_string();
            OldCopy {
                maker: match name.as_str() {
                    "a.exe" | "b.exe" => OldMaker::Signed(signer(1)),
                    "other.exe" => OldMaker::Signed(signer(2)),
                    "plain.exe" => OldMaker::Unsigned,
                    _ => OldMaker::NotRead("locked".into()),
                },
                arch: Some(pctwin_maker::Arch::X64),
            }
        };
        let p = |n: &str| PathBuf::from(format!(r"C:\App\{n}"));
        let agreed = maker_of(
            &[],
            &[p("a.exe"), p("b.exe"), p("plain.exe"), p("x.exe")],
            &read,
        );
        assert_eq!(
            agreed.0,
            AppMaker::Windows(Box::new(crate::found::signer_id(&signer(1))))
        );
        let split = maker_of(&[], &[p("a.exe"), p("other.exe")], &read);
        assert!(matches!(split.0, AppMaker::NotRead { why } if why.contains("different makers")));
        assert_eq!(
            maker_of(&[], &[p("plain.exe")], &read).0,
            AppMaker::Unsigned
        );
        // A named copy that reads comes first.
        assert_eq!(
            maker_of(&[p("plain.exe")], &[p("a.exe")], &read).0,
            AppMaker::Unsigned
        );
        // A named copy that cannot be read gives way to the folder.
        assert!(matches!(
            maker_of(&[p("x.exe")], &[p("a.exe")], &read).0,
            AppMaker::Windows(_)
        ));
        // The folder's own listing: only programs, only directly in it, never in Windows.
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("b.exe"), b"MZ").unwrap();
        std::fs::write(dir.path().join("a.EXE"), b"MZ").unwrap();
        std::fs::write(dir.path().join("readme.txt"), b"x").unwrap();
        std::fs::create_dir(dir.path().join("sub.exe")).unwrap();
        let found = programs_in_folder(dir.path(), Path::new(r"C:\Windows"));
        assert_eq!(found.len(), 2);
        assert!(
            programs_in_folder(Path::new(r"C:\Windows\System32"), Path::new(r"C:\Windows"))
                .is_empty()
        );
        assert!(programs_in_folder(Path::new("relative"), Path::new(r"C:\Windows")).is_empty());
    }

    #[test]
    fn an_entry_becomes_an_app_with_its_web_address_and_registration() {
        let e = entry(&[
            ("displayname", t("PuTTY release 0.85 (64-bit)")),
            ("displayversion", t("0.85.0.0")),
            ("publisher", t("Simon Tatham")),
            ("windowsinstaller", RegValue::Number(1)),
            ("urlinfoabout", t("javascript:alert(1)")),
            (
                "helplink",
                t("https://www.chiark.greenend.org.uk/~sgtatham/putty/"),
            ),
            ("regowner", t("Ada")),
        ]);
        let read = |_: &Path| OldCopy {
            maker: OldMaker::Unsigned,
            arch: None,
        };
        let app = app_of(
            &e,
            Owner::Shared,
            Path::new(r"C:\Windows"),
            Some(PathBuf::from(r"C:\Windows\Installer\1a2b.msi")),
            &read,
        )
        .unwrap();
        assert_eq!(app.name, "PuTTY release 0.85 (64-bit)");
        assert_eq!(app.version.as_deref(), Some("0.85.0.0"));
        assert_eq!(
            app.web.as_deref(),
            Some("https://www.chiark.greenend.org.uk/~sgtatham/putty/")
        );
        assert_eq!(app.maker, AppMaker::Unsigned);
        assert!(matches!(
            &app.found,
            FoundWhere::WindowsEntry {
                product_code: Some(_),
                ..
            }
        ));
        assert_eq!(app.notes.len(), 1);
        app.validate().unwrap();
    }

    #[test]
    fn web_addresses_are_ordinary_or_dropped() {
        assert!(web_address("https://example.com/a?b=c").is_some());
        assert!(web_address("http://example.com").is_some());
        assert!(web_address("file:///C:/x").is_none());
        assert!(web_address("https://user:pw@example.com").is_none());
        assert!(web_address("ftp://example.com").is_none());
        assert!(web_address("example.com").is_none());
    }

    #[test]
    fn packaged_apps_are_read_from_the_fixed_script() {
        let owner = Owner::Person {
            account_id: "S-1-5-21-1".into(),
        };
        let one = r#"{"n":"Microsoft.WindowsCalculator","p":"CN=Microsoft Corporation, O=Microsoft Corporation, L=Redmond, S=Washington, C=US","f":"Microsoft.WindowsCalculator_8wekyb3d8bbwe","s":"Store","a":"X64","v":"11.2502.2.0"}"#;
        let apps = packages_from(one, &owner).unwrap();
        assert_eq!(apps.len(), 1);
        assert!(
            matches!(&apps[0].found, FoundWhere::WindowsPackage { signing: PackageSigning::Store, family_name } if family_name.ends_with("8wekyb3d8bbwe"))
        );
        assert!(
            matches!(&apps[0].maker, AppMaker::WindowsPackage { publisher } if publisher.starts_with("CN=Microsoft"))
        );
        let list = format!(
            "[{one},{}]",
            one.replace("Store", "System").replace("X64", "Neutral")
        );
        let apps = packages_from(&list, &owner).unwrap();
        assert_eq!(apps.len(), 2);
        assert!(apps[1].arch.arm64 && apps[1].arch.x64);
        assert!(packages_from("", &owner).unwrap().is_empty());
        assert!(packages_from("not json", &owner).is_err());
        // An object missing a field is not what the script writes.
        assert!(packages_from(r#"{"n":"x"}"#, &owner).is_err());
    }
}
