//! Saved Wi-Fi networks (Task List 2.4, decided 20 October 2026; T24-015). The networks and
//! passwords here are made up; no real profile on this laptop is read or changed.

use std::cell::RefCell;
use std::path::{Path, PathBuf};
use std::rc::Rc;

use pctwin_settings::runner::{Output, Runner, Tool};
use pctwin_settings::wifi::{
    ASK_ADMIN, FOLDER_PREFIX, Network, Secret, Security, WifiError, export_all, import, offer,
    parse_profile, profile_xml, sweep_leftovers,
};
use zeroize::Zeroizing;

/// The private folders, made as the app makes them.
#[cfg(windows)]
struct HelperFolders;

#[cfg(windows)]
impl pctwin_settings::wifi::PrivateFolders for HelperFolders {
    fn make(&self, path: &Path) -> Result<(), pctwin_settings::wifi::FolderError> {
        pctwin_helper_os::private_folder(path).map_err(|e| match e {
            pctwin_helper_os::Error::NameTaken => pctwin_settings::wifi::FolderError::NameTaken,
            _ => pctwin_settings::wifi::FolderError::Failed,
        })
    }
    fn access_list(&self, path: &Path) -> Option<String> {
        pctwin_helper_os::folder_access_list(path).ok()
    }
    fn me(&self) -> Option<String> {
        match pctwin_helper_os::current_account().ok()? {
            pctwin_helper_os::Account::Sid(s) => Some(s.as_str().to_string()),
            _ => None,
        }
    }
}

fn folders() -> &'static dyn pctwin_settings::wifi::PrivateFolders {
    #[cfg(windows)]
    return &HelperFolders;
    #[cfg(unix)]
    return &pctwin_settings::wifi::UnixPrivateFolders;
}

const NS: &str = "http://www.microsoft.com/networking/WLAN/profile/v1";

fn home_profile(name: &str, ssid: &str, key: Option<&str>) -> String {
    let shared = key.map(|k| format!(
        "<sharedKey><keyType>passPhrase</keyType><protected>false</protected><keyMaterial>{k}</keyMaterial></sharedKey>"
    )).unwrap_or_default();
    format!(
        "\u{feff}<?xml version=\"1.0\"?>\n<WLANProfile xmlns=\"{NS}\"><name>{name}</name><SSIDConfig><SSID><hex>00</hex><name>{ssid}</name></SSID></SSIDConfig><connectionType>ESS</connectionType><connectionMode>auto</connectionMode><MSM><security><authEncryption><authentication>WPA2PSK</authentication><encryption>AES</encryption><useOneX>false</useOneX></authEncryption>{shared}</security></MSM></WLANProfile>"
    )
}

fn work_profile() -> String {
    format!(
        "<WLANProfile xmlns=\"{NS}\"><name>Example-Office</name><SSIDConfig><SSID><name>Example-Office</name></SSID></SSIDConfig><connectionMode>auto</connectionMode><MSM><security><authEncryption><authentication>WPA2</authentication><encryption>AES</encryption><useOneX>true</useOneX></authEncryption></security></MSM></WLANProfile>"
    )
}

#[test]
fn the_offer_follows_the_20_october_decision() {
    assert!(offer(true).ticked);
    assert_eq!(offer(true).note, None);
    assert!(!offer(false).ticked);
    assert_eq!(offer(false).note, Some(ASK_ADMIN));
}

#[test]
fn a_password_network_is_read_and_its_secret_never_printed() {
    let n = parse_profile(
        &home_profile(
            "Example-Home",
            "Example-Home",
            Some("correct-horse-example"),
        )
        .replace('\u{feff}', ""),
    )
    .unwrap();
    assert_eq!(n.ssid, "Example-Home");
    assert!(n.auto_connect && !n.hidden);
    assert_eq!(
        n.security,
        Security::Personal {
            auth: "WPA2PSK".into(),
            encryption: "AES".into()
        }
    );
    let shown = format!("{n:?}");
    assert!(!shown.contains("correct-horse-example"), "{shown}");
    assert!(shown.contains("Secret(hidden)"));
    assert_eq!(
        &*n.secret.as_ref().unwrap().to_session(),
        b"correct-horse-example"
    );
}

