//! Mac: every app bundle in the Applications folders, with its Team ID.
//!
//! The rule is the Finder's own: a folder named `*.app` with a `Contents/Info.plist` is an app.
//! PCTwin looks in `/Applications` and one folder down (`/Applications/Utilities`, a maker's
//! own folder), and in each person's own `~/Applications`. For each it reads the bundle ID,
//! version, name and update feed (`SUFeedURL`, a Sparkle feed) from `Info.plist`; who signed it
//! (the Team ID) and what it is built for from `codesign`, through the maker crate's reader; and,
//! when the bundle carries an App Store receipt, the App Store's ID for it from Spotlight
//! (`mdls`). Nothing is run from the bundle.

use std::path::{Path, PathBuf};
use std::time::Duration;

use pctwin_maker::mac::{display_call, parse_display};
use pctwin_maker::run::{Call, Runner};
use pctwin_record::apps::{MAX_APP_TEXT, MAX_APP_URL};
use pctwin_record::{AppMaker, AppPlace, AppSystem, FoundApp, FoundWhere, Owner};

use super::{Collector, app_arch, clean};

/// The largest `Info.plist` read.
pub const MAX_PLIST: u64 = 1024 * 1024;
/// Most bundles looked at in one folder.
const MAX_BUNDLES: usize = 4096;
const TOOL_TIME: Duration = Duration::from_secs(60);

/// What a bundle's `Info.plist` says.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BundleInfo {
    pub bundle_id: String,
    pub name: String,
    pub version: Option<String>,
    pub update_feed: Option<String>,
}

fn plist_text(d: &plist::Dictionary, key: &str) -> Option<String> {
    d.get(key)?.as_string().map(str::to_string)
}

/// Reads an `Info.plist` (XML or binary). `folder` is the bundle's folder name, `Name.app`.
pub fn bundle_info(bytes: &[u8], folder: &str) -> Result<BundleInfo, String> {
    let value: plist::Value = plist::from_bytes(bytes).map_err(|e| e.to_string())?;
    let d = value
        .as_dictionary()
        .ok_or("its Info.plist is not a dictionary")?;
    let bundle_id = plist_text(d, "CFBundleIdentifier")
        .and_then(|b| clean(&b, MAX_APP_TEXT))
        .ok_or("it has no bundle ID")?;
    let name = ["CFBundleDisplayName", "CFBundleName"]
        .iter()
        .find_map(|k| plist_text(d, k).and_then(|n| clean(&n, MAX_APP_TEXT)))
        .or_else(|| clean(folder.trim_end_matches(".app"), MAX_APP_TEXT))
        .ok_or("it has no name")?;
    let version = ["CFBundleShortVersionString", "CFBundleVersion"]
        .iter()
        .find_map(|k| plist_text(d, k).and_then(|v| clean(&v, MAX_APP_TEXT)));
    let update_feed = plist_text(d, "SUFeedURL")
        .and_then(|u| super::windows::web_address(&u))
        .filter(|u| u.len() <= MAX_APP_URL);
    Ok(BundleInfo {
        bundle_id,
        name,
        version,
        update_feed,
    })
}

/// The oldest macOS a bundle says it runs on (`LSMinimumSystemVersion`), from its `Info.plist`.
pub fn minimum_system(bytes: &[u8]) -> Option<String> {
    let value: plist::Value = plist::from_bytes(bytes).ok()?;
    plist_text(value.as_dictionary()?, "LSMinimumSystemVersion")
        .and_then(|v| clean(&v, MAX_APP_TEXT))
}

/// `mdls -raw -name kMDItemAppStoreAdamID <bundle>`.
pub fn adam_id_call(bundle: &Path) -> Call {
    Call {
        program: PathBuf::from("/usr/bin/mdls"),
        args: vec![
            "-raw".into(),
            "-name".into(),
            "kMDItemAppStoreAdamID".into(),
            bundle.as_os_str().to_owned(),
        ],
        env: vec![],
        timeout: TOOL_TIME,
        stdin: None,
        cwd: None,
    }
}

