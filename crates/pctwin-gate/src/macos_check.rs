//! macOS: the system's list of programs that have a file open (security design notes).
//! macOS cannot prove that no other program can still write to a file, so undo never removes a
//! copy there (strengthened 13 October 2026); this list only says whether the copy is kept
//! because it is "in use" or because PCTwin "cannot check". It works by path, as a check, never
//! as the proof.
//!
//! PCTwin itself is never "another program". That is this process, and also a program it is
//! starting at that very moment: macOS makes the new process as a copy of PCTwin, holding every
//! file PCTwin holds (the copy being checked too), until the new program replaces it a moment
//! later and those files close. Seen in CI on 25 October 2026: a copy kept as "in use" while
//! another part of the same process was starting a program.

use std::io;
use std::path::Path;

/// How many times the system's list is asked for when it may have been cut short.
const LIST_TRIES: usize = 3;

/// Whether any program other than PCTwin has the file at `path` open, in any way: one that only
/// watches it for changes counts too (no filter is asked for, so the list is exactly every
/// program with the file open). The system's list is an Apple interface without a promise, so a
/// failure to read it is an error and the caller keeps the file. The list is made in a space
/// sized for every program running; a list that fills that space may have been cut short
/// (programs started meanwhile), so it is asked for again, and if it stays full, that is an error
/// too.
///
/// A program counts only if two lists in a row show it: one PCTwin was starting may have become
/// its new program between the list and the look at it (no longer holding the file, but no
/// longer looking like PCTwin either), and the second list no longer shows it.
pub(crate) fn others_have_it_open(path: &Path) -> io::Result<bool> {
    let first = others(path)?;
    if first.is_empty() {
        return Ok(false);
    }
    let again = others(path)?;
    Ok(first.iter().any(|p| again.contains(p)))
}

/// The processes the system's list shows with the file at `path` open, PCTwin's own left out.
fn others(path: &Path) -> io::Result<Vec<u32>> {
    let me = Me::now();
    for _ in 0..LIST_TRIES {
        let pids = libproc::processes::pids_by_path(path, false, false)?;
        let full = pids.len() >= pids.capacity();
        let others: Vec<u32> = pids.into_iter().filter(|p| !me.is(*p)).collect();
        if !others.is_empty() || !full {
            return Ok(others);
        }
    }
    Err(io::Error::other(
        "the list of programs with the file open may be incomplete",
    ))
}

/// This process: its number and its program.
struct Me {
    pid: u32,
    program: Option<String>,
}

impl Me {
    fn now() -> Self {
        let pid = std::process::id();
        Self {
            pid,
            program: program_of(pid),
        }
    }

    /// Whether `pid` is PCTwin itself: this process, or a process it is starting that is still
    /// a copy of it (its parent is this process and it still runs this program). Anything that
    /// cannot be looked at is another program.
    fn is(&self, pid: u32) -> bool {
        if pid == self.pid {
            return true;
        }
        let Ok(raw) = i32::try_from(pid) else {
            return false;
        };
        let child = libproc::proc_pid::pidinfo::<libproc::bsd_info::BSDInfo>(raw, 0)
            .is_ok_and(|info| info.pbi_ppid == self.pid);
        child && self.program.is_some() && program_of(pid) == self.program
    }
}

/// The program `pid` runs, by its path (`None` if it cannot be looked at).
fn program_of(pid: u32) -> Option<String> {
    libproc::proc_pid::pidpath(i32::try_from(pid).ok()?).ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::time::{Duration, Instant};

    /// A file PCTwin holds is never "in use by another program" because PCTwin is starting
    /// other programs at that moment (each starts as a copy of PCTwin holding the same files).
    /// The raw list is watched too, and how often it showed such a copy is said.
    #[test]
    fn programs_pctwin_starts_never_make_its_own_file_in_use() {
        let dir = tempfile::Builder::new()
            .suffix(".noindex")
            .tempdir()
            .unwrap();
        let path = dir.path().join("held.txt");
        std::fs::write(&path, b"held").unwrap();
        let _held = std::fs::File::open(&path).unwrap();
        let stop = AtomicBool::new(false);
        let me = std::process::id();
        let (mut asked, mut copies_seen) = (0u64, 0u64);
        let mut wrong = None;
        std::thread::scope(|s| {
            for _ in 0..3 {
                s.spawn(|| {
                    while !stop.load(Ordering::Relaxed) {
                        let _ = pctwin_types::process::quiet("/usr/bin/true").status();
                    }
                });
            }
            let began = Instant::now();
            while began.elapsed() < Duration::from_secs(20) && copies_seen < 20 {
                let raw = libproc::processes::pids_by_path(&path, false, false).unwrap();
                if raw.iter().any(|p| *p != me) {
                    copies_seen += 1;
                }
                asked += 1;
                let said = others_have_it_open(&path);
                if !matches!(said, Ok(false)) {
                    wrong = Some(said);
                    break;
                }
            }
            stop.store(true, Ordering::Relaxed);
        });
        eprintln!("{asked} asks; the raw list showed a starting copy {copies_seen} times");
        assert!(
            wrong.is_none(),
            "said {wrong:?} after {asked} asks ({copies_seen} starting copies seen)"
        );
    }

    /// Another program with the file open still counts, even one PCTwin started itself.
    #[test]
    fn a_program_pctwin_started_that_opens_the_file_counts() {
        use std::io::BufRead;
        let dir = tempfile::Builder::new()
            .suffix(".noindex")
            .tempdir()
            .unwrap();
        let path = dir.path().join("theirs.txt");
        std::fs::write(&path, b"theirs").unwrap();
        let script =
            "import sys, time\nf = open(sys.argv[1])\nprint('open', flush=True)\ntime.sleep(60)\n";
        let Ok(mut child) = pctwin_types::process::quiet("python3")
            .arg("-c")
            .arg(script)
            .arg(&path)
            .stdout(std::process::Stdio::piped())
            .spawn()
        else {
            eprintln!("skipped: no python3 to hold the file open");
            return;
        };
        let mut line = String::new();
        std::io::BufReader::new(child.stdout.take().unwrap())
            .read_line(&mut line)
            .unwrap();
        let said = others_have_it_open(&path);
        let _ = child.kill();
        let _ = child.wait();
        assert_eq!(line.trim(), "open");
        assert!(matches!(said, Ok(true)), "{said:?}");
    }
}
