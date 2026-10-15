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

/// Where a screen's updates go.
pub type Tell<V> = Arc<dyn Fn(V) + Send + Sync>;

/// What this laptop is doing in pairing. Exactly one at a time, so it is never both laptops,
/// and a link is kept only while paired.
enum Side<S: Serving> {
    Idle,
    /// Starting as the old laptop, or connecting as the new one: nothing else may start.
    Busy,
    Old(Arc<OldJourney<S>>, Tell<OldView>),
    /// Waiting, as the new laptop, for the old laptop's approval.
    New(JoinHandle<()>),
    /// Paired: the link for the move, and the watch that lets it go if it dies before then.
    Paired {
        #[expect(dead_code, reason = "kept for the move, which takes it when it starts")]
        link: Box<Link>,
        watch: JoinHandle<()>,
    },
}

struct Inner<S: Serving> {
    side: Side<S>,
    /// Counts each time pairing is left, so work started before then never lands after.
    epoch: u64,
}

/// Pairing in progress on this laptop, and its result. The commands below are thin wrappers
/// over its methods; `S` is how the old laptop serves pairing (the real [`OldLaptop`] in the app).
pub struct Pairing<S: Serving = OldLaptop> {
    inner: Mutex<Inner<S>>,
    found: Mutex<Vec<Found>>,
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
            inner: Mutex::new(Inner {
                side: Side::Idle,
                epoch: 0,
            }),
            found: Mutex::new(Vec::new()),
        }
    }
}

impl<S: Serving> Pairing<S> {
    /// Marks this laptop busy, only if it is doing nothing in pairing: returns the epoch the
    /// work belongs to.
    async fn reserve(&self) -> Result<u64, Problem> {
        let mut inner = self.inner.lock().await;
        if !matches!(inner.side, Side::Idle) {
            return Err(Problem::Failed);
        }
        inner.side = Side::Busy;
        Ok(inner.epoch)
    }

    /// Ends work of `epoch` that did not get going: back to doing nothing, unless pairing was
    /// left meanwhile.
    async fn release(&self, epoch: u64) {
        let mut inner = self.inner.lock().await;
        if inner.epoch == epoch && matches!(inner.side, Side::Busy) {
            inner.side = Side::Idle;
        }
    }

    /// Starts the old laptop's side with `start`, only when this laptop is doing nothing in
    /// pairing. Its updates go to `tell`.
    async fn begin_old(
        &self,
        start: impl Future<
            Output = Result<(OldJourney<S>, mpsc::UnboundedReceiver<OldUpdate>), Problem>,
        >,
        tell: Tell<OldView>,
    ) -> Result<(), Problem> {
        let epoch = self.reserve().await?;
        let (journey, mut updates) = match start.await {
            Ok(started) => started,
            Err(p) => {
                self.release(epoch).await;
                return Err(p);
            }
        };
        let mut inner = self.inner.lock().await;
        if inner.epoch != epoch || !matches!(inner.side, Side::Busy) {
            // Pairing was left while it started.
            return Err(Problem::Failed);
        }
        inner.side = Side::Old(Arc::new(journey), tell.clone());
        tauri::async_runtime::spawn(async move {
            while let Some(update) = updates.recv().await {
                tell(update.into());
            }
        });
        Ok(())
    }

    /// The old laptop's journey, while it is serving.
    async fn old(&self) -> Result<Arc<OldJourney<S>>, Problem> {
        match &self.inner.lock().await.side {
            Side::Old(journey, _) => Ok(Arc::clone(journey)),
            _ => Err(Problem::Failed),
        }
    }

    /// The old laptop's pick for `round`; the right one pairs, and its link is kept. Leaving
    /// pairing is never held up by a pick, and a pick that ends after it is let go.
    async fn pick(self: &Arc<Self>, round: u64, number: u8) -> Result<(), Problem> {
        let (journey, tell, epoch) = {
            let inner = self.inner.lock().await;
            match &inner.side {
                Side::Old(journey, tell) => (Arc::clone(journey), tell.clone(), inner.epoch),
                _ => return Err(Problem::Failed),
            }
        };
        let link = journey.pick(round, number).await?;
        self.keep(epoch, link, move || tell(OldView::Lost)).await
    }

