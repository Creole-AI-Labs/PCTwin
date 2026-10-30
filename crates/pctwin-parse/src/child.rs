//! The parser process's side: say hello, then answer one request at a time, reading the file
//! only through the app ([`Remote`]).
//!
//! Only the parser process runs the readers: [`answer`], [`serve`] and [`Remote::new`] each take
//! a `pctwin_parse_seal::InChild`, which `deny.toml` lets only the parser crates make (review
//! finding F3). Without one, code does not build:
//!
//! ```compile_fail
//! use pctwin_parse::wire::{Request, TAG_BYTES, write_frame};
//! let mut from_app = std::io::Cursor::new(Vec::new());
//! let mut to_app = Vec::new();
//! let _ = pctwin_parse::child::answer(Request::Json { size: 2 }, &mut from_app, &mut to_app);
//! ```
//!
//! With one (here, in this crate's own doctest, which may hold it), the same call builds:
//!
//! ```
//! use pctwin_parse::wire::{Answer, Request, TAG_BYTES, write_frame};
//! let mut from_app = Vec::new();
//! write_frame(&mut from_app, TAG_BYTES, b"{}").unwrap();
//! let mut from_app = std::io::Cursor::new(from_app);
//! let mut to_app = Vec::new();
//! let seal = pctwin_parse_seal::in_child();
//! let a = pctwin_parse::child::answer(seal, Request::Json { size: 2 }, &mut from_app, &mut to_app);
//! assert!(matches!(a, Answer::Json(Ok(_))));
//! ```

use std::io::{self, Read, Seek, SeekFrom, Write};

use pctwin_parse_seal::InChild;

use crate::wire::{
    Answer, Hello, MAX_FRAME, MAX_JSON, MAX_READ, PROTOCOL, ReadAt, Request, TAG_ANSWER, TAG_BYTES,
    TAG_HELLO, TAG_PIECE, TAG_READ_AT, TAG_READ_FAILED, TAG_REQUEST, json, read_frame, write_frame,
    write_json,
};

/// The smallest piece asked for at once: small reads (a font's table records) are served from it.
const MIN_PIECE: u32 = 64 * 1024;

/// The file being read, as the app hands it over: `Read + Seek` over the pipes. Reads ahead,
/// growing while the reading goes straight on, so hashing a large program takes few round trips.
pub struct Remote<'a, R: Read, W: Write> {
    from_app: &'a mut R,
    to_app: &'a mut W,
    size: u64,
    pos: u64,
    piece_at: u64,
    piece: Vec<u8>,
    ahead: u32,
}

impl<'a, R: Read, W: Write> Remote<'a, R, W> {
    /// Only for the parser process (it takes an [`InChild`]).
    pub fn new(_: InChild, from_app: &'a mut R, to_app: &'a mut W, size: u64) -> Self {
        Self {
            from_app,
            to_app,
            size,
            pos: 0,
            piece_at: 0,
            piece: Vec::new(),
            ahead: MIN_PIECE,
        }
    }

    fn fetch(&mut self, want: usize) -> io::Result<()> {
        let straight_on = self.pos == self.piece_at + self.piece.len() as u64;
        self.ahead = if straight_on {
            (self.ahead.saturating_mul(2)).min(MAX_READ)
        } else {
            MIN_PIECE
        };
        let want = u32::try_from(want)
            .unwrap_or(MAX_READ)
            .clamp(self.ahead, MAX_READ);
        let len = u64::from(want).min(self.size - self.pos) as u32;
        write_json(
            self.to_app,
            TAG_READ_AT,
            &ReadAt {
                offset: self.pos,
                len,
            },
        )?;
        match read_frame(self.from_app, MAX_FRAME)? {
            Some((TAG_BYTES, bytes)) if bytes.len() <= len as usize => {
                self.piece_at = self.pos;
                self.piece = bytes;
                Ok(())
            }
            Some((TAG_READ_FAILED, _)) => Err(io::Error::other("the file could not be read")),
            _ => Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "the app answered out of turn",
            )),
        }
    }
}

impl<R: Read, W: Write> Read for Remote<'_, R, W> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        if buf.is_empty() || self.pos >= self.size {
            return Ok(0);
        }
        let end = self.piece_at + self.piece.len() as u64;
        if self.pos < self.piece_at || self.pos >= end {
            self.fetch(buf.len())?;
        }
        let from = (self.pos - self.piece_at) as usize;
        let n = buf.len().min(self.piece.len().saturating_sub(from));
        buf[..n].copy_from_slice(&self.piece[from..from + n]);
        self.pos += n as u64;
        Ok(n)
    }
}

impl<R: Read, W: Write> Seek for Remote<'_, R, W> {
    fn seek(&mut self, to: SeekFrom) -> io::Result<u64> {
        let base = match to {
            SeekFrom::Start(n) => Some(n),
            SeekFrom::End(d) => self.size.checked_add_signed(d),
            SeekFrom::Current(d) => self.pos.checked_add_signed(d),
        };
        let Some(at) = base else {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "a place before the start",
            ));
        };
        self.pos = at;
        Ok(at)
    }
}

