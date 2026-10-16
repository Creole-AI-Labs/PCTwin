//! Reading who could open a place, without administrator rights and without opening anything
//! inside it. Only used on laptops with two or more people.
//!
//! - Windows: one PowerShell helper reads each folder's security descriptor (`Get-Acl`, the
//!   owner and access list only) and answers in SDDL, one line per folder, for the whole sweep.
//!   A folder whose permissions this person may not read is answered "unknown", never guessed.
//! - macOS and Linux: owner, group and mode come with the folder listing itself; the groups come
//!   from the system's group list.

use std::io::{BufRead, BufReader, Write};
use std::path::Path;
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::mpsc::{Receiver, RecvTimeoutError, channel};
use std::time::Duration;

use crate::access::{AccessFact, UnixGroups, access_from_mode, access_from_sddl};

/// Unix facts about one entry, from its listing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UnixMode {
    pub uid: u32,
    pub gid: u32,
    pub mode: u32,
}

/// Reads access for the sweep.
pub trait AccessReader {
    fn read(&mut self, path: &Path, unix: Option<UnixMode>) -> AccessFact;
}

/// The reader for this system.
pub fn system_reader() -> Box<dyn AccessReader + Send> {
    if cfg!(windows) {
        Box::new(WindowsAcl::default())
    } else {
        let group = getent("group").unwrap_or_else(|| read("/etc/group"));
        let passwd = getent("passwd").unwrap_or_else(|| read("/etc/passwd"));
        let mut groups = UnixGroups::from_files(&group, &passwd, cfg!(target_os = "macos"));
        if cfg!(target_os = "macos") {
            mac_groups(&mut groups);
        }
        Box::new(UnixAccess { groups })
    }
}

fn read(path: &str) -> String {
    std::fs::read_to_string(path).unwrap_or_default()
}

fn getent(what: &str) -> Option<String> {
    if cfg!(target_os = "macos") {
        return None;
    }
    let out = Command::new("getent")
        .arg(what)
        .stdin(Stdio::null())
        .stderr(Stdio::null())
        .output()
        .ok()?;
    let text = String::from_utf8_lossy(&out.stdout).into_owned();
    (!text.trim().is_empty()).then_some(text)
}

/// macOS keeps people and groups in its directory service, not in /etc.
fn mac_groups(groups: &mut UnixGroups) {
    let users = Command::new("dscl")
        .args([".", "-list", "/Users", "UniqueID"])
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).into_owned())
        .unwrap_or_default();
    let primary = Command::new("dscl")
        .args([".", "-list", "/Users", "PrimaryGroupID"])
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).into_owned())
        .unwrap_or_default();
    let gids = Command::new("dscl")
        .args([".", "-list", "/Groups", "PrimaryGroupID"])
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).into_owned())
        .unwrap_or_default();
    let members = Command::new("dscl")
        .args([".", "-list", "/Groups", "GroupMembership"])
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).into_owned())
        .unwrap_or_default();
    *groups = mac_groups_from_dscl(&users, &primary, &gids, &members);
}

/// Builds the groups from `dscl . -list` outputs (`name value...` per line).
pub fn mac_groups_from_dscl(users: &str, primary: &str, gids: &str, members: &str) -> UnixGroups {
    let pairs = |text: &str| -> Vec<(String, Vec<String>)> {
        text.lines()
            .filter_map(|l| {
                let mut w = l.split_whitespace();
                let name = w.next()?.to_string();
                Some((name, w.map(str::to_string).collect()))
            })
            .collect()
    };
    let mut passwd = String::new();
    let primary = pairs(primary);
    for (name, v) in pairs(users) {
        let gid = primary
            .iter()
            .find(|(n, _)| *n == name)
            .and_then(|(_, g)| g.first().cloned())
            .unwrap_or_else(|| "20".into());
        if let Some(uid) = v.first() {
            passwd.push_str(&format!("{name}:*:{uid}:{gid}::/:/bin/false\n"));
        }
    }
    let members = pairs(members);
    let mut group = String::new();
    for (name, v) in pairs(gids) {
        if let Some(gid) = v.first() {
            let m = members
                .iter()
                .find(|(n, _)| *n == name)
                .map(|(_, m)| m.join(","))
                .unwrap_or_default();
            group.push_str(&format!("{name}:*:{gid}:{m}\n"));
        }
    }
    UnixGroups::from_files(&group, &passwd, true)
}

struct UnixAccess {
    groups: UnixGroups,
}

