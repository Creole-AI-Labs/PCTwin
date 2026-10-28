//! For other crates' tests: the real parser program, built if this test run has not built it.

use std::path::PathBuf;
use std::sync::OnceLock;

use crate::{PARSER_PROGRAM, Parser};

/// The real parser program in this build's own folder (`target/<profile>/`), built with cargo
/// the first time a test asks if it is not there yet.
pub fn parser_program() -> PathBuf {
    static PROGRAM: OnceLock<PathBuf> = OnceLock::new();
    PROGRAM
        .get_or_init(|| {
            let exe = std::env::current_exe().expect("the test program's path");
            // target/<profile>/deps/<test>.exe -> target/<profile>/
            let profile_dir = exe
                .parent()
                .and_then(|deps| deps.parent())
                .expect("a test program in target/<profile>/deps")
                .to_path_buf();
            let program = profile_dir.join(PARSER_PROGRAM);
            if !program.exists() {
                let cargo = std::env::var_os("CARGO").unwrap_or_else(|| "cargo".into());
                let target_dir = profile_dir.parent().expect("target folder");
                let mut build = vec!["build", "-p", "pctwin-parse", "--bin", "pctwin-parser"];
                if profile_dir.file_name().is_some_and(|n| n == "release") {
                    build.push("--release");
                }
                #[expect(
                    clippy::disallowed_methods,
                    reason = "tests only: building the parser program"
                )]
                let status = std::process::Command::new(cargo)
                    .args(build)
                    .arg("--target-dir")
                    .arg(target_dir)
                    .current_dir(env!("CARGO_MANIFEST_DIR"))
                    .status()
                    .expect("cargo runs");
                assert!(status.success(), "the parser program builds");
            }
            program
        })
        .clone()
}

/// A parser process for a test, shared by the whole test program.
pub fn parser() -> &'static Parser {
    static PARSER: OnceLock<Parser> = OnceLock::new();
    PARSER.get_or_init(|| Parser::at(parser_program()))
}

/// Answers `request` about `bytes` through the parser process's own code path ([`crate::child`],
/// piece by piece as over the pipes), with an app made of memory: for fuzzing and property tests
/// on the stable compiler, without starting a process.
pub fn answer_in_memory(request: crate::wire::Request, bytes: &[u8]) -> crate::wire::Answer {
    use std::cell::RefCell;
    use std::collections::VecDeque;
    use std::io::{self, Read, Write};

    use crate::wire::{
        MAX_FRAME, ReadAt, TAG_BYTES, TAG_READ_AT, TAG_READ_FAILED, json, read_frame, write_frame,
    };

    struct App<'a> {
        file: &'a [u8],
        written: Vec<u8>,
        replies: VecDeque<u8>,
    }
    struct ToApp<'a, 'b>(&'b RefCell<App<'a>>);
    struct FromApp<'a, 'b>(&'b RefCell<App<'a>>);

    impl Write for ToApp<'_, '_> {
        fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
            self.0.borrow_mut().written.extend_from_slice(buf);
            Ok(buf.len())
        }
        fn flush(&mut self) -> io::Result<()> {
            let mut app = self.0.borrow_mut();
            let mut reader = io::Cursor::new(std::mem::take(&mut app.written));
            while let Ok(Some((tag, body))) = read_frame(&mut reader, MAX_FRAME) {
                let mut reply = Vec::new();
                match (tag, json::<ReadAt>(&body)) {
                    (TAG_READ_AT, Ok(ask)) => {
                        let start = usize::try_from(ask.offset).unwrap_or(usize::MAX);
                        match app.file.get(start..) {
                            Some(rest) => {
                                let n = rest.len().min(ask.len as usize);
                                write_frame(&mut reply, TAG_BYTES, &rest[..n])?;
                            }
                            None => write_frame(&mut reply, TAG_READ_FAILED, &[])?,
                        }
                    }
                    _ => write_frame(&mut reply, TAG_READ_FAILED, &[])?,
                }
                app.replies.extend(reply);
            }
            Ok(())
        }
    }

    impl Read for FromApp<'_, '_> {
        fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
            let mut app = self.0.borrow_mut();
            let n = buf.len().min(app.replies.len());
            for (b, r) in buf.iter_mut().zip(app.replies.drain(..n)) {
                *b = r;
            }
            Ok(n)
        }
    }

    let app = RefCell::new(App {
        file: bytes,
        written: Vec::new(),
        replies: VecDeque::new(),
    });
    crate::child::answer(request, &mut FromApp(&app), &mut ToApp(&app))
}
