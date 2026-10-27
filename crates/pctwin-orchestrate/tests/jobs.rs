//! The plan that orders the work (Task List 2.6; acceptance T26-006 to T26-009, T26-029,
//! T26-030): child jobs with their own scopes, cycles found, bounded scheduling that never drops
//! a selected item, forged or wrong-job results refused, and a partial report that names exactly
//! what is missing.

mod common;

use std::collections::BTreeSet;

use common::*;
use pctwin_orchestrate::jobs::{
    ChildJob, ChildResult, JobId, MAX_JOBS, Missing, PlanError, Resources, ResultError, Scheduler,
    Scope, Why, Worker, concurrency, seal,
};
use pctwin_record::ItemId;

fn items(names: &[&str]) -> BTreeSet<ItemId> {
    names
        .iter()
        .map(|n| file(person("me"), &[n], 0).id)
        .collect()
}

fn job(id: &str, worker: Worker, scope: Scope, after: &[&str]) -> ChildJob {
    ChildJob {
        id: JobId(id.into()),
        worker,
        source: laptop(1),
        session: 1,
        scope,
        after: after.iter().map(|a| JobId(a.to_string())).collect(),
        essential: false,
    }
}

fn data(id: &str, names: &[&str], after: &[&str]) -> ChildJob {
    job(
        id,
        Worker::Data,
        Scope::Items {
            laptop: laptop(1),
            items: items(names),
        },
        after,
    )
}

fn install(id: &str, app: &str, after: &[&str]) -> ChildJob {
    job(id, Worker::Install, Scope::App { name: app.into() }, after)
}

const PLENTY: Resources = Resources {
    on_battery: false,
    battery_percent: None,
    memory_free_bytes: Some(16 * 1024 * 1024 * 1024),
};

fn ok(s: &Scheduler, j: &str) -> ChildResult {
    s.result_for(&JobId(j.into()), true)
}

/// T26-006: a cycle of dependencies is found and named; the plan cannot be sealed until it is
/// fixed. Unknown dependencies, jobs twice and empty scopes are refused too.
#[test]
fn a_dependency_cycle_is_found_and_named() {
    let err = seal(vec![
        install("app", "Thunderbird", &["mail"]),
        data("mail", &["inbox"], &["plugin"]),
        install("plugin", "Lightning", &["app"]),
        data("photos", &["p"], &[]),
        // Waits behind the circle, but is not part of it.
        data("calendar", &["c"], &["mail"]),
    ])
    .unwrap_err();
    assert_eq!(
        err,
        PlanError::Cycle(vec![
            JobId("app".into()),
            JobId("mail".into()),
            JobId("plugin".into())
        ])
    );
    assert_eq!(
        seal(vec![data("a", &["x"], &["a"])]).unwrap_err(),
        PlanError::Cycle(vec![JobId("a".into())])
    );
    assert_eq!(
        seal(vec![data("a", &["x"], &["ghost"])]).unwrap_err(),
        PlanError::UnknownDependency {
            job: JobId("a".into()),
            needs: JobId("ghost".into())
        }
    );
    assert_eq!(
        seal(vec![data("a", &["x"], &[]), data("a", &["y"], &[])]).unwrap_err(),
        PlanError::JobTwice(JobId("a".into()))
    );
    assert_eq!(
        seal(vec![data("a", &[], &[])]).unwrap_err(),
        PlanError::EmptyScope(JobId("a".into()))
    );
    // Too many jobs is refused before any work.
    let many: Vec<ChildJob> = (0..=MAX_JOBS)
        .map(|n| data(&n.to_string(), &["x"], &[]))
        .collect();
    assert_eq!(seal(many).unwrap_err(), PlanError::TooBig);
    // A valid plan orders every job after what it needs, in a fixed order.
    let plan = seal(vec![
        data("mail", &["inbox"], &["app"]),
        install("app", "Thunderbird", &[]),
        data("photos", &["p"], &[]),
    ])
    .unwrap();
    assert_eq!(
        plan.order(),
        [
            JobId("app".into()),
            JobId("mail".into()),
            JobId("photos".into())
        ]
    );
}

