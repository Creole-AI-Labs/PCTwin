//! The real install chain on Windows (2.1 joined to the move): for each app the plan installs,
//!
//! 1. **found by its maker**: winget is searched as the person, by the app's product code or its
//!    exact name ([`winget_hints`], before the plan is shown, so the plan says where each comes
//!    from);
//! 2. **downloaded as the person** with winget into PCTwin's own private folder; the manifest must
//!    describe the very app the old laptop had (its product code, or its name and publisher),
//!    never an older version, and its typed installer kind must be one PCTwin installs quietly;
//! 3. **one administrator prompt**: the helper is started once for the whole sealed plan (the
//!    restore point first, then every install, each naming the exact bytes and the old laptop's
//!    maker); the helper copies each download into its own folder, checks the maker itself, and
//!    installs, journaled, never twice;
//! 4. **what is really there**: Windows' own list is read again; the app must be there, at the
//!    person's version or newer, with nothing else slipped in, for `reinstalled`.
//!
//! Anything that does not hold ends needs-action or failed with its reason, never "reinstalled".
//! The helper itself sits behind [`Helper`] and Windows' list behind [`Machine`], so the chain is
//! proven up to the helper's boundary without installing anything (tests/install.rs).

use std::collections::HashMap;
use std::io::Read;
use std::path::{Path, PathBuf};

use pctwin_apps::after::{self, Installed, Opened};
use pctwin_apps::found::windows::RawEntry;
use pctwin_apps::routes::WingetBy;
use pctwin_apps::routes::winget::{self, Manifest};
use pctwin_helper::Acted;
use pctwin_helper::broker::actions::{
    Action, ApprovedPlan, CreateRestorePoint, Hex16, Hex32, Id, PlannedOperation,
};
use pctwin_helper::broker::receipt::Outcome;
use pctwin_helper::broker::restore::{NotMadeReason, RestorePointResult};
use pctwin_helper::broker::{BrokerRequest, PROTOCOL};
use pctwin_journal::RestorePoint;
use pctwin_maker::run::Runner;
use pctwin_record::{AppMaker, FoundApp, FoundWhere};
use pctwin_types::{ItemResult, Reason};
use sha2::Digest;

use crate::plan::AppEntry;
use crate::run::{AppResult, Installer};

/// The most winget searches at once.
const SEARCHES_AT_ONCE: usize = 3;
/// How long searching may take before the plan is shown: apps not searched by then are planned
/// with nowhere checked (needs-action, said), never held up.
pub const SEARCH_BUDGET: std::time::Duration = std::time::Duration::from_secs(150);

/// Why the helper is not there.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HelperDown {
    /// The person said no at the system's prompt.
    Declined,
    /// It could not be started or reached.
    Failed,
}

/// The administrator helper, as the chain uses it: started once for the sealed plan.
pub trait Helper {
    /// Starts the helper for `plan` (the system's own prompt, once) and gives it the plan.
    fn start(&mut self, plan: &ApprovedPlan) -> Result<(), HelperDown>;
    /// Sends one download's exact bytes to the helper's own folder.
    fn stage(
        &mut self,
        artifact: u64,
        sha256: Hex32,
        size: u64,
        from: &mut dyn Read,
    ) -> Result<(), String>;
    fn act(&mut self, request: &BrokerRequest) -> Result<Acted, String>;
    /// The helper's work for this move is over.
    fn bye(&mut self);
}

/// Windows' own list of apps installed for everyone, as it is now.
pub trait Machine {
    fn entries(&self) -> Vec<RawEntry>;
}

