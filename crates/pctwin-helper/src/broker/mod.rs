//! The permission broker (Phase 2.2): the helper's fixed privileged actions for the Phase 2
//! roles, and the checks every request passes before anything changes.
//!
//! The helper was started through the system's own administrator prompt with the approved plan's
//! generation and digest on its command line ([`LaunchSeal`]). The app then sends the plan
//! itself ([`Broker::seal`]); the helper keeps it only if it matches. After that, each request
//! ([`BrokerRequest`]) goes through, in order (plan section 4):
//!
//! 1. the protocol version, the session still open and not revoked, the plan sealed, its lease
//!    not over (each action renews the lease) and the session's fixed deadline not passed
//!    ([`pctwin_grant::MAX_SESSION`] from sealing; nothing renews it);
//! 2. the request's one-use nonce (a request said twice is refused);
//! 3. the role it comes from: it must be one that may ask for privileged work, and the one that
//!    owns this action;
//! 4. the plan generation and digest against the sealed plan, and the action, field for field,
//!    against the plan's operation with that ID (whatever the app says about approval is never
//!    read: there is no such field);
//! 5. whether an administrator is needed at all (if not, the app does it with the person's own
//!    rights);
//! 6. the action's one key, made from the plan, its ID and the action, and what the undo journal
//!    holds for it: a finished result is handed back, not done again; an unfinished one is first
//!    checked against what is really on the laptop;
//! 7. the exact object (a package is opened and hashed again);
//! 8. the system's own approval ([`NativeAuthority`]): not approved, or no answer, is no;
//! 9. everything again after the approval, since the person may have taken a while: revocation,
//!    the lease, the package, opened and hashed once more and then held unchanged; and, for a
//!    package, the helper's own maker check of that held copy ([`maker`]): the app's answer is
//!    never used for an install with an administrator's rights;
//! 10. the intent written durably to the journal: if it cannot be, nothing changes;
//! 11. revocation once more, then the compiled adapter, which reports what it observed;
//! 12. the outcome written to the journal before the answer is sent, so an app that went away
//!     still finds it.
//!
//! Errors carry the system's code only, never its message: a message can hold a path or a
//! secret.

pub mod actions;
pub mod installs;
pub mod journal;
pub mod maker;
pub mod receipt;
pub mod restore;
pub mod setting_rows;
pub mod staging;
pub mod tool;
pub mod uninstall;
pub mod watch;

#[cfg(test)]
mod tests;

use std::collections::BTreeSet;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use serde::{Deserialize, Serialize};

use pctwin_grant::{Audience, Expiry, MAX_LIFETIME, Now};

use actions::{
    Action, ApprovedPlan, Hex16, Hex32, Id, InstallFromMacAppStore, InstallVerifiedPackage,
    InstalledProduct, NamedSetting, UninstallWindowsInstallerProduct,
};
use receipt::{Outcome, Receipt};
use restore::RestorePointResult;
use staging::{Admission, StageError, Staging, VerifiedArtifact};

/// The broker's protocol version. An app that speaks another is refused.
pub const PROTOCOL: u32 = 1;
/// Most requests one session may make.
pub const MAX_REQUESTS: usize = 4096;

/// The approved plan's generation and digest, from the helper's command line.
///
/// What this does not prove (security review, 26 October 2026): the app chooses what it puts on
/// the command line, and the system's administrator prompt does not show the digest to the
/// person, so the digest only binds the helper to the plan the app named, not to one the person
/// saw. The real wall is the helper's fixed set of three actions (a verified package, a named
/// setting from a short list, a restore point) and its own checks of each; later roles must not
/// treat a sealed plan as the person's approval.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LaunchSeal {
    pub generation: u64,
    pub digest: Hex32,
}

impl LaunchSeal {
    /// `<generation>:<digest in hexadecimal>`, as the command line gives it.
    pub fn parse(s: &str) -> Option<Self> {
        let (generation, digest) = s.split_once(':')?;
        if generation.is_empty() || !generation.bytes().all(|b| b.is_ascii_digit()) {
            return None;
        }
        Some(Self {
            generation: generation.parse().ok()?,
            digest: Hex32::parse(digest)?,
        })
    }
}

