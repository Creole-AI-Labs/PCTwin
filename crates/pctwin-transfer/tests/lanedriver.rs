//! The lane driver on the new laptop (Security Design A, "Extra lanes"): each window it reports the
//! speed to the tuner and opens or closes lanes to match. A lane that cannot be opened stops all
//! further opening for the move (someone may be burning lane numbers); a closed lane fails at once,
//! even mid-wait, so the old laptop sees it drop and sends its blocks on the others; a lane that
//! dies on its own is counted, and may be replaced. Run in virtual time.

use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use pctwin_transfer::{Channel, ChannelError, Closable, LaneDriver, OpenLane};
use tokio::sync::mpsc;

const WINDOW: Duration = Duration::from_secs(3);

/// A lane that stays open until it is closed or told to die.
#[derive(Default)]
struct Quiet {
    dies: Arc<AtomicBool>,
}

impl Channel for Quiet {
    async fn send(&mut self, _data: &[u8]) -> Result<(), ChannelError> {
        if self.dies.load(Ordering::SeqCst) {
            return Err(ChannelError);
        }
        Ok(())
    }

    async fn recv(&mut self) -> Result<Vec<u8>, ChannelError> {
        loop {
            if self.dies.load(Ordering::SeqCst) {
                return Err(ChannelError);
            }
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    }
}

/// Opens lanes until `refuse_from` attempts, counting every attempt.
struct Opener {
    attempts: Arc<AtomicUsize>,
    refuse_from: usize,
    deaths: Arc<Mutex<Vec<Arc<AtomicBool>>>>,
}

impl OpenLane for Opener {
    type Lane = Quiet;

