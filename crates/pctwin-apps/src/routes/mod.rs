//! Step 2: where each app found on the old laptop can be had again, by who made it.
//!
//! [`plan`] gives every app one [`AppPlan`]: the routes to try, in a fixed order, and the card the
//! person sees if none works. The order (the 2.1 plan, AC01 to AC19, as adopted):
//!
//! 1. **The same maker in a package catalog**: winget (by product code, a catalogue row's ID, or
//!    the exact name), the Microsoft Store (by package family name, through the person's own
//!    Store), a Homebrew cask, the Mac App Store, apt, dnf, Flatpak (from the old laptop's own
//!    remote and key) or Snap.
//! 2. **The maker's own channel**: a catalogue row's fixed link, "latest" link, update feed or
//!    GitHub/GitLab release, and the app's own update feed read from the app itself.
//! 3. **The installer found on the old laptop**, signed by the same maker (copied if the person
//!    agrees).
//! 4. **A download behind the maker's sign-in**: the maker's own page opens in the person's
//!    browser; they sign in and download; PCTwin picks up the file and checks it. PCTwin never
//!    sees the password.
//!
//! Every route ends at the same checks: never older than the person's version
//! ([`crate::version`]), the same maker as the old laptop's copy, and the system's own malware
//! check. An app no route can verify (unsigned, its maker unreadable, from another system, or
//! with nowhere to look) ends needs-action with its [`Card`]: its name, its maker, the web
//! address the old laptop recorded, and what it needs after (a licence key, a sign-in).
//!
//! **An app from another kind of laptop** ([`counterpart`]): the same maker's edition for the new
//! laptop is planned in its place (its stand-in, checked against the maker the signed catalogue
//! names for that system), or else it ends needs-action with the choices that apply (the
//! maker's web version, alternatives, a compatibility layer as information). The address the old
//! laptop recorded is checked against the maker's own sites first ([`trail`]).

pub mod brew;
pub mod counterpart;
pub mod feeds;
pub mod rows;
pub mod trail;
pub mod winget;

use pctwin_catalogue::Catalogue;
use pctwin_record::{
    AppArch, AppMaker, AppNote, AppSystem, FoundApp, FoundWhere, ItemId, PackageSigning,
};

use crate::version::{Order, compare};
use counterpart::{Choice, CounterpartRow};
use rows::{AppRouteRow, FeedFormat, RowRoute, row_matches};
use trail::Trail;

/// The new laptop.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Target {
    pub system: AppSystem,
    /// Its processor: exactly one of `x86`, `x64`, `arm64` set.
    pub arch: AppArch,
    /// Linux only: which package system it uses.
    pub linux: Option<LinuxPackages>,
}

/// The Linux package systems.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LinuxPackages {
    Apt,
    Dnf,
}

/// The system families whose package system PCTwin knows, by their `os-release` identifier.
/// A derivative names its family in `ID_LIKE` (Linux Mint: `ubuntu`; Rocky and AlmaLinux:
/// `rhel centos fedora`; Nobara: `fedora`), so new derivatives need no row.
const FAMILIES: &[(&str, LinuxPackages)] = &[
    ("debian", LinuxPackages::Apt),
    ("ubuntu", LinuxPackages::Apt),
    ("fedora", LinuxPackages::Dnf),
    ("rhel", LinuxPackages::Dnf),
    ("centos", LinuxPackages::Dnf),
];

impl LinuxPackages {
    /// The package system an `/etc/os-release` names: its own `ID` first, then each `ID_LIKE`
    /// in order (closest first, as the format says). `None` for a family PCTwin does not install
    /// from (openSUSE's zypper, Arch's pacman).
    pub fn from_os_release(text: &str) -> Option<LinuxPackages> {
        let field = |name: &str| -> String {
            text.lines()
                .filter_map(|l| l.trim().split_once('='))
                .filter(|(k, _)| k.trim() == name)
                .map(|(_, v)| v.trim().trim_matches(['"', '\u{27}']).to_ascii_lowercase())
                .next_back()
                .unwrap_or_default()
        };
        let id = field("ID");
        let like = field("ID_LIKE");
        std::iter::once(id.as_str())
            .chain(like.split_whitespace())
            .find_map(|i| FAMILIES.iter().find(|(f, _)| *f == i).map(|(_, p)| *p))
    }
}

