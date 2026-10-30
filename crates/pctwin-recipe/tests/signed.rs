//! Signed recipes (Task List 2.1 "Recipe format: versioned, signed"): a recipe from outside the
//! shipped program is read only with a minisign signature by a trusted privileged key, bound to
//! its ID and version, never older than one already accepted. Recipes built in code, or compiled
//! into the program, are trusted as code.

use pctwin_catalogue::{MemoryMarks, RollbackMarks, test_key};
use pctwin_recipe::{
    Arch, Catalog, Check, Os, Platform, Recipe, RecipeError, RecipeId, Step, recipe_comment,
};

fn recipe(version: u32) -> Recipe {
    Recipe::new(
        RecipeId::new("org.example.writer").unwrap(),
        version,
        vec![Platform {
            os: Os::Windows,
            arch: Arch::X86_64,
        }],
        vec![
            Step::Install {
                catalog: Catalog::Winget,
                package: "Example.Writer".into(),
            },
            Step::Verify {
                check: Check::PackageInstalled {
                    catalog: Catalog::Winget,
                    package: "Example.Writer".into(),
                },
            },
        ],
    )
    .unwrap()
}

fn signed(bytes: &[u8], id: &str, version: u32) -> String {
    test_key::sign_raw_privileged(bytes, &recipe_comment(id, version))
}

#[test]
fn a_signed_recipe_reads() {
    let r = recipe(2);
    let bytes = r.to_json();
    let sig = signed(&bytes, "org.example.writer", 2);
    let marks = MemoryMarks::default();
    let back = Recipe::signed(&bytes, &sig, &test_key::privileged_keys(), &marks).unwrap();
    assert_eq!(back, r);
    // Its version is now the lowest one accepted for this recipe.
    assert_eq!(marks.highest("recipe/org.example.writer"), Ok(Some(2)));
}

#[test]
fn a_tampered_recipe_is_refused() {
    let bytes = recipe(2).to_json();
    let sig = signed(&bytes, "org.example.writer", 2);
    let tampered = String::from_utf8(bytes)
        .unwrap()
        .replace("Example.Writer", "Evil.Writer");
    assert_eq!(
        Recipe::signed(
            tampered.as_bytes(),
            &sig,
            &test_key::privileged_keys(),
            &MemoryMarks::default()
        ),
        Err(RecipeError::NotSigned)
    );
}

#[test]
fn an_unsigned_recipe_is_refused() {
    let bytes = recipe(2).to_json();
    for sig in ["", "not a signature"] {
        assert_eq!(
            Recipe::signed(
                &bytes,
                sig,
                &test_key::privileged_keys(),
                &MemoryMarks::default()
            ),
            Err(RecipeError::NotSigned)
        );
    }
    // The production keys hold none yet: nothing delivered is trusted.
    let sig = signed(&bytes, "org.example.writer", 2);
    assert_eq!(
        Recipe::signed(
            &bytes,
            &sig,
            &pctwin_catalogue::TrustedKeys::production(),
            &MemoryMarks::default()
        ),
        Err(RecipeError::NeedsPrivilegedKey)
    );
}

#[test]
fn an_ordinary_key_a_signature_for_another_recipe_and_an_older_version_are_refused() {
    let bytes = recipe(2).to_json();
    // Recipes reach the administrator helper: an ordinary key is not enough.
    let ordinary = test_key::sign_raw(&bytes, &recipe_comment("org.example.writer", 2));
    assert_eq!(
        Recipe::signed(
            &bytes,
            &ordinary,
            &test_key::keys(),
            &MemoryMarks::default()
        ),
        Err(RecipeError::NeedsPrivilegedKey)
    );
    let keys = test_key::privileged_keys();
    for (id, version) in [("org.example.writer", 3), ("org.example.other", 2)] {
        assert_eq!(
            Recipe::signed(
                &bytes,
                &signed(&bytes, id, version),
                &keys,
                &MemoryMarks::default()
            ),
            Err(RecipeError::SignedForSomethingElse),
            "{id} {version}"
        );
    }
    let marks = MemoryMarks::default();
    marks.raise("recipe/org.example.writer", 5).unwrap();
    assert_eq!(
        Recipe::signed(
            &bytes,
            &signed(&bytes, "org.example.writer", 2),
            &keys,
            &marks
        ),
        Err(RecipeError::Older {
            found: 2,
            in_use: 5
        })
    );
}

#[test]
fn a_recipe_compiled_into_the_program_reads_without_a_signature() {
    let bytes: &'static [u8] = Box::leak(recipe(1).to_json().into_boxed_slice());
    assert_eq!(Recipe::shipped(bytes).unwrap(), recipe(1));
}
