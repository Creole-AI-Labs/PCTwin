//! PCTwin's own update and uninstall wait for the engine's safe stopping points (Task List 2.9,
//! "Changes during the move").
//!
//! Each step that must not be cut part way (a move's run, rebuilding apps and settings, placing
//! or reading app data, an undo) holds a [`Hold`] for as long as it runs. While any is held:
//!
//! - the in-app update refuses to start ([`crate::updates::update_install`] answers `Busy`), and
//!   the screens can say what it waits for ([`safe_stop`], and the [`EVENT`] when it changes);
//! - on Windows, PCTwin keeps one file open with no sharing at all (`safe-stop.lock` in its local
//!   app data folder). The installer and uninstaller (windows/hooks.nsh) wait while that file
//!   cannot be opened, before they close PCTwin: NSIS's own `FileOpen` fails on a file another
//!   program holds with no sharing, which is the only case its check can see, so the file is held
//!   exactly that way. The file is closed when the last step ends, so the installer then goes on.
//!   The installer looks at every account's file (each profile in the system's list), so a move
//!   another signed-in person runs is waited for too;
//! - between the installer finding PCTwin at a safe point and closing it, no step may start: the
//!   installer keeps [`CLOSING_FILE`] beside the program open while it checks again and closes
//!   PCTwin, and a first step that finds that file held does not start ([`SafeStop::hold`] waits;
//!   PCTwin is closed a moment later). A step takes its own file first and then looks for the
//!   installer's; the installer makes its file first and then looks at the steps' files; so one of
//!   the two always sees the other;
//! - while an update is being installed ([`SafeStop::begin_update`]), no step starts at all: a
//!   step asked for then waits, and [`SafeStop::try_hold`] refuses.
//!
//! Stopping the move (X10) is a safe point: what is copied and checked stays, and the journal
//! picks up the rest. A pause that has not been reached is not.
//!
//! On a Mac and on Linux PCTwin is removed by the person or the package tool, which runs no hook:
//! the in-app update still waits, and the lock file is written for information only.

use std::collections::BTreeMap;
use std::fs::File;
use std::path::{Path, PathBuf};
use std::sync::{Condvar, Mutex, MutexGuard, OnceLock};
use std::time::{Duration, Instant};

use serde::Serialize;

pctwin_types::listed_enum! {
    /// What PCTwin is in the middle of, that an update or uninstall waits for.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
    #[serde(rename_all = "kebab-case")]
    pub enum BusyWith {
        /// Moving files (stopping the move is a safe point: what is copied and checked stays).
        Moving,
        /// Installing apps and putting settings in place.
        Rebuilding,
        /// Reading or placing app data.
        AppData,
        /// Undoing a move.
        Undoing,
    }
}

/// Whether PCTwin may be updated or removed now, and if not, what it waits for.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SafeStopView {
    pub safe: bool,
    pub busy: Vec<BusyWith>,
}

/// The event sent when PCTwin reaches, or leaves, a safe stopping point.
pub const EVENT: &str = "safe-stop";

/// The lock file's name in PCTwin's local app data folder (windows/hooks.nsh names it too).
pub const LOCK_FILE: &str = "safe-stop.lock";

/// The file the Windows installer keeps open beside the program while it closes PCTwin
/// (windows/hooks.nsh names it too).
pub const CLOSING_FILE: &str = "safe-stop-closing";

/// How long a step tries again to take its file while the installer is looking at it.
const PROBE_WAIT: Duration = Duration::from_secs(3);

type Notify = Box<dyn Fn(&SafeStopView) + Send + Sync>;

/// The steps going on now, and the lock file while any is.
pub struct SafeStop {
    inner: Mutex<Inner>,
    /// Told when an update stops being installed.
    changed: Condvar,
}

#[derive(Default)]
struct Inner {
    busy: BTreeMap<BusyWith, u32>,
    file: Option<File>,
    lock: Option<PathBuf>,
    /// The installer's file while it closes PCTwin ([`CLOSING_FILE`]).
    closing: Option<PathBuf>,
    notify: Option<Notify>,
    /// An update is being installed: no step starts.
    updating: bool,
}

