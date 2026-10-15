//! The pairing journey as the screens see it (Screen Blueprint C03 to C05): what each laptop's
//! screen shows, and what the person can do there. The app's window code only passes these
//! along; nothing here knows about windows.
//!
//! - The old laptop ([`OldJourney`]) shows its label and a code with a countdown, a new code each
//!   minute and after any attempt ([`OldUpdate`]). When a new laptop has the code, it shows three
//!   numbers; the person picks the one the new laptop shows ([`OldJourney::pick`]), or says none
//!   matches ([`OldJourney::decline`]). Shuffle and Change name change the label it announces.
//! - The new laptop ([`NewJourney`]) lists old laptops nearby, connects to the chosen one with the
//!   code the person typed (in any digits), shows its number, and waits for the old laptop's
//!   approval ([`Waiting`]).
//! - Every way it can go wrong is a [`Problem`] with a translation key for the screens, never
//!   engine text.

use std::future::Future;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, PoisonError};
use std::time::{Duration, Instant};

use pctwin_discovery::{Found, Label};
use pctwin_link::{GuestPending, HostPending, Link, LinkConfig, LinkError};
use pctwin_pairing::{PairingCode, PairingError, RotatingSender, SenderStatus};
use pctwin_session::{Network, OldLaptop, SessionError};
use tokio::sync::{Notify, mpsc};

/// How often the old laptop's screen is brought up to date (the countdown moves each second).
const TICK: Duration = Duration::from_millis(200);

/// Why a step of the journey did not go ahead, as the screens tell the person.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum Problem {
    #[error("the code must be six digits")]
    InvalidCode,
    #[error("the code was not right")]
    WrongCode,
    #[error("the code had changed")]
    CodeExpired,
    #[error("the old laptop is pairing with another laptop")]
    Busy,
    #[error("wait a moment for a new code")]
    Wait,
    #[error("pairing stopped after too many tries")]
    Locked,
    #[error("the old laptop could not be reached")]
    Unreachable,
    #[error("that number was not the one on the new laptop")]
    WrongNumber,
    #[error("the other laptop did not answer in time")]
    TimedOut,
    #[error("the connection to the other laptop was lost")]
    ConnectionLost,
    #[error("the other laptop has a different PCTwin")]
    DifferentVersion,
    #[error("that name cannot be used")]
    InvalidName,
    #[error("this laptop is not on a network it can pair on")]
    NoNetwork,
    #[error("choose the network the other laptop is on")]
    ChooseNetwork(Vec<String>),
    #[error("pairing did not work")]
    Failed,
}

impl Problem {
    /// The translation key the screens show for this problem.
    pub fn key(&self) -> &'static str {
        match self {
            Self::InvalidCode => "pairing.code_invalid",
            Self::WrongCode => "pairing.code_wrong",
            Self::CodeExpired => "pairing.code_expired",
            Self::Busy => "pairing.busy",
            Self::Wait => "pairing.wait",
            Self::Locked => "pairing.locked",
            Self::Unreachable => "pairing.unreachable",
            Self::WrongNumber => "pairing.wrong_number",
            Self::TimedOut => "pairing.timed_out",
            Self::ConnectionLost => "pairing.connection_lost",
            Self::DifferentVersion => "pairing.different_version",
            Self::InvalidName => "pairing.name_invalid",
            Self::NoNetwork => "pairing.no_network",
            Self::ChooseNetwork(_) => "pairing.choose_network",
            Self::Failed => "pairing.failed",
        }
    }

    /// One of each problem (for checking every key has a translation).
    pub fn all() -> Vec<Problem> {
        vec![
            Self::InvalidCode,
            Self::WrongCode,
            Self::CodeExpired,
            Self::Busy,
            Self::Wait,
            Self::Locked,
            Self::Unreachable,
            Self::WrongNumber,
            Self::TimedOut,
            Self::ConnectionLost,
            Self::DifferentVersion,
            Self::InvalidName,
            Self::NoNetwork,
            Self::ChooseNetwork(Vec::new()),
            Self::Failed,
        ]
    }
}

