//! The connection doctor's rules: from what the probes saw to plain-language causes and fixes.
//! When the doctor cannot be sure, it says "likely" and offers each fix, never one confident guess.

use std::net::IpAddr;

use pctwin_doctor::{
    Cause, Certainty, Evidence, Finding, Interface, InterfaceKind, NetworkProfile, Role, diagnose,
};

fn iface(name: &str, ip: &str) -> Interface {
    Interface::new(name, ip.parse::<IpAddr>().unwrap())
}

/// A laptop on ordinary home Wi-Fi where everything works.
fn healthy() -> Evidence {
    Evidence {
        interfaces: Some(vec![
            iface("Wi-Fi", "192.168.1.20"),
            iface("Loopback", "127.0.0.1"),
        ]),
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
fn only_virtual_connections_mean_probably_not_connected() {
    let mut e = healthy();
    e.interfaces = Some(vec![
        iface("Loopback", "127.0.0.1"),
        iface("vEthernet (WSL)", "172.29.48.1"),
    ]);
    e.old_laptops_found = Some(0);
    let found = causes(&e, Role::NewLaptop);
    assert_eq!(found[0], (Cause::NotConnected, Certainty::Likely));
    // Nothing else is guessed at when the laptop isn't on a real network.
    assert_eq!(found.len(), 1, "{found:?}");
}

#[test]
fn a_refused_send_on_a_mac_points_at_the_local_network_permission() {
    let mut e = healthy();
    e.is_macos = true;
    e.multicast_send_blocked = Some(true);
    e.hears_itself = Some(false);
    e.old_laptops_found = Some(0);
    let found = causes(&e, Role::NewLaptop);
    // The same refusal can appear before the person answers the prompt, so only Likely.
    assert_eq!(found[0], (Cause::MacLocalNetworkDenied, Certainty::Likely));
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
    e.interfaces
        .as_mut()
        .unwrap()
        .push(iface("Tailscale", "100.101.2.3"));
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
    e.interfaces
        .as_mut()
        .unwrap()
        .push(iface("Tailscale", "100.101.2.3"));
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
    offline.interfaces = Some(vec![iface("Loopback", "127.0.0.1")]);
    assert!(!diagnose(&offline, Role::NewLaptop).suggest_phone_hotspot);
}

#[test]
fn unknown_evidence_is_never_treated_as_a_problem() {
    let e = Evidence {
        interfaces: Some(vec![iface("Wi-Fi", "192.168.1.20")]),
        hears_itself: None,
        multicast_send_blocked: None,
        windows_profiles: vec![],
        old_laptops_found: None,
        is_macos: true,
    };
    assert!(diagnose(&e, Role::NewLaptop).findings.is_empty());
    // Knowing nothing at all is not "not connected".
    for role in [Role::OldLaptop, Role::NewLaptop] {
        assert!(
            diagnose(&Evidence::default(), role).findings.is_empty(),
            "{role:?}"
        );
    }
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
    e.interfaces = Some(vec![
        iface("Wi-Fi", "169.254.12.7"),
        iface("Wi-Fi", "fe80::1"),
    ]);
    e.old_laptops_found = Some(0);
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

#[test]
fn a_stock_mac_with_its_built_in_utun_adapters_is_not_told_a_vpn_is_on() {
    let mut e = healthy();
    e.is_macos = true;
    e.interfaces = Some(vec![
        iface("en0", "192.168.1.23"),
        iface("utun0", "fe80::1"),
        iface("utun1", "fe80::2"),
        iface("utun2", "fe80::3"),
        iface("utun3", "fe80::4"),
        iface("lo0", "127.0.0.1"),
    ]);
    e.old_laptops_found = Some(0);
    let found = causes(&e, Role::NewLaptop);
    assert!(
        !found.iter().any(|(c, _)| matches!(c, Cause::VpnOn(_))),
        "{found:?}"
    );
    assert_eq!(found[0], (Cause::OldLaptopNotPairing, Certainty::Likely));
}

#[test]
fn a_vpn_with_two_addresses_is_named_once_and_the_hotspot_offered() {
    let mut e = healthy();
    e.interfaces
        .as_mut()
        .unwrap()
        .push(iface("Tailscale", "100.101.2.3"));
    e.interfaces
        .as_mut()
        .unwrap()
        .push(iface("Tailscale", "fd7a:115c:a1e0::1"));
    e.old_laptops_found = Some(0);
    let d = diagnose(&e, Role::NewLaptop);
    assert_eq!(
        d.findings,
        vec![Finding {
            cause: Cause::VpnOn(vec!["Tailscale".into()]),
            certainty: Certainty::Likely
        }]
    );
    assert!(d.suggest_phone_hotspot);
}

#[test]
fn shared_carrier_addresses_are_not_mistaken_for_a_vpn() {
    // Hotel, carrier and satellite networks use 100.64/10 too; only the name says VPN.
    assert_eq!(iface("Wi-Fi", "100.64.0.7").kind, InterfaceKind::Network);
    assert_eq!(iface("Tailscale", "100.101.2.3").kind, InterfaceKind::Vpn);
}

#[test]
fn a_found_old_laptop_is_never_followed_by_not_connected() {
    // A direct cable gives only self-assigned addresses, yet the laptops see each other.
    let mut e = healthy();
    e.interfaces = Some(vec![iface("Ethernet", "169.254.10.2")]);
    e.old_laptops_found = Some(1);
    assert!(causes(&e, Role::NewLaptop).is_empty());
    e.interfaces = Some(vec![iface("vEthernet (WSL)", "172.29.48.1")]);
    assert!(causes(&e, Role::NewLaptop).is_empty());
}

#[test]
fn a_local_cause_is_shown_alone_without_the_general_list() {
    let mut e = healthy();
    e.windows_profiles = vec![("Wi-Fi".into(), NetworkProfile::Public)];
    e.old_laptops_found = Some(0);
    assert_eq!(
        causes(&e, Role::NewLaptop),
        vec![(
            Cause::WindowsPublicNetwork("Wi-Fi".into()),
            Certainty::Likely
        )]
    );
}

#[test]
fn when_the_old_laptop_was_found_discovery_is_not_blamed() {
    let mut e = healthy();
    e.hears_itself = Some(false);
    e.is_macos = true;
    e.multicast_send_blocked = Some(true);
    e.old_laptops_found = Some(2);
    assert!(causes(&e, Role::NewLaptop).is_empty());
}

#[test]
fn more_adapter_names_are_classified() {
    let kind = |n: &str, ip: &str| iface(n, ip).kind;
    for vpn in [
        "Mullvad",
        "CloudflareWARP",
        "Hamachi",
        "zt5u4y6",
        "mullvad-wg",
        "proton0",
        "nebula1",
    ] {
        assert_eq!(kind(vpn, "10.6.0.2"), InterfaceKind::Vpn, "{vpn}");
    }
    for virt in [
        "lxcbr0",
        "lxdbr0",
        "incusbr0",
        "podman0",
        "cni0",
        "flannel.1",
        "cilium_host",
        "Local Area Connection* 10",
        "Bluetooth Network Connection",
    ] {
        assert_eq!(kind(virt, "10.0.3.1"), InterfaceKind::Virtual, "{virt}");
    }
    assert_ne!(
        kind("Npcap Loopback Adapter", "10.0.3.1"),
        InterfaceKind::Network
    );
    assert_eq!(
        kind("vEthernet (External Switch)", "192.168.1.9"),
        InterfaceKind::Network
    );
    assert_eq!(kind("Wi-Fi 2", "192.168.1.9"), InterfaceKind::Network);
}

/// Checks the doctor's honesty rules over many combinations of evidence.
#[test]
fn the_doctor_is_honest_across_every_combination_of_evidence() {
    let interface_sets: Vec<Option<Vec<Interface>>> = vec![
        None,
        Some(vec![]),
        Some(vec![iface("Wi-Fi", "169.254.3.3")]),
        Some(vec![iface("vEthernet (WSL)", "172.29.48.1")]),
        Some(vec![iface("Wi-Fi", "192.168.1.5")]),
        Some(vec![iface("en0", "192.168.1.5"), iface("utun0", "fe80::1")]),
        Some(vec![
            iface("Wi-Fi", "192.168.1.5"),
            iface("Tailscale", "100.70.0.1"),
        ]),
    ];
    let opts = [None, Some(true), Some(false)];
    let profiles = [vec![], vec![("Wi-Fi".to_string(), NetworkProfile::Public)]];
    let found = [None, Some(0), Some(2)];
    let mut checked = 0;
    for interfaces in &interface_sets {
        for hears in opts {
            for blocked in opts {
                for prof in &profiles {
                    for f in found {
                        for mac in [false, true] {
                            for role in [Role::OldLaptop, Role::NewLaptop] {
                                let e = Evidence {
                                    interfaces: interfaces.clone(),
                                    hears_itself: hears,
                                    multicast_send_blocked: blocked,
                                    windows_profiles: prof.clone(),
                                    old_laptops_found: f,
                                    is_macos: mac,
                                };
                                let d = diagnose(&e, role);
                                let has = |c: &Cause| d.findings.iter().any(|x| &x.cause == c);
                                let no_address = matches!(interfaces, Some(list)
                                    if list.iter().all(|i| i.ip.is_loopback()
                                        || i.ip.to_string().starts_with("169.254")
                                        || i.ip.to_string().starts_with("fe80")));
                                // Sure only when there is no address at all.
                                for x in &d.findings {
                                    if x.certainty == Certainty::Sure {
                                        assert!(
                                            no_address && x.cause == Cause::NotConnected,
                                            "{e:?}"
                                        );
                                    }
                                }
                                // Not knowing the interfaces never means "not connected".
                                if interfaces.is_none() {
                                    assert!(!has(&Cause::NotConnected), "{e:?}");
                                }
                                // "Not connected" stands alone.
                                if has(&Cause::NotConnected) {
                                    assert_eq!(d.findings.len(), 1, "{e:?}");
                                }
                                // Discovery that demonstrably works is never blamed.
                                // (A search result only counts on the new laptop.)
                                let found_some =
                                    role == Role::NewLaptop && matches!(f, Some(n) if n > 0);
                                if hears == Some(true) || found_some {
                                    assert!(!has(&Cause::MacLocalNetworkDenied), "{e:?}");
                                    assert!(!has(&Cause::DiscoveryBlockedOnThisLaptop), "{e:?}");
                                }
                                // A Mac permission is only ever suspected on a Mac.
                                if !mac {
                                    assert!(!has(&Cause::MacLocalNetworkDenied), "{e:?}");
                                }
                                // Only the new laptop, and only after an empty search, hears these.
                                let search_empty = role == Role::NewLaptop && f == Some(0);
                                for c in [
                                    Cause::OldLaptopNotPairing,
                                    Cause::DifferentNetworks,
                                    Cause::GuestNetworkHidesDevices,
                                ] {
                                    if has(&c) {
                                        assert!(search_empty, "{e:?}");
                                    }
                                }
                                if d.findings
                                    .iter()
                                    .any(|x| matches!(x.cause, Cause::VpnOn(_)))
                                {
                                    assert!(search_empty, "{e:?}");
                                }
                                // An empty search on a connected new laptop always gets an answer.
                                if search_empty && !has(&Cause::NotConnected) {
                                    assert!(!d.findings.is_empty(), "{e:?}");
                                }
                                // A link-local-only utun never counts as a VPN.
                                if matches!(interfaces, Some(l) if l.iter().any(|i| i.name == "utun0"))
                                {
                                    assert!(
                                        !d.findings
                                            .iter()
                                            .any(|x| matches!(x.cause, Cause::VpnOn(_))),
                                        "{e:?}"
                                    );
                                }
                                checked += 1;
                            }
                        }
                    }
                }
            }
        }
    }
    assert_eq!(checked, 7 * 3 * 3 * 2 * 3 * 2 * 2);
}
