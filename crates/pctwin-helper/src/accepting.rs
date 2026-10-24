//! The Mac launch daemon's door (Security Design part C, "The administrator's approval on a
//! Mac"): callers are taken as they come, each in its own thread with a short total time to open
//! a move, at most one waiting per account; the first valid opening is served and every other
//! caller is let go. So callers that connect and say nothing, or say it slowly, never queue the
//! real mover behind them (found by the second fresh-context review: one at a time, each held up
//! to its read timeout, they could keep the daemon busy for as long as they liked).
//!
//! Three rules hold whatever the timing (third fresh-context review):
//! - A caller's own failure (it closed at once, its account could not be read, the system
//!   dropped the connection) lets that caller go and the door stays open; only a door that can
//!   take no one any more ends it, and running out of files makes it wait a moment and go on.
//! - The time to open is a deadline, not a per-read wait: before every read the connection's
//!   timeout is set to the time left ([`Deadlined`]), so a caller dripping one byte at a time
//!   is stopped when its time is up, without relying on anything waking a waiting read.
//! - A caller's state moves once, from waiting to opened or to cut off, so a caller cut off at
//!   its deadline can never be served, even if its opening finishes a moment later.

use std::collections::BTreeSet;
use std::io::{self, Read, Write};
use std::sync::atomic::{AtomicBool, AtomicU8, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, mpsc};
use std::time::{Duration, Instant};

use crate::opening::NotOpened;

/// The longest a caller has, from connecting, to open its move.
pub const OPENING_TIME: Duration = Duration::from_millis(2500);

/// A pause before taking callers again when the system has no more connections to give (too many
/// files open): the door waits, it never ends.
const BUSY_PAUSE: Duration = Duration::from_millis(100);

/// A connection whose reads can be given a timeout.
pub trait Timed: Read + Write {
    fn set_read_timeout(&self, t: Option<Duration>) -> io::Result<()>;
}

#[cfg(unix)]
impl Timed for std::os::unix::net::UnixStream {
    fn set_read_timeout(&self, t: Option<Duration>) -> io::Result<()> {
        std::os::unix::net::UnixStream::set_read_timeout(self, t)
    }
}

impl Timed for std::net::TcpStream {
    fn set_read_timeout(&self, t: Option<Duration>) -> io::Result<()> {
        std::net::TcpStream::set_read_timeout(self, t)
    }
}

/// A caller's connection while it opens its move: every read waits at most the time left before
/// `until`, and none starts after it.
pub struct Deadlined<S> {
    inner: S,
    until: Instant,
}

impl<S: Timed> Deadlined<S> {
    pub fn new(inner: S, until: Instant) -> Self {
        Self { inner, until }
    }

    /// The connection itself, for the session once the move is opened (reads then wait as long
    /// as the session needs: the caller of this sets that).
    pub fn into_inner(self) -> S {
        self.inner
    }
}

impl<S: Timed> Read for Deadlined<S> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let left = self.until.saturating_duration_since(Instant::now());
        if left.is_zero() {
            return Err(io::Error::new(
                io::ErrorKind::TimedOut,
                "the time to open a move ran out",
            ));
        }
        self.inner.set_read_timeout(Some(left))?;
        self.inner.read(buf)
    }
}

impl<S: Timed> Write for Deadlined<S> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.inner.write(buf)
    }
    fn flush(&mut self) -> io::Result<()> {
        self.inner.flush()
    }
}

/// One accepted caller: its connection, its account (user ID), and a way to end the connection
/// from another thread.
pub struct Accepted<S> {
    pub stream: S,
    pub uid: u32,
    pub cut: Box<dyn FnOnce() + Send>,
}

/// Why no caller came from one try at the door.
#[derive(Debug)]
pub enum NotAccepted {
    /// This caller only (it closed at once, its account could not be read, the connection was
    /// dropped or interrupted): it is let go and the door stays open.
    Caller(String),
    /// The system has no more connections to give for now (too many open): wait, then go on.
    Busy,
    /// The door itself is gone: the daemon ends.
    Closed(String),
}