/// The App Store ID `mdls` printed, or none ("(null)" when Spotlight has not indexed it).
pub fn parse_adam_id(out: &str) -> Option<u64> {
    let n: u64 = out.trim().parse().ok()?;
    (n > 0).then_some(n)
}

/// One bundle as a found app.
pub fn app_of(bundle: &Path, owner: Owner, runner: &dyn Runner) -> Result<FoundApp, String> {
    let folder = bundle
        .file_name()
        .and_then(|n| n.to_str())
        .ok_or("its folder name is not text")?
        .to_string();
    let plist = bundle.join("Contents").join("Info.plist");
    let meta = std::fs::symlink_metadata(&plist).map_err(|e| e.to_string())?;
    if !meta.is_file() || meta.len() > MAX_PLIST {
        return Err("its Info.plist is missing or too large".into());
    }
    let info = bundle_info(&std::fs::read(&plist).map_err(|e| e.to_string())?, &folder)?;
    let (maker, arch) = match runner.run(&display_call(bundle, TOOL_TIME)) {
        Ok(ran) => {
            let d = parse_display(&ran);
            let maker = match (d.team, d.unsigned) {
                (Some(t), _) => AppMaker::MacTeam { team_id: t.team_id },
                (None, true) => AppMaker::Unsigned,
                // Signed without a Team ID: Apple's own apps, or an ad-hoc signature.
                (None, false) => AppMaker::NotRead {
                    why: "Its signature names no developer team.".into(),
                },
            };
            (maker, app_arch(d.arch))
        }
        Err(e) => (
            AppMaker::NotRead {
                why: clean(
                    &format!("Its signature could not be read ({e})."),
                    MAX_APP_TEXT,
                )
                .unwrap_or_default(),
            },
            pctwin_record::AppArch::UNKNOWN,
        ),
    };
    let receipt = bundle.join("Contents").join("_MASReceipt").join("receipt");
    let app_store_id = if std::fs::symlink_metadata(&receipt).is_ok_and(|m| m.is_file()) {
        runner
            .run(&adam_id_call(bundle))
            .ok()
            .and_then(|r| parse_adam_id(&r.stdout))
    } else {
        None
    };
    Ok(FoundApp {
        name: info.name,
        version: info.version,
        publisher: None,
        owner,
        system: AppSystem::Mac,
        found: FoundWhere::MacBundle {
            bundle_id: info.bundle_id,
            folder: clean(&folder, MAX_APP_TEXT).ok_or("its folder name is empty")?,
            app_store_id,
        },
        arch,
        maker,
        web: None,
        update_feed: info.update_feed,
        installers: vec![],
        notes: vec![],
    })
}

fn is_bundle(p: &Path) -> bool {
    p.extension().is_some_and(|e| e.eq_ignore_ascii_case("app"))
        && std::fs::symlink_metadata(p).is_ok_and(|m| m.is_dir())
}

/// The bundles in `folder` and one folder down (links are not followed).
pub fn bundles_in(folder: &Path) -> Result<Vec<PathBuf>, String> {
    let mut out = Vec::new();
    for e in std::fs::read_dir(folder)
        .map_err(|e| e.to_string())?
        .flatten()
    {
        let p = e.path();
        if is_bundle(&p) {
            out.push(p);
        } else if e.file_type().is_ok_and(|t| t.is_dir())
            && let Ok(inner) = std::fs::read_dir(&p)
        {
            out.extend(inner.flatten().map(|e| e.path()).filter(|p| is_bundle(p)));
        }
        if out.len() >= MAX_BUNDLES {
            break;
        }
    }
    out.sort();
    Ok(out)
}

