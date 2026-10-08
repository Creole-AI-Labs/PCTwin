use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use pctwin_record::{FolderRole, Item, ItemId};

use crate::folders::is_within;

/// At most this many recent files are read, newest first.
const MAX_RECENT: usize = 5000;

/// What the old laptop records about what this person uses. Read only on the old laptop, used only
/// to set the order of the move, and never sent anywhere.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Usage {
    /// Files opened recently, with when (nanoseconds since 1970) when known.
    pub recent: Vec<(PathBuf, Option<i64>)>,
    /// Folders the person pinned.
    pub pinned: Vec<PathBuf>,
}

/// Reads the signed-in person's usage records: Windows Recent Items and Quick Access pins;
/// macOS Spotlight's last-opened dates and Finder favourites; Linux's recently-used list and
/// file-manager bookmarks. Anything missing or
/// switched off simply gives nothing.
pub fn read_usage(home: &Path) -> Usage {
    let mut usage = Usage::default();
    if cfg!(windows) {
        if let Some(appdata) = std::env::var_os("APPDATA") {
            let dir = PathBuf::from(appdata).join(r"Microsoft\Windows\Recent");
            usage.recent = recent_from_lnk_dir(&dir);
            let quick =
                dir.join(r"AutomaticDestinations\f01b4d95cf55d32a.automaticDestinations-ms");
            if let Ok(bytes) = std::fs::read(quick) {
                usage.pinned = pinned_from_quick_access(&bytes);
            }
        }
    } else if cfg!(target_os = "macos") {
        let out = std::process::Command::new("mdfind")
            .arg("-onlyin")
            .arg(home)
            .arg("kMDItemLastUsedDate >= $time.today(-30)")
            .output()
            .map(|o| String::from_utf8_lossy(&o.stdout).into_owned())
            .unwrap_or_default();
        usage.recent = recent_from_mdfind(&out)
            .into_iter()
            .map(|p| (p, None))
            .collect();
        let lists = home.join("Library/Application Support/com.apple.sharedfilelist");
        let favourites = ["sfl3", "sfl2"]
            .iter()
            .find_map(|ext| {
                std::fs::read(lists.join(format!("com.apple.LSSharedFileList.FavoriteItems.{ext}")))
                    .ok()
            })
            .unwrap_or_default();
        usage.pinned = pinned_from_finder_favourites(&favourites);
    } else {
        let data = std::env::var_os("XDG_DATA_HOME")
            .map(PathBuf::from)
            .filter(|p| p.is_absolute())
            .unwrap_or_else(|| home.join(".local/share"));
        if let Ok(xml) = std::fs::read_to_string(data.join("recently-used.xbel")) {
            usage.recent = recent_from_xbel(&xml);
        }
        let config = std::env::var_os("XDG_CONFIG_HOME")
            .map(PathBuf::from)
            .filter(|p| p.is_absolute())
            .unwrap_or_else(|| home.join(".config"));
        if let Ok(text) = std::fs::read_to_string(config.join("gtk-3.0/bookmarks")) {
            usage.pinned = bookmarks_from_gtk(&text);
        }
    }
    usage.recent.sort_by_key(|r| std::cmp::Reverse(r.1));
    usage.recent.truncate(MAX_RECENT);
    usage
}

/// Windows: each shortcut in the Recent Items folder points at a file opened recently; the
/// shortcut's own modified time is when.
pub fn recent_from_lnk_dir(dir: &Path) -> Vec<(PathBuf, Option<i64>)> {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut found: Vec<(PathBuf, Option<i64>)> = entries
        .flatten()
        .filter(|e| {
            e.path()
                .extension()
                .is_some_and(|x| x.eq_ignore_ascii_case("lnk"))
        })
        .filter_map(|e| {
            let when = e
                .metadata()
                .ok()
                .and_then(|m| m.modified().ok())
                .and_then(ns_of);
            let link = lnk::ShellLink::open(e.path(), lnk::encoding::WINDOWS_1252).ok()?;
            Some((PathBuf::from(link.link_target()?), when))
        })
        .collect();
    found.sort_by_key(|r| std::cmp::Reverse(r.1));
    found.truncate(MAX_RECENT);
    found
}

