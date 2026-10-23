//! Starting the helper with administrator rights: every answer is a typed outcome.
//!
//! CI has no person to click a prompt, so the refused and cancelled paths go through a stand-in
//! launcher. The real prompt is reached only where it cannot ask anyone: on a Windows runner
//! with UAC prompts switched off, and on a Linux runner with no polkit agent (gated by
//! PCTWIN_HELPER_OS_ADMIN_TESTS=1, set only in CI).

use pctwin_helper_os::{
    Elevation, Ended, Error, LaunchError, LaunchRequest, Launcher, Process, launch_elevated,
    launch_elevated_with,
};
use std::cell::{Cell, RefCell};
use std::path::{Path, PathBuf};

/// A launcher that records what it was asked and answers as told.
struct StandIn<F: Fn(&LaunchRequest) -> Result<Process, LaunchError>> {
    calls: Cell<u32>,
    seen: RefCell<Option<LaunchRequest>>,
    answer: F,
}

impl<F: Fn(&LaunchRequest) -> Result<Process, LaunchError>> StandIn<F> {
    fn new(answer: F) -> Self {
        StandIn {
            calls: Cell::new(0),
            seen: RefCell::new(None),
            answer,
        }
    }
}

impl<F: Fn(&LaunchRequest) -> Result<Process, LaunchError>> Launcher for StandIn<F> {
    fn start(&self, request: &LaunchRequest) -> Result<Process, LaunchError> {
        self.calls.set(self.calls.get() + 1);
        *self.seen.borrow_mut() = Some(request.clone());
        (self.answer)(request)
    }
}

/// A program that exists: this test program itself.
fn helper_path() -> PathBuf {
    std::env::current_exe().unwrap()
}

