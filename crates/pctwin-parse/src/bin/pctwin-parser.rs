//! The parser process: started by the app (or the administrator helper) with no arguments and
//! an empty environment; it speaks only over its own standard input and output
//! (`pctwin_parse::wire`).

use std::process::ExitCode;

fn main() -> ExitCode {
    if std::env::args_os().len() > 1 {
        // Started by hand or with something to do: it takes no commands.
        return ExitCode::from(2);
    }
    let code = pctwin_parse::child::serve(std::io::stdin().lock(), std::io::stdout().lock());
    ExitCode::from(code)
}
