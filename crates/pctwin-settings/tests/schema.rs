//! The neutral settings list (T24-001, T24-002, T24-010) and its fixed tables.

use pctwin_recipe::SettingValue;
use pctwin_settings::codec::{self, LAYOUTS};
use pctwin_settings::schema::{
    Risk, Route, SchemaError, SettingId, Support, System, check_value, full_path, plain_name,
    support,
};

fn text(s: &str) -> SettingValue {
    SettingValue::Text(s.into())
}

#[test]
fn t24_001_a_key_not_on_the_list_is_refused() {
    for key in [
        "HKCU\\Software\\Evil",
        "com.apple.loginwindow LoginHook",
        "appearance",
        "appearance.theme.extra",
        "APPEARANCE.THEME",
        "",
        "display.positions",
    ] {
        assert_eq!(SettingId::from_key(key), None, "{key}");
    }
    let bad: Result<SettingId, _> = serde_json::from_str("\"security.firewall\"");
    assert!(bad.is_err());
}

#[test]
fn every_setting_round_trips_by_its_key_and_keys_are_unique() {
    let mut keys = std::collections::BTreeSet::new();
    for id in SettingId::ALL {
        assert_eq!(SettingId::from_key(id.key()), Some(id));
        assert!(keys.insert(id.key()));
        assert!(pctwin_recipe::key_ok(id.key()), "{}", id.key());
        let json = serde_json::to_string(&id).unwrap();
        assert_eq!(serde_json::from_str::<SettingId>(&json).unwrap(), id);
    }
}

#[test]
fn t24_002_wrong_types_and_out_of_range_values_are_refused() {
    use SettingId as S;
    let cases: Vec<(S, SettingValue, SchemaError)> = vec![
        (
            S::Theme,
            text("purple"),
            SchemaError::OutOfRange("appearance.theme"),
        ),
        (
            S::Theme,
            SettingValue::Bool(true),
            SchemaError::WrongType("appearance.theme"),
        ),
        (
            S::TaskbarTheme,
            text("auto"),
            SchemaError::OutOfRange("appearance.taskbar-theme"),
        ),
        (
            S::TextSize,
            SettingValue::Int(99),
            SchemaError::OutOfRange("accessibility.text-size"),
        ),
        (
            S::TextSize,
            SettingValue::Int(226),
            SchemaError::OutOfRange("accessibility.text-size"),
        ),
        (
            S::PointerSpeed,
            SettingValue::Int(-1),
            SchemaError::OutOfRange("pointer.speed"),
        ),
        (
            S::PointerSpeed,
            SettingValue::Int(1001),
            SchemaError::OutOfRange("pointer.speed"),
        ),
        (
            S::DisplayScaling,
            SettingValue::Int(501),
            SchemaError::OutOfRange("display.scaling"),
        ),
        (
            S::NaturalScroll,
            SettingValue::Int(1),
            SchemaError::WrongType("pointer.natural-scroll"),
        ),
        (
            S::Wallpaper,
            text("pictures/sea.jpg"),
            SchemaError::OutOfRange("desktop.wallpaper"),
        ),
        (
            S::Wallpaper,
            text("C:\\Users\\a\\..\\b\\x.jpg"),
            SchemaError::OutOfRange("desktop.wallpaper"),
        ),
        (
            S::Wallpaper,
            text("\\\\server\\share\\x.jpg"),
            SchemaError::OutOfRange("desktop.wallpaper"),
        ),
        (
            S::Wallpaper,
            text("/home/a/\u{202E}gpj.exe"),
            SchemaError::HiddenText("desktop.wallpaper"),
        ),
        (
            S::KeyboardLayouts,
            text("us,fr; rm -rf /"),
            SchemaError::OutOfRange("keyboard.layouts"),
        ),
        (
            S::KeyboardLayouts,
            text(&vec!["us"; 17].join(",")),
            SchemaError::OutOfRange("keyboard.layouts"),
        ),
        (
            S::DefaultBrowser,
            text("firefox --remote-debugging-port=1"),
            SchemaError::OutOfRange("apps.default-browser"),
        ),
        (
            S::TimeZone,
            text("../../etc/passwd"),
            SchemaError::OutOfRange("time.zone"),
        ),
        (
            S::DefaultPrinter,
            text("\\\\server\\printer"),
            SchemaError::OutOfRange("devices.default-printer"),
        ),
        (
            S::WallpaperStyle,
            text("zoom"),
            SchemaError::OutOfRange("desktop.wallpaper-style"),
        ),
        (
            S::PrimaryButton,
            text("middle"),
            SchemaError::OutOfRange("pointer.primary-button"),
        ),
    ];
    for (id, v, want) in cases {
        assert_eq!(check_value(id, &v), Err(want), "{id:?} {v:?}");
    }
}

