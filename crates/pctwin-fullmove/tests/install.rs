//! The real install chain, proven up to the administrator helper's boundary, with nothing
//! installed and nothing run elevated: winget (a stand-in that writes what `winget download`
//! writes) gives a manifest and an installer; the chain checks the manifest is the old laptop's
//! very app, hashes the bytes, starts the helper ONCE for the whole sealed plan (the restore point
//! first, then every install, each naming those exact bytes and the old laptop's maker), stages
//! the bytes, asks for each install, and then judges by Windows' own list (a stand-in) what is
//! really there. A declined prompt, a different app, a swapped download and an installer that
//! slipped in another program each end honestly, never "reinstalled".

use std::cell::RefCell;
use std::collections::BTreeMap;
use std::ffi::OsString;
use std::io::Read;
use std::path::PathBuf;

use pctwin_apps::after::Todo;
use pctwin_apps::found::windows::{RawEntry, RegValue};
use pctwin_apps::routes::{AppPlan, Card, Disposition, Route, WingetBy};
use pctwin_fullmove::install::{Helper, HelperDown, Machine, WingetInstaller, winget_hints};
use pctwin_fullmove::plan::AppEntry;
use pctwin_fullmove::run::Installer;
use pctwin_helper::Acted;
use pctwin_helper::broker::actions::{
    Action, ApprovedPlan, Hex16, Hex32, InstallDate, InstalledProduct, ProductCode, ProductVersion,
};
use pctwin_helper::broker::receipt::{Outcome, Receipt};
use pctwin_helper::broker::restore::RestorePointResult;
use pctwin_helper::broker::{BrokerRequest, Denial};
use pctwin_journal::RestorePoint;
use pctwin_maker::run::{Call, Ran, RunError, Runner};
use pctwin_record::{
    AppArch, AppMaker, AppSystem, FoundApp, FoundWhere, Owner, RegistryView, SignerId,
};
use pctwin_types::{ItemResult, ItemStatus, Reason};
use sha2::Digest;

const CODE: &str = "{3B0346D3-155D-43E5-9561-B9BE51CF2EB6}";
const INSTALLER: &[u8] = b"an msi's bytes, as winget downloaded them";
const MOVE: [u8; 16] = [0x4d; 16];

fn manifest(code: &str, sha: Option<[u8; 32]>) -> String {
    let sha = sha
        .map(|s| {
            format!(
                "  InstallerSha256: {}\n",
                s.iter().map(|b| format!("{b:02x}")).collect::<String>()
            )
        })
        .unwrap_or_default();
    format!(
        "PackageIdentifier: Lab.Writer\nPackageVersion: 2.1.0\nPackageName: Lab Writer\nPublisher: PCTwin Lab Ltd\nInstallers:\n- Architecture: x64\n  InstallerType: msi\n{sha}  Scope: machine\n  ProductCode: '{code}'\n  AppsAndFeaturesEntries:\n  - DisplayName: Lab Writer\n    ProductCode: '{code}'\nManifestVersion: 1.12.0\nManifestType: merged\n"
    )
}

fn app() -> FoundApp {
    FoundApp {
        name: "Lab Writer".into(),
        version: Some("2.0".into()),
        publisher: Some("PCTwin Lab Ltd".into()),
        owner: Owner::Shared,
        system: AppSystem::Windows,
        found: FoundWhere::WindowsEntry {
            view: RegistryView::Machine,
            key: "LabWriter".into(),
            product_code: Some(CODE.into()),
        },
        arch: AppArch::UNKNOWN,
        maker: AppMaker::Windows(Box::new(SignerId {
            organization: Some("PCTwin Lab Ltd".into()),
            certificate: [4; 32],
            ..Default::default()
        })),
        web: None,
        update_feed: None,
        installers: vec![],
        notes: vec![],
    }
}

fn entry(app: FoundApp) -> AppEntry {
    AppEntry {
        plan: AppPlan {
            disposition: Disposition::Install,
            routes: vec![Route::Winget {
                id: "Lab.Writer".into(),
                by: WingetBy::ProductCode,
            }],
            flags: vec![],
            card: Card {
                name: app.name.clone(),
                version: None,
                maker: None,
                web: None,
                licence_key: false,
                sign_in: false,
                registration: vec![],
                why: None,
            },
        },
        app,
    }
}

/// `winget download` and `winget search`, as a stand-in: it writes the manifest and installer
/// into the folder it is given; nothing runs.
struct FakeWinget {
    manifest: String,
    bytes: Vec<u8>,
    calls: std::sync::Mutex<Vec<Vec<OsString>>>,
}

