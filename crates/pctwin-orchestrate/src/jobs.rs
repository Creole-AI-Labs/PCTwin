//! The plan that orders the work (Task List 2.6; acceptance T26-006 to T26-009, T26-029,
//! T26-030). Small and deterministic.
//!
//! - **Child jobs with their own scopes** ([`ChildJob`], [`Scope`]): installing one app, moving
//!   a set of one laptop's items, restoring named settings of one person. There is no scope for
//!   "everything" or for administrator rights: a parent cannot ask for one, and a child checks
//!   any grant against what it was approved for ([`Grant::check_within`]).
//! - **Sealed before it runs** ([`seal`]): every dependency must exist, cycles are found and
//!   named, the plan has a size limit, and the order is fixed (so the same plan always runs the
//!   same way).
//! - **Bounded scheduling** ([`Scheduler`]): only jobs whose dependencies finished start, at most
//!   as many at once as power and memory allow ([`concurrency`]); a critical battery pauses; the
//!   person's essentials go first; no job is ever dropped.
//! - **Results are checked** ([`Scheduler::report`]): a result for another job, laptop, worker or
//!   older session, or for a job whose contents changed, is refused, and each is taken once.
//! - **Partial results are exact** ([`Scheduler::missing`]): a failed child blocks only what
//!   depends on it; every job that did not finish is named with its exact scope and why.
//!
//! Inside one program these checks are types and plain comparisons, not signatures (ADOPTION
//! change 1): the digest only tells one job's contents from another's.

use std::collections::{BTreeMap, BTreeSet};

use pctwin_record::{ItemId, LaptopId};

/// Most jobs in one plan.
pub const MAX_JOBS: usize = 10_000;
/// Most jobs running at once, whatever the resources.
pub const MOST_AT_ONCE: usize = 4;

/// A job's name in the plan.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct JobId(pub String);

/// Which part of PCTwin does a job.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Worker {
    /// Installing apps (2.1).
    Install,
    /// Restoring settings (2.4).
    Settings,
    /// Moving data (2.5) and files.
    Data,
}

/// Exactly what one job may touch.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Scope {
    /// These items of one old laptop's approved plan.
    Items {
        laptop: LaptopId,
        items: BTreeSet<ItemId>,
    },
    /// Installing one app.
    App { name: String },
    /// These named settings of one person on one old laptop.
    Settings {
        laptop: LaptopId,
        account_id: String,
        names: BTreeSet<String>,
    },
}

impl Scope {
    fn is_empty(&self) -> bool {
        match self {
            Scope::Items { items, .. } => items.is_empty(),
            Scope::App { name } => name.is_empty(),
            Scope::Settings { names, .. } => names.is_empty(),
        }
    }

    /// Whether this scope lies within `approved`.
    fn within(&self, approved: &Scope) -> bool {
        match (self, approved) {
            (
                Scope::Items { laptop, items },
                Scope::Items {
                    laptop: l,
                    items: allowed,
                },
            ) => laptop == l && items.is_subset(allowed),
            (Scope::App { name }, Scope::App { name: allowed }) => name == allowed,
            (
                Scope::Settings {
                    laptop,
                    account_id,
                    names,
                },
                Scope::Settings {
                    laptop: l,
                    account_id: a,
                    names: allowed,
                },
            ) => laptop == l && account_id == a && names.is_subset(allowed),
            _ => false,
        }
    }

    fn digest_into(&self, h: &mut blake3::Hasher) {
        let field = |h: &mut blake3::Hasher, b: &[u8]| {
            h.update(&(b.len() as u64).to_le_bytes());
            h.update(b);
        };
        match self {
            Scope::Items { laptop, items } => {
                field(h, b"items");
                field(h, laptop.to_hex().as_bytes());
                for i in items {
                    field(h, i.to_hex().as_bytes());
                }
            }
            Scope::App { name } => {
                field(h, b"app");
                field(h, name.as_bytes());
            }
            Scope::Settings {
                laptop,
                account_id,
                names,
            } => {
                field(h, b"settings");
                field(h, laptop.to_hex().as_bytes());
                field(h, account_id.as_bytes());
                for n in names {
                    field(h, n.as_bytes());
                }
            }
        }
    }
}

/// One child job.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChildJob {
    pub id: JobId,
    pub worker: Worker,
    /// The old laptop it works for, and the session with it.
    pub source: LaptopId,
    pub session: u64,
    pub scope: Scope,
    /// Jobs that must finish first (an import after its app).
    pub after: Vec<JobId>,
    /// One of the person's essentials: goes first.
    pub essential: bool,
}

