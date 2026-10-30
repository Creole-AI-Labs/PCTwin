//! Saved Wi-Fi networks in the move (Task List 2.4, decided 20 October 2026; move protocol 8):
//! the choose page on the old laptop, reading the ticked networks there, the plan's lines and
//! adding them on the new laptop. The engine is `pctwin_fullmove::wifi`; passwords live only in
//! its wiped buffers and the sealed link, never in a view, an event, a log or the AI.

use pctwin_fullmove::wifi::{WifiDone, WifiHeld, WifiKind, WifiListed, WifiWhy};
use pctwin_settings::wifi::PrivateFolders;
use serde::Serialize;
use zeroize::Zeroizing;

/// The owner-only folders the system's Wi-Fi tools write into: made through the helper's fenced
/// system calls (Windows), mode 0700 from their first moment (Mac and Linux).
pub struct AppFolders;

impl PrivateFolders for AppFolders {
    fn make(&self, path: &std::path::Path) -> Result<(), pctwin_settings::wifi::FolderError> {
        use pctwin_settings::wifi::FolderError;
        pctwin_helper_os::private_folder(path).map_err(|e| match e {
            pctwin_helper_os::Error::NameTaken => FolderError::NameTaken,
            _ => FolderError::Failed,
        })
    }
    #[cfg(windows)]
    fn access_list(&self, path: &std::path::Path) -> Option<String> {
        pctwin_helper_os::folder_access_list(path).ok()
    }
    #[cfg(not(windows))]
    fn access_list(&self, _: &std::path::Path) -> Option<String> {
        None
    }
    fn me(&self) -> Option<String> {
        match pctwin_helper_os::current_account().ok()? {
            pctwin_helper_os::Account::Sid(s) => Some(s.as_str().to_string()),
            pctwin_helper_os::Account::Uid(_) => None,
        }
    }
}

/// One saved network on the old laptop's choose page.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WifiOffered {
    /// Its name, shown exactly as it is.
    pub name: String,
    /// Only open and password networks can be ticked; the others are said.
    pub kind: WifiKind,
}

/// The choose page's saved Wi-Fi networks (the 20 October 2026 rule).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WifiChoice {
    pub networks: Vec<WifiOffered>,
    /// The person moving is one of this laptop's administrators: the networks start ticked (one
    /// system prompt when they are read). Not: none starts ticked, and the page says "ask
    /// whoever looks after this laptop".
    pub ticked: bool,
}

/// The old laptop: the networks found here before the choose page (no prompt), and whether the
/// person moving is one of its administrators.
#[derive(Debug, Clone, Default)]
pub struct WifiFound {
    pub offered: Vec<(String, WifiKind)>,
    pub admin: bool,
}

impl WifiFound {
    /// Reads this laptop (blocking: the system's own tools, a few seconds at most).
    pub fn here() -> Self {
        let found = pctwin_fullmove::wifi::found_here(&AppFolders);
        Self {
            offered: pctwin_fullmove::wifi::offered(&found),
            admin: pctwin_settings::wifi::mover_is_admin(
                &mut pctwin_settings::runner::SystemRunner,
            ),
        }
    }

    /// What the choose page shows (`None`: no network found).
    pub fn view(&self) -> Option<WifiChoice> {
        (!self.offered.is_empty()).then(|| WifiChoice {
            networks: self
                .offered
                .iter()
                .map(|(name, kind)| WifiOffered {
                    name: name.clone(),
                    kind: *kind,
                })
                .collect(),
            ticked: pctwin_settings::wifi::offer(self.admin).ticked,
        })
    }

    /// The names of the networks ticked (`ticked`: their places on the choose page); anything
    /// not offered, or that cannot move, counts for nothing.
    pub fn ticked_names(&self, ticked: &[u32]) -> Vec<String> {
        self.offered
            .iter()
            .enumerate()
            .filter(|(at, (_, kind))| kind.movable() && ticked.contains(&(*at as u32)))
            .map(|(_, (name, _))| name.clone())
            .collect()
    }
}

/// The old laptop: reads the ticked networks with their passwords, through the system's own
/// prompt where it needs one, once. Windows: through the helper already running for this move
/// when it was started for them (`helper`), else a helper started for them alone (one prompt);
/// a Mac and Linux: their own prompts. What was not read moves without its password (the new
/// laptop says to type it).
pub fn read_ticked(
    found: &WifiFound,
    ticked: &[u32],
    helper: Option<&crate::others::Helper>,
    mover: &str,
    laptop: pctwin_record::LaptopId,
    stopped: &dyn Fn() -> bool,
) -> WifiHeld {
    let names = found.ticked_names(ticked);
    if names.is_empty() {
        return WifiHeld::new(&found.offered, ticked, Vec::new());
    }
    #[cfg(windows)]
    let read = {
        let profiles: Option<Vec<Zeroizing<String>>> = match helper {
            // One helper per move: the one running answers, or the networks are not read.
            Some(h) => h
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
                .wifi_profiles(&names)
                .ok(),
            None => {
                match crate::others::launch_elevated_with(mover, &[], laptop, None, true, stopped) {
                    Ok(Some(mut client)) => {
                        let got = client.wifi_profiles(&names).ok();
                        let _ = client.bye();
                        got
                    }
                    _ => None,
                }
            }
        };
        pctwin_fullmove::wifi::parse_profiles(profiles.unwrap_or_default(), &names)
    };
    #[cfg(not(windows))]
    let read = {
        let _ = (helper, mover, laptop, stopped);
        pctwin_fullmove::wifi::read_ticked_here(&names)
    };
    WifiHeld::new(&found.offered, ticked, read)
}

