//! Pairing for the screens (C03 to C05): the old and new laptop's journey (`pctwin-journey`) as
//! commands the screens call and events they listen to. Everything the screens receive is plain
//! data: a label as word keys to translate, a code and its countdown, numbers, and problems as
//! translation keys, never engine text.

use std::future::Future;
use std::sync::Arc;
use std::time::Duration;

use pctwin_discovery::{Found, Label};
use pctwin_journey::{NewJourney, OldJourney, OldUpdate, Problem, Serving, Waiting};
use pctwin_link::{Link, LinkConfig};
use pctwin_session::OldLaptop;
use serde::Serialize;
use tauri::async_runtime::JoinHandle;
use tauri::{AppHandle, Emitter, Runtime, State};
use tokio::sync::{Mutex, mpsc};

/// The old laptop's screen updates.
pub const OLD_EVENT: &str = "pairing-old";
/// The new laptop's screen updates.
pub const NEW_EVENT: &str = "pairing-new";

/// How long the new laptop looks for old laptops nearby.
const SEARCH_WAIT: Duration = Duration::from_secs(3);

/// Pairing in progress on this laptop, and its result. The commands below are thin wrappers
/// over its methods; `S` is how the old laptop serves pairing (the real [`OldLaptop`] in the app).
pub struct Pairing<S: Serving = OldLaptop> {
    old: Mutex<Option<OldJourney<S>>>,
    found: Mutex<Vec<Found>>,
    /// The new laptop's wait for the old laptop's approval, ended if the person leaves pairing.
    approval: Mutex<Option<JoinHandle<()>>>,
    /// The paired connection, kept for the move. Only the first is kept.
    link: Mutex<Option<Link>>,
}

/// The app's pairing state, as the commands find it.
pub type AppPairing = Arc<Pairing>;

/// A fresh pairing state for the app.
pub fn new_state() -> AppPairing {
    Arc::new(Pairing::default())
}

impl<S: Serving> Default for Pairing<S> {
    fn default() -> Self {
        Self {
            old: Mutex::new(None),
            found: Mutex::new(Vec::new()),
            approval: Mutex::new(None),
            link: Mutex::new(None),
        }
    }
}

impl<S: Serving> Pairing<S> {
    /// Keeps `link` as the paired connection, unless one is already kept (it is then dropped,
    /// which closes it): a laptop pairs once.
    async fn keep(&self, link: Link) -> bool {
        let mut slot = self.link.lock().await;
        if slot.is_some() {
            return false;
        }
        *slot = Some(link);
        true
    }

    /// Starts the old laptop's side with `start`, never replacing pairing already under way or
    /// done. Returns the screen updates to pass on.
    async fn begin_old(
        &self,
        start: impl Future<
            Output = Result<(OldJourney<S>, mpsc::UnboundedReceiver<OldUpdate>), Problem>,
        >,
    ) -> Result<mpsc::UnboundedReceiver<OldUpdate>, Problem> {
        let mut old = self.old.lock().await;
        if old.is_some() || self.link.lock().await.is_some() {
            return Err(Problem::Failed);
        }
        let (journey, updates) = start.await?;
        *old = Some(journey);
        Ok(updates)
    }

    /// The old laptop's pick for `round`; the right one pairs, and its link is kept.
    async fn pick(&self, round: u64, number: u8) -> Result<(), Problem> {
        let link = {
            let old = self.old.lock().await;
            let journey = old.as_ref().ok_or(Problem::Failed)?;
            journey.pick(round, number).await?
        };
        if self.keep(link).await {
            Ok(())
        } else {
            Err(Problem::Failed)
        }
    }

    /// Leaves pairing: the old laptop stops listening and announcing; the new laptop stops
    /// waiting for an approval.
    async fn stop(&self) {
        *self.old.lock().await = None;
        if let Some(waiting) = self.approval.lock().await.take() {
            waiting.abort();
        }
        self.found.lock().await.clear();
    }

