//! The broker's fixed actions, each with its own schema, and the approved plan they belong to.
//!
//! There are five actions and no others: [`InstallVerifiedPackage`], [`InstallFromMacAppStore`],
//! [`ApplyNamedSetting`], [`CreateRestorePoint`] and [`UninstallWindowsInstallerProduct`]. None
//! takes a command, a path, a program, an environment value or a web address; every field is a
//! typed, bounded choice. A permission for one is never read as
//! another: each action names the one role that may ask for it.

use serde::{Deserialize, Deserializer, Serialize, Serializer};

use pctwin_grant::Audience;

/// Most operations one approved plan may hold.
pub const MAX_OPERATIONS: usize = 256;
/// Longest action or request ID, in bytes.
pub const MAX_ID: usize = 64;
/// Longest time zone name, in bytes.
pub const MAX_TIME_ZONE: usize = 64;

/// 32 bytes written as 64 lowercase hexadecimal characters (a digest, a key).
#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Default)]
pub struct Hex32(pub [u8; 32]);

/// 16 bytes written as 32 lowercase hexadecimal characters (a request's one-use nonce).
#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Default)]
pub struct Hex16(pub [u8; 16]);

fn to_hex(bytes: &[u8]) -> String {
    const DIGITS: &[u8; 16] = b"0123456789abcdef";
    let mut s = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        s.push(DIGITS[usize::from(b >> 4)] as char);
        s.push(DIGITS[usize::from(b & 15)] as char);
    }
    s
}

fn from_hex<const N: usize>(s: &str) -> Option<[u8; N]> {
    let digit = |c: u8| match c {
        b'0'..=b'9' => Some(c - b'0'),
        b'a'..=b'f' => Some(c - b'a' + 10),
        _ => None,
    };
    let bytes = s.as_bytes();
    if bytes.len() != N * 2 {
        return None;
    }
    let mut out = [0u8; N];
    for (i, [high, low]) in bytes.as_chunks::<2>().0.iter().enumerate() {
        out[i] = (digit(*high)? << 4) | digit(*low)?;
    }
    Some(out)
}

macro_rules! hex_type {
    ($t:ident, $n:literal) => {
        impl $t {
            pub fn parse(s: &str) -> Option<Self> {
                from_hex::<$n>(s).map(Self)
            }
        }
        impl std::fmt::Display for $t {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.write_str(&to_hex(&self.0))
            }
        }
        impl std::fmt::Debug for $t {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(f, "{}({})", stringify!($t), to_hex(&self.0))
            }
        }
        impl Serialize for $t {
            fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
                s.serialize_str(&to_hex(&self.0))
            }
        }
        impl<'de> Deserialize<'de> for $t {
            fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
                let s = String::deserialize(d)?;
                Self::parse(&s).ok_or_else(|| {
                    serde::de::Error::custom(concat!(
                        "not ",
                        stringify!($n),
                        " bytes in lowercase hexadecimal"
                    ))
                })
            }
        }
    };
}

hex_type!(Hex32, 32);
hex_type!(Hex16, 16);

/// A short ID: 1 to [`MAX_ID`] bytes of `a-z`, `0-9`, `.`, `_` and `-`.
#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize)]
pub struct Id(String);