impl From<PairingError> for Problem {
    fn from(e: PairingError) -> Self {
        match e {
            PairingError::InvalidCode => Self::InvalidCode,
            PairingError::Expired | PairingError::UnknownSession => Self::CodeExpired,
            PairingError::CoolingDown => Self::Wait,
            PairingError::Locked => Self::Locked,
            PairingError::UnsupportedVersion(_) => Self::DifferentVersion,
            PairingError::HandshakeFailed => Self::WrongCode,
            PairingError::WrongNumber => Self::WrongNumber,
            _ => Self::Failed,
        }
    }
}

impl From<LinkError> for Problem {
    fn from(e: LinkError) -> Self {
        match e {
            LinkError::Busy => Self::Busy,
            LinkError::Unreachable => Self::Unreachable,
            LinkError::Timeout => Self::TimedOut,
            LinkError::Closed => Self::ConnectionLost,
            LinkError::Pairing(p) => p.into(),
            _ => Self::Failed,
        }
    }
}

impl From<SessionError> for Problem {
    fn from(e: SessionError) -> Self {
        match e {
            SessionError::NoNetwork => Self::NoNetwork,
            SessionError::ChooseNetwork(names) => Self::ChooseNetwork(names),
            _ => Self::Failed,
        }
    }
}

/// What the old laptop's screen should show now.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OldUpdate {
    /// The label nearby laptops see (at the start, and after Shuffle or Change name).
    Label(Label),
    /// The code to show, and the seconds until a new one replaces it.
    Code { code: String, seconds_left: u64 },
    /// No code for a moment after a failed attempt.
    Wait { seconds_left: u64 },
    /// Pairing stopped after too many failed attempts until the person chooses Start again.
    Locked,
    /// A new laptop has the code: the three numbers to offer, in display order.
    Choose([u8; 3]),
    /// The right number was picked: paired.
    Paired,
    /// The person did not pick in time; a fresh code follows.
    TimedOut,
}

/// What the old laptop serves pairing through: the real [`OldLaptop`] (listening and announced
/// on the chosen network), or anything else that hands over new laptops in the same way.
pub trait Serving: Send + Sync + 'static {
    /// The next new laptop that reached the number pick.
    fn next_peer(
        &self,
        sender: &Mutex<RotatingSender>,
    ) -> impl Future<Output = Result<HostPending, LinkError>> + Send;
    /// The label being announced.
    fn label(&self) -> Label;
    /// Announces `label` instead.
    fn relabel(&self, label: Label) -> Result<(), Problem>;
}

impl Serving for OldLaptop {
    fn next_peer(
        &self,
        sender: &Mutex<RotatingSender>,
    ) -> impl Future<Output = Result<HostPending, LinkError>> + Send {
        OldLaptop::next_peer(self, sender)
    }
    fn label(&self) -> Label {
        OldLaptop::label(self)
    }
    fn relabel(&self, label: Label) -> Result<(), Problem> {
        OldLaptop::relabel(self, label).map_err(Problem::from)
    }
}

/// The old laptop's side of the journey. Dropping it ends pairing and stops listening.
pub struct OldJourney<S: Serving> {
    serving: Arc<S>,
    sender: Arc<Mutex<RotatingSender>>,
    /// The new laptop waiting for the person's pick, if any.
    pending: Arc<tokio::sync::Mutex<Option<HostPending>>>,
    /// Told when the pick (or a decline) is made, so serving carries on.
    decided: Arc<Notify>,
    /// Set once paired: no more codes are shown or served.
    finished: Arc<AtomicBool>,
    updates: mpsc::UnboundedSender<OldUpdate>,
    tasks: Vec<tokio::task::JoinHandle<()>>,
}

impl OldJourney<OldLaptop> {
    /// Starts pairing on `network` (the one the person chose, or [`pctwin_session::likely_wifi`])
    /// with `label`.
    pub async fn start_on(
        label: Label,
        config: LinkConfig,
        network: &Network,
    ) -> Result<(Self, mpsc::UnboundedReceiver<OldUpdate>), Problem> {
        let old = OldLaptop::start_on(label, config, network).await?;
        Self::begin(old)
    }

