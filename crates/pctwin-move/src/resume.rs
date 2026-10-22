//! The way back after a dropped connection (Security Design A, "Resume"; Task List, the reconnect
//! loop). Once the person approves the plan, both laptops make resume keys from their pairing for
//! exactly that plan ([`plan_print`]); the old laptop starts listening for the new laptop's return
//! and tells it where ([`say_port`]); if the main connection drops, the new laptop reconnects with
//! its keys, no code, and both carry on where they stopped. Before the approval there are no keys,
//! so a drop ends the move as it always did.
//!
//! The move only sees the two sides through [`Comeback`] (old laptop) and [`Redial`] (new laptop);
//! [`NetComeback`] and [`NetRedial`] are the real ones, over `pctwin-link`. No key or handshake is
//! made here: the keys are `pctwin-pairing`'s ([`pctwin_pairing::LaneKeys::resume_keys`]), the
//! handshake `pctwin-link`'s.

use std::future::Future;
use std::net::SocketAddr;

use pctwin_link::{Arrival, LaneKeys, LaneListener, Link, LinkConfig, LinkError, ResumeKeys};
use pctwin_pairing::{PairingError, RESUME_LIFETIME};
use pctwin_record::{Item, ItemKind};
use pctwin_transfer::{Channel, ChannelError, TransferError};

use crate::landing::LandingRules;

/// The message kind of the old laptop's word on where it waits for the new laptop's return, kept
/// apart from the transfer's (1 to 20), the inventory's (32, 33, 35), the version's (34), the link
/// test's (36, 37), the landing rules' (39 to 41) and the greeting's (42 to 44).
const RESUME_AT: u8 = 45;

/// What the old laptop's side of a resume needs.
pub trait Comeback {
    type Link: Channel;

    /// The plan whose fingerprint is `plan` was approved: make the resume keys for it and start
    /// listening for the new laptop's return. The port it listens on, or `None` when it cannot
    /// (then a drop ends the move). Called once, before [`returned`](Self::returned).
    fn open(&mut self, plan: &[u8; 32]) -> impl Future<Output = Option<u16>>;

    /// The new laptop's return after a drop, as the move's new main connection; `None` once it
    /// can no longer come back (the keys ran out), and from then on. May be cancelled and called
    /// again.
    fn returned(&mut self) -> impl Future<Output = Option<Self::Link>>;
}

/// Why a try to come back did not bring a connection.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Redialed {
    /// Nothing answered, or not as it should: try again later.
    NotThere,
    /// The resume keys ran out of time or tries: only pairing again goes on from here.
    Over,
}

/// What the new laptop's side of a resume needs.
pub trait Redial {
    type Link: Channel;

    /// The plan whose fingerprint is `plan` was approved and the old laptop waits for this one's
    /// return on `port`: make the resume keys for it. False when it cannot (then a drop ends the
    /// move).
    fn prepare(&mut self, plan: &[u8; 32], port: u16) -> bool;

    /// One try to come back to the old laptop.
    fn redial(&mut self) -> impl Future<Output = Result<Self::Link, Redialed>>;
}

/// A connection that never exists, for a side with no way back.
pub enum NoLink {}

impl Channel for NoLink {
    async fn send(&mut self, _: &[u8]) -> Result<(), ChannelError> {
        match *self {}
    }
    async fn recv(&mut self) -> Result<Vec<u8>, ChannelError> {
        match *self {}
    }
}

/// An old laptop with no way back (the test lab): a drop after the approval ends the move.
pub struct NoComeback;

impl Comeback for NoComeback {
    type Link = NoLink;
    async fn open(&mut self, _: &[u8; 32]) -> Option<u16> {
        None
    }
    async fn returned(&mut self) -> Option<NoLink> {
        None
    }
}

/// A new laptop with no way back.
pub struct NoRedial;

impl Redial for NoRedial {
    type Link = NoLink;
    fn prepare(&mut self, _: &[u8; 32], _: u16) -> bool {
        false
    }
    async fn redial(&mut self) -> Result<NoLink, Redialed> {
        Err(Redialed::Over)
    }
}

/// The fingerprint of the approved plan as both laptops hold it, which the resume keys are made
/// for: every file of the old laptop's list as it was sent (by its ID, kind, size and whether the
/// scan included it), then the new laptop's landing rules as sent. Each laptop works it out from
/// what crossed the connection, so both get the same one, and keys made for another plan never
/// open a way back for this one. BLAKE3 in its key-derivation mode, with a context of its own.
pub fn plan_print(items: &[Item], rules: &LandingRules) -> [u8; 32] {
    finish_print(print_items(items), rules)
}

