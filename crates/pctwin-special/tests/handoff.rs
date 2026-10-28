//! Vendor handoffs and the app's own export (plan sections 7 and 10): the person's step comes from
//! the component's class; pages and apps are opened as the person and stay needs-action; an export
//! tool runs only from a signed row, from the app's own folder, signed by the app's maker, with
//! typed arguments, writing into an owner-only place.

mod common;

use std::cell::RefCell;
use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;
use std::time::Duration;

use common::*;
use pctwin_catalogue::Catalogue;
use pctwin_grant::{CaptureEffect, Now, PlanSeal, RoleKeys, RootRef};
use pctwin_maker::facts::*;
use pctwin_maker::run::{Call, Ran, RunError, Runner};
use pctwin_maker::{Expected, Limits, SystemChecks, SystemSignature, Verdict};
use pctwin_record::Owner;
use pctwin_special::catalogue::{Arg, NativeRoute, SHIPPED, SpecialRow};
use pctwin_special::classify::{FolderReport, classify_all};
use pctwin_special::component::Custody;
use pctwin_special::export::ExportSlot;
use pctwin_special::handoff::*;
use pctwin_special::land::OwnerOnly;
use pctwin_special::roots::{System as Os, find_folders, roots_of};

fn keys() -> &'static RoleKeys {
    static K: OnceLock<RoleKeys> = OnceLock::new();
    K.get_or_init(|| RoleKeys::mint().expect("minted once in this test program"))
}

fn seal() -> PlanSeal {
    PlanSeal::new(4, [1; 32])
}

fn found(home: &Path, cat: &Catalogue<SpecialRow>) -> Vec<FolderReport> {
    let owner = Owner::Person {
        account_id: "1001".into(),
    };
    let (folders, _) = find_folders(&owner, home, &roots_of(Os::Windows, home, &|_| None), &[]);
    classify_all(folders, Os::Windows, cat)
}

#[derive(Default)]
struct Recorded {
    pages: RefCell<Vec<String>>,
    apps: RefCell<Vec<PathBuf>>,
}

impl Person for Recorded {
    fn open_page(&self, url: &str) -> std::io::Result<()> {
        self.pages.borrow_mut().push(url.into());
        Ok(())
    }
    fn open_app(&self, program: &Path) -> std::io::Result<()> {
        self.apps.borrow_mut().push(program.into());
        Ok(())
    }
}

#[test]
fn every_component_that_does_not_copy_gets_its_step() {
    pctwin_special::content::install_reader(&OWN_READERS);
    let t = temp();
    let home = home_of(&t);
    chromium_profile(&home.join("AppData/Local/Chromey/User Data"));
    let rows = br#"{"format": 1, "catalogue": "special", "version": 2, "rows": [
        {"system": "windows", "root": "local", "folder": "Chromey", "checked_on": "2026-10-27",
         "relink": "https://example.org/sign-in",
         "export": {"kind": "open-page", "url": "https://example.org/export-passwords"}}]}"#;
    let with_row = Catalogue::<SpecialRow>::shipped(std::str::from_utf8(rows).unwrap());
    for (cat, has_row) in [
        (Catalogue::<SpecialRow>::shipped(SHIPPED), false),
        (with_row, true),
    ] {
        let r = found(&home, &cat);
        let f = &r[0];
        let person = Recorded::default();
        for c in &f.components {
            let step = step_for(f, c);
            match c.custody {
                Custody::Portable => assert_eq!(step, Step::None),
                Custody::ServiceHeld => assert!(matches!(step, Step::SignIn { .. })),
                Custody::MachineBound => assert!(matches!(step, Step::Relink { .. })),
                Custody::NeedsExport => assert!(matches!(step, Step::Export { .. })),
                Custody::Unresolved => assert_eq!(step, Step::LookAtIt),
            }
            let opened = open(&person, &step, None);
            if has_row && c.custody != Custody::Portable {
                assert_eq!(opened, Opened::Opened, "{step:?}");
            } else {
                assert_eq!(opened, Opened::InWords, "with no row the step is in words");
            }
        }
        if has_row {
            let pages = person.pages.borrow();
            assert!(pages.contains(&"https://example.org/sign-in".to_string()));
            assert!(pages.contains(&"https://example.org/export-passwords".to_string()));
        } else {
            assert!(person.pages.borrow().is_empty());
        }
    }
    // The app's own export screen opens the app, as the person.
    let person = Recorded::default();
    let step = Step::Export {
        route: Some(NativeRoute::OpenApp),
    };
    assert_eq!(
        open(&person, &step, Some(Path::new("app.exe"))),
        Opened::Opened
    );
    assert_eq!(*person.apps.borrow(), vec![PathBuf::from("app.exe")]);
    assert!(matches!(open(&person, &step, None), Opened::NotOpened(_)));
}

