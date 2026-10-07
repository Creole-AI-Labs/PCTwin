//! PCTwin connection doctor (Engineering Plan 9.3, screen C08): when the two laptops cannot find
//! or reach each other, work out the likely reason and the fix, in plain words.
//!
//! Probes ([`probe`]) look at this laptop: its network connections, whether it can hear its own
//! local-discovery announcement, whether sending to the discovery group is refused (one way a
//! denied macOS Local Network permission shows), and on Windows whether a network is set to
//! Public. The rules ([`diagnose`]) turn that [`Evidence`] into [`Finding`]s.
//!
//! The doctor only says [`Certainty::Sure`] when the laptop has no network address of any kind.
//! Everything else is [`Certainty::Likely`], and the screen offers the fix for every likely cause
//! instead of one confident guess. Evidence the probes could not gather is `None` and is never
//! treated as a problem; evidence that discovery works (the old laptop was found, or this laptop
//! hears itself) rules out the discovery causes. A connection only counts when it has a usable
//! address, so built-in adapters that only carry internal link-local addresses (such as the
//! `utun` adapters every Mac has) are never mistaken for a VPN. Messages are translation keys
//! ([`Cause::message_key`], [`Cause::fix_key`]); the screens hold the reviewed wording.

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

    /// Carries an address that means it is actually connected somewhere.
    fn is_live(&self) -> bool {
        self.kind != InterfaceKind::Loopback && is_usable_address(self.ip)
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
    "mullvad",
    "cloudflarewarp",
    "warp",
    "hamachi",
    "proton",
    "nebula",
];
const VPN_PREFIXES: &[&str] = &["utun", "wg", "tun", "ppp", "tap", "ipsec", "zt"];
const VIRTUAL_WORDS: &[&str] = &[
    "vethernet",
    "vmware",
    "virtualbox",
    "hyper-v",
    "docker",
    "wsl",
    "vboxnet",
    "parallels",
    "npcap",
    "bluetooth",
    "local area connection*",
];
const VIRTUAL_PREFIXES: &[&str] = &[
    "br-", "veth", "bridge", "virbr", "vmnet", "awdl", "llw", "anpi", "lxcbr", "lxdbr", "incusbr",
    "podman", "cni", "flannel", "cilium", "gif", "stf",
];

