//! Applying catalogue settings on the new laptop: handoff for protected choices (T24-011,
//! T24-012), the confirm-or-go-back window (T24-009), a restart part way (T24-021), a picture
//! that is not there (T24-007), a grant for another step, and reading this laptop's own values.

mod common;

use std::time::{Duration, Instant};

use common::{SCOPE, actor, journal, plan_of, rebuild, undo};
use pctwin_executor::recover;
use pctwin_journal::{EffectLedger, EffectUndoOutcome, JournalError};
use pctwin_recipe::{Arch, Os, Platform, SettingValue};
use pctwin_settings::catalogue::{Catalogue, System};
use pctwin_settings::confirm::{CONFIRM_WITHIN, ConfirmWindow, Decision, scope_for};
use pctwin_settings::handoff::{Handoff, item_result, judge, page, registered_apps};
use pctwin_settings::mechanism::Mechanisms;
use pctwin_settings::registry::{MemHive, RegValue};
use pctwin_settings::{SettingsAdapter, Store};
use pctwin_types::{ItemStatus, Reason};

const WIN: Platform = Platform {
    os: Os::Windows,
    arch: Arch::X86_64,
};

fn text(s: &str) -> SettingValue {
    SettingValue::Text(s.into())
}

fn adapter(h: MemHive) -> SettingsAdapter {
    SettingsAdapter {
        store: Store::new(
            Catalogue::starter().unwrap(),
            Mechanisms {
                system: System::Windows,
                user: Some(Box::new(h)),
                machine: None,
                runner: None,
                localtime: None,
            },
        ),
        platform: WIN,
    }
}

fn recipe(key: &str, v: SettingValue) -> pctwin_recipe::Recipe {
    let cat = Catalogue::starter().unwrap();
    let planned = pctwin_settings::Planned {
        id: cat.row(key).unwrap().id.clone(),
        outcome: pctwin_settings::Outcome::Exact,
        action: pctwin_settings::Action::Apply(v),
        old: None,
        before: None,
    };
    planned.recipe(Os::Windows, cat.revision).unwrap().unwrap()
}

#[test]
fn t24_011_the_default_apps_page_is_opened_for_the_app_never_the_choice_edited() {
    let cat = Catalogue::starter().unwrap();
    let row = cat.row("apps.default-browser").unwrap();
    let mut user = MemHive::default();
    user.put(
        "Software\\RegisteredApplications",
        "Firefox-308046B0AF4A39CB",
        text("x").into_reg(),
    );
    let mut machine = MemHive::default();
    machine.put(
        "SOFTWARE\\RegisteredApplications",
        "Google Chrome",
        text("x").into_reg(),
    );
    let names = registered_apps(Some(&user), Some(&machine));
    assert_eq!(
        page(&cat, row, System::Windows, Some(&text("firefox")), &names).as_deref(),
        Some("ms-settings:defaultapps?registeredAppUser=Firefox-308046B0AF4A39CB")
    );
    assert_eq!(
        page(&cat, row, System::Windows, Some(&text("chrome")), &names).as_deref(),
        Some("ms-settings:defaultapps?registeredAppUser=Google%20Chrome")
    );
    // Not registered: the general page.
    assert_eq!(
        page(&cat, row, System::Windows, Some(&text("brave")), &names).as_deref(),
        Some("ms-settings:defaultapps")
    );
    assert_eq!(
        page(&cat, row, System::Mac, Some(&text("firefox")), &names).as_deref(),
        Some("x-apple.systempreferences:com.apple.Desktop-Settings.extension")
    );
    assert_eq!(
        page(&cat, row, System::Gnome, Some(&text("firefox")), &names),
        None
    );
    // The store never writes it.
    let mut a = adapter(MemHive::default());
    assert!(
        a.store
            .write_if(
                "apps.default-browser",
                &text("edge"),
                Some(&text("firefox"))
            )
            .is_err()
    );
}

trait IntoReg {
    fn into_reg(self) -> RegValue;
}

impl IntoReg for SettingValue {
    fn into_reg(self) -> RegValue {
        match self {
            SettingValue::Text(t) => RegValue::Sz(t),
            _ => RegValue::Other,
        }
    }
}

