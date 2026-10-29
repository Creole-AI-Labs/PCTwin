//! Removing an app PCTwin installed, on undo (Security Design part C, decided 28 October 2026).
//!
//! - **What may be removed.** Only a Windows Installer product the helper's own install made
//!   appear: when the helper installs a package whose manifest names a product code, it reads
//!   Windows' list of installed apps before and after ([`installed_products`]), and records the
//!   product ([`recorded`]) only when the code was absent before, is there after with a version
//!   it can read, and nothing that was there before has gone (an install that replaced an earlier
//!   product, an upgrade, is never undone by removing: the earlier app would not come back). The
//!   record is the install's outcome in the helper's own administrators-only journal; the broker
//!   refuses an uninstall for any other code. Apps installed by an installer program (an EXE) are
//!   never removed: their uninstallers' switches are not reliable.
//! - **Looking first.** Just before removing, the list is read again ([`before`]): a product no
//!   longer there is already gone (nothing is run); one at another version than PCTwin installed
//!   changed since (the person updated it, or the app did) and is kept, and said.
//! - **Removing.** Windows' own `msiexec /x <code> /qn /norestart`, from System32, with the
//!   minimal environment ([`super::tool::msiexec_uninstall`]). Its exit code is read by Windows
//!   Installer's documented meanings ([`exit_meaning`]): 0 done; 3010 done, a restart finishes
//!   it; 1641 a restart was started (never asked for); 1605 and 1614 not installed; 1618 another
//!   install running; anything else failed.
//! - **Looking after.** The list is read once more ([`after`]): removed only when the product is
//!   no longer in it. An exit code alone is never taken as removal.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use super::actions::{InstalledProduct, ProductCode, ProductVersion};
use super::{Effect, NativeText, Observed};

/// Windows Installer products in the system's list of installed apps for everyone, each with its
/// version when it has a plain one.
pub type Products = BTreeMap<ProductCode, Option<ProductVersion>>;

/// The code for "Windows' list of installed apps could not be read".
pub const LIST_UNREADABLE: i64 = -3;
/// The code for "Windows Installer said it was removed, but the list still shows it".
pub const STILL_LISTED: i64 = -4;
/// Windows Installer: "This action is only valid for products that are currently installed."
pub const UNKNOWN_PRODUCT: i32 = 1605;
/// Windows Installer: "The product is uninstalled."
pub const PRODUCT_UNINSTALLED: i32 = 1614;
/// Windows Installer: another installation is already in progress.
pub const BUSY: i32 = 1618;

/// How removing an app ended, when it did not fail.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub enum UninstallResult {
    /// Removed, and no longer in Windows' list; whether a restart finishes it.
    Removed { restart_needed: bool },
    /// It was already gone: nothing was run.
    AlreadyGone,
    /// Another version is there now than PCTwin installed: kept, nothing was run.
    KeptOtherVersion,
}

/// The product `code` (from the install's manifest) to record for undo, from the list `before`
/// and `after` the install: only when it was absent before, is there after with a plain version,
/// and nothing listed before has gone. A list that could not be read records nothing.
pub fn recorded(
    code: Option<ProductCode>,
    before: Option<&Products>,
    after: Option<&Products>,
) -> Option<InstalledProduct> {
    let (code, before, after) = (code?, before?, after?);
    if before.contains_key(&code) || before.keys().any(|k| !after.contains_key(k)) {
        return None;
    }
    let version = after.get(&code)?.clone()?;
    Some(InstalledProduct { code, version })
}

/// What the list says just before removing.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Before {
    /// There, at the version PCTwin installed: it may be removed.
    Remove,
    /// Not there any more.
    Gone,
    /// There at another version (or one that cannot be read): kept.
    OtherVersion,
    /// The list could not be read: nothing is done.
    Unknown,
}

/// See [`Before`].
pub fn before(now: Option<&Products>, installed: &InstalledProduct) -> Before {
    let Some(now) = now else {
        return Before::Unknown;
    };
    match now.get(&installed.code) {
        None => Before::Gone,
        Some(Some(v)) if *v == installed.version => Before::Remove,
        Some(_) => Before::OtherVersion,
    }
}

