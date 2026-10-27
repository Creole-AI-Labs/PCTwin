//! The generic rules, by kind of value (the user's decision of 27 October 2026).

use pctwin_recipe::SettingValue;
use pctwin_settings::classify::{
    Ctx, Fate, FolderMap, Why, classify, classify_kind, looks_secret, store_kind, words,
};

fn t(s: &str) -> SettingValue {
    SettingValue::Text(s.into())
}

fn map() -> FolderMap {
    FolderMap(vec![
        (
            "C:\\Users\\ada\\Documents".into(),
            "C:\\Users\\ada2\\Documents".into(),
        ),
        ("/home/ada/Projects".into(), "/home/ada/Work".into()),
        ("/Users/ada/Projects".into(), "/home/ada/Work".into()),
    ])
}

fn fate(name: &str, v: SettingValue) -> Fate {
    let m = map();
    classify(
        "Software\\Vendor\\App",
        name,
        &v,
        &Ctx {
            paths: &m,
            same_screens: false,
        },
    )
}

fn why(name: &str, v: SettingValue) -> Option<Why> {
    match fate(name, v) {
        Fate::NotMoved(w) => Some(w),
        _ => None,
    }
}

#[test]
fn names_split_into_words() {
    assert_eq!(words("MachineGuid"), ["machine", "guid"]);
    assert_eq!(words("last_opened-file"), ["last", "opened", "file"]);
    assert_eq!(words("editor.fontSize"), ["editor", "font", "size"]);
    assert_eq!(words("x"), ["x"]);
}

#[test]
fn secrets_by_name() {
    for n in [
        "Password",
        "db_passwd",
        "ApiKey",
        "api_key",
        "AccessToken",
        "authToken",
        "ClientSecret",
        "Cookies",
        "PrivateKey",
        "otp",
    ] {
        assert_eq!(why(n, t("x")), Some(Why::Secret), "{n}");
    }
    for n in ["Author", "Tokenizer", "Passage", "FontSize"] {
        assert_ne!(why(n, t("x")), Some(Why::Secret), "{n}");
    }
}

#[test]
fn secrets_by_form() {
    for v in [
        "AQAAANCMnd8BFdERjHoAwE/Cl+sBAAAA",
        "01000000D08C9DDF0115D1118C7A00C04FC297EB0100",
        "-----BEGIN OPENSSH PRIVATE KEY-----",
        "Bearer abc.def",
        "ghp_0123456789abcdefghijABCDEFGHIJ012345",
        "xoxb-1234567890-abcdefghijkl",
        "AKIAIOSFODNN7EXAMPLE",
        "sk-proj-abcdefghijklmnopqrstuvwx",
        "eyJhbGciOiJIUzI1NiJ9.eyJzdWIiOiIxIn0.c2lnbmF0dXJlLXZhbHVl",
        "aG93IG5vdyBicm93biBjb3cgaG93IG5vdyBicm93bi1jb3cxMjM0NTY3ODk=",
        "9f86d081884c7d659a2feaa0c55ad015a3bf4f1b2b0b822cd15d6c15b0f00a08",
    ] {
        assert_eq!(looks_secret(v), Some(Why::Secret), "{v}");
        assert_eq!(why("Value", t(v)), Some(Why::Secret), "{v}");
    }
    for v in [
        "ocean",
        "Consolas, 'Courier New', monospace",
        "en-GB",
        "1.25",
        "true",
        "a short thing with spaces in it that is long enough",
    ] {
        assert_eq!(looks_secret(v), None, "{v}");
    }
}

#[test]
fn identities_of_the_old_laptop_and_account() {
    assert_eq!(
        why("MachineGuid", t("3f2a9c1e-5b7d-4e8f-a1b2-c3d4e5f60718")),
        Some(Why::MachineId)
    );
    assert_eq!(why("DeviceId", SettingValue::Int(7)), Some(Why::MachineId));
    assert_eq!(why("installId", t("abc")), Some(Why::MachineId));
    assert_eq!(why("telemetry.machineId", t("abc")), Some(Why::MachineId));
    assert_eq!(why("uuid", t("abc")), Some(Why::MachineId));
    assert_eq!(why("UserSid", t("x")), Some(Why::AccountBound));
    assert_eq!(
        why("Owner", t("S-1-5-21-1111111111-2222222222-3333333333-1001")),
        Some(Why::AccountBound)
    );
    // An ID of a thing in the app (a selected profile) moves.
    assert_eq!(why("SelectedProfileId", t("work")), None);
}

