//! Step 5: after an install, what is really there, and the one honest result word.
//!
//! - **What is there** ([`probe`]): the new laptop's own list of installed programs is read
//!   again and the app looked for by its product code, or else by exactly its name and
//!   publisher, never by a similar name. Its version must be the person's or newer, and its
//!   scope the one installed.
//! - **Extras** ([`extras`]): every entry that appeared during the install other than the app's
//!   own is named: an installer that slipped in another program (an "offer") is said, and the
//!   app ends needs-action, never reinstalled (T21-037).
//! - **The result word** ([`result`]): `reinstalled` only when the helper reported the install
//!   done, the app is found at the person's version or newer, nothing else appeared, no restart
//!   is pending and it opened as the person (or no check was asked). Anything else is
//!   needs-action or failed, with its reason.
//! - **What the person still has to do** ([`checklist`]): sign in, enter a licence key, the
//!   registration the old laptop's own list held (who it is registered to, its product ID; never
//!   a key read from a protected store), restart.
//! - **The receipt for 2.5** ([`InstallReceipt`]): what 2.1 observed, carrying no permission of any
//!   kind. 2.5 never trusts it: it looks at the laptop itself ([`InstallReceipt::still_true`]) and
//!   asks its own role for its own grant (T21-049, T21-050).

pub use pctwin_helper::broker::actions::InstallScope;
use pctwin_record::{AppNote, FoundApp, FoundWhere, RegistryView};
use pctwin_types::{ItemResult, ItemStatus, Reason};

use crate::found::windows::RawEntry;
use crate::version::{Order, compare};

/// What the new laptop's own list says about the app after the install.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Probe {
    pub found: bool,
    pub version: Option<String>,
    pub view: Option<RegistryView>,
    /// The key of the entry found (to tell it apart from extras).
    pub key: Option<String>,
}

fn text<'a>(e: &'a RawEntry, name: &str) -> Option<&'a str> {
    match e.values.get(name)? {
        crate::found::windows::RegValue::Text(t) => Some(t.trim()).filter(|t| !t.is_empty()),
        crate::found::windows::RegValue::Number(_) => None,
    }
}

/// Whether an entry in `view` can stand for an install of `scope`: a machine install only in the
/// machine's own lists (64- or 32-bit), which only an administrator can write; a per-person
/// install only in the person's own list (fifth review, finding 5: the person, or any program
/// running as them, can write that list, so it never proves a machine install).
pub fn view_fits(scope: InstallScope, view: RegistryView) -> bool {
    match scope {
        InstallScope::Machine => matches!(view, RegistryView::Machine | RegistryView::Machine32),
        InstallScope::User => view == RegistryView::Person,
    }
}

/// Finds `app`, installed for `scope`, in the new laptop's entries: only entries in a list that
/// fits the scope ([`view_fits`]) are looked at.
pub fn probe(app: &FoundApp, scope: InstallScope, entries: &[RawEntry]) -> Probe {
    let fits: Vec<&RawEntry> = entries
        .iter()
        .filter(|e| view_fits(scope, e.view))
        .collect();
    let entries = fits.as_slice();
    let code = match &app.found {
        FoundWhere::WindowsEntry { product_code, .. } => product_code.as_deref(),
        _ => None,
    };
    let hit = entries
        .iter()
        .copied()
        .filter(|e| e.shown())
        .find(|e| match code {
            Some(c) => e.product_code().is_some_and(|p| p.eq_ignore_ascii_case(c)),
            None => {
                text(e, "displayname") == Some(app.name.as_str())
                    && text(e, "publisher").is_some()
                    && text(e, "publisher") == app.publisher.as_deref()
            }
        });
    // An MSI app whose product code changed between versions (most upgrades keep it, some
    // makers change it every release) is found by its exact name and publisher instead.
    let hit = hit.or_else(|| {
        code.and_then(|_| {
            entries.iter().copied().filter(|e| e.shown()).find(|e| {
                text(e, "displayname").is_some_and(|n| names_match(n, &app.name))
                    && text(e, "publisher").is_some()
                    && text(e, "publisher") == app.publisher.as_deref()
            })
        })
    });
    match hit {
        Some(e) => Probe {
            found: true,
            version: text(e, "displayversion").map(str::to_string),
            view: Some(e.view),
            key: Some(e.key.clone()),
        },
        None => Probe {
            found: false,
            version: None,
            view: None,
            key: None,
        },
    }
}

