//! The permission broker (Phase 2.2): the helper's fixed privileged actions for the Phase 2
//! roles. This first part holds the actions' schemas, the helper's own package folder, the
//! system programs it may start and the honest restore point; the checks come next.

pub mod actions;
pub mod restore;
pub mod staging;
pub mod tool;
