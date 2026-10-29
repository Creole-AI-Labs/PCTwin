//! The offline recovery kit (Task List 2.9; Engineering Plan A02 as far as Phase 2 goes): the move
//! record kept, readable with no account, and saying exactly what it holds and what it does not.
//!
//! The kit itself is `pctwin_readiness::kit::Kit`: the move record, the result of every item with
//! how it was checked, and the before-wipe list when one was made; a plain JSON file and a plain
//! text "read me" that any text editor opens, written as new files in a folder the person picks
//! in the system's own picker (never over a file already there). It holds no file data, no setup
//! steps and no restore tool, and says so: instructions alone cannot bring back missing files.
//! A kit made by a newer PCTwin is refused by name, never half read (`Kit::read`).
//!
//! What a move leaves for finishing it (its record and results, kept per move beside My PCTwin
//! history and never removed) is kept by the finishing part (Task List 2.7, `finish_store`):
//! [`kit_of`] is the one join to it.

use std::path::{Path, PathBuf};

use pctwin_readiness::kit::{Contents, Kit};
use serde::Serialize;
use tauri::{AppHandle, Runtime};

use crate::moving::MoveProblem;

pctwin_types::listed_enum! {
    /// One part a kit holds, or does not.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
    #[serde(rename_all = "kebab-case")]
    pub enum KitPart {
        /// What was picked to move, and where each thing went.
        MoveRecord,
        /// The result of every item, and how each was checked.
        Results,
        /// What is still only on the old laptop.
        BeforeWipeList,
        /// The files themselves (never in a kit: instructions cannot bring back missing files).
        FileData,
        /// The steps to set apps and settings up again.
        SetupSteps,
        /// A tool that restores anything.
        RecoveryTool,
    }
}

/// A kit saved where the person chose.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct KitSaved {
    /// The folder, shown exactly as it is.
    pub folder: String,
    /// The files written there, by name.
    pub files: Vec<String>,
    pub holds: Vec<KitPart>,
    pub lacks: Vec<KitPart>,
}

/// What a kit holds and lacks, from its own contents (the same ones its read-me states).
pub fn parts(c: &Contents) -> (Vec<KitPart>, Vec<KitPart>) {
    let all = [
        (KitPart::MoveRecord, c.move_record),
        (KitPart::Results, c.results),
        (KitPart::BeforeWipeList, c.before_wipe_list),
        (KitPart::FileData, c.file_data),
        (KitPart::SetupSteps, c.setup_recipe),
        (KitPart::RecoveryTool, c.recovery_tool),
    ];
    let holds = all.iter().filter(|(_, h)| *h).map(|(p, _)| *p).collect();
    let lacks = all.iter().filter(|(_, h)| !*h).map(|(p, _)| *p).collect();
    (holds, lacks)
}

/// Writes `kit` into `folder` and reads it back as a kit, so a kit that cannot be read again is
/// never reported saved.
pub fn save(kit: &Kit, folder: &Path) -> Result<KitSaved, MoveProblem> {
    let (json, readme) = kit.export(folder).map_err(|_| MoveProblem::Failed)?;
    let back = Kit::read(&json).map_err(|_| MoveProblem::Failed)?;
    if back != *kit {
        return Err(MoveProblem::Failed);
    }
    let name = |p: &PathBuf| {
        p.file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default()
    };
    let (holds, lacks) = parts(&kit.contents);
    Ok(KitSaved {
        folder: crate::guard_views::name(&folder.to_string_lossy()),
        files: vec![name(&json), name(&readme)],
        holds,
        lacks,
    })
}

/// The kit of the move `id` (a My PCTwin history ID), from what the move left for finishing it
/// (Task List 2.7: its record, its ledger checked against the journal now, its before-wipe list).
/// `None`: nothing is kept for that move here, so the screens are told so (`NotMoving`), never
/// shown an empty kit.
fn kit_of<R: Runtime>(app: &AppHandle<R>, id: &str) -> Option<Kit> {
    let (record, ledger, checklist) = crate::finish::kit_parts(app, id)?;
    Some(Kit::new(
        record,
        ledger,
        checklist,
        crate::finish_store::now(),
    ))
}