#[test]
fn good_values_pass_at_the_edges() {
    use SettingId as S;
    for (id, v) in [
        (S::TextSize, SettingValue::Int(100)),
        (S::TextSize, SettingValue::Int(225)),
        (S::PointerSpeed, SettingValue::Int(0)),
        (S::PointerSpeed, SettingValue::Int(1000)),
        (S::DisplayScaling, SettingValue::Int(100)),
        (S::DisplayScaling, SettingValue::Int(500)),
        (S::Theme, text("auto")),
        (S::KeyboardLayouts, text("")),
        (S::KeyboardLayouts, text(&vec!["us"; 16].join(","))),
        (S::Wallpaper, text("C:\\Users\\a\\Pictures\\sea.jpg")),
        (S::Wallpaper, text("/Users/a/Pictures/sea.jpg")),
        (S::DefaultPrinter, text("Office Printer 2")),
        (S::TimeZone, text("America/Argentina/Buenos_Aires")),
    ] {
        assert_eq!(check_value(id, &v), Ok(()), "{id:?} {v:?}");
    }
}

#[test]
fn t24_010_non_ascii_and_right_to_left_values_are_kept_exactly() {
    // A desktop picture in a folder named in Arabic, Chinese and Yoruba, and an Arabic layout.
    for p in [
        "C:\\Users\\أحمد\\الصور\\بحر.jpg",
        "/home/李/图片/海.png",
        "/Users/adé/Àwòrán/òkun.jpg",
    ] {
        assert_eq!(check_value(SettingId::Wallpaper, &text(p)), Ok(()), "{p}");
    }
    assert_eq!(
        check_value(SettingId::DefaultPrinter, &text("طابعة المكتب")),
        Ok(())
    );
    assert_eq!(
        codec::layout_to_neutral(System::Windows, "00000401").as_deref(),
        Some("ar")
    );
    assert_eq!(
        codec::layout_from_neutral(System::Gnome, "ar").as_deref(),
        Some("ara")
    );
    assert_eq!(
        codec::layout_from_neutral(System::Mac, "he").as_deref(),
        Some("com.apple.keylayout.Hebrew")
    );
    // A zero-width joiner is part of real words; a direction override is not.
    assert!(plain_name("क्\u{200D}ष"));
    assert_eq!(
        check_value(SettingId::DefaultPrinter, &text("Office\u{202E}")),
        Err(SchemaError::HiddenText("devices.default-printer"))
    );
}

#[test]
fn routes_and_risks_are_fixed() {
    assert_eq!(SettingId::DefaultBrowser.route(), Route::Handoff);
    assert_eq!(SettingId::FileHandlers.route(), Route::Handoff);
    assert_eq!(SettingId::TimeZone.route(), Route::Privileged);
    assert_eq!(SettingId::StartupApps.route(), Route::OptIn);
    assert_eq!(SettingId::HighContrast.route(), Route::Guided);
    assert_eq!(SettingId::DisplayScaling.route(), Route::Guided);
    assert_eq!(SettingId::Theme.route(), Route::Native);
    assert_eq!(SettingId::PrimaryButton.risk(), Risk::MayStrand);
    assert_eq!(SettingId::Theme.risk(), Risk::Safe);
    for id in SettingId::ALL {
        for system in [
            System::Windows,
            System::Mac,
            System::Gnome,
            System::Kde,
            System::OtherLinux,
        ] {
            if id.route() != Route::Native {
                assert_ne!(support(system, id), Support::Native, "{id:?} on {system:?}");
            }
        }
    }
    assert_eq!(
        support(System::Windows, SettingId::TaskbarTheme),
        Support::Native
    );
    assert_eq!(
        support(System::Mac, SettingId::TaskbarTheme),
        Support::NotApplicable
    );
    assert_eq!(support(System::Mac, SettingId::Wallpaper), Support::Guided);
    assert_eq!(
        support(System::Gnome, SettingId::KeyboardLayouts),
        Support::Native
    );
    assert_eq!(
        support(System::Windows, SettingId::KeyboardLayouts),
        Support::Guided
    );
    assert_eq!(support(System::Kde, SettingId::Theme), Support::Guided);
}

#[test]
fn windows_pointer_steps_round_trip_exactly() {
    for step in 1..=20 {
        let n = codec::windows_speed_to_neutral(step).unwrap();
        assert!((0..=1000).contains(&n));
        assert_eq!(
            codec::neutral_to_windows_speed(n),
            Some(step),
            "step {step}"
        );
        assert_eq!(codec::quantize_speed(System::Windows, n), Some(n));
    }
    assert_eq!(codec::windows_speed_to_neutral(1), Some(0));
    assert_eq!(codec::windows_speed_to_neutral(10), Some(474));
    assert_eq!(codec::windows_speed_to_neutral(20), Some(1000));
    assert_eq!(codec::windows_speed_to_neutral(0), None);
    assert_eq!(codec::windows_speed_to_neutral(21), None);
    assert_eq!(codec::neutral_to_windows_speed(1001), None);
    assert_eq!(codec::neutral_to_windows_speed(-1), None);
}