    /// Starts pairing on the likely Wi-Fi. When it cannot tell which connection that is, fails
    /// with [`Problem::ChooseNetwork`] listing them, so the screen asks the person.
    pub async fn start(
        label: Label,
        config: LinkConfig,
    ) -> Result<(Self, mpsc::UnboundedReceiver<OldUpdate>), Problem> {
        let old = OldLaptop::start(label, config).await?;
        Self::begin(old)
    }
}

impl<S: Serving> OldJourney<S> {
    /// Starts showing codes and serving new laptops through `serving`.
    pub fn begin(serving: S) -> Result<(Self, mpsc::UnboundedReceiver<OldUpdate>), Problem> {
        let sender = RotatingSender::new(Instant::now()).map_err(|_| Problem::Failed)?;
        let (updates, receiver) = mpsc::unbounded_channel();
        let mut journey = Self {
            serving: Arc::new(serving),
            sender: Arc::new(Mutex::new(sender)),
            pending: Arc::new(tokio::sync::Mutex::new(None)),
            decided: Arc::new(Notify::new()),
            finished: Arc::new(AtomicBool::new(false)),
            updates,
            tasks: Vec::new(),
        };
        let _ = journey
            .updates
            .send(OldUpdate::Label(journey.serving.label()));
        journey.tasks.push(tokio::spawn(show_codes(
            journey.sender.clone(),
            journey.finished.clone(),
            journey.updates.clone(),
        )));
        journey.tasks.push(tokio::spawn(serve(
            journey.serving.clone(),
            journey.sender.clone(),
            journey.pending.clone(),
            journey.decided.clone(),
            journey.finished.clone(),
            journey.updates.clone(),
        )));
        Ok((journey, receiver))
    }

    /// The person picked `number` among the three. The right one pairs (the new laptop is told);
    /// any other ends this attempt, and a fresh code follows.
    pub async fn pick(&self, number: u8) -> Result<Link, Problem> {
        let pending = self.pending.lock().await.take().ok_or(Problem::TimedOut)?;
        let picked = pending.choose(number, Instant::now()).await;
        if picked.is_ok() {
            self.finished.store(true, Ordering::SeqCst);
            let _ = self.updates.send(OldUpdate::Paired);
        }
        self.decided.notify_one();
        picked.map_err(Problem::from)
    }

    /// None of the three numbers matches the new laptop's (or it shows none): this attempt ends
    /// without pairing, and a fresh code follows.
    pub async fn decline(&self) {
        drop(self.pending.lock().await.take());
        self.decided.notify_one();
    }

    /// Shows a new random colour and animal (Shuffle).
    pub fn shuffle(&self) -> Result<(), Problem> {
        let label = Label::shuffle(&[self.serving.label()]).map_err(|_| Problem::Failed)?;
        self.show_label(label)
    }

    /// Shows the name the person typed, for this session only (Change name).
    pub fn rename(&self, name: &str) -> Result<(), Problem> {
        let label = Label::named(name).map_err(|_| Problem::InvalidName)?;
        self.show_label(label)
    }

    /// After too many failed attempts, the person chose Start again.
    pub fn start_again(&self) -> Result<(), Problem> {
        lock(&self.sender)
            .unlock(Instant::now())
            .map_err(Problem::from)
    }

    fn show_label(&self, label: Label) -> Result<(), Problem> {
        self.serving.relabel(label.clone())?;
        let _ = self.updates.send(OldUpdate::Label(label));
        Ok(())
    }
}

impl<S: Serving> Drop for OldJourney<S> {
    fn drop(&mut self) {
        for task in &self.tasks {
            task.abort();
        }
    }
}

fn lock(sender: &Mutex<RotatingSender>) -> std::sync::MutexGuard<'_, RotatingSender> {
    sender.lock().unwrap_or_else(PoisonError::into_inner)
}

