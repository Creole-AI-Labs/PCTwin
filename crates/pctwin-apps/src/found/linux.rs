//! Linux: every package that gives the desktop an app, every Flatpak app, every Snap.
//!
//! **Which packages.** The desktop's own rule: an app is a `.desktop` file in
//! `/usr/share/applications` that is not hidden (`NoDisplay=true` or `Hidden=true`). Each such
//! file is traced to the package that owns it (dpkg's file lists, or `rpm -qf`), so libraries
//! and tools without a window are not listed as apps.
//!
//! **Who made it.** For dpkg, the repository the installed version came from (`apt-cache
//! policy`), or none for a package installed from a file; for rpm, the key that signed it; for
//! Flatpak, the old laptop's own remote, its address and the SHA-256 of its signing keyring; for
//! Snap, its publisher. The new laptop installs only from the same source.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::time::Duration;

use pctwin_maker::run::{Call, Runner};
use pctwin_record::apps::{MAX_APP_TEXT, MAX_APP_URL};
use pctwin_record::{AppArch, AppMaker, AppPlace, AppSystem, FoundApp, FoundWhere, Owner};
use sha2::{Digest, Sha256};

use super::{Collector, clean};

const TOOL_TIME: Duration = Duration::from_secs(120);
/// The largest file read (dpkg's status file, a desktop file, a keyring).
const MAX_FILE: u64 = 64 * 1024 * 1024;
const MAX_DESKTOP: u64 = 256 * 1024;

fn call(program: &str, args: &[&str]) -> Call {
    Call {
        program: PathBuf::from(program),
        args: args.iter().map(Into::into).collect(),
        env: vec![("LC_ALL".into(), "C".into())],
        timeout: TOOL_TIME,
        stdin: None,
        cwd: None,
    }
}

fn read_small(p: &Path, max: u64) -> Option<String> {
    let m = std::fs::symlink_metadata(p).ok()?;
    if !m.is_file() || m.len() > max {
        return None;
    }
    std::fs::read_to_string(p).ok()
}

/// Whether a desktop file is an app the menus show.
pub fn desktop_shown(text: &str) -> bool {
    let mut in_entry = false;
    let mut kind_app = false;
    for line in text.lines().map(str::trim) {
        if line.starts_with('[') {
            in_entry = line == "[Desktop Entry]";
            continue;
        }
        if !in_entry {
            continue;
        }
        match line.split_once('=').map(|(k, v)| (k.trim(), v.trim())) {
            Some(("NoDisplay" | "Hidden", "true")) => return false,
            Some(("Type", "Application")) => kind_app = true,
            _ => {}
        }
    }
    kind_app
}

/// The shown desktop files in `folder`.
pub fn shown_desktop_files(folder: &Path) -> Vec<PathBuf> {
    let Ok(rd) = std::fs::read_dir(folder) else {
        return vec![];
    };
    let mut out: Vec<PathBuf> = rd
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|e| e == "desktop"))
        .filter(|p| read_small(p, MAX_DESKTOP).is_some_and(|t| desktop_shown(&t)))
        .collect();
    out.sort();
    out
}

/// One installed package from dpkg's status file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DebPackage {
    pub name: String,
    pub version: String,
    pub arch: String,
    pub homepage: Option<String>,
}

/// dpkg's status file: the packages marked installed.
pub fn parse_dpkg_status(text: &str) -> Vec<DebPackage> {
    let mut out = Vec::new();
    for stanza in text.split("\n\n") {
        let mut f: BTreeMap<&str, &str> = BTreeMap::new();
        for line in stanza.lines() {
            if line.starts_with([' ', '\t']) {
                continue;
            }
            if let Some((k, v)) = line.split_once(':') {
                f.insert(k.trim(), v.trim());
            }
        }
        if f.get("Status").is_some_and(|s| s.ends_with(" installed"))
            && let (Some(n), Some(v)) = (f.get("Package"), f.get("Version"))
        {
            out.push(DebPackage {
                name: (*n).to_string(),
                version: (*v).to_string(),
                arch: f.get("Architecture").unwrap_or(&"").to_string(),
                homepage: f.get("Homepage").map(|h| (*h).to_string()),
            });
        }
    }
    out
}

