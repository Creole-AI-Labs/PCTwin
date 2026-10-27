//! Windows' own checks, asked of Windows itself with no tool or script in between (second
//! review, finding B3: PowerShell loads modules from the person's own folders first, even with
//! `-NoProfile`, and `SystemRoot` and `ProgramFiles` are values any program can set, so the
//! answer could come from code the person, or anything running as them, put there):
//!
//! - Whether Windows trusts the signature: `WinVerifyTrust` on the file PCTwin holds open, asked
//!   through the [`TrustOracle`] the caller gives (the administrator helper gives the real one;
//!   this crate never links the crate that makes system calls, third review, finding 8). For a
//!   Windows program Windows reads the bytes through the held handle, so the bytes trusted are the
//!   bytes hashed (finding B2). For a Windows Installer package it does not: its checker opens the
//!   file again by name (third review, finding 3), so a package's answer counts only for the
//!   helper's own copy ([`crate::check::check_staged`]).
//! - Microsoft Defender's state: from Windows' management service (the `MSFT_MpComputerStatus`
//!   class Defender publishes, the source `Get-MpComputerStatus` itself reads).
//! - The scan: Defender's own `MpCmdRun.exe`, found where Windows records Defender, started with
//!   only the system's own environment values, in Windows' own `System32` folder (never the
//!   caller's folder; third review, finding 7). The file is named as one argument.
//!
//! Defender's scan has no published table of exit codes. Its meaning is pinned by running it on
//! this laptop (26 October 2026, Defender platform 4.18.26090.9, engine 1.1.26090.9):
//! - a clean file: exit 0, and the line `Scanning <path> found no threats.`;
//! - the EICAR test file: exit 2, `Scanning <path> found 1 threats.` and a `Threat : ...` line;
//! - a missing file: exit 2 too, with `CmdTool: Failed with hr = 0x80070002`.
//!
//! So the exit code alone cannot tell a finding from a failure; both the code and the matching
//! line are required, and anything else is "not fully checked". With `-DisableRemediation` the
//! scan writes no detection event (1116) to Defender's log, so the log cannot be the source.
//!
//! What the scan does not prove: Defender reads the file by its name. The answer counts for an
//! install only when the file is the administrator helper's own copy in its administrators-only
//! folder, held so nobody can change it (the helper produces that verdict itself; see
//! `pctwin-helper`'s broker).

use std::ffi::OsString;
use std::path::Path;
use std::time::Duration;

use serde::Deserialize;

use crate::facts::{Coverage, Malware, Signature, WindowsSigner};
use crate::oracle::{NOT_ASKED, TrustOracle};
use crate::run::{Call, RunError};
use crate::signer::signer_from_der;

/// How old Defender's threat knowledge may be, in days, before a scan is not enough (T21-033).
pub const MAX_SIGNATURE_AGE_DAYS: u64 = 7;

/// Windows: the file has no signature inside it (it may be signed through a catalog only).
pub const TRUST_E_NOSIGNATURE: i32 = 0x800B_0100_u32 as i32;
/// Windows: the file's bytes are not the bytes signed.
pub const TRUST_E_BAD_DIGEST: i32 = 0x8009_6010_u32 as i32;
/// Windows: it does not know this kind of file.
pub const TRUST_E_SUBJECT_FORM_UNKNOWN: i32 = 0x800B_0003_u32 as i32;

/// The call that scans one file with `scanner` (Defender's `MpCmdRun.exe`), changing nothing,
/// with the environment `env` and nothing else, started in `system32`.
pub fn scan_call(
    scanner: &Path,
    path: &Path,
    env: Vec<(OsString, OsString)>,
    timeout: Duration,
    system32: &Path,
) -> Call {
    Call {
        program: scanner.to_path_buf(),
        args: vec![
            "-Scan".into(),
            "-ScanType".into(),
            "3".into(),
            "-File".into(),
            path.as_os_str().to_owned(),
            "-DisableRemediation".into(),
        ],
        env,
        timeout,
        stdin: None,
        cwd: Some(system32.to_path_buf()),
    }
}

/// What Windows said about a signature.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WinTrust {
    pub signature: Signature,
    /// Whether the signature is inside the file. Windows is asked about the file alone, never a
    /// catalog, so a trusted answer is always about the maker's own signature inside it.
    pub embedded: bool,
    /// The signer Windows read, when it trusts the signature.
    pub signer: Option<Result<WindowsSigner, String>>,
}

