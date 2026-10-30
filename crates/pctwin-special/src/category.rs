//! The special categories a person sees (Task List 2.5): mail, browsers, games, AI tools and
//! secrets, and the steps each category adds (plan sections 6 to 10).
//!
//! A category is read from what was found, never from an app's name:
//! - **secret**: the component holds keys, saved passwords or other secrets (its content);
//! - **email**: a mail store or a server mailbox's offline copy is in the folder (their formats);
//! - **browser**: bookmarks or a history of visited pages are in the folder (their shapes);
//! - **AI tool**: model weights or a model library named by fingerprints are in the folder;
//! - **game**: the folder is in the place the system keeps for games' saves;
//! - a signed catalogue row may name the category of a folder the rules above leave as other
//!   (an editor's settings, a game's own folder); it can never make something secret or not.
//!
//! Each category's [`Note`]s are what the person is told on top of the component's own step: mail
//! search is built again by the mail app, a server mailbox comes back from the server and its old
//! copy is never used as the mail, a browser's own sync is the easy way, passwords and passkeys
//! are moved with the browser's or provider's own tools, a game's saves may also be kept online,
//! AI models are checked by their fingerprints and never run, and secrets move only when ticked.
//! Notes are typed so the screens say them in each language; nothing in a note is a secret.

use serde::{Deserialize, Serialize};

use crate::classify::FolderReport;
use crate::component::{Component, Custody, Evidence};
use crate::roots::RootKind;

/// A special category.
#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize,
)]
#[serde(rename_all = "kebab-case")]
pub enum Category {
    Email,
    Browser,
    Game,
    AiTool,
    Secret,
    #[default]
    Other,
}

/// What the person is told about a component because of its category.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", tag = "note")]
pub enum Note {
    /// Mail moved: the mail app builds its search again the first time it opens, so searching
    /// may be slow or incomplete for a while.
    SearchRebuilds,
    /// A server mailbox: the mail comes back from the server after signing in; the old laptop's
    /// copy is kept only in case something never reached the server, never used as the mail.
    ServerResync,
    /// A browser: turning on its own sync on the new laptop is the easiest way to bring
    /// bookmarks, history, open tabs and add-ons (the maker's page, when a row names it).
    OwnSync { page: Option<String> },
    /// Saved passwords: only with the browser's own export and import, or its sync; the file a
    /// browser exports is plain text, so PCTwin keeps it private and removes it after the import.
    PasswordsGuided,
    /// Passkeys: those saved in an account (a browser's or a password manager's) come back when
    /// signing in there; those kept only on this laptop are made again on each site, while the
    /// old laptop still works. PCTwin cannot see passkeys, so it cannot list them.
    PasskeysGuided,
    /// Game saves: the game's launcher may also keep saves online; if it asks which to keep,
    /// compare the dates first. PCTwin never starts a game to check.
    CloudSavesMayDiffer,
    /// AI models: checked by their fingerprints when they land; never run or loaded as a check.
    ModelsCheckedNotRun,
    /// Settings that start programs or reach the internet (tool servers, hooks): moved as data
    /// and held for the person to look at before the app on the new laptop uses them.
    ToolSettingsReview,
    /// A secret: moves only if the person ticks it, by itself; never in a report, log or AI.
    SecretTickEach,
    /// A mail archive (an Outlook data file) moved as a file: the person opens it in the mail
    /// app on the new laptop (in Outlook: File, Open, Outlook Data File). PCTwin never opens it.
    OpenArchiveInMailApp,
    /// A server mailbox's offline copy: folders kept "on this computer only" are not on the
    /// server, so before the old laptop is wiped the person exports them with the mail app's own
    /// export to an archive file, which then moves like any file. PCTwin cannot see inside.
    LocalOnlyFoldersExport,
    /// A browser's bookmarks are also written as the standard bookmarks file: in a different
    /// browser on the new laptop, use its own "import bookmarks from an HTML file" (2.5 browsers).
    BookmarksFile,
    /// History has no shared format between different browsers: the new browser's own import
    /// (while both browsers are on one laptop) or the browser's sync brings it.
    HistoryOwnImport,
    /// The browser's add-ons and open pages are listed in a page beside its bookmarks file, for
    /// the person only; nothing in it is opened for them.
    PagesAndAddOnsListed,
    /// Open pages kept in a form PCTwin cannot list: the browser's own sync, or its option to
    /// open the pages from last time, brings them back.
    OpenPagesBySync,
    /// Game saves the game's launcher keeps online: they come back from the launcher on the new
    /// laptop, so they are not copied by default (never over the launcher's own copy), and the
    /// old laptop's copy is left as it is (`games`).
    SavesInLauncherCloud,
    /// Game saves from another kind of system: kept on the new laptop, but the game there may
    /// not read them, since no row says the format is the same (`games`).
    SaveMayNotWorkHere,
    /// Saves a store keeps protected for itself (the Microsoft Store and Xbox app): not copied;
    /// they come back from the store's own sync once signed in (`games`).
    StoreProtectedSaves,
    /// Programs inside a game's saves or add-ons: listed apart, never copied as data or turned
    /// on; they come again from their maker (`games`).
    ModProgramsHeld,
    /// Programs inside a game's saves that sit among the person's own files (Documents): they
    /// move with those files as plain files, never turned on; working copies come from their
    /// maker (`games`; the 2.5 security review, 30 October 2026: "not copied" is not true there).
    ModProgramsAsFiles,
    /// Installed games: the launcher on the new laptop finds and checks them with its own
    /// "locate" and "verify"; the games' own files are not copied unless the person chooses.
    LauncherVerify,
    /// Mail kept only on the old Mac is written as mailbox files on the new laptop; the person
    /// imports them in their mail app (its steps).
    MailImport,
    /// Apple Mail's folder could not be looked inside (Full Disk Access is off): the person turns
    /// it on to bring mail kept only on the Mac. Never said as "no mail".
    MailNoAccess,
}

