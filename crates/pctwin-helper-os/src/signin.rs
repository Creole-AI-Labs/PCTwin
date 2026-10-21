//! Handing a person's files over at their own next sign-in (Security Design part C, `HandOver`):
//! the default once the move is over, so the hand-over runs as the person, with their rights
//! only, in their own session. The administrator's direct hand-over (`HandOverNow`) is the
//! fallback.
//!
//! - Windows: the person's held folder is the person's to read (it was made with them in its
//!   access list); the mover's access is taken away and the folder and everything in it given to
//!   the administrators, never through a link (`icacls /L`). A Task Scheduler task runs the
//!   hand-over when that person signs in, as them, only while they are signed in (`/IT`), so it
//!   needs no password.
//! - macOS and Linux: the held folder is given to the person by handle
//!   ([`crate::holding::give_to`]). An entry in the system's own folder (macOS
//!   `/Library/LaunchAgents`, Linux `/etc/xdg/autostart`, both root's) starts the hand-over in
//!   every sign-in; for anyone but that person the held folder cannot be opened, and it ends at
//!   once. Nothing is written into the person's home with an administrator's rights.
//!
//! Each entry is named `PCTwin hand-over <key>`; [`clean`] removes the entries whose held folder
//! is gone (handed over), so nothing outlives the hand-over for longer than the next move.

use std::path::{Path, PathBuf};

use crate::{Account, Error};

/// The prefix of every entry's name.
const PREFIX: &str = "pctwin-handover-";

fn check_key(key: &str) -> Result<(), Error> {
    if key.is_empty() || key.len() > 32 || !key.bytes().all(|b| b.is_ascii_alphanumeric()) {
        return Err(Error::BadInput("an entry key of letters and digits"));
    }
    Ok(())
}

#[cfg(windows)]
fn run(program: &Path, args: &[&str]) -> Result<String, Error> {
    let out = std::process::Command::new(program)
        .args(args)
        .stdin(std::process::Stdio::null())
        .output()?;
    if out.status.success() {
        Ok(String::from_utf8_lossy(&out.stdout).into_owned())
    } else {
        Err(Error::Os {
            call: "a system tool",
            code: i64::from(out.status.code().unwrap_or(-1)),
        })
    }
}

/// The local account name the system gives `sid` (read with PowerShell's `Get-LocalUser`, the
/// identifier checked first so nothing else can be said to the shell).
#[cfg(windows)]
fn name_of(sid: &crate::Sid) -> Result<String, Error> {
    let shell = system_tool(r"WindowsPowerShell\v1.0\powershell.exe");
    let out = run(
        &shell,
        &[
            "-NoProfile",
            "-NonInteractive",
            "-Command",
            &format!("(Get-LocalUser -SID '{}').Name", sid.as_str()),
        ],
    )?;
    let name = out.trim().to_string();
    if name.is_empty() || name.contains(['\\', '/', '"', '\'', '\n']) {
        return Err(Error::NotSupported("that account has no local name"));
    }
    Ok(name)
}

/// Windows: gives `folder` and everything in it to the administrators and takes away the access
/// `accounts` were given (the mover's, the person's), never through a link, so nothing in it can be
/// changed or swapped for a link while an elevated hand-over reads it.
#[cfg(windows)]
pub fn lock_to_administrators(folder: &Path, accounts: &[&crate::Sid]) -> Result<(), Error> {
    let icacls = system_tool("icacls.exe");
    let folder_text = folder.to_string_lossy();
    run(
        &icacls,
        &[
            &folder_text,
            "/setowner",
            "*S-1-5-32-544",
            "/T",
            "/L",
            "/C",
            "/Q",
        ],
    )?;
    for sid in accounts {
        run(
            &icacls,
            &[
                &folder_text,
                "/remove:g",
                &format!("*{}", sid.as_str()),
                "/T",
                "/L",
                "/C",
                "/Q",
            ],
        )?;
    }
    Ok(())
}

#[cfg(windows)]
fn system_tool(name: &str) -> PathBuf {
    let root = std::env::var_os("SystemRoot").unwrap_or_else(|| r"C:\Windows".into());
    Path::new(&root).join("System32").join(name)
}

/// Makes the hand-over of `folder` run at `person`'s next sign-in, as them. `name` is the
/// account's name (Windows asks for it by name); `mover` loses its access to the folder;
/// `program` is `pctwin-handover`; `key` names the entry.
pub fn schedule(
    person: &Account,
    name: &str,
    mover: &Account,
    folder: &Path,
    program: &Path,
    key: &str,
) -> Result<(), Error> {
    check_key(key)?;
    #[cfg(windows)]
    {
        let (Account::Sid(person_sid), Account::Sid(mover)) = (person, mover) else {
            return Err(Error::BadInput("Windows accounts"));
        };
        // The scheduler takes an account by name: the one the helper made, or the system's own
        // name for an approved account ID.
        let looked_up;
        let name = if name.is_empty() {
            looked_up = name_of(person_sid)?;
            looked_up.as_str()
        } else {
            name
        };
        let icacls = system_tool("icacls.exe");
        let folder_text = folder.to_string_lossy();
        // Given to the administrators, never through a link (/L), then the mover's access
        // taken away, everything below too (/T).
        run(
            &icacls,
            &[
                &folder_text,
                "/setowner",
                "*S-1-5-32-544",
                "/T",
                "/L",
                "/C",
                "/Q",
            ],
        )?;
        run(
            &icacls,
            &[
                &folder_text,
                "/remove:g",
                &format!("*{}", mover.as_str()),
                "/T",
                "/L",
                "/C",
                "/Q",
            ],
        )?;
        let action = format!(
            "\"{}\" --holding \"{}\"",
            program.display(),
            folder.display()
        );
        if action.len() > 261 {
            return Err(Error::BadInput(
                "the hand-over's command is too long for the scheduler",
            ));
        }
        run(
            &system_tool("schtasks.exe"),
            &[
                "/Create",
                "/TN",
                &format!("PCTwin\\{PREFIX}{key}"),
                "/SC",
                "ONLOGON",
                "/RU",
                name,
                "/IT",
                "/TR",
                &action,
                "/F",
            ],
        )?;
        Ok(())
    }
    #[cfg(unix)]
    {
        let _ = name;
        let (Account::Uid(uid), Account::Uid(_)) = (person, mover) else {
            return Err(Error::BadInput("Unix accounts"));
        };
        let (gid, _) = crate::holding::passwd_entry(*uid)?;
        crate::holding::give_to(folder, *uid, gid)?;
        let (dir, file, body) = entry(key, folder, program);
        std::fs::create_dir_all(&dir)?;
        // Root's folder: written whole, then renamed into place, never over a link.
        let tmp = dir.join(format!(".{file}.tmp"));
        std::fs::write(&tmp, body)?;
        std::fs::rename(&tmp, dir.join(&file))?;
        Ok(())
    }
    #[cfg(not(any(windows, unix)))]
    Err(Error::NotSupported("a hand-over at sign-in"))
}