/// The first part of [`plan_print`]: the list. The new laptop takes it before its person changes
/// anything, and finishes it with the rules once they are sent.
pub(crate) fn print_items(items: &[Item]) -> blake3::Hasher {
    let mut h = blake3::Hasher::new_derive_key("PCTwin 2026-10-22 resume plan v1");
    h.update(&(items.len() as u64).to_be_bytes());
    for i in items {
        let id = i.id.to_hex();
        h.update(&(id.len() as u64).to_be_bytes());
        h.update(id.as_bytes());
        let kind: u8 = match i.kind {
            ItemKind::File => 1,
            _ => 2,
        };
        let included = u8::from(i.inclusion == pctwin_record::Inclusion::Included);
        h.update(&[kind, included]);
        h.update(&i.size_bytes.to_be_bytes());
    }
    h
}

pub(crate) fn finish_print(mut h: blake3::Hasher, rules: &LandingRules) -> [u8; 32] {
    // The rules are sent as exactly this JSON (fields in a fixed order, maps sorted).
    let said = serde_json::to_vec(rules).unwrap_or_default();
    h.update(&(said.len() as u64).to_be_bytes());
    h.update(&said);
    *h.finalize().as_bytes()
}

/// The old laptop, once the plan is approved: where it waits for the new laptop's return
/// (`None`: it cannot, so a drop ends the move).
pub async fn say_port(ch: &mut impl Channel, port: Option<u16>) -> Result<(), TransferError> {
    let mut msg = vec![RESUME_AT];
    if let Some(port) = port {
        msg.extend_from_slice(&port.to_be_bytes());
    }
    ch.send(&msg)
        .await
        .map_err(|_| TransferError::ConnectionDropped)
}

/// The new laptop: hears where the old laptop waits for its return.
pub async fn hear_port(ch: &mut impl Channel) -> Result<Option<u16>, TransferError> {
    let got = ch
        .recv()
        .await
        .map_err(|_| TransferError::ConnectionDropped)?;
    parse_port(&got)
}

/// Reads the old laptop's word on where it waits: nowhere, or a port that is not 0.
pub fn parse_port(msg: &[u8]) -> Result<Option<u16>, TransferError> {
    match *msg {
        [RESUME_AT] => Ok(None),
        [RESUME_AT, a, b] if u16::from_be_bytes([a, b]) != 0 => {
            Ok(Some(u16::from_be_bytes([a, b])))
        }
        _ => Err(TransferError::Protocol(
            "it did not say where to reconnect".into(),
        )),
    }
}

/* ---------- Over real connections ---------- */

/// The old laptop's way back over `pctwin-link`: once the plan is approved, a listener of its own
/// on the address the new laptop already reaches it at, taking only the new laptop's return made
/// with the resume keys (any other device's connection gets nothing), until the keys run out.
pub struct NetComeback {
    keys: Option<LaneKeys>,
    /// Where this laptop is on the main connection, and the new laptop's address there.
    here: Option<SocketAddr>,
    peer: SocketAddr,
    config: LinkConfig,
    listener: Option<LaneListener>,
    until: Option<tokio::time::Instant>,
}

impl NetComeback {
    /// Takes the pairing's keys from the main `link` (before the move uses it).
    pub fn new(link: &mut Link, config: LinkConfig) -> Self {
        Self {
            keys: link.take_lane_keys(),
            here: link.local_addr().ok(),
            peer: link.peer_addr(),
            config,
            listener: None,
            until: None,
        }
    }
}

impl Comeback for NetComeback {
    type Link = Link;

    async fn open(&mut self, plan: &[u8; 32]) -> Option<u16> {
        let mut keys = self.keys.take()?;
        let here = self.here?;
        let resumes = keys.resume_keys(plan, std::time::Instant::now()).ok()?;
        let host = pctwin_link::Host::bind(listen_at(here), self.config)
            .await
            .ok()?;
        let port = host.local_addr().ok()?.port();
        let mut listener = host.into_lanes(keys, self.peer);
        listener.allow_resumes(resumes);
        self.listener = Some(listener);
        self.until = Some(tokio::time::Instant::now() + RESUME_LIFETIME);
        Some(port)
    }

