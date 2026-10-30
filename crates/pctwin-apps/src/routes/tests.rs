use super::*;
use pctwin_record::{Owner, RegistryView, SignerId};

fn win_target() -> Target {
    Target {
        system: AppSystem::Windows,
        arch: AppArch {
            known: true,
            x64: true,
            ..AppArch::UNKNOWN
        },
        linux: None,
    }
}

fn mac_target() -> Target {
    Target {
        system: AppSystem::Mac,
        arch: AppArch {
            known: true,
            arm64: true,
            ..AppArch::UNKNOWN
        },
        linux: None,
    }
}

fn signed_windows(code: Option<&str>) -> FoundApp {
    FoundApp {
        name: "Writer".into(),
        version: Some("2.0".into()),
        publisher: Some("Example Ltd".into()),
        owner: Owner::Shared,
        system: AppSystem::Windows,
        found: FoundWhere::WindowsEntry {
            view: RegistryView::Machine,
            key: "Writer".into(),
            product_code: code.map(Into::into),
        },
        arch: AppArch::UNKNOWN,
        maker: AppMaker::Windows(Box::new(SignerId {
            organization: Some("Example Ltd".into()),
            country: Some("GB".into()),
            ..Default::default()
        })),
        web: Some("https://example.com/writer".into()),
        update_feed: None,
        installers: vec![],
        notes: vec![],
    }
}

fn mac_app() -> FoundApp {
    FoundApp {
        name: "Painter".into(),
        version: Some("4.2.1".into()),
        publisher: None,
        owner: Owner::Shared,
        system: AppSystem::Mac,
        found: FoundWhere::MacBundle {
            bundle_id: "com.example.painter".into(),
            folder: "Painter.app".into(),
            app_store_id: None,
        },
        arch: AppArch {
            known: true,
            x64: true,
            ..AppArch::UNKNOWN
        },
        maker: AppMaker::MacTeam {
            team_id: "ABCDE12345".into(),
        },
        web: None,
        update_feed: Some("https://example.com/appcast.xml".into()),
        installers: vec![],
        notes: vec![],
    }
}

fn rows(json: &str) -> Catalogue<AppRouteRow> {
    Catalogue::shipped(&format!(
        r#"{{"format":1,"catalogue":"app-routes","version":1,"rows":[{json}]}}"#
    ))
}

fn none(_: &FoundApp) -> Vec<(String, WingetBy)> {
    vec![]
}

#[test]
fn a_signed_windows_app_tries_its_catalog_then_its_maker_then_its_installer_then_the_page() {
    let r = rows(
        r#"{"id":"w","system":"windows","match":{"product-code":"{11111111-2222-3333-4444-555555555555}"},"route":{"kind":"winget","id":"Example.Writer"},"licence-key":true},
           {"id":"w2","system":"windows","match":{"windows-org":"Example Ltd","windows-country":"GB"},"route":{"kind":"github-release","owner":"example","repo":"writer","asset-suffix":"-x64.msi"}}"#,
    );
    assert!(r.report().is_clean(), "{:?}", r.report());
    let mut app = signed_windows(Some("{11111111-2222-3333-4444-555555555555}"));
    let installer = ItemId::from_hex(&"ab".repeat(16)).unwrap();
    app.installers = vec![installer];
    let search = |_: &FoundApp| {
        vec![
            ("Example.Writer".to_string(), WingetBy::ProductCode),
            ("Example.WriterBeta".to_string(), WingetBy::ProductCode),
        ]
    };
    let p = plan(
        &app,
        win_target(),
        &Here::Absent,
        &Hints {
            rows: &r,
            winget: &search,
            casks: None,
        },
    );
    assert_eq!(p.disposition, Disposition::Install);
    assert_eq!(
        p.routes,
        vec![
            Route::Winget {
                id: "Example.Writer".into(),
                by: WingetBy::Row
            },
            Route::Winget {
                id: "Example.WriterBeta".into(),
                by: WingetBy::ProductCode
            },
            Route::Maker(MakerLink::Github {
                owner: "example".into(),
                repo: "writer".into(),
                asset_suffix: "-x64.msi".into()
            }),
            Route::OldInstaller { item: installer },
            Route::SignInPage {
                url: "https://example.com/writer".into()
            },
        ]
    );
    assert!(p.card.licence_key);
    assert_eq!(p.card.maker.as_deref(), Some("Example Ltd"));
}