/// Whether the app is on the new laptop already, as its own system says.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Here {
    Absent,
    Present {
        version: Option<String>,
    },
    /// The new laptop could not tell (then the installer step looks again before anything).
    Unknown {
        why: String,
    },
}

/// How a winget ID was found.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WingetBy {
    /// A catalogue row names it for this app's identity.
    Row,
    /// winget matched the app's product code.
    ProductCode,
    /// winget matched the app's exact name (a suggestion only).
    Name,
}

/// The maker's own channel.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MakerLink {
    Fixed {
        url: String,
        version: String,
        sha256: [u8; 32],
    },
    Latest {
        url: String,
    },
    Feed {
        format: FeedFormat,
        url: String,
    },
    Github {
        owner: String,
        repo: String,
        asset_suffix: String,
    },
    Gitlab {
        project: String,
        asset_suffix: String,
    },
}

/// One place to get an app from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Route {
    Winget {
        id: String,
        by: WingetBy,
    },
    /// The Microsoft Store's own listing, opened for the person (their sign-in, their licence).
    StoreListing {
        family_name: String,
    },
    /// A plain Homebrew cask ([`brew`]), its build for the new Mac's processor: PCTwin downloads
    /// it itself (never through the `brew` program) and places its one `app`.
    Cask {
        token: String,
        version: String,
        url: String,
        sha256: Option<[u8; 32]>,
        auto_updates: bool,
        /// The `.app` it installs, as the cask names it.
        app: String,
    },
    MacAppStore {
        adam_id: u64,
    },
    Apt {
        package: String,
    },
    Dnf {
        package: String,
    },
    Flatpak {
        remote_url: String,
        app_id: String,
        key: [u8; 32],
    },
    Snap {
        name: String,
    },
    Maker(MakerLink),
    /// The installer found on the old laptop, signed by the same maker.
    OldInstaller {
        item: ItemId,
    },
    /// The maker's page, opened in the person's own browser for them to sign in and download.
    SignInPage {
        url: String,
    },
}

/// Whether `page` is exactly a store page [`Route::store_page`] makes, and nothing else: the one
/// check before the app opens it.
pub fn is_store_page(page: &str) -> bool {
    let route = if let Some(f) = page.strip_prefix("ms-windows-store://pdp/?PFN=") {
        Route::StoreListing {
            family_name: f.to_string(),
        }
    } else if let Some(id) = page.strip_prefix("macappstore://apps.apple.com/app/id") {
        match id.parse::<u64>() {
            Ok(adam_id) if id.bytes().all(|b| b.is_ascii_digit()) => Route::MacAppStore { adam_id },
            _ => return false,
        }
    } else {
        return false;
    };
    route.store_page().as_deref() == Some(page)
}

impl Route {
    /// The store's own page for this exact app, as an address the system opens in the store app
    /// (the Microsoft Store by the package family name, the App Store by its store ID), for the
    /// person to install it there with their own account. Made only from the found app's typed
    /// identity, never from text a catalogue or a screen gives; `None` for other routes, or a
    /// family name that is not one.
    pub fn store_page(&self) -> Option<String> {
        match self {
            Route::StoreListing { family_name } => {
                let ok = !family_name.is_empty()
                    && family_name.len() <= 128
                    && family_name
                        .bytes()
                        .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'.' | b'-' | b'_'));
                ok.then(|| format!("ms-windows-store://pdp/?PFN={family_name}"))
            }
            Route::MacAppStore { adam_id } if *adam_id > 0 => {
                Some(format!("macappstore://apps.apple.com/app/id{adam_id}"))
            }
            _ => None,
        }
    }
}

