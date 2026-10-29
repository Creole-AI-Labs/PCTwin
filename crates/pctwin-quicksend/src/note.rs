//! A note or a link (Quick Send plan section 5, Stage 1): a few typed lines (an address, a Wi-Fi
//! password, a PIN) or one web address, sent deliberately over the sealed link and shown on the
//! other laptop with Copy. No file is made for either. Nothing in a note is ever opened or run: a
//! link is shown as text, with the site it leads to beside it, and opens only when the person
//! there chooses Open.
//!
//! Both laptops check a note by the same rules ([`Note::new`]): it is not empty, at most
//! [`MAX_NOTE_CHARS`] characters on at most [`MAX_NOTE_LINES`] lines, and holds no control
//! characters (only the line break) and no characters that hide or reorder text (direction
//! overrides, isolates and marks, zero-width spaces, the byte-order mark), so what the receiver
//! sees is what the sender typed. Joiners stay: some scripts and emoji need them.

/// Most characters in one note.
pub const MAX_NOTE_CHARS: usize = 1000;
/// Most lines in one note.
pub const MAX_NOTE_LINES: usize = 20;
/// Most bytes of one note on the wire (four per character at most).
pub const MAX_NOTE_BYTES: usize = MAX_NOTE_CHARS * 4;

/// Why a note cannot be sent (or was refused on arrival).
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum NoteError {
    #[error("the note is empty")]
    Empty,
    #[error("the note is longer than {MAX_NOTE_CHARS} characters")]
    TooLong,
    #[error("the note has more than {MAX_NOTE_LINES} lines")]
    TooManyLines,
    /// A control character (other than a line break) or one that hides or reorders text.
    #[error("the note holds a character that cannot be shown safely")]
    Hidden,
}

/// What a note is, as the receiving screen shows it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NoteKind {
    /// Typed lines, shown with Copy.
    Text,
    /// One web address (`http` or `https`), shown as text with Copy, the site it leads to
    /// (`host`, in plain letters: a name in another script is shown in its `xn--` form, so it
    /// cannot pass for a familiar one), and Open, which only the person chooses.
    Link { host: String },
}

/// A checked note.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Note {
    text: String,
    kind: NoteKind,
}

/// Characters that hide or reorder text: never in a note.
fn hides_text(c: char) -> bool {
    matches!(
        c,
        '\u{061C}'
            | '\u{180E}'
            | '\u{200B}'
            | '\u{200E}'
            | '\u{200F}'
            | '\u{202A}'..='\u{202E}'
            | '\u{2060}'..='\u{2064}'
            | '\u{2066}'..='\u{2069}'
            | '\u{FEFF}'
            | '\u{FFF9}'..='\u{FFFB}'
    )
}

impl Note {
    /// The note the person typed: Windows line breaks become plain ones and blank space at the
    /// start and end is dropped; then the rules above. A note that is a single web address
    /// becomes a link.
    pub fn new(typed: &str) -> Result<Self, NoteError> {
        let text = typed.replace("\r\n", "\n");
        Self::checked(text.trim())
    }

    /// A note exactly as it came over the link: the same rules, with nothing tidied (a sender
    /// that skipped them is refused, never silently corrected).
    pub fn arrived(text: &str) -> Result<Self, NoteError> {
        if text.trim() != text {
            return Err(NoteError::Hidden);
        }
        Self::checked(text)
    }

    fn checked(text: &str) -> Result<Self, NoteError> {
        if text.is_empty() {
            return Err(NoteError::Empty);
        }
        if text.chars().count() > MAX_NOTE_CHARS {
            return Err(NoteError::TooLong);
        }
        if text.split('\n').count() > MAX_NOTE_LINES {
            return Err(NoteError::TooManyLines);
        }
        if text
            .chars()
            .any(|c| (c.is_control() && c != '\n') || hides_text(c))
        {
            return Err(NoteError::Hidden);
        }
        let kind = link_host(text).map_or(NoteKind::Text, |host| NoteKind::Link { host });
        Ok(Self {
            text: text.to_string(),
            kind,
        })
    }

    /// The note's text, as typed.
    pub fn text(&self) -> &str {
        &self.text
    }

    pub fn kind(&self) -> &NoteKind {
        &self.kind
    }

    /// The web address to open, only for a link: what Open hands to the system's browser when
    /// the person chooses it.
    pub fn link(&self) -> Option<&str> {
        matches!(self.kind, NoteKind::Link { .. }).then_some(self.text.as_str())
    }
}

