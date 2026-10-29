//! What a file the person works with needs (Task List 2.9, "Can I actually work?"; Product Spec
//! R17): which checks of [`crate::checks`] to run for it.
//!
//! Discovery first (ADOPTION change 8): the file is told by its **content** where its content
//! says what it is. A ZIP archive and a SQLite database are known by their first bytes, whatever
//! their names. A project that links to other files is read by a generic **reader** chosen by
//! the `project-files` catalogue (data, ADOPTION change 7): its rows name a kind of project by
//! its name ending and the reader that lists its linked files and fonts. Two readers exist, one
//! per mechanism, never one per app:
//!
//! - [`Reader::XmlAttributes`]: an XML file whose named attributes hold relative links and font
//!   families (an SVG drawing: `href`, `xlink:href`, `font-family`);
//! - [`Reader::TextLines`]: a text file whose every line that is not a comment is a link (an M3U
//!   playlist).
//!
//! A file of a kind no row names, that is not an archive or database, is "found, not yet
//! supported", never passed as fine. Reading is bounded and only reads: nothing is opened by
//! another program, nothing is followed outside the project's folder (such links are counted and
//! said, never looked up).

use std::fs::File;
use std::io::Read;
use std::path::{Component, Path, PathBuf};
use std::sync::OnceLock;

use pctwin_catalogue::{Catalogue, Row};
use serde::Deserialize;

/// The largest project file read, in bytes; a larger one is read this far and said partly read.
pub const MAX_PROJECT_BYTES: u64 = 8 * 1024 * 1024;
/// The most links one project lists (the rest are counted).
pub const MAX_PROJECT_LINKS: usize = 10_000;
/// The most fonts one project lists.
pub const MAX_PROJECT_FONTS: usize = 200;

/// How a kind of project lists what it needs.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case", deny_unknown_fields)]
pub enum Reader {
    /// XML whose attributes `links` hold relative links and `fonts` hold font families (a
    /// comma-separated list, CSS style).
    XmlAttributes {
        links: Vec<String>,
        #[serde(default)]
        fonts: Vec<String>,
    },
    /// Text whose every line is a link, except lines starting with `comment`.
    TextLines { comment: String },
}

/// One `project-files` row.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProjectRow {
    /// The name ending, without the dot, lower case.
    pub extension: String,
    /// What it is, to show.
    pub name: String,
    pub reader: Reader,
}

fn attr_ok(a: &str) -> bool {
    !a.is_empty()
        && a.len() <= 64
        && a.bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b':' | b'-' | b'_'))
}

impl Row for ProjectRow {
    const CATALOGUE: &'static str = "project-files";
    const FIELDS: &'static [&'static str] = &["extension", "name", "reader"];
    fn check(&self) -> Result<(), String> {
        let ext_ok = !self.extension.is_empty()
            && self.extension.len() <= 16
            && self
                .extension
                .bytes()
                .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit());
        if !ext_ok || self.name.trim().is_empty() || self.name.len() > 64 {
            return Err("a row is a lower-case name ending and a name".into());
        }
        match &self.reader {
            Reader::XmlAttributes { links, fonts } => {
                if links.is_empty() && fonts.is_empty() {
                    return Err("an XML reader names the attributes it reads".into());
                }
                if !links.iter().chain(fonts).all(|a| attr_ok(a)) {
                    return Err("an attribute name is letters, digits and `:-_`".into());
                }
            }
            Reader::TextLines { comment } => {
                if comment.is_empty() || comment.len() > 4 {
                    return Err("a comment mark is one to four characters".into());
                }
            }
        }
        Ok(())
    }
    fn identity(&self) -> Option<String> {
        Some(self.extension.clone())
    }
}

/// The rows compiled into this build.
pub fn shipped() -> &'static Catalogue<ProjectRow> {
    static ROWS: OnceLock<Catalogue<ProjectRow>> = OnceLock::new();
    ROWS.get_or_init(|| Catalogue::shipped(include_str!("../catalogues/project-files.json")))
}

/// What a project needs, as read.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Needs {
    /// What kind of project it is (its row's name).
    pub kind: String,
    /// Linked files inside the project's folder, relative to it.
    pub links: Vec<PathBuf>,
    /// Linked files outside the project's folder (absolute, or through `..`): counted, never
    /// looked up.
    pub outside: u64,
    /// Links past [`MAX_PROJECT_LINKS`], counted.
    pub more_links: u64,
    /// Font families it names.
    pub fonts: Vec<String>,
    /// Other font families it names past [`MAX_PROJECT_FONTS`], counted (each once).
    pub more_fonts: u64,
    /// Only the first [`MAX_PROJECT_BYTES`] were read.
    pub partly_read: bool,
}

