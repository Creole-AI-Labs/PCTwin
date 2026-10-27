//! One depth gate in front of every structured-text parser (fourth review, finding F2).
//!
//! The parsers PCTwin uses for text from the old laptop and from catalogues (XML, property lists,
//! JSON, JSON with comments, TOML, GVariant text) read nested values by calling themselves, so a
//! file nested a few thousand levels deep can use up the stack and crash the app before any of
//! their own checks run. [`check`] counts how deep a text nests by reading it once, left to
//! right, with a counter and no recursion, and refuses anything deeper than [`MAX_NESTING`]
//! (256 levels) before a parser ever sees it. The caller reports it as "found, not read: too deeply nested".
//!
//! The counting errs on the side of deeper: brackets inside comments of a format that has none,
//! or a dotted TOML key, count as levels. Real settings files nest well under twenty levels.
//! INI files and dconf dumps are read line by line (never nested); a dconf value is GVariant
//! text and goes through the gate on its own.

/// The deepest nesting any structured text may have before it is read ([`crate::limits`]).
pub use crate::limits::NESTING as MAX_NESTING;

/// The plain words for a text the gate refused.
pub const TOO_DEEP: &str = "found, not read: too deeply nested";

/// A structured-text syntax the gate knows how to count.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Syntax {
    Json,
    /// JSON with `//` and `/* */` comments (and trailing commas).
    Jsonc,
    Toml,
    /// XML, including property lists.
    Xml,
    /// GVariant text (what `dconf` and `gsettings` print).
    GVariant,
}

/// A text nested deeper than [`MAX_NESTING`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[error("found, not read: too deeply nested")]
pub struct TooDeep;

/// Refuses `text` when it nests deeper than [`MAX_NESTING`].
pub fn check(text: &str, syntax: Syntax) -> Result<(), TooDeep> {
    if deeper_than(text.as_bytes(), syntax, MAX_NESTING) {
        Err(TooDeep)
    } else {
        Ok(())
    }
}

/// [`check`] for bytes that may not be text yet (a saved file before it is decoded).
pub fn check_bytes(bytes: &[u8], syntax: Syntax) -> Result<(), TooDeep> {
    if deeper_than(bytes, syntax, MAX_NESTING) {
        Err(TooDeep)
    } else {
        Ok(())
    }
}

/// How deep `text` nests (stops counting just past `limit`).
pub fn depth(text: &str, syntax: Syntax, limit: usize) -> usize {
    let mut d = Depth::new(limit);
    scan(text.as_bytes(), syntax, &mut d);
    d.max
}

/// Whether `b` nests deeper than `limit`.
fn deeper_than(b: &[u8], syntax: Syntax, limit: usize) -> bool {
    let mut d = Depth::new(limit);
    scan(b, syntax, &mut d);
    d.max > limit
}

struct Depth {
    now: usize,
    max: usize,
    limit: usize,
}

impl Depth {
    fn new(limit: usize) -> Self {
        Self {
            now: 0,
            max: 0,
            limit,
        }
    }
    fn open(&mut self, extra: usize) {
        self.now += 1;
        self.max = self.max.max(self.now + extra);
    }
    fn close(&mut self) {
        self.now = self.now.saturating_sub(1);
    }
    /// Counting can stop: the answer is known.
    fn over(&self) -> bool {
        self.max > self.limit
    }
}

fn scan(b: &[u8], syntax: Syntax, d: &mut Depth) {
    match syntax {
        Syntax::Json => json(b, d, false),
        Syntax::Jsonc => json(b, d, true),
        Syntax::Toml => toml(b, d),
        Syntax::Xml => xml(b, d),
        Syntax::GVariant => gvariant(b, d),
    }
}

/// The index just past the string that opens at `i` (quote `q`, backslash escapes).
fn skip_quoted(b: &[u8], mut i: usize, q: u8, escapes: bool) -> usize {
    i += 1;
    while i < b.len() {
        if escapes && b[i] == b'\\' {
            i += 2;
            continue;
        }
        if b[i] == q {
            return i + 1;
        }
        i += 1;
    }
    b.len()
}

