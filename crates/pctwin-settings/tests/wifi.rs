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
    let nets = export_all(&mut netsh, tmp.path(), true).unwrap();
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
    let nets = export_all(&mut old, tmp.path(), true).unwrap();
    let home = nets.into_iter().find(|n| n.ssid == "Example-Home").unwrap();
    let mut new = Netsh::default();
    import(&mut new, tmp.path(), &home).unwrap();
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
    assert!(import(&mut Forgets, tmp.path(), &n).is_err());
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
