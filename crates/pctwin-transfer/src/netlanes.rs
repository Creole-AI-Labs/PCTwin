use std::net::SocketAddr;

use pctwin_link::{LaneKeys, LaneListener, Link, LinkConfig};
use tokio::sync::mpsc::UnboundedSender;

use crate::lanedriver::OpenLane;

/// New laptop: opens real lanes to the old laptop over the paired session, for the
/// [`LaneDriver`](crate::LaneDriver).
pub struct LinkLanes {
    addr: SocketAddr,
    keys: LaneKeys,
    config: LinkConfig,
    opened: u32,
}

impl LinkLanes {
    /// Takes the lane keys from the main link (they are handed out once), or `None` if they were
    /// taken already.
    pub fn new(main: &mut Link, config: LinkConfig) -> Option<Self> {
        let keys = main.take_lane_keys()?;
        Some(Self {
            addr: main.peer_addr(),
            keys,
            config,
            opened: 0,
        })
    }

    /// Lanes opened so far in this move (for "more details").
    pub fn opened(&self) -> u32 {
        self.opened
    }
}

impl OpenLane for LinkLanes {
    type Lane = Link;

    async fn open(&mut self) -> Option<Link> {
        let lane = pctwin_link::open_lane(self.addr, &mut self.keys, self.config)
            .await
            .ok()?;
        self.opened += 1;
        Some(lane)
    }
}

/// Old laptop: hands every lane the paired new laptop opens to the move, until the move stops
/// taking them or the listener fails.
pub async fn accept_lanes(listener: &mut LaneListener, joining: &UnboundedSender<Link>) {
    while let Ok(lane) = listener.accept().await {
        if joining.send(lane).is_err() {
            return;
        }
    }
}
