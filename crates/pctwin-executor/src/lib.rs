//! PCTwin executor (engineering notes, building block 4; Security Design B and C).
//!
//! The executor rebuilds apps and settings on the new laptop from an approved plan, and nothing
//! else:
//!
//! - A [`Plan`] can only be made from a move record together with the person's
//!   [`Approval`] of that exact revision, and only for items in that record that are part of the
//!   move. A changed record needs a new approval.
//! - Each planned item runs one checked [`Recipe`] through an OS [`Adapter`], step by typed step.
//!   The adapter is where each operating system does the work; it receives typed steps only,
//!   never a command line or an address.
//! - Every item ends with one honest [`ItemResult`]: reinstalled only when the recipe's own checks
//!   pass; failed or needs-action with its reason otherwise. A recipe that does not support the
//!   new laptop's system fails with [`Reason::NoEquivalent`] before anything runs.

use pctwin_recipe::{Catalog, Check, Platform, Recipe, SettingValue, SettingsPlace, Step};
use pctwin_record::{Approval, Inclusion, ItemId, Record, RecordError};
use pctwin_types::{ItemResult, ItemStatus, Reason};

/// Why a plan was refused.
#[derive(Debug, thiserror::Error)]
pub enum PlanError {
    #[error(transparent)]
    Record(#[from] RecordError),
    #[error("item {0} is not in the approved record")]
    UnknownItem(String),
    #[error("item {0} is not part of the move")]
    LeftOut(String),
    #[error("item {0} is planned twice")]
    Twice(String),
    #[error("a recipe in the plan is not valid: {0}")]
    BadRecipe(String),
}

/// The work an OS adapter does for each typed step. Each method returns the reason when the step
/// did not succeed.
pub trait Adapter {
    /// The new laptop's system and processor.
    fn platform(&self) -> Platform;
    fn install(&mut self, catalog: Catalog, package: &str) -> Result<(), Reason>;
    fn copy_settings(&mut self, from: &SettingsPlace, to: &SettingsPlace) -> Result<(), Reason>;
    fn apply_setting(&mut self, key: &str, value: &SettingValue) -> Result<(), Reason>;
    /// Whether a check passes on the new laptop.
    fn verify(&mut self, check: &Check) -> bool;
}

/// One planned item: the record item it rebuilds, and the recipe that rebuilds it.
#[derive(Debug, Clone)]
pub struct PlannedItem {
    pub item: ItemId,
    pub recipe: Recipe,
}

/// An approved plan. It can only be made by [`Plan::approved`].
#[derive(Debug)]
pub struct Plan {
    items: Vec<PlannedItem>,
}

impl Plan {
    /// Builds the plan for exactly the record revision the person approved.
    pub fn approved(
        record: &Record,
        approval: &Approval,
        items: Vec<PlannedItem>,
    ) -> Result<Self, PlanError> {
        record.check(approval)?;
        let mut seen = std::collections::HashSet::new();
        for p in &items {
            let hex = p.item.to_hex();
            let found = record
                .items
                .iter()
                .find(|i| i.id == p.item)
                .ok_or_else(|| PlanError::UnknownItem(hex.clone()))?;
            if found.inclusion != Inclusion::Included {
                return Err(PlanError::LeftOut(hex));
            }
            if !seen.insert(p.item) {
                return Err(PlanError::Twice(hex));
            }
            p.recipe
                .validate()
                .map_err(|e| PlanError::BadRecipe(e.to_string()))?;
        }
        Ok(Self { items })
    }

    pub fn items(&self) -> &[PlannedItem] {
        &self.items
    }

    /// Runs every planned item and returns each one's result, in plan order.
    pub fn run(&self, adapter: &mut dyn Adapter) -> Vec<(ItemId, ItemResult)> {
        self.items
            .iter()
            .map(|p| (p.item, run_recipe(&p.recipe, adapter)))
            .collect()
    }
}

/// Runs one recipe. It stops at the first step that does not succeed.
fn run_recipe(recipe: &Recipe, adapter: &mut dyn Adapter) -> ItemResult {
    if !recipe.supports(adapter.platform()) {
        return ItemResult::because(Reason::NoEquivalent);
    }
    let mut by_hand = false;
    for step in &recipe.steps {
        let done = match step {
            Step::Install { catalog, package } => adapter.install(*catalog, package),
            Step::CopySettings { from, to } => adapter.copy_settings(from, to),
            Step::ApplySetting { key, value } => adapter.apply_setting(key, value),
            Step::Verify { check } => {
                if adapter.verify(check) {
                    Ok(())
                } else {
                    Err(Reason::NotVerified)
                }
            }
            Step::Checklist { .. } => {
                by_hand = true;
                Ok(())
            }
        };
        if let Err(reason) = done {
            return ItemResult::because(reason);
        }
    }
    if by_hand {
        ItemResult::because(Reason::FinishByHand)
    } else {
        // A valid recipe that changes anything has a check, and every check passed.
        ItemResult::done(ItemStatus::Reinstalled)
            .unwrap_or(ItemResult::because(Reason::NotVerified))
    }
}
