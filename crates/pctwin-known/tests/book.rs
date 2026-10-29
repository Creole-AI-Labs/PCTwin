//! Remembered connections and their permissions (Security Design Part T).

use std::path::Path;
use std::sync::Arc;

use pctwin_known::book::{
    DAY_SECONDS, DEFAULT_DAILY_SENDS, GRANT_LIFETIME, QUIET_SPELL, TOKEN_PERIOD, UNUSED_FOR,
};
use pctwin_known::{
    Admitted, Book, BookError, Device, GrantProblem, KnownFolder, MemoryStore, NextFolder, Pending,
    SecretStore, StoreError, Swap,
};
use zeroize::Zeroizing;

const T0: u64 = 1_800_000_000;

fn pendings() -> (Pending, Pending) {
    let (a, b) = (Swap::new().unwrap(), Swap::new().unwrap());
    let (ma, mb) = (a.message(), b.message());
    (a.finish(&mb).unwrap(), b.finish(&ma).unwrap())
}

fn book(dir: &Path, store: &Arc<MemoryStore>, machine: &str) -> Book {
    Book::open(
        &dir.join("connections.json"),
        store.clone(),
        Some(machine.into()),
    )
    .unwrap()
}

/// Permission 3 with its folder always usable.
fn admit(b: &mut Book, id: &str, bytes: u64, now: u64) -> Result<Admitted, GrantProblem> {
    b.admit_with(id, bytes, now, |_| Ok(())).map(|(a, ())| a)
}

fn folder(dir: &Path, name: &str) -> KnownFolder {
    let p = dir.join(name);
    std::fs::create_dir_all(&p).unwrap();
    KnownFolder::choose_with(&p, |_| None).unwrap().0
}

#[test]
fn remembering_keeps_the_secrets_in_the_store_and_only_public_things_in_the_file() {
    let dir = tempfile::tempdir().unwrap();
    let store = Arc::new(MemoryStore::default());
    let (pa, _) = pendings();
    let own = pa.own_public();
    let mut b = book(dir.path(), &store, "m1");
    let id = b.remember(&pa, "Ada's laptop", T0, None).unwrap();
    let names = store.names();
    assert_eq!(names.len(), 2, "{names:?}");
    let private = store.get(&format!("known-{id}-key")).unwrap().unwrap();
    let pair = store.get(&format!("known-{id}-pair")).unwrap().unwrap();
    let file = std::fs::read_to_string(dir.path().join("connections.json")).unwrap();
    let hex = |b: &[u8]| b.iter().map(|x| format!("{x:02x}")).collect::<String>();
    assert!(!file.contains(&hex(&private)), "no private key in the file");
    assert!(!file.contains(&hex(&pair)), "no pair secret in the file");
    assert_eq!(b.keys_for(&id).unwrap().public(), own);
    // Read back from disk.
    let again = book(dir.path(), &store, "m1");
    let list = again.list(T0);
    assert_eq!(list.len(), 1);
    assert_eq!(list[0].label, "Ada's laptop");
    assert_eq!(list[0].last_used, T0);
    assert_eq!(list[0].expires_at, T0 + UNUSED_FOR);
    assert_eq!(list[0].device, Device::Same);
}

#[test]
fn forget_deletes_the_key_and_every_permission() {
    let dir = tempfile::tempdir().unwrap();
    let store = Arc::new(MemoryStore::default());
    let (pa, _) = pendings();
    let mut b = book(dir.path(), &store, "m1");
    let id = b.remember(&pa, "Ada", T0, None).unwrap();
    b.allow_folder(&id, folder(dir.path(), "In"), T0).unwrap();
    b.set_next_folder(&id, NextFolder(folder(dir.path(), "Next")))
        .unwrap();
    b.forget(&id).unwrap();
    assert!(store.names().is_empty(), "{:?}", store.names());
    assert!(b.list(T0).is_empty());
    assert_eq!(b.keys_for(&id).err(), Some(BookError::NotFound));
    assert_eq!(admit(&mut b, &id, 1, T0).err(), Some(GrantProblem::None));
    assert!(book(dir.path(), &store, "m1").list(T0).is_empty());
}