/// One request for a privileged action.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct BrokerRequest {
    pub protocol: u32,
    pub audience: Audience,
    pub request_id: Id,
    pub plan_generation: u64,
    pub plan_digest: Hex32,
    pub action_id: Id,
    pub action: Action,
    pub nonce: Hex16,
}

/// Why the broker did not act. Each says what happened in plain words; none carries the system's
/// message.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, thiserror::Error)]
#[serde(rename_all = "kebab-case")]
pub enum Denial {
    #[error("this version of PCTwin and its helper do not match; restart PCTwin")]
    Incompatible,
    #[error("the helper's work for this move is over")]
    Closed,
    #[error("the plan has not been given to the helper yet")]
    NotSealed,
    #[error("that is not the plan approved for this move")]
    WrongPlan,
    #[error("the permission ran out; PCTwin asks again")]
    Expired,
    #[error("the move was stopped, so nothing more was changed")]
    Revoked,
    #[error("that request was already made")]
    Replayed,
    #[error("too many requests in one move")]
    TooMany,
    #[error("that part of PCTwin may not ask for this")]
    WrongAudience,
    #[error("the plan changed since it was approved; approve it again")]
    StalePlan,
    #[error("that is not in the approved plan")]
    NotInPlan,
    #[error("this needs no administrator; PCTwin does it with your own rights")]
    NotNeeded,
    #[error("the package is not the approved one any more, so it was not used")]
    Unsafe,
    #[error("someone has the package open, so it was not used; try again")]
    InUse,
    #[error(
        "the package is not from the app's maker, or Windows' own checks did not pass, so it was not installed"
    )]
    NotTheMaker,
    #[error("the administrator did not approve, so nothing was changed")]
    NativeDenied,
    #[error("the system's approval was not answered, so nothing was changed")]
    NativeTimedOut,
    #[error("PCTwin could not record the change first, so it did not make it")]
    JournalUnavailable,
    #[error("an earlier attempt may have been partly done; PCTwin checks it before trying again")]
    RecoveryRequired,
    #[error("this laptop cannot do that")]
    NotAvailable,
    #[error("PCTwin's own record does not show it installed that app, so it does not remove it")]
    NotInstalledByPctwin,
}

/// The system's own approval for one action.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Consent {
    Granted,
    Denied,
    TimedOut,
}

/// Asks the system to approve an action, with its own window. It never widens what the plan
/// allows: it is asked only for an action already checked against the plan.
pub trait NativeAuthority {
    fn obtain(&mut self, action: &Action) -> Consent;
}

/// Windows and Linux: the administrator approved this helper, for this move, in the system's own
/// prompt when it was started (one approval per move, as decided); nothing more is asked.
pub struct SessionElevation;

impl NativeAuthority for SessionElevation {
    fn obtain(&mut self, _: &Action) -> Consent {
        Consent::Granted
    }
}

/// Where a change is in the undo journal.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct JournalRef(pub u64);

/// A change about to be made, written durably before it is.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Intent {
    pub idempotency: Hex32,
    pub plan_generation: u64,
    pub plan_digest: Hex32,
    pub action_id: Id,
    pub action: Action,
}

/// What the journal holds for an action's key.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Recorded {
    Nothing,
    /// Its intent, with no outcome: the helper stopped before it could write one.
    Unfinished(JournalRef),
    Finished(JournalRef, Outcome),
}

/// The journal could not be read or written durably.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct JournalUnavailable;

/// What the broker needs of the undo journal (2.3 implements it on `pctwin-journal`).
pub trait IntentJournal {
    fn lookup(&mut self, idempotency: &Hex32) -> Result<Recorded, JournalUnavailable>;
    /// Writes the intent durably; only then may the change be made.
    fn record_intent(&mut self, intent: &Intent) -> Result<JournalRef, JournalUnavailable>;
    fn record_outcome(
        &mut self,
        at: JournalRef,
        outcome: &Outcome,
    ) -> Result<(), JournalUnavailable>;
}

/// No journal: nothing can be recorded, so nothing is changed (for a helper whose journal could
/// not be opened; the real one is [`journal::JournalIntents`]).
pub struct NoJournal;

