//! Package catalogs are open IDs mapped to adapters by catalogue rows (Phase 2 adoption,
//! changes 7 and 8): a new catalog ID for an existing adapter needs only a row, and a recipe
//! naming a catalog no row maps is found, not yet supported, without failing the others.

use pctwin_catalogue::{Catalogue, test_key};
use pctwin_recipe::{
    Catalog, CatalogRow, Recipe, RecipeError, Step, adapter_for, catalog_id_ok, read_recipes,
    shipped_catalogs,
};

fn recipe_json(catalog: &str) -> String {
    format!(
        r#"{{"format":3,"id":"org.example.app","version":1,"platforms":[{{"os":"windows","arch":"x86-64"}}],"steps":[{{"install":{{"catalog":"{catalog}","package":"Example.App"}}}},{{"verify":{{"check":{{"package-installed":{{"catalog":"{catalog}","package":"Example.App"}}}}}}}}],"undo":{{"uninstall":false}}}}"#
    )
}

#[test]
fn the_shipped_rows_map_every_adapter_and_its_other_names() {
    let rows = shipped_catalogs();
    assert!(rows.report().is_clean(), "{:?}", rows.report());
    for (id, adapter) in [
        ("winget", Catalog::Winget),
        ("microsoft-store", Catalog::MicrosoftStore),
        ("msstore", Catalog::MicrosoftStore),
        ("homebrew", Catalog::Homebrew),
        ("brew", Catalog::Homebrew),
        ("mac-app-store", Catalog::MacAppStore),
        ("mas", Catalog::MacAppStore),
        ("flathub", Catalog::Flathub),
        ("flatpak", Catalog::Flathub),
        ("snap", Catalog::Snap),
        ("apt", Catalog::Apt),
        ("dnf", Catalog::Dnf),
    ] {
        assert_eq!(adapter_for(rows, id), Ok(adapter), "{id}");
        // Every adapter's own name is a row, so every earlier recipe still reads.
        assert_eq!(adapter_for(rows, adapter.as_str()), Ok(adapter));
        assert_eq!(adapter.to_string(), adapter.as_str());
    }
}