/// What `msiexec /x` said, by Windows Installer's documented exit codes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UninstallExit {
    Done {
        restart_needed: bool,
    },
    /// The product is not installed (1605, 1614).
    NotInstalled,
    /// Another install was running (1618); nothing was changed.
    Busy,
    Failed,
}

/// See [`UninstallExit`].
pub fn exit_meaning(code: i32) -> UninstallExit {
    match code {
        0 => UninstallExit::Done {
            restart_needed: false,
        },
        // 3010: a restart finishes it. 1641: a restart was started, never asked for
        // (/norestart), so it is said, not hidden.
        3010 | 1641 => UninstallExit::Done {
            restart_needed: true,
        },
        UNKNOWN_PRODUCT | PRODUCT_UNINSTALLED => UninstallExit::NotInstalled,
        BUSY => UninstallExit::Busy,
        _ => UninstallExit::Failed,
    }
}

/// How it ended, from what `msiexec` said (`exit`, its `code`) and the list read again `now`:
/// removed only when the product `of` is no longer listed.
pub fn after(exit: UninstallExit, code: i32, now: Option<&Products>, of: &ProductCode) -> Effect {
    let native = || NativeText::new("");
    let Some(now) = now else {
        // What happened cannot be seen: never said removed.
        return Effect::Partial {
            code: LIST_UNREADABLE,
            native: native(),
        };
    };
    let gone = !now.contains_key(of);
    match (exit, gone) {
        (UninstallExit::Done { restart_needed }, true) => {
            Effect::Uninstall(UninstallResult::Removed { restart_needed })
        }
        (UninstallExit::Done { .. }, false) => Effect::Partial {
            code: STILL_LISTED,
            native: native(),
        },
        (UninstallExit::NotInstalled, true) => Effect::Uninstall(UninstallResult::AlreadyGone),
        (UninstallExit::Busy, _) => Effect::Failed {
            code: i64::from(BUSY),
            native: native(),
        },
        // It said it failed, yet the product is gone: part of it happened.
        (UninstallExit::Failed, true) => Effect::Partial {
            code: i64::from(code),
            native: native(),
        },
        (UninstallExit::NotInstalled | UninstallExit::Failed, false) => Effect::Failed {
            code: i64::from(code),
            native: native(),
        },
    }
}

/// Removes `installed` (see the module notes): `list` reads Windows' list of installed apps,
/// `run` starts `msiexec /x` for a code and gives its exit code (`None`: it was stopped).
pub fn uninstall_with(
    installed: &InstalledProduct,
    list: &mut dyn FnMut() -> Option<Products>,
    run: &mut dyn FnMut(&ProductCode) -> Result<Option<i32>, String>,
) -> Effect {
    match before(list().as_ref(), installed) {
        Before::Remove => {}
        Before::Gone => return Effect::Uninstall(UninstallResult::AlreadyGone),
        Before::OtherVersion => return Effect::Uninstall(UninstallResult::KeptOtherVersion),
        Before::Unknown => {
            return Effect::Failed {
                code: LIST_UNREADABLE,
                native: NativeText::new(""),
            };
        }
    }
    let code = match run(&installed.code) {
        Ok(Some(code)) => code,
        Ok(None) => {
            return Effect::Partial {
                code: -1,
                native: NativeText::new("stopped"),
            };
        }
        Err(e) => {
            return Effect::Failed {
                code: -1,
                native: NativeText::new(e),
            };
        }
    };
    after(exit_meaning(code), code, list().as_ref(), &installed.code)
}

/// Whether removing `code` is in place, after the helper stopped part way: unknown while
/// Windows Installer may still be working on it (a running program names it) or the lists
/// cannot be read; in place when it is no longer listed.
pub fn observed(
    code: &ProductCode,
    running: Option<Vec<String>>,
    now: Option<&Products>,
) -> Observed {
    let (Some(running), Some(now)) = (running, now) else {
        return Observed::Unknown;
    };
    if running
        .iter()
        .any(|c| c.to_ascii_uppercase().contains(code.as_str()))
    {
        return Observed::Unknown;
    }
    if now.contains_key(code) {
        Observed::Absent
    } else {
        Observed::Present
    }
}

