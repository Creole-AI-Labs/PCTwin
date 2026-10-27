//! The system tools the settings mechanisms use, each at its fixed path, with fixed arguments
//! built from checked catalogue names and typed values: never a shell, never a program named by
//! a catalogue or a peer.

use std::io::Read;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

/// The tools. Nothing else is ever run.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Tool {
    /// macOS `defaults`.
    Defaults,
    /// macOS `spctl` (only `--status`).
    Spctl,
    /// GNOME `gsettings`.
    Gsettings,
    /// KDE `kreadconfig6`.
    Kreadconfig6,
    /// KDE `kreadconfig5`.
    Kreadconfig5,
    /// GNOME `dconf` (dump, read, write, reset).
    Dconf,
    /// `xdg-settings` (only `get`).
    XdgSettings,
    /// Windows `netsh` (Wi-Fi profiles).
    Netsh,
}

impl Tool {
    /// The tool's fixed path.
    pub fn path(self) -> String {
        match self {
            Tool::Defaults => "/usr/bin/defaults".into(),
            Tool::Spctl => "/usr/sbin/spctl".into(),
            Tool::Gsettings => "/usr/bin/gsettings".into(),
            Tool::Kreadconfig6 => "/usr/bin/kreadconfig6".into(),
            Tool::Kreadconfig5 => "/usr/bin/kreadconfig5".into(),
            Tool::XdgSettings => "/usr/bin/xdg-settings".into(),
            Tool::Dconf => "/usr/bin/dconf".into(),
            Tool::Netsh => {
                let root = std::env::var_os("SystemRoot")
                    .map(|r| r.to_string_lossy().into_owned())
                    .filter(|r| r.len() > 2 && r.as_bytes()[1] == b':')
                    .unwrap_or_else(|| "C:\\Windows".into());
                format!("{root}\\System32\\netsh.exe")
            }
        }
    }
}

/// What a tool printed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Output {
    pub ok: bool,
    pub stdout: String,
}

/// Runs a tool. The real one is [`SystemRunner`]; tests use their own.
pub trait Runner {
    fn run(&mut self, tool: Tool, args: &[&str]) -> Result<Output, String>;
}

/// The most a tool's output may be.
pub const MAX_OUTPUT: u64 = 8 * 1024 * 1024;
/// How long a tool may take.
pub const TIMEOUT: Duration = Duration::from_secs(20);

/// Runs the real tools.
#[derive(Debug, Default)]
pub struct SystemRunner;

impl Runner for SystemRunner {
    fn run(&mut self, tool: Tool, args: &[&str]) -> Result<Output, String> {
        let mut cmd = Command::new(tool.path());
        // `netsh` reads `name="value"` itself; Rust's own quoting would wrap the whole argument.
        // Every netsh argument PCTwin builds is fixed text or a checked path without quotes.
        #[cfg(windows)]
        if tool == Tool::Netsh {
            use std::os::windows::process::CommandExt;
            for a in args {
                if a.contains(['\n', '\r', '\0']) {
                    return Err("an argument that cannot be passed safely".into());
                }
                cmd.raw_arg(a);
            }
        } else {
            cmd.args(args);
        }
        #[cfg(not(windows))]
        cmd.args(args);
        let mut child = cmd
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|e| format!("the system's settings tool could not start ({})", e.kind()))?;
        let mut out = child.stdout.take().ok_or("no output")?;
        let reader = std::thread::spawn(move || {
            let mut buf = Vec::new();
            let _ = (&mut out).take(MAX_OUTPUT).read_to_end(&mut buf);
            buf
        });
        let start = Instant::now();
        let status = loop {
            if let Some(s) = child.try_wait().map_err(|e| e.kind().to_string())? {
                break s;
            }
            if start.elapsed() > TIMEOUT {
                let _ = child.kill();
                let _ = child.wait();
                return Err("the system's settings tool did not answer in time".into());
            }
            std::thread::sleep(Duration::from_millis(10));
        };
        let bytes = reader.join().map_err(|_| "the output could not be read")?;
        Ok(Output {
            ok: status.success(),
            stdout: String::from_utf8_lossy(&bytes).into_owned(),
        })
    }
}