/// The site a note leads to, when the whole note is one web address: `http` or `https`, with a
/// site name, and no user name or password before it (`https://bank.example@other.example` would
/// show one site and open another): such a note stays plain text.
fn link_host(text: &str) -> Option<String> {
    if text.chars().any(char::is_whitespace) {
        return None;
    }
    let lower = text.get(..8).unwrap_or(text).to_ascii_lowercase();
    if !(lower.starts_with("https://") || lower.starts_with("http://")) {
        return None;
    }
    let url = url::Url::parse(text).ok()?;
    if !matches!(url.scheme(), "http" | "https") || !url.username().is_empty() {
        return None;
    }
    if url.password().is_some() {
        return None;
    }
    let host = url.host_str()?;
    (!host.is_empty()).then(|| host.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_few_typed_lines_are_a_note() {
        let n = Note::new("  Wi-Fi: Home\r\nPassword: été-2026 \n").unwrap();
        assert_eq!(n.text(), "Wi-Fi: Home\nPassword: été-2026");
        assert_eq!(n.kind(), &NoteKind::Text);
        assert_eq!(n.link(), None);
    }

    #[test]
    fn an_empty_note_is_refused() {
        assert_eq!(Note::new(""), Err(NoteError::Empty));
        assert_eq!(Note::new(" \n\t \r\n"), Err(NoteError::Empty));
    }

    #[test]
    fn a_note_has_a_size_limit_in_characters_and_lines() {
        assert!(Note::new(&"a".repeat(MAX_NOTE_CHARS)).is_ok());
        assert_eq!(
            Note::new(&"a".repeat(MAX_NOTE_CHARS + 1)),
            Err(NoteError::TooLong)
        );
        // Characters, not bytes: a thousand four-byte characters fit.
        assert!(Note::new(&"\u{1F600}".repeat(MAX_NOTE_CHARS)).is_ok());
        let lines = vec!["x"; MAX_NOTE_LINES].join("\n");
        assert!(Note::new(&lines).is_ok());
        let more = vec!["x"; MAX_NOTE_LINES + 1].join("\n");
        assert_eq!(Note::new(&more), Err(NoteError::TooManyLines));
    }

    #[test]
    fn control_and_hiding_characters_are_refused() {
        for bad in [
            "a\tb",
            "a\u{0007}b",
            "a\rb",
            "a\u{0085}b",
            "invoice\u{202E}fdp.exe",
            "a\u{2066}b",
            "a\u{200B}b",
            "a\u{200E}b",
            "a\u{FEFF}b",
            "a\u{061C}b",
            "a\u{2060}b",
            "a\u{FFF9}b",
            "a\u{180E}b",
        ] {
            assert_eq!(Note::new(bad), Err(NoteError::Hidden), "{bad:?}");
        }
        // Joiners stay: scripts and emoji need them.
        assert!(Note::new("\u{1F469}\u{200D}\u{1F4BB} \u{0645}\u{200C}\u{06CC}").is_ok());
    }

    #[test]
    fn a_single_web_address_is_a_link_with_its_site() {
        let n = Note::new("https://example.org/path?q=1").unwrap();
        assert_eq!(
            n.kind(),
            &NoteKind::Link {
                host: "example.org".into()
            }
        );
        assert_eq!(n.link(), Some("https://example.org/path?q=1"));
        assert!(Note::new("HTTP://Example.org").unwrap().link().is_some());
        // A name in another script shows in its plain-letter form.
        let idn = Note::new("https://\u{0440}\u{0430}y\u{0440}al.com/").unwrap();
        match idn.kind() {
            NoteKind::Link { host } => assert!(host.starts_with("xn--"), "{host}"),
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn anything_else_stays_text_and_never_opens() {
        for text in [
            "see https://example.org",
            "javascript:alert(1)",
            "file:///C:/Windows/System32/calc.exe",
            "ftp://example.org/",
            "https://bank.example@other.example/",
            "https://user:pw@example.org/",
            "https://",
            "example.org",
            "https://example.org/\nsecond line",
        ] {
            let n = Note::new(text).unwrap();
            assert_eq!(n.kind(), &NoteKind::Text, "{text}");
            assert_eq!(n.link(), None, "{text}");
        }
    }

    #[test]
    fn a_note_from_the_other_laptop_is_checked_and_never_tidied() {
        assert!(Note::arrived("hello\nthere").is_ok());
        assert_eq!(Note::arrived(" hello"), Err(NoteError::Hidden));
        assert_eq!(Note::arrived("hello\n"), Err(NoteError::Hidden));
        assert_eq!(Note::arrived("a\r\nb"), Err(NoteError::Hidden));
        assert_eq!(Note::arrived(""), Err(NoteError::Empty));
        assert_eq!(
            Note::arrived("invoice\u{202E}fdp.exe"),
            Err(NoteError::Hidden)
        );
    }
}