    /// Keeps `link` as paired for work of `epoch`, watched until the move takes it: if it dies
    /// first, it is let go and `lost` is told. If pairing was left meanwhile, the link is let go.
    async fn keep(
        self: &Arc<Self>,
        epoch: u64,
        link: Link,
        lost: impl FnOnce() + Send + 'static,
    ) -> Result<(), Problem> {
        let mut inner = self.inner.lock().await;
        let ours = inner.epoch == epoch && matches!(inner.side, Side::Old(..) | Side::New(_));
        if !ours {
            return Err(Problem::Failed);
        }
        let Ok(watching) = link.watch() else {
            inner.side = Side::Idle;
            return Err(Problem::ConnectionLost);
        };
        let pairing = Arc::clone(self);
        let watch = tauri::async_runtime::spawn(async move {
            watching.closed().await;
            let mut inner = pairing.inner.lock().await;
            if inner.epoch == epoch && matches!(inner.side, Side::Paired { .. }) {
                inner.side = Side::Idle;
                inner.epoch += 1;
                drop(inner);
                lost();
            }
        });
        inner.side = Side::Paired {
            link: Box::new(link),
            watch,
        };
        Ok(())
    }

    /// Leaves pairing: the old laptop stops listening and announcing, the new laptop stops
    /// waiting for an approval, and a paired link not yet taken by the move is let go.
    async fn stop(&self) {
        let mut inner = self.inner.lock().await;
        inner.epoch += 1;
        match std::mem::replace(&mut inner.side, Side::Idle) {
            Side::New(waiting) => waiting.abort(),
            Side::Paired { watch, .. } => watch.abort(),
            Side::Idle | Side::Busy | Side::Old(..) => {}
        }
        drop(inner);
        self.found.lock().await.clear();
    }

    /// Whether this laptop is paired (its link kept for the move).
    #[cfg(test)]
    async fn is_paired(&self) -> bool {
        matches!(self.inner.lock().await.side, Side::Paired { .. })
    }

    /// The new laptop connects with `connect` and returns the number to show; the approval (or
    /// the attempt's end, or later the link dying before the move) goes to `tell`. Only when
    /// this laptop is doing nothing in pairing.
    async fn begin_new(
        self: &Arc<Self>,
        connect: impl Future<Output = Result<Waiting, Problem>>,
        tell: Tell<NewView>,
    ) -> Result<u8, Problem> {
        let epoch = self.reserve().await?;
        let waiting = match connect.await {
            Ok(waiting) => waiting,
            Err(p) => {
                self.release(epoch).await;
                return Err(p);
            }
        };
        let mut inner = self.inner.lock().await;
        if inner.epoch != epoch || !matches!(inner.side, Side::Busy) {
            return Err(Problem::Failed);
        }
        let number = waiting.number();
        let pairing = Arc::clone(self);
        let approval = tauri::async_runtime::spawn(async move {
            let view = match waiting.approval().await {
                Ok(link) => {
                    let lost = tell.clone();
                    match pairing.keep(epoch, link, move || lost(NewView::Lost)).await {
                        Ok(()) => NewView::Paired,
                        Err(p) => NewView::Problem { problem: p.into() },
                    }
                }
                Err(p) => {
                    let mut inner = pairing.inner.lock().await;
                    if inner.epoch == epoch && matches!(inner.side, Side::New(_)) {
                        inner.side = Side::Idle;
                    }
                    NewView::Problem { problem: p.into() }
                }
            };
            tell(view);
        });
        // Set before the wait can end: the wait takes this lock to finish.
        inner.side = Side::New(approval);
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
    Left,
    /// The paired connection died before the move began: pairing is over, start again.
    Lost,
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
            OldUpdate::Left => Self::Left,
        }
    }
}

/// What the new laptop's screen hears after its number is shown.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum NewView {
    Paired,
    Problem {
        problem: ProblemView,
    },
    /// The paired connection died before the move began: pairing is over, start again.
    Lost,
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
/// or on the likely Wi-Fi. When it cannot tell which network, the problem lists them. Only when
/// this laptop is doing nothing in pairing: leave first (`pairing_stop`).
#[tauri::command]
pub async fn pairing_old_start<R: Runtime>(
    app: AppHandle<R>,
    state: State<'_, AppPairing>,
    network: Option<String>,
) -> Result<(), ProblemView> {
    let tell: Tell<OldView> = Arc::new(move |view| {
        let _ = app.emit(OLD_EVENT, view);
    });
    state
        .begin_old(
            async {
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
            },
            tell,
        )
        .await?;
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
    if let Ok(journey) = state.old().await {
        journey.decline(round).await;
    }
    Ok(())
}