impl Runner for FakeWinget {
    fn run(&self, call: &Call) -> Result<Ran, RunError> {
        self.calls.lock().unwrap().push(call.args.clone());
        let args: Vec<String> = call
            .args
            .iter()
            .map(|a| a.to_string_lossy().into_owned())
            .collect();
        if args.first().map(String::as_str) == Some("search") {
            return Ok(Ran {
                code: Some(0),
                stdout: format!(
                    "Name        Id          Version\nLab Writer  Lab.Writer  2.1.0  {CODE}\n"
                ),
                stderr: String::new(),
            });
        }
        let at = args
            .iter()
            .position(|a| a == "--download-directory")
            .unwrap();
        let dir = PathBuf::from(&args[at + 1]);
        std::fs::write(dir.join("Lab.Writer_2.1.0.yaml"), &self.manifest).unwrap();
        std::fs::write(dir.join("Lab.Writer_2.1.0.msi"), &self.bytes).unwrap();
        Ok(Ran {
            code: Some(0),
            stdout: String::new(),
            stderr: String::new(),
        })
    }
}

/// The helper's boundary, as a stand-in: what it was asked, and the answers a real helper gives.
#[derive(Default)]
struct FakeHelper {
    declines: bool,
    started: Vec<ApprovedPlan>,
    staged: Vec<(u64, Hex32, Vec<u8>)>,
    asked: Vec<BrokerRequest>,
    bye: bool,
    /// After an install, Windows' list shows these (the fake machine reads them).
    shows: std::rc::Rc<RefCell<Vec<RawEntry>>>,
    installs_also: Option<RawEntry>,
    /// What the helper answers for an install (none: done, with no product recorded).
    install_outcome: Option<Outcome>,
    /// What Windows' list shows after an install, in place of the app's own entry.
    shows_after: Option<RawEntry>,
}

fn receipt(r: &BrokerRequest, outcome: Outcome) -> Receipt {
    Receipt {
        request_id: r.request_id.clone(),
        action_id: r.action_id.clone(),
        plan_generation: r.plan_generation,
        plan_digest: r.plan_digest,
        idempotency: Hex32([1; 32]),
        outcome,
        journal: 1,
        recorded: true,
        replayed: false,
        mac: Hex32([0; 32]),
    }
}

fn raw(key: &str, name: &str, version: &str, code: Option<&str>) -> RawEntry {
    let mut values = BTreeMap::new();
    values.insert("displayname".into(), RegValue::Text(name.into()));
    values.insert("displayversion".into(), RegValue::Text(version.into()));
    values.insert("publisher".into(), RegValue::Text("PCTwin Lab Ltd".into()));
    if code.is_some() {
        values.insert("windowsinstaller".into(), RegValue::Number(1));
    }
    RawEntry {
        view: RegistryView::Machine,
        key: code.unwrap_or(key).into(),
        values,
    }
}

impl Helper for FakeHelper {
    fn start(&mut self, plan: &ApprovedPlan) -> Result<(), HelperDown> {
        self.started.push(plan.clone());
        if self.declines {
            Err(HelperDown::Declined)
        } else {
            Ok(())
        }
    }
    fn stage(
        &mut self,
        artifact: u64,
        sha256: Hex32,
        _size: u64,
        from: &mut dyn Read,
    ) -> Result<(), String> {
        let mut bytes = Vec::new();
        from.read_to_end(&mut bytes).map_err(|e| e.to_string())?;
        self.staged.push((artifact, sha256, bytes));
        Ok(())
    }
    fn act(&mut self, request: &BrokerRequest) -> Result<Acted, String> {
        self.asked.push(request.clone());
        // The real helper refuses a request that is not exactly the sealed plan's operation.
        let plan = self.started.last().unwrap();
        if request.plan_digest != plan.digest()
            || plan.operation(&request.action_id).map(|o| &o.action) != Some(&request.action)
        {
            return Ok(Acted::Denied(Denial::NotInPlan));
        }
        Ok(Acted::Done(Box::new(match &request.action {
            Action::CreateRestorePoint(_) => receipt(
                request,
                Outcome::RestorePoint(RestorePointResult::Made {
                    at: "20261028090000.000000-000".into(),
                    number: 7,
                }),
            ),
            Action::InstallVerifiedPackage(_) => {
                let mut shows = self.shows.borrow_mut();
                shows.push(
                    self.shows_after
                        .clone()
                        .unwrap_or_else(|| raw("LabWriter", "Lab Writer", "2.1.0", Some(CODE))),
                );
                if let Some(extra) = &self.installs_also {
                    shows.push(extra.clone());
                }
                receipt(
                    request,
                    self.install_outcome.clone().unwrap_or(Outcome::Done {
                        restart_needed: false,
                    }),
                )
            }
            _ => receipt(request, Outcome::Failed { code: 1 }),
        })))
    }
    fn bye(&mut self) {
        self.bye = true;
    }
}

