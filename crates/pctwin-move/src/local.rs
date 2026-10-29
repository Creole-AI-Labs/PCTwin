//! A move inside one laptop: both sides in this app, joined by memory instead of a network (Task
//! List 2.6, the dead-laptop rescue: a drive connected to this laptop is the old laptop, and this
//! laptop is the new one). The same protocol, checks, safety gate and journal as a move between
//! two laptops; nothing leaves the laptop.

use pctwin_transfer::{Channel, ChannelError};
use tokio::sync::mpsc;

/// One end of an in-laptop link.
#[derive(Debug)]
pub struct LocalEnd {
    tx: mpsc::UnboundedSender<Vec<u8>>,
    rx: mpsc::UnboundedReceiver<Vec<u8>>,
}

/// Two joined ends: what one sends, the other receives, in order.
pub fn pair() -> (LocalEnd, LocalEnd) {
    let (a_tx, b_rx) = mpsc::unbounded_channel();
    let (b_tx, a_rx) = mpsc::unbounded_channel();
    (
        LocalEnd { tx: a_tx, rx: a_rx },
        LocalEnd { tx: b_tx, rx: b_rx },
    )
}

impl Channel for LocalEnd {
    async fn send(&mut self, data: &[u8]) -> Result<(), ChannelError> {
        // Lets the other side run between messages, as a network would.
        tokio::task::yield_now().await;
        self.tx.send(data.to_vec()).map_err(|_| ChannelError)
    }

    async fn recv(&mut self) -> Result<Vec<u8>, ChannelError> {
        self.rx.recv().await.ok_or(ChannelError)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// What one end sends arrives at the other, in order; once one end is gone the other is told.
    #[tokio::test]
    async fn the_two_ends_carry_messages_both_ways_and_say_when_the_other_went() {
        let (mut a, mut b) = pair();
        a.send(b"one").await.unwrap();
        a.send(b"two").await.unwrap();
        assert_eq!(b.recv().await.unwrap(), b"one");
        assert_eq!(b.recv().await.unwrap(), b"two");
        b.send(b"back").await.unwrap();
        assert_eq!(a.recv().await.unwrap(), b"back");
        drop(a);
        assert!(b.recv().await.is_err());
        assert!(b.send(b"x").await.is_err());
    }
}