/// A path as the system resolves it, in the form a launch request keeps (no `\\?\` prefix).
fn real(p: &Path) -> PathBuf {
    let c = std::fs::canonicalize(p).unwrap();
    let text = c.to_string_lossy().into_owned();
    PathBuf::from(text.strip_prefix(r"\\?\").unwrap_or(&text))
}

#[test]
fn a_refused_prompt_is_a_typed_answer() {
    let l = StandIn::new(|_| Err(LaunchError::Cancelled));
    let r = launch_elevated_with(&l, &helper_path(), &["--pipe", "pctwin-helper-x"]).unwrap();
    assert!(matches!(r, Elevation::Refused), "{r:?}");
    assert_eq!(l.calls.get(), 1);
    let seen = l.seen.borrow().clone().unwrap();
    assert_eq!(seen.program(), real(&helper_path()));
    assert_eq!(seen.args(), ["--pipe", "pctwin-helper-x"]);
}

#[test]
fn a_failed_launch_is_an_error_not_a_refusal() {
    let l = StandIn::new(|_| {
        Err(LaunchError::Failed(Error::Os {
            call: "test",
            code: 5,
        }))
    });
    let r = launch_elevated_with(&l, &helper_path(), &[]);
    assert!(matches!(r, Err(Error::Os { code: 5, .. })), "{r:?}");
}

#[test]
fn only_full_local_paths_and_no_nul_reach_the_launcher() {
    let l = StandIn::new(|_| Err(LaunchError::Cancelled));
    let exe = helper_path();
    let folder = exe.parent().unwrap();
    let exe_name = exe.file_name().unwrap();
    let folder_name = folder.file_name().unwrap();
    // Refused as text, before the system is asked anything.
    let mut text: Vec<(PathBuf, Vec<&str>)> = vec![
        (PathBuf::from("pctwin-helper"), vec![]),
        (PathBuf::from("bin/pctwin-helper"), vec![]),
        (exe.clone(), vec!["a\0b"]),
        (folder.join("..").join(folder_name).join(exe_name), vec![]),
        (folder.join(".").join(exe_name), vec![]),
    ];
    if cfg!(windows) {
        for p in [
            r"\\server\share\pctwin-helper.exe",
            r"\\localhost\C$\Windows\System32\cmd.exe",
            r"\\?\C:\pctwin-helper.exe",
            r"\\.\C:\pctwin-helper.exe",
            r"C:pctwin-helper.exe",
            r"\pctwin-helper.exe",
        ] {
            text.push((PathBuf::from(p), vec![]));
        }
        let mut stream = exe.clone().into_os_string();
        stream.push(":stream");
        text.push((PathBuf::from(stream), vec![]));
    }
    for (program, args) in &text {
        let r = launch_elevated_with(&l, program, args);
        assert!(
            matches!(r, Err(Error::BadInput(_))),
            "{program:?} {args:?}: {r:?}"
        );
    }
    // Refused once resolved: not there, or not a file.
    for program in [folder.join("no-such-helper-program"), folder.to_path_buf()] {
        let r = launch_elevated_with(&l, &program, &[]);
        assert!(r.is_err(), "{program:?}: {r:?}");
    }
    assert_eq!(l.calls.get(), 0, "nothing refused may reach the launcher");
}

#[test]
fn the_helper_beside_the_app_is_found_by_plain_name_only() {
    let exe = helper_path();
    let name = exe.file_name().unwrap().to_str().unwrap();
    let r = LaunchRequest::beside_current_exe(name, &["x"]).unwrap();
    assert_eq!(r.program(), real(&exe));
    for bad in ["", "..", ".", "../x", "a/b", "a\\b", "x:y", "/abs"] {
        assert!(
            LaunchRequest::beside_current_exe(bad, &[]).is_err(),
            "{bad:?}"
        );
    }
}

/// A link (a Windows junction, a Unix symbolic link) in the path is resolved: the request
/// names the real file.
#[test]
fn links_in_the_path_are_resolved() {
    let dir = tempfile::tempdir().unwrap();
    let target = dir.path().join("real");
    std::fs::create_dir(&target).unwrap();
    std::fs::write(target.join("helper.bin"), b"x").unwrap();
    let link = dir.path().join("link");
    #[cfg(windows)]
    {
        let ok = std::process::Command::new("cmd.exe")
            .args(["/c", "mklink", "/J"])
            .arg(&link)
            .arg(&target)
            .stdout(std::process::Stdio::null())
            .status()
            .unwrap()
            .success();
        assert!(ok, "mklink /J");
    }
    #[cfg(unix)]
    std::os::unix::fs::symlink(&target, &link).unwrap();
    let r = LaunchRequest::new(&link.join("helper.bin"), &[]).unwrap();
    assert_eq!(r.program(), real(&target.join("helper.bin")));
    assert!(!r.program().starts_with(&link));
}

#[cfg(unix)]
fn sh_exit(code: i32) -> std::process::Child {
    std::process::Command::new("/bin/sh")
        .arg("-c")
        .arg(format!("exit {code}"))
        .spawn()
        .unwrap()
}

#[cfg(unix)]
#[test]
fn pkexec_refusals_end_as_typed_answers() {
    for (code, want) in [
        (126, Ended::Refused),
        (127, Ended::NotAuthorized),
        (0, Ended::Exited(0)),
        (3, Ended::Exited(3)),
    ] {
        let l = StandIn::new(move |_| Ok(Process::from_pkexec(sh_exit(code))));
        match launch_elevated_with(&l, &helper_path(), &[]).unwrap() {
            Elevation::Started(mut p) => assert_eq!(p.wait().unwrap(), want, "exit {code}"),
            other => panic!("{other:?}"),
        }
    }
    // The same codes from a program not started through pkexec are only exit codes.
    let mut p = Process::from_child(sh_exit(126));
    assert_eq!(p.wait().unwrap(), Ended::Exited(126));
}

/// Windows: the helper is started with administrator rights only from inside Program Files,
/// where only administrators can change a file; this test program sits in the build folder,
/// which its account can write, so it is refused before any prompt is shown.
#[cfg(windows)]
#[test]
fn a_helper_its_own_account_could_replace_is_never_elevated() {
    let r = launch_elevated(&helper_path(), &[]);
    assert!(matches!(r, Err(Error::NotPrivate(_))), "{r:?}");
}

#[cfg(windows)]
#[test]
fn a_started_process_is_waited_for_through_its_handle() {
    let cmd = std::process::Command::new("cmd.exe")
        .args(["/c", "exit", "7"])
        .spawn()
        .unwrap();
    let id = cmd.id();
    let mut p = Process::from_child(cmd);
    assert_eq!(p.id(), id);
    assert_eq!(p.wait().unwrap(), Ended::Exited(7));
    assert_eq!(p.try_wait().unwrap(), Some(Ended::Exited(7)));

    let slow = std::process::Command::new("cmd.exe")
        .args(["/c", "ping -n 3 127.0.0.1 >nul"])
        .spawn()
        .unwrap();
    let mut p = Process::from_child(slow);
    assert_eq!(p.try_wait().unwrap(), None, "still running");
    assert_eq!(p.wait().unwrap(), Ended::Exited(0));
}

/// The command line the helper is started with comes back as exactly the arguments given: the
/// child here is this test program, which prints what Windows' parser gave it.
#[cfg(windows)]
#[test]
fn windows_arguments_come_back_exactly() {
    use std::os::windows::process::CommandExt;
    let cases: Vec<Vec<&str>> = vec![
        vec!["plain", "two words", ""],
        vec!["say \"hi\"", "C:\\path\\", "C:\\my path\\"],
        vec![
            "a\\\"b",
            "a\\b c",
            "trailing\\\\",
            "\"",
            "\\\"",
            "tab\there",
        ],
        vec!["&|<>^%PATH%!", "new\nline", "é ü 字"],
    ];
    for args in cases {
        let request = LaunchRequest::new(&helper_path(), &args).unwrap();
        let out = std::process::Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "echo_args_child",
                "--nocapture",
                "--test-threads=1",
            ])
            .raw_arg("ECHO-START")
            .raw_arg(request.windows_parameters())
            .env("PCTWIN_HELPER_OS_ECHO", "1")
            .output()
            .unwrap();
        let text = String::from_utf8(out.stdout).unwrap();
        let line = text
            .lines()
            .find_map(|l| l.split_once("ECHOED:").map(|(_, rest)| rest))
            .unwrap_or_else(|| panic!("no echo in {text}"));
        let got: Vec<String> = if line.is_empty() {
            vec![]
        } else {
            line.split(',').map(|a| unhex(&a[1..])).collect()
        };
        assert_eq!(got, args, "{}", request.windows_parameters());
    }
}