impl IntentJournal for NoJournal {
    fn lookup(&mut self, _: &Hex32) -> Result<Recorded, JournalUnavailable> {
        Err(JournalUnavailable)
    }
    fn record_intent(&mut self, _: &Intent) -> Result<JournalRef, JournalUnavailable> {
        Err(JournalUnavailable)
    }
    fn record_outcome(&mut self, _: JournalRef, _: &Outcome) -> Result<(), JournalUnavailable> {
        Err(JournalUnavailable)
    }
}

/// The system's own words about a failure: kept in the helper, never sent, never shown by
/// `Debug`.
#[derive(Clone, PartialEq, Eq)]
pub struct NativeText(String);

impl NativeText {
    pub fn new(text: impl Into<String>) -> Self {
        Self(text.into())
    }
}

impl std::fmt::Debug for NativeText {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("NativeText(redacted)")
    }
}

/// What an adapter observed.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Effect {
    Done {
        restart_needed: bool,
    },
    /// A Windows Installer package installed, its product seen to appear ([`Outcome::Installed`]).
    Installed {
        restart_needed: bool,
        product: actions::InstalledProduct,
    },
    Uninstall(uninstall::UninstallResult),
    RestorePoint(RestorePointResult),
    Partial {
        code: i64,
        native: NativeText,
    },
    Failed {
        code: i64,
        native: NativeText,
    },
}

impl Effect {
    /// The outcome sent to the app: codes only, the system's words left behind.
    fn outcome(self) -> Outcome {
        match self {
            Effect::Done { restart_needed } => Outcome::Done { restart_needed },
            Effect::Installed {
                restart_needed,
                product,
            } => Outcome::Installed {
                restart_needed,
                product,
            },
            Effect::Uninstall(u) => Outcome::Uninstall(u),
            Effect::RestorePoint(r) => Outcome::RestorePoint(r),
            Effect::Partial { code, .. } => Outcome::Partial { code },
            Effect::Failed { code, .. } => Outcome::Failed { code },
        }
    }
}

/// Whether a change is really in place, checked on the system itself.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Observed {
    Present,
    Absent,
    Unknown,
}

/// Set when the move is stopped. An adapter in the middle of a system action that cannot be
/// stopped safely (an installer) finishes it and reports what really happened; one that can stop
/// between steps does so.
#[derive(Clone, Default, Debug)]
pub struct Revoker(Arc<AtomicBool>);

impl Revoker {
    pub fn revoke(&self) {
        self.0.store(true, Ordering::SeqCst);
    }

    pub fn is_revoked(&self) -> bool {
        self.0.load(Ordering::SeqCst)
    }
}

/// The compiled adapters that make the changes.
pub trait Adapters {
    fn install(
        &mut self,
        package: &VerifiedArtifact,
        action: &InstallVerifiedPackage,
        stop: &Revoker,
    ) -> Effect;
    fn apply_setting(&mut self, setting: &NamedSetting, stop: &Revoker) -> Effect;
    fn restore_point(&mut self, stop: &Revoker) -> Effect;
    /// Installs one app from the Mac App Store (only on a Mac, with a root-owned `mas`).
    fn install_from_store(&mut self, action: &InstallFromMacAppStore, stop: &Revoker) -> Effect {
        let _ = (action, stop);
        Effect::Failed {
            code: NOT_AVAILABLE,
            native: NativeText::new(""),
        }
    }
    /// Removes `installed`, the product the helper's own journal recorded installing for
    /// `action` ([`uninstall`]): only while it is still there at that version.
    fn uninstall(
        &mut self,
        action: &UninstallWindowsInstallerProduct,
        installed: &actions::InstalledProduct,
        stop: &Revoker,
    ) -> Effect {
        let _ = (action, installed, stop);
        Effect::Failed {
            code: NOT_AVAILABLE,
            native: NativeText::new(""),
        }
    }
    /// Whether `action` is really in place now (after the helper stopped mid-way).
    fn observe(&mut self, action: &Action) -> Observed;
    /// The helper's own maker check of `package`, held, for `action` ([`maker::check_package`]
    /// in the real helper).
    fn check_maker(
        &mut self,
        package: &VerifiedArtifact,
        action: &InstallVerifiedPackage,
    ) -> maker::MakerCheck;
}