impl AccessReader for UnixAccess {
    fn read(&mut self, _path: &Path, unix: Option<UnixMode>) -> AccessFact {
        match unix {
            Some(m) => AccessFact::Known(access_from_mode(m.uid, m.gid, m.mode, &self.groups)),
            None => AccessFact::Unknown {
                why: "the folder's owner and permissions were not listed".into(),
            },
        }
    }
}

/// One PowerShell helper for the whole sweep, started on first use.
#[derive(Default)]
struct WindowsAcl {
    helper: Option<Helper>,
    broken: Option<String>,
}

struct Helper {
    child: Child,
    input: ChildStdin,
    lines: Receiver<String>,
}

impl Drop for Helper {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

const ACL_SCRIPT: &str = "$u = New-Object System.Text.UTF8Encoding $false; \
    [Console]::InputEncoding = $u; [Console]::OutputEncoding = $u; \
    while ($null -ne ($p = [Console]::In.ReadLine())) { \
      try { $s = (Get-Acl -LiteralPath $p -ErrorAction Stop).Sddl; [Console]::Out.WriteLine('OK' + [char]9 + $s) } \
      catch { [Console]::Out.WriteLine('NO' + [char]9 + $_.Exception.GetType().Name) } \
      [Console]::Out.Flush() }";

impl WindowsAcl {
    fn start(&mut self) -> Result<&mut Helper, String> {
        if let Some(why) = &self.broken {
            return Err(why.clone());
        }
        if self.helper.is_none() {
            let exe = std::env::var_os("SystemRoot")
                .map(|r| {
                    std::path::PathBuf::from(r)
                        .join(r"System32\WindowsPowerShell\v1.0\powershell.exe")
                })
                .unwrap_or_else(|| "powershell".into());
            let mut command = Command::new(exe);
            command
                .args(["-NoProfile", "-NonInteractive", "-Command", ACL_SCRIPT])
                .stdin(Stdio::piped())
                .stdout(Stdio::piped())
                .stderr(Stdio::null());
            #[cfg(windows)]
            {
                use std::os::windows::process::CommandExt;
                const CREATE_NO_WINDOW: u32 = 0x0800_0000;
                command.creation_flags(CREATE_NO_WINDOW);
            }
            let mut child = command.spawn().map_err(|e| e.to_string())?;
            let input = child.stdin.take().ok_or("no input")?;
            let output = child.stdout.take().ok_or("no output")?;
            let (send, lines) = channel();
            std::thread::spawn(move || {
                for line in BufReader::new(output).lines() {
                    let Ok(line) = line else { break };
                    if send.send(line).is_err() {
                        break;
                    }
                }
            });
            self.helper = Some(Helper {
                child,
                input,
                lines,
            });
        }
        self.helper.as_mut().ok_or_else(|| "no helper".to_string())
    }

    fn ask(&mut self, path: &Path) -> Result<String, String> {
        let Some(text) = path.to_str() else {
            return Err("the name is not valid text".into());
        };
        if text.contains(['\n', '\r']) {
            return Err("the name holds a line break".into());
        }
        let helper = self.start()?;
        let sent = writeln!(helper.input, "{text}").and_then(|()| helper.input.flush());
        let answer = match sent {
            Ok(()) => helper.lines.recv_timeout(Duration::from_secs(20)),
            Err(e) => {
                let why = format!("the permissions helper stopped: {e}");
                self.broken = Some(why.clone());
                self.helper = None;
                return Err(why);
            }
        };
        match answer {
            Ok(line) => Ok(line),
            Err(RecvTimeoutError::Timeout) | Err(RecvTimeoutError::Disconnected) => {
                let why = "the permissions helper stopped answering".to_string();
                self.broken = Some(why.clone());
                self.helper = None;
                Err(why)
            }
        }
    }
}

impl AccessReader for WindowsAcl {
    fn read(&mut self, path: &Path, _unix: Option<UnixMode>) -> AccessFact {
        match self.ask(path) {
            Ok(line) => match line.split_once('\t') {
                Some(("OK", sddl)) => match access_from_sddl(sddl) {
                    Some(access) => AccessFact::Known(access),
                    None => AccessFact::Unknown {
                        why: "the permissions could not be read".into(),
                    },
                },
                Some((_, why)) => AccessFact::Unknown {
                    why: format!("the permissions could not be read ({why})"),
                },
                None => AccessFact::Unknown {
                    why: "the permissions helper gave no answer".into(),
                },
            },
            Err(why) => AccessFact::Unknown { why },
        }
    }
}