#[cfg(windows)]
fn unhex(s: &str) -> String {
    let bytes: Vec<u8> = (0..s.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&s[i..i + 2], 16).unwrap())
        .collect();
    String::from_utf8(bytes).unwrap()
}

/// Run as a child by `windows_arguments_come_back_exactly`; does nothing otherwise.
#[test]
fn echo_args_child() {
    if std::env::var_os("PCTWIN_HELPER_OS_ECHO").is_none() {
        return;
    }
    let args: Vec<String> = std::env::args()
        .skip_while(|a| a != "ECHO-START")
        .skip(1)
        .collect();
    let hex: Vec<String> = args
        .iter()
        .map(|a| {
            let hex: String = a.bytes().map(|b| format!("{b:02x}")).collect();
            format!("x{hex}")
        })
        .collect();
    println!("ECHOED:{}", hex.join(","));
}

#[cfg(any(windows, target_os = "linux"))]
fn admin_tests() -> bool {
    std::env::var_os("PCTWIN_HELPER_OS_ADMIN_TESTS").is_some_and(|v| v == "1")
}

/// The real ShellExecuteExW `runas` call, only where it cannot show a prompt: UAC prompts off
/// for administrators (as on GitHub's Windows runners). Anywhere else it says so and stops.
#[cfg(windows)]
#[test]
#[ignore = "needs PCTWIN_HELPER_OS_ADMIN_TESTS=1 and an administrator with UAC prompts off (CI)"]
fn real_runas_starts_and_reports_the_exit_code() {
    assert!(
        admin_tests(),
        "set PCTWIN_HELPER_OS_ADMIN_TESTS=1 (CI only)"
    );
    let value = |name: &str| {
        let out = std::process::Command::new("reg")
            .args([
                "query",
                r"HKLM\SOFTWARE\Microsoft\Windows\CurrentVersion\Policies\System",
                "/v",
                name,
            ])
            .output()
            .unwrap();
        String::from_utf8_lossy(&out.stdout)
            .split_whitespace()
            .last()
            .map(str::to_owned)
    };
    let silent = value("EnableLUA").as_deref() == Some("0x0")
        || value("ConsentPromptBehaviorAdmin").as_deref() == Some("0x0");
    if !silent {
        panic!("UAC would show a prompt here; this test runs only where it cannot (CI)");
    }
    // Only a program inside Program Files is elevated: a copy of cmd is put there (this runner's
    // account is an administrator) and removed after.
    let folder = PathBuf::from(r"C:\Program Files\PCTwin-ci-test");
    std::fs::create_dir_all(&folder).unwrap();
    let cmd = folder.join("cmd.exe");
    std::fs::copy(r"C:\Windows\System32\cmd.exe", &cmd).unwrap();
    let r = launch_elevated(&cmd, &["/c", "exit", "7"]);
    let ended = match r {
        Ok(Elevation::Started(mut p)) => {
            assert_ne!(p.id(), 0);
            p.wait()
        }
        other => panic!("{other:?}"),
    };
    let _ = std::fs::remove_dir_all(&folder);
    assert_eq!(ended.unwrap(), Ended::Exited(7));
}

