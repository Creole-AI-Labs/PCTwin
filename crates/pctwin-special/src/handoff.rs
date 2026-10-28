//! Vendor handoffs and the app's own export (plan sections 4, 7 and 10; generic mechanisms, never
//! per app).
//!
//! - **What the person does** ([`step_for`]): every component that does not simply copy gets one
//!   honest step, from its class and evidence: sign in again (S), relink or re-enrol (D), the
//!   app's own export (E), or look at it (unresolved). A catalogue row may add the maker's page or
//!   a better route; without one the step is still there, in general words.
//! - **Opening, as the person** ([`open`]): the maker's page in the person's own browser, or the
//!   app itself at its export screen, through [`Person`], which the app implements with its
//!   launch-as-the-person proof (`launch.rs`, `AsPerson`): never elevated. Nothing is typed for
//!   the person, no password is asked for, nothing is sent. The result stays needs-action until
//!   the outcome is observed (an export file appears, the person confirms).
//! - **The app's own export tool** ([`run_tool`]): only through a signed catalogue row naming a
//!   tool by its plain file name with typed arguments (never a shell string); the tool must be in
//!   the installed app's own folder, signed by the app's maker as the maker check finds it
//!   (`pctwin-maker`), and still the very bytes checked when it is started. It writes into a
//!   PCTwin-owned, owner-only place ([`crate::export::ExportSlot`]); its own output is never kept
//!   or logged (it can hold secrets). The export is observed, then imported by the app's own
//!   route: needs-action until that is observed too.

use std::ffi::OsString;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::time::Duration;

use pctwin_grant::{CaptureEffect, CaptureGrant, Now, PlanSeal, RootRef};
use pctwin_maker::run::{Call, Runner};
use pctwin_maker::{Bytes, Decision, Expected, Limits, Sha256, SystemChecks, Verdict};
use sha2::Digest;

use crate::catalogue::{Arg, NativeRoute};
use crate::classify::FolderReport;
use crate::component::{Component, Custody, Evidence};
use crate::export::{ExportSlot, Sealed};

/// What the app hands in to act as the person: opening a page in their browser, or an app.
pub trait Person {
    fn open_page(&self, url: &str) -> std::io::Result<()>;
    fn open_app(&self, program: &Path) -> std::io::Result<()>;
}

/// The person's step for one component.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Step {
    /// Nothing to do: it copies.
    None,
    /// Sign in to the service again (its page, when a row names it).
    SignIn { page: Option<String> },
    /// Bound to the old laptop or a device: link again, or enrol again (the maker's page when a
    /// row names it). Before the old laptop is wiped, this stays on the list.
    Relink { page: Option<String> },
    /// The app's own export: its export screen, the maker's page, or its export tool.
    Export { route: Option<NativeRoute> },
    /// What it is could not be established: the person looks at it.
    LookAtIt,
}

/// The person's step for `c`, found in `report`.
pub fn step_for(report: &FolderReport, c: &Component) -> Step {
    let row = report.row.as_ref();
    let page = row.and_then(|r| r.relink.clone());
    match c.custody {
        Custody::Portable => Step::None,
        Custody::ServiceHeld => Step::SignIn { page },
        Custody::MachineBound => Step::Relink { page },
        Custody::NeedsExport => Step::Export {
            route: row.and_then(|r| r.export.clone()),
        },
        Custody::Unresolved => Step::LookAtIt,
    }
}

/// How opening a step went. Never "done": the outcome is observed separately.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Opened {
    /// Opened for the person; the step stays needs-action until its outcome is observed.
    Opened,
    /// Nothing to open: the person is shown the step in words.
    InWords,
    /// It could not be opened, and why.
    NotOpened(String),
}

/// Opens what the person needs for `step`: a page, or the app (`app`, its program on this
/// laptop) at its export screen.
pub fn open(person: &dyn Person, step: &Step, app: Option<&Path>) -> Opened {
    let page = |url: &str| match person.open_page(url) {
        Ok(()) => Opened::Opened,
        Err(e) => Opened::NotOpened(e.kind().to_string()),
    };
    match step {
        Step::SignIn { page: Some(p) } | Step::Relink { page: Some(p) } => page(p),
        Step::Export {
            route: Some(NativeRoute::OpenPage { url }),
        } => page(url),
        Step::Export {
            route: Some(NativeRoute::OpenApp),
        } => match app {
            Some(program) => match person.open_app(program) {
                Ok(()) => Opened::Opened,
                Err(e) => Opened::NotOpened(e.kind().to_string()),
            },
            None => Opened::NotOpened("the app is not on this laptop".into()),
        },
        _ => Opened::InWords,
    }
}

/// Why the app's own export tool was not run.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ToolError {
    #[error("this permission is not for this step: {0}")]
    Denied(&'static str),
    #[error("no signed catalogue row names an export tool for this app")]
    NoTool,
    #[error("the export tool is not in the app's own folder")]
    NotThere,
    #[error("the export tool could not be checked: {0}")]
    NotChecked(String),
    #[error(
        "the export tool is not signed by the app's maker, or did not pass the system's checks: {0}"
    )]
    NotTheMakers(String),
    #[error("the export tool changed after it was checked")]
    Changed,
    #[error("the export tool did not finish: {0}")]
    DidNotRun(String),
    #[error("the export tool ended without writing its export")]
    NoExport,
}

