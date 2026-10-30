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
            | Why::RecentMissing
            | Why::NotLanded => NotMovedGroup::CouldRun,
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
    /// Carried as it is and written out on the new laptop in a standard format the app imports
    /// (Apple Mail's mail kept only on the old Mac, as mailbox files): the import stays the
    /// person's own step, never claimed done.
    Convert,
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
    /// It holds saved sign-ins to websites or accounts (cookies, session tokens): they are tied
    /// to the old laptop, so the person signs in again on the new one. Found by the stores'
    /// shape, the same for every app.
    #[serde(default)]
    pub sites: bool,
    /// The special category the person sees it in (mail, browsers, games, AI tools, secrets).
    #[serde(default)]
    pub category: pctwin_special::category::Category,
    /// What the person is told because of its category (typed, the screens' words).
    #[serde(default)]
    pub notes: Vec<pctwin_special::category::Note>,
    /// Its files' sizes added up, where the move needs room for more than the files themselves
    /// (a line that is converted on the new laptop needs about twice its size there).
    #[serde(default)]
    pub bytes: u64,
    /// For a line converted on the new laptop: the folder each of its components holds, in the
    /// same order as `ids` (so the new laptop knows which part did not arrive). Empty otherwise.
    #[serde(default)]
    pub parts: Vec<String>,
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
    /// Links and junctions found among the person's files: counted, never followed, never moved.
    #[serde(default)]
    pub links: u64,
    /// The person chose on the old laptop to leave their apps, settings and app data behind
    /// (decided 28 October 2026): said as their own choice, never as something left to do.
    #[serde(default)]
    pub left_out_by_person: bool,
    /// The forgotten-things finder's answer on the old laptop (2.5): places found and counted,
    /// and printers; `None` when it did not run.
    #[serde(default)]
    pub forgotten: Option<pctwin_special::forgotten::Finding>,
    /// The saved Wi-Fi networks found, by name, and which ones the person ticked and were read
    /// (move protocol 8 and later; never a password). `None`: not offered (an older new laptop,
    /// or none found). Left out of the bytes when `None`, so an older new laptop reads them.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub wifi: Option<Vec<crate::wifi::WifiListed>>,
    /// The settings catalogue's rows as the old laptop read them, row by row (theme, wallpaper,
    /// pointer, scaling, keyboard layouts, default browser...; move protocol 9,
    /// [`pctwin_transfer::CATALOGUE_SINCE`]). The new laptop maps them between the two systems
    /// as the catalogue says ([`pctwin_settings::map::plan`]). `None`: not read (an older old
    /// laptop, or an older new laptop that would refuse the field). Left out of the bytes when
    /// `None`, so an older new laptop reads them.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub catalogue: Option<pctwin_settings::map::Capture>,
}

/// The most catalogue rows, and settings without a row, one capture may name.
pub const MAX_CATALOGUE: usize = 1000;

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

/// Most notes one special line may carry across.
pub const MAX_NOTES: usize = 16;

