//! Sixth review, findings 3 and 4, as tests of the safe outcome: the old laptop judges what it
//! found before anything is sent, so a password or a token never crosses the link, and one value
//! past a limit is left out and counted instead of making the new laptop refuse every setting.

use pctwin_apps::routes::{Target, WingetBy};
use pctwin_fullmove::extras::{Extras, digest, from_discovery};
use pctwin_fullmove::plan::{FullPlan, Kind, NewLaptop, Title};
use pctwin_record::{AppArch, AppList, AppSystem, FoundApp, LaptopId, Record};
use pctwin_settings::System;
use pctwin_settings::found::discover_registry;
use pctwin_settings::registry::{MemHive, RegValue};

fn target() -> Target {
    Target {
        system: AppSystem::Windows,
        arch: AppArch {
            known: true,
            x86: false,
            x64: true,
            arm64: false,
        },
        linux: None,
    }
}

fn plan_of(bytes: Option<&[u8]>) -> FullPlan {
    let decoded = bytes.and_then(|b| Some((Extras::decode(b).ok()?, digest(b))));
    let record = Record::new(LaptopId::from_hex("01010101010101010101010101010101").unwrap());
    let installed = AppList::default();
    let none = |_: &FoundApp| -> Vec<(String, WingetBy)> { vec![] };
    let here = NewLaptop {
        system: System::Windows,
        target: target(),
        installed: &installed,
        folders: vec![],
        winget: &none,
    };
    let mut plan = FullPlan::build(
        &record,
        "1000",
        decoded.as_ref().map(|(x, d)| (x, *d)),
        &here,
        &mut |_, _| Some(None),
    );
    plan.extras_refused = bytes.is_some() && decoded.is_none();
    plan
}

#[test]
fn secret_values_never_cross_the_link_they_are_only_counted() {
    pctwin_parse::testing::install_readers();
    let mut h = MemHive::default();
    h.put(
        r"Software\SimonTatham\PuTTY\Sessions\Home",
        "ProxyPassword",
        RegValue::Sz("hunter2-plaintext-proxy".into()),
    );
    h.put(
        r"Software\SimonTatham\PuTTY\Sessions\Home",
        "HostName",
        RegValue::Sz("host.example".into()),
    );
    h.put(
        r"Software\ExampleVendor\Sync",
        "AccessToken",
        RegValue::Sz("tok_live_SECRETSECRET".into()),
    );
    h.put(
        r"Software\ORL\WinVNC3",
        "Password",
        RegValue::Binary(vec![0xde, 0xad, 0xbe, 0xef, 0x13, 0x37]),
    );
    let x = from_discovery(System::Windows, discover_registry(&h), vec![]);
    let bytes = x.encode();
    let text = String::from_utf8_lossy(&bytes);
    for secret in [
        "hunter2-plaintext-proxy",
        "tok_live_SECRETSECRET",
        "deadbeef1337",
    ] {
        assert!(!text.contains(secret), "{secret} crossed the link");
    }
    assert!(
        x.not_moved.iter().map(|n| n.count).sum::<u64>() >= 3,
        "{:?}",
        x.not_moved
    );
}

#[test]
fn one_long_value_is_left_out_and_counted_and_every_other_setting_still_moves() {
    pctwin_parse::testing::install_readers();
    let mut h = MemHive::default();
    h.put(
        r"Software\Vend\Editor",
        "Theme",
        RegValue::Sz("ocean".into()),
    );
    h.put(r"Software\Vend\Editor", "FontSize", RegValue::Dword(14));
    h.put(
        r"Software\Vend\Editor",
        "RecentLayout",
        RegValue::Sz("x".repeat(5000)),
    );
    let x = from_discovery(System::Windows, discover_registry(&h), vec![]);
    let bytes = x.encode();
    assert!(Extras::decode(&bytes).is_ok(), "the new laptop reads it");
    let plan = plan_of(Some(&bytes));
    let settings: u64 = plan.groups.iter().map(|g| g.count).sum();
    assert_eq!(
        settings, 2,
        "Theme and FontSize still move: {:?}",
        plan.groups
    );
    assert!(x.not_moved.iter().any(|n| n.count >= 1));
}

#[test]
fn extras_the_new_laptop_refuses_are_said_on_the_plan() {
    pctwin_parse::testing::install_readers();
    let plan = plan_of(Some(b"{not the extras"));
    assert!(
        plan.changes().iter().any(|c| c.kind == Kind::Settings
            && c.title == Title::Key(pctwin_fullmove::plan::EXTRAS_REFUSED)
            && c.needs.is_some()),
        "{:#?}",
        plan.changes()
    );
}