/// T26-009: each child is given exactly its own scope; a grant wider than what the child was
/// approved for is refused by the child. There is no "everything" scope to ask for.
#[test]
fn a_child_refuses_a_wider_grant_than_its_own() {
    let plan = seal(vec![data("docs", &["a", "b"], &[])]).unwrap();
    let mut s = Scheduler::new(plan);
    let grants = s.next(&PLENTY);
    assert_eq!(grants.len(), 1);
    let approved = Scope::Items {
        laptop: laptop(1),
        items: items(&["a", "b"]),
    };
    assert_eq!(grants[0].scope, approved);
    assert_eq!(grants[0].check_within(&approved), Ok(()));
    let narrower = Scope::Items {
        laptop: laptop(1),
        items: items(&["a"]),
    };
    assert_eq!(
        grants[0].check_within(&narrower),
        Err(ResultError::WiderThanApproved)
    );
    // Another laptop's items, another kind of scope: refused.
    let other_laptop = Scope::Items {
        laptop: laptop(2),
        items: items(&["a", "b"]),
    };
    assert_eq!(
        grants[0].check_within(&other_laptop),
        Err(ResultError::WiderThanApproved)
    );
    assert_eq!(
        grants[0].check_within(&Scope::App { name: "x".into() }),
        Err(ResultError::WiderThanApproved)
    );
    let settings = |names: &[&str]| Scope::Settings {
        laptop: laptop(1),
        account_id: "S-1".into(),
        names: names.iter().map(|n| n.to_string()).collect(),
    };
    let g = pctwin_orchestrate::jobs::Grant::for_test(settings(&["wallpaper", "mouse"]));
    assert_eq!(
        g.check_within(&settings(&["wallpaper", "mouse", "keys"])),
        Ok(())
    );
    assert_eq!(
        g.check_within(&settings(&["wallpaper"])),
        Err(ResultError::WiderThanApproved)
    );
    let mut other = settings(&["wallpaper", "mouse"]);
    if let Scope::Settings { account_id, .. } = &mut other {
        *account_id = "S-2".into();
    }
    assert_eq!(g.check_within(&other), Err(ResultError::WiderThanApproved));
    let app = pctwin_orchestrate::jobs::Grant::for_test(Scope::App {
        name: "Zoom".into(),
    });
    assert_eq!(
        app.check_within(&Scope::App {
            name: "Zoom".into()
        }),
        Ok(())
    );
    assert_eq!(
        app.check_within(&Scope::App {
            name: "Slack".into()
        }),
        Err(ResultError::WiderThanApproved)
    );
}

/// T26-007: a result for another job, another laptop, an older session, a job not running, or
/// a job whose contents changed (another digest) is refused; a result is taken once.
#[test]
fn forged_or_wrong_job_results_are_refused() {
    let plan = seal(vec![data("docs", &["a"], &[]), data("pics", &["p"], &[])]).unwrap();
    let mut s = Scheduler::new(plan);
    // Not running yet: nothing is accepted.
    assert_eq!(s.report(&ok(&s, "docs")), Err(ResultError::NotRunning));
    let grants = s.next(&PLENTY);
    assert_eq!(grants.len(), 2);
    let good = ok(&s, "docs");
    let mut wrong_laptop = good.clone();
    wrong_laptop.source = laptop(2);
    let mut old_session = good.clone();
    old_session.session = 0;
    let mut wrong_worker = good.clone();
    wrong_worker.worker = Worker::Install;
    let mut wrong_digest = good.clone();
    wrong_digest.digest = ok(&s, "pics").digest;
    let mut unknown = good.clone();
    unknown.job = JobId("ghost".into());
    for (bad, why) in [
        (&wrong_laptop, ResultError::WrongSource),
        (&old_session, ResultError::OldSession),
        (&wrong_worker, ResultError::WrongWorker),
        (&wrong_digest, ResultError::WrongJob),
        (&unknown, ResultError::UnknownJob),
    ] {
        assert_eq!(s.report(bad), Err(why));
    }
    assert_eq!(s.report(&good), Ok(()));
    assert_eq!(s.report(&good), Err(ResultError::NotRunning), "taken once");
    assert!(s.is_done(&JobId("docs".into())));
}