#[test]
fn t24_015_an_enterprise_network_moves_without_any_secret_as_a_separate_step() {
    let n = parse_profile(&work_profile()).unwrap();
    assert_eq!(n.security, Security::Enterprise);
    assert!(n.secret.is_none());
    assert_eq!(profile_xml(&n).unwrap_err(), WifiError::Enterprise);
}

#[test]
fn a_profile_is_written_back_with_names_escaped() {
    let n = Network {
        name: "Café <&> \"5G\"".into(),
        ssid: "Café <&> \"5G\"".into(),
        hidden: true,
        auto_connect: false,
        security: Security::Personal {
            auth: "WPA3SAE".into(),
            encryption: "AES".into(),
        },
        secret: Secret::from_session(Zeroizing::new(b"p&ss<word>".to_vec())),
    };
    let xml = profile_xml(&n).unwrap();
    let back = parse_profile(&xml).unwrap();
    assert_eq!(back.ssid, n.ssid);
    assert!(back.hidden && !back.auto_connect);
    assert_eq!(&*back.secret.unwrap().to_session(), b"p&ss<word>");
    // Hostile or impossible names are refused.
    let mut bad = n;
    bad.ssid = "x\ny".into();
    assert_eq!(profile_xml(&bad).unwrap_err(), WifiError::BadName);
    bad.ssid = "a".repeat(33);
    assert_eq!(profile_xml(&bad).unwrap_err(), WifiError::BadName);
    assert!(Secret::from_session(Zeroizing::new(vec![b'a'; 65])).is_none());
    assert!(parse_profile("<WLANProfile><name>x</name></WLANProfile>").is_err());
}

/// A saved network: name, SSID, password.
type Saved = (String, String, Option<String>);

/// `netsh` stand-in: writes the saved profiles into the folder it is given, and keeps added ones.
#[derive(Clone, Default)]
struct Netsh {
    saved: Rc<RefCell<Vec<Saved>>>,
    /// Every file `add profile` was given, to check it is gone afterwards.
    given: Rc<RefCell<Vec<PathBuf>>>,
    folders: Rc<RefCell<Vec<PathBuf>>>,
}

fn quoted(arg: &str, prefix: &str) -> Option<PathBuf> {
    arg.strip_prefix(prefix)?
        .strip_prefix('"')?
        .strip_suffix('"')
        .map(PathBuf::from)
}

impl Runner for Netsh {
    fn run(&mut self, tool: Tool, args: &[&str]) -> Result<Output, String> {
        assert_eq!(tool, Tool::Netsh);
        match args {
            ["wlan", "export", "profile", rest @ ..] => {
                let keys = rest.contains(&"key=clear");
                let folder = rest.iter().find_map(|a| quoted(a, "folder=")).unwrap();
                self.folders.borrow_mut().push(folder.clone());
                for (i, (name, ssid, key)) in self.saved.borrow().iter().enumerate() {
                    let k = if keys { key.as_deref() } else { None };
                    std::fs::write(
                        folder.join(format!("Wi-Fi-{i}.xml")),
                        home_profile(name, ssid, k),
                    )
                    .unwrap();
                }
                std::fs::write(folder.join("Wi-Fi-work.xml"), work_profile()).unwrap();
                Ok(Output {
                    ok: true,
                    stdout: String::new(),
                })
            }
            ["wlan", "add", "profile", file, "user=current"] => {
                let path = quoted(file, "filename=").unwrap();
                let n = parse_profile(&std::fs::read_to_string(&path).unwrap()).unwrap();
                self.given.borrow_mut().push(path);
                let key = n
                    .secret
                    .map(|s| String::from_utf8(s.to_session().to_vec()).unwrap());
                self.saved.borrow_mut().push((n.name, n.ssid, key));
                Ok(Output {
                    ok: true,
                    stdout: String::new(),
                })
            }
            _ => Ok(Output {
                ok: false,
                stdout: String::new(),
            }),
        }
    }
}

fn leftovers(dir: &Path) -> Vec<String> {
    std::fs::read_dir(dir)
        .unwrap()
        .flatten()
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .collect()
}

