//! The parser process from the app's side: it answers exactly as the readers do in-process, and
//! every way it can fail (it stops, hangs, asks for too much, answers out of turn, is missing)
//! ends as "could not be checked", never as a pass, with a fresh process for the next request.
#![expect(
    clippy::disallowed_methods,
    reason = "the tests make and open the files the app would hold"
)]

use std::fs::File;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use pctwin_maker::facts::*;
use pctwin_maker::{Expected, OwnReader, SystemChecks, SystemSignature};
use pctwin_parse::wire::{Answer, Request};
use pctwin_parse::{Limits, NotChecked, Parser, Wall};
use pctwin_parse_seal::in_child;
use pctwin_readiness::checks::{FileReader, Shape};

fn real() -> Parser {
    Parser::at(PathBuf::from(env!("CARGO_BIN_EXE_pctwin-parser")))
}

fn double() -> Parser {
    Parser::at(PathBuf::from(env!(
        "CARGO_BIN_EXE_pctwin-parser-test-double"
    )))
}

fn file_of(dir: &tempfile::TempDir, name: &str, bytes: &[u8]) -> (PathBuf, File) {
    let p = dir.path().join(name);
    std::fs::File::create(&p).unwrap().write_all(bytes).unwrap();
    let f = File::open(&p).unwrap();
    (p, f)
}

/// What the same reading gives in this process (allowed here: this crate holds the seal).
fn in_process(request: Request, f: &File) -> Answer {
    let size = request.size();
    let mut r = f;
    let seal = in_child();
    match request {
        Request::Installer { .. } => match pctwin_maker::own::read_installer(seal, &mut r, size) {
            Ok(read) => Answer::Installer(read),
            Err(e) => Answer::Failed(e.to_string()),
        },
        Request::OldCopy { .. } => {
            Answer::OldCopy(pctwin_maker::own::read_old_copy(seal, &mut r, size))
        }
        Request::Font { .. } => {
            Answer::Font(pctwin_readiness::checks::font_names(seal, &mut r, size))
        }
        Request::Archive { .. } => {
            Answer::Archive(pctwin_readiness::checks::archive_shape(seal, &mut r, size))
        }
        Request::Database { .. } => {
            Answer::Database(pctwin_readiness::checks::database_shape(seal, &mut r, size))
        }
        Request::Json { .. } | Request::DatabaseFacts { .. } => {
            let mut bytes = Vec::new();
            std::io::Seek::seek(&mut r, std::io::SeekFrom::Start(0)).unwrap();
            std::io::Read::read_to_end(&mut r, &mut bytes).unwrap();
            pctwin_parse::testing::answer_in_memory(request, &bytes)
        }
    }
}

fn every_request(size: u64) -> [Request; 7] {
    [
        Request::Installer { size },
        Request::OldCopy { size },
        Request::Font { size },
        Request::Archive { size },
        Request::Database { size },
        Request::Json { size },
        Request::DatabaseFacts {
            size,
            log: 0,
            millis: 5000,
        },
    ]
}

/// The process's answer, with a failed read counted as the in-process `Failed`.
fn via_process(p: &Parser, request: Request, f: &File) -> Answer {
    match p.ask(request, f) {
        Ok(a) => a,
        Err(NotChecked::Unreadable(why)) => Answer::Failed(why),
        Err(e) => panic!("the parser process failed on {request:?}: {e}"),
    }
}

/// A minimal 64-bit Windows program with no signature.
fn tiny_pe() -> Vec<u8> {
    let mut b = vec![0u8; 0x400];
    b[0] = b'M';
    b[1] = b'Z';
    b[0x3C..0x40].copy_from_slice(&0x80u32.to_le_bytes());
    b[0x80..0x84].copy_from_slice(b"PE\0\0");
    let coff = 0x84;
    b[coff..coff + 2].copy_from_slice(&0x8664u16.to_le_bytes());
    b[coff + 2..coff + 4].copy_from_slice(&1u16.to_le_bytes());
    b[coff + 16..coff + 18].copy_from_slice(&240u16.to_le_bytes());
    let opt = coff + 20;
    b[opt..opt + 2].copy_from_slice(&0x20Bu16.to_le_bytes());
    b[opt + 60..opt + 64].copy_from_slice(&0x200u32.to_le_bytes());
    b[opt + 108..opt + 112].copy_from_slice(&16u32.to_le_bytes());
    let sec = opt + 240;
    b[sec + 16..sec + 20].copy_from_slice(&0x200u32.to_le_bytes());
    b[sec + 20..sec + 24].copy_from_slice(&0x200u32.to_le_bytes());
    b
}

