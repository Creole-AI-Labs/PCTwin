//! Each Phase 2 role's permission as its own Rust type (Phase 2 adoption, change 1: real walls,
//! not paperwork).
//!
//! Inside one program, roles do not hand each other tokens or signed receipts: code could simply
//! ignore them. Instead each role's permission (a *grant*) is its own type, and only that role's
//! *key* can make one:
//!
//! | Role | Key | Grant | What it may cover |
//! |---|---|---|---|
//! | 2.1 install apps | [`InstallKey`] | [`InstallGrant`] | one verified package |
//! | 2.4 settings | [`SettingKey`] | [`SettingGrant`] | one named setting |
//! | 2.5 private data | [`CaptureKey`] | [`CaptureGrant`] | read, import or the app's own export, in named roots |
//! | 2.5 secrets | [`CaptureKey`] | [`SecretGrant`] | read or land the exact secret components the person chose |
//! | 2.3 undo | [`UndoKey`] | [`UndoGrant`] | reverse, reconcile, a restore point |
//! | 2.7 clean-up | [`CleanupKey`] | [`CleanupGrant`] | remove PCTwin's own things, sanitise |
//! | 2.8 screens | none | none | nothing privileged |
//!
//! The keys are made once per program, by [`RoleKeys::mint`], and the program's starting code
//! gives each role its own key and nothing else. A key cannot be made any other way, copied or
//! turned into another key; a grant cannot be made without its key, copied, or turned into another
//! role's grant. There is no grant that covers everything, so the 2.6 move planner cannot make
//! one either.
//!
//! Every grant names the approved plan ([`PlanSeal`]: its generation and digest), the exact things
//! it covers ([`ObjectRef`] or [`RootRef`]), its effect, and when it ends ([`Expiry`], at most
//! [`MAX_LIFETIME`]). A grant is used up by the call it is given to.
//!
//! What this does not do: it is a wall inside one program, not a sandbox. Code that can write to
//! memory can do anything; that is why downloading and opening untrusted files run in a separate
//! locked-down process, and why the administrator helper checks every request again on its own.
//!
//! # The walls, proven by the compiler
//!
//! A 2.5 grant is accepted where a 2.5 grant is needed:
//!
//! ```
//! use pctwin_grant::*;
//! use std::time::Duration;
//! fn needs_capture(_: CaptureGrant) {}
//! let keys = RoleKeys::mint().unwrap();
//! let seal = PlanSeal::new(1, [7; 32]);
//! let grant = keys.capture
//!     .grant(seal, vec![RootRef::new(1)], CaptureEffect::ReadPrivate, Duration::from_secs(60), Now::real())
//!     .unwrap();
//! needs_capture(grant);
//! ```
//!
//! A 2.1 grant is not:
//!
//! ```compile_fail
//! use pctwin_grant::*;
//! use std::time::Duration;
//! fn needs_capture(_: CaptureGrant) {}
//! let keys = RoleKeys::mint().unwrap();
//! let seal = PlanSeal::new(1, [7; 32]);
//! let grant = keys.install
//!     .grant(seal, ObjectRef::new(1, [9; 32]), Duration::from_secs(60), Now::real())
//!     .unwrap();
//! needs_capture(grant);
//! ```
//!
//! Nor can a 2.1 grant be turned into one:
//!
//! ```compile_fail
//! use pctwin_grant::*;
//! use std::time::Duration;
//! let keys = RoleKeys::mint().unwrap();
//! let seal = PlanSeal::new(1, [7; 32]);
//! let grant = keys.install
//!     .grant(seal, ObjectRef::new(1, [9; 32]), Duration::from_secs(60), Now::real())
//!     .unwrap();
//! let _stolen: CaptureGrant = grant.into();
//! ```
//!
//! The 2.1 key cannot make a 2.5 grant (the same call with the 2.5 key compiles, above):
//!
//! ```compile_fail
//! use pctwin_grant::*;
//! use std::time::Duration;
//! let keys = RoleKeys::mint().unwrap();
//! let seal = PlanSeal::new(1, [7; 32]);
//! let _g = CaptureKey::grant(&keys.install, seal, vec![RootRef::new(1)],
//!     CaptureEffect::ReadPrivate, Duration::from_secs(60), Now::real());
//! ```
//!
//! A grant cannot be written out by hand, so no role builds one without its key:
//!
//! ```compile_fail
//! use pctwin_grant::*;
//! let _g = CaptureGrant { seal: PlanSeal::new(1, [7; 32]), roots: vec![], effect: CaptureEffect::Import, expiry: todo!() };
//! ```
//!
//! Nor can a key:
//!
//! ```compile_fail
//! use pctwin_grant::*;
//! let _k = CaptureKey { _sealed: todo!() };
//! ```
//!
//! A grant cannot be copied, so one use is one use:
//!
//! ```compile_fail
//! use pctwin_grant::*;
//! use std::time::Duration;
//! let keys = RoleKeys::mint().unwrap();
//! let grant = keys.install
//!     .grant(PlanSeal::new(1, [7; 32]), ObjectRef::new(1, [9; 32]), Duration::from_secs(60), Now::real())
//!     .unwrap();
//! let _again = grant.clone();
//! ```
//!
//! Nor can a key be copied and handed to a second role:
//!
//! ```compile_fail
//! use pctwin_grant::*;
//! let keys = RoleKeys::mint().unwrap();
//! let _second = keys.capture.clone();
//! ```
//!
//! A 2.5 secret is read only with a secret grant, which names the exact components the person
//! chose; an ordinary 2.5 grant (one for a mail archive, say) is not one (acceptance case T25-005):
//!
//! ```compile_fail
//! use pctwin_grant::*;
//! use std::time::Duration;
//! fn reads_secret(_: SecretGrant) {}
//! let keys = RoleKeys::mint().unwrap();
//! let mail = keys.capture
//!     .grant(PlanSeal::new(1, [7; 32]), vec![RootRef::new(1)], CaptureEffect::ReadPrivate,
//!         Duration::from_secs(60), Now::real())
//!     .unwrap();
//! reads_secret(mail);
//! ```
//!
//! nor can one be turned into it:
//!
//! ```compile_fail
//! use pctwin_grant::*;
//! use std::time::Duration;
//! let keys = RoleKeys::mint().unwrap();
//! let mail = keys.capture
//!     .grant(PlanSeal::new(1, [7; 32]), vec![RootRef::new(1)], CaptureEffect::ReadPrivate,
//!         Duration::from_secs(60), Now::real())
//!     .unwrap();
//! let _s: SecretGrant = mail.into();
//! ```
//!
//! and no other role's key makes one (the 2.5 key does, below):
//!
//! ```compile_fail
//! use pctwin_grant::*;
//! use std::time::Duration;
//! let keys = RoleKeys::mint().unwrap();
//! let _s = CaptureKey::secret(&keys.install, PlanSeal::new(1, [7; 32]),
//!     vec![ObjectRef::new(1, [9; 32])], SecretEffect::Read, Duration::from_secs(60), Now::real());
//! ```
//!
//! ```
//! use pctwin_grant::*;
//! use std::time::Duration;
//! fn reads_secret(_: SecretGrant) {}
//! let keys = RoleKeys::mint().unwrap();
//! let s = keys.capture.secret(PlanSeal::new(1, [7; 32]), vec![ObjectRef::new(1, [9; 32])],
//!     SecretEffect::Read, Duration::from_secs(60), Now::real()).unwrap();
//! reads_secret(s);
//! ```
//!
//! A secret grant cannot be copied or written out by hand:
//!
//! ```compile_fail
//! use pctwin_grant::*;
//! use std::time::Duration;
//! let keys = RoleKeys::mint().unwrap();
//! let s = keys.capture.secret(PlanSeal::new(1, [7; 32]), vec![ObjectRef::new(1, [9; 32])],
//!     SecretEffect::Read, Duration::from_secs(60), Now::real()).unwrap();
//! let _again = s.clone();
//! ```
//!
//! ```compile_fail
//! use pctwin_grant::*;
//! let _s = SecretGrant { seal: PlanSeal::new(1, [7; 32]), components: vec![], effect: SecretEffect::Read, expiry: todo!() };
//! ```
//!
//! An undo grant carries undo's effects only; a clean-up effect does not fit:
//!
//! ```compile_fail
//! use pctwin_grant::*;
//! use std::time::Duration;
//! let keys = RoleKeys::mint().unwrap();
//! let _g = keys.undo.grant(PlanSeal::new(1, [7; 32]), vec![ObjectRef::new(1, [9; 32])],
//!     CleanupEffect::Sanitise, Duration::from_secs(60), Now::real());
//! ```
//!
//! The same call with an undo effect compiles:
//!
//! ```
//! use pctwin_grant::*;
//! use std::time::Duration;
//! let keys = RoleKeys::mint().unwrap();
//! let g = keys.undo.grant(PlanSeal::new(1, [7; 32]), vec![ObjectRef::new(1, [9; 32])],
//!     UndoEffect::RecoveryPoint, Duration::from_secs(60), Now::real());
//! assert!(g.is_ok());
//! ```

