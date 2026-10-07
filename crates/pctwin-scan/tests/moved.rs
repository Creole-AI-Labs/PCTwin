//! Folders moved elsewhere are found where they are. Each check runs the probe in a separate
//! process with its own settings. Kept apart from the other folder tests because the Windows check
//! briefly changes the user's folder settings.

use std::path::Path;
use std::process::Command;
// ---------- folders moved elsewhere, checked in a separate process ----------

fn run_probe(env: &[(&str, &Path)]) -> serde_json::Value {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_pctwin-special-folders"));
    for (k, v) in env {
        cmd.env(k, v);
    }
    let out = cmd.output().unwrap();
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    serde_json::from_slice(&out.stdout).unwrap()
}

fn entry<'a>(all: &'a serde_json::Value, role: &str) -> &'a serde_json::Value {
    all.as_array()
        .unwrap()
        .iter()
        .find(|e| e["role"] == role)
        .unwrap_or_else(|| panic!("no {role} in {all}"))
}

/// Linux: folders named in another language and moved to another place are found through the
/// user-dirs settings; a folder set to the home folder itself means "none".
#[cfg(all(unix, not(target_os = "macos")))]
#[test]
fn linux_finds_translated_and_moved_folders_through_user_dirs() {
    let home = tempfile::tempdir().unwrap();
    let elsewhere = tempfile::tempdir().unwrap();
    let config = home.path().join(".config");
    std::fs::create_dir_all(&config).unwrap();
    std::fs::create_dir(home.path().join("Dokumente")).unwrap();
    std::fs::create_dir(elsewhere.path().join("Bilder")).unwrap();
    std::fs::create_dir(home.path().join("Desktop")).unwrap();
    std::fs::write(
        config.join("user-dirs.dirs"),
        format!(
            "XDG_DOCUMENTS_DIR=\"$HOME/Dokumente\"\nXDG_PICTURES_DIR=\"{}\"\nXDG_DESKTOP_DIR=\"$HOME/\"\nXDG_MUSIC_DIR=\"$HOME/Musik\"\n",
            elsewhere.path().join("Bilder").display()
        ),
    )
    .unwrap();
    let all = run_probe(&[("HOME", home.path()), ("XDG_CONFIG_HOME", &config)]);

    let docs = entry(&all, "documents");
    assert_eq!(docs["status"], "found");
    assert!(
        docs["path"].as_str().unwrap().ends_with("/Dokumente"),
        "{docs}"
    );
    assert_eq!(docs["moved"], true);

    let pictures = entry(&all, "pictures");
    assert_eq!(pictures["status"], "found");
    assert_eq!(pictures["moved"], true);

    // Set to the home folder: there is no desktop folder, even though ~/Desktop exists.
    assert_eq!(entry(&all, "desktop")["status"], "missing");
    // Configured but not there: missing, never guessed.
    assert_eq!(entry(&all, "music")["status"], "missing");
}

/// Linux without user-dirs settings: the plain folder names are used only if they exist.
#[cfg(all(unix, not(target_os = "macos")))]
#[test]
fn linux_without_settings_uses_plain_names_only_when_they_exist() {
    let home = tempfile::tempdir().unwrap();
    let config = home.path().join(".config");
    std::fs::create_dir_all(&config).unwrap();
    std::fs::create_dir(home.path().join("Documents")).unwrap();
    let all = run_probe(&[("HOME", home.path()), ("XDG_CONFIG_HOME", &config)]);
    let docs = entry(&all, "documents");
    assert_eq!(docs["status"], "found");
    assert_eq!(docs["moved"], false);
    assert_eq!(entry(&all, "pictures")["status"], "missing");
}

