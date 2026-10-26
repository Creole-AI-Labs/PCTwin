//! The laptop's PCTwin name (decided 26 October 2026): the name other laptops nearby find this one
//! by when pairing. Chosen once, the first time PCTwin opens, and kept on this laptop only (no
//! account, nothing sent anywhere); pairing uses it from then on, and a change made while pairing
//! is kept too.
//!
//! The suggestion is the laptop's own name from its settings ("Ezekiel's MacBook Air", "Ezzy
//! Laptop") when it reads like a name a person gave it. A name the factory or the system made up
//! ("DESKTOP-7H3K2LP", "localhost") is not suggested: a random colour and animal is, shown in the
//! person's language. Anyone on the same Wi-Fi can see the name, and the page says so.

use std::path::{Path, PathBuf};
use std::sync::Mutex;

use pctwin_discovery::Label;
use pctwin_journey::Problem;
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager, Runtime, State};

use crate::pairing::{LabelView, ProblemView};

/// The name being suggested on the page, so Use this name keeps exactly what was shown.
#[derive(Default)]
pub struct LaptopName {
    shown: Mutex<Option<Label>>,
}

pub fn new_state() -> LaptopName {
    LaptopName::default()
}

fn lock<T>(m: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    m.lock().unwrap_or_else(std::sync::PoisonError::into_inner)
}

/// The name as kept on disk.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case", deny_unknown_fields)]
enum Kept {
    Picked { colour: u8, animal: u8 },
    Named { name: String },
}

fn file_of<R: Runtime>(app: &AppHandle<R>) -> Option<PathBuf> {
    app.path()
        .app_data_dir()
        .ok()
        .map(|d| d.join("laptop-name.json"))
}

/// The kept name at `file`, checked again as if just typed: a file changed by hand can never put
/// a name on the network that the person could not have chosen. `None` when there is none.
fn read(file: &Path) -> Option<Label> {
    let kept: Kept = serde_json::from_slice(&std::fs::read(file).ok()?).ok()?;
    match kept {
        Kept::Picked { colour, animal } => {
            let label = Label::Picked { colour, animal };
            label.keys().is_some().then_some(label)
        }
        Kept::Named { name } => Label::named(&name).ok(),
    }
}

/// Keeps `label` at `file`, in one step so a crash never leaves half of it.
fn write(file: &Path, label: &Label) -> std::io::Result<()> {
    let kept = match label {
        Label::Picked { colour, animal } => Kept::Picked {
            colour: *colour,
            animal: *animal,
        },
        Label::Named(name) => Kept::Named { name: name.clone() },
    };
    if let Some(dir) = file.parent() {
        std::fs::create_dir_all(dir)?;
    }
    let tmp = file.with_extension("part");
    std::fs::write(&tmp, serde_json::to_vec(&kept)?)?;
    std::fs::rename(&tmp, file)
}

/// The name this laptop keeps, if one was chosen.
pub fn kept<R: Runtime>(app: &AppHandle<R>) -> Option<Label> {
    read(&file_of(app)?)
}

/// Keeps `label` as this laptop's name. Failing to keep it never fails pairing.
pub fn keep<R: Runtime>(app: &AppHandle<R>, label: &Label) {
    if let Some(file) = file_of(app) {
        let _ = write(&file, label);
    }
}

/// Whether `name` (the laptop's own name from its settings) looks made up by the factory or the
/// system rather than given by a person: Windows' "DESKTOP-7H3K2LP", "LAPTOP-…" and "WIN-…",
/// the usual Linux defaults, or a code with no word in it.
fn made_up(name: &str) -> bool {
    let lower = name.trim().to_lowercase();
    const DEFAULTS: [&str; 10] = [
        "localhost",
        "ubuntu",
        "debian",
        "fedora",
        "raspberrypi",
        "linux",
        "computer",
        "pc",
        "laptop",
        "macbook",
    ];
    if lower.is_empty() || DEFAULTS.contains(&lower.as_str()) {
        return true;
    }
    for prefix in ["desktop-", "laptop-", "win-", "pc-", "user-pc", "yourname"] {
        if lower.starts_with(prefix) {
            return true;
        }
    }
    // No word of three letters or more ("A1B2C3", "X-123"): a code, not a name.
    !name
        .split(|c: char| !c.is_alphabetic())
        .any(|word| word.chars().count() >= 3)
}

/// The laptop's own name as a PCTwin name, when it reads like one a person gave it. Windows names
/// cannot hold spaces, so people join words with dashes ("Ezzy-Laptop"); shown with spaces.
fn from_device(name: &str) -> Option<Label> {
    if made_up(name) {
        return None;
    }
    let spaced = name.trim().replace(['-', '_'], " ");
    Label::named(&spaced).ok()
}