use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant, SystemTime};

use serde::{Deserialize, Serialize};

/// The longest any grant lasts. A move asks again for what it still needs after this.
pub const MAX_LIFETIME: Duration = Duration::from_secs(15 * 60);

/// The longest one administrator helper session may act, from when its plan is sealed, however
/// often it acts (security review finding 6). A move that needs longer starts the helper again,
/// with the system's own prompt.
pub const MAX_SESSION: Duration = Duration::from_secs(4 * 60 * 60);

/// The most things one grant may name.
pub const MAX_OBJECTS: usize = 4096;

/// The approved plan a grant belongs to: its generation (raised each time the plan changes) and
/// its digest, both from the plan the person approved on this laptop.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct PlanSeal {
    generation: u64,
    digest: [u8; 32],
}

impl PlanSeal {
    pub fn new(generation: u64, digest: [u8; 32]) -> Self {
        Self { generation, digest }
    }

    pub fn generation(&self) -> u64 {
        self.generation
    }

    pub fn digest(&self) -> [u8; 32] {
        self.digest
    }
}

/// One exact thing: its handle where it is kept, and the digest of what it is (a package's bytes,
/// a setting and its value).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ObjectRef {
    handle: u64,
    digest: [u8; 32],
}

impl ObjectRef {
    pub fn new(handle: u64, digest: [u8; 32]) -> Self {
        Self { handle, digest }
    }

