//! The install chain on Linux (2.1 joined to the move, 28 October 2026): each app from the same
//! source the old laptop had it from, through the system's own package tools, which ask the
//! system's own administrator prompt (polkit) themselves. PCTwin runs nothing as an administrator
//! here and adds no privileged code.
//!
//! 1. **Which route.** The first of the app's planned routes this laptop can take:
//!    - apt and dnf packages through PackageKit (`pkcon`), the desktops' own install service
//!      (GNOME Software and KDE Discover use it on Ubuntu and Fedora alike). Every package goes in
//!      one transaction, so polkit asks once; if that transaction fails, each is tried alone.
//!    - a Flatpak from a remote here with the old laptop's very address **and** the same signing
//!      keyring (its SHA-256), installed for the same people as before (for this person alone, or
//!      for everyone, which Flatpak's own helper asks polkit for). A remote with the same address
//!      but another key is never used.
//!    - a Snap through `snapd`, which asks polkit itself.
//!
//!    An app with no route this laptop can take (a Homebrew cask or a Mac App Store app on a Mac,
//!    a tool that is not installed) ends needs-action: the person installs it; their data is ready.
//! 2. **What is really there.** The laptop's own lists are read again after the installs. An app
//!    counts as `reinstalled` only when it is there under the same package identity, from the same
//!    maker (the same repository path and release, the same package key, the same Flatpak remote
//!    and key, the same Snap publisher) and at the person's version or newer. Installed but from
//!    another source or older: needs-action, and undo lists it. Not there: failed.
//!
//! Names only ever reach a tool after checking they are plain package names, so none can be read
//! as one of the tool's options. Each tool gets a fixed environment and nothing of PCTwin's own.

use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::time::Duration;

use pctwin_apps::routes::Route;
use pctwin_journal::RestorePoint;
use pctwin_maker::run::{Call, Ran, Runner};
use pctwin_record::{AppList, AppMaker, FoundApp, FoundWhere};
use pctwin_types::{ItemResult, ItemStatus, Reason};

use crate::plan::AppEntry;
use crate::run::{AppResult, Installer};

/// The longest one install may take.
pub const INSTALL_TIME: Duration = Duration::from_secs(30 * 60);
/// The longest reading a tool's list may take.
const LIST_TIME: Duration = Duration::from_secs(120);

/// The system's package tools on this laptop (`None`: not installed).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Tools {
    pub pkcon: Option<PathBuf>,
    pub flatpak: Option<PathBuf>,
    pub snap: Option<PathBuf>,
}

impl Tools {
    /// The tools where the systems install them (never looked up through `PATH`).
    pub fn here() -> Self {
        let at = |p: &str| {
            let p = PathBuf::from(p);
            std::fs::metadata(&p)
                .is_ok_and(|m| m.is_file())
                .then_some(p)
        };
        Self {
            pkcon: at("/usr/bin/pkcon"),
            flatpak: at("/usr/bin/flatpak"),
            snap: at("/usr/bin/snap"),
        }
    }

    /// Whether any app can be installed here.
    pub fn any(&self) -> bool {
        self.pkcon.is_some() || self.flatpak.is_some() || self.snap.is_some()
    }
}

/// A Debian or RPM package name, a Snap name, a Flatpak remote: letters, digits and `+ . _ -`,
/// never starting with `-` (so never one of a tool's options).
fn plain_name(s: &str) -> bool {
    !s.is_empty()
        && s.len() <= 128
        && !s.starts_with(['-', '.'])
        && s.bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'+' | b'.' | b'_' | b'-'))
}

/// One install this laptop can make.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Step {
    Package(String),
    Flatpak {
        remote: String,
        app_id: String,
        system_wide: bool,
    },
    Snap(String),
}

/// The Linux installer (see the module notes).
pub struct LinuxInstaller<'a> {
    pub runner: &'a dyn Runner,
    pub tools: Tools,
    /// The person's home, for their own Flatpak installation.
    pub home: PathBuf,
    /// What the tools are given of the person's session (`XDG_RUNTIME_DIR`,
    /// `DBUS_SESSION_BUS_ADDRESS`, `XDG_DATA_HOME`), so the system's prompt finds their session.
    pub session: Vec<(OsString, OsString)>,
    /// The SHA-256 of a Flatpak remote's keyring here, by (name, for everyone).
    pub flatpak_key: &'a dyn Fn(&str, bool) -> Option<[u8; 32]>,
    /// The laptop's own lists, read again after the installs.
    pub list: &'a dyn Fn() -> AppList,
}

