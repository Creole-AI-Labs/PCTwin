//! The download engine against a small local server (plain HTTP on this laptop, allowed only by
//! the test rules; the real rules are proven in `policy`).

use super::*;
use std::str::FromStr;
use std::sync::atomic::{AtomicUsize, Ordering};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;

fn body(n: usize, seed: u8) -> Vec<u8> {
    (0..n)
        .map(|i| (i as u8).wrapping_mul(31).wrapping_add(seed))
        .collect()
}

struct Reply {
    status: u16,
    headers: Vec<(String, String)>,
    body: Vec<u8>,
    /// Send only this many body bytes, then close.
    cut: Option<usize>,
}

fn ok(body: Vec<u8>, etag: &str) -> Reply {
    Reply {
        status: 200,
        headers: vec![
            ("Content-Type".into(), "application/octet-stream".into()),
            ("ETag".into(), etag.into()),
            ("Content-Length".into(), body.len().to_string()),
        ],
        body,
        cut: None,
    }
}

fn redirect(to: &str) -> Reply {
    Reply {
        status: 302,
        headers: vec![
            ("Location".into(), to.into()),
            ("Content-Length".into(), "0".into()),
        ],
        body: vec![],
        cut: None,
    }
}

/// What the server answers for a path and its request headers (lower-case names).
fn answer(path: &str, headers: &[(String, String)], port: u16) -> Reply {
    let header = |n: &str| {
        headers
            .iter()
            .find(|(k, _)| k == n)
            .map(|(_, v)| v.as_str())
    };
    let file = body(3000, 7);
    match path {
        "/file" | "/cut" => {
            if let (Some(range), Some("\"v1\"")) = (header("range"), header("if-range")) {
                let start: usize = range
                    .trim_start_matches("bytes=")
                    .trim_end_matches('-')
                    .parse()
                    .unwrap();
                let rest = file[start..].to_vec();
                return Reply {
                    status: 206,
                    headers: vec![
                        ("Content-Type".into(), "application/octet-stream".into()),
                        ("ETag".into(), "\"v1\"".into()),
                        ("Content-Length".into(), rest.len().to_string()),
                        (
                            "Content-Range".into(),
                            format!("bytes {start}-{}/{}", file.len() - 1, file.len()),
                        ),
                    ],
                    body: rest,
                    cut: None,
                };
            }
            let mut r = ok(file, "\"v1\"");
            if path == "/cut" {
                r.cut = Some(1000);
            }
            r
        }
        // The same address now serves other bytes, whatever validator is sent.
        "/changed" => ok(body(3000, 99), "\"v2\""),
        "/html" => Reply {
            status: 200,
            headers: vec![
                ("Content-Type".into(), "text/html; charset=utf-8".into()),
                ("Content-Length".into(), "15".into()),
            ],
            body: b"Please sign in.".to_vec(),
            cut: None,
        },
        "/html-as-file" => ok(
            b"  <!DOCTYPE html><html><body>Please sign in</body></html>".to_vec(),
            "\"h\"",
        ),
        // A server that answers any range, with a weak validator only.
        "/cut-weak" => {
            if let Some(range) = header("range") {
                let start: usize = range
                    .trim_start_matches("bytes=")
                    .trim_end_matches('-')
                    .parse()
                    .unwrap();
                return Reply {
                    status: 206,
                    headers: vec![
                        ("Content-Type".into(), "application/octet-stream".into()),
                        ("Content-Length".into(), (file.len() - start).to_string()),
                        ("Content-Range".into(), format!("bytes {start}-2999/3000")),
                    ],
                    body: file[start..].to_vec(),
                    cut: None,
                };
            }
            let mut r = ok(file, "W/\"v1\"");
            r.cut = Some(1000);
            r
        }
        // A continuation that starts in the wrong place.
        "/wrong-range" => {
            if header("range").is_some() {
                return Reply {
                    status: 206,
                    headers: vec![
                        ("Content-Type".into(), "application/octet-stream".into()),
                        ("Content-Length".into(), "3000".into()),
                        ("Content-Range".into(), "bytes 0-2999/3000".into()),
                    ],
                    body: file,
                    cut: None,
                };
            }
            ok(file, "\"v1\"")
        }
        // Says it is huge, sends little.
        "/huge-claim" => Reply {
            status: 200,
            headers: vec![
                ("Content-Type".into(), "application/octet-stream".into()),
                ("Content-Length".into(), "1000000000000".into()),
            ],
            body: vec![1; 10],
            cut: None,
        },
        // No length said: only the bytes that come can be counted.
        "/no-length" => Reply {
            status: 200,
            headers: vec![("Content-Type".into(), "application/octet-stream".into())],
            body: body(5000, 2),
            cut: None,
        },
        "/redirect-same" => redirect("/file"),
        "/redirect-loop" => redirect("/redirect-loop"),
        "/redirect-other" => redirect(&format!("http://other.test:{port}/file")),
        "/big" => ok(body(5000, 1), "\"b\""),
        "/huge" => ok(body(9 * 1024 * 1024, 3), "\"m\""),
        _ => Reply {
            status: 404,
            headers: vec![("Content-Length".into(), "0".into())],
            body: vec![],
            cut: None,
        },
    }
}