impl Id {
    pub fn parse(s: &str) -> Option<Self> {
        let ok = !s.is_empty()
            && s.len() <= MAX_ID
            && s.bytes().all(|b| {
                b.is_ascii_lowercase() || b.is_ascii_digit() || matches!(b, b'.' | b'_' | b'-')
            });
        ok.then(|| Self(s.to_string()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl<'de> Deserialize<'de> for Id {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let s = String::deserialize(d)?;
        Self::parse(&s).ok_or_else(|| serde::de::Error::custom("not a short ID"))
    }
}

/// The length of a Windows Installer product code: `{` 32 hexadecimal digits in groups of 8, 4,
/// 4, 4 and 12 joined by `-` `}`.
pub const PRODUCT_CODE_LEN: usize = 38;

/// A Windows Installer product code (a GUID in braces), the only thing the uninstall action
/// takes. Exactly [`PRODUCT_CODE_LEN`] bytes: `{`, hexadecimal digits in groups of 8, 4, 4, 4
/// and 12 joined by `-`, `}`; nothing else, not even a space. Kept in capitals, as Windows
/// Installer names its products in the system's list. The all-zero code names no product and is
/// refused.
#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ProductCode([u8; PRODUCT_CODE_LEN]);

impl ProductCode {
    pub fn parse(s: &str) -> Option<Self> {
        let b = s.as_bytes();
        if b.len() != PRODUCT_CODE_LEN || b[0] != b'{' || b[PRODUCT_CODE_LEN - 1] != b'}' {
            return None;
        }
        let mut out = [0u8; PRODUCT_CODE_LEN];
        out[0] = b'{';
        out[PRODUCT_CODE_LEN - 1] = b'}';
        let mut any = false;
        for (i, c) in b[1..PRODUCT_CODE_LEN - 1].iter().enumerate() {
            out[i + 1] = if matches!(i, 8 | 13 | 18 | 23) {
                if *c != b'-' {
                    return None;
                }
                b'-'
            } else {
                if !c.is_ascii_hexdigit() {
                    return None;
                }
                any |= *c != b'0';
                c.to_ascii_uppercase()
            };
        }
        any.then_some(Self(out))
    }

    pub fn as_str(&self) -> &str {
        // Only ASCII is ever stored (checked in `parse`).
        std::str::from_utf8(&self.0).unwrap_or("")
    }
}

impl std::fmt::Display for ProductCode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

impl std::fmt::Debug for ProductCode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "ProductCode({})", self.as_str())
    }
}

impl Serialize for ProductCode {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(self.as_str())
    }
}

impl<'de> Deserialize<'de> for ProductCode {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let s = String::deserialize(d)?;
        Self::parse(&s).ok_or_else(|| serde::de::Error::custom("not a product code"))
    }
}

/// The longest version kept for an installed product.
pub const MAX_VERSION: usize = 64;

/// An app's version as Windows' list of installed apps writes it (`DisplayVersion`): 1 to
/// [`MAX_VERSION`] bytes of letters, digits and `. - _ +`. Anything else is not kept (and then
/// nothing PCTwin installed can be removed by it: the version could not be checked later).
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize)]
pub struct ProductVersion(String);

impl ProductVersion {
    pub fn parse(s: &str) -> Option<Self> {
        let s = s.trim();
        let ok = !s.is_empty()
            && s.len() <= MAX_VERSION
            && s.bytes()
                .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'.' | b'-' | b'_' | b'+'));
        ok.then(|| Self(s.to_string()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl<'de> Deserialize<'de> for ProductVersion {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let s = String::deserialize(d)?;
        Self::parse(&s).ok_or_else(|| serde::de::Error::custom("not a version"))
    }
}

/// A Windows Installer product the helper saw its own install add: not in the system's list
/// before, there after, at this version.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct InstalledProduct {
    pub code: ProductCode,
    pub version: ProductVersion,
}

/// One of the fixed actions.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub enum Action {
    InstallVerifiedPackage(InstallVerifiedPackage),
    InstallFromMacAppStore(InstallFromMacAppStore),
    ApplyNamedSetting(ApplyNamedSetting),
    CreateRestorePoint(CreateRestorePoint),
    UninstallWindowsInstallerProduct(UninstallWindowsInstallerProduct),
}

/// Remove one Windows Installer product PCTwin installed (undo, decided 28 October 2026): Windows'
/// own `msiexec /x <code> /qn /norestart`, nothing else. It takes the product code and the one key
/// of the helper's own install that recorded it (from that install's receipt); the helper refuses
/// it unless its own journal holds that install, finished, recording this very product as absent
/// before and present after. It is removed only while the version installed is still there; a
/// different version is kept.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct UninstallWindowsInstallerProduct {
    pub product_code: ProductCode,
    pub installed_by: Hex32,
}