impl LinuxInstaller<'_> {
    fn call(&self, program: &Path, args: &[&str], timeout: Duration) -> Call {
        let mut env: Vec<(OsString, OsString)> = vec![
            ("LC_ALL".into(), "C".into()),
            ("PATH".into(), "/usr/bin:/bin:/usr/sbin:/sbin".into()),
            ("HOME".into(), self.home.clone().into_os_string()),
        ];
        env.extend(self.session.iter().cloned());
        Call {
            program: program.to_path_buf(),
            args: args.iter().map(Into::into).collect(),
            env,
            timeout,
            stdin: None,
            cwd: Some(PathBuf::from("/")),
        }
    }

    fn run(&self, program: &Path, args: &[&str], timeout: Duration) -> Option<Ran> {
        self.runner.run(&self.call(program, args, timeout)).ok()
    }

    /// The Flatpak remote here with `url` and the keyring `key`, for `system_wide` (the same
    /// installation first, then the other).
    fn flatpak_remote(
        &self,
        url: &str,
        key: &[u8; 32],
        system_wide: bool,
    ) -> Option<(String, bool)> {
        let flatpak = self.tools.flatpak.as_deref()?;
        let ran = self.run(
            flatpak,
            &["remotes", "--columns=name,url,options"],
            LIST_TIME,
        )?;
        if ran.code != Some(0) {
            return None;
        }
        let remotes = pctwin_apps::found::linux::parse_flatpak_remotes(&ran.stdout);
        let same = |s: bool| {
            remotes.iter().find_map(|((name, sys), u)| {
                (*sys == s
                    && u.trim_end_matches('/') == url.trim_end_matches('/')
                    && plain_name(name)
                    && (self.flatpak_key)(name, s).as_ref() == Some(key))
                .then(|| (name.clone(), s))
            })
        };
        same(system_wide).or_else(|| same(!system_wide))
    }

    /// The first route this laptop can take for `e`.
    fn step_for(&self, e: &AppEntry) -> Option<Step> {
        e.plan.routes.iter().find_map(|r| match r {
            Route::Apt { package } | Route::Dnf { package }
                if self.tools.pkcon.is_some() && plain_name(package) =>
            {
                Some(Step::Package(package.clone()))
            }
            Route::Flatpak {
                remote_url,
                app_id,
                key,
            } if pctwin_maker::linux::valid_app_id(app_id) => {
                let wide = matches!(
                    e.subject().found,
                    FoundWhere::Flatpak {
                        system_wide: true,
                        ..
                    }
                );
                let (remote, system_wide) = self.flatpak_remote(remote_url, key, wide)?;
                Some(Step::Flatpak {
                    remote,
                    app_id: app_id.clone(),
                    system_wide,
                })
            }
            Route::Snap { name } if self.tools.snap.is_some() && plain_name(name) => {
                Some(Step::Snap(name.clone()))
            }
            _ => None,
        })
    }

    /// Runs one step (or several packages in one transaction); `Err` holds the reason.
    fn act(&self, step: &[&Step]) -> Result<(), Reason> {
        let ran = match step {
            [
                Step::Flatpak {
                    remote,
                    app_id,
                    system_wide,
                },
            ] => {
                let Some(flatpak) = self.tools.flatpak.as_deref() else {
                    return Err(Reason::ManualInstall);
                };
                let installation = if *system_wide { "--system" } else { "--user" };
                self.run(
                    flatpak,
                    &[
                        "install",
                        "--noninteractive",
                        "-y",
                        installation,
                        remote,
                        app_id,
                    ],
                    INSTALL_TIME,
                )
            }
            [Step::Snap(name)] => {
                let Some(snap) = self.tools.snap.as_deref() else {
                    return Err(Reason::ManualInstall);
                };
                self.run(snap, &["install", name], INSTALL_TIME)
            }
            packages => {
                let Some(pkcon) = self.tools.pkcon.as_deref() else {
                    return Err(Reason::ManualInstall);
                };
                let mut args = vec!["--noninteractive", "install"];
                for s in packages {
                    match s {
                        Step::Package(p) => args.push(p),
                        _ => return Err(Reason::InstallFailed),
                    }
                }
                self.run(pkcon, &args, INSTALL_TIME)
            }
        };
        match ran {
            Some(r) if r.code == Some(0) => Ok(()),
            // The system's prompt was dismissed or refused: the person's step, not a failure.
            Some(r) if refused(&r) => Err(Reason::Permission),
            _ => Err(Reason::InstallFailed),
        }
    }
}

/// Whether a tool said the system's prompt was dismissed or the person is not allowed (polkit's
/// own words, in the C locale every tool is run in; pkexec's exit codes 126 and 127).
fn refused(r: &Ran) -> bool {
    let words = format!("{} {}", r.stdout, r.stderr).to_ascii_lowercase();
    matches!(r.code, Some(126 | 127))
        || words.contains("not authorized")
        || words.contains("not authorised")
        || words.contains("authentication is required")
        || words.contains("dismissed")
}