/// The winget IDs to try for each app, searched as the person, at most a few at a time (key:
/// [`hint_key`]). An app with no maker to check, or not from Windows' own list, is not searched.
pub fn winget_hints(
    apps: &[&FoundApp],
    runner: &(dyn Runner + Sync),
    winget: &Path,
) -> HashMap<String, Vec<(String, WingetBy)>> {
    let wanted: Vec<&FoundApp> = apps
        .iter()
        .copied()
        .filter(|a| {
            matches!(a.maker, AppMaker::Windows(_))
                && matches!(a.found, FoundWhere::WindowsEntry { .. })
        })
        .collect();
    let mut out = HashMap::new();
    let until = std::time::Instant::now() + SEARCH_BUDGET;
    for chunk in wanted.chunks(SEARCHES_AT_ONCE) {
        if std::time::Instant::now() >= until {
            break;
        }
        let found: Vec<(String, Vec<(String, WingetBy)>)> = std::thread::scope(|s| {
            let handles: Vec<_> = chunk
                .iter()
                .map(|a| s.spawn(move || (hint_key(a), search(a, runner, winget))))
                .collect();
            handles.into_iter().filter_map(|h| h.join().ok()).collect()
        });
        out.extend(found);
    }
    out
}

/// The key an app's hints are kept under.
pub fn hint_key(a: &FoundApp) -> String {
    match &a.found {
        FoundWhere::WindowsEntry {
            product_code: Some(c),
            ..
        } => format!("code:{}", c.to_ascii_lowercase()),
        _ => format!(
            "name:{}\u{0}{}",
            a.name,
            a.publisher.as_deref().unwrap_or("")
        ),
    }
}

fn search(app: &FoundApp, runner: &dyn Runner, winget: &Path) -> Vec<(String, WingetBy)> {
    let Some((query, by_code)) = winget::search_query(app) else {
        return Vec::new();
    };
    let call = winget::search_call(winget, &query, by_code);
    match runner.run(&call) {
        Ok(ran) if ran.code == Some(0) => winget::candidates(&ran.stdout, &query)
            .into_iter()
            .map(|id| {
                let by = if by_code {
                    WingetBy::ProductCode
                } else {
                    WingetBy::Name
                };
                (id, by)
            })
            .collect(),
        _ => Vec::new(),
    }
}

/// One download, checked and ready for the helper.
struct Prepared {
    /// Its place in the list of apps.
    at: usize,
    op: PlannedOperation,
    file: PathBuf,
    size: u64,
}

fn manual() -> AppResult {
    AppResult::because(Reason::ManualInstall)
}

fn sha256_of(path: &Path) -> std::io::Result<([u8; 32], u64)> {
    let mut f = std::fs::File::open(path)?;
    let mut h = sha2::Sha256::new();
    let mut buf = vec![0u8; 1 << 16];
    let mut size = 0u64;
    loop {
        let n = f.read(&mut buf)?;
        if n == 0 {
            break;
        }
        size += n as u64;
        h.update(&buf[..n]);
    }
    Ok((h.finalize().into(), size))
}

/// The restore point's answer in the journal's words.
fn journal_point(r: &RestorePointResult) -> RestorePoint {
    let secs = |at: &str| pctwin_helper::broker::restore::wmi_time_secs(at).unwrap_or(0);
    match r {
        RestorePointResult::Made { at, number } => RestorePoint::Made {
            at: secs(at),
            number: *number,
        },
        RestorePointResult::EarlierExists { at, number } => RestorePoint::EarlierExists {
            at: secs(at),
            number: *number,
        },
        RestorePointResult::NotMade { reason } => match reason {
            NotMadeReason::NotWindows => RestorePoint::NotWindows,
            NotMadeReason::ProtectionOff => RestorePoint::NotMade {
                why: "not made: System Protection is turned off on this laptop".into(),
            },
            NotMadeReason::NeedsAdmin => RestorePoint::NotMade {
                why: "not made: it needs an administrator".into(),
            },
            NotMadeReason::Unconfirmed => RestorePoint::NotMade {
                why: "not made: Windows said yes, but no new restore point can be found".into(),
            },
            NotMadeReason::Failed => RestorePoint::NotMade {
                why: "not made: Windows could not make one".into(),
            },
        },
    }
}

