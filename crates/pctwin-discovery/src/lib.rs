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

use mdns_sd::{IfKind, ResolvedService, ScopedIp, ServiceDaemon, ServiceEvent, ServiceInfo};
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

/// The pairing protocol versions this release speaks, announced (`pv`, comma-separated) so a new
/// laptop can tell before connecting whether the two can pair, and say "update PCTwin on the
/// other laptop" rather than fail a handshake. Each release speaks its own and the one before
/// (Engineering Plan 9.2); today there is one.
pub const PAIRING_VERSIONS: &[u8] = &[1];

/// The pairing versions an announcement says it speaks: the `pv` field, or version 1 for one
/// that says nothing (the first release).
fn pairing_versions_in(fields: &[(&str, &str)]) -> Option<Vec<u8>> {
    match fields.iter().find(|(k, _)| *k == "pv") {
        None => Some(vec![1]),
        Some((_, v)) => {
            let list: Option<Vec<u8>> = v.split(',').map(|n| canonical_index(n, 256)).collect();
            list.filter(|l| !l.is_empty() && l.len() <= 8 && !l.contains(&0))
        }
    }
}

/// The newest pairing version both laptops speak, if any.
pub fn common_pairing_version(theirs: &[u8]) -> Option<u8> {
    PAIRING_VERSIONS
        .iter()
        .filter(|v| theirs.contains(v))
        .max()
        .copied()
}

/// The announcement's fields: the label's, then the pairing versions.
fn announced_txt(label: &Label) -> Vec<(&'static str, String)> {
    let mut txt = label.to_txt();
    let versions: Vec<String> = PAIRING_VERSIONS.iter().map(u8::to_string).collect();
    txt.push(("pv", versions.join(",")));
    txt
}
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
    '\u{3010}', '\u{3011}', '\u{300A}', '\u{300B}', '\u{300C}',
    '\u{300D}', // 【】《》「」
    '\u{FF1B}', '\u{FF5E}', '\u{FFE5}', '\u{00A5}', // full-width ; ~ and yen sign, yen sign
    '\u{0964}', '\u{0965}', // Devanagari danda and double danda
    '\u{061B}', // Arabic semicolon
];
/// Spaces that look like a normal space and arrive when text is pasted (no-break, narrow no-break,
/// ideographic). When typed they become ordinary spaces; from the network they are refused.
const PASTED_SPACES: [char; 3] = ['\u{00A0}', '\u{202F}', '\u{3000}'];

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
        let composed: String = typed
            .nfc()
            .map(|c| if PASTED_SPACES.contains(&c) { ' ' } else { c })
            .collect();
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
                let folded: String = name.chars().map(fold_lookalike).collect();
                let lower: String = folded
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
        // Nothing else may be present but the pairing versions, and no field twice. (The network
        // library already keeps only the first of any repeated field it receives; this also
        // covers direct callers.)
        let pv = usize::from(fields.iter().any(|(k, _)| *k == "pv"));
        (fields.len() == label.to_txt().len() + pv).then_some(label)
    }

    fn from_index(n: usize) -> Self {
        // `n` is below 1,200, so both parts fit in a u8.
        Self::Picked {
            colour: u8::try_from(n / ANIMALS.len()).unwrap_or(0),
            animal: u8::try_from(n % ANIMALS.len()).unwrap_or(0),
        }
    }
}

