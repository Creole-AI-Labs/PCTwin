//! Windows: the helper's pipe admits only the mover's account (and SYSTEM), is made first or not
//! at all, and says who is calling.
#![cfg(windows)]

use pctwin_helper_os::{
    Account, Error, PipeClient, PipeName, PipeServer, Sid, caller_of, current_account,
};
use std::io::{BufRead, BufReader, Read, Write};
use std::path::Path;
use std::process::{Command, Stdio};

/// An account that exists nowhere: a pipe made for it must refuse this test.
const STRANGER: &str = "S-1-5-21-1-2-3-1001";

fn me() -> Sid {
    match current_account().unwrap() {
        Account::Sid(s) => s,
        _ => panic!("this system names accounts by security identifier"),
    }
}

fn same_file(a: &Path, b: &Path) -> bool {
    std::fs::canonicalize(a).unwrap() == std::fs::canonicalize(b).unwrap()
}

/// The access list as the system reports it: (protected, [(type, flags, rights, account)]).
fn parse_dacl(sddl: &str) -> (bool, Vec<[String; 4]>) {
    let d = sddl.strip_prefix("D:").unwrap_or_else(|| panic!("{sddl}"));
    let (flags, aces) = d.split_at(d.find('(').unwrap_or(d.len()));
    let aces = aces
        .trim_start_matches('(')
        .trim_end_matches(')')
        .split(")(")
        .filter(|a| !a.is_empty())
        .map(|a| {
            let f: Vec<&str> = a.split(';').collect();
            assert_eq!(f.len(), 6, "{a}");
            [f[0], f[1], f[2], f[5]].map(str::to_owned)
        })
        .collect();
    (flags.contains('P'), aces)
}

/// The text form names a few well-known accounts by alias: SY is SYSTEM, LA this computer's
/// built-in Administrator (RID 500, which GitHub's Windows runners run as), LG its Guest.
fn alias<'a>(who: &'a str, mover: &'a str) -> &'a str {
    let local = mover.starts_with("S-1-5-21-");
    match who {
        "LA" if local && mover.ends_with("-500") => mover,
        "LG" if local && mover.ends_with("-501") => mover,
        other => other,
    }
}

fn assert_only(server: &PipeServer, mover: &str) {
    let sddl = server.access_list().unwrap();
    let (protected, aces) = parse_dacl(&sddl);
    assert!(protected, "inherited entries could widen it: {sddl}");
    let mut who: Vec<&str> = aces.iter().map(|a| alias(a[3].as_str(), mover)).collect();
    who.sort();
    let mut want = vec![mover, "SY"];
    want.sort();
    assert_eq!(who, want, "{sddl}");
    for a in &aces {
        assert_eq!(a[0], "A", "allow entries only: {sddl}");
        assert_eq!(a[1], "", "no inheritance flags: {sddl}");
        assert_eq!(
            a[2].to_ascii_lowercase(),
            "0x12008b",
            "read and write data only: {sddl}"
        );
    }
}

#[test]
fn the_access_list_names_exactly_the_mover_and_system() {
    let mine = me();
    let server = PipeServer::create(&PipeName::random().unwrap(), &mine).unwrap();
    assert_only(&server, mine.as_str());
    let stranger = Sid::parse(STRANGER).unwrap();
    let server = PipeServer::create(&PipeName::random().unwrap(), &stranger).unwrap();
    assert_only(&server, STRANGER);
}

#[test]
fn a_pipe_for_another_account_refuses_this_one() {
    let name = PipeName::random().unwrap();
    let _server = PipeServer::create(&name, &Sid::parse(STRANGER).unwrap()).unwrap();
    match PipeClient::connect(&name) {
        Err(Error::Io(e)) => assert_eq!(e.raw_os_error(), Some(5), "{e}"),
        other => panic!("must be refused, got {other:?}"),
    }
}

#[test]
fn a_caller_in_this_process_is_this_process() {
    let name = PipeName::random().unwrap();
    let server = PipeServer::create(&name, &me()).unwrap();
    let client = PipeClient::connect(&name).unwrap();
    server.accept().unwrap();
    // The client speaks first; the server reads before it asks who is calling.
    let mut w = client.stream();
    w.write_all(b"hello").unwrap();
    let mut buf = [0u8; 5];
    let mut r = server.stream();
    r.read_exact(&mut buf).unwrap();
    assert_eq!(&buf, b"hello");

    let caller = caller_of(&server).unwrap();
    assert_eq!(caller.process_id, std::process::id());
    assert_eq!(caller.account, Account::Sid(me()));
    assert!(same_file(
        &caller.program_hint,
        &std::env::current_exe().unwrap()
    ));
    assert_eq!(client.server_process_id().unwrap(), std::process::id());
}