/// Install one exact package already staged in the helper's own administrators-only folder: by
/// its handle there and the SHA-256 of its bytes, checked again on the open file at the moment
/// the installer is started.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct InstallVerifiedPackage {
    pub artifact: u64,
    pub sha256: Hex32,
    pub kind: PackageKind,
    pub scope: InstallScope,
    /// The maker of the app on the old laptop, as the old laptop's own copy names it: the helper
    /// checks the package against it itself before installing ([`super::maker`]).
    pub maker: Box<pctwin_maker::facts::WindowsSigner>,
    /// For a Windows Installer package: its product code, as its manifest names it. The helper
    /// records it only when its own install made it appear in the system's list (absent before,
    /// present after, nothing else gone); only then can undo remove it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub product_code: Option<ProductCode>,
}

/// The kinds of package the compiled adapters know. Each adapter builds its own fixed arguments;
/// none comes from a recipe or a website. The kind comes from the package manifest's typed
/// installer type (winget's `InstallerType`), never from switches written as free text: a
/// manifest's own `InstallerSwitches` are never used.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum PackageKind {
    /// A Windows Installer package, installed by Windows' own `msiexec` for everyone.
    Msi,
    /// A program built by one of the installer makers below, run with that maker's own
    /// documented switches for a quiet install for everyone, never restarting the laptop.
    Exe(ExeKind),
}

/// Installer programs whose quiet, no-restart switches are documented by their makers. A
/// Nullsoft (NSIS) installer has no switch that stops it restarting the laptop, so it is not
/// one: it ends needs-action (T21-040).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ExeKind {
    /// Inno Setup.
    Inno,
    /// A WiX Burn bundle.
    Burn,
}

/// Install one app from the Mac App Store by its store ID, through the `mas` tool, which needs
/// root since its version 4. The person must be signed in to the App Store app with the Apple
/// Account that has the app; a store ID is a number and nothing else.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct InstallFromMacAppStore {
    pub adam_id: u64,
}

/// For whom a package is installed.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum InstallScope {
    /// For everyone on the laptop: needs an administrator.
    Machine,
    /// For the person moving only: needs no administrator, so the app does it itself, as them.
    User,
}

/// Apply one setting from a short fixed list, with a value of its own type.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct ApplyNamedSetting {
    pub setting: NamedSetting,
}

/// The settings the helper may apply, each with its own typed value. Security settings (the
/// administrator prompt, the firewall, antivirus exclusions, remote access, policies) are not on
/// this list and cannot be written in this schema; adding a setting is a reviewed code change.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    tag = "key",
    content = "value",
    rename_all = "kebab-case",
    deny_unknown_fields
)]
pub enum NamedSetting {
    /// The laptop's time zone, by the system's own name for it.
    TimeZone(TimeZoneName),
    /// Whether the laptop may hibernate (for everyone).
    Hibernate(bool),
}

/// A time zone's name as the system writes it ("W. Europe Standard Time", "Africa/Lagos"):
/// letters, digits, spaces and `_ / + - . ( )`, at most [`MAX_TIME_ZONE`] bytes. Nothing in it
/// can be read as a command or a path step (`..` is refused).
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct TimeZoneName(String);

