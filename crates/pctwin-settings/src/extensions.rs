//! An editor's extensions or plugins (Task List 2.4, cross-OS translations for VS Code and
//! JetBrains first): carried as a list, each one with an honest result of its own.
//!
//! An add-on is code, so its files never move (their writer is the install role, 2.1). What
//! moves is the list: where the editor keeps it on each system, and how to read it, is a
//! catalogue entry ([`crate::catalogue::ExtensionList`]), never code per editor. Each add-on is
//! either installed again by its identifier through the editor's own store (the marketplace the
//! catalogue names; PCTwin never picks a download address), or it ends "needs action" with plain
//! words: one installed from a file, one whose identifier is not well formed, or an editor whose
//! list names add-ons by folder only.
//!
//! An add-on is reported reinstalled only when the new laptop's own list, read after the install
//! step, names it ([`results`]); a store that said "done" is not proof.

use std::collections::BTreeSet;

use pctwin_types::ItemStatus;
use serde_json::Value;

use crate::catalogue::{ExtensionList, ListForm};

/// The largest number of add-ons one list may name.
pub const MAX_ITEMS: usize = 2000;

/// The person's step for an add-on installed from a file rather than the editor's store.
pub const FROM_A_FILE: &str =
    "it was added from a file, not from the editor's store: add it again the same way";
/// The person's step when the editor's list names add-ons only by their folder.
pub const BY_FOLDER: &str =
    "install it again from the editor's own add-on store (search for its name)";
/// The person's step for an add-on whose name PCTwin cannot use to ask the store.
pub const NOT_A_NAME: &str =
    "PCTwin could not read this add-on's name well enough to ask the store for it";
/// The person's step when the install step ran and the new laptop's list does not name it.
pub const NOT_THERE: &str =
    "it is not on the new laptop yet: open the editor's add-on store and install it";

/// One add-on as the old laptop's list names it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Found {
    /// What the person sees (the identifier, or the folder's name).
    pub name: String,
    /// The identifier the store knows it by, when it may be asked for by it.
    pub id: Option<String>,
    /// Why it cannot be installed by its identifier.
    pub why: Option<&'static str>,
}

/// What happens to one add-on.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Planned {
    /// Install it through the editor's own store, by this identifier.
    Reinstall { id: String, store: String },
    /// The person does it; why.
    NeedsAction { name: String, why: &'static str },
}

/// One add-on's result.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Outcome {
    pub name: String,
    pub status: ItemStatus,
    pub why: Option<&'static str>,
}

/// Whether `id` is an identifier a store can be asked for: `publisher.name` style, letters,
/// digits, `-`, `_` and `.`, starting with a letter or digit, never a path or an address.
pub fn id_ok(id: &str) -> bool {
    let b = id.as_bytes();
    !b.is_empty()
        && b.len() <= 128
        && b[0].is_ascii_alphanumeric()
        && b.iter()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, b'-' | b'_' | b'.'))
        && !id.contains("..")
        && !id.ends_with('.')
}

/// The value at a `/`-separated path inside a JSON value.
fn at<'a>(v: &'a Value, path: &str) -> Option<&'a Value> {
    path.split('/').try_fold(v, |v, k| v.get(k))
}

/// Reads a JSON list of add-ons (VS Code's `extensions.json`). `None` when it is not a list.
pub fn read_json(list: &ExtensionList, bytes: &[u8]) -> Option<Vec<Found>> {
    let ListForm::Json {
        id,
        source,
        store_sources,
    } = &list.form
    else {
        return None;
    };
    pctwin_types::nesting::check_bytes(bytes, pctwin_types::nesting::Syntax::Json).ok()?;
    let v: Value = serde_json::from_slice(bytes).ok()?;
    let items = v.as_array()?;
    let mut out: Vec<Found> = Vec::new();
    let mut seen = BTreeSet::new();
    for item in items.iter().take(MAX_ITEMS) {
        let Some(name) = at(item, id).and_then(Value::as_str) else {
            continue;
        };
        if !seen.insert(name.to_ascii_lowercase()) {
            continue;
        }
        let from_store = match source.as_deref().map(|s| at(item, s)) {
            // No source recorded: the store is how the editor installs by default.
            None | Some(None) => true,
            Some(Some(Value::String(s))) => store_sources.iter().any(|x| x == s),
            Some(Some(_)) => false,
        };
        let shown: String = name.chars().filter(|c| !c.is_control()).take(128).collect();
        let (id, why) = if !id_ok(name) {
            (None, Some(NOT_A_NAME))
        } else if !from_store {
            (None, Some(FROM_A_FILE))
        } else {
            (Some(name.to_ascii_lowercase()), None)
        };
        out.push(Found {
            name: shown,
            id,
            why,
        });
    }
    Some(out)
}

/// Reads a list of add-ons kept as folders (a JetBrains IDE's plugins): each one by its folder's
/// name, to be installed again by the person.
pub fn read_folders(names: &[String]) -> Vec<Found> {
    let mut seen = BTreeSet::new();
    names
        .iter()
        .filter(|n| !n.starts_with('.') && seen.insert(n.to_ascii_lowercase()))
        .take(MAX_ITEMS)
        .map(|n| Found {
            name: n.chars().filter(|c| !c.is_control()).take(128).collect(),
            id: None,
            why: Some(BY_FOLDER),
        })
        .collect()
}

/// Plans each add-on: through the store the catalogue names when it has an identifier, else the
/// person's step.
pub fn plan(list: &ExtensionList, found: &[Found]) -> Vec<Planned> {
    found
        .iter()
        .map(|f| match (&f.id, &list.store) {
            (Some(id), Some(store)) => Planned::Reinstall {
                id: id.clone(),
                store: store.clone(),
            },
            _ => Planned::NeedsAction {
                name: f.name.clone(),
                why: f.why.unwrap_or(BY_FOLDER),
            },
        })
        .collect()
}

/// Each add-on's result, from the new laptop's own list read after the install step
/// (identifiers in any letter case). Reinstalled only when that list names it.
pub fn results(planned: &[Planned], on_new_laptop: &BTreeSet<String>) -> Vec<Outcome> {
    let there: BTreeSet<String> = on_new_laptop
        .iter()
        .map(|s| s.to_ascii_lowercase())
        .collect();
    planned
        .iter()
        .map(|p| match p {
            Planned::Reinstall { id, .. } if there.contains(id) => Outcome {
                name: id.clone(),
                status: ItemStatus::Reinstalled,
                why: None,
            },
            Planned::Reinstall { id, .. } => Outcome {
                name: id.clone(),
                status: ItemStatus::NeedsAction,
                why: Some(NOT_THERE),
            },
            Planned::NeedsAction { name, why } => Outcome {
                name: name.clone(),
                status: ItemStatus::NeedsAction,
                why: Some(why),
            },
        })
        .collect()
}