/// A server that tries to act as the client gets only an identification token: it can see who
/// the client is, never open anything as the client (`SECURITY_IDENTIFICATION`). The server
/// here is PowerShell, impersonating the client and reporting the level it was given.
#[test]
fn the_server_may_only_identify_the_client_never_act_as_it() {
    let name = PipeName::random().unwrap();
    let script = format!(
        "$s = New-Object System.IO.Pipes.NamedPipeServerStream('{}', 'InOut', 1); \
         [Console]::Out.WriteLine('ready'); [Console]::Out.Flush(); \
         $s.WaitForConnection(); [void]$s.ReadByte(); \
         $level = 'none'; \
         try {{ $s.RunAsClient({{ $script:level = [Security.Principal.WindowsIdentity]::GetCurrent($true).ImpersonationLevel }}) }} \
         catch {{ $level = 'error: ' + $_.Exception.InnerException.Message }}; \
         [Console]::Out.WriteLine('level=' + $level); [Console]::Out.Flush(); $s.Dispose()",
        name.as_str()
    );
    let mut server = Command::new("powershell.exe")
        // pwsh (CI's shell) sets a module path Windows PowerShell 5.1 cannot load from.
        .env_remove("PSModulePath")
        .args(["-NoProfile", "-NonInteractive", "-Command", &script])
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    let mut out = BufReader::new(server.stdout.take().unwrap());
    let mut line = String::new();
    out.read_line(&mut line).unwrap();
    assert_eq!(line.trim(), "ready");
    let client = PipeClient::connect(&name).unwrap();
    client.stream().write_all(b"x").unwrap();
    let mut rest = String::new();
    out.read_to_string(&mut rest).unwrap();
    let _ = server.wait();
    drop(client);
    assert!(
        rest.lines().any(|l| l.trim() == "level=Identification"),
        "the server was given more than identification: {rest}"
    );
}

#[test]
fn a_name_already_made_is_never_joined() {
    let name = PipeName::random().unwrap();
    let _first = PipeServer::create(&name, &me()).unwrap();
    assert!(matches!(
        PipeServer::create(&name, &me()),
        Err(Error::NameTaken)
    ));
}

/// Another program made the name first, with room for many instances and an open access list:
/// the helper must refuse to make a second instance (FILE_FLAG_FIRST_PIPE_INSTANCE).
#[test]
fn a_squatter_with_open_instances_is_refused() {
    let name = PipeName::random().unwrap();
    let script = format!(
        "$s = New-Object System.IO.Pipes.NamedPipeServerStream('{}', 'InOut', 254); \
         [Console]::Out.WriteLine('ready'); [Console]::Out.Flush(); [void][Console]::In.ReadLine()",
        name.as_str()
    );
    let mut squatter = Command::new("powershell.exe")
        // pwsh (CI's shell) sets a module path Windows PowerShell 5.1 cannot load from.
        .env_remove("PSModulePath")
        .args(["-NoProfile", "-NonInteractive", "-Command", &script])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    let mut line = String::new();
    BufReader::new(squatter.stdout.take().unwrap())
        .read_line(&mut line)
        .unwrap();
    assert_eq!(line.trim(), "ready");
    let r = PipeServer::create(&name, &me());
    drop(squatter.stdin.take());
    let _ = squatter.wait();
    assert!(matches!(r, Err(Error::NameTaken)), "{r:?}");
}

/// Through the network path (the server service on this same computer), the pipe refuses.
#[test]
fn a_remote_client_is_refused() {
    let name = PipeName::random().unwrap();
    let _server = PipeServer::create(&name, &me()).unwrap();
    let remote = format!(r"\\localhost\pipe\{}", name.as_str());
    use std::os::windows::fs::OpenOptionsExt;
    let r = std::fs::OpenOptions::new()
        .access_mode(0x0012_008B)
        .open(&remote);
    assert!(r.is_err(), "a client over the network path was let in");
}

fn spawn_child_client(name: &PipeName, mode: &str) -> std::process::Child {
    Command::new(std::env::current_exe().unwrap())
        .args(["--exact", "child_client", "--nocapture", "--test-threads=1"])
        .env("PCTWIN_HELPER_OS_CHILD_PIPE", name.as_str())
        .env("PCTWIN_HELPER_OS_CHILD_MODE", mode)
        .stdout(Stdio::null())
        .spawn()
        .unwrap()
}

