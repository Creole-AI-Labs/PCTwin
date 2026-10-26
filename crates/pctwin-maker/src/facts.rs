//! The facts a maker check finds, and the one decision made from them.
//!
//! A verdict is a set of separate facts, never a score (2.1 plan, section 8.1): whether the file
//! is signed and the signature holds, whether the maker is the same as the old laptop's copy,
//! what the malware check found and how far it reached, whether the file is the kind of file
//! expected, and whether it is built for this laptop. [`Verdict::decide`] installs only when every
//! required fact holds; anything else ends with plain reasons, most serious first.

use std::fmt;

use pctwin_types::{ItemResult, Reason};
use serde::{Deserialize, Serialize};

/// The system the installer is for.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum System {
    Windows,
    Mac,
    Linux,
}

/// A SHA-256 fingerprint of exact bytes.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Sha256(pub [u8; 32]);

impl fmt::Display for Sha256 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for b in self.0 {
            write!(f, "{b:02x}")?;
        }
        Ok(())
    }
}

impl fmt::Debug for Sha256 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Sha256({self})")
    }
}

/// The parts of a Windows signing certificate's subject that name the maker. Parts a certificate
/// authority may drop on renewal (department, street, email) are left out on purpose: since 2022
/// code-signing certificates may no longer carry a department, so an older copy's certificate can
/// have one the new one lacks.
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Subject {
    pub common_name: Option<String>,
    pub organization: Option<String>,
    pub locality: Option<String>,
    pub state: Option<String>,
    pub country: Option<String>,
    /// The company registration number on an extended-validation certificate.
    pub registration_number: Option<String>,
    /// Where the company is registered, on an extended-validation certificate.
    pub jurisdiction_country: Option<String>,
    pub jurisdiction_state: Option<String>,
}

/// Who signed a Windows file: the signing certificate's subject, the certificate's own
/// fingerprint, and who issued it.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct WindowsSigner {
    pub subject: Subject,
    /// SHA-256 of the whole signing certificate.
    pub certificate: Sha256,
    /// The issuer, as text, for the report only; never used to decide.
    pub issuer: String,
}

/// A Mac maker: Apple's Team ID for the developer.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct MacTeam {
    pub team_id: String,
}

/// A Linux maker: where the package came from and the key that signs it.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", tag = "kind")]
pub enum LinuxSource {
    /// An RPM package signed with a key the system already trusts. The key is the hex key ID or
    /// fingerprint rpm prints, lower case.
    Rpm { key: String },
    /// A Flatpak app: the remote's address, the app ID, and SHA-256 of the remote's signing key.
    Flatpak {
        remote_url: String,
        app_id: String,
        key: Sha256,
    },
}

/// The maker of an app, taken from the old laptop's own copy.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", tag = "system")]
pub enum Maker {
    Windows(WindowsSigner),
    Mac(MacTeam),
    Linux(LinuxSource),
}

/// The kinds of file the checker recognises from their first bytes, never from their name.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Format {
    /// A Windows program (PE).
    WindowsProgram,
    /// A Windows Installer package (MSI).
    WindowsInstaller,
    /// A Mac program (Mach-O, single or universal).
    MacProgram,
    /// A Mac installer package (xar).
    MacInstaller,
    /// A Mac disk image (UDIF).
    MacDiskImage,
    /// A Linux program (ELF).
    LinuxProgram,
    /// A Debian package.
    DebPackage,
    /// An RPM package.
    RpmPackage,
    /// A Flatpak reference file (.flatpakref).
    FlatpakRef,
    /// A ZIP archive.
    Zip,
    /// A web page: a sign-in page or error page saved under an installer's name.
    WebPage,
    /// None of the above.
    Unknown,
}

/// A processor architecture.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Arch {
    X86,
    X64,
    Arm64,
    /// Runs on more than one (a universal Mac program, a neutral MSI): the list it carries.
    Several(ArchSet),
}

/// A small set of architectures.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ArchSet {
    pub x86: bool,
    pub x64: bool,
    pub arm64: bool,
}

impl Arch {
    /// Whether a file built for `self` runs natively on a laptop of `laptop`.
    pub fn runs_on(self, laptop: Arch) -> bool {
        match self {
            Arch::Several(set) => match laptop {
                Arch::X86 => set.x86,
                Arch::X64 => set.x64,
                Arch::Arm64 => set.arm64,
                Arch::Several(_) => false,
            },
            one => one == laptop,
        }
    }
}

