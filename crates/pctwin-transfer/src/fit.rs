//! Before the move: will everything fit on the new laptop, and about how long will it take.
//!
//! Both are worked out only from what was measured: each approved place's free space as its
//! drive reports it, the plan's size, and the link speed a short test exchange measured after
//! pairing (see [`crate::probe`]). Where a number is not known the answer says so; it is never
//! guessed. The time is a range, because Wi-Fi speed changes during a move; the move meter
//! updates it once the move runs.

use std::collections::BTreeMap;
use std::time::Duration;

use pctwin_gate::Destinations;

/// Space left free on each drive after the move, for the system and the person's own use: a
/// drive filled to the last byte stops a laptop working.
pub const KEEP_FREE: u64 = 1024 * 1024 * 1024;

/// How one approved place fits what the plan puts in it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlaceFit {
    /// The place's label in the plan.
    pub destination: String,
    /// What the plan puts there, every file at its approved size.
    pub needed: u64,
    /// The space free there, as its drive reports it (`None`: not known).
    pub free: Option<u64>,
    /// How much of `needed` is an estimate, not a measured size ([`AlsoNeeded`]): the place's
    /// fit is then said with "about".
    pub estimated: u64,
}

/// Space the move uses on this laptop beyond its files, at `at` (a folder there), estimated
/// before it starts: the installers downloaded and the apps installed, the app data staged and
/// then landed (gap 10a of the complications audit, 28 October 2026). Counted on the drive that
/// holds `at`, together with the places of the move on that drive.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AlsoNeeded {
    pub at: std::path::PathBuf,
    pub bytes: u64,
}

/// The place an estimate is shown under when no place of the move is on its drive.
pub const WORK_PLACE: &str = "pctwin-work";

impl PlaceFit {
    /// Whether it fits with [`KEEP_FREE`] to spare (`None`: the free space is not known).
    pub fn fits(&self) -> Option<bool> {
        self.free
            .map(|free| self.needed.saturating_add(KEEP_FREE) <= free)
    }

    /// How much more space it needs to fit with [`KEEP_FREE`] to spare (0 when it fits or is not
    /// known).
    pub fn short_by(&self) -> u64 {
        self.free.map_or(0, |free| {
            self.needed.saturating_add(KEEP_FREE).saturating_sub(free)
        })
    }
}

/// Whether the whole plan fits: place by place.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Fit {
    pub places: Vec<PlaceFit>,
}

impl Fit {
    /// Each place in `needed` against the free space of its approved folder in `table`. Places
    /// on the same drive share its free space, so they are judged together: every place on one
    /// drive is shown as needing what all of them need there. A place whose drive cannot be
    /// told is judged on its own.
    pub fn check(needed: &BTreeMap<String, u64>, table: &Destinations) -> Self {
        let found: Vec<_> = needed
            .iter()
            .map(|(d, n)| (d.clone(), *n, table.get(d).ok()))
            .collect();
        let places = found
            .iter()
            .map(|(destination, own, dest)| {
                let together = found
                    .iter()
                    .filter(|(other, _, o)| {
                        other == destination
                            || matches!((dest, o), (Some(a), Some(b)) if a.same_drive_as(b) == Some(true))
                    })
                    .fold(0u64, |t, (_, n, _)| t.saturating_add(*n));
                PlaceFit {
                    destination: destination.clone(),
                    needed: together.max(*own),
                    free: dest.and_then(|d| d.available_bytes()),
                    estimated: 0,
                }
            })
            .collect();
        Self { places }
    }

    /// With the space the move uses beyond its files (`also`): each estimate is added to every
    /// place on its drive (they share its free space); one whose drive holds no place of the move
    /// is a place of its own ([`WORK_PLACE`]). A folder that cannot be opened is counted against
    /// every place, the cautious way.
    pub fn with_also(mut self, table: &Destinations, also: &[AlsoNeeded]) -> Self {
        for a in also.iter().filter(|a| a.bytes > 0) {
            let at = pctwin_gate::Destination::open(&a.at).ok();
            let mut counted = false;
            for p in &mut self.places {
                let same = match (&at, table.get(&p.destination).ok()) {
                    (Some(at), Some(d)) => d.same_drive_as(at) != Some(false),
                    _ => true,
                };
                if same {
                    p.needed = p.needed.saturating_add(a.bytes);
                    p.estimated = p.estimated.saturating_add(a.bytes);
                    counted = true;
                }
            }
            if !counted {
                match self.places.iter_mut().find(|p| p.destination == WORK_PLACE) {
                    Some(p) => {
                        p.needed = p.needed.saturating_add(a.bytes);
                        p.estimated = p.estimated.saturating_add(a.bytes);
                    }
                    None => self.places.push(PlaceFit {
                        destination: WORK_PLACE.into(),
                        needed: a.bytes,
                        free: at.as_ref().and_then(|d| d.available_bytes()),
                        estimated: a.bytes,
                    }),
                }
            }
        }
        self
    }

    /// Whether any place's need holds an estimate (said with "about").
    pub fn estimated(&self) -> u64 {
        self.places.iter().map(|p| p.estimated).max().unwrap_or(0)
    }

    /// `Some(true)` when every place fits, `Some(false)` when any does not, `None` when none is
    /// short but at least one could not be checked.
    pub fn fits(&self) -> Option<bool> {
        let mut known = true;
        for p in &self.places {
            match p.fits() {
                Some(false) => return Some(false),
                Some(true) => {}
                None => known = false,
            }
        }
        known.then_some(true)
    }
}

