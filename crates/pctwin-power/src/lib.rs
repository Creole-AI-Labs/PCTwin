//! Keeping both laptops awake during a move, and watching the battery (Task List 1.5).
//!
//! - [`stay_awake`] holds off *idle* sleep while a move runs, using each system's own request
//!   (Windows `SetThreadExecutionState`, macOS power assertions, Linux systemd inhibitor locks,
//!   through the `keepawake` library). Dropping the handle releases it at once. Sleep the person
//!   causes (closing the lid, the power button) is never blocked, and their power settings are
//!   never changed. On many Windows 11 laptops (Modern Standby) on battery, Windows ignores the
//!   request once the screen turns off, so on battery the screen is kept on too.
//! - [`power_now`] reads whether the laptop is on battery and how full it is
//!   (`starship-battery`); [`advise`] turns that into what to tell the person.

use serde::Serialize;

/// Below this, on battery, the person is asked to plug in.
const PLUG_IN_BELOW: f32 = 20.0;
/// At or below this, on battery, the move pauses safely (it resumes from the exact block).
const PAUSE_AT: f32 = 8.0;

/// Holds the "stay awake" request while it lives.
pub struct StayAwake {
    guard: Option<keepawake::KeepAwake>,
    why_not: Option<String>,
}

impl StayAwake {
    /// Whether the system accepted the request, or why not (shown under "more details").
    pub fn held(&self) -> Result<(), &str> {
        match (&self.guard, &self.why_not) {
            (Some(_), _) => Ok(()),
            (None, Some(why)) => Err(why),
            (None, None) => Err("the system did not accept the request"),
        }
    }
}

/// Asks the system not to sleep while idle, for `reason` (shown by the system where it lists such
/// requests). `keep_screen_on` is for running on battery, where some Windows laptops otherwise
/// stop honouring the request when the screen turns off.
pub fn stay_awake(reason: &str, keep_screen_on: bool) -> StayAwake {
    match keepawake::Builder::default()
        .idle(true)
        .display(keep_screen_on)
        .reason(reason)
        .app_name("PCTwin")
        .app_reverse_domain("com.creoleailabs.pctwin")
        .create()
    {
        Ok(guard) => StayAwake {
            guard: Some(guard),
            why_not: None,
        },
        Err(e) => StayAwake {
            guard: None,
            why_not: Some(e.to_string()),
        },
    }
}

/// The laptop's power right now.
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub struct Power {
    pub on_battery: bool,
    /// Charge, 0 to 100, when the system reports it.
    pub percent: Option<f32>,
}

/// Reads the power state. With no battery (a desktop) or no answer, it reports not on battery.
pub fn power_now() -> Power {
    let batteries = starship_battery::Manager::new()
        .and_then(|m| m.batteries().map(|b| b.flatten().collect::<Vec<_>>()));
    let Ok(batteries) = batteries else {
        return Power {
            on_battery: false,
            percent: None,
        };
    };
    if batteries.is_empty() {
        return Power {
            on_battery: false,
            percent: None,
        };
    }
    let on_battery = batteries
        .iter()
        .any(|b| b.state() == starship_battery::State::Discharging);
    // Several batteries: their combined charge.
    let (full, now): (f32, f32) = batteries.iter().fold((0.0, 0.0), |(f, n), b| {
        (f + b.energy_full().value, n + b.energy().value)
    });
    let percent = (full > 0.0).then(|| (now / full * 100.0).clamp(0.0, 100.0));
    Power {
        on_battery,
        percent,
    }
}

/// What to tell the person about power during a move.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum BatteryAdvice {
    Fine,
    /// On battery: plugging in is better for a long move.
    OnBattery,
    /// Low: "Plug in this laptop."
    PlugIn,
    /// Critical: pause safely now; the move continues from the exact block later.
    PauseNow,
}

pub fn advise(power: &Power) -> BatteryAdvice {
    if !power.on_battery {
        return BatteryAdvice::Fine;
    }
    match power.percent {
        Some(p) if p <= PAUSE_AT => BatteryAdvice::PauseNow,
        Some(p) if p <= PLUG_IN_BELOW => BatteryAdvice::PlugIn,
        _ => BatteryAdvice::OnBattery,
    }
}
