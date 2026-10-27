//! The shareable report and feedback (Task List 2.7 "Shareable report and feedback", R11 and R12;
//! plan 2.7 section 9; acceptance T27-025, T27-026, T27-028, T27-030).
//!
//! The report the person may share is built by allowing in, not by taking out: counts, result
//! words, reasons, check labels and coverage, and the kinds of what is left to do. Names of the
//! person's files and apps go in only when the person selects them, account IDs only when
//! selected, and another person's things are never named. Whatever names go in are then scrubbed
//! of what looks like a secret (an e-mail address, a web address, a long code), since a name
//! matched by patterns alone can still carry one.
//!
//! [`share_preview`] gives the exact text that would leave the laptop. Nothing here sends it, or
//! anything else: the crate has no way to reach the network, and sending is the person's own act
//! in the Help and Feedback screens, which need no account ([`FEEDBACK_WORDS`]).

use serde::{Deserialize, Serialize};

use crate::checks::{CheckResult, Summary};
use crate::goodbye::Checklist;
use crate::ledger::{Ledger, Standing};

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

/// What was kept out of the preview.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Withheld {
    pub names: u64,
    pub secrets: u64,
    pub account_ids: u64,
}

/// Exactly what would leave the laptop, and what was kept out.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SharePreview {
    pub text: String,
    pub withheld: Withheld,
}

/// How to send feedback: from Help, with no account (R12; T27-028).
pub const FEEDBACK_WORDS: &str = "To tell us about a problem, open Help, then Feedback. You do \
    not need an account. Nothing is sent until you choose Send, and you see exactly what is sent \
    first.";

/// What the report is not (T27-026).
pub const NOT_A_CERTIFICATE: &str = "This report is a record of what PCTwin observed on these \
    laptops. It is not a certificate of erasure or of anything else.";

fn standing_word(s: Standing) -> &'static str {
    match s {
        Standing::Done(pctwin_types::ItemStatus::Reinstalled) => "reinstalled",
        Standing::Done(pctwin_types::ItemStatus::Converted) => "converted",
        Standing::Done(_) => "copied",
        Standing::NeedsAction => "needs action",
        Standing::Failed => "failed",
        Standing::Deferred => "waiting",
        Standing::Inaccessible => "could not be reached",
        Standing::NotTested => "no result yet",
        Standing::SkippedByUser => "skipped by you",
        Standing::Excluded => "left out",
    }
}

fn token_char(c: char) -> bool {
    c.is_ascii_alphanumeric() || matches!(c, '_' | '-' | '+' | '/' | '=' | '.')
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
        if url || email || long_code {
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
) -> SharePreview {
    let mut w = Withheld::default();
    let t = ledger.tally();
    let mut text = String::from("PCTwin move report\n\n");
    text.push_str(summary.words());
    text.push_str("\n\n");
    text.push_str(&format!(
        "Picked to move: {}. On the new laptop: {}. Need action: {}. Failed: {}. Waiting: {}. \
         Could not be reached: {}. No result yet: {}. Skipped by you: {}. Left out: {}. Left \
         behind by your choice: {}.\n",
        t.selected,
        t.done,
        t.needs_action,
        t.failed,
        t.deferred,
        t.inaccessible,
        t.not_tested,
        t.skipped,
        t.excluded,
        t.accepted_loss
    ));
    if d.account_id {
        text.push_str(&format!("Account on the old laptop: {mover_account}\n"));
    } else {
        w.account_ids += 1;
    }
    text.push_str("\nItems not finished:\n");
    let mut others = 0u64;
    for (id, line) in ledger.lines() {
        let s = ledger.standing(*id).expect("listed");
        if matches!(s, Standing::Done(_)) {
            continue;
        }
        if !line.mine {
            others += 1;
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
            n
        } else {
            w.names += 1;
            if is_app {
                "an app or setting".into()
            } else {
                "a file".into()
            }
        };
        let reason = line
            .result
            .as_ref()
            .and_then(|a| a.result.reason())
            .map(|r| {
                format!(
                    " ({})",
                    serde_json::to_value(r)
                        .ok()
                        .and_then(|v| v.as_str().map(String::from))
                        .unwrap_or_default()
                )
            })
            .unwrap_or_default();
        text.push_str(&format!("- {name}: {}{reason}\n", standing_word(s)));
    }
    if others > 0 {
        text.push_str(&format!("- other people's items not finished: {others}\n"));
    }
    if !checks.is_empty() {
        text.push_str("\nChecks you picked:\n");
        for c in checks {
            let label = serde_json::to_value(c.outcome.label())
                .ok()
                .and_then(|v| v.as_str().map(String::from))
                .unwrap_or_default();
            text.push_str(&format!("- check {}: {label}\n", c.index + 1));
        }
    }
    if let Some(list) = checklist {
        text.push_str(&format!(
            "\nBefore leaving the old laptop: {} things on the list, {} still open.\n",
            list.exceptions.len(),
            list.open().count()
        ));
    }
    text.push('\n');
    text.push_str(NOT_A_CERTIFICATE);
    text.push('\n');
    SharePreview { text, withheld: w }
}