/// Which package owns each of `desktop` files, from dpkg's `info/*.list` files in `info`.
pub fn dpkg_owners(info: &Path, desktop: &[PathBuf]) -> BTreeMap<PathBuf, String> {
    let wanted: BTreeSet<String> = desktop
        .iter()
        .map(|p| p.to_string_lossy().into_owned())
        .collect();
    let mut out = BTreeMap::new();
    let Ok(rd) = std::fs::read_dir(info) else {
        return out;
    };
    for e in rd.flatten() {
        let p = e.path();
        let Some(name) = p.file_name().and_then(|n| n.to_str()) else {
            continue;
        };
        let Some(pkg) = name.strip_suffix(".list") else {
            continue;
        };
        // "name:arch.list" for multi-arch packages.
        let pkg = pkg.split(':').next().unwrap_or(pkg);
        if let Some(text) = read_small(&p, MAX_FILE) {
            for line in text.lines() {
                if wanted.contains(line) {
                    out.insert(PathBuf::from(line), pkg.to_string());
                }
            }
        }
    }
    out
}

/// `apt-cache policy` for each package: where its installed version comes from. A package whose
/// installed version is only in dpkg's own status (installed from a file) has no repository.
pub fn parse_apt_policy(text: &str) -> BTreeMap<String, Option<String>> {
    let mut out = BTreeMap::new();
    let mut current: Option<String> = None;
    let mut in_installed = false;
    let mut found: Option<String> = None;
    let mut finish = |pkg: &Option<String>, found: &mut Option<String>| {
        if let Some(p) = pkg {
            out.insert(p.clone(), found.take());
        }
    };
    for line in text.lines() {
        if !line.starts_with(' ') && line.ends_with(':') {
            finish(&current, &mut found);
            current = Some(line.trim_end_matches(':').to_string());
            in_installed = false;
            continue;
        }
        let t = line.trim();
        if let Some(rest) = t.strip_prefix("*** ") {
            in_installed = !rest.is_empty();
            continue;
        }
        if in_installed {
            // "500 http://archive.ubuntu.com/ubuntu noble/main amd64 Packages"
            let parts: Vec<&str> = t.split_whitespace().collect();
            if parts.len() >= 3 && parts[0].parse::<i32>().is_ok() {
                if parts[1] != "/var/lib/dpkg/status" && found.is_none() {
                    found = Some(format!("{} {}", parts[1], parts[2]));
                }
            } else {
                in_installed = false;
            }
        }
    }
    finish(&current, &mut found);
    out
}

fn deb_arch(a: &str) -> AppArch {
    let mut out = AppArch::UNKNOWN;
    match a {
        "amd64" => out.x64 = true,
        "arm64" => out.arm64 = true,
        "i386" => out.x86 = true,
        "all" => {
            out.x86 = true;
            out.x64 = true;
            out.arm64 = true;
        }
        _ => return out,
    }
    out.known = true;
    out
}

/// The dpkg apps: each package owning a shown desktop file.
pub fn deb_apps(
    status: &str,
    owners: &BTreeMap<PathBuf, String>,
    policy: &BTreeMap<String, Option<String>>,
) -> Vec<FoundApp> {
    let apps: BTreeSet<&String> = owners.values().collect();
    parse_dpkg_status(status)
        .into_iter()
        .filter(|p| apps.contains(&p.name))
        .filter_map(|p| {
            let origin = policy.get(&p.name).cloned().flatten();
            Some(FoundApp {
                name: clean(&p.name, MAX_APP_TEXT)?,
                version: clean(&p.version, MAX_APP_TEXT),
                publisher: None,
                owner: Owner::Shared,
                system: AppSystem::Linux,
                found: FoundWhere::Deb {
                    package: clean(&p.name, MAX_APP_TEXT)?,
                },
                arch: deb_arch(&p.arch),
                maker: match origin.and_then(|o| clean(&o, MAX_APP_TEXT)) {
                    Some(origin) => AppMaker::Repository { origin },
                    None => AppMaker::Unsigned,
                },
                web: p.homepage.as_deref().and_then(super::windows::web_address),
                update_feed: None,
                installers: vec![],
                notes: vec![],
            })
        })
        .collect()
}

