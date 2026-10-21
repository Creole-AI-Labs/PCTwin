//! Speed, measured: how many bytes a second a move is really doing, from the move meter's own
//! counts at the moments someone read it. Never a guess: until the readings cover
//! [`WARM_UP`] there is no speed at all, and a move that has done nothing for [`STALLED`] says so
//! instead of dividing by a speed it no longer has.
//!
//! The speed is the bytes done over the last [`WINDOW`] of readings (a sliding window, as restic
//! and rclone measure theirs): long enough that one fast or slow second barely moves it, short
//! enough that a move that speeds up or slows down shows it within that time. Readings that come
//! rarely still give a speed, measured over the time between them.

use std::collections::VecDeque;
use std::time::{Duration, Instant};

/// Readings must cover at least this long before there is a speed.
pub const WARM_UP: Duration = Duration::from_secs(5);
/// The speed is measured over about this long.
pub const WINDOW: Duration = Duration::from_secs(20);
/// Nothing done for this long: the move is waiting, not slow.
pub const STALLED: Duration = Duration::from_secs(10);
/// Readings closer together than this are not kept (they still end the window).
const MIN_GAP: Duration = Duration::from_millis(250);
/// At most this many readings are kept, however often the meter is read.
const MAX_SAMPLES: usize = 128;

/// What the readings so far say.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Measured {
    /// Bytes a second over the window, once the readings cover [`WARM_UP`].
    pub(crate) speed: Option<u64>,
    /// Nothing was done for [`STALLED`] or longer.
    pub(crate) stalled: bool,
}

/// Readings of one counter that only grows (bytes done), at the moments they were taken.
#[derive(Debug, Default)]
pub(crate) struct Pace {
    /// (when, bytes done), oldest first.
    samples: VecDeque<(Instant, u64)>,
    /// When the count last grew, and to what.
    last_change: Option<(Instant, u64)>,
}

impl Pace {
    /// Forgets every reading: the speed must be measured again from now.
    pub(crate) fn reset(&mut self) {
        self.samples.clear();
        self.last_change = None;
    }

