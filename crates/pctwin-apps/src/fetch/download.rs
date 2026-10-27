//! One download, bounded, into PCTwin's own staging folder (the 2.1 plan, section 9). It runs in
//! the locked-down fetch process ([`super::process`]), never in the app.
//!
//! - Redirects are followed by hand, one at a time, each checked before any connection
//!   ([`super::policy::Redirects`]); names are resolved once and every answer must be public
//!   ([`PublicResolver`]); no proxy; TLS checked by the system's own trust store (rustls with the
//!   platform verifier), never turned off.
//! - The bytes go to `<file>.part`, never run or opened, and become `<file>` only when complete
//!   and, where the route names them, exactly the expected bytes. A changed digest is never
//!   "updated": it fails (T21-022, T21-029).
//! - A web page where an installer was expected (a sign-in or error page) stops the download
//!   (T21-023).
//! - Resuming sends the server's own strong validator (`If-Range`); a server that answers with the
//!   whole file again, or a different file, starts over: two halves of different files are never
//!   joined (T21-024).
//! - The network's cost is looked at before and while downloading; on a metered or unknown
//!   network the download waits unless the person allowed it (T21-025). A full disk pauses with
//!   what was downloaded kept; nothing else is deleted (T21-026).

use std::io::{Read, Seek, SeekFrom, Write};
use std::net::{IpAddr, SocketAddr, ToSocketAddrs};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use serde::{Deserialize, Serialize};
use sha2::{Digest as _, Sha256, Sha512};
use url::Url;

use super::policy::{Hosts, Redirects, Refused, addresses_ok, check_url, redacted};

/// How often (in bytes) the network's cost is looked at again while downloading.
const COST_EVERY: u64 = 4 * 1024 * 1024;
/// The bytes looked at for a web page.
const HEAD: usize = 512;

/// What the network costs, as the system says.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum NetworkCost {
    Unmetered,
    Metered,
    /// The system could not say: treated like metered (a distinct question for the person).
    Unknown,
}

/// The bytes a route names for its download.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", tag = "kind", content = "hex")]
pub enum Expected {
    Sha256(String),
    Sha512(String),
}

/// One download asked of the fetch process.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Request {
    pub url: String,
    /// Hosts it may move to beside its own (GitHub's download hosts, say).
    #[serde(default)]
    pub also_hosts: Vec<String>,
    pub max_bytes: u64,
    #[serde(default)]
    pub expected: Option<Expected>,
    /// The file to make, in the fetch process's own staging folder.
    pub file: PathBuf,
    /// The person allowed downloading on a metered or unknown network.
    #[serde(default)]
    pub allow_metered: bool,
}

/// How a download ended.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", tag = "outcome")]
pub enum Outcome {
    /// Complete: its size and SHA-256, and where it finally came from (query removed).
    Done {
        size: u64,
        sha256: String,
        from: String,
        resumed: bool,
    },
    /// The address, a redirect or what a name led to was not allowed.
    Refused { why: String },
    /// A web page came back where an installer was expected.
    WebPage,
    /// Larger than the route allows.
    TooLarge,
    /// Not the bytes the route names.
    NotExpectedBytes,
    /// Waiting: the network is metered (or its cost unknown) and the person has not allowed it.
    MeteredWait,
    /// Waiting: the disk is full. What was downloaded is kept for later.
    DiskFull,
    /// The server answered with an error.
    Http { status: u16 },
    /// The connection or its security check failed (with why; never retried without the check).
    Network { why: String },
    /// The request itself was not usable.
    Bad { why: String },
}

/// Resolves names, refusing any whose answers are not all public.
#[derive(Debug, Default, Clone, Copy)]
pub struct PublicResolver;

impl reqwest::dns::Resolve for PublicResolver {
    fn resolve(&self, name: reqwest::dns::Name) -> reqwest::dns::Resolving {
        let host = name.as_str().to_string();
        Box::pin(async move {
            let found: Vec<SocketAddr> = tokio::task::spawn_blocking(move || {
                (host.as_str(), 0u16)
                    .to_socket_addrs()
                    .map(|a| a.collect::<Vec<_>>())
            })
            .await
            .map_err(|e| -> Box<dyn std::error::Error + Send + Sync> { Box::new(e) })??;
            let ips: Vec<IpAddr> = found.iter().map(SocketAddr::ip).collect();
            addresses_ok(&ips)
                .map_err(|e| -> Box<dyn std::error::Error + Send + Sync> { Box::new(e) })?;
            let addrs: reqwest::dns::Addrs = Box::new(found.into_iter());
            Ok(addrs)
        })
    }
}

/// The rules one download runs under: how addresses are checked and names resolved.
pub struct Rules {
    pub check: fn(&str) -> Result<Url, Refused>,
    pub client: reqwest::Client,
}