/// The fixed rpm query: name, version, processor, signing key line, web address.
pub const RPM_FORMAT: &str = "%{NAME}\\t%{EPOCHNUM}:%{VERSION}-%{RELEASE}\\t%{ARCH}\\t%{SIGPGP:pgpsig}|%{RSAHEADER:pgpsig}\\t%{URL}\\n";

/// The key ID in rpm's signature line ("RSA/SHA256, Mon 01 Jan 2024, Key ID 809a8d7ceb10b464").
pub fn rpm_key(sig: &str) -> Option<String> {
    sig.split('|')
        .filter_map(|s| s.rsplit_once("Key ID ").map(|(_, k)| k.trim()))
        .find(|k| !k.is_empty() && k.bytes().all(|b| b.is_ascii_hexdigit()))
        .map(str::to_ascii_lowercase)
}

/// The rpm apps: lines of [`RPM_FORMAT`] for the packages owning shown desktop files.
pub fn rpm_apps(query: &str, app_packages: &BTreeSet<String>) -> Vec<FoundApp> {
    query
        .lines()
        .filter_map(|line| {
            let f: Vec<&str> = line.split('\t').collect();
            if f.len() != 5 || !app_packages.contains(f[0]) {
                return None;
            }
            let version = f[1].strip_prefix("0:").unwrap_or(f[1]);
            let arch = match f[2] {
                "x86_64" => deb_arch("amd64"),
                "aarch64" => deb_arch("arm64"),
                "i686" => deb_arch("i386"),
                "noarch" => deb_arch("all"),
                _ => AppArch::UNKNOWN,
            };
            Some(FoundApp {
                name: clean(f[0], MAX_APP_TEXT)?,
                version: clean(version, MAX_APP_TEXT),
                publisher: None,
                owner: Owner::Shared,
                system: AppSystem::Linux,
                found: FoundWhere::Rpm {
                    package: clean(f[0], MAX_APP_TEXT)?,
                },
                arch,
                maker: match rpm_key(f[3]) {
                    Some(key) => AppMaker::Rpm { key },
                    None => AppMaker::Unsigned,
                },
                web: super::windows::web_address(f[4]),
                update_feed: None,
                installers: vec![],
                notes: vec![],
            })
        })
        .collect()
}

/// One Flatpak app as `flatpak list --app --columns=application,version,origin,installation,name,arch`
/// prints it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FlatpakApp {
    pub app_id: String,
    pub version: Option<String>,
    pub remote: String,
    pub system_wide: bool,
    pub name: String,
    pub arch: String,
}

pub fn parse_flatpak_list(text: &str) -> Vec<FlatpakApp> {
    text.lines()
        .filter_map(|l| {
            let f: Vec<&str> = l.split('\t').collect();
            if f.len() != 6 || !pctwin_maker::linux::valid_app_id(f[0]) {
                return None;
            }
            Some(FlatpakApp {
                app_id: f[0].into(),
                version: (!f[1].is_empty()).then(|| f[1].into()),
                remote: f[2].into(),
                system_wide: f[3] == "system",
                name: f[4].into(),
                arch: f[5].into(),
            })
        })
        .collect()
}

