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

/// The address a download mark records, held only to compare its site with the maker's own
/// sites (Security Design, the scan rules, decided 30 October 2026): never opened, fetched,
/// linked, logged, reported, sent to the other laptop or given to the AI. It cannot be printed
/// or serialised; [`mark_on_makers_site`] turns it into the one yes or no that is kept.
pub struct MarkAddress(String);

impl std::fmt::Debug for MarkAddress {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("MarkAddress(hidden)")
    }
}

/// Most bytes of a download mark read.
pub const MAX_MARK_BYTES: usize = 4096;

/// The address a Windows mark of the web (`Zone.Identifier`) records: `HostUrl` (the download
/// itself), else `ReferrerUrl`. Nothing for a mark that names neither.
pub fn zone_identifier_address(text: &str) -> Option<MarkAddress> {
    if text.len() > MAX_MARK_BYTES {
        return None;
    }
    let value = |key: &str| {
        text.lines()
            .filter_map(|l| l.trim().split_once('='))
            .find(|(k, _)| k.trim().eq_ignore_ascii_case(key))
            .map(|(_, v)| v.trim().to_owned())
            .filter(|v| !v.is_empty())
    };
    value("HostUrl")
        .or_else(|| value("ReferrerUrl"))
        .map(MarkAddress)
}

/// The address a Mac `kMDItemWhereFroms` mark records (a property list of addresses, the
/// download first): the first one.
pub fn where_froms_address(bytes: &[u8]) -> Option<MarkAddress> {
    if bytes.len() > MAX_MARK_BYTES {
        return None;
    }
    let v = plist::Value::from_reader(std::io::Cursor::new(bytes)).ok()?;
    v.as_array()?
        .iter()
        .find_map(|x| x.as_string().filter(|s| !s.trim().is_empty()))
        .map(|s| MarkAddress(s.trim().to_owned()))
}

/// Whether the mark's address is on one of `app`'s maker's own sites, by the signed catalogue.
/// `None` (says nothing) when the catalogue names no sites for this maker or the address is not
/// a web address with a named host.
pub fn mark_on_makers_site(
    app: &FoundApp,
    mark: &MarkAddress,
    rows: &Catalogue<CounterpartRow>,
) -> Option<bool> {
    let u = url::Url::parse(&mark.0).ok()?;
    if !matches!(u.scheme(), "https" | "http") || !matches!(u.host(), Some(url::Host::Domain(_))) {
        return None;
    }
    let row = find(app, rows).filter(|r| !r.domains.is_empty())?;
    Some(row.on_makers_site(&mark.0))
}

/// Reads the Windows mark beside `file`, at most [`MAX_MARK_BYTES`]. Nothing when there is none
/// or it cannot be read.
pub fn read_windows_mark(file: &std::path::Path) -> Option<MarkAddress> {
    #[cfg(windows)]
    {
        use std::io::Read as _;
        let mut stream = file.as_os_str().to_owned();
        stream.push(":Zone.Identifier");
        let mut text = String::new();
        std::fs::File::open(stream)
            .ok()?
            .take(MAX_MARK_BYTES as u64 + 1)
            .read_to_string(&mut text)
            .ok()?;
        zone_identifier_address(&text)
    }
    #[cfg(not(windows))]
    {
        let _ = file;
        None
    }
}

#[cfg(test)]
mod mark_tests {
    use super::super::counterpart::tests::{TUNES, rows, windows_tunes};
    use super::*;

    fn zone(host: &str) -> String {
        format!(
            "[ZoneTransfer]\r\nZoneId=3\r\nReferrerUrl=https://search.example.org/\r\nHostUrl={host}\r\n"
        )
    }

    #[test]
    fn a_mark_is_only_a_yes_or_no_against_the_makers_sites() {
        let c = rows(TUNES);
        let app = windows_tunes();
        let on =
            zone_identifier_address(&zone("https://dl.example-music.com/Tunes.msi?t=1")).unwrap();
        assert_eq!(mark_on_makers_site(&app, &on, &c), Some(true));
        let off =
            zone_identifier_address(&zone("https://example-music.com.mirror.example.net/t.msi"))
                .unwrap();
        assert_eq!(mark_on_makers_site(&app, &off, &c), Some(false));
        // HostUrl decides; ReferrerUrl only stands in when there is no HostUrl.
        let referrer = zone_identifier_address(
            "[ZoneTransfer]\r\nZoneId=3\r\nReferrerUrl=https://www.example-music.com/download\r\n",
        )
        .unwrap();
        assert_eq!(mark_on_makers_site(&app, &referrer, &c), Some(true));
    }

    #[test]
    fn a_missing_unreadable_or_unknown_mark_says_nothing() {
        let c = rows(TUNES);
        assert!(zone_identifier_address("[ZoneTransfer]\r\nZoneId=3\r\n").is_none());
        // A mark too long to be one is not read, even with an address in it.
        let long = format!(
            "{}{}",
            zone("https://dl.example-music.com/t.msi"),
            " ".repeat(MAX_MARK_BYTES)
        );
        assert!(zone_identifier_address(&long).is_none());
        // A maker the catalogue names but gives no sites for: nothing said.
        let no_sites = rows(
            r#"{"id":"t","maker":"Example Music AB","editions":[
                {"system":"windows","name":"Tunes","match":{"windows-org":"Example Music AB","windows-country":"SE","entry-name":"Tunes"}}]}"#,
        );
        let on = zone_identifier_address(&zone("https://dl.example-music.com/t.msi")).unwrap();
        assert_eq!(mark_on_makers_site(&windows_tunes(), &on, &no_sites), None);
        let bad = zone_identifier_address(&zone("about:internet")).unwrap();
        assert_eq!(mark_on_makers_site(&windows_tunes(), &bad, &c), None);
        let ip = zone_identifier_address(&zone("https://203.0.113.9/t.msi")).unwrap();
        assert_eq!(mark_on_makers_site(&windows_tunes(), &ip, &c), None);
        // A maker the catalogue names no sites for.
        let mut other = windows_tunes();
        other.maker = pctwin_record::AppMaker::Unsigned;
        let on = zone_identifier_address(&zone("https://dl.example-music.com/t.msi")).unwrap();
        assert_eq!(mark_on_makers_site(&other, &on, &c), None);
        assert!(read_windows_mark(std::path::Path::new("no-such-file.msi")).is_none());
    }

    #[test]
    fn a_mac_mark_gives_its_download_address_first() {
        let mut bytes = Vec::new();
        plist::Value::Array(vec![
            plist::Value::String("https://dl.example-music.com/Tunes.dmg".into()),
            plist::Value::String("https://elsewhere.example.net/".into()),
        ])
        .to_writer_binary(&mut bytes)
        .unwrap();
        let a = where_froms_address(&bytes).unwrap();
        assert_eq!(
            mark_on_makers_site(&windows_tunes(), &a, &rows(TUNES)),
            Some(true)
        );
        assert!(where_froms_address(b"not a plist").is_none());
    }

    #[test]
    fn the_address_never_prints() {
        let a =
            zone_identifier_address(&zone("https://dl.example-music.com/secret-token")).unwrap();
        assert!(!format!("{a:?}").contains("secret-token"));
    }
}
