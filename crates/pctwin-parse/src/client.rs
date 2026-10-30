//! The app's side: start the parser process, ask it one typed request at a time, hand it the
//! pieces of the file it asks for, and end it at the first sign of trouble.
//!
//! Every request is bounded:
//! - in time ([`Limits::time_for`]: a base plus a share per byte of the file);
//! - in bytes: each piece at most [`MAX_READ`], all pieces together at most
//!   [`Limits::bytes_for`], every piece inside the file.
//!
//! The process is ended, and the request answered "could not be checked" ([`NotChecked`]), when
//! it stops, runs out of time, asks for more than its bound, says anything out of turn or answers
//! a different question. The next request starts a fresh process. Nothing here ever turns a
//! failure into a pass.

use std::fs::File;
use std::io;
use std::path::{Path, PathBuf};
#[cfg(not(windows))]
use std::process::{Child, Stdio};
use std::sync::Mutex;
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender};
use std::time::{Duration, Instant};

use pctwin_special::content::{ContentError, DbFacts, JsonFacts};

use crate::lockdown::Confinement;
use crate::wire::{
    Answer, Hello, MAX_FRAME, MAX_JSON, MAX_READ, PROTOCOL, ReadAt, Request, TAG_ANSWER, TAG_BYTES,
    TAG_HELLO, TAG_PIECE, TAG_READ_AT, TAG_READ_FAILED, TAG_REQUEST, json, read_frame, write_frame,
};

/// The parser program's name beside the app (and beside the administrator helper).
pub const PARSER_PROGRAM: &str = if cfg!(windows) {
    "pctwin-parser.exe"
} else {
    "pctwin-parser"
};

/// Why a file could not be checked. Each one is "needs action", never a pass.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum NotChecked {
    #[error("PCTwin's file checker could not start ({0})")]
    CouldNotStart(String),
    #[error("PCTwin's file checker stopped before it finished")]
    Stopped,
    #[error("PCTwin's file checker took too long and was stopped")]
    TimedOut,
    #[error("PCTwin's file checker was stopped for breaking its rules ({0})")]
    BrokeTheRules(String),
    #[error("the file could not be read ({0})")]
    Unreadable(String),
}

/// Bounds on each request.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Limits {
    /// Time allowed for starting the process and its hello.
    pub start: Duration,
    /// Time allowed for any request, before its share per byte.
    pub base: Duration,
    /// Bytes of the file read per second, at the least, before the time runs out.
    pub bytes_per_second: u64,
    /// How many times the file's size the pieces asked for may add up to (a program's signature
    /// check reads its header twice and every other byte once).
    pub reads_of_the_file: u64,
    /// Bytes the pieces may add up to whatever the file's size (small files read around).
    pub read_slack: u64,
}

impl Default for Limits {
    fn default() -> Self {
        Self {
            start: Duration::from_secs(20),
            base: Duration::from_secs(20),
            bytes_per_second: 32 * 1024 * 1024,
            reads_of_the_file: 3,
            read_slack: 16 * 1024 * 1024,
        }
    }
}

impl Limits {
    /// The time a request about a file of `size` bytes may take.
    pub fn time_for(&self, size: u64) -> Duration {
        let per_byte = size / self.bytes_per_second.max(1);
        self.base + Duration::from_secs(per_byte)
    }

    /// The bytes its pieces may add up to.
    pub fn bytes_for(&self, size: u64) -> u64 {
        size.saturating_mul(self.reads_of_the_file)
            .saturating_add(self.read_slack)
    }
}

/// One frame from the parser, or why there was none.
type FromChild = io::Result<Option<(u8, Vec<u8>)>>;

/// The parser process itself: ended by [`Running::end`].
enum Proc {
    /// Started as a plain child (Linux and macOS: the parser walls itself in first thing).
    #[cfg(not(windows))]
    Plain(Child),
    /// Started walled in by the system (Windows); dropping the guard ends it.
    #[cfg(windows)]
    Contained(pctwin_helper_os::contain::Guard),
}