/// The entry's folder, file name and contents.
#[cfg(unix)]
fn entry(key: &str, folder: &Path, program: &Path) -> (PathBuf, String, String) {
    let esc = |p: &Path| {
        p.to_string_lossy()
            .replace('&', "&amp;")
            .replace('<', "&lt;")
            .replace('>', "&gt;")
    };
    if cfg!(target_os = "macos") {
        let label = format!("com.creoleailabs.pctwin.handover.{key}");
        (
            PathBuf::from("/Library/LaunchAgents"),
            format!("{label}.plist"),
            format!(
                "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n\
<!DOCTYPE plist PUBLIC \"-//Apple//DTD PLIST 1.0//EN\" \"http://www.apple.com/DTDs/PropertyList-1.0.dtd\">\n\
<plist version=\"1.0\"><dict>\n\
<key>Label</key><string>{label}</string>\n\
<key>ProgramArguments</key><array><string>{}</string><string>--holding</string><string>{}</string></array>\n\
<key>RunAtLoad</key><true/>\n\
<key>LimitLoadToSessionType</key><string>Aqua</string>\n\
</dict></plist>\n",
                esc(program),
                esc(folder)
            ),
        )
    } else {
        let quote = |p: &Path| format!("\"{}\"", p.to_string_lossy().replace('"', "\\\""));
        (
            PathBuf::from("/etc/xdg/autostart"),
            format!("{PREFIX}{key}.desktop"),
            format!(
                "[Desktop Entry]\nType=Application\nName=PCTwin hand-over\nExec={} --holding {}\nNoDisplay=true\nX-GNOME-Autostart-enabled=true\n",
                quote(program),
                quote(folder)
            ),
        )
    }
}

/// Removes the entries whose held folder is gone: the hand-over is done. `program` is
/// `pctwin-handover`. Best effort: an entry left is harmless (it finds nothing to do).
pub fn clean(staging_root: &Path) {
    #[cfg(windows)]
    {
        let Ok(list) = run(
            &system_tool("schtasks.exe"),
            &["/Query", "/FO", "CSV", "/NH"],
        ) else {
            return;
        };
        for line in list.lines() {
            let Some(name) = line.split(',').next().map(|n| n.trim_matches('"')) else {
                continue;
            };
            let Some(key) = name.strip_prefix(&format!("\\PCTwin\\{PREFIX}")) else {
                continue;
            };
            if check_key(key).is_err() {
                continue;
            }
            let task = format!("PCTwin\\{PREFIX}{key}");
            let still = run(
                &system_tool("schtasks.exe"),
                &["/Query", "/TN", &task, "/XML"],
            )
            .map(|xml| {
                xml.split("--holding")
                    .nth(1)
                    .and_then(|r| r.split('"').nth(1))
                    .is_some_and(|held| Path::new(held).exists())
            })
            .unwrap_or(true);
            if !still {
                let _ = run(
                    &system_tool("schtasks.exe"),
                    &["/Delete", "/TN", &task, "/F"],
                );
            }
        }
        let _ = staging_root;
    }
    #[cfg(unix)]
    {
        let dir = if cfg!(target_os = "macos") {
            Path::new("/Library/LaunchAgents")
        } else {
            Path::new("/etc/xdg/autostart")
        };
        let Ok(entries) = std::fs::read_dir(dir) else {
            return;
        };
        for e in entries.flatten() {
            let name = e.file_name().to_string_lossy().into_owned();
            let ours =
                name.starts_with(PREFIX) || name.starts_with("com.creoleailabs.pctwin.handover.");
            if !ours {
                continue;
            }
            let Ok(body) = std::fs::read_to_string(e.path()) else {
                continue;
            };
            // The held folder it names, if it is inside this laptop's staging folder.
            let held = body
                .split(&*staging_root.to_string_lossy())
                .nth(1)
                .map(|rest| {
                    let tail: String = rest
                        .chars()
                        .take_while(|c| !matches!(c, '"' | '<' | '\n'))
                        .collect();
                    staging_root.join(tail.trim_start_matches('/'))
                });
            if held.is_some_and(|h| !h.exists()) {
                let _ = std::fs::remove_file(e.path());
            }
        }
    }
}
