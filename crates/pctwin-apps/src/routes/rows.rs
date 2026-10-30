//! The `app-routes` catalogue: hints that add a route for an app found on the old laptop (its
//! maker's own download link, update feed or release page, a winget ID or a cask), never a
//! list of supported apps (ADOPTION changes 7 and 8). A row adds a place to look; it can never
//! change who the app must be signed by. Every route a row gives still ends at the maker check
//! against the old laptop's own copy, so a wrong or stale row fails safe.
//!
//! A row matches an app only when every identity it names is the app's own, exactly, and it
//! names at least one: a product code, a bundle ID, a Flatpak ID, or the maker itself (a Windows
//! signer's organisation and country, or a Mac Team ID). A name alone never matches.

use std::sync::OnceLock;

use pctwin_catalogue::{Catalogue, Row};
use pctwin_record::{AppMaker, AppSystem, FoundApp, FoundWhere};
use serde::Deserialize;

/// Who a row is for: every field given must be the app's own.
#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct RowMatch {
    pub product_code: Option<String>,
    pub bundle_id: Option<String>,
    pub flatpak_id: Option<String>,
    /// A Windows signer's organisation, with its country.
    pub windows_org: Option<String>,
    pub windows_country: Option<String>,
    pub team_id: Option<String>,
    /// The exact name of the app's entry in Windows' list of programs, with its maker: the
    /// product among that maker's apps. Never on its own (a name alone never matches).
    pub entry_name: Option<String>,
}

/// Where a row says the app can be had.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(
    rename_all = "kebab-case",
    rename_all_fields = "kebab-case",
    tag = "kind",
    deny_unknown_fields
)]
pub enum RowRoute {
    /// A winget package ID.
    Winget { id: String },
    /// A Homebrew cask token.
    Cask { token: String },
    /// The maker's fixed, versioned download, with its exact bytes.
    FixedLink {
        url: String,
        version: String,
        sha256: String,
    },
    /// The maker's "latest" link: its bytes and version are found out and checked before use,
    /// never trusted from the link.
    LatestLink { url: String },
    /// The maker's update feed.
    Feed { format: FeedFormat, url: String },
    /// The maker's verified GitHub releases: the one asset whose name ends with `asset-suffix`.
    GithubRelease {
        owner: String,
        repo: String,
        asset_suffix: String,
    },
    /// The maker's GitLab releases (gitlab.com project path).
    GitlabRelease {
        project: String,
        asset_suffix: String,
    },
    /// A download behind the maker's sign-in: the person signs in and downloads in their own
    /// browser.
    SignInPage { url: String },
}

/// The update feed formats PCTwin reads.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum FeedFormat {
    /// A Sparkle appcast (Mac).
    Sparkle,
    /// electron-builder's `latest.yml` / `latest-mac.yml`.
    Electron,
}

/// One `app-routes` row.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct AppRouteRow {
    pub id: String,
    pub system: AppSystem,
    #[serde(rename = "match")]
    pub matches: RowMatch,
    pub route: RowRoute,
    /// The app needs a licence key to work after it is installed.
    #[serde(default)]
    pub licence_key: bool,
    /// The app needs the person to sign in after it is installed.
    #[serde(default)]
    pub sign_in: bool,
}

/// Whether `url` is an ordinary secure web address a row may give: https, a named host (never
/// a bare address), no user or password, no fragment.
pub fn row_url_ok(url: &str) -> bool {
    let Ok(u) = url::Url::parse(url) else {
        return false;
    };
    u.scheme() == "https"
        && matches!(u.host(), Some(url::Host::Domain(_)))
        && u.username().is_empty()
        && u.password().is_none()
        && u.fragment().is_none()
        && url.len() <= pctwin_record::apps::MAX_APP_URL
}

fn name_ok(s: &str, max: usize) -> bool {
    !s.is_empty()
        && s.len() <= max
        && s.bytes().all(|b| {
            b.is_ascii_alphanumeric() || matches!(b, b'.' | b'-' | b'_' | b'+' | b'/' | b'{' | b'}')
        })
        && !s.contains("..")
}

