//! Apps from another kind of laptop (Task List 2.1, "cross-platform app matching table and
//! alternatives list"; Product Spec, app reinstalling, layers 6 and 7).
//!
//! The `app-counterparts` catalogue is data, never a list of supported apps: each row is one
//! maker's app, with **its edition on each system** (how it is recognised there, and what its
//! maker signs it with there), the maker's own web sites, and the **choices** a person has where
//! the maker makes no edition (the maker's web version, named alternatives, and a compatibility
//! layer such as CrossOver or Wine, given as information only).
//!
//! How a row is used, in this order:
//!
//! 1. **The same maker's edition for the new laptop.** The app is planned as that edition (its
//!    stand-in, [`stand_in`]): the routes are the ordinary ones for an app of the new laptop's
//!    system, and every check is made against the maker *the row names for that system* (a Mac
//!    Team ID, a Windows signer with its certificate authority, a Flatpak remote's key). The row
//!    is signed with the catalogue; a download from anyone else fails the maker check, so a
//!    wrong row fails safe. The publisher is never swapped: rows name one maker, and an edition
//!    by another maker is not an edition, it is an alternative.
//! 2. **Otherwise an honest card**: needs-action, with the choices that apply on the new laptop
//!    ([`CounterpartRow::choices_for`]). PCTwin never installs an alternative or a compatibility
//!    layer itself.
//!
//! A row matches an app only by identity, exactly as `app-routes` rows do (never a name alone);
//! an app two rows match is matched by neither (which would win?).

use std::sync::OnceLock;

use pctwin_catalogue::{Catalogue, Row};
use pctwin_record::{
    AppArch, AppMaker, AppSystem, FoundApp, FoundWhere, RegistryView, SignerId, apps::text_ok,
};
use serde::Deserialize;

use super::rows::{RowMatch, check_match, identity_matches, row_url_ok};

/// Most editions, domains and choices one row holds.
const MAX_EDITIONS: usize = 3;
const MAX_DOMAINS: usize = 16;
const MAX_CHOICES: usize = 16;

/// The Windows signer the maker signs this edition with: what the download's own signature must
/// name. The certificate itself changes as it is renewed, so the row names the company, its
/// country and the certificate authority that vouches for it (the maker check's renewed-
/// certificate rule).
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct WindowsSignerPin {
    pub common_name: String,
    pub organization: String,
    pub country: String,
    #[serde(default)]
    pub locality: Option<String>,
    #[serde(default)]
    pub state: Option<String>,
    pub issuer_organization: String,
}

/// Where a Linux edition comes from: a Flatpak remote and the SHA-256 of its signing keyring
/// (as `pctwin-apps` reads it on a laptop: `<remote>.trustedkeys.gpg`).
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct FlatpakSource {
    pub remote: String,
    pub remote_url: String,
    pub key_sha256: String,
}

/// One system's edition of the app.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct Edition {
    pub system: AppSystem,
    /// The edition's name on that system.
    pub name: String,
    /// How it is recognised on that system: a product code or Windows maker, a bundle ID or
    /// Team ID, a Flatpak ID.
    #[serde(rename = "match")]
    pub matches: RowMatch,
    /// Windows: the publisher text its entry in the list of programs carries.
    #[serde(default)]
    pub publisher: Option<String>,
    /// Windows: what its download must be signed by, to be installed in another edition's place.
    #[serde(default)]
    pub signer: Option<WindowsSignerPin>,
    /// Mac: its App Store ID, when the maker sells it there.
    #[serde(default)]
    pub app_store_id: Option<u64>,
    /// Linux: its Flatpak remote, to be installed in another edition's place.
    #[serde(default)]
    pub flatpak: Option<FlatpakSource>,
}

/// What a choice is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ChoiceKind {
    /// The maker's own web version (its address is on one of the maker's own sites).
    WebVersion,
    /// Another maker's app that does the same job.
    Alternative,
    /// A way to run the app's edition for another system (CrossOver, Wine): information only;
    /// PCTwin has not checked the app runs that way.
    CompatibilityLayer,
}

/// One choice for a person whose new laptop has no edition of the app.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct Choice {
    pub kind: ChoiceKind,
    /// Its name (a product name, as its maker writes it).
    pub name: String,
    pub url: String,
    /// The systems it applies on; empty for every system.
    #[serde(default)]
    pub on: Vec<AppSystem>,
}

