//! Reading what a maker's own update channel offers: a Sparkle appcast, an electron-builder
//! `latest.yml`, a GitHub or GitLab release. Each gives at most one offer: a version, a secure
//! download address on the right host, and the bytes' digest when the channel names one. The
//! fetched text is untrusted: every address is checked, every field bounded, and an offer is only
//! a place to download from. The maker check decides.

use serde::Deserialize;

/// The largest feed read.
pub const MAX_FEED: usize = 4 * 1024 * 1024;

/// A digest a channel names for its download.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Digest {
    Sha256([u8; 32]),
    Sha512(Box<[u8; 64]>),
}

/// What a channel offers.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Offer {
    pub version: String,
    pub url: String,
    pub digest: Option<Digest>,
}

fn secure(url: &str) -> Option<String> {
    super::rows::row_url_ok(url).then(|| url.to_string())
}

/// A Sparkle appcast: the item with the highest version whose enclosure is a secure address.
/// Items marked for another system (`sparkle:os` other than macos) are skipped.
pub fn sparkle(text: &str) -> Result<Offer, String> {
    if text.len() > MAX_FEED {
        return Err("the feed is too large".into());
    }
    let doc = roxmltree::Document::parse_with_options(
        text,
        roxmltree::ParsingOptions {
            allow_dtd: false,
            ..Default::default()
        },
    )
    .map_err(|e| e.to_string())?;
    const SPARKLE: &str = "http://www.andymatuschak.org/xml-namespaces/sparkle";
    let mut best: Option<Offer> = None;
    for item in doc.descendants().filter(|n| n.has_tag_name("item")) {
        let child_text = |name: &str| {
            item.children()
                .find(|c| c.has_tag_name((SPARKLE, name)))
                .and_then(|c| c.text())
                .map(|t| t.trim().to_string())
        };
        let Some(enc) = item.children().find(|c| c.has_tag_name("enclosure")) else {
            continue;
        };
        if enc
            .attribute((SPARKLE, "os"))
            .is_some_and(|os| os != "macos")
        {
            continue;
        }
        let Some(url) = enc.attribute("url").and_then(secure) else {
            continue;
        };
        let version = enc
            .attribute((SPARKLE, "shortVersionString"))
            .map(str::to_string)
            .or_else(|| child_text("shortVersionString"))
            .or_else(|| enc.attribute((SPARKLE, "version")).map(str::to_string))
            .or_else(|| child_text("version"));
        let Some(version) = version.filter(|v| !v.is_empty() && v.len() <= 128) else {
            continue;
        };
        let newer = match &best {
            None => true,
            Some(b) => {
                crate::version::compare(&version, &b.version) == crate::version::Order::Newer
            }
        };
        if newer {
            best = Some(Offer {
                version,
                url,
                digest: None,
            });
        }
    }
    best.ok_or_else(|| "the feed offers no secure download".into())
}

fn base64_64(s: &str) -> Option<Box<[u8; 64]>> {
    // electron-builder writes standard base64 with padding.
    const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let s = s.trim().trim_end_matches('=');
    let mut bits: u32 = 0;
    let mut n = 0;
    let mut out = Vec::with_capacity(64);
    for c in s.bytes() {
        let v = ALPHABET.iter().position(|a| *a == c)? as u32;
        bits = bits << 6 | v;
        n += 6;
        if n >= 8 {
            n -= 8;
            out.push((bits >> n) as u8);
            bits &= (1 << n) - 1;
        }
    }
    let arr: [u8; 64] = out.try_into().ok()?;
    Some(Box::new(arr))
}