/// Names that differ only by the version in them ("PuTTY release 0.85 (64-bit)" and "PuTTY
/// release 0.86 (64-bit)").
fn names_match(a: &str, b: &str) -> bool {
    let strip = |s: &str| {
        s.chars()
            .filter(|c| !c.is_ascii_digit() && *c != '.')
            .collect::<String>()
    };
    let (sa, sb) = (strip(a), strip(b));
    !sa.trim().is_empty() && sa == sb
}

/// The entries that appeared during the install (`before`: the keys there before) other than the
/// app's own (`own`): their names.
pub fn extras(before: &[String], after: &[RawEntry], own: Option<&str>) -> Vec<String> {
    after
        .iter()
        .filter(|e| e.shown())
        .filter(|e| !before.iter().any(|k| k.eq_ignore_ascii_case(&e.key)))
        .filter(|e| own.is_none_or(|o| !o.eq_ignore_ascii_case(&e.key)))
        .filter_map(|e| text(e, "displayname").map(str::to_string))
        .collect()
}

/// Whether the app opened when started as the person.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Opened {
    Yes,
    No,
    /// Not checked (not asked, or PCTwin could not start it as the person).
    NotChecked,
}

/// What the helper said of the install.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Installed {
    Done {
        restart_needed: bool,
    },
    /// The helper found it already in place after a crash (it did not install it twice).
    AlreadyInPlace,
    Failed,
    /// The helper refused (not the maker, not the approved bytes, no administrator).
    Refused,
}

/// The one honest result for the app.
pub fn result(
    app: &FoundApp,
    installed: Installed,
    found: &Probe,
    extra: &[String],
    opened: Opened,
) -> ItemResult {
    match installed {
        Installed::Failed => return ItemResult::because(Reason::InstallFailed),
        Installed::Refused => return ItemResult::because(Reason::NotVerified),
        Installed::Done { .. } | Installed::AlreadyInPlace => {}
    }
    if !found.found {
        // The installer said done, but the laptop's own list does not show the app: never
        // claimed.
        return ItemResult::because(Reason::NotVerified);
    }
    if let (Some(had), Some(now)) = (app.version.as_deref(), found.version.as_deref())
        && matches!(compare(now, had), Order::Older | Order::Unknown)
    {
        return ItemResult::because(Reason::FinishByHand);
    }
    if !extra.is_empty() {
        return ItemResult::because(Reason::FinishByHand);
    }
    if matches!(
        installed,
        Installed::Done {
            restart_needed: true
        }
    ) {
        return ItemResult::because(Reason::Restart);
    }
    if opened == Opened::No {
        return ItemResult::because(Reason::InstallFailed);
    }
    ItemResult::done(ItemStatus::Reinstalled)
        .unwrap_or_else(|_| ItemResult::because(Reason::NotVerified))
}

/// One thing the person still has to do.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Todo {
    SignIn,
    LicenceKey,
    /// What the old laptop's own list held: who it is registered to and its product ID.
    Registration {
        owner: Option<String>,
        company: Option<String>,
        product_id: Option<String>,
    },
    Restart,
    /// Built for Intel Macs only, on an Apple-silicon Mac: it runs through Rosetta (which macOS
    /// offers to install the first time it is opened), and Apple is ending Rosetta.
    Rosetta,
    /// Other programs appeared with it; the person can remove them.
    Extras(Vec<String>),
    /// Whether the installer found on the old laptop came from the maker's own site, as its
    /// download mark said (only the yes or no; never the address).
    CameFrom {
        makers_site: bool,
    },
}

