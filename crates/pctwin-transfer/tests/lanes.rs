//! How many lanes (connections) a move uses (Security Design A, "Extra lanes"): the tuner climbs
//! one lane at a time, keeps a lane only when total speed rises by at least 10%, and looks again
//! when the network changes. Tested with simulated speeds.

use pctwin_transfer::{LaneTuner, MAX_LANES};

/// Feeds the tuner `windows` measurements from a speed model (speed for 1, 2, 3, 4 lanes).
fn run(tuner: &mut LaneTuner, speeds: [f64; 4], windows: usize) {
    for _ in 0..windows {
        let lanes = tuner.lanes();
        assert!((1..=MAX_LANES).contains(&lanes));
        tuner.measured(speeds[usize::from(lanes) - 1]);
    }
}

#[test]
fn it_starts_with_one_lane() {
    assert_eq!(LaneTuner::new().lanes(), 1);
}

#[test]
fn it_climbs_to_the_fastest_number_of_lanes_and_stops() {
    let mut t = LaneTuner::new();
    // Speed rises, then falls as lanes compete: 3 is best.
    run(&mut t, [10.0, 18.0, 24.0, 22.0], 20);
    assert_eq!(t.lanes(), 3);
}

#[test]
fn a_network_that_one_lane_already_fills_keeps_one_lane() {
    let mut t = LaneTuner::new();
    run(&mut t, [100.0, 102.0, 103.0, 103.0], 20);
    assert_eq!(t.lanes(), 1);
}

#[test]
fn a_gain_under_ten_percent_is_not_worth_a_lane() {
    let mut t = LaneTuner::new();
    run(&mut t, [100.0, 109.0, 118.0, 127.0], 20);
    assert_eq!(t.lanes(), 1);
    let mut t = LaneTuner::new();
    run(&mut t, [100.0, 110.0, 111.0, 111.0], 20);
    assert_eq!(t.lanes(), 2, "exactly 10% is enough");
}

#[test]
fn never_more_than_four_lanes() {
    let mut t = LaneTuner::new();
    run(&mut t, [10.0, 20.0, 30.0, 40.0], 50);
    assert_eq!(t.lanes(), MAX_LANES);
    assert_eq!(MAX_LANES, 4);
}

#[test]
fn the_first_window_after_a_change_is_not_judged() {
    // A new connection starts slowly; its first window shows only half the speed.
    let mut t = LaneTuner::new();
    let speeds = [10.0, 18.0, 24.0, 22.0];
    let mut last = 0;
    for _ in 0..20 {
        let lanes = t.lanes();
        let full = speeds[usize::from(lanes) - 1];
        let speed = if lanes == last { full } else { full / 2.0 };
        last = lanes;
        t.measured(speed);
    }
    assert_eq!(t.lanes(), 3);
}

#[test]
fn when_the_network_gets_worse_it_steps_back_down() {
    let mut t = LaneTuner::new();
    run(&mut t, [10.0, 18.0, 24.0, 22.0], 20);
    assert_eq!(t.lanes(), 3);
    // Now extra lanes only get in each other's way.
    run(&mut t, [10.0, 5.0, 3.0, 2.0], 30);
    assert_eq!(t.lanes(), 1);
}

#[test]
fn it_looks_again_now_and_then_in_case_more_lanes_would_now_help() {
    let mut t = LaneTuner::new();
    run(&mut t, [100.0, 101.0, 101.0, 101.0], 20);
    assert_eq!(t.lanes(), 1);
    // Same speed on one lane, but more lanes would now help (another device stopped using it).
    run(&mut t, [100.0, 150.0, 200.0, 210.0], 400);
    while !t.settled() {
        run(&mut t, [100.0, 150.0, 200.0, 210.0], 1);
    }
    assert_eq!(t.lanes(), 3);
}