impl NotAccepted {
    /// What an accept error means for the door. Only a listener that is really gone ends it (a
    /// bad or closed handle, not a socket, one that cannot accept); the system running short (too
    /// many files open, no memory or buffers) waits and goes on; anything else is taken as this
    /// caller's alone, and the door stays open.
    pub fn of(e: &io::Error) -> Self {
        if dead_listener(e) {
            return Self::Closed(e.to_string());
        }
        if short_of(e) || e.kind() == io::ErrorKind::OutOfMemory {
            return Self::Busy;
        }
        Self::Caller(e.to_string())
    }
}

/// The listener itself is gone: EBADF, EINVAL, ENOTSOCK (and the Windows socket codes for them).
fn dead_listener(e: &io::Error) -> bool {
    #[cfg(unix)]
    {
        use rustix::io::Errno;
        e.raw_os_error().is_some_and(|c| {
            [Errno::BADF, Errno::INVAL, Errno::NOTSOCK]
                .iter()
                .any(|d| d.raw_os_error() == c)
        })
    }
    #[cfg(windows)]
    {
        // WSAEBADF, WSAEINVAL, WSAENOTSOCK.
        e.raw_os_error()
            .is_some_and(|c| [10009, 10022, 10038].contains(&c))
    }
}

/// The system is short of something for now: EMFILE, ENFILE, ENOBUFS, ENOMEM.
fn short_of(e: &io::Error) -> bool {
    #[cfg(unix)]
    {
        use rustix::io::Errno;
        e.raw_os_error().is_some_and(|c| {
            [Errno::MFILE, Errno::NFILE, Errno::NOBUFS, Errno::NOMEM]
                .iter()
                .any(|d| d.raw_os_error() == c)
        })
    }
    #[cfg(windows)]
    {
        // WSAEMFILE, WSAENOBUFS.
        e.raw_os_error()
            .is_some_and(|c| [10024, 10055].contains(&c))
    }
}

/// Most callers being opened at once; more are let go at once (they never hold a thread).
pub const MOST_IN_FLIGHT: usize = 16;

/// Callers let go are logged at most once this often, with how many there were since.
const LOG_EVERY: Duration = Duration::from_secs(1);
/// After this many callers let go in a row with none taken, the door pauses a moment.
const PAUSE_AFTER: u32 = 32;
const CALLER_PAUSE: Duration = Duration::from_millis(50);

/// The callers let go, said at most once a second.
#[derive(Default)]
struct LetGo {
    since: u64,
    last: Option<Instant>,
    in_a_row: u32,
}

impl LetGo {
    fn one(&mut self, why: &str) {
        self.since += 1;
        self.in_a_row += 1;
        if self.last.is_none_or(|t| t.elapsed() >= LOG_EVERY) {
            eprintln!(
                "pctwin-helper: {} caller(s) let go, the latest: {why}",
                self.since
            );
            self.since = 0;
            self.last = Some(Instant::now());
        }
        if self.in_a_row >= PAUSE_AFTER {
            self.in_a_row = 0;
            std::thread::sleep(CALLER_PAUSE);
        }
    }
}

/// A caller's state: it moves once, from waiting, to done (its opening finished in time) or to
/// cut off.
const WAITING: u8 = 0;
const OPENED: u8 = 1;
const CUT: u8 = 2;

/// Takes callers from `accept` until one opens a move with `open` within `time` of connecting:
/// that caller (its connection, the deadline taken off), its account and its opening. Root (uid
/// 0) is never served, and a second caller for an account that already has one waiting is let go
/// at once. A caller whose time runs out is cut off and can never be served. `log` hears why a
/// caller was let go (for the helper's own log).
pub fn first_opening<S, O>(
    accept: impl FnMut() -> Result<Accepted<S>, NotAccepted> + Send + 'static,
    open: impl Fn(&mut Deadlined<S>, u32) -> Result<O, NotOpened> + Send + Sync + 'static,
    time: Duration,
    log: impl Fn(u32, &NotOpened) + Send + Sync + 'static,
) -> Result<(S, u32, O), String>
where
    S: Timed + Send + 'static,
    O: Send + 'static,
{
    door(accept, open, time, log, Arc::default())
}