/// A store that cannot be reached: Forget claims nothing.
struct Broken(MemoryStore);

impl SecretStore for Broken {
    fn get(&self, n: &str) -> Result<Option<Zeroizing<Vec<u8>>>, StoreError> {
        self.0.get(n)
    }
    fn put(&self, n: &str, s: &[u8]) -> Result<(), StoreError> {
        self.0.put(n, s)
    }
    fn delete(&self, _: &str) -> Result<(), StoreError> {
        Err(StoreError::Unavailable)
    }
}

#[test]
fn forget_with_the_store_out_of_reach_claims_nothing() {
    let dir = tempfile::tempdir().unwrap();
    let store = Arc::new(Broken(MemoryStore::default()));
    let mut b = Book::open(&dir.path().join("c.json"), store, None).unwrap();
    let (pa, _) = pendings();
    let id = b.remember(&pa, "Ada", T0, None).unwrap();
    assert_eq!(
        b.forget(&id).err(),
        Some(BookError::Store(StoreError::Unavailable))
    );
    assert_eq!(
        b.list(T0).len(),
        1,
        "still listed, so it can be forgotten later"
    );
}

#[test]
fn a_connection_unused_for_90_days_expires_and_use_keeps_it() {
    let dir = tempfile::tempdir().unwrap();
    let store = Arc::new(MemoryStore::default());
    let mut b = book(dir.path(), &store, "m1");
    let (p1, _) = pendings();
    let (p2, _) = pendings();
    let used = b.remember(&p1, "Used", T0, None).unwrap();
    let idle = b.remember(&p2, "Idle", T0, None).unwrap();
    b.touch(&used, T0 + UNUSED_FOR - 10).unwrap();
    assert!(b.expire(T0 + UNUSED_FOR - 1).unwrap().is_empty());
    assert_eq!(b.expire(T0 + UNUSED_FOR).unwrap(), vec![idle.clone()]);
    let left: Vec<String> = b.list(T0).into_iter().map(|c| c.id).collect();
    assert_eq!(left, vec![used]);
    assert_eq!(store.names().len(), 2, "the idle one's secrets are gone");
    // Touching never moves "last used" back.
    let id = b.list(T0)[0].id.clone();
    b.touch(&id, T0).unwrap();
    assert_eq!(b.list(T0)[0].last_used, T0 + UNUSED_FOR - 10);
}

#[test]
fn tokens_are_recognised_only_by_the_other_laptop_and_change_over_time() {
    let dir_a = tempfile::tempdir().unwrap();
    let dir_b = tempfile::tempdir().unwrap();
    let dir_c = tempfile::tempdir().unwrap();
    let (sa, sb, sc) = (
        Arc::new(MemoryStore::default()),
        Arc::new(MemoryStore::default()),
        Arc::new(MemoryStore::default()),
    );
    let (pa, pb) = pendings();
    let mut a = book(dir_a.path(), &sa, "ma");
    let mut b = book(dir_b.path(), &sb, "mb");
    let ida = a.remember(&pa, "B", T0, None).unwrap();
    let idb = b.remember(&pb, "A", T0, None).unwrap();
    // C remembers A too, through a pairing of its own.
    let (pa2, pc) = pendings();
    let ida2 = a.remember(&pa2, "C", T0, None).unwrap();
    let mut c = book(dir_c.path(), &sc, "mc");
    let idc = c.remember(&pc, "A", T0, None).unwrap();

    let announced_a = a.announced(T0);
    assert_eq!(announced_a.len(), 2, "one token per connection");
    let tok_ab = announced_a.iter().find(|(id, _)| *id == ida).unwrap().1;
    let tok_ac = announced_a.iter().find(|(id, _)| *id == ida2).unwrap().1;
    assert_ne!(tok_ab, tok_ac, "two connections never share a token");
    // B looks for A's token and finds it; C's look does not match A's token for B.
    assert!(b.peer_tokens(&idb, T0).unwrap().contains(&tok_ab));
    assert!(!c.peer_tokens(&idc, T0).unwrap().contains(&tok_ab));
    // B's own announcement is not A's (each side its own), and A recognises B's hello.
    let tok_ba = b.announced(T0)[0].1;
    assert_ne!(tok_ab, tok_ba);
    assert_eq!(a.lookup(&tok_ab, T0), Some(ida.clone()));
    assert_eq!(
        a.lookup(&tok_ab, T0 + TOKEN_PERIOD),
        Some(ida.clone()),
        "a clock a little ahead"
    );
    assert_eq!(
        a.lookup(&tok_ab, T0 + 3 * TOKEN_PERIOD),
        None,
        "an old token is not recognised"
    );
    assert_eq!(
        a.lookup(&tok_ba, T0),
        None,
        "B's own token names nothing here"
    );
    assert_eq!(a.lookup(&[0; 8], T0), None);
    // It changes every period.
    let later = a.announced(T0 + TOKEN_PERIOD);
    assert_ne!(later.iter().find(|(id, _)| *id == ida).unwrap().1, tok_ab);
}

