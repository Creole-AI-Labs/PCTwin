//! What the old laptop sends beside its list (move protocol 6): the settings discovery found in
//! the person's own settings stores, the person's folders there (so a setting that names a moved
//! file can be pointed at its new place), and the special categories.
//!
//! The old laptop only reads ([`here`]). The new laptop reads the bytes as coming from a stranger
//! ([`Extras::decode`]): a fixed format number, unknown fields refused, every list bounded and
//! every text within its length.

use serde::{Deserialize, Serialize};

use pctwin_record::FolderRole;
use pctwin_settings::System;
use pctwin_settings::classify::Why;
use pctwin_settings::found::{Discovery, FoundSetting, NotMoved};

/// The format this release writes and reads.
pub const FORMAT: u32 = 1;
/// The most folders the old laptop names.
pub const MAX_FOLDERS: usize = 32;
/// The most special-category lines.
pub const MAX_SPECIAL: usize = 10_000;
/// The longest name or path, in bytes.
pub const MAX_TEXT: usize = 4096;

/// Why found settings did not move, in five groups a person can follow (the details stay on the
/// laptop that found them).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum NotMovedGroup {
    /// Security, privacy, an organisation's policy, a password or sign-in, or what identifies the
    /// old laptop or its account: never copied.
    Private,
    /// It would start or open something, or points at a place PCTwin cannot check or did not
    /// move: kept back for safety.
    CouldRun,
    /// Only fits the old laptop (caches, window places, devices, a kind of value not copied).
    OldLaptopOnly,
    /// The system's own settings, or another system's: only the ones PCTwin knows move.
    SystemOwn,
    /// Found, not read (too many, too deeply nested, or unreadable).
    NotRead,
}

impl NotMovedGroup {
    /// The group a reason belongs to.
    pub fn of(why: &Why) -> Self {
        match why {
            Why::Security
            | Why::Policy
            | Why::Secret
            | Why::MachineId
            | Why::AccountBound
            | Why::HiddenText => NotMovedGroup::Private,
            Why::StartsProgram
            | Why::Fetches
            | Why::PlaceUnknown
            | Why::PathNotMoved
            | Why::RecentMissing => NotMovedGroup::CouldRun,
            Why::Cache
            | Why::WindowPlace
            | Why::HardwarePath
            | Why::TooLarge
            | Why::Unnamed
            | Why::Kind => NotMovedGroup::OldLaptopOnly,
            Why::SystemStore | Why::OtherSystem | Why::Exception(_) => NotMovedGroup::SystemOwn,
            Why::TooDeep | Why::TooMany | Why::Unreadable(_) => NotMovedGroup::NotRead,
        }
    }
}

/// How many found settings of one group did not move.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct NotMovedCount {
    pub group: NotMovedGroup,
    pub count: u64,
}

/// Counts by group, in group order, leaving out empty groups.
pub fn count_not_moved(not_moved: &[NotMoved]) -> Vec<NotMovedCount> {
    let mut by: std::collections::BTreeMap<NotMovedGroup, u64> = Default::default();
    for n in not_moved {
        *by.entry(NotMovedGroup::of(&n.why)).or_default() += 1;
    }
    by.into_iter()
        .map(|(group, count)| NotMovedCount { group, count })
        .collect()
}

/// One of the person's folders on the old laptop, by what it is for.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct OldFolder {
    pub role: FolderRole,
    pub path: String,
}

/// What happens to one special-category component (email, a browser's data, a game's saves, an
/// AI tool's history, a secret file...), as 2.5 plans it on the old laptop.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum SpecialStep {
    /// Copied as files with undo.
    Copy,
    /// The app's own export on the old laptop first, then its own import.
    ExportFirst,
    /// Comes back by signing in to its service.
    SignIn,
    /// Linked again from the service or the app (a new device link).
    Relink,
    /// Not moved by this version of PCTwin: the person is told, never silently left.
    NotYet,
    /// PCTwin could not tell what it is: the person looks at it on the old laptop.
    LookAtIt,
}