/// About how long the move takes, before it starts.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TimeRange {
    /// The link speed was not measured (the other laptop is on an older version, or the test
    /// exchange could not finish): no time is given.
    NotKnown,
    /// Between `fastest` (the whole move at the measured speed) and `slowest` (at half of it,
    /// as Wi-Fi often slows during a long move).
    Between {
        fastest: Duration,
        slowest: Duration,
    },
}

impl TimeRange {
    /// `bytes` to move at `bytes_per_second` measured (`None` or 0: not measured).
    pub fn of(bytes: u64, bytes_per_second: Option<u64>) -> Self {
        match bytes_per_second {
            Some(speed) if speed > 0 => {
                let fastest = bytes.div_ceil(speed).max(1);
                let slowest = bytes.div_ceil(speed.div_ceil(2)).max(1);
                Self::Between {
                    fastest: Duration::from_secs(fastest),
                    slowest: Duration::from_secs(slowest),
                }
            }
            _ => Self::NotKnown,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const GB: u64 = 1024 * 1024 * 1024;

    /// Gap 10a (28 October 2026): the space the move uses beyond its files (installers, apps,
    /// app data) is counted on the drive it lands on, with the files there, and said as an
    /// estimate; on a drive with no place of the move it is a place of its own.
    #[test]
    fn installers_and_app_data_count_against_the_drive_they_land_on() {
        let dir = tempfile::tempdir().unwrap();
        let docs = dir.path();
        let mut table = Destinations::new();
        table
            .approve("documents", pctwin_gate::Approved::MyFolders, docs)
            .unwrap();
        let needed: BTreeMap<String, u64> = [("documents".to_string(), 3 * GB)].into();
        let files_only = Fit::check(&needed, &table);
        assert_eq!(files_only.estimated(), 0);
        let also = [
            AlsoNeeded {
                at: dir.path().to_path_buf(),
                bytes: 2 * GB,
            },
            AlsoNeeded {
                at: dir.path().to_path_buf(),
                bytes: 0,
            },
        ];
        let fit = Fit::check(&needed, &table).with_also(&table, &also);
        assert_eq!(fit.places.len(), 1);
        assert_eq!(fit.places[0].needed, 5 * GB);
        assert_eq!(fit.places[0].estimated, 2 * GB);
        assert_eq!(fit.estimated(), 2 * GB);
        assert!(fit.places[0].short_by() >= files_only.places[0].short_by());
        // Nothing of the move on any drive: the estimate is a place of its own.
        let none = Fit::check(&BTreeMap::new(), &table).with_also(&table, &also);
        assert_eq!(none.places.len(), 1);
        assert_eq!(none.places[0].destination, WORK_PLACE);
        assert_eq!(none.places[0].needed, 2 * GB);
        assert!(none.places[0].free.is_some());
    }

    #[test]
    fn a_place_fits_only_with_room_to_spare() {
        let at = |needed, free| PlaceFit {
            destination: "me".into(),
            needed,
            free,
            estimated: 0,
        };
        assert_eq!(at(10 * GB, Some(12 * GB)).fits(), Some(true));
        assert_eq!(at(10 * GB, Some(11 * GB)).fits(), Some(true));
        assert_eq!(at(10 * GB, Some(11 * GB - 1)).fits(), Some(false));
        assert_eq!(at(10 * GB, Some(11 * GB - 1)).short_by(), 1);
        assert_eq!(at(10 * GB, Some(5 * GB)).short_by(), 6 * GB);
        assert_eq!(at(10 * GB, None).fits(), None);
        assert_eq!(at(10 * GB, None).short_by(), 0);
    }

    #[test]
    fn the_whole_plan_fits_only_when_every_place_does() {
        let p = |needed, free| PlaceFit {
            destination: "x".into(),
            needed,
            free,
            estimated: 0,
        };
        let fit = |places| Fit { places };
        assert_eq!(
            fit(vec![p(1, Some(2 * GB)), p(1, Some(3 * GB))]).fits(),
            Some(true)
        );
        assert_eq!(
            fit(vec![p(1, Some(2 * GB)), p(GB, Some(GB))]).fits(),
            Some(false)
        );
        assert_eq!(fit(vec![p(1, Some(2 * GB)), p(1, None)]).fits(), None);
        // One that does not fit decides it, whatever could not be checked.
        assert_eq!(fit(vec![p(GB, Some(GB)), p(1, None)]).fits(), Some(false));
    }

    #[test]
    fn the_time_is_a_range_from_the_measured_speed_to_half_of_it() {
        assert_eq!(TimeRange::of(1000, None), TimeRange::NotKnown);
        assert_eq!(TimeRange::of(1000, Some(0)), TimeRange::NotKnown);
        assert_eq!(
            TimeRange::of(100 * 1024 * 1024, Some(10 * 1024 * 1024)),
            TimeRange::Between {
                fastest: Duration::from_secs(10),
                slowest: Duration::from_secs(20)
            }
        );
        // Never "no time at all".
        assert_eq!(
            TimeRange::of(1, Some(1024)),
            TimeRange::Between {
                fastest: Duration::from_secs(1),
                slowest: Duration::from_secs(1)
            }
        );
    }
}