/// The install chains PCTwin runs itself, one per kind of laptop. A route belongs to at most one
/// ([`Route::chain`]); a route of none is the person's step (a store listing, the maker's page).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Chain {
    /// Windows: winget as the person, then the administrator helper (`pctwin-fullmove`'s
    /// `install`).
    Winget,
    /// Linux: the system's own package tools (`pkcon`, `flatpak`, `snap`; `pctwin-fullmove`'s
    /// `unix`).
    LinuxPackages,
    /// A Mac: a plain Homebrew cask, downloaded and placed by PCTwin itself
    /// (`pctwin-fullmove`'s `mac`).
    MacCask,
}

impl Route {
    /// The install chain that installs from this route, if PCTwin installs from it at all. This
    /// is the one place that is decided: the plan says an app needs the person when none of its
    /// routes has a chain, and each chain installs only from its own routes, so a route can never
    /// be installable in one place and not in the other. Every route is named here (no catch-all),
    /// so a new route is decided when it is added.
    pub fn chain(&self) -> Option<Chain> {
        match self {
            Route::Winget { .. } => Some(Chain::Winget),
            Route::Apt { .. } | Route::Dnf { .. } | Route::Flatpak { .. } | Route::Snap { .. } => {
                Some(Chain::LinuxPackages)
            }
            Route::Cask { .. } => Some(Chain::MacCask),
            // The person's own steps: the store's listing (their sign-in, their licence), the
            // App Store, the maker's page or feed, an installer from the old laptop (never reused
            // yet), the maker's sign-in page.
            Route::StoreListing { .. }
            | Route::MacAppStore { .. }
            | Route::Maker(_)
            | Route::OldInstaller { .. }
            | Route::SignInPage { .. } => None,
        }
    }

    /// Whether the version it brings is known before anything is downloaded (so a downgrade can
    /// be ruled out in the plan, not only after).
    pub fn version_known_ahead(&self) -> bool {
        matches!(
            self,
            Route::Cask { .. } | Route::Maker(MakerLink::Fixed { .. })
        )
    }
}

/// What the plan says about an app beside its routes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Flag {
    /// Built for Intel Macs only, going to an Apple-silicon Mac: it runs through Rosetta, which
    /// Apple is ending, so it may stop working.
    IntelOnly,
    /// A route's version is older than the person's: that route was left out.
    OlderLeftOut { route: String },
    /// The new laptop has this app already, in a newer or the same version.
    AlreadyHere { version: Option<String> },
    /// The new laptop has an older version: it is left as it is unless the person chooses.
    OlderHere { version: Option<String> },
    /// The new laptop could not say whether it has the app; the installer step looks again.
    HereUnknown { why: String },
    /// The app was on another kind of laptop (`from`): the same maker's edition for this one is
    /// planned in its place ([`AppPlan::stand_in`]).
    Counterpart { from: AppSystem },
}

/// What happens to the app.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Disposition {
    /// Try the routes in order.
    Install,
    /// On the new laptop already: nothing is installed.
    AlreadyHere,
    /// Part of the system itself: it comes with the new laptop.
    PartOfSystem,
    /// No route can verify it: needs-action, with its card.
    NeedsAction,
}

/// Why an app has no route.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NoRoute {
    /// It carries no maker signature, so a download cannot be proven to be the same app.
    Unsigned,
    /// Who made it could not be read on the old laptop.
    MakerNotRead,
    /// It was on another kind of laptop (a Windows app going to a Mac).
    OtherSystem,
    /// Nowhere to get it from was found.
    NoSource,
    /// A packaged app installed outside the Store (a developer's or a company's).
    PackageOutsideStore,
}

