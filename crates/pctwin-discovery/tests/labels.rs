//! Session labels: how an old laptop is told apart from others on the same Wi-Fi.

use std::collections::HashSet;

use pctwin_discovery::{ANIMALS, COLOURS, DiscoveryError, Label, MAX_NAME_CHARS};

#[test]
fn there_are_twelve_colours_and_a_hundred_animals_all_different() {
    assert_eq!(COLOURS.len(), 12);
    assert_eq!(ANIMALS.len(), 100);
    assert_eq!(COLOURS.iter().collect::<HashSet<_>>().len(), 12);
    assert_eq!(ANIMALS.iter().collect::<HashSet<_>>().len(), 100);
    // Translation keys: lower-case ASCII words.
    for key in COLOURS.iter().chain(ANIMALS.iter()) {
        assert!(
            !key.is_empty() && key.bytes().all(|b| b.is_ascii_lowercase()),
            "{key}"
        );
    }
}

#[test]
fn random_labels_cover_every_colour_and_animal() {
    let mut colours = HashSet::new();
    let mut animals = HashSet::new();
    for _ in 0..20_000 {
        match Label::random().unwrap() {
            Label::Picked { colour, animal } => {
                assert!(usize::from(colour) < COLOURS.len());
                assert!(usize::from(animal) < ANIMALS.len());
                colours.insert(colour);
                animals.insert(animal);
            }
            Label::Named(_) => panic!("a random label is always picked from the lists"),
        }
    }
    assert_eq!(colours.len(), 12);
    assert_eq!(animals.len(), 100);
}

#[test]
fn every_one_of_the_1200_labels_can_come_up() {
    let mut seen = HashSet::new();
    for _ in 0..60_000 {
        seen.insert(Label::random().unwrap());
    }
    assert_eq!(seen.len(), 1200);
}

#[test]
fn a_picked_label_reads_as_colour_then_animal() {
    let label = Label::Picked {
        colour: COLOURS.iter().position(|c| *c == "blue").unwrap() as u8,
        animal: ANIMALS.iter().position(|a| *a == "fox").unwrap() as u8,
    };
    assert_eq!(label.keys(), Some(("blue", "fox")));
    assert_eq!(Label::named("Ada's laptop").unwrap().keys(), None);
}

#[test]
fn shuffle_never_picks_a_label_already_in_use_nearby() {
    let in_use: Vec<Label> = (0..1199u16)
        .map(|i| Label::Picked {
            colour: (i / 100) as u8,
            animal: (i % 100) as u8,
        })
        .collect();
    // Only one of the 1,200 labels is free.
    for _ in 0..20 {
        assert_eq!(
            Label::shuffle(&in_use).unwrap(),
            Label::Picked {
                colour: 11,
                animal: 99
            }
        );
    }
}

#[test]
fn shuffle_is_random_among_the_free_labels() {
    let in_use: Vec<Label> = (0..1198u16)
        .map(|i| Label::Picked {
            colour: (i / 100) as u8,
            animal: (i % 100) as u8,
        })
        .collect();
    let mut seen = HashSet::new();
    for _ in 0..200 {
        seen.insert(Label::shuffle(&in_use).unwrap());
    }
    assert_eq!(seen.len(), 2, "both free labels come up");
}

#[test]
fn shuffle_still_works_when_every_label_is_taken() {
    let all: Vec<Label> = (0..1200u16)
        .map(|i| Label::Picked {
            colour: (i / 100) as u8,
            animal: (i % 100) as u8,
        })
        .collect();
    assert!(Label::shuffle(&all).is_ok());
}

#[test]
fn shuffle_gives_a_new_label_each_time_it_is_pressed() {
    let mut current = Label::random().unwrap();
    for _ in 0..200 {
        let next = Label::shuffle(std::slice::from_ref(&current)).unwrap();
        assert_ne!(next, current);
        current = next;
    }
}

#[test]
fn a_typed_name_is_trimmed_and_kept_as_typed() {
    assert_eq!(
        Label::named("  Ada's laptop  ").unwrap(),
        Label::Named("Ada's laptop".into())
    );
    // Any language works, including scripts that build letters from marks.
    for name in [
        "Ọlá's PC",
        "Ноутбук Маши",
        "李明的电脑",
        "لابتوب سارة",
        "नमस्ते",
        "Nguyễn's laptop",
        "Zoë (office)",
    ] {
        assert_eq!(Label::named(name).unwrap(), Label::Named(name.into()));
    }
    // Spaces inside are tidied, and accents are stored in one standard form.
    assert_eq!(
        Label::named("Ada   old \t laptop").unwrap(),
        Label::Named("Ada old laptop".into())
    );
    assert_eq!(
        Label::named("Cafe\u{301}").unwrap(),
        Label::Named("Caf\u{e9}".into())
    );
    // 32 characters in any script.
    assert!(Label::named(&"李".repeat(MAX_NAME_CHARS)).is_ok());
    assert!(Label::named(&"李".repeat(MAX_NAME_CHARS + 1)).is_err());
}

