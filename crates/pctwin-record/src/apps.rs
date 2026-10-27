//! The apps found on the old laptop (Phase 2.1, discovery first: ADOPTION changes 3, 7 and 8).
//!
//! Every installed app found, for every person, is listed here with who made it ([`AppMaker`]),
//! read from the old laptop's own copy: never from a catalogue, never from a name. The new laptop
//! looks each one up by that maker; a catalogue only adds hints. Nothing found is dropped: an app
//! whose details could not be read is listed with the reason ([`AppMaker::NotRead`]), and a place
//! that could not be read at all ("Ada's own apps, while she is signed out") is listed in
//! [`AppList::not_read`].
//!
//! Everything here is what the old laptop *recorded*: the name, version, publisher text and web
//! address are clues for the person and the route finder, never proof of anything. Only the
//! maker (the Windows signer, the Mac Team ID, the Linux repository or Flatpak key) anchors trust,
//! and the new laptop checks every download against it again.
//!
//! The list crosses in the inventory manifest from format 4 (move protocol 5), in a compact form
//! written by hand like the rest of the inventory ([`write_apps`], [`read_apps`]): bounded, every
//! text checked, nothing PCTwin never writes accepted.

use serde::{Deserialize, Serialize};

use crate::{ItemId, Owner, RecordError};

/// Most apps in one list. A laptop with more lists the first this many and says so in
/// [`AppList::not_read`].
pub const MAX_APPS: usize = 8192;
/// Most places listed as not read.
pub const MAX_NOT_READ: usize = 1024;
/// Longest name, version, publisher, ID or reason, in bytes.
pub const MAX_APP_TEXT: usize = 512;
/// Longest web address, in bytes.
pub const MAX_APP_URL: usize = 2048;
/// Most installers on the old laptop linked to one app.
pub const MAX_INSTALLERS: usize = 16;
/// Most notes on one app.
pub const MAX_NOTES: usize = 8;

/// The system an app was found on.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum AppSystem {
    Windows,
    Mac,
    Linux,
}

/// Which list of Windows' installed programs an entry came from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum RegistryView {
    /// The machine's list for 64-bit programs (or the only one on 32-bit Windows).
    Machine,
    /// The machine's list for 32-bit programs on 64-bit Windows.
    Machine32,
    /// One person's own list.
    Person,
}

/// How Windows says a packaged (MSIX/Appx) app is signed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum PackageSigning {
    /// Signed by the Microsoft Store.
    Store,
    /// Signed by its maker and installed from outside the Store.
    Developer,
    /// Part of Windows itself.
    System,
    /// Signed by an organisation for its own laptops.
    Enterprise,
    /// Not signed (a developer's test install).
    None,
}

/// Where on the old laptop an app was found: the system's own record of it.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", tag = "kind")]
pub enum FoundWhere {
    /// An entry in Windows' list of installed programs ("Apps & features").
    WindowsEntry {
        view: RegistryView,
        /// The entry's key name (an MSI product code for Windows Installer packages).
        key: String,
        /// The Windows Installer product code, when the entry is one.
        product_code: Option<String>,
    },
    /// A packaged Windows app (MSIX/Appx), by its package family name.
    WindowsPackage {
        family_name: String,
        signing: PackageSigning,
    },
    /// A Mac app bundle.
    MacBundle {
        bundle_id: String,
        /// The bundle's folder name, `Name.app`.
        folder: String,
        /// The Mac App Store's ID for it, when it came from the App Store.
        app_store_id: Option<u64>,
    },
    /// A Debian or Ubuntu package that gives the desktop an app.
    Deb { package: String },
    /// A Fedora or RPM package that gives the desktop an app.
    Rpm { package: String },
    /// A Flatpak app, with the remote it came from on the old laptop.
    Flatpak {
        app_id: String,
        remote: String,
        /// The remote's address as the old laptop recorded it.
        remote_url: Option<String>,
        /// Installed for everyone (true) or for this person alone.
        system_wide: bool,
    },
    /// A Snap.
    Snap { name: String },
}

/// The processors an app's own program is built for. `known: false` when it could not be read.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct AppArch {
    pub known: bool,
    pub x86: bool,
    pub x64: bool,
    pub arm64: bool,
}