/// What a file is, by its content first.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Found {
    /// A ZIP archive (by its first bytes).
    Archive,
    /// A SQLite database (by its first bytes).
    Database,
    /// A project a row's reader read.
    Project(Needs),
    /// A kind PCTwin cannot check yet: found, not yet supported.
    NotSupported,
    /// It could not be read (no permission, gone).
    CouldNotRead,
}

const SQLITE: &[u8] = b"SQLite format 3\0";

/// What `path` is and, for a project, what it needs, by `rows`.
pub fn look(path: &Path, rows: &Catalogue<ProjectRow>) -> Found {
    let Ok(file) = File::open(path) else {
        return Found::CouldNotRead;
    };
    let mut bytes = Vec::new();
    if file
        .take(MAX_PROJECT_BYTES + 1)
        .read_to_end(&mut bytes)
        .is_err()
    {
        return Found::CouldNotRead;
    }
    if bytes.starts_with(b"PK\x03\x04") || bytes.starts_with(b"PK\x05\x06") {
        return Found::Archive;
    }
    if bytes.starts_with(SQLITE) {
        return Found::Database;
    }
    let partly_read = bytes.len() as u64 > MAX_PROJECT_BYTES;
    bytes.truncate(MAX_PROJECT_BYTES as usize);
    let ext = path
        .extension()
        .map(|e| e.to_string_lossy().to_lowercase())
        .unwrap_or_default();
    let Some(row) = rows.rows().iter().find(|r| r.extension == ext) else {
        return Found::NotSupported;
    };
    let text = String::from_utf8_lossy(&bytes);
    let (raw_links, fonts) = match &row.reader {
        Reader::XmlAttributes { links, fonts } => (
            attribute_values(&text, links),
            attribute_values(&text, fonts)
                .iter()
                .flat_map(|v| font_families(v))
                .collect(),
        ),
        Reader::TextLines { comment } => (text_lines(&text, comment), Vec::new()),
    };
    let mut needs = Needs {
        kind: row.name.clone(),
        partly_read,
        ..Needs::default()
    };
    for raw in raw_links {
        let Some(link) = local_link(&raw) else {
            continue;
        };
        if !inside(&link) {
            needs.outside += 1;
        } else if needs.links.contains(&link) {
            continue;
        } else if needs.links.len() < MAX_PROJECT_LINKS {
            needs.links.push(link);
        } else {
            needs.more_links += 1;
        }
    }
    let mut past = std::collections::HashSet::new();
    for f in fonts {
        if needs.fonts.iter().any(|h| h.eq_ignore_ascii_case(&f)) {
            continue;
        }
        if needs.fonts.len() < MAX_PROJECT_FONTS {
            needs.fonts.push(f);
        } else if past.insert(f.to_ascii_lowercase()) {
            needs.more_fonts += 1;
        }
    }
    Found::Project(needs)
}

/// Every value of an attribute named exactly one of `names` (`name="value"` or `name='value'`,
/// the name starting after a space, tab, line break or `<`), with XML's five entities read.
pub fn attribute_values(text: &str, names: &[String]) -> Vec<String> {
    let mut out = Vec::new();
    let bytes = text.as_bytes();
    for name in names {
        let mut from = 0;
        while let Some(at) = text[from..].find(name.as_str()) {
            let start = from + at;
            from = start + name.len();
            let before = start.checked_sub(1).map(|i| bytes[i]);
            if !matches!(before, Some(b' ' | b'\t' | b'\n' | b'\r' | b'<')) {
                continue;
            }
            let rest = text[from..].trim_start();
            let Some(rest) = rest.strip_prefix('=') else {
                continue;
            };
            let rest = rest.trim_start();
            let Some(quote) = rest.chars().next().filter(|c| *c == '"' || *c == '\'') else {
                continue;
            };
            let Some(end) = rest[1..].find(quote) else {
                continue;
            };
            out.push(entities(&rest[1..1 + end]));
        }
    }
    out
}

fn entities(v: &str) -> String {
    v.replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&apos;", "'")
        .replace("&amp;", "&")
}