#[test]
fn a_restored_or_copied_laptop_is_said_and_its_keys_are_not_used() {
    let dir = tempfile::tempdir().unwrap();
    let store = Arc::new(MemoryStore::default());
    let (pa, _) = pendings();
    let id = book(dir.path(), &store, "m1")
        .remember(&pa, "Ada", T0, None)
        .unwrap();
    let mut copied = book(dir.path(), &store, "m2");
    assert_eq!(copied.list(T0)[0].device, Device::Restored);
    assert_eq!(copied.keys_for(&id).err(), Some(BookError::Restored));
    assert_eq!(copied.peer_tokens(&id, T0).err(), Some(BookError::Restored));
    assert!(
        copied.announced(T0).is_empty(),
        "nothing announced from a copy"
    );
    // The person forgets it and pairs again.
    copied.forget(&id).unwrap();
    assert!(store.names().is_empty());
}

#[test]
fn permission_3_is_off_at_first_ends_on_its_own_and_is_capped() {
    let dir = tempfile::tempdir().unwrap();
    let store = Arc::new(MemoryStore::default());
    let (pa, _) = pendings();
    let mut b = book(dir.path(), &store, "m1");
    let id = b.remember(&pa, "Ada", T0, None).unwrap();
    assert_eq!(
        admit(&mut b, &id, 1, T0).err(),
        Some(GrantProblem::None),
        "off at first"
    );
    let inbox = folder(dir.path(), "Inbox");
    b.allow_folder(&id, inbox.clone(), T0).unwrap();
    let first = admit(&mut b, &id, 10, T0).unwrap();
    assert_eq!(first.folder, inbox);
    assert!(first.tell, "the first delivery is told");
    assert!(
        !admit(&mut b, &id, 10, T0 + 60).unwrap().tell,
        "not again so soon"
    );
    assert!(
        admit(&mut b, &id, 10, T0 + 60 + QUIET_SPELL).unwrap().tell,
        "told again after a quiet spell"
    );
    // Today's count cap.
    let day2 = T0 + 10 * DAY_SECONDS;
    for _ in 0..DEFAULT_DAILY_SENDS {
        admit(&mut b, &id, 1, day2).unwrap();
    }
    assert_eq!(
        admit(&mut b, &id, 1, day2).err(),
        Some(GrantProblem::OverCap)
    );
    // Today's size cap.
    let day3 = day2 + DAY_SECONDS;
    assert_eq!(
        admit(&mut b, &id, u64::MAX, day3).err(),
        Some(GrantProblem::OverCap)
    );
    assert!(admit(&mut b, &id, 1, day3).is_ok());
    // Ends on its own.
    assert_eq!(
        admit(&mut b, &id, 1, T0 + GRANT_LIFETIME).err(),
        Some(GrantProblem::Expired)
    );
    assert!(b.list(T0 + GRANT_LIFETIME)[0].grant.as_ref().unwrap().ended);
    b.renew_folder(&id, T0 + GRANT_LIFETIME).unwrap();
    assert!(admit(&mut b, &id, 1, T0 + GRANT_LIFETIME).is_ok());
    // Withdrawn on its own: the connection stays.
    b.revoke_folder(&id).unwrap();
    assert_eq!(
        admit(&mut b, &id, 1, T0 + GRANT_LIFETIME).err(),
        Some(GrantProblem::None)
    );
    assert_eq!(b.list(T0).len(), 1);
}

