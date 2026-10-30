//! Step 1: every app on the old laptop, for every person, with who made it (discovery first).
//!
//! Each system's own record of what is installed is read by a general rule, never matched
//! against a list of known apps:
//!
//! - **Windows** ([`windows`]): the lists behind "Installed apps" (the machine's 64-bit and
//!   32-bit lists and each person's own), shown exactly as Windows shows them (an entry with a
//!   name that is not a hidden component or an update), and each person's packaged (Store/MSIX)
//!   apps. The maker is read from the app's own program or installer package by PCTwin's own
//!   signature reader.
//! - **Mac** ([`mac`]): every `.app` in `/Applications` (and one folder down) and in each person's
//!   own `~/Applications`, with its bundle ID, version, update feed, architectures, App Store ID
//!   and Team ID (read by `codesign`, through the maker crate's reader).
//! - **Linux** ([`linux`]): every package that gives the desktop an app (dpkg or rpm), every
//!   Flatpak app and every Snap.
//!
//! Nothing found is dropped: an app whose maker could not be read says why
//! ([`pctwin_record::AppMaker::NotRead`]); a place that could not be read at all is listed in
//! [`pctwin_record::AppList::not_read`] with the reason.

pub mod linux;
pub mod mac;
pub mod windows;

use pctwin_maker::facts::{Arch, WindowsSigner};
use pctwin_record::{AppArch, AppList, AppPlace, AppsNotRead, FoundApp, Owner, SignerId};

/// The maker crate's signer, as the record keeps it.
pub fn signer_id(s: &WindowsSigner) -> SignerId {
    SignerId {
        common_name: s.subject.common_name.clone(),
        organization: s.subject.organization.clone(),
        locality: s.subject.locality.clone(),
        state: s.subject.state.clone(),
        country: s.subject.country.clone(),
        registration_number: s.subject.registration_number.clone(),
        jurisdiction_country: s.subject.jurisdiction_country.clone(),
        jurisdiction_state: s.subject.jurisdiction_state.clone(),
        certificate: s.certificate.0,
        issuer: s.issuer.clone(),
        issuer_organization: s.issuer_organization.clone(),
        public_key: s.public_key.map(|k| k.0),
    }
}

/// The record's signer, as the maker crate checks against it.
pub fn windows_signer(s: &SignerId) -> WindowsSigner {
    WindowsSigner {
        subject: pctwin_maker::facts::Subject {
            common_name: s.common_name.clone(),
            organization: s.organization.clone(),
            locality: s.locality.clone(),
            state: s.state.clone(),
            country: s.country.clone(),
            registration_number: s.registration_number.clone(),
            jurisdiction_country: s.jurisdiction_country.clone(),
            jurisdiction_state: s.jurisdiction_state.clone(),
        },
        certificate: pctwin_maker::facts::Sha256(s.certificate),
        issuer: s.issuer.clone(),
        issuer_organization: s.issuer_organization.clone(),
        public_key: s.public_key.map(pctwin_maker::facts::Sha256),
    }
}

/// The maker crate's architecture, as the record keeps it.
pub fn app_arch(a: Option<Arch>) -> AppArch {
    let mut out = AppArch::UNKNOWN;
    let Some(a) = a else { return out };
    out.known = true;
    match a {
        Arch::X86 => out.x86 = true,
        Arch::X64 => out.x64 = true,
        Arch::Arm64 => out.arm64 = true,
        Arch::Several(set) => {
            out.x86 = set.x86;
            out.x64 = set.x64;
            out.arm64 = set.arm64;
        }
    }
    out
}

/// Text from the old laptop made safe to keep: control characters and hidden direction marks
/// dropped, cut to `max` bytes on a character boundary. `None` when nothing is left.
pub fn clean(s: &str, max: usize) -> Option<String> {
    let mut out = String::new();
    for c in s.trim().chars() {
        if pctwin_record::apps::text_ok(c.encode_utf8(&mut [0; 4]), 4) {
            if out.len() + c.len_utf8() > max {
                break;
            }
            out.push(c);
        }
    }
    let out = out.trim().to_string();
    (!out.is_empty()).then_some(out)
}

/// Builds the list: every app found, at most [`pctwin_record::apps::MAX_APPS`] (the rest are
/// said, never dropped silently), and every place not read.
#[derive(Debug, Default)]
pub struct Collector {
    list: AppList,
    over: usize,
}

