//! winget, used only as a source of the manifest and the bytes, always as the person, never
//! elevated (the "Installing (2.1)" research notes, 27 October 2026).
//!
//! - **Finding the package.** `winget search` matches a Windows Installer product code exactly
//!   ("Match: ProductCode: {…}", checked on winget v1.29), so an app with one is looked up by
//!   it; an app without one by its exact name. The table winget prints is in the person's
//!   language, so PCTwin reads only what does not change: the rows that hold the product code
//!   (or the name) and the package IDs in them (a winget ID has no spaces and at least one dot).
//!   A search only suggests: the manifest and then the installer's own signer decide.
//! - **Getting it.** `winget download --id <id> --exact` writes the installer and its merged
//!   manifest (`<id>_<version>_..._<locale>.yaml`) into a folder PCTwin names. winget checks the
//!   installer's SHA-256 against the manifest; PCTwin checks it again, and then the maker.
//! - **The manifest** is read by [`Manifest::parse`], a small reader for the plain
//!   `key: value` lines winget writes (never a general YAML reader): the package's ID, version,
//!   publisher and name, and the one installer's type, address, SHA-256, scope, product code,
//!   package family name and its Apps & Features entries. Anything it cannot read is left out,
//!   and the checks that need it then fail safe.

use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::time::Duration;

use pctwin_maker::run::Call;
use pctwin_record::{FoundApp, FoundWhere};

/// The largest manifest read.
pub const MAX_MANIFEST: usize = 256 * 1024;
/// Most package IDs taken from one search.
pub const MAX_CANDIDATES: usize = 3;
const SEARCH_TIME: Duration = Duration::from_secs(120);
const DOWNLOAD_TIME: Duration = Duration::from_secs(3600);

/// A winget package ID: no spaces, at least one dot, at most 128 characters of letters, digits
/// and `.-_+`.
pub fn valid_id(id: &str) -> bool {
    id.len() <= 128
        && id.contains('.')
        && !id.starts_with('.')
        && !id.ends_with('.')
        && !id.contains("..")
        && id
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'.' | b'-' | b'_' | b'+'))
}

/// Where winget is for the person: the app alias Windows puts in their own `WindowsApps`.
pub fn winget_path(local_app_data: &Path) -> PathBuf {
    local_app_data
        .join("Microsoft")
        .join("WindowsApps")
        .join("winget.exe")
}

/// The environment winget needs to run as the person, and nothing more.
pub fn person_env() -> Vec<(OsString, OsString)> {
    [
        "SystemRoot",
        "windir",
        "USERPROFILE",
        "LOCALAPPDATA",
        "APPDATA",
        "TEMP",
        "TMP",
        "USERNAME",
        "USERDOMAIN",
        "ProgramData",
        "ProgramFiles",
        "ProgramFiles(x86)",
    ]
    .iter()
    .filter_map(|k| std::env::var_os(k).map(|v| (OsString::from(k), v)))
    .collect()
}

/// What to search winget for: an app's product code, or else its exact name.
pub fn search_query(app: &FoundApp) -> Option<(String, bool)> {
    match &app.found {
        FoundWhere::WindowsEntry {
            product_code: Some(code),
            ..
        } => Some((code.clone(), true)),
        FoundWhere::WindowsEntry { .. } => Some((app.name.clone(), false)),
        _ => None,
    }
}

/// `winget search <query> --source winget` (exactly, for a name).
pub fn search_call(winget: &Path, query: &str, by_code: bool) -> Call {
    let mut args: Vec<OsString> = vec!["search".into()];
    if by_code {
        args.push(query.into());
    } else {
        args.extend(["--name".into(), query.into(), "--exact".into()]);
    }
    args.extend(
        [
            "--source",
            "winget",
            "--accept-source-agreements",
            "--disable-interactivity",
        ]
        .map(OsString::from),
    );
    Call {
        program: winget.to_path_buf(),
        args,
        env: person_env(),
        timeout: SEARCH_TIME,
        stdin: None,
        cwd: None,
    }
}

