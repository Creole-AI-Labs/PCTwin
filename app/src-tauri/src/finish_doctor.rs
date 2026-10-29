//! The full connection doctor (Task List 2.7 "Full connection doctor, including phone-hotspot
//! fallback"; screen C08): every probe the doctor has, run on this laptop, turned into findings
//! by `pctwin_doctor`'s rules. A probe that could not run is said by its key and never blamed.
//! The phone hotspot is always offered as the last resort (both laptops join the phone's own
//! network; the move goes between the laptops, not through the phone's data), and suggested first
//! when the findings point at the network itself.

use std::time::Duration;

use pctwin_doctor::{Cause, Certainty, Evidence, Role, probe};

use crate::finish::{DoctorRole, DoctorView, FindingView, HotspotView};

/// How long this laptop listens for its own announcement.
const HEAR_WAIT: Duration = Duration::from_secs(3);

/// What the probes saw, and which could not run.
fn gather(role: DoctorRole) -> (Evidence, Vec<String>) {
    let mut not_checked = Vec::new();
    let interfaces = probe::interfaces();
    if interfaces.is_none() {
        not_checked.push("doctor-probe-interfaces".to_string());
    }
    let hears_itself = probe::hears_itself(HEAR_WAIT);
    if hears_itself.is_none() {
        not_checked.push("doctor-probe-hears-itself".to_string());
    }
    let is_macos = cfg!(target_os = "macos");
    let multicast_send_blocked = if is_macos {
        let b = probe::multicast_send_blocked();
        if b.is_none() {
            not_checked.push("doctor-probe-local-network".to_string());
        }
        b
    } else {
        None
    };
    let windows_profiles = if cfg!(windows) {
        match probe::windows_profiles_within(probe::WINDOWS_PROBE_TIMEOUT) {
            Some(p) => p,
            None => {
                not_checked.push("doctor-probe-network-profiles".to_string());
                Vec::new()
            }
        }
    } else {
        Vec::new()
    };
    (
        Evidence {
            interfaces,
            hears_itself,
            multicast_send_blocked,
            windows_profiles,
            // The doctor is asked because the laptops did not find each other.
            old_laptops_found: (role == DoctorRole::New).then_some(0),
            is_macos,
        },
        not_checked,
    )
}

/// The view of what the rules found.
pub fn view(e: &Evidence, role: DoctorRole, not_checked: Vec<String>) -> DoctorView {
    let d = pctwin_doctor::diagnose(
        e,
        match role {
            DoctorRole::Old => Role::OldLaptop,
            DoctorRole::New => Role::NewLaptop,
        },
    );
    DoctorView {
        findings: d
            .findings
            .iter()
            .map(|f| FindingView {
                cause: f.cause.message_key().to_string(),
                fix: f.cause.fix_key().to_string(),
                sure: f.certainty == Certainty::Sure,
                names: match &f.cause {
                    Cause::WindowsPublicNetwork(n) => vec![n.clone()],
                    Cause::VpnOn(v) => v.clone(),
                    _ => Vec::new(),
                },
            })
            .collect(),
        hotspot: HotspotView {
            suggested: d.suggest_phone_hotspot,
        },
        not_checked,
    }
}

/// Runs every probe on this laptop and says what the rules found.
pub fn run(role: DoctorRole) -> DoctorView {
    let (e, not_checked) = gather(role);
    view(&e, role, not_checked)
}

#[cfg(test)]
mod tests {
    use super::*;
    use pctwin_doctor::{Interface, NetworkProfile};

    fn wifi() -> Interface {
        Interface::new("Wi-Fi", "192.168.1.20".parse().unwrap())
    }

    /// A Public network is named with its fix; a guest network that hides devices suggests the
    /// phone's hotspot; with no network at all, the hotspot is not suggested (connect first).
    #[test]
    fn findings_carry_their_keys_names_and_the_hotspot_fallback() {
        let e = Evidence {
            interfaces: Some(vec![wifi()]),
            hears_itself: Some(true),
            windows_profiles: vec![("Wi-Fi".into(), NetworkProfile::Public)],
            old_laptops_found: Some(0),
            ..Evidence::default()
        };
        let v = view(&e, DoctorRole::New, vec![]);
        assert!(
            v.findings
                .iter()
                .any(|f| f.cause == "doctor-windows-public-network"
                    && f.fix == "doctor-windows-public-network-fix"
                    && f.names == ["Wi-Fi"]
                    && !f.sure)
        );
        let e = Evidence {
            interfaces: Some(vec![wifi()]),
            hears_itself: Some(true),
            old_laptops_found: Some(0),
            ..Evidence::default()
        };
        let v = view(&e, DoctorRole::New, vec![]);
        assert!(v.hotspot.suggested);
        assert!(v.findings.iter().any(|f| f.cause == "doctor-guest-network"));
        let e = Evidence {
            interfaces: Some(vec![]),
            ..Evidence::default()
        };
        let v = view(&e, DoctorRole::New, vec![]);
        assert_eq!(v.findings.len(), 1);
        assert!(v.findings[0].sure && v.findings[0].cause == "doctor-not-connected");
        assert!(!v.hotspot.suggested);
    }

    /// What a probe could not see is said, and nothing is blamed for it.
    #[test]
    fn a_probe_that_could_not_run_is_said_and_never_blamed() {
        let e = Evidence::default();
        let v = view(&e, DoctorRole::Old, vec!["doctor-probe-interfaces".into()]);
        assert!(v.findings.is_empty());
        assert_eq!(v.not_checked, ["doctor-probe-interfaces"]);
    }
}