// ---- The app's own export tool ----

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

fn bytes_of(b: &[u8]) -> Bytes {
    use sha2::Digest;
    Bytes {
        size: b.len() as u64,
        sha256: Sha256(sha2::Sha256::digest(b).into()),
    }
}

/// A verdict as the maker check gives it for a program with these bytes; `same`: whether its
/// signer is the app's maker.
fn verdict(bytes: Bytes, same: bool) -> Verdict {
    Verdict {
        system: System::Windows,
        bytes,
        steady: Steady::Yes,
        expected_bytes: None,
        format: FormatFact::Matches {
            found: Format::WindowsProgram,
        },
        arch: ArchFact::Matches,
        signature: Signature::Trusted,
        same_maker: if same {
            SameMaker::Yes {
                by: MatchedBy::SameCertificate,
            }
        } else {
            SameMaker::No {
                found: "Someone Else Ltd".into(),
            }
        },
        malware: Malware::NoDetection {
            coverage: Coverage {
                scanner: "stand-in".into(),
                later_downloads_not_checked: true,
            },
        },
        found_maker: None,
    }
}

/// Answers with a verdict for the bytes the tool had when it was checked; `after` changes the
/// tool once checked (a swap before it starts).
struct Checked {
    same: bool,
    after: Option<Vec<u8>>,
}

impl ToolMaker for Checked {
    fn verdict(&self, tool: &Path) -> Result<Verdict, String> {
        let v = verdict(bytes_of(&std::fs::read(tool).unwrap()), self.same);
        if let Some(b) = &self.after {
            std::fs::write(tool, b).unwrap();
        }
        Ok(v)
    }
}

/// Stands in for running the tool: records the call and writes the export (or not).
struct Tool {
    calls: RefCell<Vec<Call>>,
    writes: bool,
}

impl Runner for Tool {
    fn run(&self, call: &Call) -> Result<Ran, RunError> {
        self.calls.borrow_mut().push(call.clone());
        if self.writes {
            let out = call.args.last().unwrap();
            std::fs::write(
                out,
                b"url,username,password\nhttps://example.org,p,invented\n",
            )
            .unwrap();
        }
        Ok(Ran {
            code: Some(0),
            stdout: "secret output never kept".into(),
            stderr: String::new(),
        })
    }
}

struct AppOwnerOnly;
impl OwnerOnly for AppOwnerOnly {
    fn private_folder(&self, path: &Path) -> std::io::Result<()> {
        #[cfg(windows)]
        return pctwin_helper_os::private_folder(path)
            .map_err(|e| std::io::Error::other(format!("{e:?}")));
        #[cfg(unix)]
        return pctwin_special::land::UnixOwnerOnly.private_folder(path);
    }
}

struct NoChecks;
impl SystemChecks for NoChecks {
    fn signature(&self, _: &std::fs::File, _: &Path, _: Format, _: &Limits) -> SystemSignature {
        SystemSignature {
            signature: Signature::Missing,
            embedded: false,
            maker: None,
            arch: None,
        }
    }
    fn malware(&self, _: &Path, _: Format, _: &Signature, _: &Limits) -> Malware {
        Malware::NoDetection {
            coverage: Coverage {
                scanner: "stand-in".into(),
                later_downloads_not_checked: true,
            },
        }
    }
}