    pub fn handle(&self) -> u64 {
        self.handle
    }

    pub fn digest(&self) -> [u8; 32] {
        self.digest
    }
}

/// One approved root (a folder resolved by the safety gate), by its handle.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct RootRef {
    handle: u64,
}

impl RootRef {
    pub fn new(handle: u64) -> Self {
        Self { handle }
    }

    pub fn handle(&self) -> u64 {
        self.handle
    }
}

/// Who a request is from: the Phase 2 roles, as the plans name them.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Audience {
    #[serde(rename = "bot-2.1")]
    Install,
    #[serde(rename = "bot-2.2")]
    Broker,
    #[serde(rename = "bot-2.3")]
    Recovery,
    #[serde(rename = "bot-2.4")]
    Settings,
    #[serde(rename = "bot-2.5")]
    Data,
    #[serde(rename = "bot-2.6")]
    Orchestrator,
    #[serde(rename = "bot-2.7")]
    Handover,
    #[serde(rename = "bot-2.8")]
    Interface,
}

impl Audience {
    /// Whether this role may ask for privileged work at all. The broker itself, the move planner
    /// (2.6) and the screens (2.8) never may: the planner asks through each responsible role, and
    /// the screens only show.
    pub fn may_ask_privileged(self) -> bool {
        matches!(
            self,
            Audience::Install
                | Audience::Recovery
                | Audience::Settings
                | Audience::Data
                | Audience::Handover
        )
    }
}

/// What a 2.5 grant allows.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum CaptureEffect {
    /// Read private data (mail, browser data, keys the person chose) in the named roots.
    ReadPrivate,
    /// Import it on the new laptop.
    Import,
    /// Run the app's own export (its export screen, or its own export tool named by a signed
    /// catalogue row) for data in the named roots, as the person.
    NativeExport,
}

/// What a 2.5 secret grant allows, for the exact components it names.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum SecretEffect {
    /// Read the chosen secret files on the old laptop.
    Read,
    /// Land them on the new laptop, owner-only.
    Land,
}

/// What a 2.3 grant allows.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum UndoEffect {
    /// Reverse a recorded change.
    Reverse,
    /// Find out what really happened to an unfinished change.
    Reconcile,
    /// Make a Windows restore point before changes.
    RecoveryPoint,
}

