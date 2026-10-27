//! PCTwin settings and personalisation (Task List 2.4; Phase 2.4 plan, as adopted on 26 October
//! 2026).
//!
//! - [`schema`]: the fixed, versioned list of named settings, each with a typed, bounded value.
//! - [`ownership`]: one writer per settings domain, shared with the private-data role (2.5).
//! - [`map`]: from the old laptop's values to the new laptop's, with one declared outcome each
//!   (`exact`, `equivalent`, `guided`, `unsupported`, `not-applicable`).
//!
//! Nothing here runs a command a recipe or a peer wrote, copies a whole settings folder, or
//! touches a key that is not on the list.

pub mod codec;
pub mod map;
pub mod ownership;
pub mod schema;

pub use map::{
    Action, Arrival, Capture, Choice, Choices, Outcome, PathMap, People, PlanError, Planned,
    Protected, Reading, SecurityControl, SettingsPlan, Target, plan,
};
pub use schema::{CHECKED_ON, Route, SCHEMA_VERSION, SettingId, Support, System};