/// What the person still has to do for `app` (`sign_in` and `licence_key` from the routes'
/// catalogue rows).
pub fn checklist(
    app: &FoundApp,
    sign_in: bool,
    licence_key: bool,
    installed: Installed,
    extra: &[String],
) -> Vec<Todo> {
    let mut out = Vec::new();
    if sign_in {
        out.push(Todo::SignIn);
    }
    if licence_key {
        out.push(Todo::LicenceKey);
    }
    for n in &app.notes {
        let AppNote::Registration {
            owner,
            company,
            product_id,
        } = n
        else {
            if let AppNote::DownloadMark { makers_site } = n {
                out.push(Todo::CameFrom {
                    makers_site: *makers_site,
                });
            }
            continue;
        };
        out.push(Todo::Registration {
            owner: owner.clone(),
            company: company.clone(),
            product_id: product_id.clone(),
        });
    }
    if matches!(
        installed,
        Installed::Done {
            restart_needed: true
        }
    ) {
        out.push(Todo::Restart);
    }
    if !extra.is_empty() {
        out.push(Todo::Extras(extra.to_vec()));
    }
    out
}

/// What 2.1 observed of one install, for 2.5. It carries no permission and no secret: 2.5 asks
/// its own role for its own grant, and checks the laptop itself before relying on any of this.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InstallReceipt {
    /// The move (job) and the plan it was approved in.
    pub job: [u8; 16],
    pub plan_generation: u64,
    pub plan_digest: [u8; 32],
    /// The laptop it was observed on.
    pub device: [u8; 16],
    /// The app as the new laptop's own list names it.
    pub product_code: Option<String>,
    pub name: String,
    pub version: Option<String>,
    pub view: Option<RegistryView>,
    pub restart_needed: bool,
}

/// Why 2.5 does not rely on a receipt.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NotTrue {
    OtherMove,
    OtherLaptop,
    /// The laptop's own list no longer shows the app (or another version of it).
    NotThere,
    OtherVersion,
    /// The laptop shows it in another list than the one the install was observed in (a
    /// per-person entry where a machine install was observed).
    OtherView,
}

