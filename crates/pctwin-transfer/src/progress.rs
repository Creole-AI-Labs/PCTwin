/// How much each second's speed counts in the smoothed speed: about the last 20 seconds matter
/// (2 / (window + 1)).
const ALPHA: f64 = 0.1;
/// Seconds measured before giving an estimate.
const WARM_UP: u32 = 5;
/// Seconds with nothing arriving before saying the move is waiting.
const STALLED: u32 = 10;

/// What to show for time left.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TimeLeft {
    /// The first seconds of the move: "Working out time left".
    WorkingOut,
    /// "About ..." (the screen rounds it to a friendly figure).
    About {
        seconds: u64,
    },
    /// Nothing has arrived for a while: "Waiting for the old laptop".
    Waiting,
    Paused,
    Done,
}

/// One progress figure and one time left for the whole move, from the bytes confirmed on every
/// lane. Call [`confirmed`](Self::confirmed) as receipts arrive and [`tick`](Self::tick) once a
/// second.
#[derive(Debug, Clone)]
pub struct Progress {
    total: u64,
    done: u64,
    this_second: u64,
    /// Smoothed bytes a second, once measured.
    speed: Option<f64>,
    seconds: u32,
    idle: u32,
    paused: bool,
}

impl Progress {
    pub fn new(total: u64) -> Self {
        Self {
            total,
            done: 0,
            this_second: 0,
            speed: None,
            seconds: 0,
            idle: 0,
            paused: false,
        }
    }

    /// The move's size changed (files found already there, or files added or skipped).
    pub fn set_total(&mut self, total: u64) {
        self.total = total;
    }

    /// Bytes the new laptop confirmed, on any lane.
    pub fn confirmed(&mut self, bytes: u64) {
        self.done = self.done.saturating_add(bytes);
        self.this_second = self.this_second.saturating_add(bytes);
    }

    /// One second passed. Paused seconds are not counted.
    pub fn tick(&mut self) {
        if self.paused {
            self.this_second = 0;
            return;
        }
        let now = self.this_second as f64;
        self.this_second = 0;
        self.speed = Some(match self.speed {
            Some(s) => ALPHA * now + (1.0 - ALPHA) * s,
            None => now,
        });
        self.seconds = self.seconds.saturating_add(1);
        self.idle = if now > 0.0 {
            0
        } else {
            self.idle.saturating_add(1)
        };
    }

    pub fn pause(&mut self) {
        self.paused = true;
    }

    pub fn resume(&mut self) {
        self.paused = false;
    }

    pub fn total_bytes(&self) -> u64 {
        self.total
    }

    pub fn done_bytes(&self) -> u64 {
        self.done
    }

    /// How much is done, from 0 to 1 (never past 1, even if files grew since the plan).
    pub fn fraction(&self) -> f64 {
        if self.total == 0 {
            1.0
        } else {
            (self.done as f64 / self.total as f64).min(1.0)
        }
    }

    pub fn time_left(&self) -> TimeLeft {
        if self.done >= self.total {
            return TimeLeft::Done;
        }
        if self.paused {
            return TimeLeft::Paused;
        }
        if self.idle >= STALLED {
            return TimeLeft::Waiting;
        }
        match self.speed {
            Some(speed) if self.seconds >= WARM_UP && speed > 0.0 => TimeLeft::About {
                seconds: ((self.total - self.done) as f64 / speed).ceil() as u64,
            },
            _ => TimeLeft::WorkingOut,
        }
    }
}