impl ChildJob {
    /// A digest of everything the job is: a result names it, so a result for a job that has
    /// since changed is told apart.
    pub fn digest(&self) -> [u8; 32] {
        let mut h = blake3::Hasher::new();
        h.update(b"pctwin/v1/child-job\0");
        let field = |h: &mut blake3::Hasher, b: &[u8]| {
            h.update(&(b.len() as u64).to_le_bytes());
            h.update(b);
        };
        field(&mut h, self.id.0.as_bytes());
        field(&mut h, format!("{:?}", self.worker).as_bytes());
        field(&mut h, self.source.to_hex().as_bytes());
        field(&mut h, &self.session.to_le_bytes());
        self.scope.digest_into(&mut h);
        for a in &self.after {
            field(&mut h, a.0.as_bytes());
        }
        *h.finalize().as_bytes()
    }
}

/// Why a plan cannot be sealed.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum PlanError {
    #[error("these steps wait for each other in a circle: {0:?}")]
    Cycle(Vec<JobId>),
    #[error("a step waits for one that is not in the plan")]
    UnknownDependency { job: JobId, needs: JobId },
    #[error("a step is in the plan twice")]
    JobTwice(JobId),
    #[error("a step has nothing to do")]
    EmptyScope(JobId),
    #[error("the plan has too many steps")]
    TooBig,
}

/// A sealed plan: valid, and in a fixed order.
#[derive(Debug, Clone)]
pub struct Plan {
    jobs: BTreeMap<JobId, ChildJob>,
    order: Vec<JobId>,
}

impl Plan {
    /// Every job, each after what it needs; ties in name order.
    pub fn order(&self) -> &[JobId] {
        &self.order
    }
}

/// Checks and orders `jobs`.
pub fn seal(jobs: Vec<ChildJob>) -> Result<Plan, PlanError> {
    if jobs.len() > MAX_JOBS {
        return Err(PlanError::TooBig);
    }
    let mut by_id = BTreeMap::new();
    for j in jobs {
        if j.scope.is_empty() {
            return Err(PlanError::EmptyScope(j.id));
        }
        if by_id.contains_key(&j.id) {
            return Err(PlanError::JobTwice(j.id));
        }
        by_id.insert(j.id.clone(), j);
    }
    for j in by_id.values() {
        if let Some(missing) = j.after.iter().find(|a| !by_id.contains_key(*a)) {
            return Err(PlanError::UnknownDependency {
                job: j.id.clone(),
                needs: missing.clone(),
            });
        }
    }
    // Kahn's method, always taking the first ready job by name.
    let mut waiting: BTreeMap<&JobId, BTreeSet<&JobId>> = by_id
        .values()
        .map(|j| (&j.id, j.after.iter().collect()))
        .collect();
    let mut order = Vec::new();
    while let Some(next) = waiting
        .iter()
        .find(|(_, needs)| needs.is_empty())
        .map(|(id, _)| (*id).clone())
    {
        waiting.remove(&next);
        for needs in waiting.values_mut() {
            needs.remove(&next);
        }
        order.push(next);
    }
    if !waiting.is_empty() {
        return Err(PlanError::Cycle(cycle_members(&waiting)));
    }
    Ok(Plan { jobs: by_id, order })
}

/// The jobs on a cycle among those left waiting (not those merely waiting behind one).
fn cycle_members(waiting: &BTreeMap<&JobId, BTreeSet<&JobId>>) -> Vec<JobId> {
    let reaches = |from: &JobId, to: &JobId| {
        let mut seen = BTreeSet::new();
        let mut stack: Vec<&JobId> = waiting.get(from).into_iter().flatten().copied().collect();
        while let Some(j) = stack.pop() {
            if j == to {
                return true;
            }
            if seen.insert(j) {
                stack.extend(waiting.get(j).into_iter().flatten().copied());
            }
        }
        false
    };
    waiting
        .keys()
        .filter(|j| reaches(j, j))
        .map(|j| (*j).clone())
        .collect()
}

/// Power and memory right now.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Resources {
    pub on_battery: bool,
    pub battery_percent: Option<f32>,
    pub memory_free_bytes: Option<u64>,
}