/// electron-builder's `latest.yml`: its `version`, and the one file ending in `ending` among
/// `files` (its `url`, relative to the feed's own address `base`, and its `sha512`).
pub fn electron(text: &str, base: &str, ending: &str) -> Result<Offer, String> {
    if text.len() > MAX_FEED {
        return Err("the feed is too large".into());
    }
    let mut version = None;
    let mut files: Vec<(String, Option<String>)> = Vec::new();
    let mut in_files = false;
    for raw in text.lines() {
        let indent = raw.len() - raw.trim_start().len();
        let line = raw.trim();
        let (item, line) = match line.strip_prefix("- ") {
            Some(r) => (true, r),
            None => (false, line),
        };
        let Some((k, v)) = line.split_once(':') else {
            continue;
        };
        let v = v.trim().trim_matches('\'').trim_matches('"').to_string();
        if indent == 0 && !item {
            in_files = k == "files";
            if k == "version" {
                version = Some(v);
            }
            continue;
        }
        if in_files {
            if item {
                files.push((String::new(), None));
            }
            if let Some(f) = files.last_mut() {
                match k.trim() {
                    "url" => f.0 = v,
                    "sha512" => f.1 = Some(v),
                    _ => {}
                }
            }
        }
    }
    let version = version
        .filter(|v| !v.is_empty() && v.len() <= 128)
        .ok_or("the feed names no version")?;
    let matching: Vec<_> = files.iter().filter(|(u, _)| u.ends_with(ending)).collect();
    let [(name, sha)] = matching.as_slice() else {
        return Err(format!(
            "the feed has {} files ending {ending:?}, not one",
            matching.len()
        ));
    };
    let base = url::Url::parse(base).map_err(|e| e.to_string())?;
    let joined = base.join(name).map_err(|e| e.to_string())?;
    if joined.host_str() != base.host_str() {
        return Err("the feed points at another site".into());
    }
    let url = secure(joined.as_str()).ok_or("the feed's file is not a secure address")?;
    Ok(Offer {
        version,
        url,
        digest: sha.as_deref().and_then(base64_64).map(Digest::Sha512),
    })
}

#[derive(Deserialize)]
struct GithubRelease {
    tag_name: String,
    #[serde(default)]
    draft: bool,
    #[serde(default)]
    prerelease: bool,
    assets: Vec<GithubAsset>,
}

#[derive(Deserialize)]
struct GithubAsset {
    name: String,
    browser_download_url: String,
    #[serde(default)]
    digest: Option<String>,
}

/// The address of a repository's latest release.
pub fn github_latest_url(owner: &str, repo: &str) -> String {
    format!("https://api.github.com/repos/{owner}/{repo}/releases/latest")
}

/// A GitHub release (the API's JSON): the one asset whose name ends with `suffix`, from
/// `github.com/<owner>/<repo>` itself. Drafts and pre-releases are not offered.
pub fn github(text: &str, owner: &str, repo: &str, suffix: &str) -> Result<Offer, String> {
    if text.len() > MAX_FEED {
        return Err("the release is too large".into());
    }
    let r: GithubRelease = serde_json::from_str(text).map_err(|e| e.to_string())?;
    if r.draft || r.prerelease {
        return Err("the latest release is a draft or a pre-release".into());
    }
    let matching: Vec<_> = r
        .assets
        .iter()
        .filter(|a| a.name.ends_with(suffix))
        .collect();
    let [asset] = matching.as_slice() else {
        return Err(format!(
            "the release has {} files ending {suffix:?}, not one",
            matching.len()
        ));
    };
    // The asset must be this repository's own: its download address names it.
    let prefix = format!("https://github.com/{owner}/{repo}/releases/download/");
    if !asset.browser_download_url.starts_with(&prefix) {
        return Err("the release's file is not this repository's own".into());
    }
    let url = secure(&asset.browser_download_url).ok_or("not a secure address")?;
    let version = r.tag_name.trim_start_matches(['v', 'V']).to_string();
    let digest = asset
        .digest
        .as_deref()
        .and_then(|d| d.strip_prefix("sha256:"))
        .and_then(|h| {
            let mut out = [0u8; 32];
            (h.len() == 64).then_some(())?;
            for (i, p) in h.as_bytes().chunks(2).enumerate() {
                out[i] = u8::from_str_radix(std::str::from_utf8(p).ok()?, 16).ok()?;
            }
            Some(Digest::Sha256(out))
        });
    Ok(Offer {
        version,
        url,
        digest,
    })
}

