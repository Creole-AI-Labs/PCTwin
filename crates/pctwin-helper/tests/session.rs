//! One session of the helper, as the app sees it: the session token first, only approved people
//! scanned, only totals and handles handed out (never a name or a path), files read only by
//! handle, and nothing else accepted.
// Tests make files to test on.
#![allow(clippy::disallowed_methods)]

use std::collections::BTreeSet;
use std::io::{Cursor, Read, Write};

use pctwin_helper::protocol::{read_from_helper, send_request};
use pctwin_helper::{Answer, Entry, FromHelper, Handle, Refusal, Request, Session};
use pctwin_record::Item;

/// The app's requests, read by the helper; the helper's answers, kept for the test to read.
struct Channel {
    from_app: Cursor<Vec<u8>>,
    to_app: Vec<u8>,
}

impl Read for Channel {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        self.from_app.read(buf)
    }
}

impl Write for Channel {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.to_app.write(buf)
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

/// Runs one session on `requests`, with Bea approved and her two files found; returns every
/// frame the helper sent.
fn session(requests: &[Request]) -> (Vec<FromHelper>, tempfile::TempDir) {
    let dir = tempfile::tempdir().unwrap();
    let a = dir.path().join("bea-private-diary.txt");
    let b = dir.path().join("bea-tax.pdf");
    std::fs::write(&a, b"dear diary").unwrap();
    std::fs::write(&b, b"0123456789abcdef").unwrap();
    let entry = |p: &std::path::Path| {
        let meta = std::fs::metadata(p).unwrap();
        Entry {
            path: p.to_path_buf(),
            size: meta.len(),
            modified_ns: meta
                .modified()
                .unwrap()
                .duration_since(std::time::UNIX_EPOCH)
                .ok()
                .and_then(|d| i64::try_from(d.as_nanos()).ok()),
        }
    };
    let item = |name: &str, size: u64| {
        let owner = pctwin_record::Owner::Person {
            account_id: "bea".into(),
        };
        let place = pctwin_record::Place {
            role: pctwin_record::FolderRole::Documents,
            storage: pctwin_record::Storage::SystemDrive,
        };
        let path =
            pctwin_record::ItemPath::new(vec![pctwin_record::ItemName::from_text(name)]).unwrap();
        let laptop = pctwin_record::LaptopId::from_hex(&"0".repeat(32)).unwrap();
        pctwin_record::Item {
            id: pctwin_record::ItemId::derive(&laptop, &owner, &place, &path),
            kind: pctwin_record::ItemKind::File,
            owner,
            place,
            path,
            size_bytes: size,
            modified_ns: None,
            portability: pctwin_record::Portability::Portable,
            managed_by: pctwin_record::ManagedBy::Personal,
            download_bytes: None,
            inclusion: pctwin_record::Inclusion::Included,
        }
    };
    let found = vec![
        (item("bea-private-diary.txt", 10), entry(&a)),
        (item("bea-tax.pdf", 16), entry(&b)),
    ];
    let mut scanner = move |account: &str| -> Result<Vec<(Item, Entry)>, Refusal> {
        assert_eq!(account, "bea");
        Ok(found.clone())
    };
    let mut wire = Vec::new();
    for r in requests {
        send_request(&mut wire, r).unwrap();
    }
    let mut channel = Channel {
        from_app: Cursor::new(wire),
        to_app: Vec::new(),
    };
    let approved = BTreeSet::from(["bea".to_string()]);
    Session::new("token".into(), approved, &mut scanner)
        .serve(&mut channel)
        .unwrap();
    let mut out = Vec::new();
    let mut read = channel.to_app.as_slice();
    while !read.is_empty() {
        out.push(read_from_helper(&mut read).unwrap());
    }
    (out, dir)
}

fn hello() -> Request {
    Request::Hello {
        session: "token".into(),
    }
}

#[test]
fn an_approved_person_is_scanned_and_read_only_by_handle() {
    let (out, _dir) = session(&[
        hello(),
        Request::ScanPerson {
            account: "bea".into(),
        },
        Request::ReadBlocks {
            handle: 1,
            offset: 4,
            len: 4,
        },
        Request::ReadBlocks {
            handle: 2,
            offset: 0,
            len: 1,
        },
        Request::Bye,
    ]);
    // The items travel in the move record (never shown to the mover): checked on their own.
    let FromHelper::Items(items) = &out[3] else {
        panic!("{out:?}");
    };
    assert_eq!(items.len(), 2);
    assert_eq!(items[0].size_bytes, 10);
    let out: Vec<FromHelper> = out
        .into_iter()
        .filter(|f| !matches!(f, FromHelper::Items(_)))
        .collect();
    assert_eq!(
        out,
        vec![
            FromHelper::Answer(Answer::Ready),
            FromHelper::Answer(Answer::Scanned {
                files: 2,
                bytes: 26
            }),
            FromHelper::Handles(vec![
                Handle {
                    handle: 0,
                    size: 10
                },
                Handle {
                    handle: 1,
                    size: 16
                },
            ]),
            FromHelper::Blocks(b"4567".to_vec()),
            FromHelper::Answer(Answer::Refused(Refusal::UnknownHandle)),
            FromHelper::Answer(Answer::Goodbye),
        ]
    );
}

/// The totals and handles (what a screen may show) never hold another person's file names, and
/// no frame ever holds where their files are on the drive: only the record items, which travel in
/// the move record and are never shown to the mover, carry names.
#[test]
fn nothing_the_app_is_told_names_the_person_s_files() {
    let (out, dir) = session(&[
        hello(),
        Request::ScanPerson {
            account: "bea".into(),
        },
        Request::Bye,
    ]);
    let all = format!("{out:?}");
    assert!(!all.contains(&*dir.path().to_string_lossy()), "{all}");
    let told = format!(
        "{:?}",
        out.iter()
            .filter(|f| !matches!(f, FromHelper::Items(_)))
            .collect::<Vec<_>>()
    );
    assert!(!told.contains("diary"), "{told}");
    assert!(!told.contains("tax"), "{told}");
}

#[test]
fn a_person_not_approved_is_never_scanned() {
    let (out, _dir) = session(&[
        hello(),
        Request::ScanPerson {
            account: "carl".into(),
        },
        Request::ReadBlocks {
            handle: 0,
            offset: 0,
            len: 1,
        },
    ]);
    assert_eq!(
        out,
        vec![
            FromHelper::Answer(Answer::Ready),
            FromHelper::Answer(Answer::Refused(Refusal::NotApproved)),
            FromHelper::Answer(Answer::Refused(Refusal::UnknownHandle)),
        ]
    );
}

#[test]
fn nothing_is_done_before_the_right_session_token() {
    let (out, _dir) = session(&[Request::ScanPerson {
        account: "bea".into(),
    }]);
    assert_eq!(
        out,
        vec![FromHelper::Answer(Answer::Refused(Refusal::NoSession))]
    );
    let (out, _dir) = session(&[
        Request::Hello {
            session: "guess".into(),
        },
        hello(),
    ]);
    // A wrong token ends the session: no second guess.
    assert_eq!(
        out,
        vec![FromHelper::Answer(Answer::Refused(Refusal::WrongSession))]
    );
}

/// New laptop: only an approved person's account is made; the temporary password is said once,
/// in the answer, and an existing account is reported, never changed.
#[test]
fn only_an_approved_person_s_account_is_made() {
    use pctwin_helper::account::Made;
    let mut made: Vec<String> = Vec::new();
    let mut maker = |name: &str, _full: &str| -> Result<Made, Refusal> {
        if name == "bea" && made.iter().any(|m| m == name) {
            return Err(Refusal::AccountExists);
        }
        made.push(name.to_string());
        Ok(Made {
            account: "S-1-5-21-9".into(),
            password: zeroize::Zeroizing::new("abcd-efgh-jkmn-pqrs".into()),
        })
    };
    let mut scanner = |_: &str| -> Result<Vec<(Item, Entry)>, Refusal> { Ok(Vec::new()) };
    let mut wire = Vec::new();
    for r in [
        hello(),
        Request::CreateAccount {
            name: "carl".into(),
            full_name: "Carl".into(),
        },
        Request::CreateAccount {
            name: "bea".into(),
            full_name: "Bea".into(),
        },
        Request::CreateAccount {
            name: "bea".into(),
            full_name: "Bea".into(),
        },
    ] {
        send_request(&mut wire, &r).unwrap();
    }
    let mut channel = Channel {
        from_app: Cursor::new(wire),
        to_app: Vec::new(),
    };
    Session::new(
        "token".into(),
        BTreeSet::from(["bea".to_string()]),
        &mut scanner,
    )
    .with_account_maker(&mut maker)
    .serve(&mut channel)
    .unwrap();
    let mut read = channel.to_app.as_slice();
    let mut out = Vec::new();
    while !read.is_empty() {
        out.push(read_from_helper(&mut read).unwrap());
    }
    assert_eq!(
        out,
        vec![
            FromHelper::Answer(Answer::Ready),
            FromHelper::Answer(Answer::Refused(Refusal::NotApproved)),
            FromHelper::Answer(Answer::AccountMade {
                account: "S-1-5-21-9".into(),
                password: "abcd-efgh-jkmn-pqrs".into()
            }),
            FromHelper::Answer(Answer::Refused(Refusal::AccountExists)),
        ]
    );
    drop(maker);
    assert_eq!(made, ["bea"]);
}

/// New laptop: a person's folder in the holding area only for an approved person (one the
/// helper made, or an approved account ID), named by the helper, the same one each time asked;
/// a hand-over only of what is held, for that person.
#[test]
fn holding_and_handing_over_only_for_approved_people() {
    use pctwin_helper::account::Made;
    use pctwin_helper::newlaptop::{Counted, NewLaptop};
    use std::path::{Path, PathBuf};

    #[derive(Default)]
    struct Fake {
        areas: u32,
        handed: Vec<(String, String, PathBuf)>,
    }
    impl NewLaptop for Fake {
        fn area(&mut self) -> Result<PathBuf, Refusal> {
            self.areas += 1;
            Ok(PathBuf::from("/area"))
        }
        fn person_folder(&mut self, area: &Path, key: &str) -> Result<PathBuf, Refusal> {
            Ok(area.join(key))
        }
        fn hand_over(
            &mut self,
            account: &pctwin_helper_os::Account,
            name: &str,
            folder: &Path,
        ) -> Result<Counted, Refusal> {
            self.handed.push((
                format!("{account:?}"),
                name.to_string(),
                folder.to_path_buf(),
            ));
            Ok(Counted {
                landed: 3,
                kept: 0,
                failed: 0,
            })
        }
    }
    let mut maker = |_: &str, _: &str| -> Result<Made, Refusal> {
        Ok(Made {
            account: "1005".into(),
            password: zeroize::Zeroizing::new("abcd-efgh-jkmn-pqrs".into()),
        })
    };
    let mut scanner = |_: &str| -> Result<Vec<(Item, Entry)>, Refusal> { Ok(Vec::new()) };
    let mut fake = Fake::default();
    let mut wire = Vec::new();
    for r in [
        hello(),
        Request::HoldFor {
            account: "carl".into(),
        },
        Request::HandOverNow {
            account: "1007".into(),
        },
        Request::CreateAccount {
            name: "bea".into(),
            full_name: "Bea".into(),
        },
        Request::HoldFor {
            account: "bea".into(),
        },
        Request::HoldFor {
            account: "bea".into(),
        },
        Request::HoldFor {
            account: "1007".into(),
        },
        Request::HandOverNow {
            account: "bea".into(),
        },
    ] {
        send_request(&mut wire, &r).unwrap();
    }
    let mut channel = Channel {
        from_app: Cursor::new(wire),
        to_app: Vec::new(),
    };
    Session::new(
        "token".into(),
        BTreeSet::from(["bea".to_string(), "1007".to_string()]),
        &mut scanner,
    )
    .with_account_maker(&mut maker)
    .with_new_laptop(&mut fake)
    .serve(&mut channel)
    .unwrap();
    let mut read = channel.to_app.as_slice();
    let mut out = Vec::new();
    while !read.is_empty() {
        out.push(read_from_helper(&mut read).unwrap());
    }
    let held = |p: &str| {
        FromHelper::Answer(Answer::Holding {
            path: Path::new("/area").join(p).to_string_lossy().into_owned(),
        })
    };
    assert_eq!(
        out,
        vec![
            FromHelper::Answer(Answer::Ready),
            FromHelper::Answer(Answer::Refused(Refusal::NotApproved)),
            FromHelper::Answer(Answer::Refused(Refusal::NothingHeld)),
            FromHelper::Answer(Answer::AccountMade {
                account: "1005".into(),
                password: "abcd-efgh-jkmn-pqrs".into()
            }),
            held("p1"),
            held("p1"),
            held("p2"),
            FromHelper::Answer(Answer::HandedOver {
                landed: 3,
                kept: 0,
                failed: 0
            }),
        ]
    );
    // One holding area for the move; Bea's hand-over is into the account the helper made.
    assert_eq!(fake.areas, 1);
    assert_eq!(fake.handed.len(), 1);
    assert_eq!(fake.handed[0].0, "Uid(1005)");
    assert_eq!(fake.handed[0].1, "bea");
    assert_eq!(fake.handed[0].2, Path::new("/area").join("p1"));
}