#[test]
fn folder_for_next_time_never_becomes_permission_3_or_the_other_way() {
    let dir = tempfile::tempdir().unwrap();
    let store = Arc::new(MemoryStore::default());
    let (pa, _) = pendings();
    let mut b = book(dir.path(), &store, "m1");
    let id = b.remember(&pa, "Ada", T0, None).unwrap();
    let next = folder(dir.path(), "Next");
    b.set_next_folder(&id, NextFolder(next.clone())).unwrap();
    assert_eq!(admit(&mut b, &id, 1, T0).err(), Some(GrantProblem::None));
    b.allow_folder(&id, folder(dir.path(), "Allowed"), T0)
        .unwrap();
    assert_eq!(b.next_folder(&id).unwrap(), Some(NextFolder(next)));
    b.clear_next_folder(&id).unwrap();
    assert!(
        admit(&mut b, &id, 1, T0).is_ok(),
        "clearing one leaves the other"
    );
    b.revoke_folder(&id).unwrap();
    b.set_next_folder(&id, NextFolder(folder(dir.path(), "Next")))
        .unwrap();
    assert!(
        b.next_folder(&id).unwrap().is_some(),
        "revoking one leaves the other"
    );
}

#[test]
fn names_are_the_person_s_own_and_bounded() {
    let dir = tempfile::tempdir().unwrap();
    let store = Arc::new(MemoryStore::default());
    let mut b = book(dir.path(), &store, "m1");
    for bad in ["", "   ", &"x".repeat(41), "a\u{0007}b"] {
        let (p, _) = pendings();
        assert_eq!(
            b.remember(&p, bad, T0, None).err(),
            Some(BookError::BadLabel),
            "{bad:?}"
        );
    }
    assert!(store.names().is_empty(), "nothing kept for a refused name");
    let (p, _) = pendings();
    let id = b.remember(&p, "  Ada  ", T0, None).unwrap();
    assert_eq!(b.label(&id).unwrap(), "Ada");
    b.rename(&id, "Ada (work)").unwrap();
    assert_eq!(b.label(&id).unwrap(), "Ada (work)");
}

#[test]
fn the_same_other_laptop_remembered_again_replaces_the_old_connection() {
    let dir = tempfile::tempdir().unwrap();
    let store = Arc::new(MemoryStore::default());
    let mut b = book(dir.path(), &store, "m1");
    let (a1, b1) = (Swap::new().unwrap(), Swap::new().unwrap());
    let m1 = b1.message();
    let first = b
        .remember(&a1.finish(&m1).unwrap(), "Ada", T0, None)
        .unwrap();
    // The same message from the other laptop again (it kept its half): the old one goes.
    let again = b
        .remember(&Swap::new().unwrap().finish(&m1).unwrap(), "Ada", T0, None)
        .unwrap();
    assert_ne!(first, again);
    assert_eq!(b.list(T0).len(), 1);
    assert_eq!(store.names().len(), 2);
}

#[test]
fn a_damaged_book_is_never_read_as_empty() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("connections.json");
    std::fs::write(&path, b"{not json").unwrap();
    let store: Arc<MemoryStore> = Arc::new(MemoryStore::default());
    assert_eq!(
        Book::open(&path, store.clone(), None).err(),
        Some(BookError::File)
    );
    std::fs::write(&path, br#"{"version":9,"connections":[]}"#).unwrap();
    assert_eq!(Book::open(&path, store, None).err(), Some(BookError::File));
}