/// The new laptop: the old laptop's record, as it arrived, until the networks are added.
#[derive(Default)]
pub struct WifiArrival {
    record: Option<Zeroizing<Vec<u8>>>,
}

impl pctwin_move::wifi::WifiIn for WifiArrival {
    fn arrived(&mut self, record: Zeroizing<Vec<u8>>) {
        self.record = Some(record);
    }
    fn lost(&mut self) {
        self.record = None;
    }
}

/// What the plan says will happen to one network.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum WifiStep {
    /// Added for the person here.
    Add,
    /// A Mac: added once the person installs one profile in System Settings.
    AddByProfile,
    /// The old laptop's person left it unticked.
    Skipped,
    /// Set up with an organisation: the person joins it again with their details.
    Organisation,
    /// A kind PCTwin does not move.
    NotThisKind,
    /// Its password could not be read on the old laptop: the person types it once.
    NoPassword,
}

/// One network on the plan (M23).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WifiPlanLine {
    pub name: String,
    pub step: WifiStep,
}

/// The plan's network lines, from what the old laptop listed.
pub fn plan_lines(listed: &[WifiListed]) -> Vec<WifiPlanLine> {
    listed
        .iter()
        .map(|l| WifiPlanLine {
            name: l.name.clone(),
            step: match (l.kind, l.ticked, l.read) {
                (WifiKind::Organisation, ..) => WifiStep::Organisation,
                (WifiKind::Other, ..) => WifiStep::NotThisKind,
                (_, false, _) => WifiStep::Skipped,
                (_, true, false) => WifiStep::NoPassword,
                _ if cfg!(target_os = "macos") => WifiStep::AddByProfile,
                _ => WifiStep::Add,
            },
        })
        .collect()
}

/// The new laptop, once the files are in: adds the networks that arrived (a record that does
/// not hold together adds none) through the system's own tools; `waiting`: a Mac's minutes
/// left for the person to install the profile.
pub fn add_here(
    listed: &[WifiListed],
    arrival: WifiArrival,
    stopped: &dyn Fn() -> bool,
    waiting: &mut dyn FnMut(u64),
) -> Vec<WifiDone> {
    if listed.is_empty() {
        return Vec::new();
    }
    let arrived = arrival
        .record
        .as_deref()
        .and_then(|bytes| pctwin_fullmove::wifi::decode_record(bytes, listed).ok());
    drop(arrival);
    let mut here = pctwin_fullmove::wifi::SystemWifi {
        folders: &AppFolders,
        parent: std::env::temp_dir(),
        stopped,
        waiting,
    };
    pctwin_fullmove::wifi::add_networks(listed, arrived.as_deref(), &mut here)
}

/// The note an outcome line carries for why a network needs the person.
pub fn why_note(why: Option<WifiWhy>) -> Vec<crate::fullmove::LineNote> {
    why.map(|why| crate::fullmove::LineNote::Wifi { why })
        .into_iter()
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn found() -> WifiFound {
        WifiFound {
            offered: vec![
                ("Home".into(), WifiKind::Password),
                ("Office".into(), WifiKind::Organisation),
                ("Cafe".into(), WifiKind::Open),
            ],
            admin: true,
        }
    }

    /// The 20 October 2026 rule on the choose page: ticked for an administrator, not otherwise.
    #[test]
    fn the_choose_page_starts_ticked_only_for_an_administrator() {
        let f = found();
        let v = f.view().unwrap();
        assert!(v.ticked);
        assert_eq!(v.networks.len(), 3);
        assert!(
            !WifiFound {
                admin: false,
                ..found()
            }
            .view()
            .unwrap()
            .ticked
        );
        assert_eq!(WifiFound::default().view(), None);
        // An organisation's network or one not offered never counts as ticked.
        assert_eq!(f.ticked_names(&[0, 1, 2, 7]), ["Home", "Cafe"]);
    }

    #[test]
    fn the_plan_says_each_networks_step() {
        let listed = vec![
            WifiListed {
                name: "Home".into(),
                kind: WifiKind::Password,
                ticked: true,
                read: true,
            },
            WifiListed {
                name: "Lab".into(),
                kind: WifiKind::Password,
                ticked: true,
                read: false,
            },
            WifiListed {
                name: "Gran".into(),
                kind: WifiKind::Open,
                ticked: false,
                read: false,
            },
            WifiListed {
                name: "Office".into(),
                kind: WifiKind::Organisation,
                ticked: false,
                read: false,
            },
        ];
        let steps: Vec<WifiStep> = plan_lines(&listed).iter().map(|l| l.step).collect();
        let add = if cfg!(target_os = "macos") {
            WifiStep::AddByProfile
        } else {
            WifiStep::Add
        };
        assert_eq!(
            steps,
            [
                add,
                WifiStep::NoPassword,
                WifiStep::Skipped,
                WifiStep::Organisation
            ]
        );
        // Nothing listed: nothing added, nothing asked of the system.
        assert!(add_here(&[], WifiArrival::default(), &|| false, &mut |_| {}).is_empty());
    }
}