impl NoRoute {
    /// In plain words, with what to do next.
    pub fn plain(self) -> &'static str {
        match self {
            NoRoute::Unsigned => {
                "This app carries no maker signature, so PCTwin cannot prove a download is the real app. Your data is ready; install it yourself from the maker's website."
            }
            NoRoute::MakerNotRead => {
                "PCTwin could not read who made this app on your old laptop. Your data is ready; install it yourself from the maker's website."
            }
            NoRoute::OtherSystem => {
                "This app was on a different kind of laptop. If the maker has a version for this laptop, get it from their website; your data is ready."
            }
            NoRoute::NoSource => {
                "PCTwin found nowhere it can check to get this app. Your data is ready; install it yourself from the maker's website."
            }
            NoRoute::PackageOutsideStore => {
                "This app was installed from outside the Store. Get it again from where it came from; your data is ready."
            }
        }
    }
}

/// What the person sees for an app that ends needs-action.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Card {
    pub name: String,
    pub version: Option<String>,
    /// Who made it, in words (the signer's organisation, the Team ID, the repository).
    pub maker: Option<String>,
    /// The web address the old laptop recorded (never one found by a search).
    pub web: Option<String>,
    pub licence_key: bool,
    pub sign_in: bool,
    /// Who it is registered to and its product ID, as the old laptop's own list holds them.
    pub registration: Vec<AppNote>,
    pub why: Option<NoRoute>,
    /// For an app from another kind of laptop: the choices on this one (the maker's web
    /// version, alternatives, a compatibility layer as information). PCTwin installs none.
    pub choices: Vec<Choice>,
    /// What the address the old laptop recorded turned out to be, when it recorded one.
    pub trail: Option<Trail>,
}

/// The plan for one app.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppPlan {
    pub disposition: Disposition,
    pub routes: Vec<Route>,
    pub flags: Vec<Flag>,
    pub card: Card,
    /// For an app from another kind of laptop: the same maker's edition for this laptop, which
    /// the routes are for. Every check (the package's identity, the maker) is made against it,
    /// never against the old laptop's edition.
    pub stand_in: Option<FoundApp>,
}

impl AppPlan {
    /// The app the routes install: the stand-in, or else the old laptop's app itself.
    pub fn subject<'a>(&'a self, app: &'a FoundApp) -> &'a FoundApp {
        self.stand_in.as_ref().unwrap_or(app)
    }
}

/// Where hints come from.
pub struct Hints<'a> {
    pub rows: &'a Catalogue<AppRouteRow>,
    /// winget IDs suggested by searching winget on the new laptop, as the person.
    pub winget: &'a dyn Fn(&FoundApp) -> Vec<(String, WingetBy)>,
    pub casks: Option<&'a brew::CaskIndex>,
}

fn maker_words(m: &AppMaker) -> Option<String> {
    match m {
        AppMaker::Windows(s) => s.organization.clone().or_else(|| s.common_name.clone()),
        AppMaker::WindowsPackage { publisher } => Some(publisher.clone()),
        AppMaker::MacTeam { team_id } => Some(format!("Apple developer team {team_id}")),
        AppMaker::Rpm { key } => Some(format!("package key {key}")),
        AppMaker::Repository { origin } => Some(origin.clone()),
        AppMaker::Flatpak { remote_url, .. } => Some(remote_url.clone()),
        AppMaker::Unsigned | AppMaker::NotRead { .. } => None,
    }
}

fn card(app: &FoundApp, rows: &[&AppRouteRow]) -> Card {
    Card {
        name: app.name.clone(),
        version: app.version.clone(),
        maker: maker_words(&app.maker).or_else(|| app.publisher.clone()),
        web: app.web.clone(),
        licence_key: rows.iter().any(|r| r.licence_key)
            || app.notes.iter().any(|n| {
                matches!(
                    n,
                    AppNote::Registration {
                        product_id: Some(_),
                        ..
                    }
                )
            }),
        sign_in: rows.iter().any(|r| r.sign_in),
        registration: app.notes.clone(),
        why: None,
        choices: Vec::new(),
        trail: None,
    }
}

