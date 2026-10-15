//! Which connection the old laptop pairs on (Task List 1.2): only the chosen Wi-Fi (or wired)
//! connection's own addresses, never every connection, so a VPN or a virtual machine's network
//! cannot reach the pairing port. One IPv4 and one IPv6 address of that connection, both local.

use std::net::{IpAddr, SocketAddr};

use pctwin_doctor::Interface;
use pctwin_session::pairing_network;

fn iface(name: &str, ip: &str, index: u32) -> Interface {
    Interface {
        index: Some(index),
        ..Interface::new(name, ip.parse::<IpAddr>().unwrap())
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
    let net = pairing_network(&v, None).unwrap();
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
    let net = pairing_network(&v, None).unwrap();
    assert_eq!(net.addrs[1], "[fe80::1%6]:0".parse::<SocketAddr>().unwrap());
}
