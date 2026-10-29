//! Removing an app PCTwin installed, on undo (Security Design part C, decided 28 October 2026;
//! fresh-context review of 29 October 2026, findings F1 to F6).
//!
//! - **What may be removed.** Only a Windows Installer product the helper's own install made
//!   appear, for one move: when the helper installs a package for a move, and the manifest names
//!   a product code that the package's own Property table names too ([`product_code_of`], read
//!   from the helper's held, hash-pinned copy), it reads Windows' list of installed apps before
//!   and after ([`installed_products`]), and records the product ([`recorded`]) only when the
//!   code was absent before, is there after with a version and an install date it can read, and
//!   nothing that was there before has gone (an install that replaced an earlier product, an
//!   upgrade, is never undone by removing: the earlier app would not come back). The record is
//!   the install's outcome in the helper's own administrators-only journal, with the move it
//!   belongs to; the broker refuses an uninstall for any other code or move. Apps installed by an
//!   installer program (an EXE) are never removed: their uninstallers' switches are not reliable.
//! - **Once.** When a removal ends (removed, already gone, or kept because the app changed), the
//!   broker writes the install's record as used ([`consumed_key`]) and refuses it from then on: a
//!   copy of the app the person installs again later is theirs, never removed by an old undo.
//! - **Looking first.** Just before removing, the list is read again ([`before`]): a product no
//!   longer there is already gone (nothing is run); one at another version, or written into the
//!   list on another day (installed again since), changed and is kept, and said.
//! - **Removing.** Windows' own `msiexec /x <code> /qn /norestart`, from System32, with the
//!   minimal environment ([`super::tool::msiexec_uninstall`]). Its exit code is read by Windows
//!   Installer's documented meanings ([`exit_meaning`]): 0 done; 3010 done, a restart finishes
//!   it; 1641 a restart was started (never asked for); 1605 and 1614 not installed; 1618 another
//!   install running; anything else failed.
//! - **Looking after.** The list is read once more ([`after`]): removed only when the product is
//!   no longer in it. An exit code alone is never taken as removal, and a list that could not be
//!   read whole is never taken as "gone" ([`collect_view`]).

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use super::actions::{Hex32, InstallDate, InstalledProduct, ProductCode, ProductVersion};
use super::{Effect, NativeText, Observed};

/// What Windows' list says of one Windows Installer product.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Listed {
    pub version: Option<ProductVersion>,
    pub installed_on: Option<InstallDate>,
}

/// Windows Installer products in the system's list of installed apps for everyone.
pub type Products = BTreeMap<ProductCode, Listed>;

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
    /// It changed since PCTwin installed it (another version, or installed again): kept,
    /// nothing was run.
    KeptOtherVersion,
}

/// The key under which the helper's journal marks the install `installed_by` as used by a
/// removal that ended: from then on it is never removed again.
pub fn consumed_key(installed_by: &Hex32) -> Hex32 {
    Hex32(blake3::derive_key(
        "PCTwin uninstall consumed 2026-10-29 v1",
        &installed_by.0,
    ))
}

/// The product `code` (from the install's manifest) to record for undo: only when the package's
/// own Property table names the same code (`package_code`), and, from the list `before` and
/// `after` the install, it was absent before, is there after with a plain version and install
/// date, and nothing listed before has gone. A list that could not be read records nothing.
pub fn recorded(
    code: Option<ProductCode>,
    package_code: Option<ProductCode>,
    before: Option<&Products>,
    after: Option<&Products>,
) -> Option<InstalledProduct> {
    let (code, before, after) = (code?, before?, after?);
    if package_code != Some(code)
        || before.contains_key(&code)
        || before.keys().any(|k| !after.contains_key(k))
    {
        return None;
    }
    let listed = after.get(&code)?;
    Some(InstalledProduct {
        code,
        version: listed.version.clone()?,
        installed_on: listed.installed_on.clone()?,
    })
}

/// What the list says just before removing.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Before {
    /// There, as PCTwin installed it: it may be removed.
    Remove,
    /// Not there any more.
    Gone,
    /// There at another version, or written on another day (installed again), or either cannot
    /// be read: kept.
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
        Some(l)
            if l.version.as_ref() == Some(&installed.version)
                && l.installed_on.as_ref() == Some(&installed.installed_on) =>
        {
            Before::Remove
        }
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

/// Whether removing a product is in place, after the helper stopped part way. Windows
/// Installer's own work goes on in its service after the program that asked for it is gone, so
/// while any Windows Installer program runs (`installer_running`), or the two places Windows
/// keeps a product disagree (`listed`: in the list of installed apps; `registered`: under
/// `Installer\Products`), or either cannot be read, nothing is said but "unknown". In place only
/// when both say gone; not in place only when both say there and nothing is running.
pub fn observed(
    installer_running: Option<bool>,
    listed: Option<bool>,
    registered: Option<bool>,
) -> Observed {
    match (installer_running, listed, registered) {
        (Some(false), Some(false), Some(false)) => Observed::Present,
        (Some(false), Some(true), Some(true)) => Observed::Absent,
        _ => Observed::Unknown,
    }
}