fn hex32(s: &str) -> Option<[u8; 32]> {
    if s.len() != 64 {
        return None;
    }
    let mut out = [0u8; 32];
    for (i, p) in s.as_bytes().chunks(2).enumerate() {
        out[i] = u8::from_str_radix(std::str::from_utf8(p).ok()?, 16).ok()?;
    }
    Some(out)
}

fn maker_link(r: &RowRoute) -> Option<Route> {
    Some(Route::Maker(match r {
        RowRoute::FixedLink {
            url,
            version,
            sha256,
        } => MakerLink::Fixed {
            url: url.clone(),
            version: version.clone(),
            sha256: hex32(sha256)?,
        },
        RowRoute::LatestLink { url } => MakerLink::Latest { url: url.clone() },
        RowRoute::Feed { format, url } => MakerLink::Feed {
            format: *format,
            url: url.clone(),
        },
        RowRoute::GithubRelease {
            owner,
            repo,
            asset_suffix,
        } => MakerLink::Github {
            owner: owner.clone(),
            repo: repo.clone(),
            asset_suffix: asset_suffix.clone(),
        },
        RowRoute::GitlabRelease {
            project,
            asset_suffix,
        } => MakerLink::Gitlab {
            project: project.clone(),
            asset_suffix: asset_suffix.clone(),
        },
        RowRoute::Winget { .. } | RowRoute::Cask { .. } | RowRoute::SignInPage { .. } => {
            return None;
        }
    }))
}

fn needs_action(mut plan: AppPlan, why: NoRoute) -> AppPlan {
    plan.disposition = Disposition::NeedsAction;
    plan.routes.clear();
    plan.card.why = Some(why);
    plan
}

/// The plan for `app` going to `target`, whose own system says `here` about it (for an app from
/// another kind of laptop, `here` is about its stand-in: [`here_subject`]).
pub fn plan(app: &FoundApp, target: Target, here: &Here, hints: &Hints<'_>) -> AppPlan {
    plan_with(app, target, here, hints, counterpart::shipped())
}

/// The app whose presence on the new laptop decides `here` for `app`: the same maker's edition
/// for the new laptop when `app` is from another kind of laptop and has one, else `app`.
pub fn here_subject(
    app: &FoundApp,
    target: AppSystem,
    counterparts: &Catalogue<CounterpartRow>,
) -> Option<FoundApp> {
    if app.system == target {
        return None;
    }
    counterpart::stand_in(app, counterpart::find(app, counterparts)?, target)
}