/// What a 2.7 grant allows. Wiping a laptop is never one of these: it is a separate ceremony.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum CleanupEffect {
    /// Remove things PCTwin itself put there.
    RemoveOwned,
    /// Clear PCTwin's own traces (caches, models) the person chose to remove.
    Sanitise,
}

/// The time, read from two clocks: one that only goes forward while the program runs, and the
/// calendar clock.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Now {
    mono: Instant,
    wall: SystemTime,
}

impl Now {
    pub fn real() -> Self {
        Self {
            mono: Instant::now(),
            wall: SystemTime::now(),
        }
    }

    /// A given moment (for tests, and for one reading shared by several checks).
    pub fn at(mono: Instant, wall: SystemTime) -> Self {
        Self { mono, wall }
    }
}

/// When a grant ends. It ends at whichever comes first: its lifetime on the forward-only clock,
/// or its lifetime on the calendar clock. The forward-only clock cannot be set back, so turning
/// the calendar back never makes a grant last longer; the calendar keeps moving while the laptop
/// sleeps, which the forward-only clock does not do on every system, so a long sleep ends it too.
/// A calendar set back before the grant was made ends it at once.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Expiry {
    issued_wall: SystemTime,
    mono_deadline: Instant,
    wall_deadline: SystemTime,
}

impl Expiry {
    /// Ends `lifetime` after `now`; `lifetime` must be above zero and at most [`MAX_LIFETIME`].
    pub fn after(lifetime: Duration, now: Now) -> Result<Self, GrantError> {
        Self::within(lifetime, MAX_LIFETIME, now)
    }

    /// A session's fixed end, [`MAX_SESSION`] after `now` (when its plan is sealed), read from
    /// both clocks as a grant's is. Nothing renews it.
    pub fn session_from(now: Now) -> Result<Self, GrantError> {
        Self::within(MAX_SESSION, MAX_SESSION, now)
    }

    fn within(lifetime: Duration, most: Duration, now: Now) -> Result<Self, GrantError> {
        if lifetime.is_zero() || lifetime > most {
            return Err(GrantError::Lifetime);
        }
        Ok(Self {
            issued_wall: now.wall,
            mono_deadline: now.mono.checked_add(lifetime).ok_or(GrantError::Lifetime)?,
            wall_deadline: now.wall.checked_add(lifetime).ok_or(GrantError::Lifetime)?,
        })
    }

    pub fn is_over(&self, now: Now) -> bool {
        now.mono >= self.mono_deadline
            || now.wall >= self.wall_deadline
            || now.wall < self.issued_wall
    }
}

/// Why a grant was not made.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GrantError {
    /// A lifetime of zero, or longer than [`MAX_LIFETIME`].
    Lifetime,
    /// No things named, or more than [`MAX_OBJECTS`].
    Objects,
}

impl std::fmt::Display for GrantError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            GrantError::Lifetime => "a permission must last a short, limited time",
            GrantError::Objects => "a permission must name at least one thing, and not too many",
        })
    }
}

impl std::error::Error for GrantError {}

/// The things a grant names: at least one, at most [`MAX_OBJECTS`], each once, in order.
fn exact<T: Ord>(mut things: Vec<T>) -> Result<Vec<T>, GrantError> {
    things.sort();
    things.dedup();
    if things.is_empty() || things.len() > MAX_OBJECTS {
        return Err(GrantError::Objects);
    }
    Ok(things)
}

/// Can only be named, never made, outside this crate: what keeps keys from being written out.
#[derive(Debug)]
struct Sealed;

/// 2.1's grant: install one exact verified package of the approved plan.
#[derive(Debug, PartialEq, Eq)]
pub struct InstallGrant {
    seal: PlanSeal,
    artifact: ObjectRef,
    expiry: Expiry,
}

impl InstallGrant {
    pub const AUDIENCE: Audience = Audience::Install;

    pub fn seal(&self) -> PlanSeal {
        self.seal
    }

    pub fn artifact(&self) -> ObjectRef {
        self.artifact
    }

    pub fn expiry(&self) -> Expiry {
        self.expiry
    }
}

/// 2.4's grant: apply one named setting of the approved plan.
#[derive(Debug, PartialEq, Eq)]
pub struct SettingGrant {
    seal: PlanSeal,
    setting: ObjectRef,
    expiry: Expiry,
}

impl SettingGrant {
    pub const AUDIENCE: Audience = Audience::Settings;