/// One maker's app, on every system the maker makes it for.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct CounterpartRow {
    pub id: String,
    /// The maker's name, as they write it.
    pub maker: String,
    /// The maker's own web sites (a host and everything under it).
    #[serde(default)]
    pub domains: Vec<String>,
    pub editions: Vec<Edition>,
    #[serde(default)]
    pub choices: Vec<Choice>,
}

fn id_ok(s: &str) -> bool {
    !s.is_empty()
        && s.len() <= 64
        && s.bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'.' | b'-' | b'_'))
}

/// A host name as the catalogue writes a maker's site: lower-case letters, digits and hyphens in
/// two or more dot-separated labels.
pub fn domain_ok(d: &str) -> bool {
    d.len() <= 253
        && d.split('.').count() >= 2
        && d.split('.').all(|l| {
            !l.is_empty()
                && l.len() <= 63
                && !l.starts_with('-')
                && !l.ends_with('-')
                && l.bytes()
                    .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
        })
}

/// Whether `host` is `domain` or a name under it.
pub fn host_under(host: &str, domain: &str) -> bool {
    let host = host.trim_end_matches('.').to_ascii_lowercase();
    host == domain
        || (host.len() > domain.len()
            && host.ends_with(domain)
            && host.as_bytes()[host.len() - domain.len() - 1] == b'.')
}

fn hex32(s: &str) -> Option<[u8; 32]> {
    if s.len() != 64 || !s.bytes().all(|b| b.is_ascii_hexdigit()) {
        return None;
    }
    let mut out = [0u8; 32];
    for (i, p) in s.as_bytes().chunks(2).enumerate() {
        out[i] = u8::from_str_radix(std::str::from_utf8(p).ok()?, 16).ok()?;
    }
    Some(out)
}

fn plain(s: &str, max: usize) -> bool {
    !s.trim().is_empty() && text_ok(s, max)
}

impl Edition {
    fn check(&self) -> Result<(), String> {
        check_match(&self.matches)?;
        if !plain(&self.name, 256) {
            return Err("an edition's name is plain text".into());
        }
        let m = &self.matches;
        // Each system's edition is recognised by that system's own kind of identity.
        let fits = match self.system {
            AppSystem::Windows => {
                (m.product_code.is_some() || m.windows_org.is_some())
                    && m.bundle_id.is_none()
                    && m.team_id.is_none()
                    && m.flatpak_id.is_none()
            }
            AppSystem::Mac => {
                m.team_id
                    .as_deref()
                    .is_some_and(pctwin_maker::facts::valid_team_id)
                    && m.product_code.is_none()
                    && m.windows_org.is_none()
                    && m.flatpak_id.is_none()
            }
            AppSystem::Linux => {
                m.flatpak_id.is_some()
                    && m.product_code.is_none()
                    && m.windows_org.is_none()
                    && m.bundle_id.is_none()
                    && m.team_id.is_none()
            }
        };
        if !fits {
            return Err("an edition is recognised by its own system's kind of identity".into());
        }
        if self.publisher.as_ref().is_some_and(|p| !plain(p, 256)) {
            return Err("a publisher is plain text".into());
        }
        if let Some(s) = &self.signer {
            if self.system != AppSystem::Windows {
                return Err("only a Windows edition names a Windows signer".into());
            }
            for t in [
                Some(&s.common_name),
                Some(&s.organization),
                Some(&s.country),
                s.locality.as_ref(),
                s.state.as_ref(),
                Some(&s.issuer_organization),
            ]
            .into_iter()
            .flatten()
            {
                if !plain(t, 256) {
                    return Err("a signer's names are plain text".into());
                }
            }
            // The edition is recognised and pinned by the same maker, never two.
            if m.windows_org
                .as_deref()
                .is_some_and(|o| o != s.organization)
                || m.windows_country.as_deref().is_some_and(|c| c != s.country)
            {
                return Err("an edition's signer is the maker it is recognised by".into());
            }
        }
        if self.app_store_id.is_some() && self.system != AppSystem::Mac {
            return Err("only a Mac edition has an App Store ID".into());
        }
        if let Some(f) = &self.flatpak {
            if self.system != AppSystem::Linux {
                return Err("only a Linux edition has a Flatpak remote".into());
            }
            if !id_ok(&f.remote) || !row_url_ok(&f.remote_url) || hex32(&f.key_sha256).is_none() {
                return Err(
                    "a Flatpak remote is a name, a secure address and its key's SHA-256".into(),
                );
            }
        }
        Ok(())
    }

