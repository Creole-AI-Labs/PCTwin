//! Comparing app versions, so PCTwin never installs an older version than the person had
//! (the reinstall research, B4: Installomator compares only for equality and would downgrade).
//!
//! App versions are not semantic versions: "120.0.6099.71", "2.4.1-beta.3", "1.0 (build 57)",
//! "8.2.3 Build 1234", "2025.1", "v3.1". The rule is general, never per app:
//!
//! - the version is split into runs of digits and runs of letters; separators (`.`, `-`, `_`,
//!   `+`, spaces, brackets) only split;
//! - number runs compare as numbers (so 10 is after 9, and 007 equals 7), letter runs compare
//!   case-insensitively;
//! - a missing trailing number counts as 0 ("1.2" equals "1.2.0");
//! - a pre-release word (alpha, beta, rc, pre, preview, dev, nightly, canary, insider) right after
//!   the numbers makes the version come *before* the same numbers without it ("2.0-beta" is
//!   before "2.0");
//! - two versions whose shapes cannot be lined up (a number where the other has a word that is
//!   not a pre-release word) are [`Order::Unknown`]: the caller treats that as "cannot prove it is
//!   not older" and never installs on it silently.

use std::cmp::Ordering;

/// How a candidate version stands against the version the person had.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Order {
    Older,
    Same,
    Newer,
    /// The two cannot be compared by the general rule.
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum Part {
    Num(u128),
    Word(String),
}

const PRE_RELEASE: &[&str] = &[
    "alpha", "a", "beta", "b", "rc", "pre", "preview", "dev", "nightly", "canary", "insider",
    "snapshot", "test",
];

/// The longest version text compared; anything longer is not a version.
const MAX_LEN: usize = 128;

fn parts(v: &str) -> Option<Vec<Part>> {
    let v = v.trim();
    if v.is_empty() || v.len() > MAX_LEN {
        return None;
    }
    let v = v
        .strip_prefix('v')
        .or_else(|| v.strip_prefix('V'))
        .filter(|rest| rest.starts_with(|c: char| c.is_ascii_digit()))
        .unwrap_or(v);
    let mut out = Vec::new();
    let mut chars = v.chars().peekable();
    while let Some(&c) = chars.peek() {
        if c.is_ascii_digit() {
            let mut n: u128 = 0;
            while let Some(&d) = chars.peek() {
                let Some(digit) = d.to_digit(10) else { break };
                n = n.checked_mul(10)?.checked_add(u128::from(digit))?;
                chars.next();
            }
            out.push(Part::Num(n));
        } else if c.is_alphabetic() {
            let mut w = String::new();
            while let Some(&d) = chars.peek() {
                if !d.is_alphabetic() {
                    break;
                }
                w.extend(d.to_lowercase());
                chars.next();
            }
            out.push(Part::Word(w));
        } else if matches!(
            c,
            '.' | '-' | '_' | '+' | ' ' | '(' | ')' | '[' | ']' | ',' | '~'
        ) {
            chars.next();
        } else {
            // Anything else is not part of a version PCTwin can read.
            return None;
        }
    }
    if out.first().is_none_or(|p| !matches!(p, Part::Num(_))) {
        return None;
    }
    // "build" before a number is a separator word: "1.0 (build 57)" is 1.0.57.
    out.retain(|p| !matches!(p, Part::Word(w) if w == "build"));
    Some(out)
}

fn is_pre(p: &Part) -> bool {
    matches!(p, Part::Word(w) if PRE_RELEASE.contains(&w.as_str()))
}

/// How `candidate` stands against `had`.
pub fn compare(candidate: &str, had: &str) -> Order {
    let (Some(a), Some(b)) = (parts(candidate), parts(had)) else {
        return if candidate.trim() == had.trim() && !candidate.trim().is_empty() {
            Order::Same
        } else {
            Order::Unknown
        };
    };
    let mut i = 0;
    loop {
        match (a.get(i), b.get(i)) {
            (None, None) => return Order::Same,
            // One ran out: a pre-release word on the longer side makes it older; a number counts
            // against an implied 0; any other word cannot be lined up.
            (None, Some(p)) | (Some(p), None) => {
                let longer_is_candidate = a.len() > b.len();
                let rest = if longer_is_candidate {
                    &a[i..]
                } else {
                    &b[i..]
                };
                let o = if is_pre(p) {
                    Ordering::Less
                } else if rest.iter().all(|p| matches!(p, Part::Num(0))) {
                    Ordering::Equal
                } else if matches!(p, Part::Num(_)) {
                    Ordering::Greater
                } else {
                    return Order::Unknown;
                };
                let o = if longer_is_candidate { o } else { o.reverse() };
                return order(o);
            }
            (Some(Part::Num(x)), Some(Part::Num(y))) => {
                if x != y {
                    return order(x.cmp(y));
                }
            }
            (Some(Part::Word(x)), Some(Part::Word(y))) => {
                if x != y {
                    return match (is_pre(&a[i]), is_pre(&b[i])) {
                        (true, true) => order(rank(x).cmp(&rank(y)).then_with(|| x.cmp(y))),
                        _ => Order::Unknown,
                    };
                }
            }
            // A pre-release word against a number: the word side is older ("2.0-beta" against
            // "2.0.1" is decided earlier; here "2.0-beta" against "2.0.0").
            (Some(p @ Part::Word(_)), Some(Part::Num(_))) if is_pre(p) => return Order::Older,
            (Some(Part::Num(_)), Some(p @ Part::Word(_))) if is_pre(p) => return Order::Newer,
            _ => return Order::Unknown,
        }
        i += 1;
    }
}

