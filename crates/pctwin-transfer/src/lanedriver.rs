use std::future::Future;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use tokio::sync::Notify;
use tokio::sync::mpsc::UnboundedSender;

use crate::lanes::LaneTuner;
use crate::session::{Channel, ChannelError};

/// Opens one more lane to the old laptop (the new laptop's side; the app opens a real connection).
pub trait OpenLane {
    type Lane: Channel;

    /// A new lane, or `None` if it could not be opened (refused, or unreachable).
    fn open(&mut self) -> impl Future<Output = Option<Self::Lane>>;
}

/// Shared between a lane and the driver: whether the driver closed it, and whether it failed on
/// its own.
#[derive(Debug, Default)]
struct Switch {
    closed: AtomicBool,
    dead: AtomicBool,
    wake: Notify,
}

impl Switch {
    fn close(&self) {
        self.closed.store(true, Ordering::SeqCst);
        self.wake.notify_waiters();
    }
}

/// A lane the driver can close at any moment: once closed, every call fails at once, including a
/// receive already waiting, so the lane's session lets it go (and its unconfirmed blocks go to the
/// other lanes). Dropping it closes the connection underneath.
pub struct Closable<C> {
    inner: C,
    switch: Arc<Switch>,
}

impl<C: Channel> Channel for Closable<C> {
    async fn send(&mut self, data: &[u8]) -> Result<(), ChannelError> {
        if self.switch.closed.load(Ordering::SeqCst) {
            return Err(ChannelError);
        }
        let sent = self.inner.send(data).await;
        if sent.is_err() {
            self.switch.dead.store(true, Ordering::SeqCst);
        }
        sent
    }

    async fn recv(&mut self) -> Result<Vec<u8>, ChannelError> {
        // Registered before looking, so a close made meanwhile is not missed.
        let closing = self.switch.wake.notified();
        if self.switch.closed.load(Ordering::SeqCst) {
            return Err(ChannelError);
        }
        // Cancelling the receive on close is fine: the lane is being closed anyway.
        let got = tokio::select! {
            got = self.inner.recv() => got,
            () = closing => Err(ChannelError),
        };
        if got.is_err() && !self.switch.closed.load(Ordering::SeqCst) {
            self.switch.dead.store(true, Ordering::SeqCst);
        }
        got
    }
}

/// Most refused lanes in one move before opening stops for good: a refusal may be a passing
/// failure, or someone burning lane numbers (Security Design A), so lanes are tried again only a
/// few times, far apart.
pub const MAX_LANE_REFUSALS: u32 = 3;
/// Windows to wait after the first refusal before a lane may be opened again; doubled after each
/// further refusal.
const REFUSAL_HOLD_OFF: u64 = 100;

/// Keeps the number of lanes where the [`LaneTuner`] wants it during a move, on the new laptop.
/// After a lane is refused the tuner settles back, and tries again only at its next look for more
/// lanes (minutes later); after [`MAX_LANE_REFUSALS`] refusals no more lanes are opened in this
/// move, and it carries on over the lanes it has.
#[derive(Debug)]
pub struct LaneDriver {
    tuner: LaneTuner,
    open: Vec<Arc<Switch>>,
    refusals: u32,
    /// Windows to wait before opening again after a refusal (it grows with each one).
    hold_off: u64,
    window: Duration,
}

impl LaneDriver {
    /// Measures speed over windows of `window` (a few seconds).
    pub fn new(window: Duration) -> Self {
        Self {
            tuner: LaneTuner::new(),
            open: Vec::new(),
            refusals: 0,
            hold_off: 0,
            window,
        }
    }

    /// Extra lanes open now (besides the main connection).
    pub fn lanes(&self) -> usize {
        self.open.len()
    }

    /// Extra lanes the tuner wants now (for "more details"): the same as [`lanes`](Self::lanes)
    /// once the driver has caught up.
    pub fn wanted(&self) -> usize {
        usize::from(self.tuner.lanes()).saturating_sub(1)
    }

    /// Whether lanes were refused too often, so no more are opened in this move.
    pub fn stopped_opening(&self) -> bool {
        self.refusals >= MAX_LANE_REFUSALS
    }

    /// Runs until `finished` says the move is over, then closes every lane it opened. Each window
    /// it gives the tuner the speed (from `bytes_so_far`, told how many lanes are open including
    /// the main connection) and opens lanes through `opener`, handing them to the move through
    /// `joining`, or closes the newest, to match.
    pub async fn run<O: OpenLane>(
        &mut self,
        opener: &mut O,
        joining: &UnboundedSender<Closable<O::Lane>>,
        mut bytes_so_far: impl FnMut(usize) -> u64,
        finished: impl Fn() -> bool,
    ) {
        let mut last = bytes_so_far(self.open.len() + 1);
        loop {
            tokio::time::sleep(self.window).await;
            if finished() {
                break;
            }
            // Lanes that failed on their own (dropped, out of range): the tuner counts them lost.
            let before = self.open.len();
            self.open.retain(|s| !s.dead.load(Ordering::SeqCst));
            for _ in self.open.len()..before {
                self.tuner.lane_lost();
            }
            let now = bytes_so_far(self.open.len() + 1);
            let speed = now.saturating_sub(last) as f64 / self.window.as_secs_f64();
            last = now;
            let want = usize::from(self.tuner.measured(speed)).saturating_sub(1);
            self.hold_off = self.hold_off.saturating_sub(1);
            while self.open.len() < want && !self.stopped_opening() && self.hold_off == 0 {
                match opener.open().await {
                    Some(lane) => {
                        let switch = Arc::new(Switch::default());
                        let handed = joining.send(Closable {
                            inner: lane,
                            switch: switch.clone(),
                        });
                        if handed.is_err() {
                            // The move has ended.
                            self.close_all();
                            return;
                        }
                        self.open.push(switch);
                    }
                    None => {
                        // Not tried again now: the tuner goes back and settles, and asks again
                        // only at its next look for more lanes (minutes later).
                        self.tuner.refused();
                        self.refusals += 1;
                        // Exponential backoff: each refusal doubles the wait before trying
                        // again (on top of the tuner's own minutes between looks).
                        self.hold_off = REFUSAL_HOLD_OFF << (self.refusals - 1);
                        break;
                    }
                }
            }
            while self.open.len() > want {
                if let Some(newest) = self.open.pop() {
                    newest.close();
                }
            }
        }
        self.close_all();
    }

    fn close_all(&mut self) {
        for switch in self.open.drain(..) {
            switch.close();
        }
    }
}