impl Row for AppRouteRow {
    const CATALOGUE: &'static str = "app-routes";
    const FIELDS: &'static [&'static str] =
        &["id", "system", "match", "route", "licence-key", "sign-in"];

    fn check(&self) -> Result<(), String> {
        if !name_ok(&self.id, 64) || self.id.contains(['/', '{', '}']) {
            return Err("a row ID is short letters, digits and `.-_`".into());
        }
        check_match(&self.matches)?;
        match &self.route {
            RowRoute::Winget { id } => {
                if !super::winget::valid_id(id) {
                    return Err("not a winget ID".into());
                }
            }
            RowRoute::Cask { token } => {
                if !name_ok(token, 128) || token.contains(['/', '{', '}']) {
                    return Err("not a cask token".into());
                }
            }
            RowRoute::FixedLink {
                url,
                version,
                sha256,
            } => {
                if !row_url_ok(url) || version.trim().is_empty() || sha256.len() != 64 {
                    return Err(
                        "a fixed link names a secure address, a version and its SHA-256".into(),
                    );
                }
                if !sha256.bytes().all(|b| b.is_ascii_hexdigit()) {
                    return Err("a SHA-256 is 64 hexadecimal digits".into());
                }
            }
            RowRoute::LatestLink { url }
            | RowRoute::Feed { url, .. }
            | RowRoute::SignInPage { url } => {
                if !row_url_ok(url) {
                    return Err("not a secure web address".into());
                }
            }
            RowRoute::GithubRelease {
                owner,
                repo,
                asset_suffix,
            } => {
                if !name_ok(owner, 64)
                    || !name_ok(repo, 100)
                    || owner.contains('/')
                    || repo.contains('/')
                    || !name_ok(asset_suffix, 100)
                {
                    return Err(
                        "a GitHub release names an owner, a repository and an asset ending".into(),
                    );
                }
            }
            RowRoute::GitlabRelease {
                project,
                asset_suffix,
            } => {
                if !name_ok(project, 200) || !project.contains('/') || !name_ok(asset_suffix, 100) {
                    return Err("a GitLab release names a project path and an asset ending".into());
                }
            }
        }
        Ok(())
    }
}

/// Checks a row's identity: it names the app's identity or its maker, never a name alone.
pub fn check_match(m: &RowMatch) -> Result<(), String> {
    let identity = m.product_code.is_some()
        || m.bundle_id.is_some()
        || m.flatpak_id.is_some()
        || m.windows_org.is_some()
        || m.team_id.is_some();
    if !identity {
        return Err("a row must name the app's identity or its maker, never a name alone".into());
    }
    if m.entry_name.is_some() && m.windows_org.is_none() && m.product_code.is_none() {
        return Err("an entry name is given only with the app's maker or product code".into());
    }
    if m.entry_name.as_ref().is_some_and(|n| n.trim().is_empty()) {
        return Err("an entry name is plain text".into());
    }
    if m.windows_org.is_some() != m.windows_country.is_some() {
        return Err("a Windows maker is named by organisation and country together".into());
    }
    for t in [&m.product_code, &m.bundle_id, &m.flatpak_id, &m.team_id]
        .into_iter()
        .flatten()
    {
        if !name_ok(t, 255) {
            return Err("an identity holds characters no identity has".into());
        }
    }
    for t in [&m.windows_org, &m.windows_country, &m.entry_name]
        .into_iter()
        .flatten()
    {
        if t.trim().is_empty() || !pctwin_record::apps::text_ok(t, 256) {
            return Err("a maker's name is plain text".into());
        }
    }
    Ok(())
}

fn same(a: &str, b: &str) -> bool {
    a.eq_ignore_ascii_case(b)
}

/// Whether `row` is for `app`: every identity it names is the app's own.
pub fn row_matches(app: &FoundApp, row: &AppRouteRow) -> bool {
    identity_matches(app, row.system, &row.matches)
}

/// Whether every identity `m` names is `app`'s own, for an app of `system`.
pub fn identity_matches(app: &FoundApp, system: AppSystem, m: &RowMatch) -> bool {
    if system != app.system {
        return false;
    }
    let product_code = match &app.found {
        FoundWhere::WindowsEntry { product_code, .. } => product_code.as_deref(),
        _ => None,
    };
    let bundle_id = match &app.found {
        FoundWhere::MacBundle { bundle_id, .. } => Some(bundle_id.as_str()),
        _ => None,
    };
    let flatpak_id = match &app.found {
        FoundWhere::Flatpak { app_id, .. } => Some(app_id.as_str()),
        _ => None,
    };
    let (org, country) = match &app.maker {
        AppMaker::Windows(s) => (s.organization.as_deref(), s.country.as_deref()),
        _ => (None, None),
    };
    let team = match &app.maker {
        AppMaker::MacTeam { team_id } => Some(team_id.as_str()),
        _ => None,
    };
    let entry_name = match &app.found {
        FoundWhere::WindowsEntry { .. } => Some(app.name.as_str()),
        _ => None,
    };
    let want = |row: &Option<String>, have: Option<&str>, exact: bool| match row {
        None => true,
        Some(r) => have.is_some_and(|h| if exact { r == h } else { same(r, h) }),
    };
    want(&m.product_code, product_code, false)
        && want(&m.bundle_id, bundle_id, true)
        && want(&m.flatpak_id, flatpak_id, true)
        && want(&m.windows_org, org, true)
        && want(&m.windows_country, country, true)
        && want(&m.team_id, team, true)
        && want(&m.entry_name, entry_name, true)
}

/// The rows compiled into this build.
pub fn shipped() -> &'static Catalogue<AppRouteRow> {
    static ROWS: OnceLock<Catalogue<AppRouteRow>> = OnceLock::new();
    ROWS.get_or_init(|| Catalogue::shipped(include_str!("../../catalogues/app-routes.json")))
}

