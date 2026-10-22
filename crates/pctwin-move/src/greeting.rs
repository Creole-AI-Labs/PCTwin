//! The first words of a move (C06), said the moment it starts and before the old laptop looks
//! for anything: both laptops agree the move protocol and say what they are (the system, the
//! chip, the PCTwin version, and whether they run on battery), so each screen can show the other
//! laptop at once.
//!
//! The old laptop then finds the person's files, which may take minutes and may wait on the
//! person (a Mac's privacy settings, an administrator's approval). The new laptop waits for
//! [`list_coming`] for as long as that takes: it is the person's time, not a sign the other laptop
//! went away (the link itself notices that). The list then follows under the usual time limits.

use pctwin_transfer::{
    Channel, ChannelError, MOVE_PROTOCOL, TransferError, VERSION_WAIT, agree_version,
};
use serde::{Deserialize, Serialize};

/// The message kind of what a laptop says it is, kept apart from the transfer's (1 to 20), the
/// inventory's (32, 33, 35), the version's (34), the link test's (36, 37) and the landing rules'
/// (39 to 41).
const ABOUT: u8 = 42;
/// The message kind of the old laptop's word that its list follows now.
const LIST_COMING: u8 = 43;
/// What a laptop says it is: a handful of short words, far below the link's message limit.
const MAX_ABOUT: usize = 512;
/// The longest chip name and version said.
const MAX_WORD: usize = 32;

/// The system a laptop runs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum System {
    Windows,
    Mac,
    Linux,
    Other,
}

impl System {
    /// This laptop's system.
    pub fn here() -> Self {
        if cfg!(windows) {
            Self::Windows
        } else if cfg!(target_os = "macos") {
            Self::Mac
        } else if cfg!(target_os = "linux") {
            Self::Linux
        } else {
            Self::Other
        }
    }
}

/// How a laptop is powered when the move starts.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case", deny_unknown_fields)]
pub enum Power {
    /// Plugged in (or a desktop).
    Plugged,
    /// On battery, with how full it is.
    Battery { percent: u8 },
    /// The laptop could not say.
    Unknown,
}

/// What a laptop says it is (C06).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct About {
    pub system: System,
    /// The chip, as the system names it ("x86_64", "aarch64").
    pub chip: String,
    /// The PCTwin version.
    pub app: String,
    pub power: Power,
}

impl About {
    /// This laptop, powered as `power` says.
    pub fn here(power: Power) -> Self {
        Self {
            system: System::here(),
            chip: std::env::consts::ARCH.to_string(),
            app: env!("CARGO_PKG_VERSION").to_string(),
            power,
        }
    }

    fn fine(&self) -> bool {
        let word = |w: &str| {
            !w.is_empty()
                && w.len() <= MAX_WORD
                && w.chars()
                    .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-' | '+'))
        };
        word(&self.chip)
            && word(&self.app)
            && !matches!(self.power, Power::Battery { percent } if percent > 100)
    }
}

/// What the two laptops agreed when the move started.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Greeting {
    /// The move protocol version both speak.
    pub version: u16,
    /// What the other laptop said it is.
    pub theirs: About,
}

/// A link the two laptops have greeted on, with what they agreed: the app's move runs only on
/// one ([`crate::send_by_rules`], [`crate::receive_by_rules`]). Otherwise just the link.
pub struct Greeted<'l, C> {
    link: &'l mut C,
    greeting: Greeting,
}

impl<C> Greeted<'_, C> {
    pub fn greeting(&self) -> &Greeting {
        &self.greeting
    }
}

impl<C: Channel> Channel for Greeted<'_, C> {
    async fn send(&mut self, data: &[u8]) -> Result<(), ChannelError> {
        self.link.send(data).await
    }

    async fn recv(&mut self) -> Result<Vec<u8>, ChannelError> {
        self.link.recv().await
    }
}

/// Both laptops, the moment the move starts: agree the protocol, then each says what it is.
pub async fn greet<'l, C: Channel>(
    link: &'l mut C,
    mine: &About,
) -> Result<Greeted<'l, C>, TransferError> {
    let greeting = greeting(link, mine).await?;
    Ok(Greeted { link, greeting })
}

async fn greeting(ch: &mut impl Channel, mine: &About) -> Result<Greeting, TransferError> {
    let version = agree_version(ch, MOVE_PROTOCOL).await?;
    let mut msg = vec![ABOUT];
    msg.extend_from_slice(
        &serde_json::to_vec(mine).map_err(|e| TransferError::Protocol(e.to_string()))?,
    );
    ch.send(&msg)
        .await
        .map_err(|_| TransferError::ConnectionDropped)?;
    let got = tokio::time::timeout(VERSION_WAIT, ch.recv())
        .await
        .map_err(|_| TransferError::NoAnswer("it did not say what it is".into()))?
        .map_err(|_| TransferError::ConnectionDropped)?;
    Ok(Greeting {
        version,
        theirs: parse_about(&got)?,
    })
}

