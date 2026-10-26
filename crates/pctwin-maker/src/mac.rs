//! A Mac's own checks: `codesign` for the signature and Team ID, `spctl` for Gatekeeper and
//! notarisation, and `pkgutil` for installer packages. Fixed arguments, never a shell string.
//!
//! Not yet proven on a real Mac: the parsers are written against output published by Apple and in
//! Apple's developer forums (26 October 2026), kept as fixtures below.
//!
//! - A disk image may carry no usable signature while the app inside does: the app is checked,
//!   after the installer step mounts the image read-only.
//! - `spctl` asks Apple whether the app is notarised, so it needs the internet.
//! - macOS offers other apps no on-demand malware scan (XProtect cannot be asked); Apple's notary
//!   service scans an app for malware before notarising it, and that is what the report says.
//! - A Gatekeeper rejection ends "not verified", never "malware": rejections of good, stapled
//!   packages have been reported.

use std::path::{Path, PathBuf};
use std::time::Duration;

use crate::facts::{Arch, ArchSet, MacTeam, Malware, Signature, valid_team_id};
use crate::run::{Call, Ran, RunError, Runner};

fn call(program: &str, args: &[&str], path: &Path, timeout: Duration) -> Call {
    let mut a: Vec<std::ffi::OsString> = args.iter().map(Into::into).collect();
    a.push(path.as_os_str().to_owned());
    Call {
        program: PathBuf::from(program),
        args: a,
        env: vec![],
        timeout,
    }
}

/// `codesign -dv --verbose=4 <path>`: who signed it (printed on standard error).
pub fn display_call(path: &Path, t: Duration) -> Call {
    call("/usr/bin/codesign", &["-dv", "--verbose=4"], path, t)
}

/// `codesign --verify --deep --strict --verbose=2 <path>`: whether the signature holds.
pub fn verify_call(path: &Path, t: Duration) -> Call {
    call(
        "/usr/bin/codesign",
        &["--verify", "--deep", "--strict", "--verbose=2"],
        path,
        t,
    )
}

/// `spctl --assess --type execute|install -vv <path>`: what Gatekeeper decides.
pub fn gatekeeper_call(path: &Path, package: bool, t: Duration) -> Call {
    let kind = if package { "install" } else { "execute" };
    call(
        "/usr/sbin/spctl",
        &["--assess", "--type", kind, "-vv"],
        path,
        t,
    )
}

/// `pkgutil --check-signature <path>`: an installer package's signature.
pub fn pkg_call(path: &Path, t: Duration) -> Call {
    call("/usr/sbin/pkgutil", &["--check-signature"], path, t)
}

/// What `codesign -dv` says.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Display {
    pub team: Option<MacTeam>,
    pub arch: Option<Arch>,
    pub unsigned: bool,
}

/// Reads `codesign -dv --verbose=4` (its standard error).
pub fn parse_display(ran: &Ran) -> Display {
    let text = &ran.stderr;
    let unsigned = text.contains("code object is not signed at all");
    let team = text
        .lines()
        .find_map(|l| l.trim().strip_prefix("TeamIdentifier="))
        .map(str::trim)
        .filter(|t| valid_team_id(t))
        .map(|t| MacTeam {
            team_id: t.to_owned(),
        });
    let arch = text
        .lines()
        .find_map(|l| l.trim().strip_prefix("Format="))
        .and_then(format_arch);
    Display {
        team: if unsigned { None } else { team },
        arch,
        unsigned,
    }
}

/// "app bundle with Mach-O universal (x86_64 arm64)", "Mach-O thin (arm64)".
fn format_arch(format: &str) -> Option<Arch> {
    let inside = format.rsplit_once('(')?.1.strip_suffix(')')?;
    let mut set = ArchSet::default();
    for a in inside.split_whitespace() {
        match a {
            "x86_64" | "x86_64h" => set.x64 = true,
            "arm64" | "arm64e" => set.arm64 = true,
            "i386" => set.x86 = true,
            _ => {}
        }
    }
    match (set.x86, set.x64, set.arm64) {
        (false, false, false) => None,
        (false, true, false) => Some(Arch::X64),
        (false, false, true) => Some(Arch::Arm64),
        (true, false, false) => Some(Arch::X86),
        _ => Some(Arch::Several(set)),
    }
}

