//! The probes, run for real on every CI machine (Windows, macOS, Linux; Intel and ARM).

use std::time::Duration;

use pctwin_doctor::{InterfaceKind, NetworkProfile, parse_windows_profiles, probe};

#[test]
fn this_machine_has_a_loopback_and_its_interfaces_are_listed() {
    let interfaces = probe::interfaces();
    assert!(
        interfaces.iter().any(|i| i.kind == InterfaceKind::Loopback),
        "{interfaces:?}"
    );
}

#[test]
fn this_machine_can_hear_its_own_announcement() {
    // CI machines and this development laptop allow local discovery; a laptop that cannot hear
    // itself is what the doctor reports as blocked.
    assert_eq!(probe::hears_itself(Duration::from_secs(3)), Some(true));
}

#[test]
fn sending_to_the_discovery_group_is_checked_without_error() {
    assert!(probe::multicast_send_blocked().is_some());
}

#[test]
fn windows_network_profiles_are_read_on_windows() {
    let profiles = probe::windows_profiles();
    if cfg!(windows) {
        assert!(
            !profiles.is_empty(),
            "a Windows machine always has a profile"
        );
    } else {
        assert!(profiles.is_empty());
    }
}

#[test]
fn windows_profile_output_is_parsed_strictly() {
    let out = "Wi-Fi|Private\r\nEthernet 2|Public\nvEthernet (WSL)|DomainAuthenticated\n\nbroken line\n|Public\nX|Unknown\n";
    assert_eq!(
        parse_windows_profiles(out),
        vec![
            ("Wi-Fi".to_string(), NetworkProfile::Private),
            ("Ethernet 2".to_string(), NetworkProfile::Public),
            ("vEthernet (WSL)".to_string(), NetworkProfile::Domain),
        ]
    );
}