/// The index just past the first `end` at or after `i` (or the end of the text).
fn skip_past(b: &[u8], i: usize, end: &[u8]) -> usize {
    b.get(i..)
        .and_then(|rest| rest.windows(end.len()).position(|w| w == end))
        .map(|p| i + p + end.len())
        .unwrap_or(b.len())
}

fn json(b: &[u8], d: &mut Depth, comments: bool) {
    let mut i = 0;
    while i < b.len() && !d.over() {
        match b[i] {
            b'"' => {
                i = skip_quoted(b, i, b'"', true);
                continue;
            }
            b'/' if comments && b.get(i + 1) == Some(&b'/') => {
                i = skip_past(b, i, b"\n");
                continue;
            }
            b'/' if comments && b.get(i + 1) == Some(&b'*') => {
                i = skip_past(b, i + 2, b"*/");
                continue;
            }
            b'[' | b'{' => d.open(0),
            b']' | b'}' => d.close(),
            _ => {}
        }
        i += 1;
    }
}

/// TOML: arrays and inline tables nest, and so do dotted keys (`a.b.c = 1` and `[a.b.c]` make
/// tables inside tables), so every dot in a key counts as a level too. A key under a table
/// header counts from that table's own depth (`[h.h]` then `k.k = 1` is four levels deep: fifth
/// review, finding 10), and an array-of-tables header `[[a.b]]` counts like `[a.b]`.
fn toml(b: &[u8], d: &mut Depth) {
    // What each open bracket is: `true` for an inline table (its commas start keys again).
    let mut open: Vec<bool> = Vec::new();
    let mut key_dots = 0usize;
    let mut in_key = true;
    // Inside a table header, and the depth of the table the last header opened.
    let mut header = false;
    let mut base = 0usize;
    let mut i = 0;
    while i < b.len() && !d.over() {
        let c = b[i];
        match c {
            b'#' => {
                i = skip_past(b, i, b"\n");
                if open.is_empty() {
                    key_dots = 0;
                    in_key = true;
                }
                continue;
            }
            b'"' if b[i..].starts_with(b"\"\"\"") => {
                i = skip_triple(b, i, b"\"\"\"", true);
                continue;
            }
            b'\'' if b[i..].starts_with(b"'''") => {
                i = skip_triple(b, i, b"'''", false);
                continue;
            }
            b'"' => {
                i = skip_quoted(b, i, b'"', true);
                continue;
            }
            b'\'' => {
                i = skip_quoted(b, i, b'\'', false);
                continue;
            }
            b'\n' if open.is_empty() => {
                key_dots = 0;
                in_key = true;
            }
            b'[' if open.is_empty() && in_key && !header => {
                // A table header (`[a.b]`, or `[[a.b]]`, one level for either form): it keeps
                // reading keys, and its dots count from the top.
                if b.get(i + 1) == Some(&b'[') {
                    i += 1;
                }
                header = true;
                key_dots = 0;
                open.push(false);
                d.open(0);
            }
            b']' if header && open.len() == 1 => {
                if b.get(i + 1) == Some(&b']') {
                    i += 1;
                }
                open.pop();
                d.close();
                header = false;
                // The keys that follow are inside the table the header named.
                base = key_dots + 1;
                in_key = false;
            }
            b'[' => {
                // An array holds values.
                open.push(false);
                d.open(base + key_dots);
                in_key = false;
            }
            b'{' => {
                open.push(true);
                d.open(base + key_dots);
                in_key = true;
            }
            b']' | b'}' => {
                open.pop();
                d.close();
                in_key = false;
            }
            b'=' => in_key = false,
            b',' if open.last() == Some(&true) => in_key = true,
            b'.' if in_key => {
                key_dots += 1;
                let from = if header { 0 } else { base };
                d.max = d.max.max(d.now + from + key_dots);
            }
            _ => {}
        }
        i += 1;
    }
}

