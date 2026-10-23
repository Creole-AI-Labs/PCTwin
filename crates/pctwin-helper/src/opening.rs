//! macOS: how the launch daemon takes on one move (Security Design part C, "The administrator's
//! approval on a Mac"). The system starts the daemon, not the app, so nothing on its command line
//! says who is moving or who was approved. The first message on its socket must be
//! [`Request::Open`]: the administrator's approval for this move, the session's token, the people
//! approved and the laptop's ID. The mover is the account the kernel says connected, never what
//! the app says; the approval must hold PCTwin's right, asked of macOS (passed in here as
//! `holds`, so every path is tested on every system). Anything else is refused.

use std::collections::BTreeSet;
use std::io::{Read, Write};

use pctwin_helper_os::approval::Approval;
use pctwin_record::LaptopId;

use crate::protocol::{Answer, FrameError, Refusal, Request, read_request, send_answer};

/// One move, opened.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Opening {
    pub session: String,
    pub approve: BTreeSet<String>,
    pub laptop: LaptopId,
}

/// Why a connection did not open a move.
#[derive(Debug, PartialEq, Eq)]
pub enum NotOpened {
    /// It did not ask in the expected form (the reason is for the helper's own log).
    Malformed(String),
    /// The approval does not hold PCTwin's right (no administrator approved, or not now).
    NotApproved,
    /// The channel failed.
    Channel(String),
}

/// The 32 bytes of an approval from its 64 hexadecimal characters.
fn approval_from_hex(text: &str) -> Option<Approval> {
    if text.len() != 64 || !text.bytes().all(|b| b.is_ascii_hexdigit()) {
        return None;
    }
    let mut bytes = [0u8; 32];
    for (i, b) in bytes.iter_mut().enumerate() {
        *b = u8::from_str_radix(&text[i * 2..i * 2 + 2], 16).ok()?;
    }
    Some(Approval(bytes))
}

/// The approval's 64 hexadecimal characters (the app's side).
pub fn approval_to_hex(approval: &Approval) -> String {
    approval.0.iter().map(|b| format!("{b:02x}")).collect()
}

/// Tells the app `why`, and gives back `not`.
fn refuse(channel: &mut impl Write, why: Refusal, not: NotOpened) -> Result<Opening, NotOpened> {
    let _ = send_answer(channel, &Answer::Refused(why));
    Err(not)
}