/// One special-category component.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct SpecialLine {
    /// The app's name, as 2.5 shows it.
    pub name: String,
    pub step: SpecialStep,
    /// How many files it holds (they move together or not at all).
    #[serde(default)]
    pub files: u64,
    /// Secret material (keys, password stores): moved only if the person chooses it.
    #[serde(default)]
    pub secret: bool,
    /// Its components' IDs (what the new laptop chooses by).
    #[serde(default)]
    pub ids: Vec<String>,
}

/// Everything the old laptop sends beside its list.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct Extras {
    pub format: u32,
    /// The old laptop's kind of system, as settings know it.
    pub system: System,
    /// Every setting discovery found that may be planned (the new laptop plans them again).
    pub settings: Vec<FoundSetting>,
    /// Found settings the old laptop already knows will not move, counted by group.
    pub not_moved: Vec<NotMovedCount>,
    pub folders: Vec<OldFolder>,
    pub special: Vec<SpecialLine>,
    /// Whether the special categories were looked for at all (2.5 discovery ran).
    #[serde(default)]
    pub special_searched: bool,
    /// Things 2.5 found and does not move (caches, the system's own), counted.
    #[serde(default)]
    pub special_not_moved: u64,
    /// Places 2.5 could not fully check, counted.
    #[serde(default)]
    pub special_not_checked: u64,
}

/// Why the old laptop's extras were refused.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ExtrasError {
    #[error("the old laptop's extras could not be read")]
    Unreadable,
    #[error("the old laptop's extras are in a format this PCTwin does not know; update both")]
    Format,
    #[error("the old laptop's extras break a limit")]
    Limit,
}

fn text_ok(s: &str) -> bool {
    !s.is_empty() && s.len() <= MAX_TEXT && !s.chars().any(char::is_control)
}

impl Extras {
    /// None found (a laptop where nothing could be read, or a system without settings yet).
    pub fn empty(system: System) -> Self {
        Self {
            format: FORMAT,
            system,
            settings: Vec::new(),
            not_moved: Vec::new(),
            folders: Vec::new(),
            special: Vec::new(),
            special_searched: false,
            special_not_moved: 0,
            special_not_checked: 0,
        }
    }

    /// The bytes sent.
    pub fn encode(&self) -> Vec<u8> {
        serde_json::to_vec(self).unwrap_or_default()
    }

    /// The bytes received, read strictly. Empty bytes (an old laptop with nothing to send) are
    /// refused: the caller treats a missing extras as "none" itself.
    pub fn decode(bytes: &[u8]) -> Result<Self, ExtrasError> {
        let x: Extras = serde_json::from_slice(bytes).map_err(|_| ExtrasError::Unreadable)?;
        if x.format != FORMAT {
            return Err(ExtrasError::Format);
        }
        x.check()?;
        Ok(x)
    }

    /// Every list within its bound, every text within its length, no setting twice.
    pub fn check(&self) -> Result<(), ExtrasError> {
        let limit = Err(ExtrasError::Limit);
        if self.settings.len() > pctwin_types::limits::FOUND_SETTINGS
            || self.folders.len() > MAX_FOLDERS
            || self.special.len() > MAX_SPECIAL
            || self.not_moved.len() > 16
        {
            return limit;
        }
        let mut keys = std::collections::HashSet::new();
        for s in &self.settings {
            let a = &s.address;
            if !text_ok(&s.label) || !text_ok(a.store()) || a.name().len() > MAX_TEXT {
                return limit;
            }
            if let pctwin_recipe::SettingValue::Text(t) = &s.value
                && t.len() > pctwin_types::limits::SETTING_TEXT_BYTES
            {
                return limit;
            }
            if !keys.insert(a.key()) {
                return limit;
            }
        }
        let mut roles = std::collections::HashSet::new();
        for f in &self.folders {
            if !text_ok(&f.path) || !roles.insert(f.role) {
                return limit;
            }
        }
        if self.special.iter().any(|s| !text_ok(&s.name)) {
            return limit;
        }
        Ok(())
    }
}

