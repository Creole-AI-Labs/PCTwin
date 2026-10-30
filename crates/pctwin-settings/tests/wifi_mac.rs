//! Wi-Fi on a Mac (2.4, 30 October 2026), against stand-ins for `networksetup`, `security` and
//! `open` that behave like them: a saved network's password comes only from the System keychain
//! through `security`'s own output (macOS's prompt guards it), never on any command line; a
//! network with no keychain item is never guessed; the new Mac gets one configuration profile
//! for every network, written owner-only and removed when PCTwin stops waiting; a network counts
//! as added only when `networksetup` lists it.

use std::cell::RefCell;
use std::path::PathBuf;
use std::rc::Rc;

use pctwin_settings::runner::{Output, Runner, SecretOutput, Tool};
use pctwin_settings::wifi::{
    AIRPORT_KIND, Network, PROFILES_PAGE, SYSTEM_KEYCHAIN, Secret, Security, WifiError, added_mac,
    export_mac, mac_preferred, mac_profile, mac_wifi_device, offer_mac, open_profiles_page,
};
use zeroize::Zeroizing;

const PORTS: &str = "\nHardware Port: Ethernet Adapter (en4)\nDevice: en4\nEthernet Address: aa:bb:cc:dd:ee:01\n\nHardware Port: Wi-Fi\nDevice: en0\nEthernet Address: aa:bb:cc:dd:ee:02\n\nHardware Port: Thunderbolt Bridge\nDevice: bridge0\nEthernet Address: N/A\n";

#[derive(Default, Clone)]
struct Mac {
    saved: Rc<RefCell<Vec<String>>>,
    /// SSID -> password in the System keychain.
    keychain: Rc<RefCell<Vec<(String, String)>>>,
    /// The person said no at macOS's prompt.
    refuses: bool,
    argv: Rc<RefCell<Vec<String>>>,
    /// The profile file as `open` was given it: its path and text.
    opened: Rc<RefCell<Vec<(PathBuf, String)>>>,
}

fn out(ok: bool, s: &str) -> Output {
    Output {
        ok,
        stdout: s.into(),
    }
}

impl Mac {
    fn find(&self, args: &[&str]) -> Option<String> {
        let at = args.iter().position(|a| *a == "-a")?;
        assert_eq!(args[0], "find-generic-password");
        assert_eq!(
            args[args.iter().position(|a| *a == "-D")? + 1],
            AIRPORT_KIND
        );
        assert_eq!(
            *args.last()?,
            SYSTEM_KEYCHAIN,
            "the System keychain, by its path"
        );
        let ssid = args[at + 1];
        self.keychain
            .borrow()
            .iter()
            .find(|(s, _)| s == ssid)
            .map(|(_, p)| p.clone())
    }
}

impl Runner for Mac {
    fn run(&mut self, tool: Tool, args: &[&str]) -> Result<Output, String> {
        self.argv.borrow_mut().push(args.join(" "));
        Ok(match (tool, args) {
            (Tool::Networksetup, ["-listallhardwareports"]) => out(true, PORTS),
            (Tool::Networksetup, ["-listpreferredwirelessnetworks", "en0"]) => {
                let list: String = self
                    .saved
                    .borrow()
                    .iter()
                    .map(|s| format!("\t{s}\n"))
                    .collect();
                out(true, &format!("Preferred networks on en0:\n{list}"))
            }
            (Tool::Security, _) => {
                assert!(
                    !args.contains(&"-w") && !args.contains(&"-g"),
                    "no password here"
                );
                out(
                    self.find(args).is_some(),
                    "keychain: \"/Library/Keychains/System.keychain\"\n",
                )
            }
            (Tool::Open, [page]) if *page == PROFILES_PAGE => out(true, ""),
            (Tool::Open, [file]) => {
                let path = PathBuf::from(file);
                let text = std::fs::read_to_string(&path).unwrap();
                self.opened.borrow_mut().push((path, text));
                out(true, "")
            }
            other => panic!("no stand-in for {other:?}"),
        })
    }

    fn run_secret(
        &mut self,
        tool: Tool,
        args: &[&str],
        input: Option<&[u8]>,
    ) -> Result<SecretOutput, String> {
        assert_eq!(tool, Tool::Security);
        assert!(input.is_none());
        assert!(args.contains(&"-w"));
        self.argv.borrow_mut().push(args.join(" "));
        let pw = if self.refuses { None } else { self.find(args) };
        Ok(SecretOutput {
            ok: pw.is_some(),
            stdout: Zeroizing::new(pw.map(|p| format!("{p}\n")).unwrap_or_default()),
        })
    }
}