#[test]
fn passwords_are_read_into_memory_and_never_left_on_disk() {
    let tmp = tempfile::tempdir().unwrap();
    let mut netsh = Netsh::default();
    netsh.saved.borrow_mut().push((
        "Example-Home".into(),
        "Example-Home".into(),
        Some("correct-horse-example".into()),
    ));
    let nets = export_all(&mut netsh, folders(), tmp.path(), true).unwrap();
    assert_eq!(nets.len(), 2);
    let home = nets.iter().find(|n| n.ssid == "Example-Home").unwrap();
    assert_eq!(
        &*home.secret.as_ref().unwrap().to_session(),
        b"correct-horse-example"
    );
    // The tool's folder and every file in it are gone.
    assert!(
        leftovers(tmp.path()).is_empty(),
        "{:?}",
        leftovers(tmp.path())
    );
    let folder = netsh.folders.borrow()[0].clone();
    assert!(
        folder
            .file_name()
            .unwrap()
            .to_string_lossy()
            .starts_with(FOLDER_PREFIX)
    );
    assert!(!folder.exists());
}

#[test]
fn a_network_is_added_from_a_file_deleted_at_once_and_read_back() {
    let tmp = tempfile::tempdir().unwrap();
    let mut old = Netsh::default();
    old.saved.borrow_mut().push((
        "Example-Home".into(),
        "Example-Home".into(),
        Some("correct-horse-example".into()),
    ));
    let nets = export_all(&mut old, folders(), tmp.path(), true).unwrap();
    let home = nets.into_iter().find(|n| n.ssid == "Example-Home").unwrap();
    let mut new = Netsh::default();
    import(&mut new, folders(), tmp.path(), &home).unwrap();
    assert_eq!(
        new.saved.borrow()[0].2.as_deref(),
        Some("correct-horse-example")
    );
    assert!(new.given.borrow().iter().all(|p| !p.exists()));
    assert!(leftovers(tmp.path()).is_empty());
}

#[test]
fn a_network_the_tool_did_not_keep_is_not_reported_added() {
    struct Forgets;
    impl Runner for Forgets {
        fn run(&mut self, _: Tool, args: &[&str]) -> Result<Output, String> {
            Ok(Output {
                ok: args.get(1) != Some(&"export"),
                stdout: String::new(),
            })
        }
    }
    let tmp = tempfile::tempdir().unwrap();
    let n = parse_profile(
        &home_profile("Example-Home", "Example-Home", Some("x-example")).replace('\u{feff}', ""),
    )
    .unwrap();
    assert!(import(&mut Forgets, folders(), tmp.path(), &n).is_err());
    assert!(leftovers(tmp.path()).is_empty());
}

#[test]
fn folders_a_crash_left_are_swept_at_start() {
    let tmp = tempfile::tempdir().unwrap();
    std::fs::create_dir(tmp.path().join(format!("{FOLDER_PREFIX}abc"))).unwrap();
    std::fs::write(
        tmp.path()
            .join(format!("{FOLDER_PREFIX}abc"))
            .join("Wi-Fi-x.xml"),
        "secret",
    )
    .unwrap();
    std::fs::create_dir(tmp.path().join("someone-elses")).unwrap();
    assert_eq!(sweep_leftovers(tmp.path()), 1);
    assert_eq!(leftovers(tmp.path()), vec!["someone-elses".to_string()]);
}

#[cfg(windows)]
#[test]
fn the_real_tool_runs_with_its_folder_argument_and_leaves_nothing() {
    // Exports nothing: no password is asked for (no `key=clear`) and no profile is named that
    // could exist, so this laptop's networks are not read; it proves the tool, its argument and
    // the clean-up on this laptop.
    let tmp = tempfile::tempdir().unwrap();
    let mut runner = pctwin_settings::runner::SystemRunner;
    let out = runner
        .run(
            Tool::Netsh,
            &[
                "wlan",
                "export",
                "profile",
                "name=\"PCTwin-test-no-such-network\"",
                &format!("folder=\"{}\"", tmp.path().display()),
            ],
        )
        .unwrap();
    let _ = out.ok;
    assert!(leftovers(tmp.path()).is_empty());
}