#[derive(Deserialize)]
struct GitlabRelease {
    tag_name: String,
    assets: GitlabAssets,
}

#[derive(Deserialize)]
struct GitlabAssets {
    links: Vec<GitlabLink>,
}

#[derive(Deserialize)]
struct GitlabLink {
    name: String,
    #[serde(default)]
    direct_asset_url: Option<String>,
    url: String,
}

/// The address of a gitlab.com project's latest release.
pub fn gitlab_latest_url(project: &str) -> String {
    let encoded: String = project
        .bytes()
        .map(|b| {
            if b == b'/' {
                "%2F".to_string()
            } else {
                (b as char).to_string()
            }
        })
        .collect();
    format!("https://gitlab.com/api/v4/projects/{encoded}/releases/permalink/latest")
}

/// A GitLab release (the API's JSON): the one asset link whose name ends with `suffix`, on
/// gitlab.com under the project's own path.
pub fn gitlab(text: &str, project: &str, suffix: &str) -> Result<Offer, String> {
    if text.len() > MAX_FEED {
        return Err("the release is too large".into());
    }
    let r: GitlabRelease = serde_json::from_str(text).map_err(|e| e.to_string())?;
    let matching: Vec<_> = r
        .assets
        .links
        .iter()
        .filter(|l| l.name.ends_with(suffix))
        .collect();
    let [link] = matching.as_slice() else {
        return Err(format!(
            "the release has {} files ending {suffix:?}, not one",
            matching.len()
        ));
    };
    let url = link.direct_asset_url.as_deref().unwrap_or(&link.url);
    if !url.starts_with(&format!("https://gitlab.com/{project}/")) {
        return Err("the release's file is not this project's own".into());
    }
    Ok(Offer {
        version: r.tag_name.trim_start_matches(['v', 'V']).to_string(),
        url: secure(url).ok_or("not a secure address")?,
        digest: None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const APPCAST: &str = r#"<?xml version="1.0" encoding="utf-8"?>
<rss version="2.0" xmlns:sparkle="http://www.andymatuschak.org/xml-namespaces/sparkle">
<channel><title>Painter</title>
<item><title>4.2.1</title><sparkle:version>4210</sparkle:version><sparkle:shortVersionString>4.2.1</sparkle:shortVersionString>
<enclosure url="https://downloads.example.com/Painter-4.2.1.dmg" length="100" type="application/octet-stream"/></item>
<item><title>4.10.0</title><enclosure url="https://downloads.example.com/Painter-4.10.0.dmg" sparkle:shortVersionString="4.10.0" sparkle:version="4100" length="100"/></item>
<item><title>9.0 for Windows</title><enclosure url="https://downloads.example.com/Painter.exe" sparkle:os="windows" sparkle:shortVersionString="9.0"/></item>
<item><title>plain</title><enclosure url="http://downloads.example.com/Painter-99.dmg" sparkle:shortVersionString="99"/></item>
</channel></rss>"#;

    #[test]
    fn a_sparkle_feed_offers_its_newest_secure_mac_download() {
        let o = sparkle(APPCAST).unwrap();
        assert_eq!(o.version, "4.10.0");
        assert_eq!(o.url, "https://downloads.example.com/Painter-4.10.0.dmg");
        assert!(sparkle("<rss/>").is_err());
        // No DTDs: an entity cannot put another address in.
        let entity = r#"<!DOCTYPE rss [<!ENTITY u "https://evil.example.com/x.dmg">]><rss xmlns:sparkle="http://www.andymatuschak.org/xml-namespaces/sparkle"><channel><item><enclosure url="&u;" sparkle:shortVersionString="9"/></item></channel></rss>"#;
        assert!(sparkle(entity).is_err());
        // No DTDs (no entity expansion).
        assert!(sparkle(r#"<!DOCTYPE x [<!ENTITY a "aaaa">]><rss/>"#).is_err());
    }

    #[test]
    fn an_electron_feed_offers_the_one_file_named() {
        let sha = "A".repeat(86) + "==";
        let yml = format!(
            "version: 1.4.2\nfiles:\n  - url: Chat-Setup-1.4.2.exe\n    sha512: {sha}\n    size: 1000\n  - url: Chat-1.4.2.zip\n    sha512: {sha}\npath: Chat-Setup-1.4.2.exe\nsha512: {sha}\nreleaseDate: '2026-10-01T00:00:00.000Z'\n"
        );
        let o = electron(&yml, "https://updates.example.com/win/latest.yml", ".exe").unwrap();
        assert_eq!(o.version, "1.4.2");
        assert_eq!(
            o.url,
            "https://updates.example.com/win/Chat-Setup-1.4.2.exe"
        );
        assert!(matches!(o.digest, Some(Digest::Sha512(_))));
        // A file on another site is refused.
        let away = yml.replace(
            "url: Chat-Setup-1.4.2.exe",
            "url: https://evil.example.net/x.exe",
        );
        assert!(electron(&away, "https://updates.example.com/win/latest.yml", ".exe").is_err());
        // Two files with the ending: PCTwin does not pick one.
        let two = yml.replace("Chat-1.4.2.zip", "Chat-other.exe");
        assert!(electron(&two, "https://updates.example.com/win/latest.yml", ".exe").is_err());
    }

    #[test]
    fn a_github_release_offers_only_this_repositorys_own_asset() {
        let digest = "a".repeat(64);
        let rel = format!(
            r#"{{"tag_name":"v2.3.0","draft":false,"prerelease":false,"assets":[
            {{"name":"tool-2.3.0-x64.msi","browser_download_url":"https://github.com/maker/tool/releases/download/v2.3.0/tool-2.3.0-x64.msi","digest":"sha256:{digest}"}},
            {{"name":"tool-2.3.0-arm64.msi","browser_download_url":"https://github.com/maker/tool/releases/download/v2.3.0/tool-2.3.0-arm64.msi"}}]}}"#
        );
        let o = github(&rel, "maker", "tool", "-x64.msi").unwrap();
        assert_eq!(o.version, "2.3.0");
        assert!(matches!(o.digest, Some(Digest::Sha256(d)) if d == [0xaa; 32]));
        // The same JSON claimed for another repository (a fork, a moved owner: T21-007).
        assert!(github(&rel, "other", "tool", "-x64.msi").is_err());
        assert!(github(&rel, "maker", "tool", ".msi").is_err());
        assert!(
            github(
                &rel.replace("\"prerelease\":false", "\"prerelease\":true"),
                "maker",
                "tool",
                "-x64.msi"
            )
            .is_err()
        );
        assert_eq!(
            github_latest_url("maker", "tool"),
            "https://api.github.com/repos/maker/tool/releases/latest"
        );
    }

    #[test]
    fn a_gitlab_release_offers_only_this_projects_own_asset() {
        let rel = r#"{"tag_name":"v1.0","assets":{"links":[{"name":"app-1.0.AppImage","url":"https://gitlab.com/group/app/-/releases/v1.0/downloads/app-1.0.AppImage"}]}}"#;
        assert_eq!(
            gitlab(rel, "group/app", ".AppImage").unwrap().version,
            "1.0"
        );
        assert!(gitlab(rel, "other/app", ".AppImage").is_err());
        assert_eq!(
            gitlab_latest_url("group/app"),
            "https://gitlab.com/api/v4/projects/group%2Fapp/releases/permalink/latest"
        );
    }
}