/// Reads Windows' answer (`code`, 0 when trusted) and, when trusted, the signing certificate's
/// bytes as Windows read them.
pub fn trust_from(code: i32, signer: Option<&[u8]>) -> WinTrust {
    match code {
        0 => WinTrust {
            signature: Signature::Trusted,
            embedded: true,
            signer: Some(
                signer
                    .ok_or_else(|| "Windows trusted the file but named no signer".to_owned())
                    .and_then(signer_from_der),
            ),
        },
        TRUST_E_NOSIGNATURE => WinTrust {
            signature: Signature::Missing,
            embedded: false,
            signer: None,
        },
        TRUST_E_BAD_DIGEST => WinTrust {
            signature: Signature::Broken {
                why: "Windows says the file was changed after it was signed".into(),
            },
            embedded: true,
            signer: None,
        },
        TRUST_E_SUBJECT_FORM_UNKNOWN => WinTrust {
            signature: Signature::NotChecked {
                why: "Windows does not check this kind of file".into(),
            },
            embedded: false,
            signer: None,
        },
        other => WinTrust {
            signature: Signature::NotTrusted {
                why: format!(
                    "Windows does not trust this signature (code {:#010x})",
                    other as u32
                ),
            },
            embedded: true,
            signer: None,
        },
    }
}

/// Asks Windows itself, through `oracle`, about the signature inside `file`, held open by the
/// caller at `path`. With no answer from Windows (a stand-in oracle), the signature is not checked.
pub fn check_signature(oracle: &dyn TrustOracle, file: &std::fs::File, path: &Path) -> WinTrust {
    let (code, signer) = oracle.verify_file_trust(file, path);
    if code == NOT_ASKED {
        return WinTrust {
            signature: Signature::NotChecked {
                why: "Windows' own signature check was not available".into(),
            },
            embedded: false,
            signer: None,
        };
    }
    trust_from(code, signer.as_deref())
}

/// Defender's state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DefenderState {
    /// On, with its versions and how old its threat knowledge is.
    Ready {
        description: String,
        signature_age_days: u64,
    },
    /// Off, replaced by another product, or not answering.
    Unavailable(String),
}

/// Defender's state as Windows' management service gives it (`MSFT_MpComputerStatus`).
#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize)]
#[serde(rename = "MSFT_MpComputerStatus", rename_all = "PascalCase")]
pub struct DefenderStatus {
    #[serde(rename = "AMServiceEnabled")]
    pub am_service_enabled: Option<bool>,
    pub antivirus_enabled: Option<bool>,
    #[serde(rename = "AMRunningMode")]
    pub am_running_mode: Option<String>,
    pub antivirus_signature_age: Option<u64>,
    #[serde(rename = "AMEngineVersion")]
    pub am_engine_version: Option<String>,
    pub antivirus_signature_version: Option<String>,
}

/// Reads Defender's state.
pub fn judge_status(status: Result<DefenderStatus, String>) -> DefenderState {
    let j = match status {
        Ok(j) => j,
        Err(why) => {
            return DefenderState::Unavailable(format!("Microsoft Defender did not answer: {why}"));
        }
    };
    if j.am_service_enabled != Some(true) || j.antivirus_enabled != Some(true) {
        return DefenderState::Unavailable("Microsoft Defender's scanning is turned off".into());
    }
    let Some(age) = j.antivirus_signature_age else {
        return DefenderState::Unavailable(
            "Microsoft Defender's threat knowledge has no date".into(),
        );
    };
    DefenderState::Ready {
        description: format!(
            "Microsoft Defender (engine {}, threat knowledge {}, {} mode)",
            j.am_engine_version.as_deref().unwrap_or("unknown"),
            j.antivirus_signature_version
                .as_deref()
                .unwrap_or("unknown"),
            j.am_running_mode.as_deref().unwrap_or("unknown"),
        ),
        signature_age_days: age,
    }
}

/// Asks Windows' management service for Defender's state, on a thread of its own so the
/// caller's own COM setup never matters, trying a few times (the service can be busy).
#[cfg(windows)]
pub fn defender_status() -> Result<DefenderStatus, String> {
    let mut last = String::new();
    for wait in [0u64, 200, 1000] {
        std::thread::sleep(std::time::Duration::from_millis(wait));
        match std::thread::spawn(ask_defender_status).join() {
            Ok(Ok(s)) => return Ok(s),
            Ok(Err(e)) => last = e,
            Err(_) => last = "the question stopped part way".into(),
        }
    }
    Err(last)
}

#[cfg(windows)]
fn ask_defender_status() -> Result<DefenderStatus, String> {
    let wmi = wmi::WMIConnection::with_namespace_path("ROOT\\Microsoft\\Windows\\Defender")
        .map_err(|e| e.to_string())?;
    let mut all: Vec<DefenderStatus> = wmi.query().map_err(|e| e.to_string())?;
    match (all.pop(), all.is_empty()) {
        (Some(one), true) => Ok(one),
        _ => Err("Microsoft Defender's state is not one answer".into()),
    }
}