/// The app's own export, written by its tool and observed: still to be imported by the app's own
/// route on the new laptop.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Exported {
    pub sealed: Sealed,
    /// The tool's exit code.
    pub code: Option<i32>,
}

/// The maker check of an export tool, bound to what the old laptop's copy of the app says its
/// maker is. [`MakerCheck`] is the real one.
pub trait ToolMaker {
    fn verdict(&self, tool: &Path) -> Result<Verdict, String>;
}

/// The maker check (`pctwin-maker`) with the system's own signature and malware checks.
pub struct MakerCheck<'a> {
    /// What the check must find: the old laptop's copy's maker, this laptop's system and
    /// architecture, the kinds of program accepted.
    pub expected: &'a Expected,
    /// PCTwin's own reader of the tool's insides: the parser process (`pctwin-parse`).
    pub reader: &'a dyn pctwin_maker::OwnReader,
    pub checks: &'a dyn SystemChecks,
    pub limits: &'a Limits,
}

impl ToolMaker for MakerCheck<'_> {
    fn verdict(&self, tool: &Path) -> Result<Verdict, String> {
        pctwin_maker::check(tool, self.expected, self.reader, self.checks, self.limits)
            .map_err(|e| e.to_string())
    }
}

/// Everything one run of an app's export tool needs.
pub struct ToolRun<'a> {
    pub seal: PlanSeal,
    pub root: RootRef,
    /// The installed app's own folder.
    pub app_folder: &'a Path,
    pub route: Option<&'a NativeRoute>,
    /// The maker check.
    pub maker: &'a dyn ToolMaker,
    pub runner: &'a dyn Runner,
    /// The tool's whole environment (nothing of PCTwin's own is passed on).
    pub env: Vec<(OsString, OsString)>,
    pub timeout: Duration,
    pub slot: &'a ExportSlot,
}

fn bytes_of(path: &Path) -> std::io::Result<Bytes> {
    let mut f = std::fs::File::open(path)?;
    let mut h = sha2::Sha256::new();
    let mut buf = vec![0u8; 1 << 16];
    let mut size = 0u64;
    loop {
        let n = f.read(&mut buf)?;
        if n == 0 {
            break;
        }
        h.update(&buf[..n]);
        size += n as u64;
    }
    Ok(Bytes {
        size,
        sha256: Sha256(h.finalize().into()),
    })
}

/// Runs the app's own export tool, as the person (this program's own rights), with the grant
/// for the app's own export (used up by the call).
pub fn run_tool(grant: CaptureGrant, run: &ToolRun<'_>, now: Now) -> Result<Exported, ToolError> {
    if grant.effect() != CaptureEffect::NativeExport {
        return Err(ToolError::Denied("it allows another kind of work"));
    }
    if grant.seal() != run.seal || !grant.roots().contains(&run.root) {
        return Err(ToolError::Denied("it is for another plan or folder"));
    }
    if grant.expiry().is_over(now) {
        return Err(ToolError::Denied("it has ended"));
    }
    let Some(NativeRoute::Tool { tool, args }) = run.route else {
        return Err(ToolError::NoTool);
    };
    let path: PathBuf = run.app_folder.join(tool);
    match std::fs::symlink_metadata(&path) {
        Ok(m) if m.is_file() => {}
        _ => return Err(ToolError::NotThere),
    }
    let verdict = run.maker.verdict(&path).map_err(ToolError::NotChecked)?;
    let decision = verdict.decide();
    if let Decision::NeedsAction { why } = &decision {
        let words: Vec<&str> = why.iter().map(|w| w.plain()).collect();
        return Err(ToolError::NotTheMakers(words.join("; ")));
    }
    // The very bytes checked, just before they run.
    let now_bytes = bytes_of(&path).map_err(|e| ToolError::NotChecked(e.kind().to_string()))?;
    if !pctwin_maker::approved_bytes_match(&decision, &now_bytes) {
        return Err(ToolError::Changed);
    }
    let call = Call {
        program: path,
        args: args
            .iter()
            .map(|a| match a {
                Arg::Switch(s) => OsString::from(s),
                Arg::OutputFile => run.slot.file().as_os_str().to_owned(),
            })
            .collect(),
        env: run.env.clone(),
        timeout: run.timeout,
        stdin: None,
        cwd: run.slot.file().parent().map(Path::to_path_buf),
    };
    // The tool's own output can hold secrets: only its exit code is kept.
    let ran = run
        .runner
        .run(&call)
        .map_err(|e| ToolError::DidNotRun(e.to_string()))?;
    match run.slot.seal() {
        Ok(sealed) if sealed.len > 0 => Ok(Exported {
            sealed,
            code: ran.code,
        }),
        _ => Err(ToolError::NoExport),
    }
}

/// Whether a component's evidence shows it is a sign-in or device link (for the before-wipe list).
pub fn is_access(c: &Component) -> bool {
    c.evidence.iter().any(|e| {
        matches!(
            e,
            Evidence::ServiceAccount
                | Evidence::SessionStore
                | Evidence::HardwareKeyStub
                | Evidence::MachineBoundProtection
                | Evidence::CredentialValues
        )
    })
}