/// Punctuation from other scripts that looks like other punctuation, folded so lookalike names are
/// flagged as the same.
fn fold_lookalike(c: char) -> String {
    match c {
        '\u{0964}' => "|".into(),        // danda
        '\u{0965}' => "||".into(),       // double danda
        '\u{300A}' => "\u{00AB}".into(), // 《 as «
        '\u{300B}' => "\u{00BB}".into(), // 》 as »
        '\u{061B}' => ";".into(),        // Arabic semicolon
        other => other.to_string(),
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
    /// The pairing versions it speaks: with no version in common
    /// ([`common_pairing_version`] is `None`), the screen asks to update PCTwin on the older
    /// laptop instead of trying to pair.
    pub pairing_versions: Vec<u8>,
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
        let txt = announced_txt(label);
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

    /// As [`announce`](Self::announce), but only on the connection named `interface` and with
    /// only its addresses `ips`: nothing is announced on, or points to, any other connection of
    /// this laptop (a VPN, a virtual machine's network).
    pub fn announce_on(
        &self,
        label: &Label,
        port: u16,
        ips: &[IpAddr],
        interface: &str,
    ) -> Result<Announcement, DiscoveryError> {
        self.daemon.disable_interface(IfKind::All)?;
        self.daemon
            .enable_interface(IfKind::Name(interface.to_string()))?;
        let tag = random_tag()?;
        let instance = format!("pctwin-{tag}");
        let host = format!("pctwin-{tag}.local.");
        let txt = announced_txt(label);
        let props: Vec<(&str, &str)> = txt.iter().map(|(k, v)| (*k, v.as_str())).collect();
        let info = ServiceInfo::new(SERVICE_TYPE, &instance, &host, ips, port, &props[..])?;
        let fullname = info.get_fullname().to_string();
        self.daemon.register(info)?;
        Ok(Announcement {
            daemon: self.daemon.clone(),
            fullname,
        })
    }

    /// Announces one remembered connection's current token (Security Design Part T), with the
    /// port this laptop listens on for remembered connections, until the returned
    /// [`Announcement`] is dropped. Nothing else is announced: no label, no name. Each token is
    /// its own announcement under its own random names, but all of one laptop's announcements
    /// carry the same addresses and port, so anyone on the network can group them by where they
    /// come from (and count them); the tokens say nothing about who the connections are.
    /// Announce only on `ips` (an empty list: the system's).
    pub fn announce_known(
        &self,
        token: &[u8; KNOWN_TOKEN_LEN],
        port: u16,
        ips: &[IpAddr],
    ) -> Result<Announcement, DiscoveryError> {
        let tag = random_tag()?;
        let instance = format!("pctwin-{tag}");
        let host = format!("pctwin-{tag}.local.");
        let token_hex: String = token.iter().map(|b| format!("{b:02x}")).collect();
        let props = [("v", FORMAT_VERSION), ("t", token_hex.as_str())];
        let info = if ips.is_empty() {
            ServiceInfo::new(KNOWN_SERVICE_TYPE, &instance, &host, "", port, &props[..])?
                .enable_addr_auto()
        } else {
            ServiceInfo::new(KNOWN_SERVICE_TYPE, &instance, &host, ips, port, &props[..])?
        };
        let fullname = info.get_fullname().to_string();
        self.daemon.register(info)?;
        Ok(Announcement {
            daemon: self.daemon.clone(),
            fullname,
        })
    }

    /// Looks for the `wanted` tokens (the remembered laptops this laptop is looking for) for
    /// the whole of `wait`: each one seen, and where to connect (local-network addresses only).
    /// Every other announcement is passed over, however many there are, so a crowded network or
    /// a flood of fake announcements never ends the look early; a token copied by someone else
    /// keeps at most [`MAX_NAMES_PER_TOKEN`] announcements, each with its addresses once.
    pub fn browse_known(
        &self,
        wanted: &[[u8; KNOWN_TOKEN_LEN]],
        wait: Duration,
    ) -> Result<Vec<FoundKnown>, DiscoveryError> {
        let events = self.daemon.browse(KNOWN_SERVICE_TYPE)?;
        let deadline = Instant::now() + wait;
        let mut seen = KnownSeen::new(wanted);
        loop {
            let left = deadline.saturating_duration_since(Instant::now());
            if left.is_zero() {
                break;
            }
            match events.recv_timeout(left) {
                Ok(ServiceEvent::ServiceResolved(info)) => {
                    seen.resolved(info.get_fullname(), read_known(&info));
                }
                Ok(ServiceEvent::ServiceRemoved(_, name)) => seen.removed(&name),
                Ok(_) => {}
                Err(_) => break,
            }
        }
        let _ = self.daemon.stop_browse(KNOWN_SERVICE_TYPE);
        Ok(seen.found())
    }

    /// Looks for old laptops for `wait`, and returns what was found.
    pub fn browse(&self, wait: Duration) -> Result<Vec<Found>, DiscoveryError> {
        let events = self.daemon.browse(SERVICE_TYPE)?;
        let deadline = Instant::now() + wait;
        let mut nearby = Nearby::default();
        loop {
            let left = deadline.saturating_duration_since(Instant::now());
            if left.is_zero() {
                break;
            }
            match events.recv_timeout(left) {
                Ok(ServiceEvent::ServiceResolved(info)) => {
                    nearby.seen(info.get_fullname().to_string(), read_announcement(&info))
                }
                Ok(ServiceEvent::ServiceRemoved(_, fullname)) => nearby.gone(&fullname),
                Ok(_) => {}
                Err(_) => break,
            }
        }
        let _ = self.daemon.stop_browse(SERVICE_TYPE);
        Ok(nearby.list())
    }
}

/// The old laptops a search has seen so far, by the name each announcement is made under; what
/// the network said last about each one counts.
#[derive(Default)]
struct Nearby {
    by_instance: HashMap<String, Found>,
}

impl Nearby {
    /// `instance` announced itself: listed as `found`, or no longer listed when its announcement
    /// is not one PCTwin would make (`None`), even if it was listed before.
    fn seen(&mut self, instance: String, found: Option<Found>) {
        match found {
            Some(found) => {
                self.by_instance.insert(instance, found);
            }
            None => {
                self.by_instance.remove(&instance);
            }
        }
    }

    /// `instance` said goodbye (or its announcement ran out): no longer listed.
    fn gone(&mut self, instance: &str) {
        self.by_instance.remove(instance);
    }

    /// The list, each one flagged when another shows the same label.
    fn list(&self) -> Vec<Found> {
        let mut found: Vec<Found> = self.by_instance.values().cloned().collect();
        let keys: Vec<String> = found.iter().map(|f| f.label.comparison_key()).collect();
        for f in &mut found {
            let key = f.label.comparison_key();
            f.same_label_nearby = keys.iter().filter(|k| **k == key).count() > 1;
        }
        found
    }
}

/// The multicast DNS service type remembered connections announce their tokens under.
pub const KNOWN_SERVICE_TYPE: &str = "_pctwin-k._tcp.local.";
/// Bytes in a remembered connection's token.
pub const KNOWN_TOKEN_LEN: usize = 8;
/// Most announcements one look keeps for one wanted token (someone else can copy a token seen on
/// the network and announce it with their own addresses).
pub const MAX_NAMES_PER_TOKEN: usize = 8;
/// Most addresses one announcement may give.
const MAX_ADDRS_PER_NAME: usize = 8;

/// What one look for remembered laptops has seen: only the wanted tokens, each from at most
/// [`MAX_NAMES_PER_TOKEN`] announcements (by name), so memory stays small whatever is announced.
#[derive(Debug, Default)]
struct KnownSeen {
    wanted: Vec<[u8; KNOWN_TOKEN_LEN]>,
    /// Announcement name to its token and addresses, in the order first seen.
    names: Vec<(String, [u8; KNOWN_TOKEN_LEN], Vec<SocketAddr>)>,
}

impl KnownSeen {
    fn new(wanted: &[[u8; KNOWN_TOKEN_LEN]]) -> Self {
        Self {
            wanted: wanted.to_vec(),
            names: Vec::new(),
        }
    }

    /// The announcement `name` resolved to `found` (`None`: not in PCTwin's form).
    fn resolved(&mut self, name: &str, found: Option<FoundKnown>) {
        let at = self.names.iter().position(|(n, _, _)| n == name);
        let Some(found) = found.filter(|f| self.wanted.contains(&f.token)) else {
            // Junk, or a token nobody here wants: passed over (and any earlier form removed).
            if let Some(at) = at {
                self.names.remove(at);
            }
            return;
        };
        let mut addrs: Vec<SocketAddr> = Vec::new();
        for a in found.addrs {
            if !addrs.contains(&a) && addrs.len() < MAX_ADDRS_PER_NAME {
                addrs.push(a);
            }
        }
        match at {
            Some(at) => self.names[at] = (name.to_string(), found.token, addrs),
            None => {
                let held = self
                    .names
                    .iter()
                    .filter(|(_, t, _)| *t == found.token)
                    .count();
                if held < MAX_NAMES_PER_TOKEN {
                    self.names.push((name.to_string(), found.token, addrs));
                }
            }
        }
    }

    fn removed(&mut self, name: &str) {
        self.names.retain(|(n, _, _)| n != name);
    }

    /// Each wanted token seen, with every address it was seen at, each once.
    fn found(self) -> Vec<FoundKnown> {
        let mut out: Vec<FoundKnown> = Vec::new();
        for (_, token, addrs) in self.names {
            let at = match out.iter().position(|f| f.token == token) {
                Some(at) => at,
                None => {
                    out.push(FoundKnown {
                        token,
                        addrs: Vec::new(),
                    });
                    out.len() - 1
                }
            };
            for a in addrs {
                if !out[at].addrs.contains(&a) {
                    out[at].addrs.push(a);
                }
            }
        }
        out
    }
}

/// A remembered connection's token seen nearby.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FoundKnown {
    pub token: [u8; KNOWN_TOKEN_LEN],
    pub addrs: Vec<SocketAddr>,
}

