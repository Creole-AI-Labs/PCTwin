//! The Windows registry mechanism: values in one hive, through a safe wrapper.
//!
//! [`Hive`] is the few raw operations settings need. [`MemHive`] keeps a hive in memory (tests on
//! every system); [`WinHive`] is the real one, on Windows, rooted at a key so tests can use a key
//! of their own under `HKEY_CURRENT_USER` and never touch a real setting.

use std::collections::BTreeMap;

/// A registry value.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RegValue {
    Dword(u32),
    Qword(u64),
    Sz(String),
    ExpandSz(String),
    MultiSz(Vec<String>),
    Binary(Vec<u8>),
    /// Any other type: never written.
    Other,
}

/// Why a write did not happen.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum WriteError {
    /// The system or the person said no (access denied, a declined prompt).
    #[error("the system did not allow the change")]
    Denied,
    #[error("{0}")]
    Failed(String),
}

/// One registry hive (or a key standing in for one). Missing keys and values read as `None` or
/// empty, never as an error.
pub trait Hive {
    fn get(&self, key: &str, name: &str) -> Result<Option<RegValue>, String>;
    fn set(&mut self, key: &str, name: &str, value: &RegValue) -> Result<(), WriteError>;
    fn delete(&mut self, key: &str, name: &str) -> Result<(), WriteError>;
    /// The names of the key's values.
    fn names(&self, key: &str) -> Result<Vec<String>, String>;
    /// The names of the key's subkeys.
    fn subkeys(&self, key: &str) -> Result<Vec<String>, String>;
    fn key_exists(&self, key: &str) -> Result<bool, String>;
    /// Removes the key only if it has no values and no subkeys; whether it did.
    fn delete_key_if_empty(&mut self, key: &str) -> Result<bool, WriteError>;
}

/// A hive in memory. Names ignore letter case, as the registry's do.
#[derive(Debug, Clone, Default)]
pub struct MemHive {
    keys: BTreeMap<String, BTreeMap<String, (String, RegValue)>>,
    /// Each key's name as written (for listing subkeys).
    spelled: BTreeMap<String, String>,
    /// Writes fail as denied (a hive the person may not change).
    pub deny_writes: bool,
    /// Reads fail.
    pub unreadable: bool,
    /// Writes report success and change nothing (a system that does not keep the change).
    pub lies: bool,
}

impl MemHive {
    fn make_key(&mut self, key: &str) {
        let mut path = String::new();
        for part in key.split('\\') {
            if !path.is_empty() {
                path.push('\\');
            }
            path.push_str(part);
            let lower = path.to_lowercase();
            self.keys.entry(lower.clone()).or_default();
            self.spelled.entry(lower).or_insert_with(|| path.clone());
        }
    }

    pub fn put(&mut self, key: &str, name: &str, value: RegValue) {
        self.make_key(key);
        self.keys
            .entry(key.to_lowercase())
            .or_default()
            .insert(name.to_lowercase(), (name.to_string(), value));
    }
}

impl Hive for MemHive {
    fn get(&self, key: &str, name: &str) -> Result<Option<RegValue>, String> {
        if self.unreadable {
            return Err("the registry could not be read".into());
        }
        Ok(self
            .keys
            .get(&key.to_lowercase())
            .and_then(|k| k.get(&name.to_lowercase()))
            .map(|(_, v)| v.clone()))
    }
    fn set(&mut self, key: &str, name: &str, value: &RegValue) -> Result<(), WriteError> {
        if self.deny_writes {
            return Err(WriteError::Denied);
        }
        if !self.lies {
            self.put(key, name, value.clone());
        }
        Ok(())
    }
    fn delete(&mut self, key: &str, name: &str) -> Result<(), WriteError> {
        if self.deny_writes {
            return Err(WriteError::Denied);
        }
        if let Some(k) = self.keys.get_mut(&key.to_lowercase()) {
            k.remove(&name.to_lowercase());
        }
        Ok(())
    }
    fn names(&self, key: &str) -> Result<Vec<String>, String> {
        if self.unreadable {
            return Err("the registry could not be read".into());
        }
        Ok(self
            .keys
            .get(&key.to_lowercase())
            .map(|k| k.values().map(|(n, _)| n.clone()).collect())
            .unwrap_or_default())
    }
    fn subkeys(&self, key: &str) -> Result<Vec<String>, String> {
        if self.unreadable {
            return Err("the registry could not be read".into());
        }
        let prefix = format!("{}\\", key.to_lowercase());
        Ok(self
            .keys
            .keys()
            .filter(|k| k.starts_with(&prefix) && !k[prefix.len()..].contains('\\'))
            .map(|k| {
                let spelled = self.spelled.get(k).cloned().unwrap_or_else(|| k.clone());
                spelled[prefix.len()..].to_string()
            })
            .collect())
    }
    fn key_exists(&self, key: &str) -> Result<bool, String> {
        Ok(self.keys.contains_key(&key.to_lowercase()))
    }
    fn delete_key_if_empty(&mut self, key: &str) -> Result<bool, WriteError> {
        if self.deny_writes {
            return Err(WriteError::Denied);
        }
        let lower = key.to_lowercase();
        let empty = self.keys.get(&lower).is_some_and(BTreeMap::is_empty)
            && !self
                .keys
                .keys()
                .any(|k| k.starts_with(&format!("{lower}\\")));
        if empty {
            self.keys.remove(&lower);
            self.spelled.remove(&lower);
        }
        Ok(empty)
    }
}