    /// Bytes `done` at `now`.
    pub(crate) fn measure(&mut self, now: Instant, done: u64) -> Measured {
        if let Some(&(t, d)) = self.samples.back()
            && (done < d || now < t)
        {
            // The count started again (a session took the meter over), or time ran backwards:
            // nothing measured before means anything now.
            self.reset();
        }
        match self.last_change {
            Some((_, d)) if d >= done => {}
            _ => self.last_change = Some((now, done)),
        }
        let keep = self
            .samples
            .back()
            .is_none_or(|&(t, _)| now.saturating_duration_since(t) >= MIN_GAP);
        if keep {
            self.samples.push_back((now, done));
        }
        // Keep the newest reading at least a window old as the start, and nothing older.
        while self.samples.len() >= 2
            && self
                .samples
                .get(1)
                .is_some_and(|&(t, _)| now.saturating_duration_since(t) >= WINDOW)
        {
            self.samples.pop_front();
        }
        while self.samples.len() > MAX_SAMPLES {
            self.samples.pop_front();
        }
        let stalled = self
            .last_change
            .is_some_and(|(t, _)| now.saturating_duration_since(t) >= STALLED);
        let speed = self.samples.front().and_then(|&(t, d)| {
            let span = now.saturating_duration_since(t);
            if span < WARM_UP {
                return None;
            }
            let bytes = done.saturating_sub(d) as f64;
            Some((bytes / span.as_secs_f64()).floor() as u64)
        });
        Measured { speed, stalled }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const MB: u64 = 1_000_000;

    /// Reads `pace` once a second for `n` seconds from `*at`, `per_second` bytes more each time.
    fn seconds(
        pace: &mut Pace,
        at: &mut Instant,
        done: &mut u64,
        n: u32,
        per_second: u64,
    ) -> Measured {
        let mut last = pace.measure(*at, *done);
        for _ in 0..n {
            *at += Duration::from_secs(1);
            *done += per_second;
            last = pace.measure(*at, *done);
        }
        last
    }

    #[test]
    fn there_is_no_speed_until_the_readings_cover_the_warm_up_then_it_is_the_measured_one() {
        let (mut p, mut at, mut done) = (Pace::default(), Instant::now(), 0);
        assert_eq!(p.measure(at, 0).speed, None);
        let m = seconds(&mut p, &mut at, &mut done, 4, MB);
        assert_eq!(m.speed, None, "4 seconds is too few to say");
        let m = seconds(&mut p, &mut at, &mut done, 1, MB);
        assert_eq!(m.speed, Some(MB));
        assert!(!m.stalled);
    }

    #[test]
    fn the_speed_follows_a_change_within_the_window_and_forgets_what_came_before() {
        let (mut p, mut at, mut done) = (Pace::default(), Instant::now(), 0);
        seconds(&mut p, &mut at, &mut done, 30, MB);
        // Ten seconds into a faster rate the window holds both.
        let m = seconds(&mut p, &mut at, &mut done, 10, 3 * MB);
        assert_eq!(m.speed, Some(2 * MB));
        // A whole window later, only the new rate is left.
        let m = seconds(&mut p, &mut at, &mut done, 10, 3 * MB);
        assert_eq!(m.speed, Some(3 * MB));
    }

    #[test]
    fn one_fast_second_barely_moves_the_speed() {
        let (mut p, mut at, mut done) = (Pace::default(), Instant::now(), 0);
        seconds(&mut p, &mut at, &mut done, 30, MB);
        let m = seconds(&mut p, &mut at, &mut done, 1, 21 * MB);
        assert_eq!(m.speed, Some(2 * MB));
    }

    #[test]
    fn nothing_done_for_the_stall_time_is_stalled_and_anything_done_ends_it() {
        let (mut p, mut at, mut done) = (Pace::default(), Instant::now(), 0);
        seconds(&mut p, &mut at, &mut done, 10, MB);
        let m = seconds(&mut p, &mut at, &mut done, 9, 0);
        assert!(!m.stalled, "9 quiet seconds is not yet waiting");
        let m = seconds(&mut p, &mut at, &mut done, 1, 0);
        assert!(m.stalled);
        let m = seconds(&mut p, &mut at, &mut done, 1, 1);
        assert!(!m.stalled);
    }

    #[test]
    fn a_count_that_starts_again_is_measured_again_from_nothing() {
        let (mut p, mut at, mut done) = (Pace::default(), Instant::now(), 0);
        seconds(&mut p, &mut at, &mut done, 10, MB);
        at += Duration::from_secs(1);
        done = 0;
        assert_eq!(p.measure(at, done).speed, None);
        let m = seconds(&mut p, &mut at, &mut done, 5, 2 * MB);
        assert_eq!(m.speed, Some(2 * MB));
    }

    #[test]
    fn readings_far_apart_still_give_the_speed_between_them() {
        let mut p = Pace::default();
        let at = Instant::now();
        p.measure(at, 0);
        let m = p.measure(at + Duration::from_secs(60), 120 * MB);
        assert_eq!(m.speed, Some(2 * MB));
    }

    #[test]
    fn readings_however_often_are_kept_within_bounds() {
        let mut p = Pace::default();
        let at = Instant::now();
        for i in 0..100_000u64 {
            p.measure(at + Duration::from_millis(i), i);
        }
        assert!(p.samples.len() <= MAX_SAMPLES, "{}", p.samples.len());
        // The window still measures: a byte a millisecond.
        assert_eq!(
            p.measure(at + Duration::from_millis(100_000), 100_000)
                .speed,
            Some(1000)
        );
    }

    #[test]
    fn time_that_runs_backwards_is_measured_again_not_trusted() {
        let (mut p, mut at, mut done) = (Pace::default(), Instant::now(), 0);
        seconds(&mut p, &mut at, &mut done, 10, MB);
        assert_eq!(p.measure(at - Duration::from_secs(5), done).speed, None);
    }
}
