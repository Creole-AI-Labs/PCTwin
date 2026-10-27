//! A Windows restore point before the move's changes, with an honest result.
//!
//! What Windows does (Microsoft's `CreateRestorePoint` documentation, checked 26 October 2026):
//! it needs an administrator; when any restore point was made in the last 24 hours it silently
//! makes none and still answers success; and it makes none at all when System Protection is off
//! for the system drive. So the helper never trusts the answer alone: it lists the restore points
//! before and after, and reports one of three things:
//!
//! - **made**, at the time Windows gives;
//! - **an earlier one exists**, from the time Windows gives (made in the last 24 hours);
//! - **not made**, with the reason.
//!
//! It never changes `SystemRestorePointCreationFrequency` or turns System Protection on: those
//! are the person's settings.

use serde::{Deserialize, Serialize};

/// The restore point's description, fixed.
pub const DESCRIPTION: &str = "PCTwin: before moving to this laptop";

/// How long Windows waits between restore points made by programs.
pub const WINDOWS_WAIT_SECS: u64 = 24 * 60 * 60;

/// What happened.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub enum RestorePointResult {
    /// A new restore point was made, at this time (as Windows writes it), with its number in
    /// Windows' list.
    Made { at: String, number: u32 },
    /// Windows made none because it made one recently, at this time.
    EarlierExists { at: String, number: u32 },
    /// None was made.
    NotMade { reason: NotMadeReason },
}

/// Why no restore point was made.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum NotMadeReason {
    /// Restore points exist only on Windows.
    NotWindows,
    /// System Protection is off for the system drive (the person can turn it on in System
    /// Properties; PCTwin does not).
    ProtectionOff,
    /// It needs an administrator.
    NeedsAdmin,
    /// Windows said yes, but no new restore point and no recent one can be found.
    Unconfirmed,
    /// Windows refused or failed.
    Failed,
}

/// A restore point as Windows lists it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Point {
    pub sequence: u32,
    /// As Windows writes it: `yyyymmddHHMMSS.ffffff+UUU` (UTC offset in minutes).
    pub created: String,
}

/// Why listing or making failed: the system's code only, never its message.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SystemError {
    AccessDenied,
    ProtectionOff,
    Other(i64),
}

/// The system's restore points (Windows Management Instrumentation in the real helper, a fake in
/// tests).
pub trait RestorePoints {
    fn list(&mut self) -> Result<Vec<Point>, SystemError>;
    /// Asks for a restore point; `Ok` is Windows' return value (0 is success).
    fn create(&mut self, description: &str) -> Result<u32, SystemError>;
}