    pub fn seal(&self) -> PlanSeal {
        self.seal
    }

    pub fn setting(&self) -> ObjectRef {
        self.setting
    }

    pub fn expiry(&self) -> Expiry {
        self.expiry
    }
}

/// 2.5's grant: read or import private data, inside the named roots only.
#[derive(Debug, PartialEq, Eq)]
pub struct CaptureGrant {
    seal: PlanSeal,
    roots: Vec<RootRef>,
    effect: CaptureEffect,
    expiry: Expiry,
}

impl CaptureGrant {
    pub const AUDIENCE: Audience = Audience::Data;

    pub fn seal(&self) -> PlanSeal {
        self.seal
    }

    pub fn roots(&self) -> &[RootRef] {
        &self.roots
    }

    pub fn effect(&self) -> CaptureEffect {
        self.effect
    }

    pub fn expiry(&self) -> Expiry {
        self.expiry
    }
}

/// 2.5's grant for secrets: read or land exactly the secret components the person chose (each
/// named by its identity and the generation approved), nothing else. An ordinary 2.5 grant never
/// reaches a secret.
#[derive(Debug, PartialEq, Eq)]
pub struct SecretGrant {
    seal: PlanSeal,
    components: Vec<ObjectRef>,
    effect: SecretEffect,
    expiry: Expiry,
}

impl SecretGrant {
    pub const AUDIENCE: Audience = Audience::Data;

    pub fn seal(&self) -> PlanSeal {
        self.seal
    }

    pub fn components(&self) -> &[ObjectRef] {
        &self.components
    }

    pub fn effect(&self) -> SecretEffect {
        self.effect
    }

    pub fn expiry(&self) -> Expiry {
        self.expiry
    }
}

/// 2.3's grant: reverse, reconcile or make a restore point, for the named things only.
#[derive(Debug, PartialEq, Eq)]
pub struct UndoGrant {
    seal: PlanSeal,
    objects: Vec<ObjectRef>,
    effect: UndoEffect,
    expiry: Expiry,
}

impl UndoGrant {
    pub const AUDIENCE: Audience = Audience::Recovery;

    pub fn seal(&self) -> PlanSeal {
        self.seal
    }

    pub fn objects(&self) -> &[ObjectRef] {
        &self.objects
    }

    pub fn effect(&self) -> UndoEffect {
        self.effect
    }

    pub fn expiry(&self) -> Expiry {
        self.expiry
    }
}

/// 2.7's grant: remove PCTwin's own named things, or sanitise them.
#[derive(Debug, PartialEq, Eq)]
pub struct CleanupGrant {
    seal: PlanSeal,
    objects: Vec<ObjectRef>,
    effect: CleanupEffect,
    expiry: Expiry,
}

impl CleanupGrant {
    pub const AUDIENCE: Audience = Audience::Handover;

    pub fn seal(&self) -> PlanSeal {
        self.seal
    }

    pub fn objects(&self) -> &[ObjectRef] {
        &self.objects
    }

    pub fn effect(&self) -> CleanupEffect {
        self.effect
    }

    pub fn expiry(&self) -> Expiry {
        self.expiry
    }
}

/// 2.1's key: makes [`InstallGrant`]s and nothing else.
#[derive(Debug)]
pub struct InstallKey {
    _sealed: Sealed,
}

impl InstallKey {
    pub fn grant(
        &self,
        seal: PlanSeal,
        artifact: ObjectRef,
        lifetime: Duration,
        now: Now,
    ) -> Result<InstallGrant, GrantError> {
        Ok(InstallGrant {
            seal,
            artifact,
            expiry: Expiry::after(lifetime, now)?,
        })
    }
}

/// 2.4's key: makes [`SettingGrant`]s and nothing else.
#[derive(Debug)]
pub struct SettingKey {
    _sealed: Sealed,
}

impl SettingKey {
    pub fn grant(
        &self,
        seal: PlanSeal,
        setting: ObjectRef,
        lifetime: Duration,
        now: Now,
    ) -> Result<SettingGrant, GrantError> {
        Ok(SettingGrant {
            seal,
            setting,
            expiry: Expiry::after(lifetime, now)?,
        })
    }
}

/// 2.5's key: makes [`CaptureGrant`]s and nothing else.
#[derive(Debug)]
pub struct CaptureKey {
    _sealed: Sealed,
}