/// Reads `codesign --verify`. Its exit codes are documented in its manual: 0 valid, 1 failed,
/// 2 wrong arguments, 3 a requirement not met.
pub fn parse_verify(ran: &Result<Ran, RunError>) -> Signature {
    match ran {
        Ok(r) if r.code == Some(0) => Signature::Trusted,
        Ok(r) if r.stderr.contains("not signed at all") => Signature::Missing,
        Ok(r) if r.code == Some(1) => Signature::Broken {
            why: first_line(&r.stderr).into(),
        },
        Ok(r) => Signature::NotChecked {
            why: format!("codesign could not check it: {}", first_line(&r.stderr)),
        },
        Err(e) => Signature::NotChecked {
            why: format!("codesign gave no answer: {e}"),
        },
    }
}

fn first_line(s: &str) -> &str {
    s.lines()
        .map(str::trim)
        .find(|l| !l.is_empty())
        .unwrap_or("no message")
}

/// What Gatekeeper decided.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Gatekeeper {
    /// Accepted; `notarised` when its source is "Notarized Developer ID".
    Accepted {
        notarised: bool,
        origin: Option<String>,
    },
    /// Rejected, with the source Gatekeeper gave.
    Rejected {
        source: String,
    },
    NotChecked {
        why: String,
    },
}

/// Reads `spctl --assess -vv` (its standard error; exit 0 accepted, 3 rejected).
pub fn parse_gatekeeper(ran: &Result<Ran, RunError>) -> Gatekeeper {
    let r = match ran {
        Ok(r) => r,
        Err(e) => {
            return Gatekeeper::NotChecked {
                why: format!("Gatekeeper gave no answer: {e}"),
            };
        }
    };
    let text = format!("{}\n{}", r.stderr, r.stdout);
    let field = |name: &str| {
        text.lines()
            .find_map(|l| l.trim().strip_prefix(name))
            .map(|v| v.trim().to_owned())
    };
    let source = field("source=");
    let first = text
        .lines()
        .map(str::trim)
        .find(|l| !l.is_empty())
        .unwrap_or("");
    match r.code {
        Some(0) if first.ends_with(": accepted") => Gatekeeper::Accepted {
            notarised: source.as_deref() == Some("Notarized Developer ID"),
            origin: field("origin="),
        },
        Some(3) if first.ends_with(": rejected") => Gatekeeper::Rejected {
            source: source.unwrap_or_else(|| "no reason given".into()),
        },
        _ => Gatekeeper::NotChecked {
            why: format!("Gatekeeper could not check it: {first}"),
        },
    }
}

/// What `pkgutil --check-signature` says about an installer package.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PkgSignature {
    pub signature: Signature,
    pub notarised: bool,
    pub team: Option<MacTeam>,
}

/// Reads `pkgutil --check-signature`.
pub fn parse_pkg(ran: &Result<Ran, RunError>) -> PkgSignature {
    let r = match ran {
        Ok(r) => r,
        Err(e) => {
            return PkgSignature {
                signature: Signature::NotChecked {
                    why: format!("pkgutil gave no answer: {e}"),
                },
                notarised: false,
                team: None,
            };
        }
    };
    let status = r
        .stdout
        .lines()
        .find_map(|l| l.trim().strip_prefix("Status:"))
        .map(str::trim)
        .unwrap_or("");
    let signature = if status == "no signature" {
        Signature::Missing
    } else if status.starts_with("signed by a developer certificate issued by Apple")
        || status.starts_with("signed Apple Software")
    {
        Signature::Trusted
    } else if status.starts_with("signed by a certificate") || status.contains("untrusted") {
        Signature::NotTrusted { why: status.into() }
    } else if status.is_empty() {
        Signature::NotChecked {
            why: format!("pkgutil could not check it: {}", first_line(&r.stderr)),
        }
    } else {
        Signature::NotTrusted { why: status.into() }
    };
    let notarised = r
        .stdout
        .lines()
        .any(|l| l.trim() == "Notarization: trusted by the Apple notary service");
    // The first certificate in the chain: "1. Developer ID Installer: Name (TEAMID)".
    let team = r
        .stdout
        .lines()
        .find_map(|l| l.trim().strip_prefix("1. Developer ID Installer: "))
        .and_then(team_in_brackets)
        .map(|t| MacTeam { team_id: t });
    PkgSignature {
        signature,
        notarised,
        team,
    }
}