/// Pairing the same laptop again makes fresh keys, so nothing but the person can say it is the
/// same laptop: when they say which connection it replaces, that one is forgotten (its keys and
/// permission 3) before the new one is kept. A lost or copied laptop keeps nothing.
#[test]
fn pairing_again_replaces_the_connection_the_person_names() {
    let dir = tempfile::tempdir().unwrap();
    let store = Arc::new(MemoryStore::default());
    let mut b = book(dir.path(), &store, "m1");
    let (p1, _) = pendings();
    let old = b.remember(&p1, "Ada's laptop", T0, None).unwrap();
    b.allow_folder(&old, folder(dir.path(), "In"), T0).unwrap();
    let (p2, _) = pendings();
    let other = b.remember(&p2, "Bo", T0, None).unwrap();
    assert_eq!(b.same_label("  ada's LAPTOP "), vec![old.clone()]);
    // A connection that is not there: nothing is kept.
    let (p3, _) = pendings();
    assert_eq!(
        b.remember(&p3, "Ada's laptop", T0, Some("0011223344556677"))
            .err(),
        Some(BookError::NotFound)
    );
    assert_eq!(store.names().len(), 4);
    let (p4, _) = pendings();
    let new = b.remember(&p4, "Ada's laptop", T0 + 5, Some(&old)).unwrap();
    assert!(!b.has(&old));
    assert!(b.has(&new) && b.has(&other));
    assert_eq!(b.keys_for(&old).err(), Some(BookError::NotFound));
    assert_eq!(admit(&mut b, &old, 1, T0).err(), Some(GrantProblem::None));
    assert_eq!(admit(&mut b, &new, 1, T0).err(), Some(GrantProblem::None));
    let names = store.names();
    assert_eq!(names.len(), 4, "{names:?}");
    assert!(names.iter().all(|n| !n.contains(&old)), "{names:?}");
    // Without `replaces` the earlier connection stays (two connections with one name).
    let (p5, _) = pendings();
    b.remember(&p5, "Bo", T0, None).unwrap();
    assert_eq!(b.same_label("Bo").len(), 2);
}

/// Hardware that gives no identity telling laptops apart is never said to be "the same device".
#[test]
fn a_laptop_that_cannot_tell_says_so_and_never_same_device() {
    use pctwin_known::machine::fingerprint;
    for bad in [
        "FFFFFFFF-FFFF-FFFF-FFFF-FFFFFFFFFFFF",
        "00000000-0000-0000-0000-000000000000",
        "03000200-0400-0500-0006-000700080009",
        "Not Settable",
        "To Be Filled By O.E.M.",
        "",
        "1234",
    ] {
        assert_eq!(fingerprint(bad), None, "{bad}");
    }
    let real = "4C4C4544-0051-3510-8052-B4C04F4A3732";
    assert!(fingerprint(real).is_some());
    assert_eq!(fingerprint(real), fingerprint(&real.to_lowercase()));
    assert_ne!(
        fingerprint(real),
        fingerprint("4C4C4544-0051-3510-8052-B4C04F4A3733")
    );

    let dir = tempfile::tempdir().unwrap();
    let store = Arc::new(MemoryStore::default());
    let (pa, _) = pendings();
    let path = dir.path().join("connections.json");
    let mut unknown = Book::open(&path, store.clone(), None).unwrap();
    let id = unknown.remember(&pa, "Ada", T0, None).unwrap();
    assert_eq!(unknown.list(T0)[0].device, Device::CannotTell);
    assert!(unknown.keys_for(&id).is_ok(), "still usable, said plainly");
    // Remembered where it could not tell, opened where it can: still cannot tell.
    let now_known = book(dir.path(), &store, "m1");
    assert_eq!(now_known.list(T0)[0].device, Device::CannotTell);
}