impl AppArch {
    pub const UNKNOWN: AppArch = AppArch {
        known: false,
        x86: false,
        x64: false,
        arm64: false,
    };

    fn code(self) -> u8 {
        u8::from(self.known)
            | u8::from(self.x86) << 1
            | u8::from(self.x64) << 2
            | u8::from(self.arm64) << 3
    }

    fn of(code: u8) -> Option<Self> {
        let a = AppArch {
            known: code & 1 != 0,
            x86: code & 2 != 0,
            x64: code & 4 != 0,
            arm64: code & 8 != 0,
        };
        // Nothing beyond the four bits, and no processors claimed when they are not known.
        (code < 16 && (a.known || code == 0)).then_some(a)
    }

    /// Built for Intel Macs only: it needs Rosetta on an Apple-silicon Mac, which Apple is
    /// ending.
    pub fn intel_only(self) -> bool {
        self.known && self.x64 && !self.arm64
    }
}

/// The Windows signer of an app's own program, as the old laptop's copy names it (mirrors
/// `pctwin_maker::facts::WindowsSigner`, which `pctwin-apps` converts to and from).
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SignerId {
    pub common_name: Option<String>,
    pub organization: Option<String>,
    pub locality: Option<String>,
    pub state: Option<String>,
    pub country: Option<String>,
    pub registration_number: Option<String>,
    pub jurisdiction_country: Option<String>,
    pub jurisdiction_state: Option<String>,
    /// SHA-256 of the whole signing certificate, hex.
    pub certificate: [u8; 32],
    pub issuer: String,
    pub issuer_organization: Option<String>,
    /// SHA-256 of the certificate's public key, when read.
    pub public_key: Option<[u8; 32]>,
}

/// Who made an app, from the old laptop's own copy.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", tag = "kind")]
pub enum AppMaker {
    /// The Windows signer of the app's own program or installer package.
    Windows(Box<SignerId>),
    /// The publisher of a packaged Windows app, as its package identity names it (Windows
    /// itself checks the package's signature against it).
    WindowsPackage { publisher: String },
    /// Apple's Team ID for the developer.
    MacTeam { team_id: String },
    /// An RPM package's signing key, as rpm names it.
    Rpm { key: String },
    /// A Debian or Ubuntu package: the repository it came from (`Origin`), or the snap
    /// publisher.
    Repository { origin: String },
    /// A Flatpak app's remote, app ID and the SHA-256 of the remote's signing key.
    Flatpak {
        remote_url: String,
        app_id: String,
        key: [u8; 32],
    },
    /// Read, and carries no maker signature: who made it cannot be proven.
    Unsigned,
    /// Could not be read, with the reason in plain words.
    NotRead { why: String },
}

/// Something the person may need to know to get the app working again.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", tag = "kind")]
pub enum AppNote {
    /// The registration the old laptop's own list of programs holds (who it is registered to and
    /// its product ID). Never a key read from a protected store.
    Registration {
        owner: Option<String>,
        company: Option<String>,
        product_id: Option<String>,
    },
}

/// One app found on the old laptop.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FoundApp {
    /// The name the old laptop shows for it.
    pub name: String,
    pub version: Option<String>,
    /// The publisher text the old laptop recorded: a clue, never proof.
    pub publisher: Option<String>,
    /// Whose it is: one person's, or everyone's.
    pub owner: Owner,
    pub system: AppSystem,
    pub found: FoundWhere,
    pub arch: AppArch,
    pub maker: AppMaker,
    /// The maker's web address as the old laptop recorded it (for the person's card).
    pub web: Option<String>,
    /// The app's own update feed address, read from the app itself (a Sparkle feed, say).
    pub update_feed: Option<String>,
    /// Installer files in this move signed by the same maker, found on the old laptop.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub installers: Vec<ItemId>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub notes: Vec<AppNote>,
}

/// A place apps live that could not be read at all.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AppsNotRead {
    pub owner: Owner,
    pub place: AppPlace,
    /// Why, in plain words.
    pub why: String,
}

