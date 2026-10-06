//! PCTwin discovery: the old laptop announces itself on the local network while pairing, and the
//! new laptop finds it, using multicast DNS (Bonjour on Macs) through `mdns-sd`.
//!
//! # What is announced
//!
//! Only a session [`Label`] and the port to connect to. A label is either picked at random from
//! [`COLOURS`] × [`ANIMALS`] (1,200 labels, such as "Blue Fox"; the person can [`Label::shuffle`]
//! for another) or a short name the person types for this session ([`Label::named`]). The
//! laptop's real name, the code and account details are never announced: the instance and host
//! names are random for each announcement.
//!
//! Labels tell laptops apart; they are not for security. Anyone on the network can announce any
//! label, so the new laptop flags labels seen more than once ([`Found::same_label_nearby`]) and
//! connects only to the laptop the person chose. The code and the number match keep pairing safe.
//!
//! Names are checked against an allow-list, not a block-list: letters and numbers in any script,
//! accent marks on them (at most four in a row), joiners only inside words, single spaces between
//! words and everyday punctuation from many languages, at most [`MAX_NAME_CHARS`] visible
//! characters, with at least one letter or number. Emoji, invisible, blank-looking, control, text-direction and private characters are refused.
//! Names are stored in one standard accent form (Unicode NFC), and two labels count as the same
//! when they match ignoring case and accent form ([`Label::comparison_key`]).
//!
//! Everything that arrives is checked like any other input: an announcement must contain exactly
//! the fields PCTwin writes, once each, in the form PCTwin writes them (version, canonical
//! in-range numbers, a tidy NFC name). Only local-network addresses that are not this computer's
//! own loopback are kept, at most [`MAX_ADDRS`] of them, and an IPv6 link-local address only with
//! its interface.

use std::collections::HashMap;
use std::net::{IpAddr, SocketAddr, SocketAddrV6};
use std::time::{Duration, Instant};

use mdns_sd::{ResolvedService, ScopedIp, ServiceDaemon, ServiceEvent, ServiceInfo};
use pctwin_link::is_local_peer;
use unicode_normalization::char::is_combining_mark;
use unicode_normalization::{UnicodeNormalization, is_nfc};

/// The multicast DNS service type PCTwin old laptops announce while pairing.
pub const SERVICE_TYPE: &str = "_pctwin._tcp.local.";
/// Longest name a person may type, in letters people see (accent marks and joiners don't count).
pub const MAX_NAME_CHARS: usize = 32;
/// Longest name in bytes, so it always fits in one announcement field.
const MAX_NAME_BYTES: usize = 240;
/// Most addresses kept for one old laptop.
pub const MAX_ADDRS: usize = 8;
/// Version of the announcement format.
const FORMAT_VERSION: &str = "1";
/// Most accent marks allowed on one letter (Burmese and Hindi stack several).
const MAX_MARKS_IN_A_ROW: usize = 4;
/// Zero-width non-joiner and joiner: needed inside words in Persian, Sinhala and other scripts,
/// so allowed only between two letters.
const JOINERS: [char; 2] = ['\u{200C}', '\u{200D}'];
/// Everyday punctuation from many languages, besides all ASCII punctuation.
const EXTRA_PUNCTUATION: &[char] = &[
    '\u{00B7}', // middle dot, in Chinese transliterated names
    '\u{30FB}', // katakana middle dot
    '\u{FF08}', '\u{FF09}', '\u{FF0C}', '\u{FF1A}', '\u{FF01}',
    '\u{FF1F}', // full-width ( ) , : ! ?
    '\u{3001}', '\u{3002}', // ideographic comma and full stop
    '\u{00B0}', '\u{2116}', // degree sign, numero sign
    '\u{00AB}', '\u{00BB}', // guillemets
    '\u{2013}', '\u{2014}', '\u{2026}', // en dash, em dash, ellipsis
    '\u{2018}', '\u{2019}', '\u{201C}', '\u{201D}', // curly quotes
    '\u{00A1}', '\u{00BF}', // inverted ! and ?
    '\u{060C}', '\u{061F}', // Arabic comma and question mark
    '\u{0970}', // Devanagari abbreviation sign
    '\u{0F0B}', // Tibetan syllable mark
];

/// Label colours, as translation keys. Shown with the animal's picture and its name, never as
/// colour alone.
pub const COLOURS: [&str; 12] = [
    "red", "orange", "yellow", "green", "blue", "purple", "pink", "brown", "black", "white",
    "grey", "gold",
];