/// At most so many connections; replacing one at the limit is still allowed.
#[test]
fn the_book_holds_at_most_its_limit() {
    use pctwin_known::book::MAX_CONNECTIONS;
    let dir = tempfile::tempdir().unwrap();
    let store = Arc::new(MemoryStore::default());
    let mut b = book(dir.path(), &store, "m1");
    let mut first = String::new();
    for n in 0..MAX_CONNECTIONS {
        let (p, _) = pendings();
        let id = b.remember(&p, &format!("L{n}"), T0, None).unwrap();
        if n == 0 {
            first = id;
        }
    }
    let (p, _) = pendings();
    assert_eq!(
        b.remember(&p, "One more", T0, None).err(),
        Some(BookError::Full)
    );
    let (p, _) = pendings();
    assert!(b.remember(&p, "L0", T0, Some(&first)).is_ok());
    assert_eq!(b.list(T0).len(), MAX_CONNECTIONS);
}

/// A store that counts its reads.
struct Counting(MemoryStore, std::sync::atomic::AtomicUsize);

impl SecretStore for Counting {
    fn get(&self, n: &str) -> Result<Option<Zeroizing<Vec<u8>>>, StoreError> {
        self.1.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        self.0.get(n)
    }
    fn put(&self, n: &str, s: &[u8]) -> Result<(), StoreError> {
        self.0.put(n, s)
    }
    fn delete(&self, n: &str) -> Result<(), StoreError> {
        self.0.delete(n)
    }
}

/// Any device nearby can say hello with junk: that is a lookup in memory, never a read of the
/// key store, which is read once per token window.
#[test]
fn hellos_never_read_the_key_store() {
    use std::sync::atomic::Ordering::Relaxed;
    let dir = tempfile::tempdir().unwrap();
    let store = Arc::new(Counting(
        MemoryStore::default(),
        std::sync::atomic::AtomicUsize::new(0),
    ));
    let mut b = Book::open(&dir.path().join("c.json"), store.clone(), None).unwrap();
    let mut ids = Vec::new();
    for n in 0..3 {
        let (p, _) = pendings();
        ids.push(b.remember(&p, &format!("L{n}"), T0, None).unwrap());
    }
    let mine = b.announced(T0);
    let after_first = store.1.load(Relaxed);
    for n in 0u64..20_000 {
        assert_eq!(b.lookup(&n.to_be_bytes(), T0 + n % 60), None);
    }
    for (id, t) in &mine {
        assert_eq!(b.lookup(t, T0).as_ref(), Some(id));
    }
    assert_eq!(store.1.load(Relaxed), after_first, "no reads for hellos");
    // A forgotten connection's token names nothing at once, in the same window.
    b.forget(&ids[1]).unwrap();
    let gone1 = mine.iter().find(|(id, _)| *id == ids[1]).unwrap().1;
    assert_eq!(b.lookup(&gone1, T0), None);
    // The next window: worked out once more.
    b.lookup(&[9; 8], T0 + TOKEN_PERIOD);
    let next = store.1.load(Relaxed);
    assert!(next > after_first);
    b.lookup(&[8; 8], T0 + TOKEN_PERIOD + 1);
    assert_eq!(store.1.load(Relaxed), next);
    // A forgotten connection's token names nothing at once.
    b.forget(&ids[0]).unwrap();
    let gone = mine.iter().find(|(id, _)| *id == ids[0]).unwrap().1;
    assert_eq!(b.lookup(&gone, T0), None);
}