/// A help page from the old laptop's catalogue, as the new laptop accepts it: an https address,
/// within the text limits (security review of 2.5, 29 October 2026: a page is never trusted
/// just because the other laptop sent it; nothing here opens it).
fn page_ok(page: Option<&str>) -> bool {
    page.is_none_or(|p| text_ok(p) && p.starts_with("https://") && !p.contains(char::is_whitespace))
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
            links: 0,
            left_out_by_person: false,
            forgotten: None,
            wifi: None,
            catalogue: None,
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
        if let Some(f) = &self.forgotten {
            let texts = f.found.iter().all(|x| {
                text_ok(&x.kind)
                    && text_ok(&x.name)
                    && text_ok(&x.place)
                    && page_ok(x.page.as_deref())
            }) && f.printers.iter().all(|p| text_ok(&p.name))
                && f.not_checked.iter().all(|(a, b)| text_ok(a) && text_ok(b));
            if f.found.len() > MAX_FORGOTTEN
                || f.not_checked.len() > MAX_FORGOTTEN
                || f.printers.len() > pctwin_special::forgotten::MAX_PRINTERS
                || !texts
            {
                return limit;
            }
        }
        let mut keys = std::collections::HashSet::new();
        for s in &self.settings {
            if !setting_ok(s) || !keys.insert(s.address.key()) {
                return limit;
            }
        }
        let mut roles = std::collections::HashSet::new();
        for f in &self.folders {
            if !text_ok(&f.path) || !roles.insert(f.role) {
                return limit;
            }
        }
        let note_ok = |n: &pctwin_special::category::Note| match n {
            pctwin_special::category::Note::OwnSync { page } => page_ok(page.as_deref()),
            _ => true,
        };
        if self
            .special
            .iter()
            .any(|s| !text_ok(&s.name) || s.notes.len() > MAX_NOTES || !s.notes.iter().all(note_ok))
        {
            return limit;
        }
        // Each part a plain folder name, one per component, or none.
        let part_ok = |p: &String| text_ok(p) && !p.contains(['/', '\\']) && p != "." && p != "..";
        if self.special.iter().any(|s| {
            !s.parts.is_empty() && (s.parts.len() != s.ids.len() || !s.parts.iter().all(part_ok))
        }) {
            return limit;
        }
        if self
            .wifi
            .as_ref()
            .is_some_and(|w| !crate::wifi::listed_ok(w))
        {
            return limit;
        }
        if let Some(c) = &self.catalogue
            && !catalogue_ok(c, self.system)
        {
            return limit;
        }
        Ok(())
    }

    /// Carries the settings catalogue's rows as this laptop read them (only to a new laptop on
    /// move protocol 9 or later: an older one refuses a field it does not know).
    pub fn with_catalogue(&mut self, capture: pctwin_settings::map::Capture) {
        self.catalogue = Some(capture);
    }
}

/// Whether a capture of the catalogue's rows may cross: read on the extras' own system, within
/// its bounds, every text within its length. The values themselves are checked against the new
/// laptop's own catalogue when it plans them ([`pctwin_settings::map::plan`]).
pub fn catalogue_ok(c: &pctwin_settings::map::Capture, system: System) -> bool {
    use pctwin_settings::map::Reading;
    let reading_ok = |r: &Reading| match r {
        Reading::Value(pctwin_recipe::SettingValue::Text(t)) => {
            t.len() <= pctwin_types::limits::SETTING_TEXT_BYTES
        }
        Reading::Unreadable(why) => text_ok(why),
        _ => true,
    };
    c.system == system
        && c.readings.len() <= MAX_CATALOGUE
        && c.unsupported.len() <= MAX_CATALOGUE
        && c.protected.len() <= MAX_CATALOGUE
        && c.readings.values().all(reading_ok)
        && c.unsupported
            .iter()
            .all(|f| text_ok(&f.store) && f.name.len() <= MAX_TEXT)
}

/// The settings catalogue's rows on this laptop, read only through its own mechanisms
/// ([`pctwin_settings::Store::capture`]); `None` when the catalogue cannot be read.
pub fn catalogue_here() -> Option<pctwin_settings::map::Capture> {
    let system = this_system();
    let cat = pctwin_settings::catalogue::Catalogue::starter().ok()?;
    let mut store =
        pctwin_settings::Store::new(cat, pctwin_settings::mechanism::Mechanisms::real(system));
    Some(store.capture())
}

/// Whether one found setting may cross: within every limit the new laptop checks. The one rule
/// both laptops use: the old laptop leaves out (and counts) what breaks it, the new laptop refuses
/// a message that holds one.
pub fn setting_ok(s: &FoundSetting) -> bool {
    let a = &s.address;
    text_ok(&s.label)
        && text_ok(a.store())
        && a.name().len() <= MAX_TEXT
        && match &s.value {
            pctwin_recipe::SettingValue::Text(t) => {
                t.len() <= pctwin_types::limits::SETTING_TEXT_BYTES
            }
            _ => true,
        }
}

