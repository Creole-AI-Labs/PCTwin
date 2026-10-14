//! The AI layer's interface (Security Design D: "the AI suggests, the rules verify").
//!
//! The model itself lives outside the open-source core and plugs in through [`Model`]. Whatever
//! it is, its output is only text, and the core treats that text as untrusted:
//!
//! - The model sees small [`Clues`] only: app names and publishers, never document contents.
//! - Its answer must parse as a list of typed [`Suggestion`]s. Unknown kinds or fields are
//!   refused, so it cannot add a command, a link or a path.
//! - [`verify`] checks each suggestion on its own: the item must be one of the clues it was given,
//!   and a recipe must be one the catalog really has, at that exact version. Anything else is
//!   dropped with its [`Rejection`]. The checked suggestions then go into the plan the person
//!   approves; nothing the model says runs by itself.
//! - With no model ([`NoModel`]) everything still works: every app simply gets no suggestion.

use std::collections::{BTreeMap, HashSet};

use pctwin_recipe::RecipeRef;
use pctwin_record::ItemId;
use serde::{Deserialize, Serialize};

/// The most text a model answer may hold before it is refused unread.
pub const MAX_ANSWER: usize = 256 * 1024;
/// The most suggestions read from one answer.
pub const MAX_SUGGESTIONS: usize = 2000;
/// Clue text is cut to this many characters.
const MAX_CLUE: usize = 120;

/// One app found on the old laptop, as the model may see it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct AppClue {
    pub item: ItemId,
    pub name: String,
    pub publisher: Option<String>,
}

/// Everything the model is shown.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct Clues {
    pub apps: Vec<AppClue>,
}

/// Cuts clue text short and drops control characters, so a crafted app name cannot hide extra
/// lines or markup in what the model reads.
pub fn clean_clue(text: &str) -> String {
    text.chars()
        .filter(|c| !c.is_control())
        .take(MAX_CLUE)
        .collect()
}

impl Clues {
    pub fn app(&mut self, item: ItemId, name: &str, publisher: Option<&str>) {
        self.apps.push(AppClue {
            item,
            name: clean_clue(name),
            publisher: publisher.map(clean_clue),
        });
    }

    pub fn to_json(&self) -> String {
        serde_json::to_string(self).unwrap_or_default()
    }
}

/// Why the model could not answer.
#[derive(Debug, thiserror::Error)]
pub enum ModelError {
    #[error("no model is installed")]
    NotInstalled,
    #[error("the model stopped: {0}")]
    Stopped(String),
}

/// A model: clues in, text out. It has no network and cannot write files (Security Design D);
/// its answer is checked by [`verify`].
pub trait Model {
    fn suggest(&self, clues: &Clues) -> Result<String, ModelError>;
}

/// The free app without a model: no suggestions, everything else unchanged.
pub struct NoModel;

impl Model for NoModel {
    fn suggest(&self, _: &Clues) -> Result<String, ModelError> {
        Ok("[]".into())
    }
}

/// What a model may suggest. The set is closed.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case", deny_unknown_fields)]
pub enum Suggestion {
    /// This app on the old laptop is rebuilt by this recipe.
    AppMatch { item: ItemId, recipe: RecipeRef },
    /// No recipe fits this app; it goes on the person's checklist.
    NoMatch { item: ItemId },
}

impl Suggestion {
    pub fn item(&self) -> ItemId {
        match self {
            Suggestion::AppMatch { item, .. } | Suggestion::NoMatch { item } => *item,
        }
    }
}

/// Why a suggestion was dropped.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Rejection {
    /// The whole answer was not a list of known suggestions, or was too long.
    Malformed(String),
    /// The suggestion names an item the model was not shown.
    UnknownItem(ItemId),
    /// The suggestion names a recipe or version the catalog does not have.
    UnknownRecipe(RecipeRef),
    /// A second suggestion for the same item; the first one counts.
    Duplicate(ItemId),
}

/// The checked result of one model answer.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct Verified {
    pub accepted: BTreeMap<ItemId, Suggestion>,
    pub rejected: Vec<Rejection>,
}

/// Checks a model's answer against the clues it was shown and the catalog. `in_catalog` answers
/// whether the catalog holds that exact recipe version.
pub fn verify(answer: &str, clues: &Clues, in_catalog: &dyn Fn(&RecipeRef) -> bool) -> Verified {
    let mut out = Verified::default();
    if answer.len() > MAX_ANSWER {
        out.rejected
            .push(Rejection::Malformed("the answer is too long".into()));
        return out;
    }
    let list: Vec<Suggestion> = match serde_json::from_str(answer) {
        Ok(list) => list,
        Err(e) => {
            out.rejected.push(Rejection::Malformed(e.to_string()));
            return out;
        }
    };
    if list.len() > MAX_SUGGESTIONS {
        out.rejected
            .push(Rejection::Malformed("too many suggestions".into()));
        return out;
    }
    let shown: HashSet<ItemId> = clues.apps.iter().map(|a| a.item).collect();
    for s in list {
        let item = s.item();
        if !shown.contains(&item) {
            out.rejected.push(Rejection::UnknownItem(item));
            continue;
        }
        if let Suggestion::AppMatch { recipe, .. } = &s
            && !in_catalog(recipe)
        {
            out.rejected.push(Rejection::UnknownRecipe(recipe.clone()));
            continue;
        }
        if out.accepted.contains_key(&item) {
            out.rejected.push(Rejection::Duplicate(item));
            continue;
        }
        out.accepted.insert(item, s);
    }
    out
}