    /// Whether PCTwin can install this edition in another edition's place: it names everything
    /// the maker check needs on its system.
    pub fn installable(&self) -> bool {
        match self.system {
            AppSystem::Windows => self.signer.is_some() && self.publisher.is_some(),
            AppSystem::Mac => self.matches.bundle_id.is_some() && self.matches.team_id.is_some(),
            AppSystem::Linux => self.flatpak.is_some() && self.matches.flatpak_id.is_some(),
        }
    }
}

impl Row for CounterpartRow {
    const CATALOGUE: &'static str = "app-counterparts";
    const FIELDS: &'static [&'static str] = &["id", "maker", "domains", "editions", "choices"];

    fn check(&self) -> Result<(), String> {
        if !id_ok(&self.id) {
            return Err("a row ID is short letters, digits and `.-_`".into());
        }
        if !plain(&self.maker, 256) {
            return Err("a maker's name is plain text".into());
        }
        if self.editions.is_empty() || self.editions.len() > MAX_EDITIONS {
            return Err("a row names one to three editions".into());
        }
        for (i, e) in self.editions.iter().enumerate() {
            e.check()?;
            if self.editions[..i].iter().any(|o| o.system == e.system) {
                return Err("a row names one edition per system".into());
            }
        }
        if self.domains.len() > MAX_DOMAINS || !self.domains.iter().all(|d| domain_ok(d)) {
            return Err("a maker's sites are plain host names".into());
        }
        if self.choices.len() > MAX_CHOICES {
            return Err("too many choices".into());
        }
        for c in &self.choices {
            if !plain(&c.name, 128) || !row_url_ok(&c.url) {
                return Err("a choice is a name and a secure web address".into());
            }
            // The maker's web version is on the maker's own sites, or it is not theirs.
            if c.kind == ChoiceKind::WebVersion && !self.on_makers_site(&c.url) {
                return Err("a web version is on the maker's own sites".into());
            }
        }
        Ok(())
    }

    fn identity(&self) -> Option<String> {
        Some(self.id.clone())
    }
}

impl CounterpartRow {
    /// The row's edition `app` is, if any.
    pub fn edition_of(&self, app: &FoundApp) -> Option<&Edition> {
        self.editions
            .iter()
            .find(|e| identity_matches(app, e.system, &e.matches))
    }

    /// The edition for `system`, if the maker makes one.
    pub fn edition_for(&self, system: AppSystem) -> Option<&Edition> {
        self.editions.iter().find(|e| e.system == system)
    }

    /// Whether `url`'s host is one of the maker's own sites.
    pub fn on_makers_site(&self, url: &str) -> bool {
        self.site_of(url).is_some()
    }

    /// The maker's site `url` is on, if it is on one.
    pub fn site_of(&self, url: &str) -> Option<&str> {
        let u = url::Url::parse(url).ok()?;
        let host = u.host_str()?;
        self.domains
            .iter()
            .find(|d| host_under(host, d))
            .map(String::as_str)
    }

    /// The choices that apply on a new laptop of `system`, the maker's web version first, then
    /// alternatives, then compatibility layers (information only).
    pub fn choices_for(&self, system: AppSystem) -> Vec<Choice> {
        let mut out: Vec<Choice> = self
            .choices
            .iter()
            .filter(|c| c.on.is_empty() || c.on.contains(&system))
            .cloned()
            .collect();
        out.sort_by_key(|c| match c.kind {
            ChoiceKind::WebVersion => 0,
            ChoiceKind::Alternative => 1,
            ChoiceKind::CompatibilityLayer => 2,
        });
        out
    }
}