/// Reads what the other laptop said it is, refusing anything else.
pub fn parse_about(msg: &[u8]) -> Result<About, TransferError> {
    let bad = |why: &str| TransferError::Protocol(format!("what it is: {why}"));
    let (&kind, body) = msg.split_first().ok_or_else(|| bad("empty"))?;
    if kind != ABOUT {
        return Err(bad("not what it is"));
    }
    if body.len() > MAX_ABOUT {
        return Err(bad("too long"));
    }
    let about: About = serde_json::from_slice(body).map_err(|_| bad("not readable"))?;
    if !about.fine() {
        return Err(bad("not plain words"));
    }
    Ok(about)
}

/// The old laptop, once it has found the files: its list follows now.
pub async fn list_coming(ch: &mut impl Channel) -> Result<(), TransferError> {
    ch.send(&[LIST_COMING])
        .await
        .map_err(|_| TransferError::ConnectionDropped)
}

/// The new laptop: waits, for as long as the old laptop's person takes, for its list to come.
pub async fn wait_for_list(ch: &mut impl Channel) -> Result<(), TransferError> {
    let got = ch
        .recv()
        .await
        .map_err(|_| TransferError::ConnectionDropped)?;
    if got != [LIST_COMING] {
        return Err(TransferError::Protocol(
            "it sent something before its list".into(),
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use pctwin_transfer::ChannelError;
    use tokio::sync::mpsc;

    struct Mem {
        tx: mpsc::UnboundedSender<Vec<u8>>,
        rx: mpsc::UnboundedReceiver<Vec<u8>>,
    }

    impl Channel for Mem {
        async fn send(&mut self, data: &[u8]) -> Result<(), ChannelError> {
            self.tx.send(data.to_vec()).map_err(|_| ChannelError)
        }
        async fn recv(&mut self) -> Result<Vec<u8>, ChannelError> {
            self.rx.recv().await.ok_or(ChannelError)
        }
    }

    fn pair() -> (Mem, Mem) {
        let (a_tx, b_rx) = mpsc::unbounded_channel();
        let (b_tx, a_rx) = mpsc::unbounded_channel();
        (Mem { tx: a_tx, rx: a_rx }, Mem { tx: b_tx, rx: b_rx })
    }

    fn about(system: System, power: Power) -> About {
        About {
            system,
            chip: "aarch64".into(),
            app: "0.1.0".into(),
            power,
        }
    }

    #[tokio::test]
    async fn each_laptop_hears_what_the_other_is() {
        let (mut old, mut new) = pair();
        let windows = about(System::Windows, Power::Battery { percent: 40 });
        let mac = about(System::Mac, Power::Plugged);
        let (a, b) = tokio::join!(greet(&mut old, &windows), greet(&mut new, &mac));
        let (a, b) = (a.unwrap(), b.unwrap());
        assert_eq!(a.greeting().theirs, mac);
        assert_eq!(b.greeting().theirs, windows);
        assert_eq!(a.greeting().version, b.greeting().version);
    }

    #[test]
    fn anything_but_plain_words_is_refused() {
        let msg = |json: &str| [&[ABOUT][..], json.as_bytes()].concat();
        let good = r#"{"system":"linux","chip":"x86_64","app":"0.1.0","power":{"kind":"unknown"}}"#;
        assert!(parse_about(&msg(good)).is_ok());
        for bad in [
            r#"{"system":"linux","chip":"x86 64","app":"0.1.0","power":{"kind":"unknown"}}"#,
            r#"{"system":"linux","chip":"x86_64","app":"","power":{"kind":"unknown"}}"#,
            r#"{"system":"linux","chip":"x86_64","app":"0.1.0","power":{"kind":"battery","percent":101}}"#,
            r#"{"system":"amiga","chip":"x86_64","app":"0.1.0","power":{"kind":"unknown"}}"#,
            r#"{"system":"linux","chip":"x86_64","app":"0.1.0","power":{"kind":"unknown"},"x":1}"#,
        ] {
            assert!(parse_about(&msg(bad)).is_err(), "{bad}");
        }
        let long = format!(
            r#"{{"system":"linux","chip":"x86_64","app":"{}","power":{{"kind":"unknown"}}}}"#,
            "1".repeat(MAX_ABOUT)
        );
        assert!(parse_about(&msg(&long)).is_err());
        assert!(parse_about(&[LIST_COMING]).is_err());
        assert!(parse_about(&[]).is_err());
    }

    /// The new laptop waits as long as the old laptop's person takes: far past any time limit
    /// on the list itself.
    #[tokio::test(start_paused = true)]
    async fn the_new_laptop_waits_for_the_list_as_long_as_it_takes() {
        let (mut old, mut new) = pair();
        let late = async {
            tokio::time::sleep(std::time::Duration::from_secs(3600)).await;
            list_coming(&mut old).await.unwrap();
            old
        };
        let (got, _old) = tokio::join!(wait_for_list(&mut new), late);
        got.unwrap();
    }

    #[tokio::test]
    async fn anything_before_the_list_is_refused() {
        let (mut old, mut new) = pair();
        old.send(&[32, 0]).await.unwrap();
        assert!(wait_for_list(&mut new).await.is_err());
        drop(old);
        assert!(matches!(
            wait_for_list(&mut new).await,
            Err(TransferError::ConnectionDropped)
        ));
    }
}