async fn serve() -> (u16, Arc<AtomicUsize>) {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    let hits = Arc::new(AtomicUsize::new(0));
    let counted = Arc::clone(&hits);
    tokio::spawn(async move {
        loop {
            let Ok((mut s, _)) = listener.accept().await else {
                return;
            };
            counted.fetch_add(1, Ordering::SeqCst);
            tokio::spawn(async move {
                let mut buf = Vec::new();
                let mut one = [0u8; 1024];
                while !buf.windows(4).any(|w| w == b"\r\n\r\n") {
                    let Ok(n) = s.read(&mut one).await else {
                        return;
                    };
                    if n == 0 {
                        return;
                    }
                    buf.extend_from_slice(&one[..n]);
                }
                let text = String::from_utf8_lossy(&buf).to_string();
                let mut lines = text.lines();
                let path = lines
                    .next()
                    .and_then(|l| l.split_whitespace().nth(1))
                    .unwrap_or("/")
                    .to_string();
                let headers: Vec<(String, String)> = lines
                    .filter_map(|l| l.split_once(':'))
                    .map(|(k, v)| (k.trim().to_ascii_lowercase(), v.trim().to_string()))
                    .collect();
                let r = answer(&path, &headers, port);
                let mut head = format!("HTTP/1.1 {} X\r\nConnection: close\r\n", r.status);
                for (k, v) in &r.headers {
                    head.push_str(&format!("{k}: {v}\r\n"));
                }
                head.push_str("\r\n");
                let _ = s.write_all(head.as_bytes()).await;
                let n = r.cut.unwrap_or(r.body.len());
                let _ = s.write_all(&r.body[..n]).await;
                let _ = s.shutdown().await;
            });
        }
    });
    (port, hits)
}

/// The test rules: plain addresses on this laptop (and a second test host name for it) only.
fn local(s: &str) -> Result<Url, Refused> {
    let u = Url::parse(s).map_err(|_| Refused::Unreadable)?;
    match u.host_str() {
        Some("127.0.0.1" | "other.test") if u.scheme() == "http" => Ok(u),
        _ => Err(Refused::NotHttps),
    }
}

fn rules() -> Rules {
    let _ = rustls::crypto::ring::default_provider().install_default();
    Rules {
        check: local,
        client: reqwest::Client::builder()
            .redirect(reqwest::redirect::Policy::none())
            .no_proxy()
            .resolve("other.test", "127.0.0.1:0".parse().unwrap())
            .build()
            .unwrap(),
    }
}

fn request(port: u16, path: &str, dir: &Path) -> Request {
    Request {
        url: format!("http://127.0.0.1:{port}{path}"),
        also_hosts: vec![],
        max_bytes: 1_000_000_000,
        expected: None,
        file: dir.join("setup.bin"),
        allow_metered: false,
    }
}