#[test]
fn a_caller_in_another_process_is_that_process() {
    let name = PipeName::random().unwrap();
    let server = PipeServer::create(&name, &me()).unwrap();
    let mut child = spawn_child_client(&name, "stay");
    server.accept().unwrap();
    let mut b = [0u8; 1];
    server.stream().read_exact(&mut b).unwrap();
    let caller = caller_of(&server).unwrap();
    server.stream().write_all(b"k").unwrap();
    assert!(child.wait().unwrap().success());
    assert_eq!(caller.process_id, child.id());
    assert_ne!(caller.process_id, std::process::id());
    assert_eq!(caller.account, Account::Sid(me()));
    assert!(same_file(
        &caller.program_hint,
        &std::env::current_exe().unwrap()
    ));
}

#[test]
fn a_caller_that_has_ended_is_not_vouched_for() {
    let name = PipeName::random().unwrap();
    let server = PipeServer::create(&name, &me()).unwrap();
    let mut child = spawn_child_client(&name, "leave");
    server.accept().unwrap();
    let mut b = [0u8; 1];
    server.stream().read_exact(&mut b).unwrap();
    assert!(child.wait().unwrap().success());
    let r = caller_of(&server);
    assert!(matches!(r, Err(Error::CallerGone)), "{r:?}");
}

/// Run as a child by the tests above; does nothing otherwise.
#[test]
fn child_client() {
    let Some(name) = std::env::var_os("PCTWIN_HELPER_OS_CHILD_PIPE") else {
        return;
    };
    let name = PipeName::parse(name.to_str().unwrap()).unwrap();
    let client = PipeClient::connect(&name).unwrap();
    client.stream().write_all(b"x").unwrap();
    if std::env::var("PCTWIN_HELPER_OS_CHILD_MODE").as_deref() == Ok("stay") {
        let mut b = [0u8; 1];
        client.stream().read_exact(&mut b).unwrap();
    }
}