fn kind_from_name(name: &str) -> InterfaceKind {
    let n = name.to_lowercase();
    if n.contains("loopback") {
        InterfaceKind::Loopback
    } else if n.contains("vethernet") && n.contains("external") {
        // A Hyper-V external switch carries the laptop's real network.
        InterfaceKind::Network
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
/// link-local address (what a laptop gets when no network answered), not unspecified.
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
    /// This laptop's interface addresses, or `None` if they could not be listed.
    pub interfaces: Option<Vec<Interface>>,
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
    /// macOS may not be allowing PCTwin to talk to devices on the local network.
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
    let finding = |cause, certainty| Finding { cause, certainty };
    let likely = |cause| finding(cause, Certainty::Likely);
    // Discovery works if the old laptop was found or this laptop hears itself.
    // (A search result only means something on the new laptop.)
    let found_some = role == Role::NewLaptop && matches!(e.old_laptops_found, Some(n) if n > 0);

    // Finding the old laptop proves a working connection, whatever the addresses look like (a
    // direct cable gives only self-assigned addresses).
    if let Some(interfaces) = e.interfaces.as_ref().filter(|_| !found_some) {
        let live: Vec<&Interface> = interfaces.iter().filter(|i| i.is_live()).collect();
        if live.is_empty() {
            // No address of any kind: the one thing the doctor can be sure of.
            return Diagnosis {
                findings: vec![finding(Cause::NotConnected, Certainty::Sure)],
                suggest_phone_hotspot: false,
            };
        }
        if !live.iter().any(|i| i.kind == InterfaceKind::Network) {
            // Only VPN or virtual connections: probably not on a real network.
            return Diagnosis {
                findings: vec![likely(Cause::NotConnected)],
                suggest_phone_hotspot: false,
            };
        }
    }

    let mut findings = Vec::new();
    if !found_some && e.hears_itself != Some(true) {
        if e.is_macos && e.multicast_send_blocked == Some(true) {
            findings.push(likely(Cause::MacLocalNetworkDenied));
        } else if e.hears_itself == Some(false) {
            findings.push(likely(Cause::DiscoveryBlockedOnThisLaptop));
        }
    }
    for (name, profile) in &e.windows_profiles {
        if *profile == NetworkProfile::Public && kind_from_name(name) == InterfaceKind::Network {
            findings.push(likely(Cause::WindowsPublicNetwork(name.clone())));
        }
    }

    if role == Role::NewLaptop && e.old_laptops_found == Some(0) {
        let mut vpns: Vec<String> = Vec::new();
        for i in e.interfaces.iter().flatten() {
            if i.kind == InterfaceKind::Vpn && i.is_live() && !vpns.contains(&i.name) {
                vpns.push(i.name.clone());
            }
        }
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
/// are skipped. The category is after the last `|`, so a name may itself contain `|`.
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
    use std::sync::mpsc;
    use std::time::{Duration, Instant};

    use mdns_sd::{ServiceDaemon, ServiceEvent, ServiceInfo};

    use super::{Evidence, Interface, NetworkProfile, Role};

    /// A service type used only for this check, so other laptops never list it.
    const CHECK_SERVICE: &str = "_pctwin-check._tcp.local.";
    /// Longest wait for the Windows profile check.
    pub const WINDOWS_PROBE_TIMEOUT: Duration = Duration::from_secs(10);

    /// This laptop's interface addresses, or `None` if they could not be listed.
    pub fn interfaces() -> Option<Vec<Interface>> {
        if_addrs::get_if_addrs().ok().map(|list| {
            list.iter()
                .map(|i| Interface::new(&i.name, i.ip()))
                .collect()
        })
    }

    /// Whether this laptop hears its own local-discovery announcement within `wait`. `Some(false)`
    /// only when the announcement went out and never came back; `None` if the check could not run.
    pub fn hears_itself(wait: Duration) -> Option<bool> {
        let daemon = ServiceDaemon::new().ok()?;
        let heard = listen_for_self(&daemon, wait);
        let _ = daemon.shutdown();
        heard
    }

    fn listen_for_self(daemon: &ServiceDaemon, wait: Duration) -> Option<bool> {
        let mut tag = [0u8; 8];
        getrandom::fill(&mut tag).ok()?;
        let tag: String = tag.iter().map(|b| format!("{b:02x}")).collect();
        let instance = format!("pctwin-check-{tag}");
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
        let events = daemon.browse(CHECK_SERVICE).ok()?;
        daemon.register(info).ok()?;
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
    }

    /// Whether the system refuses a send to the local-discovery group. A denied macOS Local Network
    /// permission is one cause of that. `None` if the check itself could not run or the answer is
    /// ambiguous (such as no IPv4 route on an IPv6-only network).
    pub fn multicast_send_blocked() -> Option<bool> {
        let socket = UdpSocket::bind(SocketAddrV4::new(Ipv4Addr::UNSPECIFIED, 0)).ok()?;
        // A minimal, well-formed DNS query with no questions: harmless to every listener.
        let query = [0u8; 12];
        match socket.send_to(
            &query,
            SocketAddrV4::new(Ipv4Addr::new(224, 0, 0, 251), 5353),
        ) {
            Ok(_) => Some(false),
            Err(e) => send_error_means_blocked(e.kind()),
        }
    }

    /// Which send errors mean "the system refused": only a refusal, not a missing route.
    pub fn send_error_means_blocked(kind: std::io::ErrorKind) -> Option<bool> {
        match kind {
            std::io::ErrorKind::HostUnreachable | std::io::ErrorKind::PermissionDenied => {
                Some(true)
            }
            _ => None,
        }
    }

    /// The PowerShell script the Windows probe runs. It asks for UTF-8 output so names in any
    /// language arrive intact.
    pub const WINDOWS_PROBE_SCRIPT: &str = "[Console]::OutputEncoding = [System.Text.Encoding]::UTF8; \
         Get-NetConnectionProfile | ForEach-Object { $_.InterfaceAlias + '|' + $_.NetworkCategory }";
    /// Windows process flag that stops the probe opening a console window.
    pub const CREATE_NO_WINDOW: u32 = 0x0800_0000;

    /// Windows network names and how Windows treats each. `None` on other systems, on failure, or
    /// if the check takes longer than [`WINDOWS_PROBE_TIMEOUT`] (then the check is stopped): the
    /// doctor treats that as "could not tell", never as "no Public network".
    pub fn windows_profiles() -> Option<Vec<(String, NetworkProfile)>> {
        windows_profiles_within(WINDOWS_PROBE_TIMEOUT)
    }

    /// [`windows_profiles`] with a chosen time limit.
    pub fn windows_profiles_within(limit: Duration) -> Option<Vec<(String, NetworkProfile)>> {
        if !cfg!(windows) {
            return None;
        }
        run_check(
            "powershell",
            &[
                "-NoProfile",
                "-NonInteractive",
                "-Command",
                WINDOWS_PROBE_SCRIPT,
            ],
            limit,
        )
        .map(|text| super::parse_windows_profiles(&text))
    }

    /// Runs a system check and returns what it printed, or `None` if it failed or took longer than
    /// `limit` (then it is stopped, never left running).
    pub fn run_check(program: &str, args: &[&str], limit: Duration) -> Option<String> {
        let mut command = std::process::Command::new(program);
        command
            .args(args)
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::null());
        hide_console_window(&mut command);
        let mut child = command.spawn().ok()?;
        // Read the output on its own thread so a full pipe can never stall the check.
        let mut stdout = child.stdout.take()?;
        let (tx, rx) = mpsc::channel();
        std::thread::spawn(move || {
            let mut text = String::new();
            let _ = std::io::Read::read_to_string(&mut stdout, &mut text);
            let _ = tx.send(text);
        });
        let deadline = Instant::now() + limit;
        loop {
            match child.try_wait() {
                Ok(Some(status)) => {
                    let text = rx.recv_timeout(Duration::from_secs(1)).ok()?;
                    return status.success().then_some(text);
                }
                Ok(None) if Instant::now() < deadline => {
                    std::thread::sleep(Duration::from_millis(25));
                }
                _ => {
                    // Too slow or unreadable: stop it rather than leave it running.
                    let _ = child.kill();
                    let _ = child.wait();
                    return None;
                }
            }
        }
    }

    #[cfg(windows)]
    fn hide_console_window(command: &mut std::process::Command) {
        use std::os::windows::process::CommandExt;
        command.creation_flags(CREATE_NO_WINDOW);
    }

    #[cfg(not(windows))]
    fn hide_console_window(_command: &mut std::process::Command) {}

    /// Runs every probe. `old_laptops_found` is the new laptop's last search result.
    pub fn gather(role: Role, old_laptops_found: Option<usize>) -> Evidence {
        Evidence {
            interfaces: interfaces(),
            hears_itself: hears_itself(Duration::from_secs(3)),
            multicast_send_blocked: multicast_send_blocked(),
            windows_profiles: windows_profiles().unwrap_or_default(),
            old_laptops_found: if role == Role::NewLaptop {
                old_laptops_found
            } else {
                None
            },
            is_macos: cfg!(target_os = "macos"),
        }
    }
}