fn mac(saved: &[&str], keys: &[(&str, &str)]) -> Mac {
    let m = Mac::default();
    *m.saved.borrow_mut() = saved.iter().map(|s| s.to_string()).collect();
    *m.keychain.borrow_mut() = keys
        .iter()
        .map(|(s, p)| (s.to_string(), p.to_string()))
        .collect();
    m
}

fn home(ssid: &str, pw: &str) -> Network {
    Network {
        name: ssid.into(),
        ssid: ssid.into(),
        hidden: false,
        auto_connect: true,
        security: Security::Personal {
            auth: "WPA2PSK".into(),
            encryption: "AES".into(),
        },
        secret: Some(Secret::new(Zeroizing::new(pw.into()))),
    }
}

#[test]
fn the_wifi_device_and_saved_networks_are_read_as_networksetup_prints_them() {
    assert_eq!(mac_wifi_device(PORTS).as_deref(), Some("en0"));
    assert_eq!(
        mac_wifi_device("Hardware Port: AirPort\nDevice: en1\n").as_deref(),
        Some("en1")
    );
    assert_eq!(
        mac_wifi_device("Hardware Port: Ethernet\nDevice: en0\n"),
        None
    );
    assert_eq!(
        mac_wifi_device("Hardware Port: Wi-Fi\nDevice: en0; rm\n"),
        None,
        "only a plain device name"
    );
    assert_eq!(
        mac_preferred("Preferred networks on en0:\n\tHome Net \n\tCafe\n\t\n"),
        ["Home Net ", "Cafe"]
    );
    assert!(mac_preferred("Error: en9 is not a Wi-Fi interface.\n").is_empty());
}

#[test]
fn passwords_come_only_from_the_keychain_through_securitys_output() {
    let m = mac(
        &["Home", "Cafe Open", "-dash"],
        &[("Home", "correct horse"), ("-dash", "p&ss<w>rd")],
    );
    let found = export_mac(&mut m.clone(), true).unwrap();
    let names: Vec<&str> = found.iter().map(|n| n.ssid.as_str()).collect();
    assert_eq!(
        names,
        ["Home", "-dash"],
        "a network with no keychain item is never guessed"
    );
    let pw: Vec<Vec<u8>> = found
        .iter()
        .map(|n| n.secret.as_ref().unwrap().to_session().to_vec())
        .collect();
    assert_eq!(pw, [b"correct horse".to_vec(), b"p&ss<w>rd".to_vec()]);
    for line in m.argv.borrow().iter() {
        assert!(
            !line.contains("correct horse") && !line.contains("p&ss"),
            "{line}"
        );
    }
    // Without keys: no password is asked for at all (no prompt).
    let m = mac(&["Home"], &[("Home", "x")]);
    let found = export_mac(&mut m.clone(), false).unwrap();
    assert!(found[0].secret.is_none());
    assert!(m.argv.borrow().iter().all(|l| !l.contains("-w")));
    // The person said no at macOS's prompt: the network moves without its password.
    let mut m = mac(&["Home"], &[("Home", "x")]);
    m.refuses = true;
    assert!(export_mac(&mut m, true).unwrap()[0].secret.is_none());
}