/// Windows: Documents moved to another drive and Pictures moved into OneDrive are found where they
/// are. This changes the signed-in user's folder settings, so it only runs on GitHub's throwaway
/// test machines, never on a person's laptop.
#[cfg(windows)]
#[test]
fn windows_finds_folders_moved_to_another_drive_or_onedrive() {
    if std::env::var("GITHUB_ACTIONS").as_deref() != Ok("true") {
        eprintln!("skipped: changes the user's folder settings; runs only on CI");
        return;
    }
    let base = tempfile::tempdir().unwrap();
    let drive_root = base.path().join("drive");
    std::fs::create_dir_all(drive_root.join("Moved Documents")).unwrap();
    let onedrive = base.path().join("OneDrive");
    std::fs::create_dir_all(onedrive.join("Pictures")).unwrap();

    let _drive = Subst::new("P:", &drive_root);
    let key = r"HKCU\Software\Microsoft\Windows\CurrentVersion\Explorer\User Shell Folders";
    let _docs = RegValue::set(key, "Personal", r"P:\Moved Documents");
    let pictures = onedrive.join("Pictures");
    let _pics = RegValue::set(key, "My Pictures", &pictures.to_string_lossy());

    let all = run_probe(&[("OneDrive", &onedrive)]);
    let docs = entry(&all, "documents");
    assert_eq!(docs["status"], "found", "{all}");
    assert_eq!(docs["moved"], true);
    assert_eq!(
        docs["storage"],
        serde_json::json!({"other-drive": {"drive": "P:"}})
    );
    let pics = entry(&all, "pictures");
    assert_eq!(pics["status"], "found", "{all}");
    assert_eq!(
        pics["storage"],
        serde_json::json!({"cloud": {"provider": "one-drive"}})
    );
}

/// A drive letter for a folder, removed again when dropped.
#[cfg(windows)]
struct Subst(String);

#[cfg(windows)]
impl Subst {
    fn new(letter: &str, target: &Path) -> Self {
        let ok = Command::new("subst")
            .arg(letter)
            .arg(target)
            .status()
            .unwrap()
            .success();
        assert!(ok, "subst {letter} failed");
        Self(letter.to_string())
    }
}

#[cfg(windows)]
impl Drop for Subst {
    fn drop(&mut self) {
        let _ = Command::new("subst").args([&self.0, "/D"]).status();
    }
}

/// A user folder setting, put back as it was when dropped.
#[cfg(windows)]
struct RegValue {
    key: String,
    name: String,
    old: Option<String>,
}

#[cfg(windows)]
impl RegValue {
    fn set(key: &str, name: &str, value: &str) -> Self {
        let out = Command::new("reg")
            .args(["query", key, "/v", name])
            .output()
            .unwrap();
        let old = String::from_utf8_lossy(&out.stdout)
            .lines()
            .find_map(|l| {
                l.split_once("REG_EXPAND_SZ")
                    .or_else(|| l.split_once("REG_SZ"))
            })
            .map(|(_, v)| v.trim().to_string());
        let ok = Command::new("reg")
            .args([
                "add",
                key,
                "/v",
                name,
                "/t",
                "REG_EXPAND_SZ",
                "/d",
                value,
                "/f",
            ])
            .status()
            .unwrap()
            .success();
        assert!(ok);
        Self {
            key: key.into(),
            name: name.into(),
            old,
        }
    }
}

#[cfg(windows)]
impl Drop for RegValue {
    fn drop(&mut self) {
        let _ = match &self.old {
            Some(v) => Command::new("reg")
                .args([
                    "add",
                    &self.key,
                    "/v",
                    &self.name,
                    "/t",
                    "REG_EXPAND_SZ",
                    "/d",
                    v,
                    "/f",
                ])
                .status(),
            None => Command::new("reg")
                .args(["delete", &self.key, "/v", &self.name, "/f"])
                .status(),
        };
    }
}

/// Linux: a folder on another drive (here the memory-backed /dev/shm) is reported as another
/// drive, by its mount point.
#[cfg(all(unix, not(target_os = "macos")))]
#[test]
fn linux_reports_a_folder_on_another_drive_by_its_mount_point() {
    use std::os::unix::fs::MetadataExt;
    let shm = Path::new("/dev/shm");
    let home = tempfile::tempdir().unwrap();
    let dev = |p: &Path| std::fs::metadata(p).map(|m| m.dev()).ok();
    if !shm.is_dir() || dev(shm) == dev(home.path()) {
        eprintln!("skipped: /dev/shm is not a separate drive here");
        return;
    }
    let pictures = tempfile::tempdir_in(shm).unwrap();
    let config = home.path().join(".config");
    std::fs::create_dir_all(&config).unwrap();
    std::fs::write(
        config.join("user-dirs.dirs"),
        format!("XDG_PICTURES_DIR=\"{}\"\n", pictures.path().display()),
    )
    .unwrap();
    let all = run_probe(&[("HOME", home.path()), ("XDG_CONFIG_HOME", &config)]);
    let pics = entry(&all, "pictures");
    assert_eq!(pics["status"], "found", "{all}");
    assert_eq!(
        pics["storage"],
        serde_json::json!({"other-drive": {"drive": "/dev/shm"}})
    );
}
