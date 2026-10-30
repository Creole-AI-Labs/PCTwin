//! What a recipe says about undoing its app (System Design S14, "Apps, imports and source
//! policy"; S02-P06): removing an app is not a universal inverse of installing it, so each recipe
//! declares whether removal through its catalog is a safe undo and what else removal touches.
//! A recipe that says nothing (every format-1 recipe) is never removed by undo.

use std::path::Path;

use pctwin_recipe::{
    Arch, Catalog, Os, Platform, Recipe, RecipeError, RecipeId, SideEffect, Step, UndoDeclaration,
};

fn win() -> Platform {
    Platform {
        os: Os::Windows,
        arch: Arch::X86_64,
    }
}

fn app(undo: UndoDeclaration) -> Result<Recipe, RecipeError> {
    Recipe::new(
        RecipeId::new("org.videolan.vlc").unwrap(),
        1,
        vec![win()],
        vec![Step::Install {
            catalog: Catalog::Winget,
            package: "VideoLAN.VLC".into(),
        }],
    )?
    .with_undo(undo)
}

#[test]
fn a_new_recipe_says_nothing_about_undo_so_undo_never_removes_its_app() {
    let r = Recipe::new(
        RecipeId::new("org.videolan.vlc").unwrap(),
        1,
        vec![win()],
        vec![Step::Install {
            catalog: Catalog::Winget,
            package: "VideoLAN.VLC".into(),
        }],
    )
    .unwrap();
    assert_eq!(r.undo, UndoDeclaration::default());
    assert!(!r.undo.uninstall);
    assert!(!r.undo.removable());
}

#[test]
fn removal_is_safe_only_when_declared_and_it_touches_no_user_data_or_shared_parts() {
    let plain = UndoDeclaration {
        uninstall: true,
        side_effects: vec![],
    };
    assert!(plain.removable());
    for blocking in [SideEffect::RemovesUserData, SideEffect::SharedParts] {
        let d = UndoDeclaration {
            uninstall: true,
            side_effects: vec![blocking],
        };
        assert!(!d.removable(), "{blocking:?} must stop removal");
        assert!(d.why_kept().is_some());
    }
    for told in [SideEffect::VendorAccount, SideEffect::LeavesSettings] {
        let d = UndoDeclaration {
            uninstall: true,
            side_effects: vec![told],
        };
        assert!(d.removable(), "{told:?} is told, not blocking");
        assert_eq!(d.why_kept(), None);
    }
    let not = UndoDeclaration {
        uninstall: false,
        side_effects: vec![],
    };
    assert!(!not.removable());
    assert!(not.why_kept().is_some());
}

#[test]
fn reasons_for_keeping_an_app_are_plain_words() {
    for d in [
        UndoDeclaration::default(),
        UndoDeclaration {
            uninstall: true,
            side_effects: vec![SideEffect::RemovesUserData],
        },
        UndoDeclaration {
            uninstall: true,
            side_effects: vec![SideEffect::SharedParts],
        },
    ] {
        let why = d.why_kept().unwrap();
        assert!(!why.is_empty());
        for jargon in ["purge", "zap", "catalog", "uninstall", "recipe"] {
            assert!(!why.to_lowercase().contains(jargon), "{why}");
        }
    }
}

#[test]
fn a_declaration_round_trips_and_a_repeated_side_effect_is_refused() {
    let r = app(UndoDeclaration {
        uninstall: true,
        side_effects: vec![SideEffect::VendorAccount, SideEffect::LeavesSettings],
    })
    .unwrap();
    assert_eq!(from_json(&r.to_json()).unwrap(), r);
    assert_eq!(
        app(UndoDeclaration {
            uninstall: true,
            side_effects: vec![SideEffect::VendorAccount, SideEffect::VendorAccount],
        }),
        Err(RecipeError::Damaged("a side effect is listed twice".into()))
    );
    let json = String::from_utf8(r.to_json()).unwrap();
    let unknown = json.replacen("vendor-account", "wipes-the-disk", 1);
    assert!(from_json(unknown.as_bytes()).is_err());
}

#[test]
fn the_recipe_format_is_3_and_a_format_1_recipe_is_read_as_saying_nothing_about_undo() {
    assert_eq!(pctwin_recipe::FORMAT, 3);
    let r = app(UndoDeclaration {
        uninstall: true,
        side_effects: vec![],
    })
    .unwrap();
    let json = String::from_utf8(r.to_json()).unwrap();
    assert!(json.contains(r#""format":3"#));
    // The same recipe as format 1 wrote it: no undo field.
    let one = json.replacen(r#""format":3"#, r#""format":1"#, 1).replacen(
        r#","undo":{"uninstall":true,"side-effects":[]}"#,
        "",
        1,
    );
    assert_ne!(one, json, "the undo field was taken out");
    let read = from_json(one.as_bytes()).unwrap();
    assert_eq!(read.undo, UndoDeclaration::default());
    assert!(!read.undo.removable());
    // Written back, it is a format-3 recipe.
    assert!(
        String::from_utf8(read.to_json())
            .unwrap()
            .contains(r#""format":3"#)
    );
    // Format 2 reads as it was.
    let two = json.replacen(r#""format":3"#, r#""format":2"#, 1);
    assert_eq!(from_json(two.as_bytes()).unwrap(), r);
    // A format-1 recipe that somehow carries an undo field is damaged, not trusted.
    let forged = json.replacen(r#""format":3"#, r#""format":1"#, 1);
    assert!(matches!(
        from_json(forged.as_bytes()),
        Err(RecipeError::Damaged(_))
    ));
}

/// The saved examples of formats 1, 2 and 3, read by every later build (engineering notes 9.11).
/// Set `PCTWIN_WRITE_FORMAT_FIXTURES=1` to write the current one, only when its number is new.
#[test]
fn the_saved_recipe_examples_still_read() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/formats");
    let two = dir.join("recipe-2.json");
    let three = dir.join("recipe-3.json");
    if std::env::var_os("PCTWIN_WRITE_FORMAT_FIXTURES").is_some() && pctwin_recipe::FORMAT == 3 {
        let r = app(UndoDeclaration {
            uninstall: true,
            side_effects: vec![SideEffect::LeavesSettings],
        })
        .unwrap();
        std::fs::write(&three, r.to_json()).unwrap();
    }
    let one = from_json(&std::fs::read(dir.join("recipe-1.json")).unwrap()).unwrap();
    assert_eq!(one.undo, UndoDeclaration::default());
    assert_eq!(one.steps.len(), 1);
    for file in [two, three] {
        let read = from_json(&std::fs::read(file).unwrap()).unwrap();
        assert!(read.undo.removable());
        assert_eq!(read.undo.side_effects, vec![SideEffect::LeavesSettings]);
    }
}

/// Bytes as a recipe compiled into the program would be (`&'static`): the tests' stand-in.
fn leak(b: &[u8]) -> &'static [u8] {
    Box::leak(b.to_vec().into_boxed_slice())
}

#[allow(dead_code)]
fn from_json(b: &[u8]) -> Result<Recipe, RecipeError> {
    Recipe::shipped(leak(b))
}

#[allow(dead_code)]
fn from_json_with(
    catalogs: &pctwin_catalogue::Catalogue<pctwin_recipe::CatalogRow>,
    b: &[u8],
) -> Result<Recipe, RecipeError> {
    Recipe::shipped_with(catalogs, leak(b))
}
