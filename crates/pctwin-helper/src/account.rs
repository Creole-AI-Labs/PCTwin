//! `CreateAccount` on the new laptop: an approved person's standard account (never an
//! administrator), with a temporary password from the system's random source that they must
//! change at their first sign-in (Security Design part C). The password is said once to the app,
//! which shows it to the approving administrator in a box of the system's own; the helper keeps
//! it nowhere, and clears its own copy when it is done.

use zeroize::Zeroizing;

use crate::protocol::Refusal;

/// Letters and digits that cannot be mistaken for each other when read aloud or copied by hand
/// (no 0/O, 1/l/I, 5/S, 2/Z).
const ALPHABET: &[u8] = b"abcdefghjkmnpqrstuvwxyzABCDEFGHJKLMNPQRTUVWXY346789";

/// A made account: its ID (a SID on Windows, a user ID on Mac and Linux) and its temporary
/// password, cleared from memory when dropped.
pub struct Made {
    pub account: String,
    pub password: Zeroizing<String>,
}

/// A temporary password: four groups of four, about 90 bits from the system's random source,
/// easy to type, accepted by every system's rules.
pub fn temporary_password() -> Result<Zeroizing<String>, Refusal> {
    let mut bytes = Zeroizing::new([0u8; 16]);
    getrandom::fill(bytes.as_mut()).map_err(|_| Refusal::AccountNotMade)?;
    let mut out = Zeroizing::new(String::with_capacity(19));
    for (n, b) in bytes.iter().enumerate() {
        if n > 0 && n % 4 == 0 {
            out.push('-');
        }
        // 256 is not a multiple of the alphabet's size; the slight lean is harmless at 90 bits.
        out.push(char::from(ALPHABET[usize::from(*b) % ALPHABET.len()]));
    }
    Ok(out)
}

/// Makes the account `name` (approved by the administrator), shown as `full_name`.
pub fn create_account(name: &str, full_name: &str) -> Result<Made, Refusal> {
    let password = temporary_password()?;
    match pctwin_helper_os::add_standard_account(name, full_name, &password) {
        Ok(account) => Ok(Made {
            account: match account {
                pctwin_helper_os::Account::Sid(sid) => sid.as_str().to_string(),
                pctwin_helper_os::Account::Uid(uid) => uid.to_string(),
            },
            password,
        }),
        Err(pctwin_helper_os::Error::AlreadyExists) => Err(Refusal::AccountExists),
        Err(_) => Err(Refusal::AccountNotMade),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn temporary_passwords_are_long_unambiguous_and_never_the_same() {
        let a = temporary_password().unwrap();
        let b = temporary_password().unwrap();
        assert_ne!(*a, *b);
        assert_eq!(a.len(), 19);
        assert_eq!(a.matches('-').count(), 3);
        assert!(
            a.chars()
                .filter(|c| *c != '-')
                .all(|c| ALPHABET.contains(&(c as u8)))
        );
        for confusable in ['0', 'O', '1', 'l', 'I', '5', 'S', '2', 'Z'] {
            assert!(!ALPHABET.contains(&(confusable as u8)), "{confusable}");
        }
    }
}