impl Collector {
    pub fn add(&mut self, app: FoundApp) {
        if app.validate().is_err() {
            // Every text was cleaned on the way in; one that still breaks the rules is said.
            self.not_read(
                app.owner.clone(),
                AppPlace::TooMany,
                format!("An app could not be listed: {}", short(&app.name)),
            );
            return;
        }
        if self.list.found.len() >= pctwin_record::apps::MAX_APPS {
            self.over += 1;
            return;
        }
        self.list.found.push(app);
    }

    pub fn not_read(&mut self, owner: Owner, place: AppPlace, why: String) {
        if self.list.not_read.len() < pctwin_record::apps::MAX_NOT_READ {
            let why = clean(&why, pctwin_record::apps::MAX_APP_TEXT)
                .unwrap_or_else(|| "It could not be read.".into());
            self.list.not_read.push(AppsNotRead { owner, place, why });
        }
    }

    pub fn finish(mut self) -> AppList {
        if self.over > 0 {
            let n = self.over;
            self.not_read(
                Owner::Shared,
                AppPlace::TooMany,
                format!("{n} more apps were found than one list holds; they were not listed."),
            );
        }
        self.list
    }
}

/// The record's maker, as the maker crate compares it; `None` when there is nothing to anchor on.
pub fn maker_of_record(m: &pctwin_record::AppMaker) -> Option<pctwin_maker::Maker> {
    use pctwin_maker::facts::{LinuxSource, MacTeam, Maker, Sha256};
    use pctwin_record::AppMaker;
    Some(match m {
        AppMaker::Windows(s) => Maker::Windows(windows_signer(s)),
        AppMaker::MacTeam { team_id } => Maker::Mac(MacTeam {
            team_id: team_id.clone(),
        }),
        AppMaker::Rpm { key } => Maker::Linux(LinuxSource::Rpm { key: key.clone() }),
        AppMaker::Flatpak {
            remote_url,
            app_id,
            key,
        } => Maker::Linux(LinuxSource::Flatpak {
            remote_url: remote_url.clone(),
            app_id: app_id.clone(),
            key: Sha256(*key),
        }),
        AppMaker::WindowsPackage { .. }
        | AppMaker::Repository { .. }
        | AppMaker::Unsigned
        | AppMaker::NotRead { .. } => return None,
    })
}

/// File name endings of installers PCTwin looks at on the old laptop (the bytes decide what a
/// file is; the ending only picks which files are worth reading).
pub const INSTALLER_ENDINGS: &[&str] = &[
    "exe",
    "msi",
    "msix",
    "msixbundle",
    "appx",
    "dmg",
    "pkg",
    "zip",
];
/// Most installer files read on the old laptop.
pub const MAX_INSTALLER_FILES: usize = 256;

/// Links each Windows app to the installer files in this move that its own maker signed
/// (`candidates`: an item and where it is on the old laptop; `read`: PCTwin's own signature
/// reader). Only the same maker by the maker check's own rule counts, never a file name. Each app
/// keeps at most [`pctwin_record::apps::MAX_INSTALLERS`].
pub fn link_installers(
    apps: &mut AppList,
    candidates: &[(pctwin_record::ItemId, std::path::PathBuf)],
    read: &dyn Fn(&std::path::Path) -> pctwin_maker::OldCopy,
) {
    let signed: Vec<(pctwin_record::ItemId, pctwin_maker::Maker)> = candidates
        .iter()
        .take(MAX_INSTALLER_FILES)
        .filter_map(|(id, path)| match read(path).maker {
            pctwin_maker::OldMaker::Signed(s) => Some((*id, pctwin_maker::Maker::Windows(s))),
            _ => None,
        })
        .collect();
    for app in &mut apps.found {
        let Some(old @ pctwin_maker::Maker::Windows(_)) = maker_of_record(&app.maker) else {
            continue;
        };
        for (id, found) in &signed {
            if app.installers.len() >= pctwin_record::apps::MAX_INSTALLERS {
                break;
            }
            if matches!(
                pctwin_maker::compare(&old, found),
                pctwin_maker::SameMaker::Yes { .. }
            ) && !app.installers.contains(id)
            {
                app.installers.push(*id);
            }
        }
    }
}

