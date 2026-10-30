//! Download-trail clues (Task List 2.1, "download-trail clues checked against catalogs and
//! publishers"; Product Spec, app reinstalling, layer 2).
//!
//! A web address the old laptop recorded for an app is a clue, never proof. It is checked
//! against the maker's own sites as the signed `app-counterparts` catalogue names them:
//!
//! - on one of them: the address may be used as the maker's page (and the download from it
//!   still has to pass the maker check);
//! - on none of them: the person is told, and the address is never used or shown as a link;
//! - the catalogue names no sites for this maker: the address stays the old laptop's record, as
//!   before, and is only ever a page the person opens (the maker check still decides).
//!
//! Which clues exist: the address the old laptop's own list of programs records for the app
//! (`FoundApp::web`). A download mark's recorded address (Windows' `Zone.Identifier`
//! `HostUrl`/`ReferrerUrl`, a Mac's `kMDItemWhereFroms`) is **not read**: the Security Design
//! (Part C, "What the scan reads") checks a mark by its presence only, never by reading it. If
//! that rule changes, a mark's address goes through [`check`] exactly like any other clue.

use pctwin_catalogue::Catalogue;
use pctwin_record::FoundApp;

use super::counterpart::{CounterpartRow, find};
use super::rows::row_url_ok;

/// What a recorded address turned out to be.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Trail {
    /// On the maker's own site (`site`, as the catalogue names it).
    OnMakersSite { site: String },
    /// On none of the maker's sites: said to the person, never used.
    NotMakersSite { host: String },
    /// The catalogue names no sites for this app's maker.
    SitesNotKnown,
    /// Not a secure web address with a named host: never used.
    NotUsable,
}

impl Trail {
    /// Whether the address may be used as a page for the person to open.
    pub fn usable(&self) -> bool {
        matches!(self, Trail::OnMakersSite { .. } | Trail::SitesNotKnown)
    }
}

/// Checks the address `url` recorded for `app` against its maker's sites.
pub fn check(app: &FoundApp, url: &str, rows: &Catalogue<CounterpartRow>) -> Trail {
    if !row_url_ok(url) {
        return Trail::NotUsable;
    }
    let Some(row) = find(app, rows).filter(|r| !r.domains.is_empty()) else {
        return Trail::SitesNotKnown;
    };
    match row.site_of(url) {
        Some(site) => Trail::OnMakersSite { site: site.into() },
        None => Trail::NotMakersSite {
            host: url::Url::parse(url)
                .ok()
                .and_then(|u| u.host_str().map(str::to_owned))
                .unwrap_or_default(),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::super::counterpart::tests::{TUNES, rows, windows_tunes};
    use super::*;

    #[test]
    fn an_address_on_the_makers_site_is_used_and_one_elsewhere_is_said_never_used() {
        let c = rows(TUNES);
        let app = windows_tunes();
        assert_eq!(
            check(&app, "https://www.example-music.com/download", &c),
            Trail::OnMakersSite {
                site: "example-music.com".into()
            }
        );
        let elsewhere = check(
            &app,
            "https://example-music.com.downloads.example.net/x",
            &c,
        );
        assert_eq!(
            elsewhere,
            Trail::NotMakersSite {
                host: "example-music.com.downloads.example.net".into()
            }
        );
        assert!(!elsewhere.usable());
    }

    #[test]
    fn an_unknown_maker_keeps_the_record_and_a_bad_address_is_never_used() {
        let c = rows(TUNES);
        let mut other = windows_tunes();
        other.maker = pctwin_record::AppMaker::Unsigned;
        assert_eq!(
            check(&other, "https://anything.example.org/", &c),
            Trail::SitesNotKnown
        );
        for bad in [
            "http://www.example-music.com/",
            "https://203.0.113.9/setup.exe",
            "https://user:pw@www.example-music.com/",
            "not an address",
        ] {
            assert_eq!(check(&windows_tunes(), bad, &c), Trail::NotUsable, "{bad}");
        }
    }
}