#[test]
fn names_cannot_be_invisible_or_disguised() {
    for bad in [
        "\u{3164}", // Hangul filler: looks blank
        "\u{00AD}", // soft hyphen
        "\u{115F}",
        "\u{FFA0}",
        "\u{2800}",       // blank braille pattern
        "Ada\u{061C}",    // Arabic letter mark (text direction)
        "Ada\u{2028}Bob", // line separator
        "Ada\u{2029}",    // paragraph separator
        "Ada\u{180E}",
        "A\u{034F}da",  // combining grapheme joiner
        "Ada\u{FE0F}",  // variation selector
        "Ada\u{FFF9}",  // interlinear annotation
        "Ada\u{E0041}", // tag character (hidden text)
        "Ada\u{E000}",  // private use
        "Ada\u{0378}",  // unassigned
        "Ada\u{FFFF}",  // noncharacter
        "Ada\u{FEFF}",  // byte order mark
        "Ada\u{85}",    // next line (a control character)
        "Ada 🦊",       // pictures are not names
        "\u{301}Ada",   // starts with a mark
        "...",          // nothing to read
    ] {
        assert!(
            matches!(Label::named(bad), Err(DiscoveryError::InvalidName)),
            "{bad:?}"
        );
    }
    // A pile of accents on one letter.
    let flood = format!("a{}", "\u{301}".repeat(31));
    assert!(Label::named(&flood).is_err());
}

#[test]
fn names_that_differ_only_in_case_or_accent_form_count_as_the_same() {
    let a = Label::named("Twin laptop").unwrap();
    let b = Label::named("TWIN LAPTOP").unwrap();
    let c = Label::named("Café").unwrap();
    let d = Label::named("Cafe\u{301}").unwrap();
    assert_eq!(a.comparison_key(), b.comparison_key());
    assert_eq!(c.comparison_key(), d.comparison_key());
    assert_ne!(a.comparison_key(), c.comparison_key());
}

#[test]
fn a_typed_name_must_be_short_and_plain() {
    assert!(matches!(Label::named(""), Err(DiscoveryError::InvalidName)));
    assert!(matches!(
        Label::named("   "),
        Err(DiscoveryError::InvalidName)
    ));
    let longest = "a".repeat(MAX_NAME_CHARS);
    assert!(Label::named(&longest).is_ok());
    assert!(matches!(
        Label::named(&format!("{longest}a")),
        Err(DiscoveryError::InvalidName)
    ));
    // Control characters and text-direction tricks that could make one name look like another.
    for bad in [
        "Ada\nBob",
        "Ada\u{0}",
        "Ada\u{202E}xof",
        "\u{2066}Blue Fox",
        "Ada\u{200F}",
    ] {
        assert!(
            matches!(Label::named(bad), Err(DiscoveryError::InvalidName)),
            "{bad:?}"
        );
    }
}

#[test]
fn labels_survive_the_trip_through_an_announcement() {
    for label in [
        Label::random().unwrap(),
        Label::named("Ada's laptop").unwrap(),
        Label::named("李明的电脑").unwrap(),
    ] {
        let txt = label.to_txt();
        let fields: Vec<(&str, &str)> = txt.iter().map(|(k, v)| (*k, v.as_str())).collect();
        let back = Label::from_txt(&fields);
        assert_eq!(back, Some(label));
    }
}

#[test]
fn announcements_from_others_are_checked_like_anything_else_that_arrives() {
    let parse = |pairs: &[(&str, &str)]| Label::from_txt(pairs);
    assert_eq!(
        parse(&[("v", "1"), ("c", "5"), ("a", "27")]),
        Some(Label::Picked {
            colour: 5,
            animal: 27
        })
    );
    for bad in [
        &[("v", "2"), ("c", "5"), ("a", "27")][..], // unknown version
        &[("c", "5"), ("a", "27")],                 // no version
        &[("v", "1"), ("c", "12"), ("a", "27")],    // colour out of range
        &[("v", "1"), ("c", "5"), ("a", "100")],    // animal out of range
        &[("v", "1"), ("c", "-1"), ("a", "3")],     // not a number
        &[("v", "1"), ("c", "05"), ("a", "3")],     // not canonical
        &[("v", "1"), ("c", "5")],                  // half a label
        &[("v", "1"), ("n", "")],                   // empty name
        &[("v", "1"), ("n", "Ada\u{202E}")],        // direction trick
        &[("v", "1"), ("c", "5"), ("a", "3"), ("n", "Ada")], // both kinds at once
        &[("v", "1"), ("c", "5"), ("a", "3"), ("x", "1")], // a field PCTwin never writes
        &[("v", "1"), ("v", "1"), ("c", "5"), ("a", "3")], // a field twice
        &[("v", "1"), ("n", " Ada")],               // not tidied
        &[("v", "1"), ("n", "Ada  laptop")],        // not tidied
        &[("v", "1"), ("n", "Cafe\u{301}")],        // not in the standard accent form
        &[("v", "1"), ("n", "\u{3164}")],           // looks blank
    ] {
        assert_eq!(parse(bad), None, "{bad:?}");
    }
    let too_long = "a".repeat(MAX_NAME_CHARS + 1);
    assert_eq!(parse(&[("v", "1"), ("n", too_long.as_str())]), None);
}