/// The chain: winget as the person, then the administrator helper once.
pub struct WingetInstaller<'a> {
    pub winget: PathBuf,
    /// This laptop's processor, as winget names it (`x64`, `x86`, `arm64`).
    pub arch: &'static str,
    /// PCTwin's own private folder for downloads (removed afterwards).
    pub work: PathBuf,
    pub runner: &'a dyn Runner,
    pub helper: &'a mut dyn Helper,
    pub machine: &'a dyn Machine,
    /// The one-use numbers of the helper's requests.
    pub nonce: &'a mut dyn FnMut() -> Option<[u8; 16]>,
    point: Option<RestorePoint>,
    down: Option<HelperDown>,
    plan: Option<ApprovedPlan>,
}

const RESTORE_ID: &str = "restore-point";

impl<'a> WingetInstaller<'a> {
    pub fn new(
        winget: PathBuf,
        arch: &'static str,
        work: PathBuf,
        runner: &'a dyn Runner,
        helper: &'a mut dyn Helper,
        machine: &'a dyn Machine,
        nonce: &'a mut dyn FnMut() -> Option<[u8; 16]>,
    ) -> Self {
        Self {
            winget,
            arch,
            work,
            runner,
            helper,
            machine,
            nonce,
            point: None,
            down: None,
            plan: None,
        }
    }

    /// Downloads and checks `app`'s package (`at`: its place in the list).
    fn prepare(&mut self, at: usize, e: &AppEntry) -> Result<Prepared, AppResult> {
        let id = e.winget().ok_or_else(manual)?;
        let folder = self.work.join(at.to_string());
        std::fs::create_dir_all(&folder).map_err(|_| manual())?;
        let call =
            winget::download_call(&self.winget, id, self.arch, &folder).ok_or_else(manual)?;
        match self.runner.run(&call) {
            Ok(ran) if ran.code == Some(0) => {}
            _ => return Err(manual()),
        }
        let (yaml, file) = winget::downloaded(&folder).map_err(|_| manual())?;
        let text = std::fs::read_to_string(&yaml).map_err(|_| manual())?;
        let m = Manifest::parse(&text).map_err(|_| manual())?;
        // The very app the old laptop had, never an older version.
        winget::confirms(&m, &e.app).ok_or_else(manual)?;
        if !pctwin_apps::version::not_a_downgrade(m.version.as_deref(), e.app.version.as_deref()) {
            return Err(manual());
        }
        let kind = pctwin_apps::install::kind_of(
            m.installer_type.as_deref().unwrap_or(""),
            m.scope.as_deref(),
        )
        .map_err(|_| manual())?;
        let (sha, size) = sha256_of(&file).map_err(|_| manual())?;
        // The manifest's own hash, when it gives one, must be these bytes.
        if m.sha256.is_some_and(|want| want != sha) {
            return Err(AppResult::because(Reason::NotVerified));
        }
        let op =
            pctwin_apps::install::operation(&format!("install.{at}"), at as u64, sha, kind, &e.app)
                .map_err(|_| manual())?;
        Ok(Prepared { at, op, file, size })
    }

    /// Starts the helper once for `ops` (the restore point first), and asks for the restore point.
    fn start(&mut self, ops: Vec<PlannedOperation>) {
        if self.point.is_some() || self.down.is_some() {
            return;
        }
        let Some(restore) = Id::parse(RESTORE_ID) else {
            self.down = Some(HelperDown::Failed);
            return;
        };
        let mut operations = vec![PlannedOperation {
            action_id: restore.clone(),
            action: Action::CreateRestorePoint(CreateRestorePoint {}),
        }];
        operations.extend(ops);
        let plan = ApprovedPlan {
            generation: 1,
            operations,
        };
        if let Err(down) = self.helper.start(&plan) {
            self.down = Some(down);
            self.point = Some(RestorePoint::NotMade {
                why: match down {
                    HelperDown::Declined => "not made: you chose not to".into(),
                    HelperDown::Failed => {
                        "not made: PCTwin could not reach its administrator helper".into()
                    }
                },
            });
            return;
        }
        let point = match self.request(
            &plan,
            &restore,
            Action::CreateRestorePoint(CreateRestorePoint {}),
        ) {
            Ok(Acted::Done(r)) => match &r.outcome {
                Outcome::RestorePoint(p) => journal_point(p),
                _ => RestorePoint::NotMade {
                    why: "not made: PCTwin's administrator helper did not make it".into(),
                },
            },
            _ => RestorePoint::NotMade {
                why: "not made: PCTwin's administrator helper did not make it".into(),
            },
        };
        self.point = Some(point);
        self.plan = Some(plan);
    }

