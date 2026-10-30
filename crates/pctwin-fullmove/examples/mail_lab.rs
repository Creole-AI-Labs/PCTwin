//! The two-laptop lane for Apple Mail (CI, `.github/workflows/ci.yml`: `mail-lab-send` and
//! `mail-lab-receive`).
//!
//! - `mail_lab send <package>` (a macOS runner, the old laptop): builds a synthetic Mac's
//!   `~/Library/Mail/V10` (mailboxes On My Mac, a POP account, an IMAP account, an account
//!   removed from Mail, the index with rows in its log), plans and captures it as the old laptop
//!   does, writes the stream it would send into `<package>`, and checks the Mac's mail is byte
//!   for byte what it was.
//! - `mail_lab receive <package> <evidence>` (Windows and Linux runners, the new laptop): reads
//!   the stream into a staging folder as the new laptop does, writes the mail out through the
//!   real parser process and the safety gate, and checks that only the mail kept on the Mac
//!   arrived, as mbox files read back whole, the IMAP account is a sign-in line, and the staging
//!   folder is gone. What it checked is written to `<evidence>/check.txt`.
#![allow(clippy::disallowed_methods)]

#[path = "../tests/mail_fixture/mod.rs"]
mod mail_fixture;

use std::path::Path;

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args
        .iter()
        .map(String::as_str)
        .collect::<Vec<_>>()
        .as_slice()
    {
        ["send", package] => send(Path::new(package)),
        ["receive", package, evidence] => receive(Path::new(package), Path::new(evidence)),
        _ => {
            eprintln!("use: mail_lab send <package> | mail_lab receive <package> <evidence>");
            std::process::exit(2);
        }
    }
}

fn send(package: &Path) {
    let t = tempfile::tempdir().unwrap();
    let home = t.path().join("old");
    mail_fixture::old_mac(&home);
    let before = mail_fixture::tree(&home.join("Library/Mail"));
    let line = mail_fixture::send(&home, package);
    assert_eq!(
        mail_fixture::tree(&home.join("Library/Mail")),
        before,
        "the old Mac's mail is only read"
    );
    std::fs::write(
        package.join("origin.txt"),
        format!(
            "{} ({}): {} components, {} files, {} bytes\n",
            std::env::consts::OS,
            std::env::consts::ARCH,
            line.ids.len(),
            line.files,
            line.bytes
        ),
    )
    .unwrap();
    println!(
        "sent: {} components, the old Mac's mail unchanged",
        line.ids.len()
    );
}

fn receive(package: &Path, evidence: &Path) {
    // What the old Mac held is made again here (the same synthetic Mac), only to know what to
    // expect; nothing of it is read by the new laptop's side.
    let t = tempfile::tempdir().unwrap();
    let expect = mail_fixture::old_mac(&t.path().join("expected"));
    let home = t.path().join("new");
    let work = t.path().join("work");
    std::fs::create_dir_all(&work).unwrap();
    let r = mail_fixture::receive(package, &home, &work);
    let report = mail_fixture::check(&expect, &home, &r);
    std::fs::create_dir_all(evidence).unwrap();
    let origin = std::fs::read_to_string(package.join("origin.txt")).unwrap_or_default();
    let text = format!(
        "sent from: {origin}received on: {} ({})\n{report}",
        std::env::consts::OS,
        std::env::consts::ARCH
    );
    std::fs::write(evidence.join("check.txt"), &text).unwrap();
    print!("{text}");
}