/// The places apps live, by kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum AppPlace {
    /// Windows' list of programs installed for everyone.
    WindowsMachineList,
    /// One person's own list of Windows programs.
    WindowsPersonList,
    /// One person's packaged (Store) apps.
    WindowsPackages,
    /// The Mac's Applications folder.
    MacApplications,
    /// One person's own Applications folder.
    MacPersonApplications,
    /// The Linux system's packages.
    LinuxPackages,
    /// Flatpak apps.
    Flatpak,
    /// Snaps.
    Snap,
    /// More apps than one list holds: the rest were not listed.
    TooMany,
}

/// Every app found on the old laptop, and every place that could not be read.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AppList {
    pub found: Vec<FoundApp>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub not_read: Vec<AppsNotRead>,
}

impl AppList {
    pub fn is_empty(&self) -> bool {
        self.found.is_empty() && self.not_read.is_empty()
    }

    /// The rules every list keeps: within its bounds, every text plain and within its length.
    pub fn validate(&self) -> Result<(), RecordError> {
        if self.found.len() > MAX_APPS || self.not_read.len() > MAX_NOT_READ {
            return Err(bad("more apps than one list holds"));
        }
        for app in &self.found {
            app.validate()?;
        }
        for n in &self.not_read {
            owner_ok(&n.owner)?;
            text(&n.why, MAX_APP_TEXT)?;
        }
        Ok(())
    }
}

fn bad(why: &'static str) -> RecordError {
    RecordError::BadInventory(why)
}

/// Text PCTwin keeps: within `max` bytes, with no control characters and no hidden direction or
/// zero-width marks (it is shown to the person).
pub fn text_ok(s: &str, max: usize) -> bool {
    s.len() <= max
        && !s.chars().any(|c| {
            c.is_control()
                || matches!(c, '\u{200B}'..='\u{200F}' | '\u{202A}'..='\u{202E}' | '\u{2066}'..='\u{2069}' | '\u{FEFF}')
        })
}

fn text(s: &str, max: usize) -> Result<(), RecordError> {
    if text_ok(s, max) {
        Ok(())
    } else {
        Err(bad("an app's text is too long or holds hidden characters"))
    }
}

fn opt(s: &Option<String>, max: usize) -> Result<(), RecordError> {
    s.as_deref().map_or(Ok(()), |s| text(s, max))
}

fn owner_ok(o: &Owner) -> Result<(), RecordError> {
    match o {
        Owner::Person { account_id } if account_id.is_empty() => {
            Err(bad("an app's owner has no account"))
        }
        Owner::Person { account_id } => text(account_id, MAX_APP_TEXT),
        Owner::Shared | Owner::System => Ok(()),
    }
}

impl FoundApp {
    pub fn validate(&self) -> Result<(), RecordError> {
        if self.name.trim().is_empty() {
            return Err(bad("an app without a name"));
        }
        text(&self.name, MAX_APP_TEXT)?;
        opt(&self.version, MAX_APP_TEXT)?;
        opt(&self.publisher, MAX_APP_TEXT)?;
        opt(&self.web, MAX_APP_URL)?;
        opt(&self.update_feed, MAX_APP_URL)?;
        owner_ok(&self.owner)?;
        if self.installers.len() > MAX_INSTALLERS || self.notes.len() > MAX_NOTES {
            return Err(bad("an app with too many installers or notes"));
        }
        match &self.found {
            FoundWhere::WindowsEntry {
                key, product_code, ..
            } => {
                text(key, MAX_APP_TEXT)?;
                opt(product_code, MAX_APP_TEXT)?;
            }
            FoundWhere::WindowsPackage { family_name, .. } => text(family_name, MAX_APP_TEXT)?,
            FoundWhere::MacBundle {
                bundle_id, folder, ..
            } => {
                text(bundle_id, MAX_APP_TEXT)?;
                text(folder, MAX_APP_TEXT)?;
            }
            FoundWhere::Deb { package } | FoundWhere::Rpm { package } => {
                text(package, MAX_APP_TEXT)?
            }
            FoundWhere::Flatpak {
                app_id,
                remote,
                remote_url,
                ..
            } => {
                text(app_id, MAX_APP_TEXT)?;
                text(remote, MAX_APP_TEXT)?;
                opt(remote_url, MAX_APP_URL)?;
            }
            FoundWhere::Snap { name } => text(name, MAX_APP_TEXT)?,
        }
        match &self.maker {
            AppMaker::Windows(s) => {
                for f in [
                    &s.common_name,
                    &s.organization,
                    &s.locality,
                    &s.state,
                    &s.country,
                    &s.registration_number,
                    &s.jurisdiction_country,
                    &s.jurisdiction_state,
                    &s.issuer_organization,
                ] {
                    opt(f, MAX_APP_TEXT)?;
                }
                text(&s.issuer, MAX_APP_TEXT)?;
            }
            AppMaker::WindowsPackage { publisher: t }
            | AppMaker::MacTeam { team_id: t }
            | AppMaker::Rpm { key: t }
            | AppMaker::Repository { origin: t }
            | AppMaker::NotRead { why: t } => text(t, MAX_APP_TEXT)?,
            AppMaker::Flatpak {
                remote_url, app_id, ..
            } => {
                text(remote_url, MAX_APP_URL)?;
                text(app_id, MAX_APP_TEXT)?;
            }
            AppMaker::Unsigned => {}
        }
        for n in &self.notes {
            let AppNote::Registration {
                owner,
                company,
                product_id,
            } = n;
            opt(owner, MAX_APP_TEXT)?;
            opt(company, MAX_APP_TEXT)?;
            opt(product_id, MAX_APP_TEXT)?;
        }
        Ok(())
    }
}

