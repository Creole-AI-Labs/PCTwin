//! How many mail accounts the person has set up on the old laptop, counted at scan time for the
//! after-move check ("You had 3 mail accounts: sign in to them on the new laptop"). Only what the
//! mail programs record about their accounts is read (Outlook's profile keys, Thunderbird's
//! settings, Apple Mail's account folders); no mail is ever opened. Mail itself is not moved in
//! this version, so the count is what the person is told to set up again.

use std::path::Path;

/// Mail accounts found, per program (`None`: the program's records were not there or could not
/// be read, so nothing is claimed).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct MailAccounts {
    pub outlook: Option<u32>,
    pub thunderbird: Option<u32>,
    pub apple_mail: Option<u32>,
}

impl MailAccounts {
    /// All accounts found.
    pub fn total(&self) -> u32 {
        [self.outlook, self.thunderbird, self.apple_mail]
            .iter()
            .flatten()
            .sum()
    }
}

/// The mail accounts of the person whose home is `home` (the signed-in person: Outlook's are read
/// from their own registry).
pub fn mail_accounts(home: &Path) -> MailAccounts {
    MailAccounts {
        outlook: outlook_accounts(),
        thunderbird: thunderbird_accounts(home),
        apple_mail: apple_mail_accounts(home),
    }
}

/// Outlook keeps each account of each profile as a numbered key under this one.
const OUTLOOK_ACCOUNTS_KEY: &str = "9375cff0413111d3b88a00104b2a6676\\";

/// Accounts in `reg query HKCU\Software\Microsoft\Office\16.0\Outlook\Profiles /s` output: each
/// key directly under an account list (`...\9375CFF0...\00000002`), counted once.
pub fn outlook_accounts_in(reg_output: &str) -> u32 {
    let mut seen = std::collections::BTreeSet::new();
    for line in reg_output.lines() {
        let lower = line.trim().to_lowercase();
        let Some(at) = lower.find(OUTLOOK_ACCOUNTS_KEY) else {
            continue;
        };
        let rest = &lower[at + OUTLOOK_ACCOUNTS_KEY.len()..];
        if !rest.is_empty() && !rest.contains('\\') && rest.chars().all(|c| c.is_ascii_hexdigit()) {
            seen.insert(lower);
        }
    }
    u32::try_from(seen.len()).unwrap_or(u32::MAX)
}

fn outlook_accounts() -> Option<u32> {
    if !cfg!(windows) {
        return None;
    }
    let root = std::env::var_os("SystemRoot").unwrap_or_else(|| r"C:\Windows".into());
    let out = std::process::Command::new(Path::new(&root).join(r"System32\reg.exe"))
        .args([
            "query",
            r"HKCU\Software\Microsoft\Office\16.0\Outlook\Profiles",
            "/s",
        ])
        .stdin(std::process::Stdio::null())
        .output()
        .ok()?;
    // No Outlook profile at all: the key is missing, and that is nothing to claim either way.
    out.status
        .success()
        .then(|| outlook_accounts_in(&String::from_utf8_lossy(&out.stdout)))
}

/// Mail servers in a Thunderbird `prefs.js`: each `mail.server.serverN.type` that is a real
/// account (IMAP, POP3, Exchange), never the local folders (`none`) or news and feeds.
pub fn thunderbird_accounts_in(prefs: &str) -> u32 {
    let count = prefs
        .lines()
        .filter_map(|l| {
            let l = l.trim();
            let rest = l.strip_prefix("user_pref(\"mail.server.server")?;
            let (_, rest) = rest.split_once(".type\",")?;
            let kind = rest.trim().trim_end_matches(");").trim().trim_matches('"');
            Some(kind.to_string())
        })
        .filter(|k| matches!(k.as_str(), "imap" | "pop3" | "owl" | "exchange" | "ews"))
        .count();
    u32::try_from(count).unwrap_or(u32::MAX)
}