#[test]
fn an_app_no_route_can_verify_ends_needs_action_with_its_card() {
    let r = rows("");
    let h = Hints {
        rows: &r,
        winget: &none,
        casks: None,
    };
    let mut unsigned = signed_windows(None);
    unsigned.maker = AppMaker::Unsigned;
    let p = plan(&unsigned, win_target(), &Here::Absent, &h);
    assert_eq!(p.disposition, Disposition::NeedsAction);
    assert!(p.routes.is_empty());
    assert_eq!(p.card.why, Some(NoRoute::Unsigned));
    assert_eq!(p.card.web.as_deref(), Some("https://example.com/writer"));
    assert!(!p.card.why.unwrap().plain().is_empty());

    let mut unread = signed_windows(None);
    unread.maker = AppMaker::NotRead { why: "x".into() };
    assert_eq!(
        plan(&unread, win_target(), &Here::Absent, &h).card.why,
        Some(NoRoute::MakerNotRead)
    );

    // Signed but with nowhere to look (no web address, no rows, nothing in winget).
    let mut nowhere = signed_windows(None);
    nowhere.web = None;
    let p = plan(&nowhere, win_target(), &Here::Absent, &h);
    assert_eq!(p.card.why, Some(NoRoute::NoSource));

    // A Windows app going to a Mac.
    let p = plan(&signed_windows(None), mac_target(), &Here::Absent, &h);
    assert_eq!(p.card.why, Some(NoRoute::OtherSystem));

    // A packaged app from outside the Store.
    let mut dev = signed_windows(None);
    dev.found = FoundWhere::WindowsPackage {
        family_name: "Dev.App_abc".into(),
        signing: PackageSigning::Developer,
    };
    dev.maker = AppMaker::WindowsPackage {
        publisher: "CN=Dev".into(),
    };
    assert_eq!(
        plan(&dev, win_target(), &Here::Absent, &h).card.why,
        Some(NoRoute::PackageOutsideStore)
    );
}

#[test]
fn an_app_already_on_the_new_laptop_is_never_installed_again_or_replaced() {
    let r = rows("");
    let h = Hints {
        rows: &r,
        winget: &none,
        casks: None,
    };
    let newer = plan(
        &signed_windows(None),
        win_target(),
        &Here::Present {
            version: Some("3.0".into()),
        },
        &h,
    );
    assert_eq!(newer.disposition, Disposition::AlreadyHere);
    assert!(newer.routes.is_empty());
    assert!(matches!(newer.flags[0], Flag::AlreadyHere { .. }));
    // T21-004: an older one is left as it is (no silent change), and said.
    let older = plan(
        &signed_windows(None),
        win_target(),
        &Here::Present {
            version: Some("1.0".into()),
        },
        &h,
    );
    assert_eq!(older.disposition, Disposition::AlreadyHere);
    assert!(matches!(older.flags[0], Flag::OlderHere { .. }));
    // "Update it": an older one carries the routes it could be updated by, the same as for an
    // app not here, used only if the person chooses; a same or newer one carries none.
    let absent = plan(&signed_windows(None), win_target(), &Here::Absent, &h);
    assert_eq!(older.routes, absent.routes);
    let unknown = plan(
        &signed_windows(None),
        win_target(),
        &Here::Unknown { why: "x".into() },
        &h,
    );
    assert_eq!(unknown.disposition, Disposition::Install);
    assert!(matches!(unknown.flags[0], Flag::HereUnknown { .. }));
}

