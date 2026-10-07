//! PCTwin session: joins discovery and pairing into the journey up to the number pick.
//!
//! - [`OldLaptop::start`] opens the pairing listener and announces the laptop's label on the local
//!   network. Dropping the [`OldLaptop`] (leaving the pairing screen) says goodbye and stops
//!   listening, so the laptop is only visible and reachable while pairing.
//! - [`search`] lists old laptops nearby; the person taps the one whose label matches.
//! - [`connect_to`] connects to that laptop only. It tries the laptop's addresses in order and moves
//!   to the next one only when an address could not be reached at all, so a code is never tried
//!   twice and a stranger's laptop is never charged a guess.

use std::net::SocketAddr;
use std::sync::Mutex;
use std::time::Duration;

use pctwin_discovery::{Announcement, Discovery, DiscoveryError, Found, Label};
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
}

/// The old laptop while it is pairing: listening and announcing its label.
pub struct OldLaptop {
    announcement: Announcement,
    discovery: Discovery,
    host: Host,
    label: Label,
}

impl OldLaptop {
    /// Starts listening on every interface and announces `label` with the listening port. Peers outside
    /// the local network are refused by the link; devices on other private networks this laptop is on
    /// (a VPN, a virtual machine) can still reach the port while pairing, and still need the code and
    /// the person's pick. Binding only the Wi-Fi interface is a known follow-up.
    pub async fn start(label: Label, config: LinkConfig) -> Result<Self, SessionError> {
        let any: SocketAddr = SocketAddr::from(([0, 0, 0, 0], 0));
        let host = Host::bind(any, config)
            .await
            .map_err(SessionError::Listen)?;
        let port = host.local_addr().map_err(SessionError::Listen)?.port();
        let discovery = Discovery::new()?;
        let announcement = discovery.announce(&label, port)?;
        Ok(Self {
            announcement,
            discovery,
            host,
            label,
        })
    }

    /// The label being announced, to show on screen.
    pub fn label(&self) -> &Label {
        &self.label
    }

    /// Announces a different label (Shuffle or Change name) and withdraws the previous one. For about a
    /// third of a second a search may still list both (multicast DNS's goodbye delay).
    pub fn relabel(&mut self, label: Label) -> Result<(), SessionError> {
        let port = self.host.local_addr().map_err(SessionError::Listen)?.port();
        let fresh = self.discovery.announce(&label, port)?;
        // Replacing the announcement drops the old one, which says goodbye.
        self.announcement = fresh;
        self.label = label;
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