    async fn open(&mut self) -> Option<Quiet> {
        let n = self.attempts.fetch_add(1, Ordering::SeqCst) + 1;
        if n >= self.refuse_from {
            return None;
        }
        let lane = Quiet::default();
        self.deaths.lock().unwrap().push(lane.dies.clone());
        Some(lane)
    }
}

fn opener(refuse_from: usize) -> Opener {
    Opener {
        attempts: Arc::default(),
        refuse_from,
        deaths: Arc::default(),
    }
}

/// Runs the driver for `windows` windows over a network where `speeds[n]` is the speed with n + 1
/// lanes (the main connection and n extra). Returns the driver and the lanes it handed over.
async fn drive(
    opener: &mut Opener,
    speeds: [f64; 4],
    windows: u32,
) -> (LaneDriver, mpsc::UnboundedReceiver<Closable<Quiet>>) {
    let mut driver = LaneDriver::new(WINDOW);
    let (tx, rx) = mpsc::unbounded_channel();
    let mut bytes = 0u64;
    let finished = Arc::new(AtomicBool::new(false));
    let stop = finished.clone();
    let deadline = async move {
        tokio::time::sleep(WINDOW * windows + WINDOW / 2).await;
        stop.store(true, Ordering::SeqCst);
    };
    tokio::join!(
        driver.run(
            opener,
            &tx,
            |lanes| {
                bytes += (speeds[(lanes - 1).min(3)] * WINDOW.as_secs_f64()) as u64;
                bytes
            },
            || finished.load(Ordering::SeqCst),
        ),
        deadline
    );
    (driver, rx)
}

#[tokio::test(start_paused = true)]
async fn it_opens_lanes_while_they_make_the_move_faster() {
    let mut o = opener(usize::MAX);
    // Three lanes in all is best: two extra.
    let (driver, mut rx) = drive(&mut o, [10e6, 18e6, 24e6, 22e6], 20).await;
    let mut handed = 0;
    while rx.try_recv().is_ok() {
        handed += 1;
    }
    // It tried a third extra lane, found it slower, and closed it again.
    assert_eq!(handed, 3);
    assert_eq!(driver.lanes(), 0, "all lanes closed when the move finished");
    assert!(!driver.stopped_opening());
}

#[tokio::test(start_paused = true)]
async fn a_refused_lane_stops_all_further_opening() {
    // The second lane is refused (its number was burned, or a firewall).
    let mut o = opener(2);
    // Long enough for the tuner's look again for more lanes (every 100 windows).
    let (driver, _rx) = drive(&mut o, [10e6, 20e6, 30e6, 40e6], 130).await;
    assert_eq!(
        o.attempts.load(Ordering::SeqCst),
        2,
        "no retry after a refusal"
    );
    assert!(driver.stopped_opening());
}

#[tokio::test(start_paused = true)]
async fn a_closed_lane_fails_at_once_even_mid_wait() {
    let mut o = opener(usize::MAX);
    let mut driver = LaneDriver::new(WINDOW);
    let (tx, mut rx) = mpsc::unbounded_channel::<Closable<Quiet>>();
    // Speed only rises with one extra lane, so it is opened, then the next try is closed again.
    let speeds = [10e6, 20e6, 20e6, 20e6];
    let mut bytes = 0u64;
    let finished = Arc::new(AtomicBool::new(false));
    let stop = finished.clone();
    let watch = async move {
        // The second lane handed over is the one tried and closed: it is waiting to receive.
        let _first = rx.recv().await.unwrap();
        let mut second = rx.recv().await.unwrap();
        let waited = tokio::time::timeout(WINDOW * 10, second.recv()).await;
        stop.store(true, Ordering::SeqCst);
        assert!(matches!(waited, Ok(Err(ChannelError))), "{waited:?}");
        // Sending fails too.
        assert!(second.send(b"x").await.is_err());
    };
    tokio::join!(
        driver.run(
            &mut o,
            &tx,
            |lanes| {
                bytes += (speeds[(lanes - 1).min(3)] * WINDOW.as_secs_f64()) as u64;
                bytes
            },
            || finished.load(Ordering::SeqCst),
        ),
        watch
    );
}

/// Lanes are used (by sending, or by receiving) until they die all at once; returns, for each
/// lane handed over after the deaths, the window it came in.
async fn deaths_and_replacements(receiving: bool) -> Vec<u32> {
    let mut o = opener(usize::MAX);
    let deaths = o.deaths.clone();
    let mut driver = LaneDriver::new(WINDOW);
    let (tx, mut rx) = mpsc::unbounded_channel::<Closable<Quiet>>();
    let speeds = [10e6, 20e6, 30e6, 40e6];
    let mut bytes = 0u64;
    let finished = Arc::new(AtomicBool::new(false));
    let stop = finished.clone();
    let session = async move {
        // As the move does: use each lane until it fails.
        let mut held = Vec::new();
        let mut after = Vec::new();
        let start = tokio::time::Instant::now();
        let died_at = WINDOW * 15;
        while start.elapsed() < WINDOW * 40 {
            while let Ok(lane) = rx.try_recv() {
                held.push(lane);
                if start.elapsed() > died_at {
                    after.push((start.elapsed().as_secs_f64() / WINDOW.as_secs_f64()) as u32);
                }
            }
            if start.elapsed() > died_at && start.elapsed() < died_at + WINDOW {
                // Every lane opened so far dies (out of range a moment).
                for d in deaths.lock().unwrap().iter() {
                    d.store(true, Ordering::SeqCst);
                }
            }
            for lane in &mut held {
                if receiving {
                    let _ = tokio::time::timeout(Duration::from_millis(10), lane.recv()).await;
                } else {
                    let _ = lane.send(b"x").await;
                }
            }
            tokio::time::sleep(Duration::from_millis(500)).await;
        }
        stop.store(true, Ordering::SeqCst);
        after
    };
    let ((), after) = tokio::join!(
        driver.run(
            &mut o,
            &tx,
            |lanes| {
                bytes += (speeds[(lanes - 1).min(3)] * WINDOW.as_secs_f64()) as u64;
                bytes
            },
            || finished.load(Ordering::SeqCst),
        ),
        session
    );
    after
}

#[tokio::test(start_paused = true)]
async fn lanes_that_die_while_sending_are_counted_and_replaced_one_at_a_time() {
    let after = deaths_and_replacements(false).await;
    assert!(!after.is_empty(), "no lane was opened in their place");
    // The tuner climbs again from the main connection alone: never two new lanes in one window.
    let mut windows = after.clone();
    windows.dedup();
    assert_eq!(windows.len(), after.len(), "{after:?}");
}

#[tokio::test(start_paused = true)]
async fn lanes_that_die_while_receiving_are_counted_and_replaced_one_at_a_time() {
    let after = deaths_and_replacements(true).await;
    assert!(!after.is_empty(), "no lane was opened in their place");
    let mut windows = after.clone();
    windows.dedup();
    assert_eq!(windows.len(), after.len(), "{after:?}");
}
