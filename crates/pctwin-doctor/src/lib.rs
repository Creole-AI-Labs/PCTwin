//! PCTwin connection doctor (Engineering Plan 9.3, screen C08): when the two laptops cannot find
//! or reach each other, work out the likely reason and the fix, in plain words.
//!
//! Probes ([`probe`]) look at this laptop: its network connections, whether it can hear its own
//! local-discovery announcement, whether sending to the discovery group is blocked (how a denied
//! macOS Local Network permission shows), and on Windows whether a network is set to Public. The
//! rules ([`diagnose`]) turn that [`Evidence`] into [`Finding`]s.
//!
//! Each finding is [`Certainty::Sure`] only when the evidence leaves no doubt; otherwise it is
//! [`Certainty::Likely`] and the screen offers the fix for every likely cause instead of one
//! confident guess. Evidence the probes could not gather is `None` and is never treated as a
//! problem. Messages are translation keys ([`Cause::message_key`], [`Cause::fix_key`]); the
//! screens hold the reviewed wording.

use std::net::IpAddr;

/// Which laptop is asking.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Role {
    /// The old laptop, showing its code and waiting.
    OldLaptop,
    /// The new laptop, looking for the old one.
    NewLaptop,
}

/// What kind of connection an interface is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InterfaceKind {
    /// A real network connection (Wi-Fi, Ethernet).
    Network,
    /// This laptop talking to itself.
    Loopback,
    /// A VPN.
    Vpn,
    /// A virtual machine, container or system-internal adapter.
    Virtual,
}

/// One network interface address on this laptop.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Interface {
    pub name: String,
    pub ip: IpAddr,
    pub kind: InterfaceKind,
}

impl Interface {
    /// An interface, classified from its address and name.
    pub fn new(name: &str, ip: IpAddr) -> Self {
        let kind = if ip.is_loopback() {
            InterfaceKind::Loopback
        } else {
            kind_from_name(name)
        };
        Self {
            name: name.to_string(),
            ip,
            kind,
        }
    }

    fn connects_to_a_network(&self) -> bool {
        self.kind == InterfaceKind::Network && is_usable_address(self.ip)
    }
}

const VPN_WORDS: &[&str] = &[
    "vpn",
    "tailscale",
    "wireguard",
    "zerotier",
    "nordlynx",
    "openvpn",
    "tap-windows",
    "wintun",
    "anyconnect",
    "globalprotect",
    "fortinet",
    "ipsec",
];
const VPN_PREFIXES: &[&str] = &["utun", "wg", "tun", "ppp", "tap", "ipsec"];
const VIRTUAL_WORDS: &[&str] = &[
    "vethernet",
    "vmware",
    "virtualbox",
    "hyper-v",
    "docker",
    "wsl",
    "vboxnet",
    "parallels",
];
const VIRTUAL_PREFIXES: &[&str] = &[
    "br-", "veth", "bridge", "virbr", "vmnet", "awdl", "llw", "anpi",
];

fn kind_from_name(name: &str) -> InterfaceKind {
    let n = name.to_lowercase();
    if n.contains("loopback") {
        InterfaceKind::Loopback
    } else if VPN_WORDS.iter().any(|w| n.contains(w))
        || VPN_PREFIXES.iter().any(|p| n.starts_with(p))
    {
        InterfaceKind::Vpn
    } else if VIRTUAL_WORDS.iter().any(|w| n.contains(w))
        || VIRTUAL_PREFIXES.iter().any(|p| n.starts_with(p))
    {
        InterfaceKind::Virtual
    } else {
        InterfaceKind::Network
    }
}

/// An address that means the laptop is actually on a network: not loopback, not a self-assigned
/// link-local address (which is what a laptop gets when no network answered), not unspecified.
fn is_usable_address(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(v4) => !(v4.is_loopback() || v4.is_link_local() || v4.is_unspecified()),
        IpAddr::V6(v6) => !(v6.is_loopback() || v6.is_unicast_link_local() || v6.is_unspecified()),
    }
}

/// How Windows treats a network. Public blocks other devices from connecting by default.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NetworkProfile {
    Private,
    Public,
    Domain,
}

/// What the probes saw. `None` means "could not tell" and is never treated as a problem.
#[derive(Debug, Clone, Default)]
pub struct Evidence {
    pub interfaces: Vec<Interface>,
    /// Whether this laptop heard its own local-discovery announcement.
    pub hears_itself: Option<bool>,
    /// Whether sending to the local-discovery group was refused by the system.
    pub multicast_send_blocked: Option<bool>,
    /// Windows network names and how Windows treats each. Empty elsewhere.
    pub windows_profiles: Vec<(String, NetworkProfile)>,
    /// On the new laptop: how many old laptops the last search found.
    pub old_laptops_found: Option<usize>,
    pub is_macos: bool,
}