/// The most entries read from one view of the list.
pub const MAX_ENTRIES: usize = 100_000;

/// Windows Installer products in Windows' list of installed apps for everyone (both the 64-bit
/// and the 32-bit view of `HKLM\SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall`): each entry
/// named by a product code and marked `WindowsInstaller = 1`, with its `DisplayVersion`. `None`
/// when either view cannot be read.
#[cfg(windows)]
pub fn installed_products() -> Option<Products> {
    use winreg::RegKey;
    use winreg::enums::{HKEY_LOCAL_MACHINE, KEY_READ, KEY_WOW64_32KEY, KEY_WOW64_64KEY};
    const UNINSTALL: &str = r"SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall";
    let mut out = Products::new();
    for view in [KEY_WOW64_64KEY, KEY_WOW64_32KEY] {
        let list = RegKey::predef(HKEY_LOCAL_MACHINE)
            .open_subkey_with_flags(UNINSTALL, KEY_READ | view)
            .ok()?;
        for name in list.enum_keys().take(MAX_ENTRIES) {
            let Ok(name) = name else { continue };
            let Some(code) = ProductCode::parse(&name) else {
                continue;
            };
            let Ok(entry) = list.open_subkey_with_flags(&name, KEY_READ | view) else {
                continue;
            };
            if entry.get_value::<u32, _>("WindowsInstaller").ok() != Some(1) {
                continue;
            }
            let version = entry
                .get_value::<String, _>("DisplayVersion")
                .ok()
                .and_then(|v| ProductVersion::parse(&v));
            out.entry(code).or_insert(version);
        }
    }
    Some(out)
}

#[cfg(not(windows))]
pub fn installed_products() -> Option<Products> {
    None
}