/// Whether any of `running` (command lines and program paths) is Windows Installer's program.
pub fn installer_among(running: Option<Vec<String>>) -> Option<bool> {
    Some(
        running?
            .iter()
            .any(|c| c.to_ascii_lowercase().contains("msiexec")),
    )
}

/// The most entries read from one view of the list.
pub const MAX_ENTRIES: usize = 100_000;

/// What one entry of the list holds, as read.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct EntryRead {
    pub windows_installer: Option<u32>,
    pub version: Option<String>,
    pub installed_on: Option<String>,
}

/// Adds one view of the list to `out`: `names` are its entries' names (an `Err` is a name that
/// could not be read), `open` reads an entry (`None`: it could not be opened). Only entries named
/// by a product code and marked `WindowsInstaller = 1` are kept. A name that cannot be read, an
/// entry named by a product code that cannot be opened, or more than [`MAX_ENTRIES`] entries make
/// the whole list unknown (`None`): a product is never taken as gone because it could not be
/// seen.
pub fn collect_view(
    out: &mut Products,
    names: &mut dyn Iterator<Item = Result<String, ()>>,
    open: &mut dyn FnMut(&str) -> Option<EntryRead>,
) -> Option<()> {
    for (n, name) in names.enumerate() {
        if n >= MAX_ENTRIES {
            return None;
        }
        let name = name.ok()?;
        let Some(code) = ProductCode::parse(&name) else {
            continue;
        };
        let entry = open(&name)?;
        if entry.windows_installer != Some(1) {
            continue;
        }
        out.entry(code).or_insert(Listed {
            version: entry.version.as_deref().and_then(ProductVersion::parse),
            installed_on: entry.installed_on.as_deref().and_then(InstallDate::parse),
        });
    }
    Some(())
}

/// Windows Installer products in Windows' list of installed apps for everyone (both the 64-bit
/// and the 32-bit view of `HKLM\SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall`), each with
/// its `DisplayVersion` and `InstallDate`. `None` when the list cannot be read whole.
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
        let mut names = list.enum_keys().map(|n| n.map_err(|_| ()));
        collect_view(&mut out, &mut names, &mut |name| {
            let entry = list.open_subkey_with_flags(name, KEY_READ | view).ok()?;
            Some(EntryRead {
                windows_installer: entry.get_value::<u32, _>("WindowsInstaller").ok(),
                version: entry.get_value::<String, _>("DisplayVersion").ok(),
                installed_on: entry.get_value::<String, _>("InstallDate").ok(),
            })
        })?;
    }
    Some(out)
}

#[cfg(not(windows))]
pub fn installed_products() -> Option<Products> {
    None
}

/// Whether Windows Installer still keeps `code` for everyone (`HKLM\SOFTWARE\Classes\Installer\
/// Products\<packed code>`): `None` when that cannot be told.
#[cfg(windows)]
pub fn registered(code: &ProductCode) -> Option<bool> {
    use winreg::RegKey;
    use winreg::enums::{HKEY_LOCAL_MACHINE, KEY_READ, KEY_WOW64_64KEY};
    let path = format!(r"SOFTWARE\Classes\Installer\Products\{}", code.packed());
    match RegKey::predef(HKEY_LOCAL_MACHINE)
        .open_subkey_with_flags(path, KEY_READ | KEY_WOW64_64KEY)
    {
        Ok(_) => Some(true),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Some(false),
        Err(_) => None,
    }
}