/// The package IDs in the rows of a search's table that hold `query` (case-insensitively), at
/// most [`MAX_CANDIDATES`], in winget's order.
pub fn candidates(table: &str, query: &str) -> Vec<String> {
    let q = query.to_lowercase();
    let mut out = Vec::new();
    for row in table.lines() {
        if !row.to_lowercase().contains(&q) {
            continue;
        }
        for token in row.split_whitespace() {
            if valid_id(token)
                && !token.to_lowercase().contains(&q)
                && !out.iter().any(|o: &String| o == token)
                && !looks_like_version(token)
            {
                out.push(token.to_string());
            }
        }
        if out.len() >= MAX_CANDIDATES {
            break;
        }
    }
    out.truncate(MAX_CANDIDATES);
    out
}

fn looks_like_version(t: &str) -> bool {
    t.bytes().all(|b| b.is_ascii_digit() || b == b'.')
}

/// `winget download --id <id> --exact` into `folder`, for this laptop's processor.
pub fn download_call(winget: &Path, id: &str, arch: &str, folder: &Path) -> Option<Call> {
    if !valid_id(id) || !matches!(arch, "x64" | "x86" | "arm64") {
        return None;
    }
    let args: Vec<OsString> = vec![
        "download".into(),
        "--id".into(),
        id.into(),
        "--exact".into(),
        "--source".into(),
        "winget".into(),
        "--architecture".into(),
        arch.into(),
        "--download-directory".into(),
        folder.as_os_str().to_owned(),
        "--accept-source-agreements".into(),
        "--accept-package-agreements".into(),
        "--disable-interactivity".into(),
        // Only the installer: a dependency is its own app, checked on its own.
        "--skip-dependencies".into(),
    ];
    Some(Call {
        program: winget.to_path_buf(),
        args,
        env: person_env(),
        timeout: DOWNLOAD_TIME,
        stdin: None,
        cwd: Some(folder.to_path_buf()),
    })
}

/// One Apps & Features entry a manifest says its installer makes.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ArpEntry {
    pub display_name: Option<String>,
    pub publisher: Option<String>,
    pub product_code: Option<String>,
    pub upgrade_code: Option<String>,
}

/// What PCTwin reads from a merged manifest.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Manifest {
    pub id: Option<String>,
    pub version: Option<String>,
    pub publisher: Option<String>,
    pub name: Option<String>,
    pub installer_type: Option<String>,
    pub nested_installer_type: Option<String>,
    pub url: Option<String>,
    pub sha256: Option<[u8; 32]>,
    pub scope: Option<String>,
    pub architecture: Option<String>,
    pub product_code: Option<String>,
    pub family_name: Option<String>,
    pub entries: Vec<ArpEntry>,
    /// More than one installer was listed: which one was downloaded is not known from the
    /// manifest alone.
    pub several_installers: bool,
}

fn unquote(v: &str) -> Option<String> {
    let v = v.trim();
    let v = if v.len() >= 2
        && ((v.starts_with('\'') && v.ends_with('\'')) || (v.starts_with('"') && v.ends_with('"')))
    {
        &v[1..v.len() - 1]
    } else {
        v
    };
    // A value carried on the next lines (a block or a folded value) is not read.
    if v.is_empty() || v == "|" || v == ">" || v.starts_with("|-") || v.starts_with(">-") {
        return None;
    }
    // An escaped character (in double quotes) is not read: the text is not taken as written.
    if v.contains('\\') {
        return None;
    }
    Some(v.to_string())
}

fn hex32(s: &str) -> Option<[u8; 32]> {
    let s = s.trim();
    if s.len() != 64 {
        return None;
    }
    let mut out = [0u8; 32];
    for (i, pair) in s.as_bytes().chunks(2).enumerate() {
        out[i] = u8::from_str_radix(std::str::from_utf8(pair).ok()?, 16).ok()?;
    }
    Some(out)
}