#[test]
fn t24_012_a_page_that_opened_is_not_a_setting_restored() {
    let want = text("firefox");
    assert_eq!(judge(&want, Ok(Some(text("edge"))), true), Handoff::NotYet);
    assert_eq!(judge(&want, Ok(None), true), Handoff::NotYet);
    assert_eq!(
        judge(&want, Ok(Some(text("firefox"))), false),
        Handoff::Verified
    );
    assert_eq!(
        judge(&want, Err("unreadable".into()), true),
        Handoff::PersonConfirmed
    );
    assert_eq!(
        judge(&want, Err("unreadable".into()), false),
        Handoff::NotYet
    );
    assert_eq!(
        item_result(&Handoff::Verified).status(),
        ItemStatus::Converted
    );
    assert_eq!(
        item_result(&Handoff::NotYet).reason(),
        Some(Reason::FinishByHand)
    );
    assert_eq!(
        item_result(&Handoff::PersonConfirmed).status(),
        ItemStatus::NeedsAction
    );
    // The real read-back: the browser the system's protected choice names.
    let mut h = MemHive::default();
    h.put(
        "Software\\Microsoft\\Windows\\Shell\\Associations\\UrlAssociations\\https\\UserChoice",
        "ProgId",
        RegValue::Sz("FirefoxURL-308046B0AF4A39CB".into()),
    );
    let mut a = adapter(h);
    assert_eq!(
        a.store.read("apps.default-browser"),
        Ok(Some(text("firefox")))
    );
}

#[test]
fn t24_009_a_risky_change_goes_back_by_itself_unless_confirmed() {
    let t0 = Instant::now();
    let mut w = ConfirmWindow::start(t0);
    assert_eq!(w.decide(t0 + Duration::from_secs(5)), Decision::Wait);
    assert_eq!(w.decide(t0 + CONFIRM_WITHIN), Decision::GoBack);
    assert!(!w.confirm(t0 + CONFIRM_WITHIN), "too late to confirm");
    let mut w = ConfirmWindow::start(t0);
    assert!(w.confirm(t0 + Duration::from_secs(3)));
    assert_eq!(w.decide(t0 + Duration::from_secs(60)), Decision::Keep);
    assert_ne!(scope_for(SCOPE, "pointer.primary-button"), SCOPE);
    assert_ne!(
        scope_for(SCOPE, "pointer.primary-button"),
        scope_for(SCOPE, "pointer.speed")
    );

    // Applied in its own scope beside an ordinary setting; not confirmed; only it goes back.
    let mut h = MemHive::default();
    h.put(
        "Control Panel\\Mouse",
        "SwapMouseButtons",
        RegValue::Sz("0".into()),
    );
    let mut a = adapter(h);
    let (_d, j) = journal();
    let normal = vec![recipe("pointer.speed", SettingValue::Int(474))];
    let risky = vec![recipe("pointer.primary-button", text("right"))];
    let own = scope_for(SCOPE, "pointer.primary-button");
    assert!(
        plan_of(&normal)
            .run(&mut a, &rebuild(&j), SCOPE, &actor())
            .stopped
            .is_none()
    );
    let run = plan_of(&risky).run(&mut a, &rebuild(&j), own, &actor());
    assert_eq!(run.results[0].1.status(), ItemStatus::Converted);
    assert_eq!(
        a.store.read("pointer.primary-button"),
        Ok(Some(text("right")))
    );
    let w = ConfirmWindow::start(t0);
    assert_eq!(w.decide(t0 + CONFIRM_WITHIN), Decision::GoBack);
    assert_eq!(
        undo(&j, &mut a, &risky, own),
        vec![EffectUndoOutcome::Restored]
    );
    assert_eq!(
        a.store.read("pointer.primary-button"),
        Ok(Some(text("left")))
    );
    assert_eq!(
        a.store.read("pointer.speed"),
        Ok(Some(SettingValue::Int(474)))
    );
}

#[test]
fn t24_007_a_picture_that_is_not_on_the_laptop_is_never_set() {
    let mut a = adapter(MemHive::default());
    let (_d, j) = journal();
    let missing = vec![recipe(
        "desktop.wallpaper",
        text("C:\\Users\\ada\\Pictures\\not-there.jpg"),
    )];
    let run = plan_of(&missing).run(&mut a, &rebuild(&j), SCOPE, &actor());
    assert_eq!(run.results[0].1.status(), ItemStatus::Failed);
    assert_eq!(a.store.read("desktop.wallpaper"), Ok(None));
    // A file that is there is set.
    let dir = tempfile::tempdir().unwrap();
    let pic = dir.path().join("sea.jpg");
    std::fs::write(&pic, b"jpg").unwrap();
    let there = vec![recipe("desktop.wallpaper", text(pic.to_str().unwrap()))];
    let run = plan_of(&there).run(&mut a, &rebuild(&j), SCOPE, &actor());
    assert_eq!(run.results[0].1.status(), ItemStatus::Converted);
}