impl TimeZoneName {
    pub fn parse(s: &str) -> Option<Self> {
        let ok = !s.is_empty()
            && s.len() <= MAX_TIME_ZONE
            && !s.contains("..")
            && !s.starts_with(['/', ' ', '-'])
            && s.bytes().all(|b| {
                b.is_ascii_alphanumeric()
                    || matches!(b, b' ' | b'_' | b'/' | b'+' | b'-' | b'.' | b'(' | b')')
            });
        ok.then(|| Self(s.to_string()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl<'de> Deserialize<'de> for TimeZoneName {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let s = String::deserialize(d)?;
        Self::parse(&s).ok_or_else(|| serde::de::Error::custom("not a time zone name"))
    }
}

/// Make a Windows restore point before the move's changes. It takes nothing: its description is
/// fixed, and it never changes how often Windows allows restore points.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CreateRestorePoint {}

impl Action {
    /// The one role that may ask for this action.
    pub fn audience(&self) -> Audience {
        match self {
            Action::InstallVerifiedPackage(_) | Action::InstallFromMacAppStore(_) => {
                Audience::Install
            }
            // Removing an app PCTwin installed undoes the install role's own work.
            Action::UninstallWindowsInstallerProduct(_) => Audience::Install,
            Action::ApplyNamedSetting(_) => Audience::Settings,
            Action::CreateRestorePoint(_) => Audience::Recovery,
        }
    }

    /// Whether it needs an administrator on this system. One that does not is done by the app
    /// itself with the person's own rights, never by the helper.
    pub fn needs_admin(&self) -> bool {
        match self {
            Action::InstallVerifiedPackage(p) => p.scope == InstallScope::Machine,
            // Only a Mac has the App Store; elsewhere the helper never runs it.
            Action::InstallFromMacAppStore(_) => cfg!(target_os = "macos"),
            // Windows lets every account change the time zone; macOS and Linux ask an
            // administrator.
            Action::ApplyNamedSetting(ApplyNamedSetting {
                setting: NamedSetting::TimeZone(_),
            }) => !cfg!(windows),
            Action::ApplyNamedSetting(ApplyNamedSetting {
                setting: NamedSetting::Hibernate(_),
            }) => true,
            Action::CreateRestorePoint(_) => true,
            // Only for products installed for everyone (the helper installs no other kind).
            Action::UninstallWindowsInstallerProduct(_) => true,
        }
    }

    /// What exactly this action is: a digest of its whole schema, so two actions are the same
    /// only when every field is.
    pub fn identity(&self) -> Hex32 {
        let mut h = blake3::Hasher::new_derive_key("PCTwin broker action 2026-10-26 v1");
        // Every field is a fixed-order struct field or enum: the JSON is the same every time.
        h.update(&serde_json::to_vec(self).unwrap_or_default());
        Hex32(*h.finalize().as_bytes())
    }
}

/// One operation of the approved plan: its ID and its exact action.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct PlannedOperation {
    pub action_id: Id,
    pub action: Action,
}

/// The plan the person approved, as the helper keeps it: its generation and its operations. Its
/// digest was given to the helper on its command line, through the system's own administrator
/// prompt, before the app could say anything; the plan the app sends later must match it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct ApprovedPlan {
    pub generation: u64,
    pub operations: Vec<PlannedOperation>,
}

impl ApprovedPlan {
    pub fn digest(&self) -> Hex32 {
        let mut h = blake3::Hasher::new_derive_key("PCTwin approved plan 2026-10-26 v1");
        h.update(&serde_json::to_vec(self).unwrap_or_default());
        Hex32(*h.finalize().as_bytes())
    }

    /// At least one operation, at most [`MAX_OPERATIONS`], each ID once.
    pub fn is_well_formed(&self) -> bool {
        let mut ids: Vec<&Id> = self.operations.iter().map(|o| &o.action_id).collect();
        ids.sort();
        ids.dedup();
        !self.operations.is_empty()
            && self.operations.len() <= MAX_OPERATIONS
            && ids.len() == self.operations.len()
    }