#[cfg(test)]
mod tests {
    use super::*;
    use pctwin_record::{AppArch, Owner, RegistryView, SignerId};

    pub(crate) fn writer() -> FoundApp {
        FoundApp {
            name: "Writer".into(),
            version: Some("2.0".into()),
            publisher: Some("Example Ltd".into()),
            owner: Owner::Shared,
            system: AppSystem::Windows,
            found: FoundWhere::WindowsEntry {
                view: RegistryView::Machine,
                key: "Writer".into(),
                product_code: Some("{11111111-2222-3333-4444-555555555555}".into()),
            },
            arch: AppArch::UNKNOWN,
            maker: AppMaker::Windows(Box::new(SignerId {
                organization: Some("Example Ltd".into()),
                country: Some("GB".into()),
                ..Default::default()
            })),
            web: None,
            update_feed: None,
            installers: vec![],
            notes: vec![],
        }
    }

    fn row(json: &str) -> Catalogue<AppRouteRow> {
        Catalogue::shipped(&format!(
            r#"{{"format":1,"catalogue":"app-routes","version":1,"rows":[{json}]}}"#
        ))
    }

    #[test]
    fn a_row_matches_only_by_identity() {
        let by_code = row(
            r#"{"id":"w","system":"windows","match":{"product-code":"{11111111-2222-3333-4444-555555555555}"},"route":{"kind":"winget","id":"Example.Writer"}}"#,
        );
        assert!(by_code.report().is_clean(), "{:?}", by_code.report());
        assert!(row_matches(&writer(), &by_code.rows()[0]));
        let by_maker = row(
            r#"{"id":"w","system":"windows","match":{"windows-org":"Example Ltd","windows-country":"GB"},"route":{"kind":"latest-link","url":"https://example.com/writer.msi"}}"#,
        );
        assert!(row_matches(&writer(), &by_maker.rows()[0]));
        let other_maker = row(
            r#"{"id":"w","system":"windows","match":{"windows-org":"Example Ltd","windows-country":"US"},"route":{"kind":"latest-link","url":"https://example.com/writer.msi"}}"#,
        );
        assert!(!row_matches(&writer(), &other_maker.rows()[0]));
        let mac = row(
            r#"{"id":"w","system":"mac","match":{"product-code":"{11111111-2222-3333-4444-555555555555}"},"route":{"kind":"cask","token":"writer"}}"#,
        );
        assert!(!row_matches(&writer(), &mac.rows()[0]));
    }

    #[test]
    fn rows_that_could_mislead_are_skipped() {
        for bad in [
            // A name alone.
            r#"{"id":"w","system":"windows","match":{},"route":{"kind":"winget","id":"A.B"}}"#,
            // Not https, a bare address, a user in the address.
            r#"{"id":"w","system":"windows","match":{"team-id":"ABCDE12345"},"route":{"kind":"latest-link","url":"http://example.com/x"}}"#,
            r#"{"id":"w","system":"windows","match":{"team-id":"ABCDE12345"},"route":{"kind":"latest-link","url":"https://10.0.0.1/x"}}"#,
            r#"{"id":"w","system":"windows","match":{"team-id":"ABCDE12345"},"route":{"kind":"sign-in-page","url":"https://a:b@example.com/"}}"#,
            // A fixed link without its bytes.
            r#"{"id":"w","system":"windows","match":{"team-id":"ABCDE12345"},"route":{"kind":"fixed-link","url":"https://example.com/x","version":"1","sha256":"zz"}}"#,
            r#"{"id":"w","system":"windows","match":{"team-id":"ABCDE12345"},"route":{"kind":"fixed-link","url":"https://example.com/x","version":"1","sha256":"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"}}"#,
            // An entry name with no Windows maker or product code to go with it.
            r#"{"id":"w","system":"windows","match":{"team-id":"ABCDE12345","entry-name":"Writer"},"route":{"kind":"winget","id":"A.B"}}"#,
            // An organisation without its country.
            r#"{"id":"w","system":"windows","match":{"windows-org":"Example Ltd"},"route":{"kind":"winget","id":"A.B"}}"#,
            // A command where an ID goes; an unknown kind of route.
            r#"{"id":"w","system":"windows","match":{"team-id":"ABCDE12345"},"route":{"kind":"winget","id":"A.B; calc"}}"#,
            r#"{"id":"w","system":"windows","match":{"team-id":"ABCDE12345"},"route":{"kind":"shell","command":"calc"}}"#,
            r#"{"id":"w","system":"windows","match":{"team-id":"ABCDE12345"},"route":{"kind":"github-release","owner":"a/b","repo":"c","asset-suffix":".msi"}}"#,
        ] {
            let c = row(bad);
            assert!(c.rows().is_empty(), "{bad}");
            assert_eq!(c.report().skipped.len(), 1, "{bad}");
        }
    }

    #[test]
    fn the_shipped_rows_load_clean() {
        assert!(shipped().report().is_clean(), "{:?}", shipped().report());
    }
}