fn first_line(s: &str) -> &str {
    s.lines()
        .map(str::trim)
        .find(|l| !l.is_empty())
        .unwrap_or("no message")
}

/// Reads a single-file scan's answer, by the meanings pinned above. `path` is the file scanned:
/// the answer must name it.
pub fn parse_scan(
    path: &Path,
    state: &DefenderState,
    ran: &Result<crate::run::Ran, RunError>,
) -> Malware {
    let (description, age) = match state {
        DefenderState::Ready {
            description,
            signature_age_days,
        } => (description, *signature_age_days),
        DefenderState::Unavailable(why) => return Malware::Unavailable { why: why.clone() },
    };
    if age > MAX_SIGNATURE_AGE_DAYS {
        return Malware::NotFullyChecked {
            why: format!("Microsoft Defender's threat knowledge is {age} days old"),
        };
    }
    let ran = match ran {
        Ok(r) => r,
        Err(RunError::NotStarted(e)) => {
            return Malware::Unavailable {
                why: format!("Microsoft Defender's scanner could not be started ({e})"),
            };
        }
        Err(e) => {
            return Malware::NotFullyChecked {
                why: format!("the scan did not finish: {e}"),
            };
        }
    };
    let shown = path.display().to_string();
    let verdict_line = ran.stdout.lines().map(str::trim).find_map(|l| {
        let rest = l.strip_prefix("Scanning ")?;
        let (named, tail) = rest.rsplit_once(" found ")?;
        named.eq_ignore_ascii_case(&shown).then_some(tail)
    });
    match (ran.code, verdict_line) {
        (Some(0), Some("no threats.")) => Malware::NoDetection {
            coverage: Coverage {
                scanner: description.clone(),
                later_downloads_not_checked: true,
            },
        },
        (Some(2), Some(tail)) if found_some(tail) => {
            let name = ran
                .stdout
                .lines()
                .find_map(|l| {
                    let (k, v) = l.split_once(':')?;
                    (k.trim() == "Threat").then(|| v.trim().to_owned())
                })
                .filter(|n| !n.is_empty())
                .unwrap_or_else(|| "an unnamed threat".into());
            Malware::Detected { name }
        }
        (code, _) => Malware::NotFullyChecked {
            why: format!(
                "the scan did not finish (code {}: {})",
                code.map_or_else(|| "none".into(), |c| c.to_string()),
                first_line(&ran.stdout)
            ),
        },
    }
}

/// "1 threats." and up; never "0 threats.".
fn found_some(tail: &str) -> bool {
    tail.strip_suffix(" threats.")
        .and_then(|n| n.parse::<u64>().ok())
        .is_some_and(|n| n > 0)
}