/// The adapters this build has: installs (2.1: Windows Installer packages and installer programs
/// of a known kind, and the Mac App Store; [`installs`]), and Windows' own restore point.
/// Settings come with 2.4, so they say "not available".
pub struct BuiltInAdapters;

/// The code for "this laptop cannot do that yet".
pub const NOT_AVAILABLE: i64 = -2;

impl Adapters for BuiltInAdapters {
    fn check_maker(
        &mut self,
        package: &VerifiedArtifact,
        action: &InstallVerifiedPackage,
    ) -> maker::MakerCheck {
        maker::check_package(package, action, &pctwin_maker::Limits::default())
    }

    /// The installer runs to its end once started, whatever stop arrives meanwhile: stopping one
    /// part way can leave an app half installed. Its real result is reported. For a Windows
    /// Installer package naming its product code, Windows' list of installed apps is read just
    /// before and just after, so undo can later remove what this install added, and only that.
    fn install(
        &mut self,
        package: &VerifiedArtifact,
        action: &InstallVerifiedPackage,
        _: &Revoker,
    ) -> Effect {
        let before = installs::watches_product(action)
            .then(uninstall::installed_products)
            .flatten();
        let effect = installs::install(package, action);
        installs::with_product(
            effect,
            action,
            before.as_ref(),
            &mut uninstall::installed_products,
        )
    }

    /// `msiexec /x` runs to its end once started, like an installer; its real result is
    /// reported.
    fn uninstall(
        &mut self,
        _: &UninstallWindowsInstallerProduct,
        installed: &actions::InstalledProduct,
        _: &Revoker,
    ) -> Effect {
        #[cfg(windows)]
        {
            uninstall::uninstall_with(
                installed,
                &mut uninstall::installed_products,
                &mut uninstall::run_msiexec,
            )
        }
        #[cfg(not(windows))]
        {
            let _ = installed;
            Effect::Failed {
                code: NOT_AVAILABLE,
                native: NativeText::new(""),
            }
        }
    }

    fn install_from_store(&mut self, action: &InstallFromMacAppStore, _: &Revoker) -> Effect {
        installs::install_from_store(action)
    }

    fn apply_setting(&mut self, _: &NamedSetting, _: &Revoker) -> Effect {
        Effect::Failed {
            code: NOT_AVAILABLE,
            native: NativeText::new(""),
        }
    }

    fn restore_point(&mut self, _: &Revoker) -> Effect {
        #[cfg(windows)]
        {
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map_or(0, |d| i64::try_from(d.as_secs()).unwrap_or(i64::MAX));
            Effect::RestorePoint(restore::create_honestly(
                &mut restore::WindowsRestorePoints,
                now,
            ))
        }
        #[cfg(not(windows))]
        Effect::RestorePoint(RestorePointResult::NotMade {
            reason: restore::NotMadeReason::NotWindows,
        })
    }

    /// A restore point a crash interrupted is asked for again: Windows makes at most one a day
    /// for programs and says so, so asking twice never makes a second one silently, and the
    /// answer stays honest. Installs and settings cannot be looked at by this build: unknown, so
    /// the broker asks for recovery instead of acting.
    fn observe(&mut self, action: &Action) -> Observed {
        match action {
            Action::CreateRestorePoint(_) => Observed::Absent,
            #[cfg(windows)]
            Action::InstallVerifiedPackage(p) => installs::observed(
                p,
                installs::running_command_lines(),
                installs::installed_package_names(),
            ),
            #[cfg(not(windows))]
            Action::InstallVerifiedPackage(_) => Observed::Unknown,
            #[cfg(windows)]
            Action::UninstallWindowsInstallerProduct(u) => uninstall::observed(
                &u.product_code,
                installs::running_command_lines(),
                uninstall::installed_products().as_ref(),
            ),
            #[cfg(not(windows))]
            Action::UninstallWindowsInstallerProduct(_) => Observed::Unknown,
            Action::InstallFromMacAppStore(_) | Action::ApplyNamedSetting(_) => Observed::Unknown,
        }
    }
}

/// What the broker answers.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Answer {
    Done(Box<Receipt>),
    Denied(Denial),
}

