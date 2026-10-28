//! Starting the fetch process, locked down, for one download (see the module notes of
//! [`super`]). The app hands it a [`Request`] on its standard input and reads one [`Outcome`] from
//! its standard output; it never shares memory, files or rights with it beyond its own staging
//! folder.

use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use super::download::{Outcome, Request};

/// The fetch program's file name.
#[cfg(windows)]
pub const PROGRAM: &str = "pctwin-fetch.exe";
#[cfg(not(windows))]
pub const PROGRAM: &str = "pctwin-fetch";
/// The largest answer read from the fetch process.
const MAX_ANSWER: u64 = 64 * 1024;
/// The largest request sent to it.
pub const MAX_REQUEST: usize = 64 * 1024;

/// Why the fetch process could not be used.
#[derive(Debug, thiserror::Error)]
pub enum ProcessError {
    #[error("PCTwin's download program is missing")]
    Missing,
    #[error("PCTwin's download program could not be prepared ({0})")]
    Prepare(String),
    #[error("PCTwin's download program could not be started ({0})")]
    Start(String),
    #[error("PCTwin's download program did not finish in time")]
    TimedOut,
    #[error("PCTwin's download program gave no usable answer")]
    NoAnswer,
    #[error("the download must be made in the download program's own folder")]
    OutsideStaging,
}

/// The fetch process, ready to run.
#[derive(Debug, Clone)]
pub struct Fetcher {
    /// The program as installed beside PCTwin.
    program: PathBuf,
    /// Its own folder: its copy of itself, its staging and its temporary files.
    home: PathBuf,
}

#[cfg(windows)]
fn sha(path: &Path) -> std::io::Result<[u8; 32]> {
    use sha2::{Digest, Sha256};
    let mut f = std::fs::File::open(path)?;
    let mut h = Sha256::new();
    let mut buf = vec![0u8; 256 * 1024];
    loop {
        let n = f.read(&mut buf)?;
        if n == 0 {
            break;
        }
        h.update(&buf[..n]);
    }
    Ok(h.finalize().into())
}

/// The folder the fetch process may write: on Windows the person's `LocalLow`, which Windows
/// itself marks for low-integrity programs.
pub fn default_home() -> Option<PathBuf> {
    #[cfg(windows)]
    {
        let profile = std::env::var_os("USERPROFILE")?;
        Some(
            PathBuf::from(profile)
                .join("AppData")
                .join("LocalLow")
                .join("PCTwin")
                .join("fetch"),
        )
    }
    #[cfg(not(windows))]
    {
        let home = std::env::var_os("HOME")?;
        Some(
            PathBuf::from(home)
                .join(".cache")
                .join("pctwin")
                .join("fetch"),
        )
    }
}

impl Fetcher {
    /// The fetch program installed beside this one, with its own folder `home`.
    pub fn beside_this(home: PathBuf) -> Result<Fetcher, ProcessError> {
        let me = std::env::current_exe().map_err(|_| ProcessError::Missing)?;
        let program = me.parent().ok_or(ProcessError::Missing)?.join(PROGRAM);
        Fetcher::new(program, home)
    }

    pub fn new(program: PathBuf, home: PathBuf) -> Result<Fetcher, ProcessError> {
        if !std::fs::symlink_metadata(&program).is_ok_and(|m| m.is_file()) {
            return Err(ProcessError::Missing);
        }
        Ok(Fetcher { program, home })
    }

    /// Where downloads are made: files here only.
    pub fn staging(&self) -> PathBuf {
        self.home.join("files")
    }

    /// The program to start: on Windows a fresh copy of it, marked low integrity by Windows' own
    /// `icacls`, checked to be the same bytes as the installed one after marking.
    fn prepared(&self) -> Result<PathBuf, ProcessError> {
        let prep = |e: std::io::Error| ProcessError::Prepare(e.to_string());
        std::fs::create_dir_all(self.staging()).map_err(prep)?;
        std::fs::create_dir_all(self.home.join("tmp")).map_err(prep)?;
        #[cfg(windows)]
        {
            let bin = self.home.join("bin");
            std::fs::create_dir_all(&bin).map_err(prep)?;
            let copy = bin.join(PROGRAM);
            let _ = std::fs::remove_file(&copy);
            std::fs::copy(&self.program, &copy).map_err(prep)?;
            let system = std::env::var_os("SystemRoot")
                .map(PathBuf::from)
                .ok_or_else(|| ProcessError::Prepare("Windows' folder is not known".into()))?;
            let icacls = system.join("System32").join("icacls.exe");
            let out = pctwin_types::process::quiet(&icacls)
                .arg(&copy)
                .args(["/setintegritylevel", "low"])
                .env_clear()
                .env("SystemRoot", &system)
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .status()
                .map_err(prep)?;
            if !out.success() {
                return Err(ProcessError::Prepare(
                    "Windows would not mark the download program low".into(),
                ));
            }
            if sha(&copy).map_err(prep)? != sha(&self.program).map_err(prep)? {
                return Err(ProcessError::Prepare(
                    "the download program changed while it was prepared".into(),
                ));
            }
            Ok(copy)
        }
        #[cfg(not(windows))]
        {
            Ok(self.program.clone())
        }
    }