fn free(_: ()) -> NetworkCost {
    NetworkCost::Unmetered
}

fn sha(b: &[u8]) -> String {
    hex(&Sha256::digest(b))
}

#[tokio::test]
async fn a_download_arrives_whole_and_checked() {
    let (port, _) = serve().await;
    let dir = tempfile::tempdir().unwrap();
    let mut req = request(port, "/redirect-same", dir.path());
    req.expected = Some(Expected::Sha256(sha(&body(3000, 7))));
    let got = download(&req, &rules(), &|| free(())).await;
    assert!(
        matches!(&got, Outcome::Done { size: 3000, resumed: false, from, .. } if from.ends_with("/file")),
        "{got:?}"
    );
    assert_eq!(std::fs::read(&req.file).unwrap(), body(3000, 7));
    assert!(!part_path(&req.file).exists());
}

#[tokio::test]
async fn bytes_other_than_the_route_names_are_refused_and_not_kept() {
    let (port, _) = serve().await;
    let dir = tempfile::tempdir().unwrap();
    let mut req = request(port, "/file", dir.path());
    req.expected = Some(Expected::Sha256("00".repeat(32)));
    assert_eq!(
        download(&req, &rules(), &|| free(())).await,
        Outcome::NotExpectedBytes
    );
    assert!(!req.file.exists());
    assert!(!part_path(&req.file).exists());
}

#[tokio::test]
async fn a_web_page_is_never_taken_for_an_installer() {
    let (port, _) = serve().await;
    let dir = tempfile::tempdir().unwrap();
    for path in ["/html", "/html-as-file"] {
        let req = request(port, path, dir.path());
        assert_eq!(
            download(&req, &rules(), &|| free(())).await,
            Outcome::WebPage,
            "{path}"
        );
        assert!(!req.file.exists());
    }
}

#[tokio::test]
async fn redirects_are_bounded_and_kept_on_the_route_s_hosts() {
    let (port, _) = serve().await;
    let dir = tempfile::tempdir().unwrap();
    let got = download(
        &request(port, "/redirect-loop", dir.path()),
        &rules(),
        &|| free(()),
    )
    .await;
    assert!(
        matches!(&got, Outcome::Refused { why } if why.contains("too many")),
        "{got:?}"
    );
    let got = download(
        &request(port, "/redirect-other", dir.path()),
        &rules(),
        &|| free(()),
    )
    .await;
    assert!(
        matches!(&got, Outcome::Refused { why } if why.contains("site")),
        "{got:?}"
    );
    // The same move, with the other host named by the route, is followed.
    let mut req = request(port, "/redirect-other", dir.path());
    req.also_hosts = vec!["other.test".into()];
    assert!(matches!(
        download(&req, &rules(), &|| free(())).await,
        Outcome::Done { .. }
    ));
}

#[tokio::test]
async fn too_large_is_stopped_before_and_while_reading() {
    let (port, _) = serve().await;
    let dir = tempfile::tempdir().unwrap();
    let mut req = request(port, "/big", dir.path());
    req.max_bytes = 1000;
    assert_eq!(
        download(&req, &rules(), &|| free(())).await,
        Outcome::TooLarge
    );
    assert!(!part_path(&req.file).exists());
}

#[tokio::test]
async fn a_cut_download_resumes_only_with_the_same_validator() {
    let (port, _) = serve().await;
    let dir = tempfile::tempdir().unwrap();
    let mut req = request(port, "/cut", dir.path());
    let got = download(&req, &rules(), &|| free(())).await;
    assert!(matches!(&got, Outcome::Network { .. }), "{got:?}");
    assert_eq!(std::fs::read(part_path(&req.file)).unwrap().len(), 1000);
    // The same address, now complete: it continues from 1000 with "v1" and checks the whole.
    req.url = format!("http://127.0.0.1:{port}/cut");
    req.expected = Some(Expected::Sha256(sha(&body(3000, 7))));
    // The server cuts only fresh downloads; a continued one comes whole.
    let got = download(&req, &rules(), &|| free(())).await;
    assert!(
        matches!(
            got,
            Outcome::Done {
                resumed: true,
                size: 3000,
                ..
            }
        ),
        "{got:?}"
    );
    assert_eq!(std::fs::read(&req.file).unwrap(), body(3000, 7));
}