struct FakeMachine(std::rc::Rc<RefCell<Vec<RawEntry>>>);

impl Machine for FakeMachine {
    fn entries(&self) -> Vec<RawEntry> {
        self.0.borrow().clone()
    }
}

fn sha(bytes: &[u8]) -> [u8; 32] {
    sha2::Sha256::digest(bytes).into()
}

struct Run {
    results: Vec<pctwin_fullmove::run::AppResult>,
    point: Option<RestorePoint>,
    helper: FakeHelper,
    winget: FakeWinget,
}

fn run(winget: FakeWinget, helper: FakeHelper) -> Run {
    run_entry(winget, helper, entry(app()))
}

fn run_entry(winget: FakeWinget, mut helper: FakeHelper, e: AppEntry) -> Run {
    let work = tempfile::tempdir().unwrap();
    let machine = FakeMachine(helper.shows.clone());
    let mut n = 0u8;
    let mut nonce = || {
        n += 1;
        Some([n; 16])
    };
    let apps = [e];
    let refs: Vec<&AppEntry> = apps.iter().collect();
    let (results, point) = {
        let mut chain = WingetInstaller::new(
            PathBuf::from("C:\\winget.exe"),
            "x64",
            work.path().join("downloads"),
            &winget,
            &mut helper,
            &machine,
            &mut nonce,
        )
        .for_move(MOVE);
        let results = chain.install(&refs, &mut || {});
        let point = chain.asked();
        (results, point)
    };
    Run {
        results,
        point,
        helper,
        winget,
    }
}

#[test]
fn one_prompt_for_the_sealed_plan_the_exact_bytes_staged_and_reinstalled_only_when_windows_shows_it()
 {
    let r = run(
        FakeWinget {
            manifest: manifest(CODE, Some(sha(INSTALLER))),
            bytes: INSTALLER.to_vec(),
            calls: Default::default(),
        },
        FakeHelper::default(),
    );
    assert_eq!(
        r.results[0].result.status(),
        ItemStatus::Reinstalled,
        "{:?}",
        r.results
    );
    // One prompt: the helper was started once, for a plan holding the restore point first, then
    // the install naming the exact bytes and the old laptop's maker.
    assert_eq!(r.helper.started.len(), 1);
    let plan = &r.helper.started[0];
    assert!(matches!(
        plan.operations[0].action,
        Action::CreateRestorePoint(_)
    ));
    let Action::InstallVerifiedPackage(p) = &plan.operations[1].action else {
        panic!("{plan:?}")
    };
    assert_eq!(p.sha256, Hex32(sha(INSTALLER)));
    assert_eq!(p.maker.certificate, pctwin_maker::Sha256([4; 32]));
    // The bytes the helper got are the downloaded bytes.
    assert_eq!(r.helper.staged[0].2, INSTALLER);
    // The restore point's honest answer, before the install; the session ends with the move.
    assert!(matches!(
        r.helper.asked[0].action,
        Action::CreateRestorePoint(_)
    ));
    assert!(matches!(
        r.point,
        Some(RestorePoint::Made { number: 7, .. })
    ));
    assert!(r.helper.bye);
    // winget ran as the person, downloading exactly the plan's package for this processor.
    let download = &r.winget.calls.lock().unwrap()[0];
    assert!(download.contains(&OsString::from("Lab.Writer")));
    assert!(download.contains(&OsString::from("x64")));
}

/// Gap 1 (complications audit): an app the catalogue knows needs a sign-in or its licence after
/// installing is installed and checked, but never a bare "reinstalled": it needs the person, with
/// the step on its to-do list, and undo still lists it as installed by PCTwin.
#[test]
fn an_installed_app_that_needs_a_sign_in_or_its_licence_says_so_and_is_not_a_bare_reinstalled() {
    let good = || FakeWinget {
        manifest: manifest(CODE, Some(sha(INSTALLER))),
        bytes: INSTALLER.to_vec(),
        calls: Default::default(),
    };
    let mut signs_in = entry(app());
    signs_in.plan.card.sign_in = true;
    let r = run_entry(good(), FakeHelper::default(), signs_in);
    let done = &r.results[0];
    assert_eq!(done.result, ItemResult::because(Reason::SignIn), "{done:?}");
    assert!(done.todo.contains(&Todo::SignIn), "{done:?}");
    assert!(done.installed);

    let mut licensed = entry(app());
    licensed.plan.card.sign_in = true;
    licensed.plan.card.licence_key = true;
    let r = run_entry(good(), FakeHelper::default(), licensed);
    let done = &r.results[0];
    assert_eq!(
        done.result,
        ItemResult::because(Reason::Activation),
        "{done:?}"
    );
    assert!(done.todo.contains(&Todo::LicenceKey) && done.todo.contains(&Todo::SignIn));
    assert!(done.installed);

    // With nothing known after, it is reinstalled, with nothing to do.
    let r = run(good(), FakeHelper::default());
    assert_eq!(r.results[0].result.status(), ItemStatus::Reinstalled);
    assert!(r.results[0].todo.is_empty() && r.results[0].installed);
}

