//! Windows' own checks: whether Windows trusts the signature (WinVerifyTrust, reached through
//! PowerShell's `Get-AuthenticodeSignature`, so PCTwin needs no unsafe code of its own for it), and
//! Microsoft Defender's single-file scan.
//!
//! The file's path is never part of any command text: PowerShell reads it from an environment
//! value, and Defender gets it as one argument.
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

use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::time::Duration;

use base64::Engine as _;
use base64::engine::general_purpose::STANDARD;
use serde::Deserialize;

use crate::facts::{Coverage, Malware, Signature, WindowsSigner};
use crate::run::{Call, RunError, Runner};
use crate::signer::signer_from_der;

/// The environment value PowerShell reads the file's path from.
pub const PATH_VAR: &str = "PCTWIN_MAKER_FILE";

/// How old Defender's threat knowledge may be, in days, before a scan is not enough (T21-033).
pub const MAX_SIGNATURE_AGE_DAYS: u64 = 7;

const SIGNATURE_SCRIPT: &str = "$ErrorActionPreference = 'Stop'
$s = Get-AuthenticodeSignature -LiteralPath $env:PCTWIN_MAKER_FILE
$c = $s.SignerCertificate
$der = ''
if ($c) { $der = [Convert]::ToBase64String($c.RawData) }
[pscustomobject]@{ status = [string]$s.Status; kind = [string]$s.SignatureType; message = [string]$s.StatusMessage; signer = $der } | ConvertTo-Json -Compress";

const STATUS_SCRIPT: &str = "$ErrorActionPreference = 'Stop'
Get-MpComputerStatus | Select-Object AMServiceEnabled, AntivirusEnabled, AMRunningMode, AntivirusSignatureAge, AMEngineVersion, AntivirusSignatureVersion | ConvertTo-Json -Compress";

fn system_root() -> PathBuf {
    std::env::var_os("SystemRoot")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(r"C:\Windows"))
}

fn program_files() -> PathBuf {
    std::env::var_os("ProgramFiles")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(r"C:\Program Files"))
}

/// Windows PowerShell 5.1, which every Windows 10 and 11 has.
pub fn powershell() -> PathBuf {
    system_root().join(r"System32\WindowsPowerShell\v1.0\powershell.exe")
}

/// Defender's command-line tool.
pub fn mpcmdrun() -> PathBuf {
    program_files().join(r"Windows Defender\MpCmdRun.exe")
}

/// A fixed script, passed encoded so nothing in it is read by the command line.
fn powershell_call(script: &str, env: Vec<(OsString, OsString)>, timeout: Duration) -> Call {
    let utf16: Vec<u8> = script.encode_utf16().flat_map(u16::to_le_bytes).collect();
    Call {
        program: powershell(),
        args: [
            "-NoLogo",
            "-NoProfile",
            "-NonInteractive",
            "-EncodedCommand",
        ]
        .into_iter()
        .map(OsString::from)
        .chain([OsString::from(STANDARD.encode(utf16))])
        .collect(),
        env,
        timeout,
    }
}

/// The call that asks Windows whether it trusts the file's signature.
pub fn signature_call(path: &Path, timeout: Duration) -> Call {
    powershell_call(
        SIGNATURE_SCRIPT,
        vec![(PATH_VAR.into(), path.as_os_str().to_owned())],
        timeout,
    )
}

/// The call that asks for Defender's state.
pub fn status_call(timeout: Duration) -> Call {
    powershell_call(STATUS_SCRIPT, vec![], timeout)
}

/// The call that scans one file, changing nothing.
pub fn scan_call(path: &Path, timeout: Duration) -> Call {
    Call {
        program: mpcmdrun(),
        args: vec![
            "-Scan".into(),
            "-ScanType".into(),
            "3".into(),
            "-File".into(),
            path.as_os_str().to_owned(),
            "-DisableRemediation".into(),
        ],
        env: vec![],
        timeout,
    }
}

/// What Windows said about a signature.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WinTrust {
    pub signature: Signature,
    /// Whether the signature is inside the file (not through a Windows catalog).
    pub embedded: bool,
    /// The signer Windows read, when there is one.
    pub signer: Option<Result<WindowsSigner, String>>,
}

