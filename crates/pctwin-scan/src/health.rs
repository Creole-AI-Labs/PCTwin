use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use serde::Serialize;

/// How healthy a drive says it is. Only what the system reports; nothing is guessed.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "health", content = "detail", rename_all = "kebab-case")]
pub enum Health {
    Good,
    /// The drive warns it may fail (for example, worn spare area).
    Warning(String),
    /// The drive reports it is failing.
    Failing(String),
    /// No verdict: not supported, not reported, or the check could not run. Never read as good.
    Unknown(String),
}

/// How to read from a drive during a move.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum ReadPlan {
    Normal,
    /// For a weak drive: the most important things first, each file read only once, and a clean
    /// stop after repeated read errors so the drive isn't strained.
    Careful {
        most_important_first: bool,
        read_each_file_once: bool,
        stop_after_read_errors: u32,
    },
}

impl ReadPlan {
    pub fn for_health(health: &Health) -> Self {
        match health {
            Health::Warning(_) | Health::Failing(_) => ReadPlan::Careful {
                most_important_first: true,
                read_each_file_once: true,
                stop_after_read_errors: 5,
            },
            // Unknown is the common case (Apple SSDs, USB drives, virtual machines); the
            // read-error watch during the move protects it.
            Health::Good | Health::Unknown(_) => ReadPlan::Normal,
        }
    }
}

/// The longest any health check may take.
const CHECK_LIMIT: Duration = Duration::from_secs(15);

/// The health of the drive the running system started from.
pub fn system_drive_health() -> Health {
    if cfg!(windows) {
        let script = "$ErrorActionPreference='Stop'; \
            [Console]::OutputEncoding=[Text.Encoding]::UTF8; \
            $sys=(Get-Partition -DriveLetter $env:SystemDrive[0] | Get-Disk).Number; \
            Get-PhysicalDisk | ForEach-Object { [pscustomobject]@{ \
              Name=$_.FriendlyName; Health=[string]$_.HealthStatus; \
              Operational=($_.OperationalStatus -join ', '); System=([string]$_.DeviceId -eq [string]$sys) } } \
            | ConvertTo-Json -Compress";
        match run(
            "powershell",
            &["-NoProfile", "-NonInteractive", "-Command", script],
        ) {
            Ok(out) => health_from_get_physical_disk(&out),
            Err(why) => Health::Unknown(why),
        }
    } else if cfg!(target_os = "macos") {
        match run("diskutil", &["info", "/"]) {
            Ok(out) => health_from_diskutil(&out),
            Err(why) => Health::Unknown(why),
        }
    } else {
        let Some(device) = linux_root_disk() else {
            return Health::Unknown("the system drive's device could not be found".into());
        };
        match run("udisksctl", &["info", "-b", &device]) {
            Ok(out) => health_from_udisks(&out),
            Err(why) => Health::Unknown(why),
        }
    }
}

/// Runs a system tool with a time limit, without a console window on Windows.
fn run(program: &str, args: &[&str]) -> Result<String, String> {
    let mut command = Command::new(program);
    command
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        command.creation_flags(CREATE_NO_WINDOW);
    }
    let mut child = command
        .spawn()
        .map_err(|e| format!("{program} could not start: {e}"))?;
    let started = Instant::now();
    loop {
        match child.try_wait() {
            Ok(Some(_)) => break,
            Ok(None) if started.elapsed() > CHECK_LIMIT => {
                let _ = child.kill();
                let _ = child.wait();
                return Err(format!("{program} took too long"));
            }
            Ok(None) => std::thread::sleep(Duration::from_millis(50)),
            Err(e) => return Err(format!("{program} failed: {e}")),
        }
    }
    let output = child
        .wait_with_output()
        .map_err(|e| format!("{program} failed: {e}"))?;
    if !output.status.success() {
        return Err(format!("{program} reported a problem"));
    }
    Ok(String::from_utf8_lossy(&output.stdout).into_owned())
}

/// Windows: `Get-PhysicalDisk` as JSON (one object or a list), with which disk holds the system.
pub fn health_from_get_physical_disk(json: &str) -> Health {
    let Ok(value) = serde_json::from_str::<serde_json::Value>(json.trim()) else {
        return Health::Unknown("Windows gave no readable answer".into());
    };
    let disks = match value {
        serde_json::Value::Array(list) => list,
        one => vec![one],
    };
    let Some(system) = disks.iter().find(|d| d["System"] == true) else {
        return Health::Unknown("Windows did not say which disk holds the system".into());
    };
    let detail = system["Operational"].as_str().unwrap_or("").to_string();
    match system["Health"].as_str().unwrap_or("") {
        "Healthy" => Health::Good,
        "Warning" => Health::Warning(detail),
        "Unhealthy" => Health::Failing(detail),
        other => Health::Unknown(format!("Windows reports health as {other:?}")),
    }
}

/// macOS: the `SMART Status:` line of `diskutil info`.
pub fn health_from_diskutil(output: &str) -> Health {
    let Some(status) = output
        .lines()
        .find_map(|l| l.trim().strip_prefix("SMART Status:"))
        .map(str::trim)
    else {
        return Health::Unknown("macOS gave no SMART status".into());
    };
    match status {
        "Verified" => Health::Good,
        "Failing" => Health::Failing("macOS reports SMART status Failing".into()),
        other => Health::Unknown(format!("macOS reports SMART status {other:?}")),
    }
}

/// Linux: `udisksctl info` for the system disk. ATA drives report `SmartFailing`; NVMe drives
/// report `SmartCriticalWarning` (empty when there is nothing to warn about).
pub fn health_from_udisks(output: &str) -> Health {
    let value = |key: &str| {
        output
            .lines()
            .find_map(|l| l.trim().strip_prefix(key).map(|v| v.trim().to_string()))
    };
    if let Some(failing) = value("SmartFailing:") {
        return match failing.as_str() {
            "false" => Health::Good,
            "true" => Health::Failing("the drive reports it is failing".into()),
            other => Health::Unknown(format!("udisks reports SmartFailing {other:?}")),
        };
    }
    if let Some(warnings) = value("SmartCriticalWarning:") {
        if warnings.is_empty() {
            return Health::Good;
        }
        // Read-only or reliability warnings mean the drive is failing; others are warnings.
        let severe = warnings
            .split(',')
            .map(str::trim)
            .any(|w| w == "readonly" || w == "reliability" || w == "volatile_mem");
        return if severe {
            Health::Failing(format!("the drive warns: {warnings}"))
        } else {
            Health::Warning(format!("the drive warns: {warnings}"))
        };
    }
    Health::Unknown("the drive reports no health".into())
}

/// Linux: the whole disk holding `/` (`/dev/nvme0n1` for `/dev/nvme0n1p2`, `/dev/sda` for
/// `/dev/sda1`), from the system's mount list.
fn linux_root_disk() -> Option<String> {
    let mounts = std::fs::read_to_string("/proc/self/mounts").ok()?;
    let source = mounts.lines().find_map(|line| {
        let mut parts = line.split_whitespace();
        let (source, target) = (parts.next()?, parts.next()?);
        (target == "/").then(|| source.to_string())
    })?;
    let name = source.strip_prefix("/dev/")?;
    // The partition's parent in /sys is the whole disk.
    let parent = std::fs::canonicalize(format!("/sys/class/block/{name}/.."))
        .ok()
        .and_then(|p| p.file_name().map(|n| n.to_string_lossy().into_owned()))
        .filter(|p| p != "block");
    Some(format!(
        "/dev/{}",
        parent.unwrap_or_else(|| name.to_string())
    ))
}
