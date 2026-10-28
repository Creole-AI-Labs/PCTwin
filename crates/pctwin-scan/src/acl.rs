//! Reading who could open a place, without administrator rights and without opening anything
//! inside it. Only used on laptops with two or more people.
//!
//! - Windows: one PowerShell helper reads each folder's security descriptor (the owner, group
//!   and access list, through .NET itself, never a cmdlet: see `ACL_SCRIPT`) and answers in
//!   SDDL, one line per folder, for the whole sweep.
//!   A folder whose permissions this person may not read is answered "unknown", never guessed.
//! - macOS and Linux: owner, group and mode come with the folder listing itself; the groups come
//!   from the system's group list.

use std::io::{BufRead, BufReader, Write};
use std::path::Path;
use std::process::{Child, ChildStdin, Stdio};
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
/// The elevated helper's reader on Windows: PowerShell at `powershell` (a full path the caller
/// found from the system itself) with only `env`, nothing passed on from whoever started the
/// helper. Elsewhere, the same as [`system_reader`].
pub fn system_reader_clean(
    powershell: std::path::PathBuf,
    env: Vec<(std::ffi::OsString, std::ffi::OsString)>,
) -> Box<dyn AccessReader + Send> {
    if cfg!(windows) {
        Box::new(WindowsAcl {
            clean: Some((powershell, env)),
            ..WindowsAcl::default()
        })
    } else {
        system_reader()
    }
}

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
    let out = pctwin_types::process::quiet("getent")
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
    let users = pctwin_types::process::quiet("dscl")
        .args([".", "-list", "/Users", "UniqueID"])
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).into_owned())
        .unwrap_or_default();
    let primary = pctwin_types::process::quiet("dscl")
        .args([".", "-list", "/Users", "PrimaryGroupID"])
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).into_owned())
        .unwrap_or_default();
    let gids = pctwin_types::process::quiet("dscl")
        .args([".", "-list", "/Groups", "PrimaryGroupID"])
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).into_owned())
        .unwrap_or_default();
    let members = pctwin_types::process::quiet("dscl")
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
    /// Set in the helper's environment on top of this process's (tests: what a parent passes on).
    env: Vec<(String, String)>,
    /// The elevated helper's reader: PowerShell by its full path, with only this environment
    /// (nothing passed on from whoever started the helper).
    clean: Option<(
        std::path::PathBuf,
        Vec<(std::ffi::OsString, std::ffi::OsString)>,
    )>,
    helper: Option<Helper>,
    broken: Option<String>,
    /// Times the helper had to be started again in a row (a slow answer, or it stopped).
    restarts: u32,
}

/// One folder whose permissions take longer than this to read is answered "unknown" and the
/// helper is started again; only after this many failures in a row is it given up for the sweep.
const ASK_LIMIT: Duration = Duration::from_secs(20);
const MOST_RESTARTS: u32 = 3;

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

/// .NET itself, never a cmdlet. A cmdlet comes from a module found through `PSModulePath`, which
/// the helper inherits from whatever started PCTwin; started from PowerShell 7 (a terminal, a
/// script, a CI runner), that path names PowerShell 7's own `Microsoft.PowerShell.Security`
/// first, Windows PowerShell cannot load it, and `Get-Acl` is "not found": every folder's
/// permissions read as unknown, so on a laptop with several people every place outside the
/// homes became a blind spot and nothing in it moved. The same sections as `Get-Acl` (owner,
/// group, access list), in the same SDDL.
const ACL_SCRIPT: &str = "$u = [System.Text.UTF8Encoding]::new($false); \
    [Console]::InputEncoding = $u; [Console]::OutputEncoding = $u; \
    $w = [System.Security.AccessControl.AccessControlSections]'Access, Owner, Group'; \
    while ($null -ne ($p = [Console]::In.ReadLine())) { \
      try { \
        if ([System.IO.File]::Exists($p)) { $i = [System.IO.FileInfo]::new($p) } \
        else { $i = [System.IO.DirectoryInfo]::new($p) } \
        $s = $i.GetAccessControl($w).GetSecurityDescriptorSddlForm($w); \
        [Console]::Out.WriteLine('OK' + [char]9 + $s) } \
      catch { [Console]::Out.WriteLine('NO' + [char]9 + $_.Exception.GetType().Name) } \
      [Console]::Out.Flush() }";

