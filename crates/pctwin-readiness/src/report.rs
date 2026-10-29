//! The shareable report and feedback (Task List 2.7 "Shareable report and feedback", R11 and R12;
//! plan 2.7 section 9; acceptance T27-025, T27-026, T27-028, T27-030).
//!
//! The report the person may share is built by allowing in, not by taking out: counts, result
//! keys, reasons, check labels and the before-wipe list's counts. Names of the person's files and
//! apps go in only when the person selects them, their account ID only when selected, and another
//! person's things are never named. Whatever names go in are scrubbed of what looks like a secret
//! (an e-mail address, a web address, a long code), since a name can still carry one.
//!
//! The report is typed ([`ShareReport`]): the screens put it in the person's language. Before the
//! person's text is saved, [`check_text`] refuses it if it holds anything the report kept out (a
//! name not chosen, the account ID not chosen, anything that looks like a secret), so the screens
//! cannot widen what leaves. Nothing here sends anything: the crate cannot reach the network, and
//! feedback is the person's own act in Help, which needs no account.

use serde::{Deserialize, Serialize};

use crate::checks::{CheckResult, Label, Summary};
use crate::goodbye::Checklist;
use crate::ledger::{Ledger, Standing, Tally};

/// What the person chose to include. Nothing is included unless chosen.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Disclosure {
    /// The names of their own files and folders.
    pub file_names: bool,
    /// The names of their apps and settings.
    pub app_names: bool,
    /// Their account ID on the old laptop.
    pub account_id: bool,
}

/// What was kept out.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Withheld {
    pub names: u64,
    pub secrets: u64,
    pub account_ids: u64,
}

/// One unfinished line of the report.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ShareLine {
    /// The name, only when chosen, scrubbed.
    pub name: Option<String>,
    pub is_app: bool,
    pub standing: Standing,
    pub reason: Option<pctwin_types::Reason>,
}

/// Exactly what the shareable report holds.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ShareReport {
    pub summary: Summary,
    pub tally: Tally,
    pub account_id: Option<String>,
    pub lines: Vec<ShareLine>,
    pub others_not_finished: u64,
    /// The checks the person picked, by label, in their order.
    pub checks: Vec<Label>,
    /// The before-wipe list: how many things, how many still open.
    pub goodbye: Option<(u64, u64)>,
    pub withheld: Withheld,
    /// What was kept out, to refuse text that holds it ([`check_text`]). Never sent anywhere.
    #[serde(skip)]
    kept_out: Vec<String>,
}

impl ShareReport {
    /// Binds a save to exactly this report.
    pub fn digest(&self) -> [u8; 32] {
        let mut h = blake3::Hasher::new();
        h.update(b"pctwin/v1/share-report\0");
        h.update(&serde_json::to_vec(self).expect("serialises"));
        *h.finalize().as_bytes()
    }
}

fn token_char(c: char) -> bool {
    c.is_ascii_alphanumeric() || matches!(c, '_' | '-' | '+' | '/' | '=' | '.')
}

fn secret_like(word: &str) -> bool {
    let lower = word.to_ascii_lowercase();
    let url = lower.contains("://") || lower.starts_with("www.");
    let email = word
        .split_once('@')
        .is_some_and(|(a, b)| !a.is_empty() && b.contains('.'));
    let long_code = word
        .split(|c: char| !token_char(c) || c == '/' || c == '.')
        .any(|run| {
            run.chars().count() >= 20
                && run.chars().any(|c| c.is_ascii_digit())
                && run.chars().any(|c| c.is_ascii_alphabetic())
        });
    url || email || long_code
}

/// Removes what looks like a secret from `text`: e-mail addresses, web addresses and long codes
/// (20 or more letters and digits mixed). Returns the text and how many were removed.
pub fn scrub(text: &str) -> (String, u64) {
    let mut out = String::with_capacity(text.len());
    let mut removed = 0;
    for (i, word) in text.split(' ').enumerate() {
        if i > 0 {
            out.push(' ');
        }
        if secret_like(word) {
            removed += 1;
            out.push_str("[removed]");
        } else {
            out.push_str(word);
        }
    }
    (out, removed)
}