fn sqlite_header(pages: u32) -> Vec<u8> {
    let mut h = vec![0u8; 4096 * pages as usize];
    h[..16].copy_from_slice(b"SQLite format 3\0");
    h[16..18].copy_from_slice(&4096u16.to_be_bytes());
    h[18] = 1;
    h[19] = 1;
    h[21] = 64;
    h[22] = 32;
    h[23] = 32;
    h[28..32].copy_from_slice(&pages.to_be_bytes());
    h
}

#[test]
fn the_process_says_how_it_is_locked_down() {
    let dir = tempfile::tempdir().unwrap();
    let (_, f) = file_of(&dir, "a.db", &sqlite_header(1));
    let p = real();
    assert!(p.confinement().is_none(), "started only when first needed");
    assert_eq!(
        p.ask(Request::Database { size: 4096 }, &f).unwrap(),
        Answer::Database(Shape::Sound)
    );
    let c = p.confinement().unwrap();
    for wall in [
        Wall::OwnProcess,
        Wall::EmptyEnvironment,
        Wall::ReadsOnlyWhatItIsHanded,
        Wall::BoundedRequests,
    ] {
        assert!(c.in_place.contains(&wall), "{wall:?} in place");
    }
    // What waits for approval is said, never claimed.
    for wall in [Wall::NoNetwork, Wall::NoFileSystem, Wall::LowRights] {
        assert!(c.not_yet.contains(&wall) && !c.in_place.contains(&wall));
    }
}

#[test]
fn it_answers_as_the_readers_do_in_process_on_known_files() {
    let dir = tempfile::tempdir().unwrap();
    let mut zip = Vec::new();
    zip.extend_from_slice(b"PK\x05\x06");
    zip.extend_from_slice(&[0u8; 18]);
    let mut ttf = vec![0, 1, 0, 0, 0, 0];
    ttf.resize(64, 0);
    let flatpak = b"[Flatpak Ref]\nName=org.example.App\nUrl=https://dl.example.org/repo/\n";
    let files: Vec<(&str, Vec<u8>)> = vec![
        ("setup.exe", tiny_pe()),
        ("a.db", sqlite_header(2)),
        ("cut.db", sqlite_header(2)[..5000].to_vec()),
        ("empty.zip", zip),
        ("font.ttf", ttf),
        ("app.flatpakref", flatpak.to_vec()),
        ("nothing", Vec::new()),
    ];
    let p = real();
    for (name, bytes) in files {
        let (_, f) = file_of(&dir, name, &bytes);
        for r in every_request(bytes.len() as u64) {
            assert_eq!(via_process(&p, r, &f), in_process(r, &f), "{name} {r:?}");
        }
    }
}

/// Real Windows programs (some with embedded signatures, some large): the same answer through
/// the process, read piece by piece, as in-process.
#[cfg(windows)]
#[test]
fn it_reads_real_windows_programs_as_in_process() {
    let windir = std::env::var_os("WINDIR").unwrap_or_else(|| "C:\\Windows".into());
    let mut seen = 0;
    let mut signed = 0;
    let p = real();
    for dir in ["System32", "SysWOW64", "explorer.exe"] {
        let path = Path::new(&windir).join(dir);
        let entries: Vec<PathBuf> = if path.is_file() {
            vec![path]
        } else {
            let Ok(list) = std::fs::read_dir(&path) else {
                continue;
            };
            list.flatten()
                .map(|e| e.path())
                .filter(|p| {
                    p.extension().is_some_and(|x| {
                        x.eq_ignore_ascii_case("exe") || x.eq_ignore_ascii_case("dll")
                    })
                })
                .take(60)
                .collect()
        };
        for e in entries {
            let Ok(f) = File::open(&e) else { continue };
            let size = f.metadata().unwrap().len();
            for r in [Request::OldCopy { size }, Request::Installer { size }] {
                let a = via_process(&p, r, &f);
                assert_eq!(a, in_process(r, &f), "{}", e.display());
                if let Answer::OldCopy(c) = &a
                    && matches!(c.maker, pctwin_maker::OldMaker::Signed(_))
                {
                    signed += 1;
                }
            }
            seen += 1;
        }
    }
    assert!(seen > 20, "programs read: {seen}");
    assert!(signed > 0, "some carry their maker's signature: {signed}");
}

