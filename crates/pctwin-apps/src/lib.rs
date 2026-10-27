//! Phase 2.1: getting a person's apps back on the new laptop (Task List 2.1; the 2.1 plan in
//! `docs/plans/PCTwin_Phase_2_Bots/`, as adopted with the changes in its `ADOPTION.md`).
//!
//! **Discovery first** (ADOPTION changes 7 and 8). Nothing here starts from a list of known apps.
//! [`found`] reads every app the old laptop's own system records, for every person, with who
//! made it, read from the old laptop's own copy (the Windows signer, the Mac Team ID, the Linux
//! repository or Flatpak key). [`routes`] then looks each one up *by that maker* in the places
//! apps come from, in a fixed order; signed catalogue rows only add hints (a maker's own download
//! link, say). An app no route can verify ends needs-action with its name, its maker and the web
//! address the old laptop recorded: its data still moves.
//!
//! **The anchor.** A download is installed only when the administrator helper's own maker check
//! finds the same maker as the old laptop's copy (and the system's own malware check finished
//! without a finding). A fake or swapped download fails, whatever site it came from, so a stale
//! or wrong catalogue fails safe.
//!
//! **Walls** (ADOPTION change 1). Downloads run in a separate, locked-down process ([`fetch`]);
//! installs go through the administrator helper, which checks the exact bytes again itself; the
//! installs role's grant cannot be turned into any other role's; the receipt handed to 2.5 carries
//! no permission ([`after`]).

pub mod fetch;
pub mod found;
pub mod routes;
pub mod version;