#[test]
fn caches_window_places_and_devices() {
    assert_eq!(
        why("ThumbnailCache", SettingValue::Int(1)),
        Some(Why::Cache)
    );
    assert_eq!(why("WindowPlacement", t("2c00")), Some(Why::WindowPlace));
    assert_eq!(
        why("MainWindowLeft", SettingValue::Int(-1900)),
        Some(Why::WindowPlace)
    );
    assert_eq!(
        why("dialogWidth", SettingValue::Int(800)),
        Some(Why::WindowPlace)
    );
    assert_eq!(why("Geometry", t("x")), Some(Why::WindowPlace));
    assert_eq!(why("FontSize", SettingValue::Int(12)), None);
    let m = map();
    assert!(matches!(
        classify(
            "Software\\V\\A",
            "WindowPlacement",
            &t("2c00"),
            &Ctx {
                paths: &m,
                same_screens: true
            }
        ),
        Fate::Copy(_)
    ));
    assert_eq!(
        why("Microphone", t("USB\\VID_046D&PID_0825\\1234")),
        Some(Why::HardwarePath)
    );
    assert_eq!(why("Port", t("/dev/ttyUSB0")), Some(Why::HardwarePath));
    assert_eq!(
        why("Disk", t("\\\\?\\Volume{1234}\\")),
        Some(Why::HardwarePath)
    );
}

#[test]
fn paths_are_rewritten_or_left_with_the_reason() {
    assert_eq!(
        fate("LastFile", t("C:\\Users\\ada\\Documents\\x.txt")),
        Fate::Rewritten(t("C:\\Users\\ada2\\Documents\\x.txt"))
    );
    assert_eq!(
        fate("Folder", t("c:\\users\\ADA\\documents")),
        Fate::Rewritten(t("C:\\Users\\ada2\\Documents"))
    );
    assert_eq!(
        fate("Folder", t("C:\\Users\\ada\\DocumentsOld\\x")),
        Fate::NotMoved(Why::PathNotMoved)
    );
    // A Mac project folder onto Linux: separators follow the new laptop.
    assert_eq!(
        fate("Root", t("/Users/ada/Projects/site")),
        Fate::Rewritten(t("/home/ada/Work/site"))
    );
    assert_eq!(
        fate("Root", t("/home/ADA/Projects/site")),
        Fate::NotMoved(Why::PathNotMoved)
    );
    // file:// addresses, Windows and Unix.
    assert_eq!(
        fate("Workspace", t("file:///c%3A/Users/ada/Documents/proj")),
        Fate::Rewritten(t("file:///C%3A/Users/ada2/Documents/proj"))
    );
    assert_eq!(
        fate("Workspace", t("file:///home/ada/Projects/my%20site")),
        Fate::Rewritten(t("file:///home/ada/Work/my%20site"))
    );
    // A recent-files entry whose file did not move.
    assert_eq!(
        fate("RecentFile3", t("C:\\Temp\\x.txt")),
        Fate::NotMoved(Why::RecentMissing)
    );
    // A recent list keeps the entries that moved.
    let list =
        serde_json::to_string(&["C:\\Users\\ada\\Documents\\a.txt", "C:\\Gone\\b.txt"]).unwrap();
    assert_eq!(
        fate("MRUList", t(&list)),
        Fate::Rewritten(t(&serde_json::to_string(&[
            "C:\\Users\\ada2\\Documents\\a.txt"
        ])
        .unwrap()))
    );
    // A list holding a secret is not moved at all.
    let list =
        serde_json::to_string(&["fine", "ghp_0123456789abcdefghijABCDEFGHIJ012345"]).unwrap();
    assert_eq!(fate("Items", t(&list)), Fate::NotMoved(Why::Secret));
    // Environment-variable paths are portable as they are.
    assert_eq!(
        fate("Out", t("%USERPROFILE%\\Exports")),
        Fate::Copy(t("%USERPROFILE%\\Exports"))
    );
}