#[test]
fn the_written_profile_is_exactly_the_systems_shape() {
    let n = parse_profile(
        &home_profile(
            "Example-Home",
            "Example-Home",
            Some("correct-horse-example"),
        )
        .replace('\u{feff}', ""),
    )
    .unwrap();
    let xml = profile_xml(&n).unwrap();
    assert!(xml.contains("<connectionType>ESS</connectionType>"));
    assert!(xml.contains("</security></MSM></WLANProfile>"));
    // Names up to 255 characters; an SSID up to 32 bytes.
    let mut long =
        parse_profile(&home_profile("x", "Example", Some("k-example")).replace('\u{feff}', ""))
            .unwrap();
    long.name = "n".repeat(255);
    assert!(profile_xml(&long).is_ok());
    long.name = "n".repeat(256);
    assert_eq!(profile_xml(&long).unwrap_err(), WifiError::BadName);
    // An "open" network with a cipher is not an open network.
    let wep = format!(
        "<WLANProfile xmlns=\"{NS}\"><name>Old</name><SSIDConfig><SSID><name>Old</name></SSID></SSIDConfig><MSM><security><authEncryption><authentication>open</authentication><encryption>WEP</encryption><useOneX>false</useOneX></authEncryption></security></MSM></WLANProfile>"
    );
    assert!(matches!(
        parse_profile(&wep).unwrap().security,
        Security::Other(_)
    ));
}

#[test]
fn an_oversized_profile_file_is_not_read() {
    struct Big;
    impl Runner for Big {
        fn run(&mut self, _: Tool, args: &[&str]) -> Result<Output, String> {
            let folder = args.iter().find_map(|a| quoted(a, "folder=")).unwrap();
            let padded = format!(
                "{}{}",
                home_profile("Example-Home", "Example-Home", Some("k-example")),
                " ".repeat(70 * 1024)
            );
            std::fs::write(folder.join("big.xml"), padded).unwrap();
            Ok(Output {
                ok: true,
                stdout: String::new(),
            })
        }
    }
    let tmp = tempfile::tempdir().unwrap();
    assert!(
        export_all(&mut Big, folders(), tmp.path(), true)
            .unwrap()
            .is_empty()
    );
    assert!(leftovers(tmp.path()).is_empty());
}

/// Fourth review, finding F3: the folder the password is written into in plain text inherited
/// its parent's access. It is now owner-only and protected from its first moment, read back.
#[test]
fn f3_the_wifi_folder_is_owner_only_from_its_first_moment() {
    struct Acl(Vec<String>);
    impl Runner for Acl {
        fn run(&mut self, _t: Tool, args: &[&str]) -> Result<Output, String> {
            for a in args {
                if let Some(p) = a
                    .strip_prefix("folder=")
                    .or_else(|| a.strip_prefix("filename="))
                {
                    let p = Path::new(p.trim_matches('"'));
                    let dir = if a.starts_with("filename=") {
                        p.parent().unwrap().to_path_buf()
                    } else {
                        p.to_path_buf()
                    };
                    #[cfg(windows)]
                    self.0
                        .push(pctwin_helper_os::folder_access_list(&dir).unwrap());
                    #[cfg(unix)]
                    {
                        use std::os::unix::fs::PermissionsExt;
                        let m = std::fs::metadata(&dir).unwrap().permissions().mode();
                        self.0.push(format!("{:o}", m & 0o777));
                    }
                }
            }
            Ok(Output {
                ok: true,
                stdout: String::new(),
            })
        }
    }
    let parent = tempfile::tempdir().unwrap();
    let n = Network {
        name: "Example-Home".into(),
        ssid: "Example-Home".into(),
        hidden: false,
        auto_connect: true,
        security: Security::Personal {
            auth: "WPA2PSK".into(),
            encryption: "AES".into(),
        },
        secret: Some(Secret::new(Zeroizing::new("correct-horse-example".into()))),
    };
    let mut r = Acl(Vec::new());
    let _ = import(&mut r, folders(), parent.path(), &n);
    assert!(r.0.len() >= 2, "{:?}", r.0);
    for acl in &r.0 {
        #[cfg(windows)]
        {
            let me = folders().me().unwrap();
            assert!(pctwin_settings::wifi::private_sddl(acl, &me), "{acl}");
            assert!(acl.starts_with("D:P"), "{acl}");
            assert!(!acl.contains(";ID;") && !acl.contains("ID;;"), "{acl}");
        }
        #[cfg(unix)]
        assert_eq!(acl, "700");
    }
}