#[test]
fn a_steady_choice_is_not_disturbed_by_ordinary_ups_and_downs() {
    let mut t = LaneTuner::new();
    run(&mut t, [10.0, 18.0, 24.0, 22.0], 20);
    let mut changes = 0;
    for i in 0..60 {
        let before = t.lanes();
        // Wobbles of 15% either way.
        let wobble = if i % 2 == 0 { 0.85 } else { 1.15 };
        t.measured([10.0, 18.0, 24.0, 22.0][usize::from(before) - 1] * wobble);
        if t.lanes() != before {
            changes += 1;
        }
    }
    assert_eq!(changes, 0);
}

#[test]
fn short_dips_and_mild_slowdowns_are_not_a_changed_network() {
    let speeds = [10.0, 18.0, 24.0, 22.0];
    let mut t = LaneTuner::new();
    run(&mut t, speeds, 20);
    for i in 0..60 {
        // A one-window dip to half (a microwave, say), then a few windows at 80%.
        let share = match i % 10 {
            0 => 0.5,
            1..=4 => 0.8,
            _ => 1.2,
        };
        t.measured(speeds[usize::from(t.lanes()) - 1] * share);
        assert_eq!(t.lanes(), 3, "window {i}");
    }
}

#[test]
fn a_routine_look_again_only_tries_more_lanes() {
    let mut t = LaneTuner::new();
    run(&mut t, [10.0, 18.0, 24.0, 22.0], 20);
    for _ in 0..400 {
        run(&mut t, [10.0, 18.0, 24.0, 22.0], 1);
        assert!(t.lanes() >= 3);
    }
}

#[test]
fn fewer_lanes_are_kept_only_when_they_are_about_as_fast() {
    let mut t = LaneTuner::new();
    run(&mut t, [10.0, 18.0, 24.0, 22.0], 20);
    // Slower all round, but the same shape: three is still best.
    run(&mut t, [6.0, 9.0, 12.0, 11.0], 30);
    assert!(t.settled());
    assert_eq!(t.lanes(), 3);
}

#[test]
fn nonsense_measurements_change_nothing() {
    let mut t = LaneTuner::new();
    run(&mut t, [100.0, 102.0, 103.0, 103.0], 1);
    for bad in [f64::NAN, f64::INFINITY, -5.0] {
        t.measured(bad);
    }
    run(&mut t, [100.0, 102.0, 103.0, 103.0], 20);
    assert_eq!(t.lanes(), 1);
}

#[test]
fn a_lane_that_cannot_be_opened_is_not_tried_again() {
    let mut t = LaneTuner::new();
    run(&mut t, [10.0, 20.0, 30.0, 40.0], 3);
    assert_eq!(t.lanes(), 2);
    // The second lane could not be opened (a firewall, say): back to one, and one is the most.
    t.could_not_open();
    assert_eq!(t.lanes(), 1);
    run(&mut t, [10.0, 20.0, 30.0, 40.0], 400);
    assert_eq!(t.lanes(), 1);
}

#[test]
fn a_lost_lane_is_counted_and_the_tuner_carries_on() {
    let mut t = LaneTuner::new();
    run(&mut t, [10.0, 20.0, 30.0, 40.0], 50);
    assert_eq!(t.lanes(), 4);
    t.lane_lost();
    assert_eq!(t.lanes(), 3);
    run(&mut t, [10.0, 20.0, 30.0, 40.0], 20);
    assert_eq!(t.lanes(), 4, "a lost lane may be replaced by a new one");
    let mut one = LaneTuner::new();
    one.lane_lost();
    assert_eq!(one.lanes(), 1, "the first connection is never counted away");
}

#[test]
fn a_refused_lane_is_tried_again_only_at_the_next_look() {
    let mut t = LaneTuner::new();
    run(&mut t, [10.0, 20.0, 30.0, 40.0], 3);
    assert_eq!(t.lanes(), 2);
    // Refused this time: straight back to one lane, settled.
    t.refused();
    assert_eq!(t.lanes(), 1);
    assert!(t.settled());
    run(&mut t, [10.0, 20.0, 30.0, 40.0], 50);
    assert_eq!(t.lanes(), 1, "not asked again soon");
    // At the next look (every 100 windows) more lanes are tried again, unlike after could_not_open.
    run(&mut t, [10.0, 20.0, 30.0, 40.0], 200);
    assert!(t.lanes() > 1);
}
