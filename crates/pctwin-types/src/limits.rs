//! Every safety bound in one place (the user, 27 October 2026: "don't hard code").
//!
//! These are code on purpose, never catalogue data: a downloaded catalogue may add caution, but
//! it must never loosen a limit that keeps a hostile old laptop or a damaged file from using up
//! the new laptop's stack, memory or time. Each limit says what it protects.

/// How deep any structured text (XML, property lists, JSON, TOML, GVariant) may nest before it
/// is read. Parsers call themselves once per level; 256 levels fit easily in the smallest stack a
/// thread here runs on, and real settings files nest under twenty.
pub const NESTING: usize = 256;

/// The most settings one discovery reads from one laptop's stores (registry, Mac preferences,
/// dconf). The rest are reported, store by store, as "found, not read: too many".
pub const FOUND_SETTINGS: usize = 50_000;

/// How many keys deep discovery walks below an app's own registry key. Deeper keys are
/// reported as "found, not read: too deeply nested".
pub const REGISTRY_DEPTH: usize = 12;

/// The largest raw (binary) registry value moved as a setting, in bytes.
pub const REGISTRY_BINARY_BYTES: usize = 1024;

/// The longest text one setting may carry, in bytes.
pub const SETTING_TEXT_BYTES: usize = 4000;

/// The largest app settings file read or planned, in bytes. A larger one is "found, not read:
/// too large".
pub const SETTINGS_FILE_BYTES: u64 = 1024 * 1024;

/// How many folders deep, and how many files in all, the app-settings-file walk looks.
pub const SETTINGS_FILE_DEPTH: usize = 8;
pub const SETTINGS_FILES: usize = 20_000;

/// The largest property list `defaults export` may print for one domain, in bytes.
pub const PREFERENCES_BYTES: usize = 8 * 1024 * 1024;

/// The largest Wi-Fi profile read, in bytes, and the most profiles read.
pub const WIFI_PROFILE_BYTES: u64 = 64 * 1024;
pub const WIFI_PROFILES: usize = 1000;

/// How many entries the mail-store walk looks at before it stops (and says so).
pub const MAIL_WALK_ENTRIES: usize = 50_000;