struct Running {
    child: Proc,
    to_child: Sender<(u8, Vec<u8>)>,
    from_child: Receiver<FromChild>,
    confinement: Confinement,
}

impl Running {
    /// Whether nothing has come from it since its last answer (anything is discarded).
    fn quiet(&self) -> bool {
        self.from_child
            .try_recv()
            .is_err_and(|e| e == mpsc::TryRecvError::Empty)
    }

    fn end(self) {
        match self.child {
            #[cfg(not(windows))]
            Proc::Plain(mut child) => {
                let _ = child.kill();
                let _ = child.wait();
            }
            // The job's last handle closes: the system ends the process.
            #[cfg(windows)]
            Proc::Contained(guard) => drop(guard),
        }
    }
}

/// The parser process, started when first needed and again after it was ended. Safe to share
/// between threads: requests take turns.
pub struct Parser {
    program: Result<PathBuf, String>,
    limits: Limits,
    running: Mutex<Option<Running>>,
    started: std::sync::atomic::AtomicUsize,
}

impl std::fmt::Debug for Parser {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Parser")
            .field("program", &self.program)
            .finish_non_exhaustive()
    }
}

impl Parser {
    /// The parser program beside the program running now (the app or the administrator helper,
    /// both installed with it).
    pub fn beside_this_program() -> Self {
        let program = std::env::current_exe()
            .map_err(|e| e.to_string())
            .and_then(|exe| {
                exe.parent()
                    .map(|dir| dir.join(PARSER_PROGRAM))
                    .ok_or_else(|| "this program has no folder".to_owned())
            });
        // Linux packages put PCTwin's helper programs in /usr/lib/pctwin (bundle/linux.json),
        // beside the administrator helper, not beside the app.
        #[cfg(all(unix, not(target_os = "macos")))]
        let program = program.map(|p| {
            let packaged = Path::new("/usr/lib/pctwin").join(PARSER_PROGRAM);
            if !p.exists() && packaged.exists() {
                packaged
            } else {
                p
            }
        });
        Self {
            program,
            limits: Limits::default(),
            running: Mutex::new(None),
            started: std::sync::atomic::AtomicUsize::new(0),
        }
    }

    /// The parser program at `program`.
    pub fn at(program: PathBuf) -> Self {
        Self {
            program: Ok(program),
            limits: Limits::default(),
            running: Mutex::new(None),
            started: std::sync::atomic::AtomicUsize::new(0),
        }
    }

    /// How many parser processes this one has started so far.
    pub fn started(&self) -> usize {
        self.started.load(std::sync::atomic::Ordering::SeqCst)
    }

    pub fn with_limits(mut self, limits: Limits) -> Self {
        self.limits = limits;
        self
    }

    /// Where the parser program is looked for.
    pub fn program(&self) -> Option<&Path> {
        self.program.as_deref().ok()
    }

    /// How the running parser process says it is locked down (None: not running now).
    pub fn confinement(&self) -> Option<Confinement> {
        let running = self.running.lock().unwrap_or_else(|p| p.into_inner());
        running.as_ref().map(|r| r.confinement.clone())
    }

    /// Asks one request about `file`, held open by the caller. The answer is of the request's
    /// own kind, or the file could not be checked.
    pub fn ask(&self, request: Request, file: &File) -> Result<Answer, NotChecked> {
        self.ask_about(request, &[Source::File(file)])
    }

