//! The administrator helper's own record of its privileged actions: intent durably first, the
//! outcome once, found again by the action's key after a restart, and never trusted if changed.

mod common;
use common::keys;

use pctwin_journal::{BrokerAction, BrokerState, JournalError, MAX_BROKER_TEXT};
use redb::ReadableTable;

fn action(n: u8) -> BrokerAction {
    BrokerAction {
        key: [n; 32],
        plan_generation: 7,
        plan_digest: [9; 32],
        action_id: format!("install.app{n}"),
        action: r#"{"create-restore-point":{}}"#.into(),
    }
}

#[test]
fn an_action_is_recorded_before_and_finished_once() {
    let (_d, path, j) = common::journal();
    let b = j.broker(&keys().broker);
    assert_eq!(b.lookup(&[1; 32]).unwrap(), BrokerState::Nothing);
    let id = b.intend(&action(1)).unwrap();
    assert_eq!(b.lookup(&[1; 32]).unwrap(), BrokerState::Unfinished(id));
    // The same action is never recorded twice: it is looked up and answered from the record.
    assert!(matches!(
        b.intend(&action(1)),
        Err(JournalError::OutOfOrder { .. })
    ));
    // A crash: the next helper finds the intent with no outcome.
    drop(j);
    let j = common::open(&path).unwrap();
    let b = j.broker(&keys().broker);
    assert_eq!(b.lookup(&[1; 32]).unwrap(), BrokerState::Unfinished(id));
    b.outcome(id, r#"{"already-in-place":null}"#).unwrap();
    assert_eq!(
        b.lookup(&[1; 32]).unwrap(),
        BrokerState::Finished(id, r#"{"already-in-place":null}"#.into())
    );
    assert!(matches!(
        b.outcome(id, "again"),
        Err(JournalError::OutOfOrder { .. })
    ));
    assert!(matches!(
        b.outcome(99, "x"),
        Err(JournalError::NoSuchEntry(99))
    ));
    // Another action has its own number.
    let other = b.intend(&action(2)).unwrap();
    assert_ne!(other, id);
}

#[test]
fn text_too_long_or_empty_is_not_kept() {
    let (_d, _p, j) = common::journal();
    let b = j.broker(&keys().broker);
    let mut long = action(3);
    long.action = "x".repeat(MAX_BROKER_TEXT + 1);
    assert!(matches!(b.intend(&long), Err(JournalError::TooLarge)));
    let mut empty = action(3);
    empty.action_id = String::new();
    assert!(matches!(b.intend(&empty), Err(JournalError::TooLarge)));
    let id = b.intend(&action(3)).unwrap();
    assert!(matches!(
        b.outcome(id, &"y".repeat(MAX_BROKER_TEXT + 1)),
        Err(JournalError::TooLarge)
    ));
    assert_eq!(b.lookup(&[3; 32]).unwrap(), BrokerState::Unfinished(id));
}

/// A broker record changed by hand (an outcome written in, so the action would never be checked)
/// is damaged: the helper treats the record as unavailable and acts on nothing.
#[test]
fn a_broker_record_changed_by_hand_is_never_trusted() {
    let (_d, path, j) = common::journal();
    let id = j.broker(&keys().broker).intend(&action(4)).unwrap();
    drop(j);
    {
        let db = redb::Database::open(&path).unwrap();
        let tx = db.begin_write().unwrap();
        {
            let mut t = tx
                .open_table(redb::TableDefinition::<u64, &[u8]>::new("broker-actions"))
                .unwrap();
            let stored = t.get(id).unwrap().unwrap().value().to_vec();
            let (seal, json) = stored.split_at(32);
            let text = String::from_utf8(json.to_vec())
                .unwrap()
                .replace("\"outcome\":null", "\"outcome\":\"done\"");
            t.insert(id, [seal, text.as_bytes()].concat().as_slice())
                .unwrap();
        }
        tx.commit().unwrap();
    }
    let j = common::open(&path).unwrap();
    assert!(matches!(
        j.broker(&keys().broker).lookup(&[4; 32]),
        Err(JournalError::Damaged(_))
    ));
}