#[tokio::test]
async fn a_changed_file_at_the_same_address_starts_over_never_mixed() {
    let (port, _) = serve().await;
    let dir = tempfile::tempdir().unwrap();
    let req = request(port, "/changed", dir.path());
    // A part from before, said to be from this address with another validator.
    std::fs::write(part_path(&req.file), body(1000, 7)).unwrap();
    std::fs::write(
        meta_path(&part_path(&req.file)),
        serde_json::to_vec(&PartMeta {
            url: req.url.clone(),
            etag: Some("\"v1\"".into()),
            last_modified: None,
            total: Some(3000),
        })
        .unwrap(),
    )
    .unwrap();
    let got = download(&req, &rules(), &|| free(())).await;
    assert!(
        matches!(
            got,
            Outcome::Done {
                resumed: false,
                size: 3000,
                ..
            }
        ),
        "{got:?}"
    );
    assert_eq!(std::fs::read(&req.file).unwrap(), body(3000, 99));
}

#[tokio::test]
async fn a_metered_network_waits_unless_the_person_allowed_it() {
    let (port, hits) = serve().await;
    let dir = tempfile::tempdir().unwrap();
    let req = request(port, "/file", dir.path());
    assert_eq!(
        download(&req, &rules(), &|| NetworkCost::Metered).await,
        Outcome::MeteredWait
    );
    assert_eq!(
        download(&req, &rules(), &|| NetworkCost::Unknown).await,
        Outcome::MeteredWait
    );
    assert_eq!(
        hits.load(Ordering::SeqCst),
        0,
        "nothing was asked of the server"
    );
    let mut allowed = request(port, "/file", dir.path());
    allowed.allow_metered = true;
    assert!(matches!(
        download(&allowed, &rules(), &|| NetworkCost::Metered).await,
        Outcome::Done { .. }
    ));
    // The network becomes metered part way: it stops, keeping what came.
    let calls = AtomicUsize::new(0);
    let changing = || {
        if calls.fetch_add(1, Ordering::SeqCst) == 0 {
            NetworkCost::Unmetered
        } else {
            NetworkCost::Metered
        }
    };
    let fresh = tempfile::tempdir().unwrap();
    let huge = request(port, "/huge", fresh.path());
    assert_eq!(
        download(&huge, &rules(), &changing).await,
        Outcome::MeteredWait
    );
    assert!(std::fs::metadata(part_path(&huge.file)).unwrap().len() >= COST_EVERY);
    assert!(!huge.file.exists());
}

#[tokio::test]
async fn the_real_rules_refuse_before_connecting() {
    let real = Rules::public().unwrap();
    let dir = tempfile::tempdir().unwrap();
    for (url, why) in [
        ("http://example.com/a.msi", "secure"),
        ("https://127.0.0.1/a.msi", "number"),
        ("file:///C:/Windows/notepad.exe", "secure"),
    ] {
        let mut req = request(0, "/", dir.path());
        req.url = url.into();
        let got = download(&req, &real, &|| free(())).await;
        assert!(
            matches!(&got, Outcome::Refused { why: w } if w.contains(why)),
            "{url}: {got:?}"
        );
    }
}

/// A name whose answers are not all public is refused by the resolver itself, so no connection
/// is made to it (`localtest.me` and its kind resolve to 127.0.0.1 on the public internet; this
/// uses the system's own `localhost` name through the same resolver).
#[tokio::test]
async fn a_name_leading_to_this_laptop_is_refused_by_the_resolver() {
    use reqwest::dns::Resolve;
    let got = PublicResolver
        .resolve(reqwest::dns::Name::from_str("localhost").unwrap())
        .await;
    assert!(got.is_err());
}

