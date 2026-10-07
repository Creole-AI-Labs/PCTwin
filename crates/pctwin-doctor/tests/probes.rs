//! The probes, run for real on every CI machine (Windows, macOS, Linux; Intel and ARM).

use std::time::Duration;

use pctwin_doctor::{
    Cause, InterfaceKind, NetworkProfile, Role, diagnose, parse_windows_profiles, probe,
};

#[test]
fn this_machine_has_a_loopback_and_its_interfaces_are_listed() {
    let interfaces = probe::interfaces().expect("interfaces can be listed");
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
fn sending_to_the_discovery_group_is_allowed_here() {
    // This development laptop and the CI machines allow it.
    assert_eq!(probe::multicast_send_blocked(), Some(false));
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
    let out = "Wi-Fi|Private\r\nEthernet 2|Public\nWi|Fi|Private\nБеспроводная сеть|Public\nvEthernet (WSL)|DomainAuthenticated\n\nbroken line\n|Public\nX|Unknown\n";
    assert_eq!(
        parse_windows_profiles(out),
        vec![
            ("Wi-Fi".to_string(), NetworkProfile::Private),
            ("Ethernet 2".to_string(), NetworkProfile::Public),
            ("Wi|Fi".to_string(), NetworkProfile::Private),
            ("Беспроводная сеть".to_string(), NetworkProfile::Public),
            ("vEthernet (WSL)".to_string(), NetworkProfile::Domain),
        ]
    );
}

#[test]
fn on_this_machine_the_doctor_blames_no_vpn_and_sees_a_connection() {
    // Runs on every CI machine, including real Macs with their built-in utun adapters.
    let d = diagnose(&probe::gather(Role::NewLaptop, Some(0)), Role::NewLaptop);
    assert!(
        !d.findings
            .iter()
            .any(|f| matches!(f.cause, Cause::VpnOn(_) | Cause::NotConnected)),
        "{d:?}"
    );
    assert!(
        !d.findings.is_empty(),
        "an empty search always gets an answer"
    );
}