/// Notes, for each app with installer files linked ([`link_installers`]), whether those files were
/// downloaded from its maker's own sites, as their download marks say (`read_mark`), against the
/// signed catalogue `rows` (Security Design, the scan rules, 30 October 2026). Only the yes or no
/// is kept, as one note: no for any file off the maker's sites, yes when every file that says
/// anything is on them. A file with no mark, or a mark that says nothing, adds nothing. The
/// address never leaves this function, and the app is still installed only from its maker.
pub fn note_download_marks(
    apps: &mut AppList,
    candidates: &[(pctwin_record::ItemId, std::path::PathBuf)],
    read_mark: &dyn Fn(&std::path::Path) -> Option<crate::routes::trail::MarkAddress>,
    rows: &pctwin_catalogue::Catalogue<crate::routes::counterpart::CounterpartRow>,
) {
    for app in &mut apps.found {
        let mut said: Option<bool> = None;
        for id in &app.installers {
            let Some((_, path)) = candidates.iter().find(|(c, _)| c == id) else {
                continue;
            };
            let Some(mark) = read_mark(path) else {
                continue;
            };
            if let Some(on) = crate::routes::trail::mark_on_makers_site(app, &mark, rows) {
                said = Some(said.unwrap_or(true) && on);
            }
        }
        if let Some(makers_site) = said {
            app.notes
                .retain(|n| !matches!(n, pctwin_record::AppNote::DownloadMark { .. }));
            app.notes
                .push(pctwin_record::AppNote::DownloadMark { makers_site });
        }
    }
}

/// Every app on this laptop, by its system's own rule (see the module notes). `people` are
/// everyone on the laptop (account ID and home folder), `me` the person PCTwin runs as;
/// `reader` (the parser process) reads installers' signatures.
pub fn read_here(
    people: &[(String, Option<std::path::PathBuf>)],
    me: &str,
    reader: &dyn pctwin_maker::OwnReader,
) -> AppList {
    let runner = pctwin_maker::run::SystemRunner;
    #[cfg(windows)]
    {
        let ids: Vec<String> = people.iter().map(|(id, _)| id.clone()).collect();
        windows::read_windows_apps(&ids, me, &runner, reader)
    }
    #[cfg(target_os = "macos")]
    {
        // Mac apps' makers are read by the system's own tools, not by PCTwin's reader.
        let _ = (me, reader);
        let mut places = vec![(
            std::path::PathBuf::from("/Applications"),
            Owner::Shared,
            AppPlace::MacApplications,
        )];
        for (id, home) in people {
            if let Some(home) = home {
                places.push((
                    home.join("Applications"),
                    Owner::Person {
                        account_id: id.clone(),
                    },
                    AppPlace::MacPersonApplications,
                ));
            }
        }
        mac::read_mac_apps(&places, &runner)
    }
    #[cfg(all(unix, not(target_os = "macos")))]
    {
        let _ = reader;
        let home = people
            .iter()
            .find(|(id, _)| id == me)
            .and_then(|(_, h)| h.clone())
            .unwrap_or_default();
        linux::read_linux_apps(
            &Owner::Person {
                account_id: me.to_string(),
            },
            &home,
            &runner,
        )
    }
}