impl Inner {
    fn view(&self) -> SafeStopView {
        let busy: Vec<BusyWith> = self.busy.keys().copied().collect();
        SafeStopView {
            safe: busy.is_empty(),
            busy,
        }
    }
}

/// One step in progress; the step ends when this is dropped.
#[must_use = "the step is over as soon as its hold is dropped"]
pub struct Hold {
    stop: &'static SafeStop,
    what: BusyWith,
}

impl Drop for Hold {
    fn drop(&mut self) {
        self.stop.end(self.what);
    }
}

/// An update being installed; when this is dropped (the install failed), steps may start again.
#[must_use = "steps may start again as soon as this is dropped"]
pub struct Staged {
    stop: &'static SafeStop,
}

impl Drop for Staged {
    fn drop(&mut self) {
        self.stop.lock().updating = false;
        self.stop.changed.notify_all();
    }
}

fn no_sharing(o: &mut std::fs::OpenOptions) {
    #[cfg(windows)]
    {
        use std::os::windows::fs::OpenOptionsExt;
        o.share_mode(0);
    }
    #[cfg(not(windows))]
    let _ = o;
}

/// Opens `path` so that no other program may open it at all while PCTwin holds it (Windows:
/// share mode 0, through the standard library's safe API).
fn open_exclusive(path: &Path) -> std::io::Result<File> {
    let mut o = std::fs::OpenOptions::new();
    o.write(true).create(true).truncate(true);
    no_sharing(&mut o);
    o.open(path)
}

/// Whether `err` is Windows saying another program has the file open (`ERROR_SHARING_VIOLATION`).
fn shared_elsewhere(err: &std::io::Error) -> bool {
    cfg!(windows) && err.raw_os_error() == Some(32)
}

/// [`open_exclusive`], tried again for a moment while another program has the file open: the
/// installer opens it for an instant to see whether PCTwin holds it.
fn open_exclusive_waiting(path: &Path) -> Option<File> {
    let until = Instant::now() + PROBE_WAIT;
    loop {
        match open_exclusive(path) {
            Ok(f) => return Some(f),
            Err(e) if shared_elsewhere(&e) && Instant::now() < until => {
                std::thread::sleep(Duration::from_millis(50));
            }
            Err(_) => return None,
        }
    }
}

/// Whether the installer is closing PCTwin now: its file is there and another program (the
/// installer) has it open. A file left behind by an installer that stopped is not held.
fn installer_closing(path: &Path) -> bool {
    if std::fs::symlink_metadata(path).is_err() {
        return false;
    }
    let mut o = std::fs::OpenOptions::new();
    o.read(true);
    no_sharing(&mut o);
    match o.open(path) {
        Ok(_) => false,
        Err(e) => shared_elsewhere(&e),
    }
}

impl SafeStop {
    /// A gate with no lock file (yet) and nobody told.
    pub const fn new() -> Self {
        Self {
            inner: Mutex::new(Inner {
                busy: BTreeMap::new(),
                file: None,
                lock: None,
                closing: None,
                notify: None,
                updating: false,
            }),
            changed: Condvar::new(),
        }
    }