/// Label animals, as translation keys. Common, easy to picture and to tell apart.
pub const ANIMALS: [&str; 100] = [
    "ant",
    "badger",
    "bat",
    "bear",
    "bee",
    "beetle",
    "bison",
    "butterfly",
    "camel",
    "cat",
    "cheetah",
    "chicken",
    "crab",
    "crane",
    "crocodile",
    "crow",
    "deer",
    "dolphin",
    "donkey",
    "dove",
    "dragonfly",
    "duck",
    "eagle",
    "elephant",
    "falcon",
    "flamingo",
    "fox",
    "frog",
    "gazelle",
    "swordfish",
    "giraffe",
    "goat",
    "goose",
    "gorilla",
    "hamster",
    "hawk",
    "hedgehog",
    "armadillo",
    "hippo",
    "horse",
    "hummingbird",
    "jaguar",
    "jellyfish",
    "kangaroo",
    "kingfisher",
    "koala",
    "ladybird",
    "lemur",
    "leopard",
    "lion",
    "lizard",
    "llama",
    "lobster",
    "lynx",
    "macaw",
    "meerkat",
    "mole",
    "moose",
    "pufferfish",
    "mouse",
    "octopus",
    "ostrich",
    "otter",
    "owl",
    "panda",
    "kiwi",
    "parrot",
    "peacock",
    "pelican",
    "penguin",
    "puffin",
    "rabbit",
    "raccoon",
    "rhino",
    "robin",
    "salmon",
    "seahorse",
    "seal",
    "shark",
    "sheep",
    "sloth",
    "snail",
    "sparrow",
    "squirrel",
    "starfish",
    "stork",
    "swan",
    "tiger",
    "snake",
    "toucan",
    "turtle",
    "walrus",
    "whale",
    "wolf",
    "woodpecker",
    "yak",
    "zebra",
    "dog",
    "cow",
    "beaver",
];

/// Why discovery failed.
#[derive(Debug, thiserror::Error)]
pub enum DiscoveryError {
    #[error("the system's secure random source failed")]
    Random,
    #[error(
        "that name can't be used; use up to 32 letters, numbers, spaces and ordinary punctuation"
    )]
    InvalidName,
    #[error("local discovery is not available on this network: {0}")]
    Network(String),
}

impl From<mdns_sd::Error> for DiscoveryError {
    fn from(e: mdns_sd::Error) -> Self {
        Self::Network(e.to_string())
    }
}

/// How an old laptop is shown to the person while pairing.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Label {
    /// A colour and an animal, as indexes into [`COLOURS`] and [`ANIMALS`].
    Picked { colour: u8, animal: u8 },
    /// A name the person typed for this session.
    Named(String),
}

impl Label {
    /// A random colour and animal.
    pub fn random() -> Result<Self, DiscoveryError> {
        let n = random_below(COLOURS.len() * ANIMALS.len())?;
        Ok(Self::from_index(n))
    }

    /// A random colour and animal that no laptop nearby is using, for the Shuffle button. Pass the
    /// current label and every label seen nearby.
    pub fn shuffle(in_use: &[Label]) -> Result<Self, DiscoveryError> {
        let total = COLOURS.len() * ANIMALS.len();
        let free: Vec<usize> = (0..total)
            .filter(|&n| !in_use.contains(&Self::from_index(n)))
            .collect();
        if free.is_empty() {
            // Every label is taken nearby; any label still works, and duplicates are flagged.
            return Self::random();
        }
        Ok(Self::from_index(free[random_below(free.len())?]))
    }

    /// A name the person typed, for this session only. Spaces are tidied (none at the ends, one
    /// between words) and accents are put in the standard form; then the name must pass the
    /// allow-list described at the top of this module.
    pub fn named(typed: &str) -> Result<Self, DiscoveryError> {
        let composed: String = typed.nfc().collect();
        // Only spaces and tabs are tidied; line breaks and other spacing characters are refused.
        let name = composed
            .split([' ', '\t'])
            .filter(|word| !word.is_empty())
            .collect::<Vec<_>>()
            .join(" ");
        if is_acceptable_name(&name) {
            Ok(Self::Named(name))
        } else {
            Err(DiscoveryError::InvalidName)
        }
    }