#[test]
fn store_apps_go_to_the_store_and_windows_parts_stay_with_windows() {
    let r = rows("");
    let h = Hints {
        rows: &r,
        winget: &none,
        casks: None,
    };
    let mut store = signed_windows(None);
    store.found = FoundWhere::WindowsPackage {
        family_name: "Example.Notes_8wekyb3d8bbwe".into(),
        signing: PackageSigning::Store,
    };
    store.maker = AppMaker::WindowsPackage {
        publisher: "CN=Example".into(),
    };
    store.web = None;
    let p = plan(&store, win_target(), &Here::Absent, &h);
    assert_eq!(
        p.routes,
        vec![Route::StoreListing {
            family_name: "Example.Notes_8wekyb3d8bbwe".into()
        }]
    );
    store.found = FoundWhere::WindowsPackage {
        family_name: "Microsoft.Calc_x".into(),
        signing: PackageSigning::System,
    };
    assert_eq!(
        plan(&store, win_target(), &Here::Absent, &h).disposition,
        Disposition::PartOfSystem
    );
}

#[test]
fn a_mac_app_tries_the_app_store_casks_then_its_own_feed_and_never_an_older_cask() {
    let casks = brew::CaskIndex::parse(
        br#"[{"token":"painter","version":"4.3.0","sha256":"no_check","url":"https://example.com/p.dmg","artifacts":[{"app":["Painter.app"]}]},
             {"token":"painter-old","version":"3.0","sha256":"no_check","url":"https://example.com/old.dmg","artifacts":[{"app":["Painter.app"]},{"uninstall":[{"quit":"com.example.painter"}]}]}]"#,
    )
    .unwrap();
    let r = rows("");
    let mut app = mac_app();
    app.found = FoundWhere::MacBundle {
        bundle_id: "com.example.painter".into(),
        folder: "Painter.app".into(),
        app_store_id: Some(42),
    };
    let p = plan(
        &app,
        mac_target(),
        &Here::Absent,
        &Hints {
            rows: &r,
            winget: &none,
            casks: Some(&casks),
        },
    );
    assert_eq!(p.routes[0], Route::MacAppStore { adam_id: 42 });
    assert!(matches!(&p.routes[1], Route::Cask { token, .. } if token == "painter"));
    assert_eq!(
        p.routes[2],
        Route::Maker(MakerLink::Feed {
            format: FeedFormat::Sparkle,
            url: "https://example.com/appcast.xml".into()
        })
    );
    assert_eq!(p.routes.len(), 3, "{:?}", p.routes);
    assert!(p.flags.contains(&Flag::IntelOnly));
    assert!(
        p.flags
            .iter()
            .any(|f| matches!(f, Flag::OlderLeftOut { route } if route == "3.0"))
    );
}

#[test]
fn only_plain_casks_are_routes_each_with_the_build_for_the_new_mac() {
    let casks = brew::CaskIndex::parse(
        br#"[{"token":"painter","version":"4.3.0","sha256":"no_check","url":"https://example.com/arm.dmg","artifacts":[{"app":["Painter.app"]}],
              "variations":{"sonoma":{"url":"https://example.com/intel.dmg"}}},
             {"token":"painter-pkg","version":"4.3.0","sha256":"no_check","url":"https://example.com/p.pkg","artifacts":[{"app":["Painter.app"]},{"pkg":["p.pkg"]}]}]"#,
    )
    .unwrap();
    let r = rows("");
    let mut app = mac_app();
    app.found = FoundWhere::MacBundle {
        bundle_id: "com.example.painter".into(),
        folder: "Painter.app".into(),
        app_store_id: None,
    };
    let hints = Hints {
        rows: &r,
        winget: &none,
        casks: Some(&casks),
    };
    let casks_of = |t: Target| -> Vec<(String, String, String)> {
        plan(&app, t, &Here::Absent, &hints)
            .routes
            .into_iter()
            .filter_map(|r| match r {
                Route::Cask {
                    token, url, app, ..
                } => Some((token, url, app)),
                _ => None,
            })
            .collect()
    };
    // The package cask is never a route; Apple silicon gets the list's own build.
    assert_eq!(
        casks_of(mac_target()),
        [(
            "painter".to_string(),
            "https://example.com/arm.dmg".to_string(),
            "Painter.app".to_string()
        )]
    );
    let mut intel = mac_target();
    intel.arch = AppArch {
        known: true,
        x86: false,
        x64: true,
        arm64: false,
    };
    assert_eq!(casks_of(intel)[0].1, "https://example.com/intel.dmg");
}

