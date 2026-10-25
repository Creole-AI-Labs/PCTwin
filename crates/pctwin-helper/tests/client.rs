//! The app's side of the new laptop's conversation with the helper: an account made, a person's
//! files held and handed over, now or at their sign-in; a refusal comes back as one, and nothing
//! for a person not approved.

use std::collections::BTreeSet;
use std::net::{TcpListener, TcpStream};
use std::path::{Path, PathBuf};

use pctwin_helper::account::Made;
use pctwin_helper::newlaptop::{Counted, NewLaptop};
use pctwin_helper::{Client, ClientError, HandedOver, Refusal, Session};

struct Fake;

impl NewLaptop for Fake {
    fn area(&mut self) -> Result<PathBuf, Refusal> {
        Ok(PathBuf::from("/area"))
    }
    fn person_folder(
        &mut self,
        area: &Path,
        key: &str,
        _person: &pctwin_helper_os::Account,
    ) -> Result<PathBuf, Refusal> {
        Ok(area.join(key))
    }
    fn hand_over_at_sign_in(
        &mut self,
        _person: &pctwin_helper_os::Account,
        _name: &str,
        _folder: &Path,
        _key: &str,
    ) -> Result<(), Refusal> {
        Ok(())
    }
    fn hand_over(
        &mut self,
        _account: &pctwin_helper_os::Account,
        _name: &str,
        _folder: &Path,
    ) -> Result<Counted, Refusal> {
        Ok(Counted {
            landed: 2,
            kept: 1,
            failed: 0,
        })
    }
}

#[test]
fn the_new_laptop_s_steps_through_the_client() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = listener.local_addr().unwrap();
    let helper = std::thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        // A Mac privacy setting kept the helper out of two of Bea's places.
        let mut scanner = |_: &str| -> Result<pctwin_helper::person::PersonScan, Refusal> {
            Ok(pctwin_helper::person::PersonScan {
                found: Vec::new(),
                privacy_blocked: 2,
                system_only: vec![std::path::PathBuf::from(r"C:\pctwin-lab\locked")],
            })
        };
        let mut maker = |name: &str, _full: &str| -> Result<Made, Refusal> {
            assert_eq!(name, "bea");
            Ok(Made {
                account: "1005".into(),
                password: zeroize::Zeroizing::new("abcd-efgh-jkmn-pqrs".into()),
                secure_token: false,
            })
        };
        let mut fake = Fake;
        Session::new(
            "tok".into(),
            BTreeSet::from(["bea".to_string()]),
            &mut scanner,
        )
        .with_account_maker(&mut maker)
        .with_new_laptop(&mut fake)
        .serve(&mut stream)
        .unwrap();
    });
    let mut client = Client::open(TcpStream::connect(addr).unwrap(), "tok").unwrap();
    // Nothing for a person not approved.
    assert!(matches!(
        client.create_account("carl", "Carl"),
        Err(ClientError::Refused(Refusal::NotApproved))
    ));
    // Before the account is made there is no one to hand over to.
    assert!(matches!(
        client.hand_over_now("bea"),
        Err(ClientError::Refused(Refusal::NotApproved))
    ));
    // The app hears it, to show how to give the helper Full Disk Access.
    let scanned = client.scan_person("bea").unwrap();
    assert_eq!((scanned.files, scanned.privacy_blocked), (0, 2));
    // Which of the places the mover could not read only the administrators and the system may
    // open: the mover asks about its own, the helper answers yes or no for each (no path ever
    // comes from the helper). A place it did not find so, or one in a home, is a no.
    let asked = [
        std::path::PathBuf::from(r"C:\pctwin-lab\locked"),
        std::path::PathBuf::from(r"C:\pctwin-lab\bea-only"),
        std::path::PathBuf::from(r"C:\Users\bea\Documents"),
    ];
    assert_eq!(
        client.system_only(&asked).unwrap(),
        vec![true, false, false]
    );
    // Never more at once than the helper answers.
    let many = vec![std::path::PathBuf::from(r"C:\x"); pctwin_helper::MAX_SYSTEM_ASK + 1];
    assert!(matches!(
        client.system_only(&many),
        Err(ClientError::Refused(Refusal::TooMuch))
    ));
    let made = client.create_account("bea", "Bea").unwrap();
    assert_eq!(made.account, "1005");
    assert_eq!(made.password.as_str(), "abcd-efgh-jkmn-pqrs");
    assert!(!made.secure_token, "said as the helper said it");
    assert!(matches!(
        client.hand_over_now("bea"),
        Err(ClientError::Refused(Refusal::NothingHeld))
    ));
    let held = client.hold_for("bea").unwrap();
    assert_eq!(held, Path::new("/area").join("p1"));
    assert_eq!(
        client.hand_over_now("bea").unwrap(),
        HandedOver {
            landed: 2,
            kept: 1,
            failed: 0
        }
    );
    client.hand_over_at_sign_in("bea").unwrap();
    client.bye().unwrap();
    helper.join().unwrap();
}