/// Whether the file is signed and whether the signature holds.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", tag = "state")]
pub enum Signature {
    /// No signature at all.
    Missing,
    /// A signature is there but does not match the file, or cannot be read: the file was changed
    /// after signing, or the signature is malformed.
    Broken { why: String },
    /// The signature matches the file but the system does not trust its certificate (expired,
    /// revoked, unknown authority, or not notarised on a Mac).
    NotTrusted { why: String },
    /// The signature matches the file and the system trusts it.
    Trusted,
    /// The system's own check could not run.
    NotChecked { why: String },
}

/// How a maker was found to be the same.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum MatchedBy {
    /// The very same signing certificate.
    SameCertificate,
    /// A renewed certificate naming the same company, in the same place.
    SameCompany,
    /// The same Apple Team ID.
    SameTeam,
    /// The same signing key (Linux).
    SameKey,
}

/// Whether the maker is the same as the old laptop's copy.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", tag = "state")]
pub enum SameMaker {
    Yes { by: MatchedBy },
    No { found: String },
    Unknown { why: String },
}

/// What the malware check found and how far it reached. An incomplete or unavailable check is
/// never "clean".
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", tag = "state")]
pub enum Malware {
    /// The scan finished and found nothing. Coverage says what was and was not looked at.
    NoDetection { coverage: Coverage },
    /// The scan found something.
    Detected { name: String },
    /// The scan started but did not finish, or its evidence is too old.
    NotFullyChecked { why: String },
    /// This system offers no on-demand scan to other apps; trust rests on the system's own
    /// check of the maker (Apple's notarisation, the repository's signature).
    CoveredBySystemTrust { how: String },
    /// No scanner could be used.
    Unavailable { why: String },
}

/// What a finished scan covered.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Coverage {
    /// The scanner, its engine and its signature versions, for the record.
    pub scanner: String,
    /// Always true: a scan covers this file's bytes only. Anything the installer downloads when it
    /// runs is not checked by it (T21-038).
    pub later_downloads_not_checked: bool,
}

/// Whether the file is the kind of file expected.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", tag = "state")]
pub enum FormatFact {
    Matches { found: Format },
    Wrong { found: Format },
}

/// Whether the file is built for this laptop.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", tag = "state")]
pub enum ArchFact {
    Matches,
    Wrong { found: Arch },
    Unknown,
}

/// The file's bytes, as checked: the installer step must find exactly these (T21-035).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Bytes {
    pub size: u64,
    pub sha256: Sha256,
}

/// Whether the bytes stayed the same from the first read to the last.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", tag = "state")]
pub enum Steady {
    /// Read twice, the same both times, and the name still led to the same file.
    Yes,
    /// The file changed or was replaced while it was being checked.
    Changed,
}

/// Everything a check found. Built only by the checker; [`Verdict::decide`] reads it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Verdict {
    pub system: System,
    pub bytes: Bytes,
    pub steady: Steady,
    /// Some only when a trusted recipe named the exact bytes; then they must match.
    pub expected_bytes: Option<Sha256>,
    pub format: FormatFact,
    pub arch: ArchFact,
    pub signature: Signature,
    pub same_maker: SameMaker,
    pub malware: Malware,
    /// The maker found in the file, for the report.
    pub found_maker: Option<Maker>,
}

/// Why an installer was not installed. Each has plain words for the person.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Why {
    ChangedDuringCheck,
    MalwareFound,
    NotExpectedBytes,
    WrongKindOfFile,
    SignatureBroken,
    OtherMaker,
    WrongArchitecture,
    Unsigned,
    NotTrusted,
    MakerUnknown,
    ArchitectureUnknown,
    NotFullyChecked,
}