/// [`plan`], with the counterparts catalogue given.
pub fn plan_with(
    app: &FoundApp,
    target: Target,
    here: &Here,
    hints: &Hints<'_>,
    counterparts: &Catalogue<CounterpartRow>,
) -> AppPlan {
    let reading = hints.rows.read([app], |a, r| row_matches(a, r));
    // Every row for this app, not only the first: each adds a place to look.
    let rows: Vec<&AppRouteRow> = hints
        .rows
        .rows()
        .iter()
        .filter(|r| !reading.known.is_empty() && row_matches(app, r))
        .collect();
    let mut plan = AppPlan {
        disposition: Disposition::Install,
        routes: Vec::new(),
        flags: Vec::new(),
        card: card(app, &rows),
        stand_in: None,
    };
    // The address the old laptop recorded: a clue, checked against the maker's own sites; one
    // on none of them is said and never used.
    if let Some(web) = &app.web {
        let t = trail::check(app, web, counterparts);
        if !t.usable() {
            plan.card.web = None;
        }
        plan.card.trail = Some(t);
    }
    if matches!(
        app.found,
        FoundWhere::WindowsPackage {
            signing: PackageSigning::System,
            ..
        }
    ) {
        plan.disposition = Disposition::PartOfSystem;
        return plan;
    }
    if app.system == AppSystem::Mac
        && target.system == AppSystem::Mac
        && app.arch.intel_only()
        && target.arch.arm64
    {
        plan.flags.push(Flag::IntelOnly);
    }
    match here {
        Here::Present { version } => {
            // Versions of different systems' editions are not compared.
            let older_here = match (version.as_deref(), app.version.as_deref()) {
                (Some(h), Some(had)) if app.system == target.system => {
                    compare(h, had) == Order::Older
                }
                _ => false,
            };
            if older_here {
                plan.flags.push(Flag::OlderHere {
                    version: version.clone(),
                });
                // The routes the person may choose to update it by ("Update it"): worked out as
                // for an app that is not here, never used unless they choose; the app stays as
                // it is by default.
                let fresh = plan_with(app, target, &Here::Absent, hints, counterparts);
                if fresh.disposition == Disposition::Install {
                    plan.routes = fresh.routes;
                }
            } else {
                plan.flags.push(Flag::AlreadyHere {
                    version: version.clone(),
                });
            }
            // Never installed again, never replaced without the person's choice.
            plan.disposition = Disposition::AlreadyHere;
            return plan;
        }
        Here::Unknown { why } => plan.flags.push(Flag::HereUnknown { why: why.clone() }),
        Here::Absent => {}
    }
    if app.system != target.system {
        return other_system(app, target, here, hints, counterparts, plan);
    }
    match &app.maker {
        AppMaker::Unsigned => return needs_action(plan, NoRoute::Unsigned),
        AppMaker::NotRead { .. } => return needs_action(plan, NoRoute::MakerNotRead),
        _ => {}
    }

    // 1. The same maker in a package catalog.
    match (&app.found, &app.maker) {
        (FoundWhere::WindowsEntry { .. }, AppMaker::Windows(_)) => {
            for r in &rows {
                if let RowRoute::Winget { id } = &r.route {
                    plan.routes.push(Route::Winget {
                        id: id.clone(),
                        by: WingetBy::Row,
                    });
                }
            }
            for (id, by) in (hints.winget)(app) {
                if !plan
                    .routes
                    .iter()
                    .any(|r| matches!(r, Route::Winget { id: i, .. } if *i == id))
                {
                    plan.routes.push(Route::Winget { id, by });
                }
            }
        }
        (
            FoundWhere::WindowsPackage {
                family_name,
                signing,
            },
            _,
        ) => {
            if *signing == PackageSigning::Store {
                plan.routes.push(Route::StoreListing {
                    family_name: family_name.clone(),
                });
            } else {
                return needs_action(plan, NoRoute::PackageOutsideStore);
            }
        }
        (
            FoundWhere::MacBundle {
                bundle_id,
                folder,
                app_store_id,
            },
            AppMaker::MacTeam { .. },
        ) => {
            if let Some(id) = app_store_id {
                plan.routes.push(Route::MacAppStore { adam_id: *id });
            }
            let mut casks: Vec<&brew::Cask> = Vec::new();
            if let Some(ix) = hints.casks {
                for r in &rows {
                    if let RowRoute::Cask { token } = &r.route
                        && let Some(c) = ix.by_token(token)
                    {
                        casks.push(c);
                    }
                }
                for c in ix.for_bundle(bundle_id, folder) {
                    if !casks.iter().any(|k| k.token == c.token) {
                        casks.push(c);
                    }
                }
            }
            // Only a plain cask: one app, nothing the cask would run.
            for c in casks.into_iter().filter(|c| c.plain) {
                let Some(app) = c.apps.first() else { continue };
                let build = c.build_for(target.arch.arm64);
                plan.routes.push(Route::Cask {
                    token: c.token.clone(),
                    version: build.version,
                    url: build.url,
                    sha256: build.sha256,
                    auto_updates: c.auto_updates,
                    app: app.clone(),
                });
            }
        }
        (FoundWhere::Deb { package }, AppMaker::Repository { .. }) => {
            if target.linux == Some(LinuxPackages::Apt) {
                plan.routes.push(Route::Apt {
                    package: package.clone(),
                });
            }
        }
        (FoundWhere::Rpm { package }, AppMaker::Rpm { .. }) => {
            if target.linux == Some(LinuxPackages::Dnf) {
                plan.routes.push(Route::Dnf {
                    package: package.clone(),
                });
            }
        }
        (
            FoundWhere::Flatpak { .. },
            AppMaker::Flatpak {
                remote_url,
                app_id,
                key,
            },
        ) => plan.routes.push(Route::Flatpak {
            remote_url: remote_url.clone(),
            app_id: app_id.clone(),
            key: *key,
        }),
        (FoundWhere::Snap { name }, AppMaker::Repository { .. }) => {
            plan.routes.push(Route::Snap { name: name.clone() })
        }
        _ => {}
    }

    // 2. The maker's own channel: catalogue rows, then the app's own update feed.
    for r in &rows {
        if let Some(route) = maker_link(&r.route) {
            plan.routes.push(route);
        }
    }
    if let Some(feed) = &app.update_feed
        && app.system == AppSystem::Mac
        && rows::row_url_ok(feed)
    {
        let route = Route::Maker(MakerLink::Feed {
            format: FeedFormat::Sparkle,
            url: feed.clone(),
        });
        if !plan.routes.contains(&route) {
            plan.routes.push(route);
        }
    }

    // 3. The installer found on the old laptop.
    for item in &app.installers {
        plan.routes.push(Route::OldInstaller { item: *item });
    }

    // 4. The maker's own page, for a download behind their sign-in.
    let page = rows
        .iter()
        .find_map(|r| match &r.route {
            RowRoute::SignInPage { url } => Some(url.clone()),
            _ => None,
        })
        .or_else(|| plan.card.web.clone().filter(|w| rows::row_url_ok(w)));
    // Only where the maker can be checked on the download (a signed program or a Mac app).
    if let (Some(url), AppMaker::Windows(_) | AppMaker::MacTeam { .. }) = (page, &app.maker) {
        plan.routes.push(Route::SignInPage { url });
    }

    // Never older than the person's version, where the route says its version ahead.
    if let Some(had) = app.version.as_deref() {
        plan.routes.retain(|r| {
            let v = match r {
                Route::Cask { version, .. } | Route::Maker(MakerLink::Fixed { version, .. }) => {
                    version.as_str()
                }
                _ => return true,
            };
            if crate::version::not_a_downgrade(Some(v), Some(had)) {
                true
            } else {
                plan.flags.push(Flag::OlderLeftOut {
                    route: v.to_string(),
                });
                false
            }
        });
    }
    if plan.routes.is_empty() {
        return needs_action(plan, NoRoute::NoSource);
    }
    plan
}

