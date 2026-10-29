//! Step 4: installing a checked download through the administrator helper.
//!
//! The app never installs a package for everyone itself. For each download it:
//!
//! 1. decides how it can be installed from the package's *typed* installer kind
//!    ([`kind_of`]): a Windows Installer package (`msi`, `wix`), or an installer program whose
//!    maker documents quiet, no-restart switches (Inno Setup, a WiX Burn bundle). Switches never
//!    come from text: a manifest's `InstallerSwitches` are never read. A Nullsoft installer has
//!    no switch that stops it restarting the laptop, a plain `exe` has no documented switches,
//!    and portable, archive and packaged (MSIX) installs are not built yet: each ends
//!    needs-action with its reason;
//! 2. puts one [`InstallVerifiedPackage`] per package in the plan the person approves
//!    ([`operation`]), naming the package's exact bytes and **the old laptop's maker**;
//! 3. sends the bytes to the helper's own administrators-only folder ([`stage`]), where the
//!    helper checks the maker itself and installs, journaled, never twice
//!    (`pctwin_helper::broker`).

use std::io::Read;

use pctwin_helper::broker::actions::{
    Action, ExeKind, Hex16, Hex32, Id, InstallScope, InstallVerifiedPackage, PackageKind,
    PlannedOperation, ProductCode,
};
use pctwin_record::{AppMaker, FoundApp};

/// Why a download cannot be installed by PCTwin.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NotInstallable {
    /// A Nullsoft (NSIS) installer: nothing stops it restarting the laptop (T21-040).
    MayRestart,
    /// A program whose quiet switches its maker does not document.
    UnknownSwitches,
    /// The package installs for one person only; PCTwin installs for everyone through the
    /// helper (the person's own installs are not built yet).
    PersonOnly,
    /// A portable app or an archive: not built yet (AC18).
    Portable,
    /// A packaged app (MSIX/Appx) from outside the Store: not built yet.
    Packaged,
    /// The old laptop's copy has no Windows maker to check the download against.
    NoMaker,
    /// An installer kind PCTwin does not know.
    Unknown(String),
}

impl NotInstallable {
    pub fn plain(&self) -> &'static str {
        match self {
            NotInstallable::MayRestart => {
                "This app's installer could restart your laptop without asking, so PCTwin did not run it. Install it yourself when it suits you."
            }
            NotInstallable::UnknownSwitches | NotInstallable::Unknown(_) => {
                "PCTwin cannot install this kind of installer safely by itself. Install it yourself; your data is ready."
            }
            NotInstallable::PersonOnly => {
                "This installer installs for one person only. Install it yourself while signed in as them."
            }
            NotInstallable::Portable | NotInstallable::Packaged => {
                "PCTwin cannot set up this kind of app yet. Install it yourself from the maker's website."
            }
            NotInstallable::NoMaker => {
                "PCTwin could not tell who made this app on your old laptop, so it cannot check a download. Install it yourself."
            }
        }
    }
}

/// How a package of winget installer type `installer_type` (and scope) is installed.
pub fn kind_of(installer_type: &str, scope: Option<&str>) -> Result<PackageKind, NotInstallable> {
    if scope.is_some_and(|s| s.eq_ignore_ascii_case("user")) {
        return Err(NotInstallable::PersonOnly);
    }
    match installer_type.to_ascii_lowercase().as_str() {
        "msi" | "wix" => Ok(PackageKind::Msi),
        "inno" => Ok(PackageKind::Exe(ExeKind::Inno)),
        "burn" => Ok(PackageKind::Exe(ExeKind::Burn)),
        "nullsoft" => Err(NotInstallable::MayRestart),
        "exe" => Err(NotInstallable::UnknownSwitches),
        "portable" | "zip" => Err(NotInstallable::Portable),
        "msix" | "appx" => Err(NotInstallable::Packaged),
        other => Err(NotInstallable::Unknown(other.chars().take(32).collect())),
    }
}

/// The plan operation that installs `sha256` (staged as `artifact`) for `app`, checked by the
/// helper against the old laptop's own maker. `product_code`, the manifest's, goes with a
/// Windows Installer package only: the helper records it for undo when its install made it
/// appear, for the move `for_move` (only an undo of that move can remove it). An installer
/// program never carries either (PCTwin never removes those).
pub fn operation(
    action_id: &str,
    artifact: u64,
    sha256: [u8; 32],
    kind: PackageKind,
    app: &FoundApp,
    product_code: Option<ProductCode>,
    for_move: Option<[u8; 16]>,
) -> Result<PlannedOperation, NotInstallable> {
    let AppMaker::Windows(signer) = &app.maker else {
        return Err(NotInstallable::NoMaker);
    };
    let action_id = Id::parse(action_id).ok_or(NotInstallable::Unknown("action ID".into()))?;
    Ok(PlannedOperation {
        action_id,
        action: Action::InstallVerifiedPackage(InstallVerifiedPackage {
            artifact,
            sha256: Hex32(sha256),
            kind,
            scope: InstallScope::Machine,
            maker: Box::new(crate::found::windows_signer(signer)),
            product_code: product_code.filter(|_| kind == PackageKind::Msi),
            for_move: for_move
                .filter(|_| kind == PackageKind::Msi && product_code.is_some())
                .map(Hex16),
        }),
    })
}