    /// What two labels are compared by when flagging duplicates: the colour and animal, or the
    /// name ignoring case, accent form, width and joiners.
    pub fn comparison_key(&self) -> String {
        match self {
            Self::Picked { colour, animal } => format!("picked:{colour}:{animal}"),
            Self::Named(name) => {
                let lower: String = name
                    .nfkc()
                    .filter(|c| !JOINERS.contains(c))
                    .flat_map(char::to_lowercase)
                    .collect();
                format!("named:{}", lower.nfc().collect::<String>())
            }
        }
    }

    /// The colour and animal translation keys, for a picked label.
    pub fn keys(&self) -> Option<(&'static str, &'static str)> {
        match self {
            Self::Picked { colour, animal } => Some((
                COLOURS.get(usize::from(*colour))?,
                ANIMALS.get(usize::from(*animal))?,
            )),
            Self::Named(_) => None,
        }
    }

    /// The announcement fields for this label.
    pub fn to_txt(&self) -> Vec<(&'static str, String)> {
        let mut txt = vec![("v", FORMAT_VERSION.to_string())];
        match self {
            Self::Picked { colour, animal } => {
                txt.push(("c", colour.to_string()));
                txt.push(("a", animal.to_string()));
            }
            Self::Named(name) => txt.push(("n", name.clone())),
        }
        txt
    }

    /// Reads a label from all of an announcement's fields; `None` unless they are exactly the
    /// fields PCTwin writes, once each, in the form PCTwin writes them.
    pub fn from_txt(fields: &[(&str, &str)]) -> Option<Self> {
        // A field given twice is caught by the exact count check at the end.
        let get = |key: &str| fields.iter().find(|(k, _)| *k == key).map(|(_, v)| *v);
        if get("v")? != FORMAT_VERSION {
            return None;
        }
        let label = match (get("c"), get("a"), get("n")) {
            (Some(c), Some(a), None) => {
                let colour = canonical_index(c, COLOURS.len())?;
                let animal = canonical_index(a, ANIMALS.len())?;
                Self::Picked { colour, animal }
            }
            (None, None, Some(n)) if is_acceptable_name(n) => Self::Named(n.to_string()),
            _ => return None,
        };
        // Nothing else may be present, and no field twice. (The network library already keeps
        // only the first of any repeated field it receives; this also covers direct callers.)
        (fields.len() == label.to_txt().len()).then_some(label)
    }

    fn from_index(n: usize) -> Self {
        // `n` is below 1,200, so both parts fit in a u8.
        Self::Picked {
            colour: u8::try_from(n / ANIMALS.len()).unwrap_or(0),
            animal: u8::try_from(n % ANIMALS.len()).unwrap_or(0),
        }
    }
}

/// The allow-list for names: see the module documentation.
fn is_acceptable_name(name: &str) -> bool {
    if name.is_empty() || name.len() > MAX_NAME_BYTES || !is_nfc(name) {
        return false;
    }
    let chars: Vec<char> = name.chars().collect();
    let is_letter = |c: char| c.is_alphanumeric() && !is_invisible(c);
    let mut visible = 0;
    let mut marks_in_a_row = 0;
    for (i, &c) in chars.iter().enumerate() {
        let previous = i.checked_sub(1).map(|j| chars[j]);
        let next = chars.get(i + 1).copied();
        if is_invisible(c) {
            return false;
        }
        if is_combining_mark(c) {
            // A mark sits on a letter or on another mark, and does not pile up.
            marks_in_a_row += 1;
            if marks_in_a_row > MAX_MARKS_IN_A_ROW
                || !previous.is_some_and(|p| is_letter(p) || is_combining_mark(p))
            {
                return false;
            }
            continue;
        }
        marks_in_a_row = 0;
        if JOINERS.contains(&c) {
            // Inside a word only: after a letter or mark, before a letter.
            let after_letter = previous.is_some_and(|p| is_letter(p) || is_combining_mark(p));
            if !after_letter || !next.is_some_and(is_letter) {
                return false;
            }
        } else if c == ' ' {
            // Single spaces between words.
            if previous.is_none_or(|p| p == ' ') || next.is_none() {
                return false;
            }
            visible += 1;
        } else if is_letter(c) || c.is_ascii_punctuation() || EXTRA_PUNCTUATION.contains(&c) {
            visible += 1;
        } else {
            return false;
        }
    }
    let has_letter = chars.iter().any(|&c| is_letter(c));
    has_letter && visible <= MAX_NAME_CHARS
}