/// An app from another kind of laptop: the same maker's edition for this one in its place, or
/// else an honest card with the choices that apply. Never another maker's app.
fn other_system(
    app: &FoundApp,
    target: Target,
    here: &Here,
    hints: &Hints<'_>,
    counterparts: &Catalogue<CounterpartRow>,
    mut plan: AppPlan,
) -> AppPlan {
    let Some(row) = counterpart::find(app, counterparts) else {
        return needs_action(plan, NoRoute::OtherSystem);
    };
    plan.card.choices = row.choices_for(target.system);
    if let Some(stand_in) = counterpart::stand_in(app, row, target.system) {
        let mut theirs = plan_with(&stand_in, target, here, hints, counterparts);
        if theirs.disposition != Disposition::NeedsAction {
            theirs.flags.push(Flag::Counterpart { from: app.system });
            // The person's card stays theirs: the old laptop's name, registration and clue,
            // with the choices beside it.
            theirs.card = Card {
                licence_key: plan.card.licence_key || theirs.card.licence_key,
                sign_in: plan.card.sign_in || theirs.card.sign_in,
                ..plan.card
            };
            theirs.stand_in = Some(stand_in);
            return theirs;
        }
    }
    needs_action(plan, NoRoute::OtherSystem)
}

#[cfg(test)]
mod tests;
