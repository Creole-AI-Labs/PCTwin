//! End to end, as an administrator on Windows (run by the CI job "Admin winget install", never by
//! an ordinary test run): a real, signed Windows Installer package is downloaded with winget as
//! the runner's own account, its manifest read and matched to the app by product code, kept in
//! the helper's own folder, checked by the helper's own maker check against the maker the old
//! laptop's copy would carry, installed by the helper's real adapter, found by Windows' own list
//! of installed products (these exact bytes), then found again by the new laptop's probe and
//! given its result word. It is removed at the end.
//!
//! Run: `cargo test -p pctwin-apps --test admin_winget -- --ignored --nocapture` (elevated).

#![cfg(windows)]

use std::process::Command;

use pctwin_apps::routes::winget::{Manifest, confirms, download_call, downloaded, winget_path};
use pctwin_helper::broker::actions::{Hex32, InstallScope, InstallVerifiedPackage, PackageKind};
use pctwin_helper::broker::maker::MakerCheck;
use pctwin_helper::broker::{Adapters, BuiltInAdapters, Effect, Observed, Revoker};
use pctwin_maker::run::Runner;
use sha2::Digest;

#[test]
#[ignore = "needs an administrator on Windows and the internet: run by the CI job \"Admin winget install\""]
fn a_winget_download_is_installed_by_the_helper_and_found_by_its_exact_bytes() {
    assert!(
        pctwin_helper_os::this_process_elevated().unwrap(),
        "run as an administrator"
    );
    let work = tempfile::tempdir().unwrap();
    let folder = work.path().join("download");
    std::fs::create_dir(&folder).unwrap();
    let local = std::env::var_os("LOCALAPPDATA").expect("LOCALAPPDATA");
    let call = download_call(
        &winget_path(std::path::Path::new(&local)),
        "PuTTY.PuTTY",
        "x64",
        &folder,
    )
    .unwrap();
    let ran = pctwin_maker::run::SystemRunner
        .run(&call)
        .expect("winget runs");
    assert_eq!(ran.code, Some(0), "{}\n{}", ran.stdout, ran.stderr);
    let (yaml, msi) = downloaded(&folder).unwrap();
    let manifest = Manifest::parse(&std::fs::read_to_string(yaml).unwrap()).unwrap();
    let bytes = std::fs::read(&msi).unwrap();
    let sha: [u8; 32] = sha2::Sha256::digest(&bytes).into();
    assert_eq!(manifest.sha256, Some(sha));
    let code = manifest.product_code.clone().expect("a product code");
    let app = pctwin_record::FoundApp {
        name: manifest
            .entries
            .first()
            .and_then(|e| e.display_name.clone())
            .unwrap_or_default(),
        version: manifest.version.clone(),
        publisher: manifest.publisher.clone(),
        owner: pctwin_record::Owner::Shared,
        system: pctwin_record::AppSystem::Windows,
        found: pctwin_record::FoundWhere::WindowsEntry {
            view: pctwin_record::RegistryView::Machine,
            key: code.clone(),
            product_code: Some(code.clone()),
        },
        arch: pctwin_record::AppArch::UNKNOWN,
        maker: pctwin_record::AppMaker::Unsigned,
        web: None,
        update_feed: None,
        installers: vec![],
        notes: vec![],
    };
    assert!(confirms(&manifest, &app).is_some());
    let pctwin_maker::OldMaker::Signed(signer) = pctwin_maker::windows_old_copy(&msi).maker else {
        panic!("the package is signed");
    };
    let packages = work.path().join("packages");
    std::fs::create_dir(&packages).unwrap();
    let mut staging = pctwin_helper::broker::staging::Staging::new(
        pctwin_helper::broker::staging::HelperFolder::for_tests(packages),
    );
    let handle = staging.admit(&mut bytes.as_slice(), sha).unwrap();
    let action = InstallVerifiedPackage {
        artifact: handle,
        sha256: Hex32(sha),
        kind: PackageKind::Msi,
        scope: InstallScope::Machine,
        maker: Box::new(signer),
    };
    let mut adapters = BuiltInAdapters;
    let as_action = pctwin_helper::broker::actions::Action::InstallVerifiedPackage(action.clone());
    assert_eq!(
        adapters.observe(&as_action),
        Observed::Absent,
        "not installed yet"
    );
    let held = staging.open_verified(handle, sha).unwrap();
    assert_eq!(adapters.check_maker(&held, &action), MakerCheck::Approved);
    let effect = adapters.install(&held, &action, &Revoker::default());
    drop(held);
    println!("install: {effect:?}");
    assert!(matches!(effect, Effect::Done { .. }), "{effect:?}");
    // After a crash, the helper would find these exact bytes installed: never installed twice.
    assert_eq!(adapters.observe(&as_action), Observed::Present);
    // The new laptop's probe and the result word.
    let entries = read_machine_entries();
    let probe =
        pctwin_apps::after::probe(&app, pctwin_apps::after::InstallScope::Machine, &entries);
    assert!(probe.found);
    let result = pctwin_apps::after::result(
        &app,
        pctwin_apps::after::Installed::Done {
            restart_needed: false,
        },
        &probe,
        &[],
        pctwin_apps::after::Opened::NotChecked,
    );
    assert_eq!(result.status(), pctwin_types::ItemStatus::Reinstalled);
    // Removed again.
    let removed = Command::new(r"C:\Windows\System32\msiexec.exe")
        .args(["/x", &code, "/qn", "/norestart"])
        .status()
        .unwrap();
    assert!(removed.success());
    assert_eq!(adapters.observe(&as_action), Observed::Absent);
}

fn read_machine_entries() -> Vec<pctwin_apps::found::windows::RawEntry> {
    use winreg::RegKey;
    use winreg::enums::{HKEY_LOCAL_MACHINE, KEY_READ, KEY_WOW64_64KEY};
    let list = RegKey::predef(HKEY_LOCAL_MACHINE)
        .open_subkey_with_flags(
            r"SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall",
            KEY_READ | KEY_WOW64_64KEY,
        )
        .unwrap();
    list.enum_keys()
        .flatten()
        .filter_map(|k| {
            let key = list
                .open_subkey_with_flags(&k, KEY_READ | KEY_WOW64_64KEY)
                .ok()?;
            let mut values = std::collections::BTreeMap::new();
            for (n, v) in key.enum_values().flatten() {
                let v = match v.vtype {
                    winreg::enums::REG_DWORD => pctwin_apps::found::windows::RegValue::Number(
                        winreg::types::FromRegValue::from_reg_value(&v).ok()?,
                    ),
                    _ => pctwin_apps::found::windows::RegValue::Text(
                        winreg::types::FromRegValue::from_reg_value(&v).unwrap_or_default(),
                    ),
                };
                values.insert(n.to_ascii_lowercase(), v);
            }
            Some(pctwin_apps::found::windows::RawEntry {
                view: pctwin_record::RegistryView::Machine,
                key: k,
                values,
            })
        })
        .collect()
}