/// Permission 3 counts a delivery and ends the quiet spell only when its folder could be used:
/// a delivery that falls back to asking is not counted, and the next one is still told.
#[test]
fn a_delivery_that_could_not_use_the_folder_counts_nothing() {
    use pctwin_known::FolderProblem;
    let dir = tempfile::tempdir().unwrap();
    let store = Arc::new(MemoryStore::default());
    let mut b = book(dir.path(), &store, "m1");
    let (pa, _) = pendings();
    let id = b.remember(&pa, "Ada", T0, None).unwrap();
    b.allow_folder(&id, folder(dir.path(), "In"), T0).unwrap();
    for _ in 0..DEFAULT_DAILY_SENDS + 5 {
        let r: Result<(Admitted, ()), _> =
            b.admit_with(&id, 1, T0, |_| Err(FolderProblem::NotTheSame));
        assert_eq!(r.err(), Some(GrantProblem::Unusable));
    }
    let first = admit(&mut b, &id, 1, T0).unwrap();
    assert!(first.tell, "still the first delivery after the quiet spell");
    for _ in 1..DEFAULT_DAILY_SENDS {
        admit(&mut b, &id, 1, T0).unwrap();
    }
    assert_eq!(admit(&mut b, &id, 1, T0).err(), Some(GrantProblem::OverCap));
    // Over the cap: the folder is not even opened.
    let mut opened = false;
    let _ = b.admit_with(&id, 1, T0, |_| {
        opened = true;
        Ok(())
    });
    assert!(!opened);
}

/// Remembered on hardware that could tell, and now the hardware cannot: that is never taken for
/// the same laptop.
#[test]
fn a_laptop_that_now_cannot_tell_is_taken_for_another() {
    let dir = tempfile::tempdir().unwrap();
    let store = Arc::new(MemoryStore::default());
    let (pa, _) = pendings();
    let id = book(dir.path(), &store, "m1")
        .remember(&pa, "Ada", T0, None)
        .unwrap();
    let now = Book::open(&dir.path().join("connections.json"), store.clone(), None).unwrap();
    assert_eq!(now.list(T0)[0].device, Device::Restored);
    assert_eq!(now.keys_for(&id).err(), Some(BookError::Restored));
}

/// The new connection is written before the old one is forgotten; when forgetting fails, the new
/// one is taken back and the old one stays, so nothing is lost and the person can try again.
#[test]
fn replacing_writes_the_new_one_first_and_takes_it_back_on_failure() {
    let dir = tempfile::tempdir().unwrap();
    let inner = Arc::new(MemoryStore::default());
    let store = Arc::new(Flaky(
        inner.clone(),
        std::sync::atomic::AtomicBool::new(false),
    ));
    let mut b = Book::open(&dir.path().join("c.json"), store.clone(), Some("m1".into())).unwrap();
    let (p1, _) = pendings();
    let old = b.remember(&p1, "Ada", T0, None).unwrap();
    store.1.store(true, std::sync::atomic::Ordering::Relaxed);
    let (p2, _) = pendings();
    assert!(b.remember(&p2, "Ada", T0, Some(&old)).is_err());
    assert_eq!(b.list(T0).len(), 1, "only the old one: nothing half done");
    assert!(b.has(&old));
    store.1.store(false, std::sync::atomic::Ordering::Relaxed);
    // The same pending, still held, works the second time.
    let new = b.remember(&p2, "Ada", T0, Some(&old)).unwrap();
    assert!(!b.has(&old) && b.has(&new));
    let names = inner.names();
    assert!(names.iter().all(|n| !n.contains(&old)), "{names:?}");
    assert!(names.iter().any(|n| n.contains(&new)), "{names:?}");
}

/// A store whose deletes fail while its flag is set (puts and reads work).
struct Flaky(Arc<MemoryStore>, std::sync::atomic::AtomicBool);

impl SecretStore for Flaky {
    fn get(&self, n: &str) -> Result<Option<Zeroizing<Vec<u8>>>, StoreError> {
        self.0.get(n)
    }
    fn put(&self, n: &str, s: &[u8]) -> Result<(), StoreError> {
        self.0.put(n, s)
    }
    fn delete(&self, n: &str) -> Result<(), StoreError> {
        if self.1.load(std::sync::atomic::Ordering::Relaxed) {
            return Err(StoreError::Unavailable);
        }
        self.0.delete(n)
    }
}
