//! A stand-in for the parser process that breaks the rules on purpose, for the tests of the
//! app's side only (never staged or shipped). What it does is chosen by the request's size:
//!
//! - 1: stops dead in the middle of the request;
//! - 2: never answers;
//! - 3: asks for a piece outside the file;
//! - 4: asks for the whole file again and again, past its bound;
//! - 5: answers another question;
//! - 6: says something that is not a frame of the protocol;
//! - 7: answers as the real parser does, then says more after its answer;
//! - 8 or more, starting with a path: tries to open the file at the path the bytes name (first
//!   line), connect to the port they name on this laptop (second line), and start a program, and
//!   says what was refused, as a font's family names;
//! - anything else: answers as the real parser does.

use std::io::Write;
use std::process::ExitCode;

use pctwin_parse::wire::*;

fn main() -> ExitCode {
    let mut input = std::io::stdin().lock();
    let mut output = std::io::stdout().lock();
    let hello = Hello {
        protocol: PROTOCOL,
        confinement: pctwin_parse::lockdown::confine_self(),
    };
    if write_json(&mut output, TAG_HELLO, &hello).is_err() {
        return ExitCode::from(2);
    }
    loop {
        let Ok(Some((TAG_REQUEST, body))) = read_frame(&mut input, MAX_JSON) else {
            return ExitCode::SUCCESS;
        };
        let Ok(request) = json::<Request>(&body) else {
            return ExitCode::from(2);
        };
        match request.size() {
            1 => std::process::abort(),
            2 => loop {
                std::thread::sleep(std::time::Duration::from_secs(60));
            },
            3 => {
                let _ = write_json(
                    &mut output,
                    TAG_READ_AT,
                    &ReadAt {
                        offset: 100,
                        len: 10,
                    },
                );
                let _ = read_frame(&mut input, MAX_FRAME);
            }
            4 => loop {
                if write_json(&mut output, TAG_READ_AT, &ReadAt { offset: 0, len: 4 }).is_err()
                    || read_frame(&mut input, MAX_FRAME).is_err()
                {
                    return ExitCode::from(2);
                }
            },
            5 => {
                let _ = write_json(
                    &mut output,
                    TAG_ANSWER,
                    &Answer::Archive(pctwin_readiness::checks::Shape::Sound),
                );
            }
            6 => {
                let _ = output.write_all(b"\xff\xff\xff\xff\xff not a frame");
                let _ = output.flush();
            }
            n if n >= 8 && first_byte_is_path(&mut input, &mut output, n) => {}
            7 => {
                let answer = pctwin_parse::child::answer(
                    pctwin_parse_seal::in_child(),
                    request,
                    &mut input,
                    &mut output,
                );
                let _ = write_json(&mut output, TAG_ANSWER, &answer);
                let _ = write_json(&mut output, TAG_READ_AT, &ReadAt { offset: 0, len: 1 });
            }
            _ => {
                let answer = pctwin_parse::child::answer(
                    pctwin_parse_seal::in_child(),
                    request,
                    &mut input,
                    &mut output,
                );
                if write_json(&mut output, TAG_ANSWER, &answer).is_err() {
                    return ExitCode::from(2);
                }
            }
        }
    }
}

/// Size 8 and more: the walls' test (see the notes above). True when it was that test.
fn first_byte_is_path(
    input: &mut impl std::io::Read,
    output: &mut impl std::io::Write,
    n: u64,
) -> bool {
    use std::io::Read as _;
    let seal = pctwin_parse_seal::in_child();
    let mut bytes = Vec::new();
    {
        let mut file = pctwin_parse::child::Remote::new(seal, input, output, n);
        if file.read_to_end(&mut bytes).is_err() {
            return false;
        }
    }
    let text = String::from_utf8_lossy(&bytes).into_owned();
    let mut lines = text.lines();
    let (Some(path), Some(port)) = (lines.next(), lines.next()) else {
        return false;
    };
    if !(path.contains('\\') || path.starts_with('/')) {
        return false;
    }
    let mut tried = Vec::new();
    #[expect(
        clippy::disallowed_methods,
        reason = "the walls' test: this must be refused"
    )]
    let file = std::fs::File::open(path.trim());
    tried.push(if file.is_err() {
        "file refused"
    } else {
        "file opened"
    });
    let socket = port.trim().parse::<u16>().ok().map(|p| {
        std::net::TcpStream::connect_timeout(
            &([127, 0, 0, 1], p).into(),
            std::time::Duration::from_secs(2),
        )
    });
    tried.push(match socket {
        Some(Ok(_)) => "socket connected",
        _ => "socket refused",
    });
    #[expect(
        clippy::disallowed_methods,
        reason = "the walls' test: this must be refused"
    )]
    let program = std::process::Command::new(std::env::current_exe().unwrap_or_default())
        .arg("--must-not-start")
        .spawn();
    tried.push(match program {
        Ok(mut c) => {
            let _ = c.kill();
            "program started"
        }
        Err(_) => "program refused",
    });
    let answer = Answer::Font(pctwin_readiness::checks::FontNames {
        is_font: false,
        families: tried.into_iter().map(String::from).collect(),
    });
    let _ = write_json(output, TAG_ANSWER, &answer);
    true
}