    pub fn operation(&self, id: &Id) -> Option<&PlannedOperation> {
        self.operations.iter().find(|o| &o.action_id == id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn maker() -> pctwin_maker::facts::WindowsSigner {
        pctwin_maker::facts::WindowsSigner {
            subject: pctwin_maker::facts::Subject {
                common_name: Some("Example Ltd".into()),
                organization: Some("Example Ltd".into()),
                country: Some("NG".into()),
                ..Default::default()
            },
            certificate: pctwin_maker::facts::Sha256([1; 32]),
            issuer: "CN=Some CA".into(),
            issuer_organization: Some("Some CA".into()),
            public_key: None,
        }
    }

    fn install(scope: InstallScope) -> Action {
        Action::InstallVerifiedPackage(InstallVerifiedPackage {
            product_code: None,
            artifact: 1,
            sha256: Hex32([0xab; 32]),
            kind: PackageKind::Msi,
            scope,
            maker: Box::new(maker()),
        })
    }

    #[test]
    fn hex_crosses_whole_and_refuses_anything_else() {
        let h = Hex32([0x0f; 32]);
        assert_eq!(h.to_string(), "0f".repeat(32));
        assert_eq!(Hex32::parse(&"0f".repeat(32)), Some(h));
        assert_eq!(Hex32::parse(&"0F".repeat(32)), None);
        assert_eq!(Hex32::parse(&"0f".repeat(31)), None);
        assert_eq!(Hex32::parse(&format!("{}g0", "0f".repeat(31))), None);
        assert_eq!(Hex16::parse(&"a9".repeat(16)), Some(Hex16([0xa9; 16])));
        assert_eq!(
            format!("{:?}", Hex16([1; 16])),
            format!("Hex16({})", "01".repeat(16))
        );
        let json = serde_json::to_string(&h).unwrap();
        assert_eq!(serde_json::from_str::<Hex32>(&json).unwrap(), h);
        assert!(serde_json::from_str::<Hex32>("\"zz\"").is_err());
    }

    const CODE: &str = "{A1B2C3D4-E5F6-4789-9ABC-DEF012345678}";

    /// The uninstall action's one input: a product code, braced, in hexadecimal, of the fixed
    /// length and shape; anything else is refused when read, before anything starts.
    #[test]
    fn a_product_code_is_a_braced_guid_and_nothing_else() {
        let good = ProductCode::parse(CODE).unwrap();
        assert_eq!(good.as_str(), CODE);
        assert_eq!(good.to_string(), CODE);
        assert_eq!(format!("{good:?}"), format!("ProductCode({CODE})"));
        // Small letters are the same code, kept in capitals as Windows writes it.
        let lower = ProductCode::parse(&CODE.to_ascii_lowercase()).unwrap();
        assert_eq!(lower, good);
        assert_eq!(lower.as_str(), CODE);
        assert!(ProductCode::parse("{00000000-0000-0000-0000-000000000001}").is_some());
        let long = format!("{CODE}}}");
        let mut table: Vec<String> = vec![
            "".into(),
            "{}".into(),
            "{00000000-0000-0000-0000-000000000000}".into(),
            "A1B2C3D4-E5F6-4789-9ABC-DEF012345678".into(),
            "{A1B2C3D4-E5F6-4789-9ABC-DEF012345678".into(),
            "A1B2C3D4-E5F6-4789-9ABC-DEF012345678}".into(),
            "(A1B2C3D4-E5F6-4789-9ABC-DEF012345678)".into(),
            "[A1B2C3D4-E5F6-4789-9ABC-DEF012345678]".into(),
            "{A1B2C3D4E5F647899ABCDEF012345678}".into(),
            "{A1B2C3D4-E5F6-4789-9ABC-DEF01234567}".into(),
            "{A1B2C3D4-E5F6-4789-9ABC-DEF0123456789}".into(),
            "{A1B2C3D4-E5F6-4789-9ABC-DEF01234567G}".into(),
            "{A1B2C3D4_E5F6-4789-9ABC-DEF012345678}".into(),
            "{A1B2C3D-4E5F6-4789-9ABC-DEF012345678}".into(),
            "{A1B2C3D4-E5F64-789-9ABC-DEF012345678}".into(),
            "{A1B2C3D4-E5F6-47899-ABC-DEF012345678}".into(),
            "{A1B2C3D4-E5F6-4789-9ABCD-EF012345678}".into(),
            " {A1B2C3D4-E5F6-4789-9ABC-DEF012345678}".into(),
            "{A1B2C3D4-E5F6-4789-9ABC-DEF012345678} ".into(),
            "{A1B2C3D4-E5F6-4789-9ABC-DEF01234567 }".into(),
            "{A1B2C3D4-E5F6-4789-9ABC-DEF01234567\0}".into(),
            "{A1B2C3D4-E5F6-4789-9ABC-DEF0123456\u{e9}}".into(),
            "{A1B2C3D4-E5F6-4789-9ABC-DEF012345678};calc".into(),
            "{A1B2C3D4-E5F6-4789-9ABC-DEF012345678} /i x.msi".into(),
            "/i {A1B2C3D4-E5F6-4789-9ABC-DEF012345678}".into(),
            "\"{A1B2C3D4-E5F6-4789-9ABC-DEF012345678}\"".into(),
            "{A1B2C3D4-E5F6-4789-9ABC-DEF01234567&}".into(),
            "{A1B2C3D4-E5F6-4789-9ABC-DEF01234567|}".into(),
            "{A1B2C3D4-E5F6-4789-9ABC-DEF01234567%}".into(),
            "{A1B2C3D4-E5F6-4789-9ABC-DEF0123456\\8}".into(),
            "{A1B2C3D4-E5F6-4789-9ABC-DEF0123456/8}".into(),
            "{-1B2C3D4-E5F6-4789-9ABC-DEF012345678}".into(),
            "{+1B2C3D4-E5F6-4789-9ABC-DEF012345678}".into(),
            // Only one brace wrong.
            "(A1B2C3D4-E5F6-4789-9ABC-DEF012345678}".into(),
            "{A1B2C3D4-E5F6-4789-9ABC-DEF012345678)".into(),
            "AA1B2C3D4-E5F6-4789-9ABC-DEF012345678}".into(),
            "{A1B2C3D4-E5F6-4789-9ABC-DEF0123456788".into(),
            "C:\\Windows\\x.msi".into(),
            long,
        ];
        // Every single byte swapped for each character that is not hexadecimal, at every
        // place (the braces too), is refused (and a dash where a digit belongs, and a digit
        // where a dash or a brace belongs).
        for at in 0..PRODUCT_CODE_LEN {
            for c in ['g', 'G', ' ', '-', 'x', '{', '}', '0'] {
                let mut s: Vec<char> = CODE.chars().collect();
                if s[at] == c || (c == '0' && s[at].is_ascii_hexdigit()) {
                    continue;
                }
                s[at] = c;
                table.push(s.into_iter().collect());
            }
        }
        for bad in &table {
            assert!(ProductCode::parse(bad).is_none(), "{bad:?}");
            let json = serde_json::to_string(bad).unwrap();
            assert!(
                serde_json::from_str::<ProductCode>(&json).is_err(),
                "{bad:?}"
            );
        }
        let json = serde_json::to_string(&good).unwrap();
        assert_eq!(json, format!("\"{CODE}\""));
        assert_eq!(serde_json::from_str::<ProductCode>(&json).unwrap(), good);
    }

    #[test]
    fn a_version_is_short_and_plain() {
        for good in [
            "1",
            "1.2.3",
            "24.08.1-beta+2",
            "1_0",
            &"9".repeat(MAX_VERSION),
        ] {
            assert_eq!(ProductVersion::parse(good).unwrap().as_str(), good);
        }
        assert_eq!(ProductVersion::parse(" 1.2 ").unwrap().as_str(), "1.2");
        for bad in [
            "",
            " ",
            "1 2",
            "1;2",
            "1/2",
            "1\\2",
            "1\"",
            &"9".repeat(MAX_VERSION + 1),
        ] {
            assert!(ProductVersion::parse(bad).is_none(), "{bad:?}");
            let json = serde_json::to_string(bad).unwrap();
            assert!(
                serde_json::from_str::<ProductVersion>(&json).is_err(),
                "{bad:?}"
            );
        }
    }

    /// The uninstall action takes a product code and the key of the install that recorded it,
    /// and nothing else: no path, no switch, no program.
    #[test]
    fn the_uninstall_action_takes_only_a_code_and_its_install() {
        let key = "cd".repeat(32);
        let good = format!(
            r#"{{"uninstall-windows-installer-product":{{"product-code":"{CODE}","installed-by":"{key}"}}}}"#
        );
        let a: Action = serde_json::from_str(&good).unwrap();
        assert_eq!(
            a,
            Action::UninstallWindowsInstallerProduct(UninstallWindowsInstallerProduct {
                product_code: ProductCode::parse(CODE).unwrap(),
                installed_by: Hex32([0xcd; 32]),
            })
        );
        assert_eq!(a.audience(), Audience::Install);
        assert!(a.needs_admin());
        for bad in [
            format!(
                r#"{{"uninstall-windows-installer-product":{{"product-code":"{CODE}","installed-by":"{key}","args":"/i x"}}}}"#
            ),
            format!(
                r#"{{"uninstall-windows-installer-product":{{"product-code":"{CODE}","installed-by":"{key}","path":"C:/x.msi"}}}}"#
            ),
            format!(r#"{{"uninstall-windows-installer-product":{{"product-code":"{CODE}"}}}}"#),
            format!(
                r#"{{"uninstall-windows-installer-product":{{"product-code":"{CODE} /i x","installed-by":"{key}"}}}}"#
            ),
            format!(
                r#"{{"uninstall-windows-installer-product":{{"product-code":"C:/x.msi","installed-by":"{key}"}}}}"#
            ),
            format!(
                r#"{{"uninstall-windows-installer-product":{{"product-code":"{CODE}","installed-by":"zz"}}}}"#
            ),
        ] {
            assert!(serde_json::from_str::<Action>(&bad).is_err(), "{bad}");
        }
        // A product code on an install is optional and, when given, as strict.
        let with_code = format!(
            r#"{{"install-verified-package":{{"artifact":1,"sha256":"{}","kind":"msi","scope":"machine","maker":{},"product-code":"not a code"}}}}"#,
            "ab".repeat(32),
            serde_json::to_string(&maker()).unwrap()
        );
        assert!(serde_json::from_str::<Action>(&with_code).is_err());
        // An install with none says nothing about it, so its identity is as before.
        let none = serde_json::to_string(&install(InstallScope::Machine)).unwrap();
        assert!(!none.contains("product-code"), "{none}");
    }

    #[test]
    fn ids_are_short_and_plain() {
        assert!(Id::parse("install.firefox-1_2").is_some());
        assert!(Id::parse(&"a".repeat(MAX_ID)).is_some());
        for bad in ["", "A", "a b", "a/b", "a;rm", &"a".repeat(MAX_ID + 1)] {
            assert!(Id::parse(bad).is_none(), "{bad}");
        }
        assert!(serde_json::from_str::<Id>("\"x$(id)\"").is_err());
    }

    /// T22-006 and T22-014: anything but the three fixed actions, or a field the schema does not
    /// name, or a command hidden in a value, is refused when read.
    #[test]
    fn only_the_fixed_schemas_are_read() {
        for bad in [
            r#"{"run-command":{"command":"cmd /c whoami"}}"#,
            r#"{"install-verified-package":{"artifact":1,"sha256":"00","kind":"msi","scope":"machine"}}"#,
            r#"{"install-verified-package":{"artifact":1,"sha256":"__","kind":"exe","scope":"machine"}}"#,
            r#"{"install-verified-package":{"path":"C:/x.msi","artifact":1,"kind":"msi","scope":"machine"}}"#,
            r#"{"apply-named-setting":{"setting":{"key":"uac-level","value":0}}}"#,
            r#"{"apply-named-setting":{"setting":{"key":"time-zone","value":"x; rm -rf /"}}}"#,
            r#"{"apply-named-setting":{"setting":{"key":"time-zone","value":"$(whoami)"}}}"#,
            r#"{"apply-named-setting":{"setting":{"key":"time-zone","value":"`calc`"}}}"#,
            r#"{"apply-named-setting":{"setting":{"key":"hibernate","value":"true"}}}"#,
            r#"{"apply-named-setting":{"setting":{"key":"hibernate","value":true,"extra":1}}}"#,
            r#"{"create-restore-point":{"description":"anything"}}"#,
        ] {
            assert!(serde_json::from_str::<Action>(bad).is_err(), "{bad}");
        }
        // No maker named: refused.
        let unnamed = format!(
            r#"{{"install-verified-package":{{"artifact":1,"sha256":"{}","kind":"msi","scope":"machine"}}}}"#,
            "ab".repeat(32)
        );
        assert!(serde_json::from_str::<Action>(&unnamed).is_err());
        let good = format!(
            r#"{{"install-verified-package":{{"artifact":1,"sha256":"{}","kind":"msi","scope":"machine","maker":{}}}}}"#,
            "ab".repeat(32),
            serde_json::to_string(&maker()).unwrap()
        );
        assert_eq!(
            serde_json::from_str::<Action>(&good).unwrap(),
            install(InstallScope::Machine)
        );
        let tz: Action = serde_json::from_str(
            r#"{"apply-named-setting":{"setting":{"key":"time-zone","value":"W. Europe Standard Time"}}}"#,
        )
        .unwrap();
        assert!(matches!(tz, Action::ApplyNamedSetting(_)));
        let rp: Action = serde_json::from_str(r#"{"create-restore-point":{}}"#).unwrap();
        assert_eq!(rp, Action::CreateRestorePoint(CreateRestorePoint {}));
    }

    #[test]
    fn time_zone_names_are_plain() {
        for good in [
            "UTC",
            "Africa/Lagos",
            "America/Argentina/Buenos_Aires",
            "Etc/GMT+1",
            "W. Europe Standard Time",
            "(UTC) Coordinated",
        ] {
            assert!(TimeZoneName::parse(good).is_some(), "{good}");
        }
        for bad in [
            "",
            "../../etc/passwd",
            "/etc/localtime",
            "-rf",
            " UTC",
            "a;b",
            "a|b",
            "a&b",
            "a\"b",
            "a\nb",
            &"a".repeat(MAX_TIME_ZONE + 1),
        ] {
            assert!(TimeZoneName::parse(bad).is_none(), "{bad:?}");
        }
        assert!(TimeZoneName::parse(&"a".repeat(MAX_TIME_ZONE)).is_some());
        assert_eq!(TimeZoneName::parse("UTC").unwrap().as_str(), "UTC");
    }

    /// T22-007: each action belongs to one role; the move planner and the screens own none.
    #[test]
    fn each_action_belongs_to_one_role() {
        assert_eq!(install(InstallScope::Machine).audience(), Audience::Install);
        let s = Action::ApplyNamedSetting(ApplyNamedSetting {
            setting: NamedSetting::Hibernate(false),
        });
        assert_eq!(s.audience(), Audience::Settings);
        assert_eq!(
            Action::CreateRestorePoint(CreateRestorePoint {}).audience(),
            Audience::Recovery
        );
    }

    /// T22-019: an install for the person only, or a time zone on Windows, needs no
    /// administrator.
    #[test]
    fn what_needs_an_administrator() {
        assert!(install(InstallScope::Machine).needs_admin());
        assert!(!install(InstallScope::User).needs_admin());
        let tz = Action::ApplyNamedSetting(ApplyNamedSetting {
            setting: NamedSetting::TimeZone(TimeZoneName::parse("UTC").unwrap()),
        });
        assert_eq!(tz.needs_admin(), !cfg!(windows));
        let hib = Action::ApplyNamedSetting(ApplyNamedSetting {
            setting: NamedSetting::Hibernate(true),
        });
        assert!(hib.needs_admin());
        assert!(Action::CreateRestorePoint(CreateRestorePoint {}).needs_admin());
    }

    #[test]
    fn an_action_is_the_same_only_when_every_field_is() {
        let a = install(InstallScope::Machine);
        assert_eq!(a.identity(), install(InstallScope::Machine).identity());
        assert_ne!(a.identity(), install(InstallScope::User).identity());
        let mut other = InstallVerifiedPackage {
            product_code: None,
            artifact: 1,
            sha256: Hex32([0xab; 32]),
            kind: PackageKind::Msi,
            scope: InstallScope::Machine,
            maker: Box::new(maker()),
        };
        other.sha256.0[31] ^= 1;
        assert_ne!(
            a.identity(),
            Action::InstallVerifiedPackage(other).identity()
        );
    }

    #[test]
    fn a_plan_has_operations_each_named_once() {
        let op = |id: &str| PlannedOperation {
            action_id: Id::parse(id).unwrap(),
            action: install(InstallScope::Machine),
        };
        let plan = ApprovedPlan {
            generation: 2,
            operations: vec![op("a"), op("b")],
        };
        assert!(plan.is_well_formed());
        assert!(plan.operation(&Id::parse("b").unwrap()).is_some());
        assert!(plan.operation(&Id::parse("c").unwrap()).is_none());
        let twice = ApprovedPlan {
            generation: 2,
            operations: vec![op("a"), op("a")],
        };
        assert!(!twice.is_well_formed());
        let empty = ApprovedPlan {
            generation: 2,
            operations: vec![],
        };
        assert!(!empty.is_well_formed());
        let most = ApprovedPlan {
            generation: 2,
            operations: (0..MAX_OPERATIONS).map(|i| op(&format!("o{i}"))).collect(),
        };
        assert!(most.is_well_formed());
        let mut too_many = most.clone();
        too_many.operations.push(op("extra"));
        assert!(!too_many.is_well_formed());
        // Any change to the plan changes its digest.
        let mut changed = plan.clone();
        changed.generation = 3;
        assert_ne!(plan.digest(), changed.digest());
        assert_eq!(plan.digest(), plan.clone().digest());
    }
}