/// The one row `app` is an edition of. An app no row names, or two rows name, has none.
pub fn find<'c>(app: &FoundApp, rows: &'c Catalogue<CounterpartRow>) -> Option<&'c CounterpartRow> {
    let reading = rows.read([app], |a, r| r.edition_of(a).is_some());
    let (_, first) = reading.known.into_iter().next()?;
    // Every row, not only the first the reading found: a second one makes it ambiguous.
    let named = rows
        .rows()
        .iter()
        .filter(|r| r.edition_of(app).is_some())
        .count();
    (named == 1).then_some(first)
}

/// The app PCTwin installs in `app`'s place on a laptop of `system`: the same maker's edition
/// for that system, as `row` names it, with the maker the row pins for it. `None` when the maker
/// makes none, or the row does not name everything the maker check needs.
pub fn stand_in(app: &FoundApp, row: &CounterpartRow, system: AppSystem) -> Option<FoundApp> {
    row.edition_of(app)?;
    let e = row.edition_for(system).filter(|e| e.installable())?;
    let (found, maker) = match system {
        AppSystem::Windows => {
            let s = e.signer.as_ref()?;
            (
                FoundWhere::WindowsEntry {
                    view: RegistryView::Machine,
                    key: e.name.clone(),
                    product_code: e.matches.product_code.clone(),
                },
                AppMaker::Windows(Box::new(SignerId {
                    common_name: Some(s.common_name.clone()),
                    organization: Some(s.organization.clone()),
                    locality: s.locality.clone(),
                    state: s.state.clone(),
                    country: Some(s.country.clone()),
                    issuer_organization: Some(s.issuer_organization.clone()),
                    ..Default::default()
                })),
            )
        }
        AppSystem::Mac => (
            FoundWhere::MacBundle {
                bundle_id: e.matches.bundle_id.clone()?,
                folder: format!("{}.app", e.name),
                app_store_id: e.app_store_id,
            },
            AppMaker::MacTeam {
                team_id: e.matches.team_id.clone()?,
            },
        ),
        AppSystem::Linux => {
            let f = e.flatpak.as_ref()?;
            let app_id = e.matches.flatpak_id.clone()?;
            (
                FoundWhere::Flatpak {
                    app_id: app_id.clone(),
                    remote: f.remote.clone(),
                    remote_url: Some(f.remote_url.clone()),
                    system_wide: true,
                },
                AppMaker::Flatpak {
                    remote_url: f.remote_url.clone(),
                    app_id,
                    key: hex32(&f.key_sha256)?,
                },
            )
        }
    };
    Some(FoundApp {
        name: e.name.clone(),
        // Versions are not comparable between systems' editions: no version is carried over.
        version: None,
        publisher: e.publisher.clone().or_else(|| Some(row.maker.clone())),
        owner: app.owner.clone(),
        system,
        found,
        arch: AppArch::UNKNOWN,
        maker,
        // The old laptop's recorded address belonged to the other edition: never a sign-in page
        // for this one.
        web: None,
        update_feed: None,
        installers: vec![],
        notes: app.notes.clone(),
    })
}