#[test]
fn linux_apps_come_from_the_same_kind_of_source() {
    let r = rows("");
    let h = Hints {
        rows: &r,
        winget: &none,
        casks: None,
    };
    let linux = |found, maker| FoundApp {
        name: "Viewer".into(),
        version: None,
        publisher: None,
        owner: Owner::Shared,
        system: AppSystem::Linux,
        found,
        arch: AppArch::UNKNOWN,
        maker,
        web: None,
        update_feed: None,
        installers: vec![],
        notes: vec![],
    };
    let t = |p| Target {
        system: AppSystem::Linux,
        arch: AppArch::UNKNOWN,
        linux: Some(p),
    };
    let deb = linux(
        FoundWhere::Deb {
            package: "viewer".into(),
        },
        AppMaker::Repository {
            origin: "http://archive.ubuntu.com/ubuntu noble/main".into(),
        },
    );
    assert_eq!(
        plan(&deb, t(LinuxPackages::Apt), &Here::Absent, &h).routes,
        vec![Route::Apt {
            package: "viewer".into()
        }]
    );
    // A Debian package going to a Fedora laptop: no same source.
    assert_eq!(
        plan(&deb, t(LinuxPackages::Dnf), &Here::Absent, &h)
            .card
            .why,
        Some(NoRoute::NoSource)
    );
    let fp = linux(
        FoundWhere::Flatpak {
            app_id: "org.example.Maps".into(),
            remote: "flathub".into(),
            remote_url: None,
            system_wide: true,
        },
        AppMaker::Flatpak {
            remote_url: "https://dl.flathub.org/repo/".into(),
            app_id: "org.example.Maps".into(),
            key: [1; 32],
        },
    );
    assert!(
        matches!(&plan(&fp, t(LinuxPackages::Dnf), &Here::Absent, &h).routes[0], Route::Flatpak { key, .. } if *key == [1; 32])
    );
}

/// ADOPTION change 7: a new route added only as a catalogue row works with no code change.
#[test]
fn a_route_added_as_a_row_alone_is_used() {
    let r = rows(
        r#"{"id":"painter-fixed","system":"mac","match":{"team-id":"ABCDE12345","bundle-id":"com.example.painter"},"route":{"kind":"fixed-link","url":"https://example.com/Painter-5.0.dmg","version":"5.0","sha256":"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"},"sign-in":true}"#,
    );
    let mut app = mac_app();
    app.update_feed = None;
    let p = plan(
        &app,
        mac_target(),
        &Here::Absent,
        &Hints {
            rows: &r,
            winget: &none,
            casks: None,
        },
    );
    assert_eq!(
        p.routes,
        vec![Route::Maker(MakerLink::Fixed {
            url: "https://example.com/Painter-5.0.dmg".into(),
            version: "5.0".into(),
            sha256: [0xaa; 32]
        })]
    );
    assert!(p.card.sign_in);
    // A row for another team adds nothing.
    let other = rows(
        r#"{"id":"x","system":"mac","match":{"team-id":"ZZZZZ99999"},"route":{"kind":"latest-link","url":"https://evil.example.com/x.dmg"}}"#,
    );
    let p = plan(
        &app,
        mac_target(),
        &Here::Absent,
        &Hints {
            rows: &other,
            winget: &none,
            casks: None,
        },
    );
    assert_eq!(p.card.why, Some(NoRoute::NoSource));
}