/// `flatpak remotes --columns=name,url,options`: each remote's address, by (name, system-wide).
pub fn parse_flatpak_remotes(text: &str) -> BTreeMap<(String, bool), String> {
    text.lines()
        .filter_map(|l| {
            let f: Vec<&str> = l.split('\t').collect();
            (f.len() >= 3).then(|| {
                let system = !f[2].split(',').any(|o| o.trim() == "user");
                ((f[0].to_string(), system), f[1].to_string())
            })
        })
        .collect()
}

/// The Flatpak apps, each with its remote's address and the SHA-256 of that remote's keyring
/// (`key_of(remote, system_wide)`: `None` when it cannot be read).
pub fn flatpak_apps(
    list: &[FlatpakApp],
    remotes: &BTreeMap<(String, bool), String>,
    key_of: &dyn Fn(&str, bool) -> Option<[u8; 32]>,
    me: &Owner,
) -> Vec<FoundApp> {
    list.iter()
        .filter_map(|a| {
            let url = remotes
                .get(&(a.remote.clone(), a.system_wide))
                .and_then(|u| super::windows::web_address(u))
                .filter(|u| u.starts_with("https://") && u.len() <= MAX_APP_URL);
            let maker = match (&url, key_of(&a.remote, a.system_wide)) {
                (Some(u), Some(key)) => AppMaker::Flatpak {
                    remote_url: u.clone(),
                    app_id: a.app_id.clone(),
                    key,
                },
                (None, _) => AppMaker::NotRead {
                    why: "Its remote has no secure web address.".into(),
                },
                (_, None) => AppMaker::NotRead {
                    why: "Its remote's signing key could not be read.".into(),
                },
            };
            Some(FoundApp {
                name: clean(&a.name, MAX_APP_TEXT).or_else(|| clean(&a.app_id, MAX_APP_TEXT))?,
                version: a.version.as_deref().and_then(|v| clean(v, MAX_APP_TEXT)),
                publisher: None,
                owner: if a.system_wide {
                    Owner::Shared
                } else {
                    me.clone()
                },
                system: AppSystem::Linux,
                found: FoundWhere::Flatpak {
                    app_id: clean(&a.app_id, MAX_APP_TEXT)?,
                    remote: clean(&a.remote, MAX_APP_TEXT)?,
                    remote_url: url,
                    system_wide: a.system_wide,
                },
                arch: match a.arch.as_str() {
                    "x86_64" => deb_arch("amd64"),
                    "aarch64" => deb_arch("arm64"),
                    _ => AppArch::UNKNOWN,
                },
                maker,
                web: None,
                update_feed: None,
                installers: vec![],
                notes: vec![],
            })
        })
        .collect()
}

/// `snap list --unicode=never --color=never`: apps, leaving out the system's own parts (bases,
/// the snap system itself, kernels and gadgets), as the desktop leaves them out.
pub fn snap_apps(text: &str) -> Vec<FoundApp> {
    text.lines()
        .skip(1)
        .filter_map(|l| {
            let f: Vec<&str> = l.split_whitespace().collect();
            if f.len() < 5 {
                return None;
            }
            let notes = f.get(5).copied().unwrap_or("-");
            if notes
                .split(',')
                .any(|n| matches!(n, "base" | "core" | "snapd" | "kernel" | "gadget"))
            {
                return None;
            }
            // "canonical*" (a verified publisher), "-" (none).
            let publisher = f[4].trim_end_matches('*');
            Some(FoundApp {
                name: clean(f[0], MAX_APP_TEXT)?,
                version: clean(f[1], MAX_APP_TEXT),
                publisher: clean(publisher, MAX_APP_TEXT).filter(|p| p != "-"),
                owner: Owner::Shared,
                system: AppSystem::Linux,
                found: FoundWhere::Snap {
                    name: clean(f[0], MAX_APP_TEXT)?,
                },
                arch: AppArch::UNKNOWN,
                maker: match clean(publisher, MAX_APP_TEXT).filter(|p| p != "-") {
                    Some(p) => AppMaker::Repository {
                        origin: format!("snap store: {p}"),
                    },
                    None => AppMaker::Unsigned,
                },
                web: None,
                update_feed: None,
                installers: vec![],
                notes: vec![],
            })
        })
        .collect()
}

