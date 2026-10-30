//! One component of an app's data, and how it ended (work package B25-01; plan sections 2, 5
//! and 13).
//!
//! An app's data is never one thing. Its local messages may be a portable file, its mail on a
//! server, its sign-in token bound to this laptop. Each part is a [`Component`] with its own
//! custody class ([`Custody`]: P, S, E or D), the evidence that decided it ([`Evidence`], never a
//! percentage), the person it belongs to, and the files that only work together. Evidence that
//! cannot be established leaves it [`Custody::Unresolved`]: unknown stays unknown, and is never
//! read as "portable".
//!
//! How a component ended is kept in six separate dimensions ([`Outcome`]: preservation,
//! transport, access, import, readiness, reversal), so "the bytes arrived" is never mistaken for
//! "the app can open them". [`Outcome::result`] maps them onto the six result words; a component
//! is `copied` or `converted` only when every dimension that matters says so.

use pctwin_record::Owner;
use pctwin_types::{ItemResult, ItemStatus, Reason};
use serde::{Deserialize, Serialize};

/// Who holds the key to a component (the plan's four-way map).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Custody {
    /// P: the data and every key it needs can be captured and copied.
    #[serde(rename = "P")]
    Portable,
    /// S: the real copy is held by a service; the person signs in again.
    #[serde(rename = "S")]
    ServiceHeld,
    /// E: the app's own export and import is the route.
    #[serde(rename = "E")]
    NeedsExport,
    /// D: bound to this laptop or a device; it is never copied, the person re-enrols or relinks.
    #[serde(rename = "D")]
    MachineBound,
    /// The evidence could not decide. Never treated as portable or complete.
    #[serde(rename = "unresolved")]
    Unresolved,
}

/// One fact that decided a component's class or handling, read from the files themselves (their
/// content and how they relate), never from an app's name.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", tag = "fact")]
pub enum Evidence {
    /// An SQLite database (by its header).
    SqliteDatabase,
    /// An SQLite write-ahead log or rollback journal that belongs to the database beside it.
    SqliteCompanion,
    /// A memory-mapped key-value store (LMDB) or a log-structured one (LevelDB): copied only
    /// whole, with the app closed.
    LiveStore,
    /// Plain structured text or data (JSON, XML, INI, property list).
    StructuredData,
    /// A mail store by its format (mbox, maildir, an Outlook data file).
    MailStore,
    /// A mail archive the mail app opens as a file of its own (an Outlook data file): it moves,
    /// and the person opens it in the mail app on the new laptop.
    MailArchive,
    /// A mail program's copy of a server mailbox (an Outlook offline cache): the mail is on the
    /// server, but whether every item reached it cannot be read from the file.
    ServerMailCache,
    /// An encrypted password database (KeePass format).
    PasswordDatabase,
    /// A key file in a password manager's key-file format.
    PasswordKeyFile,
    /// A private key protected by a passphrase.
    EncryptedPrivateKey,
    /// A private key with no passphrase: anyone who gets the file has the key.
    UnprotectedPrivateKey,
    /// A public key or certificate: not secret.
    PublicKey,
    /// A stub for a key held in a hardware security key or smartcard: the private key is not in
    /// the file and never can be.
    HardwareKeyStub,
    /// A key store (an NSS key database) that decrypts other files beside it.
    KeyDatabase,
    /// Values encrypted with a key kept in a key database beside them: they only work together.
    EncryptedWithKeyDatabase,
    /// Data protected by this laptop's account or chip (a Windows DPAPI blob, an app-bound key):
    /// it cannot be opened anywhere else.
    MachineBoundProtection,
    /// A store of cookies or session tokens (by its tables' shape): signing in again is the route.
    SessionStore,
    /// A store of saved passwords (by its tables' shape): the app's own export is the route.
    CredentialStore,
    /// Values that look like credentials (tokens, keys) inside a data file.
    CredentialValues,
    /// A cache the app rebuilds (by an OS cache place, a cache marker or a cache format).
    Cache,
    /// A file named by its own hash; `verified` once its bytes were hashed and matched.
    ContentAddressed { verified: bool },
    /// A file named by a hash its bytes do not have: damaged or replaced.
    HashMismatch,
    /// Model weights (GGUF, safetensors) or a manifest that lists them.
    ModelData,
    /// Saved web addresses, by the file's shape: bookmarks, or a history of visited pages.
    BrowsingData,
    /// It uses files kept in other folders, which were not chosen (a catalog's originals, a
    /// library's attachments): the person is asked whether to add them.
    LinksOutside { folders: Vec<String> },
    /// A file another file of this component names is not there.
    MissingPart { what: String },
    /// A program, library or script: never moved as data or activated; 2.1 reacquires code.
    ExecutableCode,
    /// A settings file whose values start programs or reach the network: moved as data, held
    /// for review before the app on the new laptop uses it.
    EnablesCommands,
    /// The app keeps an account with a service (sessions were found): what the service holds
    /// comes back by signing in.
    ServiceAccount,
    /// The bytes were read and no protected or special shape was found. Detection is advisory:
    /// this is not proof the file holds no secret.
    NoSpecialShape,
    /// The file could not be read.
    Unreadable { why: String },
    /// A signed catalogue row adds knowledge (a native route, an exception, a friendly name).
    Catalogue { row: String },
}