/// Several approved people: what each person's scan found only the administrators and the
/// system may open is kept together, so every one of those places is answered yes, whichever
/// scan found it.
#[test]
fn every_scan_s_system_only_places_are_kept() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = listener.local_addr().unwrap();
    let helper = std::thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        let mut scanner = |who: &str| -> Result<pctwin_helper::person::PersonScan, Refusal> {
            Ok(pctwin_helper::person::PersonScan {
                found: Vec::new(),
                privacy_blocked: 0,
                system_only: vec![PathBuf::from(format!("/lab/locked-{who}"))],
            })
        };
        Session::new(
            "tok".into(),
            BTreeSet::from(["bea".to_string(), "cal".to_string()]),
            &mut scanner,
        )
        .serve(&mut stream)
        .unwrap();
    });
    let mut client = Client::open(TcpStream::connect(addr).unwrap(), "tok").unwrap();
    client.scan_person("bea").unwrap();
    client.scan_person("cal").unwrap();
    client.scan_person("bea").unwrap();
    let asked = [
        PathBuf::from("/lab/locked-bea"),
        PathBuf::from("/lab/locked-cal"),
        PathBuf::from("/lab/elsewhere"),
    ];
    assert_eq!(client.system_only(&asked).unwrap(), vec![true, true, false]);
    client.bye().unwrap();
    helper.join().unwrap();
}

/// The largest asks there can be (64 places of 900 bytes each, every byte one JSON writes as
/// six): asked through the client, they go in requests that always fit, and every place is
/// answered, never a broken frame. A place too long to ask about is a no.
#[test]
fn the_largest_asks_always_fit_and_are_all_answered() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = listener.local_addr().unwrap();
    let worst = |n: usize| PathBuf::from(format!("/{n:03}{}", "\u{1}".repeat(896)));
    let helper = std::thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        let mut scanner = |_: &str| -> Result<pctwin_helper::person::PersonScan, Refusal> {
            Ok(pctwin_helper::person::PersonScan {
                found: Vec::new(),
                privacy_blocked: 0,
                system_only: vec![worst(7)],
            })
        };
        Session::new(
            "tok".into(),
            BTreeSet::from(["bea".to_string()]),
            &mut scanner,
        )
        .serve(&mut stream)
        .unwrap();
    });
    let mut client = Client::open(TcpStream::connect(addr).unwrap(), "tok").unwrap();
    client.scan_person("bea").unwrap();
    let mut asked: Vec<PathBuf> = (0..200).map(worst).collect();
    asked.push(PathBuf::from(format!(
        "/{}",
        "x".repeat(pctwin_helper::MAX_SYSTEM_PATH)
    )));
    let answers = client.system_only_all(&asked).unwrap();
    assert_eq!(answers.len(), asked.len());
    assert_eq!(answers.iter().filter(|a| **a).count(), 1);
    assert!(answers[7]);
    client.bye().unwrap();
    helper.join().unwrap();
}