#[cfg(windows)]
pub use win::WinHive;

#[cfg(windows)]
mod win {
    use super::{Hive, RegValue, WriteError};
    use std::io::ErrorKind;
    use winreg::enums::{HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE, KEY_READ, KEY_WRITE, RegType};
    use winreg::{RegKey, RegValue as Raw};

    /// The most bytes of one value PCTwin reads.
    const MAX_VALUE: usize = 64 * 1024;

    /// The real registry, rooted at a key (a hive's root, or a test's own key).
    pub struct WinHive {
        root: RegKey,
        read_only: bool,
    }

    impl WinHive {
        /// The person's own settings (`HKEY_CURRENT_USER`).
        pub fn current_user() -> Self {
            Self {
                root: RegKey::predef(HKEY_CURRENT_USER),
                read_only: false,
            }
        }

        /// The machine's settings (`HKEY_LOCAL_MACHINE`), read only.
        pub fn local_machine() -> Self {
            Self {
                root: RegKey::predef(HKEY_LOCAL_MACHINE),
                read_only: true,
            }
        }

        /// A key under `HKEY_CURRENT_USER` standing in for a hive (tests), made if missing.
        pub fn user_subkey(path: &str) -> std::io::Result<Self> {
            let (root, _) = RegKey::predef(HKEY_CURRENT_USER).create_subkey(path)?;
            Ok(Self {
                root,
                read_only: false,
            })
        }

        fn open(&self, key: &str) -> Result<Option<RegKey>, String> {
            match self.root.open_subkey_with_flags(key, KEY_READ) {
                Ok(k) => Ok(Some(k)),
                Err(e) if e.kind() == ErrorKind::NotFound => Ok(None),
                Err(e) => Err(format!("the registry could not be read ({})", e.kind())),
            }
        }
    }

    fn write_err(e: std::io::Error) -> WriteError {
        if e.kind() == ErrorKind::PermissionDenied {
            WriteError::Denied
        } else {
            WriteError::Failed(format!("the registry could not be written ({})", e.kind()))
        }
    }

    fn utf16(bytes: &[u8]) -> Option<Vec<u16>> {
        bytes.len().is_multiple_of(2).then(|| {
            bytes
                .as_chunks::<2>()
                .0
                .iter()
                .map(|c| u16::from_le_bytes(*c))
                .collect()
        })
    }

    fn text(bytes: &[u8]) -> Option<String> {
        let units = utf16(bytes)?;
        let end = units.iter().position(|&u| u == 0).unwrap_or(units.len());
        String::from_utf16(&units[..end]).ok()
    }