impl Evidence {
    /// The fact in plain words for the person.
    pub fn words(&self) -> String {
        match self {
            Evidence::SqliteDatabase => "a database the app keeps open while it runs".into(),
            Evidence::SqliteCompanion => {
                "recent changes to the database beside it, which must travel with it".into()
            }
            Evidence::LiveStore => "a store the app writes while it runs".into(),
            Evidence::StructuredData => "a settings or data file".into(),
            Evidence::MailStore => "saved mail".into(),
            Evidence::MailArchive => "a mail archive your mail app opens".into(),
            Evidence::ServerMailCache => {
                "a copy of mail kept on a server; anything never sent to the server may exist only here"
                    .into()
            }
            Evidence::PasswordDatabase => "an encrypted password database".into(),
            Evidence::PasswordKeyFile => "a key file that unlocks a password database".into(),
            Evidence::EncryptedPrivateKey => "a private key protected by a passphrase".into(),
            Evidence::UnprotectedPrivateKey => {
                "a private key with no passphrase: anyone with the file has the key".into()
            }
            Evidence::PublicKey => "a public key or certificate".into(),
            Evidence::HardwareKeyStub => {
                "a pointer to a key held on a security key or smartcard; the key itself never leaves the device"
                    .into()
            }
            Evidence::KeyDatabase => "the key store that unlocks saved passwords beside it".into(),
            Evidence::EncryptedWithKeyDatabase => {
                "saved passwords that only open with the key store beside them".into()
            }
            Evidence::MachineBoundProtection => {
                "protected by this laptop itself; it cannot be opened on another computer".into()
            }
            Evidence::SessionStore => "sign-ins to websites or services".into(),
            Evidence::CredentialStore => "saved passwords".into(),
            Evidence::CredentialValues => "passwords, keys or sign-in tokens inside a file".into(),
            Evidence::Cache => "a cache the app rebuilds by itself".into(),
            Evidence::ContentAddressed { verified: true } => {
                "a file named by its own fingerprint, checked".into()
            }
            Evidence::ContentAddressed { verified: false } => {
                "a file named by its own fingerprint, to be checked".into()
            }
            Evidence::HashMismatch => {
                "a file whose content does not match the fingerprint in its name".into()
            }
            Evidence::ModelData => "AI model files".into(),
            Evidence::BrowsingData => "bookmarks or the history of pages visited".into(),
            Evidence::MissingPart { what } => format!("a part it needs is missing: {what}"),
            Evidence::LinksOutside { folders } => format!(
                "it uses files kept in {} other folder(s); add them to the move?",
                folders.len()
            ),
            Evidence::ExecutableCode => {
                "a program or add-on; programs are installed again, never copied".into()
            }
            Evidence::EnablesCommands => {
                "settings that start programs or reach the internet; check them before use".into()
            }
            Evidence::ServiceAccount => "an account with an online service".into(),
            Evidence::NoSpecialShape => "a data file".into(),
            Evidence::Unreadable { why } => format!("it could not be read ({why})"),
            Evidence::Catalogue { row } => format!("known: {row}"),
        }
    }
}

/// A component's identity: a fingerprint of whose it is, where it is, and the files in it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct ComponentId(pub [u8; 32]);

