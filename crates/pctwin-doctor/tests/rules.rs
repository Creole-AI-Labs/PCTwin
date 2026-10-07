//! The connection doctor's rules: from what the probes saw to plain-language causes and fixes.
//! When the doctor cannot be sure, it says "likely" and offers each fix, never one confident guess.

use std::net::IpAddr;

use pctwin_doctor::{
    Cause, Certainty, Evidence, Interface, InterfaceKind, NetworkProfile, Role, diagnose,
};

fn iface(name: &str, ip: &str) -> Interface {
    Interface::new(name, ip.parse::<IpAddr>().unwrap())
}

/// A laptop on ordinary home Wi-Fi where everything works.
fn healthy() -> Evidence {
    Evidence {
        interfaces: vec![
            iface("Wi-Fi", "192.168.1.20"),
            iface("Loopback", "127.0.0.1"),
        ],
        hears_itself: Some(true),
        multicast_send_blocked: Some(false),
        windows_profiles: vec![("Wi-Fi".into(), NetworkProfile::Private)],
        old_laptops_found: Some(1),
        is_macos: false,
    }
}

fn causes(e: &Evidence, role: Role) -> Vec<(Cause, Certainty)> {
    diagnose(e, role)
        .findings
        .into_iter()
        .map(|f| (f.cause, f.certainty))
        .collect()
}

#[test]
fn interfaces_are_recognised_by_kind() {
    let kind = |n: &str, ip: &str| iface(n, ip).kind;
    assert_eq!(kind("Wi-Fi", "192.168.1.2"), InterfaceKind::Network);
    assert_eq!(kind("en0", "10.0.0.5"), InterfaceKind::Network);
    assert_eq!(
        kind("Local Area Connection", "192.168.0.4"),
        InterfaceKind::Network
    );
    assert_eq!(kind("Ethernet 2", "172.16.0.9"), InterfaceKind::Network);
    assert_eq!(
        kind("Loopback Pseudo-Interface 1", "127.0.0.1"),
        InterfaceKind::Loopback
    );
    assert_eq!(kind("lo0", "::1"), InterfaceKind::Loopback);
    for vpn in [
        "Tailscale",
        "utun3",
        "wg0",
        "tun0",
        "ProtonVPN",
        "NordLynx",
        "OpenVPN TAP-Windows6",
        "ppp0",
        "ZeroTier One",
    ] {
        assert_eq!(kind(vpn, "10.8.0.2"), InterfaceKind::Vpn, "{vpn}");
    }
    for virt in [
        "vEthernet (WSL)",
        "vEthernet (Default Switch)",
        "VMware Network Adapter VMnet8",
        "VirtualBox Host-Only Network",
        "docker0",
        "br-1a2b",
        "veth12ab",
        "bridge100",
    ] {
        assert_eq!(kind(virt, "172.29.48.1"), InterfaceKind::Virtual, "{virt}");
    }
}

#[test]
fn a_healthy_laptop_gets_no_findings() {
    assert!(diagnose(&healthy(), Role::NewLaptop).findings.is_empty());
    assert!(diagnose(&healthy(), Role::OldLaptop).findings.is_empty());
}

#[test]
fn no_real_network_connection_is_certain_and_comes_first() {
    let mut e = healthy();
    e.interfaces = vec![
        iface("Loopback", "127.0.0.1"),
        iface("vEthernet (WSL)", "172.29.48.1"),
    ];
    e.old_laptops_found = Some(0);
    let found = causes(&e, Role::NewLaptop);
    assert_eq!(found[0], (Cause::NotConnected, Certainty::Sure));
    // Nothing else is guessed at when the laptop simply isn't connected.
    assert_eq!(found.len(), 1, "{found:?}");
}

#[test]
fn a_denied_mac_local_network_permission_is_certain() {
    let mut e = healthy();
    e.is_macos = true;
    e.multicast_send_blocked = Some(true);
    e.hears_itself = Some(false);
    e.old_laptops_found = Some(0);
    let found = causes(&e, Role::NewLaptop);
    assert_eq!(found[0], (Cause::MacLocalNetworkDenied, Certainty::Sure));
    assert!(
        !found
            .iter()
            .any(|(c, _)| *c == Cause::DiscoveryBlockedOnThisLaptop)
    );
}

#[test]
fn not_hearing_itself_points_at_this_laptop_but_only_as_likely() {
    let mut e = healthy();
    e.hears_itself = Some(false);
    e.old_laptops_found = Some(0);
    let found = causes(&e, Role::NewLaptop);
    assert_eq!(
        found[0],
        (Cause::DiscoveryBlockedOnThisLaptop, Certainty::Likely)
    );
}