/// Starts Windows' own `msiexec /x <code> /qn /norestart` and waits for its exit code.
#[cfg(windows)]
pub fn run_msiexec(code: &ProductCode) -> Result<Option<i32>, String> {
    let mut command = super::tool::msiexec_uninstall(code).map_err(|e| format!("{e:?}"))?;
    command
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .map(|s| s.code())
        .map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    const A: &str = "{A1B2C3D4-E5F6-4789-9ABC-DEF012345678}";
    const B: &str = "{11111111-2222-3333-4444-555555555555}";

    fn code(s: &str) -> ProductCode {
        ProductCode::parse(s).unwrap()
    }

    fn v(s: &str) -> Option<ProductVersion> {
        ProductVersion::parse(s)
    }

    fn list(entries: &[(&str, Option<&str>)]) -> Products {
        entries
            .iter()
            .map(|(c, ver)| (code(c), ver.and_then(ProductVersion::parse)))
            .collect()
    }

    fn product(c: &str, ver: &str) -> InstalledProduct {
        InstalledProduct {
            code: code(c),
            version: ProductVersion::parse(ver).unwrap(),
        }
    }

    /// Step 1's acceptance: a code is recorded only for a Windows Installer product that was
    /// absent before and present after, with a version; anything else records none.
    #[test]
    fn a_product_is_recorded_only_when_this_install_made_it_appear() {
        let before = list(&[(B, Some("9"))]);
        let after = list(&[(B, Some("9")), (A, Some("2.0"))]);
        assert_eq!(
            recorded(Some(code(A)), Some(&before), Some(&after)),
            Some(product(A, "2.0"))
        );
        // No code named (an installer program, a manifest without one): nothing.
        assert_eq!(recorded(None, Some(&before), Some(&after)), None);
        // Already there before the move: never PCTwin's to remove.
        let there = list(&[(A, Some("2.0"))]);
        assert_eq!(recorded(Some(code(A)), Some(&there), Some(&after)), None);
        // Not there after: the manifest named another product, or the install did not take.
        assert_eq!(recorded(Some(code(A)), Some(&before), Some(&before)), None);
        // There, but with no version that can be checked later.
        let unversioned = list(&[(B, Some("9")), (A, None)]);
        assert_eq!(
            recorded(Some(code(A)), Some(&before), Some(&unversioned)),
            None
        );
        // Something that was there before is gone: an upgrade replaced it, so removing this one
        // would not bring that one back.
        let replaced = list(&[(A, Some("2.0"))]);
        assert_eq!(
            recorded(Some(code(A)), Some(&before), Some(&replaced)),
            None
        );
        // A list that could not be read, before or after: nothing.
        assert_eq!(recorded(Some(code(A)), None, Some(&after)), None);
        assert_eq!(recorded(Some(code(A)), Some(&before), None), None);
    }

    #[test]
    fn just_before_removing_the_list_says_whether_to() {
        let p = product(A, "2.0");
        assert_eq!(before(Some(&list(&[(A, Some("2.0"))])), &p), Before::Remove);
        assert_eq!(before(Some(&list(&[(B, Some("2.0"))])), &p), Before::Gone);
        assert_eq!(
            before(Some(&list(&[(A, Some("2.1"))])), &p),
            Before::OtherVersion
        );
        assert_eq!(before(Some(&list(&[(A, None)])), &p), Before::OtherVersion);
        assert_eq!(before(None, &p), Before::Unknown);
    }

    /// Windows Installer's documented exit codes ("MsiExec.exe and InstMsi.exe error messages").
    #[test]
    fn exit_codes_mean_what_windows_installer_documents() {
        assert_eq!(
            exit_meaning(0),
            UninstallExit::Done {
                restart_needed: false
            }
        );
        for c in [3010, 1641] {
            assert_eq!(
                exit_meaning(c),
                UninstallExit::Done {
                    restart_needed: true
                },
                "{c}"
            );
        }
        assert_eq!(exit_meaning(1605), UninstallExit::NotInstalled);
        assert_eq!(exit_meaning(1614), UninstallExit::NotInstalled);
        assert_eq!(exit_meaning(1618), UninstallExit::Busy);
        for c in [
            1, -1, 1602, 1603, 1604, 1606, 1613, 1615, 1619, 1620, 1638, 3011,
        ] {
            assert_eq!(exit_meaning(c), UninstallExit::Failed, "{c}");
        }
    }

    /// Never removed on an exit code alone: the list is read again.
    #[test]
    fn removed_only_when_the_list_no_longer_shows_it() {
        let a = code(A);
        let gone = list(&[(B, Some("1"))]);
        let still = list(&[(A, Some("2.0"))]);
        let done = UninstallExit::Done {
            restart_needed: false,
        };
        let restart = UninstallExit::Done {
            restart_needed: true,
        };
        assert_eq!(
            after(done, 0, Some(&gone), &a),
            Effect::Uninstall(UninstallResult::Removed {
                restart_needed: false
            })
        );
        assert_eq!(
            after(restart, 3010, Some(&gone), &a),
            Effect::Uninstall(UninstallResult::Removed {
                restart_needed: true
            })
        );
        assert!(matches!(
            after(done, 0, Some(&still), &a),
            Effect::Partial {
                code: STILL_LISTED,
                ..
            }
        ));
        assert!(matches!(
            after(done, 0, None, &a),
            Effect::Partial {
                code: LIST_UNREADABLE,
                ..
            }
        ));
        assert_eq!(
            after(UninstallExit::NotInstalled, 1605, Some(&gone), &a),
            Effect::Uninstall(UninstallResult::AlreadyGone)
        );
        assert!(matches!(
            after(UninstallExit::NotInstalled, 1605, Some(&still), &a),
            Effect::Failed { code: 1605, .. }
        ));
        assert!(matches!(
            after(UninstallExit::Busy, 1618, Some(&still), &a),
            Effect::Failed { code: 1618, .. }
        ));
        assert!(matches!(
            after(UninstallExit::Busy, 1618, Some(&gone), &a),
            Effect::Failed { code: 1618, .. }
        ));
        assert!(matches!(
            after(UninstallExit::Failed, 1603, Some(&still), &a),
            Effect::Failed { code: 1603, .. }
        ));
        assert!(matches!(
            after(UninstallExit::Failed, 1603, Some(&gone), &a),
            Effect::Partial { code: 1603, .. }
        ));
    }

    /// The whole removal: looked at, run only when the version is PCTwin's, looked at again.
    #[test]
    fn msiexec_runs_only_for_the_version_pctwin_installed() {
        let p = product(A, "2.0");
        let mut ran: Vec<ProductCode> = Vec::new();
        let mut lists = vec![Some(list(&[])), Some(list(&[(A, Some("2.0"))]))];
        let got = uninstall_with(&p, &mut || lists.pop().flatten(), &mut |c| {
            ran.push(*c);
            Ok(Some(0))
        });
        assert_eq!(
            got,
            Effect::Uninstall(UninstallResult::Removed {
                restart_needed: false
            })
        );
        assert_eq!(ran, [code(A)]);

        for (now, want) in [
            (
                Some(list(&[(A, Some("2.1"))])),
                Effect::Uninstall(UninstallResult::KeptOtherVersion),
            ),
            (
                Some(list(&[])),
                Effect::Uninstall(UninstallResult::AlreadyGone),
            ),
            (
                None,
                Effect::Failed {
                    code: LIST_UNREADABLE,
                    native: NativeText::new(""),
                },
            ),
        ] {
            let mut ran = 0;
            let mut once = Some(now);
            let got = uninstall_with(&p, &mut || once.take().flatten(), &mut |_| {
                ran += 1;
                Ok(Some(0))
            });
            assert_eq!(got, want);
            assert_eq!(ran, 0, "{want:?}");
        }

        // Could not start, or was stopped: never removed.
        let there = || Some(list(&[(A, Some("2.0"))]));
        assert!(matches!(
            uninstall_with(&p, &mut there.clone(), &mut |_| Err("no".into())),
            Effect::Failed { code: -1, .. }
        ));
        assert!(matches!(
            uninstall_with(&p, &mut there.clone(), &mut |_| Ok(None)),
            Effect::Partial { code: -1, .. }
        ));
        // It said done but is still listed.
        assert!(matches!(
            uninstall_with(&p, &mut there.clone(), &mut |_| Ok(Some(0))),
            Effect::Partial {
                code: STILL_LISTED,
                ..
            }
        ));
    }

    #[test]
    fn after_a_crash_removal_is_looked_at_not_guessed() {
        let a = code(A);
        let gone = list(&[]);
        let still = list(&[(A, Some("2.0"))]);
        assert_eq!(observed(&a, Some(vec![]), Some(&gone)), Observed::Present);
        assert_eq!(observed(&a, Some(vec![]), Some(&still)), Observed::Absent);
        let busy = vec![format!(
            r"C:\Windows\System32\msiexec.exe /x {} /qn",
            A.to_lowercase()
        )];
        assert_eq!(observed(&a, Some(busy), Some(&gone)), Observed::Unknown);
        assert_eq!(observed(&a, None, Some(&gone)), Observed::Unknown);
        assert_eq!(observed(&a, Some(vec![]), None), Observed::Unknown);
    }

    #[test]
    fn a_result_crosses_in_its_own_words() {
        for r in [
            UninstallResult::Removed {
                restart_needed: true,
            },
            UninstallResult::AlreadyGone,
            UninstallResult::KeptOtherVersion,
        ] {
            let json = serde_json::to_string(&r).unwrap();
            assert_eq!(serde_json::from_str::<UninstallResult>(&json).unwrap(), r);
        }
        for bad in [
            r#"{"removed":{}}"#,
            r#"{"removed":{"restart_needed":true,"x":1}}"#,
            r#""removed""#,
            r#""gone""#,
        ] {
            assert!(
                serde_json::from_str::<UninstallResult>(bad).is_err(),
                "{bad}"
            );
        }
    }

    #[test]
    fn versions_are_compared_whole() {
        assert_ne!(v("2.0"), v("2.0.0"));
        assert_eq!(v("2.0"), v(" 2.0"));
    }
}