    fn request(&mut self, plan: &ApprovedPlan, id: &Id, action: Action) -> Result<Acted, String> {
        let nonce = (self.nonce)().ok_or("no one-use number")?;
        let request = BrokerRequest {
            protocol: PROTOCOL,
            audience: action.audience(),
            request_id: id.clone(),
            plan_generation: plan.generation,
            plan_digest: plan.digest(),
            action_id: id.clone(),
            action,
            nonce: Hex16(nonce),
        };
        self.helper.act(&request)
    }

    /// Installs one prepared download through the helper, then looks at what is really there.
    fn install_one(&mut self, p: &Prepared, app: &FoundApp) -> AppResult {
        let Some(plan) = self.plan.clone() else {
            // The helper never ran (declined, or it could not start, such as a packages folder
            // it would not trust): nothing was installed; the person installs it.
            return AppResult::because(match self.down {
                Some(HelperDown::Declined) => Reason::Permission,
                _ => Reason::ManualInstall,
            });
        };
        let Action::InstallVerifiedPackage(pkg) = &p.op.action else {
            return manual();
        };
        let staged = std::fs::File::open(&p.file)
            .map_err(|e| e.to_string())
            .and_then(|mut f| self.helper.stage(pkg.artifact, pkg.sha256, p.size, &mut f));
        if staged.is_err() {
            // The helper did not take the download (its own folder refused it): nothing was
            // installed; the person installs it from the maker.
            return AppResult::because(Reason::ManualInstall);
        }
        let before: Vec<String> = self.machine.entries().into_iter().map(|e| e.key).collect();
        let installed = match self.request(&plan, &p.op.action_id, p.op.action.clone()) {
            Ok(Acted::Done(r)) => match r.outcome {
                Outcome::Done { restart_needed } => Installed::Done { restart_needed },
                Outcome::AlreadyInPlace => Installed::AlreadyInPlace,
                Outcome::Failed { .. } | Outcome::Partial { .. } | Outcome::RestorePoint(_) => {
                    Installed::Failed
                }
            },
            Ok(Acted::Denied(_)) => Installed::Refused,
            Err(_) => Installed::Failed,
        };
        let now = self.machine.entries();
        let probe = after::probe(app, pkg.scope, &now);
        let extra = after::extras(&before, &now, probe.key.as_deref());
        let result: ItemResult = after::result(app, installed, &probe, &extra, Opened::NotChecked);
        AppResult {
            result,
            todo: after::checklist(app, false, false, installed, &extra),
        }
    }
}

impl Installer for WingetInstaller<'_> {
    fn asked(&self) -> Option<RestorePoint> {
        self.point.clone()
    }

    fn restore_point(&mut self) -> RestorePoint {
        self.start(Vec::new());
        self.point.clone().unwrap_or(RestorePoint::NotMade {
            why: "not made: PCTwin's administrator helper did not make it".into(),
        })
    }

    fn install(&mut self, apps: &[&AppEntry], done: &mut dyn FnMut()) -> Vec<AppResult> {
        let mut results: Vec<Option<AppResult>> = vec![None; apps.len()];
        let mut ready = Vec::new();
        for (at, e) in apps.iter().enumerate() {
            match self.prepare(at, e) {
                Ok(p) => ready.push(p),
                Err(r) => {
                    results[at] = Some(r);
                    done();
                }
            }
        }
        if !ready.is_empty() {
            self.start(ready.iter().map(|p| p.op.clone()).collect());
            for p in &ready {
                results[p.at] = Some(self.install_one(p, &apps[p.at].app));
                done();
            }
            self.helper.bye();
        }
        let _ = std::fs::remove_dir_all(&self.work);
        results
            .into_iter()
            .map(|r| r.unwrap_or_else(|| AppResult::because(Reason::NotVerified)))
            .collect()
    }
}