/// Reads one token announcement: exactly the fields PCTwin writes (`v` and `t`, once each, the
/// token as 16 lower-case hex characters) and usable addresses, or `None`.
fn read_known(info: &ResolvedService) -> Option<FoundKnown> {
    let mut fields = Vec::new();
    for prop in info.get_properties().iter() {
        fields.push((prop.key(), std::str::from_utf8(prop.val()?).ok()?));
    }
    known_fields(&fields).and_then(|token| {
        let addrs = usable_addrs(info.get_addresses().iter(), info.get_port());
        (!addrs.is_empty()).then_some(FoundKnown { token, addrs })
    })
}

/// The token in a token announcement's fields, read strictly.
fn known_fields(fields: &[(&str, &str)]) -> Option<[u8; KNOWN_TOKEN_LEN]> {
    if fields.len() != 2 {
        return None;
    }
    let get = |k: &str| {
        let mut it = fields.iter().filter(|(key, _)| *key == k);
        match (it.next(), it.next()) {
            (Some((_, v)), None) => Some(*v),
            _ => None,
        }
    };
    if get("v")? != FORMAT_VERSION {
        return None;
    }
    let t = get("t")?;
    if t.len() != 2 * KNOWN_TOKEN_LEN
        || !t
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
    {
        return None;
    }
    let mut token = [0u8; KNOWN_TOKEN_LEN];
    for (i, o) in token.iter_mut().enumerate() {
        *o = u8::from_str_radix(&t[2 * i..2 * i + 2], 16).ok()?;
    }
    Some(token)
}

