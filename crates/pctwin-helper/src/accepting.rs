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
use std::sync::atomic::{AtomicBool, AtomicU8, Ordering};
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
    /// What an accept error means for the door: the system's own codes for "this caller" and
    /// "too many open" are kept apart from a door that is gone.
    pub fn of(e: &io::Error) -> Self {
        match e.kind() {
            io::ErrorKind::Interrupted
            | io::ErrorKind::ConnectionAborted
            | io::ErrorKind::ConnectionReset
            | io::ErrorKind::WouldBlock
            | io::ErrorKind::TimedOut
            | io::ErrorKind::UnexpectedEof
            | io::ErrorKind::BrokenPipe
            | io::ErrorKind::NotConnected
            | io::ErrorKind::PermissionDenied => Self::Caller(e.to_string()),
            _ if is_busy(e) => Self::Busy,
            _ => Self::Closed(e.to_string()),
        }
    }
}

/// Too many files open, for this process or the whole system.
fn is_busy(e: &io::Error) -> bool {
    #[cfg(unix)]
    {
        e.raw_os_error().is_some_and(|c| {
            c == rustix::io::Errno::MFILE.raw_os_error()
                || c == rustix::io::Errno::NFILE.raw_os_error()
        })
    }
    #[cfg(not(unix))]
    {
        let _ = e;
        false
    }
}

/// A caller's state: it moves once, from waiting, to opened or to cut off.
const WAITING: u8 = 0;
const OPENED: u8 = 1;
const CUT: u8 = 2;

/// Takes callers from `accept` until one opens a move with `open` within `time` of connecting:
/// that caller (its connection, the deadline taken off), its account and its opening. Root (uid
/// 0) is never served, and a second caller for an account that already has one waiting is let go
/// at once. A caller whose time runs out is cut off and can never be served. `log` hears why a
/// caller was let go (for the helper's own log).
pub fn first_opening<S, O>(
    mut accept: impl FnMut() -> Result<Accepted<S>, NotAccepted> + Send + 'static,
    open: impl Fn(&mut Deadlined<S>, u32) -> Result<O, NotOpened> + Send + Sync + 'static,
    time: Duration,
    log: impl Fn(u32, &NotOpened) + Send + Sync + 'static,
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
        loop {
            let caller = match accept() {
                Ok(c) => c,
                Err(NotAccepted::Caller(why)) => {
                    eprintln!("pctwin-helper: a caller was let go: {why}");
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
            // At most one caller waiting per account.
            let fresh = waiting.lock().map(|mut w| w.insert(uid)).unwrap_or(false);
            if !fresh {
                cut();
                continue;
            }
            let state = Arc::new(AtomicU8::new(WAITING));
            let watch = Arc::clone(&state);
            std::thread::spawn(move || {
                std::thread::sleep(time);
                // Cut off only a caller still waiting; one already opened is left be.
                if watch
                    .compare_exchange(WAITING, CUT, Ordering::AcqRel, Ordering::Acquire)
                    .is_ok()
                {
                    cut();
                }
            });
            let (tx, open, log, won, waiting) = (
                tx.clone(),
                Arc::clone(&open),
                Arc::clone(&log),
                Arc::clone(&won),
                Arc::clone(&waiting),
            );
            std::thread::spawn(move || {
                let mut stream = Deadlined::new(stream, until);
                let opened = open(&mut stream, uid);
                // Opened in time, or not at all: a caller already cut off never counts.
                let in_time = Instant::now() <= until
                    && state
                        .compare_exchange(WAITING, OPENED, Ordering::AcqRel, Ordering::Acquire)
                        .is_ok();
                if let Ok(mut w) = waiting.lock() {
                    w.remove(&uid);
                }
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
    fn door(listener: TcpListener) -> impl FnMut() -> Result<Accepted<TcpStream>, NotAccepted> {
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
            first_opening(door(listener), opener(late), time, move |uid, _| {
                log.lock().unwrap().push(uid)
            })
            .map(|(_, uid, ())| uid)
        });
        (first, heard)
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