#[test]
fn a_recipe_naming_another_name_of_an_adapter_reads_as_that_adapter() {
    let r = Recipe::from_json(recipe_json("brew").as_bytes()).unwrap();
    assert!(matches!(
        &r.steps[0],
        Step::Install {
            catalog: Catalog::Homebrew,
            ..
        }
    ));
    // Written back under the adapter's own name.
    assert!(
        String::from_utf8(r.to_json())
            .unwrap()
            .contains(r#""catalog":"homebrew""#)
    );
}

/// The no-row proof: a catalog nobody mapped makes that recipe found, not yet supported, and
/// the other recipes still read.
#[test]
fn an_unknown_catalog_marks_one_recipe_not_the_whole_set() {
    assert_eq!(
        Recipe::from_json(recipe_json("chocolatey").as_bytes()),
        Err(RecipeError::NotYetSupported("chocolatey".into()))
    );
    let good = recipe_json("winget");
    let unknown = recipe_json("chocolatey");
    let damaged = recipe_json("Bad Id");
    let newer = good.replacen(r#""format":3"#, r#""format":4"#, 1);
    let files: Vec<&[u8]> = vec![
        good.as_bytes(),
        unknown.as_bytes(),
        damaged.as_bytes(),
        newer.as_bytes(),
    ];
    let read = read_recipes(shipped_catalogs(), &files);
    assert_eq!(read.ready.len(), 1);
    assert_eq!(read.not_yet_supported, [(1, "chocolatey".to_string())]);
    let damaged_at: Vec<usize> = read.damaged.iter().map(|(i, _)| *i).collect();
    assert_eq!(damaged_at, [2, 3]);
    // A catalog ID that is not text at all is damage, not an unknown catalog.
    let not_text = good.replacen(r#""catalog":"winget""#, r#""catalog":7"#, 1);
    assert!(matches!(
        Recipe::from_json(not_text.as_bytes()),
        Err(RecipeError::Damaged(_))
    ));
}

/// The new-row proof: a catalog ID added only as a signed row installs through its adapter with
/// no code change.
#[test]
fn a_new_signed_row_maps_a_new_catalog_id_with_no_code_change() {
    let bytes = br#"{"format":1,"catalogue":"package-catalogs","version":2,"rows":[
        {"id":"winget","adapter":"winget","name":"winget"},
        {"id":"winget-community","adapter":"winget","name":"winget community"},
        {"id":"chocolatey","adapter":"choco","name":"Chocolatey"},
        {"id":"Bad","adapter":"winget","name":"Bad"},
        {"id":"x","adapter":"winget","name":""}
    ]}"#;
    let sig = test_key::sign(bytes, "package-catalogs", 2);
    let rows = Catalogue::<CatalogRow>::signed(
        bytes,
        &sig,
        &test_key::keys(),
        &pctwin_catalogue::MemoryMarks::at(1),
    )
    .unwrap();
    assert_eq!(rows.rows().len(), 2);
    assert_eq!(rows.report().skipped.len(), 3);
    let r = Recipe::from_json_with(&rows, recipe_json("winget-community").as_bytes()).unwrap();
    assert!(matches!(
        &r.steps[0],
        Step::Install {
            catalog: Catalog::Winget,
            ..
        }
    ));
    // Not in the shipped rows.
    assert_eq!(
        Recipe::from_json(recipe_json("winget-community").as_bytes()),
        Err(RecipeError::NotYetSupported("winget-community".into()))
    );
}

#[test]
fn catalog_ids_are_checked() {
    for ok in ["winget", "a", "x.y-2", &"a".repeat(64)] {
        assert!(catalog_id_ok(ok), "{ok}");
    }
    for bad in ["", "-a", ".a", "A", "a b", "a/b", "a_b", &"a".repeat(65)] {
        assert!(!catalog_id_ok(bad), "{bad}");
    }
}

/// Recipe IDs keep their own rule (a survivor of the catalog mutation run, which first matched
/// the recipe ID check): no `_`.
#[test]
fn recipe_ids_refuse_underscores() {
    assert!(pctwin_recipe::RecipeId::new("org.example.app").is_ok());
    assert!(pctwin_recipe::RecipeId::new("org_example").is_err());
}

/// Fourth review: a damaged recipe naming a catalog no row maps was reported "not yet supported"
/// (the ID was looked up before the recipe was checked). The recipe is checked first.
#[test]
fn review4_a_damaged_recipe_is_damaged_whatever_catalog_it_names() {
    let damaged = recipe_json("chocolatey").replacen('}', r#","bogus":1}"#, 1);
    assert!(matches!(
        Recipe::from_json_with(shipped_catalogs(), damaged.as_bytes()),
        Err(RecipeError::Damaged(_))
    ));
    let bad_package = recipe_json("chocolatey").replace("Example.App", "../../x");
    let e = Recipe::from_json_with(shipped_catalogs(), bad_package.as_bytes()).unwrap_err();
    assert!(!matches!(e, RecipeError::NotYetSupported(_)), "{e:?}");
    // A sound recipe with an unknown catalog is still found, not yet supported.
    assert_eq!(
        Recipe::from_json_with(shipped_catalogs(), recipe_json("chocolatey").as_bytes())
            .unwrap_err(),
        RecipeError::NotYetSupported("chocolatey".into())
    );
    let set = read_recipes(
        shipped_catalogs(),
        &[damaged.as_bytes(), recipe_json("chocolatey").as_bytes()],
    );
    assert_eq!(set.damaged.len(), 1);
    assert_eq!(set.not_yet_supported, [(1, "chocolatey".to_string())]);
}

/// Fourth review: two rows mapping one catalog ID to two adapters loaded clean, the first
/// silently winning. Neither is used, and the report says so.
#[test]
fn review4_two_rows_for_one_catalog_id_are_both_refused_and_reported() {
    let cat = Catalogue::<CatalogRow>::shipped(
        r#"{"format":1,"catalogue":"package-catalogs","version":1,"rows":[{"id":"winget","adapter":"apt","name":"x"},{"id":"winget","adapter":"winget","name":"winget"}]}"#,
    );
    assert!(!cat.report().is_clean());
    assert_eq!(cat.report().skipped.len(), 2);
    assert_eq!(
        adapter_for(&cat, "winget").unwrap_err(),
        RecipeError::NotYetSupported("winget".into())
    );
}