/// The family names in a CSS-style list (`"Inter", Arial, sans-serif`), without the generic
/// families every system provides (CSS's own words, not a list of fonts).
pub fn font_families(list: &str) -> Vec<String> {
    const GENERIC: [&str; 12] = [
        "serif",
        "sans-serif",
        "monospace",
        "cursive",
        "fantasy",
        "system-ui",
        "ui-serif",
        "ui-sans-serif",
        "ui-monospace",
        "ui-rounded",
        "math",
        "emoji",
    ];
    list.split(',')
        .map(|f| f.trim().trim_matches(['"', '\'']).trim().to_string())
        .filter(|f| !f.is_empty() && f.len() <= 128)
        .filter(|f| !GENERIC.iter().any(|g| f.eq_ignore_ascii_case(g)))
        .collect()
}

/// Every line that is not empty and does not start with `comment`.
pub fn text_lines(text: &str, comment: &str) -> Vec<String> {
    text.lines()
        .map(str::trim)
        .filter(|l| !l.is_empty() && !l.starts_with(comment))
        .map(str::to_string)
        .collect()
}

/// A link as a path on this laptop, if it is one: never a web address, a `data:` value or a
/// reference inside the same file; its query and fragment are dropped and `%XX` read.
pub fn local_link(raw: &str) -> Option<PathBuf> {
    let raw = raw.trim();
    if raw.is_empty() || raw.starts_with('#') || raw.contains("://") {
        return None;
    }
    let lower = raw.to_ascii_lowercase();
    if ["data:", "mailto:", "javascript:", "urn:", "about:"]
        .iter()
        .any(|s| lower.starts_with(s))
    {
        return None;
    }
    let raw = raw.split(['?', '#']).next().unwrap_or_default();
    let decoded = percent(raw)?;
    if decoded.is_empty() {
        return None;
    }
    // Links written on Windows use back slashes; the project's own folder is the same either way.
    Some(PathBuf::from(decoded.replace('\\', "/")))
}

fn percent(s: &str) -> Option<String> {
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
    String::from_utf8(out).ok()
}

/// Whether `link` stays inside the project's folder: relative, with no `..`, no drive.
pub fn inside(link: &Path) -> bool {
    let s = link.to_string_lossy();
    // `C:` and `C:/` are a drive on Windows whatever this laptop is.
    let drive = s.as_bytes().get(1) == Some(&b':');
    !drive
        && link.components().next().is_some()
        && link
            .components()
            .all(|c| matches!(c, Component::Normal(_) | Component::CurDir))
}

#[cfg(test)]
// Tests make their own fixture files (the crate's fence is for its own code: lib.rs).
#[allow(clippy::disallowed_methods)]
mod tests {
    use super::*;

    fn rows(r: &str) -> Catalogue<ProjectRow> {
        Catalogue::shipped(&format!(
            r#"{{"format":1,"catalogue":"project-files","version":1,"rows":[{r}]}}"#
        ))
    }

