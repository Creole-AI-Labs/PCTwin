//! The helper program itself, over its real private channel (Windows: the named pipe only the
//! mover's account can open). Started here without administrator rights, as this account; the
//! real prompt cannot be clicked in a test.

#[cfg(windows)]
#[test]
fn the_program_serves_one_session_from_the_mover_and_ends() {
    use pctwin_helper::protocol::{read_from_helper, send_request};
    use pctwin_helper::{Answer, FromHelper, Refusal, Request};
    use pctwin_helper_os::{Account, PipeClient, PipeName, current_account};

    let Account::Sid(me) = current_account().unwrap() else {
        panic!("a Windows account is a SID");
    };
    let name = PipeName::random().unwrap();
    let mut helper = std::process::Command::new(env!("CARGO_BIN_EXE_pctwin-helper"))
        .args([
            "--session",
            "the-token",
            "--mover",
            me.as_str(),
            "--approve",
            "S-1-5-21-1-2-3-4242",
            "--channel",
            name.as_str(),
        ])
        .spawn()
        .unwrap();
    // The pipe exists once the program has made it.
    let client = (0..200)
        .find_map(|_| {
            PipeClient::connect(&name).ok().or_else(|| {
                std::thread::sleep(std::time::Duration::from_millis(25));
                None
            })
        })
        .expect("the helper made its pipe");
    let mut stream = client.stream();
    for r in [
        Request::Hello {
            session: "the-token".into(),
        },
        Request::ScanPerson {
            account: "S-1-5-21-1-2-3-4242".into(),
        },
        Request::Bye,
    ] {
        send_request(&mut stream, &r).unwrap();
    }
    let answers: Vec<FromHelper> = (0..3)
        .map(|_| read_from_helper(&mut stream).unwrap())
        .collect();
    assert_eq!(
        answers,
        vec![
            FromHelper::Answer(Answer::Ready),
            // Approved, but no such person on this machine.
            FromHelper::Answer(Answer::Refused(Refusal::NoHome)),
            FromHelper::Answer(Answer::Goodbye),
        ]
    );
    assert!(helper.wait().unwrap().success());
}

/// Anything it does not understand on its command line ends it before any channel is made.
#[test]
fn the_program_refuses_a_command_line_it_does_not_understand() {
    let out = std::process::Command::new(env!("CARGO_BIN_EXE_pctwin-helper"))
        .args(["--run", "cmd"])
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(2));
}
