use std::io;

use pctwin_scan::ReadPlan;

/// On an ordinary drive, this many read errors in a row mean it is failing.
const NORMAL_STOP_AFTER: u32 = 20;

/// Decides when to stop reading a drive that keeps failing (the ddrescue approach): copy what can
/// be read first, never hammer a bad area, and stop cleanly when errors come in a row rather than
/// strain a dying drive. Scattered errors do not add up; a good read resets the count.
#[derive(Debug, Clone)]
pub struct ReadBudget {
    stop_after: u32,
    in_a_row: u32,
    try_again_later: bool,
    stopped: bool,
}

impl ReadBudget {
    pub fn for_plan(plan: ReadPlan) -> Self {
        match plan {
            ReadPlan::Normal => Self {
                stop_after: NORMAL_STOP_AFTER,
                in_a_row: 0,
                try_again_later: true,
                stopped: false,
            },
            ReadPlan::Careful {
                read_each_file_once,
                stop_after_read_errors,
                ..
            } => Self {
                stop_after: stop_after_read_errors.max(1),
                in_a_row: 0,
                try_again_later: !read_each_file_once,
                stopped: false,
            },
        }
    }

    pub fn read_ok(&mut self) {
        self.in_a_row = 0;
    }

    /// Counts a drive read error; true when reading should now stop.
    pub fn read_failed(&mut self) -> bool {
        self.in_a_row += 1;
        if self.in_a_row >= self.stop_after {
            self.stopped = true;
        }
        self.stopped
    }

    /// Whether a file that failed to read gets one more try after everything else.
    pub fn try_again_later(&self) -> bool {
        self.try_again_later
    }

    pub fn stopped(&self) -> bool {
        self.stopped
    }
}

/// Whether an error came from the drive itself, rather than a file that is gone or locked.
pub fn is_drive_error(e: &io::Error) -> bool {
    !matches!(
        e.kind(),
        io::ErrorKind::NotFound
            | io::ErrorKind::PermissionDenied
            | io::ErrorKind::InvalidInput
            | io::ErrorKind::IsADirectory
            | io::ErrorKind::NotADirectory
    )
}