#[test]
fn a_declined_prompt_installs_nothing_and_says_so() {
    let r = run(
        FakeWinget {
            manifest: manifest(CODE, None),
            bytes: INSTALLER.to_vec(),
            calls: Default::default(),
        },
        FakeHelper {
            declines: true,
            ..FakeHelper::default()
        },
    );
    assert_eq!(r.results[0].result.reason(), Some(Reason::Permission));
    assert!(r.helper.staged.is_empty() && r.helper.asked.is_empty());
    assert_eq!(
        r.point,
        Some(RestorePoint::NotMade {
            why: "not made: you chose not to".into()
        })
    );
}

#[test]
fn another_app_or_swapped_bytes_never_reach_the_helper() {
    // A manifest for a different product.
    let other = run(
        FakeWinget {
            manifest: manifest("{00000000-0000-0000-0000-000000000000}", None),
            bytes: INSTALLER.to_vec(),
            calls: Default::default(),
        },
        FakeHelper::default(),
    );
    assert_eq!(
        other.results[0].result.reason(),
        Some(Reason::ManualInstall)
    );
    assert!(
        other.helper.started.is_empty(),
        "no prompt for nothing to install"
    );
    assert_eq!(other.point, None);
    // Bytes that are not the manifest's.
    let swapped = run(
        FakeWinget {
            manifest: manifest(CODE, Some(sha(b"the real installer"))),
            bytes: INSTALLER.to_vec(),
            calls: Default::default(),
        },
        FakeHelper::default(),
    );
    assert_eq!(
        swapped.results[0].result.reason(),
        Some(Reason::NotVerified)
    );
    assert!(swapped.helper.staged.is_empty());
}

#[test]
fn an_installer_that_slips_in_another_program_is_not_reinstalled() {
    let r = run(
        FakeWinget {
            manifest: manifest(CODE, None),
            bytes: INSTALLER.to_vec(),
            calls: Default::default(),
        },
        FakeHelper {
            installs_also: Some(raw("Toolbar", "Shopping Toolbar", "1.0", None)),
            ..FakeHelper::default()
        },
    );
    assert_eq!(r.results[0].result.reason(), Some(Reason::FinishByHand));
    assert!(
        r.results[0]
            .todo
            .iter()
            .any(|t| matches!(t, Todo::Extras(e) if e == &["Shopping Toolbar".to_string()]))
    );
}

#[test]
fn winget_is_searched_by_the_product_code_for_the_plan() {
    let w = FakeWinget {
        manifest: String::new(),
        bytes: vec![],
        calls: Default::default(),
    };
    let a = app();
    let hints = winget_hints(&[&a], &w, std::path::Path::new("C:\\winget.exe"));
    let got = hints.values().next().cloned().unwrap_or_default();
    assert_eq!(got, vec![("Lab.Writer".to_string(), WingetBy::ProductCode)]);
}

fn installed(code: &str, version: &str) -> Outcome {
    installed_for(code, version, MOVE)
}

fn installed_for(code: &str, version: &str, for_move: [u8; 16]) -> Outcome {
    Outcome::Installed {
        restart_needed: false,
        product: InstalledProduct {
            code: ProductCode::parse(code).unwrap(),
            version: ProductVersion::parse(version).unwrap(),
            installed_on: InstallDate::parse("20261028").unwrap(),
        },
        for_move: Hex16(for_move),
    }
}

fn good_winget() -> FakeWinget {
    FakeWinget {
        manifest: manifest(CODE, Some(sha(INSTALLER))),
        bytes: INSTALLER.to_vec(),
        calls: Default::default(),
    }
}