/// The repository path and release of an apt origin ("http://gb.archive.ubuntu.com/ubuntu
/// noble/main" is `/ubuntu noble/main`): a country's mirror of the same archive is the same
/// source; another path (a personal archive) or another release is not.
fn apt_source(origin: &str) -> Option<(String, String)> {
    let (url, release) = origin.split_once(' ')?;
    let rest = url.split_once("://").map_or(url, |(_, r)| r);
    let path = rest.find('/').map_or("/", |i| &rest[i..]);
    Some((
        path.trim_end_matches('/').to_string(),
        release.trim().to_string(),
    ))
}

/// Whether `here` (just installed) comes from the same maker as `had` (the old laptop's).
fn same_maker(had: &AppMaker, here: &AppMaker) -> bool {
    match (had, here) {
        (AppMaker::Repository { origin: a }, AppMaker::Repository { origin: b }) => {
            a == b || matches!((apt_source(a), apt_source(b)), (Some(x), Some(y)) if x == y)
        }
        (AppMaker::Rpm { key: a }, AppMaker::Rpm { key: b }) => a.eq_ignore_ascii_case(b),
        (
            AppMaker::Flatpak {
                remote_url: a,
                key: ka,
                ..
            },
            AppMaker::Flatpak {
                remote_url: b,
                key: kb,
                ..
            },
        ) => a.trim_end_matches('/') == b.trim_end_matches('/') && ka == kb,
        _ => false,
    }
}

/// Whether two apps are the same package (by the system's own identity, never by a name alike).
fn same_package(a: &FoundWhere, b: &FoundWhere) -> bool {
    match (a, b) {
        (FoundWhere::Deb { package: x }, FoundWhere::Deb { package: y })
        | (FoundWhere::Rpm { package: x }, FoundWhere::Rpm { package: y })
        | (FoundWhere::Snap { name: x }, FoundWhere::Snap { name: y }) => x == y,
        (FoundWhere::Flatpak { app_id: x, .. }, FoundWhere::Flatpak { app_id: y, .. }) => x == y,
        _ => false,
    }
}

/// The honest result for `e` after its install, as the laptop's own `list` shows it now.
pub fn verdict(e: &AppEntry, list: &AppList) -> AppResult {
    let app: &FoundApp = e.subject();
    let Some(here) = list
        .found
        .iter()
        .find(|h| same_package(&h.found, &app.found))
    else {
        return AppResult::because(Reason::InstallFailed);
    };
    let todo = pctwin_apps::after::checklist(
        app,
        e.plan.card.sign_in,
        e.plan.card.licence_key,
        pctwin_apps::after::Installed::Done {
            restart_needed: false,
        },
        &[],
    );
    let fits = same_maker(&app.maker, &here.maker)
        && pctwin_apps::version::not_a_downgrade(here.version.as_deref(), app.version.as_deref());
    let result = if fits {
        ItemResult::done(ItemStatus::Reinstalled)
            .unwrap_or(ItemResult::because(Reason::FinishByHand))
    } else {
        // There, but from another source or older: the person looks at it.
        ItemResult::because(Reason::FinishByHand)
    };
    AppResult::installed(result, todo)
}