    /// The new laptop connects with `connect` and returns the number to show; the approval (or
    /// the attempt's end) goes to `tell` later. Only one attempt at a time, and none once paired.
    async fn begin_new(
        self: &Arc<Self>,
        connect: impl Future<Output = Result<Waiting, Problem>>,
        tell: impl FnOnce(NewView) + Send + 'static,
    ) -> Result<u8, Problem> {
        let mut approval = self.approval.lock().await;
        if approval.as_ref().is_some_and(|a| !a.inner().is_finished())
            || self.link.lock().await.is_some()
        {
            return Err(Problem::Failed);
        }
        let waiting = connect.await?;
        let number = waiting.number();
        let pairing = Arc::clone(self);
        *approval = Some(tauri::async_runtime::spawn(async move {
            let view = match waiting.approval().await {
                Ok(link) => {
                    if pairing.keep(link).await {
                        NewView::Paired
                    } else {
                        NewView::Problem {
                            problem: Problem::Failed.into(),
                        }
                    }
                }
                Err(p) => NewView::Problem { problem: p.into() },
            };
            tell(view);
        }));
        Ok(number)
    }
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
        round: u64,
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
            OldUpdate::Choose { round, numbers } => Self::Choose { round, numbers },
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
/// or on the likely Wi-Fi. When it cannot tell which network, the problem lists them. Pairing
/// already under way is never replaced: leave it first (`pairing_stop`).
#[tauri::command]
pub async fn pairing_old_start<R: Runtime>(
    app: AppHandle<R>,
    state: State<'_, AppPairing>,
    network: Option<String>,
) -> Result<(), ProblemView> {
    let mut updates = state
        .begin_old(async {
            let label = Label::random().map_err(|_| Problem::Failed)?;
            match network {
                Some(name) => {
                    let interfaces =
                        pctwin_doctor::probe::interfaces().ok_or(Problem::NoNetwork)?;
                    let chosen = pctwin_session::pairing_network(&interfaces, Some(&name))
                        .ok_or(Problem::NoNetwork)?;
                    OldJourney::start_on(label, config(), &chosen).await
                }
                None => OldJourney::start(label, config()).await,
            }
        })
        .await?;
    tauri::async_runtime::spawn(async move {
        while let Some(update) = updates.recv().await {
            if app.emit(OLD_EVENT, OldView::from(update)).is_err() {
                return;
            }
        }
    });
    Ok(())
}

/// Old laptop: the person picked `number` among the numbers shown for `round`. The right one
/// pairs; a pick for numbers no longer shown does nothing (and says the time ran out).
#[tauri::command]
pub async fn pairing_old_pick(
    state: State<'_, AppPairing>,
    round: u64,
    number: u8,
) -> Result<(), ProblemView> {
    Ok(state.pick(round, number).await?)
}

/// Old laptop: none of the three numbers shown for `round` matches the new laptop's.
#[tauri::command]
pub async fn pairing_old_decline(
    state: State<'_, AppPairing>,
    round: u64,
) -> Result<(), ProblemView> {
    if let Some(journey) = state.old.lock().await.as_ref() {
        journey.decline(round).await;
    }
    Ok(())
}

/// Old laptop: a new random colour and animal.
#[tauri::command]
pub async fn pairing_old_shuffle(state: State<'_, AppPairing>) -> Result<(), ProblemView> {
    let old = state.old.lock().await;
    old.as_ref().ok_or(Problem::Failed)?.shuffle()?;
    Ok(())
}

/// Old laptop: the name the person typed, for this session.
#[tauri::command]
pub async fn pairing_old_rename(
    state: State<'_, AppPairing>,
    name: String,
) -> Result<(), ProblemView> {
    let old = state.old.lock().await;
    old.as_ref().ok_or(Problem::Failed)?.rename(&name)?;
    Ok(())
}

/// Old laptop: Start again after too many tries.
#[tauri::command]
pub async fn pairing_old_start_again(state: State<'_, AppPairing>) -> Result<(), ProblemView> {
    let old = state.old.lock().await;
    old.as_ref().ok_or(Problem::Failed)?.start_again()?;
    Ok(())
}

/// Either laptop: leaves pairing (the old laptop stops listening and announcing; the new laptop
/// stops waiting for an approval).
#[tauri::command]
pub async fn pairing_stop(state: State<'_, AppPairing>) -> Result<(), ProblemView> {
    state.stop().await;
    Ok(())
}

/// New laptop: the old laptops nearby.
#[tauri::command]
pub async fn pairing_new_search(
    state: State<'_, AppPairing>,
) -> Result<Vec<NearbyView>, ProblemView> {
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
/// event. Only one attempt at a time, and none once paired.
#[tauri::command]
pub async fn pairing_new_connect<R: Runtime>(
    app: AppHandle<R>,
    state: State<'_, AppPairing>,
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
    let number = state
        .begin_new(
            async { NewJourney::new(config()).connect(&old, &code).await },
            move |view| {
                let _ = app.emit(NEW_EVENT, view);
            },
        )
        .await?;
    Ok(number)
}

#[cfg(test)]
mod tests {
    use super::*;

    use pctwin_link::{Host, HostPending, LinkError};
    use pctwin_pairing::RotatingSender;

    /// An old laptop serving on this laptop's loopback, with no announcement.
    struct Loopback(Host);

    impl Serving for Loopback {
        async fn next_peer(
            &self,
            sender: &std::sync::Mutex<RotatingSender>,
        ) -> Result<HostPending, LinkError> {
            self.0.next_peer(sender).await
        }
        fn label(&self) -> Label {
            Label::named("Blue Fox").unwrap()
        }
        fn relabel(&self, _: Label) -> Result<(), Problem> {
            Ok(())
        }
    }

    fn quick() -> LinkConfig {
        LinkConfig {
            connect_timeout: Duration::from_millis(500),
            ..LinkConfig::default()
        }
    }

    /// Starts the old laptop's side of `pairing` on loopback: its updates, and how to reach it.
    async fn start_old(
        pairing: &Pairing<Loopback>,
    ) -> Result<(mpsc::UnboundedReceiver<OldUpdate>, Found), Problem> {
        let host = Host::bind("127.0.0.1:0".parse().unwrap(), quick())
            .await
            .unwrap();
        let found = Found {
            label: Label::named("Blue Fox").unwrap(),
            addrs: vec![host.local_addr().unwrap()],
            same_label_nearby: false,
        };
        let updates = pairing
            .begin_old(async { OldJourney::begin(Loopback(host)) })
            .await?;
        Ok((updates, found))
    }

    /// The next update `want` picks out.
    async fn next<T>(
        updates: &mut mpsc::UnboundedReceiver<OldUpdate>,
        want: impl Fn(&OldUpdate) -> Option<T>,
    ) -> T {
        tokio::time::timeout(Duration::from_secs(5), async {
            loop {
                if let Some(t) = want(&updates.recv().await.expect("updates keep coming")) {
                    return t;
                }
            }
        })
        .await
        .expect("the update came")
    }

    fn code_of(u: &OldUpdate) -> Option<String> {
        match u {
            OldUpdate::Code { code, .. } => Some(code.clone()),
            _ => None,
        }
    }

    fn round_of(u: &OldUpdate) -> Option<u64> {
        match u {
            OldUpdate::Choose { round, .. } => Some(*round),
            _ => None,
        }
    }

    /// Pairs a new laptop's `pairing` with the old laptop's, through the bridge's own methods.
    /// Returns what the new laptop's screen was told.
    async fn pair(
        old: &Pairing<Loopback>,
        new: &Arc<Pairing<Loopback>>,
    ) -> (Result<(), Problem>, NewView) {
        let (mut updates, found) = start_old(old).await.unwrap();
        let code = next(&mut updates, code_of).await;
        let (told, heard) = tokio::sync::oneshot::channel();
        let number = new
            .begin_new(
                async { NewJourney::new(quick()).connect(&found, &code).await },
                move |view| {
                    let _ = told.send(view);
                },
            )
            .await
            .unwrap();
        let round = next(&mut updates, round_of).await;
        let picked = old.pick(round, number).await;
        (picked, heard.await.unwrap())
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn both_laptops_keep_the_link_once_paired() {
        let (old, new) = (Pairing::default(), Arc::new(Pairing::default()));
        let (picked, view) = pair(&old, &new).await;
        assert_eq!(picked, Ok(()));
        assert_eq!(view, NewView::Paired);
        assert!(old.link.lock().await.is_some());
        assert!(new.link.lock().await.is_some());
    }

    /// A laptop pairs once: a second link is never kept over the first, and once paired no
    /// further attempt starts on either side.
    #[tokio::test(flavor = "multi_thread")]
    async fn once_paired_no_second_link_or_attempt() {
        let (old, new) = (Pairing::default(), Arc::new(Pairing::default()));
        let _ = pair(&old, &new).await;
        // A second pairing between two other laptops gives a second link.
        let (other_old, other_new) = (Pairing::default(), Arc::new(Pairing::default()));
        let _ = pair(&other_old, &other_new).await;
        let second = other_new.link.lock().await.take().unwrap();
        assert!(!new.keep(second).await, "a second link replaced the first");

        assert_eq!(
            start_old(&old).await.err(),
            Some(Problem::Failed),
            "the old laptop started again once paired"
        );
        // Not even after leaving the pairing screen: the kept link is the one for the move.
        old.stop().await;
        assert_eq!(
            start_old(&old).await.err(),
            Some(Problem::Failed),
            "the old laptop started again after stop, once paired"
        );
        let attempt = new
            .begin_new(async { Err(Problem::Unreachable) }, |_| {})
            .await;
        assert_eq!(
            attempt.err(),
            Some(Problem::Failed),
            "the new laptop tried again once paired"
        );
    }

    /// Pairing already under way is never replaced by a second start.
    #[tokio::test(flavor = "multi_thread")]
    async fn a_second_start_is_refused_while_pairing() {
        let old = Pairing::default();
        let (mut first, _) = start_old(&old).await.unwrap();
        assert_eq!(start_old(&old).await.err(), Some(Problem::Failed));
        // The first is still the one running.
        next(&mut first, code_of).await;
    }

    /// Leaving pairing ends the new laptop's wait for an approval: nothing is told afterwards.
    #[tokio::test(flavor = "multi_thread")]
    async fn stop_ends_the_wait_for_an_approval() {
        let old = Pairing::default();
        let new = Arc::new(Pairing::<Loopback>::default());
        let (mut updates, found) = start_old(&old).await.unwrap();
        let code = next(&mut updates, code_of).await;
        let (told, mut heard) = mpsc::unbounded_channel();
        new.begin_new(
            async { NewJourney::new(quick()).connect(&found, &code).await },
            move |view| {
                let _ = told.send(view);
            },
        )
        .await
        .unwrap();
        new.stop().await;
        // The wait was ended, so its way to tell the screen is gone without a word.
        let after = tokio::time::timeout(Duration::from_secs(3), heard.recv()).await;
        assert!(matches!(after, Ok(None)), "the wait went on after stop");
    }

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
        let choose = OldView::from(OldUpdate::Choose {
            round: 3,
            numbers: [11, 42, 87],
        });
        assert_eq!(
            serde_json::to_value(choose).unwrap(),
            serde_json::json!({ "kind": "choose", "round": 3, "numbers": [11, 42, 87] })
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