fn short(s: &str) -> String {
    clean(s, 80).unwrap_or_else(|| "an app".into())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The download-trail clue (Security Design, the scan rules, 30 October 2026): a linked
    /// installer's mark becomes one yes or no note; the address itself reaches nothing the move
    /// keeps or shows (the record, the plan, the checklist), and the routes stay the maker's.
    #[test]
    fn a_download_mark_is_kept_only_as_a_yes_or_no() {
        use crate::routes::counterpart::tests::{TUNES, rows as counterparts, windows_tunes};
        use crate::routes::trail::zone_identifier_address;
        use pctwin_record::{AppNote, ItemId};
        let c = counterparts(TUNES);
        let item = ItemId::from_hex(&"07".repeat(16)).unwrap();
        let other = ItemId::from_hex(&"08".repeat(16)).unwrap();
        let mut app = windows_tunes();
        app.web = None;
        app.installers = vec![item];
        let candidates = vec![
            (item, std::path::PathBuf::from("Downloads/TunesSetup.msi")),
            (other, std::path::PathBuf::from("Downloads/Other.msi")),
        ];
        let secret = "https://dl.example-music.com/TunesSetup.msi?token=SECRET-TOKEN-123";
        let mark = |zone: &'static str| {
            move |_: &std::path::Path| {
                zone_identifier_address(&format!(
                    "[ZoneTransfer]\r\nZoneId=3\r\nHostUrl={zone}\r\n"
                ))
            }
        };
        let mut list = AppList {
            found: vec![app.clone()],
            not_read: vec![],
        };
        let on = mark("https://dl.example-music.com/TunesSetup.msi?token=SECRET-TOKEN-123");
        note_download_marks(&mut list, &candidates, &on, &c);
        assert_eq!(
            list.found[0].notes,
            [AppNote::DownloadMark { makers_site: true }]
        );
        // Nothing the move keeps or shows holds the address.
        let record = serde_json::to_string(&list).unwrap();
        let crate::routes::AppPlan { routes, card, .. } = crate::routes::plan_with(
            &list.found[0],
            crate::routes::Target {
                system: pctwin_record::AppSystem::Windows,
                arch: pctwin_record::AppArch::UNKNOWN,
                linux: None,
            },
            &crate::routes::Here::Absent,
            &crate::routes::Hints {
                rows: &pctwin_catalogue::Catalogue::shipped(
                    r#"{"format":1,"catalogue":"app-routes","version":1,"rows":[]}"#,
                ),
                winget: &|_: &FoundApp| vec![],
                casks: None,
            },
            &c,
        );
        let todo = crate::after::checklist(
            &list.found[0],
            false,
            false,
            crate::after::Installed::Failed,
            &[],
        );
        for shown in [record, format!("{routes:?}{card:?}"), format!("{todo:?}")] {
            assert!(
                !shown.contains("SECRET-TOKEN") && !shown.contains("dl.example-music"),
                "{shown}"
            );
        }
        assert!(secret.contains("SECRET-TOKEN"));
        // The app is still never installed from the mark's address: only its maker's routes
        // (here, the installer file itself, which the maker check still decides).
        assert_eq!(routes, [crate::routes::Route::OldInstaller { item }]);
        assert!(todo.contains(&crate::after::Todo::CameFrom { makers_site: true }));

        // Off the maker's sites: no. No mark, or a mark that says nothing: no note at all.
        let mut list = AppList {
            found: vec![app.clone()],
            not_read: vec![],
        };
        note_download_marks(
            &mut list,
            &candidates,
            &mark("https://mirror.example.net/t.msi"),
            &c,
        );
        assert_eq!(
            list.found[0].notes,
            [AppNote::DownloadMark { makers_site: false }]
        );
        for says_nothing in [mark("about:internet"), mark("https://203.0.113.9/t.msi")] {
            let mut list = AppList {
                found: vec![app.clone()],
                not_read: vec![],
            };
            note_download_marks(&mut list, &candidates, &says_nothing, &c);
            assert!(list.found[0].notes.is_empty());
        }
        let mut list = AppList {
            found: vec![app.clone()],
            not_read: vec![],
        };
        note_download_marks(&mut list, &candidates, &|_: &std::path::Path| None, &c);
        assert!(list.found[0].notes.is_empty());
        // Each file by its own mark: another app's file (listed first, off the maker's sites) is
        // never read for this one; and one file off the maker's sites among several is a no.
        let by_file = |p: &std::path::Path| {
            let url = if p.ends_with("Other.msi") {
                "https://mirror.example.net/o.msi"
            } else {
                "https://dl.example-music.com/t.msi"
            };
            zone_identifier_address(&format!(
                "[ZoneTransfer]
HostUrl={url}
"
            ))
        };
        let reversed: Vec<_> = candidates.iter().rev().cloned().collect();
        let mut list = AppList {
            found: vec![app.clone()],
            not_read: vec![],
        };
        note_download_marks(&mut list, &reversed, &by_file, &c);
        assert_eq!(
            list.found[0].notes,
            [AppNote::DownloadMark { makers_site: true }]
        );
        for installers in [vec![item, other], vec![other, item]] {
            let mut list = AppList {
                found: vec![app.clone()],
                not_read: vec![],
            };
            list.found[0].installers = installers;
            note_download_marks(&mut list, &candidates, &by_file, &c);
            assert_eq!(
                list.found[0].notes,
                [AppNote::DownloadMark { makers_site: false }]
            );
        }
    }

    #[test]
    fn text_from_the_old_laptop_is_cleaned_and_cut() {
        assert_eq!(clean("  Writer\u{202E}  ", 100).as_deref(), Some("Writer"));
        assert_eq!(clean("a\u{0007}b", 100).as_deref(), Some("ab"));
        assert_eq!(clean("   ", 100), None);
        assert_eq!(clean("ééé", 5).as_deref(), Some("éé"));
    }

    #[test]
    fn the_list_keeps_its_bound_and_says_what_it_left_out() {
        let mut c = Collector::default();
        let app = FoundApp {
            name: "A".into(),
            version: None,
            publisher: None,
            owner: Owner::Shared,
            system: pctwin_record::AppSystem::Windows,
            found: pctwin_record::FoundWhere::Snap { name: "a".into() },
            arch: AppArch::UNKNOWN,
            maker: pctwin_record::AppMaker::Unsigned,
            web: None,
            update_feed: None,
            installers: vec![],
            notes: vec![],
        };
        for _ in 0..pctwin_record::apps::MAX_APPS + 3 {
            c.add(app.clone());
        }
        let list = c.finish();
        assert_eq!(list.found.len(), pctwin_record::apps::MAX_APPS);
        assert_eq!(list.not_read.len(), 1);
        assert!(list.not_read[0].why.starts_with("3 more apps"));
        list.validate().unwrap();
    }

    #[test]
    fn installers_on_the_old_laptop_are_linked_by_their_maker_only() {
        let signer = |c: u8, org: &str| WindowsSigner {
            subject: pctwin_maker::facts::Subject {
                common_name: Some(org.into()),
                organization: Some(org.into()),
                country: Some("GB".into()),
                ..Default::default()
            },
            certificate: pctwin_maker::facts::Sha256([c; 32]),
            issuer: "CA".into(),
            issuer_organization: Some("CA".into()),
            public_key: None,
        };
        let mut apps = AppList::default();
        apps.found.push(FoundApp {
            name: "Writer".into(),
            version: None,
            publisher: None,
            owner: Owner::Shared,
            system: pctwin_record::AppSystem::Windows,
            found: pctwin_record::FoundWhere::Snap { name: "w".into() },
            arch: AppArch::UNKNOWN,
            maker: pctwin_record::AppMaker::Windows(Box::new(signer_id(&signer(1, "Writer Ltd")))),
            web: None,
            update_feed: None,
            installers: vec![],
            notes: vec![],
        });
        let id = |n: u8| pctwin_record::ItemId::from_hex(&format!("{n:02x}").repeat(16)).unwrap();
        // This system's own separator, so the file names read the same on every system.
        let at = |n: &str| std::path::Path::new("d").join(n);
        let candidates = vec![
            (id(1), at("WriterSetup.exe")),
            (id(2), at("Writer-fake.exe")),
            (id(3), at("renewed.msi")),
            (id(4), at("plain.exe")),
        ];
        let read = |p: &std::path::Path| pctwin_maker::OldCopy {
            maker: match p.file_name().unwrap().to_str().unwrap() {
                "WriterSetup.exe" => pctwin_maker::OldMaker::Signed(signer(1, "Writer Ltd")),
                // Named like the app, signed by someone else.
                "Writer-fake.exe" => pctwin_maker::OldMaker::Signed(signer(9, "Other Ltd")),
                // A renewed certificate of the same company.
                "renewed.msi" => pctwin_maker::OldMaker::Signed(signer(2, "Writer Ltd")),
                _ => pctwin_maker::OldMaker::Unsigned,
            },
            arch: None,
        };
        link_installers(&mut apps, &candidates, &read);
        assert_eq!(apps.found[0].installers, vec![id(1), id(3)]);
    }

    #[test]
    fn signers_cross_unchanged() {
        let s = WindowsSigner {
            subject: pctwin_maker::facts::Subject {
                common_name: Some("Example".into()),
                country: Some("GB".into()),
                ..Default::default()
            },
            certificate: pctwin_maker::facts::Sha256([1; 32]),
            issuer: "CA".into(),
            issuer_organization: Some("CA Org".into()),
            public_key: Some(pctwin_maker::facts::Sha256([2; 32])),
        };
        assert_eq!(windows_signer(&signer_id(&s)), s);
    }
}