impl Installer for LinuxInstaller<'_> {
    fn restore_point(&mut self) -> RestorePoint {
        RestorePoint::NotWindows
    }

    fn asked(&self) -> Option<RestorePoint> {
        Some(RestorePoint::NotWindows)
    }

    fn install(&mut self, apps: &[&AppEntry], done: &mut dyn FnMut()) -> Vec<AppResult> {
        let steps: Vec<Option<Step>> = apps.iter().map(|e| self.step_for(e)).collect();
        let mut tried: Vec<Option<Result<(), Reason>>> = vec![None; apps.len()];
        // Every package in one transaction (one prompt); alone each if that one failed.
        let packages: Vec<usize> = (0..apps.len())
            .filter(|i| matches!(steps[*i], Some(Step::Package(_))))
            .collect();
        if !packages.is_empty() {
            let all: Vec<&Step> = packages.iter().filter_map(|i| steps[*i].as_ref()).collect();
            match self.act(&all) {
                Ok(()) => packages.iter().for_each(|i| tried[*i] = Some(Ok(()))),
                // Refused: nobody is asked again for each one.
                Err(Reason::Permission) => packages
                    .iter()
                    .for_each(|i| tried[*i] = Some(Err(Reason::Permission))),
                Err(_) if packages.len() == 1 => {
                    tried[packages[0]] = Some(Err(Reason::InstallFailed))
                }
                Err(_) => {
                    for i in &packages {
                        if let Some(s) = steps[*i].as_ref() {
                            tried[*i] = Some(self.act(&[s]));
                        }
                    }
                }
            }
        }
        for (i, s) in steps.iter().enumerate() {
            if tried[i].is_none()
                && let Some(s) = s
            {
                tried[i] = Some(self.act(&[s]));
            }
        }
        let list = if tried.iter().any(|t| matches!(t, Some(Ok(())))) {
            Some((self.list)())
        } else {
            None
        };
        apps.iter()
            .zip(tried)
            .map(|(e, t)| {
                let r = match (t, &list) {
                    (None, _) => AppResult::because(Reason::ManualInstall),
                    (Some(Err(why)), _) => AppResult::because(why),
                    (Some(Ok(())), Some(list)) => verdict(e, list),
                    (Some(Ok(())), None) => AppResult::because(Reason::InstallFailed),
                };
                done();
                r
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plain_names_only() {
        for ok in [
            "firefox",
            "libreoffice-writer",
            "g++",
            "python3.12",
            "org_x",
        ] {
            assert!(plain_name(ok), "{ok}");
        }
        for bad in [
            "",
            "-y",
            "--allow-untrusted",
            ".hidden",
            "a b",
            "a;b",
            "a/b",
            "ä",
        ] {
            assert!(!plain_name(bad), "{bad}");
        }
    }

    /// A Fedora-style laptop (dnf through PackageKit): an RPM app found on the old laptop is
    /// planned from dnf, and reinstalled only when the laptop's own list shows it signed with
    /// the same package key.
    #[test]
    fn a_fedora_package_is_reinstalled_only_with_the_same_key() {
        use pctwin_apps::routes::{self, Hints, LinuxPackages, Route, Target, WingetBy};
        use pctwin_record::{AppArch, AppSystem, Owner};
        let rpm = |key: &str| FoundApp {
            name: "lab-fedora".into(),
            version: Some("1.0".into()),
            publisher: None,
            owner: Owner::Shared,
            system: AppSystem::Linux,
            found: FoundWhere::Rpm {
                package: "lab-fedora".into(),
            },
            arch: AppArch::UNKNOWN,
            maker: AppMaker::Rpm { key: key.into() },
            web: None,
            update_feed: None,
            installers: vec![],
            notes: vec![],
        };
        let fedora = Target {
            system: AppSystem::Linux,
            arch: AppArch::UNKNOWN,
            linux: LinuxPackages::from_os_release(
                "ID=nobara
ID_LIKE=\"fedora\"
",
            ),
        };
        let none = |_: &FoundApp| Vec::<(String, WingetBy)>::new();
        let app = rpm("809a8d7ceb10b464");
        let plan = routes::plan(
            &app,
            fedora,
            &routes::Here::Absent,
            &Hints {
                rows: routes::rows::shipped(),
                winget: &none,
                casks: None,
            },
        );
        assert_eq!(
            plan.routes,
            vec![Route::Dnf {
                package: "lab-fedora".into()
            }]
        );
        let e = AppEntry { app, plan };
        let list = |key: &str| AppList {
            found: vec![rpm(key)],
            not_read: vec![],
        };
        assert_eq!(
            verdict(&e, &list("809A8D7CEB10B464")).result.status(),
            ItemStatus::Reinstalled
        );
        assert_eq!(
            verdict(&e, &list("0000000000000000")).result.reason(),
            Some(Reason::FinishByHand)
        );
        assert_eq!(
            verdict(&e, &AppList::default()).result.reason(),
            Some(Reason::InstallFailed)
        );
    }

    #[test]
    fn a_country_mirror_is_the_same_archive_another_path_or_release_is_not() {
        let r = |o: &str| AppMaker::Repository { origin: o.into() };
        let had = r("http://archive.ubuntu.com/ubuntu noble/main");
        assert!(same_maker(
            &had,
            &r("http://gb.archive.ubuntu.com/ubuntu noble/main")
        ));
        assert!(same_maker(
            &had,
            &r("https://archive.ubuntu.com/ubuntu/ noble/main")
        ));
        assert!(!same_maker(
            &had,
            &r("http://archive.ubuntu.com/ubuntu oracular/main")
        ));
        assert!(!same_maker(
            &had,
            &r("https://ppa.launchpadcontent.net/x/y/ubuntu noble/main")
        ));
        let snap = r("snap store: mozilla");
        assert!(same_maker(&snap, &r("snap store: mozilla")));
        assert!(!same_maker(&snap, &r("snap store: someone")));
        assert!(!same_maker(
            &AppMaker::Rpm { key: "ab".into() },
            &AppMaker::Rpm { key: "cd".into() }
        ));
        let fp = |k: u8| AppMaker::Flatpak {
            remote_url: "https://dl.flathub.org/repo/".into(),
            app_id: "org.x.App".into(),
            key: [k; 32],
        };
        assert!(same_maker(&fp(1), &fp(1)));
        assert!(
            !same_maker(&fp(1), &fp(2)),
            "the same address with another key is not the maker"
        );
    }
}