#[test]
fn a_large_file_is_read_through_in_time() {
    // 256 MiB that looks like a program: every byte is hashed through the pipes.
    let dir = tempfile::tempdir().unwrap();
    let mut b = tiny_pe();
    b.resize(256 * 1024 * 1024, 7);
    let (_, f) = file_of(&dir, "big.exe", &b);
    let p = real();
    let started = Instant::now();
    let a = via_process(
        &p,
        Request::Installer {
            size: b.len() as u64,
        },
        &f,
    );
    assert_eq!(
        a,
        in_process(
            Request::Installer {
                size: b.len() as u64
            },
            &f
        )
    );
    assert!(
        started.elapsed() < Duration::from_secs(60),
        "{:?}",
        started.elapsed()
    );
}

fn quick() -> Limits {
    Limits {
        start: Duration::from_secs(20),
        base: Duration::from_secs(2),
        read_slack: 1024,
        ..Limits::default()
    }
}

/// Every way the process can fail, and that the next request gets a fresh, working process.
#[test]
fn every_failure_is_could_not_be_checked_and_the_next_request_works() {
    let dir = tempfile::tempdir().unwrap();
    let p = double().with_limits(quick());
    let good = sqlite_header(1);
    let (_, ok) = file_of(&dir, "ok.db", &good);
    type Expect = fn(&NotChecked) -> bool;
    let cases: [(u64, Expect); 6] = [
        (1, |e| *e == NotChecked::Stopped),
        (2, |e| *e == NotChecked::TimedOut),
        (3, |e| matches!(e, NotChecked::BrokeTheRules(_))),
        (4, |e| matches!(e, NotChecked::BrokeTheRules(_))),
        (5, |e| matches!(e, NotChecked::BrokeTheRules(_))),
        (6, |e| matches!(e, NotChecked::BrokeTheRules(_))),
    ];
    for (size, expect) in cases {
        let (_, f) = file_of(&dir, &format!("bad-{size}"), &vec![0u8; size as usize]);
        let started = Instant::now();
        let e = p.ask(Request::Font { size }, &f).unwrap_err();
        assert!(expect(&e), "size {size}: {e:?}");
        assert!(
            started.elapsed() < Duration::from_secs(10),
            "size {size} took {:?}",
            started.elapsed()
        );
        assert!(p.confinement().is_none(), "the broken process was ended");
        assert_eq!(
            p.ask(Request::Database { size: 4096 }, &ok).unwrap(),
            Answer::Database(Shape::Sound),
            "a fresh process after size {size}"
        );
    }
}

#[test]
fn a_missing_parser_program_is_could_not_be_checked() {
    let dir = tempfile::tempdir().unwrap();
    let (_, f) = file_of(&dir, "a.db", &sqlite_header(1));
    let p = Parser::at(dir.path().join("no-such-parser.exe"));
    assert!(matches!(
        p.ask(Request::Database { size: 4096 }, &f),
        Err(NotChecked::CouldNotStart(_))
    ));
}

#[test]
fn a_program_that_is_not_the_parser_is_refused() {
    // A program that says nothing in the parser's protocol: never trusted.
    let dir = tempfile::tempdir().unwrap();
    let (_, f) = file_of(&dir, "a.db", &sqlite_header(1));
    let p =
        Parser::at(PathBuf::from(env!("CARGO_BIN_EXE_pctwin-parser")).with_extension("missing"))
            .with_limits(quick());
    assert!(p.ask(Request::Database { size: 4096 }, &f).is_err());
}

// ---- The honest result in each part that uses the parser ----