#[test]
fn a_full_disk_is_told_apart() {
    assert!(disk_full(&std::io::Error::from_raw_os_error(112)));
    assert!(disk_full(&std::io::Error::from(
        std::io::ErrorKind::StorageFull
    )));
    assert!(!disk_full(&std::io::Error::from(
        std::io::ErrorKind::PermissionDenied
    )));
}

#[tokio::test]
async fn sizes_are_bounded_by_what_is_said_and_by_what_comes() {
    let (port, _) = serve().await;
    let dir = tempfile::tempdir().unwrap();
    let mut said = request(port, "/huge-claim", dir.path());
    said.max_bytes = 1000;
    assert_eq!(
        download(&said, &rules(), &|| free(())).await,
        Outcome::TooLarge
    );
    let mut sent = request(port, "/no-length", dir.path());
    sent.max_bytes = 1000;
    assert_eq!(
        download(&sent, &rules(), &|| free(())).await,
        Outcome::TooLarge
    );
    assert!(!part_path(&sent.file).exists());
}

#[tokio::test]
async fn a_weak_validator_or_another_address_never_resumes() {
    let (port, _) = serve().await;
    let dir = tempfile::tempdir().unwrap();
    let req = request(port, "/cut-weak", dir.path());
    assert!(matches!(
        download(&req, &rules(), &|| free(())).await,
        Outcome::Network { .. }
    ));
    // Only a weak validator was given: the next try starts over (the server then cuts again).
    let got = download(&req, &rules(), &|| free(())).await;
    assert!(
        !matches!(got, Outcome::Done { resumed: true, .. }),
        "{got:?}"
    );
    // A part said to be from another address is not continued.
    let other = tempfile::tempdir().unwrap();
    let req = request(port, "/file", other.path());
    std::fs::write(part_path(&req.file), body(1000, 7)).unwrap();
    std::fs::write(
        meta_path(&part_path(&req.file)),
        serde_json::to_vec(&PartMeta {
            url: format!("http://127.0.0.1:{port}/elsewhere"),
            etag: Some("\"v1\"".into()),
            last_modified: None,
            total: Some(3000),
        })
        .unwrap(),
    )
    .unwrap();
    let got = download(&req, &rules(), &|| free(())).await;
    assert!(
        matches!(
            got,
            Outcome::Done {
                resumed: false,
                size: 3000,
                ..
            }
        ),
        "{got:?}"
    );
}

#[tokio::test]
async fn a_continuation_from_the_wrong_place_is_refused() {
    let (port, _) = serve().await;
    let dir = tempfile::tempdir().unwrap();
    let req = request(port, "/wrong-range", dir.path());
    std::fs::write(part_path(&req.file), body(1000, 7)).unwrap();
    std::fs::write(
        meta_path(&part_path(&req.file)),
        serde_json::to_vec(&PartMeta {
            url: req.url.clone(),
            etag: Some("\"v1\"".into()),
            last_modified: None,
            total: Some(3000),
        })
        .unwrap(),
    )
    .unwrap();
    assert!(matches!(
        download(&req, &rules(), &|| free(())).await,
        Outcome::Bad { .. }
    ));
}

#[tokio::test]
async fn a_fresh_download_never_keeps_old_bytes_after_it() {
    let (port, _) = serve().await;
    let dir = tempfile::tempdir().unwrap();
    let mut req = request(port, "/file", dir.path());
    // A longer part from before, with nothing saying where it came from.
    std::fs::write(part_path(&req.file), vec![9u8; 5000]).unwrap();
    req.expected = Some(Expected::Sha256(sha(&body(3000, 7))));
    let got = download(&req, &rules(), &|| free(())).await;
    assert!(matches!(got, Outcome::Done { size: 3000, .. }), "{got:?}");
}