impl CaptureKey {
    pub fn grant(
        &self,
        seal: PlanSeal,
        roots: Vec<RootRef>,
        effect: CaptureEffect,
        lifetime: Duration,
        now: Now,
    ) -> Result<CaptureGrant, GrantError> {
        Ok(CaptureGrant {
            seal,
            roots: exact(roots)?,
            effect,
            expiry: Expiry::after(lifetime, now)?,
        })
    }

    /// A grant for exactly the secret `components` the person chose.
    pub fn secret(
        &self,
        seal: PlanSeal,
        components: Vec<ObjectRef>,
        effect: SecretEffect,
        lifetime: Duration,
        now: Now,
    ) -> Result<SecretGrant, GrantError> {
        Ok(SecretGrant {
            seal,
            components: exact(components)?,
            effect,
            expiry: Expiry::after(lifetime, now)?,
        })
    }
}

/// 2.3's key: makes [`UndoGrant`]s and nothing else.
#[derive(Debug)]
pub struct UndoKey {
    _sealed: Sealed,
}

impl UndoKey {
    pub fn grant(
        &self,
        seal: PlanSeal,
        objects: Vec<ObjectRef>,
        effect: UndoEffect,
        lifetime: Duration,
        now: Now,
    ) -> Result<UndoGrant, GrantError> {
        Ok(UndoGrant {
            seal,
            objects: exact(objects)?,
            effect,
            expiry: Expiry::after(lifetime, now)?,
        })
    }
}

/// 2.7's key: makes [`CleanupGrant`]s and nothing else.
#[derive(Debug)]
pub struct CleanupKey {
    _sealed: Sealed,
}

impl CleanupKey {
    pub fn grant(
        &self,
        seal: PlanSeal,
        objects: Vec<ObjectRef>,
        effect: CleanupEffect,
        lifetime: Duration,
        now: Now,
    ) -> Result<CleanupGrant, GrantError> {
        Ok(CleanupGrant {
            seal,
            objects: exact(objects)?,
            effect,
            expiry: Expiry::after(lifetime, now)?,
        })
    }
}

/// 2.2's key: lets the administrator helper's broker write its own record of the actions it takes
/// (`pctwin_journal::Journal::broker`). It makes no grant: the broker acts only on the requests
/// it checks itself.
#[derive(Debug)]
pub struct BrokerKey {
    _sealed: Sealed,
}

/// Every role's key, once per program. The program's starting code takes them apart and gives
/// each role its own key only.
#[derive(Debug)]
pub struct RoleKeys {
    pub install: InstallKey,
    pub setting: SettingKey,
    pub capture: CaptureKey,
    pub undo: UndoKey,
    /// 2.2's key: the administrator helper's broker writes its own record with it.
    pub broker: BrokerKey,
    pub cleanup: CleanupKey,
}

static MINTED: AtomicBool = AtomicBool::new(false);

impl RoleKeys {
    /// The keys, the first time this is asked in this program; never again after that (code
    /// that asks later gets nothing, and a program whose starting code gets nothing must stop).
    pub fn mint() -> Option<Self> {
        if MINTED.swap(true, Ordering::SeqCst) {
            return None;
        }
        Some(Self::make())
    }

