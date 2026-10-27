//! Every structured-text parser a settings file, a preference list, a Wi-Fi profile, a dconf
//! dump or a catalogue passes through, behind the one depth gate (fourth review, finding F2):
//! anything is read or refused, never a crash or a stack overflow, and nothing the gate passes
//! nests deeper than it allows.
#![no_main]

use libfuzzer_sys::fuzz_target;
use pctwin_settings::catalogue::{Catalogue, System};
use pctwin_settings::classify::{Ctx, FolderMap};
use pctwin_settings::files::{FileCtx, FoundFile, format_of, plan_file};
use pctwin_types::nesting::{MAX_NESTING, Syntax, check, depth};
use std::collections::{BTreeMap, BTreeSet};

fuzz_target!(|data: &[u8]| {
    let Some((&pick, rest)) = data.split_first() else {
        return;
    };
    let Ok(text) = std::str::from_utf8(rest) else {
        return;
    };
    for syn in [
        Syntax::Json,
        Syntax::Jsonc,
        Syntax::Toml,
        Syntax::Xml,
        Syntax::GVariant,
    ] {
        if check(text, syn).is_ok() {
            assert!(depth(text, syn, MAX_NESTING) <= MAX_NESTING);
        }
    }
    let name = ["a/x.json", "a/x.jsonc", "a/x.toml", "a/x.ini", "a/x.xml"][usize::from(pick % 5)];
    let cat = Catalogue::starter().unwrap();
    let map = FolderMap(vec![(r"C:\Users\ada\Documents".into(), r"D:\Docs".into())]);
    let (running, versions, renames) = (BTreeSet::new(), BTreeMap::new(), BTreeMap::new());
    let ctx = FileCtx {
        classify: Ctx {
            paths: &map,
            same_screens: false,
            hints: &cat.action_hints,
        },
        from: System::Windows,
        to: System::Windows,
        home_old: r"C:\Users\ada",
        home_new: r"C:\Users\ada",
        running: &running,
        versions: &versions,
        folder_renames: &renames,
    };
    let f = FoundFile {
        root: pctwin_recipe::SettingsRoot::Config,
        rel: name.into(),
        label: "a".into(),
        format: format_of(name),
    };
    let _ = plan_file(&cat, &f, rest, &ctx);
    let _ = pctwin_settings::plist::parse(text);
    let _ = pctwin_settings::wifi::parse_profile(text);
    let _ = pctwin_settings::found::parse_dconf_dump(text);
    let _ = pctwin_settings::gvariant::strings(text);
    let _ = Catalogue::from_json(rest);
});