fn skip_triple(b: &[u8], i: usize, q: &[u8], escapes: bool) -> usize {
    let mut j = i + q.len();
    while j < b.len() {
        if escapes && b[j] == b'\\' {
            j += 2;
            continue;
        }
        if b[j..].starts_with(q) {
            return j + q.len();
        }
        j += 1;
    }
    b.len()
}

/// XML: an element opens a level until its end tag; comments, character data, processing
/// instructions and the document type declaration are skipped whole.
fn xml(b: &[u8], d: &mut Depth) {
    let mut i = 0;
    while i < b.len() && !d.over() {
        if b[i] != b'<' {
            i += 1;
            continue;
        }
        let rest = &b[i..];
        if rest.starts_with(b"<!--") {
            i = skip_past(b, i + 4, b"-->");
        } else if rest.starts_with(b"<![CDATA[") {
            i = skip_past(b, i + 9, b"]]>");
        } else if rest.starts_with(b"<?") {
            i = skip_past(b, i + 2, b"?>");
        } else if rest.starts_with(b"<!") {
            i = skip_declaration(b, i);
        } else if rest.starts_with(b"</") {
            d.close();
            i = skip_past(b, i + 2, b">");
        } else {
            // A start tag: to its `>`, attribute values skipped; `/>` closes it at once.
            let mut j = i + 1;
            let mut empty = false;
            while j < b.len() {
                match b[j] {
                    b'"' | b'\'' => {
                        j = skip_quoted(b, j, b[j], false);
                        continue;
                    }
                    b'>' => {
                        empty = b[j - 1] == b'/';
                        break;
                    }
                    _ => {}
                }
                j += 1;
            }
            if !empty {
                d.open(0);
            }
            i = j + 1;
        }
    }
}

/// The index just past a declaration (`<!DOCTYPE ...>`) that opens at `i`, read as XML reads
/// it: quoted literals whole; an internal subset in brackets, inside which comments, processing
/// instructions and each markup declaration (its own quoted literals whole) are skipped. A quote
/// inside a comment in the subset never hides what follows.
fn skip_declaration(b: &[u8], i: usize) -> usize {
    let mut j = i + 2;
    let mut subset = false;
    while j < b.len() {
        let rest = &b[j..];
        if subset && rest.starts_with(b"<!--") {
            j = skip_past(b, j + 4, b"-->");
            continue;
        }
        if subset && rest.starts_with(b"<?") {
            j = skip_past(b, j + 2, b"?>");
            continue;
        }
        if subset && rest.starts_with(b"<!") {
            // One markup declaration inside the subset, to its `>`.
            let mut k = j + 2;
            while k < b.len() && b[k] != b'>' {
                k = if matches!(b[k], b'"' | b'\'') {
                    skip_quoted(b, k, b[k], false)
                } else {
                    k + 1
                };
            }
            j = k + 1;
            continue;
        }
        match b[j] {
            b'"' | b'\'' => {
                j = skip_quoted(b, j, b[j], false);
                continue;
            }
            b'[' if !subset => subset = true,
            b']' if subset => subset = false,
            b'>' if !subset => return j + 1,
            _ => {}
        }
        j += 1;
    }
    b.len()
}