#[derive(Deserialize)]
struct SigJson {
    status: String,
    kind: String,
    message: String,
    signer: String,
}

/// Reads PowerShell's answer about a signature.
pub fn parse_signature(ran: &Result<crate::run::Ran, RunError>) -> WinTrust {
    let not_checked = |why: String| WinTrust {
        signature: Signature::NotChecked { why },
        embedded: false,
        signer: None,
    };
    let ran = match ran {
        Ok(r) => r,
        Err(e) => return not_checked(format!("Windows' signature check gave no answer: {e}")),
    };
    if ran.code != Some(0) {
        return not_checked(format!(
            "Windows' signature check failed: {}",
            first_line(&ran.stderr)
        ));
    }
    let Ok(j) = serde_json::from_str::<SigJson>(ran.stdout.trim()) else {
        return not_checked("Windows' signature check gave an answer PCTwin cannot read".into());
    };
    let signer = (!j.signer.is_empty()).then(|| {
        STANDARD
            .decode(j.signer.as_bytes())
            .map_err(|e| e.to_string())
            .and_then(|der| signer_from_der(&der))
    });
    let signature = match j.status.as_str() {
        "Valid" => Signature::Trusted,
        "NotSigned" => Signature::Missing,
        "HashMismatch" => Signature::Broken {
            why: "Windows says the file was changed after it was signed".into(),
        },
        "NotTrusted" | "UnknownError" | "Incompatible" => Signature::NotTrusted { why: j.message },
        other => Signature::NotChecked {
            why: format!("Windows could not check this file ({other})"),
        },
    };
    WinTrust {
        signature,
        embedded: j.kind == "Authenticode",
        signer,
    }
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

#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
struct StatusJson {
    #[serde(rename = "AMServiceEnabled")]
    am_service_enabled: Option<bool>,
    antivirus_enabled: Option<bool>,
    #[serde(rename = "AMRunningMode")]
    am_running_mode: Option<String>,
    antivirus_signature_age: Option<u64>,
    #[serde(rename = "AMEngineVersion")]
    am_engine_version: Option<String>,
    antivirus_signature_version: Option<String>,
}

/// Reads Defender's state.
pub fn parse_status(ran: &Result<crate::run::Ran, RunError>) -> DefenderState {
    let ran = match ran {
        Ok(r) if r.code == Some(0) => r,
        Ok(r) => {
            return DefenderState::Unavailable(format!(
                "Microsoft Defender did not answer: {}",
                first_line(&r.stderr)
            ));
        }
        Err(e) => {
            return DefenderState::Unavailable(format!("Microsoft Defender did not answer: {e}"));
        }
    };
    let Ok(j) = serde_json::from_str::<StatusJson>(ran.stdout.trim()) else {
        return DefenderState::Unavailable("Microsoft Defender's state cannot be read".into());
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

/// Asks Windows about a file's signature.
pub fn check_signature(runner: &dyn Runner, path: &Path, timeout: Duration) -> WinTrust {
    parse_signature(&runner.run(&signature_call(path, timeout)))
}

/// Scans a file with Defender.
pub fn scan(runner: &dyn Runner, path: &Path, timeout: Duration) -> Malware {
    let state = parse_status(&runner.run(&status_call(Duration::from_secs(60))));
    if let DefenderState::Unavailable(why) = state {
        return Malware::Unavailable { why };
    }
    let ran = runner.run(&scan_call(path, timeout));
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

    // Shape captured from this laptop's Get-MpComputerStatus, 26 October 2026.
    const STATUS: &str = r#"{"AMServiceEnabled":true,"AntivirusEnabled":true,"AMRunningMode":"Normal","AntivirusSignatureAge":0,"AMEngineVersion":"1.1.26090.9","AntivirusSignatureVersion":"1.461.209.0"}"#;

    #[test]
    fn defender_state() {
        assert_eq!(
            parse_status(&ran(0, STATUS)),
            DefenderState::Ready {
                description: "Microsoft Defender (engine 1.1.26090.9, threat knowledge 1.461.209.0, Normal mode)".into(),
                signature_age_days: 0
            }
        );
        let off = STATUS.replace(r#""AntivirusEnabled":true"#, r#""AntivirusEnabled":false"#);
        assert!(matches!(
            parse_status(&ran(0, &off)),
            DefenderState::Unavailable(_)
        ));
        let svc = STATUS.replace(r#""AMServiceEnabled":true"#, r#""AMServiceEnabled":false"#);
        assert!(matches!(
            parse_status(&ran(0, &svc)),
            DefenderState::Unavailable(_)
        ));
        let undated = STATUS.replace(r#""AntivirusSignatureAge":0,"#, "");
        assert!(matches!(
            parse_status(&ran(0, &undated)),
            DefenderState::Unavailable(_)
        ));
        assert!(matches!(
            parse_status(&ran(1, STATUS)),
            DefenderState::Unavailable(_)
        ));
        assert!(matches!(
            parse_status(&ran(0, "not json")),
            DefenderState::Unavailable(_)
        ));
        assert!(matches!(
            parse_status(&Err(RunError::TimedOut)),
            DefenderState::Unavailable(_)
        ));
    }

    fn sig_json(status: &str, kind: &str, signer: &str) -> String {
        format!(r#"{{"status":"{status}","kind":"{kind}","message":"msg","signer":"{signer}"}}"#)
    }

    #[test]
    fn signature_statuses() {
        let cases = [
            ("Valid", Signature::Trusted),
            ("NotSigned", Signature::Missing),
            (
                "HashMismatch",
                Signature::Broken {
                    why: "Windows says the file was changed after it was signed".into(),
                },
            ),
            ("NotTrusted", Signature::NotTrusted { why: "msg".into() }),
            ("UnknownError", Signature::NotTrusted { why: "msg".into() }),
            ("Incompatible", Signature::NotTrusted { why: "msg".into() }),
        ];
        for (status, want) in cases {
            let t = parse_signature(&ran(0, &sig_json(status, "Authenticode", "")));
            assert_eq!(t.signature, want, "{status}");
            assert!(t.embedded);
            assert_eq!(t.signer, None);
        }
        let t = parse_signature(&ran(0, &sig_json("NotSupportedFileFormat", "None", "")));
        assert!(matches!(t.signature, Signature::NotChecked { .. }));
        assert!(!t.embedded);
        let catalog = parse_signature(&ran(0, &sig_json("Valid", "Catalog", "")));
        assert!(!catalog.embedded);
        assert!(matches!(
            parse_signature(&ran(1, "")).signature,
            Signature::NotChecked { .. }
        ));
        assert!(matches!(
            parse_signature(&ran(0, "garbage")).signature,
            Signature::NotChecked { .. }
        ));
        assert!(matches!(
            parse_signature(&Err(RunError::TimedOut)).signature,
            Signature::NotChecked { .. }
        ));
        // A signer that is not base64, or not a certificate, is read as an error, not dropped.
        let bad = parse_signature(&ran(0, &sig_json("Valid", "Authenticode", "!!!")));
        assert!(matches!(bad.signer, Some(Err(_))));
        let bad = parse_signature(&ran(0, &sig_json("Valid", "Authenticode", "AAAA")));
        assert!(matches!(bad.signer, Some(Err(_))));
    }

    #[test]
    fn the_path_never_enters_the_command_text() {
        let p = Path::new(r"C:\x\'; Remove-Item -Recurse C:\ ; '.exe");
        let c = signature_call(p, Duration::from_secs(1));
        assert!(
            c.args
                .iter()
                .all(|a| !a.to_string_lossy().contains("Remove-Item"))
        );
        assert_eq!(
            c.env,
            vec![(OsString::from(PATH_VAR), p.as_os_str().to_owned())]
        );
        assert!(
            c.program
                .ends_with(r"WindowsPowerShell\v1.0\powershell.exe")
        );
        let encoded = c.args.last().unwrap().to_string_lossy().into_owned();
        let bytes = STANDARD.decode(encoded).unwrap();
        let utf16: Vec<u16> = bytes
            .chunks(2)
            .map(|c| u16::from_le_bytes([c[0], c[1]]))
            .collect();
        assert_eq!(String::from_utf16(&utf16).unwrap(), SIGNATURE_SCRIPT);
        let s = scan_call(p, Duration::from_secs(1));
        assert_eq!(s.args[4], p.as_os_str());
        assert_eq!(s.args.len(), 6);
        assert_eq!(s.args[5], "-DisableRemediation");
    }
}