impl InstallReceipt {
    /// 2.5's own check: this move, this laptop, and the laptop itself still shows the app at that
    /// version, in the same list (`now`: 2.5's own fresh probe, made for the receipt's scope). A receipt from another move, laptop or version, or
    /// one forged, fails here whatever it says.
    pub fn still_true(&self, job: [u8; 16], device: [u8; 16], now: &Probe) -> Result<(), NotTrue> {
        if self.job != job {
            return Err(NotTrue::OtherMove);
        }
        if self.device != device {
            return Err(NotTrue::OtherLaptop);
        }
        if !now.found {
            return Err(NotTrue::NotThere);
        }
        if now.version != self.version {
            return Err(NotTrue::OtherVersion);
        }
        if self.view.is_none() || now.view != self.view {
            return Err(NotTrue::OtherView);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::found::windows::RegValue;
    use pctwin_record::{AppArch, AppMaker, AppSystem, Owner};
    use std::collections::BTreeMap;

    fn entry(key: &str, name: &str, version: &str, msi: bool) -> RawEntry {
        let mut values = BTreeMap::new();
        values.insert("displayname".into(), RegValue::Text(name.into()));
        values.insert("displayversion".into(), RegValue::Text(version.into()));
        values.insert("publisher".into(), RegValue::Text("Simon Tatham".into()));
        if msi {
            values.insert("windowsinstaller".into(), RegValue::Number(1));
        }
        RawEntry {
            view: RegistryView::Machine,
            key: key.into(),
            values,
        }
    }

    const CODE: &str = "{3B0346D3-155D-43E5-9561-B9BE51CF2EB6}";

    fn putty(code: Option<&str>, version: &str) -> FoundApp {
        FoundApp {
            name: "PuTTY release 0.85 (64-bit)".into(),
            version: Some(version.into()),
            publisher: Some("Simon Tatham".into()),
            owner: Owner::Shared,
            system: AppSystem::Windows,
            found: FoundWhere::WindowsEntry {
                view: RegistryView::Machine,
                key: "k".into(),
                product_code: code.map(Into::into),
            },
            arch: AppArch::UNKNOWN,
            maker: AppMaker::Unsigned,
            web: None,
            update_feed: None,
            installers: vec![],
            notes: vec![],
        }
    }

    #[test]
    fn the_app_is_found_by_its_identity_never_by_a_similar_name() {
        let after = vec![
            entry(CODE, "PuTTY release 0.85 (64-bit)", "0.85.0.0", true),
            entry("Fake", "PuTTY release 0.85 (64-bit) Tools", "9.9", false),
        ];
        let p = probe(
            &putty(Some(CODE), "0.85.0.0"),
            InstallScope::Machine,
            &after,
        );
        assert!(p.found);
        assert_eq!(p.key.as_deref(), Some(CODE));
        assert_eq!(p.version.as_deref(), Some("0.85.0.0"));
        // A newer release with a new product code is still found, by exactly its name pattern
        // and publisher.
        let newer = vec![entry(
            "{00000000-0000-0000-0000-000000000001}",
            "PuTTY release 0.86 (64-bit)",
            "0.86.0.0",
            true,
        )];
        assert!(
            probe(
                &putty(Some(CODE), "0.85.0.0"),
                InstallScope::Machine,
                &newer
            )
            .found
        );
        // Without a product code, only the exact name and publisher.
        assert!(probe(&putty(None, "0.85"), InstallScope::Machine, &after).found);
        let other_publisher = {
            let mut e = entry("X", "PuTTY release 0.85 (64-bit)", "0.85", false);
            e.values
                .insert("publisher".into(), RegValue::Text("Someone".into()));
            vec![e]
        };
        assert!(
            !probe(
                &putty(None, "0.85"),
                InstallScope::Machine,
                &other_publisher
            )
            .found
        );
        assert!(!probe(&putty(None, "0.85"), InstallScope::Machine, &[]).found);
    }

    #[test]
    fn a_hidden_entry_or_another_product_is_never_taken_for_the_app() {
        let app = putty(Some(CODE), "0.85.0.0");
        let mut hidden = entry(CODE, "PuTTY release 0.85 (64-bit)", "0.85.0.0", true);
        hidden
            .values
            .insert("systemcomponent".into(), RegValue::Number(1));
        assert!(!probe(&app, InstallScope::Machine, &[hidden]).found);
        let other = entry(
            "{11111111-1111-1111-1111-111111111111}",
            "Other tool",
            "1.0",
            true,
        );
        let ours = entry(CODE, "PuTTY release 0.85 (64-bit)", "0.85.0.0", true);
        assert_eq!(
            probe(&app, InstallScope::Machine, &[other, ours])
                .key
                .as_deref(),
            Some(CODE)
        );
    }

    #[test]
    fn reinstalled_only_when_every_check_holds() {
        let app = putty(Some(CODE), "0.85.0.0");
        let here = probe(
            &app,
            InstallScope::Machine,
            &[entry(CODE, "PuTTY release 0.85 (64-bit)", "0.85.0.0", true)],
        );
        let done = Installed::Done {
            restart_needed: false,
        };
        assert_eq!(
            result(&app, done, &here, &[], Opened::Yes).status(),
            ItemStatus::Reinstalled
        );
        assert_eq!(
            result(&app, done, &here, &[], Opened::NotChecked).status(),
            ItemStatus::Reinstalled
        );
        assert_eq!(
            result(
                &app,
                Installed::AlreadyInPlace,
                &here,
                &[],
                Opened::NotChecked
            )
            .status(),
            ItemStatus::Reinstalled
        );
        // Every way it is not.
        let not = |r: ItemResult| r.status() != ItemStatus::Reinstalled;
        assert!(not(result(
            &app,
            Installed::Failed,
            &here,
            &[],
            Opened::Yes
        )));
        assert!(not(result(
            &app,
            Installed::Refused,
            &here,
            &[],
            Opened::Yes
        )));
        assert!(not(result(
            &app,
            done,
            &probe(&app, InstallScope::Machine, &[]),
            &[],
            Opened::Yes
        )));
        assert!(not(result(
            &app,
            done,
            &here,
            &["Toolbar".into()],
            Opened::Yes
        )));
        assert!(not(result(
            &app,
            Installed::Done {
                restart_needed: true
            },
            &here,
            &[],
            Opened::Yes
        )));
        assert!(not(result(&app, done, &here, &[], Opened::No)));
        let older = putty(Some(CODE), "0.90");
        assert!(not(result(&older, done, &here, &[], Opened::Yes)));
    }

    #[test]
    fn programs_that_slipped_in_are_named() {
        let before = vec!["Old".to_string()];
        let after = vec![
            entry("Old", "Old app", "1", false),
            entry(CODE, "PuTTY", "0.85", true),
            entry("Offer", "Shopping Toolbar", "1", false),
        ];
        assert_eq!(
            extras(&before, &after, Some(CODE)),
            vec!["Shopping Toolbar".to_string()]
        );
        assert!(extras(&before, &after[..2], Some(CODE)).is_empty());
    }

    #[test]
    fn the_checklist_says_what_is_left() {
        let mut app = putty(None, "1");
        app.notes = vec![AppNote::Registration {
            owner: Some("Ada".into()),
            company: None,
            product_id: Some("123".into()),
        }];
        let list = checklist(
            &app,
            true,
            true,
            Installed::Done {
                restart_needed: true,
            },
            &["X".into()],
        );
        assert_eq!(list.len(), 5);
        assert!(list.contains(&Todo::Restart));
        assert!(
            checklist(
                &putty(None, "1"),
                false,
                false,
                Installed::AlreadyInPlace,
                &[]
            )
            .is_empty()
        );
    }

    /// T21-050: a receipt from another move, another laptop or another version, or one for an app
    /// the laptop does not show, is refused by 2.5's own check.
    #[test]
    fn a_receipt_is_never_taken_on_its_word() {
        let r = InstallReceipt {
            job: [1; 16],
            plan_generation: 1,
            plan_digest: [2; 32],
            device: [3; 16],
            product_code: Some(CODE.into()),
            name: "PuTTY".into(),
            version: Some("0.85.0.0".into()),
            view: Some(RegistryView::Machine),
            restart_needed: false,
        };
        let here = Probe {
            found: true,
            version: Some("0.85.0.0".into()),
            view: Some(RegistryView::Machine),
            key: Some(CODE.into()),
        };
        assert_eq!(r.still_true([1; 16], [3; 16], &here), Ok(()));
        assert_eq!(
            r.still_true([9; 16], [3; 16], &here),
            Err(NotTrue::OtherMove)
        );
        assert_eq!(
            r.still_true([1; 16], [9; 16], &here),
            Err(NotTrue::OtherLaptop)
        );
        let gone = Probe {
            found: false,
            ..here.clone()
        };
        assert_eq!(
            r.still_true([1; 16], [3; 16], &gone),
            Err(NotTrue::NotThere)
        );
        let other = Probe {
            version: Some("0.70".into()),
            ..here.clone()
        };
        assert_eq!(
            r.still_true([1; 16], [3; 16], &other),
            Err(NotTrue::OtherVersion)
        );
        let person = Probe {
            view: Some(RegistryView::Person),
            ..here.clone()
        };
        assert_eq!(
            r.still_true([1; 16], [3; 16], &person),
            Err(NotTrue::OtherView)
        );
        let unrecorded = InstallReceipt { view: None, ..r };
        assert_eq!(
            unrecorded.still_true([1; 16], [3; 16], &here),
            Err(NotTrue::OtherView)
        );
    }

    /// Fifth review, finding 5: a per-person entry (which the person, or any program running as
    /// them, can write) never proves a machine install; a machine entry never proves a
    /// per-person one.
    #[test]
    fn a_machine_install_is_only_found_in_the_machines_own_lists() {
        let app = putty(None, "0.85");
        let mut person = entry("k", "PuTTY release 0.85 (64-bit)", "0.85", false);
        person.view = RegistryView::Person;
        let p = probe(&app, InstallScope::Machine, std::slice::from_ref(&person));
        assert!(!p.found);
        let done = Installed::Done {
            restart_needed: false,
        };
        assert_ne!(
            result(&app, done, &p, &[], Opened::NotChecked).status(),
            ItemStatus::Reinstalled
        );
        assert!(probe(&app, InstallScope::User, std::slice::from_ref(&person)).found);
        let mut m32 = entry("k", "PuTTY release 0.85 (64-bit)", "0.85", false);
        m32.view = RegistryView::Machine32;
        let p = probe(&app, InstallScope::Machine, &[person.clone(), m32.clone()]);
        assert_eq!(p.view, Some(RegistryView::Machine32));
        assert!(!probe(&app, InstallScope::User, &[m32]).found);
    }
}