    /// Asks one request about the bytes of `sources`, one after another (a database, then its
    /// log), handed over in the pieces the parser asks for.
    pub fn ask_about(
        &self,
        request: Request,
        sources: &[Source<'_>],
    ) -> Result<Answer, NotChecked> {
        self.ask_with_back(request, sources)
            .map(|(answer, _)| answer)
    }

    /// [`Parser::ask_about`], with the bytes the parser hands back ([`Request::most_back`]).
    pub fn ask_with_back(
        &self,
        request: Request,
        sources: &[Source<'_>],
    ) -> Result<(Answer, Vec<u8>), NotChecked> {
        let mut slot = self.running.lock().unwrap_or_else(|p| p.into_inner());
        // A process that said anything between requests spoke out of turn: it is ended, and a
        // fresh one answers.
        let running = match slot.take() {
            Some(r) if r.quiet() => r,
            Some(r) => {
                r.end();
                self.start()?
            }
            None => self.start()?,
        };
        match self.exchange(&running, request, sources) {
            Ok(answer) => {
                *slot = Some(running);
                match answer {
                    (Answer::Failed(why), _) => Err(NotChecked::Unreadable(why)),
                    answer => Ok(answer),
                }
            }
            Err(e) => {
                running.end();
                Err(e)
            }
        }
    }

    fn start(&self) -> Result<Running, NotChecked> {
        let program = self.program.clone().map_err(NotChecked::CouldNotStart)?;
        self.started
            .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        let (child, stdin, stdout, mut set_here) = spawn(&program)?;
        let to_child = writer(stdin);
        let from_child = reader(stdout);
        let hello = match from_child.recv_timeout(self.limits.start) {
            Ok(Ok(Some((TAG_HELLO, body)))) => json::<Hello>(&body).ok(),
            _ => None,
        };
        let running = |confinement| Running {
            child,
            to_child,
            from_child,
            confinement,
        };
        match hello {
            Some(h) if h.protocol == PROTOCOL => {
                let mut confinement = h.confinement;
                for wall in set_here.drain(..) {
                    confinement.add(wall);
                }
                Ok(running(confinement))
            }
            other => {
                running(Confinement {
                    in_place: Vec::new(),
                    not_yet: Vec::new(),
                })
                .end();
                Err(NotChecked::CouldNotStart(match other {
                    Some(_) => "it speaks another version".into(),
                    None => "it did not say hello".into(),
                }))
            }
        }
    }

    fn exchange(
        &self,
        running: &Running,
        request: Request,
        sources: &[Source<'_>],
    ) -> Result<(Answer, Vec<u8>), NotChecked> {
        let size = request.size();
        let deadline = Instant::now() + self.limits.time_for(size) + request.own_time();
        let budget = self.limits.bytes_for(size);
        let mut served: u64 = 0;
        let mut back: Vec<u8> = Vec::new();
        let body =
            serde_json::to_vec(&request).map_err(|e| NotChecked::Unreadable(e.to_string()))?;
        running
            .to_child
            .send((TAG_REQUEST, body))
            .map_err(|_| NotChecked::Stopped)?;
        loop {
            let left = deadline.saturating_duration_since(Instant::now());
            let frame = match running.from_child.recv_timeout(left) {
                Ok(frame) => frame,
                Err(RecvTimeoutError::Timeout) => return Err(NotChecked::TimedOut),
                Err(RecvTimeoutError::Disconnected) => return Err(NotChecked::Stopped),
            };
            let (tag, body) = match frame {
                Ok(Some(frame)) => frame,
                Ok(None) => return Err(NotChecked::Stopped),
                Err(e) if e.kind() == io::ErrorKind::InvalidData => {
                    return Err(NotChecked::BrokeTheRules(e.to_string()));
                }
                Err(_) => return Err(NotChecked::Stopped),
            };
            match tag {
                TAG_READ_AT => {
                    let ask: ReadAt =
                        json(&body).map_err(|e| NotChecked::BrokeTheRules(e.to_string()))?;
                    let end = ask.offset.checked_add(u64::from(ask.len));
                    if ask.len > MAX_READ || end.is_none_or(|end| end > size) {
                        return Err(NotChecked::BrokeTheRules(
                            "it asked for a piece outside the file".into(),
                        ));
                    }
                    served = served.saturating_add(u64::from(ask.len));
                    if served > budget {
                        return Err(NotChecked::BrokeTheRules(
                            "it asked for more of the file than a check needs".into(),
                        ));
                    }
                    let reply = match read_sources(sources, ask.offset, ask.len as usize) {
                        Ok(bytes) => (TAG_BYTES, bytes),
                        Err(_) => (TAG_READ_FAILED, Vec::new()),
                    };
                    running
                        .to_child
                        .send(reply)
                        .map_err(|_| NotChecked::Stopped)?;
                }
                TAG_PIECE => {
                    if back.len() as u64 + body.len() as u64 > request.most_back()
                        || body.len() > MAX_READ as usize
                    {
                        return Err(NotChecked::BrokeTheRules(
                            "it handed back more than it may".into(),
                        ));
                    }
                    back.extend_from_slice(&body);
                }
                TAG_ANSWER => {
                    let answer: Answer =
                        json(&body).map_err(|e| NotChecked::BrokeTheRules(e.to_string()))?;
                    if !answer.answers(&request) {
                        return Err(NotChecked::BrokeTheRules(
                            "it answered another question".into(),
                        ));
                    }
                    return Ok((answer, back));
                }
                _ => {
                    return Err(NotChecked::BrokeTheRules(
                        "it said something out of turn".into(),
                    ));
                }
            }
        }
    }
}

impl Drop for Parser {
    fn drop(&mut self) {
        let slot = self.running.get_mut().unwrap_or_else(|p| p.into_inner());
        if let Some(running) = slot.take() {
            running.end();
        }
    }
}

type Spawned = (
    Proc,
    Box<dyn std::io::Write + Send>,
    Box<dyn std::io::Read + Send>,
    Vec<crate::lockdown::Wall>,
);

/// Windows: the parser started walled in by the system ([`pctwin_helper_os::contain`]): an
/// AppContainer with no capabilities, a restricted token, no programs of its own, in a job that
/// ends it with the app, allows one process and caps its memory. Never started without them:
/// if they cannot be put up, the file is not checked. Returns the walls set here that the
/// system accepted (the memory cap; the rest the parser reads back from its own token).
#[cfg(windows)]
fn spawn(program: &Path) -> Result<Spawned, NotChecked> {
    let contained =
        pctwin_helper_os::contain::start_contained(program, crate::lockdown::MEMORY_CAP).map_err(
            |e| NotChecked::CouldNotStart(format!("its walls could not be put up ({e})")),
        )?;
    let (stdin, stdout, guard) = contained.split();
    Ok((
        Proc::Contained(guard),
        Box::new(stdin),
        Box::new(stdout),
        vec![crate::lockdown::Wall::MemoryCap],
    ))
}

/// Linux and macOS: the parser started plainly, with an empty environment; it walls itself in
/// first thing, before it reads anything ([`crate::lockdown::confine_self`]).
#[cfg(not(windows))]
fn spawn(program: &Path) -> Result<Spawned, NotChecked> {
    #[expect(
        clippy::disallowed_methods,
        reason = "the app's side starts the parser: the one program this crate starts"
    )]
    let mut command = pctwin_types::process::quiet(program);
    command
        .env_clear()
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null());
    if let Some(dir) = program.parent() {
        command.current_dir(dir);
    }
    let mut child = command
        .spawn()
        .map_err(|e| NotChecked::CouldNotStart(e.to_string()))?;
    let (Some(stdin), Some(stdout)) = (child.stdin.take(), child.stdout.take()) else {
        let _ = child.kill();
        let _ = child.wait();
        return Err(NotChecked::CouldNotStart("no pipes".into()));
    };
    Ok((
        Proc::Plain(child),
        Box::new(stdin),
        Box::new(stdout),
        Vec::new(),
    ))
}

