//! The Mac launch daemon's door (Security Design part C, "The administrator's approval on a
//! Mac"): callers are taken as they come, each in its own thread with a short total time to open
//! a move, at most one waiting per account; the first valid opening is served and every other
//! caller is let go. So callers that connect and say nothing, or say it slowly, never queue the
//! real mover behind them (found by the second fresh-context review: one at a time, each held up
//! to its read timeout, they could keep the daemon busy for as long as they liked).

use std::collections::BTreeSet;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, mpsc};
use std::time::Duration;

use crate::opening::NotOpened;

/// The longest a caller has, from connecting, to open its move.
pub const OPENING_TIME: Duration = Duration::from_millis(2500);

/// One accepted caller: its connection, its account (user ID), and a way to end the connection
/// from another thread (what is waiting to read from it then ends).
pub struct Accepted<S> {
    pub stream: S,
    pub uid: u32,
    pub cut: Box<dyn FnOnce() + Send>,
}

/// Takes callers from `accept` until one opens a move with `open` within `time` of connecting:
/// that caller, its account and its opening. Root (uid 0) is never served, and a second caller
/// for an account that already has one waiting is let go at once. A caller that has not opened
/// its move in time is cut off. `log` hears why a caller was let go (for the helper's own log).
pub fn first_opening<S, O>(
    mut accept: impl FnMut() -> Result<Accepted<S>, String> + Send + 'static,
    open: impl Fn(&mut S, u32) -> Result<O, NotOpened> + Send + Sync + 'static,
    time: Duration,
    log: impl Fn(u32, &NotOpened) + Send + Sync + 'static,
) -> Result<(S, u32, O), String>
where
    S: Send + 'static,
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
                Err(why) => {
                    let _ = tx.send(Err(why));
                    return;
                }
            };
            let Accepted {
                mut stream,
                uid,
                cut,
            } = caller;
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
            let done = Arc::new(AtomicBool::new(false));
            let watch = Arc::clone(&done);
            std::thread::spawn(move || {
                std::thread::sleep(time);
                if !watch.load(Ordering::Acquire) {
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
                let opened = open(&mut stream, uid);
                done.store(true, Ordering::Release);
                if let Ok(mut w) = waiting.lock() {
                    w.remove(&uid);
                }
                match opened {
                    // The first valid opening is served; any later one is let go.
                    Ok(o) if !won.swap(true, Ordering::AcqRel) => {
                        let _ = tx.send(Ok((stream, uid, o)));
                    }
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
    use std::io::{BufRead, BufReader, Write};
    use std::net::{TcpListener, TcpStream};
    use std::time::Instant;

    /// A door on a local port: each connection says its account first ("uid 501"), standing
    /// for the system's word on who called. As the daemon does, each read has the opening's
    /// time at most, besides the cut (some systems wake a waiting read only by its timeout).
    fn door(
        listener: TcpListener,
        read_for: Duration,
    ) -> impl FnMut() -> Result<Accepted<BufReader<TcpStream>>, String> {
        move || {
            let (s, _) = listener.accept().map_err(|e| e.to_string())?;
            let cutter = s.try_clone().map_err(|e| e.to_string())?;
            s.set_read_timeout(Some(read_for))
                .map_err(|e| e.to_string())?;
            let mut r = BufReader::new(s);
            let mut line = String::new();
            r.read_line(&mut line).map_err(|e| e.to_string())?;
            let uid = line
                .trim()
                .strip_prefix("uid ")
                .and_then(|u| u.parse().ok())
                .unwrap_or(0);
            Ok(Accepted {
                stream: r,
                uid,
                cut: Box::new(move || {
                    let _ = cutter.shutdown(std::net::Shutdown::Both);
                }),
            })
        }
    }

    /// Opens a move when the caller says "open", within the time it is given.
    fn open(r: &mut BufReader<TcpStream>, _uid: u32) -> Result<(), NotOpened> {
        let mut line = String::new();
        match r.read_line(&mut line) {
            Ok(_) if line.trim() == "open" => Ok(()),
            _ => Err(NotOpened::Malformed("did not open".into())),
        }
    }

    fn call(addr: std::net::SocketAddr, uid: u32) -> TcpStream {
        let mut s = TcpStream::connect(addr).unwrap();
        writeln!(s, "uid {uid}").unwrap();
        s
    }

    #[test]
    fn silent_callers_never_queue_the_real_mover() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();
        let time = Duration::from_secs(2);
        let started = Instant::now();
        let first = std::thread::spawn(move || {
            first_opening(door(listener, time), open, time, |_, _| {}).map(|(_, uid, ())| uid)
        });
        // Five accounts connect and say nothing; then the mover opens a move.
        let silent: Vec<TcpStream> = (501..506).map(|u| call(addr, u)).collect();
        std::thread::sleep(Duration::from_millis(200));
        let mut mover = call(addr, 507);
        writeln!(mover, "open").unwrap();
        assert_eq!(first.join().unwrap().unwrap(), 507);
        assert!(
            started.elapsed() < time,
            "served after {:?}: the silent callers held it",
            started.elapsed()
        );
        drop(silent);
    }

    #[test]
    fn one_caller_waits_per_account_and_the_first_opening_wins() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();
        let first = std::thread::spawn(move || {
            first_opening(
                door(listener, Duration::from_secs(5)),
                open,
                Duration::from_secs(5),
                |_, _| {},
            )
            .map(|(_, uid, ())| uid)
        });
        // Account 501 waits silently; a second caller for 501 is let go at once (it reads the
        // end of the connection), and root is never served.
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
        assert!(matches!(
            std::io::Read::read(&mut again, &mut end),
            Ok(0) | Err(_)
        ));
        let mut mover = call(addr, 502);
        writeln!(mover, "open").unwrap();
        assert_eq!(first.join().unwrap().unwrap(), 502);
    }

    #[test]
    fn a_caller_too_slow_to_open_is_cut_off() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();
        let cut = Arc::new(Mutex::new(Vec::new()));
        let heard = Arc::clone(&cut);
        let first = std::thread::spawn(move || {
            first_opening(
                door(listener, Duration::from_millis(300)),
                open,
                Duration::from_millis(300),
                move |uid, _| heard.lock().unwrap().push(uid),
            )
            .map(|(_, uid, ())| uid)
        });
        let mut slow = call(addr, 501);
        std::thread::sleep(Duration::from_millis(800));
        // Too late: the connection was cut, so this opening is never read.
        let _ = writeln!(slow, "open");
        std::thread::sleep(Duration::from_millis(200));
        assert_eq!(*cut.lock().unwrap(), vec![501]);
        let mut mover = call(addr, 502);
        writeln!(mover, "open").unwrap();
        assert_eq!(first.join().unwrap().unwrap(), 502);
    }
}
