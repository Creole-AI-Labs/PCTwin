//! Running the system's own checking tools: a fixed program, fixed arguments (never a shell
//! string), a time limit and a cap on how much output is kept.

use std::ffi::OsString;
use std::io::Read;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant};

/// The most output kept from a tool, per stream.
pub const MAX_OUTPUT: usize = 1024 * 1024;

/// One call of a system tool.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Call {
    pub program: PathBuf,
    pub args: Vec<OsString>,
    /// The tool's whole environment: nothing of this process's own is passed on (a value such
    /// as `PATH` or `SystemRoot` set by whoever started PCTwin could make a tool load other
    /// code; second review, finding B3).
    pub env: Vec<(OsString, OsString)>,
    pub timeout: Duration,
}

/// What a tool did.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Ran {
    /// None when it was stopped by a signal.
    pub code: Option<i32>,
    pub stdout: String,
    pub stderr: String,
}

/// Why a tool gave no answer.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum RunError {
    #[error("the tool could not be started ({0})")]
    NotStarted(String),
    #[error("the tool did not finish in time")]
    TimedOut,
    #[error("the tool's output could not be read ({0})")]
    Output(String),
}

/// Runs system tools. Tests use a stand-in that answers from captured output.
pub trait Runner {
    fn run(&self, call: &Call) -> Result<Ran, RunError>;
}

/// Runs the real tools.
#[derive(Debug, Default, Clone, Copy)]
pub struct SystemRunner;

type Drained = mpsc::Receiver<Result<Vec<u8>, String>>;

/// Reads a stream on its own thread, keeping at most [`MAX_OUTPUT`] bytes. The answer comes on a
/// channel so the caller can stop waiting: a tool's child can hold the stream open after the tool
/// itself has finished.
fn drain(mut r: impl Read + Send + 'static) -> Drained {
    let (tx, rx) = mpsc::channel();
    thread::spawn(move || {
        let mut kept = Vec::new();
        let mut buf = [0u8; 8192];
        let result = loop {
            match r.read(&mut buf) {
                Ok(0) => break Ok(kept),
                Ok(n) => {
                    let room = MAX_OUTPUT.saturating_sub(kept.len());
                    kept.extend_from_slice(&buf[..n.min(room)]);
                }
                Err(e) => break Err(e.to_string()),
            }
        };
        let _ = tx.send(result);
    });
    rx
}

impl Runner for SystemRunner {
    fn run(&self, call: &Call) -> Result<Ran, RunError> {
        let mut cmd = Command::new(&call.program);
        cmd.args(&call.args)
            .env_clear()
            .envs(call.env.iter().map(|(k, v)| (k, v)))
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        let mut child = cmd
            .spawn()
            .map_err(|e| RunError::NotStarted(e.to_string()))?;
        let out = drain(
            child
                .stdout
                .take()
                .ok_or(RunError::Output("no stdout".into()))?,
        );
        let err = drain(
            child
                .stderr
                .take()
                .ok_or(RunError::Output("no stderr".into()))?,
        );
        let start = Instant::now();
        let status = loop {
            match child.try_wait() {
                Ok(Some(s)) => break s,
                Ok(None) if start.elapsed() >= call.timeout => {
                    let _ = child.kill();
                    let _ = child.wait();
                    return Err(RunError::TimedOut);
                }
                Ok(None) => thread::sleep(Duration::from_millis(20)),
                Err(e) => return Err(RunError::Output(e.to_string())),
            }
        };
        let take = |rx: Drained| {
            let left = call.timeout.saturating_sub(start.elapsed());
            rx.recv_timeout(left.max(Duration::from_secs(1)))
                .map_err(|_| RunError::TimedOut)?
                .map_err(RunError::Output)
                .map(|b| String::from_utf8_lossy(&b).into_owned())
        };
        Ok(Ran {
            code: status.code(),
            stdout: take(out)?,
            stderr: take(err)?,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(windows)]
    fn shell(script: &str) -> Call {
        Call {
            program: PathBuf::from(r"C:\Windows\System32\cmd.exe"),
            args: vec!["/d".into(), "/c".into(), script.into()],
            env: vec![("PCTWIN_RUN_TEST".into(), "kept".into())],
            timeout: Duration::from_secs(20),
        }
    }

    #[cfg(unix)]
    fn shell(script: &str) -> Call {
        Call {
            program: PathBuf::from("/bin/sh"),
            args: vec!["-c".into(), script.into()],
            env: vec![("PCTWIN_RUN_TEST".into(), "kept".into())],
            timeout: Duration::from_secs(20),
        }
    }

    #[test]
    fn output_code_and_environment_come_back() {
        #[cfg(windows)]
        let r = SystemRunner
            .run(&shell("echo %PCTWIN_RUN_TEST% & echo oops 1>&2 & exit 3"))
            .unwrap();
        #[cfg(unix)]
        let r = SystemRunner
            .run(&shell("echo $PCTWIN_RUN_TEST; echo oops >&2; exit 3"))
            .unwrap();
        assert_eq!(r.code, Some(3));
        assert!(r.stdout.contains("kept"), "{r:?}");
        assert!(r.stderr.contains("oops"), "{r:?}");
        // Nothing of this process's own environment is passed on (second review, B3).
        #[cfg(windows)]
        let r = SystemRunner
            .run(&shell("echo [%USERNAME%] [%PATH%]"))
            .unwrap();
        #[cfg(unix)]
        let r = SystemRunner
            .run(&shell("echo \"[$HOME] [$PATH]\""))
            .unwrap();
        #[cfg(windows)]
        assert!(r.stdout.contains("[%USERNAME%] [%PATH%]"), "{r:?}");
        #[cfg(unix)]
        assert!(
            !r.stdout.contains("/home") && !r.stdout.contains("/Users"),
            "{r:?}"
        );
    }

    #[test]
    fn a_tool_that_hangs_is_stopped() {
        #[cfg(windows)]
        let mut c = shell(r"C:\Windows\System32\PING.EXE -n 30 127.0.0.1 > nul");
        #[cfg(unix)]
        let mut c = shell("/bin/sleep 30");
        c.timeout = Duration::from_millis(300);
        let start = Instant::now();
        assert_eq!(SystemRunner.run(&c), Err(RunError::TimedOut));
        assert!(start.elapsed() < Duration::from_secs(10));
    }

    #[test]
    fn a_flood_of_output_is_capped() {
        #[cfg(windows)]
        let c = shell("for /L %i in (1,1,40000) do @echo 0123456789012345678901234567890123456789");
        #[cfg(unix)]
        let c = shell("yes 0123456789012345678901234567890123456789 | head -n 40000");
        let r = SystemRunner.run(&c).unwrap();
        assert_eq!(r.stdout.len(), MAX_OUTPUT);
    }

    #[test]
    fn a_missing_tool_is_not_started() {
        let c = Call {
            program: PathBuf::from("/no/such/pctwin/tool"),
            args: vec![],
            env: vec![],
            timeout: Duration::from_secs(5),
        };
        assert!(matches!(SystemRunner.run(&c), Err(RunError::NotStarted(_))));
    }
}
