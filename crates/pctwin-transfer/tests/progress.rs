//! One progress figure and one time left for the whole move, however many lanes carry it (Security
//! Design A, "Lanes share one plan"). Speed is measured each second and smoothed (an exponentially
//! weighted average, so one fast or slow second barely moves it); the first seconds say "working
//! out time left" rather than guess; a move with nothing arriving says it is waiting; paused time
//! does not count.

use pctwin_transfer::{Progress, TimeLeft};

const MB: u64 = 1_000_000;

/// `n` seconds at `per_second` bytes.
fn seconds(p: &mut Progress, n: u32, per_second: u64) {
    for _ in 0..n {
        p.confirmed(per_second);
        p.tick();
    }
}

fn about(p: &Progress) -> u64 {
    match p.time_left() {
        TimeLeft::About { seconds } => seconds,
        other => panic!("expected an estimate, got {other:?}"),
    }
}

#[test]
fn the_first_seconds_work_out_the_time_rather_than_guess() {
    let mut p = Progress::new(100 * MB);
    assert_eq!(p.time_left(), TimeLeft::WorkingOut);
    seconds(&mut p, 4, MB);
    assert_eq!(p.time_left(), TimeLeft::WorkingOut);
    seconds(&mut p, 1, MB);
    assert!(matches!(p.time_left(), TimeLeft::About { .. }));
}

#[test]
fn a_steady_speed_gives_the_plain_answer() {
    let mut p = Progress::new(100 * MB);
    seconds(&mut p, 10, 5 * MB);
    // 50 MB left at 5 MB a second.
    assert_eq!(about(&p), 10);
    assert_eq!(p.done_bytes(), 50 * MB);
    assert!((p.fraction() - 0.5).abs() < 1e-9);
    // A part second rounds up: the estimate never promises less than it will take.
    p.set_total(101 * MB);
    assert_eq!(about(&p), 11);
}

#[test]
fn one_fast_second_barely_moves_the_estimate() {
    let mut p = Progress::new(1000 * MB);
    seconds(&mut p, 20, 5 * MB);
    let before = about(&p);
    // One second ten times faster (a burst of small files already cached).
    seconds(&mut p, 1, 50 * MB);
    let after = about(&p);
    // Unsmoothed it would say 850 MB at 50 MB a second: 17 seconds. Smoothed it stays far above.
    let unsmoothed = (p.total_bytes() - p.done_bytes()) / (50 * MB);
    assert_eq!(unsmoothed, 17);
    assert!(after > 4 * unsmoothed, "{before} -> {after}");
    assert!(after < before);
}

#[test]
fn a_lasting_change_of_speed_is_followed() {
    let mut p = Progress::new(10_000 * MB);
    seconds(&mut p, 20, 5 * MB);
    seconds(&mut p, 60, 10 * MB);
    let left = p.total_bytes() - p.done_bytes();
    let plain = left / (10 * MB);
    let est = about(&p);
    assert!(est.abs_diff(plain) * 100 <= plain * 5, "{est} vs {plain}");
}

#[test]
fn nothing_arriving_for_a_while_says_waiting_and_recovers() {
    let mut p = Progress::new(100 * MB);
    seconds(&mut p, 10, 5 * MB);
    seconds(&mut p, 9, 0);
    assert!(matches!(p.time_left(), TimeLeft::About { .. }));
    seconds(&mut p, 1, 0);
    assert_eq!(p.time_left(), TimeLeft::Waiting);
    // Data flows again.
    seconds(&mut p, 1, 5 * MB);
    assert!(matches!(p.time_left(), TimeLeft::About { .. }));
}

#[test]
fn paused_time_does_not_count() {
    let mut p = Progress::new(100 * MB);
    seconds(&mut p, 10, 5 * MB);
    let before = about(&p);
    p.pause();
    assert_eq!(p.time_left(), TimeLeft::Paused);
    // A long pause: neither waiting, nor a slower speed afterwards.
    seconds(&mut p, 300, 0);
    p.resume();
    assert_eq!(about(&p), before);
}

#[test]
fn a_finished_move_says_done_and_never_passes_all_of_it() {
    let mut p = Progress::new(10 * MB);
    seconds(&mut p, 2, 5 * MB);
    assert_eq!(p.time_left(), TimeLeft::Done);
    // A file grew a little since the plan: more arrives than planned.
    p.confirmed(MB);
    assert_eq!(p.time_left(), TimeLeft::Done);
    assert!((p.fraction() - 1.0).abs() < 1e-9);
    assert_eq!(Progress::new(0).time_left(), TimeLeft::Done);
    assert!((Progress::new(0).fraction() - 1.0).abs() < 1e-9);
}

#[test]
fn the_total_can_change_as_the_move_goes() {
    let mut p = Progress::new(100 * MB);
    seconds(&mut p, 10, 5 * MB);
    // Files found already on the new laptop are taken off the total.
    p.set_total(60 * MB);
    assert_eq!(about(&p), 2);
    assert!((p.fraction() - 50.0 / 60.0).abs() < 1e-9);
}