/// Letters and marks that Unicode treats as invisible ("default ignorable") or that render blank,
/// which `is_alphanumeric` and `is_combining_mark` would otherwise let through.
fn is_invisible(c: char) -> bool {
    matches!(
        c,
        '\u{034F}'
            | '\u{115F}'
            | '\u{1160}'
            | '\u{17B4}'
            | '\u{17B5}'
            | '\u{180B}'..='\u{180F}'
            | '\u{3164}'
            | '\u{FE00}'..='\u{FE0F}'
            | '\u{FFA0}'
            | '\u{E0100}'..='\u{E01EF}'
    )
}

/// Parses a number written exactly as PCTwin writes it (no sign, no leading zeros) below `limit`.
fn canonical_index(s: &str, limit: usize) -> Option<u8> {
    let n: u8 = s.parse().ok()?;
    (usize::from(n) < limit && n.to_string() == s).then_some(n)
}

fn random_below(limit: usize) -> Result<usize, DiscoveryError> {
    // Rejection sampling over u32 keeps the choice uniform.
    let limit = u32::try_from(limit).map_err(|_| DiscoveryError::Random)?;
    let zone = u32::MAX - (u32::MAX % limit);
    loop {
        let mut b = [0u8; 4];
        getrandom::fill(&mut b).map_err(|_| DiscoveryError::Random)?;
        let n = u32::from_le_bytes(b);
        if n < zone {
            return usize::try_from(n % limit).map_err(|_| DiscoveryError::Random);
        }
    }
}

fn random_tag() -> Result<String, DiscoveryError> {
    let mut b = [0u8; 8];
    getrandom::fill(&mut b).map_err(|_| DiscoveryError::Random)?;
    Ok(b.iter().map(|x| format!("{x:02x}")).collect())
}

/// An old laptop found nearby.
#[derive(Debug, Clone)]
pub struct Found {
    /// Its label, as shown on its screen.
    pub label: Label,
    /// Where to connect, local-network addresses only.
    pub addrs: Vec<SocketAddr>,
    /// Another laptop nearby shows the same label: ask the person to check the old laptop's screen.
    pub same_label_nearby: bool,
}

/// Local discovery on this laptop.
pub struct Discovery {
    daemon: ServiceDaemon,
}

impl Discovery {
    /// Starts local discovery.
    pub fn new() -> Result<Self, DiscoveryError> {
        Ok(Self {
            daemon: ServiceDaemon::new()?,
        })
    }

    /// Announces this old laptop with `label` and the pairing `port`, until the returned
    /// [`Announcement`] is dropped. Only announce while pairing.
    pub fn announce(&self, label: &Label, port: u16) -> Result<Announcement, DiscoveryError> {
        let tag = random_tag()?;
        let instance = format!("pctwin-{tag}");
        let host = format!("pctwin-{tag}.local.");
        let txt = label.to_txt();
        let props: Vec<(&str, &str)> = txt.iter().map(|(k, v)| (*k, v.as_str())).collect();
        let info = ServiceInfo::new(SERVICE_TYPE, &instance, &host, "", port, &props[..])?
            .enable_addr_auto();
        let fullname = info.get_fullname().to_string();
        self.daemon.register(info)?;
        Ok(Announcement {
            daemon: self.daemon.clone(),
            fullname,
        })
    }

    /// Looks for old laptops for `wait`, and returns what was found.
    pub fn browse(&self, wait: Duration) -> Result<Vec<Found>, DiscoveryError> {
        let events = self.daemon.browse(SERVICE_TYPE)?;
        let deadline = Instant::now() + wait;
        let mut by_instance: HashMap<String, Found> = HashMap::new();
        loop {
            let left = deadline.saturating_duration_since(Instant::now());
            if left.is_zero() {
                break;
            }
            match events.recv_timeout(left) {
                Ok(ServiceEvent::ServiceResolved(info)) => {
                    let fullname = info.get_fullname().to_string();
                    match read_announcement(&info) {
                        Some(found) => {
                            by_instance.insert(fullname, found);
                        }
                        // An instance whose announcement turned invalid is no longer listed.
                        None => {
                            by_instance.remove(&fullname);
                        }
                    }
                }
                Ok(ServiceEvent::ServiceRemoved(_, fullname)) => {
                    by_instance.remove(&fullname);
                }
                Ok(_) => {}
                Err(_) => break,
            }
        }
        let _ = self.daemon.stop_browse(SERVICE_TYPE);
        let mut found: Vec<Found> = by_instance.into_values().collect();
        let keys: Vec<String> = found.iter().map(|f| f.label.comparison_key()).collect();
        for f in &mut found {
            let key = f.label.comparison_key();
            f.same_label_nearby = keys.iter().filter(|k| **k == key).count() > 1;
        }
        Ok(found)
    }
}