struct NoSystem;
impl SystemChecks for NoSystem {
    fn signature(
        &self,
        _: &File,
        _: &Path,
        _: Format,
        _: &pctwin_maker::Limits,
    ) -> SystemSignature {
        SystemSignature {
            signature: Signature::Trusted,
            embedded: true,
            maker: None,
            arch: None,
        }
    }
    fn malware(&self, _: &Path, _: Format, _: &Signature, _: &pctwin_maker::Limits) -> Malware {
        Malware::NotFullyChecked {
            why: "not asked".into(),
        }
    }
}

fn expected() -> Expected {
    Expected {
        system: System::Windows,
        maker: Maker::Linux(LinuxSource::Rpm { key: "x".into() }),
        formats: vec![Format::WindowsProgram],
        arch: Arch::X64,
        bytes: None,
    }
}

#[test]
fn a_parser_that_stops_makes_the_maker_check_not_checked_never_install() {
    let dir = tempfile::tempdir().unwrap();
    for size in [1u64, 2] {
        let (p, _) = file_of(
            &dir,
            &format!("setup-{size}.exe"),
            &vec![b'M'; size as usize],
        );
        let parser = double().with_limits(quick());
        let v = pctwin_maker::check(
            &p,
            &expected(),
            &parser,
            &NoSystem,
            &pctwin_maker::Limits::default(),
        )
        .unwrap();
        assert!(
            matches!(&v.signature, Signature::NotChecked { why } if why.starts_with(pctwin_maker::NOT_CHECKED_BY_READER)),
            "{:?}",
            v.signature
        );
        assert!(matches!(v.same_maker, SameMaker::Unknown { .. }));
        assert!(!matches!(v.decide(), Decision::Install { .. }));
        // The old laptop's own copy: not read, never "unsigned" or "signed".
        let old = pctwin_maker::windows_old_copy(&p, &parser);
        assert!(
            matches!(old.maker, pctwin_maker::OldMaker::NotRead(_)),
            "{old:?}"
        );
    }
}

#[test]
fn a_parser_that_stops_makes_a_readiness_check_could_not_be_checked() {
    use pctwin_readiness::checks::{
        Choices, Chosen, Goal, NotRun, Outcome, font_with, shipped_font_files,
    };
    use pctwin_readiness::ledger::Role;
    let dir = tempfile::tempdir().unwrap();
    let parser = double().with_limits(quick());
    for size in [1usize, 2] {
        let (p, _) = file_of(&dir, &format!("x-{size}.db"), &vec![0u8; size]);
        let choices = Choices::picked(
            1,
            vec![
                Chosen {
                    item: pctwin_record::ItemId::from_hex(&format!("{:032x}", 1)).unwrap(),
                    owner: Role::Data,
                    goal: Goal::DatabaseOpens { file: p.clone() },
                },
                Chosen {
                    item: pctwin_record::ItemId::from_hex(&format!("{:032x}", 2)).unwrap(),
                    owner: Role::Data,
                    goal: Goal::ArchiveOpens { file: p.clone() },
                },
            ],
        )
        .unwrap();
        for r in pctwin_readiness::checks::run(&choices, &parser) {
            assert_eq!(
                r.outcome,
                Outcome::NotRun {
                    why: NotRun::CouldNotBeChecked
                }
            );
        }
    }
    // A font folder whose only file stops the checker: the family is never "installed".
    let fonts = tempfile::tempdir().unwrap();
    file_of(&fonts, "a.ttf", &[0u8]);
    let o = font_with(
        shipped_font_files(),
        "Family",
        &[fonts.path().to_path_buf()],
        &parser,
    );
    assert!(!matches!(o, Outcome::MachineChecked { .. }), "{o:?}");
}

// ---- Same answers on any bytes: the stable-toolchain fuzz of every entry point ----

mod same_on_any_bytes {
    use super::*;
    use proptest::prelude::*;

    fn seeds() -> Vec<Vec<u8>> {
        let mut zip = b"PK\x03\x04".to_vec();
        zip.resize(64, 0);
        zip.extend_from_slice(b"PK\x05\x06");
        zip.extend_from_slice(&[0u8; 18]);
        vec![
            tiny_pe(),
            sqlite_header(1),
            zip,
            vec![0, 1, 0, 0, 0, 3, 0, 0, 0, 0, 0, 0],
            b"ttcf\0\0\0\x02\0\0\0\x10".to_vec(),
            vec![0xD0, 0xCF, 0x11, 0xE0, 0xA1, 0xB1, 0x1A, 0xE1],
            b"[Flatpak Ref]\nName=a.b.C\n".to_vec(),
        ]
    }