#[test]
fn a_grant_for_another_step_or_a_managed_setting_is_refused() {
    use pctwin_grant::{ObjectRef, PlanSeal};
    let mut h = MemHive::default();
    h.put(
        "Software\\Policies\\Microsoft\\Control Panel\\Mouse",
        "MouseSensitivity",
        RegValue::Sz("10".into()),
    );
    let mut a = adapter(h);
    let keys = common::keys();
    let step = pctwin_recipe::Step::ApplySetting {
        key: "pointer.speed".into(),
        value: SettingValue::Int(474),
    };
    let good = pctwin_executor::object_of(0, &step);
    let seal = PlanSeal::new(1, [1; 32]);
    let g = keys
        .setting
        .grant(
            seal,
            good,
            Duration::from_secs(60),
            pctwin_grant::Now::real(),
        )
        .unwrap();
    // Managed by the organisation.
    assert_eq!(
        a.store.apply(g, "pointer.speed", &SettingValue::Int(474)),
        Err(Reason::Permission)
    );
    let mut a = adapter(MemHive::default());
    let g = keys
        .setting
        .grant(
            seal,
            good,
            Duration::from_secs(60),
            pctwin_grant::Now::real(),
        )
        .unwrap();
    // The grant names 474, not 1000.
    assert_eq!(
        a.store.apply(g, "pointer.speed", &SettingValue::Int(1000)),
        Err(Reason::RefusedBySafetyCheck)
    );
    let other = ObjectRef::new(0, [9; 32]);
    let g = keys
        .setting
        .grant(
            seal,
            other,
            Duration::from_secs(60),
            pctwin_grant::Now::real(),
        )
        .unwrap();
    assert_eq!(
        a.store.apply(g, "pointer.speed", &SettingValue::Int(474)),
        Err(Reason::RefusedBySafetyCheck)
    );
    // A key that is not a catalogue row or a remembered found setting.
    let step = pctwin_recipe::Step::ApplySetting {
        key: "found.0123".into(),
        value: text("x"),
    };
    let g = keys
        .setting
        .grant(
            seal,
            pctwin_executor::object_of(0, &step),
            Duration::from_secs(60),
            pctwin_grant::Now::real(),
        )
        .unwrap();
    assert_eq!(
        a.store.apply(g, "found.0123", &text("x")),
        Err(Reason::RefusedBySafetyCheck)
    );
}

/// A journal that takes `left` writes, then fails every write (the power going out).
struct Faulty<'a> {
    inner: pctwin_journal::Rebuild<'a>,
    left: std::cell::Cell<usize>,
}

impl Faulty<'_> {
    fn spend(&self) -> Result<(), JournalError> {
        if self.left.get() == 0 {
            return Err(JournalError::Storage("the power went out".into()));
        }
        self.left.set(self.left.get() - 1);
        Ok(())
    }
}

impl EffectLedger for Faulty<'_> {
    fn intend_install(
        &self,
        p: &pctwin_journal::PlannedEffect,
        i: &pctwin_journal::InstallIntent,
    ) -> Result<u64, JournalError> {
        self.spend()?;
        self.inner.intend_install(p, i)
    }
    fn intend_setting(
        &self,
        p: &pctwin_journal::PlannedEffect,
        i: &pctwin_journal::SettingsIntent,
    ) -> Result<u64, JournalError> {
        self.spend()?;
        self.inner.intend_setting(p, i)
    }
    fn observed(
        &self,
        r: pctwin_journal::RebuildRole,
        id: u64,
        f: &pctwin_journal::Found,
        after: bool,
    ) -> Result<(), JournalError> {
        self.spend()?;
        self.inner.observed(r, id, f, after)
    }
    fn receipt(
        &self,
        r: pctwin_journal::RebuildRole,
        id: u64,
        rc: &pctwin_journal::Receipt,
    ) -> Result<(), JournalError> {
        self.spend()?;
        self.inner.receipt(r, id, rc)
    }
    fn unfinished_effects(&self) -> Result<Vec<pctwin_journal::EffectEntry>, JournalError> {
        self.inner.unfinished_effects()
    }
    fn restore_point(
        &self,
        s: pctwin_journal::Scope,
    ) -> Result<Option<pctwin_journal::RestorePoint>, JournalError> {
        self.inner.restore_point(s)
    }
    fn record_restore_point(
        &self,
        s: pctwin_journal::Scope,
        p: &pctwin_journal::RestorePoint,
    ) -> Result<(), JournalError> {
        self.spend()?;
        self.inner.record_restore_point(s, p)
    }
    fn install_grant(
        &self,
        s: pctwin_grant::PlanSeal,
        a: pctwin_grant::ObjectRef,
    ) -> Result<pctwin_grant::InstallGrant, pctwin_grant::GrantError> {
        self.inner.install_grant(s, a)
    }
    fn setting_grant(
        &self,
        s: pctwin_grant::PlanSeal,
        o: pctwin_grant::ObjectRef,
    ) -> Result<pctwin_grant::SettingGrant, pctwin_grant::GrantError> {
        self.inner.setting_grant(s, o)
    }
}