fn keyring_hash(remote: &str, system_wide: bool, home: &Path) -> Option<[u8; 32]> {
    if remote.contains(['/', '\\']) || remote.starts_with('.') {
        return None;
    }
    let base = if system_wide {
        PathBuf::from("/var/lib/flatpak/repo")
    } else {
        home.join(".local/share/flatpak/repo")
    };
    let p = base.join(format!("{remote}.trustedkeys.gpg"));
    let m = std::fs::symlink_metadata(&p).ok()?;
    if !m.is_file() || m.len() > MAX_FILE {
        return None;
    }
    Some(Sha256::digest(std::fs::read(&p).ok()?).into())
}

/// Every Linux app: dpkg or rpm (whichever the laptop uses), Flatpak and Snap. `me` is the
/// person PCTwin runs as (their Flatpak apps installed for them alone) and `home` their home.
pub fn read_linux_apps(me: &Owner, home: &Path, runner: &dyn Runner) -> pctwin_record::AppList {
    let mut c = Collector::default();
    let desktop = shown_desktop_files(Path::new("/usr/share/applications"));
    if Path::new("/var/lib/dpkg/status").exists() {
        match read_small(Path::new("/var/lib/dpkg/status"), MAX_FILE) {
            Some(status) => {
                let owners = dpkg_owners(Path::new("/var/lib/dpkg/info"), &desktop);
                let names: BTreeSet<&str> = owners.values().map(String::as_str).collect();
                let mut args = vec!["policy"];
                args.extend(names.iter().copied());
                let policy = runner
                    .run(&call("/usr/bin/apt-cache", &args))
                    .map(|r| parse_apt_policy(&r.stdout))
                    .unwrap_or_default();
                deb_apps(&status, &owners, &policy)
                    .into_iter()
                    .for_each(|a| c.add(a));
            }
            None => c.not_read(
                Owner::Shared,
                AppPlace::LinuxPackages,
                "The list of installed packages could not be read.".into(),
            ),
        }
    } else if Path::new("/usr/bin/rpm").exists() {
        let mut args: Vec<String> = vec!["-qf".into(), "--qf".into(), "%{NAME}\\n".into()];
        args.extend(desktop.iter().map(|p| p.to_string_lossy().into_owned()));
        let owning: BTreeSet<String> = match runner.run(&call(
            "/usr/bin/rpm",
            &args.iter().map(String::as_str).collect::<Vec<_>>(),
        )) {
            Ok(r) => r
                .stdout
                .lines()
                .filter(|l| !l.contains(' '))
                .map(str::to_string)
                .collect(),
            Err(_) => BTreeSet::new(),
        };
        match runner.run(&call("/usr/bin/rpm", &["-qa", "--qf", RPM_FORMAT])) {
            Ok(r) if r.code == Some(0) => rpm_apps(&r.stdout, &owning)
                .into_iter()
                .for_each(|a| c.add(a)),
            _ => c.not_read(
                Owner::Shared,
                AppPlace::LinuxPackages,
                "The list of installed packages could not be read.".into(),
            ),
        }
    }
    if Path::new("/usr/bin/flatpak").exists() {
        let list = runner.run(&call(
            "/usr/bin/flatpak",
            &[
                "list",
                "--app",
                "--columns=application,version,origin,installation,name,arch",
            ],
        ));
        let remotes = runner
            .run(&call(
                "/usr/bin/flatpak",
                &["remotes", "--columns=name,url,options"],
            ))
            .map(|r| parse_flatpak_remotes(&r.stdout))
            .unwrap_or_default();
        match list {
            Ok(r) if r.code == Some(0) => flatpak_apps(
                &parse_flatpak_list(&r.stdout),
                &remotes,
                &|remote, system| keyring_hash(remote, system, home),
                me,
            )
            .into_iter()
            .for_each(|a| c.add(a)),
            _ => c.not_read(
                Owner::Shared,
                AppPlace::Flatpak,
                "The list of Flatpak apps could not be read.".into(),
            ),
        }
    }
    if Path::new("/usr/bin/snap").exists() {
        match runner.run(&call(
            "/usr/bin/snap",
            &["list", "--unicode=never", "--color=never"],
        )) {
            Ok(r) if r.code == Some(0) => snap_apps(&r.stdout).into_iter().for_each(|a| c.add(a)),
            _ => c.not_read(
                Owner::Shared,
                AppPlace::Snap,
                "The list of Snaps could not be read.".into(),
            ),
        }
    }
    c.finish()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_shown_desktop_entries_are_apps() {
        assert!(desktop_shown(
            "[Desktop Entry]\nType=Application\nName=Maps\n"
        ));
        assert!(!desktop_shown(
            "[Desktop Entry]\nType=Application\nNoDisplay=true\n"
        ));
        assert!(!desktop_shown(
            "[Desktop Entry]\nType=Application\nHidden=true\n"
        ));
        assert!(!desktop_shown("[Desktop Entry]\nType=Link\n"));
        // A NoDisplay in another group is not the entry's.
        assert!(desktop_shown(
            "[Desktop Entry]\nType=Application\n[Desktop Action new]\nNoDisplay=true\n"
        ));
    }

    const STATUS: &str = "Package: viewer\nStatus: install ok installed\nVersion: 3.2-1ubuntu1\nArchitecture: amd64\nHomepage: https://viewer.example.org\nDescription: a viewer\n continued\n\nPackage: libfoo\nStatus: install ok installed\nVersion: 1.0\nArchitecture: amd64\n\nPackage: gone\nStatus: deinstall ok config-files\nVersion: 2.0\n";

    #[test]
    fn dpkg_packages_that_give_an_app_are_found_with_their_repository() {
        let pkgs = parse_dpkg_status(STATUS);
        assert_eq!(pkgs.len(), 2);
        let owners: BTreeMap<PathBuf, String> = [(
            PathBuf::from("/usr/share/applications/viewer.desktop"),
            "viewer".to_string(),
        )]
        .into();
        let policy = parse_apt_policy(
            "viewer:\n  Installed: 3.2-1ubuntu1\n  Candidate: 3.2-1ubuntu1\n  Version table:\n *** 3.2-1ubuntu1 500\n        500 http://archive.ubuntu.com/ubuntu noble/main amd64 Packages\n        100 /var/lib/dpkg/status\nlocal:\n  Installed: 1.0\n  Version table:\n *** 1.0 100\n        100 /var/lib/dpkg/status\n",
        );
        assert_eq!(
            policy.get("viewer").cloned().flatten().as_deref(),
            Some("http://archive.ubuntu.com/ubuntu noble/main")
        );
        assert_eq!(policy.get("local"), Some(&None));
        let apps = deb_apps(STATUS, &owners, &policy);
        assert_eq!(apps.len(), 1);
        assert_eq!(apps[0].name, "viewer");
        assert!(
            matches!(&apps[0].maker, AppMaker::Repository { origin } if origin.contains("noble/main"))
        );
        assert!(apps[0].arch.x64);
        assert_eq!(apps[0].web.as_deref(), Some("https://viewer.example.org/"));
    }

    #[test]
    fn dpkg_file_lists_say_which_package_owns_a_desktop_file() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            // dpkg names multi-arch lists "viewer:amd64.list"; a colon cannot be in a Windows
            // file name, so the test uses the plain form.
            dir.path().join("viewer.list"),
            "/usr/bin/viewer\n/usr/share/applications/viewer.desktop\n",
        )
        .unwrap();
        std::fs::write(dir.path().join("viewer.md5sums"), "x").unwrap();
        let owners = dpkg_owners(
            dir.path(),
            &[PathBuf::from("/usr/share/applications/viewer.desktop")],
        );
        assert_eq!(
            owners.get(Path::new("/usr/share/applications/viewer.desktop")),
            Some(&"viewer".to_string())
        );
    }

    #[test]
    fn rpm_apps_carry_their_signing_key() {
        let q = "editor\t0:5.1-2.fc41\tx86_64\tRSA/SHA256, Mon 01 Jan 2024, Key ID 809A8D7CEB10B464|(none)\thttps://editor.example.org\nlib\t0:1-1\tx86_64\t(none)|(none)\t(none)\n";
        let apps = rpm_apps(q, &["editor".to_string()].into());
        assert_eq!(apps.len(), 1);
        assert_eq!(apps[0].version.as_deref(), Some("5.1-2.fc41"));
        assert_eq!(
            apps[0].maker,
            AppMaker::Rpm {
                key: "809a8d7ceb10b464".into()
            }
        );
        assert_eq!(rpm_key("(none)|(none)"), None);
        let unsigned = rpm_apps(
            &q.replace("editor\t", "lib2\t").replace(
                "RSA/SHA256, Mon 01 Jan 2024, Key ID 809A8D7CEB10B464",
                "(none)",
            ),
            &["lib2".to_string()].into(),
        );
        assert_eq!(unsigned[0].maker, AppMaker::Unsigned);
    }

    #[test]
    fn flatpak_apps_carry_the_old_laptops_remote_and_key() {
        let list = parse_flatpak_list(
            "org.example.Maps\t3.1\tflathub\tsystem\tMaps\tx86_64\nbad id\t1\tflathub\tsystem\tX\tx86_64\norg.example.Notes\t\tmyremote\tuser\tNotes\taarch64\n",
        );
        assert_eq!(list.len(), 2);
        let remotes = parse_flatpak_remotes(
            "flathub\thttps://dl.flathub.org/repo/\tsystem\nmyremote\thttp://plain.example/\tuser\n",
        );
        let me = Owner::Person {
            account_id: "1000".into(),
        };
        // Every remote's key is readable here: only the address decides the second app.
        let apps = flatpak_apps(&list, &remotes, &|_, _| Some([4; 32]), &me);
        assert_eq!(apps.len(), 2);
        assert_eq!(
            apps[0].maker,
            AppMaker::Flatpak {
                remote_url: "https://dl.flathub.org/repo/".into(),
                app_id: "org.example.Maps".into(),
                key: [4; 32]
            }
        );
        assert_eq!(apps[0].owner, Owner::Shared);
        // A remote without a secure address cannot anchor anything: listed, with why.
        assert!(matches!(&apps[1].maker, AppMaker::NotRead { why } if why.contains("secure")));
        assert_eq!(apps[1].owner, me);
        // A remote whose key cannot be read: listed, with why.
        let no_key = flatpak_apps(&list[..1], &remotes, &|_, _| None, &me);
        assert!(matches!(&no_key[0].maker, AppMaker::NotRead { why } if why.contains("key")));
    }

    #[test]
    fn snaps_leave_out_the_systems_own_parts() {
        let out = "Name      Version   Rev    Tracking       Publisher   Notes\ncore22    20240111  1122   latest/stable  canonical*  base\nsnapd     2.61      20671  latest/stable  canonical*  snapd\nchat      1.2       33     latest/stable  chatco*     -\nlocal     0.1       x1     -              -           -\n";
        let apps = snap_apps(out);
        assert_eq!(apps.len(), 2);
        assert_eq!(apps[0].name, "chat");
        assert!(
            matches!(&apps[0].maker, AppMaker::Repository { origin } if origin == "snap store: chatco")
        );
        assert_eq!(apps[1].maker, AppMaker::Unsigned);
    }
}
