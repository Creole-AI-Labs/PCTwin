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
//!
//! Stopping the move (X10) is a safe point: what is copied and checked stays, and the journal
//! picks up the rest. A pause that has not been reached is not.
//!
//! On a Mac and on Linux PCTwin is removed by the person or the package tool, which runs no hook:
//! the in-app update still waits, and the lock file is written for information only.

use std::collections::BTreeMap;
use std::fs::File;
use std::path::PathBuf;
use std::sync::{Mutex, MutexGuard, OnceLock};

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

type Notify = Box<dyn Fn(&SafeStopView) + Send + Sync>;

/// The steps going on now, and the lock file while any is.
pub struct SafeStop {
    inner: Mutex<Inner>,
}

#[derive(Default)]
struct Inner {
    busy: BTreeMap<BusyWith, u32>,
    file: Option<File>,
    lock: Option<PathBuf>,
    notify: Option<Notify>,
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

/// Opens `path` so that no other program may open it at all while PCTwin holds it (Windows:
/// share mode 0, through the standard library's safe API).
fn open_exclusive(path: &std::path::Path) -> std::io::Result<File> {
    let mut o = std::fs::OpenOptions::new();
    o.write(true).create(true).truncate(true);
    #[cfg(windows)]
    {
        use std::os::windows::fs::OpenOptionsExt;
        o.share_mode(0);
    }
    o.open(path)
}

impl SafeStop {
    /// A gate with no lock file (yet) and nobody told.
    pub const fn new() -> Self {
        Self {
            inner: Mutex::new(Inner {
                busy: BTreeMap::new(),
                file: None,
                lock: None,
                notify: None,
            }),
        }
    }

    fn lock(&self) -> MutexGuard<'_, Inner> {
        // A step that panicked still ended: its count is right, so carry on with it.
        self.inner.lock().unwrap_or_else(|e| e.into_inner())
    }

    /// Where the lock file goes, and who is told when the view changes. A step already going on
    /// takes the file at once.
    pub fn set_up(&self, lock: Option<PathBuf>, notify: Option<Notify>) {
        let mut inner = self.lock();
        inner.lock = lock;
        inner.notify = notify;
        if !inner.busy.is_empty() && inner.file.is_none() {
            inner.file = inner.lock.as_deref().and_then(|p| open_exclusive(p).ok());
        }
    }

    /// Starts a step of kind `what`.
    pub fn hold(&'static self, what: BusyWith) -> Hold {
        let mut inner = self.lock();
        let was = inner.view();
        if inner.busy.is_empty() && inner.file.is_none() {
            // Held before the step starts: an installer that could open the file a moment ago
            // finds it held now. If it cannot be made, the in-app update still waits.
            inner.file = inner.lock.as_deref().and_then(|p| open_exclusive(p).ok());
        }
        *inner.busy.entry(what).or_insert(0) += 1;
        Self::told(&inner, &was);
        Hold { stop: self, what }
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
    let app = app.clone();
    gate().set_up(
        lock,
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
        gate.set_up(Some(path.clone()), None);
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
        gate.set_up(Some(path.clone()), None);
        if cfg!(windows) {
            assert!(!opens());
        }
        drop(early);
        assert!(opens());
    }
}