#[test]
fn t24_021_a_restart_part_way_is_reconciled_setting_by_setting_and_undone_fully() {
    let recipes = vec![
        recipe("appearance.theme", text("dark")),
        recipe("pointer.speed", SettingValue::Int(474)),
        recipe("accessibility.text-size", SettingValue::Int(150)),
    ];
    let start = || {
        let mut h = MemHive::default();
        h.put(
            "Control Panel\\Mouse",
            "MouseSensitivity",
            RegValue::Sz("4".into()),
        );
        h
    };
    // Every point at which the power can go out: 1 restore point + 3 settings x 3 writes.
    for writes in 0..=10 {
        let (_d, j) = journal();
        let mut a = adapter(start());
        {
            let faulty = Faulty {
                inner: rebuild(&j),
                left: std::cell::Cell::new(writes),
            };
            let run = plan_of(&recipes).run(&mut a, &faulty, SCOPE, &actor());
            assert_eq!(run.stopped.is_some(), writes < 10, "writes {writes}");
        }
        // Restart: each half-done setting is looked at again and settled on its own.
        recover(&rebuild(&j), &mut a).unwrap();
        assert!(
            j.unfinished_effects().unwrap().is_empty(),
            "writes {writes}"
        );
        let run = plan_of(&recipes).run(&mut a, &rebuild(&j), SCOPE, &actor());
        assert!(run.stopped.is_none());
        for (_, r) in &run.results {
            assert_eq!(r.status(), ItemStatus::Converted, "writes {writes}: {r:?}");
        }
        assert_eq!(a.store.read("appearance.theme"), Ok(Some(text("dark"))));
        assert_eq!(
            a.store.read("accessibility.text-size"),
            Ok(Some(SettingValue::Int(150)))
        );
        // Undo leaves the laptop as it was before the move.
        for o in undo(&j, &mut a, &recipes, SCOPE) {
            assert!(!o.needs_action(), "writes {writes}: {o:?}");
        }
        assert_eq!(
            a.store.read("appearance.theme"),
            Ok(None),
            "writes {writes}"
        );
        assert_eq!(
            a.store.read("accessibility.text-size"),
            Ok(None),
            "writes {writes}"
        );
        let h = a.store.mechanisms().user.as_deref().unwrap();
        assert_eq!(
            h.get("Control Panel\\Mouse", "MouseSensitivity").unwrap(),
            Some(RegValue::Sz("4".into())),
            "writes {writes}"
        );
    }
}

#[cfg(windows)]
#[test]
fn this_laptops_catalogue_settings_are_read_without_changing_anything() {
    // Read only, through the real registry. Only which rows were read is looked at; no value of
    // the person's leaves this test.
    let mut store = Store::new(
        Catalogue::starter().unwrap(),
        Mechanisms::real(System::Windows),
    );
    let c = store.capture();
    let (read, total) = c.coverage();
    assert_eq!(total, Catalogue::starter().unwrap().settings.len());
    assert!(read >= 3, "read {read} of {total}");
    assert!(matches!(
        c.readings.get(
            &Catalogue::starter()
                .unwrap()
                .row("appearance.theme")
                .unwrap()
                .id
        ),
        Some(pctwin_settings::Reading::Value(_) | pctwin_settings::Reading::NotSet)
    ));
}