impl Rules {
    /// The real rules: [`check_url`], [`PublicResolver`], no proxy, no automatic redirects, the
    /// system's trust store.
    pub fn public() -> Result<Rules, String> {
        let _ = rustls::crypto::ring::default_provider().install_default();
        let client = reqwest::Client::builder()
            .redirect(reqwest::redirect::Policy::none())
            .no_proxy()
            .https_only(true)
            .dns_resolver(Arc::new(PublicResolver))
            .connect_timeout(Duration::from_secs(30))
            .read_timeout(Duration::from_secs(60))
            .user_agent(concat!("PCTwin/", env!("CARGO_PKG_VERSION")))
            .build()
            .map_err(|e| e.to_string())?;
        Ok(Rules {
            check: check_url,
            client,
        })
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
struct PartMeta {
    url: String,
    etag: Option<String>,
    last_modified: Option<String>,
    total: Option<u64>,
}

fn meta_path(part: &Path) -> PathBuf {
    let mut p = part.as_os_str().to_owned();
    p.push(".json");
    PathBuf::from(p)
}

fn part_path(file: &Path) -> PathBuf {
    let mut p = file.as_os_str().to_owned();
    p.push(".part");
    PathBuf::from(p)
}

fn looks_like_web_page(head: &[u8]) -> bool {
    let start: Vec<u8> = head
        .iter()
        .copied()
        .skip_while(|b| b.is_ascii_whitespace() || *b == 0xef || *b == 0xbb || *b == 0xbf)
        .take(64)
        .map(|b| b.to_ascii_lowercase())
        .collect();
    start.starts_with(b"<!doctype html")
        || start.starts_with(b"<html")
        || start.starts_with(b"<head")
        || start.starts_with(b"<?xml")
        || start.starts_with(b"<script")
}

fn hex(b: &[u8]) -> String {
    b.iter().map(|x| format!("{x:02x}")).collect()
}

fn disk_full(e: &std::io::Error) -> bool {
    e.kind() == std::io::ErrorKind::StorageFull || e.raw_os_error() == Some(112)
}

/// Downloads `req` under `rules`, asking `cost` what the network costs.
pub async fn download(req: &Request, rules: &Rules, cost: &dyn Fn() -> NetworkCost) -> Outcome {
    let first = match (rules.check)(&req.url) {
        Ok(u) => u,
        Err(e) => return Outcome::Refused { why: e.to_string() },
    };
    if req.max_bytes == 0 {
        return Outcome::Bad {
            why: "no size allowed".into(),
        };
    }
    let costs_ok = |c: NetworkCost| c == NetworkCost::Unmetered || req.allow_metered;
    if !costs_ok(cost()) {
        return Outcome::MeteredWait;
    }
    let part = part_path(&req.file);
    let meta_file = meta_path(&part);
    // What is there from before, and whether it can be continued.
    let had = std::fs::read(&meta_file)
        .ok()
        .and_then(|b| serde_json::from_slice::<PartMeta>(&b).ok())
        .filter(|m| m.url == req.url && (m.etag.is_some() || m.last_modified.is_some()));
    let have = std::fs::metadata(&part).map(|m| m.len()).unwrap_or(0);
    let resume_from = match &had {
        Some(m) if have > 0 && m.total.is_some_and(|t| have < t) => have,
        _ => 0,
    };

    let mut hosts = Hosts::SameHost;
    if !req.also_hosts.is_empty() {
        hosts = Hosts::These(req.also_hosts.clone());
    }
    let mut redirects = Redirects::new(&first, hosts);
    let mut at = first;
    let response = loop {
        let mut get = rules.client.get(at.clone());
        if resume_from > 0
            && let Some(m) = &had
        {
            get = get.header(reqwest::header::RANGE, format!("bytes={resume_from}-"));
            if let Some(validator) = m.etag.as_ref().or(m.last_modified.as_ref()) {
                get = get.header(reqwest::header::IF_RANGE, validator.as_str());
            }
        }
        let r = match get.send().await {
            Ok(r) => r,
            Err(e) => {
                // A refused name comes back from the resolver as a connection error.
                let text = format!("{e:?}");
                if text.contains(&Refused::Private.to_string()) {
                    return Outcome::Refused {
                        why: Refused::Private.to_string(),
                    };
                }
                return Outcome::Network { why: e.to_string() };
            }
        };
        if r.status().is_redirection() {
            let Some(to) = r
                .headers()
                .get(reqwest::header::LOCATION)
                .and_then(|v| v.to_str().ok())
                .and_then(|l| at.join(l).ok())
            else {
                return Outcome::Refused {
                    why: "the download moved without saying where".into(),
                };
            };
            match redirects.follow_with(to.as_str(), rules.check) {
                Ok(next) => {
                    at = next;
                    continue;
                }
                Err(e) => return Outcome::Refused { why: e.to_string() },
            }
        }
        break r;
    };
    let status = response.status().as_u16();
    let resumed = status == 206 && resume_from > 0;
    if status != 200 && !resumed {
        return Outcome::Http { status };
    }
    if response
        .headers()
        .get(reqwest::header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .is_some_and(|t| t.to_ascii_lowercase().starts_with("text/html"))
    {
        return Outcome::WebPage;
    }
    let header = |h| {
        response
            .headers()
            .get(h)
            .and_then(|v| v.to_str().ok())
            .map(str::to_string)
    };
    // A weak validator ("W/...") does not promise the same bytes, so it is never used to resume.
    let etag = header(reqwest::header::ETAG).filter(|e| !e.starts_with("W/"));
    let last_modified = header(reqwest::header::LAST_MODIFIED);
    let total = if resumed {
        // "bytes 100-999/1000": it must continue exactly where the part ends.
        let range = header(reqwest::header::CONTENT_RANGE).unwrap_or_default();
        let ok = range
            .strip_prefix("bytes ")
            .and_then(|r| r.split_once('/'))
            .and_then(|(span, total)| {
                let start: u64 = span.split_once('-')?.0.parse().ok()?;
                let total: u64 = total.parse().ok()?;
                (start == resume_from && Some(total) == had.as_ref().and_then(|m| m.total))
                    .then_some(total)
            });
        match ok {
            Some(t) => Some(t),
            None => {
                return Outcome::Bad {
                    why: "the server continued from the wrong place".into(),
                };
            }
        }
    } else {
        response.content_length()
    };
    if total.is_some_and(|t| t > req.max_bytes) {
        return Outcome::TooLarge;
    }

    let mut out = match std::fs::OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(!resumed)
        .open(&part)
    {
        Ok(f) => f,
        Err(e) if disk_full(&e) => return Outcome::DiskFull,
        Err(e) => return Outcome::Bad { why: e.to_string() },
    };
    if resumed && let Err(e) = out.seek(SeekFrom::Start(resume_from)) {
        return Outcome::Bad { why: e.to_string() };
    }
    let meta = PartMeta {
        url: req.url.clone(),
        etag,
        last_modified,
        total,
    };
    if let Ok(b) = serde_json::to_vec(&meta) {
        let _ = std::fs::write(&meta_file, b);
    }
    let mut written = if resumed { resume_from } else { 0 };
    let mut checked_at = written;
    let mut head = Vec::new();
    let mut response = response;
    loop {
        let chunk = match response.chunk().await {
            Ok(Some(c)) => c,
            Ok(None) => break,
            Err(e) => return Outcome::Network { why: e.to_string() },
        };
        if !resumed && head.len() < HEAD {
            head.extend_from_slice(&chunk[..chunk.len().min(HEAD - head.len())]);
            if head.len() >= 16 && looks_like_web_page(&head) {
                drop(out);
                let _ = std::fs::remove_file(&part);
                let _ = std::fs::remove_file(&meta_file);
                return Outcome::WebPage;
            }
        }
        written += chunk.len() as u64;
        if written > req.max_bytes {
            drop(out);
            let _ = std::fs::remove_file(&part);
            let _ = std::fs::remove_file(&meta_file);
            return Outcome::TooLarge;
        }
        if let Err(e) = out.write_all(&chunk) {
            return if disk_full(&e) {
                Outcome::DiskFull
            } else {
                Outcome::Bad { why: e.to_string() }
            };
        }
        if written - checked_at >= COST_EVERY {
            checked_at = written;
            if !costs_ok(cost()) {
                let _ = out.flush();
                return Outcome::MeteredWait;
            }
        }
    }
    if let Err(e) = out.flush().and_then(|()| out.sync_all()) {
        return if disk_full(&e) {
            Outcome::DiskFull
        } else {
            Outcome::Bad { why: e.to_string() }
        };
    }
    drop(out);
    if total.is_some_and(|t| t != written) {
        return Outcome::Network {
            why: "the download ended early".into(),
        };
    }
    if !resumed && head.len() < 16 && looks_like_web_page(&head) {
        let _ = std::fs::remove_file(&part);
        return Outcome::WebPage;
    }
    // The whole file, read again from the disk: what was downloaded in two parts is checked as
    // one.
    let (size, sha256, sha512) = match digests(&part) {
        Ok(d) => d,
        Err(e) => return Outcome::Bad { why: e.to_string() },
    };
    let matches = match &req.expected {
        None => true,
        Some(Expected::Sha256(h)) => h.eq_ignore_ascii_case(&hex(&sha256)),
        Some(Expected::Sha512(h)) => h.eq_ignore_ascii_case(&hex(&sha512)),
    };
    if !matches {
        let _ = std::fs::remove_file(&part);
        let _ = std::fs::remove_file(&meta_file);
        return Outcome::NotExpectedBytes;
    }
    if let Err(e) = std::fs::rename(&part, &req.file) {
        return Outcome::Bad { why: e.to_string() };
    }
    let _ = std::fs::remove_file(&meta_file);
    Outcome::Done {
        size,
        sha256: hex(&sha256),
        from: redacted(&at),
        resumed,
    }
}

fn digests(p: &Path) -> std::io::Result<(u64, [u8; 32], [u8; 64])> {
    let mut f = std::fs::File::open(p)?;
    let mut a = Sha256::new();
    let mut b = Sha512::new();
    let mut buf = vec![0u8; 1024 * 1024];
    let mut size = 0u64;
    loop {
        let n = f.read(&mut buf)?;
        if n == 0 {
            break;
        }
        a.update(&buf[..n]);
        b.update(&buf[..n]);
        size += n as u64;
    }
    Ok((size, a.finalize().into(), b.finalize().into()))
}

#[cfg(test)]
mod tests;