/// The broker for one session.
pub struct Broker<'a> {
    launch: LaunchSeal,
    plan: Option<ApprovedPlan>,
    lease: Option<Expiry>,
    /// The session's fixed end, set when the plan is sealed and never renewed.
    deadline: Option<Expiry>,
    nonces: BTreeSet<Hex16>,
    closed: bool,
    revoker: Revoker,
    receipt_key: [u8; 32],
    staging: Staging,
    /// A package arriving from the app, for the sealed plan.
    admission: Option<Admission>,
    authority: &'a mut dyn NativeAuthority,
    journal: &'a mut dyn IntentJournal,
    adapters: &'a mut dyn Adapters,
    clock: &'a dyn Fn() -> Now,
}

impl<'a> Broker<'a> {
    pub fn new(
        launch: LaunchSeal,
        session_token: &str,
        staging: Staging,
        authority: &'a mut dyn NativeAuthority,
        journal: &'a mut dyn IntentJournal,
        adapters: &'a mut dyn Adapters,
        clock: &'a dyn Fn() -> Now,
    ) -> Self {
        Self {
            launch,
            plan: None,
            lease: None,
            deadline: None,
            nonces: BTreeSet::new(),
            closed: false,
            revoker: Revoker::default(),
            receipt_key: receipt::receipt_key(session_token),
            staging,
            admission: None,
            authority,
            journal,
            adapters,
            clock,
        }
    }

    /// A handle that stops the move from elsewhere (the app's stop, a sleep, a sign-out, a switch
    /// of user): no new change starts after it, ever, in this session.
    pub fn revoker(&self) -> Revoker {
        self.revoker.clone()
    }

    /// The helper's packages.
    pub fn staging(&mut self) -> &mut Staging {
        &mut self.staging
    }

    /// Starts taking package `artifact` from the app: `size` bytes whose SHA-256 is `sha256`.
    /// Only for a package an install of the sealed plan names by exactly that handle and those
    /// bytes, once, and one at a time.
    pub fn stage_begin(&mut self, artifact: u64, sha256: Hex32, size: u64) -> Result<(), Denial> {
        self.still_open()?;
        let plan = self.plan.as_ref().ok_or(Denial::NotSealed)?;
        let named = plan.operations.iter().any(|o| {
            matches!(&o.action, Action::InstallVerifiedPackage(p)
                if p.artifact == artifact && p.sha256 == sha256)
        });
        if !named {
            return Err(Denial::NotInPlan);
        }
        if self.admission.is_some() || self.staging.holds(artifact) {
            return Err(Denial::Replayed);
        }
        self.admission = Some(
            self.staging
                .begin(artifact, sha256.0, size)
                .map_err(|_| Denial::Unsafe)?,
        );
        Ok(())
    }

    /// Drops the package being taken (nothing of it is kept).
    pub fn stage_abort(&mut self) {
        self.admission = None;
    }

    /// The next bytes of the package being taken. `Ok(Some(handle))` once every byte has come
    /// and they are the approved ones.
    pub fn stage_bytes(&mut self, bytes: &[u8]) -> Result<Option<u64>, Denial> {
        if let Err(e) = self.still_open() {
            self.admission = None;
            return Err(e);
        }
        let Some(a) = self.admission.as_mut() else {
            return Err(Denial::NotInPlan);
        };
        if a.write(bytes).is_err() {
            self.admission = None;
            return Err(Denial::Unsafe);
        }
        if !a.complete() {
            return Ok(None);
        }
        let Some(a) = self.admission.take() else {
            return Err(Denial::Unsafe);
        };
        self.staging.finish(a).map(Some).map_err(|_| Denial::Unsafe)
    }

    pub fn is_closed(&self) -> bool {
        self.closed
    }

    /// Keeps the approved plan, once, if it is the one named on the command line.
    pub fn seal(&mut self, plan: ApprovedPlan) -> Result<(), Denial> {
        if self.closed {
            return Err(Denial::Closed);
        }
        if self.plan.is_some() {
            return Err(Denial::Replayed);
        }
        if !plan.is_well_formed()
            || plan.generation != self.launch.generation
            || plan.digest() != self.launch.digest
        {
            return Err(Denial::WrongPlan);
        }
        let now = (self.clock)();
        self.lease = Some(Expiry::after(MAX_LIFETIME, now).map_err(|_| Denial::Expired)?);
        self.deadline = Some(Expiry::session_from(now).map_err(|_| Denial::Expired)?);
        self.plan = Some(plan);
        Ok(())
    }

