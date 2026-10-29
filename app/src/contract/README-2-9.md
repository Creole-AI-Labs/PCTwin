# 2.9 deliverables: the screens' contract

Engine side of Task List 2.9 ("Phase 2 deliverables made explicit"). Every shape below has an
example file here, written by `app/src-tauri/src/contract.rs`
(`PCTWIN_WRITE_CONTRACT=1 cargo test -p pctwin-app contract`). Each new file needs a parse
function in `bridge.ts` before `contract.test.ts` passes again; `shell/calls.json` lists the new
commands and events with their argument names.

## Inside the plan the person approves (`plan-view.json`, `rebuild` section)

| Field | Shape | Task List 2.9 item |
|---|---|---|
| `rebuild.internet` | `{ downloads, signIns }`: apps PCTwin downloads (sizes known only once downloaded) and things the person signs in to again online | 1. "Before we change anything" preview |
| `rebuild.room` | `{ installs, appData }` bytes beyond the files at the busiest moment; margins, say "about" | 1. preview |
| `rebuild.prompt`, `rebuild.restorePoint` (already there) | the one administrator prompt, the restore point | 1. preview (permissions) |
| `changes[].action` (already there) | `install`, `merge`, `replace`, `leave-alone` | 1. preview |
| `changes[].undo` | new words `removes-if-package` (Windows: undo removes it if it came as a Windows Installer package and is still that version), `needs-you-maker` (sign-ins and relinking); also `can-reverse`, `must-keep`, `remove-yourself` | 5. Undo explanation for every change |
| `changes[].undoIfUpdated` | `keeps-newer` when the person may tick "Update it": undo cannot bring the older version back | 5. Undo explanation |
| `changes[].tested` | apps only: `{ outcome, aspects: [{ aspect, result }], steps, checkedOn, how, version }`; `outcome` is `works`, `works-with-steps`, `partly`, `not-supported` or `not-tested`; `aspect` is `install`, `data-import`, `settings`, `launch`, `undo`; `result` is `works`, `guided`, `not-supported`, `not-applicable`, `not-tested`; `steps` are `sign-in`, `licence`, `import-in-app`, `relink`, `restart`; `how` is `lab` or `real-laptops` | 8. Compatibility record |

`guard-snapshot.json` gains the same two undo words for the guard's `changes` screen.

## New commands

| Command | Arguments | Returns | Item |
|---|---|---|---|
| `work_checks` | none | `WorkChecks` (`work-checks.json`) | 2. "Can I actually work?" |
| `work_check_add` | `pick`: `{kind:"file"}` (system picker), `{kind:"font", family}`, `{kind:"task", task}` | `WorkChecks` or `WorkProblem` | 2 |
| `work_check_run` | none | `WorkChecks` or `WorkProblem` | 2 |
| `work_check_answer` | `index`, `generation`, `worked` | `WorkChecks` or `WorkProblem` | 2 |
| `work_check_remove` | `index`, `generation` | `WorkChecks` or `WorkProblem` | 2 |
| `repair_offer` | none | `RepairOffer` (`repair-offer.json`) | 4. Repair only what failed |
| `repair_run` | `generation`, `lines` | `RepairDone` or `RepairProblem` | 4 |
| `move_changes` | none | `ChangesView` or null (`move-changed.json`) | 6. Changes during the move |
| `move_reapprove` | `generation`, `approve` | nothing, or `MoveProblem` | 6 |
| `safe_stop` | none | `SafeStopView` (`safe-stop.json`) | 6. Update and uninstall wait for a safe point |
| `recovery_kit_save` | `id` (a My PCTwin history ID) | `KitSaved` or null (picked nowhere), or `MoveProblem` | 7. Offline recovery kit |

## New events

| Event | Payload | Item |
|---|---|---|
| `move-changed` | `ChangesView` | 6 |
| `safe-stop` | `SafeStopView` | 6 |

## Words

- `WorkCheck.what`: `linked-files`, `font`, `archive`, `database`, `task`.
- `WorkCheck.label`: `machine-checked`, `person-confirmed`, `still-needs-attention` (always shown).
- `WorkCheck.state`: `not-run-yet`, `all-there`, `missing`, `damaged`, `not-there`, `waiting-for-you`, `you-said-no`, `outside-project`, `could-not-read`, `not-supported`.
- `WorkProblem`: `not-chosen`, `too-many`, `no-such-check`, `stale`, `bad-name`, `wrong-kind`, `failed`.
- `RepairLine.waitsFor`: `old-laptop`, `internet`, `administrator`, `installer-tool`, `space`, `close-app` (empty: it can be tried now).
- `RepairProblem`: `nothing-to-repair`, `stale`, `busy`, `failed`.
- `ChangedAfter.why`: `app-updated`, `app-removed`, `destination-changed`, `project-edited`; `steps`: `install`, `settings`, `app-data`, `files`.
- `SafeStopView.busy`: `moving`, `rebuilding`, `app-data`, `undoing`.
- `KitSaved.holds` / `lacks`: `move-record`, `results`, `before-wipe-list`, `file-data`, `setup-steps`, `recovery-tool`.

Names (`name`, `missing`, `folder`, `files`, `version`) are shown exactly as they are, never as
markup or links. Item 3 (consistent capture of running apps) adds no new shape: it shows through
the existing `move-appdata-open` (close these apps) and the result words.