    fn lock(&self) -> MutexGuard<'_, Inner> {
        // A step that panicked still ended: its count is right, so carry on with it.
        self.inner.lock().unwrap_or_else(|e| e.into_inner())
    }

    /// Where the lock file goes, where the installer's file is ([`CLOSING_FILE`]), and who is
    /// told when the view changes. A step already going on takes the file at once.
    pub fn set_up(&self, lock: Option<PathBuf>, closing: Option<PathBuf>, notify: Option<Notify>) {
        let mut inner = self.lock();
        inner.lock = lock;
        inner.closing = closing;
        inner.notify = notify;
        if !inner.busy.is_empty() && inner.file.is_none() {
            inner.file = inner.lock.as_deref().and_then(open_exclusive_waiting);
        }
    }

    /// Takes the file for a first step. `false`: the installer is closing PCTwin right now, so
    /// the step must not start (the file is let go again, so the installer sees PCTwin idle).
    fn take_file(inner: &mut Inner) -> bool {
        if !inner.busy.is_empty() || inner.file.is_some() {
            return true;
        }
        // Held before the step starts: an installer that could open the file a moment ago finds
        // it held now. If it cannot be made, the in-app update still waits.
        inner.file = inner.lock.as_deref().and_then(open_exclusive_waiting);
        // Only then is the installer's file looked at (see the module's notes).
        if inner.closing.as_deref().is_some_and(installer_closing) {
            inner.file = None;
            return false;
        }
        true
    }

    fn start(&'static self, inner: &mut Inner, what: BusyWith) -> Hold {
        let was = inner.view();
        *inner.busy.entry(what).or_insert(0) += 1;
        Self::told(inner, &was);
        Hold { stop: self, what }
    }

    /// Starts a step of kind `what`. While an update is being installed, or the installer is
    /// closing PCTwin, it waits first (PCTwin is about to close; an update that failed lets it
    /// start). It never starts without its file while an installer holds its own open (re-check
    /// of 2.9): a file left behind by an installer that stopped is not held, so it never waits.
    pub fn hold(&'static self, what: BusyWith) -> Hold {
        let mut inner = self.lock();
        loop {
            if inner.updating {
                inner = self.changed.wait(inner).unwrap_or_else(|e| e.into_inner());
                continue;
            }
            if Self::take_file(&mut inner) {
                return self.start(&mut inner, what);
            }
            inner = self
                .changed
                .wait_timeout(inner, Duration::from_millis(250))
                .unwrap_or_else(|e| e.into_inner())
                .0;
        }
    }

    /// Starts a step of kind `what` only if nothing else is going on, no update is being
    /// installed and the installer is not closing PCTwin: checked and held in one go, so two
    /// steps can never both find PCTwin free.
    pub fn try_hold(&'static self, what: BusyWith) -> Option<Hold> {
        let mut inner = self.lock();
        if inner.updating || !inner.busy.is_empty() || !Self::take_file(&mut inner) {
            return None;
        }
        Some(self.start(&mut inner, what))
    }

    /// Stops any step from starting while an update is installed: `None` when a step is going
    /// on now. Checked and set in one go, so no step can start between the check and the
    /// install.
    pub fn begin_update(&'static self) -> Option<Staged> {
        let mut inner = self.lock();
        if inner.updating || !inner.busy.is_empty() {
            return None;
        }
        inner.updating = true;
        Some(Staged { stop: self })
    }

    fn end(&self, what: BusyWith) {
        let mut inner = self.lock();
        let was = inner.view();
        if let Some(n) = inner.busy.get_mut(&what) {
            *n -= 1;
            if *n == 0 {
                inner.busy.remove(&what);
            }
        }
        if inner.busy.is_empty() {
            // Closing the file is the safe point the installer waits for.
            inner.file = None;
        }
        Self::told(&inner, &was);
    }

    fn told(inner: &Inner, was: &SafeStopView) {
        let now = inner.view();
        if now != *was
            && let Some(notify) = &inner.notify
        {
            notify(&now);
        }
    }

    /// Whether PCTwin may stop now, and what it waits for.
    pub fn view(&self) -> SafeStopView {
        self.lock().view()
    }
}

impl Default for SafeStop {
    fn default() -> Self {
        Self::new()
    }
}

/// The program's one gate.
pub fn gate() -> &'static SafeStop {
    static GATE: OnceLock<SafeStop> = OnceLock::new();
    GATE.get_or_init(SafeStop::new)
}

/// Starts a step of kind `what` on the program's gate.
pub fn hold(what: BusyWith) -> Hold {
    gate().hold(what)
}

/// Sets the program's gate up as the app starts: its lock file in PCTwin's local app data folder,
/// and the screens told of every change.
pub fn set_up<R: tauri::Runtime>(app: &tauri::AppHandle<R>) {
    use tauri::{Emitter, Manager};
    let lock = app.path().app_local_data_dir().ok().and_then(|d| {
        std::fs::create_dir_all(&d).ok()?;
        Some(d.join(LOCK_FILE))
    });
    // The installer's file, beside the program it installed (Windows only: nothing else runs a
    // hook).
    let closing = if cfg!(windows) {
        std::env::current_exe()
            .ok()
            .and_then(|exe| Some(exe.parent()?.join(CLOSING_FILE)))
    } else {
        None
    };
    let app = app.clone();
    gate().set_up(
        lock,
        closing,
        Some(Box::new(move |v: &SafeStopView| {
            let _ = app.emit(EVENT, v.clone());
        })),
    );
}