/// The old laptop's own judgement of what it found, before anything is sent: the general rules
/// (secrets, security, machine identities, what would run or fetch...) run here, so a password or
/// a token never crosses the link; only what may move, or may once its path is pointed at a moved
/// folder on the new laptop, is sent. What stays is counted, never sent.
fn screened(system: System, d: Discovery) -> (Vec<FoundSetting>, Vec<NotMoved>) {
    use pctwin_settings::classify::{Ctx, FolderMap};
    let Ok(cat) = pctwin_settings::catalogue::Catalogue::starter() else {
        // Without the rules nothing is judged, so nothing is sent.
        let mut not = d.not_moved;
        not.extend(d.found.iter().map(|f| NotMoved {
            label: f.label.clone(),
            store: f.address.store().to_string(),
            name: f.address.name().to_string(),
            why: Why::Unreadable("the rules could not be read".into()),
        }));
        return (Vec::new(), not);
    };
    let none = FolderMap(Vec::new());
    let ctx = Ctx {
        paths: &none,
        same_screens: false,
        hints: pctwin_settings::catalogue::Catalogue::starter_hints(),
        facts: pctwin_settings::judge::Facts::system(),
    };
    let plan = pctwin_settings::generic::plan_found(&cat, system, system, &d, &ctx);
    let moves: std::collections::HashSet<String> =
        plan.moves.iter().map(|m| m.key.clone()).collect();
    // A path to something not moved may be one the new laptop moves: the new laptop decides.
    let for_the_new_laptop = |w: &Why| matches!(w, Why::PathNotMoved | Why::RecentMissing);
    let path_ones: std::collections::HashSet<(String, String)> = plan
        .not_moved
        .iter()
        .filter(|n| for_the_new_laptop(&n.why))
        .map(|n| (n.store.clone(), n.name.clone()))
        .collect();
    let kept: Vec<NotMoved> = plan
        .not_moved
        .into_iter()
        .filter(|n| !for_the_new_laptop(&n.why))
        .collect();
    let send = d
        .found
        .into_iter()
        .filter(|f| {
            moves.contains(&f.address.key())
                || path_ones
                    .contains(&(f.address.store().to_string(), f.address.name().to_string()))
        })
        .collect();
    (send, kept)
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
    let (mut found, mut not_moved) = screened(system, d);
    found.truncate(pctwin_types::limits::FOUND_SETTINGS);
    let mut seen = std::collections::HashSet::new();
    let mut settings = Vec::with_capacity(found.len());
    for s in found {
        if !seen.insert(s.address.key()) {
            continue;
        }
        if setting_ok(&s) {
            settings.push(s);
        } else {
            // Too long to cross (a value past its limit, say): left out, and counted.
            not_moved.push(NotMoved {
                label: s.label.clone(),
                store: s.address.store().to_string(),
                name: s.address.name().to_string(),
                why: Why::TooLarge,
            });
        }
    }
    Extras {
        format: FORMAT,
        system,
        not_moved: count_not_moved(&not_moved),
        settings,
        folders,
        special: Vec::new(),
        special_searched: false,
        special_not_moved: 0,
        special_not_checked: 0,
        links: 0,
        left_out_by_person: false,
        forgotten: None,
        wifi: None,
        catalogue: None,
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
    deadline: std::time::Instant,
) -> SpecialFound {
    special_of(
        pctwin_special::roots::System::here(),
        owner,
        home,
        apps,
        deadline,
    )
}

/// [`special_here`] for a home laid out as `system` lays out its people's homes: the laptop's
/// own system in a move ([`special_here`]); another one only in tests, which build a Windows
/// home on every system they run on.
pub fn special_of(
    system: pctwin_special::roots::System,
    owner: &pctwin_record::Owner,
    home: &std::path::Path,
    apps: &[pctwin_record::FoundApp],
    deadline: std::time::Instant,
) -> SpecialFound {
    special_with(system, owner, home, None, apps, deadline)
}

/// [`special_of`] with the person's Documents folder where the system has it (`None`: its plain
/// place in the home folder), for the forgotten-things finder's places kept there.
pub fn special_with(
    system: pctwin_special::roots::System,
    owner: &pctwin_record::Owner,
    home: &std::path::Path,
    documents: Option<&std::path::Path>,
    apps: &[pctwin_record::FoundApp],
    deadline: std::time::Instant,
) -> SpecialFound {
    use pctwin_special::component::Custody;
    let cat = pctwin_catalogue::Catalogue::<pctwin_special::catalogue::SpecialRow>::shipped(
        pctwin_special::catalogue::SHIPPED,
    );
    // Planned for this laptop's own kind of system: the old laptop says only where each
    // component is here (its root and folder). Where it goes on the new laptop is decided there,
    // from that laptop's own catalogue (`data::placed_here`), never by this laptop.
    let mut plan =
        pctwin_special::api::plan_within(owner, home, apps, system, system, &cat, deadline);
    // Games (2.5): saves the shipped rows and the person's own save list name, found by their
    // places; the app data holding them is marked as games, other save places join the plan.
    games_of(system, owner, home, documents, &mut plan, deadline);
    // Apple Mail (2.5 mail): its raw files are carried; the new laptop writes the mail kept only
    // on this Mac as mailbox files. A folder PCTwin could not look into is said, never "no mail".
    let mail = if system == pctwin_special::roots::System::Mac {
        mail_of(owner, home, &mut plan, deadline)
    } else {
        MailHere::None
    };
    // One line per app and step (an app's many components with the same step are one line, with
    // their files counted), in the order found.
    let mut lines: Vec<SpecialLine> = Vec::new();
    for p in plan.components.iter().filter(|p| text_ok(&p.app)) {
        let step = match p.component.custody {
            _ if p.root == pctwin_special::roots::RootKind::MacMail => SpecialStep::Convert,
            Custody::Portable => SpecialStep::Copy,
            Custody::ServiceHeld => SpecialStep::SignIn,
            Custody::MachineBound => SpecialStep::Relink,
            Custody::NeedsExport => SpecialStep::ExportFirst,
            Custody::Unresolved => SpecialStep::LookAtIt,
        };
        let files = p.component.members.len() as u64;
        let id = crate::data::id_hex(&p.component);
        let sites = p.component.evidence.iter().any(|e| {
            matches!(
                e,
                pctwin_special::component::Evidence::SessionStore
                    | pctwin_special::component::Evidence::CredentialValues
            )
        });
        // The folder a carried component holds (its files' first folder), for the new laptop.
        let part = (step == SpecialStep::Convert).then(|| {
            p.component
                .members
                .first()
                .and_then(|m| m.split('/').next())
                .unwrap_or_default()
                .to_string()
        });
        // A secret is always its own line: the person ticks each one by itself (D07).
        let same = |l: &&mut SpecialLine| {
            !p.component.secret
                && !l.secret
                && l.name == p.app
                && l.step == step
                && l.category == p.category
        };
        match lines.iter_mut().find(same) {
            Some(l) => {
                l.files += files;
                l.ids.push(id);
                l.parts.extend(part);
                l.sites |= sites;
                for n in &p.notes {
                    if !l.notes.contains(n) {
                        l.notes.push(n.clone());
                    }
                }
            }
            None => lines.push(SpecialLine {
                name: p.app.clone(),
                step,
                files,
                secret: p.component.secret,
                ids: vec![id],
                sites,
                category: p.category,
                notes: p.notes.clone(),
                bytes: 0,
                parts: part.into_iter().collect(),
            }),
        }
    }
    let mut mail_not_all = false;
    match mail {
        MailHere::None => {}
        MailHere::NoAccess(name) => lines.push(SpecialLine {
            name,
            step: SpecialStep::Convert,
            files: 0,
            secret: false,
            ids: Vec::new(),
            sites: false,
            category: pctwin_special::category::Category::Email,
            notes: vec![pctwin_special::category::Note::MailNoAccess],
            bytes: 0,
            parts: Vec::new(),
        }),
        MailHere::Found {
            name,
            bytes,
            not_all,
        } => {
            mail_not_all = not_all;
            if let Some(l) = lines
                .iter_mut()
                .find(|l| l.step == SpecialStep::Convert && l.name == name)
            {
                l.bytes = bytes;
            }
        }
    }
    lines.truncate(MAX_SPECIAL);
    // What 2.5 could not fully check (out of time, no answer, unreadable) is said, never dropped.
    let not_checked = plan
        .not_moved
        .iter()
        .filter(|n| n.why.contains("not fully checked"))
        .map(|n| n.count.max(1))
        .sum::<usize>()
        + plan.coverage.unreadable.len()
        + usize::from(mail_not_all);
    let forgotten = forgotten_of(system, home, documents, &plan.components, deadline);
    SpecialFound {
        forgotten,
        lines,
        not_moved: plan.not_moved.iter().map(|n| n.count.max(1) as u64).sum(),
        not_checked: not_checked as u64,
        planned: plan.components,
    }
}

/// Adds the games 2.5 finds to `plan` (read only, within the scan's time and its own).
fn games_of(
    system: pctwin_special::roots::System,
    owner: &pctwin_record::Owner,
    home: &std::path::Path,
    documents: Option<&std::path::Path>,
    plan: &mut pctwin_special::api::Plan,
    deadline: std::time::Instant,
) {
    let docs = documents
        .map(std::path::Path::to_path_buf)
        .unwrap_or_else(|| home.join("Documents"));
    // This program's own folder values, only where they lie in this person's home (another
    // person's folders come from their home's plain layout).
    let env = |v: &str| {
        std::env::var_os(v)
            .map(std::path::PathBuf::from)
            .filter(|p| local_folder(p) && p.starts_with(home))
    };
    // Saves among the person's own files move with them.
    let moving = |p: &std::path::Path| p.starts_with(&docs);
    let until = deadline.min(std::time::Instant::now() + GAMES_TIME);
    let _ = pctwin_special::games::plan_games(
        plan,
        owner,
        home,
        Some(docs.clone()),
        system,
        system,
        &env,
        &moving,
        until,
    );
}

/// What 2.5 found of Apple Mail on this laptop.
enum MailHere {
    None,
    /// Its folder could not be looked into (named as the catalogue names it).
    NoAccess(String),
    /// Its files joined the plan.
    Found {
        name: String,
        bytes: u64,
        not_all: bool,
    },
}

/// Apple Mail's raw files of `owner` (home `home`), added to `plan` as components carried raw
/// (nothing in them is opened here), named as the `special-places` catalogue names the place.
fn mail_of(
    owner: &pctwin_record::Owner,
    home: &std::path::Path,
    plan: &mut pctwin_special::api::Plan,
    deadline: std::time::Instant,
) -> MailHere {
    use pctwin_special::applemail::{self, Found};
    use pctwin_special::forgotten::{self, PlaceStep};
    let rows = pctwin_catalogue::Catalogue::<forgotten::PlaceRow>::shipped(forgotten::SHIPPED);
    let place = applemail::MAIL_FOLDER.join("/");
    let Some(name) = rows
        .rows()
        .iter()
        .find(|r| {
            r.system.covers(pctwin_special::roots::System::Mac)
                && r.path == place
                && r.step == PlaceStep::ExportFirst
        })
        .map(|r| r.name.clone())
        .filter(|n| text_ok(n))
    else {
        // No row says how this mail moves: nothing is carried.
        return MailHere::None;
    };
    match applemail::find(owner, home, deadline) {
        Found::NotHere => MailHere::None,
        Found::NoAccess => MailHere::NoAccess(name),
        Found::Mail(m) => {
            for c in m.components {
                plan.components.push(pctwin_special::api::Planned {
                    app: name.clone(),
                    why: String::new(),
                    step: pctwin_special::handoff::Step::None,
                    proposed: true,
                    component: c,
                    base: m.base.clone(),
                    placement: pctwin_special::classify::Placement::PreservedOnly,
                    root: pctwin_special::roots::RootKind::MacMail,
                    category: pctwin_special::category::Category::Email,
                    notes: vec![pctwin_special::category::Note::MailImport],
                });
            }
            MailHere::Found {
                name,
                bytes: m.bytes,
                not_all: m.not_all,
            }
        }
    }
}

/// The most time finding games takes.
pub const GAMES_TIME: std::time::Duration = std::time::Duration::from_secs(20);

/// The forgotten-things finder for the person with home folder `home` (read only), with the
/// places the plan already takes (`planned`: a place inside a proposed component's folder moves
/// with it; a place in Documents moves with the person's files).
fn forgotten_of(
    system: pctwin_special::roots::System,
    home: &std::path::Path,
    documents: Option<&std::path::Path>,
    planned: &[pctwin_special::api::Planned],
    deadline: std::time::Instant,
) -> pctwin_special::forgotten::Finding {
    use pctwin_special::forgotten::{self, Under};
    let rows = pctwin_catalogue::Catalogue::<forgotten::PlaceRow>::shipped(forgotten::SHIPPED);
    let roots = pctwin_special::roots::roots_of(system, home, &|_| None);
    let docs = documents
        .map(std::path::Path::to_path_buf)
        .unwrap_or_else(|| home.join("Documents"));
    let env = |v: &str| {
        std::env::var_os(v)
            .map(std::path::PathBuf::from)
            .filter(|p| local_folder(p))
    };
    let under = |u: Under| match u {
        Under::Home => Some(home.to_path_buf()),
        Under::Documents => Some(docs.clone()),
        Under::ProgramFiles => env("ProgramFiles"),
        Under::ProgramFilesX86 => env("ProgramFiles(x86)"),
        Under::ProgramData => env("ProgramData"),
    };
    let moving = |p: &std::path::Path| {
        p.starts_with(&docs)
            || planned
                .iter()
                .any(|c| c.proposed && !c.base.as_os_str().is_empty() && p.starts_with(&c.base))
            // Apple Mail's folder, whose raw files the move carries (`V<n>` is inside it).
            || planned.iter().any(|c| {
                c.root == pctwin_special::roots::RootKind::MacMail
                    && c.base.parent().is_some_and(|mail| p.starts_with(mail))
            })
    };
    // The finder only counts; it keeps to the scan's own time, and to its own.
    let until = deadline.min(std::time::Instant::now() + FORGOTTEN_TIME);
    let mut f = forgotten::find(system, home, &roots, &under, &rows, &moving, until);
    f.found
        .retain(|x| text_ok(&x.kind) && text_ok(&x.name) && text_ok(&x.place));
    f.found.truncate(MAX_FORGOTTEN);
    f.printers.retain(|p| text_ok(&p.name));
    f.not_checked.retain(|(a, b)| text_ok(a) && text_ok(b));
    f.not_checked.truncate(MAX_FORGOTTEN);
    f
}

/// A folder named by an environment value, accepted only on a drive of this laptop: never a
/// network or device path (a share, `\\?\UNC\...`, `\\.\...`), which would make the finder reach
/// out to another computer (security review of 2.5, F3).
fn local_folder(p: &std::path::Path) -> bool {
    if !p.is_absolute() {
        return false;
    }
    #[cfg(windows)]
    {
        use std::path::{Component, Prefix};
        matches!(
            p.components().next(),
            Some(Component::Prefix(x)) if matches!(x.kind(), Prefix::Disk(_) | Prefix::VerbatimDisk(_))
        )
    }
    #[cfg(not(windows))]
    {
        true
    }
}

/// The most places the finder's answer carries (and places it could not check).
pub const MAX_FORGOTTEN: usize = 200;
/// The most time the forgotten-things finder takes.
pub const FORGOTTEN_TIME: std::time::Duration = std::time::Duration::from_secs(20);

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
    /// The forgotten-things finder's answer.
    pub forgotten: pctwin_special::forgotten::Finding,
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
        Self::start_for(pctwin_special::roots::System::here(), owner, home)
    }

    /// [`SpecialScan::start`] with the person's Documents folder where the system has it.
    pub fn start_with_documents(
        owner: &pctwin_record::Owner,
        home: &std::path::Path,
        documents: Option<std::path::PathBuf>,
    ) -> Option<Self> {
        Self::spawn(
            pctwin_special::roots::System::here(),
            owner,
            home,
            documents,
        )
    }

    /// [`SpecialScan::start`] for a home laid out as `system` lays out its people's homes (see
    /// [`special_of`]).
    pub fn start_for(
        system: pctwin_special::roots::System,
        owner: &pctwin_record::Owner,
        home: &std::path::Path,
    ) -> Option<Self> {
        Self::spawn(system, owner, home, None)
    }

    fn spawn(
        system: pctwin_special::roots::System,
        owner: &pctwin_record::Owner,
        home: &std::path::Path,
        documents: Option<std::path::PathBuf>,
    ) -> Option<Self> {
        let (tx, rx) = std::sync::mpsc::channel();
        let (owner, home) = (owner.clone(), home.to_path_buf());
        std::thread::Builder::new()
            .name("pctwin-special-scan".into())
            .spawn(move || {
                // 2.5 stops reading in time for its answer to arrive within the budget.
                let deadline = std::time::Instant::now() + SPECIAL_BUDGET - SPECIAL_MARGIN;
                let _ = tx.send(special_with(
                    system,
                    &owner,
                    &home,
                    documents.as_deref(),
                    &[],
                    deadline,
                ));
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
            self.forgotten = Some(f.forgotten.clone());
        }
    }
}