/// "Name (ABCDE12345)" gives "ABCDE12345".
fn team_in_brackets(s: &str) -> Option<String> {
    let inner = s.trim().strip_suffix(')')?.rsplit_once('(')?.1;
    valid_team_id(inner).then(|| inner.to_owned())
}

/// The malware fact a Mac can give: notarised and accepted by Gatekeeper means Apple's notary
/// service scanned it; anything else is "not fully checked", never "malware".
pub fn malware_from(gatekeeper: &Gatekeeper) -> Malware {
    match gatekeeper {
        Gatekeeper::Accepted {
            notarised: true, ..
        } => Malware::CoveredBySystemTrust {
            how: "macOS offers no on-demand malware scan to other apps; Apple's notary service scanned this app before notarising it, and Gatekeeper accepted it".into(),
        },
        Gatekeeper::Accepted { .. } => Malware::NotFullyChecked {
            why: "the app is not notarised by Apple".into(),
        },
        Gatekeeper::Rejected { source } => Malware::NotFullyChecked {
            why: format!("Gatekeeper did not verify it ({source})"),
        },
        Gatekeeper::NotChecked { why } => Malware::NotFullyChecked { why: why.clone() },
    }
}

/// The facts a Mac gives about one app bundle or installer package.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MacFacts {
    pub signature: Signature,
    pub team: Option<MacTeam>,
    pub malware: Malware,
    pub arch: Option<Arch>,
}

/// Checks an installer package (`.pkg`).
pub fn check_pkg(runner: &dyn Runner, path: &Path, t: Duration) -> MacFacts {
    let pkg = parse_pkg(&runner.run(&pkg_call(path, t)));
    let gk = parse_gatekeeper(&runner.run(&gatekeeper_call(path, true, t)));
    let signature = match (&pkg.signature, &gk) {
        (Signature::Trusted, Gatekeeper::Accepted { .. }) => Signature::Trusted,
        (Signature::Trusted, Gatekeeper::Rejected { source }) => Signature::NotTrusted {
            why: format!("Gatekeeper did not verify it ({source})"),
        },
        (Signature::Trusted, Gatekeeper::NotChecked { why }) => {
            Signature::NotChecked { why: why.clone() }
        }
        (other, _) => other.clone(),
    };
    MacFacts {
        signature,
        team: pkg.team,
        malware: malware_from(&gk),
        arch: None,
    }
}

