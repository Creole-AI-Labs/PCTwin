//! Which connection the old laptop pairs on (Task List 1.2): only the chosen Wi-Fi (or wired)
//! connection's own addresses, never every connection, so a VPN or a virtual machine's network
//! cannot reach the pairing port. One IPv4 and one IPv6 address of that connection, both local.

use std::net::{IpAddr, SocketAddr};

use pctwin_doctor::Interface;
use pctwin_session::{likely_wifi, pairing_candidates, pairing_network};

/// A connection's address, up, on an ordinary shared network (/24, or /64 for IPv6).
fn iface(name: &str, ip: &str, index: u32) -> Interface {
    let ip = ip.parse::<IpAddr>().unwrap();
    Interface {
        index: Some(index),
        prefix_len: Some(if ip.is_ipv4() { 24 } else { 64 }),
        ..Interface::new(name, ip)
    }
}

/// A VPN tunnel as Windows lists it: a single address (/32), reaching one other end.
fn tunnel(name: &str, ip: &str, index: u32) -> Interface {
    Interface {
        prefix_len: Some(32),
        point_to_point: true,
        ..iface(name, ip, index)
    }
}

fn laptop() -> Vec<Interface> {
    vec![
        iface("lo", "127.0.0.1", 1),
        iface("vEthernet (WSL)", "172.28.0.1", 9),
        iface("utun3", "10.8.0.2", 12),
        iface("Wi-Fi", "192.168.1.23", 4),
        iface("Wi-Fi", "fe80::1c2b:3a4d:5e6f:7081", 4),
        iface("Wi-Fi", "2a02:1234::99", 4),
        iface("Ethernet", "10.0.0.7", 5),
    ]
}

#[test]
fn the_likely_wifi_is_the_first_real_connection_with_a_private_address() {
    let net = pairing_network(&laptop(), None).unwrap();
    assert_eq!(net.name, "Wi-Fi");
    assert_eq!(
        net.addrs,
        vec![
            "192.168.1.23:0".parse::<SocketAddr>().unwrap(),
            // Link-local only works with its connection, so it keeps its scope.
            "[fe80::1c2b:3a4d:5e6f:7081%4]:0".parse().unwrap(),
        ]
    );
}

#[test]
fn the_persons_choice_is_used_when_given() {
    let net = pairing_network(&laptop(), Some("Ethernet")).unwrap();
    assert_eq!(net.name, "Ethernet");
    assert_eq!(net.addrs, vec!["10.0.0.7:0".parse::<SocketAddr>().unwrap()]);
}

#[test]
fn vpns_virtual_machines_and_loopback_are_never_used() {
    for name in ["lo", "vEthernet (WSL)", "utun3"] {
        assert!(pairing_network(&laptop(), Some(name)).is_none(), "{name}");
    }
    let only_odd = vec![iface("lo", "127.0.0.1", 1), iface("utun3", "10.8.0.2", 12)];
    assert!(pairing_network(&only_odd, None).is_none());
}

#[test]
fn a_public_address_is_never_listened_on() {
    let public = vec![
        iface("Wi-Fi", "203.0.113.5", 4),
        iface("Wi-Fi", "2a02:1234::99", 4),
    ];
    assert!(pairing_network(&public, Some("Wi-Fi")).is_none());
}

#[test]
fn a_private_ipv6_address_is_kept_when_there_is_no_link_local_one() {
    let v = vec![
        iface("en0", "192.168.0.9", 6),
        iface("en0", "fd12:3456::7", 6),
    ];
    let net = pairing_network(&v, Some("en0")).unwrap();
    assert_eq!(
        net.addrs[1],
        "[fd12:3456::7]:0".parse::<SocketAddr>().unwrap()
    );
}

/// The first real connection is skipped when it has no private IPv4 address (a cable plugged
/// into a modem with a public address, say): the likely Wi-Fi is the first one that has one.
#[test]
fn a_real_connection_without_a_private_address_is_passed_over() {
    let v = vec![
        iface("Ethernet", "203.0.113.5", 3),
        iface("Wi-Fi", "192.168.1.23", 4),
    ];
    assert_eq!(pairing_network(&v, None).unwrap().name, "Wi-Fi");
    assert_eq!(
        pairing_candidates(&v),
        ["Wi-Fi"],
        "a public address cannot pair"
    );
}

/// When the connection has both, the link-local IPv6 address (always there on Wi-Fi, and only
/// ever on this one network) is used rather than a private one.
#[test]
fn link_local_ipv6_is_preferred_to_a_private_one() {
    let v = vec![
        iface("en0", "192.168.0.9", 6),
        iface("en0", "fd12:3456::7", 6),
        iface("en0", "fe80::1", 6),
    ];
    let net = pairing_network(&v, Some("en0")).unwrap();
    assert_eq!(net.addrs[1], "[fe80::1%6]:0".parse::<SocketAddr>().unwrap());
}

/// The laptop this was found on: Proton VPN's tunnel, "ProTUN" (10.2.0.2/32), is listed before
/// the Wi-Fi. Judged by what the connection is (a single-address tunnel), not by its name, the
/// tunnel is never chosen.
#[test]
fn a_vpn_tunnel_listed_first_is_never_chosen() {
    let v = vec![
        tunnel("ProTUN", "10.2.0.2", 34),
        iface("Wi-Fi", "192.168.0.3", 13),
        iface("Loopback Pseudo-Interface 1", "127.0.0.1", 1),
    ];
    assert_eq!(pairing_candidates(&v), ["Wi-Fi"]);
    assert_eq!(pairing_network(&v, None).unwrap().name, "Wi-Fi");
    assert!(pairing_network(&v, Some("ProTUN")).is_none());
}