impl Manifest {
    /// Reads the plain lines of a merged manifest. Only top-level keys, the keys of the first
    /// item of `Installers` and the items of its `AppsAndFeaturesEntries` are read.
    pub fn parse(text: &str) -> Result<Manifest, String> {
        if text.len() > MAX_MANIFEST {
            return Err("the manifest is too large".into());
        }
        let mut m = Manifest::default();
        #[derive(PartialEq)]
        enum In {
            Top,
            Installers,
            Entries,
            Other,
        }
        let mut section = In::Top;
        let mut installers = 0;
        for raw in text.lines() {
            if raw.trim().is_empty() || raw.trim_start().starts_with('#') {
                continue;
            }
            let indent = raw.len() - raw.trim_start().len();
            let line = raw.trim();
            let (item, line) = match line.strip_prefix("- ") {
                Some(rest) => (true, rest),
                None => (false, line),
            };
            let Some((key, value)) = line.split_once(':') else {
                continue;
            };
            let key = key.trim();
            if indent == 0 && !item {
                section = match key {
                    "Installers" => In::Installers,
                    _ if value.trim().is_empty() => In::Other,
                    _ => In::Top,
                };
                let v = unquote(value);
                match key {
                    "PackageIdentifier" => m.id = v,
                    "PackageVersion" => m.version = v,
                    "Publisher" => m.publisher = v,
                    "PackageName" => m.name = v,
                    // Fields winget lets either level carry: the top level is the default.
                    "InstallerType" => m.installer_type = m.installer_type.take().or(v),
                    "Scope" => m.scope = m.scope.take().or(v),
                    "ProductCode" => m.product_code = m.product_code.take().or(v),
                    "PackageFamilyName" => m.family_name = m.family_name.take().or(v),
                    _ => {}
                }
                continue;
            }
            // An item at the left edge of `Installers` starts another installer.
            if section == In::Entries && indent == 0 && item {
                section = In::Installers;
            }
            if section == In::Installers && item && indent == 0 {
                installers += 1;
            }
            if installers > 1 {
                m.several_installers = true;
                section = In::Other;
            }
            match section {
                In::Installers if (indent == 0 && item) || (indent == 2 && !item) => {
                    let v = unquote(value);
                    match key {
                        "Architecture" => m.architecture = v,
                        "InstallerType" => m.installer_type = v.or(m.installer_type.take()),
                        "NestedInstallerType" => m.nested_installer_type = v,
                        "InstallerUrl" => m.url = v,
                        "InstallerSha256" => m.sha256 = v.as_deref().and_then(hex32),
                        "Scope" => m.scope = v.or(m.scope.take()),
                        "ProductCode" => m.product_code = v.or(m.product_code.take()),
                        "PackageFamilyName" => m.family_name = v.or(m.family_name.take()),
                        "AppsAndFeaturesEntries" => section = In::Entries,
                        _ => {}
                    }
                }
                // An entry starts with "  - " and carries on at four spaces.
                In::Entries if (indent == 2 && item) || indent >= 4 => {
                    if item {
                        m.entries.push(ArpEntry::default());
                    }
                    let Some(e) = m.entries.last_mut() else {
                        continue;
                    };
                    let v = unquote(value);
                    match key {
                        "DisplayName" => e.display_name = v,
                        "Publisher" => e.publisher = v,
                        "ProductCode" => e.product_code = v,
                        "UpgradeCode" => e.upgrade_code = v,
                        _ => {}
                    }
                }
                In::Entries => {
                    // Back to the installer's own keys.
                    section = In::Installers;
                    let v = unquote(value);
                    match key {
                        "InstallerType" => m.installer_type = v.or(m.installer_type.take()),
                        "InstallerUrl" => m.url = v,
                        "InstallerSha256" => m.sha256 = v.as_deref().and_then(hex32),
                        "Scope" => m.scope = v.or(m.scope.take()),
                        "ProductCode" => m.product_code = v.or(m.product_code.take()),
                        _ => {}
                    }
                }
                _ => {}
            }
        }
        Ok(m)
    }
}