/// What the naming page shows: the name kept already, if any, and the suggestion.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct NameView {
    pub kept: Option<LabelView>,
    pub suggested: LabelView,
}

/// The kept name, and a suggestion: the one shown already, else the laptop's own name, else a
/// random colour and animal.
#[tauri::command]
pub fn laptop_name<R: Runtime>(
    app: AppHandle<R>,
    state: State<'_, LaptopName>,
) -> Result<NameView, ProblemView> {
    let mut shown = lock(&state.shown);
    let suggested = match shown.clone() {
        Some(label) => label,
        None => {
            let label = match whoami::fallible::devicename()
                .ok()
                .and_then(|n| from_device(&n))
            {
                Some(label) => label,
                None => Label::random().map_err(|_| Problem::Failed)?,
            };
            *shown = Some(label.clone());
            label
        }
    };
    Ok(NameView {
        kept: kept(&app).as_ref().map(LabelView::from),
        suggested: LabelView::from(&suggested),
    })
}

/// Choose another name: a new random colour and animal, shown in place of the suggestion.
#[tauri::command]
pub fn laptop_name_shuffle(state: State<'_, LaptopName>) -> Result<LabelView, ProblemView> {
    let mut shown = lock(&state.shown);
    let current: Vec<Label> = shown.iter().cloned().collect();
    let label = Label::shuffle(&current).map_err(|_| Problem::Failed)?;
    *shown = Some(label.clone());
    Ok(LabelView::from(&label))
}

/// Use this name: keeps the name shown.
#[tauri::command]
pub fn laptop_name_use<R: Runtime>(
    app: AppHandle<R>,
    state: State<'_, LaptopName>,
) -> Result<LabelView, ProblemView> {
    let label = lock(&state.shown).clone().ok_or(Problem::Failed)?;
    let file = file_of(&app).ok_or(Problem::Failed)?;
    write(&file, &label).map_err(|_| Problem::Failed)?;
    Ok(LabelView::from(&label))
}

/// A name the person typed, checked as any name shown on the network, and kept.
#[tauri::command]
pub fn laptop_name_type<R: Runtime>(
    app: AppHandle<R>,
    state: State<'_, LaptopName>,
    name: String,
) -> Result<LabelView, ProblemView> {
    let label = Label::named(&name).map_err(|_| Problem::InvalidName)?;
    let file = file_of(&app).ok_or(Problem::Failed)?;
    write(&file, &label).map_err(|_| Problem::Failed)?;
    *lock(&state.shown) = Some(label.clone());
    Ok(LabelView::from(&label))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_the_factory_or_system_made_up_are_not_suggested() {
        for name in [
            "DESKTOP-7H3K2LP",
            "LAPTOP-QJ8V2M1A",
            "WIN-4F2KD8S0QPL",
            "localhost",
            "ubuntu",
            "raspberrypi",
            "A1B2C3",
            "X-123",
            "",
        ] {
            assert!(made_up(name), "{name}");
            assert_eq!(from_device(name), None, "{name}");
        }
    }

    #[test]
    fn a_name_a_person_gave_is_suggested_with_spaces_for_dashes() {
        assert_eq!(
            from_device("Ezzy-Laptop"),
            Some(Label::Named("Ezzy Laptop".into()))
        );
        assert_eq!(
            from_device("Ezekiel’s MacBook Air"),
            Some(Label::Named("Ezekiel’s MacBook Air".into()))
        );
        assert_eq!(
            from_device("kitchen_pc"),
            Some(Label::Named("kitchen pc".into()))
        );
    }

    #[test]
    fn a_name_is_kept_and_read_back_and_a_file_changed_by_hand_is_checked_again() {
        let dir = std::env::temp_dir().join(format!("pctwin-laptop-name-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let file = dir.join("laptop-name.json");
        assert_eq!(read(&file), None, "nothing chosen yet");
        let picked = Label::Picked {
            colour: 1,
            animal: 2,
        };
        write(&file, &picked).unwrap();
        assert_eq!(read(&file), Some(picked));
        let named = Label::Named("Ezzy Laptop".into());
        write(&file, &named).unwrap();
        assert_eq!(read(&file), Some(named));
        // Changed by hand: a name no one could type, or a colour out of range, is not used.
        std::fs::write(&file, "{\"kind\":\"named\",\"name\":\"a\\u202Eb\"}").unwrap();
        assert_eq!(read(&file), None);
        std::fs::write(&file, r#"{"kind":"picked","colour":250,"animal":0}"#).unwrap();
        assert_eq!(read(&file), None);
        std::fs::write(&file, b"not json").unwrap();
        assert_eq!(read(&file), None);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
