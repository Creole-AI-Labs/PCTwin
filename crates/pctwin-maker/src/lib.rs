//! The maker check (Task List 2.1): does a downloaded or found installer come from the same maker
//! as the app on the old laptop?
//!
//! The old laptop's own copy is the anchor (Phase 2 adoption, change 3): its maker signature
//! (Windows signer, Apple Team ID, Linux signing key) names the maker, whatever website the
//! download came from. A download is installed only if it carries that maker's signature, the
//! system trusts it, and the system's own malware check finished without a finding (change 4:
//! Microsoft Defender on Windows; on a Mac, Gatekeeper and Apple's notarisation; on Linux, the
//! package manager's signature).
//!
//! The answer is a set of separate facts ([`Verdict`]), never a score, and one decision
//! ([`Verdict::decide`]). The checker never runs the file, bounds every read and every tool, and
//! binds its answer to the SHA-256 of the exact bytes it checked, which the installer step must
//! find again ([`approved_bytes_match`]).

pub mod check;
pub mod facts;
pub mod linux;
pub mod mac;
pub(crate) mod msi;
pub mod native;
pub mod oracle;
pub mod own;
pub(crate) mod pe;
pub mod run;
pub(crate) mod signer;
pub mod sniff;
pub mod windows;

pub use check::{
    CheckError, Expected, HelperStaged, Limits, NOT_CHECKED_BY_READER, OldCopy, OldMaker,
    SystemChecks, SystemSignature, check, check_staged, windows_maker_of, windows_old_copy,
};
pub use facts::{
    Arch, ArchFact, ArchSet, Bytes, Coverage, Decision, Format, FormatFact, LinuxSource, MacTeam,
    Maker, Malware, MatchedBy, SameMaker, Sha256, Signature, Steady, Subject, System, Verdict, Why,
    WindowsSigner, approved_bytes_match, compare,
};
pub use native::NativeChecks;
pub use oracle::{NoOracle, TrustOracle};
pub use own::{InThisProcess, InstallerRead, OwnRead, OwnReader};