/// A drive letter mapped to a network share (here this computer's own administrative share)
/// resolves to the network, so a program on it is refused.
#[cfg(windows)]
#[test]
#[ignore = "needs PCTWIN_HELPER_OS_ADMIN_TESTS=1 and an administrator to map a drive (CI)"]
fn a_program_on_a_mapped_network_drive_is_refused() {
    assert!(
        admin_tests(),
        "set PCTWIN_HELPER_OS_ADMIN_TESTS=1 (CI only)"
    );
    let free = ('M'..='Y')
        .rev()
        .find(|d| !Path::new(&format!("{d}:\\")).exists())
        .expect("a free drive letter");
    let drive = format!("{free}:");
    let mapped = std::process::Command::new("net")
        .args(["use", &drive, r"\\localhost\C$", "/persistent:no"])
        .output()
        .unwrap();
    assert!(mapped.status.success(), "net use: {mapped:?}");
    let cmd = format!(r"{drive}\Windows\System32\cmd.exe");
    let seen = Path::new(&cmd).exists();
    let r = LaunchRequest::new(Path::new(&cmd), &[]);
    let _ = std::process::Command::new("net")
        .args(["use", &drive, "/delete", "/y"])
        .output();
    assert!(seen, "the mapped drive did not show the file");
    assert!(
        matches!(r, Err(Error::BadInput(_))),
        "a program on a network drive was accepted: {r:?}"
    );
}

/// The real pkexec, on a runner with no polkit agent: it cannot ask anyone, so it must end as a
/// typed answer (or run /bin/true if the runner lets it), never hang or crash.
#[cfg(target_os = "linux")]
#[test]
#[ignore = "needs PCTWIN_HELPER_OS_ADMIN_TESTS=1 on a machine with no polkit agent (CI)"]
fn real_pkexec_without_an_agent_is_a_typed_answer() {
    assert!(
        admin_tests(),
        "set PCTWIN_HELPER_OS_ADMIN_TESTS=1 (CI only)"
    );
    match launch_elevated(Path::new("/bin/true"), &[]) {
        Err(Error::NotSupported(why)) => {
            assert!(
                !Path::new("/usr/bin/pkexec").exists(),
                "pkexec is here: {why}"
            );
            println!("pkexec is not installed here: {why}");
        }
        Err(e) => panic!("{e}"),
        Ok(Elevation::Refused) => panic!("pkexec never refuses before starting"),
        Ok(Elevation::Started(mut p)) => {
            let deadline = std::time::Instant::now() + std::time::Duration::from_secs(60);
            let ended = loop {
                if let Some(e) = p.try_wait().unwrap() {
                    break e;
                }
                assert!(std::time::Instant::now() < deadline, "pkexec hung");
                std::thread::sleep(std::time::Duration::from_millis(100));
            };
            println!("pkexec ended: {ended:?}");
            assert!(
                matches!(
                    ended,
                    Ended::NotAuthorized | Ended::Refused | Ended::Exited(0)
                ),
                "{ended:?}"
            );
        }
    }
}

#[cfg(target_os = "macos")]
#[test]
fn macos_has_no_prompt_and_run() {
    let r = launch_elevated(Path::new("/usr/bin/true"), &[]);
    assert!(matches!(r, Err(Error::NotSupported(_))), "{r:?}");
}