// ---------------------------------------------------------------- the inventory form

/// What one app takes in the reader before its texts (the app and its place in the list, twice
/// over as the list grows).
const APP_COST: u64 = 2 * 640;
/// What each text takes beyond its bytes.
const TEXT_COST: u64 = 48;

struct Out<'a>(&'a mut Vec<u8>);

impl Out<'_> {
    fn u8(&mut self, v: u8) {
        self.0.push(v);
    }
    fn u32(&mut self, v: u32) {
        self.0.extend_from_slice(&v.to_be_bytes());
    }
    fn text(&mut self, s: &str) {
        // Lengths are bounded by `validate` (at most MAX_APP_URL), so they fit.
        self.u32(s.len() as u32);
        self.0.extend_from_slice(s.as_bytes());
    }
    fn opt(&mut self, s: &Option<String>) {
        match s {
            None => self.u8(0),
            Some(s) => {
                self.u8(1);
                self.text(s);
            }
        }
    }
    fn owner(&mut self, o: &Owner) {
        match o {
            Owner::Person { account_id } => {
                self.u8(0);
                self.text(account_id);
            }
            Owner::Shared => self.u8(1),
            Owner::System => self.u8(2),
        }
    }
}

/// Writes `apps` (checked first) in the inventory's form.
pub(crate) fn write_apps(out: &mut Vec<u8>, apps: &AppList) -> Result<(), RecordError> {
    apps.validate()?;
    let mut w = Out(out);
    w.u32(apps.found.len() as u32);
    for a in &apps.found {
        w.text(&a.name);
        w.opt(&a.version);
        w.opt(&a.publisher);
        w.owner(&a.owner);
        w.u8(match a.system {
            AppSystem::Windows => 0,
            AppSystem::Mac => 1,
            AppSystem::Linux => 2,
        });
        match &a.found {
            FoundWhere::WindowsEntry {
                view,
                key,
                product_code,
            } => {
                w.u8(0);
                w.u8(match view {
                    RegistryView::Machine => 0,
                    RegistryView::Machine32 => 1,
                    RegistryView::Person => 2,
                });
                w.text(key);
                w.opt(product_code);
            }
            FoundWhere::WindowsPackage {
                family_name,
                signing,
            } => {
                w.u8(1);
                w.text(family_name);
                w.u8(match signing {
                    PackageSigning::Store => 0,
                    PackageSigning::Developer => 1,
                    PackageSigning::System => 2,
                    PackageSigning::Enterprise => 3,
                    PackageSigning::None => 4,
                });
            }
            FoundWhere::MacBundle {
                bundle_id,
                folder,
                app_store_id,
            } => {
                w.u8(2);
                w.text(bundle_id);
                w.text(folder);
                match app_store_id {
                    None => w.u8(0),
                    Some(id) => {
                        w.u8(1);
                        w.0.extend_from_slice(&id.to_be_bytes());
                    }
                }
            }
            FoundWhere::Deb { package } => {
                w.u8(3);
                w.text(package);
            }
            FoundWhere::Rpm { package } => {
                w.u8(4);
                w.text(package);
            }
            FoundWhere::Flatpak {
                app_id,
                remote,
                remote_url,
                system_wide,
            } => {
                w.u8(5);
                w.text(app_id);
                w.text(remote);
                w.opt(remote_url);
                w.u8(u8::from(*system_wide));
            }
            FoundWhere::Snap { name } => {
                w.u8(6);
                w.text(name);
            }
        }
        w.u8(a.arch.code());
        match &a.maker {
            AppMaker::Windows(s) => {
                w.u8(0);
                for f in [
                    &s.common_name,
                    &s.organization,
                    &s.locality,
                    &s.state,
                    &s.country,
                    &s.registration_number,
                    &s.jurisdiction_country,
                    &s.jurisdiction_state,
                ] {
                    w.opt(f);
                }
                w.0.extend_from_slice(&s.certificate);
                w.text(&s.issuer);
                w.opt(&s.issuer_organization);
                match &s.public_key {
                    None => w.u8(0),
                    Some(k) => {
                        w.u8(1);
                        w.0.extend_from_slice(k);
                    }
                }
            }
            AppMaker::WindowsPackage { publisher } => {
                w.u8(1);
                w.text(publisher);
            }
            AppMaker::MacTeam { team_id } => {
                w.u8(2);
                w.text(team_id);
            }
            AppMaker::Rpm { key } => {
                w.u8(3);
                w.text(key);
            }
            AppMaker::Repository { origin } => {
                w.u8(4);
                w.text(origin);
            }
            AppMaker::Flatpak {
                remote_url,
                app_id,
                key,
            } => {
                w.u8(5);
                w.text(remote_url);
                w.text(app_id);
                w.0.extend_from_slice(key);
            }
            AppMaker::Unsigned => w.u8(6),
            AppMaker::NotRead { why } => {
                w.u8(7);
                w.text(why);
            }
        }
        w.opt(&a.web);
        w.opt(&a.update_feed);
        w.u8(a.installers.len() as u8);
        for id in &a.installers {
            w.0.extend_from_slice(&id.0);
        }
        w.u8(a.notes.len() as u8);
        for n in &a.notes {
            let AppNote::Registration {
                owner,
                company,
                product_id,
            } = n;
            w.u8(0);
            w.opt(owner);
            w.opt(company);
            w.opt(product_id);
        }
    }
    w.u32(apps.not_read.len() as u32);
    for n in &apps.not_read {
        w.owner(&n.owner);
        w.u8(place_code(n.place));
        w.text(&n.why);
    }
    Ok(())
}

