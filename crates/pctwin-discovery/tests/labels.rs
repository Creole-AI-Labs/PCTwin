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
    // Any language works.
    for name in ["Ọlá's PC", "Ноутбук Маши", "李明的电脑", "لابتوب سارة"]
    {
        assert_eq!(Label::named(name).unwrap(), Label::Named(name.into()));
    }
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
        let back = Label::from_txt(|k| {
            txt.iter()
                .find(|(key, _)| *key == k)
                .map(|(_, v)| v.as_str())
        });
        assert_eq!(back, Some(label));
    }
}

#[test]
fn announcements_from_others_are_checked_like_anything_else_that_arrives() {
    let parse = |pairs: &[(&str, &str)]| {
        Label::from_txt(|k| pairs.iter().find(|(key, _)| *key == k).map(|(_, v)| *v))
    };
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
    ] {
        assert_eq!(parse(bad), None, "{bad:?}");
    }
    let too_long = "a".repeat(MAX_NAME_CHARS + 1);
    assert_eq!(parse(&[("v", "1"), ("n", too_long.as_str())]), None);
}