/// How a manifest was found to describe the app on the old laptop.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Confirmed {
    /// The same Windows Installer product code.
    ProductCode,
    /// The same Apps & Features name and publisher.
    NameAndPublisher,
}

fn same_text(a: &str, b: &str) -> bool {
    a.trim().eq_ignore_ascii_case(b.trim())
}

/// Names that are the same once their version numbers are set aside, ignoring case.
fn same_name(a: &str, b: &str) -> bool {
    let strip = |s: &str| {
        s.chars()
            .filter(|c| !c.is_ascii_digit() && *c != '.')
            .collect::<String>()
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ")
            .to_lowercase()
    };
    let (x, y) = (strip(a), strip(b));
    !x.is_empty() && x == y
}

/// Whether manifest `m` describes `app`: by the product code when the app has one (and then only
/// by it), or else by an Apps & Features entry with exactly its name and publisher. This only
/// picks the package; the installer's signer is what PCTwin trusts.
pub fn confirms(m: &Manifest, app: &FoundApp) -> Option<Confirmed> {
    let codes = m
        .entries
        .iter()
        .filter_map(|e| e.product_code.as_deref())
        .chain(m.product_code.as_deref());
    match &app.found {
        FoundWhere::WindowsEntry {
            product_code: Some(code),
            ..
        } => {
            let mut codes = codes;
            // The product code, and the name too (sixth review, finding 5): an old laptop's list
            // that names an app one thing and gives another package's code never installs that
            // other package under the name shown. Names compare without their version numbers
            // ("PuTTY release 0.85" and "PuTTY release 0.86"), and the package's own name counts.
            let named = m
                .entries
                .iter()
                .filter_map(|e| e.display_name.as_deref())
                .chain(m.name.as_deref())
                .any(|n| same_name(n, &app.name));
            (codes.any(|c| same_text(c, code)) && named).then_some(Confirmed::ProductCode)
        }
        FoundWhere::WindowsEntry { .. } => {
            let publisher = app.publisher.as_deref()?;
            // An entry's missing fields are the package's own (winget's rule).
            m.entries
                .iter()
                .any(|e| {
                    e.display_name
                        .as_deref()
                        .or(m.name.as_deref())
                        .is_some_and(|n| same_text(n, &app.name))
                        && e.publisher
                            .as_deref()
                            .or(m.publisher.as_deref())
                            .is_some_and(|p| same_text(p, publisher))
                })
                .then_some(Confirmed::NameAndPublisher)
        }
        _ => None,
    }
}

impl Manifest {
    /// The one Windows Installer product code this manifest names for its installer (its
    /// `ProductCode`, or else the one its Apps & Features entries share), checked as a product
    /// code. Two different codes, or none, name none: the helper then records nothing, and undo
    /// lists the app for the person instead of removing it.
    pub fn product_code_of_installer(&self) -> Option<pctwin_helper::broker::actions::ProductCode> {
        use pctwin_helper::broker::actions::ProductCode;
        let mut codes = self
            .product_code
            .iter()
            .chain(self.entries.iter().filter_map(|e| e.product_code.as_ref()))
            .map(|c| ProductCode::parse(c.trim()));
        let first = codes.next()??;
        codes.all(|c| c == Some(first)).then_some(first)
    }
}

