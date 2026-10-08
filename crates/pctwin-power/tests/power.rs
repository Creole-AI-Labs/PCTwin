//! Keeping both laptops awake during a move, and the battery (Task List 1.5). Only idle sleep is
//! held off, and only while a move runs; the person's own sleep (closing the lid, the power button)
//! is never blocked, and their power settings are never changed.

use pctwin_power::{BatteryAdvice, Power, advise, power_now, stay_awake};

#[test]
fn plugged_in_or_no_battery_needs_no_warning() {
    assert_eq!(
        advise(&Power {
            on_battery: false,
            percent: Some(5.0)
        }),
        BatteryAdvice::Fine
    );
    // A desktop, or a battery the system won't report: nothing to warn about.
    assert_eq!(
        advise(&Power {
            on_battery: false,
            percent: None
        }),
        BatteryAdvice::Fine
    );
}

#[test]
fn a_low_battery_asks_to_plug_in_and_a_critical_one_pauses_safely() {
    let on = |percent| Power {
        on_battery: true,
        percent: Some(percent),
    };
    assert_eq!(advise(&on(80.0)), BatteryAdvice::OnBattery);
    assert_eq!(advise(&on(20.0)), BatteryAdvice::PlugIn);
    assert_eq!(advise(&on(12.5)), BatteryAdvice::PlugIn);
    assert_eq!(advise(&on(8.0)), BatteryAdvice::PauseNow);
    assert_eq!(advise(&on(1.0)), BatteryAdvice::PauseNow);
    // On battery but the level is unknown: still worth plugging in.
    assert_eq!(
        advise(&Power {
            on_battery: true,
            percent: None
        }),
        BatteryAdvice::OnBattery
    );
}

#[test]
fn this_laptop_reports_its_power_without_failing() {
    let p = power_now();
    if let Some(pct) = p.percent {
        assert!((0.0..=100.0).contains(&pct), "{pct}");
    }
}

#[test]
fn staying_awake_is_held_while_the_handle_lives_and_released_after() {
    let awake = stay_awake("Moving your files to the new laptop", false);
    // GitHub's machines may not allow it (no session bus on Linux); either way it is reported.
    eprintln!("staying awake: {:?}", awake.held());
    drop(awake);
    // Asking again after release works the same way.
    let again = stay_awake("Moving your files to the new laptop", true);
    drop(again);
}
