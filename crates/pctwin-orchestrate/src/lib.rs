//! More move types (Task List 2.6): the planning that sits above one plain move.
//!
//! - [`cloud`]: files in a personal cloud folder are honest by default. A file stored online only
//!   is "safe in OneDrive, comes back when you sign in" (needs action, never copied); a file whose
//!   contents are on the laptop is copied like any other, and counts as only on this laptop
//!   unless PCTwin can tell it is uploaded; work or school folders stay left out; a sync app's own
//!   sign-in state is never copied.
//!
//! Nothing here changes a file on the old laptop's drives: the crate's clippy fence bans every
//! way to make, write, rename or remove one.

#![cfg_attr(not(test), forbid(clippy::disallowed_methods))]

pub mod cloud;