/// Only a secure feed the app itself names is used, and a sign-in page only where the download's
/// maker can be checked.
#[test]
fn feeds_and_pages_only_where_they_can_be_checked() {
    let r = rows("");
    let h = Hints {
        rows: &r,
        winget: &none,
        casks: None,
    };
    let mut app = mac_app();
    app.update_feed = Some("http://example.com/appcast.xml".into());
    assert_eq!(
        plan(&app, mac_target(), &Here::Absent, &h).card.why,
        Some(NoRoute::NoSource)
    );
    let deb = FoundApp {
        name: "Viewer".into(),
        version: None,
        publisher: None,
        owner: Owner::Shared,
        system: AppSystem::Linux,
        found: FoundWhere::Deb {
            package: "viewer".into(),
        },
        arch: AppArch::UNKNOWN,
        maker: AppMaker::Repository { origin: "x".into() },
        web: Some("https://viewer.example.org/".into()),
        update_feed: None,
        installers: vec![],
        notes: vec![],
    };
    let t = Target {
        system: AppSystem::Linux,
        arch: AppArch::UNKNOWN,
        linux: Some(LinuxPackages::Apt),
    };
    assert_eq!(
        plan(&deb, t, &Here::Absent, &h).routes,
        vec![Route::Apt {
            package: "viewer".into()
        }]
    );
}

mod other_systems {
    use super::super::counterpart::tests::{TUNES, rows as counterparts, windows_tunes};
    use super::*;

    fn linux_target() -> Target {
        Target {
            system: AppSystem::Linux,
            arch: AppArch::UNKNOWN,
            linux: Some(LinuxPackages::Dnf),
        }
    }

    const ROUTES: &str = r#"{"id":"tunes-mac","system":"mac","match":{"team-id":"ABCDE12345"},"route":{"kind":"latest-link","url":"https://example-music.com/Tunes.dmg"}},
        {"id":"tunes-win","system":"windows","match":{"windows-org":"Example Music AB","windows-country":"SE"},"route":{"kind":"winget","id":"ExampleMusic.Tunes"}}"#;

    #[test]
    fn a_windows_app_going_to_a_mac_becomes_its_makers_mac_edition() {
        let c = counterparts(TUNES);
        let r = rows(ROUTES);
        let h = Hints {
            rows: &r,
            winget: &none,
            casks: None,
        };
        let p = plan_with(&windows_tunes(), mac_target(), &Here::Absent, &h, &c);
        assert_eq!(p.disposition, Disposition::Install);
        assert_eq!(
            p.routes,
            vec![Route::Maker(MakerLink::Latest {
                url: "https://example-music.com/Tunes.dmg".into()
            })]
        );
        assert!(p.flags.contains(&Flag::Counterpart {
            from: AppSystem::Windows
        }));
        // Every check is against the maker the row names for the Mac.
        let s = p.stand_in.as_ref().unwrap();
        assert_eq!(
            s.maker,
            AppMaker::MacTeam {
                team_id: "ABCDE12345".into()
            }
        );
        assert_eq!(p.subject(&windows_tunes()), s);
        // The person's card is still their app's, with the choices beside it.
        assert_eq!(p.card.name, "Tunes");
        assert_eq!(p.card.version.as_deref(), Some("1.2.3"));
        assert_eq!(p.card.choices[0].name, "Tunes on the web");
    }

    #[test]
    fn a_mac_app_going_to_windows_is_checked_against_the_pinned_signer() {
        let c = counterparts(TUNES);
        let r = rows(ROUTES);
        let h = Hints {
            rows: &r,
            winget: &none,
            casks: None,
        };
        let mac =
            super::super::counterpart::stand_in(&windows_tunes(), &c.rows()[0], AppSystem::Mac)
                .unwrap();
        let p = plan_with(&mac, win_target(), &Here::Absent, &h, &c);
        assert_eq!(p.disposition, Disposition::Install);
        assert_eq!(
            p.routes,
            vec![Route::Winget {
                id: "ExampleMusic.Tunes".into(),
                by: WingetBy::Row
            }]
        );
        let AppMaker::Windows(s) = &p.stand_in.as_ref().unwrap().maker else {
            panic!("not a Windows maker")
        };
        assert_eq!(s.issuer_organization.as_deref(), Some("Example CA Inc."));
    }