/// The digest of the extras exactly as they arrived: it joins the plan the person approves.
pub fn digest(bytes: &[u8]) -> [u8; 32] {
    *blake3::Hasher::new_derive_key("PCTwin full move extras 2026-10-27 v1")
        .update(bytes)
        .finalize()
        .as_bytes()
}

/// Builds the extras from what discovery found on this laptop (`system`) and the person's folders
/// here. Settings already known not to move are only counted.
pub fn from_discovery(system: System, d: Discovery, folders: Vec<OldFolder>) -> Extras {
    let mut folders = folders;
    folders.retain(|f| text_ok(&f.path));
    folders.truncate(MAX_FOLDERS);
    let mut settings = d.found;
    settings.truncate(pctwin_types::limits::FOUND_SETTINGS);
    let mut seen = std::collections::HashSet::new();
    settings.retain(|s| {
        text_ok(&s.label)
            && text_ok(s.address.store())
            && s.address.name().len() <= MAX_TEXT
            && seen.insert(s.address.key())
    });
    Extras {
        format: FORMAT,
        system,
        not_moved: count_not_moved(&d.not_moved),
        settings,
        folders,
        special: Vec::new(),
        special_searched: false,
        special_not_moved: 0,
        special_not_checked: 0,
    }
}

/// The special categories of the person `owner` (home folder `home`, apps `apps`) on this laptop,
/// as 2.5 plans them (only reading): one line per component, with the person's step. Copying a
/// component's files is not joined to the move yet, so a component that would copy says "not
/// moved yet"; a sign-in, a relink, an export first, or something to look at says that step.
pub fn special_here(
    owner: &pctwin_record::Owner,
    home: &std::path::Path,
    apps: &[pctwin_record::FoundApp],
) -> SpecialFound {
    use pctwin_special::component::Custody;
    let system = pctwin_special::roots::System::here();
    let cat = pctwin_catalogue::Catalogue::<pctwin_special::catalogue::SpecialRow>::shipped(
        pctwin_special::catalogue::SHIPPED,
    );
    let plan = pctwin_special::api::plan(owner, home, apps, system, system, &cat);
    // One line per app and step (an app's many components with the same step are one line, with
    // their files counted), in the order found.
    let mut lines: Vec<SpecialLine> = Vec::new();
    for p in plan.components.iter().filter(|p| text_ok(&p.app)) {
        let step = match p.component.custody {
            Custody::Portable => SpecialStep::Copy,
            Custody::ServiceHeld => SpecialStep::SignIn,
            Custody::MachineBound => SpecialStep::Relink,
            Custody::NeedsExport => SpecialStep::ExportFirst,
            Custody::Unresolved => SpecialStep::LookAtIt,
        };
        let files = p.component.members.len() as u64;
        let id = crate::data::id_hex(&p.component);
        // A secret is always its own line: the person ticks each one by itself (D07).
        let same = |l: &&mut SpecialLine| {
            !p.component.secret && !l.secret && l.name == p.app && l.step == step
        };
        match lines.iter_mut().find(same) {
            Some(l) => {
                l.files += files;
                l.ids.push(id);
            }
            None => lines.push(SpecialLine {
                name: p.app.clone(),
                step,
                files,
                secret: p.component.secret,
                ids: vec![id],
            }),
        }
    }
    lines.truncate(MAX_SPECIAL);
    // What 2.5 could not fully check (out of time, no answer, unreadable) is said, never dropped.
    let not_checked = plan
        .not_moved
        .iter()
        .filter(|n| n.why.contains("not fully checked"))
        .count()
        + plan.coverage.unreadable.len();
    SpecialFound {
        lines,
        not_moved: plan.not_moved.len() as u64,
        not_checked: not_checked as u64,
        planned: plan.components,
    }
}

