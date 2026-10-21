//! The app's side of the new laptop's conversation with the helper: an account made, a person's
//! files held and handed over, now or at their sign-in; a refusal comes back as one, and nothing
//! for a person not approved.

use std::collections::BTreeSet;
use std::net::{TcpListener, TcpStream};
use std::path::{Path, PathBuf};

use pctwin_helper::account::Made;
use pctwin_helper::newlaptop::{Counted, NewLaptop};
use pctwin_helper::{Client, ClientError, Entry, HandedOver, Refusal, Session};
use pctwin_record::Item;

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
        let mut scanner = |_: &str| -> Result<Vec<(Item, Entry)>, Refusal> { Ok(Vec::new()) };
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