#[test]
fn programs_to_start_are_held_back() {
    assert_eq!(
        why("Editor", t("C:\\Tools\\edit.exe")),
        Some(Why::StartsProgram)
    );
    assert_eq!(
        why("Hook", t("\"C:\\Program Files\\X\\x.exe\" --run")),
        Some(Why::StartsProgram)
    );
    assert_eq!(
        why("Script", t("/home/ada/Projects/run.sh")),
        Some(Why::StartsProgram)
    );
}

#[test]
fn stores_by_kind() {
    assert_eq!(store_kind("Software\\Policies\\Vendor"), Some(Why::Policy));
    assert_eq!(
        store_kind("Software\\Vendor\\App\\Policies"),
        Some(Why::Policy)
    );
    assert_eq!(
        store_kind("Software\\Microsoft\\Windows\\CurrentVersion\\Run"),
        Some(Why::StartsProgram)
    );
    assert_eq!(
        store_kind("Software\\Microsoft\\Windows\\CurrentVersion\\RunOnce"),
        Some(Why::StartsProgram)
    );
    assert_eq!(
        store_kind("Software\\Classes\\.txt"),
        Some(Why::StartsProgram)
    );
    assert_eq!(
        store_kind("Software\\Vendor\\App\\shell\\open\\command"),
        Some(Why::StartsProgram)
    );
    assert_eq!(
        store_kind(
            "Software\\Microsoft\\Windows\\CurrentVersion\\CapabilityAccessManager\\ConsentStore"
        ),
        Some(Why::Security)
    );
    assert_eq!(
        store_kind("com.apple.security.revocation"),
        Some(Why::Security)
    );
    assert_eq!(
        store_kind("com.apple.loginwindow"),
        Some(Why::StartsProgram)
    );
    assert_eq!(
        store_kind("/org/gnome/desktop/privacy/"),
        Some(Why::Security)
    );
    assert_eq!(
        store_kind("/org/gnome/settings-daemon/plugins/media-keys/"),
        Some(Why::StartsProgram)
    );
    assert_eq!(store_kind("Software\\Vendor\\App\\Cache"), Some(Why::Cache));
    assert_eq!(store_kind("Software\\Vendor\\App"), None);
    assert_eq!(store_kind("Software\\Vendor\\Runner"), None);
    assert_eq!(store_kind("com.vendor.app"), None);
    assert_eq!(store_kind("/org/gnome/desktop/interface/"), None);
}

#[test]
fn binary_values_are_only_checked_for_known_credential_forms() {
    let m = map();
    let ctx = Ctx {
        paths: &m,
        same_screens: false,
    };
    let random = "9f86d081884c7d659a2feaa0c55ad015a3bf4f1b2b0b822cd15d6c15b0f00a08";
    assert_eq!(
        classify_kind("Software\\V\\A", "Colors", &t(random), true, &ctx),
        Fate::Copy(t(random))
    );
    assert_eq!(
        classify_kind(
            "Software\\V\\A",
            "Blob",
            &t("01000000d08c9ddf0115d1118c7a00c04fc297eb00"),
            true,
            &ctx
        ),
        Fate::NotMoved(Why::Secret)
    );
}

#[test]
fn hidden_characters_and_oversized_values_stay() {
    assert_eq!(why("Name", t("abc\u{202E}gpj.exe")), Some(Why::HiddenText));
    assert_eq!(why("Notes", t(&"a ".repeat(3000))), Some(Why::TooLarge));
}

#[test]
fn every_reason_has_plain_words() {
    for w in [
        Why::Security,
        Why::Secret,
        Why::MachineId,
        Why::OtherSystem,
        Why::Exception("x".into()),
    ] {
        assert!(!w.words().is_empty());
    }
}