/// A reason the laptops may not be able to find or reach each other.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Cause {
    /// This laptop is not connected to any network.
    NotConnected,
    /// macOS is not allowing PCTwin to talk to devices on the local network.
    MacLocalNetworkDenied,
    /// Something on this laptop (a firewall or security software) stops local discovery.
    DiscoveryBlockedOnThisLaptop,
    /// Windows treats this network as Public, which blocks other devices.
    WindowsPublicNetwork(String),
    /// A VPN is on and may hide the local network.
    VpnOn(Vec<String>),
    /// The old laptop is not on its pairing screen.
    OldLaptopNotPairing,
    /// The laptops are on different networks.
    DifferentNetworks,
    /// The network stops devices from seeing each other (guest, hotel, office Wi-Fi).
    GuestNetworkHidesDevices,
}

impl Cause {
    /// Translation key for what happened.
    pub fn message_key(&self) -> &'static str {
        match self {
            Self::NotConnected => "doctor.not_connected",
            Self::MacLocalNetworkDenied => "doctor.mac_local_network_denied",
            Self::DiscoveryBlockedOnThisLaptop => "doctor.discovery_blocked",
            Self::WindowsPublicNetwork(_) => "doctor.windows_public_network",
            Self::VpnOn(_) => "doctor.vpn_on",
            Self::OldLaptopNotPairing => "doctor.old_laptop_not_pairing",
            Self::DifferentNetworks => "doctor.different_networks",
            Self::GuestNetworkHidesDevices => "doctor.guest_network",
        }
    }

    /// Translation key for what to do.
    pub fn fix_key(&self) -> &'static str {
        match self {
            Self::NotConnected => "doctor.not_connected.fix",
            Self::MacLocalNetworkDenied => "doctor.mac_local_network_denied.fix",
            Self::DiscoveryBlockedOnThisLaptop => "doctor.discovery_blocked.fix",
            Self::WindowsPublicNetwork(_) => "doctor.windows_public_network.fix",
            Self::VpnOn(_) => "doctor.vpn_on.fix",
            Self::OldLaptopNotPairing => "doctor.old_laptop_not_pairing.fix",
            Self::DifferentNetworks => "doctor.different_networks.fix",
            Self::GuestNetworkHidesDevices => "doctor.guest_network.fix",
        }
    }
}

/// How sure the doctor is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Certainty {
    Sure,
    Likely,
}

/// One cause with how sure the doctor is about it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Finding {
    pub cause: Cause,
    pub certainty: Certainty,
}

/// The doctor's answer: findings, surest first, and whether to suggest a phone hotspot.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Diagnosis {
    pub findings: Vec<Finding>,
    /// Joining both laptops to a phone's hotspot sidesteps network problems.
    pub suggest_phone_hotspot: bool,
}

/// Turns what the probes saw into findings.
pub fn diagnose(e: &Evidence, role: Role) -> Diagnosis {
    let sure = |cause| Finding {
        cause,
        certainty: Certainty::Sure,
    };
    let likely = |cause| Finding {
        cause,
        certainty: Certainty::Likely,
    };

    if !e.interfaces.iter().any(Interface::connects_to_a_network) {
        // Nothing else is worth guessing at until the laptop is connected.
        return Diagnosis {
            findings: vec![sure(Cause::NotConnected)],
            suggest_phone_hotspot: false,
        };
    }

    let mut findings = Vec::new();
    if e.is_macos && e.multicast_send_blocked == Some(true) {
        findings.push(sure(Cause::MacLocalNetworkDenied));
    } else if e.hears_itself == Some(false) {
        findings.push(likely(Cause::DiscoveryBlockedOnThisLaptop));
    }
    for (name, profile) in &e.windows_profiles {
        if *profile == NetworkProfile::Public && kind_from_name(name) == InterfaceKind::Network {
            findings.push(likely(Cause::WindowsPublicNetwork(name.clone())));
        }
    }

    if role == Role::NewLaptop && e.old_laptops_found == Some(0) {
        let vpns: Vec<String> = e
            .interfaces
            .iter()
            .filter(|i| i.kind == InterfaceKind::Vpn)
            .map(|i| i.name.clone())
            .fold(Vec::new(), |mut names, n| {
                if !names.contains(&n) {
                    names.push(n);
                }
                names
            });
        if !vpns.is_empty() {
            findings.push(likely(Cause::VpnOn(vpns)));
        }
        if findings.is_empty() {
            // Nothing on this laptop explains it: name each likely cause with its fix.
            findings.push(likely(Cause::OldLaptopNotPairing));
            findings.push(likely(Cause::DifferentNetworks));
            findings.push(likely(Cause::GuestNetworkHidesDevices));
        }
    }

    // Findings are added surest first: only the checks above can be Sure.
    let suggest_phone_hotspot = findings.iter().any(|f| {
        matches!(
            f.cause,
            Cause::DifferentNetworks | Cause::GuestNetworkHidesDevices | Cause::VpnOn(_)
        )
    });
    Diagnosis {
        findings,
        suggest_phone_hotspot,
    }
}

