//! Third security review, finding 3: Windows' installer-package checker reads the package by its
//! name, not through the file PCTwin holds, so a folder link swapped while Windows looks can make
//! it check another file. A package checked anywhere but the administrator helper's own copy is
//! therefore never "install".
// Tests copy and change files, and make folder links.
#![allow(clippy::disallowed_methods)]
#![cfg(windows)]

mod common;

use std::fs::File;
use std::path::{Path, PathBuf};

use common::os_checks;
use pctwin_maker::check::{Expected, Limits, SystemChecks, SystemSignature, check};
use pctwin_maker::facts::{Arch, Decision, Format, Maker, Malware, Signature, System};

fn junction(link: &Path, target: &Path) {
    let _ = std::process::Command::new("cmd")
        .args(["/d", "/c", "rmdir"])
        .arg(link)
        .output();
    let o = std::process::Command::new("cmd")
        .args(["/d", "/c", "mklink", "/J"])
        .arg(link)
        .arg(target)
        .output()
        .unwrap();
    assert!(o.status.success(), "{o:?}");
}

/// The race, made certain: the folder link points at the good copy only while Windows checks
/// the signature, and at the changed copy the rest of the time.
struct Swap<'a, S: SystemChecks> {
    inner: S,
    link: &'a Path,
    good: &'a Path,
    evil: &'a Path,
}

impl<S: SystemChecks> SystemChecks for Swap<'_, S> {
    fn signature(
        &self,
        file: &File,
        path: &Path,
        format: Format,
        limits: &Limits,
    ) -> SystemSignature {
        junction(self.link, self.good);
        let s = self.inner.signature(file, path, format, limits);
        junction(self.link, self.evil);
        s
    }
    fn malware(
        &self,
        path: &Path,
        format: Format,
        signature: &Signature,
        limits: &Limits,
    ) -> Malware {
        self.inner.malware(path, format, signature, limits)
    }
}

/// A signed Windows Installer package on this laptop (Windows keeps copies of installed ones).
fn signed_msi() -> Option<PathBuf> {
    std::fs::read_dir(r"C:\Windows\Installer")
        .ok()?
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|x| x.eq_ignore_ascii_case("msi")))
        .filter(|p| std::fs::metadata(p).is_ok_and(|m| m.len() < 5_000_000))
        .find(|p| pctwin_maker::check::windows_maker_of(p).is_ok())
}

#[test]
fn a_package_changed_behind_a_folder_link_is_never_installed() {
    let Some(good_src) = signed_msi() else {
        eprintln!("no signed installer package on this laptop: nothing to check");
        return;
    };
    let maker = pctwin_maker::check::windows_maker_of(&good_src).unwrap();
    let bytes = std::fs::read(&good_src).unwrap();
    let mut changed = bytes.clone();
    let at = changed.len() / 3;
    for b in &mut changed[at..at + 64] {
        *b ^= 0xFF;
    }
    let dir = tempfile::tempdir().unwrap();
    let (good, evil, link) = (
        dir.path().join("good"),
        dir.path().join("evil"),
        dir.path().join("j"),
    );
    std::fs::create_dir_all(&good).unwrap();
    std::fs::create_dir_all(&evil).unwrap();
    std::fs::write(good.join("setup.msi"), &bytes).unwrap();
    std::fs::write(evil.join("setup.msi"), &changed).unwrap();
    junction(&link, &evil);
    let path = link.join("setup.msi");
    let sys = Swap {
        inner: os_checks(),
        link: &link,
        good: &good,
        evil: &evil,
    };
    let expected = Expected {
        system: System::Windows,
        maker: Maker::Windows(maker),
        formats: vec![Format::WindowsInstaller],
        arch: Arch::X64,
        bytes: None,
    };
    let v = check(&path, &expected, &sys, &Limits::default()).unwrap();
    let _ = std::process::Command::new("cmd")
        .args(["/d", "/c", "rmdir"])
        .arg(&link)
        .output();
    assert!(
        !matches!(v.decide(), Decision::Install { .. }),
        "a changed package was approved: {v:?}"
    );
    assert_ne!(v.signature, Signature::Trusted, "{v:?}");
}

/// The helper's own copy still counts: the same good package, checked as the helper's copy, is
/// approved; without Windows' own answers it never is.
#[test]
fn the_helpers_own_copy_is_checked_by_windows() {
    struct Staged(PathBuf);
    impl pctwin_maker::HelperStaged for Staged {
        fn staged_path(&self) -> &Path {
            &self.0
        }
    }
    let Some(good_src) = signed_msi() else {
        eprintln!("no signed installer package on this laptop: nothing to check");
        return;
    };
    let maker = pctwin_maker::check::windows_maker_of(&good_src).unwrap();
    let dir = tempfile::tempdir().unwrap();
    let copy = dir.path().join("setup.msi");
    std::fs::copy(&good_src, &copy).unwrap();
    let expected = Expected {
        system: System::Windows,
        maker: Maker::Windows(maker),
        formats: vec![Format::WindowsInstaller],
        arch: Arch::X64,
        bytes: None,
    };
    let staged = Staged(copy.clone());
    let v =
        pctwin_maker::check_staged(&staged, &expected, &os_checks(), &Limits::default()).unwrap();
    assert_eq!(v.signature, Signature::Trusted, "{v:?}");
    // The same file outside the helper's copy: never trusted.
    let v = check(&copy, &expected, &os_checks(), &Limits::default()).unwrap();
    assert!(matches!(v.signature, Signature::NotChecked { .. }), "{v:?}");
    // No answers from Windows: never trusted either.
    let none = pctwin_maker::NativeChecks::default();
    let v = pctwin_maker::check_staged(&staged, &expected, &none, &Limits::default()).unwrap();
    assert_ne!(v.signature, Signature::Trusted, "{v:?}");
}
