//! Wi-Fi on Linux through NetworkManager (2.4 follow-up, 28 October 2026), against a stand-in
//! for `nmcli` that behaves like it: a saved network moves with its password, and the password is
//! never on any command line (only on `nmcli`'s standard input and output); an enterprise network
//! and a name that cannot go into a line safely are refused; a password NetworkManager did not
//! keep is never "added".

use std::cell::RefCell;
use std::collections::BTreeMap;
use std::rc::Rc;

use pctwin_settings::runner::{Output, Runner, SecretOutput, Tool};
use pctwin_settings::wifi::{Network, Secret, Security, WifiError, export_nm, import_nm};
use zeroize::Zeroizing;

#[derive(Default)]
struct Profile {
    ssid: String,
    hidden: bool,
    auto: bool,
    key_mgmt: String,
    psk: String,
}

#[derive(Default, Clone)]
struct Nm {
    profiles: Rc<RefCell<BTreeMap<String, Profile>>>,
    argv: Rc<RefCell<Vec<String>>>,
    /// The editor drops the password (NetworkManager did not keep it).
    loses_psk: bool,
}

fn out(ok: bool, s: &str) -> Output {
    Output {
        ok,
        stdout: s.into(),
    }
}

impl Nm {
    fn show(&self, args: &[&str]) -> (bool, String) {
        let p = self.profiles.borrow();
        let secrets = args.contains(&"--show-secrets");
        let Some(p) = args.last().and_then(|name| p.get(*name)) else {
            return (false, String::new());
        };
        let yes = |b: bool| if b { "yes" } else { "no" };
        let mut s = format!(
            "802-11-wireless.ssid:{}\n802-11-wireless.hidden:{}\nconnection.autoconnect:{}\n802-11-wireless-security.key-mgmt:{}\n",
            p.ssid,
            yes(p.hidden),
            yes(p.auto),
            p.key_mgmt
        );
        if args.iter().any(|a| a.contains("psk")) {
            s.push_str(&format!(
                "802-11-wireless-security.psk:{}\n",
                if secrets { p.psk.as_str() } else { "" }
            ));
        }
        (true, s)
    }
}

impl Runner for Nm {
    fn run(&mut self, tool: Tool, args: &[&str]) -> Result<Output, String> {
        assert_eq!(tool, Tool::Nmcli);
        self.argv.borrow_mut().push(args.join(" "));
        match args {
            [
                "--terse",
                "--escape",
                "no",
                "--fields",
                "TYPE,NAME",
                "connection",
                "show",
            ] => {
                let mut s = String::from("802-3-ethernet:Wired connection 1\n");
                for n in self.profiles.borrow().keys() {
                    s.push_str(&format!("802-11-wireless:{n}\n"));
                }
                Ok(out(true, &s))
            }
            [
                "connection",
                "add",
                "type",
                "wifi",
                "con-name",
                name,
                "ssid",
                ssid,
                rest @ ..,
            ] => {
                let get = |k: &str| {
                    rest.chunks(2)
                        .find(|c| c[0] == k)
                        .map(|c| c[1].to_string())
                        .unwrap_or_default()
                };
                self.profiles.borrow_mut().insert(
                    name.to_string(),
                    Profile {
                        ssid: ssid.to_string(),
                        hidden: get("802-11-wireless.hidden") == "yes",
                        auto: get("connection.autoconnect") == "yes",
                        key_mgmt: get("802-11-wireless-security.key-mgmt"),
                        psk: String::new(),
                    },
                );
                Ok(out(true, ""))
            }
            ["connection", "delete", "id", name] => {
                self.profiles.borrow_mut().remove(*name);
                Ok(out(true, ""))
            }
            _ => Ok(out(false, "")),
        }
    }

    fn run_secret(
        &mut self,
        tool: Tool,
        args: &[&str],
        input: Option<&[u8]>,
    ) -> Result<SecretOutput, String> {
        assert_eq!(tool, Tool::Nmcli);
        self.argv.borrow_mut().push(args.join(" "));
        let (ok, s) = match args {
            ["connection", "edit", "id", name] => {
                let script = std::str::from_utf8(input.unwrap()).unwrap().to_string();
                let mut all = self.profiles.borrow_mut();
                match all.get_mut(*name) {
                    None => (false, String::new()),
                    Some(p) => {
                        let mut saved = false;
                        for line in script.lines() {
                            if let Some(v) = line.strip_prefix("set 802-11-wireless-security.psk ")
                            {
                                if !self.loses_psk {
                                    p.psk = v.to_string();
                                }
                            } else if line == "save yes" {
                                saved = true;
                            }
                        }
                        (saved, String::new())
                    }
                }
            }
            [.., "connection", "show", "id", _] => self.show(args),
            _ => (false, String::new()),
        };
        Ok(SecretOutput {
            ok,
            stdout: Zeroizing::new(s),
        })
    }
}