/// Checks an app bundle (`.app`), such as the one inside a mounted disk image.
pub fn check_app(runner: &dyn Runner, path: &Path, t: Duration) -> MacFacts {
    let display = runner.run(&display_call(path, t));
    let shown = display.as_ref().map(parse_display).ok();
    let verified = parse_verify(&runner.run(&verify_call(path, t)));
    let gk = parse_gatekeeper(&runner.run(&gatekeeper_call(path, false, t)));
    let signature = match (verified, &gk) {
        (Signature::Trusted, Gatekeeper::Accepted { .. }) => Signature::Trusted,
        (Signature::Trusted, Gatekeeper::Rejected { source }) => Signature::NotTrusted {
            why: format!("Gatekeeper did not verify it ({source})"),
        },
        (Signature::Trusted, Gatekeeper::NotChecked { why }) => {
            Signature::NotChecked { why: why.clone() }
        }
        (other, _) => other,
    };
    MacFacts {
        signature,
        team: shown.as_ref().and_then(|d| d.team.clone()),
        malware: malware_from(&gk),
        arch: shown.and_then(|d| d.arch),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn err(text: &str, code: i32) -> Result<Ran, RunError> {
        Ok(Ran {
            code: Some(code),
            stdout: String::new(),
            stderr: text.into(),
        })
    }

    fn out(text: &str, code: i32) -> Result<Ran, RunError> {
        Ok(Ran {
            code: Some(code),
            stdout: text.into(),
            stderr: String::new(),
        })
    }

    // From Apple's developer forums (the Developer ID authority chain) and Jamf's guide
    // (TeamIdentifier line); the shape `codesign -dv --verbose=4` prints.
    const DISPLAY: &str = "Executable=/Applications/Example.app/Contents/MacOS/Example
Identifier=com.example.Example
Format=app bundle with Mach-O universal (x86_64 arm64)
CodeDirectory v=20500 size=1234 flags=0x10000(runtime) hashes=27+7 location=embedded
Signature size=9000
Authority=Developer ID Application: Example Ltd (SKMME9E2Y8)
Authority=Developer ID Certification Authority
Authority=Apple Root CA
Timestamp=26 Oct 2026 at 10:00:00
Notarization Ticket=stapled
TeamIdentifier=SKMME9E2Y8
Runtime Version=14.0.0
";

    #[test]
    fn codesign_display() {
        let d = parse_display(&err(DISPLAY, 0).unwrap());
        assert_eq!(
            d.team,
            Some(MacTeam {
                team_id: "SKMME9E2Y8".into()
            })
        );
        assert!(!d.unsigned);
        let both = ArchSet {
            x86: false,
            x64: true,
            arm64: true,
        };
        assert_eq!(d.arch, Some(Arch::Several(both)));
        let thin = DISPLAY.replace("universal (x86_64 arm64)", "thin (arm64)");
        assert_eq!(
            parse_display(&err(&thin, 0).unwrap()).arch,
            Some(Arch::Arm64)
        );
        let intel = DISPLAY.replace("universal (x86_64 arm64)", "thin (x86_64)");
        assert_eq!(
            parse_display(&err(&intel, 0).unwrap()).arch,
            Some(Arch::X64)
        );
        let odd = DISPLAY.replace("universal (x86_64 arm64)", "thin (ppc)");
        assert_eq!(parse_display(&err(&odd, 0).unwrap()).arch, None);
        let adhoc = DISPLAY.replace("TeamIdentifier=SKMME9E2Y8", "TeamIdentifier=not set");
        assert_eq!(parse_display(&err(&adhoc, 0).unwrap()).team, None);
        let unsigned = parse_display(
            &err("/Applications/X.app: code object is not signed at all\n", 1).unwrap(),
        );
        assert!(unsigned.unsigned);
        assert_eq!(unsigned.team, None);
    }

    #[test]
    fn codesign_verify() {
        assert_eq!(parse_verify(&err("", 0)), Signature::Trusted);
        assert_eq!(
            parse_verify(&err("X.app: code object is not signed at all", 1)),
            Signature::Missing
        );
        assert!(matches!(
            parse_verify(&err("X.app: a sealed resource is missing or invalid", 1)),
            Signature::Broken { why } if why.contains("sealed resource")
        ));
        assert!(matches!(
            parse_verify(&err("bad args", 2)),
            Signature::NotChecked { .. }
        ));
        assert!(matches!(
            parse_verify(&Err(RunError::TimedOut)),
            Signature::NotChecked { .. }
        ));
    }

    // From Apple's developer forums (DTS reply): notarised and un-notarised Developer ID.
    #[test]
    fn gatekeeper() {
        let ok = "/Applications/Example.app: accepted\nsource=Notarized Developer ID\norigin=Developer ID Application: Example Ltd (SKMME9E2Y8)\n";
        assert_eq!(
            parse_gatekeeper(&err(ok, 0)),
            Gatekeeper::Accepted {
                notarised: true,
                origin: Some("Developer ID Application: Example Ltd (SKMME9E2Y8)".into())
            }
        );
        let plain = ok.replace("Notarized Developer ID", "Developer ID");
        assert!(matches!(
            parse_gatekeeper(&err(&plain, 0)),
            Gatekeeper::Accepted {
                notarised: false,
                ..
            }
        ));
        let no = "/Applications/Example.app: rejected\nsource=Unnotarized Developer ID\n";
        assert_eq!(
            parse_gatekeeper(&err(no, 3)),
            Gatekeeper::Rejected {
                source: "Unnotarized Developer ID".into()
            }
        );
        // An accepted line with a failing code, or the reverse, is not believed.
        assert!(matches!(
            parse_gatekeeper(&err(ok, 3)),
            Gatekeeper::NotChecked { .. }
        ));
        assert!(matches!(
            parse_gatekeeper(&err(no, 0)),
            Gatekeeper::NotChecked { .. }
        ));
        assert!(matches!(
            parse_gatekeeper(&Err(RunError::TimedOut)),
            Gatekeeper::NotChecked { .. }
        ));
        let bare = "/x.app: rejected\n";
        assert_eq!(
            parse_gatekeeper(&err(bare, 3)),
            Gatekeeper::Rejected {
                source: "no reason given".into()
            }
        );
    }

    #[test]
    fn a_rejection_is_never_malware() {
        let gk = Gatekeeper::Rejected {
            source: "Unnotarized Developer ID".into(),
        };
        assert!(matches!(malware_from(&gk), Malware::NotFullyChecked { .. }));
        let gk = Gatekeeper::Accepted {
            notarised: false,
            origin: None,
        };
        assert!(matches!(malware_from(&gk), Malware::NotFullyChecked { .. }));
        let gk = Gatekeeper::NotChecked {
            why: "offline".into(),
        };
        assert!(matches!(malware_from(&gk), Malware::NotFullyChecked { .. }));
        let gk = Gatekeeper::Accepted {
            notarised: true,
            origin: None,
        };
        assert!(matches!(
            malware_from(&gk),
            Malware::CoveredBySystemTrust { .. }
        ));
    }

    // The shape in Apple's notarisation troubleshooting page and forum posts.
    const PKG: &str = "Package \"Example.pkg\":
   Status: signed by a developer certificate issued by Apple for distribution
   Notarization: trusted by the Apple notary service
   Signed with a trusted timestamp on: 2026-10-26 10:00:00 +0000
   Certificate Chain:
    1. Developer ID Installer: Example Ltd (SKMME9E2Y8)
       Expires: 2031-01-01 00:00:00 +0000
       SHA256 Fingerprint:
           AB CD EF
       ------------------------------------------------------------------------
    2. Developer ID Certification Authority
    3. Apple Root CA
";

    #[test]
    fn pkgutil() {
        let p = parse_pkg(&out(PKG, 0));
        assert_eq!(p.signature, Signature::Trusted);
        assert!(p.notarised);
        assert_eq!(
            p.team,
            Some(MacTeam {
                team_id: "SKMME9E2Y8".into()
            })
        );
        let un = parse_pkg(&out("Package \"x.pkg\":\n   Status: no signature\n", 1));
        assert_eq!(un.signature, Signature::Missing);
        assert!(!un.notarised);
        assert_eq!(un.team, None);
        let untrusted = PKG.replace(
            "signed by a developer certificate issued by Apple for distribution",
            "signed by a certificate trusted by Mac OS X",
        );
        assert!(matches!(
            parse_pkg(&out(&untrusted, 0)).signature,
            Signature::NotTrusted { .. }
        ));
        let odd = PKG.replace(
            "signed by a developer certificate issued by Apple for distribution",
            "something new",
        );
        assert!(matches!(
            parse_pkg(&out(&odd, 0)).signature,
            Signature::NotTrusted { .. }
        ));
        assert!(matches!(
            parse_pkg(&out("", 1)).signature,
            Signature::NotChecked { .. }
        ));
        assert!(matches!(
            parse_pkg(&Err(RunError::TimedOut)).signature,
            Signature::NotChecked { .. }
        ));
        let bad_team = PKG.replace("(SKMME9E2Y8)", "(short)");
        assert_eq!(parse_pkg(&out(&bad_team, 0)).team, None);
        assert_eq!(
            team_in_brackets("Name (ABCDE12345)"),
            Some("ABCDE12345".into())
        );
        assert_eq!(team_in_brackets("Name ABCDE12345"), None);
    }

    struct Canned(Vec<(&'static str, Result<Ran, RunError>)>);
    impl Runner for Canned {
        fn run(&self, call: &Call) -> Result<Ran, RunError> {
            let key = format!(
                "{} {}",
                call.program.display(),
                call.args[0].to_string_lossy()
            );
            self.0
                .iter()
                .find(|(k, _)| key.ends_with(k))
                .map(|(_, r)| r.clone())
                .unwrap_or(Err(RunError::NotStarted(key)))
        }
    }

    #[test]
    fn a_notarised_app_and_package() {
        let ok = "/x.app: accepted\nsource=Notarized Developer ID\n";
        let app = Canned(vec![
            ("codesign -dv", err(DISPLAY, 0)),
            ("codesign --verify", err("", 0)),
            ("spctl --assess", err(ok, 0)),
        ]);
        let f = check_app(&app, Path::new("/x.app"), Duration::from_secs(1));
        assert_eq!(f.signature, Signature::Trusted);
        assert_eq!(
            f.team,
            Some(MacTeam {
                team_id: "SKMME9E2Y8".into()
            })
        );
        assert!(matches!(f.malware, Malware::CoveredBySystemTrust { .. }));
        assert!(f.arch.is_some());

        let rejected = Canned(vec![
            ("codesign -dv", err(DISPLAY, 0)),
            ("codesign --verify", err("", 0)),
            (
                "spctl --assess",
                err("/x.app: rejected\nsource=Unnotarized Developer ID\n", 3),
            ),
        ]);
        let f = check_app(&rejected, Path::new("/x.app"), Duration::from_secs(1));
        assert!(matches!(f.signature, Signature::NotTrusted { .. }));
        assert!(matches!(f.malware, Malware::NotFullyChecked { .. }));

        let offline = Canned(vec![
            ("codesign -dv", err(DISPLAY, 0)),
            ("codesign --verify", err("", 0)),
        ]);
        let f = check_app(&offline, Path::new("/x.app"), Duration::from_secs(1));
        assert!(matches!(f.signature, Signature::NotChecked { .. }));

        let pkg = Canned(vec![
            ("pkgutil --check-signature", out(PKG, 0)),
            (
                "spctl --assess",
                err("/x.pkg: accepted\nsource=Notarized Developer ID\n", 0),
            ),
        ]);
        let f = check_pkg(&pkg, Path::new("/x.pkg"), Duration::from_secs(1));
        assert_eq!(f.signature, Signature::Trusted);
        assert!(matches!(f.malware, Malware::CoveredBySystemTrust { .. }));
        let pkg_rejected = Canned(vec![
            ("pkgutil --check-signature", out(PKG, 0)),
            (
                "spctl --assess",
                err("/x.pkg: rejected\nsource=Unnotarized Developer ID\n", 3),
            ),
        ]);
        let f = check_pkg(&pkg_rejected, Path::new("/x.pkg"), Duration::from_secs(1));
        assert!(matches!(f.signature, Signature::NotTrusted { .. }));
        let pkg_offline = Canned(vec![("pkgutil --check-signature", out(PKG, 0))]);
        let f = check_pkg(&pkg_offline, Path::new("/x.pkg"), Duration::from_secs(1));
        assert!(matches!(f.signature, Signature::NotChecked { .. }));
        let unsigned = Canned(vec![
            (
                "pkgutil --check-signature",
                out("   Status: no signature\n", 1),
            ),
            ("spctl --assess", err("/x.pkg: rejected\n", 3)),
        ]);
        let f = check_pkg(&unsigned, Path::new("/x.pkg"), Duration::from_secs(1));
        assert_eq!(f.signature, Signature::Missing);
    }

    #[test]
    fn calls_are_fixed() {
        let p = Path::new("/Volumes/X/-rf Example.app");
        let c = gatekeeper_call(p, false, Duration::from_secs(1));
        assert_eq!(c.program, PathBuf::from("/usr/sbin/spctl"));
        assert_eq!(c.args.len(), 5);
        assert_eq!(c.args[2], "execute");
        assert_eq!(c.args[4], p.as_os_str());
        assert_eq!(
            gatekeeper_call(p, true, Duration::from_secs(1)).args[2],
            "install"
        );
        assert_eq!(verify_call(p, Duration::from_secs(1)).args.len(), 5);
        assert_eq!(display_call(p, Duration::from_secs(1)).args.len(), 3);
        assert_eq!(pkg_call(p, Duration::from_secs(1)).args.len(), 2);
    }
}