/// Reads the first message from `channel`, sent by the account `mover` (as the kernel reports
/// it), and opens the move it asks for if `holds` says the approval holds PCTwin's right. A
/// refusal is told to the app before it is returned.
pub fn open(
    channel: &mut (impl Read + Write),
    mover: &str,
    holds: &mut dyn FnMut(&Approval) -> Result<bool, String>,
) -> Result<Opening, NotOpened> {
    let request = match read_request(channel) {
        Ok(r) => r,
        Err(FrameError::Io(e)) => return Err(NotOpened::Channel(e.to_string())),
        Err(e) => {
            return refuse(
                channel,
                Refusal::NoSession,
                NotOpened::Malformed(e.to_string()),
            );
        }
    };
    let Request::Open {
        approval,
        session,
        approve,
        laptop,
    } = request
    else {
        return refuse(
            channel,
            Refusal::NoSession,
            NotOpened::Malformed("the first message was not Open".into()),
        );
    };
    let malformed = |why: &str| NotOpened::Malformed(why.to_string());
    let Some(approval) = approval_from_hex(&approval) else {
        return refuse(channel, Refusal::NotApproved, malformed("not an approval"));
    };
    if session.is_empty() || session.len() > 128 || session.chars().any(char::is_control) {
        return refuse(channel, Refusal::WrongSession, malformed("not a session"));
    }
    let approve: BTreeSet<String> = approve.into_iter().filter(|a| !a.is_empty()).collect();
    if approve.is_empty() || approve.len() > 64 || approve.contains(mover) {
        return refuse(
            channel,
            Refusal::NotApproved,
            malformed("the people approved are the other people, never the mover"),
        );
    }
    let Ok(laptop) = LaptopId::from_hex(&laptop) else {
        return refuse(channel, Refusal::NoSession, malformed("not a laptop ID"));
    };
    match holds(&approval) {
        Ok(true) => Ok(Opening {
            session,
            approve,
            laptop,
        }),
        Ok(false) => refuse(channel, Refusal::NotApproved, NotOpened::NotApproved),
        Err(why) => refuse(channel, Refusal::NotApproved, NotOpened::Malformed(why)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocol::{FromHelper, read_from_helper, send_request};
    use std::io::Cursor;

    struct Channel {
        from_app: Cursor<Vec<u8>>,
        to_app: Vec<u8>,
    }
    impl Read for Channel {
        fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
            self.from_app.read(buf)
        }
    }
    impl Write for Channel {
        fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
            self.to_app.write(buf)
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }

    fn channel(r: &Request) -> Channel {
        let mut wire = Vec::new();
        send_request(&mut wire, r).unwrap();
        Channel {
            from_app: Cursor::new(wire),
            to_app: Vec::new(),
        }
    }

    fn opening(approve: &[&str]) -> Request {
        Request::Open {
            approval: approval_to_hex(&Approval([0xab; 32])),
            session: "token".into(),
            approve: approve.iter().map(|a| a.to_string()).collect(),
            laptop: "1".repeat(32),
        }
    }

    fn told(c: &Channel) -> Option<FromHelper> {
        let mut read = c.to_app.as_slice();
        (!read.is_empty()).then(|| read_from_helper(&mut read).unwrap())
    }

    #[test]
    fn an_approved_opening_opens_the_move_with_the_approval_checked() {
        let mut c = channel(&opening(&["502", "503"]));
        let mut seen = None;
        let o = open(&mut c, "501", &mut |a| {
            seen = Some(a.clone());
            Ok(true)
        })
        .unwrap();
        assert_eq!(seen, Some(Approval([0xab; 32])));
        assert_eq!(o.session, "token");
        assert_eq!(o.approve.len(), 2);
        // Nothing said yet: the session answers the Hello that follows.
        assert_eq!(told(&c), None);
    }

    #[test]
    fn an_approval_macos_does_not_confirm_is_refused() {
        let mut c = channel(&opening(&["502"]));
        assert_eq!(
            open(&mut c, "501", &mut |_| Ok(false)),
            Err(NotOpened::NotApproved)
        );
        assert_eq!(
            told(&c),
            Some(FromHelper::Answer(Answer::Refused(Refusal::NotApproved)))
        );
        let mut c = channel(&opening(&["502"]));
        assert!(open(&mut c, "501", &mut |_| Err("not an approval".into())).is_err());
    }

    #[test]
    fn the_mover_is_never_among_the_people_approved_and_someone_must_be() {
        for approve in [&["501"][..], &["502", "501"], &[], &[""]] {
            let mut c = channel(&opening(approve));
            let mut asked = false;
            let r = open(&mut c, "501", &mut |_| {
                asked = true;
                Ok(true)
            });
            assert!(matches!(r, Err(NotOpened::Malformed(_))), "{approve:?}");
            assert!(!asked, "macOS is asked only about a well-formed opening");
        }
    }

    #[test]
    fn anything_but_a_well_formed_opening_is_refused() {
        let mut bad = vec![Request::Hello {
            session: "token".into(),
        }];
        for (approval, session, laptop) in [
            ("ab".repeat(31), "token", "1".repeat(32)),
            ("zz".repeat(32), "token", "1".repeat(32)),
            ("ab".repeat(32), "", "1".repeat(32)),
            ("ab".repeat(32), "a\nb", "1".repeat(32)),
            ("ab".repeat(32), "token", "x".into()),
        ] {
            bad.push(Request::Open {
                approval,
                session: session.into(),
                approve: vec!["502".into()],
                laptop,
            });
        }
        for r in bad {
            let mut c = channel(&r);
            let r2 = open(&mut c, "501", &mut |_| Ok(true));
            assert!(matches!(r2, Err(NotOpened::Malformed(_))), "{r:?}");
            assert!(matches!(
                told(&c),
                Some(FromHelper::Answer(Answer::Refused(_)))
            ));
        }
    }

    #[test]
    fn approvals_go_to_hex_and_back() {
        let a = Approval(std::array::from_fn(|i| i as u8));
        assert_eq!(approval_from_hex(&approval_to_hex(&a)), Some(a));
    }
}