    #[test]
    fn a_windows_app_going_to_linux_becomes_its_flatpak_from_the_named_remote() {
        let c = counterparts(TUNES);
        let r = rows("");
        let h = Hints {
            rows: &r,
            winget: &none,
            casks: None,
        };
        let p = plan_with(&windows_tunes(), linux_target(), &Here::Absent, &h, &c);
        assert_eq!(
            p.routes,
            vec![Route::Flatpak {
                remote_url: "https://dl.flathub.org/repo/".into(),
                app_id: "com.example.Tunes".into(),
                key: [1; 32],
            }]
        );
    }

    #[test]
    fn with_no_edition_it_ends_needs_action_with_the_choices_and_never_another_maker() {
        // The maker makes no Mac edition.
        let c = counterparts(&TUNES.replace(
            r#"{"system":"mac","name":"Tunes","match":{"bundle-id":"com.example.tunes","team-id":"ABCDE12345"}},"#,
            "",
        ));
        assert_eq!(c.rows().len(), 1);
        let r = rows(ROUTES);
        let h = Hints {
            rows: &r,
            winget: &none,
            casks: None,
        };
        let p = plan_with(&windows_tunes(), mac_target(), &Here::Absent, &h, &c);
        assert_eq!(p.disposition, Disposition::NeedsAction);
        assert_eq!(p.card.why, Some(NoRoute::OtherSystem));
        assert!(p.routes.is_empty());
        assert_eq!(p.stand_in, None);
        let names: Vec<_> = p.card.choices.iter().map(|c| c.name.as_str()).collect();
        assert_eq!(names, ["Tunes on the web", "Other Player", "Wine"]);

        // An edition PCTwin could install, with nowhere to get it: the same card, never a
        // route to anything else.
        let full = counterparts(TUNES);
        let none_rows = rows("");
        let h = Hints {
            rows: &none_rows,
            winget: &none,
            casks: None,
        };
        let p = plan_with(&windows_tunes(), mac_target(), &Here::Absent, &h, &full);
        assert_eq!(p.disposition, Disposition::NeedsAction);
        assert_eq!(p.card.why, Some(NoRoute::OtherSystem));
        assert_eq!(p.stand_in, None);
        assert!(
            !p.flags
                .iter()
                .any(|f| matches!(f, Flag::Counterpart { .. }))
        );

        // No row at all.
        let p = plan_with(
            &windows_tunes(),
            mac_target(),
            &Here::Absent,
            &h,
            &counterparts(""),
        );
        assert_eq!(p.card.why, Some(NoRoute::OtherSystem));
        assert!(p.card.choices.is_empty());
    }

    #[test]
    fn the_edition_already_here_is_left_and_versions_are_not_compared_across_systems() {
        let c = counterparts(TUNES);
        let r = rows(ROUTES);
        let h = Hints {
            rows: &r,
            winget: &none,
            casks: None,
        };
        let here = Here::Present {
            version: Some("0.1".into()),
        };
        let p = plan_with(&windows_tunes(), mac_target(), &here, &h, &c);
        assert_eq!(p.disposition, Disposition::AlreadyHere);
        assert!(!p.flags.iter().any(|f| matches!(f, Flag::OlderHere { .. })));
        // The stand-in is what decides "here".
        assert!(here_subject(&windows_tunes(), AppSystem::Mac, &c).is_some());
        assert!(here_subject(&windows_tunes(), AppSystem::Windows, &c).is_none());
    }

    #[test]
    fn a_recorded_address_off_the_makers_sites_is_said_and_never_used() {
        let c = counterparts(TUNES);
        let r = rows("");
        let h = Hints {
            rows: &r,
            winget: &none,
            casks: None,
        };
        let mut app = windows_tunes();
        app.web = Some("https://tunes-download.example.net/setup".into());
        let p = plan_with(&app, win_target(), &Here::Absent, &h, &c);
        assert_eq!(p.card.web, None);
        assert!(
            matches!(&p.card.trail, Some(trail::Trail::NotMakersSite { host }) if host == "tunes-download.example.net")
        );
        assert!(
            !p.routes
                .iter()
                .any(|r| matches!(r, Route::SignInPage { .. }))
        );

        // On the maker's own site: used as the page.
        let p = plan_with(&windows_tunes(), win_target(), &Here::Absent, &h, &c);
        assert_eq!(
            p.routes,
            vec![Route::SignInPage {
                url: "https://www.example-music.com/download".into()
            }]
        );
        assert!(matches!(
            p.card.trail,
            Some(trail::Trail::OnMakersSite { .. })
        ));
    }
}