/// What 2.5 found on the old laptop.
#[derive(Debug, Clone, Default)]
pub struct SpecialFound {
    pub lines: Vec<SpecialLine>,
    /// Found and not moved (caches, the system's own).
    pub not_moved: u64,
    /// Places not fully checked (out of time, no answer, unreadable).
    pub not_checked: u64,
    /// The components themselves, kept on the old laptop for capture (never sent).
    pub planned: Vec<pctwin_special::api::Planned>,
}

/// The special-category scan, running beside the scan of files and people.
pub struct SpecialScan {
    rx: std::sync::mpsc::Receiver<SpecialFound>,
    until: std::time::Instant,
}

impl SpecialScan {
    /// Starts looking at `owner`'s app data in `home` (read only), on a thread of its own, with
    /// [`SPECIAL_BUDGET`] in all from now.
    pub fn start(owner: &pctwin_record::Owner, home: &std::path::Path) -> Option<Self> {
        let (tx, rx) = std::sync::mpsc::channel();
        let (owner, home) = (owner.clone(), home.to_path_buf());
        std::thread::Builder::new()
            .name("pctwin-special-scan".into())
            .spawn(move || {
                let _ = tx.send(special_here(&owner, &home, &[]));
            })
            .ok()?;
        Some(Self {
            rx,
            until: std::time::Instant::now() + SPECIAL_BUDGET,
        })
    }

    /// Whether it has finished (never waits).
    pub fn ready(&self) -> Option<SpecialFound> {
        self.rx.try_recv().ok()
    }

    /// Its answer, waiting at most until its budget runs out (`None`: not in time; the reading
    /// carries on, read only, and its answer is not used). `stopped` ends the wait early.
    pub fn wait(self, stopped: &dyn Fn() -> bool) -> Option<SpecialFound> {
        loop {
            let left = self
                .until
                .saturating_duration_since(std::time::Instant::now());
            if left.is_zero() || stopped() {
                return self.rx.try_recv().ok();
            }
            match self
                .rx
                .recv_timeout(left.min(std::time::Duration::from_millis(500)))
            {
                Ok(found) => return Some(found),
                Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {}
                Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => return None,
            }
        }
    }
}

impl Extras {
    /// Adds what the special-category scan found (`None`: it did not finish in time).
    pub fn with_special(&mut self, found: Option<&SpecialFound>) {
        if let Some(f) = found {
            self.special = f.lines.clone();
            self.special_searched = true;
            self.special_not_moved = f.not_moved;
            self.special_not_checked = f.not_checked;
        }
    }
}

/// This laptop's kind of system, as settings know it.
pub fn this_system() -> System {
    if cfg!(windows) {
        System::Windows
    } else if cfg!(target_os = "macos") {
        System::Mac
    } else {
        System::Gnome
    }
}

/// The old laptop: reads (only reads) every setting in the signed-in person's own settings
/// stores, with the person's `folders` here. The special categories are added from their own
/// scan ([`SpecialScan`], [`Extras::with_special`]).
pub fn here(folders: Vec<OldFolder>) -> Extras {
    let system = this_system();
    #[cfg(windows)]
    let found = pctwin_settings::found::discover_registry(
        &pctwin_settings::registry::WinHive::current_user(),
    );
    #[cfg(target_os = "macos")]
    let found = pctwin_settings::found::discover_mac(
        &mut pctwin_settings::mechanism::Mechanisms::real(System::Mac),
    );
    #[cfg(all(unix, not(target_os = "macos")))]
    let found = pctwin_settings::found::discover_dconf(
        &mut pctwin_settings::mechanism::Mechanisms::real(System::Gnome),
    );
    from_discovery(system, found, folders)
}

/// How long looking for the special categories may take on the old laptop in all, from when the
/// move starts (2.5's own budgets: 10 s per read, 120 s per app folder, 600 s per kind of root).
pub const SPECIAL_BUDGET: std::time::Duration = std::time::Duration::from_secs(600);