#[test]
fn a_public_windows_network_is_flagged_on_either_laptop() {
    let mut e = healthy();
    e.windows_profiles = vec![("Wi-Fi".into(), NetworkProfile::Public)];
    for role in [Role::OldLaptop, Role::NewLaptop] {
        let found = causes(&e, role);
        assert!(
            found.contains(&(
                Cause::WindowsPublicNetwork("Wi-Fi".into()),
                Certainty::Likely
            )),
            "{role:?}: {found:?}"
        );
    }
    // A public profile on a virtual adapter does not matter.
    e.windows_profiles = vec![
        ("Wi-Fi".into(), NetworkProfile::Private),
        ("vEthernet (WSL)".into(), NetworkProfile::Public),
    ];
    assert!(causes(&e, Role::OldLaptop).is_empty());
}

#[test]
fn a_running_vpn_is_named() {
    let mut e = healthy();
    e.interfaces.push(iface("Tailscale", "100.101.2.3"));
    e.old_laptops_found = Some(0);
    let found = causes(&e, Role::NewLaptop);
    // The VPN explains it, so the general guesses are not added.
    assert_eq!(
        found,
        vec![(Cause::VpnOn(vec!["Tailscale".into()]), Certainty::Likely)]
    );
}

#[test]
fn a_vpn_alone_is_not_blamed_when_the_old_laptop_was_found() {
    let mut e = healthy();
    e.interfaces.push(iface("Tailscale", "100.101.2.3"));
    assert!(causes(&e, Role::NewLaptop).is_empty());
}

#[test]
fn when_nothing_explains_it_the_doctor_lists_the_likely_causes_with_a_fix_for_each() {
    let mut e = healthy();
    e.old_laptops_found = Some(0);
    let found = causes(&e, Role::NewLaptop);
    assert_eq!(
        found,
        vec![
            (Cause::OldLaptopNotPairing, Certainty::Likely),
            (Cause::DifferentNetworks, Certainty::Likely),
            (Cause::GuestNetworkHidesDevices, Certainty::Likely),
        ]
    );
    // Every finding has a plain-language message and a fix to show.
    for f in diagnose(&e, Role::NewLaptop).findings {
        assert!(f.cause.message_key().starts_with("doctor."));
        assert!(f.cause.fix_key().starts_with("doctor."));
    }
}

#[test]
fn the_phone_hotspot_is_offered_whenever_the_network_may_be_the_problem() {
    let mut e = healthy();
    e.old_laptops_found = Some(0);
    assert!(diagnose(&e, Role::NewLaptop).suggest_phone_hotspot);
    // Not when the laptop just isn't connected, or everything is fine.
    assert!(!diagnose(&healthy(), Role::NewLaptop).suggest_phone_hotspot);
    let mut offline = healthy();
    offline.interfaces = vec![iface("Loopback", "127.0.0.1")];
    assert!(!diagnose(&offline, Role::NewLaptop).suggest_phone_hotspot);
}

#[test]
fn unknown_evidence_is_never_treated_as_a_problem() {
    let e = Evidence {
        interfaces: vec![iface("Wi-Fi", "192.168.1.20")],
        hears_itself: None,
        multicast_send_blocked: None,
        windows_profiles: vec![],
        old_laptops_found: None,
        is_macos: true,
    };
    assert!(diagnose(&e, Role::NewLaptop).findings.is_empty());
}

#[test]
fn the_old_laptop_is_never_told_that_nothing_was_found() {
    let mut e = healthy();
    e.old_laptops_found = Some(0); // meaningless on the old laptop
    assert!(causes(&e, Role::OldLaptop).is_empty());
}

#[test]
fn a_self_assigned_address_means_not_connected() {
    // What a laptop gets when no network answered it.
    let mut e = healthy();
    e.interfaces = vec![iface("Wi-Fi", "169.254.12.7"), iface("Wi-Fi", "fe80::1")];
    assert_eq!(
        causes(&e, Role::NewLaptop),
        vec![(Cause::NotConnected, Certainty::Sure)]
    );
}

#[test]
fn the_mac_permission_is_only_blamed_on_a_mac() {
    let mut e = healthy();
    e.is_macos = false;
    e.multicast_send_blocked = Some(true);
    assert!(
        !causes(&e, Role::NewLaptop)
            .iter()
            .any(|(c, _)| *c == Cause::MacLocalNetworkDenied)
    );
    e.hears_itself = Some(false);
    assert_eq!(
        causes(&e, Role::OldLaptop),
        vec![(Cause::DiscoveryBlockedOnThisLaptop, Certainty::Likely)]
    );
}