/// The report the person may share, as it would leave the laptop (T27-025).
pub fn share_preview(
    ledger: &Ledger,
    summary: Summary,
    checks: &[CheckResult],
    checklist: Option<&Checklist>,
    mover_account: &str,
    d: Disclosure,
) -> ShareReport {
    let mut w = Withheld::default();
    let mut kept_out = Vec::new();
    let account_id = if d.account_id {
        Some(mover_account.to_string())
    } else {
        w.account_ids += 1;
        kept_out.push(mover_account.to_string());
        None
    };
    let mut lines = Vec::new();
    let mut others = 0u64;
    for (id, line) in ledger.lines() {
        let s = ledger.standing(*id).expect("listed");
        if matches!(s, Standing::Done(_)) {
            continue;
        }
        if !line.mine {
            others += 1;
            kept_out.push(line.label.clone());
            continue;
        }
        let is_app = matches!(
            line.kind,
            pctwin_record::ItemKind::App | pctwin_record::ItemKind::Setting
        );
        let shown = if is_app { d.app_names } else { d.file_names };
        let name = if shown {
            let (n, k) = scrub(&line.label);
            w.secrets += k;
            Some(n)
        } else {
            w.names += 1;
            kept_out.push(line.label.clone());
            None
        };
        lines.push(ShareLine {
            name,
            is_app,
            standing: s,
            reason: line.result.as_ref().and_then(|a| a.result.reason()),
        });
    }
    // Done lines are never named: their names are kept out too.
    for (id, line) in ledger.lines() {
        if matches!(ledger.standing(*id), Some(Standing::Done(_))) {
            kept_out.push(line.label.clone());
        }
    }
    // A path is kept out whole and by its last part, the name a screen would show.
    let last: Vec<String> = kept_out
        .iter()
        .filter_map(|n| n.rsplit('/').next().filter(|l| l.len() < n.len()))
        .map(String::from)
        .collect();
    kept_out.extend(last);
    kept_out.retain(|n| !n.trim().is_empty());
    kept_out.sort();
    kept_out.dedup();
    ShareReport {
        summary,
        tally: ledger.tally(),
        account_id,
        lines,
        others_not_finished: others,
        checks: checks.iter().map(|c| c.outcome.label()).collect(),
        goodbye: checklist.map(|l| (l.exceptions.len() as u64, l.open().count() as u64)),
        withheld: w,
        kept_out,
    }
}

/// Whether `needle` is in `hay` as a whole: not inside a longer word.
fn contains_whole(hay: &str, needle: &str) -> bool {
    if needle.is_empty() {
        return false;
    }
    hay.match_indices(needle).any(|(at, _)| {
        let before = hay[..at].chars().next_back();
        let after = hay[at + needle.len()..].chars().next();
        !before.is_some_and(char::is_alphanumeric) && !after.is_some_and(char::is_alphanumeric)
    })
}

/// Why the person's text was refused. Nothing was saved.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum Leak {
    #[error("it names something the report kept out")]
    KeptOut,
    #[error("it holds something that looks like a secret")]
    SecretLike,
}

/// Whether `text` (the report as the screens put it in words) holds only what `report` allows: no
/// name, file or account ID it kept out (in any letter case), and nothing that looks like a secret.
pub fn check_text(report: &ShareReport, text: &str) -> Result<(), Leak> {
    let lower = text.to_lowercase();
    // A chosen name is in the text by choice, even when a name kept out is part of it.
    let mut allowed = lower.clone();
    for l in &report.lines {
        if let Some(n) = &l.name {
            allowed = allowed.replace(&n.to_lowercase(), " ");
        }
    }
    if let Some(a) = &report.account_id {
        allowed = allowed.replace(&a.to_lowercase(), " ");
    }
    if report
        .kept_out
        .iter()
        .any(|n| contains_whole(&allowed, &n.to_lowercase()))
    {
        return Err(Leak::KeptOut);
    }
    if allowed.split_whitespace().any(secret_like) {
        return Err(Leak::SecretLike);
    }
    Ok(())
}