fn any(c: &Component, f: impl Fn(&Evidence) -> bool) -> bool {
    c.evidence.iter().any(f)
}

fn mail(e: &Evidence) -> bool {
    matches!(e, Evidence::MailStore | Evidence::ServerMailCache)
}

/// Model weights or a model library: a library named by fingerprints always carries the model
/// fact too.
fn models(e: &Evidence) -> bool {
    *e == Evidence::ModelData
}

/// The category of the app data folder in `report`: its row's, else by what it holds.
pub fn of_folder(report: &FolderReport) -> Category {
    let found = |f: fn(&Evidence) -> bool| report.components.iter().any(|c| any(c, f));
    if found(mail) {
        Category::Email
    } else if found(|e| *e == Evidence::BrowsingData) {
        Category::Browser
    } else if found(models) {
        Category::AiTool
    } else if let Some(c) = report.row.as_ref().and_then(|r| r.category) {
        c
    } else if report.folder.root == RootKind::SavedGames {
        Category::Game
    } else {
        Category::Other
    }
}

/// The category of `c`, found in `report`: a secret is always a secret.
pub fn of(report: &FolderReport, c: &Component) -> Category {
    if c.secret {
        Category::Secret
    } else {
        of_folder(report)
    }
}

/// What the person is told about `c` (found in `report`) because of its category.
pub fn notes_for(report: &FolderReport, c: &Component) -> Vec<Note> {
    let mut out = Vec::new();
    let folder = of_folder(report);
    if any(c, |e| *e == Evidence::ServerMailCache) {
        out.push(Note::ServerResync);
        out.push(Note::LocalOnlyFoldersExport);
    }
    if c.custody == Custody::Portable && any(c, |e| *e == Evidence::MailArchive) {
        out.push(Note::OpenArchiveInMailApp);
    }
    if c.custody == Custody::Portable && any(c, |e| *e == Evidence::MailStore) {
        out.push(Note::SearchRebuilds);
    }
    if folder == Category::Browser {
        out.push(Note::OwnSync {
            page: report.row.as_ref().and_then(|r| r.relink.clone()),
        });
        if any(c, |e| *e == Evidence::CredentialStore) {
            out.push(Note::PasswordsGuided);
        }
        if c.custody == Custody::ServiceHeld {
            out.push(Note::PasskeysGuided);
        }
    }
    if folder == Category::Game && c.custody == Custody::Portable {
        out.push(Note::CloudSavesMayDiffer);
    }
    if any(c, models) {
        out.push(Note::ModelsCheckedNotRun);
    }
    if any(c, |e| *e == Evidence::EnablesCommands) {
        out.push(Note::ToolSettingsReview);
    }
    if c.secret {
        out.push(Note::SecretTickEach);
    }
    out
}