    /// Ends the session: no more actions, the plan and the packages gone.
    pub fn close(&mut self) {
        self.closed = true;
        self.plan = None;
        self.lease = None;
        self.deadline = None;
        self.admission = None;
        self.staging.clear();
    }

    /// Whether the session may still act now; if not, it is closed for good.
    fn still_open(&mut self) -> Result<(), Denial> {
        if self.closed {
            return Err(Denial::Closed);
        }
        if self.revoker.is_revoked() {
            self.close();
            return Err(Denial::Revoked);
        }
        let now = (self.clock)();
        match (self.lease, self.deadline) {
            (Some(lease), Some(deadline)) if lease.is_over(now) || deadline.is_over(now) => {
                self.close();
                Err(Denial::Expired)
            }
            (Some(_), Some(_)) => Ok(()),
            _ => Err(Denial::NotSealed),
        }
    }

    fn idempotency(&self, request: &BrokerRequest) -> Hex32 {
        let mut h = blake3::Hasher::new_derive_key("PCTwin broker idempotency 2026-10-26 v1");
        h.update(&request.plan_digest.0);
        h.update(&(request.action_id.as_str().len() as u64).to_be_bytes());
        h.update(request.action_id.as_str().as_bytes());
        h.update(&request.action.identity().0);
        Hex32(*h.finalize().as_bytes())
    }

    fn receipt(
        &self,
        request: &BrokerRequest,
        idempotency: Hex32,
        outcome: Outcome,
        journal: JournalRef,
        recorded: bool,
        replayed: bool,
    ) -> Answer {
        Answer::Done(Box::new(
            Receipt {
                request_id: request.request_id.clone(),
                action_id: request.action_id.clone(),
                plan_generation: request.plan_generation,
                plan_digest: request.plan_digest,
                idempotency,
                outcome,
                journal: journal.0,
                recorded,
                replayed,
                mac: Hex32::default(),
            }
            .signed(&self.receipt_key),
        ))
    }

    /// The package of an install, opened and checked now.
    fn verify_object(&self, action: &Action) -> Result<Option<VerifiedArtifact>, Denial> {
        match action {
            Action::InstallVerifiedPackage(p) => self
                .staging
                .open_verified(p.artifact, p.sha256.0)
                .map(Some)
                .map_err(|e| match e {
                    StageError::InUse => Denial::InUse,
                    _ => Denial::Unsafe,
                }),
            _ => Ok(None),
        }
    }

    /// For a removal: the product this helper's own journal recorded installing, finished, under
    /// the install's key the action names, and only if it is this very product code. Anything
    /// else (no such install, unfinished, failed, an install that recorded no product or another
    /// one, a journal that cannot be read) is refused before anything starts. Other actions:
    /// nothing to look up.
    fn recorded_install(&mut self, action: &Action) -> Result<Option<InstalledProduct>, Denial> {
        let Action::UninstallWindowsInstallerProduct(u) = action else {
            return Ok(None);
        };
        match self
            .journal
            .lookup(&u.installed_by)
            .map_err(|_| Denial::JournalUnavailable)?
        {
            Recorded::Finished(_, Outcome::Installed { product, .. })
                if product.code == u.product_code =>
            {
                Ok(Some(product))
            }
            _ => Err(Denial::NotInstalledByPctwin),
        }
    }

    /// Checks `request` and, if everything holds, makes the change.
    pub fn handle(&mut self, request: &BrokerRequest) -> Answer {
        match self.try_handle(request) {
            Ok(answer) => answer,
            Err(denial) => Answer::Denied(denial),
        }
    }