/// Old laptop: a new random colour and animal.
#[tauri::command]
pub async fn pairing_old_shuffle(state: State<'_, AppPairing>) -> Result<(), ProblemView> {
    state.old().await?.shuffle()?;
    Ok(())
}

/// Old laptop: the name the person typed, for this session.
#[tauri::command]
pub async fn pairing_old_rename(
    state: State<'_, AppPairing>,
    name: String,
) -> Result<(), ProblemView> {
    state.old().await?.rename(&name)?;
    Ok(())
}

/// Old laptop: Start again after too many tries.
#[tauri::command]
pub async fn pairing_old_start_again(state: State<'_, AppPairing>) -> Result<(), ProblemView> {
    state.old().await?.start_again()?;
    Ok(())
}

/// Either laptop: leaves pairing (the old laptop stops listening and announcing; the new laptop
/// stops waiting for an approval; a paired link not yet used is let go).
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
/// event. Only when this laptop is doing nothing in pairing.
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
    let tell: Tell<NewView> = Arc::new(move |view| {
        let _ = app.emit(NEW_EVENT, view);
    });
    let number = state
        .begin_new(
            async { NewJourney::new(config()).connect(&old, &code).await },
            tell,
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

    type Laptop = Arc<Pairing<Loopback>>;

    fn laptop() -> Laptop {
        Arc::new(Pairing::default())
    }

    fn quick() -> LinkConfig {
        LinkConfig {
            connect_timeout: Duration::from_millis(500),
            ..LinkConfig::default()
        }
    }

    /// What a screen was told, in order.
    fn screen<V: Send + 'static>() -> (Tell<V>, mpsc::UnboundedReceiver<V>) {
        let (tx, rx) = mpsc::unbounded_channel();
        (
            Arc::new(move |v| {
                let _ = tx.send(v);
            }),
            rx,
        )
    }

    /// Starts the old laptop's side on loopback: what its screen is told, and how to reach it.
    async fn start_old(
        pairing: &Laptop,
    ) -> Result<(mpsc::UnboundedReceiver<OldView>, Found), Problem> {
        let host = Host::bind("127.0.0.1:0".parse().unwrap(), quick())
            .await
            .unwrap();
        let found = Found {
            label: Label::named("Blue Fox").unwrap(),
            addrs: vec![host.local_addr().unwrap()],
            same_label_nearby: false,
        };
        let (tell, views) = screen();
        pairing
            .begin_old(async { OldJourney::begin(Loopback(host)) }, tell)
            .await?;
        Ok((views, found))
    }

    /// The next view `want` picks out, within five seconds.
    async fn next<V, T>(
        views: &mut mpsc::UnboundedReceiver<V>,
        want: impl Fn(&V) -> Option<T>,
    ) -> T {
        tokio::time::timeout(Duration::from_secs(5), async {
            loop {
                if let Some(t) = want(&views.recv().await.expect("views keep coming")) {
                    return t;
                }
            }
        })
        .await
        .expect("the view came")
    }

    fn code_of(v: &OldView) -> Option<String> {
        match v {
            OldView::Code { code, .. } => Some(code.clone()),
            _ => None,
        }
    }

    fn round_of(v: &OldView) -> Option<u64> {
        match v {
            OldView::Choose { round, .. } => Some(*round),
            _ => None,
        }
    }

    fn is<V: PartialEq>(want: V) -> impl Fn(&V) -> Option<()> {
        move |v| (*v == want).then_some(())
    }

    /// The new laptop connects to the old one: its number, and what its screen is told.
    async fn connect(
        new: &Laptop,
        old_views: &mut mpsc::UnboundedReceiver<OldView>,
        found: &Found,
    ) -> (u8, u64, mpsc::UnboundedReceiver<NewView>) {
        let code = next(old_views, code_of).await;
        let (tell, views) = screen();
        let number = new
            .begin_new(
                async { NewJourney::new(quick()).connect(found, &code).await },
                tell,
            )
            .await
            .unwrap();
        let round = next(old_views, round_of).await;
        (number, round, views)
    }

    /// Pairs `new` with `old` through the bridge's own methods.
    async fn pair(
        old: &Laptop,
        new: &Laptop,
    ) -> (
        mpsc::UnboundedReceiver<OldView>,
        mpsc::UnboundedReceiver<NewView>,
    ) {
        let (mut old_views, found) = start_old(old).await.unwrap();
        let (number, round, mut new_views) = connect(new, &mut old_views, &found).await;
        assert_eq!(old.pick(round, number).await, Ok(()));
        next(&mut new_views, is(NewView::Paired)).await;
        (old_views, new_views)
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn both_laptops_keep_the_link_once_paired() {
        let (old, new) = (laptop(), laptop());
        let _ = pair(&old, &new).await;
        assert!(old.is_paired().await);
        assert!(new.is_paired().await);
    }

    /// Once paired, neither laptop starts pairing again until it leaves pairing.
    #[tokio::test(flavor = "multi_thread")]
    async fn once_paired_nothing_starts_until_pairing_is_left() {
        let (old, new) = (laptop(), laptop());
        let _views = pair(&old, &new).await;
        assert_eq!(start_old(&old).await.err(), Some(Problem::Failed));
        let (tell, _) = screen();
        assert_eq!(
            new.begin_new(async { Err(Problem::Unreachable) }, tell)
                .await
                .err(),
            Some(Problem::Failed)
        );
        // Leaving lets the link go, and pairing can start afresh.
        old.stop().await;
        assert!(!old.is_paired().await);
        assert!(start_old(&old).await.is_ok());
    }

    /// A paired link that dies before the move is let go on both laptops, each screen is told,
    /// and both can pair again: Paired is never a trap.
    #[tokio::test(flavor = "multi_thread")]
    async fn a_paired_link_that_dies_is_let_go_and_each_screen_told() {
        // The new laptop leaves pairing: its link is let go, and the old laptop notices.
        let (old, new) = (laptop(), laptop());
        let (mut old_views, _new_views) = pair(&old, &new).await;
        new.stop().await;
        next(&mut old_views, is(OldView::Lost)).await;
        assert!(!old.is_paired().await);
        assert!(start_old(&old).await.is_ok());

        // The other way round: the old laptop leaves, and the new laptop notices.
        let (old, new) = (laptop(), laptop());
        let (_old_views, mut new_views) = pair(&old, &new).await;
        old.stop().await;
        next(&mut new_views, is(NewView::Lost)).await;
        assert!(!new.is_paired().await);
        let (tell, _) = screen();
        assert_eq!(
            new.begin_new(async { Err(Problem::Unreachable) }, tell)
                .await
                .err(),
            Some(Problem::Unreachable),
            "the new laptop stayed stuck after its link died"
        );
    }

    /// Pairing already under way is never replaced by a second start.
    #[tokio::test(flavor = "multi_thread")]
    async fn a_second_start_is_refused_while_pairing() {
        let old = laptop();
        let (mut first, _) = start_old(&old).await.unwrap();
        assert_eq!(start_old(&old).await.err(), Some(Problem::Failed));
        next(&mut first, code_of).await;
    }

    /// A new laptop that left during the pick never pairs: the old laptop keeps no link, is
    /// told, and can start again.
    #[tokio::test(flavor = "multi_thread")]
    async fn a_new_laptop_that_left_never_pairs_and_the_old_laptop_can_start_again() {
        let (old, new) = (laptop(), laptop());
        let (mut old_views, found) = start_old(&old).await.unwrap();
        let (number, round, _) = connect(&new, &mut old_views, &found).await;
        new.stop().await;
        next(&mut old_views, is(OldView::Left)).await;
        assert!(old.pick(round, number).await.is_err());
        assert!(!old.is_paired().await);
        old.stop().await;
        assert!(start_old(&old).await.is_ok());
    }

    /// One thing at a time: a second new-laptop attempt, or the other side, never starts while
    /// one runs; a failed attempt frees the laptop again.
    #[tokio::test(flavor = "multi_thread")]
    async fn one_attempt_at_a_time_and_never_both_sides() {
        let (old, new) = (laptop(), laptop());
        let (mut old_views, found) = start_old(&old).await.unwrap();
        let (_, round, mut new_views) = connect(&new, &mut old_views, &found).await;
        let (tell, _) = screen();
        assert_eq!(
            new.begin_new(async { Err(Problem::Unreachable) }, tell.clone())
                .await
                .err(),
            Some(Problem::Failed),
            "a second attempt started"
        );
        assert_eq!(start_old(&new).await.err(), Some(Problem::Failed));
        assert_eq!(
            old.begin_new(async { Err(Problem::Unreachable) }, tell.clone())
                .await
                .err(),
            Some(Problem::Failed),
            "the old laptop also became a new laptop"
        );
        // The numbers did not match: the attempt ends, and the new laptop is free again.
        old.old().await.unwrap().decline(round).await;
        next(&mut new_views, |v| {
            matches!(v, NewView::Problem { .. }).then_some(())
        })
        .await;
        assert_eq!(
            new.begin_new(async { Err(Problem::Unreachable) }, tell)
                .await
                .err(),
            Some(Problem::Unreachable),
            "a finished attempt still blocked the next"
        );
    }

    /// Starting both sides at the same moment never leaves a laptop as both.
    #[tokio::test(flavor = "multi_thread")]
    async fn starting_both_sides_at_once_gives_one() {
        for _ in 0..200 {
            let me = laptop();
            let host = Host::bind("127.0.0.1:0".parse().unwrap(), quick())
                .await
                .unwrap();
            let (old_tell, _old_views) = screen();
            let (new_tell, _new_views) = screen();
            let (as_old, as_new) = tokio::join!(
                me.begin_old(async { OldJourney::begin(Loopback(host)) }, old_tell),
                me.begin_new(async { Err(Problem::Unreachable) }, new_tell),
            );
            assert!(
                !(as_old.is_ok() && as_new.is_ok()),
                "this laptop became both laptops"
            );
            me.stop().await;
        }
    }

    /// Leaving pairing ends the new laptop's wait for an approval: nothing is told afterwards.
    #[tokio::test(flavor = "multi_thread")]
    async fn stop_ends_the_wait_for_an_approval() {
        let (old, new) = (laptop(), laptop());
        let (mut old_views, found) = start_old(&old).await.unwrap();
        let (_, _, mut new_views) = connect(&new, &mut old_views, &found).await;
        new.stop().await;
        let after = tokio::time::timeout(Duration::from_secs(3), new_views.recv()).await;
        assert!(
            matches!(after, Ok(None)),
            "the wait went on after stop: {after:?}"
        );
    }

    /// A pick that ends after the old laptop left pairing is let go, never kept, even once
    /// pairing has started again: the new laptop confirms only after the old laptop left.
    #[tokio::test(flavor = "multi_thread")]
    async fn a_pick_that_ends_after_leaving_is_let_go() {
        let old = laptop();
        let (mut old_views, found) = start_old(&old).await.unwrap();
        let code = next(&mut old_views, code_of).await;
        // The new laptop, driven by hand so its confirmation can be held back.
        let waiting = NewJourney::new(quick())
            .connect(&found, &code)
            .await
            .unwrap();
        let round = next(&mut old_views, round_of).await;
        let number = waiting.number();
        let picking = {
            let old = Arc::clone(&old);
            tokio::spawn(async move { old.pick(round, number).await })
        };
        // The pick is waiting for the confirmation; the person leaves and starts again.
        tokio::time::sleep(Duration::from_millis(300)).await;
        old.stop().await;
        let _again = start_old(&old).await.unwrap();
        let _new_link = waiting.approval().await.unwrap();
        assert_eq!(picking.await.unwrap(), Err(Problem::Failed));
        assert!(
            !old.is_paired().await,
            "a stale pick landed in the new start"
        );
    }

    /// An attempt that fails to start frees the laptop for the next one.
    #[tokio::test(flavor = "multi_thread")]
    async fn a_failed_start_frees_the_laptop() {
        let me = laptop();
        for _ in 0..2 {
            let (tell, _) = screen();
            assert_eq!(
                me.begin_new(async { Err(Problem::Unreachable) }, tell)
                    .await
                    .err(),
                Some(Problem::Unreachable)
            );
            let (tell, _) = screen();
            assert_eq!(
                me.begin_old(async { Err(Problem::NoNetwork) }, tell)
                    .await
                    .err(),
                Some(Problem::NoNetwork)
            );
        }
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
        assert_eq!(
            serde_json::to_value(OldView::from(OldUpdate::Left)).unwrap(),
            serde_json::json!({ "kind": "left" })
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

    #[test]
    fn a_lost_link_reaches_the_screens_as_lost() {
        assert_eq!(
            serde_json::to_value(OldView::Lost).unwrap(),
            serde_json::json!({ "kind": "lost" })
        );
        assert_eq!(
            serde_json::to_value(NewView::Lost).unwrap(),
            serde_json::json!({ "kind": "lost" })
        );
    }
}
