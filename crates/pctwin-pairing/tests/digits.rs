//! Language follow-up (Task List 1.3): a person types the pairing code, or reads back a number,
//! in the digits their keyboard and language give them. Every Unicode decimal digit counts as
//! its value (full-width, Arabic-Indic, Persian, Devanagari, Bengali, Thai and the rest), and
//! spaces and dashes of every kind are only separators. Anything else, including superscripts,
//! fractions and Roman numerals, is not a digit and the code is refused.

use pctwin_pairing::{PairingCode, PairingError, read_digits};

fn code(typed: &str) -> Result<String, PairingError> {
    PairingCode::parse(typed).map(|c| c.digits())
}

#[test]
fn every_digit_system_reads_as_its_value() {
    for typed in [
        "123456",
        "１２３４５６", // full-width
        "١٢٣٤٥٦",       // Arabic-Indic
        "۱۲۳۴۵۶",       // Persian and Urdu
        "१२३४५६",       // Devanagari
        "১২৩৪৫৬",       // Bengali
        "๑๒๓๔๕๖",       // Thai
        "12٣4५6",       // mixed, digit by digit
    ] {
        assert_eq!(code(typed).unwrap(), "123456", "{typed}");
    }
}

#[test]
fn spaces_and_dashes_of_every_kind_are_only_separators() {
    for typed in [
        "123 456",
        "123-456",
        "123\u{00A0}456", // no-break space
        "123\u{2009}456", // thin space
        "123\u{2013}456", // en dash
        "123\u{2212}456", // minus sign
        "１２３－４５６", // full-width hyphen
        " 1 2 3 4 5 6 ",
    ] {
        assert_eq!(code(typed).unwrap(), "123456", "{typed:?}");
    }
}

#[test]
fn anything_that_is_not_a_digit_is_refused() {
    for typed in [
        "12345²", // superscript two
        "12345½", // a fraction
        "1234Ⅻ",  // a Roman numeral
        "12345a",
        "12345:", // the character right after 9
        "12345٪", // the character right after Arabic-Indic 9
        "12345/", // the character right before 0
        "12345.6",
        "1234567",
        "١٢٣٤٥", // five digits
        "",
        "------",
    ] {
        assert_eq!(code(typed), Err(PairingError::InvalidCode), "{typed:?}");
    }
}

#[test]
fn a_typed_number_reads_the_same_way() {
    assert_eq!(
        read_digits("٤٢").map(|d| d.to_string()),
        Some("42".to_string())
    );
    assert_eq!(
        read_digits("４２").map(|d| d.to_string()),
        Some("42".to_string())
    );
    assert_eq!(
        read_digits("4-2").map(|d| d.to_string()),
        Some("42".to_string())
    );
    assert!(read_digits("4x2").is_none());
    assert!(read_digits("²").is_none());
}

/// The digit table is exactly Unicode 15.1's: every one of its 680 decimal digits reads as its
/// value, and no other character of the whole Unicode range reads as a digit (separators aside).
#[test]
fn every_unicode_digit_and_nothing_else_reads_as_a_digit() {
    let listed: std::collections::BTreeMap<u32, u8> = include_str!("data/unicode-digits.txt")
        .lines()
        .filter(|l| !l.starts_with('#') && !l.is_empty())
        .map(|l| {
            let (cp, value) = l.split_once(' ').unwrap();
            (u32::from_str_radix(cp, 16).unwrap(), value.parse().unwrap())
        })
        .collect();
    assert_eq!(listed.len(), 680);
    for cp in 0..=0x10FFFFu32 {
        let Some(c) = char::from_u32(cp) else {
            continue;
        };
        let read = read_digits(&c.to_string()).map(|d| d.to_string());
        match listed.get(&cp) {
            Some(value) => assert_eq!(read, Some(value.to_string()), "U+{cp:04X}"),
            // Not a digit: refused, or only a separator (read as nothing).
            None => assert!(
                matches!(read.as_deref(), None | Some("")),
                "U+{cp:04X} read as {read:?}"
            ),
        }
    }
}
