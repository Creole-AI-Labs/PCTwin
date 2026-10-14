//! The format rule (engineering notes 9.11): every saved format carries a version number, and
//! every reader checks it before anything else.
//!
//! - The version this app writes is read as [`Verdict::Current`].
//! - An older version this app still has a tested reader for is [`Verdict::Older`]; the caller
//!   reads or upgrades it.
//! - A higher version came from a newer PCTwin: it is refused with [`FormatError::Newer`] and a
//!   message to update, never read by guessing.
//! - Anything else (0, or an older version whose reader was retired) is
//!   [`FormatError::Unknown`].
//!
//! Each format's saved examples live in `fixtures/formats/` and are read by the current build in
//! CI, so a change of shape without a new number fails a test.

use serde::Deserialize;

/// One saved format: its name (for messages), the version this app writes, and the older versions
/// it can still read.
#[derive(Debug, Clone, Copy)]
pub struct Format {
    pub name: &'static str,
    pub current: u32,
    pub readable: &'static [u32],
}

/// What a found version means for this app.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Verdict {
    Current,
    Older(u32),
}

/// Why a saved file was refused before it was read.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum FormatError {
    #[error(
        "this {name} was made by a newer version of PCTwin (format {found}); update PCTwin to open it"
    )]
    Newer { name: &'static str, found: u32 },
    #[error("this {name} has a format this PCTwin cannot read ({found})")]
    Unknown { name: &'static str, found: u32 },
    #[error("this is not a PCTwin {name}: {detail}")]
    NotPctwin { name: &'static str, detail: String },
}

impl Format {
    /// Classifies a found version.
    pub fn check(&self, found: u32) -> Result<Verdict, FormatError> {
        if found == self.current {
            Ok(Verdict::Current)
        } else if found > self.current {
            Err(FormatError::Newer {
                name: self.name,
                found,
            })
        } else if found != 0 && self.readable.contains(&found) {
            Ok(Verdict::Older(found))
        } else {
            Err(FormatError::Unknown {
                name: self.name,
                found,
            })
        }
    }

    /// Reads only the top-level `format` number of a JSON file and classifies it, so a newer file
    /// is refused safely even when the rest of its shape has changed.
    pub fn check_json(&self, bytes: &[u8]) -> Result<Verdict, FormatError> {
        #[derive(Deserialize)]
        struct FormatOnly {
            format: u32,
        }
        let FormatOnly { format } =
            serde_json::from_slice(bytes).map_err(|e| FormatError::NotPctwin {
                name: self.name,
                detail: e.to_string(),
            })?;
        self.check(format)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const F: Format = Format {
        name: "test file",
        current: 3,
        readable: &[2],
    };

    #[test]
    fn current_and_readable_older_versions_are_read() {
        assert_eq!(F.check(3), Ok(Verdict::Current));
        assert_eq!(F.check(2), Ok(Verdict::Older(2)));
    }

    #[test]
    fn newer_versions_are_refused_with_a_message_to_update() {
        let err = F.check(4).unwrap_err();
        assert_eq!(
            err,
            FormatError::Newer {
                name: "test file",
                found: 4
            }
        );
        assert!(err.to_string().contains("update PCTwin"));
        assert!(F.check(u32::MAX).is_err());
    }

    #[test]
    fn zero_and_retired_versions_are_unknown() {
        for found in [0, 1] {
            assert_eq!(
                F.check(found),
                Err(FormatError::Unknown {
                    name: "test file",
                    found
                })
            );
        }
        // Even if a format lists 0 by mistake, 0 is never a real version.
        let odd = Format {
            readable: &[0],
            ..F
        };
        assert!(odd.check(0).is_err());
    }

    #[test]
    fn json_is_checked_by_its_format_number_alone() {
        assert_eq!(
            F.check_json(br#"{"format":3,"anything":[1,2]}"#),
            Ok(Verdict::Current)
        );
        // A newer file with a shape this app has never seen is still refused as newer.
        assert!(matches!(
            F.check_json(br#"{"format":9,"items":"new shape"}"#),
            Err(FormatError::Newer { found: 9, .. })
        ));
        for bad in [
            &b"{}"[..],
            b"not json",
            br#"{"format":-1}"#,
            br#"{"format":"3"}"#,
        ] {
            assert!(matches!(
                F.check_json(bad),
                Err(FormatError::NotPctwin { .. })
            ));
        }
    }
}
