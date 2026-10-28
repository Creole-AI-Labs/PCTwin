//! Get help (S11): a short problem code with no private data, to send by WhatsApp or any app to a
//! relative or a shop (Product Spec, "Help without email").
//!
//! The code holds six facts and nothing else: the code's format, the kind of problem, the step it
//! happened at, the PCTwin version and this laptop's system. The screens name the problem and the
//! step from closed lists, so no text of the person's (a name, a path, a network) can reach it. A
//! check byte catches a mistyped code. Twelve characters from Crockford's Base32 (no I, L, O or
//! U, so it reads aloud cleanly), in three groups of four.

use serde::{Deserialize, Serialize};

/// The code's format, so a later version can change what it holds.
const FORMAT: u8 = 1;
/// Crockford's Base32 alphabet.
const ALPHABET: &[u8; 32] = b"0123456789ABCDEFGHJKMNPQRSTVWXYZ";

pctwin_types::listed_enum! {
    /// The kind of problem, as the screen that offers help knows it.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
    #[serde(rename_all = "kebab-case")]
    pub enum HelpTopic {
        PairingFailed,
        SearchFailed,
        ConnectionLost,
        NotApproved,
        MoveFailed,
        MoveRefused,
        DriveProblem,
        LowBattery,
    }
}

pctwin_types::listed_enum! {
    /// How far the task got.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
    #[serde(rename_all = "kebab-case")]
    pub enum HelpStep {
        Pairing,
        Finding,
        Approving,
        Moving,
        Finished,
    }
}

impl HelpTopic {
    /// Every topic, in its code order. Never reordered: codes already sent must still read.
    pub const ALL: &'static [Self] = &[
        Self::PairingFailed,
        Self::SearchFailed,
        Self::ConnectionLost,
        Self::NotApproved,
        Self::MoveFailed,
        Self::MoveRefused,
        Self::DriveProblem,
        Self::LowBattery,
    ];
}

impl HelpStep {
    /// Every step, in its code order. Never reordered.
    pub const ALL: &'static [Self] = &[
        Self::Pairing,
        Self::Finding,
        Self::Approving,
        Self::Moving,
        Self::Finished,
    ];
}

/// The systems PCTwin runs on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum System {
    Windows,
    Mac,
    Linux,
    Other,
}

impl System {
    fn this() -> Self {
        if cfg!(windows) {
            Self::Windows
        } else if cfg!(target_os = "macos") {
            Self::Mac
        } else if cfg!(target_os = "linux") {
            Self::Linux
        } else {
            Self::Other
        }
    }

    fn number(self) -> u8 {
        match self {
            Self::Windows => 1,
            Self::Mac => 2,
            Self::Linux => 3,
            Self::Other => 0,
        }
    }

    #[cfg_attr(
        not(test),
        expect(dead_code, reason = "for reading a code back (see decode)")
    )]
    fn from_number(n: u8) -> Option<Self> {
        Some(match n {
            1 => Self::Windows,
            2 => Self::Mac,
            3 => Self::Linux,
            0 => Self::Other,
            _ => return None,
        })
    }
}

/// What a code says, read back (for whoever helps).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Said {
    pub topic: HelpTopic,
    pub step: HelpStep,
    pub version: (u8, u8, u8),
    pub system: System,
}

fn index<T: PartialEq + Copy>(all: &[T], value: T) -> u8 {
    all.iter()
        .position(|v| *v == value)
        .and_then(|i| u8::try_from(i).ok())
        .unwrap_or(u8::MAX)
}

/// The check byte: a CRC-8 (polynomial 0x07) over the six facts.
fn check(bytes: &[u8]) -> u8 {
    bytes.iter().fold(0u8, |mut crc, b| {
        crc ^= b;
        for _ in 0..8 {
            crc = if crc & 0x80 != 0 {
                (crc << 1) ^ 0x07
            } else {
                crc << 1
            };
        }
        crc
    })
}

/// A version as three numbers (a part that does not fit is 255): this build's own, as the
/// release set it (never the crate's placeholder, which said 0.1.0 on every build).
fn this_version(version: &str) -> (u8, u8, u8) {
    let mut parts = version
        .split(['.', '-', '+'])
        .map(|p| p.parse::<u8>().unwrap_or(u8::MAX));
    (
        parts.next().unwrap_or(0),
        parts.next().unwrap_or(0),
        parts.next().unwrap_or(0),
    )
}

/// The code for `said`.
pub fn encode(said: &Said) -> String {
    let mut bytes = vec![
        (FORMAT << 4) | said.system.number(),
        index(HelpTopic::ALL, said.topic),
        index(HelpStep::ALL, said.step),
        said.version.0,
        said.version.1,
        said.version.2,
    ];
    bytes.push(check(&bytes));
    // 56 bits, padded to 60, as twelve five-bit characters.
    let mut bits: u64 = 0;
    for b in &bytes {
        bits = (bits << 8) | u64::from(*b);
    }
    bits <<= 4;
    let chars: Vec<u8> = (0..12)
        .rev()
        .map(|i| ALPHABET[usize::try_from((bits >> (i * 5)) & 31).unwrap_or(0)])
        .collect();
    let text = String::from_utf8(chars).unwrap_or_default();
    format!("{}-{}-{}", &text[0..4], &text[4..8], &text[8..12])
}