/// Whole seconds from `now` until `then`, rounded up (a countdown never shows 0 too early).
fn seconds_until(then: Instant, now: Instant) -> u64 {
    let left = then.saturating_duration_since(now);
    left.as_secs() + u64::from(left.subsec_nanos() > 0)
}

/// Keeps the old laptop's screen up to date: the code and its countdown, a pause, or the lock.
async fn show_codes(
    sender: Arc<Mutex<RotatingSender>>,
    finished: Arc<AtomicBool>,
    updates: mpsc::UnboundedSender<OldUpdate>,
) {
    let mut last = None;
    while !finished.load(Ordering::SeqCst) {
        let now = Instant::now();
        let view = {
            let mut s = lock(&sender);
            let _ = s.tick(now);
            match s.status() {
                SenderStatus::Showing { code_expires_at } => OldUpdate::Code {
                    code: s.code().unwrap_or_default(),
                    seconds_left: seconds_until(code_expires_at, now),
                },
                SenderStatus::CoolingDown { until } => OldUpdate::Wait {
                    seconds_left: seconds_until(until, now),
                },
                SenderStatus::Locked => OldUpdate::Locked,
            }
        };
        if last.as_ref() != Some(&view) {
            if updates.send(view.clone()).is_err() {
                return;
            }
            last = Some(view);
        }
        tokio::time::sleep(TICK).await;
    }
}

/// Serves new laptops one at a time: each that reaches the number pick waits for the person's
/// pick (or decline, or the pick's deadline), then the next is served, until paired.
async fn serve<S: Serving>(
    serving: Arc<S>,
    sender: Arc<Mutex<RotatingSender>>,
    pending: Arc<tokio::sync::Mutex<Option<HostPending>>>,
    decided: Arc<Notify>,
    finished: Arc<AtomicBool>,
    updates: mpsc::UnboundedSender<OldUpdate>,
) {
    while !finished.load(Ordering::SeqCst) {
        match serving.next_peer(&sender).await {
            Ok(peer) => {
                let choices = peer.choices();
                let deadline = tokio::time::Instant::from_std(peer.deadline());
                *pending.lock().await = Some(peer);
                if updates.send(OldUpdate::Choose(choices)).is_err() {
                    return;
                }
                tokio::select! {
                    () = decided.notified() => {}
                    () = tokio::time::sleep_until(deadline) => {
                        if pending.lock().await.take().is_some() {
                            let _ = updates.send(OldUpdate::TimedOut);
                        }
                    }
                }
            }
            // Only a failed secure random source ends serving; wait and try again.
            Err(_) => tokio::time::sleep(TICK).await,
        }
    }
}

/// The new laptop's side of the journey.
pub struct NewJourney {
    config: LinkConfig,
}

impl NewJourney {
    pub fn new(config: LinkConfig) -> Self {
        Self { config }
    }

    /// Old laptops nearby, found within `wait` (each flagged if another shows the same label).
    pub async fn search(&self, wait: Duration) -> Result<Vec<Found>, Problem> {
        pctwin_session::search(wait).await.map_err(Problem::from)
    }

    /// Connects to `old` with the code the person typed (any digits, spaces and dashes). Nothing
    /// is sent unless it is six digits.
    pub async fn connect(&self, old: &Found, typed: &str) -> Result<Waiting, Problem> {
        let code = PairingCode::parse(typed).map_err(|_| Problem::InvalidCode)?;
        let pending = pctwin_session::connect_to(old, &code, self.config).await?;
        Ok(Waiting { pending })
    }
}

/// The new laptop, showing its number while the person picks on the old laptop.
pub struct Waiting {
    pending: GuestPending,
}

impl Waiting {
    /// The number to show.
    pub fn number(&self) -> u8 {
        self.pending.match_number()
    }

    /// Waits for the old laptop's approval: paired, or the attempt ended.
    pub async fn approval(self) -> Result<Link, Problem> {
        self.pending.approval().await.map_err(Problem::from)
    }
}