/// T26-008: one child failing blocks only what depends on it; independent work carries on and
/// successful results are kept.
#[test]
fn a_failed_child_blocks_only_its_dependents() {
    let plan = seal(vec![
        install("app", "Thunderbird", &[]),
        data("mail", &["inbox"], &["app"]),
        data("photos", &["p"], &[]),
        data("calendar", &["c"], &["mail"]),
    ])
    .unwrap();
    let mut s = Scheduler::new(plan);
    let first: Vec<JobId> = s.next(&PLENTY).into_iter().map(|g| g.job).collect();
    // The import never starts before its app.
    assert_eq!(first, [JobId("app".into()), JobId("photos".into())]);
    assert!(!s.finished(), "two are still running");
    s.report(&ok(&s, "photos")).unwrap();
    s.report(&s.result_for(&JobId("app".into()), false))
        .unwrap();
    assert!(s.next(&PLENTY).is_empty());
    assert!(s.finished());
    let report = s.missing();
    assert!(s.is_done(&JobId("photos".into())));
    assert_eq!(
        report,
        [
            Missing {
                job: JobId("app".into()),
                scope: Scope::App {
                    name: "Thunderbird".into()
                },
                why: Why::Failed,
            },
            Missing {
                job: JobId("calendar".into()),
                scope: Scope::Items {
                    laptop: laptop(1),
                    items: items(&["c"])
                },
                why: Why::WaitingFor(JobId("mail".into())),
            },
            Missing {
                job: JobId("mail".into()),
                scope: Scope::Items {
                    laptop: laptop(1),
                    items: items(&["inbox"])
                },
                why: Why::WaitingFor(JobId("app".into())),
            },
        ]
    );
}

/// T26-029: low battery or little memory runs fewer jobs at once, a critical battery pauses, and
/// every selected job still ends; none is dropped.
#[test]
fn scarce_resources_slow_the_work_but_drop_nothing() {
    let low_battery = Resources {
        on_battery: true,
        battery_percent: Some(15.0),
        ..PLENTY
    };
    let critical = Resources {
        on_battery: true,
        battery_percent: Some(5.0),
        ..PLENTY
    };
    let low_memory = Resources {
        memory_free_bytes: Some(512 * 1024 * 1024),
        ..PLENTY
    };
    let unknown_memory = Resources {
        memory_free_bytes: None,
        ..PLENTY
    };
    let charging_low = Resources {
        on_battery: false,
        battery_percent: Some(5.0),
        ..PLENTY
    };
    assert_eq!(concurrency(&PLENTY, 4), 4);
    assert_eq!(concurrency(&low_battery, 4), 1);
    assert_eq!(concurrency(&critical, 4), 0);
    assert_eq!(concurrency(&low_memory, 4), 1);
    assert_eq!(concurrency(&unknown_memory, 4), 1);
    assert_eq!(concurrency(&charging_low, 4), 4);
    // The edges: 20% is low, 8% pauses, 1 GB free is enough.
    let at = |p: f32| Resources {
        on_battery: true,
        battery_percent: Some(p),
        ..PLENTY
    };
    assert_eq!(concurrency(&at(20.0), 4), 1);
    assert_eq!(concurrency(&at(20.5), 4), 4);
    assert_eq!(concurrency(&at(8.0), 4), 0);
    assert_eq!(concurrency(&at(8.5), 4), 1);
    let gb = Resources {
        memory_free_bytes: Some(1024 * 1024 * 1024),
        ..PLENTY
    };
    assert_eq!(concurrency(&gb, 4), 4);
    let on_battery_unknown = Resources {
        on_battery: true,
        battery_percent: None,
        ..PLENTY
    };
    assert_eq!(concurrency(&on_battery_unknown, 4), 1);

    let jobs: Vec<ChildJob> = (0..12)
        .map(|n| data(&format!("j{n:02}"), &[&n.to_string()], &[]))
        .collect();
    let mut s = Scheduler::new(seal(jobs).unwrap());
    let weather = [PLENTY, critical, low_battery, low_memory, critical, PLENTY];
    let mut step = 0;
    while !s.finished() {
        let r = weather[step % weather.len()];
        let grants = s.next(&r);
        assert!(grants.len() <= concurrency(&r, 4));
        if concurrency(&r, 4) == 0 {
            assert!(grants.is_empty(), "paused");
        }
        for g in grants {
            let res = s.result_for(&g.job, true);
            s.report(&res).unwrap();
        }
        step += 1;
        assert!(step < 100);
    }
    assert!(s.missing().is_empty(), "every job ran");
    assert_eq!(s.done_count(), 12);
}