/// Reads `alias|Category` lines as printed by the Windows probe. Lines that are not exactly that
/// are skipped.
pub fn parse_windows_profiles(output: &str) -> Vec<(String, NetworkProfile)> {
    output
        .lines()
        .filter_map(|line| {
            let (alias, category) = line.trim().rsplit_once('|')?;
            let profile = match category.trim() {
                "Private" => NetworkProfile::Private,
                "Public" => NetworkProfile::Public,
                "DomainAuthenticated" => NetworkProfile::Domain,
                _ => return None,
            };
            let alias = alias.trim();
            (!alias.is_empty()).then(|| (alias.to_string(), profile))
        })
        .collect()
}

/// Probes that look at this laptop. Each returns "could not tell" rather than failing.
pub mod probe {
    use std::net::{Ipv4Addr, SocketAddrV4, UdpSocket};
    use std::time::{Duration, Instant};

    use mdns_sd::{ServiceDaemon, ServiceEvent, ServiceInfo};

    use super::{Evidence, Interface, NetworkProfile, Role};

    /// A service type used only for this check, so other laptops never list it.
    const CHECK_SERVICE: &str = "_pctwin-check._tcp.local.";

    /// This laptop's interface addresses.
    pub fn interfaces() -> Vec<Interface> {
        if_addrs::get_if_addrs()
            .map(|list| {
                list.iter()
                    .map(|i| Interface::new(&i.name, i.ip()))
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Whether this laptop hears its own local-discovery announcement within `wait`. `Some(false)`
    /// if discovery cannot start or the announcement never comes back.
    pub fn hears_itself(wait: Duration) -> Option<bool> {
        let Ok(daemon) = ServiceDaemon::new() else {
            return Some(false);
        };
        let mut tag = [0u8; 8];
        getrandom::fill(&mut tag).ok()?;
        let tag: String = tag.iter().map(|b| format!("{b:02x}")).collect();
        let instance = format!("pctwin-check-{tag}");
        let heard = (|| {
            let info = ServiceInfo::new(
                CHECK_SERVICE,
                &instance,
                &format!("{instance}.local."),
                "",
                9,
                &[("v", "1")][..],
            )
            .ok()?
            .enable_addr_auto();
            let fullname = info.get_fullname().to_string();
            daemon.register(info).ok()?;
            let events = daemon.browse(CHECK_SERVICE).ok()?;
            let deadline = Instant::now() + wait;
            let mut heard = false;
            while let Ok(event) =
                events.recv_timeout(deadline.saturating_duration_since(Instant::now()))
            {
                if let ServiceEvent::ServiceResolved(found) = event
                    && found.get_fullname() == fullname
                {
                    heard = true;
                    break;
                }
            }
            let _ = daemon.unregister(&fullname);
            Some(heard)
        })();
        let _ = daemon.shutdown();
        Some(heard.unwrap_or(false))
    }

    /// Whether the system refuses a send to the local-discovery group, which is how a denied macOS
    /// Local Network permission shows. `None` if the check itself could not run.
    pub fn multicast_send_blocked() -> Option<bool> {
        let socket = UdpSocket::bind(SocketAddrV4::new(Ipv4Addr::UNSPECIFIED, 0)).ok()?;
        // A minimal, well-formed DNS query with no questions: harmless to every listener.
        let query = [0u8; 12];
        match socket.send_to(
            &query,
            SocketAddrV4::new(Ipv4Addr::new(224, 0, 0, 251), 5353),
        ) {
            Ok(_) => Some(false),
            Err(e) => match e.kind() {
                std::io::ErrorKind::HostUnreachable
                | std::io::ErrorKind::NetworkUnreachable
                | std::io::ErrorKind::PermissionDenied => Some(true),
                _ => None,
            },
        }
    }

    /// Windows network names and how Windows treats each. Empty on other systems or on failure.
    pub fn windows_profiles() -> Vec<(String, NetworkProfile)> {
        if !cfg!(windows) {
            return Vec::new();
        }
        let output = std::process::Command::new("powershell")
            .args([
                "-NoProfile",
                "-NonInteractive",
                "-Command",
                "Get-NetConnectionProfile | ForEach-Object { $_.InterfaceAlias + '|' + $_.NetworkCategory }",
            ])
            .output();
        match output {
            Ok(out) if out.status.success() => {
                super::parse_windows_profiles(&String::from_utf8_lossy(&out.stdout))
            }
            _ => Vec::new(),
        }
    }

    /// Runs every probe. `old_laptops_found` is the new laptop's last search result.
    pub fn gather(role: Role, old_laptops_found: Option<usize>) -> Evidence {
        Evidence {
            interfaces: interfaces(),
            hears_itself: hears_itself(Duration::from_secs(3)),
            multicast_send_blocked: multicast_send_blocked(),
            windows_profiles: windows_profiles(),
            old_laptops_found: if role == Role::NewLaptop {
                old_laptops_found
            } else {
                None
            },
            is_macos: cfg!(target_os = "macos"),
        }
    }
}
