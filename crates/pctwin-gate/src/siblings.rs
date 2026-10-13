//! The visible name a file is kept under beside its own, when undo cannot put it back under its
//! name (another file took it meanwhile) or saves bytes written while it was being removed.
//!
//! The name is the file's stem, the words the app passes in from its reviewed catalog in the
//! person's language (" (kept by PCTwin undo)", say), a number from the second try on, and the
//! file's last extension: `report (kept by PCTwin undo) 2.docx`. Names stay within 255 bytes,
//! the limit of every drive PCTwin writes to, by shortening the stem only, and only between
//! whole characters. Whether a name is free is never checked first: the caller moves the file
//! there without replacing anything and tries the next number if the name is taken, which also
//! settles names a drive treats as the same (differing only in case or accents).

/// The most bytes in one name on the drives PCTwin writes to.
pub const NAME_BYTES: usize = 255;

/// The most numbers tried before giving up on a visible name.
pub const MOST_TRIES: u32 = 99;

/// The `n`th visible name (from 1) for keeping `name` beside itself, with the catalog words
/// `words`. `room_for_words` is the byte length of the longest words in any of the app's
/// languages, so a name is cut the same way whatever language is chosen (at least `words`).
pub fn sibling_name(name: &str, words: &str, n: u32, room_for_words: usize) -> String {
    let (stem, ext) = split_extension(name);
    let number = if n <= 1 {
        String::new()
    } else {
        format!(" {n}")
    };
    let room_for_number = format!(" {MOST_TRIES}").len();
    let fixed = room_for_words.max(words.len()) + room_for_number + ext.len();
    let budget = NAME_BYTES.saturating_sub(fixed);
    let stem = cut_at_char(stem, budget);
    format!("{stem}{words}{number}{ext}")
}

/// The stem and the last extension (with its dot). A name that only starts with a dot
/// (`.bashrc`) has no extension; an extension too long to leave room for the rest is treated as
/// part of the stem.
fn split_extension(name: &str) -> (&str, &str) {
    match name.rfind('.') {
        Some(0) | None => (name, ""),
        Some(i) if name.len() - i > 32 => (name, ""),
        Some(i) => name.split_at(i),
    }
}

/// The longest start of `s` within `bytes` bytes that ends between whole characters.
fn cut_at_char(s: &str, bytes: usize) -> &str {
    if s.len() <= bytes {
        return s;
    }
    let mut end = bytes;
    while !s.is_char_boundary(end) {
        end -= 1;
    }
    &s[..end]
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The words in each of the seven launch languages (stand-ins for the reviewed catalog).
    const WORDS: [&str; 7] = [
        " (kept by PCTwin undo)",
        " (conservé par l'annulation PCTwin)",
        " (conservado al deshacer en PCTwin)",
        " (von PCTwin-Rückgängig behalten)",
        " (mantido pelo desfazer do PCTwin)",
        " (محفوظ بواسطة تراجع PCTwin)",
        " (PCTwin 撤销时保留)",
    ];

    fn longest() -> usize {
        WORDS.iter().map(|w| w.len()).max().unwrap()
    }

    #[test]
    fn the_name_is_stem_words_number_and_extension() {
        let w = WORDS[0];
        assert_eq!(
            sibling_name("report.docx", w, 1, longest()),
            "report (kept by PCTwin undo).docx"
        );
        assert_eq!(
            sibling_name("report.docx", w, 2, longest()),
            "report (kept by PCTwin undo) 2.docx"
        );
        assert_eq!(
            sibling_name("archive.tar.gz", w, 1, longest()),
            "archive.tar (kept by PCTwin undo).gz"
        );
        assert_eq!(
            sibling_name(".bashrc", w, 1, longest()),
            ".bashrc (kept by PCTwin undo)"
        );
        assert_eq!(
            sibling_name("README", w, 3, longest()),
            "README (kept by PCTwin undo) 3"
        );
    }

    #[test]
    fn every_language_stays_within_255_bytes_and_cuts_between_characters() {
        let stems = [
            "a".repeat(300),
            "é".repeat(200),
            "文".repeat(120),
            "😀".repeat(80),
            "ب".repeat(150),
        ];
        for words in WORDS {
            for stem in &stems {
                for ext in ["", ".txt", ".docx"] {
                    let name = format!("{stem}{ext}");
                    for n in [1, 2, MOST_TRIES] {
                        let s = sibling_name(&name, words, n, longest());
                        assert!(s.len() <= NAME_BYTES, "{} bytes", s.len());
                        assert!(s.ends_with(ext), "{s}");
                        assert!(s.contains(words.trim_start()), "{s}");
                    }
                }
            }
        }
    }

    #[test]
    fn a_short_name_is_never_cut_and_the_cut_is_the_same_in_every_language() {
        let name = "x".repeat(250) + ".txt";
        let cuts: Vec<usize> = WORDS
            .iter()
            .map(|w| sibling_name(&name, w, 1, longest()).find(w).unwrap())
            .collect();
        assert!(cuts.windows(2).all(|p| p[0] == p[1]), "{cuts:?}");
        assert_eq!(
            sibling_name("notes.txt", WORDS[6], 1, longest()),
            "notes (PCTwin 撤销时保留).txt"
        );
    }

    #[test]
    fn a_long_extension_counts_as_stem() {
        let name = format!("data.{}", "x".repeat(40));
        let s = sibling_name(&name, WORDS[0], 1, longest());
        assert!(s.ends_with(" (kept by PCTwin undo)"), "{s}");
    }
}