#[test]
fn the_package_system_comes_from_os_release_and_its_family() {
    let of = LinuxPackages::from_os_release;
    assert_eq!(
        of("NAME=\"Fedora Linux\"\nID=fedora\nVERSION_ID=41\n"),
        Some(LinuxPackages::Dnf)
    );
    assert_eq!(of("ID=ubuntu\nID_LIKE=debian\n"), Some(LinuxPackages::Apt));
    assert_eq!(of("ID=debian\n"), Some(LinuxPackages::Apt));
    // Derivatives, by their family.
    assert_eq!(
        of("ID=\"rocky\"\nID_LIKE=\"rhel centos fedora\"\n"),
        Some(LinuxPackages::Dnf)
    );
    assert_eq!(
        of("ID=nobara\nID_LIKE=\"fedora\"\n"),
        Some(LinuxPackages::Dnf)
    );
    assert_eq!(
        of("ID=linuxmint\nID_LIKE=ubuntu\n"),
        Some(LinuxPackages::Apt)
    );
    assert_eq!(
        of("ID='almalinux'\nID_LIKE='rhel centos fedora'\n"),
        Some(LinuxPackages::Dnf)
    );
    // Families PCTwin does not install from, and nothing at all.
    assert_eq!(
        of("ID=\"opensuse-tumbleweed\"\nID_LIKE=\"opensuse suse\"\n"),
        None
    );
    assert_eq!(of("ID=arch\n"), None);
    assert_eq!(of(""), None);
    // A name that only contains a family's is not that family.
    assert_eq!(of("ID=fedorafake\n"), None);
    // The own ID wins over a family named later.
    assert_eq!(of("ID=ubuntu\nID_LIKE=fedora\n"), Some(LinuxPackages::Apt));
}

/// A store's page is made only from the found app's own identity, for the two stores only.
#[test]
fn a_store_page_is_the_exact_apps_own_page() {
    let ms = Route::StoreListing {
        family_name: "Example.Writer_8wekyb3d8bbwe".into(),
    };
    assert_eq!(
        ms.store_page().as_deref(),
        Some("ms-windows-store://pdp/?PFN=Example.Writer_8wekyb3d8bbwe")
    );
    let apple = Route::MacAppStore { adam_id: 409183694 };
    assert_eq!(
        apple.store_page().as_deref(),
        Some("macappstore://apps.apple.com/app/id409183694")
    );
    for bad in ["", "a&b=c", "x/../y", "name with space"] {
        let r = Route::StoreListing {
            family_name: bad.into(),
        };
        assert_eq!(r.store_page(), None, "{bad}");
    }
    assert_eq!(Route::MacAppStore { adam_id: 0 }.store_page(), None);
    // The check before opening takes back exactly those pages.
    assert!(is_store_page(
        "ms-windows-store://pdp/?PFN=Example.Writer_8wekyb3d8bbwe"
    ));
    assert!(is_store_page(
        "macappstore://apps.apple.com/app/id409183694"
    ));
    for bad in [
        "https://example.com",
        "ms-windows-store://pdp/?PFN=a&b",
        "ms-windows-store://pdp/?PFN=",
        "macappstore://apps.apple.com/app/id+12",
        "macappstore://apps.apple.com/app/id12/../x",
        "macappstore://apps.apple.com/app/id0",
        "file:///C:/Windows/System32/calc.exe",
    ] {
        assert!(!is_store_page(bad), "{bad}");
    }
    assert_eq!(
        Route::Snap {
            name: "writer".into()
        }
        .store_page(),
        None
    );
}