impl Why {
    /// What happened, in plain words, and what to do next.
    pub fn plain(self) -> &'static str {
        match self {
            Why::ChangedDuringCheck => {
                "The download changed while PCTwin was checking it, so PCTwin did not install it. Download it again from the maker's website."
            }
            Why::MalwareFound => {
                "The security check on this laptop found something harmful in the download, so PCTwin did not install it. Your files are safe. Get the app from the maker's own website."
            }
            Why::NotExpectedBytes => {
                "The download is not the exact file PCTwin expected, so PCTwin did not install it. Get the app from the maker's own website."
            }
            Why::WrongKindOfFile => {
                "The download is not an installer (it may be a web page, such as a sign-in page). Open the maker's website and download it yourself."
            }
            Why::SignatureBroken => {
                "The download's maker signature does not match its contents, so it may have been changed. PCTwin did not install it. Get the app from the maker's own website."
            }
            Why::OtherMaker => {
                "The download is signed by a different maker from the app on your old laptop, so PCTwin did not install it. Get the app from the maker's own website."
            }
            Why::WrongArchitecture => {
                "The download is built for a different kind of laptop. Get the version for this laptop from the maker's website."
            }
            Why::Unsigned => {
                "The download carries no maker signature, so PCTwin cannot prove who made it. Your data is ready; install the app yourself from the maker's website."
            }
            Why::NotTrusted => {
                "This laptop does not trust the download's maker signature (it may have expired). Install the app yourself from the maker's website."
            }
            Why::MakerUnknown => {
                "PCTwin could not tell whether the download comes from the same maker as the app on your old laptop. Install the app yourself from the maker's website."
            }
            Why::ArchitectureUnknown => {
                "PCTwin could not tell whether the download is built for this laptop. Install the app yourself from the maker's website."
            }
            Why::NotFullyChecked => {
                "The security check on this laptop could not finish checking the download, so it was not installed. Try again later, or install it yourself from the maker's website."
            }
        }
    }
}

/// The one decision.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", tag = "decision")]
pub enum Decision {
    /// Every required fact holds. The installer step must find exactly these bytes.
    Install { bytes: Bytes },
    /// Not installed by this route. Never empty; most serious first.
    NeedsAction { why: Vec<Why> },
}

impl Decision {
    /// How the app's item ends when it is not installed: needs-action, so the person can get it
    /// from the maker. None when it may be installed (the installer step reports that).
    pub fn item_result(&self) -> Option<ItemResult> {
        match self {
            Decision::Install { .. } => None,
            Decision::NeedsAction { .. } => Some(ItemResult::because(Reason::ManualInstall)),
        }
    }
}

impl Verdict {
    /// Install only when every required fact holds: the bytes stayed the same and are the ones
    /// expected, the file is the kind expected and built for this laptop, signed with a signature
    /// the system trusts, by the same maker as the old copy, and the malware check finished
    /// without a finding (on a Mac or Linux, where no on-demand scan exists, the system's own
    /// trust in the maker stands in, as the report says; never on Windows).
    pub fn decide(&self) -> Decision {
        let mut why = Vec::new();
        if self.steady != Steady::Yes {
            why.push(Why::ChangedDuringCheck);
        }
        match &self.malware {
            Malware::Detected { .. } => why.push(Why::MalwareFound),
            Malware::NoDetection { coverage } if coverage.later_downloads_not_checked => {}
            Malware::CoveredBySystemTrust { .. } if self.system != System::Windows => {}
            _ => why.push(Why::NotFullyChecked),
        }
        if let Some(expected) = self.expected_bytes
            && expected != self.bytes.sha256
        {
            why.push(Why::NotExpectedBytes);
        }
        if let FormatFact::Wrong { .. } = self.format {
            why.push(Why::WrongKindOfFile);
        }
        match &self.signature {
            Signature::Trusted => {}
            Signature::Missing => why.push(Why::Unsigned),
            Signature::Broken { .. } => why.push(Why::SignatureBroken),
            Signature::NotTrusted { .. } => why.push(Why::NotTrusted),
            Signature::NotChecked { .. } => why.push(Why::NotFullyChecked),
        }
        match &self.same_maker {
            SameMaker::Yes { .. } => {}
            SameMaker::No { .. } => why.push(Why::OtherMaker),
            SameMaker::Unknown { .. } => why.push(Why::MakerUnknown),
        }
        match self.arch {
            ArchFact::Matches => {}
            ArchFact::Wrong { .. } => why.push(Why::WrongArchitecture),
            ArchFact::Unknown => why.push(Why::ArchitectureUnknown),
        }
        if why.is_empty() {
            Decision::Install { bytes: self.bytes }
        } else {
            why.sort();
            why.dedup();
            Decision::NeedsAction { why }
        }
    }
}

/// Whether a file's bytes are the ones a decision approved: the installer step calls this on the
/// exact file it is about to run (T21-035).
pub fn approved_bytes_match(decision: &Decision, found: &Bytes) -> bool {
    matches!(decision, Decision::Install { bytes } if bytes == found)
}

