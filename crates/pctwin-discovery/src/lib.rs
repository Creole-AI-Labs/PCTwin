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
//! Everything that arrives is checked like any other input: unknown versions, out-of-range or
//! non-canonical numbers, over-long names and names with control or text-direction characters are
//! ignored, and only local-network addresses are kept.

use std::collections::HashMap;
use std::net::SocketAddr;
use std::time::{Duration, Instant};

use mdns_sd::{ServiceDaemon, ServiceEvent, ServiceInfo};
use pctwin_link::is_local_peer;

/// The multicast DNS service type PCTwin old laptops announce while pairing.
pub const SERVICE_TYPE: &str = "_pctwin._tcp.local.";
/// Longest name a person may type, in characters.
pub const MAX_NAME_CHARS: usize = 32;
/// Version of the announcement format.
const FORMAT_VERSION: &str = "1";

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
    #[error("that name can't be used; use up to 32 ordinary letters, numbers and spaces")]
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

    /// A name the person typed, for this session only. Leading and trailing spaces are removed.
    /// Refused if empty, longer than [`MAX_NAME_CHARS`], or containing control or
    /// text-direction characters (which could make one name look like another).
    pub fn named(typed: &str) -> Result<Self, DiscoveryError> {
        let name = typed.trim();
        if is_acceptable_name(name) {
            Ok(Self::Named(name.to_string()))
        } else {
            Err(DiscoveryError::InvalidName)
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

    /// Reads a label from announcement fields; `None` for anything not exactly as PCTwin writes it.
    pub fn from_txt<'a>(get: impl Fn(&str) -> Option<&'a str>) -> Option<Self> {
        if get("v")? != FORMAT_VERSION {
            return None;
        }
        match (get("c"), get("a"), get("n")) {
            (Some(c), Some(a), None) => {
                let colour = canonical_index(c, COLOURS.len())?;
                let animal = canonical_index(a, ANIMALS.len())?;
                Some(Self::Picked { colour, animal })
            }
            (None, None, Some(n)) => is_acceptable_name(n).then(|| Self::Named(n.to_string())),
            _ => None,
        }
    }

    fn from_index(n: usize) -> Self {
        // `n` is below 1,200, so both parts fit in a u8.
        Self::Picked {
            colour: u8::try_from(n / ANIMALS.len()).unwrap_or(0),
            animal: u8::try_from(n % ANIMALS.len()).unwrap_or(0),
        }
    }
}

fn is_acceptable_name(name: &str) -> bool {
    let chars = name.chars().count();
    (1..=MAX_NAME_CHARS).contains(&chars)
        && name.trim() == name
        && !name.chars().any(|c| c.is_control() || is_direction_mark(c))
}

/// Characters that change text direction or are invisible marks, used to disguise names.
fn is_direction_mark(c: char) -> bool {
    matches!(c, '\u{200B}'..='\u{200F}' | '\u{202A}'..='\u{202E}' | '\u{2060}'..='\u{206F}' | '\u{FEFF}')
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
                    let Some(label) = Label::from_txt(|k| info.get_property_val_str(k)) else {
                        continue;
                    };
                    let port = info.get_port();
                    let mut addrs: Vec<SocketAddr> = info
                        .get_addresses()
                        .iter()
                        .map(|ip| ip.to_ip_addr())
                        .filter(|ip| is_local_peer(*ip))
                        .map(|ip| SocketAddr::new(ip, port))
                        .collect();
                    addrs.sort();
                    if addrs.is_empty() {
                        continue;
                    }
                    by_instance.insert(
                        info.get_fullname().to_string(),
                        Found {
                            label,
                            addrs,
                            same_label_nearby: false,
                        },
                    );
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
        let labels: Vec<Label> = found.iter().map(|f| f.label.clone()).collect();
        for f in &mut found {
            f.same_label_nearby = labels.iter().filter(|l| **l == f.label).count() > 1;
        }
        Ok(found)
    }
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
        if let Ok(done) = self.daemon.unregister(&self.fullname) {
            // Wait briefly so the goodbye is sent before the daemon may shut down.
            let _ = done.recv_timeout(Duration::from_secs(1));
        }
    }
}