fn ns_of(t: std::time::SystemTime) -> Option<i64> {
    let d = t.duration_since(std::time::UNIX_EPOCH).ok()?;
    i64::try_from(d.as_nanos()).ok()
}

/// macOS: `mdfind` prints one path per line.
pub fn recent_from_mdfind(output: &str) -> Vec<PathBuf> {
    output
        .lines()
        .map(str::trim)
        .filter(|l| l.starts_with('/'))
        .map(PathBuf::from)
        .take(MAX_RECENT)
        .collect()
}

/// Linux: `recently-used.xbel` lists `<bookmark href="file:///…" modified=… visited=…>`.
pub fn recent_from_xbel(xml: &str) -> Vec<(PathBuf, Option<i64>)> {
    let Ok(doc) = roxmltree::Document::parse(xml) else {
        return Vec::new();
    };
    doc.descendants()
        .filter(|n| n.has_tag_name("bookmark"))
        .filter_map(|n| {
            let path = file_uri_to_path(n.attribute("href")?)?;
            let when = ["visited", "modified", "added"]
                .iter()
                .filter_map(|a| n.attribute(*a).and_then(parse_iso_utc_ns))
                .max();
            Some((path, when))
        })
        .collect()
}

/// Linux: `gtk-3.0/bookmarks` holds one `file:///path optional label` per line.
pub fn bookmarks_from_gtk(text: &str) -> Vec<PathBuf> {
    text.lines()
        .filter_map(|l| file_uri_to_path(l.split_whitespace().next()?))
        .collect()
}

/// `file:///home/ada/My%20Doc.pdf` as a path; anything that is not a local file is `None`.
fn file_uri_to_path(uri: &str) -> Option<PathBuf> {
    let rest = uri.strip_prefix("file://")?;
    let rest = rest.strip_prefix("localhost").unwrap_or(rest);
    if !rest.starts_with('/') {
        return None;
    }
    let bytes = percent_decode(rest)?;
    #[cfg(unix)]
    {
        use std::os::unix::ffi::OsStringExt;
        Some(PathBuf::from(std::ffi::OsString::from_vec(bytes)))
    }
    #[cfg(not(unix))]
    {
        Some(PathBuf::from(String::from_utf8(bytes).ok()?))
    }
}

fn percent_decode(s: &str) -> Option<Vec<u8>> {
    let b = s.as_bytes();
    let mut out = Vec::with_capacity(b.len());
    let mut i = 0;
    while i < b.len() {
        if b[i] == b'%' {
            let hex = s.get(i + 1..i + 3)?;
            out.push(u8::from_str_radix(hex, 16).ok()?);
            i += 3;
        } else {
            out.push(b[i]);
            i += 1;
        }
    }
    Some(out)
}

/// `2026-10-01T09:30:00.5Z` (UTC) as nanoseconds since 1970.
pub fn parse_iso_utc_ns(s: &str) -> Option<i64> {
    let s = s.strip_suffix('Z')?;
    let (date, time) = s.split_once('T')?;
    let mut d = date.split('-');
    let (y, mo, da): (i64, i64, i64) = (
        d.next()?.parse().ok()?,
        d.next()?.parse().ok()?,
        d.next()?.parse().ok()?,
    );
    if d.next().is_some() || !(1..=12).contains(&mo) || !(1..=31).contains(&da) {
        return None;
    }
    let (hms, frac) = time.split_once('.').unwrap_or((time, ""));
    let mut t = hms.split(':');
    let (h, mi, se): (i64, i64, i64) = (
        t.next()?.parse().ok()?,
        t.next()?.parse().ok()?,
        t.next()?.parse().ok()?,
    );
    if t.next().is_some() || h > 23 || mi > 59 || se > 60 {
        return None;
    }
    let mut frac_ns: i64 = 0;
    if !frac.is_empty() {
        if !frac.bytes().all(|c| c.is_ascii_digit()) {
            return None;
        }
        let digits: String = frac.chars().chain("000000000".chars()).take(9).collect();
        frac_ns = digits.parse().ok()?;
    }
    // Days from 1970-01-01 for a date in the proleptic Gregorian calendar.
    let (y2, m2) = if mo <= 2 {
        (y - 1, mo + 9)
    } else {
        (y, mo - 3)
    };
    let era = y2.div_euclid(400);
    let yoe = y2 - era * 400;
    let doy = (153 * m2 + 2) / 5 + da - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    let days = era * 146_097 + doe - 719_468;
    let secs = days * 86_400 + h * 3600 + mi * 60 + se;
    secs.checked_mul(1_000_000_000)?.checked_add(frac_ns)
}