/// Answers one request, reading through `from_app` and `to_app`. Only the parser process runs
/// readers (it takes an [`InChild`]).
pub fn answer<R: Read, W: Write>(
    seal: InChild,
    request: Request,
    from_app: &mut R,
    to_app: &mut W,
) -> Answer {
    let mut file = Remote::new(seal, from_app, to_app, request.size());
    let size = request.size();
    match request {
        Request::Installer { .. } => {
            match pctwin_maker::own::read_installer(seal, &mut file, size) {
                Ok(read) => Answer::Installer(read),
                Err(e) => Answer::Failed(e.to_string()),
            }
        }
        Request::OldCopy { .. } => {
            Answer::OldCopy(pctwin_maker::own::read_old_copy(seal, &mut file, size))
        }
        Request::Font { .. } => {
            Answer::Font(pctwin_readiness::checks::font_names(seal, &mut file, size))
        }
        Request::Archive { .. } => Answer::Archive(pctwin_readiness::checks::archive_shape(
            seal, &mut file, size,
        )),
        Request::Database { .. } => Answer::Database(pctwin_readiness::checks::database_shape(
            seal, &mut file, size,
        )),
        Request::Json { size } => Answer::Json(
            whole(&mut file, size, pctwin_special::content::MAX_JSON as u64)
                .and_then(|b| pctwin_special::content::json_facts(seal, &b)),
        ),
        Request::DatabaseSnapshot { size, log } => {
            let read = whole(&mut file, size, pctwin_special::content::MAX_DB).and_then(|db| {
                let log = whole(&mut file, log, pctwin_special::content::MAX_LOG)?;
                Ok((db, log))
            });
            drop(file);
            match read.and_then(|(db, log)| {
                let log = (!log.is_empty()).then_some(log);
                pctwin_special::content::db_snapshot(seal, &db, log.as_deref())
            }) {
                Err(e) => Answer::DatabaseSnapshot(Err(e)),
                Ok(image) => {
                    for piece in image.chunks(MAX_READ as usize) {
                        if write_frame(to_app, TAG_PIECE, piece).is_err() {
                            return Answer::Failed("the app stopped listening".into());
                        }
                    }
                    Answer::DatabaseSnapshot(Ok(image.len() as u64))
                }
            }
        }
        Request::Emlx { size } => Answer::Emlx(
            whole(&mut file, size, pctwin_special::mail::MAX_MESSAGE)
                .and_then(|b| pctwin_special::mail::emlx_facts(seal, &b)),
        ),
        Request::AddOn { size } => Answer::AddOn(
            whole(&mut file, size, 1024 * 1024)
                .is_ok_and(|b| pctwin_special::content::add_on_manifest(seal, &b)),
        ),
        Request::Browser { size, second, what } => {
            use pctwin_special::browser_read::BrowserRead;
            let most = match what {
                BrowserRead::Places => pctwin_special::content::MAX_DB,
                BrowserRead::PropertyList => pctwin_special::browser_read::MAX_PLIST as u64,
                BrowserRead::Json | BrowserRead::Session => {
                    pctwin_special::content::MAX_JSON as u64
                }
            };
            let most_second = match what {
                BrowserRead::Places => pctwin_special::content::MAX_LOG,
                _ => pctwin_special::content::MAX_JSON as u64,
            };
            Answer::Browser(whole(&mut file, size, most).and_then(|b| {
                let s = whole(&mut file, second, most_second)?;
                let s = (!s.is_empty()).then_some(s);
                pctwin_special::browser_read::read(seal, what, &b, s.as_deref())
            }))
        }
        Request::GameRecord { size, format } => Answer::GameRecord(
            whole(&mut file, size, pctwin_special::gamefmt::MAX_RECORD)
                .and_then(|b| pctwin_special::gamefmt::record_tree(seal, format, &b)),
        ),
        Request::SaveManifest { wanted, size } => Answer::SaveManifest(
            whole(&mut file, wanted, pctwin_special::gamefmt::MAX_WANTED)
                .and_then(|w| {
                    serde_json::from_slice::<pctwin_special::gamefmt::Wanted>(&w)
                        .map_err(|e| pctwin_special::content::ContentError::Damaged(e.to_string()))
                })
                .and_then(|w| {
                    // The list is read line by line as it comes, never held whole.
                    let mut lines = std::io::BufReader::new(&mut file);
                    pctwin_special::gamefmt::save_manifest(seal, &mut lines, size, &w)
                }),
        ),
        Request::DatabaseFacts { size, log, millis } => {
            let until = std::time::Instant::now() + request.own_time();
            let _ = millis;
            Answer::DatabaseFacts(
                whole(&mut file, size, pctwin_special::content::MAX_DB).and_then(|db| {
                    let log = whole(&mut file, log, pctwin_special::content::MAX_LOG)?;
                    let log = (!log.is_empty()).then_some(log);
                    pctwin_special::content::db_facts(seal, &db, log.as_deref(), until)
                }),
            )
        }
    }
}

/// The next `n` bytes of `file`, at most `max`.
fn whole(
    file: &mut impl Read,
    n: u64,
    max: u64,
) -> Result<Vec<u8>, pctwin_special::content::ContentError> {
    use pctwin_special::content::ContentError;
    if n > max {
        return Err(ContentError::TooLarge);
    }
    let mut out = vec![0u8; n as usize];
    file.read_exact(&mut out)
        .map_err(|e| ContentError::Damaged(e.to_string()))?;
    Ok(out)
}

/// The parser process: hello, then one request at a time until the app closes the pipe. Returns
/// the exit code: 0 when the app closed it, 2 when the app said something out of turn.
pub fn serve(seal: InChild, mut from_app: impl Read, mut to_app: impl Write) -> u8 {
    let hello = Hello {
        protocol: PROTOCOL,
        confinement: crate::lockdown::confine_self(),
    };
    if write_json(&mut to_app, TAG_HELLO, &hello).is_err() {
        return 2;
    }
    loop {
        let request = match read_frame(&mut from_app, MAX_JSON) {
            Ok(None) => return 0,
            Ok(Some((TAG_REQUEST, body))) => match json::<Request>(&body) {
                Ok(r) => r,
                Err(_) => return 2,
            },
            _ => return 2,
        };
        let answer = answer(seal, request, &mut from_app, &mut to_app);
        if write_json(&mut to_app, TAG_ANSWER, &answer).is_err() {
            return 2;
        }
    }
}