#[test]
fn every_system_reads_back_its_own_quantized_speed() {
    for system in [System::Windows, System::Mac, System::Gnome] {
        for p in 0..=1000 {
            let q = codec::quantize_speed(system, p).unwrap();
            assert_eq!(codec::quantize_speed(system, q), Some(q), "{system:?} {p}");
        }
    }
    assert_eq!(codec::mac_speed_to_neutral(3.0), Some(1000));
    assert_eq!(codec::mac_speed_to_neutral(3.5), None);
    assert_eq!(codec::mac_speed_to_neutral(f64::NAN), None);
    assert_eq!(codec::gnome_speed_to_neutral(-1.0), Some(0));
    assert_eq!(codec::gnome_speed_to_neutral(1.0001), None);
    assert_eq!(codec::neutral_to_gnome_speed(500).as_deref(), Some("0.000"));
    assert_eq!(codec::neutral_to_mac_speed(1000).as_deref(), Some("3.000"));
    assert_eq!(codec::quantize_speed(System::Kde, 10), None);
}

#[test]
fn layouts_not_on_the_table_stay_with_their_own_system() {
    assert_eq!(
        codec::layout_to_neutral(System::Windows, "0000041E").as_deref(),
        Some("windows:0000041E")
    );
    assert_eq!(
        codec::layout_from_neutral(System::Windows, "windows:0000041E").as_deref(),
        Some("0000041E")
    );
    assert_eq!(
        codec::layout_from_neutral(System::Gnome, "windows:0000041E"),
        None
    );
    assert_eq!(codec::layout_to_neutral(System::Gnome, "us; rm"), None);
    for l in LAYOUTS {
        for s in [System::Windows, System::Mac, System::Gnome] {
            let native = codec::layout_from_neutral(s, l.neutral).unwrap();
            assert_eq!(
                codec::layout_to_neutral(s, &native).as_deref(),
                Some(l.neutral)
            );
        }
        assert!(plain_name(l.neutral));
    }
}

#[test]
fn browsers_are_recognised_by_their_own_names() {
    assert_eq!(
        codec::browser_to_neutral(System::Windows, "FirefoxURL-308046B0AF4A39CB"),
        Some("firefox")
    );
    assert_eq!(
        codec::browser_to_neutral(System::Windows, "ChromeHTML"),
        Some("chrome")
    );
    assert_eq!(
        codec::browser_to_neutral(System::Windows, "ChromeHTMLX"),
        None
    );
    assert_eq!(
        codec::browser_to_neutral(System::Windows, "FirefoxURLevil"),
        None
    );
    assert_eq!(codec::browser_to_neutral(System::Windows, ""), None);
    assert_eq!(
        codec::browser_to_neutral(System::Mac, "com.apple.Safari"),
        Some("safari")
    );
    assert_eq!(
        codec::browser_to_neutral(System::Gnome, "firefox_firefox.desktop"),
        Some("firefox")
    );
    assert!(!codec::browser_exists_on(System::Windows, "safari"));
    assert!(!codec::browser_exists_on(System::Gnome, "safari"));
    assert!(codec::browser_exists_on(System::Mac, "safari"));
    assert!(!codec::browser_exists_on(System::Mac, "netscape"));
}

#[test]
fn time_zones_map_both_ways() {
    assert_eq!(
        codec::windows_zone_to_iana("W. Central Africa Standard Time"),
        Some("Africa/Lagos")
    );
    assert_eq!(
        codec::iana_to_windows_zone("Asia/Kolkata"),
        Some("India Standard Time")
    );
    assert_eq!(codec::windows_zone_to_iana("Mars Standard Time"), None);
    for (w, i) in codec::WINDOWS_ZONES {
        assert_eq!(codec::iana_to_windows_zone(i), Some(*w));
        assert!(plain_name(i));
    }
}

#[test]
fn wallpaper_styles_and_file_addresses_round_trip() {
    for s in pctwin_settings::schema::WALLPAPER_STYLES {
        let (style, tile) = codec::windows_style(s).unwrap();
        assert_eq!(codec::windows_style_to_neutral(style, tile), Some(*s));
        assert_eq!(
            codec::gnome_style_to_neutral(codec::gnome_style(s).unwrap()),
            Some(*s)
        );
    }
    assert_eq!(codec::windows_style("zoom"), None);
    assert_eq!(codec::windows_style_to_neutral("7", "0"), None);
    let p = "/home/adé/My Pictures/sea%20.jpg";
    let uri = codec::file_uri(p).unwrap();
    assert_eq!(uri, "file:///home/ad%C3%A9/My%20Pictures/sea%2520.jpg");
    assert_eq!(codec::path_of_file_uri(&uri).as_deref(), Some(p));
    assert_eq!(codec::file_uri("relative/x"), None);
    assert_eq!(codec::path_of_file_uri("https://example.com/x.jpg"), None);
    assert_eq!(codec::path_of_file_uri("file:///a/%zz"), None);
    assert_eq!(codec::path_of_file_uri("file:///a/%00b"), None);
    assert_eq!(codec::path_of_file_uri("file://host/a"), None);
}

#[test]
fn full_paths_are_absolute_and_plain() {
    assert!(full_path("C:\\x\\y.jpg"));
    assert!(full_path("/x/y.jpg"));
    assert!(!full_path("//server/x"));
    assert!(!full_path("C:x"));
    assert!(!full_path("C:\\"));
    assert!(!full_path("/a/./b"));
    assert!(!full_path("x/y"));
}