/// What the parser is handed: a file held open by the caller, or bytes it copied (bounded).
#[derive(Debug, Clone, Copy)]
pub enum Source<'a> {
    File(&'a File),
    Bytes(&'a [u8]),
}

impl Source<'_> {
    fn len(&self) -> io::Result<u64> {
        match self {
            Source::File(f) => Ok(f.metadata()?.len()),
            Source::Bytes(b) => Ok(b.len() as u64),
        }
    }
}

/// Reads `len` bytes at `offset` of the sources laid end to end, fewer only at their end.
fn read_sources(sources: &[Source<'_>], offset: u64, len: usize) -> io::Result<Vec<u8>> {
    let mut out = Vec::with_capacity(len);
    let mut start = 0u64;
    for s in sources {
        let n = s.len()?;
        let end = start + n;
        if offset + (out.len() as u64) < end && out.len() < len {
            let at = offset + out.len() as u64 - start;
            let want = (len - out.len()).min((n - at) as usize);
            match s {
                Source::File(f) => out.extend(read_piece(f, at, want)?),
                Source::Bytes(b) => out.extend_from_slice(&b[at as usize..at as usize + want]),
            }
        }
        start = end;
    }
    Ok(out)
}

/// Reads `len` bytes at `offset` of `file`, fewer only at its end, without the caller's place
/// in the file mattering.
fn read_piece(file: &File, offset: u64, len: usize) -> io::Result<Vec<u8>> {
    let mut buf = vec![0u8; len];
    let mut got = 0;
    while got < len {
        #[cfg(windows)]
        let n = {
            use std::os::windows::fs::FileExt;
            file.seek_read(&mut buf[got..], offset + got as u64)?
        };
        #[cfg(unix)]
        let n = {
            use std::os::unix::fs::FileExt;
            file.read_at(&mut buf[got..], offset + got as u64)?
        };
        if n == 0 {
            break;
        }
        got += n;
    }
    buf.truncate(got);
    Ok(buf)
}

/// The thread that writes to the parser, so a parser that stops reading can never hold the app
/// past its time: the app only ever waits on [`reader`]'s channel, with a deadline.
fn writer(mut stdin: Box<dyn std::io::Write + Send>) -> Sender<(u8, Vec<u8>)> {
    let (tx, rx) = mpsc::channel::<(u8, Vec<u8>)>();
    std::thread::spawn(move || {
        for (tag, body) in rx {
            if write_frame(&mut stdin, tag, &body).is_err() {
                break;
            }
        }
    });
    tx
}

/// The thread that reads the parser's frames, each at most [`MAX_FRAME`] (a JSON frame at most
/// [`MAX_JSON`]).
fn reader(mut stdout: Box<dyn std::io::Read + Send>) -> Receiver<FromChild> {
    let (tx, rx) = mpsc::channel::<FromChild>();
    std::thread::spawn(move || {
        loop {
            let frame = read_frame(&mut stdout, MAX_FRAME.max(MAX_JSON));
            let end = !matches!(frame, Ok(Some(_)));
            if tx.send(frame).is_err() || end {
                break;
            }
        }
        // Drain whatever else comes so the parser is never blocked writing; it is being ended.
        let _ = io::copy(&mut stdout, &mut io::sink());
    });
    rx
}

/// The maker check's reader, run in the parser process.
impl pctwin_maker::OwnReader for Parser {
    fn installer(&self, file: &File, size: u64) -> Result<pctwin_maker::InstallerRead, String> {
        match self.ask(Request::Installer { size }, file) {
            Ok(Answer::Installer(read)) => Ok(read),
            Ok(_) => Err("it answered another question".into()),
            Err(e) => Err(e.to_string()),
        }
    }

    fn old_copy(&self, file: &File, size: u64) -> Result<pctwin_maker::OldCopy, String> {
        match self.ask(Request::OldCopy { size }, file) {
            Ok(Answer::OldCopy(read)) => Ok(read),
            Ok(_) => Err("it answered another question".into()),
            Err(e) => Err(e.to_string()),
        }
    }
}

/// The readiness checks' reader, run in the parser process.
impl pctwin_readiness::checks::FileReader for Parser {
    fn font(&self, file: &File, size: u64) -> Result<pctwin_readiness::checks::FontNames, String> {
        match self.ask(Request::Font { size }, file) {
            Ok(Answer::Font(names)) => Ok(names),
            Ok(_) => Err("it answered another question".into()),
            Err(e) => Err(e.to_string()),
        }
    }

    fn archive(&self, file: &File, size: u64) -> Result<pctwin_readiness::checks::Shape, String> {
        match self.ask(Request::Archive { size }, file) {
            Ok(Answer::Archive(shape)) => Ok(shape),
            Ok(_) => Err("it answered another question".into()),
            Err(e) => Err(e.to_string()),
        }
    }

    fn database(&self, file: &File, size: u64) -> Result<pctwin_readiness::checks::Shape, String> {
        match self.ask(Request::Database { size }, file) {
            Ok(Answer::Database(shape)) => Ok(shape),
            Ok(_) => Err("it answered another question".into()),
            Err(e) => Err(e.to_string()),
        }
    }
}

/// Files' contents (2.5's JSON facts and databases), read in the parser process.
impl pctwin_special::content::ContentReader for Parser {
    fn json(&self, bytes: &[u8]) -> Result<JsonFacts, ContentError> {
        let request = Request::Json {
            size: bytes.len() as u64,
        };
        match self.ask_about(request, &[Source::Bytes(bytes)]) {
            Ok(Answer::Json(facts)) => facts,
            Ok(_) => Err(ContentError::Damaged("it answered another question".into())),
            Err(e) => Err(ContentError::Damaged(e.to_string())),
        }
    }

    fn sqlite(
        &self,
        db: &[u8],
        log: Option<&[u8]>,
        time: Duration,
    ) -> Result<DbFacts, ContentError> {
        let log = log.unwrap_or_default();
        let request = Request::DatabaseFacts {
            size: db.len() as u64,
            log: log.len() as u64,
            millis: u64::try_from(time.as_millis()).unwrap_or(u64::MAX),
        };
        match self.ask_about(request, &[Source::Bytes(db), Source::Bytes(log)]) {
            Ok(Answer::DatabaseFacts(facts)) => facts,
            Ok(_) => Err(ContentError::Damaged("it answered another question".into())),
            Err(e) => Err(ContentError::Damaged(e.to_string())),
        }
    }

    fn snapshot(&self, db: &[u8], log: Option<&[u8]>) -> Result<Vec<u8>, ContentError> {
        let log = log.unwrap_or_default();
        let request = Request::DatabaseSnapshot {
            size: db.len() as u64,
            log: log.len() as u64,
        };
        match self.ask_with_back(request, &[Source::Bytes(db), Source::Bytes(log)]) {
            Ok((Answer::DatabaseSnapshot(Ok(len)), back)) if len == back.len() as u64 => Ok(back),
            Ok((Answer::DatabaseSnapshot(Ok(_)), _)) => Err(ContentError::Damaged(
                "it handed back another length than it said".into(),
            )),
            Ok((Answer::DatabaseSnapshot(Err(e)), _)) => Err(e),
            Ok(_) => Err(ContentError::Damaged("it answered another question".into())),
            Err(e) => Err(ContentError::Damaged(e.to_string())),
        }
    }

    fn add_on_manifest(&self, bytes: &[u8]) -> Result<bool, ContentError> {
        let request = Request::AddOn {
            size: bytes.len() as u64,
        };
        match self.ask_about(request, &[Source::Bytes(bytes)]) {
            Ok(Answer::AddOn(yes)) => Ok(yes),
            Ok(_) => Err(ContentError::Damaged("it answered another question".into())),
            Err(e) => Err(ContentError::Damaged(e.to_string())),
        }
    }
    fn emlx(&self, bytes: &[u8]) -> Result<pctwin_special::mail::EmlxFacts, ContentError> {
        let request = Request::Emlx {
            size: bytes.len() as u64,
        };
        match self.ask_about(request, &[Source::Bytes(bytes)]) {
            Ok(Answer::Emlx(facts)) => facts,
            Ok(_) => Err(ContentError::Damaged("it answered another question".into())),
            Err(e) => Err(ContentError::Damaged(e.to_string())),
        }
    }

    fn mail_kinds(
        &self,
        db: &[u8],
        log: Option<&[u8]>,
    ) -> Result<Vec<pctwin_special::applemail::AccountScheme>, ContentError> {
        let log = log.unwrap_or_default();
        let request = Request::MailKinds {
            size: db.len() as u64,
            log: log.len() as u64,
        };
        match self.ask_about(request, &[Source::Bytes(db), Source::Bytes(log)]) {
            Ok(Answer::MailKinds(kinds)) => kinds,
            Ok(_) => Err(ContentError::Damaged("it answered another question".into())),
            Err(e) => Err(ContentError::Damaged(e.to_string())),
        }
    }

    fn browser(
        &self,
        what: pctwin_special::browser_read::BrowserRead,
        bytes: &[u8],
        second: Option<&[u8]>,
    ) -> Result<pctwin_special::browser_read::Found, ContentError> {
        let second = second.unwrap_or_default();
        let request = Request::Browser {
            size: bytes.len() as u64,
            second: second.len() as u64,
            what,
        };
        match self.ask_about(request, &[Source::Bytes(bytes), Source::Bytes(second)]) {
            Ok(Answer::Browser(found)) => found,
            Ok(_) => Err(ContentError::Damaged("it answered another question".into())),
            Err(e) => Err(ContentError::Damaged(e.to_string())),
        }
    }

    fn stored_value(
        &self,
        db: &[u8],
        want: &pctwin_special::stored::StoredKey,
    ) -> Result<Option<String>, ContentError> {
        let wanted = serde_json::to_vec(want).map_err(|e| ContentError::Damaged(e.to_string()))?;
        let request = Request::StoredValue {
            wanted: wanted.len() as u64,
            size: db.len() as u64,
        };
        match self.ask_about(request, &[Source::Bytes(&wanted), Source::Bytes(db)]) {
            Ok(Answer::StoredValue(read)) => read,
            Ok(_) => Err(ContentError::Damaged("it answered another question".into())),
            Err(e) => Err(ContentError::Damaged(e.to_string())),
        }
    }

    fn replace_value(
        &self,
        db: &[u8],
        change: &pctwin_special::stored::StoredChange,
    ) -> Result<Vec<u8>, ContentError> {
        let wanted =
            serde_json::to_vec(change).map_err(|e| ContentError::Damaged(e.to_string()))?;
        let request = Request::ReplaceValue {
            wanted: wanted.len() as u64,
            size: db.len() as u64,
        };
        match self.ask_with_back(request, &[Source::Bytes(&wanted), Source::Bytes(db)]) {
            Ok((Answer::ReplaceValue(Ok(len)), back)) if len == back.len() as u64 => Ok(back),
            Ok((Answer::ReplaceValue(Ok(_)), _)) => Err(ContentError::Damaged(
                "it handed back another length than it said".into(),
            )),
            Ok((Answer::ReplaceValue(Err(e)), _)) => Err(e),
            Ok(_) => Err(ContentError::Damaged("it answered another question".into())),
            Err(e) => Err(ContentError::Damaged(e.to_string())),
        }
    }
}

/// Several parser processes taking requests in turn, for readers that work on several threads
/// at once (the app data plan reads folders in parallel).
#[derive(Debug)]
pub struct Pool {
    parsers: Vec<Parser>,
    next: std::sync::atomic::AtomicUsize,
}

impl Pool {
    /// `n` (at least one) parser processes beside this program.
    pub fn beside_this_program(n: usize) -> Self {
        Self::of(
            (0..n.max(1))
                .map(|_| Parser::beside_this_program())
                .collect(),
        )
    }

    /// These parsers (at least one).
    pub fn of(parsers: Vec<Parser>) -> Self {
        assert!(!parsers.is_empty(), "a pool has a parser");
        Self {
            parsers,
            next: std::sync::atomic::AtomicUsize::new(0),
        }
    }

    fn one(&self) -> &Parser {
        let i = self.next.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        &self.parsers[i % self.parsers.len()]
    }
}

impl pctwin_special::content::ContentReader for Pool {
    fn json(&self, bytes: &[u8]) -> Result<JsonFacts, ContentError> {
        self.one().json(bytes)
    }

    fn sqlite(
        &self,
        db: &[u8],
        log: Option<&[u8]>,
        time: Duration,
    ) -> Result<DbFacts, ContentError> {
        self.one().sqlite(db, log, time)
    }

    fn snapshot(&self, db: &[u8], log: Option<&[u8]>) -> Result<Vec<u8>, ContentError> {
        self.one().snapshot(db, log)
    }

    fn add_on_manifest(&self, bytes: &[u8]) -> Result<bool, ContentError> {
        self.one().add_on_manifest(bytes)
    }
    fn emlx(&self, bytes: &[u8]) -> Result<pctwin_special::mail::EmlxFacts, ContentError> {
        self.one().emlx(bytes)
    }
    fn mail_kinds(
        &self,
        db: &[u8],
        log: Option<&[u8]>,
    ) -> Result<Vec<pctwin_special::applemail::AccountScheme>, ContentError> {
        self.one().mail_kinds(db, log)
    }

    fn browser(
        &self,
        what: pctwin_special::browser_read::BrowserRead,
        bytes: &[u8],
        second: Option<&[u8]>,
    ) -> Result<pctwin_special::browser_read::Found, ContentError> {
        self.one().browser(what, bytes, second)
    }

    fn stored_value(
        &self,
        db: &[u8],
        want: &pctwin_special::stored::StoredKey,
    ) -> Result<Option<String>, ContentError> {
        self.one().stored_value(db, want)
    }

    fn replace_value(
        &self,
        db: &[u8],
        change: &pctwin_special::stored::StoredChange,
    ) -> Result<Vec<u8>, ContentError> {
        self.one().replace_value(db, change)
    }
}

/// Game launchers' records and save lists (2.5 games), read in the parser process.
impl pctwin_special::games::GameReader for Parser {
    fn record(
        &self,
        format: pctwin_special::gamefmt::RecordFormat,
        bytes: &[u8],
    ) -> Result<pctwin_special::gamefmt::Tree, ContentError> {
        let request = Request::GameRecord {
            size: bytes.len() as u64,
            format,
        };
        match self.ask_about(request, &[Source::Bytes(bytes)]) {
            Ok(Answer::GameRecord(tree)) => tree,
            Ok(_) => Err(ContentError::Damaged("it answered another question".into())),
            Err(e) => Err(ContentError::Damaged(e.to_string())),
        }
    }

    fn save_manifest(
        &self,
        bytes: &[u8],
        wanted: &pctwin_special::gamefmt::Wanted,
    ) -> Result<pctwin_special::gamefmt::ManifestRead, ContentError> {
        let wanted =
            serde_json::to_vec(wanted).map_err(|e| ContentError::Damaged(e.to_string()))?;
        let request = Request::SaveManifest {
            wanted: wanted.len() as u64,
            size: bytes.len() as u64,
        };
        match self.ask_about(request, &[Source::Bytes(&wanted), Source::Bytes(bytes)]) {
            Ok(Answer::SaveManifest(read)) => read,
            Ok(_) => Err(ContentError::Damaged("it answered another question".into())),
            Err(e) => Err(ContentError::Damaged(e.to_string())),
        }
    }
}

impl pctwin_special::games::GameReader for Pool {
    fn record(
        &self,
        format: pctwin_special::gamefmt::RecordFormat,
        bytes: &[u8],
    ) -> Result<pctwin_special::gamefmt::Tree, ContentError> {
        self.one().record(format, bytes)
    }

    fn save_manifest(
        &self,
        bytes: &[u8],
        wanted: &pctwin_special::gamefmt::Wanted,
    ) -> Result<pctwin_special::gamefmt::ManifestRead, ContentError> {
        self.one().save_manifest(bytes, wanted)
    }
}