/// Pre-release words in their usual order.
fn rank(w: &str) -> u8 {
    match w {
        "dev" | "nightly" | "canary" | "snapshot" | "test" => 0,
        "alpha" | "a" => 1,
        "beta" | "b" | "insider" | "preview" => 2,
        "pre" | "rc" => 3,
        _ => 4,
    }
}

fn order(o: Ordering) -> Order {
    match o {
        Ordering::Less => Order::Older,
        Ordering::Equal => Order::Same,
        Ordering::Greater => Order::Newer,
    }
}

/// Whether installing `candidate` keeps the person on their version or newer. `None` for the
/// candidate's version means the route does not say which version it brings: never proven.
pub fn not_a_downgrade(candidate: Option<&str>, had: Option<&str>) -> bool {
    match (candidate, had) {
        // The old laptop recorded no version: there is nothing to go below.
        (_, None) => true,
        (None, Some(_)) => false,
        (Some(c), Some(h)) => matches!(compare(c, h), Order::Same | Order::Newer),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn numbers_compare_as_numbers() {
        assert_eq!(compare("1.10", "1.9"), Order::Newer);
        assert_eq!(compare("1.9", "1.10"), Order::Older);
        assert_eq!(compare("1.2", "1.2.0"), Order::Same);
        assert_eq!(compare("1.2.0.0", "1.2"), Order::Same);
        assert_eq!(compare("1.2.1", "1.2"), Order::Newer);
        assert_eq!(compare("1.2", "1.2.1"), Order::Older);
        assert_eq!(compare("007", "7"), Order::Same);
        assert_eq!(compare("120.0.6099.71", "120.0.6099.109"), Order::Older);
        assert_eq!(compare("v3.1", "3.0"), Order::Newer);
        assert_eq!(compare("2025.1", "2024.12"), Order::Newer);
        assert_eq!(compare("1.0 (build 57)", "1.0 (build 56)"), Order::Newer);
    }

    #[test]
    fn a_pre_release_comes_before_its_release() {
        assert_eq!(compare("2.0-beta", "2.0"), Order::Older);
        assert_eq!(compare("2.0", "2.0-beta"), Order::Newer);
        assert_eq!(compare("2.0-beta.3", "2.0-beta.2"), Order::Newer);
        assert_eq!(compare("2.0-rc1", "2.0-beta9"), Order::Newer);
        assert_eq!(compare("2.0-alpha", "2.0-beta"), Order::Older);
        assert_eq!(compare("2.0.1-beta", "2.0"), Order::Newer);
        assert_eq!(compare("2.0-beta", "2.0.0"), Order::Older);
        assert_eq!(compare("2.0.0", "2.0-beta"), Order::Newer);
    }

    #[test]
    fn shapes_that_cannot_be_lined_up_are_unknown() {
        assert_eq!(compare("1.0-pro", "1.0-home"), Order::Unknown);
        assert_eq!(compare("1.0-pro", "1.0"), Order::Unknown);
        assert_eq!(compare("latest", "1.0"), Order::Unknown);
        assert_eq!(compare("", "1.0"), Order::Unknown);
        assert_eq!(compare("1.0;rm", "1.0"), Order::Unknown);
        assert_eq!(compare(&"9".repeat(200), "1.0"), Order::Unknown);
        assert_eq!(compare(&"9".repeat(60), "1.0"), Order::Unknown);
        // The same text is the same even when it cannot be read.
        assert_eq!(compare("latest", "latest"), Order::Same);
    }

    #[test]
    fn never_a_downgrade() {
        assert!(not_a_downgrade(Some("2.0"), Some("1.9")));
        assert!(not_a_downgrade(Some("2.0"), Some("2.0")));
        assert!(!not_a_downgrade(Some("1.9"), Some("2.0")));
        assert!(!not_a_downgrade(Some("1.0-pro"), Some("1.0")));
        assert!(!not_a_downgrade(None, Some("1.0")));
        assert!(not_a_downgrade(None, None));
        assert!(not_a_downgrade(Some("1.0"), None));
    }
}