    fn mutated() -> impl Strategy<Value = Vec<u8>> {
        (
            0..seeds().len(),
            proptest::collection::vec((any::<u16>(), any::<u8>()), 0..32),
            0usize..4096,
        )
            .prop_map(|(i, flips, extra)| {
                let mut b = seeds()[i].clone();
                b.resize(b.len() + extra % 64, 0);
                for (at, v) in flips {
                    if !b.is_empty() {
                        let n = b.len();
                        b[at as usize % n] = v;
                    }
                }
                b
            })
    }

    proptest! {
        #![proptest_config(ProptestConfig { cases: 96, ..ProptestConfig::default() })]
        #[test]
        fn mutated_files(bytes in mutated()) {
            check_same(&bytes);
        }

        #[test]
        fn random_files(bytes in proptest::collection::vec(any::<u8>(), 0..2048)) {
            check_same(&bytes);
        }
    }

    fn check_same(bytes: &[u8]) {
        use std::sync::OnceLock;
        static P: OnceLock<Parser> = OnceLock::new();
        let p = P.get_or_init(real);
        let dir = tempfile::tempdir().unwrap();
        let (_, f) = file_of(&dir, "f", bytes);
        for r in every_request(bytes.len() as u64) {
            assert_eq!(via_process(p, r, &f), in_process(r, &f), "{r:?}");
        }
    }
}

#[test]
fn the_reader_traits_go_through_the_process() {
    let dir = tempfile::tempdir().unwrap();
    let (_, f) = file_of(&dir, "a.db", &sqlite_header(1));
    let p = real();
    assert_eq!(FileReader::database(&p, &f, 4096).unwrap(), Shape::Sound);
    let (_, exe) = file_of(&dir, "a.exe", &tiny_pe());
    let read = OwnReader::installer(&p, &exe, tiny_pe().len() as u64).unwrap();
    assert_eq!(read.format, Format::WindowsProgram);
    assert_eq!(read.own, pctwin_maker::OwnRead::None);
}

// ---- Files' contents (2.5): JSON facts and live databases, read in the parser process ----

mod contents {
    use super::*;
    use pctwin_special::content::{ContentError, ContentReader, DbKind, InThisProcess};

    const OWN: InThisProcess = InThisProcess::new(pctwin_parse_seal::in_child());

    /// A live database: its log beside it holds changes not yet folded into the file (a table
    /// made, rows added), as a running app leaves it.
    fn live_db(dir: &tempfile::TempDir) -> (rusqlite::Connection, PathBuf) {
        let p = dir.path().join("live.sqlite");
        let c = rusqlite::Connection::open(&p).unwrap();
        c.execute_batch(
            "PRAGMA journal_mode=WAL; PRAGMA wal_autocheckpoint=0;
             CREATE TABLE old(id INTEGER, path TEXT);
             INSERT INTO old VALUES(1, 'C:\\Users\\a\\one.txt');",
        )
        .unwrap();
        c.execute_batch("PRAGMA wal_checkpoint(TRUNCATE);").unwrap();
        c.execute_batch(
            "CREATE TABLE logins(origin_url TEXT, password_value BLOB);
             INSERT INTO old VALUES(2, 'C:\\Users\\a\\two.txt');",
        )
        .unwrap();
        (c, p)
    }

