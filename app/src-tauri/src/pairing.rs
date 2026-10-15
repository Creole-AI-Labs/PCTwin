//! Pairing for the screens (C03 to C05): the old and new laptop's journey (`pctwin-journey`) as
//! commands the screens call and events they listen to. Everything the screens receive is plain
//! data: a label as word keys to translate, a code and its countdown, numbers, and problems as
//! translation keys, never engine text.

use std::time::Duration;

use pctwin_discovery::{Found, Label};
use pctwin_journey::{NewJourney, OldJourney, OldUpdate, Problem, Waiting};
use pctwin_link::{Link, LinkConfig};
use pctwin_session::OldLaptop;
use serde::Serialize;
use tauri::{AppHandle, Emitter, Runtime, State};
use tokio::sync::Mutex;

/// The old laptop's screen updates.
pub const OLD_EVENT: &str = "pairing-old";
/// The new laptop's screen updates.
pub const NEW_EVENT: &str = "pairing-new";

/// How long the new laptop looks for old laptops nearby.
const SEARCH_WAIT: Duration = Duration::from_secs(3);

/// Pairing in progress on this laptop, and its result.
#[derive(Default)]
pub struct Pairing {
    old: Mutex<Option<OldJourney<OldLaptop>>>,
    found: Mutex<Vec<Found>>,
    /// The paired connection, kept for the move.
    link: Mutex<Option<Link>>,
}

/// A label as the screens show it: a colour and an animal to translate, or the typed name.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum LabelView {
    Picked {
        colour: &'static str,
        animal: &'static str,
    },
    Named {
        name: String,
    },
}

impl From<&Label> for LabelView {
    fn from(label: &Label) -> Self {
        match (label.keys(), label) {
            (Some((colour, animal)), _) => Self::Picked { colour, animal },
            (None, Label::Named(name)) => Self::Named { name: name.clone() },
            // A colour or animal out of range never comes from this app; shown as unnamed.
            (None, Label::Picked { .. }) => Self::Named {
                name: String::new(),
            },
        }
    }
}

/// What the old laptop's screen shows now.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum OldView {
    Label {
        label: LabelView,
    },
    Code {
        code: String,
        #[serde(rename = "secondsLeft")]
        seconds_left: u64,
    },
    Wait {
        #[serde(rename = "secondsLeft")]
        seconds_left: u64,
    },
    Locked,
    Choose {
        numbers: [u8; 3],
    },
    Paired,
    TimedOut,
}

impl From<OldUpdate> for OldView {
    fn from(u: OldUpdate) -> Self {
        match u {
            OldUpdate::Label(l) => Self::Label { label: (&l).into() },
            OldUpdate::Code { code, seconds_left } => Self::Code { code, seconds_left },
            OldUpdate::Wait { seconds_left } => Self::Wait { seconds_left },
            OldUpdate::Locked => Self::Locked,
            OldUpdate::Choose(numbers) => Self::Choose { numbers },
            OldUpdate::Paired => Self::Paired,
            OldUpdate::TimedOut => Self::TimedOut,
        }
    }
}

/// What the new laptop's screen hears after its number is shown.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum NewView {
    Paired,
    Problem { problem: ProblemView },
}

/// A problem as the screens show it: its translation key, and the networks to choose from when
/// the person must pick one.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ProblemView {
    pub key: &'static str,
    pub networks: Vec<String>,
}

impl From<Problem> for ProblemView {
    fn from(p: Problem) -> Self {
        let networks = match &p {
            Problem::ChooseNetwork(names) => names.clone(),
            _ => Vec::new(),
        };
        Self {
            key: p.key(),
            networks,
        }
    }
}

/// An old laptop nearby, as the new laptop's screen lists it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct NearbyView {
    pub index: usize,
    pub label: LabelView,
    #[serde(rename = "sameLabelNearby")]
    pub same_label_nearby: bool,
}

fn config() -> LinkConfig {
    LinkConfig::default()
}

/// Old laptop: starts pairing with a new random label, on `network` (the one the person chose)
/// or on the likely Wi-Fi. When it cannot tell which network, the problem lists them.
#[tauri::command]
pub async fn pairing_old_start<R: Runtime>(
    app: AppHandle<R>,
    state: State<'_, Pairing>,
    network: Option<String>,
) -> Result<(), ProblemView> {
    let label = Label::random().map_err(|_| Problem::Failed)?;
    let (journey, mut updates) = match network {
        Some(name) => {
            let interfaces = pctwin_doctor::probe::interfaces().ok_or(Problem::NoNetwork)?;
            let chosen = pctwin_session::pairing_network(&interfaces, Some(&name))
                .ok_or(Problem::NoNetwork)?;
            OldJourney::start_on(label, config(), &chosen).await?
        }
        None => OldJourney::start(label, config()).await?,
    };
    *state.old.lock().await = Some(journey);
    tauri::async_runtime::spawn(async move {
        while let Some(update) = updates.recv().await {
            if app.emit(OLD_EVENT, OldView::from(update)).is_err() {
                return;
            }
        }
    });
    Ok(())
}

/// Old laptop: the person picked `number`. The right one pairs.
#[tauri::command]
pub async fn pairing_old_pick(state: State<'_, Pairing>, number: u8) -> Result<(), ProblemView> {
    let link = {
        let old = state.old.lock().await;
        let journey = old.as_ref().ok_or(Problem::Failed)?;
        journey.pick(number).await?
    };
    *state.link.lock().await = Some(link);
    Ok(())
}