impl ComponentId {
    /// The identity of the files `members` (paths inside `place`, `/` between folders) of
    /// `owner`. The same files in any order give the same identity.
    pub fn of(owner: &Owner, place: &str, members: &[String]) -> Self {
        let mut sorted: Vec<&String> = members.iter().collect();
        sorted.sort();
        let mut h = blake3::Hasher::new();
        h.update(b"pctwin-special component 1\0");
        let who = serde_json::to_vec(owner).unwrap_or_default();
        for part in [who.as_slice(), place.as_bytes()] {
            h.update(&(part.len() as u64).to_le_bytes());
            h.update(part);
        }
        for m in sorted {
            h.update(&(m.len() as u64).to_le_bytes());
            h.update(m.as_bytes());
        }
        Self(*h.finalize().as_bytes())
    }
}

/// One component: files that only work together, whose they are, and who holds their key.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Component {
    pub id: ComponentId,
    /// The person it belongs to, by the system's own account ID.
    pub owner: Owner,
    /// The app data folder it was found in (`root` kind and path inside it).
    pub place: String,
    /// Its files, inside the place. A group (a database and its log, a store and the key that
    /// opens it) moves whole or not at all.
    pub members: Vec<String>,
    pub custody: Custody,
    /// Secret material (keys, password stores, credentials): never moved without the person's
    /// explicit choice, never in a log, report, catalogue or AI input.
    pub secret: bool,
    /// Why, as facts. At least one.
    pub evidence: Vec<Evidence>,
    /// Other components it needs to be usable (the originals a catalog points at, the key store
    /// that opens it).
    pub needs: Vec<ComponentId>,
    /// A labelled copy is kept even though the service holds the data, because items that never
    /// reached the service cannot be ruled out (a server mailbox's offline copy).
    #[serde(default)]
    pub preserve: bool,
}

impl Component {
    /// A component of `owner` at `place` made of `members`.
    pub fn new(
        owner: Owner,
        place: &str,
        members: Vec<String>,
        custody: Custody,
        secret: bool,
        evidence: Vec<Evidence>,
    ) -> Self {
        Self {
            id: ComponentId::of(&owner, place, &members),
            owner,
            place: place.to_string(),
            members,
            custody,
            secret,
            evidence,
            needs: Vec::new(),
            preserve: false,
        }
    }
}

/// Whether the component's data was captured on the old laptop.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Preservation {
    /// The person left it out.
    NotSelected,
    /// Not captured yet.
    Pending,
    /// Captured consistently (an online backup, or every file steady with the app closed).
    Captured,
    /// Only part of it could be read.
    Partial,
    /// Kept byte for byte with a label, but its consistency could not be established (the app
    /// kept writing): never "copied".
    PreservedOnly,
    /// Could not be read at all.
    Unavailable,
}

/// Whether its bytes reached the new laptop.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Transport {
    NotYet,
    /// Every byte read back on the new laptop and matched.
    Verified,
    Interrupted,
    /// Nothing moves (a sign-in, a re-enrolment).
    NotApplicable,
}

/// Whether the person can open it on the new laptop.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Access {
    /// No key is needed.
    NotNeeded,
    /// Every key it needs travelled with it or is on the new laptop.
    KeyAvailable,
    /// It opens with something the person knows (a master password, a passphrase).
    AwaitingUnlock,
    /// A key part it needs (a key file, a key store) is not here.
    MissingKeyComponent,
    /// It is bound to the old device: the person enrols or links again.
    NeedsReenrolment,
    /// The person signs in to the service.
    SignInNeeded,
    /// Could not be told.
    Unknown,
}

/// Whether it is in place for the app on the new laptop.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Import {
    NotYet,
    /// Its files are in place (a new profile, or beside the person's own).
    Placed,
    /// The app's own import took it in.
    Imported,
    /// Merged by the app's own importer.
    Merged,
    /// Kept on the new laptop, not handed to the app (a format or version the app there cannot
    /// read, or a route that needs the person).
    PreservedOnly,
    /// The app on the new laptop cannot read it.
    Incompatible,
    Failed,
    /// Nothing to import (a sign-in).
    NotApplicable,
}

/// What was checked after it landed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Readiness {
    /// A machine check covered it (the database opens and passes its own check, every file
    /// matches).
    MachineChecked,
    /// The person checked `checked` items (of `of`, when known); that sample only.
    UserConfirmedSample {
        checked: u32,
        of: Option<u32>,
    },
    /// The person has a step to take first.
    NeedsAction,
    Untested,
}