const PLACES: [AppPlace; 9] = [
    AppPlace::WindowsMachineList,
    AppPlace::WindowsPersonList,
    AppPlace::WindowsPackages,
    AppPlace::MacApplications,
    AppPlace::MacPersonApplications,
    AppPlace::LinuxPackages,
    AppPlace::Flatpak,
    AppPlace::Snap,
    AppPlace::TooMany,
];

fn place_code(p: AppPlace) -> u8 {
    PLACES.iter().position(|q| *q == p).unwrap_or(0) as u8
}

/// Reads the apps written by [`write_apps`] from the front of `rest`, as from a stranger: every
/// length checked before it is read, every kind known, the result checked by the same rules.
/// Returns what the list will hold in memory, as the inventory reader counts it.
pub(crate) fn read_apps(rest: &mut &[u8]) -> Result<(AppList, u64), RecordError> {
    let mut r = In { rest, held: 0 };
    let n = r.u32()? as usize;
    if n > MAX_APPS {
        return Err(bad("more apps than one list holds"));
    }
    let mut found = Vec::with_capacity(n.min(1024));
    for _ in 0..n {
        r.held += APP_COST;
        let name = r.text(MAX_APP_TEXT)?;
        let version = r.opt(MAX_APP_TEXT)?;
        let publisher = r.opt(MAX_APP_TEXT)?;
        let owner = r.owner()?;
        let system = match r.u8()? {
            0 => AppSystem::Windows,
            1 => AppSystem::Mac,
            2 => AppSystem::Linux,
            _ => return Err(bad("an unknown kind of system")),
        };
        let found_where = match r.u8()? {
            0 => {
                let view = match r.u8()? {
                    0 => RegistryView::Machine,
                    1 => RegistryView::Machine32,
                    2 => RegistryView::Person,
                    _ => return Err(bad("an unknown list of programs")),
                };
                FoundWhere::WindowsEntry {
                    view,
                    key: r.text(MAX_APP_TEXT)?,
                    product_code: r.opt(MAX_APP_TEXT)?,
                }
            }
            1 => FoundWhere::WindowsPackage {
                family_name: r.text(MAX_APP_TEXT)?,
                signing: match r.u8()? {
                    0 => PackageSigning::Store,
                    1 => PackageSigning::Developer,
                    2 => PackageSigning::System,
                    3 => PackageSigning::Enterprise,
                    4 => PackageSigning::None,
                    _ => return Err(bad("an unknown kind of package signing")),
                },
            },
            2 => FoundWhere::MacBundle {
                bundle_id: r.text(MAX_APP_TEXT)?,
                folder: r.text(MAX_APP_TEXT)?,
                app_store_id: match r.u8()? {
                    0 => None,
                    1 => Some(u64::from_be_bytes(r.array::<8>()?)),
                    _ => return Err(bad("an unknown kind of store ID")),
                },
            },
            3 => FoundWhere::Deb {
                package: r.text(MAX_APP_TEXT)?,
            },
            4 => FoundWhere::Rpm {
                package: r.text(MAX_APP_TEXT)?,
            },
            5 => FoundWhere::Flatpak {
                app_id: r.text(MAX_APP_TEXT)?,
                remote: r.text(MAX_APP_TEXT)?,
                remote_url: r.opt(MAX_APP_URL)?,
                system_wide: r.flag()?,
            },
            6 => FoundWhere::Snap {
                name: r.text(MAX_APP_TEXT)?,
            },
            _ => return Err(bad("an unknown kind of place an app was found")),
        };
        let arch = AppArch::of(r.u8()?).ok_or_else(|| bad("an unknown processor set"))?;
        let maker = match r.u8()? {
            0 => {
                let mut s = SignerId {
                    common_name: r.opt(MAX_APP_TEXT)?,
                    organization: r.opt(MAX_APP_TEXT)?,
                    locality: r.opt(MAX_APP_TEXT)?,
                    state: r.opt(MAX_APP_TEXT)?,
                    country: r.opt(MAX_APP_TEXT)?,
                    registration_number: r.opt(MAX_APP_TEXT)?,
                    jurisdiction_country: r.opt(MAX_APP_TEXT)?,
                    jurisdiction_state: r.opt(MAX_APP_TEXT)?,
                    ..SignerId::default()
                };
                s.certificate = r.array::<32>()?;
                s.issuer = r.text(MAX_APP_TEXT)?;
                s.issuer_organization = r.opt(MAX_APP_TEXT)?;
                s.public_key = match r.u8()? {
                    0 => None,
                    1 => Some(r.array::<32>()?),
                    _ => return Err(bad("an unknown kind of key")),
                };
                AppMaker::Windows(Box::new(s))
            }
            1 => AppMaker::WindowsPackage {
                publisher: r.text(MAX_APP_TEXT)?,
            },
            2 => AppMaker::MacTeam {
                team_id: r.text(MAX_APP_TEXT)?,
            },
            3 => AppMaker::Rpm {
                key: r.text(MAX_APP_TEXT)?,
            },
            4 => AppMaker::Repository {
                origin: r.text(MAX_APP_TEXT)?,
            },
            5 => AppMaker::Flatpak {
                remote_url: r.text(MAX_APP_URL)?,
                app_id: r.text(MAX_APP_TEXT)?,
                key: r.array::<32>()?,
            },
            6 => AppMaker::Unsigned,
            7 => AppMaker::NotRead {
                why: r.text(MAX_APP_TEXT)?,
            },
            _ => return Err(bad("an unknown kind of maker")),
        };
        let web = r.opt(MAX_APP_URL)?;
        let update_feed = r.opt(MAX_APP_URL)?;
        let k = r.u8()? as usize;
        if k > MAX_INSTALLERS {
            return Err(bad("an app with too many installers"));
        }
        let mut installers = Vec::with_capacity(k);
        for _ in 0..k {
            installers.push(ItemId(r.array::<16>()?));
        }
        r.held += 32 * k as u64;
        let k = r.u8()? as usize;
        if k > MAX_NOTES {
            return Err(bad("an app with too many notes"));
        }
        let mut notes = Vec::with_capacity(k);
        for _ in 0..k {
            if r.u8()? != 0 {
                return Err(bad("an unknown kind of note"));
            }
            notes.push(AppNote::Registration {
                owner: r.opt(MAX_APP_TEXT)?,
                company: r.opt(MAX_APP_TEXT)?,
                product_id: r.opt(MAX_APP_TEXT)?,
            });
            r.held += 96;
        }
        found.push(FoundApp {
            name,
            version,
            publisher,
            owner,
            system,
            found: found_where,
            arch,
            maker,
            web,
            update_feed,
            installers,
            notes,
        });
    }
    let n = r.u32()? as usize;
    if n > MAX_NOT_READ {
        return Err(bad("too many places listed as not read"));
    }
    let mut not_read = Vec::with_capacity(n);
    for _ in 0..n {
        r.held += 128;
        let owner = r.owner()?;
        let place = *PLACES
            .get(r.u8()? as usize)
            .ok_or_else(|| bad("an unknown kind of app place"))?;
        not_read.push(AppsNotRead {
            owner,
            place,
            why: r.text(MAX_APP_TEXT)?,
        });
    }
    let held = r.held;
    let list = AppList { found, not_read };
    list.validate()?;
    // Written by PCTwin only in order, so a re-encoding is the same bytes; anything else (a
    // value no writer makes, such as a flag of 2) was refused above.
    Ok((list, held))
}