/// The scanned items this person used recently or keeps in a pinned folder, each with when it was
/// last used (pinned folders without a time count as used now-ish: they rank after dated ones).
/// `roots` gives where each role's folder is on the old laptop.
pub fn personal_essentials(
    items: &[Item],
    roots: &BTreeMap<FolderRole, PathBuf>,
    usage: &Usage,
) -> BTreeMap<ItemId, i64> {
    let mut out = BTreeMap::new();
    for item in items {
        let Some(root) = roots.get(&item.place.role) else {
            continue;
        };
        let mut path = root.clone();
        for part in item.path.parts() {
            path.push(part.display());
        }
        let used = usage
            .recent
            .iter()
            .filter(|(p, _)| crate::folders::same_place(p, &path))
            .map(|(_, when)| when.unwrap_or(0))
            .max();
        let pinned = usage.pinned.iter().any(|f| is_within(&path, f));
        match (used, pinned) {
            (Some(when), _) => {
                out.insert(item.id, when);
            }
            (None, true) => {
                out.insert(item.id, 0);
            }
            (None, false) => {}
        }
    }
    out
}

/// Most entries read from a Quick Access list (it holds a few hundred at most).
const MAX_DEST_ENTRIES: u32 = 10_000;

/// Windows: the folders pinned to Quick Access, in their pinned order. They are kept in the jump
/// list `f01b4d95cf55d32a.automaticDestinations-ms`: a compound file whose `DestList` stream has
/// one entry per item, each with a pin status (-1 for a recent item, else its place among the
/// pinned) and its path (layout as documented by libyal's dtformats). Anything damaged gives
/// nothing.
pub fn pinned_from_quick_access(bytes: &[u8]) -> Vec<PathBuf> {
    let Ok(mut file) = cfb::CompoundFile::open(std::io::Cursor::new(bytes)) else {
        return Vec::new();
    };
    let mut list = Vec::new();
    let read = file
        .open_stream("DestList")
        .and_then(|mut s| std::io::Read::read_to_end(&mut s, &mut list));
    if read.is_err() {
        return Vec::new();
    }
    pinned_in_destlist(&list)
}

fn pinned_in_destlist(d: &[u8]) -> Vec<PathBuf> {
    let le32 = |at: usize| -> Option<u32> {
        d.get(at..at.checked_add(4)?)
            .and_then(|b| b.try_into().ok())
            .map(u32::from_le_bytes)
    };
    let (Some(version), Some(count)) = (le32(0), le32(4)) else {
        return Vec::new();
    };
    // Version 1 entries are 114 bytes before the path; later ones 130, with 4 more after it.
    let (fixed, trailing) = if version >= 2 { (130, 4) } else { (114, 0) };
    let mut pinned: Vec<(u32, PathBuf)> = Vec::new();
    let mut at = 32usize;
    for _ in 0..count.min(MAX_DEST_ENTRIES) {
        let Some(entry) = at.checked_add(fixed).and_then(|end| d.get(at..end)) else {
            break;
        };
        let pin = i32::from_le_bytes([entry[108], entry[109], entry[110], entry[111]]);
        let chars = usize::from(u16::from_le_bytes([entry[fixed - 2], entry[fixed - 1]]));
        let start = at + fixed;
        let Some(raw) = start
            .checked_add(chars * 2)
            .and_then(|end| d.get(start..end))
        else {
            break;
        };
        let wide: Vec<u16> = raw
            .as_chunks::<2>()
            .0
            .iter()
            .map(|c| u16::from_le_bytes(*c))
            .collect();
        at = start + chars * 2 + trailing;
        if let Ok(place) = u32::try_from(pin) {
            pinned.push((place, path_from_wide(&wide)));
        }
    }
    pinned.sort_by_key(|(place, _)| *place);
    pinned.into_iter().map(|(_, p)| p).collect()
}