/// On battery at or below this, one job at a time ("plug in this laptop").
pub const LOW_BATTERY: f32 = 20.0;
/// On battery at or below this, nothing starts (the move pauses safely).
pub const PAUSE_BATTERY: f32 = 8.0;
/// Below this much free memory (or unknown), one job at a time.
pub const LOW_MEMORY: u64 = 1024 * 1024 * 1024;

/// How many jobs may run at once, at most `most`.
pub fn concurrency(r: &Resources, most: usize) -> usize {
    let battery_low = match (r.on_battery, r.battery_percent) {
        (false, _) => false,
        (true, Some(p)) if p <= PAUSE_BATTERY => return 0,
        (true, Some(p)) => p <= LOW_BATTERY,
        (true, None) => true,
    };
    let memory_low = r.memory_free_bytes.is_none_or(|m| m < LOW_MEMORY);
    if battery_low || memory_low {
        most.min(1)
    } else {
        most
    }
}

/// Why a result was refused, or a grant is wider than allowed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum ResultError {
    #[error("no such step in the plan")]
    UnknownJob,
    #[error("that step is not running")]
    NotRunning,
    #[error("the result is for another step")]
    WrongJob,
    #[error("the result comes from another laptop")]
    WrongSource,
    #[error("the result comes from an older session")]
    OldSession,
    #[error("the result comes from another part of PCTwin")]
    WrongWorker,
    #[error("the grant is wider than what was approved")]
    WiderThanApproved,
}

/// What a started job is given: its own scope, and its digest to name in its result.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Grant {
    pub job: JobId,
    pub digest: [u8; 32],
    pub scope: Scope,
}

impl Grant {
    /// The child's own check: the grant must lie within what it was approved for.
    pub fn check_within(&self, approved: &Scope) -> Result<(), ResultError> {
        if self.scope.within(approved) {
            Ok(())
        } else {
            Err(ResultError::WiderThanApproved)
        }
    }

    /// A grant of `scope` alone, for a child's own tests.
    pub fn for_test(scope: Scope) -> Self {
        Self {
            job: JobId("test".into()),
            digest: [0; 32],
            scope,
        }
    }
}

/// A child's result for one job.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChildResult {
    pub job: JobId,
    pub worker: Worker,
    pub source: LaptopId,
    pub session: u64,
    pub digest: [u8; 32],
    /// It finished (its own checks passed).
    pub finished: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum State {
    Waiting,
    Running,
    Done,
    Failed,
    SourceUnavailable,
}

/// Why a job did not finish.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Why {
    /// It ran and did not finish.
    Failed,
    /// Its old laptop is not connected.
    SourceUnavailable,
    /// It waits for this job, which did not finish.
    WaitingFor(JobId),
    /// It never started (the move stopped first).
    NotStarted,
}

/// One job that did not finish, with its exact scope.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Missing {
    pub job: JobId,
    pub scope: Scope,
    pub why: Why,
}

impl Missing {
    /// The plain words, naming the old laptop as `laptop` (its label).
    pub fn words(&self, laptop: &str) -> String {
        let what = match &self.scope {
            Scope::Items { items, .. } if items.len() == 1 => "1 item".to_string(),
            Scope::Items { items, .. } => format!("{} items", items.len()),
            Scope::App { name } => format!("The app {name}"),
            Scope::Settings { names, .. } if names.len() == 1 => "1 setting".to_string(),
            Scope::Settings { names, .. } => format!("{} settings", names.len()),
        };
        let plural = !matches!(&self.scope, Scope::App { .. }) && !what.starts_with("1 ");
        match (&self.why, &self.scope) {
            (Why::SourceUnavailable, _) => format!(
                "{what} from {laptop} did not move: that laptop is not connected. Connect it to the same Wi-Fi, keep it awake, and choose Try again."
            ),
            (Why::Failed, Scope::App { .. }) => format!(
                "{what} from {laptop} did not finish. Choose Try again, or see more details."
            ),
            (Why::Failed, _) => {
                format!("{what} from {laptop} did not move. Choose Try again, or see more details.")
            }
            (Why::WaitingFor(_), _) => format!(
                "{what} from {laptop} did not move: {} for another step that did not finish. Fix that step first, then choose Try again.",
                if plural { "they wait" } else { "it waits" }
            ),
            (Why::NotStarted, _) => format!(
                "{what} from {laptop} did not move: the move stopped before {}. Choose Try again.",
                if plural { "they started" } else { "it started" }
            ),
        }
    }
}

/// Runs a sealed plan's jobs as resources allow.
#[derive(Debug, Clone)]
pub struct Scheduler {
    plan: Plan,
    state: BTreeMap<JobId, State>,
}