#[test]
fn an_export_tool_runs_only_when_it_is_the_makers_own() {
    pctwin_special::content::install_reader(&OWN_READERS);
    let t = temp();
    let app = t.path().join("Program Files/Synthetic");
    put(&app.join("synthetic-export.exe"), &tiny_pe());
    let route = NativeRoute::Tool {
        tool: "synthetic-export.exe".into(),
        args: vec![Arg::Switch("--export".into()), Arg::OutputFile],
    };
    let k = &keys().capture;
    let env = vec![(OsString::from("SystemRoot"), OsString::from(r"C:\Windows"))];
    let try_run = |maker: &dyn ToolMaker,
                   route: Option<&NativeRoute>,
                   folder: &Path,
                   writes: bool,
                   effect: CaptureEffect| {
        let slot = ExportSlot::prepare(
            &t.path().join("PCTwin/exports"),
            "export.csv",
            &AppOwnerOnly,
        )
        .unwrap();
        let tool = Tool {
            calls: RefCell::new(Vec::new()),
            writes,
        };
        let grant = k
            .grant(
                seal(),
                vec![RootRef::new(3)],
                effect,
                Duration::from_secs(60),
                Now::real(),
            )
            .unwrap();
        let got = run_tool(
            grant,
            &ToolRun {
                seal: seal(),
                root: RootRef::new(3),
                app_folder: folder,
                route,
                maker,
                runner: &tool,
                env: env.clone(),
                timeout: Duration::from_secs(30),
                slot: &slot,
            },
            Now::real(),
        );
        (got, tool.calls.into_inner(), slot)
    };
    let native = CaptureEffect::NativeExport;
    let good = Checked {
        same: true,
        after: None,
    };

    // The maker's own tool: run with typed arguments, and its export observed.
    let (got, calls, slot) = try_run(&good, Some(&route), &app, true, native);
    let exported = got.unwrap();
    assert!(exported.sealed.len > 0);
    assert_eq!(calls.len(), 1);
    let call = &calls[0];
    assert_eq!(call.program, app.join("synthetic-export.exe"));
    assert_eq!(
        call.args,
        vec![
            OsString::from("--export"),
            slot.file().as_os_str().to_owned()
        ]
    );
    assert_eq!(call.env, env, "nothing of PCTwin's own environment");
    assert!(call.stdin.is_none());
    assert!(!format!("{exported:?}").contains("secret output"));

    // Signed by another maker: never run.
    let other_maker = Checked {
        same: false,
        after: None,
    };
    let (got, calls, _) = try_run(&other_maker, Some(&route), &app, true, native);
    assert!(matches!(got, Err(ToolError::NotTheMakers(_))), "{got:?}");
    assert!(calls.is_empty());

    // Swapped between the check and the start: never run.
    let mut swapped = tiny_pe();
    swapped[0x300] = 0x90;
    let swapping = Checked {
        same: true,
        after: Some(swapped),
    };
    let (got, calls, _) = try_run(&swapping, Some(&route), &app, true, native);
    assert_eq!(got.unwrap_err(), ToolError::Changed);
    assert!(calls.is_empty());
    std::fs::write(app.join("synthetic-export.exe"), tiny_pe()).unwrap();

    // Through the real maker check, an unsigned tool never runs.
    let expected = Expected {
        system: System::Windows,
        maker: Maker::Windows(WindowsSigner {
            subject: Subject {
                common_name: Some("Synthetic Maker Ltd".into()),
                organization: Some("Synthetic Maker Ltd".into()),
                country: Some("NG".into()),
                ..Subject::default()
            },
            certificate: Sha256([1; 32]),
            issuer: "CN=CA".into(),
            issuer_organization: Some("Some CA Ltd".into()),
            public_key: Some(Sha256([2; 32])),
        }),
        formats: vec![Format::WindowsProgram],
        arch: Arch::X64,
        bytes: None,
    };
    let limits = Limits::default();
    let real = MakerCheck {
        expected: &expected,
        reader: pctwin_parse::testing::parser(),
        checks: &NoChecks,
        limits: &limits,
    };
    let (got, calls, _) = try_run(&real, Some(&route), &app, true, native);
    assert!(matches!(got, Err(ToolError::NotTheMakers(_))), "{got:?}");
    assert!(calls.is_empty());

    // No row naming a tool, or the tool not in the app's own folder: nothing runs.
    let (got, _, _) = try_run(&good, None, &app, true, native);
    assert_eq!(got.unwrap_err(), ToolError::NoTool);
    let (got, _, _) = try_run(&good, Some(&NativeRoute::OpenApp), &app, true, native);
    assert_eq!(got.unwrap_err(), ToolError::NoTool);
    let (got, _, _) = try_run(
        &good,
        Some(&route),
        &t.path().join("elsewhere"),
        true,
        native,
    );
    assert_eq!(got.unwrap_err(), ToolError::NotThere);

    // It ran but wrote nothing: not exported.
    let (got, _, _) = try_run(&good, Some(&route), &app, false, native);
    assert_eq!(got.unwrap_err(), ToolError::NoExport);

    // Another kind of grant does not run it.
    for effect in [CaptureEffect::ReadPrivate, CaptureEffect::Import] {
        let (got, calls, _) = try_run(&good, Some(&route), &app, true, effect);
        assert!(matches!(got, Err(ToolError::Denied(_))));
        assert!(calls.is_empty());
    }
}

/// PCTwin's readers of files' contents, run in the test's own process (only a parser crate's
/// tests may).
static OWN_READERS: pctwin_special::content::InThisProcess =
    pctwin_special::content::InThisProcess::new(pctwin_parse_seal::in_child());