struct In<'a, 'b> {
    rest: &'b mut &'a [u8],
    held: u64,
}

impl<'a> In<'a, '_> {
    fn take(&mut self, n: usize) -> Result<&'a [u8], RecordError> {
        if self.rest.len() < n {
            return Err(bad("it ends too soon"));
        }
        let (a, b) = self.rest.split_at(n);
        *self.rest = b;
        Ok(a)
    }
    fn u8(&mut self) -> Result<u8, RecordError> {
        Ok(self.take(1)?[0])
    }
    fn flag(&mut self) -> Result<bool, RecordError> {
        match self.u8()? {
            0 => Ok(false),
            1 => Ok(true),
            _ => Err(bad("a yes-or-no that is neither")),
        }
    }
    fn u32(&mut self) -> Result<u32, RecordError> {
        Ok(u32::from_be_bytes(self.array::<4>()?))
    }
    fn array<const N: usize>(&mut self) -> Result<[u8; N], RecordError> {
        let mut a = [0u8; N];
        a.copy_from_slice(self.take(N)?);
        Ok(a)
    }
    fn text(&mut self, max: usize) -> Result<String, RecordError> {
        let n = self.u32()? as usize;
        if n > max {
            return Err(bad("an app's text is too long"));
        }
        let s = std::str::from_utf8(self.take(n)?)
            .map_err(|_| bad("an app's text is not valid UTF-8"))?;
        self.held += TEXT_COST + n as u64 + n as u64 / 4;
        Ok(s.to_string())
    }
    fn opt(&mut self, max: usize) -> Result<Option<String>, RecordError> {
        match self.u8()? {
            0 => Ok(None),
            1 => self.text(max).map(Some),
            _ => Err(bad("an unknown kind of optional text")),
        }
    }
    fn owner(&mut self) -> Result<Owner, RecordError> {
        Ok(match self.u8()? {
            0 => Owner::Person {
                account_id: self.text(MAX_APP_TEXT)?,
            },
            1 => Owner::Shared,
            2 => Owner::System,
            _ => return Err(bad("an unknown kind of owner")),
        })
    }
}