// ---------- real names people type, in many languages ----------

#[test]
fn everyday_punctuation_from_many_languages_is_accepted() {
    for name in [
        "艾伦·图灵", // Chinese middle dot in a name
        "买买提·艾力",
        "张伟的电脑（旧）", // full-width brackets, as Chinese keyboards type them
        "我的电脑，旧的",
        "家里、办公室",
        "Bureau n°2",
        "« Bureau »",
        "Salon – PC",
        "¡Mi PC!",
        "¿Dónde?",
        "José/María",
        "حاسوب سارة، القديم", // Arabic comma
        "هل هذا؟",            // Arabic question mark
        "Home/Office PC",
        "MacBook Pro 15\"",
        "“Ada’s” laptop",
        "Ada — work",
        "Old one…",
        "[Office]",
        "PC; old",
        "PC*",
        "डॉ॰ शर्मा",
        "ジョン・スミス", // Japanese middle dot
        "བསྟན་འཛིན",        // Tibetan syllable mark (Tenzin)
    ] {
        assert_eq!(
            Label::named(name).unwrap(),
            Label::Named(name.into()),
            "{name}"
        );
    }
}

#[test]
fn scripts_that_need_joiners_or_stacked_marks_are_accepted() {
    for name in [
        "لپ‌تاپ", // Persian: zero-width non-joiner between letters
        "ශ්‍රී",    // Sinhala: zero-width joiner
        "ကျော်",  // Burmese: four marks on one letter
        "में",     // Hindi: two marks in a row
        "हैं",
        "مُحَمَّد", // Arabic: shadda with a vowel mark
    ] {
        assert_eq!(
            Label::named(name).unwrap(),
            Label::Named(name.into()),
            "{name:?}"
        );
    }
}

#[test]
fn joiners_are_only_allowed_between_letters() {
    for bad in [
        "\u{200C}Ada",
        "Ada\u{200C}",
        "A\u{200C}\u{200C}da",
        "Ada \u{200C}x",
        "Ada\u{200D} x",
        "Ada.\u{200D}x",
        "\u{200D}",
    ] {
        assert!(Label::named(bad).is_err(), "{bad:?}");
    }
}

#[test]
fn marks_must_sit_on_a_letter_and_cannot_pile_up() {
    for bad in [
        "Ada \u{301}x",
        "Ada.\u{301}",
        "Ada,\u{301}",
        "a\u{301}\u{301}\u{301}\u{301}\u{301}\u{301}",
    ] {
        assert!(Label::named(bad).is_err(), "{bad:?}");
    }
}

#[test]
fn more_invisible_characters_are_refused() {
    for bad in [
        "Ada\u{180B}",
        "Ada\u{180D}",
        "Ada\u{E0100}",
        "Ada\u{1160}",
        "Ada\u{17B4}",
        "Ada\u{17B5}",
        "Ada\u{A0}laptop", // no-break space looks like a space but is not one
        "Ada\u{2007}laptop",
    ] {
        assert!(Label::named(bad).is_err(), "{bad:?}");
    }
    // Arriving from the network, a trailing space is not tidy and is refused.
    assert_eq!(Label::from_txt(&[("v", "1"), ("n", "Ada ")]), None);
}

#[test]
fn the_length_limit_counts_letters_people_see() {
    // About 20 visible letters, 36 code points: fits.
    let hindi = "श्रीमती प्रियंका चतुर्वेदी का लैपटॉप";
    assert!(hindi.chars().count() > MAX_NAME_CHARS);
    assert!(Label::named(hindi).is_ok());
    assert!(Label::named(&"a".repeat(MAX_NAME_CHARS)).is_ok());
    assert!(Label::named(&"a".repeat(MAX_NAME_CHARS + 1)).is_err());
}

#[test]
fn twins_are_recognised_through_width_and_joiners() {
    let key = |s: &str| Label::named(s).unwrap().comparison_key();
    assert_eq!(key("ＡＤＡ ＰＣ"), key("ada pc"));
    assert_eq!(key("ﬁle"), key("file"));
    assert_eq!(key("لپ‌تاپ"), key("لپتاپ"));
}

#[test]
fn every_colour_and_animal_label_has_its_own_comparison_key() {
    let keys: HashSet<String> = (0..1200u16)
        .map(|i| {
            Label::Picked {
                colour: (i / 100) as u8,
                animal: (i % 100) as u8,
            }
            .comparison_key()
        })
        .collect();
    assert_eq!(keys.len(), 1200);
}

#[test]
fn a_name_that_looks_short_but_is_huge_underneath_is_refused() {
    // 32 visible letters, each carrying four accent marks: 288 bytes, too big for one field.
    let heavy = "x\u{301}\u{301}\u{301}\u{301}".repeat(MAX_NAME_CHARS);
    assert!(heavy.len() > 255);
    assert!(Label::named(&heavy).is_err());
    // The same with fewer letters fits.
    assert!(Label::named(&"x\u{301}\u{301}\u{301}\u{301}".repeat(20)).is_ok());
}