/// Step 1's acceptance (uninstall on undo): a Windows Installer install names the manifest's
/// product code in the plan; when the helper saw its install make that product appear, and
/// Windows' list agrees (absent before, present after), the app keeps it for undo with the
/// install's own key from its receipt.
#[test]
fn an_msi_install_the_helper_saw_appear_is_kept_for_undo() {
    let r = run(
        good_winget(),
        FakeHelper {
            install_outcome: Some(installed(CODE, "2.1.0")),
            ..FakeHelper::default()
        },
    );
    let Action::InstallVerifiedPackage(p) = &r.helper.started[0].operations[1].action else {
        panic!()
    };
    assert_eq!(p.product_code, ProductCode::parse(CODE));
    // The install names its move (review finding F3).
    assert_eq!(p.for_move, Some(Hex16(MOVE)));
    let done = &r.results[0];
    assert_eq!(done.result.status(), ItemStatus::Reinstalled);
    let kept = done.removable.as_deref().expect("kept for undo");
    assert_eq!(kept.product.code, ProductCode::parse(CODE).unwrap());
    assert_eq!(kept.product.version.as_str(), "2.1.0");
    assert_eq!(kept.installed_by, Hex32([1; 32]));
}

/// Step 1's acceptance: no record unless the helper recorded this very product and Windows'
/// list shows it absent before and present after; an installer program (EXE) records none.
#[test]
fn nothing_is_kept_for_undo_unless_the_helper_and_the_list_agree() {
    // The helper recorded no product (it could not see one appear).
    let r = run(good_winget(), FakeHelper::default());
    assert!(r.results[0].installed);
    assert_eq!(r.results[0].removable, None);
    // The helper recorded another product than the plan named.
    let r = run(
        good_winget(),
        FakeHelper {
            install_outcome: Some(installed("{11111111-2222-3333-4444-555555555555}", "2.1.0")),
            ..FakeHelper::default()
        },
    );
    assert_eq!(r.results[0].removable, None);
    // The helper recorded it for another move (review finding F3).
    let r = run(
        good_winget(),
        FakeHelper {
            install_outcome: Some(installed_for(CODE, "2.1.0", [0x99; 16])),
            ..FakeHelper::default()
        },
    );
    assert!(r.results[0].installed);
    assert_eq!(r.results[0].removable, None);
    // The list showed the product before the install.
    let helper = FakeHelper {
        install_outcome: Some(installed(CODE, "2.1.0")),
        ..FakeHelper::default()
    };
    helper
        .shows
        .borrow_mut()
        .push(raw("LabWriter", "Lab Writer", "2.0", Some(CODE)));
    let r = run(good_winget(), helper);
    assert_eq!(r.results[0].removable, None);
    // An installer program: the plan names no product code, and nothing is kept, whatever the
    // helper says.
    let inno = FakeWinget {
        manifest: manifest(CODE, Some(sha(INSTALLER)))
            .replace("InstallerType: msi", "InstallerType: inno"),
        bytes: INSTALLER.to_vec(),
        calls: Default::default(),
    };
    let r = run(
        inno,
        FakeHelper {
            install_outcome: Some(installed(CODE, "2.1.0")),
            ..FakeHelper::default()
        },
    );
    let Action::InstallVerifiedPackage(p) = &r.helper.started[0].operations[1].action else {
        panic!()
    };
    assert_eq!(p.product_code, None);
    assert_eq!(r.results[0].removable, None);
    // The app is found by its name, but no entry has the product code: nothing kept.
    let r = run(
        good_winget(),
        FakeHelper {
            install_outcome: Some(installed(CODE, "2.1.0")),
            shows_after: Some(raw("LabWriter", "Lab Writer", "2.1.0", None)),
            ..FakeHelper::default()
        },
    );
    assert!(r.results[0].installed, "{:?}", r.results[0]);
    assert_eq!(r.results[0].removable, None);
    // The product code is listed, but hidden from "Installed apps", so the app is not found
    // installed: nothing kept.
    let mut hidden = raw("LabWriter", "Lab Writer", "2.1.0", Some(CODE));
    hidden
        .values
        .insert("systemcomponent".into(), RegValue::Number(1));
    let r = run(
        good_winget(),
        FakeHelper {
            install_outcome: Some(installed(CODE, "2.1.0")),
            shows_after: Some(hidden),
            ..FakeHelper::default()
        },
    );
    assert!(!r.results[0].installed, "{:?}", r.results[0]);
    assert_eq!(r.results[0].removable, None);
    // A failed install keeps nothing.
    let r = run(
        good_winget(),
        FakeHelper {
            install_outcome: Some(Outcome::Failed { code: 1603 }),
            ..FakeHelper::default()
        },
    );
    assert_eq!(r.results[0].removable, None);
}
