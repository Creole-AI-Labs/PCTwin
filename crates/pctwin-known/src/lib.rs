//! Quick Send's remembered connections (Security Design Part T, Quick Send Stage 1).
//!
//! - [`swap`]: inside the first sealed pairing, each laptop makes a key pair for the connection
//!   and only the public halves are swapped.
//! - [`book`]: the remembered connections and their three permissions, each stored separately and
//!   each withdrawn on its own; Forget deletes the key and every permission; a connection unused
//!   for 90 days expires; tokens that change every 15 minutes, which only the other laptop can
//!   recognise, are what is announced on the network (never names).
//! - [`store`]: the private keys and pair secrets, only in the system's key store.
//! - [`folders`]: folders held by identity and path, rechecked at every exchange, never a place
//!   that runs things or holds keys (a catalogue).
//! - [`machine`]: telling a restored backup or a copied drive.
//!
//! A remembered connection proves "the same device as before", never a person.

pub mod book;
pub mod folders;
pub mod machine;
pub mod store;
pub mod swap;

pub use book::{
    Admitted, Book, BookError, ConnectionInfo, Device, GrantInfo, GrantProblem, NextFolder,
};
pub use folders::{FolderProblem, KnownFolder};
pub use store::{MemoryStore, SecretStore, StoreError, SystemStore};
pub use swap::{Pending, Swap, SwapError};

/// Lower-case hex.
pub(crate) fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

/// 32 bytes from 64 lower-case hex characters.
pub(crate) fn unhex(text: &str) -> Option<[u8; 32]> {
    if text.len() != 64
        || !text
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
    {
        return None;
    }
    let mut out = [0u8; 32];
    for (i, o) in out.iter_mut().enumerate() {
        *o = u8::from_str_radix(&text[2 * i..2 * i + 2], 16).ok()?;
    }
    Some(out)
}
