//! PCTwin session: joins discovery and pairing into the journey up to the number pick.
//!
//! - [`OldLaptop::start`] opens the pairing listener and announces the laptop's label on the local
//!   network. Dropping the [`OldLaptop`] (leaving the pairing screen) says goodbye and stops
//!   listening, so the laptop is only visible and reachable while pairing.
//! - [`search`] lists old laptops nearby; the person taps the one whose label matches.
//! - [`connect_to`] connects to that laptop only. It tries the laptop's addresses in order and moves
//!   to the next one only when an address could not be reached at all, so a code is never tried
//!   twice and a stranger's laptop is never charged a guess.

use std::net::{IpAddr, SocketAddr, SocketAddrV4, SocketAddrV6};
use std::sync::Mutex;
use std::time::Duration;

use pctwin_discovery::{Announcement, Discovery, DiscoveryError, Found, Label};
use pctwin_doctor::{Interface, InterfaceKind};
use pctwin_link::{GuestPending, Host, HostPending, LinkConfig, LinkError, connect};
use pctwin_pairing::{PairingCode, RotatingSender};

/// Why a session step failed.
#[derive(Debug, thiserror::Error)]
pub enum SessionError {
    #[error(transparent)]
    Discovery(#[from] DiscoveryError),
    #[error("could not start listening for the new laptop: {0}")]
    Listen(std::io::Error),
    #[error("searching for nearby laptops stopped unexpectedly")]
    Search,
    #[error("this laptop is not on a Wi-Fi or wired network it can pair on")]
    NoNetwork,
    /// PCTwin cannot tell which connection is the Wi-Fi (several could be, or the only one is not
    /// named like Wi-Fi): the person picks or confirms one (by its name, with
    /// [`OldLaptop::start_on`]); PCTwin does not guess.
    #[error("choose the network the other laptop is on")]
    ChooseNetwork(Vec<String>),
}

/// The connection the old laptop pairs on: its name and its own local-network addresses (one
/// IPv4, one IPv6), with port 0.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Network {
    pub name: String,
    pub addrs: Vec<SocketAddr>,
}

/// Whether `i` is a connection pairing can run over: running, not a VPN, virtual machine or
/// loopback by its name, not a tunnel to a single other end, with an address on a shared network
/// (not a single-address /32, as VPN tunnels have). Judged by what the system reports about the
/// connection, so a VPN with an ordinary-looking name (Proton's "ProTUN") is still left out.
fn can_pair_on(i: &Interface) -> bool {
    i.kind == InterfaceKind::Network
        && i.up
        && !i.point_to_point
        && i.prefix_len != Some(32)
        && i.prefix_len != Some(128)
}

/// The connections this laptop could pair on, by name, in the order the system lists them: each
/// has a private IPv4 address on a shared network (see `can_pair_on`).
pub fn pairing_candidates(interfaces: &[Interface]) -> Vec<String> {
    let mut names: Vec<String> = Vec::new();
    for i in interfaces.iter().filter(|i| can_pair_on(i)) {
        if matches!(i.ip, IpAddr::V4(v4) if v4.is_private()) && !names.contains(&i.name) {
            names.push(i.name.clone());
        }
    }
    names
}

/// The connection to pair on without asking: the only candidate the system says is Wi-Fi (by the
/// adapter's kind: a Mac's `en0`, a German Windows' "WLAN" and a renamed adapter all count); where
/// the system could not say, the only candidate named like Wi-Fi (Wi-Fi, WLAN, wireless, wlan0,
/// wlp2s0, wlo1). `None` otherwise, even with a single candidate: a lone wired or unrecognised
/// connection may be a VPN or overlay network that looks ordinary (when the real Wi-Fi has no
/// private address), so the person confirms it rather than PCTwin assuming it.
pub fn likely_wifi(interfaces: &[Interface]) -> Option<String> {
    let candidates = pairing_candidates(interfaces);
    let told = interfaces.iter().any(|i| i.wifi.is_some());
    let is_wifi = |name: &String| {
        if told {
            interfaces
                .iter()
                .any(|i| i.name == *name && i.wifi == Some(true))
        } else {
            looks_like_wifi(name)
        }
    };
    let wifi: Vec<&String> = candidates.iter().filter(|n| is_wifi(n)).collect();
    match wifi.as_slice() {
        [only] => Some((*only).clone()),
        _ => None,
    }
}

fn looks_like_wifi(name: &str) -> bool {
    let n = name.to_lowercase();
    ["wi-fi", "wifi", "wlan", "wireless"]
        .iter()
        .any(|w| n.contains(w))
        || ["wlp", "wlx", "wlo", "wls"]
            .iter()
            .any(|p| n.starts_with(p))
}

