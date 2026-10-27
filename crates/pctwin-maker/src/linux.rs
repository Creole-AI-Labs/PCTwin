//! Linux: the package manager's own signature checks. Linux has no standard on-demand malware
//! scan, so trust rests on the signing key the system already trusts, and the report says so.
//!
//! Not yet proven on a real Linux machine: the parsers follow rpm's manual and output shown in
//! Red Hat's and others' published examples (26 October 2026), kept as fixtures below.
//!
//! - RPM: `rpm --checksig --verbose` checks the package's signature against keys this system has
//!   imported. A key the system does not know is "not trusted", never trusted on the download's
//!   word.
//! - Flatpak: a `.flatpakref` file names the app, its remote and the remote's signing key; Flatpak
//!   refuses anything from that remote not signed by that key. The key must be the old laptop's.
//! - A `.deb` or AppImage downloaded from a website carries no signature the system checks: it
//!   ends needs-action, with the data still moved.

use std::path::{Path, PathBuf};
use std::time::Duration;

use base64::Engine as _;
use base64::engine::general_purpose::STANDARD;
use sha2::{Digest, Sha256 as Sha256Hasher};

use crate::facts::{Arch, ArchSet, LinuxSource, Malware, Sha256, Signature};
use crate::run::{Call, Ran, RunError, Runner};

/// The most bytes of a `.flatpakref` file read.
pub const MAX_FLATPAKREF: usize = 64 * 1024;

/// `rpm --checksig --verbose <path>`.
pub fn checksig_call(path: &Path, t: Duration) -> Call {
    Call {
        program: PathBuf::from("/usr/bin/rpm"),
        args: vec![
            "--checksig".into(),
            "--verbose".into(),
            path.as_os_str().to_owned(),
        ],
        env: vec![],
        timeout: t,
    }
}

/// `rpm --query --package --queryformat %{ARCH} <path>`: what the package is built for.
pub fn arch_call(path: &Path, t: Duration) -> Call {
    Call {
        program: PathBuf::from("/usr/bin/rpm"),
        args: vec![
            "--query".into(),
            "--package".into(),
            "--queryformat".into(),
            "%{ARCH}".into(),
            path.as_os_str().to_owned(),
        ],
        env: vec![],
        timeout: t,
    }
}

/// What rpm says about a package's signature, and the key that signed it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RpmCheck {
    pub signature: Signature,
    pub key: Option<String>,
}

/// Reads `rpm --checksig --verbose`. Each line ends with a result: OK, BAD, NOKEY (the key is not
/// imported), NOTTRUSTED or NOTFOUND (rpm's manual).
pub fn parse_checksig(ran: &Result<Ran, RunError>) -> RpmCheck {
    let r = match ran {
        Ok(r) => r,
        Err(e) => {
            return RpmCheck {
                signature: Signature::NotChecked {
                    why: format!("rpm gave no answer: {e}"),
                },
                key: None,
            };
        }
    };
    let text = format!("{}\n{}", r.stdout, r.stderr);
    let mut sigs = Vec::new();
    let mut digests_ok = true;
    let mut saw_digest = false;
    for line in text.lines().map(str::trim) {
        let Some((what, result)) = line.rsplit_once(": ") else {
            continue;
        };
        let result = result.trim();
        if what.contains("Signature") {
            sigs.push((what, result));
        } else if what.contains("digest") {
            saw_digest = true;
            digests_ok &= result == "OK";
        }
    }
    let key = sigs.iter().find_map(|(what, _)| key_of(what));
    let signature = if !digests_ok || sigs.iter().any(|(_, r)| *r == "BAD") {
        Signature::Broken {
            why: "rpm says the package does not match its signature or digest".into(),
        }
    } else if !saw_digest {
        Signature::NotChecked {
            why: "rpm gave an answer PCTwin cannot read".into(),
        }
    } else if sigs.iter().all(|(_, r)| *r == "NOTFOUND") {
        Signature::Missing
    } else if sigs.iter().all(|(_, r)| *r == "OK" || *r == "NOTFOUND") {
        Signature::Trusted
    } else {
        Signature::NotTrusted {
            why: "this system does not trust the key that signed the package".into(),
        }
    };
    let key =
        key.filter(|_| matches!(signature, Signature::Trusted | Signature::NotTrusted { .. }));
    RpmCheck { signature, key }
}

/// "Header V4 RSA/SHA256 Signature, key ID 05d7eace" or "... key fingerprint 5d4a...".
fn key_of(what: &str) -> Option<String> {
    let after = what
        .split_once("key ID ")
        .or_else(|| what.split_once("key fingerprint"))?
        .1
        .trim()
        .trim_start_matches(':')
        .trim();
    let hex: String = after.chars().take_while(char::is_ascii_hexdigit).collect();
    (hex.len() >= 8 && hex.len() == after.len()).then(|| hex.to_ascii_lowercase())
}