/// Whether PCTwin may be updated or removed now.
#[tauri::command]
pub fn safe_stop() -> SafeStopView {
    gate().view()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    fn leaked() -> &'static SafeStop {
        Box::leak(Box::new(SafeStop::new()))
    }

    #[test]
    fn it_is_safe_only_when_every_step_has_ended() {
        let gate = leaked();
        let told = Arc::new(Mutex::new(Vec::new()));
        let t = told.clone();
        gate.set_up(
            None,
            None,
            Some(Box::new(move |v: &SafeStopView| {
                t.lock().unwrap().push(v.clone())
            })),
        );
        assert_eq!(
            gate.view(),
            SafeStopView {
                safe: true,
                busy: vec![]
            }
        );
        let moving = gate.hold(BusyWith::Moving);
        let undo1 = gate.hold(BusyWith::Undoing);
        let undo2 = gate.hold(BusyWith::Undoing);
        assert_eq!(
            gate.view(),
            SafeStopView {
                safe: false,
                busy: vec![BusyWith::Moving, BusyWith::Undoing]
            }
        );
        drop(moving);
        drop(undo1);
        // One undo still going: not safe.
        assert_eq!(gate.view().busy, vec![BusyWith::Undoing]);
        assert!(!gate.view().safe);
        drop(undo2);
        assert!(gate.view().safe);
        // Told once per change, never for a change that changed nothing (the second undo).
        let told = told.lock().unwrap();
        let safe: Vec<bool> = told.iter().map(|v| v.safe).collect();
        assert_eq!(safe, [false, false, false, true]);
        assert_eq!(told.len(), 4, "{told:?}");
    }

    #[test]
    fn a_step_that_panicked_still_ends() {
        let gate = leaked();
        let r = std::thread::spawn(move || {
            let _h = gate.hold(BusyWith::Rebuilding);
            panic!("a step went wrong");
        })
        .join();
        assert!(r.is_err());
        assert!(gate.view().safe);
    }

    /// The installer's check (windows/hooks.nsh): while any step runs, the lock file cannot be
    /// opened by another program; once the last ends, it can. `File::open` asks for the ordinary
    /// sharing, as NSIS's `FileOpen` does.
    #[test]
    fn the_lock_file_is_held_while_and_only_while_a_step_runs() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join(LOCK_FILE);
        let gate = leaked();
        gate.set_up(Some(path.clone()), None, None);
        let opens = || {
            std::fs::OpenOptions::new()
                .read(true)
                .write(true)
                .open(&path)
                .is_ok()
        };
        let a = gate.hold(BusyWith::AppData);
        assert!(path.exists());
        let b = gate.hold(BusyWith::Moving);
        drop(a);
        if cfg!(windows) {
            assert!(!opens(), "held with no sharing while a step runs");
        }
        drop(b);
        assert!(opens(), "free once every step has ended");
        // A step started before the gate knew its file takes it as soon as it does.
        let gate = leaked();
        let early = gate.hold(BusyWith::Undoing);
        gate.set_up(Some(path.clone()), None, None);
        if cfg!(windows) {
            assert!(!opens());
        }
        drop(early);
        assert!(opens());
    }

    /// Fresh-context review F7: "is anything going on?" and taking the step are one act, so two
    /// steps can never both find PCTwin free.
    #[test]
    fn try_hold_checks_and_holds_in_one_go() {
        let gate = leaked();
        let first = gate.try_hold(BusyWith::Rebuilding).expect("free");
        assert!(gate.try_hold(BusyWith::Undoing).is_none());
        assert!(gate.try_hold(BusyWith::Rebuilding).is_none());
        assert_eq!(gate.view().busy, vec![BusyWith::Rebuilding]);
        drop(first);
        assert!(gate.try_hold(BusyWith::Undoing).is_some());
        assert!(gate.view().safe);
    }

    /// Fresh-context review F2: once an update is being installed no step starts: a step asked
    /// for waits until the install failed, and `try_hold` refuses. An update cannot be staged
    /// while a step runs.
    #[test]
    fn no_step_starts_while_an_update_is_installed() {
        let gate = leaked();
        let step = gate.hold(BusyWith::Moving);
        assert!(gate.begin_update().is_none(), "a step is going on");
        drop(step);
        let staged = gate.begin_update().expect("safe now");
        assert!(gate.begin_update().is_none(), "one update at a time");
        assert!(gate.try_hold(BusyWith::Undoing).is_none());
        let (tx, rx) = std::sync::mpsc::channel();
        let waiter = std::thread::spawn(move || {
            let h = gate.hold(BusyWith::Rebuilding);
            tx.send(()).unwrap();
            drop(h);
        });
        assert!(
            rx.recv_timeout(Duration::from_millis(400)).is_err(),
            "the step waits while the update is installed"
        );
        assert!(gate.view().safe);
        drop(staged);
        rx.recv_timeout(Duration::from_secs(5))
            .expect("the step starts once the install failed");
        waiter.join().unwrap();
    }

    /// Fresh-context review F4: the installer opens the lock file for an instant to see whether
    /// PCTwin holds it; a step starting in that instant still takes the file once the installer
    /// lets go, instead of running with nothing held.
    #[cfg(windows)]
    #[test]
    fn a_step_takes_its_file_after_the_installers_look() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join(LOCK_FILE);
        std::fs::write(&path, b"").unwrap();
        let gate = leaked();
        gate.set_up(Some(path.clone()), None, None);
        // The installer's look: ordinary sharing, held for a moment.
        let look = std::fs::File::open(&path).unwrap();
        let let_go = std::thread::spawn(move || {
            std::thread::sleep(Duration::from_millis(300));
            drop(look);
        });
        let step = gate.hold(BusyWith::Moving);
        let_go.join().unwrap();
        assert!(
            std::fs::File::open(&path).is_err(),
            "the step holds the file with no sharing"
        );
        drop(step);
        assert!(std::fs::File::open(&path).is_ok());
    }

    /// Fresh-context review F4 (the gap between the installer's check and its closing PCTwin):
    /// while the installer holds its file open, a first step does not start, and its own file
    /// stays free so the installer, checking again, sees PCTwin idle. A file the installer left
    /// behind and no longer holds stops nothing.
    #[cfg(windows)]
    #[test]
    fn a_first_step_waits_while_the_installer_closes_pctwin() {
        let dir = tempfile::tempdir().unwrap();
        let lock = dir.path().join(LOCK_FILE);
        let closing = dir.path().join(CLOSING_FILE);
        std::fs::write(&closing, b"").unwrap();
        let gate = leaked();
        gate.set_up(Some(lock.clone()), Some(closing.clone()), None);
        // Left behind, not held: the step starts at once.
        let h = gate
            .try_hold(BusyWith::Moving)
            .expect("a file left behind stops nothing");
        drop(h);
        let installer = std::fs::File::open(&closing).unwrap();
        assert!(gate.try_hold(BusyWith::Moving).is_none());
        assert!(gate.view().safe);
        let (tx, rx) = std::sync::mpsc::channel();
        let waiter = std::thread::spawn(move || {
            let h = gate.hold(BusyWith::Rebuilding);
            tx.send(()).unwrap();
            drop(h);
        });
        assert!(
            rx.recv_timeout(Duration::from_millis(600)).is_err(),
            "no step starts while the installer closes PCTwin"
        );
        assert!(
            std::fs::OpenOptions::new()
                .read(true)
                .write(true)
                .open(&lock)
                .is_ok(),
            "the installer, checking again, finds PCTwin idle"
        );
        drop(installer);
        rx.recv_timeout(Duration::from_secs(5))
            .expect("the step starts once the installer let go");
        waiter.join().unwrap();
    }
}