/// How many callers are being opened, and how many watches over their time are running.
#[derive(Debug, Default)]
struct Counts {
    opening: AtomicUsize,
    watching: AtomicUsize,
}

/// [`first_opening`], counting in `counts`.
fn door<S, O>(
    mut accept: impl FnMut() -> Result<Accepted<S>, NotAccepted> + Send + 'static,
    open: impl Fn(&mut Deadlined<S>, u32) -> Result<O, NotOpened> + Send + Sync + 'static,
    time: Duration,
    log: impl Fn(u32, &NotOpened) + Send + Sync + 'static,
    counts: Arc<Counts>,
) -> Result<(S, u32, O), String>
where
    S: Timed + Send + 'static,
    O: Send + 'static,
{
    let (tx, rx) = mpsc::channel::<Result<(S, u32, O), String>>();
    let open = Arc::new(open);
    let log = Arc::new(log);
    let won = Arc::new(AtomicBool::new(false));
    let waiting: Arc<Mutex<BTreeSet<u32>>> = Arc::default();
    std::thread::spawn(move || {
        let mut let_go = LetGo::default();
        loop {
            let caller = match accept() {
                Ok(c) => {
                    let_go.in_a_row = 0;
                    c
                }
                Err(NotAccepted::Caller(why)) => {
                    let_go.one(&why);
                    continue;
                }
                Err(NotAccepted::Busy) => {
                    std::thread::sleep(BUSY_PAUSE);
                    continue;
                }
                Err(NotAccepted::Closed(why)) => {
                    let _ = tx.send(Err(why));
                    return;
                }
            };
            let until = Instant::now() + time;
            let Accepted { stream, uid, cut } = caller;
            if uid == 0 || won.load(Ordering::Acquire) {
                cut();
                continue;
            }
            // At most a few callers being opened at once, and one per account.
            if counts.opening.load(Ordering::Acquire) >= MOST_IN_FLIGHT {
                cut();
                let_go.one("too many callers at once");
                continue;
            }
            let fresh = waiting.lock().map(|mut w| w.insert(uid)).unwrap_or(false);
            if !fresh {
                cut();
                continue;
            }
            counts.opening.fetch_add(1, Ordering::AcqRel);
            counts.watching.fetch_add(1, Ordering::AcqRel);
            let state = Arc::new(AtomicU8::new(WAITING));
            let watch = Arc::clone(&state);
            let watched = Arc::clone(&counts);
            // The watch over a caller's time ends as soon as its opening does (it is woken
            // then), so a caller that fails at once holds nothing for the rest of its time.
            let watcher = std::thread::spawn(move || {
                // Counted while it runs, whichever way it ends.
                struct Done(Arc<Counts>);
                impl Drop for Done {
                    fn drop(&mut self) {
                        self.0.watching.fetch_sub(1, Ordering::AcqRel);
                    }
                }
                let _done = Done(watched);
                loop {
                    if watch.load(Ordering::Acquire) != WAITING {
                        return;
                    }
                    let left = until.saturating_duration_since(Instant::now());
                    if left.is_zero() {
                        break;
                    }
                    std::thread::park_timeout(left);
                }
                // Cut off only a caller still waiting; one whose opening finished is left be.
                if watch
                    .compare_exchange(WAITING, CUT, Ordering::AcqRel, Ordering::Acquire)
                    .is_ok()
                {
                    cut();
                }
            });
            let (tx, open, log, won, waiting, counts) = (
                tx.clone(),
                Arc::clone(&open),
                Arc::clone(&log),
                Arc::clone(&won),
                Arc::clone(&waiting),
                Arc::clone(&counts),
            );
            std::thread::spawn(move || {
                let mut stream = Deadlined::new(stream, until);
                let opened = open(&mut stream, uid);
                // Finished in time, or not at all: a caller already cut off never counts.
                let in_time = Instant::now() <= until
                    && state
                        .compare_exchange(WAITING, OPENED, Ordering::AcqRel, Ordering::Acquire)
                        .is_ok();
                watcher.thread().unpark();
                if let Ok(mut w) = waiting.lock() {
                    w.remove(&uid);
                }
                counts.opening.fetch_sub(1, Ordering::AcqRel);
                match opened {
                    // The first opening in time is served; any other is let go.
                    Ok(o) if in_time && !won.swap(true, Ordering::AcqRel) => {
                        let _ = tx.send(Ok((stream.into_inner(), uid, o)));
                    }
                    Ok(_) if !in_time => log(
                        uid,
                        &NotOpened::Channel("its time to open a move ran out".into()),
                    ),
                    Ok(_) => {}
                    Err(why) => log(uid, &why),
                }
            });
        }
    });
    rx.recv()
        .map_err(|_| "the daemon stopped taking callers".to_string())?
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::{TcpListener, TcpStream};

    /// A door on a local port: each connection says its account first ("uid 501"), standing
    /// for the system's word on who called. One that closes before saying it is that caller's
    /// failure, as a failed peer-credential read is in the daemon.
    fn door_of(listener: TcpListener) -> impl FnMut() -> Result<Accepted<TcpStream>, NotAccepted> {
        move || {
            let (s, _) = listener.accept().map_err(|e| NotAccepted::of(&e))?;
            s.set_read_timeout(Some(Duration::from_secs(5)))
                .map_err(|e| NotAccepted::Caller(e.to_string()))?;
            let mut uid_line = Vec::new();
            let mut byte = [0u8; 1];
            loop {
                match s.try_clone().and_then(|mut c| c.read(&mut byte)) {
                    Ok(1) if byte[0] == b'\n' => break,
                    Ok(1) => uid_line.push(byte[0]),
                    _ => return Err(NotAccepted::Caller("closed before saying who".into())),
                }
            }
            let uid = String::from_utf8_lossy(&uid_line)
                .strip_prefix("uid ")
                .and_then(|u| u.trim().parse().ok())
                .unwrap_or(0);
            let cutter = s
                .try_clone()
                .map_err(|e| NotAccepted::Caller(e.to_string()))?;
            Ok(Accepted {
                stream: s,
                uid,
                cut: Box::new(move || {
                    let _ = cutter.shutdown(std::net::Shutdown::Both);
                }),
            })
        }
    }

    /// Reads one line, a byte at a time, through the deadline.
    fn line(r: &mut Deadlined<TcpStream>) -> Option<String> {
        let mut out = Vec::new();
        let mut byte = [0u8; 1];
        loop {
            match r.read(&mut byte) {
                Ok(1) if byte[0] == b'\n' => return Some(String::from_utf8_lossy(&out).into()),
                Ok(1) => out.push(byte[0]),
                _ => return None,
            }
        }
    }

    /// Opens a move when the caller says "open"; "slow" opens it, but only after `late`.
    fn opener(late: Duration) -> impl Fn(&mut Deadlined<TcpStream>, u32) -> Result<(), NotOpened> {
        move |r, _| match line(r).as_deref() {
            Some("open") => Ok(()),
            Some("slow") => {
                std::thread::sleep(late);
                Ok(())
            }
            _ => Err(NotOpened::Malformed("did not open".into())),
        }
    }

    fn call(addr: std::net::SocketAddr, uid: u32) -> TcpStream {
        let mut s = TcpStream::connect(addr).unwrap();
        writeln!(s, "uid {uid}").unwrap();
        s
    }

    /// The door's result, and the accounts it let go.
    type Served = (
        std::thread::JoinHandle<Result<u32, String>>,
        Arc<Mutex<Vec<u32>>>,
    );

    fn served(listener: TcpListener, time: Duration, late: Duration) -> Served {
        let heard = Arc::new(Mutex::new(Vec::new()));
        let log = Arc::clone(&heard);
        let first = std::thread::spawn(move || {
            first_opening(door_of(listener), opener(late), time, move |uid, _| {
                log.lock().unwrap().push(uid)
            })
            .map(|(_, uid, ())| uid)
        });
        (first, heard)
    }

    /// A flood of callers from one account, each sending nonsense at once: each is let go as
    /// soon as it fails, nothing of it is held for the rest of its time, never more than a few
    /// are being opened at once, and the mover is served straight after.
    #[test]
    fn a_flood_of_instant_failures_holds_nothing_and_the_mover_is_served() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();
        let time = Duration::from_secs(5);
        let counts = Arc::new(Counts::default());
        let seen = Arc::clone(&counts);
        let first = std::thread::spawn(move || {
            door(
                door_of(listener),
                opener(Duration::ZERO),
                time,
                |_, _| {},
                seen,
            )
            .map(|(_, uid, ())| uid)
        });
        let started = Instant::now();
        let mut most = 0;
        for _ in 0..200 {
            let mut c = call(addr, 501);
            let _ = writeln!(c, "nonsense");
            most = most.max(counts.opening.load(Ordering::Acquire));
        }
        // Every one has ended, its watch included, long before its time would have run out.
        let settle = Instant::now() + Duration::from_secs(3);
        while (counts.opening.load(Ordering::Acquire) > 0
            || counts.watching.load(Ordering::Acquire) > 0)
            && Instant::now() < settle
        {
            std::thread::sleep(Duration::from_millis(20));
        }
        assert_eq!(counts.opening.load(Ordering::Acquire), 0);
        assert_eq!(counts.watching.load(Ordering::Acquire), 0);
        assert!(most <= MOST_IN_FLIGHT, "{most} at once");
        let mut mover = call(addr, 502);
        writeln!(mover, "open").unwrap();
        assert_eq!(first.join().unwrap().unwrap(), 502);
        assert!(
            started.elapsed() < time,
            "served after {:?}",
            started.elapsed()
        );
    }

    /// Only a listener that is really gone ends the door; the system running short waits; any
    /// other failure is one caller's.
    #[test]
    fn accept_failures_end_the_door_only_when_it_is_gone() {
        let kind = |k: io::ErrorKind| NotAccepted::of(&io::Error::from(k));
        for k in [
            io::ErrorKind::Interrupted,
            io::ErrorKind::ConnectionAborted,
            io::ErrorKind::ConnectionReset,
            io::ErrorKind::PermissionDenied,
            io::ErrorKind::Other,
        ] {
            assert!(matches!(kind(k), NotAccepted::Caller(_)), "{k:?}");
        }
        assert!(matches!(
            kind(io::ErrorKind::OutOfMemory),
            NotAccepted::Busy
        ));
        #[cfg(unix)]
        {
            use rustix::io::Errno;
            let code = |e: Errno| NotAccepted::of(&io::Error::from_raw_os_error(e.raw_os_error()));
            for e in [Errno::MFILE, Errno::NFILE, Errno::NOBUFS, Errno::NOMEM] {
                assert!(matches!(code(e), NotAccepted::Busy), "{e:?}");
            }
            for e in [Errno::PROTO, Errno::CONNABORTED, Errno::INTR] {
                assert!(matches!(code(e), NotAccepted::Caller(_)), "{e:?}");
            }
            for e in [Errno::BADF, Errno::INVAL, Errno::NOTSOCK] {
                assert!(matches!(code(e), NotAccepted::Closed(_)), "{e:?}");
            }
        }
        #[cfg(windows)]
        {
            let code = |c: i32| NotAccepted::of(&io::Error::from_raw_os_error(c));
            assert!(matches!(code(10024), NotAccepted::Busy));
            assert!(matches!(code(10055), NotAccepted::Busy));
            assert!(matches!(code(10053), NotAccepted::Caller(_)));
            for c in [10009, 10022, 10038] {
                assert!(matches!(code(c), NotAccepted::Closed(_)), "{c}");
            }
        }
    }

    #[test]
    fn silent_callers_never_queue_the_real_mover() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();
        let time = Duration::from_secs(2);
        let started = Instant::now();
        let (first, _) = served(listener, time, Duration::ZERO);
        let silent: Vec<TcpStream> = (501..506).map(|u| call(addr, u)).collect();
        std::thread::sleep(Duration::from_millis(200));
        let mut mover = call(addr, 507);
        writeln!(mover, "open").unwrap();
        assert_eq!(first.join().unwrap().unwrap(), 507);
        assert!(
            started.elapsed() < time,
            "served after {:?}",
            started.elapsed()
        );
        drop(silent);
    }

    #[test]
    fn one_caller_waits_per_account_and_the_first_opening_wins() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();
        let (first, _) = served(listener, Duration::from_secs(5), Duration::ZERO);
        let _waiting = call(addr, 501);
        std::thread::sleep(Duration::from_millis(100));
        let mut again = call(addr, 501);
        writeln!(again, "open").unwrap();
        let mut root = call(addr, 0);
        writeln!(root, "open").unwrap();
        let mut end = [0u8; 1];
        again
            .set_read_timeout(Some(Duration::from_secs(5)))
            .unwrap();
        assert!(matches!(again.read(&mut end), Ok(0) | Err(_)));
        let mut mover = call(addr, 502);
        writeln!(mover, "open").unwrap();
        assert_eq!(first.join().unwrap().unwrap(), 502);
    }

    /// A caller that connects and closes at once (or whose account cannot be read) is that
    /// caller's failure: the door stays open and the mover is served after it.
    #[test]
    fn a_caller_that_closes_at_once_never_closes_the_door() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();
        let (first, _) = served(listener, Duration::from_secs(5), Duration::ZERO);
        for _ in 0..3 {
            drop(TcpStream::connect(addr).unwrap());
        }
        std::thread::sleep(Duration::from_millis(200));
        let mut mover = call(addr, 502);
        writeln!(mover, "open").unwrap();
        assert_eq!(first.join().unwrap().unwrap(), 502);
    }

    /// A caller that drips its opening a byte at a time, each well inside any one read's wait, is
    /// stopped when its total time is up and never served.
    #[test]
    fn a_caller_dripping_its_opening_is_stopped_at_its_deadline() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();
        let time = Duration::from_millis(600);
        let (first, heard) = served(listener, time, Duration::ZERO);
        let mut drip = call(addr, 501);
        let started = Instant::now();
        for b in b"open\n" {
            std::thread::sleep(Duration::from_millis(250));
            if drip.write_all(&[*b]).is_err() {
                break;
            }
        }
        let deadline = Instant::now() + Duration::from_secs(5);
        while heard.lock().unwrap().is_empty() && Instant::now() < deadline {
            std::thread::sleep(Duration::from_millis(20));
        }
        assert_eq!(*heard.lock().unwrap(), vec![501]);
        assert!(started.elapsed() < Duration::from_secs(3));
        let mut mover = call(addr, 502);
        writeln!(mover, "open").unwrap();
        assert_eq!(first.join().unwrap().unwrap(), 502);
    }

    /// An opening that finishes after its caller was cut off (the approval check cannot be
    /// interrupted) never wins: the daemon never serves a connection it cut.
    #[test]
    fn an_opening_finished_after_the_cut_never_wins() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();
        let time = Duration::from_millis(300);
        let (first, heard) = served(listener, time, Duration::from_millis(700));
        let mut slow = call(addr, 501);
        writeln!(slow, "slow").unwrap();
        std::thread::sleep(Duration::from_millis(1100));
        assert_eq!(
            *heard.lock().unwrap(),
            vec![501],
            "the late opening was let go"
        );
        let mut mover = call(addr, 502);
        writeln!(mover, "open").unwrap();
        assert_eq!(first.join().unwrap().unwrap(), 502);
    }
}
