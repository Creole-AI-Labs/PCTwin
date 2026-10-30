//! App data put right as it lands (Task List 2.4: cross-OS translations for the most-used apps,
//! workspace and recent-file paths rewritten).
//!
//! App data moves whole (2.5's own capture and landing). An app's settings file the settings
//! catalogue describes (VS Code's settings, keyboard shortcuts and recent lists; JetBrains' recent
//! projects) is then put through the catalogue's own file rules
//! ([`pctwin_settings::files::plan_file`]) before it lands: between different systems its
//! keyboard shortcuts become this system's (Ctrl and Cmd), a path into the person's moved files
//! points at where that file really landed here, a recent entry whose file did not move is
//! dropped, and what the rules leave out (sign-in state, telemetry) is left out. Nothing else is
//! touched: a file no catalogue entry describes, or one the rules would not move, lands exactly as
//! it was captured.
//!
//! The change happens in PCTwin's own staging folder, before the landing reads it: the capture
//! record is updated with each changed file's new length and hash, so the landing checks and
//! journals exactly the bytes it writes, and undo removes exactly those.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use pctwin_settings::System;
use pctwin_settings::catalogue::Catalogue;
use pctwin_settings::classify::{Ctx, InsideHome, Landed};
use pctwin_settings::files::{FileCtx, FileOutcome, FoundFile, MAX_FILE, entry_for, format_of};
use pctwin_special::roots::RootKind;

use crate::data::{PlacementWire, Record};

/// What the new laptop knows about the move while app data lands.
pub struct FixCtx<'a> {
    /// The old laptop's kind of system and this one's.
    pub from: System,
    pub to: System,
    /// The person's home folder there and here (for `$USER_HOME$`-style placeholders).
    pub home_old: &'a str,
    pub home_new: &'a str,
    /// Every file of the move on this laptop now, as (its full path on the old laptop, where it
    /// is stored here) ([`crate::run::DataDone::files`]).
    pub files: &'a [(String, String)],
}

/// The settings root an app data root stands for, where the settings catalogue names one.
fn settings_root(root: RootKind) -> Option<pctwin_recipe::SettingsRoot> {
    use pctwin_recipe::SettingsRoot;
    match root {
        RootKind::Roaming | RootKind::ApplicationSupport | RootKind::XdgConfig => {
            Some(SettingsRoot::Config)
        }
        RootKind::Local | RootKind::XdgData => Some(SettingsRoot::LocalData),
        RootKind::Preferences => Some(SettingsRoot::Preferences),
        RootKind::HomeDot => Some(SettingsRoot::Home),
        _ => None,
    }
}