/// The manifest and installer winget wrote into `folder`: exactly one `.yaml` and one other
/// file. Anything else is refused (PCTwin never picks the first match).
pub fn downloaded(folder: &Path) -> Result<(PathBuf, PathBuf), String> {
    let mut yaml = Vec::new();
    let mut other = Vec::new();
    for e in std::fs::read_dir(folder)
        .map_err(|e| e.to_string())?
        .flatten()
    {
        let ft = e.file_type().map_err(|e| e.to_string())?;
        if !ft.is_file() {
            return Err("winget left something that is not a file".into());
        }
        let p = e.path();
        if p.extension()
            .is_some_and(|x| x.eq_ignore_ascii_case("yaml"))
        {
            yaml.push(p);
        } else {
            other.push(p);
        }
    }
    match (yaml.as_slice(), other.as_slice()) {
        ([y], [i]) => Ok((y.clone(), i.clone())),
        _ => Err(format!(
            "winget left {} manifests and {} other files, not one of each",
            yaml.len(),
            other.len()
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The manifest winget v1.29 wrote for PuTTY 0.85 on 27 October 2026 (trimmed of its long
    /// texts, which the reader skips).
    pub const PUTTY: &str = r#"PackageIdentifier: PuTTY.PuTTY
PackageVersion: 0.85.0.0
Moniker: putty
Tags:
- puttygen
- ssh
PackageLocale: en-US
Author: Simon Tatham
Copyright: "\xA9 1997-2026 Simon Tatham. All rights reserved."
Description: 'PuTTY is a client program for the SSH, Telnet, Rlogin, and SUPDUP network
  protocols.

  These protocols are all used to run a remote session on a computer, over a network.'
License: MIT
PackageName: PuTTY
Publisher: Simon Tatham
Localization:
- Documentations:
  - DocumentLabel: "\u6587\u6863"
  PackageLocale: zh-CN
Installers:
- Architecture: x64
  InstallerType: wix
  InstallerSha256: 2213ebacf962b709411d654a40087508a694756bc4e41c5a1c24ad86596b2511
  InstallerUrl: https://the.earth.li/~sgtatham/putty/0.85/w64/putty-64bit-0.85-installer.msi
  Scope: machine
  ElevationRequirement: elevatesSelf
  ProductCode: '{3B0346D3-155D-43E5-9561-B9BE51CF2EB6}'
  UpgradeBehavior: uninstallPrevious
  FileExtensions:
  - ppk
  AppsAndFeaturesEntries:
  - DisplayName: PuTTY release 0.85 (64-bit)
    ProductCode: '{3B0346D3-155D-43E5-9561-B9BE51CF2EB6}'
    UpgradeCode: '{C9EAA861-2B72-4FAF-9FEE-EEB1AD5FD15E}'
  ExpectedReturnCodes:
  - InstallerReturnCode: 87
    ReturnResponse: invalidParameter
  - InstallerReturnCode: 3010
    ReturnResponse: rebootRequiredToFinish
  InstallerSwitches:
    Silent: /quiet /norestart
    Custom: /evil
ManifestVersion: 1.12.0
ManifestType: merged
"#;

    fn putty_app(code: Option<&str>) -> FoundApp {
        FoundApp {
            name: "PuTTY release 0.85 (64-bit)".into(),
            version: Some("0.85.0.0".into()),
            publisher: Some("Simon Tatham".into()),
            owner: pctwin_record::Owner::Shared,
            system: pctwin_record::AppSystem::Windows,
            found: FoundWhere::WindowsEntry {
                view: pctwin_record::RegistryView::Machine,
                key: "k".into(),
                product_code: code.map(Into::into),
            },
            arch: pctwin_record::AppArch::UNKNOWN,
            maker: pctwin_record::AppMaker::Unsigned,
            web: None,
            update_feed: None,
            installers: vec![],
            notes: vec![],
        }
    }

    #[test]
    fn the_manifest_winget_writes_is_read() {
        let m = Manifest::parse(PUTTY).unwrap();
        assert_eq!(m.id.as_deref(), Some("PuTTY.PuTTY"));
        assert_eq!(m.version.as_deref(), Some("0.85.0.0"));
        assert_eq!(m.publisher.as_deref(), Some("Simon Tatham"));
        assert_eq!(m.installer_type.as_deref(), Some("wix"));
        assert_eq!(m.architecture.as_deref(), Some("x64"));
        assert_eq!(m.scope.as_deref(), Some("machine"));
        assert_eq!(
            m.url.as_deref(),
            Some("https://the.earth.li/~sgtatham/putty/0.85/w64/putty-64bit-0.85-installer.msi")
        );
        assert_eq!(m.sha256.unwrap()[0], 0x22);
        assert_eq!(
            m.product_code.as_deref(),
            Some("{3B0346D3-155D-43E5-9561-B9BE51CF2EB6}")
        );
        assert_eq!(m.entries.len(), 1);
        assert_eq!(
            m.entries[0].display_name.as_deref(),
            Some("PuTTY release 0.85 (64-bit)")
        );
        assert!(!m.several_installers);
    }

    /// The product code recorded for undo is the manifest's one code, checked; two codes that
    /// differ, a code that is not one, or none, give none.
    #[test]
    fn the_installers_product_code_is_its_one_checked_code() {
        let m = Manifest::parse(PUTTY).unwrap();
        assert_eq!(
            m.product_code_of_installer().unwrap().as_str(),
            "{3B0346D3-155D-43E5-9561-B9BE51CF2EB6}"
        );
        // Only the entries name it.
        let entries_only = PUTTY.replacen(
            "  ProductCode: '{3B0346D3-155D-43E5-9561-B9BE51CF2EB6}'\n",
            "",
            1,
        );
        let m = Manifest::parse(&entries_only).unwrap();
        assert_eq!(m.product_code, None);
        assert_eq!(
            m.product_code_of_installer().unwrap().as_str(),
            "{3B0346D3-155D-43E5-9561-B9BE51CF2EB6}"
        );
        // The installer and its entry disagree.
        let two = PUTTY.replacen(
            "    ProductCode: '{3B0346D3-155D-43E5-9561-B9BE51CF2EB6}'",
            "    ProductCode: '{11111111-2222-3333-4444-555555555555}'",
            1,
        );
        assert_eq!(
            Manifest::parse(&two).unwrap().product_code_of_installer(),
            None
        );
        // Not a product code.
        let bad = PUTTY.replace("{3B0346D3-155D-43E5-9561-B9BE51CF2EB6}", "3B0346D3 155D");
        assert_eq!(
            Manifest::parse(&bad).unwrap().product_code_of_installer(),
            None
        );
        // None at all.
        assert_eq!(Manifest::default().product_code_of_installer(), None);
    }

    #[test]
    fn an_escaped_value_is_not_taken_as_written() {
        let escaped = PUTTY.replace("Publisher: Simon Tatham", r#"Publisher: "Simon\x41Tatham""#);
        let m = Manifest::parse(&escaped).unwrap();
        assert_eq!(m.publisher, None);
    }

    #[test]
    fn a_second_installer_is_noticed_and_not_mixed_in() {
        let two = PUTTY.replace(
            "ManifestVersion: 1.12.0",
            "- Architecture: arm64\n  InstallerType: exe\n  InstallerUrl: https://elsewhere.invalid/x.exe\nManifestVersion: 1.12.0",
        );
        let two = two.replace(
            "  InstallerSwitches:\n    Silent: /quiet /norestart\n    Custom: /evil\n",
            "",
        );
        let m = Manifest::parse(&two).unwrap();
        assert!(m.several_installers);
        assert_eq!(m.architecture.as_deref(), Some("x64"));
        assert_eq!(m.installer_type.as_deref(), Some("wix"));
        assert!(m.url.as_deref().unwrap().contains("the.earth.li"));
    }

    #[test]
    fn a_manifest_confirms_the_app_by_its_product_code_only() {
        let m = Manifest::parse(PUTTY).unwrap();
        assert_eq!(
            confirms(
                &m,
                &putty_app(Some("{3b0346d3-155d-43e5-9561-b9be51cf2eb6}"))
            ),
            Some(Confirmed::ProductCode)
        );
        // Another product code: never confirmed by the name instead (T21-001).
        assert_eq!(
            confirms(
                &m,
                &putty_app(Some("{00000000-155D-43E5-9561-B9BE51CF2EB6}"))
            ),
            None
        );
        assert_eq!(
            confirms(&m, &putty_app(None)),
            Some(Confirmed::NameAndPublisher)
        );
        let mut other = putty_app(None);
        other.publisher = Some("Someone Else".into());
        assert_eq!(confirms(&m, &other), None);
        other.publisher = None;
        assert_eq!(confirms(&m, &other), None);
    }

    #[test]
    fn search_rows_give_only_package_ids() {
        let table = "Name  Id          Version  Match\n------------------------------------------------------------------------------\nPuTTY PuTTY.PuTTY 0.85.0.0 ProductCode: {3b0346d3-155d-43e5-9561-b9be51cf2eb6}\nOther Evil.Pkg 1.0 Tag: ssh\n";
        assert_eq!(
            candidates(table, "{3B0346D3-155D-43E5-9561-B9BE51CF2EB6}"),
            vec!["PuTTY.PuTTY".to_string()]
        );
        let named = "Nom      ID            Version\n-------------------------------\nNode.js  OpenJS.NodeJS  20.1.0\n";
        // "Node.js" looks like an ID too; both are suggestions only, and at most three.
        let got = candidates(named, "Node.js");
        assert!(got.contains(&"OpenJS.NodeJS".to_string()));
        assert!(!got.contains(&"20.1.0".to_string()));
        assert!(candidates("", "x").is_empty());
    }

    #[test]
    fn ids_and_calls_take_nothing_else() {
        assert!(valid_id("PuTTY.PuTTY"));
        assert!(valid_id("Microsoft.VisualStudioCode"));
        assert!(!valid_id("PuTTY"));
        assert!(!valid_id("a b.c"));
        assert!(!valid_id("--ignore-security-hash"));
        assert!(!valid_id("x..y"));
        assert!(!valid_id("x.y;calc"));
        let folder = Path::new(r"C:\Users\Ada\AppData\Local\PCTwin\fetch\1");
        let call = download_call(Path::new("winget.exe"), "PuTTY.PuTTY", "x64", folder).unwrap();
        assert!(call.args.contains(&OsString::from("--skip-dependencies")));
        assert!(!call.args.iter().any(|a| a == "--ignore-security-hash"));
        assert!(
            download_call(
                Path::new("winget.exe"),
                "--ignore-security-hash",
                "x64",
                folder
            )
            .is_none()
        );
        assert!(
            download_call(
                Path::new("winget.exe"),
                "PuTTY.PuTTY",
                "x64 --force",
                folder
            )
            .is_none()
        );
    }

    #[test]
    fn only_one_manifest_and_one_installer_are_taken() {
        let dir = tempfile::tempdir().unwrap();
        assert!(downloaded(dir.path()).is_err());
        std::fs::write(dir.path().join("a.yaml"), "x").unwrap();
        std::fs::write(dir.path().join("a.msi"), "x").unwrap();
        assert!(downloaded(dir.path()).is_ok());
        std::fs::write(dir.path().join("b.exe"), "x").unwrap();
        assert!(downloaded(dir.path()).is_err());
    }

    /// Sixth review, finding 5: an app named one thing on the old laptop with another package's
    /// product code is never confirmed as that package.
    #[test]
    fn a_product_code_under_another_name_is_not_confirmed() {
        let m = Manifest::parse(PUTTY).unwrap();
        let code = Some("{3B0346D3-155D-43E5-9561-B9BE51CF2EB6}");
        assert_eq!(confirms(&m, &putty_app(code)), Some(Confirmed::ProductCode));
        let mut other = putty_app(code);
        other.name = "Calculator".into();
        assert_eq!(confirms(&m, &other), None);
        // Another release of the same app is still the same name.
        let mut newer = putty_app(code);
        newer.name = "PuTTY release 0.86 (64-bit)".into();
        assert_eq!(confirms(&m, &newer), Some(Confirmed::ProductCode));
    }
}