/// Every app in `places`: each a folder, whose apps are whose, and which place it is.
pub fn read_mac_apps(
    places: &[(PathBuf, Owner, AppPlace)],
    runner: &dyn Runner,
) -> pctwin_record::AppList {
    let mut c = Collector::default();
    for (folder, owner, place) in places {
        match bundles_in(folder) {
            Ok(bundles) => {
                for b in bundles {
                    match app_of(&b, owner.clone(), runner) {
                        Ok(app) => c.add(app),
                        Err(why) => {
                            // Listed by its folder, with why its details are missing.
                            let name = b
                                .file_name()
                                .and_then(|n| n.to_str())
                                .and_then(|n| clean(n.trim_end_matches(".app"), MAX_APP_TEXT));
                            match name {
                                Some(name) => c.add(FoundApp {
                                    name: name.clone(),
                                    version: None,
                                    publisher: None,
                                    owner: owner.clone(),
                                    system: AppSystem::Mac,
                                    found: FoundWhere::MacBundle {
                                        bundle_id: String::new(),
                                        folder: format!("{name}.app"),
                                        app_store_id: None,
                                    },
                                    arch: pctwin_record::AppArch::UNKNOWN,
                                    maker: AppMaker::NotRead {
                                        why: clean(
                                            &format!("Its details could not be read ({why})."),
                                            MAX_APP_TEXT,
                                        )
                                        .unwrap_or_default(),
                                    },
                                    web: None,
                                    update_feed: None,
                                    installers: vec![],
                                    notes: vec![],
                                }),
                                None => c.not_read(
                                    owner.clone(),
                                    *place,
                                    format!("An app could not be read ({why})."),
                                ),
                            }
                        }
                    }
                }
            }
            // A person with no Applications folder of their own installed nothing there.
            Err(_) if *place == AppPlace::MacPersonApplications && !folder.exists() => {}
            Err(e) => c.not_read(
                owner.clone(),
                *place,
                format!("The apps folder could not be read ({e})."),
            ),
        }
    }
    c.finish()
}

#[cfg(test)]
mod tests {
    use super::*;
    use pctwin_maker::run::{Ran, RunError};

    const PLIST: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0"><dict>
<key>CFBundleIdentifier</key><string>com.example.painter</string>
<key>CFBundleName</key><string>Painter</string>
<key>CFBundleShortVersionString</key><string>4.2.1</string>
<key>CFBundleVersion</key><string>4210</string>
<key>SUFeedURL</key><string>https://example.invalid/appcast.xml</string>
</dict></plist>"#;

    #[test]
    fn a_bundle_says_who_it_is() {
        let i = bundle_info(PLIST.as_bytes(), "Painter.app").unwrap();
        assert_eq!(i.bundle_id, "com.example.painter");
        assert_eq!(i.name, "Painter");
        assert_eq!(i.version.as_deref(), Some("4.2.1"));
        assert_eq!(
            i.update_feed.as_deref(),
            Some("https://example.invalid/appcast.xml")
        );
        let no_id = PLIST.replace("CFBundleIdentifier", "Other");
        assert!(bundle_info(no_id.as_bytes(), "Painter.app").is_err());
        let file_feed = PLIST.replace("https://example.invalid/appcast.xml", "file:///etc/passwd");
        assert_eq!(
            bundle_info(file_feed.as_bytes(), "Painter.app")
                .unwrap()
                .update_feed,
            None
        );
        assert!(bundle_info(b"not a plist", "x.app").is_err());
    }

    #[test]
    fn app_store_ids_from_spotlight() {
        assert_eq!(parse_adam_id("497799835\n"), Some(497_799_835));
        assert_eq!(parse_adam_id("(null)"), None);
        assert_eq!(parse_adam_id("0"), None);
    }

    /// Answers `codesign` as a Developer ID app built for both chips, and `mdls` with an ID.
    struct Tools;
    impl Runner for Tools {
        fn run(&self, call: &Call) -> Result<Ran, RunError> {
            if call.program.ends_with("codesign") {
                Ok(Ran {
                    code: Some(0),
                    stdout: String::new(),
                    stderr: "Executable=/Applications/Painter.app/Contents/MacOS/Painter\nFormat=app bundle with Mach-O universal (x86_64 arm64)\nTeamIdentifier=ABCDE12345\n".into(),
                })
            } else {
                Ok(Ran {
                    code: Some(0),
                    stdout: "12345\n".into(),
                    stderr: String::new(),
                })
            }
        }
    }