fn home(name: &str, psk: &str) -> Network {
    Network {
        name: name.into(),
        ssid: format!("{name} ssid"),
        hidden: false,
        auto_connect: true,
        security: Security::Personal {
            auth: "WPA2PSK".into(),
            encryption: "AES".into(),
        },
        secret: Some(Secret::new(Zeroizing::new(psk.into()))),
    }
}

const PSK: &str = "correct horse battery";

#[test]
fn a_network_moves_with_its_password_and_the_password_is_never_on_a_command_line() {
    let old = Nm::default();
    old.profiles.borrow_mut().insert(
        "Home".into(),
        Profile {
            ssid: "Home Net".into(),
            hidden: true,
            auto: true,
            key_mgmt: "sae".into(),
            psk: PSK.into(),
        },
    );
    old.profiles.borrow_mut().insert(
        "Work".into(),
        Profile {
            ssid: "Corp".into(),
            key_mgmt: "wpa-eap".into(),
            auto: true,
            ..Default::default()
        },
    );
    let found = export_nm(&mut old.clone(), true).unwrap();
    assert_eq!(found.len(), 2, "only Wi-Fi, never the wired connection");
    let home_net = found.iter().find(|n| n.name == "Home").unwrap();
    assert_eq!(home_net.ssid, "Home Net");
    assert!(home_net.hidden);
    assert!(home_net.secret.is_some());
    assert!(
        !format!("{home_net:?}").contains(PSK),
        "a password is never printed"
    );
    let work = found.iter().find(|n| n.name == "Work").unwrap();
    assert_eq!(work.security, Security::Enterprise);
    assert!(work.secret.is_none());

    let new = Nm::default();
    import_nm(&mut new.clone(), home_net).unwrap();
    assert_eq!(new.profiles.borrow()["Home"].psk, PSK);
    assert_eq!(new.profiles.borrow()["Home"].key_mgmt, "sae");
    assert_eq!(
        import_nm(&mut new.clone(), work),
        Err(WifiError::Enterprise)
    );
    for argv in old.argv.borrow().iter().chain(new.argv.borrow().iter()) {
        assert!(!argv.contains(PSK), "a password on a command line: {argv}");
    }
}

#[test]
fn a_password_not_kept_is_never_added_and_nothing_half_made_is_left() {
    let new = Nm {
        loses_psk: true,
        ..Default::default()
    };
    assert_eq!(
        import_nm(&mut new.clone(), &home("Cafe", PSK)),
        Err(WifiError::NotThere)
    );
    // A name that would break the editor's line, or read as an option, is refused before any
    // tool runs.
    let fresh = Nm::default();
    for bad in ["two\nlines", "-x", " padded "] {
        assert_eq!(
            import_nm(&mut fresh.clone(), &home(bad, PSK)),
            Err(WifiError::BadName)
        );
    }
    assert!(fresh.argv.borrow().is_empty());
    // A password the editor cannot take on one line is not tried.
    assert_eq!(
        import_nm(&mut fresh.clone(), &home("Cafe", "pass\nsave yes")),
        Err(WifiError::Unreadable)
    );
    assert!(fresh.profiles.borrow().is_empty());
}

/// The real NetworkManager (CI's Linux runner, which installs and starts it for this test, as
/// root so its permission check allows the change): a network added with its password through
/// the editor's standard input is there, with that very password, then removed.
#[test]
#[ignore = "needs a running NetworkManager (CI sets PCTWIN_NM_REAL=1)"]
fn the_real_network_manager_keeps_a_password_given_only_on_standard_input() {
    if std::env::var_os("PCTWIN_NM_REAL").is_none() {
        return;
    }
    let mut runner = pctwin_settings::runner::SystemRunner;
    let name = format!("PCTwin lab {}", std::process::id());
    let mut n = home(&name, PSK);
    n.ssid = "PCTwin Lab Net".into();
    import_nm(&mut runner, &n).expect("added, with its password, and read back");
    let found = export_nm(&mut runner, true).unwrap();
    assert!(found.iter().any(|x| x.name == name && x.secret.is_some()));
    let gone = runner
        .run(Tool::Nmcli, &["connection", "delete", "id", &name])
        .unwrap();
    assert!(gone.ok);
}
