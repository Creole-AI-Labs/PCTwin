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
