//! The system tools the settings mechanisms use, each at its fixed path, with fixed arguments
//! built from checked catalogue names and typed values: never a shell, never a program named by
//! a catalogue or a peer.

use std::io::Read;
use std::process::Stdio;
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
    /// Linux NetworkManager's `nmcli` (Wi-Fi profiles; secrets only on its standard input and
    /// output, never on its command line: [`Runner::run_secret`]).
    Nmcli,
    /// macOS `networksetup` (only reading the Wi-Fi device and its saved networks).
    Networksetup,
    /// macOS `security` (only `find-generic-password` for a saved Wi-Fi network's item in the
    /// System keychain; its password only on standard output, after macOS's own prompt).
    Security,
    /// macOS `open` (only a file PCTwin wrote, or a System Settings page).
    Open,
    /// Windows `whoami` (only `/groups`: whether the person is one of this laptop's
    /// administrators, for the 20 October 2026 Wi-Fi rule).
    Whoami,
    /// Mac and Linux `id` (only `-G` and `-Gn`: the person's groups, for the same rule).
    Id,
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
            Tool::Nmcli => "/usr/bin/nmcli".into(),
            Tool::Networksetup => "/usr/sbin/networksetup".into(),
            Tool::Security => "/usr/bin/security".into(),
            Tool::Open => "/usr/bin/open".into(),
            Tool::Id => "/usr/bin/id".into(),
            Tool::Netsh => format!("{}\\System32\\netsh.exe", windows_folder()),
            Tool::Whoami => format!("{}\\System32\\whoami.exe", windows_folder()),
        }
    }
}

/// The Windows folder, from `SystemRoot` when it names a drive's folder, else its usual place.
fn windows_folder() -> String {
    std::env::var_os("SystemRoot")
        .map(|r| r.to_string_lossy().into_owned())
        .filter(|r| r.len() > 2 && r.as_bytes()[1] == b':')
        .unwrap_or_else(|| "C:\\Windows".into())
}

/// What a tool printed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Output {
    pub ok: bool,
    pub stdout: String,
}

/// What a tool printed that may hold a secret: wiped from memory when dropped.
pub struct SecretOutput {
    pub ok: bool,
    pub stdout: zeroize::Zeroizing<String>,
}

/// Runs a tool. The real one is [`SystemRunner`]; tests use their own.
pub trait Runner {
    fn run(&mut self, tool: Tool, args: &[&str]) -> Result<Output, String>;

    /// Runs a tool that is given a secret, or prints one: the secret goes only on its standard
    /// input (`input`) and comes back only on its standard output, both wiped from memory
    /// afterwards; never on a command line, where other programs on the laptop can read it. A
    /// runner that cannot do this says so.
    fn run_secret(
        &mut self,
        tool: Tool,
        args: &[&str],
        input: Option<&[u8]>,
    ) -> Result<SecretOutput, String> {
        let _ = (tool, args, input);
        Err("this runner passes no secrets".into())
    }
}

/// The most a tool's output may be.
pub const MAX_OUTPUT: u64 = 8 * 1024 * 1024;
/// How long a tool may take.
pub const TIMEOUT: Duration = Duration::from_secs(20);
/// How long a tool that waits on the system's own prompt for a secret may take: macOS asks for
/// the administrator's password before `security` gives a saved Wi-Fi password, and the person
/// may take a while to type it.
pub const PROMPT_TIMEOUT: Duration = Duration::from_secs(5 * 60);

/// How long `tool` may take when it is given or prints a secret ([`Runner::run_secret`]).
pub fn secret_timeout(tool: Tool) -> Duration {
    match tool {
        Tool::Security => PROMPT_TIMEOUT,
        _ => TIMEOUT,
    }
}

/// Runs the real tools.
#[derive(Debug, Default)]
pub struct SystemRunner;

impl Runner for SystemRunner {
    fn run(&mut self, tool: Tool, args: &[&str]) -> Result<Output, String> {
        let mut cmd = pctwin_types::process::quiet(tool.path());
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

    fn run_secret(
        &mut self,
        tool: Tool,
        args: &[&str],
        input: Option<&[u8]>,
    ) -> Result<SecretOutput, String> {
        use std::io::Write;
        use zeroize::Zeroizing;
        let mut cmd = pctwin_types::process::quiet(tool.path());
        cmd.args(args)
            .env_clear()
            .env("LC_ALL", "C")
            .env("PATH", "/usr/bin:/bin")
            .stdin(if input.is_some() {
                Stdio::piped()
            } else {
                Stdio::null()
            })
            .stdout(Stdio::piped())
            .stderr(Stdio::null());
        let mut child = cmd
            .spawn()
            .map_err(|e| format!("the system's settings tool could not start ({})", e.kind()))?;
        let mut out = child.stdout.take().ok_or("no output")?;
        let reader = std::thread::spawn(move || {
            let mut buf = Zeroizing::new(Vec::new());
            let _ = (&mut out).take(MAX_OUTPUT).read_to_end(&mut buf);
            buf
        });
        if let (Some(bytes), Some(mut stdin)) = (input, child.stdin.take()) {
            // Written whole, then closed, so the tool sees the end of its input.
            let _ = stdin.write_all(bytes);
        }
        let start = Instant::now();
        let status = loop {
            if let Some(s) = child.try_wait().map_err(|e| e.kind().to_string())? {
                break s;
            }
            if start.elapsed() > secret_timeout(tool) {
                let _ = child.kill();
                let _ = child.wait();
                return Err("the system's settings tool did not answer in time".into());
            }
            std::thread::sleep(Duration::from_millis(10));
        };
        let bytes = reader.join().map_err(|_| "the output could not be read")?;
        let text = Zeroizing::new(String::from_utf8_lossy(&bytes).into_owned());
        Ok(SecretOutput {
            ok: status.success(),
            stdout: text,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_the_tool_behind_the_systems_password_prompt_waits_longer() {
        assert_eq!(secret_timeout(Tool::Security), PROMPT_TIMEOUT);
        assert!(PROMPT_TIMEOUT >= Duration::from_secs(5 * 60));
        for t in [
            Tool::Nmcli,
            Tool::Netsh,
            Tool::Networksetup,
            Tool::Open,
            Tool::Defaults,
        ] {
            assert_eq!(secret_timeout(t), TIMEOUT, "{t:?}");
        }
    }
}