    fn command(&self, program: &Path, args: &[&std::ffi::OsStr]) -> Command {
        #[cfg(target_os = "macos")]
        let mut c = {
            let mut c = pctwin_types::process::quiet("/usr/bin/sandbox-exec");
            c.arg("-p").arg(mac_profile(&self.home)).arg(program);
            c
        };
        #[cfg(not(target_os = "macos"))]
        let mut c = pctwin_types::process::quiet(program);
        c.args(args)
            .env_clear()
            .current_dir(&self.home)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null());
        let tmp = self.home.join("tmp");
        c.env("TEMP", &tmp).env("TMP", &tmp).env("TMPDIR", &tmp);
        #[cfg(windows)]
        if let Some(root) = std::env::var_os("SystemRoot") {
            c.env("SystemRoot", root);
        }
        c
    }

    fn run_raw(
        &self,
        args: &[&std::ffi::OsStr],
        input: &[u8],
        time: Duration,
    ) -> Result<Vec<u8>, ProcessError> {
        let program = self.prepared()?;
        let mut child = self
            .command(&program, args)
            .spawn()
            .map_err(|e| ProcessError::Start(e.to_string()))?;
        if let Some(mut stdin) = child.stdin.take() {
            let _ = stdin.write_all(input);
        }
        let mut stdout = child.stdout.take().ok_or(ProcessError::NoAnswer)?;
        let reader = std::thread::spawn(move || {
            let mut out = Vec::new();
            let _ = (&mut stdout).take(MAX_ANSWER).read_to_end(&mut out);
            out
        });
        let start = Instant::now();
        loop {
            match child.try_wait() {
                Ok(Some(_)) => break,
                Ok(None) if start.elapsed() >= time => {
                    let _ = child.kill();
                    let _ = child.wait();
                    return Err(ProcessError::TimedOut);
                }
                Ok(None) => std::thread::sleep(Duration::from_millis(50)),
                Err(e) => return Err(ProcessError::Start(e.to_string())),
            }
        }
        reader.join().map_err(|_| ProcessError::NoAnswer)
    }

    /// One download, in the locked-down process, given at most `time`. The file must be in
    /// [`Fetcher::staging`].
    pub fn fetch(&self, req: &Request, time: Duration) -> Result<Outcome, ProcessError> {
        if req.file.parent() != Some(self.staging().as_path()) || req.file.file_name().is_none() {
            return Err(ProcessError::OutsideStaging);
        }
        let input = serde_json::to_vec(req).map_err(|_| ProcessError::NoAnswer)?;
        let staging = self.staging();
        let out = self.run_raw(&["fetch".as_ref(), staging.as_os_str()], &input, time)?;
        let outcome: Outcome = serde_json::from_slice(&out).map_err(|_| ProcessError::NoAnswer)?;
        if let Outcome::Done { from, .. } = &outcome {
            mark_here(&req.file, from);
        }
        Ok(outcome)
    }

    /// Asks the fetch process to try writing `outside` (a file in a folder of the person's):
    /// true when it could. Proves the lock-down on the laptop it runs on.
    pub fn could_write(&self, outside: &Path) -> Result<bool, ProcessError> {
        let out = self.run_raw(
            &["probe".as_ref(), outside.as_os_str()],
            b"",
            Duration::from_secs(60),
        )?;
        Ok(out.starts_with(b"wrote"))
    }
}

