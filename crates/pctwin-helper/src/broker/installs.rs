//! The installs the helper makes for 2.1, and how it tells afterwards whether one happened.
//!
//! - **Installing.** The helper's own held copy is given to Windows Installer (`msiexec`, fixed
//!   switches) or, for an installer program of a known kind, started itself with that kind's
//!   fixed switches ([`super::tool::exe_install`]). The laptop is never restarted: a needed
//!   restart is reported. The exit code is read by the installer kind's documented meanings
//!   ([`super::tool::meaning`]).
//! - **Looking afterwards** (after a crash, before anything is tried again: T21-041, T21-042).
//!   First, whether an installer from the helper's own folder may still be running (Windows'
//!   process list, through its management service): then nothing is said but "unknown", and the
//!   broker asks for recovery instead of acting. Then, for a Windows Installer package: every
//!   product Windows Installer installs records the name of the package it came from
//!   (`SourceList\PackageName`), and the helper's package names begin with half of the
//!   package's SHA-256 ([`super::staging::staged_prefix`]), so Windows' own list says whether
//!   these exact bytes were installed. An installer program leaves no such record: unknown.
//! - **The Mac App Store**: `mas install <id>` as root, only from a `mas` that root owns and
//!   nobody else can change (every folder above it too): `mas` from a folder the person owns
//!   (Homebrew's) is never run as root. Otherwise it is not available, and the app sends the
//!   person to the App Store's own page instead.

use super::actions::{InstallFromMacAppStore, InstallVerifiedPackage, PackageKind};
use super::staging::{VerifiedArtifact, staged_prefix};
use super::tool::{ExitMeaning, exe_install, meaning, msiexec_install};
use super::{Effect, NOT_AVAILABLE, NativeText, Observed};

/// The code for "another install was running" (Windows Installer's own, 1618).
pub const BUSY: i64 = 1618;
/// The code for "the installer was cancelled".
pub const CANCELLED: i64 = 1602;

/// Installs `package` for `action`: never restarting the laptop.
pub fn install(package: &VerifiedArtifact, action: &InstallVerifiedPackage) -> Effect {
    let (command, kind) = match action.kind {
        PackageKind::Msi => (msiexec_install(package, action.scope), None),
        PackageKind::Exe(k) => (exe_install(package, k, action.scope), Some(k)),
    };
    let Ok(mut command) = command else {
        return Effect::Failed {
            code: NOT_AVAILABLE,
            native: NativeText::new(""),
        };
    };
    let code = match command.status() {
        Ok(status) => status.code(),
        Err(e) => {
            return Effect::Failed {
                code: -1,
                native: NativeText::new(e.to_string()),
            };
        }
    };
    let Some(code) = code else {
        return Effect::Failed {
            code: -1,
            native: NativeText::new("stopped"),
        };
    };
    match meaning(kind, code) {
        ExitMeaning::Done { restart_needed } => Effect::Done { restart_needed },
        ExitMeaning::Busy => Effect::Failed {
            code: BUSY,
            native: NativeText::new(""),
        },
        ExitMeaning::Cancelled => Effect::Failed {
            code: CANCELLED,
            native: NativeText::new(""),
        },
        ExitMeaning::Failed => Effect::Failed {
            code: i64::from(code),
            native: NativeText::new(""),
        },
    }
}

/// Whether the product of `action` is to be watched: a Windows Installer package whose
/// manifest named a product code.
pub fn watches_product(action: &InstallVerifiedPackage) -> bool {
    action.kind == PackageKind::Msi && action.product_code.is_some()
}

