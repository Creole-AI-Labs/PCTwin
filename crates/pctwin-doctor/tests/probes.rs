//! The probes, run for real on every CI machine (Windows, macOS, Linux; Intel and ARM).

use std::time::Duration;

use pctwin_doctor::{
    Cause, InterfaceKind, NetworkProfile, Role, diagnose, probe, profile_from_category,
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
fn windows_network_profiles_are_read_on_windows_quickly() {
    // Asked of Windows directly (WMI), not through PowerShell, whose start-up took over 90 s on a
    // busy CI machine. The same busy machines must now answer well inside 15 s.
    let started = std::time::Instant::now();
    let profiles = probe::windows_profiles_within(Duration::from_secs(15));
    if cfg!(windows) {
        let profiles = profiles.expect("the profile check works on this machine");
        assert!(
            !profiles.is_empty(),
            "a Windows machine always has a profile"
        );
        assert!(started.elapsed() < Duration::from_secs(15));
    } else {
        assert!(profiles.is_none());
    }
}

#[test]
fn only_a_refusal_counts_as_blocked() {
    use std::io::ErrorKind;
    assert_eq!(
        probe::send_error_means_blocked(ErrorKind::HostUnreachable),
        Some(true)
    );
    assert_eq!(
        probe::send_error_means_blocked(ErrorKind::PermissionDenied),
        Some(true)
    );
    // No route (an IPv6-only network) or anything else: could not tell.
    for kind in [
        ErrorKind::NetworkUnreachable,
        ErrorKind::AddrNotAvailable,
        ErrorKind::Other,
    ] {
        assert_eq!(probe::send_error_means_blocked(kind), None, "{kind:?}");
    }
}

#[test]
fn windows_network_categories_are_read_strictly() {
    // As Windows numbers them (MSFT_NetConnectionProfile.NetworkCategory).
    assert_eq!(profile_from_category(0), Some(NetworkProfile::Public));
    assert_eq!(profile_from_category(1), Some(NetworkProfile::Private));
    assert_eq!(profile_from_category(2), Some(NetworkProfile::Domain));
    for unknown in [3, 4, 255, u32::MAX] {
        assert_eq!(profile_from_category(unknown), None, "{unknown}");
    }
    assert_eq!(probe::CREATE_NO_WINDOW, 0x0800_0000);
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

#[test]
fn a_check_that_hangs_is_stopped_at_its_limit_and_reported_as_unknown() {
    // A command that would run for about 30 seconds.
    let (program, args): (&str, &[&str]) = if cfg!(windows) {
        ("ping", &["-n", "30", "127.0.0.1"])
    } else {
        ("sleep", &["30"])
    };
    let started = std::time::Instant::now();
    let out = probe::run_check(program, args, Duration::from_millis(500));
    assert_eq!(out, None, "too slow means could not tell");
    assert!(
        started.elapsed() < Duration::from_secs(5),
        "{:?}",
        started.elapsed()
    );
}

#[test]
fn a_quick_check_returns_what_it_printed() {
    let (program, args): (&str, &[&str]) = if cfg!(windows) {
        ("cmd", &["/C", "echo Wi-Fi^|Private"])
    } else {
        ("echo", &["Wi-Fi|Private"])
    };
    let out = probe::run_check(program, args, Duration::from_secs(10)).unwrap();
    assert_eq!(out.trim(), "Wi-Fi|Private");
}

/// Every supported system can say which adapters are Wi-Fi (CI runners have none, so the list
/// may be empty, but the question gets an answer), and the address list carries the answer.
#[test]
fn the_system_says_which_adapters_are_wifi() {
    let wifi = probe::wifi_adapters();
    assert!(wifi.is_some(), "the system could not be asked");
    let interfaces = probe::interfaces().expect("interfaces can be listed");
    assert!(interfaces.iter().all(|i| i.wifi.is_some()));
    for i in &interfaces {
        assert_eq!(
            i.wifi,
            Some(wifi.as_ref().unwrap().contains(&i.name)),
            "{}",
            i.name
        );
    }
}

/// A Mac's hardware ports: the Wi-Fi port's device, whatever it is called, and nothing else.
#[test]
fn a_macs_wifi_device_is_read_from_its_hardware_ports() {
    let out = "\nHardware Port: Thunderbolt Bridge\nDevice: bridge0\nEthernet Address: 36:aa:aa:aa:aa:00\n\nHardware Port: Wi-Fi\nDevice: en0\nEthernet Address: a4:bb:bb:bb:bb:01\n\nHardware Port: Thunderbolt 1\nDevice: en1\n\nHardware Port: USB 10/100/1000 LAN\nDevice: en7\n\nVLAN Configurations\n===================\n";
    assert_eq!(probe::wifi_from_hardware_ports(out), ["en0"]);
    let older = "Hardware Port: AirPort\nDevice: en1\n";
    assert_eq!(probe::wifi_from_hardware_ports(older), ["en1"]);
    assert!(probe::wifi_from_hardware_ports("Hardware Port: Ethernet\nDevice: en0\n").is_empty());
}