    fn decode(raw: Raw) -> RegValue {
        if raw.bytes.len() > MAX_VALUE {
            return RegValue::Other;
        }
        let b = &raw.bytes;
        match raw.vtype {
            RegType::REG_DWORD if b.len() == 4 => {
                RegValue::Dword(u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
            }
            RegType::REG_QWORD if b.len() == 8 => {
                let mut a = [0u8; 8];
                a.copy_from_slice(b);
                RegValue::Qword(u64::from_le_bytes(a))
            }
            RegType::REG_SZ => text(b).map(RegValue::Sz).unwrap_or(RegValue::Other),
            RegType::REG_EXPAND_SZ => text(b).map(RegValue::ExpandSz).unwrap_or(RegValue::Other),
            RegType::REG_MULTI_SZ => match utf16(b) {
                Some(units) => {
                    let mut out = Vec::new();
                    for part in units.split(|&u| u == 0) {
                        if part.is_empty() {
                            continue;
                        }
                        match String::from_utf16(part) {
                            Ok(s) => out.push(s),
                            Err(_) => return RegValue::Other,
                        }
                    }
                    RegValue::MultiSz(out)
                }
                None => RegValue::Other,
            },
            RegType::REG_BINARY => RegValue::Binary(b.clone()),
            _ => RegValue::Other,
        }
    }

    fn encode(v: &RegValue) -> Option<Raw> {
        let wide = |s: &str| -> Vec<u8> {
            s.encode_utf16()
                .chain(std::iter::once(0))
                .flat_map(u16::to_le_bytes)
                .collect()
        };
        Some(match v {
            RegValue::Dword(d) => Raw {
                bytes: d.to_le_bytes().to_vec(),
                vtype: RegType::REG_DWORD,
            },
            RegValue::Qword(q) => Raw {
                bytes: q.to_le_bytes().to_vec(),
                vtype: RegType::REG_QWORD,
            },
            RegValue::Sz(s) => Raw {
                bytes: wide(s),
                vtype: RegType::REG_SZ,
            },
            RegValue::ExpandSz(s) => Raw {
                bytes: wide(s),
                vtype: RegType::REG_EXPAND_SZ,
            },
            RegValue::MultiSz(parts) => {
                let mut bytes: Vec<u8> = parts.iter().flat_map(|p| wide(p)).collect();
                bytes.extend_from_slice(&[0, 0]);
                Raw {
                    bytes,
                    vtype: RegType::REG_MULTI_SZ,
                }
            }
            RegValue::Binary(b) => Raw {
                bytes: b.clone(),
                vtype: RegType::REG_BINARY,
            },
            RegValue::Other => return None,
        })
    }

    impl Hive for WinHive {
        fn get(&self, key: &str, name: &str) -> Result<Option<RegValue>, String> {
            let Some(k) = self.open(key)? else {
                return Ok(None);
            };
            match k.get_raw_value(name) {
                Ok(r) => Ok(Some(decode(r))),
                Err(e) if e.kind() == ErrorKind::NotFound => Ok(None),
                Err(e) => Err(format!("the registry could not be read ({})", e.kind())),
            }
        }

        fn set(&mut self, key: &str, name: &str, value: &RegValue) -> Result<(), WriteError> {
            if self.read_only {
                return Err(WriteError::Denied);
            }
            let raw = encode(value)
                .ok_or_else(|| WriteError::Failed("not a value PCTwin writes".into()))?;
            let (k, _) = self
                .root
                .create_subkey_with_flags(key, KEY_READ | KEY_WRITE)
                .map_err(write_err)?;
            k.set_raw_value(name, &raw).map_err(write_err)
        }

        fn delete(&mut self, key: &str, name: &str) -> Result<(), WriteError> {
            if self.read_only {
                return Err(WriteError::Denied);
            }
            let k = match self.root.open_subkey_with_flags(key, KEY_READ | KEY_WRITE) {
                Ok(k) => k,
                Err(e) if e.kind() == ErrorKind::NotFound => return Ok(()),
                Err(e) => return Err(write_err(e)),
            };
            match k.delete_value(name) {
                Err(e) if e.kind() != ErrorKind::NotFound => Err(write_err(e)),
                _ => Ok(()),
            }
        }

        fn names(&self, key: &str) -> Result<Vec<String>, String> {
            let Some(k) = self.open(key)? else {
                return Ok(Vec::new());
            };
            k.enum_values()
                .take(4096)
                .map(|r| r.map(|(n, _)| n).map_err(|e| e.kind().to_string()))
                .collect()
        }

        fn subkeys(&self, key: &str) -> Result<Vec<String>, String> {
            let Some(k) = self.open(key)? else {
                return Ok(Vec::new());
            };
            k.enum_keys()
                .take(4096)
                .map(|r| r.map_err(|e| e.kind().to_string()))
                .collect()
        }

        fn key_exists(&self, key: &str) -> Result<bool, String> {
            Ok(self.open(key)?.is_some())
        }

        fn delete_key_if_empty(&mut self, key: &str) -> Result<bool, WriteError> {
            if self.read_only {
                return Err(WriteError::Denied);
            }
            let Some(k) = self.open(key).map_err(WriteError::Failed)? else {
                return Ok(false);
            };
            let info = k.query_info().map_err(write_err)?;
            if info.values != 0 || info.sub_keys != 0 {
                return Ok(false);
            }
            drop(k);
            match self.root.delete_subkey(key) {
                Ok(()) => Ok(true),
                Err(e) if e.kind() == ErrorKind::NotFound => Ok(false),
                Err(e) => Err(write_err(e)),
            }
        }
    }
}