    fn try_handle(&mut self, request: &BrokerRequest) -> Result<Answer, Denial> {
        // 1.
        if request.protocol != PROTOCOL {
            return Err(Denial::Incompatible);
        }
        self.still_open()?;
        // 2. Spent whether or not the request is then refused.
        if self.nonces.len() >= MAX_REQUESTS {
            return Err(Denial::TooMany);
        }
        if !self.nonces.insert(request.nonce) {
            return Err(Denial::Replayed);
        }
        // 3.
        if !request.audience.may_ask_privileged() || request.audience != request.action.audience() {
            return Err(Denial::WrongAudience);
        }
        // 4.
        let plan = self.plan.as_ref().ok_or(Denial::NotSealed)?;
        if request.plan_generation != plan.generation || request.plan_digest != self.launch.digest {
            return Err(Denial::StalePlan);
        }
        match plan.operation(&request.action_id) {
            Some(op) if op.action == request.action => {}
            _ => return Err(Denial::NotInPlan),
        }
        // 5.
        if !request.action.needs_admin() {
            return Err(Denial::NotNeeded);
        }
        // 5b. Removing an app: only one this helper's own journal recorded installing.
        self.recorded_install(&request.action)?;
        // 6.
        let idempotency = self.idempotency(request);
        let earlier = match self
            .journal
            .lookup(&idempotency)
            .map_err(|_| Denial::JournalUnavailable)?
        {
            Recorded::Finished(at, outcome) => {
                return Ok(self.receipt(request, idempotency, outcome, at, true, true));
            }
            Recorded::Unfinished(at) => match self.adapters.observe(&request.action) {
                Observed::Present => {
                    let outcome = Outcome::AlreadyInPlace;
                    let recorded = self.journal.record_outcome(at, &outcome).is_ok();
                    return Ok(self.receipt(request, idempotency, outcome, at, recorded, true));
                }
                Observed::Absent => Some(at),
                Observed::Unknown => return Err(Denial::RecoveryRequired),
            },
            Recorded::Nothing => None,
        };
        // 7.
        drop(self.verify_object(&request.action)?);
        // 8.
        match self.authority.obtain(&request.action) {
            Consent::Granted => {}
            Consent::Denied => return Err(Denial::NativeDenied),
            Consent::TimedOut => return Err(Denial::NativeTimedOut),
        }
        // 9. Everything again: the approval may have taken a while.
        self.still_open()?;
        let installed = self.recorded_install(&request.action)?;
        let package = self.verify_object(&request.action)?;
        if let (Action::InstallVerifiedPackage(p), Some(held)) = (&request.action, &package)
            && self.adapters.check_maker(held, p) != maker::MakerCheck::Approved
        {
            return Err(Denial::NotTheMaker);
        }
        // 10.
        let at = match earlier {
            Some(at) => at,
            None => self
                .journal
                .record_intent(&Intent {
                    idempotency,
                    plan_generation: request.plan_generation,
                    plan_digest: request.plan_digest,
                    action_id: request.action_id.clone(),
                    action: request.action.clone(),
                })
                .map_err(|_| Denial::JournalUnavailable)?,
        };
        // 11.
        if self.revoker.is_revoked() {
            self.close();
            return Err(Denial::Revoked);
        }
        let stop = self.revoker.clone();
        let effect = match (&request.action, &package) {
            (Action::InstallVerifiedPackage(p), Some(package)) => {
                self.adapters.install(package, p, &stop)
            }
            (Action::InstallFromMacAppStore(a), _) => self.adapters.install_from_store(a, &stop),
            (Action::ApplyNamedSetting(s), _) => self.adapters.apply_setting(&s.setting, &stop),
            (Action::CreateRestorePoint(_), _) => self.adapters.restore_point(&stop),
            (Action::UninstallWindowsInstallerProduct(u), _) => match &installed {
                Some(installed) => self.adapters.uninstall(u, installed, &stop),
                None => return Err(Denial::NotInstalledByPctwin),
            },
            (Action::InstallVerifiedPackage(_), None) => return Err(Denial::Unsafe),
        };
        drop(package);
        // 12.
        let outcome = effect.outcome();
        let recorded = self.journal.record_outcome(at, &outcome).is_ok();
        if self.revoker.is_revoked() {
            // Stopped during the action: this result is reported, nothing new starts.
            self.close();
        } else {
            self.lease =
                Some(Expiry::after(MAX_LIFETIME, (self.clock)()).map_err(|_| Denial::Expired)?);
        }
        Ok(self.receipt(request, idempotency, outcome, at, recorded, false))
    }
}