/// A real second account (as the lab's two-accounts job makes one): it can use a pipe made for
/// it, is reported as itself, and is refused by a pipe made for this account.
#[test]
#[ignore = "needs PCTWIN_HELPER_OS_ADMIN_TESTS=1 and administrator rights to make an account (CI)"]
fn a_real_second_account_is_refused_and_identified() {
    assert!(
        std::env::var("PCTWIN_HELPER_OS_ADMIN_TESTS").as_deref() == Ok("1"),
        "set PCTWIN_HELPER_OS_ADMIN_TESTS=1 (CI only)"
    );
    // One PowerShell holds the throwaway account's password from start to end; it is never
    // printed. It prints the account's SID, waits for the two pipe names, runs a client as that
    // account, prints its exit code and removes the account.
    let script = r#"
$ErrorActionPreference = 'Stop'
$u = 'pctwin-helper-b'
$bytes = New-Object byte[] 18
[System.Security.Cryptography.RandomNumberGenerator]::Create().GetBytes($bytes)
$secure = ConvertTo-SecureString ([Convert]::ToBase64String($bytes) + 'a1!') -AsPlainText -Force
if (Get-LocalUser -Name $u -ErrorAction SilentlyContinue) { Remove-LocalUser -Name $u }
$user = New-LocalUser -Name $u -Password $secure -PasswordNeverExpires -AccountNeverExpires
try {
  [Console]::Out.WriteLine($user.SID.Value); [Console]::Out.Flush()
  $mine = [Console]::In.ReadLine(); $theirs = [Console]::In.ReadLine()
  $client = @"
`$code = 0
try { `$p = New-Object System.IO.Pipes.NamedPipeClientStream('.', '$theirs', [System.IO.Pipes.PipeAccessRights]0x12008B, 'None', 'Identification', 'None'); `$p.Connect(10000); `$p.WriteByte(1); [void]`$p.ReadByte(); `$p.Dispose(); `$code += 1 } catch { [Console]::Error.WriteLine("its own pipe: `$_") }
try { `$q = New-Object System.IO.Pipes.NamedPipeClientStream('.', '$mine', [System.IO.Pipes.PipeAccessRights]0x12008B, 'None', 'Identification', 'None'); `$q.Connect(3000); `$q.Dispose(); `$code += 2 } catch { [Console]::Error.WriteLine("the mover's pipe: `$_") }
exit `$code
"@
  $cred = New-Object System.Management.Automation.PSCredential("$env:COMPUTERNAME\$u", $secure)
  # Its output goes to a folder both accounts may write, as the lab's run.ps1 does.
  $logs = Join-Path $env:SystemDrive ("pctwin-helper-test-" + [Guid]::NewGuid().ToString('N'))
  New-Item -ItemType Directory $logs | Out-Null
  icacls $logs /grant '*S-1-1-0:(OI)(CI)M' | Out-Null
  # A script file, not -EncodedCommand: a command line started under another account's
  # credentials may hold at most 1024 characters.
  $clientFile = Join-Path $logs 'client.ps1'
  Set-Content -Path $clientFile -Value $client -Encoding UTF8
  $p = Start-Process -FilePath 'powershell.exe' -ArgumentList "-NoProfile -NonInteractive -ExecutionPolicy Bypass -File `"$clientFile`"" -Credential $cred -LoadUserProfile -WorkingDirectory $env:SystemRoot -WindowStyle Hidden -PassThru -RedirectStandardOutput (Join-Path $logs 'out.txt') -RedirectStandardError (Join-Path $logs 'err.txt')
  # Without its handle taken now, a process started under other credentials reports no exit code.
  $null = $p.Handle
  if ($p.WaitForExit(120000)) {
    [Console]::Out.WriteLine("exit=$($p.ExitCode)")
  } else {
    $p.Kill(); [Console]::Out.WriteLine("exit=timeout")
  }
  Get-Content (Join-Path $logs 'err.txt') -ErrorAction SilentlyContinue | ForEach-Object { [Console]::Out.WriteLine("err: $_") }
  [Console]::Out.Flush()
  Remove-Item -Recurse -Force $logs -ErrorAction SilentlyContinue
} finally {
  Remove-LocalUser -Name $u
}
"#;
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("second-account.ps1");
    std::fs::write(&file, script).unwrap();
    let mut ps = Command::new("powershell.exe")
        // pwsh (CI's shell) sets a module path Windows PowerShell 5.1 cannot load from.
        .env_remove("PSModulePath")
        .args([
            "-NoProfile",
            "-NonInteractive",
            "-ExecutionPolicy",
            "Bypass",
            "-File",
        ])
        .arg(&file)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    let mut out = BufReader::new(ps.stdout.take().unwrap());
    let mut sid = String::new();
    out.read_line(&mut sid).unwrap();
    let theirs_sid = Sid::parse(sid.trim()).unwrap_or_else(|_| panic!("no SID: {sid:?}"));

    let mine = PipeName::random().unwrap();
    let theirs = PipeName::random().unwrap();
    let mine_server = PipeServer::create(&mine, &me()).unwrap();
    let theirs_server = PipeServer::create(&theirs, &theirs_sid).unwrap();
    let mut stdin = ps.stdin.take().unwrap();
    writeln!(stdin, "{}\n{}", mine.as_str(), theirs.as_str()).unwrap();
    drop(stdin);

    // The helper's side runs on its own thread, so a client that never arrives fails the test
    // with PowerShell's own report instead of waiting for ever.
    let (tx, rx) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let r = (|| {
            theirs_server.accept()?;
            let mut b = [0u8; 1];
            theirs_server.stream().read_exact(&mut b)?;
            let caller = caller_of(&theirs_server)?;
            theirs_server.stream().write_all(b"k")?;
            Ok::<_, Error>(caller)
        })();
        let _ = tx.send(r);
    });

    // PowerShell's report is read on a thread too, and PowerShell is stopped after four
    // minutes, so nothing here can wait for ever.
    let (otx, orx) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let mut rest = String::new();
        let _ = out.read_to_string(&mut rest);
        let _ = otx.send(rest);
    });
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(240);
    let ended = loop {
        if let Some(status) = ps.try_wait().unwrap() {
            break status;
        }
        if std::time::Instant::now() > deadline {
            let _ = ps.kill();
            panic!("PowerShell did not finish in four minutes");
        }
        std::thread::sleep(std::time::Duration::from_millis(200));
    };
    let rest = orx
        .recv_timeout(std::time::Duration::from_secs(10))
        .unwrap_or_default();
    println!("{rest}");
    drop(mine_server);
    let caller = rx
        .recv_timeout(std::time::Duration::from_secs(10))
        .unwrap_or_else(|_| panic!("the second account never reached its pipe: {rest}"))
        .unwrap();
    assert_eq!(caller.account, Account::Sid(theirs_sid));
    println!("PowerShell ended: {ended}");
    // 1 = reached the pipe made for it; 2 would mean it reached the mover's pipe.
    assert!(rest.contains("exit=1"), "{rest}");
}