    #[test]
    fn a_live_databases_log_is_read_as_sqlite_reads_it() {
        let dir = tempfile::tempdir().unwrap();
        let (c, p) = live_db(&dir);
        let db = std::fs::read(&p).unwrap();
        let log = std::fs::read(pctwin_special::content::wal_of(&p)).unwrap();
        assert!(!log.is_empty(), "the changes are in the log");
        let parser = real();
        let t = Duration::from_secs(5);
        let via = parser.sqlite(&db, Some(&log), t).unwrap();
        assert_eq!(via, OWN.sqlite(&db, Some(&log), t).unwrap());
        // The table made only in the log is seen: a store of saved passwords.
        let names: Vec<&str> = via.tables.iter().map(|t| t.name.as_str()).collect();
        assert_eq!(names, ["logins", "old"]);
        assert_eq!(via.kind, DbKind::Credentials);
        // Without its log, only what was folded in.
        let without = parser.sqlite(&db, None, t).unwrap();
        assert_eq!(without.kind, DbKind::Ordinary);
        assert_eq!(
            without.paths.unwrap().0,
            vec!["C:\\Users\\a\\one.txt".to_string()]
        );
        // An ordinary live store: its paths, including the one only in the log.
        c.execute_batch("DROP TABLE logins;").unwrap();
        let log = std::fs::read(pctwin_special::content::wal_of(&p)).unwrap();
        let facts = parser.sqlite(&db, Some(&log), t).unwrap();
        assert_eq!(facts.kind, DbKind::Ordinary);
        let (paths, complete) = facts.paths.unwrap();
        assert!(complete);
        assert_eq!(paths.len(), 2, "{paths:?}");
        drop(c);
    }

    #[test]
    fn a_torn_or_foreign_log_is_left_out_never_trusted() {
        let dir = tempfile::tempdir().unwrap();
        let (c, p) = live_db(&dir);
        let db = std::fs::read(&p).unwrap();
        let log = std::fs::read(pctwin_special::content::wal_of(&p)).unwrap();
        let t = Duration::from_secs(5);
        let parser = real();
        // A byte changed in the first frame: its checksum fails, so nothing after it counts.
        let mut torn = log.clone();
        torn[32 + 24 + 10] ^= 0xff;
        let facts = parser.sqlite(&db, Some(&torn), t).unwrap();
        assert_eq!(facts, OWN.sqlite(&db, Some(&torn), t).unwrap());
        assert_eq!(facts, parser.sqlite(&db, None, t).unwrap());
        assert_ne!(facts.kind, DbKind::Credentials);
        // Only the last frame torn: the transactions before it still count.
        let mut tail = log.clone();
        let n = tail.len();
        tail[n - 10] ^= 0xff;
        assert_eq!(
            parser.sqlite(&db, Some(&tail), t).unwrap(),
            OWN.sqlite(&db, Some(&tail), t).unwrap()
        );
        // A log of nonsense is no log.
        let junk = vec![0x5a; 4096];
        assert_eq!(
            parser.sqlite(&db, Some(&junk), t).unwrap(),
            parser.sqlite(&db, None, t).unwrap()
        );
        drop(c);
    }

    #[test]
    fn json_facts_come_back_as_in_process() {
        let parser = real();
        let doc = br#"{"token": "abc", "file": "C:\\Users\\a\\b.txt", "command": "run.exe"}"#;
        assert_eq!(parser.json(doc).unwrap(), OWN.json(doc).unwrap());
        assert!(parser.json(b"{not json").is_err());
    }

    #[test]
    fn a_parser_that_stops_leaves_a_database_not_checked() {
        let parser = double().with_limits(quick());
        for size in [1usize, 2] {
            let e = parser.sqlite(&vec![0u8; size], None, Duration::from_secs(1));
            assert!(matches!(e, Err(ContentError::Damaged(_))), "{e:?}");
            let e = parser.json(&vec![b'{'; size]);
            assert!(matches!(e, Err(ContentError::Damaged(_))), "{e:?}");
        }
    }

    #[test]
    fn the_app_data_plan_reads_databases_through_the_parser() {
        // Installed once for this test program: every database this program's plans read goes
        // through the real parser process.
        pctwin_special::content::install_reader(pctwin_parse::testing::parser());
        let dir = tempfile::tempdir().unwrap();
        let (c, p) = live_db(&dir);
        let until = std::time::Instant::now() + Duration::from_secs(8);
        let (kind, _) = pctwin_special::content::sqlite_facts(&p, until).unwrap();
        assert_eq!(kind, DbKind::Credentials);
        // Nothing beside the live database was written by the reading.
        let mut names: Vec<_> = std::fs::read_dir(dir.path())
            .unwrap()
            .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
            .collect();
        names.sort();
        assert_eq!(names, ["live.sqlite", "live.sqlite-shm", "live.sqlite-wal"]);
        drop(c);
    }
}
