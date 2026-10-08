/// The most lanes (connections) one move uses.
pub const MAX_LANES: u8 = 4;

/// A lane is kept only when total speed rises by at least this many tenths (10%).
const GAIN_TENTHS: f64 = 11.0;
/// Going down a lane is kept when speed stays at least this share of what it was: fewer lanes are
/// gentler on the network, so a near-tie goes to fewer.
const KEEP_FEWER: f64 = 0.95;
/// Speed below this share of the settled speed means the network changed.
const DROP: f64 = 0.75;
/// This many slow windows in a row start a fresh look.
const SLOW_WINDOWS: u8 = 3;
/// A settled choice is looked at again after this many windows, in case more lanes now help.
const LOOK_AGAIN_AFTER: u32 = 100;
/// Windows not judged after a change: a new connection starts slowly.
const WARM_WINDOWS: u8 = 1;

#[derive(Debug, Clone, Copy, PartialEq)]
enum Phase {
    /// Measure the speed at the current number of lanes, then try a step.
    Base {
        both_ways: bool,
    },
    /// Trying `lanes` after `from`, which ran at `base`.
    Trying {
        from: u8,
        base: f64,
        down: bool,
        both_ways: bool,
    },
    Settled {
        speed: f64,
        slow: u8,
        age: u32,
    },
}

/// Decides how many lanes to use by climbing one at a time (the GridFTP-APT approach: the speed of
/// several connections rises and then falls as more are added). The caller measures total speed
/// across all lanes over a window of a few seconds and reports it with [`LaneTuner::measured`];
/// report only windows in which there was always something waiting to be sent, so a move that is
/// running out of files is not mistaken for a slow network.
#[derive(Debug, Clone)]
pub struct LaneTuner {
    lanes: u8,
    most: u8,
    warm: u8,
    phase: Phase,
}

impl Default for LaneTuner {
    fn default() -> Self {
        Self::new()
    }
}

impl LaneTuner {
    /// One lane: the paired connection itself.
    pub fn new() -> Self {
        Self {
            lanes: 1,
            most: MAX_LANES,
            warm: WARM_WINDOWS,
            phase: Phase::Base { both_ways: false },
        }
    }

    /// How many lanes to have open now.
    pub fn lanes(&self) -> u8 {
        self.lanes
    }

    /// Whether the tuner has made its choice and is not trying a step.
    pub fn settled(&self) -> bool {
        matches!(self.phase, Phase::Settled { .. })
    }

    /// Reports the total speed (bytes a second) over the last window; returns how many lanes to
    /// have open next. Measurements that make no sense are ignored.
    pub fn measured(&mut self, speed: f64) -> u8 {
        if !speed.is_finite() || speed < 0.0 {
            return self.lanes;
        }
        if self.warm > 0 {
            self.warm -= 1;
            return self.lanes;
        }
        match self.phase {
            Phase::Base { both_ways } => self.step_from(speed, both_ways),
            Phase::Trying {
                from,
                base,
                down: false,
                both_ways,
            } => {
                if speed * 10.0 >= base * GAIN_TENTHS {
                    self.step_up_or_settle(speed, both_ways);
                } else {
                    self.change_to(from);
                    if both_ways && from > 1 {
                        self.try_down(base, true);
                    } else {
                        self.settle(base);
                    }
                }
            }
            Phase::Trying {
                from,
                base,
                down: true,
                ..
            } => {
                if speed >= base * KEEP_FEWER {
                    if self.lanes > 1 {
                        self.try_down(speed.max(base), true);
                    } else {
                        self.settle(speed);
                    }
                } else {
                    self.change_to(from);
                    self.settle(base);
                }
            }
            Phase::Settled {
                speed: settled,
                slow,
                age,
            } => {
                let slow = if speed < settled * DROP { slow + 1 } else { 0 };
                let age = age + 1;
                if slow >= SLOW_WINDOWS {
                    self.step_from(speed, true);
                } else if age >= LOOK_AGAIN_AFTER {
                    self.step_from(speed, false);
                } else {
                    self.phase = Phase::Settled {
                        speed: settled,
                        slow,
                        age,
                    };
                }
            }
        }
        self.lanes
    }

    /// The lane just tried could not be opened (a firewall, say): go back, and never try that
    /// many again in this move.
    pub fn could_not_open(&mut self) {
        if let Phase::Trying {
            from,
            base,
            down: false,
            ..
        } = self.phase
        {
            self.most = from;
            self.change_to(from);
            self.settle(base);
        } else {
            self.most = self.lanes.saturating_sub(1).max(1);
            if self.lanes > self.most {
                self.change_to(self.most);
            }
            self.phase = Phase::Base { both_ways: false };
        }
    }

    /// The lane just tried was refused this time: go back at once and settle, trying more lanes
    /// again only at the next look for them (unlike [`could_not_open`](Self::could_not_open),
    /// which rules that many out for the move).
    pub fn refused(&mut self) {
        if let Phase::Trying {
            from,
            base,
            down: false,
            ..
        } = self.phase
        {
            self.change_to(from);
            self.settle(base);
        }
    }

    /// A lane dropped. The first connection is never counted away; a new lane may replace it later.
    pub fn lane_lost(&mut self) {
        if self.lanes > 1 {
            self.change_to(self.lanes - 1);
            self.phase = Phase::Base { both_ways: false };
        }
    }

    /// From a measured speed at the current lanes, try one more (or one fewer when looking both
    /// ways and no more is allowed).
    fn step_from(&mut self, base: f64, both_ways: bool) {
        if self.lanes < self.most {
            self.phase = Phase::Trying {
                from: self.lanes,
                base,
                down: false,
                both_ways,
            };
            self.change_to(self.lanes + 1);
        } else if both_ways && self.lanes > 1 {
            self.try_down(base, both_ways);
        } else {
            self.settle(base);
        }
    }

    fn step_up_or_settle(&mut self, speed: f64, both_ways: bool) {
        if self.lanes < self.most {
            self.phase = Phase::Trying {
                from: self.lanes,
                base: speed,
                down: false,
                both_ways,
            };
            self.change_to(self.lanes + 1);
        } else {
            self.settle(speed);
        }
    }

    fn try_down(&mut self, base: f64, both_ways: bool) {
        self.phase = Phase::Trying {
            from: self.lanes,
            base,
            down: true,
            both_ways,
        };
        self.change_to(self.lanes - 1);
    }

    fn change_to(&mut self, lanes: u8) {
        if lanes != self.lanes {
            self.lanes = lanes;
            self.warm = WARM_WINDOWS;
        }
    }

    fn settle(&mut self, speed: f64) {
        self.phase = Phase::Settled {
            speed,
            slow: 0,
            age: 0,
        };
    }
}
