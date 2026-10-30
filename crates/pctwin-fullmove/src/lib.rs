//! The full move (Phase 2, "Queue of joining work"): the parts built for apps (2.1), the
//! administrator helper (2.2), undo (2.3), settings (2.4), special categories (2.5) and finishing
//! (2.7), joined to the move of files and people, so one move does all of it with one plan, one
//! approval, one administrator prompt and an honest result word for every item.
//!
//! - [`extras`]: what the old laptop sends beside its list (settings found by discovery, its own
//!   folders, the special categories), read strictly on the new laptop; only reading happens on
//!   the old laptop.
//! - [`plan`]: on the new laptop, before the approval: every app with where it comes from (or
//!   why it needs the person), every setting that moves or why not, every special category, and
//!   what undo can do for each change. Its digest joins the plan the person approves.
//! - [`run`]: after the files arrived: settings through the executor and the effect journal,
//!   apps through an [`run::Installer`] (the administrator helper on Windows; the system's own
//!   package tools on Linux, [`unix`]; plain Homebrew casks placed as the person on a Mac,
//!   [`mac`]; a fake in tests),
//!   and one result word per item.
//! - [`live`]: on the old laptop, the person's files that are live while their app runs (a mail
//!   store, a database): copied only once that app is closed, or left needs-action.
//! - [`undo`]: what undo would do, looked at now, and undo itself, which keeps a setting the
//!   person changed after the move.
//!
//! Nothing here decides alone that something worked: every result comes from the executor's
//! read-back, the helper's receipt and the laptop's own list after an install.

pub mod data;
pub mod drift;
pub mod extras;
pub mod fixups;
pub mod install;
pub mod live;
pub mod mac;
pub mod mailout;
pub mod plan;
pub mod run;
pub mod undo;
pub mod unix;
pub mod wifi;