/// How a laptop of settings' kind of system `s` lays out a person's home (2.5's roots).
pub fn roots_system(s: System) -> pctwin_special::roots::System {
    match s {
        System::Windows => pctwin_special::roots::System::Windows,
        System::Mac => pctwin_special::roots::System::Mac,
        System::Gnome | System::Kde | System::OtherLinux => pctwin_special::roots::System::Linux,
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

/// How much earlier than the budget 2.5 is asked to stop reading, so its answer is ready.
pub const SPECIAL_MARGIN: std::time::Duration = std::time::Duration::from_secs(30);

#[cfg(test)]
mod tests {
    use super::page_ok;

    /// Only a folder on a drive of this laptop is taken from an environment value (F3).
    #[cfg(windows)]
    #[test]
    fn only_a_local_drive_folder_is_taken_from_the_environment() {
        use super::local_folder;
        assert!(local_folder(std::path::Path::new(r"C:\Program Files")));
        for bad in [
            r"\\server\share\x",
            r"\\?\UNC\server\share",
            r"\\.\pipe\x",
            "Program Files",
        ] {
            assert!(!local_folder(std::path::Path::new(bad)), "{bad}");
        }
    }

    /// A help page the other laptop sends is accepted only as a plain https address within the
    /// limits; anything else refuses the whole message (security review of 2.5, F2).
    #[test]
    fn only_a_plain_https_page_crosses() {
        assert!(page_ok(None));
        assert!(page_ok(Some("https://support.example.org/sync")));
        for bad in [
            "http://example.org/",
            "javascript:alert(1)",
            "file:///C:/x",
            "https://example.org/ a",
            "https://example.org/\u{0007}",
            "",
        ] {
            assert!(!page_ok(Some(bad)), "{bad}");
        }
        assert!(!page_ok(Some(&format!("https://{}", "a".repeat(5000)))));
    }

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

    /// A line converted on the new laptop names one plain folder per component, or none; a list
    /// of another length, or a name that is a path, is refused.
    #[test]
    fn a_converted_line_names_one_plain_folder_per_component() {
        let line = |ids: &[&str], parts: &[&str]| SpecialLine {
            name: "Apple Mail".into(),
            step: SpecialStep::Convert,
            files: 2,
            secret: false,
            ids: ids.iter().map(|s| s.to_string()).collect(),
            sites: false,
            category: pctwin_special::category::Category::Email,
            notes: Vec::new(),
            bytes: 10,
            parts: parts.iter().map(|s| s.to_string()).collect(),
        };
        let with = |l: SpecialLine| {
            let mut x = sample();
            x.special = vec![l];
            x.check()
        };
        assert_eq!(with(line(&["a", "b"], &["Mailboxes", "MailData"])), Ok(()));
        assert_eq!(with(line(&["a", "b"], &[])), Ok(()));
        for bad in [
            line(&["a", "b"], &["Mailboxes"]),
            line(&["a"], &["Mailboxes", "MailData"]),
            line(&["a", "b"], &["Mailboxes", "x/y"]),
            line(&["a", "b"], &["Mailboxes", "x\\y"]),
            line(&["a", "b"], &["Mailboxes", ".."]),
            line(&["a", "b"], &["Mailboxes", ""]),
        ] {
            assert_eq!(
                with(bad.clone()),
                Err(ExtrasError::Limit),
                "{:?}",
                bad.parts
            );
        }
    }

    #[test]
    fn the_forgotten_things_found_cross_within_their_bounds() {
        use pctwin_special::forgotten::{Finding, Forgotten, MAX_PRINTERS, PlaceStep, Printer};
        let place = Forgotten {
            kind: "fonts".into(),
            name: "Your own fonts".into(),
            place: "AppData/Local/Microsoft/Windows/Fonts".into(),
            files: 2,
            bytes: 10,
            online_only: 0,
            complete: true,
            moving: false,
            step: PlaceStep::Install,
            page: None,
        };
        let printer = Printer {
            name: "Office".into(),
            network: false,
        };
        let mut x = sample();
        x.forgotten = Some(Finding {
            found: vec![place.clone()],
            printers: vec![printer.clone()],
            not_checked: vec![("Library/Mail".into(), "denied".into())],
        });
        assert_eq!(Extras::decode(&x.encode()).unwrap(), x);
        let with = |f: &dyn Fn(&mut Finding)| {
            let mut y = x.clone();
            f(y.forgotten.as_mut().unwrap());
            Extras::decode(&y.encode())
        };
        assert!(with(&|f| f.found = vec![place.clone(); MAX_FORGOTTEN]).is_ok());
        assert!(with(&|f| f.printers = vec![printer.clone(); MAX_PRINTERS]).is_ok());
        type Change = Box<dyn Fn(&mut Finding)>;
        let bad: Vec<Change> = vec![
            Box::new(move |f| f.found = vec![f.found[0].clone(); MAX_FORGOTTEN + 1]),
            Box::new(|f| f.not_checked = vec![("a".into(), "b".into()); MAX_FORGOTTEN + 1]),
            Box::new(|f| f.printers = vec![f.printers[0].clone(); MAX_PRINTERS + 1]),
            Box::new(|f| f.found[0].kind = "a\u{7}".into()),
            Box::new(|f| f.found[0].name = String::new()),
            Box::new(|f| f.found[0].place = "x".repeat(MAX_TEXT + 1)),
            Box::new(|f| f.printers[0].name = "\u{0}".into()),
            Box::new(|f| f.not_checked[0].0 = String::new()),
            Box::new(|f| f.not_checked[0].1 = "\n".into()),
        ];
        for (i, b) in bad.iter().enumerate() {
            assert_eq!(with(b.as_ref()), Err(ExtrasError::Limit), "case {i}");
        }
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

#[cfg(test)]
mod catalogue_tests {
    use super::*;
    use pctwin_settings::map::{Capture, Reading};

    fn capture(system: System) -> Capture {
        let mut c = Capture::new(system, 1);
        c.readings.insert(
            pctwin_settings::SettingId::new("appearance.theme").unwrap(),
            Reading::Value(pctwin_recipe::SettingValue::Text("dark".into())),
        );
        c
    }

    /// The catalogue's rows cross in the extras (move protocol 9); an older new laptop is sent
    /// none (the field is left out of the bytes), and an older old laptop's extras, without
    /// it, read as before.
    #[test]
    fn the_catalogues_rows_cross_and_older_laptops_read_as_before() {
        let mut x = Extras::empty(System::Windows);
        let before = x.encode();
        assert!(!String::from_utf8_lossy(&before).contains("catalogue"));
        assert_eq!(Extras::decode(&before).unwrap().catalogue, None);
        x.with_catalogue(capture(System::Windows));
        let bytes = x.encode();
        assert_eq!(
            Extras::decode(&bytes).unwrap().catalogue,
            Some(capture(System::Windows))
        );
    }

    /// A capture said to be another system's, or past its bounds, is refused with the extras.
    #[test]
    fn a_capture_of_another_system_or_past_its_bounds_is_refused() {
        let mut x = Extras::empty(System::Windows);
        x.with_catalogue(capture(System::Mac));
        assert_eq!(Extras::decode(&x.encode()), Err(ExtrasError::Limit));
        let mut big = capture(System::Windows);
        big.readings.insert(
            pctwin_settings::SettingId::new("appearance.theme").unwrap(),
            Reading::Value(pctwin_recipe::SettingValue::Text(
                "x".repeat(pctwin_types::limits::SETTING_TEXT_BYTES + 1),
            )),
        );
        let mut x = Extras::empty(System::Windows);
        x.with_catalogue(big);
        assert_eq!(Extras::decode(&x.encode()), Err(ExtrasError::Limit));
    }
}