/// A Windows path as stored (UTF-16, which may hold unpaired surrogates), kept exact on Windows.
fn path_from_wide(wide: &[u16]) -> PathBuf {
    #[cfg(windows)]
    {
        use std::os::windows::ffi::OsStringExt;
        PathBuf::from(std::ffi::OsString::from_wide(wide))
    }
    #[cfg(not(windows))]
    {
        PathBuf::from(String::from_utf16_lossy(wide))
    }
}

/// How deep a Finder favourites list is searched for bookmarks.
const MAX_ARCHIVE_DEPTH: usize = 32;

/// macOS: the folders in the Finder sidebar's Favourites. They are kept as bookmarks inside a
/// keyed archive (`com.apple.LSSharedFileList.FavoriteItems.sfl3`, or `.sfl2` on older systems);
/// each bookmark's path components are read (layout as documented by mac_alias). Anything damaged
/// gives nothing.
pub fn pinned_from_finder_favourites(bytes: &[u8]) -> Vec<PathBuf> {
    let Ok(archive) = plist::Value::from_reader(std::io::Cursor::new(bytes)) else {
        return Vec::new();
    };
    let mut found = Vec::new();
    collect_bookmarks(&archive, 0, &mut found);
    found
}

fn collect_bookmarks(value: &plist::Value, depth: usize, found: &mut Vec<PathBuf>) {
    if depth > MAX_ARCHIVE_DEPTH {
        return;
    }
    match value {
        plist::Value::Data(d) if d.starts_with(b"book") => {
            if let Some(path) = bookmark_path(d) {
                found.push(path);
            }
        }
        plist::Value::Array(items) => {
            for item in items {
                collect_bookmarks(item, depth + 1, found);
            }
        }
        plist::Value::Dictionary(map) => {
            for item in map.values() {
                collect_bookmarks(item, depth + 1, found);
            }
        }
        _ => {}
    }
}

/// The target path held in a bookmark: the array of path components under key 0x1004 in its first
/// table of contents. Every offset is checked before it is used.
fn bookmark_path(d: &[u8]) -> Option<PathBuf> {
    let le32 = |at: usize| -> Option<usize> {
        d.get(at..at.checked_add(4)?)
            .and_then(|b| b.try_into().ok())
            .map(u32::from_le_bytes)
            .and_then(|v| usize::try_from(v).ok())
    };
    // Offsets are from the end of the header, whose size is at byte 12.
    let base = le32(12)?;
    let toc = base.checked_add(le32(base)?)?;
    if le32(toc.checked_add(4)?)? != 0xffff_fffe {
        return None;
    }
    let count = le32(toc.checked_add(16)?)?.min(4096);
    let path_record = (0..count).find_map(|i| {
        let entry = toc.checked_add(20)?.checked_add(i.checked_mul(12)?)?;
        (le32(entry)? == 0x1004).then(|| le32(entry + 4))?
    })?;
    let array = base.checked_add(path_record)?;
    let (length, kind) = (le32(array)?, le32(array.checked_add(4)?)?);
    if kind != 0x0601 {
        return None;
    }
    let mut path = PathBuf::from("/");
    for i in 0..(length / 4).min(256) {
        let record = base.checked_add(le32(array.checked_add(8 + i * 4)?)?)?;
        let (len, kind) = (le32(record)?, le32(record.checked_add(4)?)?);
        if kind != 0x0101 {
            return None;
        }
        let start = record.checked_add(8)?;
        let text = std::str::from_utf8(d.get(start..start.checked_add(len)?)?).ok()?;
        if text.is_empty() || text.contains('/') || text == ".." {
            return None;
        }
        path.push(text);
    }
    Some(path)
}