#[test]
fn one_profile_for_every_network_written_owner_only_and_gone_when_waiting_ends() {
    let parent = tempfile::tempdir().unwrap();
    let mut m = mac(&[], &[]);
    let a = home("Home & Garden", "pa<ss>\"word'");
    let open = Network {
        security: Security::Open,
        secret: None,
        hidden: true,
        auto_connect: false,
        ..home("Library", "")
    };
    let offered = offer_mac(&mut m, &folders(), parent.path(), &[&a, &open]).unwrap();
    let (path, text) = m.opened.borrow()[0].clone();
    assert_eq!(path, offered.file());
    // Apple's plist, one payload per network, every value escaped.
    let options = roxmltree::ParsingOptions {
        allow_dtd: true,
        ..Default::default()
    };
    let doc = roxmltree::Document::parse_with_options(&text, options).unwrap();
    let strings: Vec<&str> = doc
        .descendants()
        .filter(|n| n.has_tag_name("string"))
        .filter_map(|n| n.text())
        .collect();
    assert!(strings.contains(&"Home & Garden"));
    assert!(strings.contains(&"pa<ss>\"word'"));
    assert_eq!(
        strings
            .iter()
            .filter(|s| **s == "com.apple.wifi.managed")
            .count(),
        2
    );
    assert!(strings.contains(&"WPA") && strings.contains(&"None"));
    assert!(strings.contains(&"System"));
    // The password never went on a command line.
    assert!(m.argv.borrow().iter().all(|l| !l.contains("pa<ss>")));
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let dir = path.parent().unwrap();
        let mode = std::fs::metadata(dir).unwrap().permissions().mode();
        assert_eq!(mode & 0o077, 0, "only the person may open the folder");
    }
    // Not added until macOS lists it; then only that one.
    assert_eq!(added_mac(&mut m, &offered).unwrap(), [false, false]);
    m.saved.borrow_mut().push("Home & Garden".into());
    assert_eq!(added_mac(&mut m, &offered).unwrap(), [true, false]);
    open_profiles_page(&mut m).unwrap();
    drop(offered);
    assert!(
        !path.exists(),
        "the profile is removed when PCTwin stops waiting"
    );
    assert!(!path.parent().unwrap().exists());
}

#[test]
fn a_profile_is_never_made_for_what_cannot_go_into_it_safely() {
    let r = [7u8; 48];
    let enterprise = Network {
        security: Security::Enterprise,
        secret: None,
        ..home("Work", "")
    };
    assert_eq!(
        mac_profile(&[&enterprise], &r).err(),
        Some(WifiError::Enterprise)
    );
    assert_eq!(
        mac_profile(&[&home("Home", "line\nbreak")], &r).err(),
        Some(WifiError::Unreadable)
    );
    let no_pw = Network {
        secret: None,
        ..home("Home", "")
    };
    assert_eq!(
        mac_profile(&[&no_pw], &r).err(),
        Some(WifiError::Unreadable)
    );
    assert_eq!(
        mac_profile(&[&home(&"x".repeat(33), "pw")], &r).err(),
        Some(WifiError::BadName)
    );
    assert_eq!(
        mac_profile(&[&home("A\u{7}", "pw")], &r).err(),
        Some(WifiError::BadName)
    );
    assert_eq!(mac_profile(&[], &r).err(), Some(WifiError::Tool));
    // Too few random bytes for an identifier each.
    assert_eq!(
        mac_profile(&[&home("A", "pw"), &home("B", "pw")], &r[..32]).err(),
        Some(WifiError::Tool)
    );
    // Each payload has its own identifier, a version-4 UUID.
    let mut random = [0u8; 48];
    for (i, b) in random.iter_mut().enumerate() {
        *b = i as u8;
    }
    let text = mac_profile(&[&home("A", "pw"), &home("B", "pw")], &random).unwrap();
    let ids: Vec<&str> = text
        .split("<key>PayloadUUID</key><string>")
        .skip(1)
        .map(|s| s.split('<').next().unwrap())
        .collect();
    assert_eq!(ids.len(), 3);
    assert!(ids[0] != ids[1] && ids[1] != ids[2]);
    for id in ids {
        assert_eq!(id.len(), 36);
        assert_eq!(&id[14..15], "4", "{id}");
    }
}

#[cfg(unix)]
fn folders() -> pctwin_settings::wifi::UnixPrivateFolders {
    pctwin_settings::wifi::UnixPrivateFolders
}

/// Elsewhere (the Windows build of these tests) a folder made plainly: the owner-only check is
/// Windows' own there and is proven in tests/wifi.rs.
#[cfg(not(unix))]
fn folders() -> PlainFolders {
    PlainFolders
}

#[cfg(not(unix))]
struct PlainFolders;

#[cfg(not(unix))]
impl pctwin_settings::wifi::PrivateFolders for PlainFolders {
    fn make(&self, path: &std::path::Path) -> Result<(), pctwin_settings::wifi::FolderError> {
        std::fs::create_dir(path).map_err(|_| pctwin_settings::wifi::FolderError::Failed)
    }
    fn access_list(&self, _: &std::path::Path) -> Option<String> {
        Some("D:P(A;OICI;FA;;;S-1-5-21-1-2-3-1001)(A;OICI;FA;;;SY)".into())
    }
    fn me(&self) -> Option<String> {
        Some("S-1-5-21-1-2-3-1001".into())
    }
}
