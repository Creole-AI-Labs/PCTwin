//! The system's own key store, used for real (run by hand: `cargo test -- --ignored`), so the
//! test runs never fill a person's key store with left-over entries.

use pctwin_known::{SecretStore, SystemStore};

#[test]
#[ignore = "uses the system's own key store"]
fn the_system_store_keeps_reads_back_and_deletes() {
    let mut raw = [0u8; 8];
    getrandom::fill(&mut raw).unwrap();
    let name = format!(
        "known-test{}-key",
        raw.iter().map(|b| format!("{b:02x}")).collect::<String>()
    );
    let store = SystemStore;
    assert_eq!(store.get(&name).unwrap(), None);
    store.put(&name, &[7u8; 32]).unwrap();
    let back = store.get(&name);
    let gone = store.delete(&name);
    assert_eq!(back.unwrap().unwrap().as_slice(), &[7u8; 32]);
    gone.unwrap();
    assert_eq!(store.get(&name).unwrap(), None);
    store.delete(&name).unwrap();
}