/// An install's effect with its product recorded for undo, when the install was done and
/// [`super::uninstall::recorded`] finds this install made the product appear (`before`: the list
/// read just before the install; `after` reads it again). Anything else is the effect as it was.
pub fn with_product(
    effect: Effect,
    action: &InstallVerifiedPackage,
    before: Option<&super::uninstall::Products>,
    after: &mut dyn FnMut() -> Option<super::uninstall::Products>,
) -> Effect {
    let Effect::Done { restart_needed } = effect else {
        return effect;
    };
    if !watches_product(action) {
        return effect;
    }
    match super::uninstall::recorded(action.product_code, before, after().as_ref()) {
        Some(product) => Effect::Installed {
            restart_needed,
            product,
        },
        None => effect,
    }
}

/// What Windows says is there, for the decision after a crash (see the module notes):
/// `running` is the command lines of the processes now running, `packages` the package names
/// Windows Installer recorded for its products.
pub fn observed(
    action: &InstallVerifiedPackage,
    running: Option<Vec<String>>,
    packages: Option<Vec<String>>,
) -> Observed {
    let prefix = staged_prefix(&action.sha256.0).to_ascii_lowercase();
    let Some(running) = running else {
        return Observed::Unknown;
    };
    // Anything of the helper's own package still running: an installer started from its
    // folder, or Windows Installer working on it.
    if running
        .iter()
        .any(|c| c.to_ascii_lowercase().contains(&prefix))
    {
        return Observed::Unknown;
    }
    match action.kind {
        PackageKind::Msi => match packages {
            Some(names)
                if names
                    .iter()
                    .any(|n| n.to_ascii_lowercase().starts_with(&prefix)) =>
            {
                Observed::Present
            }
            Some(_) => Observed::Absent,
            None => Observed::Unknown,
        },
        PackageKind::Exe(_) => Observed::Unknown,
    }
}

/// The command lines of the processes running now (Windows' management service).
#[cfg(windows)]
pub fn running_command_lines() -> Option<Vec<String>> {
    #[derive(serde::Deserialize)]
    #[serde(rename = "Win32_Process")]
    #[serde(rename_all = "PascalCase")]
    struct Process {
        command_line: Option<String>,
        executable_path: Option<String>,
    }
    let wmi = wmi::WMIConnection::new().ok()?;
    let all: Vec<Process> = wmi.query().ok()?;
    Some(
        all.into_iter()
            .flat_map(|p| [p.command_line, p.executable_path])
            .flatten()
            .collect(),
    )
}

/// The package names Windows Installer recorded for the products installed for everyone.
#[cfg(windows)]
pub fn installed_package_names() -> Option<Vec<String>> {
    use winreg::RegKey;
    use winreg::enums::{HKEY_LOCAL_MACHINE, KEY_READ, KEY_WOW64_64KEY};
    let products = RegKey::predef(HKEY_LOCAL_MACHINE)
        .open_subkey_with_flags(
            r"SOFTWARE\Classes\Installer\Products",
            KEY_READ | KEY_WOW64_64KEY,
        )
        .ok()?;
    let mut out = Vec::new();
    for name in products.enum_keys().take(100_000) {
        let Ok(name) = name else { continue };
        if let Ok(list) = products
            .open_subkey_with_flags(format!(r"{name}\SourceList"), KEY_READ | KEY_WOW64_64KEY)
            && let Ok(package) = list.get_value::<String, _>("PackageName")
        {
            out.push(package);
        }
    }
    Some(out)
}

/// Where `mas` may be, in order.
pub const MAS_PLACES: &[&str] = &["/usr/local/bin/mas", "/opt/homebrew/bin/mas"];

/// One step of a path, as the check needs it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Owned {
    pub uid: u32,
    /// Group or others may write it.
    pub writable_by_others: bool,
    pub is_link: bool,
}

/// Whether a program at `path` may be run as root: it and every folder above it are owned by
/// root, are not links, and only root may write them (`meta` reads each step).
pub fn root_only(path: &std::path::Path, meta: &dyn Fn(&std::path::Path) -> Option<Owned>) -> bool {
    path.has_root()
        && path.ancestors().all(|step| {
            meta(step).is_some_and(|m| m.uid == 0 && !m.writable_by_others && !m.is_link)
        })
}