    fn put(path: &Path, bytes: &[u8]) {
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, bytes).unwrap();
    }

    #[test]
    fn content_decides_before_the_name() {
        let t = tempfile::tempdir().unwrap();
        let zip = t.path().join("photos.svg");
        put(&zip, b"PK\x03\x04rest");
        assert_eq!(look(&zip, shipped()), Found::Archive);
        let empty_zip = t.path().join("e.bin");
        put(&empty_zip, b"PK\x05\x06rest");
        assert_eq!(look(&empty_zip, shipped()), Found::Archive);
        let db = t.path().join("notes.m3u");
        put(&db, b"SQLite format 3\0and more");
        assert_eq!(look(&db, shipped()), Found::Database);
        let other = t.path().join("drawing.xyz");
        put(&other, b"<xml href=\"a.png\"/>");
        assert_eq!(look(&other, shipped()), Found::NotSupported);
        assert_eq!(
            look(&t.path().join("gone.svg"), shipped()),
            Found::CouldNotRead
        );
        assert!(shipped().report().refused.is_none());
        assert!(shipped().report().skipped.is_empty());
    }

    #[test]
    fn an_svg_lists_its_linked_images_and_fonts() {
        let t = tempfile::tempdir().unwrap();
        let svg = t.path().join("Poster.SVG");
        put(
            &svg,
            br##"<svg xmlns:xlink="http://www.w3.org/1999/xlink">
  <image href="images/front%20cover.jpg?v=2"/>
  <image xlink:href='logo.png#part'/>
  <image href="images/front%20cover.jpg"/>
  <use href="#local"/>
  <image href="https://example.org/a.png"/>
  <image href="data:image/png;base64,AAAA"/>
  <image href="../shared/bg.png"/>
  <image href="C:\pics\abs.png"/>
  <image dataref="not-a-link.png"/>
  <text font-family="'Inter', Arial, sans-serif">Hi</text>
  <text font-family="inter">Hi</text>
  <a title="Tom &amp; Jerry" href="notes/Tom &amp; Jerry.txt"/>
</svg>"##,
        );
        let Found::Project(n) = look(&svg, shipped()) else {
            panic!("an SVG is a project");
        };
        assert_eq!(n.kind, "SVG drawing");
        assert_eq!(
            n.links,
            [
                PathBuf::from("images/front cover.jpg"),
                PathBuf::from("notes/Tom & Jerry.txt"),
                PathBuf::from("logo.png"),
            ]
        );
        assert_eq!(n.outside, 2, "../ and a drive are counted, never looked up");
        assert_eq!(n.fonts, ["Inter", "Arial"]);
        assert_eq!(n.more_fonts, 0);
        assert!(!n.partly_read);
    }

    /// Fresh-context review F3: fonts past the limit are counted (each once), never dropped
    /// silently.
    #[test]
    fn fonts_past_the_limit_are_counted() {
        let t = tempfile::tempdir().unwrap();
        let svg = t.path().join("Many.svg");
        let mut text = String::from("<svg>");
        for i in 0..MAX_PROJECT_FONTS + 5 {
            text.push_str(&format!("<text font-family=\"Family{i}\">x</text>"));
        }
        // The same names again, in another case: not counted twice.
        for i in 0..MAX_PROJECT_FONTS + 5 {
            text.push_str(&format!("<text font-family=\"FAMILY{i}\">x</text>"));
        }
        text.push_str("</svg>");
        put(&svg, text.as_bytes());
        let Found::Project(n) = look(&svg, shipped()) else {
            panic!("an SVG is a project");
        };
        assert_eq!(n.fonts.len(), MAX_PROJECT_FONTS);
        assert_eq!(n.more_fonts, 5);
    }

    #[test]
    fn a_playlist_lists_its_songs() {
        let t = tempfile::tempdir().unwrap();
        let m3u = t.path().join("Road trip.m3u8");
        put(
            &m3u,
            b"#EXTM3U\n#EXTINF:123,Song\nMusic\\Song one.mp3\n\n  Music/Song two.flac  \nhttp://radio.example/stream\n/home/a/abs.mp3\n",
        );
        let Found::Project(n) = look(&m3u, shipped()) else {
            panic!("a playlist is a project");
        };
        assert_eq!(
            n.links,
            [
                PathBuf::from("Music/Song one.mp3"),
                PathBuf::from("Music/Song two.flac")
            ]
        );
        assert_eq!(n.outside, 1);
        assert!(n.fonts.is_empty());
        // Written with back slashes on Windows, read the same on every system.
        assert_eq!(n.links[0].to_string_lossy(), "Music/Song one.mp3");
    }

    /// ADOPTION change 7: a new kind of project added only as a row works with no code change.
    #[test]
    fn a_kind_added_as_a_row_alone_is_read() {
        let r = rows(
            r#"{"extension":"lst","name":"Shot list","reader":{"kind":"text-lines","comment":";"}},
               {"extension":"scene","name":"Scene","reader":{"kind":"xml-attributes","links":["file"],"fonts":["face"]}}"#,
        );
        let t = tempfile::tempdir().unwrap();
        let lst = t.path().join("a.lst");
        put(&lst, b"; shots\nshot1.mov\n");
        let Found::Project(n) = look(&lst, &r) else {
            panic!()
        };
        assert_eq!((n.kind.as_str(), n.links.len()), ("Shot list", 1));
        let scene = t.path().join("b.scene");
        put(&scene, b"<s file=\"x.obj\" face=\"Lato\"/>");
        let Found::Project(n) = look(&scene, &r) else {
            panic!()
        };
        assert_eq!(n.links, [PathBuf::from("x.obj")]);
        assert_eq!(n.fonts, ["Lato"]);
        // Without its row, the same file is found, not supported.
        assert_eq!(look(&scene, shipped()), Found::NotSupported);
    }

    #[test]
    fn rows_that_cannot_be_read_safely_are_skipped() {
        for bad in [
            r##"{"extension":"SVG","name":"x","reader":{"kind":"text-lines","comment":"#"}}"##,
            r##"{"extension":"","name":"x","reader":{"kind":"text-lines","comment":"#"}}"##,
            r##"{"extension":"svg","name":" ","reader":{"kind":"text-lines","comment":"#"}}"##,
            r##"{"extension":"svg","name":"x","reader":{"kind":"text-lines","comment":""}}"##,
            r##"{"extension":"svg","name":"x","reader":{"kind":"text-lines","comment":"/////"}}"##,
            r##"{"extension":"svg","name":"x","reader":{"kind":"xml-attributes","links":[]}}"##,
            r##"{"extension":"svg","name":"x","reader":{"kind":"xml-attributes","links":["a b"]}}"##,
            r##"{"extension":"svg","name":"x","reader":{"kind":"xml-attributes","links":["a"],"fonts":["f\"x"]}}"##,
        ] {
            assert!(rows(bad).rows().is_empty(), "accepted: {bad}");
        }
        assert_eq!(
            rows(r##"{"extension":"svg","name":"x","reader":{"kind":"xml-attributes","links":[],"fonts":["f"]}}"##)
                .rows()
                .len(),
            1
        );
    }

    #[test]
    fn a_huge_project_is_read_only_so_far_and_said() {
        let t = tempfile::tempdir().unwrap();
        let m3u = t.path().join("big.m3u");
        let mut text = b"a.mp3\n".to_vec();
        text.resize(MAX_PROJECT_BYTES as usize + 10, b'#');
        put(&m3u, &text);
        let Found::Project(n) = look(&m3u, shipped()) else {
            panic!()
        };
        assert!(n.partly_read);
        assert_eq!(n.links, [PathBuf::from("a.mp3")]);
        // The byte past the limit is never read as part of the project.
        let mut cut = vec![b'#'; MAX_PROJECT_BYTES as usize - 1];
        cut.extend_from_slice(b"\nz\n");
        put(&m3u, &cut);
        let Found::Project(c) = look(&m3u, shipped()) else {
            panic!()
        };
        assert!(c.partly_read);
        assert!(c.links.is_empty(), "{:?}", c.links);
        // Exactly at the limit is whole.
        text.truncate(MAX_PROJECT_BYTES as usize);
        put(&m3u, &text);
        let Found::Project(n) = look(&m3u, shipped()) else {
            panic!()
        };
        assert!(!n.partly_read);
    }

    #[test]
    fn links_are_bounded_and_counted() {
        let t = tempfile::tempdir().unwrap();
        let m3u = t.path().join("many.m3u");
        let text: String = (0..MAX_PROJECT_LINKS + 3)
            .map(|i| format!("s{i}.mp3\n"))
            .collect();
        put(&m3u, text.as_bytes());
        let Found::Project(n) = look(&m3u, shipped()) else {
            panic!()
        };
        assert_eq!(n.links.len(), MAX_PROJECT_LINKS);
        assert_eq!(n.more_links, 3);
    }

    #[test]
    fn link_words() {
        assert_eq!(local_link(""), None);
        assert_eq!(local_link("#x"), None);
        assert_eq!(local_link("ftp://x/y"), None);
        assert_eq!(local_link("DATA:x"), None);
        assert_eq!(local_link("mailto:a@b"), None);
        assert_eq!(local_link("?only"), None);
        assert_eq!(local_link("bad%zz.png"), None);
        assert_eq!(local_link("bad%2"), None);
        assert_eq!(
            local_link("%C3%A9t%C3%A9.png"),
            Some(PathBuf::from("été.png"))
        );
        assert!(inside(Path::new("a/b.png")));
        assert!(inside(Path::new("./a.png")));
        assert!(!inside(Path::new("../a.png")));
        assert!(!inside(Path::new("/a.png")));
        assert!(!inside(Path::new("C:/a.png")));
        assert!(!inside(Path::new("")));
        assert_eq!(font_families(" 'A B' , \"C\",, serif,EMOJI"), ["A B", "C"]);
        assert_eq!(text_lines("a\n  \n# c\n b \n", "#"), ["a", "b"]);
        let names = vec!["href".to_string()];
        assert_eq!(
            attribute_values(
                "<a href = \"x\" xhref=\"no\" href=noquote href='y'>",
                &names
            ),
            ["x", "y"]
        );
        assert_eq!(
            attribute_values("<a href=\"unclosed>", &names),
            Vec::<String>::new()
        );
        assert_eq!(
            attribute_values("<a href \"no-equals\">", &names),
            Vec::<String>::new()
        );
        assert_eq!(entities("&lt;&gt;&quot;&apos;&amp;amp;"), "<>\"'&amp;");
    }
}