/// `r` with each file the settings catalogue describes put through its rules, in `work` (the
/// component's staging folder); the capture record says the new lengths and hashes. A file that
/// cannot be read, or that the rules would not move, is left exactly as it is.
pub fn fix(r: &Record, work: &Path, ctx: &FixCtx<'_>) -> Record {
    let (root, folder) = match &r.placement {
        PlacementWire::Same { root, folder } | PlacementWire::Translated { root, folder } => {
            (*root, folder.as_str())
        }
        PlacementWire::PreservedOnly => return r.clone(),
    };
    let Some(root) = settings_root(root) else {
        return r.clone();
    };
    let Ok(catalogue) = Catalogue::starter() else {
        return r.clone();
    };
    let there = |p: &str| std::fs::symlink_metadata(p).is_ok();
    let landed = Landed {
        files: ctx.files,
        there: &there,
    };
    let inside = InsideHome {
        inner: &landed,
        home: ctx.home_new,
    };
    let running = BTreeSet::new();
    let versions = BTreeMap::new();
    let renames = BTreeMap::new();
    let file_ctx = FileCtx {
        classify: Ctx {
            paths: &inside,
            same_screens: false,
            hints: Catalogue::starter_hints(),
            facts: pctwin_settings::judge::Facts::system(),
        },
        from: ctx.from,
        to: ctx.to,
        home_old: ctx.home_old,
        home_new: ctx.home_new,
        running: &running,
        versions: &versions,
        folder_renames: &renames,
    };
    let mut out = r.clone();
    for f in &mut out.capture.files {
        let rel = format!("{folder}/{}", f.rel);
        let found = FoundFile {
            root,
            label: folder.to_string(),
            format: format_of(rel.rsplit('/').next().unwrap_or(&rel)),
            rel,
        };
        if entry_for(&catalogue, &found).is_none() || f.len > MAX_FILE {
            continue;
        }
        let path = f
            .rel
            .split('/')
            .fold(work.to_path_buf(), |p, part| p.join(part));
        let Ok(bytes) = std::fs::read(&path) else {
            continue;
        };
        if let FileOutcome::Move { bytes: new, .. } =
            pctwin_settings::files::plan_file(&catalogue, &found, &bytes, &file_ctx)
            && new != bytes
            && std::fs::write(&path, &new).is_ok()
        {
            f.len = new.len() as u64;
            f.hash = *blake3::hash(&new).as_bytes();
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A VS Code folder captured on Windows lands on a Mac: its keyboard shortcuts become the
    /// Mac's in staging, the record names the new bytes, and a file no catalogue entry describes
    /// is left exactly as captured.
    #[test]
    fn vs_code_shortcuts_are_put_right_in_staging_and_the_record_says_so() {
        use pctwin_special::capture::{Capture, CapturedFile, Method, generation};
        use pctwin_special::component::{Component, Custody, Evidence, Preservation};
        let work = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(work.path().join("User")).unwrap();
        let keys = br#"[{ "key": "ctrl+s", "command": "workbench.action.files.save" }]"#;
        std::fs::write(work.path().join("User/keybindings.json"), keys).unwrap();
        let other = b"left exactly as it was";
        std::fs::write(work.path().join("User/notes.txt"), other).unwrap();
        let members = vec![
            "User/keybindings.json".to_string(),
            "User/notes.txt".to_string(),
        ];
        let captured = |rel: &str, bytes: &[u8]| CapturedFile {
            rel: rel.into(),
            len: bytes.len() as u64,
            hash: *blake3::hash(bytes).as_bytes(),
        };
        let component = Component::new(
            pctwin_record::Owner::Person {
                account_id: "ada".into(),
            },
            "Code",
            members.clone(),
            Custody::Portable,
            false,
            vec![Evidence::Unreadable {
                why: "a test".into(),
            }],
        );
        let r = Record {
            app: "Visual Studio Code".into(),
            capture: Capture {
                component: component.id,
                method: Method::SteadyCopy,
                preservation: Preservation::Captured,
                generation: generation(work.path(), &members),
                files: vec![
                    captured("User/keybindings.json", keys),
                    captured("User/notes.txt", other),
                ],
                folded: Vec::new(),
                evidence: Vec::new(),
                closed_unconfirmed: false,
            },
            component,
            placement: PlacementWire::Translated {
                root: RootKind::ApplicationSupport,
                folder: "Code".into(),
            },
            root: RootKind::Roaming,
        };
        let ctx = FixCtx {
            from: System::Windows,
            to: System::Mac,
            home_old: r"C:\Users\ada",
            home_new: "/Users/ada",
            files: &[],
        };
        let fixed = fix(&r, work.path(), &ctx);
        let now = std::fs::read(work.path().join("User/keybindings.json")).unwrap();
        assert!(
            String::from_utf8_lossy(&now).contains("cmd+s"),
            "{:?}",
            String::from_utf8_lossy(&now)
        );
        assert_eq!(
            fixed.capture.files[0],
            captured("User/keybindings.json", &now)
        );
        assert_ne!(fixed.capture.files[0], r.capture.files[0]);
        assert_eq!(
            std::fs::read(work.path().join("User/notes.txt")).unwrap(),
            other
        );
        assert_eq!(fixed.capture.files[1], r.capture.files[1]);
        // Kept only (never landed as settings): nothing is touched.
        let kept = Record {
            placement: PlacementWire::PreservedOnly,
            ..r.clone()
        };
        assert_eq!(fix(&kept, work.path(), &ctx), kept);
    }

    #[test]
    fn roots_the_catalogue_names_and_no_others() {
        use pctwin_recipe::SettingsRoot;
        assert_eq!(settings_root(RootKind::Roaming), Some(SettingsRoot::Config));
        assert_eq!(
            settings_root(RootKind::ApplicationSupport),
            Some(SettingsRoot::Config)
        );
        assert_eq!(
            settings_root(RootKind::XdgConfig),
            Some(SettingsRoot::Config)
        );
        assert_eq!(settings_root(RootKind::Containers), None);
        assert_eq!(settings_root(RootKind::MacMail), None);
    }
}