/// The person's essentials go first when only one job can run at a time.
#[test]
fn essentials_go_first() {
    let mut a = data("a", &["a"], &[]);
    let mut b = data("b", &["b"], &[]);
    b.essential = true;
    a.essential = false;
    let mut s = Scheduler::new(seal(vec![a, b]).unwrap());
    let low = Resources {
        on_battery: true,
        battery_percent: Some(10.0),
        ..PLENTY
    };
    let g = s.next(&low);
    assert_eq!(g.len(), 1);
    assert_eq!(g[0].job, JobId("b".into()));
    // While one runs, no second starts at that limit.
    assert!(s.next(&low).is_empty());
}

/// T26-030: one old laptop is not connected: its jobs are reported as missing with their exact
/// scope and the next step; the other laptop's work completes.
#[test]
fn a_partial_report_names_the_exact_missing_scope() {
    let mut other = data("b-docs", &["bd"], &[]);
    other.source = laptop(2);
    other.scope = Scope::Items {
        laptop: laptop(2),
        items: items(&["bd"]),
    };
    let mut s = Scheduler::new(seal(vec![data("a-docs", &["ad"], &[]), other]).unwrap());
    s.source_unavailable(&laptop(2));
    let g = s.next(&PLENTY);
    assert_eq!(g.len(), 1);
    s.report(&ok(&s, "a-docs")).unwrap();
    assert!(s.finished());
    let missing = s.missing();
    assert_eq!(
        missing,
        [Missing {
            job: JobId("b-docs".into()),
            scope: Scope::Items {
                laptop: laptop(2),
                items: items(&["bd"])
            },
            why: Why::SourceUnavailable,
        }]
    );
    assert_eq!(
        missing[0].words("Ada's old laptop"),
        "1 item from Ada's old laptop did not move: that laptop is not connected. Connect it to the same Wi-Fi, keep it awake, and choose Try again."
    );
    assert_eq!(
        Missing {
            job: JobId("x".into()),
            scope: Scope::App {
                name: "Zoom".into()
            },
            why: Why::Failed
        }
        .words("Ada's old laptop"),
        "The app Zoom from Ada's old laptop did not finish. Choose Try again, or see more details."
    );
    assert_eq!(
        Missing {
            job: JobId("x".into()),
            scope: Scope::Items {
                laptop: laptop(1),
                items: items(&["1", "2"])
            },
            why: Why::WaitingFor(JobId("app".into()))
        }
        .words("Ada's old laptop"),
        "2 items from Ada's old laptop did not move: they wait for another step that did not finish. Fix that step first, then choose Try again."
    );
    // A running job whose laptop goes away is not counted done.
    let mut s = Scheduler::new(seal(vec![data("a", &["x"], &[])]).unwrap());
    s.next(&PLENTY);
    s.source_unavailable(&laptop(1));
    assert!(s.finished());
    assert_eq!(s.missing()[0].why, Why::SourceUnavailable);
    assert_eq!(s.report(&ok(&s, "a")), Err(ResultError::NotRunning));
}
