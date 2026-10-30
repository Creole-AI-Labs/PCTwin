//! The Mac app's Info.plist additions (src-tauri/Info.plist, which `tauri build` merges into the
//! bundle's own). Apple's TN3179, "Understanding local network privacy"
//! (https://developer.apple.com/documentation/technotes/tn3179-understanding-local-network-privacy):
//! `NSLocalNetworkUsageDescription` is the reason the system shows when it asks for local network
//! access, and `NSBonjourServices` lists each Bonjour service type the app announces or browses
//! for. Without them the question may come without PCTwin's reason, and browsing for an unlisted
//! service type can fail, so the laptops may not find each other. CI also reads the keys back from
//! the built PCTwin.app (ci.yml, "The Mac app declares the local network"). `CFBundleDocumentTypes`
//! (rank "Alternate") offers PCTwin in Open With for any file or folder without making it the
//! default app (Quick Send, handed.rs).

use std::path::PathBuf;

fn plist() -> String {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("Info.plist");
    std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("cannot read {}: {e}", path.display()))
        .replace("\r\n", "\n")
}

/// The XML element that follows `<key>name</key>`, whitespace trimmed.
fn value_of(text: &str, name: &str) -> String {
    let key = format!("<key>{name}</key>");
    let start = text
        .find(&key)
        .unwrap_or_else(|| panic!("Info.plist has no {name}"))
        + key.len();
    let rest = text[start..].trim_start();
    let tag = &rest[1..rest.find('>').expect("an element")];
    let close = format!("</{tag}>");
    let end = rest.find(&close).expect("the element closes") + close.len();
    rest[..end].to_owned()
}

/// The `<string>` values inside `element`.
fn strings(element: &str) -> Vec<String> {
    element
        .split("<string>")
        .skip(1)
        .map(|s| s.split("</string>").next().unwrap().trim().to_owned())
        .collect()
}

/// A service type as Info.plist names it: without the `.local.` domain.
fn bonjour_name(service_type: &str) -> &str {
    service_type
        .strip_suffix(".local.")
        .expect("a .local. service")
}

#[test]
fn the_mac_app_says_why_it_uses_the_local_network() {
    let text = plist();
    let why = value_of(&text, "NSLocalNetworkUsageDescription");
    let why = strings(&why);
    assert_eq!(why.len(), 1, "one plain sentence");
    let why = &why[0];
    assert!(why.starts_with("PCTwin "), "{why}");
    assert!(why.len() > 30 && why.len() < 200, "{why}");
}

#[test]
fn the_mac_app_names_every_service_it_announces_or_looks_for() {
    let text = plist();
    let services = strings(&value_of(&text, "NSBonjourServices"));
    assert!(value_of(&text, "NSBonjourServices").starts_with("<array>"));
    let want = [
        // Pairing: the old laptop announces it, the new laptop looks for it.
        bonjour_name(pctwin_discovery::SERVICE_TYPE),
        // The connection doctor's check that this laptop hears its own announcement.
        bonjour_name(pctwin_doctor::probe::CHECK_SERVICE),
    ];
    assert_eq!(services, want);
    assert_eq!(want, ["_pctwin._tcp", "_pctwin-check._tcp"]);
}

#[test]
fn the_plist_only_adds_keys_and_never_overrides_what_tauri_writes() {
    // Tauri writes the name, identifier and version from tauri.conf.json; a second copy here
    // could disagree with it.
    let text = plist();
    let keys: Vec<&str> = text
        .split("<key>")
        .skip(1)
        .map(|s| s.split("</key>").next().unwrap())
        .collect();
    assert_eq!(
        keys,
        [
            "NSLocalNetworkUsageDescription",
            "NSBonjourServices",
            "CFBundleDocumentTypes",
            "CFBundleTypeName",
            "CFBundleTypeRole",
            "LSHandlerRank",
            "LSItemContentTypes"
        ]
    );
}

#[test]
fn finders_open_with_offers_pctwin_but_pctwin_never_becomes_the_default() {
    // Quick Send's Mac entry point (handed.rs): Open With and a drop on the Dock icon. "Alternate"
    // lists PCTwin without taking over any file type; "Owner" or "Default" would make a double
    // click on someone's files open PCTwin.
    let text = plist();
    // Everything from the key on: the entry holds an array of its own, so `value_of` would stop early.
    let docs = &text[text
        .find("<key>CFBundleDocumentTypes</key>")
        .expect("declared")..];
    assert_eq!(strings(&value_of(docs, "LSHandlerRank")), ["Alternate"]);
    assert_eq!(strings(&value_of(docs, "CFBundleTypeRole")), ["Viewer"]);
    assert_eq!(
        strings(&value_of(docs, "LSItemContentTypes")),
        ["public.item", "public.folder"]
    );
    for never in ["Owner", "Default"] {
        assert!(
            !docs.contains(never),
            "{never} would make PCTwin the default app"
        );
    }
    assert_eq!(
        docs.matches("<dict>").count(),
        1,
        "one entry for everything"
    );
}