#[test]
fn a_single_address_or_point_to_point_connection_is_never_a_candidate() {
    let single = Interface {
        point_to_point: false,
        ..tunnel("Ethernet 3", "10.8.0.6", 21)
    };
    let p2p = Interface {
        prefix_len: Some(24),
        ..tunnel("Ethernet 4", "10.9.0.6", 22)
    };
    assert!(pairing_candidates(&[single, p2p]).is_empty());
}

#[test]
fn a_connection_that_is_down_is_never_a_candidate() {
    let down = Interface {
        up: false,
        ..iface("Wi-Fi", "192.168.1.5", 4)
    };
    assert!(pairing_candidates(&[down]).is_empty());
}

/// A VPN adapter that looks like an ordinary network (OpenVPN's "Ethernet 2", a /24) beside the
/// Wi-Fi: the Wi-Fi is the likely choice, by its name.
#[test]
fn beside_another_real_looking_connection_the_wifi_is_chosen() {
    let v = vec![
        iface("Ethernet 2", "10.8.0.6", 21),
        iface("Wi-Fi", "192.168.1.23", 4),
    ];
    assert_eq!(likely_wifi(&v).as_deref(), Some("Wi-Fi"));
    let linux = vec![
        iface("eth0", "192.168.1.9", 2),
        iface("wlp2s0", "192.168.1.10", 3),
    ];
    assert_eq!(likely_wifi(&linux).as_deref(), Some("wlp2s0"));
}

/// When it cannot tell (two wired connections, or a Mac's en0 and en1), PCTwin does not guess:
/// the person is asked which connection their Wi-Fi is on.
#[test]
fn when_it_cannot_tell_it_asks_instead_of_guessing() {
    let v = vec![
        iface("Ethernet", "192.168.1.9", 5),
        iface("Ethernet 2", "10.8.0.6", 21),
    ];
    assert_eq!(likely_wifi(&v), None);
    assert_eq!(pairing_candidates(&v), ["Ethernet", "Ethernet 2"]);
    assert!(pairing_network(&v, None).is_none());
    assert_eq!(
        pairing_network(&v, Some("Ethernet")).unwrap().name,
        "Ethernet"
    );
    let mac = vec![
        iface("en0", "192.168.1.9", 6),
        iface("en1", "192.168.1.10", 7),
    ];
    assert_eq!(likely_wifi(&mac), None);
}

/// A lone connection not named like Wi-Fi is offered, not assumed: when the real Wi-Fi has no
/// private address (a carrier-shared 100.64 address, say), the only candidate left may be a VPN
/// that looks like an ordinary network, so the person confirms it.
#[test]
fn a_lone_connection_not_named_like_wifi_is_offered_not_assumed() {
    let desk = vec![
        iface("Ethernet", "192.168.1.9", 5),
        tunnel("ProTUN", "10.2.0.2", 34),
    ];
    assert_eq!(likely_wifi(&desk), None);
    assert_eq!(pairing_candidates(&desk), ["Ethernet"]);
    let cgnat = vec![
        iface("Wi-Fi", "100.64.12.7", 4),
        iface("office", "10.20.0.5", 30),
    ];
    assert_eq!(
        likely_wifi(&cgnat),
        None,
        "the WireGuard tunnel is never assumed"
    );
    assert_eq!(pairing_candidates(&cgnat), ["office"]);
}

#[test]
fn linux_wifi_names_are_recognised() {
    for name in ["wlo1", "wls1", "wlp2s0", "wlx00c0ca", "wlan0"] {
        let v = vec![
            iface("eth0", "192.168.1.9", 2),
            iface(name, "192.168.1.10", 3),
        ];
        assert_eq!(likely_wifi(&v).as_deref(), Some(name), "{name}");
    }
}

/// Two connections named like Wi-Fi (a second adapter plugged in): it cannot tell, so it asks.
#[test]
fn two_wifi_connections_are_a_question_not_a_guess() {
    let v = vec![
        iface("Wi-Fi", "192.168.1.23", 4),
        iface("Wi-Fi 2", "192.168.50.7", 9),
    ];
    assert_eq!(likely_wifi(&v), None);
    assert_eq!(pairing_candidates(&v), ["Wi-Fi", "Wi-Fi 2"]);
}

/// When the system says which adapter is Wi-Fi, that decides, whatever it is called: a Mac's
/// `en0`, a German Windows' "WLAN", a Wi-Fi adapter someone renamed "Home". A connection named
/// "Wi-Fi" that the system says is not Wi-Fi is not taken for it (found by the how-and-why walk,
/// 20 October 2026: a Mac was always asked, in system names).
#[test]
fn the_systems_own_word_on_wifi_decides_over_the_name() {
    let said = |name: &str, ip: &str, index: u32, wifi: bool| Interface {
        wifi: Some(wifi),
        ..iface(name, ip, index)
    };
    let mac = vec![
        said("en0", "192.168.1.9", 6, true),
        said("en7", "192.168.1.10", 7, false),
    ];
    assert_eq!(likely_wifi(&mac).as_deref(), Some("en0"));
    let renamed = vec![
        said("Home", "192.168.1.9", 6, true),
        said("Ethernet", "192.168.1.10", 7, false),
    ];
    assert_eq!(likely_wifi(&renamed).as_deref(), Some("Home"));
    let misnamed = vec![said("Wi-Fi", "192.168.1.9", 6, false)];
    assert_eq!(likely_wifi(&misnamed), None);
    // Two Wi-Fi adapters: PCTwin still asks.
    let two = vec![
        said("en0", "192.168.1.9", 6, true),
        said("en1", "192.168.2.9", 8, true),
    ];
    assert_eq!(likely_wifi(&two), None);
}