fn gvariant(b: &[u8], d: &mut Depth) {
    let mut i = 0;
    while i < b.len() && !d.over() {
        match b[i] {
            b'\'' | b'"' => {
                i = skip_quoted(b, i, b[i], true);
                continue;
            }
            b'[' | b'(' | b'{' | b'<' => d.open(0),
            b']' | b')' | b'}' | b'>' => d.close(),
            _ => {}
        }
        i += 1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn nest(open: &str, close: &str, n: usize, inner: &str) -> String {
        format!("{}{inner}{}", open.repeat(n), close.repeat(n))
    }

    #[test]
    fn each_syntax_counts_its_own_nesting() {
        assert_eq!(depth(&nest("[", "]", 7, "1"), Syntax::Json, 1000), 7);
        assert_eq!(depth(&nest("{\"a\":", "}", 5, "1"), Syntax::Json, 1000), 5);
        assert_eq!(depth(&nest("<a>", "</a>", 9, "v"), Syntax::Xml, 1000), 9);
        assert_eq!(depth(&nest("[", "]", 4, "'x'"), Syntax::GVariant, 1000), 4);
        assert_eq!(depth(&nest("(", ")", 3, "'x'"), Syntax::GVariant, 1000), 3);
        assert_eq!(depth(&nest("<", ">", 6, "'x'"), Syntax::GVariant, 1000), 6);
        assert_eq!(
            depth(
                &format!("a = {}\n", nest("[", "]", 6, "1")),
                Syntax::Toml,
                1000
            ),
            6
        );
        assert_eq!(
            depth(
                &format!("a = {}\n", nest("{b=", "}", 4, "1")),
                Syntax::Toml,
                1000
            ),
            4
        );
        // Fifth review, finding 10: a key under a header counts from the header's table, and an
        // array-of-tables header counts like a table header.
        assert_eq!(depth("[h.h]\nk.k = 1\n", Syntax::Toml, 1000), 3);
        assert_eq!(depth("[h.h]\nk.k.k = {a = 1}\n", Syntax::Toml, 1000), 5);
        assert_eq!(depth("[[a.b.c]]\nx = 1\n", Syntax::Toml, 1000), 3);
        assert_eq!(depth("[[a]]\n[[a]]\nx.y = 1\n", Syntax::Toml, 1000), 2);
        let aot = format!("[[{}]]\nx = 1\n", vec!["k"; 20_000].join("."));
        assert_eq!(check(&aot, Syntax::Toml), Err(TooDeep));
        let split = format!(
            "[{}]\n{} = 1\n",
            vec!["h"; 200].join("."),
            vec!["k"; 200].join(".")
        );
        assert_eq!(check(&split, Syntax::Toml), Err(TooDeep));
        let fits = format!(
            "[{}]\n{} = 1\n",
            vec!["h"; 100].join("."),
            vec!["k"; 100].join(".")
        );
        assert_eq!(check(&fits, Syntax::Toml), Ok(()));
        // Dotted keys and headers make tables inside tables.
        assert_eq!(depth("a.b.c.d = 1\n", Syntax::Toml, 1000), 3);
        assert_eq!(depth("[a.b.c]\nx = 1\n", Syntax::Toml, 1000), 3);
        // Dots in values are not keys.
        assert_eq!(depth("a = [1.5, 2.5, 3.5]\n", Syntax::Toml, 1000), 1);
        assert_eq!(depth("a = 1.5\nb = 2\n", Syntax::Toml, 1000), 0);
    }

    #[test]
    fn strings_comments_and_declarations_do_not_count() {
        assert_eq!(depth(r#"{"a":"[[[[{{{{\"[["}"#, Syntax::Json, 1000), 1);
        assert_eq!(depth("// [[[[\n/* {{{{ */ [1]", Syntax::Jsonc, 1000), 1);
        // Plain JSON has no comments: brackets in one count (erring on the deeper side).
        assert_eq!(depth("/* [[[ */ [1]", Syntax::Json, 1000), 4);
        assert_eq!(
            depth(
                "<?xml version=\"1.0\"?><!DOCTYPE p [<!ENTITY x \"<a><a>\">]><!-- <a><a><a> --><p a=\"<b>\"><![CDATA[<c><c>]]><e/><e/></p>",
                Syntax::Xml,
                1000
            ),
            1
        );
        assert_eq!(
            depth(
                "a = \"[[[[\" # [[[[\nb = '''[[\n[[''' \n",
                Syntax::Toml,
                1000
            ),
            0
        );
        assert_eq!(depth("['[[[', \"((\\\"((\"]", Syntax::GVariant, 1000), 1);
        // Closing more than was opened never goes below nothing.
        assert_eq!(depth("]]]]][", Syntax::Json, 1000), 1);
        assert_eq!(depth("</a></a><a>", Syntax::Xml, 1000), 1);
    }

    #[test]
    fn the_gate_refuses_past_the_limit_and_allows_the_limit() {
        let at = nest("[", "]", MAX_NESTING, "1");
        let past = nest("[", "]", MAX_NESTING + 1, "1");
        assert_eq!(check(&at, Syntax::Json), Ok(()));
        assert_eq!(check(&past, Syntax::Json), Err(TooDeep));
        let x_at = nest("<a>", "</a>", MAX_NESTING, "v");
        let x_past = nest("<a>", "</a>", MAX_NESTING + 1, "v");
        assert_eq!(check(&x_at, Syntax::Xml), Ok(()));
        assert_eq!(check(&x_past, Syntax::Xml), Err(TooDeep));
        let keys = format!("{} = 1\n", vec!["k"; MAX_NESTING + 2].join("."));
        assert_eq!(check(&keys, Syntax::Toml), Err(TooDeep));
        assert_eq!(TooDeep.to_string(), TOO_DEEP);
    }

    #[test]
    fn a_huge_nesting_is_counted_without_recursion_and_stops_early() {
        // Far past what any recursive parser survives on a small stack.
        let deep = nest("<a>", "</a>", 500_000, "v");
        let t = std::thread::Builder::new()
            .stack_size(64 * 1024)
            .spawn(move || check(&deep, Syntax::Xml))
            .unwrap();
        assert_eq!(t.join().unwrap(), Err(TooDeep));
    }

    #[test]
    fn unclosed_text_ends_the_count_without_a_panic() {
        for s in [
            "\"abc",
            "/* x",
            "<!-- x",
            "\"\"\"",
            "<!DOCTYPE [<!x",
            "<!DOCTYPE [<!-- ",
            "<![CDATA[",
            "<a b=\"",
            "'''",
            "<!DOCTYPE [",
            "\\",
        ] {
            for syn in [
                Syntax::Json,
                Syntax::Jsonc,
                Syntax::Toml,
                Syntax::Xml,
                Syntax::GVariant,
            ] {
                let _ = depth(s, syn, 10);
            }
        }
    }

    /// What comes after every construct the gate skips is still counted: a string, a comment, a
    /// declaration can never hide the nesting behind it (the mutation tests of the fourth-review
    /// fix: each skip that stopped the count early survived before these).
    #[test]
    fn nothing_skipped_hides_the_nesting_after_it() {
        let deep = |o: &str, c: &str| format!("{}1{}", o.repeat(300), c.repeat(300));
        let json_deep = deep("[", "]");
        for (syn, before) in [
            (Syntax::Json, r#"{"a":"x\"]]\\","b":"#),
            (Syntax::Json, r#"["\\", "#),
            (Syntax::Jsonc, "// ]]]\n["),
            (Syntax::Jsonc, "/* ]]] */ ["),
            (Syntax::Jsonc, r#"["a\"b", "#),
        ] {
            let t = format!("{before}{json_deep}");
            assert_eq!(check(&t, syn), Err(TooDeep), "{syn:?} {before}");
        }
        let toml_deep = deep("[", "]");
        for before in [
            r#"a = "x\"]]" # ]]]
b = "#,
            "a = 'x' # c\nb = ",
            "# c\nb = ",
            "a = \"\"\"\n]]]\n\"\"\"\nb = ",
            "a = '''\n]]]\n'''\nb = ",
            r"a = 'a\'
b = ",
            r"s = '''a\'''
t = ",
            "[t]\nk = ",
            "x = 1\ny = ",
            r#"a = """x\y"""
b = "#,
        ] {
            let t = format!("{before}{toml_deep}\n");
            assert_eq!(check(&t, Syntax::Toml), Err(TooDeep), "{before:?}");
        }
        let keys = vec!["k"; 300].join(".");
        for t in [
            format!("x = 1\n{keys} = 1\n"),
            format!("# c\n{keys} = 1\n"),
            format!("x = {{a = 1, {keys} = 1}}\n"),
            format!("x = {{{keys} = 1}}\n"),
            format!("[{keys}]\n"),
            format!(
                "{} = {}1{}\n",
                vec!["k"; 200].join("."),
                "[".repeat(100),
                "]".repeat(100)
            ),
        ] {
            assert_eq!(check(&t, Syntax::Toml), Err(TooDeep), "{t:.40}");
        }
        let xml_deep = deep("<a>", "</a>");
        for before in [
            "text",
            "<!-- </a></a> -->",
            "<![CDATA[</a></a>]]>",
            "<?pi </a> ?>",
            r#"<!DOCTYPE r [<!-- " --><!ENTITY e "]>"><?p ]> ?>]>"#,
            r#"<!DOCTYPE r PUBLIC "a>b" 'c>d'>"#,
            r#"<r x="/>" y='</a>'/>"#,
            // XML has no escapes in a quoted value: a backslash never hides its closing quote.
            r#"<r a="\"/>"#,
        ] {
            let t = format!("{before}{xml_deep}");
            assert_eq!(check(&t, Syntax::Xml), Err(TooDeep), "{before}");
        }
        // A quote inside the internal subset's comments, instructions and declarations, or in an
        // external ID, never runs on to a quote in the text after the nesting.
        for before in [
            r#"<!DOCTYPE r [<!-- " -->]>"#,
            r#"<!DOCTYPE r [<?p " ?>]>"#,
            r#"<!DOCTYPE r [<!ENTITY e "\">]>"#,
            r#"<!DOCTYPE r SYSTEM "\">"#,
        ] {
            let t = format!("{before}{xml_deep}\"");
            assert_eq!(check(&t, Syntax::Xml), Err(TooDeep), "{before}");
        }
        for before in [r"'x\'[' ", r#""y" "#, "('a', "] {
            let t = format!("{before}{}", deep("[", "]"));
            assert_eq!(check(&t, Syntax::GVariant), Err(TooDeep), "{before}");
        }
    }

    /// And legitimate text whose strings and comments hold brackets is never refused.
    #[test]
    fn brackets_inside_strings_and_comments_never_count() {
        let open = "[".repeat(300);
        let tags = "<a>".repeat(300);
        let floats = vec!["1.5"; 300].join(", ");
        for (syn, t) in [
            (Syntax::Json, format!(r#"{{"a":"{open}\"{open}"}}"#)),
            (Syntax::Jsonc, format!("// {open}\n/* {open} */ [1]")),
            (Syntax::Toml, format!(r#"a = "{open}\"{open}""#)),
            (Syntax::Toml, format!(r#"a = """{open}\"""{open}""""#)),
            (
                Syntax::Toml,
                format!("a = '''{open}'''\nb = '{open}' # {open}\n"),
            ),
            (Syntax::Toml, format!("a = [[1], {floats}]\n")),
            (Syntax::Toml, format!("[a]\nb = {{c = [{floats}]}}\n")),
            (
                Syntax::Xml,
                format!("<r a=\"{tags}\"><!-- {tags} --><![CDATA[{tags}]]></r>"),
            ),
            (Syntax::GVariant, format!(r#"['{open}', "{open}\"{open}"]"#)),
            // Many empty elements with attributes side by side are no nesting at all.
            (
                Syntax::Xml,
                format!("<r>{}</r>", r#"<e x="1" y='2'/>"#.repeat(300)),
            ),
        ] {
            assert_eq!(check(&t, syn), Ok(()), "{syn:?} {t:.60}");
        }
        // A start tag the text ends inside still counts as a level.
        assert_eq!(depth("<a><a", Syntax::Xml, 10), 2);
        assert_eq!(depth("<a><a x='1'", Syntax::Xml, 10), 2);
    }
}