/// How far undo can go.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Reversal {
    /// Undo removes what PCTwin placed, keeping anything changed since.
    LocallyReversible,
    /// A compensating step exists, not an exact undo.
    Compensating,
    /// Done with a service: only the service can undo it.
    RemoteEffect,
    Unsupported,
    NotApplicable,
}

/// How one component ended, in its six separate dimensions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Outcome {
    pub preservation: Preservation,
    pub transport: Transport,
    pub access: Access,
    pub import: Import,
    pub readiness: Readiness,
    pub reversal: Reversal,
}

impl Outcome {
    /// Nothing done yet.
    pub fn pending() -> Self {
        Self {
            preservation: Preservation::Pending,
            transport: Transport::NotYet,
            access: Access::Unknown,
            import: Import::NotYet,
            readiness: Readiness::Untested,
            reversal: Reversal::NotApplicable,
        }
    }

    /// The one result word for a component of class `custody` that ended like this. Success
    /// (`copied`, `converted`) needs every dimension to say so; anything unfinished, unknown or
    /// unverified is `needs-action` or `failed` with its reason. Never success by default.
    pub fn result(&self, custody: Custody) -> ItemResult {
        use Reason::*;
        let because = ItemResult::because;
        if self.preservation == Preservation::NotSelected {
            return ItemResult::skipped_by_user();
        }
        match custody {
            // Nothing is copied for these: the person's step is the result.
            Custody::ServiceHeld => return because(SignIn),
            Custody::MachineBound | Custody::Unresolved => return because(FinishByHand),
            Custody::Portable | Custody::NeedsExport => {}
        }
        match self.preservation {
            Preservation::Unavailable => return because(Unreadable),
            Preservation::Partial => return because(PartsMissing),
            Preservation::PreservedOnly | Preservation::Pending => return because(FinishByHand),
            Preservation::Captured | Preservation::NotSelected => {}
        }
        match self.transport {
            Transport::Verified => {}
            Transport::Interrupted => return because(NotVerified),
            Transport::NotYet | Transport::NotApplicable => return because(FinishByHand),
        }
        match self.access {
            Access::NotNeeded | Access::KeyAvailable | Access::AwaitingUnlock => {}
            Access::SignInNeeded => return because(SignIn),
            Access::MissingKeyComponent | Access::NeedsReenrolment | Access::Unknown => {
                return because(FinishByHand);
            }
        }
        if self.readiness == Readiness::NeedsAction {
            return because(FinishByHand);
        }
        let done = |s| ItemResult::done(s).unwrap_or_else(|_| because(FinishByHand));
        match (self.import, custody) {
            (Import::Placed, Custody::Portable) => done(ItemStatus::Copied),
            (Import::Imported | Import::Merged, _) => done(ItemStatus::Converted),
            // An export route whose files were only placed was not imported by the app.
            (Import::Placed, _) => because(FinishByHand),
            (Import::NotYet | Import::PreservedOnly | Import::Incompatible, _) => {
                because(FinishByHand)
            }
            (Import::Failed, _) => because(FinishByHand),
            (Import::NotApplicable, _) => because(FinishByHand),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn me() -> Owner {
        Owner::Person {
            account_id: "S-1-5-21-1-2-3-1001".into(),
        }
    }

    fn landed() -> Outcome {
        Outcome {
            preservation: Preservation::Captured,
            transport: Transport::Verified,
            access: Access::NotNeeded,
            import: Import::Placed,
            readiness: Readiness::MachineChecked,
            reversal: Reversal::LocallyReversible,
        }
    }

    fn status(o: &Outcome, c: Custody) -> (ItemStatus, Option<Reason>) {
        let r = o.result(c);
        (r.status(), r.reason())
    }

    #[test]
    fn a_component_is_copied_only_when_every_dimension_says_so() {
        assert_eq!(
            status(&landed(), Custody::Portable),
            (ItemStatus::Copied, None)
        );
        // A person's own password still opens it: copied, the password is theirs.
        let mut o = landed();
        o.access = Access::AwaitingUnlock;
        assert_eq!(status(&o, Custody::Portable).0, ItemStatus::Copied);
        // Untested after a verified copy is still a copy (the bytes were checked).
        o.readiness = Readiness::Untested;
        assert_eq!(status(&o, Custody::Portable).0, ItemStatus::Copied);
    }

    type Case = (Box<dyn Fn(&mut Outcome)>, ItemStatus, Option<Reason>);

    #[test]
    fn each_unfinished_dimension_gives_its_own_honest_word() {
        let cases: Vec<Case> = vec![
            (
                Box::new(|o| o.preservation = Preservation::NotSelected),
                ItemStatus::SkippedByUser,
                None,
            ),
            (
                Box::new(|o| o.preservation = Preservation::Unavailable),
                ItemStatus::Failed,
                Some(Reason::Unreadable),
            ),
            (
                Box::new(|o| o.preservation = Preservation::Partial),
                ItemStatus::NeedsAction,
                Some(Reason::PartsMissing),
            ),
            (
                Box::new(|o| o.preservation = Preservation::PreservedOnly),
                ItemStatus::NeedsAction,
                Some(Reason::FinishByHand),
            ),
            (
                Box::new(|o| o.preservation = Preservation::Pending),
                ItemStatus::NeedsAction,
                Some(Reason::FinishByHand),
            ),
            (
                Box::new(|o| o.transport = Transport::Interrupted),
                ItemStatus::Failed,
                Some(Reason::NotVerified),
            ),
            (
                Box::new(|o| o.transport = Transport::NotYet),
                ItemStatus::NeedsAction,
                Some(Reason::FinishByHand),
            ),
            (
                Box::new(|o| o.transport = Transport::NotApplicable),
                ItemStatus::NeedsAction,
                Some(Reason::FinishByHand),
            ),
            (
                Box::new(|o| o.access = Access::MissingKeyComponent),
                ItemStatus::NeedsAction,
                Some(Reason::FinishByHand),
            ),
            (
                Box::new(|o| o.access = Access::NeedsReenrolment),
                ItemStatus::NeedsAction,
                Some(Reason::FinishByHand),
            ),
            (
                Box::new(|o| o.access = Access::Unknown),
                ItemStatus::NeedsAction,
                Some(Reason::FinishByHand),
            ),
            (
                Box::new(|o| o.access = Access::SignInNeeded),
                ItemStatus::NeedsAction,
                Some(Reason::SignIn),
            ),
            (
                Box::new(|o| o.readiness = Readiness::NeedsAction),
                ItemStatus::NeedsAction,
                Some(Reason::FinishByHand),
            ),
            (
                Box::new(|o| o.import = Import::NotYet),
                ItemStatus::NeedsAction,
                Some(Reason::FinishByHand),
            ),
            (
                Box::new(|o| o.import = Import::PreservedOnly),
                ItemStatus::NeedsAction,
                Some(Reason::FinishByHand),
            ),
            (
                Box::new(|o| o.import = Import::Incompatible),
                ItemStatus::NeedsAction,
                Some(Reason::FinishByHand),
            ),
            (
                Box::new(|o| o.import = Import::Failed),
                ItemStatus::NeedsAction,
                Some(Reason::FinishByHand),
            ),
            (
                Box::new(|o| o.import = Import::NotApplicable),
                ItemStatus::NeedsAction,
                Some(Reason::FinishByHand),
            ),
            (
                Box::new(|o| o.import = Import::Imported),
                ItemStatus::Converted,
                None,
            ),
            (
                Box::new(|o| o.import = Import::Merged),
                ItemStatus::Converted,
                None,
            ),
        ];
        for (i, (change, want, why)) in cases.into_iter().enumerate() {
            let mut o = landed();
            change(&mut o);
            assert_eq!(status(&o, Custody::Portable), (want, why), "case {i}");
        }
    }

    #[test]
    fn service_bound_and_unknown_components_are_never_copied() {
        // Even with every dimension green, the class decides.
        let o = landed();
        assert_eq!(
            status(&o, Custody::ServiceHeld),
            (ItemStatus::NeedsAction, Some(Reason::SignIn))
        );
        assert_eq!(
            status(&o, Custody::MachineBound),
            (ItemStatus::NeedsAction, Some(Reason::FinishByHand))
        );
        // T25-002: unknown stays unknown, not portable or complete.
        assert_eq!(
            status(&o, Custody::Unresolved),
            (ItemStatus::NeedsAction, Some(Reason::FinishByHand))
        );
        // Left out by the person: skipped, whatever its class.
        let mut o = landed();
        o.preservation = Preservation::NotSelected;
        for c in [
            Custody::Portable,
            Custody::ServiceHeld,
            Custody::NeedsExport,
            Custody::MachineBound,
            Custody::Unresolved,
        ] {
            assert_eq!(status(&o, c).0, ItemStatus::SkippedByUser);
        }
    }

    #[test]
    fn an_export_route_is_converted_only_through_the_apps_own_import() {
        let mut o = landed();
        assert_eq!(
            status(&o, Custody::NeedsExport),
            (ItemStatus::NeedsAction, Some(Reason::FinishByHand))
        );
        o.import = Import::Imported;
        assert_eq!(
            status(&o, Custody::NeedsExport),
            (ItemStatus::Converted, None)
        );
    }

    #[test]
    fn the_same_files_give_the_same_identity_and_others_do_not() {
        let a = ComponentId::of(&me(), "Config/App", &["db".into(), "db-wal".into()]);
        let b = ComponentId::of(&me(), "Config/App", &["db-wal".into(), "db".into()]);
        assert_eq!(a, b);
        let other_person = Owner::Person {
            account_id: "S-1-5-21-1-2-3-1002".into(),
        };
        assert_ne!(
            a,
            ComponentId::of(&other_person, "Config/App", &["db".into(), "db-wal".into()])
        );
        assert_ne!(a, ComponentId::of(&me(), "Config/App", &["db".into()]));
        assert_ne!(
            a,
            ComponentId::of(&me(), "Config/Ap", &["pdb".into(), "db-wal".into()])
        );
        // Lengths are bound, so moving a character between parts changes the identity.
        assert_ne!(
            ComponentId::of(&me(), "ab", &["c".into()]),
            ComponentId::of(&me(), "a", &["bc".into()])
        );
    }

    #[test]
    fn every_fact_has_plain_words_and_classes_are_written_as_the_plan_names_them() {
        let all = [
            Evidence::SqliteDatabase,
            Evidence::SqliteCompanion,
            Evidence::LiveStore,
            Evidence::StructuredData,
            Evidence::MailStore,
            Evidence::ServerMailCache,
            Evidence::PasswordDatabase,
            Evidence::PasswordKeyFile,
            Evidence::EncryptedPrivateKey,
            Evidence::UnprotectedPrivateKey,
            Evidence::PublicKey,
            Evidence::HardwareKeyStub,
            Evidence::KeyDatabase,
            Evidence::EncryptedWithKeyDatabase,
            Evidence::MachineBoundProtection,
            Evidence::SessionStore,
            Evidence::CredentialStore,
            Evidence::CredentialValues,
            Evidence::Cache,
            Evidence::ContentAddressed { verified: true },
            Evidence::ContentAddressed { verified: false },
            Evidence::HashMismatch,
            Evidence::ModelData,
            Evidence::BrowsingData,
            Evidence::MissingPart { what: "x".into() },
            Evidence::LinksOutside {
                folders: vec!["f".into()],
            },
            Evidence::ExecutableCode,
            Evidence::EnablesCommands,
            Evidence::ServiceAccount,
            Evidence::NoSpecialShape,
            Evidence::Unreadable { why: "y".into() },
            Evidence::Catalogue { row: "z".into() },
        ];
        let words: std::collections::BTreeSet<String> = all.iter().map(Evidence::words).collect();
        assert_eq!(
            words.len(),
            all.len(),
            "each fact says something of its own"
        );
        for (c, s) in [
            (Custody::Portable, "\"P\""),
            (Custody::ServiceHeld, "\"S\""),
            (Custody::NeedsExport, "\"E\""),
            (Custody::MachineBound, "\"D\""),
            (Custody::Unresolved, "\"unresolved\""),
        ] {
            assert_eq!(serde_json::to_string(&c).unwrap(), s);
        }
        let c = Component::new(
            me(),
            "Config/App",
            vec!["a".into()],
            Custody::Portable,
            false,
            vec![Evidence::NoSpecialShape],
        );
        assert_eq!(c.id, ComponentId::of(&me(), "Config/App", &["a".into()]));
        assert_eq!(
            Outcome::pending().result(Custody::Portable).status(),
            ItemStatus::NeedsAction
        );
    }
}