    /// Answers `codesign` as an unsigned bundle, or as one signed without a team (Apple's own,
    /// or ad hoc).
    struct Unsigned(bool);
    impl Runner for Unsigned {
        fn run(&self, _: &Call) -> Result<Ran, RunError> {
            Ok(Ran {
                code: Some(1),
                stdout: String::new(),
                stderr: if self.0 {
                    "/Applications/X.app: code object is not signed at all
"
                    .into()
                } else {
                    "Executable=/Applications/X.app/Contents/MacOS/X
Signature=adhoc
TeamIdentifier=not set
"
                    .into()
                },
            })
        }
    }

    #[test]
    fn an_unsigned_bundle_is_said_to_be_unsigned() {
        let dir = tempfile::tempdir().unwrap();
        let b = dir.path().join("Painter.app");
        std::fs::create_dir_all(b.join("Contents")).unwrap();
        std::fs::write(b.join("Contents").join("Info.plist"), PLIST).unwrap();
        assert_eq!(
            app_of(&b, Owner::Shared, &Unsigned(true)).unwrap().maker,
            AppMaker::Unsigned
        );
        assert!(matches!(
            app_of(&b, Owner::Shared, &Unsigned(false)).unwrap().maker,
            AppMaker::NotRead { .. }
        ));
    }

    #[test]
    fn every_bundle_in_the_folders_is_found_with_its_team() {
        let dir = tempfile::tempdir().unwrap();
        let make = |rel: &str, plist: Option<&str>, receipt: bool| {
            let b = dir.path().join(rel);
            std::fs::create_dir_all(b.join("Contents")).unwrap();
            if let Some(p) = plist {
                std::fs::write(b.join("Contents").join("Info.plist"), p).unwrap();
            }
            if receipt {
                std::fs::create_dir_all(b.join("Contents/_MASReceipt")).unwrap();
                std::fs::write(b.join("Contents/_MASReceipt/receipt"), b"r").unwrap();
            }
        };
        make("Painter.app", Some(PLIST), true);
        make(
            "Utilities/Other.app",
            Some(&PLIST.replace("painter", "other")),
            false,
        );
        make("Broken.app", None, false);
        std::fs::write(dir.path().join("notes.txt"), b"x").unwrap();
        let list = read_mac_apps(
            &[(
                dir.path().to_path_buf(),
                Owner::Shared,
                AppPlace::MacApplications,
            )],
            &Tools,
        );
        assert_eq!(list.found.len(), 3, "{list:?}");
        let painter = list.found.iter().find(|a| a.name == "Painter").unwrap();
        assert_eq!(
            painter.maker,
            AppMaker::MacTeam {
                team_id: "ABCDE12345".into()
            }
        );
        assert!(painter.arch.x64 && painter.arch.arm64);
        assert!(matches!(
            &painter.found,
            FoundWhere::MacBundle {
                app_store_id: Some(12345),
                ..
            }
        ));
        let other = list.found.iter().find(|a| matches!(&a.found, FoundWhere::MacBundle { bundle_id, .. } if bundle_id == "com.example.other")).unwrap();
        assert!(matches!(
            &other.found,
            FoundWhere::MacBundle {
                app_store_id: None,
                ..
            }
        ));
        // Never dropped: the bundle without a plist is listed with why.
        let broken = list.found.iter().find(|a| a.name == "Broken").unwrap();
        assert!(matches!(&broken.maker, AppMaker::NotRead { .. }));
        list.validate().unwrap();
        // A folder that cannot be read is said.
        let gone = read_mac_apps(
            &[(
                dir.path().join("missing"),
                Owner::Shared,
                AppPlace::MacApplications,
            )],
            &Tools,
        );
        assert_eq!(gone.not_read.len(), 1);
        // A person without their own Applications folder installed nothing there.
        let none = read_mac_apps(
            &[(
                dir.path().join("missing"),
                Owner::Shared,
                AppPlace::MacPersonApplications,
            )],
            &Tools,
        );
        assert!(none.is_empty());
    }
}