/// Saves the recovery kit of the move `id` (an ID from My PCTwin history) in a folder the person
/// picks. `None`: they picked nowhere. `NotMoving`: nothing is kept for that move.
#[tauri::command]
pub async fn recovery_kit_save<R: Runtime>(
    app: AppHandle<R>,
    id: String,
) -> Result<Option<KitSaved>, MoveProblem> {
    if id.is_empty() || id.len() > 64 || !id.bytes().all(|b| b.is_ascii_digit() || b == b'-') {
        return Err(MoveProblem::Failed);
    }
    let kit = kit_of(&app, &id).ok_or(MoveProblem::NotMoving)?;
    let folder = tauri::async_runtime::spawn_blocking(|| rfd::FileDialog::new().pick_folder())
        .await
        .map_err(|_| MoveProblem::Failed)?;
    let Some(folder) = folder else {
        return Ok(None);
    };
    save(&kit, &folder).map(Some)
}

#[cfg(test)]
mod tests {
    use super::*;
    use pctwin_readiness::ledger::{Ledger, MoveId};
    use pctwin_record::{LaptopId, Record};

    fn kit() -> Kit {
        let old = LaptopId::from_hex("01010101010101010101010101010101").unwrap();
        let new = LaptopId::from_hex("02020202020202020202020202020202").unwrap();
        let record = Record::new(old);
        let id = MoveId {
            source: old,
            target: new,
            session: 7,
        };
        let ledger = Ledger::from_record(&record, "1001", id, 1_792_627_200);
        Kit::new(record, ledger, None, 1_792_627_200)
    }

    /// Task List 2.9: the kit says exactly what it holds and lacks (its read-me and the screens
    /// say the same), it is read back before it is called saved, and it never writes over a file.
    #[test]
    fn a_kit_is_saved_readable_and_says_exactly_what_it_holds() {
        use KitPart::*;
        let t = tempfile::tempdir().unwrap();
        let k = kit();
        let saved = save(&k, t.path()).unwrap();
        assert_eq!(saved.holds, [MoveRecord, Results]);
        assert_eq!(
            saved.lacks,
            [BeforeWipeList, FileData, SetupSteps, RecoveryTool]
        );
        assert_eq!(saved.files.len(), 2);
        for f in &saved.files {
            assert!(t.path().join(f).is_file(), "{f}");
        }
        let readme = std::fs::read_to_string(t.path().join(&saved.files[1])).unwrap();
        assert!(readme.contains("does not hold"));
        assert!(readme.contains("cannot bring back missing files"));
        assert!(readme.contains("no account"));
        // Saved again in the same folder: new files, the first kept as it was.
        let again = save(&k, t.path()).unwrap();
        assert_ne!(again.files, saved.files);
        assert_eq!(Kit::read(&t.path().join(&saved.files[0])).unwrap(), k);
    }

    #[test]
    fn every_part_is_either_held_or_lacking() {
        let mut c = Contents {
            move_record: true,
            results: false,
            before_wipe_list: true,
            file_data: true,
            setup_recipe: false,
            recovery_tool: true,
        };
        let (holds, lacks) = parts(&c);
        assert_eq!(holds.len() + lacks.len(), KitPart::VARIANTS.len());
        assert_eq!(
            holds,
            [
                KitPart::MoveRecord,
                KitPart::BeforeWipeList,
                KitPart::FileData,
                KitPart::RecoveryTool
            ]
        );
        c.results = true;
        c.setup_recipe = true;
        assert!(parts(&c).1.is_empty());
    }

    #[test]
    fn a_folder_that_cannot_take_the_kit_is_never_reported_saved() {
        let t = tempfile::tempdir().unwrap();
        let file = t.path().join("not a folder");
        std::fs::write(&file, b"x").unwrap();
        assert_eq!(save(&kit(), &file), Err(MoveProblem::Failed));
    }
}