/// Reads rpm's word for the package's architecture.
pub fn parse_arch(ran: &Result<Ran, RunError>) -> Option<Arch> {
    let r = ran.as_ref().ok().filter(|r| r.code == Some(0))?;
    match r.stdout.trim() {
        "x86_64" => Some(Arch::X64),
        "aarch64" => Some(Arch::Arm64),
        "i386" | "i486" | "i586" | "i686" => Some(Arch::X86),
        "noarch" => Some(Arch::Several(ArchSet {
            x86: true,
            x64: true,
            arm64: true,
        })),
        _ => None,
    }
}

/// The malware fact on Linux: the system's trust in the signing key stands in for a scan.
pub fn malware_from(signature: &Signature) -> Malware {
    match signature {
        Signature::Trusted => Malware::CoveredBySystemTrust {
            how: "Linux has no standard malware scan; this package is signed with a key this system already trusts".into(),
        },
        _ => Malware::NotFullyChecked {
            why: "Linux has no standard malware scan, and the package's signing key is not trusted here".into(),
        },
    }
}

/// Checks an RPM package.
pub fn check_rpm(runner: &dyn Runner, path: &Path, t: Duration) -> (RpmCheck, Option<Arch>) {
    let check = parse_checksig(&runner.run(&checksig_call(path, t)));
    let arch = parse_arch(&runner.run(&arch_call(path, t)));
    (check, arch)
}