/// The lowest port an announcement may name: below it are ports only system services use (and 0,
/// which no listener has), never a PCTwin listener's.
const FIRST_USABLE_PORT: u16 = 1024;

/// Reads one announcement: its label and usable addresses, or `None` if either is missing.
fn read_announcement(info: &ResolvedService) -> Option<Found> {
    let mut fields = Vec::new();
    for prop in info.get_properties().iter() {
        // A field without a value, or not valid UTF-8, is not something PCTwin writes.
        let value = std::str::from_utf8(prop.val()?).ok()?;
        fields.push((prop.key(), value));
    }
    let label = Label::from_txt(&fields)?;
    let pairing_versions = pairing_versions_in(&fields)?;
    let addrs = usable_addrs(info.get_addresses().iter(), info.get_port());
    (!addrs.is_empty()).then_some(Found {
        label,
        addrs,
        same_label_nearby: false,
        pairing_versions,
    })
}

/// Keeps local-network addresses that are not loopback, IPv4 first, at most [`MAX_ADDRS`]. An
/// IPv6 link-local address is only usable with its interface, so it keeps its scope.
fn usable_addrs<'a>(ips: impl Iterator<Item = &'a ScopedIp>, port: u16) -> Vec<SocketAddr> {
    // PCTwin listens on a port the system gives it: never 0, never one only system services use.
    if port < FIRST_USABLE_PORT {
        return Vec::new();
    }
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

    fn laptop(name: &str, port: u16) -> Found {
        Found {
            label: Label::named(name).unwrap(),
            addrs: vec![SocketAddr::from(([192, 168, 1, 20], port))],
            same_label_nearby: false,
            pairing_versions: PAIRING_VERSIONS.to_vec(),
        }
    }

    fn ports(found: &[Found]) -> Vec<u16> {
        let mut p: Vec<u16> = found.iter().map(|f| f.addrs[0].port()).collect();
        p.sort_unstable();
        p
    }

    fn found(token: u8, addrs: &[&str]) -> Option<FoundKnown> {
        Some(FoundKnown {
            token: [token; KNOWN_TOKEN_LEN],
            addrs: addrs.iter().map(|a| a.parse().unwrap()).collect(),
        })
    }

    /// Thousands of fake or unwanted announcements never end the look or crowd out the one
    /// wanted; a copied token keeps a bounded number of announcements, each address once.
    #[test]
    fn a_look_passes_over_junk_and_keeps_only_what_it_wants() {
        let mut seen = KnownSeen::new(&[[7; KNOWN_TOKEN_LEN]]);
        for n in 0..5000u32 {
            seen.resolved(
                &format!("junk-{n}"),
                found((n % 7) as u8, &["10.0.0.9:4000"]),
            );
            seen.resolved(&format!("bad-{n}"), None);
        }
        assert!(seen.names.is_empty());
        seen.resolved("real", found(7, &["192.168.1.5:5000", "192.168.1.5:5000"]));
        for n in 0..100 {
            seen.resolved(
                &format!("copy-{n}"),
                found(7, &["10.0.0.66:5000", "10.0.0.66:5000"]),
            );
        }
        assert_eq!(seen.names.len(), MAX_NAMES_PER_TOKEN);
        // The real one, seen again, keeps its place.
        seen.resolved("real", found(7, &["192.168.1.5:5000"]));
        let out = seen.found();
        assert_eq!(out.len(), 1);
        let want: Vec<SocketAddr> = vec![
            "192.168.1.5:5000".parse().unwrap(),
            "10.0.0.66:5000".parse().unwrap(),
        ];
        assert_eq!(out[0].addrs, want);
    }

    #[test]
    fn a_wanted_announcement_that_goes_or_turns_to_junk_is_dropped() {
        let mut seen = KnownSeen::new(&[[7; KNOWN_TOKEN_LEN]]);
        seen.resolved("a", found(7, &["192.168.1.5:5000"]));
        seen.resolved("b", found(7, &["192.168.1.6:5000"]));
        seen.removed("a");
        seen.resolved("b", None);
        assert!(seen.found().is_empty());
    }

    #[test]
    fn a_token_announcement_is_read_only_in_its_exact_form() {
        assert_eq!(
            known_fields(&[("v", "1"), ("t", "0011223344556677")]),
            Some([0x00, 0x11, 0x22, 0x33, 0x44, 0x55, 0x66, 0x77])
        );
        assert_eq!(
            known_fields(&[("t", "0011223344556677"), ("v", "1")]),
            Some([0x00, 0x11, 0x22, 0x33, 0x44, 0x55, 0x66, 0x77])
        );
        for bad in [
            vec![("v", "1")],
            vec![("v", "2"), ("t", "0011223344556677")],
            vec![("v", "1"), ("t", "0011223344556677"), ("n", "Ada")],
            vec![("v", "1"), ("t", "00112233445566")],
            vec![("v", "1"), ("t", "00112233445566778")],
            vec![("v", "1"), ("t", "00112233445566AA")],
            vec![("v", "1"), ("t", "001122334455667g")],
            vec![("v", "1"), ("v", "1")],
            vec![("t", "0011223344556677"), ("t", "0011223344556677")],
        ] {
            assert_eq!(known_fields(&bad), None, "{bad:?}");
        }
    }

    #[test]
    fn a_laptop_that_says_goodbye_during_the_search_is_not_listed() {
        let mut nearby = Nearby::default();
        nearby.seen("pctwin-a".into(), Some(laptop("Ada's laptop", 4000)));
        nearby.seen("pctwin-b".into(), Some(laptop("Bo's laptop", 4001)));
        nearby.gone("pctwin-a");
        assert_eq!(ports(&nearby.list()), [4001]);
    }

    #[test]
    fn a_laptop_whose_announcement_turns_invalid_is_not_listed() {
        let mut nearby = Nearby::default();
        nearby.seen("pctwin-a".into(), Some(laptop("Ada's laptop", 4000)));
        nearby.seen("pctwin-a".into(), None);
        assert!(nearby.list().is_empty());
        // Invalid first, then a valid one: listed again; the last word counts.
        nearby.seen("pctwin-a".into(), Some(laptop("Ada's laptop", 4002)));
        assert_eq!(ports(&nearby.list()), [4002]);
    }

    #[test]
    fn a_goodbye_from_a_laptop_never_listed_changes_nothing() {
        let mut nearby = Nearby::default();
        nearby.seen("pctwin-a".into(), Some(laptop("Ada's laptop", 4000)));
        nearby.gone("pctwin-z");
        nearby.seen("pctwin-y".into(), None);
        assert_eq!(ports(&nearby.list()), [4000]);
    }

    #[test]
    fn a_label_shown_twice_flags_both_and_one_leaving_clears_it() {
        let mut nearby = Nearby::default();
        nearby.seen("pctwin-a".into(), Some(laptop("Same name", 4000)));
        nearby.seen("pctwin-b".into(), Some(laptop("same NAME", 4001)));
        assert!(nearby.list().iter().all(|f| f.same_label_nearby));
        nearby.gone("pctwin-b");
        let left = nearby.list();
        assert_eq!(ports(&left), [4000]);
        assert!(!left[0].same_label_nearby);
    }

    #[test]
    fn the_pairing_versions_are_announced_and_read_back() {
        let label = Label::named("Kitchen laptop").unwrap();
        let txt = announced_txt(&label);
        let fields: Vec<(&str, &str)> = txt.iter().map(|(k, v)| (*k, v.as_str())).collect();
        assert_eq!(Label::from_txt(&fields), Some(label.clone()));
        assert_eq!(
            pairing_versions_in(&fields),
            Some(PAIRING_VERSIONS.to_vec())
        );
        // The first release said nothing: version 1.
        let plain = label.to_txt();
        let plain: Vec<(&str, &str)> = plain.iter().map(|(k, v)| (*k, v.as_str())).collect();
        assert_eq!(pairing_versions_in(&plain), Some(vec![1]));
        // Only a short list of plain numbers, never 0.
        for bad in ["", "0", "1,", "01", "1,x", "1,2,3,4,5,6,7,8,9", "300"] {
            let mut f = plain.clone();
            f.push(("pv", bad));
            assert_eq!(pairing_versions_in(&f), None, "{bad:?}");
        }
        assert_eq!(common_pairing_version(&[1, 2]), Some(1));
        assert_eq!(common_pairing_version(&[7]), None);
    }

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

    /// PCTwin listens on a port the system gives it, never port 0 and never one of the ports
    /// below 1024 that only system services use: an announcement naming one is not PCTwin's
    /// and is not followed.
    #[test]
    fn an_announcement_on_port_0_or_a_system_port_is_not_followed() {
        let ips = [v4("192.168.1.20")];
        for port in [0, 1, 22, 80, 443, 1023] {
            assert!(usable_addrs(ips.iter(), port).is_empty(), "{port}");
        }
        assert_eq!(usable_addrs(ips.iter(), 1024).len(), 1);
        assert_eq!(usable_addrs(ips.iter(), 65535).len(), 1);
    }

    #[test]
    fn no_more_than_eight_addresses_are_kept() {
        let ips: Vec<ScopedIp> = (1..=64).map(|i| v4(&format!("10.0.0.{i}"))).collect();
        assert_eq!(usable_addrs(ips.iter(), 4000).len(), MAX_ADDRS);
    }
}