    fn make() -> Self {
        Self {
            install: InstallKey { _sealed: Sealed },
            setting: SettingKey { _sealed: Sealed },
            capture: CaptureKey { _sealed: Sealed },
            undo: UndoKey { _sealed: Sealed },
            broker: BrokerKey { _sealed: Sealed },
            cleanup: CleanupKey { _sealed: Sealed },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const MINUTE: Duration = Duration::from_secs(60);

    fn seal() -> PlanSeal {
        PlanSeal::new(3, [5; 32])
    }

    #[test]
    fn keys_are_made_once_per_program() {
        // The only test that mints: tests share one program.
        assert!(RoleKeys::mint().is_some());
        assert!(RoleKeys::mint().is_none());
        assert!(RoleKeys::mint().is_none());
    }

    #[test]
    fn a_grant_carries_exactly_what_it_was_made_for() {
        let keys = RoleKeys::make();
        let now = Now::real();
        let a = ObjectRef::new(4, [1; 32]);
        let g = keys.install.grant(seal(), a, MINUTE, now).unwrap();
        assert_eq!(g.seal(), seal());
        assert_eq!(g.seal().generation(), 3);
        assert_eq!(g.seal().digest(), [5; 32]);
        assert_eq!(g.artifact(), a);
        assert_eq!(g.artifact().handle(), 4);
        assert_eq!(g.artifact().digest(), [1; 32]);
        assert_eq!(InstallGrant::AUDIENCE, Audience::Install);

        let s = keys.setting.grant(seal(), a, MINUTE, now).unwrap();
        assert_eq!((s.seal(), s.setting()), (seal(), a));
        assert_eq!(SettingGrant::AUDIENCE, Audience::Settings);

        let c = keys
            .capture
            .grant(
                seal(),
                vec![RootRef::new(9), RootRef::new(2), RootRef::new(9)],
                CaptureEffect::Import,
                MINUTE,
                now,
            )
            .unwrap();
        assert_eq!(c.roots(), &[RootRef::new(2), RootRef::new(9)]);
        assert_eq!(c.roots()[0].handle(), 2);
        assert_eq!(c.effect(), CaptureEffect::Import);
        assert_eq!(c.seal(), seal());
        assert_eq!(CaptureGrant::AUDIENCE, Audience::Data);

        let x = keys
            .capture
            .secret(
                seal(),
                vec![a, a, ObjectRef::new(1, [2; 32])],
                SecretEffect::Land,
                MINUTE,
                now,
            )
            .unwrap();
        assert_eq!(x.components(), &[ObjectRef::new(1, [2; 32]), a]);
        assert_eq!((x.seal(), x.effect()), (seal(), SecretEffect::Land));
        assert!(!x.expiry().is_over(now));
        assert_eq!(SecretGrant::AUDIENCE, Audience::Data);
        assert_eq!(
            keys.capture
                .secret(seal(), vec![], SecretEffect::Read, MINUTE, now)
                .unwrap_err(),
            GrantError::Objects
        );
        assert_eq!(
            keys.capture
                .secret(seal(), vec![a], SecretEffect::Read, Duration::ZERO, now)
                .unwrap_err(),
            GrantError::Lifetime
        );

        let u = keys
            .undo
            .grant(seal(), vec![a], UndoEffect::Reconcile, MINUTE, now)
            .unwrap();
        assert_eq!(
            (u.seal(), u.objects(), u.effect()),
            (seal(), &[a][..], UndoEffect::Reconcile)
        );
        assert_eq!(UndoGrant::AUDIENCE, Audience::Recovery);

        let k = keys
            .cleanup
            .grant(seal(), vec![a], CleanupEffect::RemoveOwned, MINUTE, now)
            .unwrap();
        assert_eq!(
            (k.seal(), k.objects(), k.effect()),
            (seal(), &[a][..], CleanupEffect::RemoveOwned)
        );
        assert_eq!(CleanupGrant::AUDIENCE, Audience::Handover);

        for e in [g.expiry(), s.expiry(), c.expiry(), u.expiry(), k.expiry()] {
            assert!(!e.is_over(now));
        }
    }

    #[test]
    fn a_grant_names_at_least_one_thing_and_not_too_many() {
        let keys = RoleKeys::make();
        let now = Now::real();
        assert_eq!(
            keys.capture
                .grant(seal(), vec![], CaptureEffect::ReadPrivate, MINUTE, now)
                .unwrap_err(),
            GrantError::Objects
        );
        assert_eq!(
            keys.undo
                .grant(seal(), vec![], UndoEffect::Reverse, MINUTE, now)
                .unwrap_err(),
            GrantError::Objects
        );
        assert_eq!(
            keys.cleanup
                .grant(seal(), vec![], CleanupEffect::Sanitise, MINUTE, now)
                .unwrap_err(),
            GrantError::Objects
        );
        let most: Vec<RootRef> = (0..MAX_OBJECTS as u64).map(RootRef::new).collect();
        assert!(
            keys.capture
                .grant(
                    seal(),
                    most.clone(),
                    CaptureEffect::ReadPrivate,
                    MINUTE,
                    now
                )
                .is_ok()
        );
        let mut too_many = most;
        too_many.push(RootRef::new(u64::MAX));
        assert_eq!(
            keys.capture
                .grant(seal(), too_many, CaptureEffect::ReadPrivate, MINUTE, now)
                .unwrap_err(),
            GrantError::Objects
        );
    }

    #[test]
    fn a_grant_lasts_a_short_limited_time() {
        let keys = RoleKeys::make();
        let now = Now::real();
        let a = ObjectRef::new(1, [0; 32]);
        assert_eq!(
            keys.install
                .grant(seal(), a, Duration::ZERO, now)
                .unwrap_err(),
            GrantError::Lifetime
        );
        assert!(keys.install.grant(seal(), a, MAX_LIFETIME, now).is_ok());
        assert_eq!(
            keys.setting
                .grant(seal(), a, MAX_LIFETIME + Duration::from_nanos(1), now)
                .unwrap_err(),
            GrantError::Lifetime
        );
    }

    /// T22-009: the calendar set back never extends a grant; the forward-only clock running on
    /// ends it; a sleep the forward-only clock did not count still ends it by the calendar.
    #[test]
    fn turning_the_clock_back_never_extends_a_grant() {
        let mono = Instant::now();
        let wall = SystemTime::now();
        let e = Expiry::after(MINUTE, Now::at(mono, wall)).unwrap();
        assert!(!e.is_over(Now::at(mono, wall)));
        assert!(!e.is_over(Now::at(mono + MINUTE / 2, wall + MINUTE / 2)));
        // Just before the end, on both clocks.
        let nearly = MINUTE - Duration::from_nanos(1);
        assert!(!e.is_over(Now::at(mono + nearly, wall + nearly)));
        // The forward-only clock says it is over, however far back the calendar was set.
        assert!(e.is_over(Now::at(mono + MINUTE, wall)));
        assert!(e.is_over(Now::at(mono + MINUTE, wall - Duration::from_secs(86_400))));
        // A sleep the forward-only clock did not count: the calendar ends it.
        assert!(e.is_over(Now::at(mono + Duration::from_secs(1), wall + MINUTE)));
        // The calendar set back to before the grant was made ends it at once.
        assert!(e.is_over(Now::at(
            mono + Duration::from_secs(1),
            wall - Duration::from_secs(1)
        )));
    }

    /// Review finding 6: a session's end is fixed, longer than a grant's, on both clocks.
    #[test]
    fn a_session_ends_at_its_fixed_deadline() {
        let mono = Instant::now();
        let wall = SystemTime::now();
        let e = Expiry::session_from(Now::at(mono, wall)).unwrap();
        let nearly = MAX_SESSION - Duration::from_secs(1);
        assert!(!e.is_over(Now::at(mono + nearly, wall + nearly)));
        assert!(e.is_over(Now::at(mono + MAX_SESSION, wall)));
        assert!(e.is_over(Now::at(mono, wall + MAX_SESSION)));
        assert!(MAX_SESSION > MAX_LIFETIME);
    }

    #[test]
    fn only_the_roles_that_act_may_ask_for_privileged_work() {
        use Audience::*;
        for a in [Install, Recovery, Settings, Data, Handover] {
            assert!(a.may_ask_privileged(), "{a:?}");
        }
        for a in [Broker, Orchestrator, Interface] {
            assert!(!a.may_ask_privileged(), "{a:?}");
        }
    }

    #[test]
    fn audiences_are_written_as_the_plans_name_them() {
        let all = [
            (Audience::Install, "bot-2.1"),
            (Audience::Broker, "bot-2.2"),
            (Audience::Recovery, "bot-2.3"),
            (Audience::Settings, "bot-2.4"),
            (Audience::Data, "bot-2.5"),
            (Audience::Orchestrator, "bot-2.6"),
            (Audience::Handover, "bot-2.7"),
            (Audience::Interface, "bot-2.8"),
        ];
        for (a, name) in all {
            assert_eq!(serde_json::to_string(&a).unwrap(), format!("\"{name}\""));
            let back: Audience = serde_json::from_str(&format!("\"{name}\"")).unwrap();
            assert_eq!(back, a);
        }
        assert!(serde_json::from_str::<Audience>("\"bot-2.9\"").is_err());
        assert!(serde_json::from_str::<Audience>("\"admin\"").is_err());
    }

    #[test]
    fn errors_say_what_to_fix_in_plain_words() {
        assert!(GrantError::Lifetime.to_string().contains("limited time"));
        assert!(GrantError::Objects.to_string().contains("at least one"));
    }
}