/// Reads a `.flatpakref` file: the maker it names, if it names one with a key.
pub fn parse_flatpakref(bytes: &[u8]) -> Result<LinuxSource, String> {
    if bytes.len() > MAX_FLATPAKREF {
        return Err("the Flatpak reference is too large".into());
    }
    let text = std::str::from_utf8(bytes).map_err(|_| "the Flatpak reference is not text")?;
    let mut lines = text.lines().map(str::trim);
    if lines.next() != Some("[Flatpak Ref]") {
        return Err("not a Flatpak reference".into());
    }
    let (mut name, mut url, mut key) = (None, None, None);
    for line in lines {
        if line.starts_with('[') {
            break;
        }
        let Some((k, v)) = line.split_once('=') else {
            continue;
        };
        let slot = match k.trim() {
            "Name" => &mut name,
            "Url" => &mut url,
            "GPGKey" => &mut key,
            _ => continue,
        };
        if slot.is_some() {
            return Err(format!("the Flatpak reference names {} twice", k.trim()));
        }
        *slot = Some(v.trim().to_owned());
    }
    let name = name
        .filter(|n| !n.is_empty())
        .ok_or("the Flatpak reference names no app")?;
    let url = url
        .filter(|u| u.starts_with("https://"))
        .ok_or("the Flatpak reference names no secure remote")?;
    let key = key.ok_or("the Flatpak reference carries no signing key")?;
    let key = STANDARD
        .decode(key.as_bytes())
        .map_err(|_| "the Flatpak reference's key cannot be read")?;
    if key.is_empty() {
        return Err("the Flatpak reference carries no signing key".into());
    }
    Ok(LinuxSource::Flatpak {
        remote_url: url,
        app_id: name,
        key: Sha256(Sha256Hasher::digest(&key).into()),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn out(text: &str) -> Result<Ran, RunError> {
        Ok(Ran {
            code: Some(0),
            stdout: text.into(),
            stderr: String::new(),
        })
    }

    // The oVirt list's capture (a key not imported) and the OK form rpm prints for the same lines.
    const NOKEY: &str = "areplicate-2023.5.0-213.x86_64.rpm:
    Header V4 RSA/SHA256 Signature, key ID 24901d0c: NOKEY
    Header SHA256 digest: OK
    Header SHA1 digest: OK
    Payload SHA256 digest: OK
";

    #[test]
    fn rpm_results() {
        let n = parse_checksig(&out(NOKEY));
        assert!(matches!(n.signature, Signature::NotTrusted { .. }));
        assert_eq!(n.key.as_deref(), Some("24901d0c"));

        let ok = NOKEY.replace("NOKEY", "OK");
        let o = parse_checksig(&out(&ok));
        assert_eq!(o.signature, Signature::Trusted);
        assert_eq!(o.key.as_deref(), Some("24901d0c"));

        let bad = NOKEY.replace("NOKEY", "BAD");
        assert!(matches!(
            parse_checksig(&out(&bad)).signature,
            Signature::Broken { .. }
        ));
        assert_eq!(parse_checksig(&out(&bad)).key, None);
        let bad_digest = ok.replace("Payload SHA256 digest: OK", "Payload SHA256 digest: BAD");
        assert!(matches!(
            parse_checksig(&out(&bad_digest)).signature,
            Signature::Broken { .. }
        ));

        let none = NOKEY.replace("key ID 24901d0c: NOKEY", "key ID 24901d0c: NOTFOUND");
        assert_eq!(parse_checksig(&out(&none)).signature, Signature::Missing);
        let untrusted = NOKEY.replace("NOKEY", "NOTTRUSTED");
        assert!(matches!(
            parse_checksig(&out(&untrusted)).signature,
            Signature::NotTrusted { .. }
        ));

        // Newer rpm prints the full fingerprint.
        let fp = "x.rpm:\n    Header OpenPGP V4 RSA/SHA256 signature, key fingerprint: 5D4A1B2C3D4E5F60718293A4B5C6D7E8F9012345: OK\n    Header SHA256 digest: OK\n    Payload SHA256 digest: OK\n"
            .replace("signature", "Signature");
        let f = parse_checksig(&out(&fp));
        assert_eq!(f.signature, Signature::Trusted);
        assert_eq!(
            f.key.as_deref(),
            Some("5d4a1b2c3d4e5f60718293a4b5c6d7e8f9012345")
        );

        assert!(matches!(
            parse_checksig(&out("")).signature,
            Signature::NotChecked { .. }
        ));
        assert!(matches!(
            parse_checksig(&Err(RunError::TimedOut)).signature,
            Signature::NotChecked { .. }
        ));
    }

    #[test]
    fn rpm_keys() {
        assert_eq!(
            key_of("Header V4 RSA/SHA256 Signature, key ID 05D7EACE"),
            Some("05d7eace".into())
        );
        assert_eq!(key_of("Header V4 RSA/SHA256 Signature, key ID 05d7"), None);
        assert_eq!(
            key_of("Header V4 RSA/SHA256 Signature, key ID 05d7eacezz"),
            None
        );
        assert_eq!(key_of("Header V4 RSA/SHA256 Signature"), None);
    }

    #[test]
    fn rpm_arch() {
        assert_eq!(parse_arch(&out("x86_64")), Some(Arch::X64));
        assert_eq!(parse_arch(&out("aarch64\n")), Some(Arch::Arm64));
        assert_eq!(parse_arch(&out("i686")), Some(Arch::X86));
        for a in [Arch::X86, Arch::X64, Arch::Arm64] {
            assert!(parse_arch(&out("noarch")).is_some_and(|f| f.runs_on(a)));
        }
        assert_eq!(parse_arch(&out("ppc64le")), None);
        let failed = Ok(Ran {
            code: Some(1),
            stdout: "x86_64".into(),
            stderr: String::new(),
        });
        assert_eq!(parse_arch(&failed), None);
    }

    #[test]
    fn linux_malware_rests_on_the_key() {
        assert!(matches!(
            malware_from(&Signature::Trusted),
            Malware::CoveredBySystemTrust { .. }
        ));
        assert!(matches!(
            malware_from(&Signature::Missing),
            Malware::NotFullyChecked { .. }
        ));
    }

    const REF: &str = "[Flatpak Ref]
Title=Example
Name=org.example.App
Branch=stable
Url=https://dl.flathub.org/repo/
IsRuntime=false
GPGKey=AAECAwQ=
RuntimeRepo=https://dl.flathub.org/repo/flathub.flatpakrepo
";

    #[test]
    fn flatpak_references() {
        let s = parse_flatpakref(REF.as_bytes()).unwrap();
        assert_eq!(
            s,
            LinuxSource::Flatpak {
                remote_url: "https://dl.flathub.org/repo/".into(),
                app_id: "org.example.App".into(),
                key: Sha256(Sha256Hasher::digest([0u8, 1, 2, 3, 4]).into()),
            }
        );
        for (from, to) in [
            ("[Flatpak Ref]", "[Something]"),
            ("Url=https://", "Url=http://"),
            ("GPGKey=AAECAwQ=\n", ""),
            ("GPGKey=AAECAwQ=", "GPGKey=***"),
            ("GPGKey=AAECAwQ=", "GPGKey="),
            ("Name=org.example.App\n", ""),
            ("Name=org.example.App", "Name="),
            ("Branch=stable", "Name=org.evil.App"),
        ] {
            assert!(
                parse_flatpakref(REF.replace(from, to).as_bytes()).is_err(),
                "{to}"
            );
        }
        // A second section is not read.
        let later = REF.replace("GPGKey=AAECAwQ=\n", "") + "[Other]\nGPGKey=AAECAwQ=\n";
        assert!(parse_flatpakref(later.as_bytes()).is_err());
        assert!(parse_flatpakref(&vec![b' '; MAX_FLATPAKREF + 1]).is_err());
        assert!(parse_flatpakref(b"[Flatpak Ref]\xff").is_err());
    }

    #[test]
    fn calls_are_fixed() {
        let p = Path::new("/home/a/--nosignature.rpm");
        let c = checksig_call(p, Duration::from_secs(1));
        assert_eq!(c.args.last().unwrap(), p.as_os_str());
        assert_eq!(c.args.len(), 3);
        assert_eq!(arch_call(p, Duration::from_secs(1)).args.len(), 5);
    }

    #[test]
    fn flatpak_reference_edges() {
        // A blank line or comment before the keys is fine.
        let spaced = REF.replace("Title=Example", "\n# a comment\nTitle=Example");
        assert!(parse_flatpakref(spaced.as_bytes()).is_ok());
        // Exactly the size limit is read.
        let mut full = REF.to_owned();
        full.push_str(&"#".repeat(MAX_FLATPAKREF - REF.len()));
        assert_eq!(full.len(), MAX_FLATPAKREF);
        assert!(parse_flatpakref(full.as_bytes()).is_ok());
    }
}