/// What `code` says, or `None` when it is not a whole, correct code. Reads lower case, and the
/// letters people confuse (O for 0, I and L for 1), as Crockford's Base32 does.
#[cfg_attr(
    not(test),
    expect(
        dead_code,
        reason = "the rule whoever helps reads a code by; the app only makes codes"
    )
)]
pub fn decode(code: &str) -> Option<Said> {
    let mut bits: u64 = 0;
    let mut n = 0;
    for c in code.chars().filter(|c| *c != '-' && !c.is_whitespace()) {
        let c = match c.to_ascii_uppercase() {
            'O' => '0',
            'I' | 'L' => '1',
            c => c,
        };
        let v = ALPHABET.iter().position(|a| char::from(*a) == c)?;
        bits = (bits << 5) | u64::try_from(v).ok()?;
        n += 1;
    }
    if n != 12 || bits & 0xF != 0 {
        return None;
    }
    bits >>= 4;
    let bytes: Vec<u8> = (0..7)
        .rev()
        .map(|i| u8::try_from((bits >> (i * 8)) & 0xFF).unwrap_or(0))
        .collect();
    if check(&bytes[..6]) != bytes[6] || bytes[0] >> 4 != FORMAT {
        return None;
    }
    Some(Said {
        system: System::from_number(bytes[0] & 0xF)?,
        topic: *HelpTopic::ALL.get(usize::from(bytes[1]))?,
        step: *HelpStep::ALL.get(usize::from(bytes[2]))?,
        version: (bytes[3], bytes[4], bytes[5]),
    })
}

/// A problem code for the screens' Get help: the kind of problem and the step, with this
/// version of PCTwin and this laptop's system.
#[tauri::command]
pub fn help_code<R: tauri::Runtime>(
    app: tauri::AppHandle<R>,
    topic: HelpTopic,
    step: HelpStep,
) -> String {
    code_for(&app.package_info().version.to_string(), topic, step)
}

/// The code for a problem on the build `version`.
fn code_for(version: &str, topic: HelpTopic, step: HelpStep) -> String {
    encode(&Said {
        topic,
        step,
        version: this_version(version),
        system: System::this(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_code_order_lists_every_topic_and_step_once() {
        let names: Vec<&str> = HelpTopic::ALL.iter().map(HelpTopic::variant).collect();
        assert_eq!(names, HelpTopic::VARIANTS);
        let names: Vec<&str> = HelpStep::ALL.iter().map(HelpStep::variant).collect();
        assert_eq!(names, HelpStep::VARIANTS);
    }

    #[test]
    fn every_code_reads_back_as_it_was_made() {
        for topic in HelpTopic::ALL {
            for step in HelpStep::ALL {
                for system in [System::Windows, System::Mac, System::Linux, System::Other] {
                    let said = Said {
                        topic: *topic,
                        step: *step,
                        version: (1, 2, 255),
                        system,
                    };
                    let code = encode(&said);
                    assert_eq!(code.len(), 14, "{code}");
                    assert!(
                        code.chars()
                            .all(|c| c == '-' || ALPHABET.contains(&(c as u8)))
                    );
                    assert_eq!(decode(&code), Some(said), "{code}");
                    assert_eq!(decode(&code.to_lowercase()), Some(said));
                }
            }
        }
    }

    #[test]
    fn a_mistyped_code_is_refused() {
        let code = code_for("0.1.8", HelpTopic::ConnectionLost, HelpStep::Moving);
        assert!(decode(&code).is_some());
        let chars: Vec<char> = code.chars().collect();
        let mut caught = 0;
        let mut tried = 0;
        for i in (0..chars.len()).filter(|i| chars[*i] != '-') {
            for c in ALPHABET.iter().map(|a| char::from(*a)) {
                if c == chars[i] {
                    continue;
                }
                let mut wrong = chars.clone();
                wrong[i] = c;
                let wrong: String = wrong.into_iter().collect();
                tried += 1;
                if decode(&wrong).is_none() {
                    caught += 1;
                }
            }
        }
        // Every single-character slip is caught.
        assert_eq!(caught, tried);
        assert_eq!(decode("ABCD-EFGH"), None);
        assert_eq!(decode("UUUU-UUUU-UUUU"), None);
    }

    #[test]
    fn the_code_holds_only_the_six_facts() {
        // Every topic and step is from a closed list, and the version and system are the app's
        // own: there is no input of the person's for a code to carry.
        let said = decode(&code_for(
            "0.1.8",
            HelpTopic::DriveProblem,
            HelpStep::Moving,
        ))
        .unwrap();
        assert_eq!(said.topic, HelpTopic::DriveProblem);
        assert_eq!(said.step, HelpStep::Moving);
        assert_eq!(said.version, this_version("0.1.8"));
        assert_eq!(this_version("0.1.8"), (0, 1, 8));
        assert_eq!(said.system, System::this());
    }
}