fn thunderbird_accounts(home: &Path) -> Option<u32> {
    let base = if cfg!(windows) {
        home.join(r"AppData\Roaming\Thunderbird")
    } else if cfg!(target_os = "macos") {
        home.join("Library/Thunderbird")
    } else {
        home.join(".thunderbird")
    };
    let profiles = std::fs::read_to_string(base.join("profiles.ini")).ok()?;
    let mut total = 0u32;
    for line in profiles.lines() {
        let Some(path) = line.trim().strip_prefix("Path=") else {
            continue;
        };
        // Only profiles inside Thunderbird's own folder are read.
        if path.contains("..") || Path::new(path).is_absolute() {
            continue;
        }
        if let Ok(prefs) = std::fs::read_to_string(base.join(path).join("prefs.js")) {
            total = total.saturating_add(thunderbird_accounts_in(&prefs));
        }
    }
    Some(total)
}

/// Apple Mail's accounts: the account folders in its newest `V<n>` folder (each named for the
/// account), never its own `MailData`.
pub fn apple_mail_accounts_in(names: &[String]) -> u32 {
    let count = names
        .iter()
        .filter(|n| !n.starts_with('.') && *n != "MailData")
        .count();
    u32::try_from(count).unwrap_or(u32::MAX)
}

fn apple_mail_accounts(home: &Path) -> Option<u32> {
    if !cfg!(target_os = "macos") {
        return None;
    }
    let mail = home.join("Library/Mail");
    // A privacy setting may refuse this: then nothing is claimed.
    let newest = std::fs::read_dir(&mail)
        .ok()?
        .flatten()
        .filter_map(|e| {
            let name = e.file_name().to_string_lossy().into_owned();
            name.strip_prefix('V')?.parse::<u32>().ok()
        })
        .max()?;
    let names: Vec<String> = std::fs::read_dir(mail.join(format!("V{newest}")))
        .ok()?
        .flatten()
        .filter(|e| e.file_type().is_ok_and(|t| t.is_dir()))
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .collect();
    Some(apple_mail_accounts_in(&names))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn outlook_accounts_are_counted_from_the_profile_keys() {
        let out = "\r\nHKEY_CURRENT_USER\\Software\\Microsoft\\Office\\16.0\\Outlook\\Profiles\\Outlook\r\n\
HKEY_CURRENT_USER\\Software\\Microsoft\\Office\\16.0\\Outlook\\Profiles\\Outlook\\9375CFF0413111d3B88A00104B2A6676\r\n    {ED475418-B0D6-11D2-8C3B-00104B2A6676}    REG_BINARY    02000000\r\n\
HKEY_CURRENT_USER\\Software\\Microsoft\\Office\\16.0\\Outlook\\Profiles\\Outlook\\9375CFF0413111d3B88A00104B2A6676\\00000002\r\n    Account Name    REG_BINARY    61006400\r\n\
HKEY_CURRENT_USER\\Software\\Microsoft\\Office\\16.0\\Outlook\\Profiles\\Outlook\\9375CFF0413111d3B88A00104B2A6676\\00000003\r\n\
HKEY_CURRENT_USER\\Software\\Microsoft\\Office\\16.0\\Outlook\\Profiles\\Outlook\\9375CFF0413111d3B88A00104B2A6676\\00000003\\Extra\r\n";
        assert_eq!(outlook_accounts_in(out), 2);
        assert_eq!(outlook_accounts_in(""), 0);
    }

    #[test]
    fn thunderbird_counts_real_accounts_not_local_folders_or_feeds() {
        let prefs = r#"
user_pref("mail.server.server1.type", "none");
user_pref("mail.server.server2.type", "imap");
user_pref("mail.server.server3.type", "pop3");
user_pref("mail.server.server4.type", "rss");
user_pref("mail.server.server5.type", "nntp");
user_pref("mail.server.server2.hostname", "imap.example.com");
"#;
        assert_eq!(thunderbird_accounts_in(prefs), 2);
    }

    #[test]
    fn apple_mail_counts_account_folders_only() {
        let names: Vec<String> = [
            "MailData",
            ".DS_Store",
            "IMAP-ada@imap.example.com",
            "4F2C-UUID",
        ]
        .iter()
        .map(|s| s.to_string())
        .collect();
        assert_eq!(apple_mail_accounts_in(&names), 2);
        let none = MailAccounts::default();
        assert_eq!(none.total(), 0);
        let some = MailAccounts {
            outlook: Some(2),
            thunderbird: None,
            apple_mail: Some(1),
        };
        assert_eq!(some.total(), 3);
    }
}