/// The rows compiled into this build.
pub fn shipped() -> &'static Catalogue<CounterpartRow> {
    static ROWS: OnceLock<Catalogue<CounterpartRow>> = OnceLock::new();
    ROWS.get_or_init(|| Catalogue::shipped(include_str!("../../catalogues/app-counterparts.json")))
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use pctwin_record::Owner;

    pub(crate) fn rows(json: &str) -> Catalogue<CounterpartRow> {
        Catalogue::shipped(&format!(
            r#"{{"format":1,"catalogue":"app-counterparts","version":1,"rows":[{json}]}}"#
        ))
    }

    pub(crate) const TUNES: &str = r#"{"id":"tunes","maker":"Example Music AB",
        "domains":["example-music.com"],
        "editions":[
          {"system":"windows","name":"Tunes","publisher":"Example Music AB",
           "match":{"windows-org":"Example Music AB","windows-country":"SE"},
           "signer":{"common-name":"Example Music AB","organization":"Example Music AB","country":"SE","issuer-organization":"Example CA Inc."}},
          {"system":"mac","name":"Tunes","match":{"bundle-id":"com.example.tunes","team-id":"ABCDE12345"}},
          {"system":"linux","name":"Tunes","match":{"flatpak-id":"com.example.Tunes"},
           "flatpak":{"remote":"flathub","remote-url":"https://dl.flathub.org/repo/","key-sha256":"0101010101010101010101010101010101010101010101010101010101010101"}}
        ],
        "choices":[
          {"kind":"compatibility-layer","name":"Wine","url":"https://www.winehq.org/","on":["linux","mac"]},
          {"kind":"alternative","name":"Other Player","url":"https://other.example.org/"},
          {"kind":"web-version","name":"Tunes on the web","url":"https://open.example-music.com/"}
        ]}"#;

    pub(crate) fn windows_tunes() -> FoundApp {
        FoundApp {
            name: "Tunes".into(),
            version: Some("1.2.3".into()),
            publisher: Some("Example Music AB".into()),
            owner: Owner::Shared,
            system: AppSystem::Windows,
            found: FoundWhere::WindowsEntry {
                view: RegistryView::Person,
                key: "Tunes".into(),
                product_code: None,
            },
            arch: AppArch::UNKNOWN,
            maker: AppMaker::Windows(Box::new(SignerId {
                organization: Some("Example Music AB".into()),
                country: Some("SE".into()),
                ..Default::default()
            })),
            web: Some("https://www.example-music.com/download".into()),
            update_feed: None,
            installers: vec![],
            notes: vec![],
        }
    }

    #[test]
    fn a_row_loads_and_names_each_systems_edition() {
        let c = rows(TUNES);
        assert!(c.report().is_clean(), "{:?}", c.report());
        let row = find(&windows_tunes(), &c).unwrap();
        assert_eq!(
            row.edition_of(&windows_tunes()).unwrap().system,
            AppSystem::Windows
        );
        assert!(row.edition_for(AppSystem::Mac).unwrap().installable());
    }

    #[test]
    fn the_stand_in_carries_the_makers_own_identity_for_the_new_system() {
        let c = rows(TUNES);
        let row = find(&windows_tunes(), &c).unwrap();
        let mac = stand_in(&windows_tunes(), row, AppSystem::Mac).unwrap();
        assert_eq!(mac.system, AppSystem::Mac);
        assert_eq!(
            mac.maker,
            AppMaker::MacTeam {
                team_id: "ABCDE12345".into()
            }
        );
        assert!(
            matches!(&mac.found, FoundWhere::MacBundle { bundle_id, folder, .. }
            if bundle_id == "com.example.tunes" && folder == "Tunes.app")
        );
        // No version or page carried across systems.
        assert_eq!(mac.version, None);
        assert_eq!(mac.web, None);
        let linux = stand_in(&windows_tunes(), row, AppSystem::Linux).unwrap();
        assert!(matches!(&linux.maker, AppMaker::Flatpak { key, .. } if *key == [1; 32]));
        // The Mac edition back to Windows: pinned to the signer and its authority.
        let back = stand_in(&mac, row, AppSystem::Windows).unwrap();
        let AppMaker::Windows(s) = &back.maker else {
            panic!("not a Windows maker")
        };
        assert_eq!(s.organization.as_deref(), Some("Example Music AB"));
        assert_eq!(s.common_name.as_deref(), Some("Example Music AB"));
        assert_eq!(s.issuer_organization.as_deref(), Some("Example CA Inc."));
    }

    #[test]
    fn an_edition_without_everything_the_maker_check_needs_is_not_installed() {
        // The Mac edition names no bundle ID; the Windows edition no signer.
        let c = rows(
            r#"{"id":"t","maker":"M","editions":[
                {"system":"windows","name":"T","publisher":"M","match":{"windows-org":"M","windows-country":"SE"}},
                {"system":"mac","name":"T","match":{"team-id":"ABCDE12345"}}]}"#,
        );
        assert!(c.report().is_clean(), "{:?}", c.report());
        let row = &c.rows()[0];
        assert!(!row.edition_for(AppSystem::Mac).unwrap().installable());
        assert!(!row.edition_for(AppSystem::Windows).unwrap().installable());
        assert!(stand_in(&windows_tunes_of("M"), row, AppSystem::Mac).is_none());
        let mut mac = windows_tunes();
        mac.system = AppSystem::Mac;
        mac.found = FoundWhere::MacBundle {
            bundle_id: "x.y".into(),
            folder: "T.app".into(),
            app_store_id: None,
        };
        mac.maker = AppMaker::MacTeam {
            team_id: "ABCDE12345".into(),
        };
        assert!(row.edition_of(&mac).is_some());
        assert!(stand_in(&mac, row, AppSystem::Windows).is_none());
    }

    fn windows_tunes_of(org: &str) -> FoundApp {
        let mut a = windows_tunes();
        a.maker = AppMaker::Windows(Box::new(SignerId {
            organization: Some(org.into()),
            country: Some("SE".into()),
            ..Default::default()
        }));
        a
    }

    #[test]
    fn another_maker_is_never_matched() {
        let c = rows(TUNES);
        assert!(find(&windows_tunes_of("Example Music Ltd"), &c).is_none());
        // The same name from another country is another maker.
        let mut a = windows_tunes();
        a.maker = AppMaker::Windows(Box::new(SignerId {
            organization: Some("Example Music AB".into()),
            country: Some("US".into()),
            ..Default::default()
        }));
        assert!(find(&a, &c).is_none());
        // An app two rows name is named by neither.
        let two = rows(&format!(
            "{TUNES},{}",
            TUNES.replace("\"tunes\"", "\"tunes2\"")
        ));
        assert_eq!(two.rows().len(), 2);
        assert!(find(&windows_tunes(), &two).is_none());
    }

    #[test]
    fn choices_apply_by_system_with_the_makers_web_version_first() {
        let c = rows(TUNES);
        let kinds = |s| {
            c.rows()[0]
                .choices_for(s)
                .iter()
                .map(|c| c.kind)
                .collect::<Vec<_>>()
        };
        assert_eq!(
            kinds(AppSystem::Mac),
            vec![
                ChoiceKind::WebVersion,
                ChoiceKind::Alternative,
                ChoiceKind::CompatibilityLayer
            ]
        );
        assert_eq!(
            kinds(AppSystem::Windows),
            vec![ChoiceKind::WebVersion, ChoiceKind::Alternative]
        );
    }

    #[test]
    fn sites_match_by_whole_labels() {
        assert!(host_under("example-music.com", "example-music.com"));
        assert!(host_under(
            "download.example-music.com",
            "example-music.com"
        ));
        assert!(host_under(
            "Download.Example-Music.com.",
            "example-music.com"
        ));
        assert!(!host_under("evilexample-music.com", "example-music.com"));
        assert!(!host_under(
            "example-music.com.evil.net",
            "example-music.com"
        ));
        assert!(!host_under("music.com", "example-music.com"));
    }

    #[test]
    fn rows_that_could_mislead_are_skipped() {
        for bad in [
            // A web version off the maker's sites.
            TUNES.replace(
                "https://open.example-music.com/",
                "https://open.example.net/",
            ),
            // Two editions for one system.
            TUNES.replace("\"system\":\"linux\"", "\"system\":\"mac\""),
            // A Mac edition recognised by a Windows identity.
            TUNES.replace(
                r#"{"bundle-id":"com.example.tunes","team-id":"ABCDE12345"}"#,
                r#"{"windows-org":"X","windows-country":"SE"}"#,
            ),
            // Not a Team ID.
            TUNES.replace("ABCDE12345", "abc"),
            // A signer that is not the maker it is recognised by.
            TUNES.replace(
                r#""organization":"Example Music AB","country":"SE""#,
                r#""organization":"Someone Else","country":"SE""#,
            ),
            // A Flatpak key that is not a SHA-256.
            TUNES.replace(&"01".repeat(32), "zz"),
            // A choice that is not a secure address; a site that is not a host name.
            TUNES.replace("https://www.winehq.org/", "http://www.winehq.org/"),
            TUNES.replace(r#"["example-music.com"]"#, r#"["*.example-music.com"]"#),
            // A name alone.
            TUNES.replace(r#"{"flatpak-id":"com.example.Tunes"}"#, r#"{}"#),
        ] {
            let c = rows(&bad);
            assert!(c.rows().is_empty(), "{bad}");
        }
    }

    #[test]
    fn the_shipped_rows_load_clean() {
        assert!(shipped().report().is_clean(), "{:?}", shipped().report());
        assert!(!shipped().rows().is_empty());
    }
}
