//! Setup recipes (engineering notes, building block 1): a closed set of typed steps, paths that
//! stay inside the app's settings, and the shared format rule.

use pctwin_recipe::{
    Arch, Catalog, Check, Os, Platform, Recipe, RecipeError, RecipeId, RelPath, SettingValue,
    SettingsPlace, SettingsRoot, Step,
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
fn a_recipe_that_changes_something_must_check_it() {
    let r = Recipe::new(
        RecipeId::new("x").unwrap(),
        1,
        vec![win()],
        vec![Step::Install {
            catalog: Catalog::Winget,
            package: "X".into(),
        }],
    );
    assert_eq!(r, Err(RecipeError::NoVerify));
    // A recipe that only hands the person a checklist changes nothing, so needs no check.
    assert!(
        Recipe::new(
            RecipeId::new("x").unwrap(),
            1,
            vec![win()],
            vec![Step::Checklist {
                what: "Sign in to the app".into()
            }],
        )
        .is_ok()
    );
}

#[test]
fn empty_or_versionless_recipes_are_refused() {
    let id = || RecipeId::new("x").unwrap();
    let check = || vec![Step::Checklist { what: "x".into() }];
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