/// The connection to pair on: the one named `chosen` (one of [`pairing_candidates`]), or else
/// [`likely_wifi`]. Only its own local-network addresses: a private or link-local IPv4 address,
/// and an IPv6 link-local address (with the connection's number, without which it cannot be
/// used) or else a private one. `None` if there is no such connection, it cannot be told which
/// one, or it has no such address.
pub fn pairing_network(interfaces: &[Interface], chosen: Option<&str>) -> Option<Network> {
    let name = match chosen {
        Some(name) => name.to_string(),
        None => likely_wifi(interfaces)?,
    };
    let own: Vec<&Interface> = interfaces
        .iter()
        .filter(|i| can_pair_on(i))
        .filter(|i| i.name == name)
        .collect();
    let v4 = own.iter().find_map(|i| match i.ip {
        IpAddr::V4(a) if a.is_private() || a.is_link_local() => {
            Some(SocketAddr::V4(SocketAddrV4::new(a, 0)))
        }
        _ => None,
    });
    let link_local = own.iter().find_map(|i| match (i.ip, i.index) {
        (IpAddr::V6(a), Some(scope)) if a.is_unicast_link_local() => {
            Some(SocketAddr::V6(SocketAddrV6::new(a, 0, 0, scope)))
        }
        _ => None,
    });
    let private = own.iter().find_map(|i| match i.ip {
        IpAddr::V6(a) if a.is_unique_local() => Some(SocketAddr::V6(SocketAddrV6::new(a, 0, 0, 0))),
        _ => None,
    });
    let addrs: Vec<SocketAddr> = v4.into_iter().chain(link_local.or(private)).collect();
    (!addrs.is_empty()).then_some(Network { name, addrs })
}

/// The old laptop while it is pairing: listening and announcing its label.
pub struct OldLaptop {
    /// What is announced, and the label it shows: changed together (Shuffle, Change name) while
    /// the laptop keeps serving.
    shown: Mutex<(Announcement, Label)>,
    discovery: Discovery,
    host: Host,
    /// The connection paired on, and its addresses being listened on.
    network: String,
    ips: Vec<IpAddr>,
}

impl OldLaptop {
    /// Starts pairing on the likely Wi-Fi connection ([`likely_wifi`]): see
    /// [`start_on`](Self::start_on). When it cannot tell which connection that is, fails with
    /// [`SessionError::ChooseNetwork`] listing the candidates, so the app asks the person.
    pub async fn start(label: Label, config: LinkConfig) -> Result<Self, SessionError> {
        let interfaces = pctwin_doctor::probe::interfaces().ok_or(SessionError::NoNetwork)?;
        let Some(network) = pairing_network(&interfaces, None) else {
            let candidates = pairing_candidates(&interfaces);
            return Err(if candidates.is_empty() {
                SessionError::NoNetwork
            } else {
                SessionError::ChooseNetwork(candidates)
            });
        };
        Self::start_on(label, config, &network).await
    }

    /// Starts listening on `network`'s own addresses only (one port for all) and announces
    /// `label` there with those addresses. No other connection of this laptop (a VPN, a virtual
    /// machine's network) can reach the pairing port. If the IPv6 address cannot be used, IPv4
    /// alone is.
    pub async fn start_on(
        label: Label,
        config: LinkConfig,
        network: &Network,
    ) -> Result<Self, SessionError> {
        let host = match Host::bind_all(&network.addrs, config).await {
            Ok(host) => host,
            Err(_) if network.addrs.len() > 1 => Host::bind_all(&network.addrs[..1], config)
                .await
                .map_err(SessionError::Listen)?,
            Err(e) => return Err(SessionError::Listen(e)),
        };
        let addrs = host.local_addrs();
        let port = host.local_addr().map_err(SessionError::Listen)?.port();
        let ips: Vec<IpAddr> = addrs.iter().map(SocketAddr::ip).collect();
        let discovery = Discovery::new()?;
        let announcement = discovery.announce_on(&label, port, &ips, &network.name)?;
        Ok(Self {
            shown: Mutex::new((announcement, label)),
            discovery,
            host,
            network: network.name.clone(),
            ips,
        })
    }

    /// The label being announced, to show on screen.
    pub fn label(&self) -> Label {
        self.shown
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .1
            .clone()
    }

    /// Announces a different label (Shuffle or Change name) and withdraws the previous one, while
    /// the laptop keeps serving. For about a third of a second a search may still list both
    /// (multicast DNS's goodbye delay).
    pub fn relabel(&self, label: Label) -> Result<(), SessionError> {
        let port = self.host.local_addr().map_err(SessionError::Listen)?.port();
        let fresh = self
            .discovery
            .announce_on(&label, port, &self.ips, &self.network)?;
        // Replacing the announcement drops the old one, which says goodbye.
        *self
            .shown
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner) = (fresh, label);
        Ok(())
    }

    /// Serves new laptops one at a time until one reaches the number pick (see [`Host::next_peer`]).
    pub async fn next_peer(
        &self,
        sender: &Mutex<RotatingSender>,
    ) -> Result<HostPending, LinkError> {
        self.host.next_peer(sender).await
    }
}

/// Looks for old laptops nearby for `wait`.
pub async fn search(wait: Duration) -> Result<Vec<Found>, SessionError> {
    tokio::task::spawn_blocking(move || Discovery::new()?.browse(wait))
        .await
        .map_err(|_| SessionError::Search)?
        .map_err(SessionError::from)
}

/// Connects to the old laptop the person chose, with the code they typed. Addresses are tried in
/// order; the next one is tried only if the previous could not be reached at all.
pub async fn connect_to(
    chosen: &Found,
    code: &PairingCode,
    config: LinkConfig,
) -> Result<GuestPending, LinkError> {
    for addr in &chosen.addrs {
        match connect(*addr, code, config).await {
            Err(LinkError::Unreachable) => continue,
            result => return result,
        }
    }
    Err(LinkError::Unreachable)
}