/// Seconds since 1970 (UTC) of a time as Windows writes it, or `None` if it is not one.
pub fn wmi_time_secs(s: &str) -> Option<i64> {
    let b = s.as_bytes();
    if b.len() != 25 || b[14] != b'.' || !matches!(b[21], b'+' | b'-') {
        return None;
    }
    let num = |r: std::ops::Range<usize>| -> Option<i64> {
        let t = s.get(r)?;
        if !t.bytes().all(|c| c.is_ascii_digit()) {
            return None;
        }
        t.parse().ok()
    };
    let (y, mo, d) = (num(0..4)?, num(4..6)?, num(6..8)?);
    let (h, mi, se) = (num(8..10)?, num(10..12)?, num(12..14)?);
    num(15..21)?;
    let offset = num(22..25)? * if b[21] == b'-' { -1 } else { 1 };
    if !(1..=12).contains(&mo) || !(1..=31).contains(&d) || h > 23 || mi > 59 || se > 60 {
        return None;
    }
    // Days from 1970-01-01 to y-mo-d (proleptic Gregorian; Howard Hinnant's method).
    let (y, mo) = if mo <= 2 {
        (y - 1, mo + 9)
    } else {
        (y, mo - 3)
    };
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let doy = (153 * mo + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    let days = era * 146_097 + doe - 719_468;
    Some(days * 86_400 + h * 3600 + mi * 60 + se - offset * 60)
}

/// Makes a restore point through `system`, and says honestly what happened; `now_secs` is the
/// time now, in seconds since 1970 (UTC).
pub fn create_honestly(system: &mut dyn RestorePoints, now_secs: i64) -> RestorePointResult {
    let not_made = |reason| RestorePointResult::NotMade { reason };
    let from = |e: SystemError| match e {
        SystemError::AccessDenied => NotMadeReason::NeedsAdmin,
        SystemError::ProtectionOff => NotMadeReason::ProtectionOff,
        SystemError::Other(_) => NotMadeReason::Failed,
    };
    let before = match system.list() {
        Ok(points) => points,
        Err(e) => return not_made(from(e)),
    };
    match system.create(DESCRIPTION) {
        Ok(0) => {}
        // ERROR_SERVICE_DISABLED: System Restore is off.
        Ok(1058) => return not_made(NotMadeReason::ProtectionOff),
        Ok(_) => return not_made(NotMadeReason::Failed),
        Err(e) => return not_made(from(e)),
    }
    let after = match system.list() {
        Ok(points) => points,
        Err(e) => return not_made(from(e)),
    };
    let newest = |points: &[Point]| points.iter().max_by_key(|p| p.sequence).cloned();
    if let Some(new) = newest(&after).filter(|n| before.iter().all(|b| b.sequence != n.sequence)) {
        return RestorePointResult::Made {
            at: new.created,
            number: new.sequence,
        };
    }
    match newest(&after) {
        Some(recent)
            if wmi_time_secs(&recent.created).is_some_and(|t| {
                // Within Windows' wait, allowing a little for the two clocks to differ.
                t <= now_secs + 300 && now_secs - t <= WINDOWS_WAIT_SECS as i64 + 300
            }) =>
        {
            RestorePointResult::EarlierExists {
                at: recent.created,
                number: recent.sequence,
            }
        }
        _ => not_made(NotMadeReason::Unconfirmed),
    }
}

/// The real thing, through Windows Management Instrumentation (`root\default`, the
/// `SystemRestore` class): safe Rust through the `wmi` crate the workspace already uses, so no
/// new unsafe code.
#[cfg(windows)]
pub struct WindowsRestorePoints;

#[cfg(windows)]
mod windows_wmi {
    use super::{Point, RestorePoints, SystemError, WindowsRestorePoints};
    use serde::{Deserialize, Serialize};

    #[derive(Deserialize)]
    #[expect(non_snake_case, reason = "WMI's own names")]
    struct SystemRestore {
        SequenceNumber: u32,
        CreationTime: String,
    }

    #[derive(Serialize)]
    #[expect(non_snake_case, reason = "WMI's own names")]
    struct CreateIn<'a> {
        Description: &'a str,
        /// APPLICATION_INSTALL.
        RestorePointType: u32,
        /// BEGIN_SYSTEM_CHANGE.
        EventType: u32,
    }

    #[derive(Deserialize)]
    #[expect(non_snake_case, reason = "WMI's own names")]
    struct CreateOut {
        ReturnValue: u32,
    }

    const WBEM_E_ACCESS_DENIED: i64 = 0x8004_1003;
    const E_ACCESSDENIED: i64 = 0x8007_0005;

    pub(super) fn code(e: &wmi::WMIError) -> SystemError {
        let c = match e {
            wmi::WMIError::HResultError { hres } => i64::from(*hres) & 0xFFFF_FFFF,
            _ => -1,
        };
        match c {
            WBEM_E_ACCESS_DENIED | E_ACCESSDENIED => SystemError::AccessDenied,
            // ERROR_SERVICE_DISABLED as an HRESULT.
            0x8007_0422 => SystemError::ProtectionOff,
            c => SystemError::Other(c),
        }
    }

    fn connect() -> Result<wmi::WMIConnection, SystemError> {
        wmi::WMIConnection::with_namespace_path("ROOT\\DEFAULT").map_err(|e| code(&e))
    }

    impl RestorePoints for WindowsRestorePoints {
        fn list(&mut self) -> Result<Vec<Point>, SystemError> {
            let wmi = connect()?;
            let points: Vec<SystemRestore> = wmi.query().map_err(|e| code(&e))?;
            Ok(points
                .into_iter()
                .map(|p| Point {
                    sequence: p.SequenceNumber,
                    created: p.CreationTime,
                })
                .collect())
        }

        fn create(&mut self, description: &str) -> Result<u32, SystemError> {
            let wmi = connect()?;
            let out: CreateOut = wmi
                .exec_class_method::<SystemRestore, _>(
                    "CreateRestorePoint",
                    CreateIn {
                        Description: description,
                        RestorePointType: 0,
                        EventType: 100,
                    },
                )
                .map_err(|e| code(&e))?;
            Ok(out.ReturnValue)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A system that behaves as Windows is documented to.
    struct Fake {
        points: Vec<Point>,
        list_error: Option<SystemError>,
        create: Result<u32, SystemError>,
        /// Whether a call to create really makes one (false: skipped within 24 hours, or off).
        makes: bool,
        asked: Vec<String>,
    }

    impl Fake {
        fn new(points: Vec<Point>, makes: bool) -> Self {
            Self {
                points,
                list_error: None,
                create: Ok(0),
                makes,
                asked: Vec::new(),
            }
        }
    }

    impl RestorePoints for Fake {
        fn list(&mut self) -> Result<Vec<Point>, SystemError> {
            match self.list_error {
                Some(e) => Err(e),
                None => Ok(self.points.clone()),
            }
        }
        fn create(&mut self, description: &str) -> Result<u32, SystemError> {
            self.asked.push(description.to_string());
            if self.makes {
                let sequence = self.points.iter().map(|p| p.sequence).max().unwrap_or(0) + 1;
                self.points.push(Point {
                    sequence,
                    created: "20261026120000.000000+060".into(),
                });
            }
            self.create
        }
    }

    fn at(created: &str, sequence: u32) -> Point {
        Point {
            sequence,
            created: created.into(),
        }
    }

    const NOW: &str = "20261026113000.000000+000";

    fn now() -> i64 {
        wmi_time_secs(NOW).unwrap()
    }

    #[test]
    fn windows_times_are_read_exactly() {
        assert_eq!(wmi_time_secs("19700101000000.000000+000"), Some(0));
        assert_eq!(wmi_time_secs("19700101010000.000000+060"), Some(0));
        assert_eq!(wmi_time_secs("19700101000000.000000-060"), Some(3600));
        assert_eq!(
            wmi_time_secs("20000301000000.000000+000"),
            Some(951_868_800)
        );
        assert_eq!(
            wmi_time_secs("20261026113000.000000+000"),
            Some(1_793_014_200)
        );
        assert_eq!(
            wmi_time_secs("20240229235959.000000+000"),
            Some(1_709_251_199)
        );
        for bad in [
            "",
            "2026102611300.000000+000",
            "20261026113000x000000+000",
            "20261026113000.000000*000",
            "20261326113000.000000+000",
            "20261000113000.000000+000",
            "20261026243000.000000+000",
            "20261026116000.000000+000",
            "20261026113061.000000+000",
            "2026102611300a.000000+000",
            "20261026113000.00000a+000",
            "20261026113000.000000+0a0",
        ] {
            assert_eq!(wmi_time_secs(bad), None, "{bad}");
        }
        assert!(wmi_time_secs("20261032113000.000000+000").is_none());
        assert!(wmi_time_secs("20261026113060.000000+000").is_some());
    }

    #[test]
    fn a_new_point_is_reported_as_made() {
        let mut sys = Fake::new(vec![at("20261001000000.000000+000", 4)], true);
        assert_eq!(
            create_honestly(&mut sys, now()),
            RestorePointResult::Made {
                at: "20261026120000.000000+060".into(),
                number: 5,
            }
        );
        assert_eq!(sys.asked, [DESCRIPTION]);
        let mut empty = Fake::new(vec![], true);
        assert!(matches!(
            create_honestly(&mut empty, now()),
            RestorePointResult::Made { .. }
        ));
    }

    /// Windows says yes but makes none because one is less than 24 hours old: reported as the
    /// earlier one, never as made.
    #[test]
    fn a_skipped_point_is_reported_as_the_earlier_one() {
        let recent = "20261026080000.000000+000";
        let mut sys = Fake::new(
            vec![at("20261001000000.000000+000", 4), at(recent, 5)],
            false,
        );
        assert_eq!(
            create_honestly(&mut sys, now()),
            RestorePointResult::EarlierExists {
                at: recent.into(),
                number: 5,
            }
        );
        // Exactly at the edge of Windows' wait, with its five minutes of slack.
        let edge = now() - WINDOWS_WAIT_SECS as i64 - 300;
        let mut sys = Fake::new(vec![at("20261025112500.000000+000", 1)], false);
        assert_eq!(wmi_time_secs("20261025112500.000000+000"), Some(edge));
        assert!(matches!(
            create_honestly(&mut sys, now()),
            RestorePointResult::EarlierExists { .. }
        ));
        let mut sys = Fake::new(vec![at("20261025112459.000000+000", 1)], false);
        assert_eq!(
            create_honestly(&mut sys, now()),
            RestorePointResult::NotMade {
                reason: NotMadeReason::Unconfirmed
            }
        );
        // From the future (more than five minutes): not trusted.
        let mut sys = Fake::new(vec![at("20261026113501.000000+000", 1)], false);
        assert_eq!(
            create_honestly(&mut sys, now()),
            RestorePointResult::NotMade {
                reason: NotMadeReason::Unconfirmed
            }
        );
        let mut sys = Fake::new(vec![at("20261026113500.000000+000", 1)], false);
        assert!(matches!(
            create_honestly(&mut sys, now()),
            RestorePointResult::EarlierExists { .. }
        ));
    }

    #[test]
    fn yes_with_nothing_to_show_is_not_made() {
        let mut sys = Fake::new(vec![at("20250101000000.000000+000", 1)], false);
        assert_eq!(
            create_honestly(&mut sys, now()),
            RestorePointResult::NotMade {
                reason: NotMadeReason::Unconfirmed
            }
        );
        let mut none = Fake::new(vec![], false);
        assert_eq!(
            create_honestly(&mut none, now()),
            RestorePointResult::NotMade {
                reason: NotMadeReason::Unconfirmed
            }
        );
    }

    #[test]
    fn refusals_say_why() {
        let cases = [
            (Err(SystemError::AccessDenied), NotMadeReason::NeedsAdmin),
            (
                Err(SystemError::ProtectionOff),
                NotMadeReason::ProtectionOff,
            ),
            (Err(SystemError::Other(5)), NotMadeReason::Failed),
            (Ok(1058), NotMadeReason::ProtectionOff),
            (Ok(1), NotMadeReason::Failed),
        ];
        for (create, reason) in cases {
            let mut sys = Fake::new(vec![], true);
            sys.create = create;
            assert_eq!(
                create_honestly(&mut sys, now()),
                RestorePointResult::NotMade { reason },
                "{create:?}"
            );
        }
        let mut sys = Fake::new(vec![], true);
        sys.list_error = Some(SystemError::AccessDenied);
        assert_eq!(
            create_honestly(&mut sys, now()),
            RestorePointResult::NotMade {
                reason: NotMadeReason::NeedsAdmin
            }
        );
        // Nothing was asked of Windows when the list could not be read.
        assert!(sys.asked.is_empty());
    }

    #[test]
    fn a_list_failing_after_making_is_not_made() {
        struct AfterFails(u32);
        impl RestorePoints for AfterFails {
            fn list(&mut self) -> Result<Vec<Point>, SystemError> {
                self.0 += 1;
                if self.0 > 1 {
                    Err(SystemError::Other(7))
                } else {
                    Ok(vec![])
                }
            }
            fn create(&mut self, _: &str) -> Result<u32, SystemError> {
                Ok(0)
            }
        }
        assert_eq!(
            create_honestly(&mut AfterFails(0), now()),
            RestorePointResult::NotMade {
                reason: NotMadeReason::Failed
            }
        );
    }

    /// Windows' two ways of saying "administrators only", and "System Restore is off".
    #[cfg(windows)]
    #[test]
    fn the_systems_codes_are_read_as_what_they_mean() {
        let of = |h: u32| windows_wmi::code(&wmi::WMIError::HResultError { hres: h as i32 });
        assert_eq!(of(0x8004_1003), SystemError::AccessDenied);
        assert_eq!(of(0x8007_0005), SystemError::AccessDenied);
        assert_eq!(of(0x8007_0422), SystemError::ProtectionOff);
        assert_eq!(of(0x8000_4005), SystemError::Other(0x8000_4005));
    }

    /// Run for real on this laptop without administrator rights: Windows refuses to list restore
    /// points, and the result is "not made: needs an administrator", never "made". Nothing is
    /// changed. (With administrator rights this test is skipped: it would really make one.)
    #[cfg(windows)]
    #[test]
    fn for_real_without_an_administrator_nothing_is_claimed() {
        let mut sys = WindowsRestorePoints;
        match sys.list() {
            Err(SystemError::AccessDenied) => {
                let now = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map_or(0, |d| d.as_secs() as i64);
                assert_eq!(
                    create_honestly(&mut sys, now),
                    RestorePointResult::NotMade {
                        reason: NotMadeReason::NeedsAdmin
                    }
                );
            }
            other => eprintln!("running as an administrator here ({other:?}); not run"),
        }
    }
}