/// The Mac's quarantine flag and Linux's origin, set by the app on the file it reads back.
fn mark_here(file: &Path, from: &str) {
    #[cfg(target_os = "macos")]
    {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |d| d.as_secs());
        let _ = pctwin_types::process::quiet("/usr/bin/xattr")
            .args([
                "-w",
                "com.apple.quarantine",
                &format!("0081;{now:x};PCTwin;"),
            ])
            .arg(file)
            .env_clear()
            .status();
        let _ = from;
    }
    #[cfg(all(unix, not(target_os = "macos")))]
    {
        let _ = pctwin_types::process::quiet("/usr/bin/setfattr")
            .args(["-n", "user.xdg.origin.url", "-v", from])
            .arg(file)
            .env_clear()
            .status();
    }
    #[cfg(windows)]
    {
        // Written by the fetch process itself (it may write its own file's streams).
        let _ = (file, from);
    }
}

/// The Mac sandbox profile: the network, reading the system, and writing only its own folder.
#[cfg(any(target_os = "macos", test))]
pub fn mac_profile(home: &Path) -> String {
    let home = home.to_string_lossy().replace(['"', '\\'], "");
    format!(
        "(version 1)(deny default)(allow process-exec)(allow process-fork)(allow sysctl-read)(allow mach-lookup)(allow network-outbound)(allow system-socket)(allow file-read*)(deny file-read* (subpath \"/Users\"))(allow file-read* file-write* (subpath \"{home}\"))"
    )
}

/// The fetch program's own work, run in the fetch process (`pctwin-fetch fetch <staging>`): one
/// request from standard input, one answer on standard output.
pub fn serve(staging: &Path, input: &[u8]) -> Outcome {
    if input.len() > MAX_REQUEST {
        return Outcome::Bad {
            why: "the request is too large".into(),
        };
    }
    let req: Request = match serde_json::from_slice(input) {
        Ok(r) => r,
        Err(e) => return Outcome::Bad { why: e.to_string() },
    };
    if req.file.parent() != Some(staging) || req.file.file_name().is_none() {
        return Outcome::Bad {
            why: "the file is not in the download folder".into(),
        };
    }
    let rules = match super::download::Rules::public() {
        Ok(r) => r,
        Err(why) => return Outcome::Bad { why },
    };
    let rt = match tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
    {
        Ok(rt) => rt,
        Err(e) => return Outcome::Bad { why: e.to_string() },
    };
    let outcome = rt.block_on(super::download::download(
        &req,
        &rules,
        &super::network_cost,
    ));
    if let Outcome::Done { from, .. } = &outcome {
        let _ = super::marks::mark(&req.file, from);
    }
    outcome
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_download_outside_the_staging_folder_is_never_asked_for() {
        let dir = tempfile::tempdir().unwrap();
        let program = dir.path().join(PROGRAM);
        std::fs::write(&program, b"x").unwrap();
        let f = Fetcher::new(program, dir.path().join("home")).unwrap();
        let req = Request {
            url: "https://example.com/a".into(),
            also_hosts: vec![],
            max_bytes: 1,
            expected: None,
            file: dir.path().join("elsewhere.bin"),
            allow_metered: false,
        };
        assert!(matches!(
            f.fetch(&req, Duration::from_secs(1)),
            Err(ProcessError::OutsideStaging)
        ));
        // And the program itself refuses one.
        let out = serve(&f.staging(), &serde_json::to_vec(&req).unwrap());
        assert!(matches!(out, Outcome::Bad { .. }));
        assert!(matches!(
            serve(&f.staging(), b"not json"),
            Outcome::Bad { .. }
        ));
        // A well-formed request padded past the bound is refused before it is read.
        let fine = Request {
            url: "https://127.0.0.1/a".into(),
            file: f.staging().join("a.bin"),
            ..req.clone()
        };
        let mut padded = serde_json::to_vec(&fine).unwrap();
        assert!(matches!(
            serve(&f.staging(), &padded),
            Outcome::Refused { .. }
        ));
        padded.extend(std::iter::repeat_n(b' ', MAX_REQUEST));
        assert!(matches!(serve(&f.staging(), &padded), Outcome::Bad { .. }));
        assert!(Fetcher::new(dir.path().join("missing"), dir.path().into()).is_err());
    }

    #[test]
    fn the_mac_profile_writes_only_its_own_folder() {
        let p = mac_profile(Path::new("/Users/ada/.cache/pctwin/fetch"));
        assert!(p.starts_with("(version 1)(deny default)"));
        assert!(p.contains("(deny file-read* (subpath \"/Users\"))"));
        assert!(p.ends_with(
            "(allow file-read* file-write* (subpath \"/Users/ada/.cache/pctwin/fetch\"))"
        ));
        // A quote in the folder cannot end the rule early.
        assert!(
            !mac_profile(Path::new("/x\")(allow file-write* (subpath \"/"))
                .contains("\")(allow file-write* (subpath \"/\"")
        );
    }
}