/// Compares the maker found in the download with the old laptop's copy.
///
/// Windows: the same certificate, or a renewed one naming the same company in the same place
/// (common name, organisation and country must be there and equal; locality, state, registration
/// number and jurisdiction must be equal wherever both certificates carry them). A company name
/// alone, or a name in a different place, is not the same maker. This is only meaningful together
/// with a trusted signature, which [`Verdict::decide`] requires separately.
pub fn compare(old: &Maker, found: &Maker) -> SameMaker {
    match (old, found) {
        (Maker::Windows(a), Maker::Windows(b)) => compare_windows(a, b),
        (Maker::Mac(a), Maker::Mac(b)) => {
            if valid_team_id(&a.team_id) && a.team_id == b.team_id {
                SameMaker::Yes {
                    by: MatchedBy::SameTeam,
                }
            } else {
                SameMaker::No {
                    found: format!("Team ID {}", b.team_id),
                }
            }
        }
        (Maker::Linux(a), Maker::Linux(b)) => compare_linux(a, b),
        _ => SameMaker::Unknown {
            why: "the old copy and the download are for different systems".into(),
        },
    }
}

/// An Apple Team ID: ten capital letters or digits.
pub fn valid_team_id(id: &str) -> bool {
    id.len() == 10
        && id
            .bytes()
            .all(|b| b.is_ascii_uppercase() || b.is_ascii_digit())
}

fn compare_windows(old: &WindowsSigner, found: &WindowsSigner) -> SameMaker {
    if old.certificate == found.certificate {
        return SameMaker::Yes {
            by: MatchedBy::SameCertificate,
        };
    }
    let (a, b) = (&old.subject, &found.subject);
    let required = |x: &Option<String>, y: &Option<String>| match (x, y) {
        (Some(x), Some(y)) => !x.trim().is_empty() && x == y,
        _ => false,
    };
    let where_both = |x: &Option<String>, y: &Option<String>| match (x, y) {
        (Some(x), Some(y)) => x == y,
        _ => true,
    };
    let same = required(&a.common_name, &b.common_name)
        && required(&a.organization, &b.organization)
        && required(&a.country, &b.country)
        && where_both(&a.locality, &b.locality)
        && where_both(&a.state, &b.state)
        && where_both(&a.registration_number, &b.registration_number)
        && where_both(&a.jurisdiction_country, &b.jurisdiction_country)
        && where_both(&a.jurisdiction_state, &b.jurisdiction_state);
    if same {
        SameMaker::Yes {
            by: MatchedBy::SameCompany,
        }
    } else {
        SameMaker::No {
            found: describe_subject(b),
        }
    }
}

/// A short description of a certificate subject, for the report.
pub fn describe_subject(s: &Subject) -> String {
    let parts = [
        &s.common_name,
        &s.organization,
        &s.locality,
        &s.state,
        &s.country,
    ];
    let text: Vec<&str> = parts.iter().filter_map(|p| p.as_deref()).collect();
    if text.is_empty() {
        "a signer with no name".into()
    } else {
        text.join(", ")
    }
}

fn compare_linux(old: &LinuxSource, found: &LinuxSource) -> SameMaker {
    match (old, found) {
        (LinuxSource::Rpm { key: a }, LinuxSource::Rpm { key: b }) => {
            // rpm prints a short key ID (8 or 16 hex digits) or a full fingerprint. A short ID
            // only matches as the end of the other, and only because rpm itself already trusts
            // the key (the signature fact), which a look-alike short ID cannot fake.
            let (a, b) = (a.to_ascii_lowercase(), b.to_ascii_lowercase());
            let ok = is_hex_key(&a)
                && is_hex_key(&b)
                && (a == b || (a.len() > b.len() && a.ends_with(&b)) || b.ends_with(&a));
            if ok {
                SameMaker::Yes {
                    by: MatchedBy::SameKey,
                }
            } else {
                SameMaker::No {
                    found: format!("signing key {b}"),
                }
            }
        }
        (
            LinuxSource::Flatpak {
                remote_url: ua,
                app_id: ia,
                key: ka,
            },
            LinuxSource::Flatpak {
                remote_url: ub,
                app_id: ib,
                key: kb,
            },
        ) => {
            if ua == ub && ia == ib && ka == kb {
                SameMaker::Yes {
                    by: MatchedBy::SameKey,
                }
            } else {
                SameMaker::No {
                    found: format!("{ib} from {ub}"),
                }
            }
        }
        _ => SameMaker::No {
            found: "a different kind of package".into(),
        },
    }
}

/// A key ID or fingerprint: 8, 16 or 40 (or 64, for newer keys) hex digits.
fn is_hex_key(k: &str) -> bool {
    matches!(k.len(), 8 | 16 | 40 | 64) && k.bytes().all(|b| b.is_ascii_hexdigit())
}