impl WindowsAcl {
    fn start(&mut self) -> Result<&mut Helper, String> {
        if let Some(why) = &self.broken {
            return Err(why.clone());
        }
        if self.helper.is_none() {
            let mut command = match &self.clean {
                Some((exe, env)) => {
                    let mut c = pctwin_types::process::quiet(exe);
                    c.env_clear().envs(env.iter().map(|(k, v)| (k, v)));
                    c
                }
                None => pctwin_types::process::quiet(
                    std::env::var_os("SystemRoot")
                        .map(|r| {
                            std::path::PathBuf::from(r)
                                .join(r"System32\WindowsPowerShell\v1.0\powershell.exe")
                        })
                        .unwrap_or_else(|| "powershell".into()),
                ),
            };
            command
                .args(["-NoProfile", "-NonInteractive", "-Command", ACL_SCRIPT])
                .envs(self.env.iter().map(|(k, v)| (k, v)))
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
            Ok(()) => helper.lines.recv_timeout(ASK_LIMIT),
            Err(e) => {
                return Err(self.failed(format!("the permissions helper stopped: {e}")));
            }
        };
        match answer {
            Ok(line) => {
                self.restarts = 0;
                Ok(line)
            }
            Err(RecvTimeoutError::Timeout) => {
                Err(self.failed("reading this folder's permissions took too long".to_string()))
            }
            Err(RecvTimeoutError::Disconnected) => {
                Err(self.failed("the permissions helper stopped answering".to_string()))
            }
        }
    }

    /// This one answer failed: the helper is started again for the next folder, and given up
    /// only after several failures in a row (then every later folder says why).
    fn failed(&mut self, why: String) -> String {
        self.helper = None;
        self.restarts += 1;
        if self.restarts >= MOST_RESTARTS {
            self.broken = Some(format!("{why} ({MOST_RESTARTS} times in a row)"));
        }
        why
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

#[cfg(all(test, windows))]
mod tests {
    use super::*;

    /// Started from PowerShell 7 (a terminal, a script, a CI runner), the helper inherits a
    /// module path that names a module Windows PowerShell cannot load before its own (made here
    /// as a stand-in for PowerShell 7's `Microsoft.PowerShell.Security`): every folder's
    /// permissions are read all the same, never "unknown".
    #[test]
    fn permissions_are_read_whatever_module_path_the_helper_inherits() {
        let dir = tempfile::tempdir().unwrap();
        let module = dir.path().join("Microsoft.PowerShell.Security");
        std::fs::create_dir_all(&module).unwrap();
        std::fs::write(
            module.join("Microsoft.PowerShell.Security.psd1"),
            "@{ ModuleVersion='7.0.0.0'; CompatiblePSEditions=@('Core'); PowerShellVersion='7.0'; \
             RootModule='Microsoft.PowerShell.Security.dll'; CmdletsToExport=@('Get-Acl') }",
        )
        .unwrap();
        let path = format!(
            "{};{}",
            dir.path().display(),
            std::env::var("PSModulePath").unwrap_or_default()
        );
        let mut reader = WindowsAcl {
            env: vec![("PSModulePath".into(), path)],
            ..WindowsAcl::default()
        };
        let folder = dir.path().join("plain");
        std::fs::create_dir_all(&folder).unwrap();
        let file = folder.join("a.txt");
        std::fs::write(&file, b"x").unwrap();
        for p in [&folder, &file] {
            let read = reader.read(p, None);
            assert!(
                matches!(read, AccessFact::Known(_)),
                "{}: {read:?}",
                p.display()
            );
        }
    }

    /// The elevated helper's reader: PowerShell by its full path with only a minimal
    /// environment (nothing of the starter's) still reads every permission.
    #[test]
    fn the_clean_reader_reads_permissions_with_only_a_minimal_environment() {
        let windows = std::path::PathBuf::from(std::env::var_os("SystemRoot").unwrap());
        let env: Vec<(std::ffi::OsString, std::ffi::OsString)> = vec![
            ("SystemRoot".into(), windows.clone().into_os_string()),
            ("windir".into(), windows.clone().into_os_string()),
        ];
        let mut reader = WindowsAcl {
            clean: Some((
                windows.join(r"System32\WindowsPowerShell\v1.0\powershell.exe"),
                env,
            )),
            ..WindowsAcl::default()
        };
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("a.txt");
        std::fs::write(&file, b"x").unwrap();
        for p in [dir.path(), file.as_path()] {
            let read = reader.read(p, None);
            assert!(
                matches!(read, AccessFact::Known(_)),
                "{}: {read:?}",
                p.display()
            );
        }
    }
}