#[cfg(unix)]
fn owned(path: &std::path::Path) -> Option<Owned> {
    use std::os::unix::fs::MetadataExt;
    let m = std::fs::symlink_metadata(path).ok()?;
    Some(Owned {
        uid: m.uid(),
        writable_by_others: m.mode() & 0o022 != 0,
        is_link: m.file_type().is_symlink(),
    })
}

/// Installs one Mac App Store app with a root-owned `mas` (see the module notes).
pub fn install_from_store(action: &InstallFromMacAppStore) -> Effect {
    #[cfg(target_os = "macos")]
    {
        let Some(mas) = MAS_PLACES
            .iter()
            .map(std::path::Path::new)
            .find(|p| root_only(p, &owned))
        else {
            return Effect::Failed {
                code: NOT_AVAILABLE,
                native: NativeText::new("no root-owned mas"),
            };
        };
        let status = pctwin_types::process::quiet(mas)
            .arg("install")
            .arg(action.adam_id.to_string())
            .env_clear()
            .env("PATH", "/usr/bin:/bin:/usr/sbin:/sbin")
            .current_dir("/")
            .stdin(std::process::Stdio::null())
            .status();
        match status.map(|s| s.code()) {
            Ok(Some(0)) => Effect::Done {
                restart_needed: false,
            },
            Ok(code) => Effect::Failed {
                code: i64::from(code.unwrap_or(-1)),
                native: NativeText::new(""),
            },
            Err(e) => Effect::Failed {
                code: -1,
                native: NativeText::new(e.to_string()),
            },
        }
    }
    #[cfg(not(target_os = "macos"))]
    {
        #[cfg(unix)]
        let _ = owned;
        let _ = action;
        Effect::Failed {
            code: NOT_AVAILABLE,
            native: NativeText::new(""),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::broker::actions::{ExeKind, Hex32, InstallScope};
    use std::path::Path;

    fn action(kind: PackageKind) -> InstallVerifiedPackage {
        InstallVerifiedPackage {
            product_code: None,
            artifact: 1,
            sha256: Hex32([0xab; 32]),
            kind,
            scope: InstallScope::Machine,
            maker: Box::new(pctwin_maker::facts::WindowsSigner {
                subject: Default::default(),
                certificate: pctwin_maker::facts::Sha256([0; 32]),
                issuer: String::new(),
                issuer_organization: None,
                public_key: None,
            }),
        }
    }

    #[test]
    fn windows_own_list_says_whether_these_exact_bytes_were_installed() {
        let msi = action(PackageKind::Msi);
        let ours = format!("{}0123456789abcdef.pkg", staged_prefix(&[0xab; 32]));
        assert_eq!(
            observed(
                &msi,
                Some(vec![]),
                Some(vec!["other.msi".into(), ours.to_uppercase()])
            ),
            Observed::Present
        );
        assert_eq!(
            observed(&msi, Some(vec![]), Some(vec!["other.msi".into()])),
            Observed::Absent
        );
        // Another package's name (other bytes) is not this one.
        let theirs = format!("{}x.pkg", staged_prefix(&[0xac; 32]));
        assert_eq!(
            observed(&msi, Some(vec![]), Some(vec![theirs])),
            Observed::Absent
        );
        // Still running, or a list that could not be read: unknown, never "absent".
        let running = vec![format!(
            r"C:\Windows\System32\msiexec.exe /i C:\x\{ours} /qn"
        )];
        assert_eq!(
            observed(&msi, Some(running), Some(vec![])),
            Observed::Unknown
        );
        assert_eq!(
            observed(&msi, None, Some(vec![ours.clone()])),
            Observed::Unknown
        );
        assert_eq!(observed(&msi, Some(vec![]), None), Observed::Unknown);
        // An installer program leaves no record of its package: unknown.
        assert_eq!(
            observed(
                &action(PackageKind::Exe(ExeKind::Inno)),
                Some(vec![]),
                Some(vec![ours])
            ),
            Observed::Unknown
        );
    }

    /// Step 1's acceptance at the helper: the product is recorded only for a Windows Installer
    /// install that was done and made it appear; an installer program (EXE) records none.
    #[test]
    fn only_a_done_msi_install_that_made_its_product_appear_records_it() {
        use crate::broker::actions::{ProductCode, ProductVersion};
        use crate::broker::uninstall::Products;
        let code = ProductCode::parse("{A1B2C3D4-E5F6-4789-9ABC-DEF012345678}").unwrap();
        let before = Products::new();
        let appeared = || {
            let mut p = Products::new();
            p.insert(code, ProductVersion::parse("2.0"));
            Some(p)
        };
        let done = Effect::Done {
            restart_needed: true,
        };
        let mut msi = action(PackageKind::Msi);
        msi.product_code = Some(code);
        let mut looked = 0;
        let got = with_product(done.clone(), &msi, Some(&before), &mut || {
            looked += 1;
            appeared()
        });
        assert_eq!(
            got,
            Effect::Installed {
                restart_needed: true,
                product: crate::broker::actions::InstalledProduct {
                    code,
                    version: ProductVersion::parse("2.0").unwrap(),
                },
            }
        );
        assert_eq!(looked, 1);
        // Not appeared (the list after has nothing new): done, nothing recorded.
        assert_eq!(
            with_product(done.clone(), &msi, Some(&before), &mut || Some(
                Products::new()
            )),
            done
        );
        // Already there before.
        assert_eq!(
            with_product(
                done.clone(),
                &msi,
                appeared().as_ref(),
                &mut appeared.clone()
            ),
            done
        );
        // A failed install records nothing and does not look.
        let failed = Effect::Failed {
            code: 1603,
            native: NativeText::new(""),
        };
        let mut looked = 0;
        assert_eq!(
            with_product(failed.clone(), &msi, Some(&before), &mut || {
                looked += 1;
                appeared()
            }),
            failed
        );
        assert_eq!(looked, 0);
        // An installer program never records a product, even if its manifest names one.
        let mut exe = action(PackageKind::Exe(ExeKind::Inno));
        exe.product_code = Some(code);
        assert!(!watches_product(&exe));
        assert_eq!(
            with_product(done.clone(), &exe, Some(&before), &mut appeared.clone()),
            done
        );
        // A package with no product code named: nothing to watch.
        let unnamed = action(PackageKind::Msi);
        assert!(!watches_product(&unnamed));
        assert_eq!(
            with_product(done.clone(), &unnamed, Some(&before), &mut appeared.clone()),
            done
        );
        assert!(watches_product(&msi));
    }

    #[test]
    fn mas_runs_as_root_only_when_nobody_else_can_change_it() {
        let root = Owned {
            uid: 0,
            writable_by_others: false,
            is_link: false,
        };
        let all_root = |_: &Path| Some(root);
        assert!(root_only(Path::new("/usr/local/bin/mas"), &all_root));
        // Homebrew's folder belongs to the person.
        let brew = |p: &Path| {
            Some(if p.starts_with("/opt/homebrew") {
                Owned { uid: 501, ..root }
            } else {
                root
            })
        };
        assert!(!root_only(Path::new("/opt/homebrew/bin/mas"), &brew));
        let writable = |p: &Path| {
            Some(if p == Path::new("/usr/local/bin") {
                Owned {
                    writable_by_others: true,
                    ..root
                }
            } else {
                root
            })
        };
        assert!(!root_only(Path::new("/usr/local/bin/mas"), &writable));
        let link = |p: &Path| {
            Some(Owned {
                is_link: p.ends_with("mas"),
                ..root
            })
        };
        assert!(!root_only(Path::new("/usr/local/bin/mas"), &link));
        assert!(!root_only(Path::new("/usr/local/bin/mas"), &|_| None));
        assert!(!root_only(Path::new("mas"), &all_root));
    }
}