    async fn returned(&mut self) -> Option<Link> {
        let until = self.until?;
        loop {
            let listener = self.listener.as_mut()?;
            match tokio::time::timeout_at(until, listener.accept_any()).await {
                Ok(Ok(Arrival::Resumed(link))) => return Some(link),
                // The new laptop opens no extra lanes in this version.
                Ok(Ok(Arrival::Lane(_))) => {}
                // The keys ran out, or the listener broke: it listens no more.
                Err(_) | Ok(Err(_)) => {
                    self.listener = None;
                    return None;
                }
            }
        }
    }
}

/// Where the old laptop listens for the new laptop's return: the very address it has on the main
/// connection (an IPv6 link-local address keeps its interface, its scope), on any free port.
fn listen_at(here: SocketAddr) -> SocketAddr {
    let mut at = here;
    at.set_port(0);
    at
}

/// The new laptop's way back over `pctwin-link`: reconnects to the old laptop's listener with the
/// resume keys, no code.
pub struct NetRedial {
    keys: Option<LaneKeys>,
    /// The old laptop's address on the main connection.
    peer: SocketAddr,
    config: LinkConfig,
    resumes: Option<(ResumeKeys, SocketAddr, std::time::Instant)>,
}

impl NetRedial {
    /// Takes the pairing's keys from the main `link` (before the move uses it).
    pub fn new(link: &mut Link, config: LinkConfig) -> Self {
        Self {
            keys: link.take_lane_keys(),
            peer: link.peer_addr(),
            config,
            resumes: None,
        }
    }
}

impl Redial for NetRedial {
    type Link = Link;

    fn prepare(&mut self, plan: &[u8; 32], port: u16) -> bool {
        let Some(mut keys) = self.keys.take() else {
            return false;
        };
        let now = std::time::Instant::now();
        let Ok(resumes) = keys.resume_keys(plan, now) else {
            return false;
        };
        let mut at = self.peer;
        at.set_port(port);
        self.resumes = Some((resumes, at, now + RESUME_LIFETIME));
        true
    }

    async fn redial(&mut self) -> Result<Link, Redialed> {
        let Some((keys, at, until)) = self.resumes.as_mut() else {
            return Err(Redialed::Over);
        };
        // Known here, even while the old laptop cannot be reached to say so.
        if std::time::Instant::now() > *until {
            return Err(Redialed::Over);
        }
        match pctwin_link::open_resume(*at, keys, self.config).await {
            Ok(link) => Ok(link),
            Err(LinkError::Pairing(PairingError::Expired | PairingError::LaneRefused)) => {
                Err(Redialed::Over)
            }
            Err(_) => Err(Redialed::NotThere),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn where_to_reconnect_is_nowhere_or_a_real_port() {
        assert_eq!(parse_port(&[RESUME_AT]).unwrap(), None);
        assert_eq!(parse_port(&[RESUME_AT, 0x10, 0x92]).unwrap(), Some(4242));
        assert!(parse_port(&[RESUME_AT, 0, 0]).is_err());
        assert!(parse_port(&[RESUME_AT, 1]).is_err());
        assert!(parse_port(&[RESUME_AT, 1, 2, 3]).is_err());
        assert!(parse_port(&[39, 1, 2]).is_err());
        assert!(parse_port(&[]).is_err());
    }

    #[test]
    fn the_way_back_listens_on_the_main_connection_s_own_address_and_interface() {
        let v6: SocketAddr = "[fe80::1%7]:51515".parse().unwrap();
        let at = listen_at(v6);
        assert_eq!(at.port(), 0);
        let std::net::SocketAddr::V6(at) = at else {
            panic!("still IPv6");
        };
        assert_eq!(at.scope_id(), 7, "the interface is kept");
        assert_eq!(*at.ip(), "fe80::1".parse::<std::net::Ipv6Addr>().unwrap());
        let v4: SocketAddr = "192.168.1.20:51515".parse().unwrap();
        assert_eq!(
            listen_at(v4),
            "192.168.1.20:0".parse::<SocketAddr>().unwrap()
        );
    }

    fn rules(mover: &str) -> LandingRules {
        LandingRules {
            mover: mover.into(),
            ..LandingRules::default()
        }
    }

    #[test]
    fn the_plan_print_changes_with_the_list_and_with_the_rules() {
        let none: Vec<Item> = Vec::new();
        let a = plan_print(&none, &rules("1000"));
        assert_eq!(a, plan_print(&none, &rules("1000")));
        assert_ne!(a, plan_print(&none, &rules("1001")));
        // Taken in two parts, as the new laptop does, it is the same.
        assert_eq!(a, finish_print(print_items(&none), &rules("1000")));
    }
}