impl Scheduler {
    pub fn new(plan: Plan) -> Self {
        let state = plan
            .order
            .iter()
            .map(|j| (j.clone(), State::Waiting))
            .collect();
        Self { plan, state }
    }

    pub fn running(&self) -> usize {
        self.state
            .values()
            .filter(|s| **s == State::Running)
            .count()
    }

    pub fn is_done(&self, job: &JobId) -> bool {
        self.state.get(job) == Some(&State::Done)
    }

    pub fn done_count(&self) -> usize {
        self.state.values().filter(|s| **s == State::Done).count()
    }

    fn ready(&self, job: &ChildJob) -> bool {
        // A job of a laptop that went away is no longer waiting (see `source_unavailable`).
        self.state[&job.id] == State::Waiting
            && job.after.iter().all(|a| self.state[a] == State::Done)
    }

    /// Starts what may start now: ready jobs, essentials first, then in plan order, up to the
    /// number `resources` allow. Each is given exactly its own scope.
    pub fn next(&mut self, resources: &Resources) -> Vec<Grant> {
        let room = concurrency(resources, MOST_AT_ONCE).saturating_sub(self.running());
        let mut ready: Vec<&ChildJob> = self
            .plan
            .order
            .iter()
            .map(|id| &self.plan.jobs[id])
            .filter(|j| self.ready(j))
            .collect();
        // Stable: essentials first, plan order kept within each.
        ready.sort_by_key(|j| !j.essential);
        let grants: Vec<Grant> = ready
            .into_iter()
            .take(room)
            .map(|j| Grant {
                job: j.id.clone(),
                digest: j.digest(),
                scope: j.scope.clone(),
            })
            .collect();
        for g in &grants {
            self.state.insert(g.job.clone(), State::Running);
        }
        grants
    }

    /// The result a child would give for `job` (its own fields), finished or not.
    pub fn result_for(&self, job: &JobId, finished: bool) -> ChildResult {
        let j = &self.plan.jobs[job];
        ChildResult {
            job: job.clone(),
            worker: j.worker,
            source: j.source,
            session: j.session,
            digest: j.digest(),
            finished,
        }
    }

    /// Takes a child's result, once, only if it is for the running job it names, from its
    /// laptop's current session and its own worker, with the job's own digest.
    pub fn report(&mut self, result: &ChildResult) -> Result<(), ResultError> {
        let job = self
            .plan
            .jobs
            .get(&result.job)
            .ok_or(ResultError::UnknownJob)?;
        if self.state[&job.id] != State::Running {
            return Err(ResultError::NotRunning);
        }
        if result.source != job.source {
            return Err(ResultError::WrongSource);
        }
        if result.session != job.session {
            return Err(ResultError::OldSession);
        }
        if result.worker != job.worker {
            return Err(ResultError::WrongWorker);
        }
        if result.digest != job.digest() {
            return Err(ResultError::WrongJob);
        }
        let next = if result.finished {
            State::Done
        } else {
            State::Failed
        };
        self.state.insert(job.id.clone(), next);
        Ok(())
    }

    /// An old laptop went away: its jobs that have not finished are unavailable, never done.
    pub fn source_unavailable(&mut self, laptop: &LaptopId) {
        for (id, j) in &self.plan.jobs {
            let s = self.state.get_mut(id).expect("every job has a state");
            if j.source == *laptop && matches!(*s, State::Waiting | State::Running) {
                *s = State::SourceUnavailable;
            }
        }
    }

    /// Nothing is running and nothing more can start.
    pub fn finished(&self) -> bool {
        self.running() == 0 && !self.plan.jobs.values().any(|j| self.ready(j))
    }

    /// Every job that did not finish, in name order, with its exact scope and why.
    pub fn missing(&self) -> Vec<Missing> {
        self.plan
            .jobs
            .values()
            .filter_map(|j| {
                let why = match self.state[&j.id] {
                    State::Done => return None,
                    State::Failed => Why::Failed,
                    State::SourceUnavailable => Why::SourceUnavailable,
                    State::Waiting | State::Running => {
                        match j.after.iter().find(|a| self.state[*a] != State::Done) {
                            Some(a) => Why::WaitingFor(a.clone()),
                            None => Why::NotStarted,
                        }
                    }
                };
                Some(Missing {
                    job: j.id.clone(),
                    scope: j.scope.clone(),
                    why,
                })
            })
            .collect()
    }
}