/// Reads one announcement: its label and usable addresses, or `None` if either is missing.
fn read_announcement(info: &ResolvedService) -> Option<Found> {
    let mut fields = Vec::new();
    for prop in info.get_properties().iter() {
        // A field without a value, or not valid UTF-8, is not something PCTwin writes.
        let value = std::str::from_utf8(prop.val()?).ok()?;
        fields.push((prop.key(), value));
    }
    let label = Label::from_txt(&fields)?;
    let addrs = usable_addrs(info.get_addresses().iter(), info.get_port());
    (!addrs.is_empty()).then_some(Found {
        label,
        addrs,
        same_label_nearby: false,
    })
}

/// Keeps local-network addresses that are not loopback, IPv4 first, at most [`MAX_ADDRS`]. An
/// IPv6 link-local address is only usable with its interface, so it keeps its scope.
fn usable_addrs<'a>(ips: impl Iterator<Item = &'a ScopedIp>, port: u16) -> Vec<SocketAddr> {
    let mut addrs: Vec<SocketAddr> = ips
        .filter_map(|ip| {
            let plain = ip.to_ip_addr();
            if plain.is_loopback() || !is_local_peer(plain) {
                return None;
            }
            match (ip, plain) {
                (ScopedIp::V6(v6), IpAddr::V6(addr)) if addr.is_unicast_link_local() => {
                    let scope = v6.scope_id().index;
                    (scope != 0).then(|| SocketAddr::V6(SocketAddrV6::new(addr, port, 0, scope)))
                }
                (_, IpAddr::V6(addr))
                    if addr.to_ipv4_mapped().is_some_and(|v4| v4.is_loopback()) =>
                {
                    None
                }
                _ => Some(SocketAddr::new(plain, port)),
            }
        })
        .collect();
    addrs.sort_by_key(|a| (a.is_ipv6(), *a));
    addrs.dedup();
    addrs.truncate(MAX_ADDRS);
    addrs
}

impl Drop for Discovery {
    fn drop(&mut self) {
        let _ = self.daemon.shutdown();
    }
}

/// A running announcement. Dropping it says goodbye on the network and stops announcing.
pub struct Announcement {
    daemon: ServiceDaemon,
    fullname: String,
}

impl Drop for Announcement {
    fn drop(&mut self) {
        // The goodbye is queued at once; waiting for confirmation was shown to add nothing.
        let _ = self.daemon.unregister(&self.fullname);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn v4(s: &str) -> ScopedIp {
        ScopedIp::from(s.parse::<IpAddr>().unwrap())
    }

    #[test]
    fn only_usable_local_addresses_are_kept() {
        let ips = [
            v4("192.168.1.20"),
            v4("127.0.0.1"),
            v4("::1"),
            v4("::ffff:127.0.0.1"),
            v4("8.8.8.8"),
            v4("10.0.0.5"),
            v4("fd00::5"),
            v4("fe80::1"), // no interface known: cannot be used
        ];
        let kept = usable_addrs(ips.iter(), 4000);
        let kept: Vec<String> = kept.iter().map(ToString::to_string).collect();
        assert_eq!(
            kept,
            ["10.0.0.5:4000", "192.168.1.20:4000", "[fd00::5]:4000"]
        );
    }

    #[test]
    fn ipv4_addresses_are_kept_first_when_there_are_too_many() {
        let mut ips: Vec<ScopedIp> = (1..=7).map(|i| v4(&format!("fd00::{i}"))).collect();
        ips.insert(3, v4("192.168.1.9"));
        ips.push(v4("10.0.0.2"));
        ips.push(v4("10.0.0.1"));
        let kept = usable_addrs(ips.iter(), 4000);
        assert_eq!(kept.len(), MAX_ADDRS);
        let v4_kept = kept.iter().filter(|a| a.is_ipv4()).count();
        assert_eq!(v4_kept, 3, "{kept:?}");
        assert!(kept[..3].iter().all(SocketAddr::is_ipv4));
    }

    #[test]
    fn no_more_than_eight_addresses_are_kept() {
        let ips: Vec<ScopedIp> = (1..=64).map(|i| v4(&format!("10.0.0.{i}"))).collect();
        assert_eq!(usable_addrs(ips.iter(), 4000).len(), MAX_ADDRS);
    }
}