/// Scans a file with Defender: its state first, then its own scanner on the file, found, set up
/// and started where `oracle` says Windows keeps them.
#[cfg(windows)]
pub fn scan(
    runner: &dyn crate::run::Runner,
    oracle: &dyn TrustOracle,
    path: &Path,
    timeout: Duration,
) -> Malware {
    let (Some(scanner), Some(env), Some(system32)) = (
        oracle.defender_scanner(),
        oracle.minimal_env(),
        oracle.system_dir(),
    ) else {
        return Malware::Unavailable {
            why: "Microsoft Defender's scanner could not be found where Windows keeps it".into(),
        };
    };
    let state = judge_status(defender_status());
    if let DefenderState::Unavailable(why) = state {
        return Malware::Unavailable { why };
    }
    let ran = runner.run(&scan_call(&scanner, path, env, timeout, &system32));
    parse_scan(path, &state, &ran)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::run::Ran;

    fn ran(code: i32, stdout: &str) -> Result<Ran, RunError> {
        Ok(Ran {
            code: Some(code),
            stdout: stdout.into(),
            stderr: String::new(),
        })
    }

    fn ready(age: u64) -> DefenderState {
        DefenderState::Ready {
            description: "Microsoft Defender".into(),
            signature_age_days: age,
        }
    }

    const P: &str = r"C:\Users\Ann\Downloads\setup.exe";

    // Captured from this laptop, 26 October 2026 (paths changed).
    const CLEAN: &str = "Scan starting...\r\nScan finished.\r\nScanning C:\\Users\\Ann\\Downloads\\setup.exe found no threats.\r\n";
    const EICAR: &str = "Scan starting...\r\nScan finished.\r\nScanning C:\\Users\\Ann\\Downloads\\setup.exe found 1 threats.\r\n\r\n<===========================LIST OF DETECTED THREATS==========================>\r\n----------------------------- Threat information ------------------------------\r\nThreat                  : Virus:DOS/EICAR_Test_File\r\nResources               : 1 total\r\n    file                : C:\\Users\\Ann\\Downloads\\setup.exe\r\n-------------------------------------------------------------------------------\r\n";
    const MISSING: &str = "CmdTool: Failed with hr = 0x80070002. Check C:\\Users\\Ann\\AppData\\Local\\Temp\\MpCmdRun.log for more information\r\n";

    #[test]
    fn pinned_scan_meanings() {
        let p = Path::new(P);
        assert!(matches!(
            parse_scan(p, &ready(0), &ran(0, CLEAN)),
            Malware::NoDetection { coverage } if coverage.later_downloads_not_checked
        ));
        assert_eq!(
            parse_scan(p, &ready(0), &ran(2, EICAR)),
            Malware::Detected {
                name: "Virus:DOS/EICAR_Test_File".into()
            }
        );
        // Exit 2 also means "failed": without the finding line it is not a detection, and never
        // clean.
        assert!(matches!(
            parse_scan(p, &ready(0), &ran(2, MISSING)),
            Malware::NotFullyChecked { why } if why.contains("code 2") && why.contains("0x80070002")
        ));
    }

    #[test]
    fn a_clean_line_needs_exit_zero_and_the_right_file() {
        let p = Path::new(P);
        assert!(matches!(
            parse_scan(p, &ready(0), &ran(2, CLEAN)),
            Malware::NotFullyChecked { .. }
        ));
        assert!(matches!(
            parse_scan(p, &ready(0), &ran(1, CLEAN)),
            Malware::NotFullyChecked { .. }
        ));
        let other = CLEAN.replace("setup.exe", "other.exe");
        assert!(matches!(
            parse_scan(p, &ready(0), &ran(0, &other)),
            Malware::NotFullyChecked { .. }
        ));
        // The name is compared without regard to case, as Windows does.
        let upper = CLEAN.replace("setup.exe", "SETUP.EXE");
        assert!(matches!(
            parse_scan(p, &ready(0), &ran(0, &upper)),
            Malware::NoDetection { .. }
        ));
        assert!(matches!(
            parse_scan(p, &ready(0), &ran(0, "")),
            Malware::NotFullyChecked { .. }
        ));
        // A finding needs exit 2.
        assert!(matches!(
            parse_scan(p, &ready(0), &ran(0, EICAR)),
            Malware::NotFullyChecked { .. }
        ));
        let zero = EICAR.replace("found 1 threats", "found 0 threats");
        assert!(matches!(
            parse_scan(p, &ready(0), &ran(2, &zero)),
            Malware::NotFullyChecked { .. }
        ));
        let unnamed = EICAR.replace("Virus:DOS/EICAR_Test_File", "");
        assert_eq!(
            parse_scan(p, &ready(0), &ran(2, &unnamed)),
            Malware::Detected {
                name: "an unnamed threat".into()
            }
        );
    }

    #[test]
    fn stale_knowledge_or_no_scanner_is_never_clean() {
        let p = Path::new(P);
        // T21-033.
        assert!(matches!(
            parse_scan(p, &ready(MAX_SIGNATURE_AGE_DAYS + 1), &ran(0, CLEAN)),
            Malware::NotFullyChecked { why } if why.contains("days old")
        ));
        assert!(matches!(
            parse_scan(p, &ready(MAX_SIGNATURE_AGE_DAYS), &ran(0, CLEAN)),
            Malware::NoDetection { .. }
        ));
        assert!(matches!(
            parse_scan(p, &DefenderState::Unavailable("off".into()), &ran(0, CLEAN)),
            Malware::Unavailable { .. }
        ));
        // T21-032: a timeout.
        assert!(matches!(
            parse_scan(p, &ready(0), &Err(RunError::TimedOut)),
            Malware::NotFullyChecked { .. }
        ));
        assert!(matches!(
            parse_scan(p, &ready(0), &Err(RunError::NotStarted("gone".into()))),
            Malware::Unavailable { .. }
        ));
    }

    // As this laptop's Defender reported itself, 26 October 2026.
    fn status() -> DefenderStatus {
        DefenderStatus {
            am_service_enabled: Some(true),
            antivirus_enabled: Some(true),
            am_running_mode: Some("Normal".into()),
            antivirus_signature_age: Some(0),
            am_engine_version: Some("1.1.26090.9".into()),
            antivirus_signature_version: Some("1.461.209.0".into()),
        }
    }

    #[test]
    fn defender_state() {
        assert_eq!(
            judge_status(Ok(status())),
            DefenderState::Ready {
                description: "Microsoft Defender (engine 1.1.26090.9, threat knowledge 1.461.209.0, Normal mode)".into(),
                signature_age_days: 0
            }
        );
        for change in [
            |s: &mut DefenderStatus| s.antivirus_enabled = Some(false),
            |s: &mut DefenderStatus| s.antivirus_enabled = None,
            |s: &mut DefenderStatus| s.am_service_enabled = Some(false),
            |s: &mut DefenderStatus| s.am_service_enabled = None,
            |s: &mut DefenderStatus| s.antivirus_signature_age = None,
        ] {
            let mut s = status();
            change(&mut s);
            assert!(
                matches!(judge_status(Ok(s.clone())), DefenderState::Unavailable(_)),
                "{s:?}"
            );
        }
        assert!(matches!(
            judge_status(Err("no service".into())),
            DefenderState::Unavailable(_)
        ));
    }

    /// The state is read from Windows' management service, for real (second review, B3).
    #[cfg(windows)]
    #[test]
    fn defenders_state_comes_from_windows_itself() {
        let s = defender_status().expect("Defender's state on this laptop");
        assert!(s.am_service_enabled.is_some(), "{s:?}");
        assert!(s.antivirus_signature_age.is_some(), "{s:?}");
    }

    #[test]
    fn signature_answers() {
        let t = trust_from(0, None);
        assert_eq!(t.signature, Signature::Trusted);
        assert!(t.embedded);
        assert!(matches!(t.signer, Some(Err(_))));
        let t = trust_from(0, Some(b"not a certificate"));
        assert!(matches!(t.signer, Some(Err(_))));
        let t = trust_from(TRUST_E_NOSIGNATURE, None);
        assert_eq!(t.signature, Signature::Missing);
        assert!(!t.embedded);
        assert_eq!(t.signer, None);
        let t = trust_from(TRUST_E_BAD_DIGEST, Some(b"x"));
        assert!(matches!(t.signature, Signature::Broken { .. }));
        assert_eq!(t.signer, None);
        let t = trust_from(TRUST_E_SUBJECT_FORM_UNKNOWN, None);
        assert!(matches!(t.signature, Signature::NotChecked { .. }));
        assert!(!t.embedded);
        // An untrusted root, a revoked or expired certificate, an explicit distrust.
        for code in [
            0x800B_0109_u32,
            0x800B_010C,
            0x800B_0101,
            0x800B_0111,
            0x8009_2010,
        ] {
            let t = trust_from(code as i32, Some(b"x"));
            assert!(
                matches!(&t.signature, Signature::NotTrusted { why } if why.contains(&format!("{code:#010x}"))),
                "{t:?}"
            );
            assert_eq!(t.signer, None);
        }
    }

    /// Second review, finding B3: no answer comes from PowerShell or an environment value any
    /// more. (The words are put together here so this test's own text does not hold them.)
    #[test]
    fn no_script_host_or_environment_value_is_used() {
        let words = [
            ["power", "shell.exe"].concat(),
            ["-encoded", "command"].concat(),
            ["get-authenticode", "signature"].concat(),
            ["env::", "var"].concat(),
        ];
        for src in [
            include_str!("windows.rs"),
            include_str!("native.rs"),
            include_str!("check.rs"),
            include_str!("run.rs"),
        ] {
            let lower = src.to_ascii_lowercase();
            for word in &words {
                assert!(!lower.contains(word.as_str()), "{word}");
            }
        }
    }

    #[test]
    fn the_scan_is_one_fixed_call_with_only_the_given_environment() {
        let p = Path::new(r"C:\x\'; Remove-Item -Recurse C:\ ; '.exe");
        let scanner =
            Path::new(r"C:\ProgramData\Microsoft\Windows Defender\Platform\4\MpCmdRun.exe");
        let env = vec![(OsString::from("SystemRoot"), OsString::from(r"C:\Windows"))];
        let system32 = Path::new(r"C:\Windows\System32");
        let s = scan_call(scanner, p, env.clone(), Duration::from_secs(1), system32);
        assert_eq!(s.program, scanner);
        // Started in Windows' own folder, never the caller's (third review, finding 7).
        assert_eq!(s.cwd.as_deref(), Some(system32));
        assert_eq!(s.stdin, None);
        assert_eq!(s.args[4], p.as_os_str());
        assert_eq!(s.args.len(), 6);
        assert_eq!(s.args[5], "-DisableRemediation");
        assert_eq!(s.env, env);
    }
}