/// Sends a checked download to the helper for the sealed plan's `operation`. The helper keeps
/// it only if every byte arrives and they are the plan's.
pub fn stage<S: std::io::Read + std::io::Write>(
    client: &mut pctwin_helper::client::Client<S>,
    operation: &PlannedOperation,
    size: u64,
    from: &mut impl Read,
) -> Result<(), String> {
    let Action::InstallVerifiedPackage(p) = &operation.action else {
        return Err("not an install".into());
    };
    match client.stage(p.artifact, p.sha256, size, from) {
        Ok(Ok(())) => Ok(()),
        Ok(Err(why)) => Err(why.to_string()),
        Err(e) => Err(e.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pctwin_record::{AppArch, AppSystem, FoundWhere, Owner, RegistryView, SignerId};

    #[test]
    fn only_typed_installer_kinds_with_documented_switches_are_installed() {
        assert_eq!(kind_of("wix", Some("machine")), Ok(PackageKind::Msi));
        assert_eq!(kind_of("MSI", None), Ok(PackageKind::Msi));
        assert_eq!(kind_of("inno", None), Ok(PackageKind::Exe(ExeKind::Inno)));
        assert_eq!(kind_of("burn", None), Ok(PackageKind::Exe(ExeKind::Burn)));
        assert_eq!(kind_of("nullsoft", None), Err(NotInstallable::MayRestart));
        assert_eq!(kind_of("exe", None), Err(NotInstallable::UnknownSwitches));
        assert_eq!(kind_of("zip", None), Err(NotInstallable::Portable));
        assert_eq!(kind_of("msix", None), Err(NotInstallable::Packaged));
        assert_eq!(
            kind_of("wix", Some("user")),
            Err(NotInstallable::PersonOnly)
        );
        assert!(matches!(
            kind_of("pwa", None),
            Err(NotInstallable::Unknown(_))
        ));
    }

    #[test]
    fn the_plan_names_the_old_laptops_maker_and_the_exact_bytes() {
        let mut app = FoundApp {
            name: "Writer".into(),
            version: None,
            publisher: None,
            owner: Owner::Shared,
            system: AppSystem::Windows,
            found: FoundWhere::WindowsEntry {
                view: RegistryView::Machine,
                key: "w".into(),
                product_code: None,
            },
            arch: AppArch::UNKNOWN,
            maker: AppMaker::Windows(Box::new(SignerId {
                organization: Some("Example Ltd".into()),
                certificate: [4; 32],
                ..Default::default()
            })),
            web: None,
            update_feed: None,
            installers: vec![],
            notes: vec![],
        };
        let code = ProductCode::parse("{11111111-2222-3333-4444-555555555555}");
        let op = operation(
            "install.writer",
            3,
            [9; 32],
            PackageKind::Msi,
            &app,
            code,
            Some([5; 16]),
        )
        .unwrap();
        let Action::InstallVerifiedPackage(p) = &op.action else {
            panic!()
        };
        assert_eq!(p.artifact, 3);
        assert_eq!(p.sha256, Hex32([9; 32]));
        assert_eq!(p.scope, InstallScope::Machine);
        assert_eq!(p.maker.certificate, pctwin_maker::Sha256([4; 32]));
        // A Windows Installer package carries its product code for undo; an installer program
        // never does, even when its manifest names one.
        assert_eq!(p.product_code, code);
        assert_eq!(p.for_move, Some(Hex16([5; 16])));
        // No product code: no move either (nothing would be recorded).
        let unnamed = operation(
            "install.writer",
            3,
            [9; 32],
            PackageKind::Msi,
            &app,
            None,
            Some([5; 16]),
        )
        .unwrap();
        let Action::InstallVerifiedPackage(u) = &unnamed.action else {
            panic!()
        };
        assert_eq!((u.product_code, u.for_move), (None, None));
        let exe = operation(
            "install.writer",
            3,
            [9; 32],
            PackageKind::Exe(ExeKind::Inno),
            &app,
            code,
            Some([5; 16]),
        )
        .unwrap();
        let Action::InstallVerifiedPackage(e) = &exe.action else {
            panic!()
        };
        assert_eq!((e.product_code, e.for_move), (None, None));
        app.maker = AppMaker::Unsigned;
        assert_eq!(
            operation(
                "install.writer",
                3,
                [9; 32],
                PackageKind::Msi,
                &app,
                code,
                None
            ),
            Err(NotInstallable::NoMaker)
        );
    }
}