/// Old laptop: none of the three numbers matches the new laptop's.
#[tauri::command]
pub async fn pairing_old_decline(state: State<'_, Pairing>) -> Result<(), ProblemView> {
    if let Some(journey) = state.old.lock().await.as_ref() {
        journey.decline().await;
    }
    Ok(())
}

/// Old laptop: a new random colour and animal.
#[tauri::command]
pub async fn pairing_old_shuffle(state: State<'_, Pairing>) -> Result<(), ProblemView> {
    let old = state.old.lock().await;
    old.as_ref().ok_or(Problem::Failed)?.shuffle()?;
    Ok(())
}

/// Old laptop: the name the person typed, for this session.
#[tauri::command]
pub async fn pairing_old_rename(
    state: State<'_, Pairing>,
    name: String,
) -> Result<(), ProblemView> {
    let old = state.old.lock().await;
    old.as_ref().ok_or(Problem::Failed)?.rename(&name)?;
    Ok(())
}

/// Old laptop: Start again after too many tries.
#[tauri::command]
pub async fn pairing_old_start_again(state: State<'_, Pairing>) -> Result<(), ProblemView> {
    let old = state.old.lock().await;
    old.as_ref().ok_or(Problem::Failed)?.start_again()?;
    Ok(())
}

/// Either laptop: leaves pairing (the old laptop stops listening and announcing).
#[tauri::command]
pub async fn pairing_stop(state: State<'_, Pairing>) -> Result<(), ProblemView> {
    *state.old.lock().await = None;
    state.found.lock().await.clear();
    Ok(())
}

/// New laptop: the old laptops nearby.
#[tauri::command]
pub async fn pairing_new_search(state: State<'_, Pairing>) -> Result<Vec<NearbyView>, ProblemView> {
    let found = NewJourney::new(config()).search(SEARCH_WAIT).await?;
    let views = nearby_views(&found);
    *state.found.lock().await = found;
    Ok(views)
}

fn nearby_views(found: &[Found]) -> Vec<NearbyView> {
    found
        .iter()
        .enumerate()
        .map(|(index, f)| NearbyView {
            index,
            label: (&f.label).into(),
            same_label_nearby: f.same_label_nearby,
        })
        .collect()
}

/// New laptop: connects to old laptop `index` (from the last search) with the code the person
/// typed, and returns the number to show. The approval (or the attempt's end) follows as an
/// event.
#[tauri::command]
pub async fn pairing_new_connect<R: Runtime>(
    app: AppHandle<R>,
    state: State<'_, Pairing>,
    index: usize,
    code: String,
) -> Result<u8, ProblemView> {
    let old = state
        .found
        .lock()
        .await
        .get(index)
        .cloned()
        .ok_or(Problem::Failed)?;
    let waiting: Waiting = NewJourney::new(config()).connect(&old, &code).await?;
    let number = waiting.number();
    let link_slot = app.clone();
    tauri::async_runtime::spawn(async move {
        let view = match waiting.approval().await {
            Ok(link) => {
                if let Some(pairing) = tauri::Manager::try_state::<Pairing>(&link_slot) {
                    *pairing.link.lock().await = Some(link);
                }
                NewView::Paired
            }
            Err(p) => NewView::Problem { problem: p.into() },
        };
        let _ = app.emit(NEW_EVENT, view);
    });
    Ok(number)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_picked_label_reaches_the_screens_as_word_keys() {
        let label = Label::Picked {
            colour: 4,
            animal: 0,
        };
        let json = serde_json::to_value(LabelView::from(&label)).unwrap();
        assert_eq!(
            json,
            serde_json::json!({ "kind": "picked", "colour": "blue", "animal": "ant" })
        );
        let named = Label::named("Kitchen laptop").unwrap();
        let json = serde_json::to_value(LabelView::from(&named)).unwrap();
        assert_eq!(
            json,
            serde_json::json!({ "kind": "named", "name": "Kitchen laptop" })
        );
    }

    #[test]
    fn updates_reach_the_screens_in_plain_shapes() {
        let code = OldView::from(OldUpdate::Code {
            code: "123456".into(),
            seconds_left: 42,
        });
        assert_eq!(
            serde_json::to_value(code).unwrap(),
            serde_json::json!({ "kind": "code", "code": "123456", "secondsLeft": 42 })
        );
        let choose = OldView::from(OldUpdate::Choose([11, 42, 87]));
        assert_eq!(
            serde_json::to_value(choose).unwrap(),
            serde_json::json!({ "kind": "choose", "numbers": [11, 42, 87] })
        );
        assert_eq!(
            serde_json::to_value(OldView::from(OldUpdate::TimedOut)).unwrap(),
            serde_json::json!({ "kind": "timed-out" })
        );
    }

    #[test]
    fn problems_reach_the_screens_as_keys_with_networks_to_choose_from() {
        let p = ProblemView::from(Problem::ChooseNetwork(vec![
            "Ethernet".into(),
            "Wi-Fi".into(),
        ]));
        assert_eq!(
            serde_json::to_value(p).unwrap(),
            serde_json::json!({ "key": "pairing-choose-network", "networks": ["Ethernet", "Wi-Fi"] })
        );
        let p = ProblemView::from(Problem::WrongCode);
        assert_eq!(p.key, "pairing-code-wrong");
        assert!(p.networks.is_empty());
    }
}