#[test]
fn f3_an_inherited_or_shared_access_list_is_not_private() {
    use pctwin_settings::wifi::private_sddl;
    let me = "S-1-5-21-1-2-3-1001";
    // Exactly what the folder is made with: the person and the system, protected.
    assert!(private_sddl(
        &format!("D:P(A;OICI;FA;;;{me})(A;OICI;FA;;;SY)"),
        me
    ));
    assert!(private_sddl(
        &format!("D:PAI(A;OICI;FA;;;SY)(A;OICI;FA;;;{me})"),
        me
    ));
    // Not protected, inherited, missing the system, or a second account or group.
    assert!(!private_sddl(&format!("D:PAI(A;OICI;FA;;;{me})"), me));
    assert!(!private_sddl(
        &format!("D:(A;OICI;FA;;;{me})(A;OICI;FA;;;SY)"),
        me
    ));
    assert!(!private_sddl(
        &format!("D:P(A;OICIID;FA;;;{me})(A;OICI;FA;;;SY)"),
        me
    ));
    assert!(!private_sddl(
        &format!("D:P(A;OICI;FA;;;{me})(A;OICI;FA;;;SY)(A;OICI;FA;;;BA)"),
        me
    ));
    assert!(!private_sddl(
        &format!("D:P(A;OICI;FA;;;{me})(A;OICI;FA;;;SY)(A;;FR;;;WD)"),
        me
    ));
    assert!(!private_sddl("D:P(D;;FA;;;WD)", me));
    assert!(!private_sddl("D:P", me));
    assert!(!private_sddl("O:SY", me));
    assert!(!private_sddl(
        &format!("D:P(A;OICI;FA;;;{me})(A;OICI;FA;;;SY)"),
        ""
    ));
    // Fifth review, finding 8: another single account is never the person, whoever it is.
    for other in ["WD", "BU", "AU", "BA", "S-1-5-21-1-2-3-1002"] {
        for s in [
            format!("D:P(A;OICI;FA;;;{other})"),
            format!("D:PAI(A;OICI;FA;;;{other})(A;OICI;FA;;;SY)"),
        ] {
            assert!(!private_sddl(&s, me), "{s}");
        }
    }
    // Less than full access for the person is not what the folder is made with either.
    assert!(!private_sddl(
        &format!("D:P(A;OICI;FR;;;{me})(A;OICI;FA;;;SY)"),
        me
    ));
}

/// If the folder cannot be made private (or does not read back private), nothing goes on: the
/// system's Wi-Fi tool is never asked to write the password anywhere.
#[test]
fn f3_no_private_folder_means_the_tool_is_never_run() {
    use pctwin_settings::wifi::{FolderError, PrivateFolders};
    struct Counts(usize);
    impl Runner for Counts {
        fn run(&mut self, _t: Tool, _a: &[&str]) -> Result<Output, String> {
            self.0 += 1;
            Ok(Output {
                ok: true,
                stdout: String::new(),
            })
        }
    }
    struct Fails;
    impl PrivateFolders for Fails {
        fn make(&self, _: &Path) -> Result<(), FolderError> {
            Err(FolderError::Failed)
        }
        fn access_list(&self, _: &Path) -> Option<String> {
            None
        }
        fn me(&self) -> Option<String> {
            None
        }
    }
    let parent = tempfile::tempdir().unwrap();
    let mut r = Counts(0);
    assert!(export_all(&mut r, &Fails, parent.path(), true).is_err());
    assert_eq!(r.0, 0);
    // Made, but it reads back as sharing its parent's access (Windows).
    #[cfg(windows)]
    {
        struct Inherits;
        impl PrivateFolders for Inherits {
            fn make(&self, path: &Path) -> Result<(), FolderError> {
                std::fs::create_dir(path).map_err(|_| FolderError::Failed)
            }
            fn access_list(&self, _: &Path) -> Option<String> {
                Some("D:AI(A;OICIID;FA;;;S-1-5-21-1-2-3-1001)(A;OICIID;FA;;;BA)".into())
            }
            fn me(&self) -> Option<String> {
                Some("S-1-5-21-1-2-3-1001".into())
            }
        }
        assert!(export_all(&mut r, &Inherits, parent.path(), true).is_err());
        assert_eq!(r.0, 0);
        // And the folder it made is gone again.
        assert_eq!(std::fs::read_dir(parent.path()).unwrap().count(), 0);
    }
}
