//! One session of the helper, as the app sees it: the session token first, only approved people
//! scanned, only totals and handles handed out (never a name or a path), files read only by
//! handle, and nothing else accepted.
// Tests make files to test on.
#![allow(clippy::disallowed_methods)]

use std::collections::BTreeSet;
use std::io::{Cursor, Read, Write};

use pctwin_helper::protocol::{read_from_helper, send_request};
use pctwin_helper::{Answer, Entry, FromHelper, Handle, Refusal, Request, Session};

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
    let found = vec![entry(&a), entry(&b)];
    let mut scanner = move |account: &str| -> Result<Vec<Entry>, Refusal> {
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

/// What reaches the app never holds another person's file names or folders.
#[test]
fn nothing_the_app_is_told_names_the_person_s_files() {
    let (out, dir) = session(&[
        hello(),
        Request::ScanPerson {
            account: "bea".into(),
        },
        Request::Bye,
    ]);
    let told = format!("{out:?}");
    assert!(!told.contains("diary"), "{told}");
    assert!(!told.contains("tax"), "{told}");
    assert!(!told.contains(&*dir.path().to_string_lossy()), "{told}");
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