#[cfg(test)]
mod tests {
    use super::*;
    use pctwin_recipe::SettingValue;
    use pctwin_settings::found::{Address, RegKind};

    fn setting(name: &str, value: SettingValue) -> FoundSetting {
        FoundSetting {
            address: Address::Registry {
                key: "Software\\Fake\\App".into(),
                name: name.into(),
                kind: RegKind::Sz,
            },
            label: "Fake App".into(),
            value,
        }
    }

    fn sample() -> Extras {
        let d = Discovery {
            found: vec![
                setting("Theme", SettingValue::Text("ocean".into())),
                setting("Theme", SettingValue::Text("twice".into())),
            ],
            not_moved: vec![
                NotMoved {
                    label: "Fake App".into(),
                    store: "HKCU\\Software\\Fake\\App".into(),
                    name: "Token".into(),
                    why: Why::Secret,
                },
                NotMoved {
                    label: "Fake App".into(),
                    store: "HKCU\\Software\\Fake\\App".into(),
                    name: "Run".into(),
                    why: Why::StartsProgram,
                },
                NotMoved {
                    label: "Fake App".into(),
                    store: "HKCU\\Software\\Fake\\App".into(),
                    name: "Login".into(),
                    why: Why::Secret,
                },
            ],
        };
        from_discovery(
            System::Windows,
            d,
            vec![OldFolder {
                role: FolderRole::Documents,
                path: "C:\\Users\\ada\\Documents".into(),
            }],
        )
    }

    #[test]
    fn extras_cross_exactly_and_a_setting_found_twice_is_sent_once() {
        let x = sample();
        assert_eq!(x.settings.len(), 1);
        assert_eq!(
            x.not_moved,
            vec![
                NotMovedCount {
                    group: NotMovedGroup::Private,
                    count: 2
                },
                NotMovedCount {
                    group: NotMovedGroup::CouldRun,
                    count: 1
                }
            ]
        );
        let bytes = x.encode();
        assert_eq!(Extras::decode(&bytes).unwrap(), x);
        assert_eq!(digest(&bytes), digest(&x.encode()));
    }

    #[test]
    fn extras_from_a_stranger_are_read_strictly() {
        let x = sample();
        // Another format.
        let mut other = x.clone();
        other.format = 2;
        assert_eq!(Extras::decode(&other.encode()), Err(ExtrasError::Format));
        // An unknown field.
        let mut v: serde_json::Value = serde_json::from_slice(&x.encode()).unwrap();
        v["extra"] = serde_json::json!(1);
        assert_eq!(
            Extras::decode(&serde_json::to_vec(&v).unwrap()),
            Err(ExtrasError::Unreadable)
        );
        // A setting twice.
        let mut twice = x.clone();
        twice.settings.push(twice.settings[0].clone());
        assert_eq!(Extras::decode(&twice.encode()), Err(ExtrasError::Limit));
        // A folder named with a control character.
        let mut bad = x.clone();
        bad.folders[0].path = "C:\\a\u{7}".into();
        assert_eq!(Extras::decode(&bad.encode()), Err(ExtrasError::Limit));
        // Not JSON at all.
        assert_eq!(Extras::decode(b"\x00\x01"), Err(ExtrasError::Unreadable));
    }

    #[test]
    fn every_reason_belongs_to_one_group() {
        assert_eq!(NotMovedGroup::of(&Why::Secret), NotMovedGroup::Private);
        assert_eq!(NotMovedGroup::of(&Why::Fetches), NotMovedGroup::CouldRun);
        assert_eq!(NotMovedGroup::of(&Why::Cache), NotMovedGroup::OldLaptopOnly);
        assert_eq!(
            NotMovedGroup::of(&Why::Exception("x".into())),
            NotMovedGroup::SystemOwn
        );
        assert_eq!(NotMovedGroup::of(&Why::TooMany), NotMovedGroup::NotRead);
    }
}