/// The product code a Windows Installer package names in its own Property table, read from
/// `file` (the helper's held, hash-pinned copy). `None` when it is not a package, has no such
/// row, or the value is not a product code.
pub fn product_code_of<F: std::io::Read + std::io::Seek>(file: F) -> Option<ProductCode> {
    use msi::{Expr, Select};
    let mut package = msi::Package::open(file).ok()?;
    let query = Select::table("Property")
        .columns(&["Value"])
        .with(Expr::col("Property").eq(Expr::string("ProductCode")));
    let rows: Vec<_> = package.select_rows(query).ok()?.collect();
    let [row] = rows.as_slice() else {
        return None;
    };
    match &row[0] {
        msi::Value::Str(s) => ProductCode::parse(s),
        _ => None,
    }
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
    const DAY: &str = "20261028";

    fn code(s: &str) -> ProductCode {
        ProductCode::parse(s).unwrap()
    }

    fn at(version: Option<&str>, day: Option<&str>) -> Listed {
        Listed {
            version: version.and_then(ProductVersion::parse),
            installed_on: day.and_then(InstallDate::parse),
        }
    }

    fn list(entries: &[(&str, Option<&str>)]) -> Products {
        entries
            .iter()
            .map(|(c, ver)| (code(c), at(*ver, Some(DAY))))
            .collect()
    }

    fn product(c: &str, ver: &str) -> InstalledProduct {
        InstalledProduct {
            code: code(c),
            version: ProductVersion::parse(ver).unwrap(),
            installed_on: InstallDate::parse(DAY).unwrap(),
        }
    }

    /// A Windows Installer package with `code` in its Property table, made in memory.
    fn package_naming(code: Option<&str>) -> std::io::Cursor<Vec<u8>> {
        use msi::{Column, Insert, PackageType, Value};
        let mut p =
            msi::Package::create(PackageType::Installer, std::io::Cursor::new(Vec::new())).unwrap();
        p.create_table(
            "Property",
            vec![
                Column::build("Property").primary_key().id_string(72),
                Column::build("Value").text_string(0),
            ],
        )
        .unwrap();
        let mut rows =
            Insert::into("Property").row(vec![Value::from("ProductName"), Value::from("Example")]);
        if let Some(code) = code {
            rows = rows.row(vec![Value::from("ProductCode"), Value::from(code)]);
        }
        p.insert_rows(rows).unwrap();
        let mut cursor = p.into_inner().unwrap();
        cursor.set_position(0);
        cursor
    }

    /// F4: the code is read from the package's own Property table; anything else is none.
    #[test]
    fn the_packages_own_product_code_is_read_from_its_property_table() {
        assert_eq!(product_code_of(package_naming(Some(A))), Some(code(A)));
        assert_eq!(
            product_code_of(package_naming(Some(&A.to_lowercase()))),
            Some(code(A))
        );
        assert_eq!(product_code_of(package_naming(None)), None);
        assert_eq!(product_code_of(package_naming(Some("not a code"))), None);
        assert_eq!(
            product_code_of(std::io::Cursor::new(b"MZ not a package".to_vec())),
            None
        );
    }

    /// Step 1's acceptance: a code is recorded only for a Windows Installer product the package
    /// itself names, that was absent before and present after, with a version and a day.
    #[test]
    fn a_product_is_recorded_only_when_this_install_made_it_appear() {
        let before = list(&[(B, Some("9"))]);
        let after = list(&[(B, Some("9")), (A, Some("2.0"))]);
        let a = Some(code(A));
        assert_eq!(
            recorded(a, a, Some(&before), Some(&after)),
            Some(product(A, "2.0"))
        );
        // No code named (an installer program, a manifest without one): nothing.
        assert_eq!(recorded(None, a, Some(&before), Some(&after)), None);
        // The package names another code, or none (F4).
        assert_eq!(
            recorded(a, Some(code(B)), Some(&before), Some(&after)),
            None
        );
        assert_eq!(recorded(a, None, Some(&before), Some(&after)), None);
        // Already there before the move: never PCTwin's to remove.
        let there = list(&[(A, Some("2.0"))]);
        assert_eq!(recorded(a, a, Some(&there), Some(&after)), None);
        // Not there after: the manifest named another product, or the install did not take.
        assert_eq!(recorded(a, a, Some(&before), Some(&before)), None);
        // There, but with no version, or no day, that can be checked later.
        let mut unversioned = before.clone();
        unversioned.insert(code(A), at(None, Some(DAY)));
        assert_eq!(recorded(a, a, Some(&before), Some(&unversioned)), None);
        let mut undated = before.clone();
        undated.insert(code(A), at(Some("2.0"), None));
        assert_eq!(recorded(a, a, Some(&before), Some(&undated)), None);
        // Something that was there before is gone: an upgrade replaced it, so removing this one
        // would not bring that one back.
        let replaced = list(&[(A, Some("2.0"))]);
        assert_eq!(recorded(a, a, Some(&before), Some(&replaced)), None);
        // A list that could not be read, before or after: nothing.
        assert_eq!(recorded(a, a, None, Some(&after)), None);
        assert_eq!(recorded(a, a, Some(&before), None), None);
    }

    /// F1: just before removing, the product must be as PCTwin installed it: the same version,
    /// written on the same day. One installed again since is the person's, and kept.
    #[test]
    fn just_before_removing_the_list_says_whether_to() {
        let p = product(A, "2.0");
        let one = |l: Listed| {
            let mut m = Products::new();
            m.insert(code(A), l);
            m
        };
        assert_eq!(
            before(Some(&one(at(Some("2.0"), Some(DAY)))), &p),
            Before::Remove
        );
        assert_eq!(before(Some(&list(&[(B, Some("2.0"))])), &p), Before::Gone);
        assert_eq!(
            before(Some(&one(at(Some("2.1"), Some(DAY)))), &p),
            Before::OtherVersion
        );
        assert_eq!(
            before(Some(&one(at(Some("2.0"), Some("20261105")))), &p),
            Before::OtherVersion
        );
        assert_eq!(
            before(Some(&one(at(None, Some(DAY)))), &p),
            Before::OtherVersion
        );
        assert_eq!(
            before(Some(&one(at(Some("2.0"), None))), &p),
            Before::OtherVersion
        );
        assert_eq!(before(None, &p), Before::Unknown);
    }

    /// F5: a name or an entry that cannot be read, or too many entries, make the list unknown,
    /// never "gone".
    #[test]
    fn a_list_that_cannot_be_read_whole_is_unknown_not_empty() {
        let msi = |v: &str| EntryRead {
            windows_installer: Some(1),
            version: Some(v.into()),
            installed_on: Some(DAY.into()),
        };
        let mut out = Products::new();
        let mut names =
            vec![Ok(A.to_string()), Ok("NotACode".into()), Ok(B.to_string())].into_iter();
        let got = collect_view(&mut out, &mut names, &mut |n| {
            Some(if n == B {
                EntryRead {
                    windows_installer: None,
                    ..msi("1")
                }
            } else {
                msi("2.0")
            })
        });
        assert_eq!(got, Some(()));
        // Only the Windows Installer product named by a code.
        assert_eq!(out.len(), 1);
        assert_eq!(out[&code(A)], at(Some("2.0"), Some(DAY)));

        // A name that cannot be read.
        let mut names = vec![Ok(A.to_string()), Err(())].into_iter();
        assert_eq!(
            collect_view(&mut Products::new(), &mut names, &mut |_| Some(msi("1"))),
            None
        );
        // An entry named by a code that cannot be opened.
        let mut names = vec![Ok(A.to_string())].into_iter();
        assert_eq!(
            collect_view(&mut Products::new(), &mut names, &mut |_| None),
            None
        );
        // One that is not named by a code need not be opened.
        let mut names = vec![Ok("Other".to_string())].into_iter();
        assert_eq!(
            collect_view(&mut Products::new(), &mut names, &mut |_| None),
            Some(())
        );
        // More entries than the cap.
        let mut many = (0..=MAX_ENTRIES).map(|i| Ok(format!("app{i}")));
        assert_eq!(
            collect_view(&mut Products::new(), &mut many, &mut |_| None),
            None
        );
        let mut most = (0..MAX_ENTRIES).map(|i| Ok(format!("app{i}")));
        assert_eq!(
            collect_view(&mut Products::new(), &mut most, &mut |_| None),
            Some(())
        );
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

    /// The whole removal: looked at, run only when it is as PCTwin installed it, looked at again.
    #[test]
    fn msiexec_runs_only_for_the_app_as_pctwin_installed_it() {
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

        let mut again = Products::new();
        again.insert(code(A), at(Some("2.0"), Some("20261105")));
        for (now, want) in [
            (
                Some(list(&[(A, Some("2.1"))])),
                Effect::Uninstall(UninstallResult::KeptOtherVersion),
            ),
            (
                Some(again),
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

    /// F6: after a crash, in place only when both of Windows' records say gone and no Windows
    /// Installer program runs; otherwise unknown unless both say there.
    #[test]
    fn after_a_crash_removal_is_looked_at_not_guessed() {
        let (t, f) = (Some(true), Some(false));
        assert_eq!(observed(f, f, f), Observed::Present);
        assert_eq!(observed(f, t, t), Observed::Absent);
        for (running, listed, registered) in [
            (t, f, f),
            (t, t, t),
            (None, f, f),
            (f, t, f),
            (f, f, t),
            (f, None, f),
            (f, f, None),
            (f, None, None),
        ] {
            assert_eq!(
                observed(running, listed, registered),
                Observed::Unknown,
                "{running:?} {listed:?} {registered:?}"
            );
        }
        assert_eq!(installer_among(None), None);
        assert_eq!(installer_among(Some(vec![])), Some(false));
        assert_eq!(
            installer_among(Some(vec![r"C:\Windows\System32\MsiExec.exe /V".into()])),
            Some(true)
        );
        assert_eq!(
            installer_among(Some(vec![r"C:\Windows\explorer.exe".into()])),
            Some(false)
        );
    }

    /// F1: the mark that an install was used is its own key, the same for the same install and
    /// never the install's own key.
    #[test]
    fn the_used_mark_is_one_key_per_install() {
        let one = Hex32([1; 32]);
        assert_eq!(consumed_key(&one), consumed_key(&Hex32([1; 32])));
        assert_ne!(consumed_key(&one), consumed_key(&Hex32([2; 32])));
        assert_ne!(consumed_key(&one), one);
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
}
