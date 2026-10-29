# Finishing a move (Task List 2.7): the engine's commands for the screens

Written by the engine side. The shapes are in `app/src-tauri/src/finish.rs`; one example of every
shape is in `finish/*.json` and the exact command and argument names are in
`shell/finish-calls.json`. Both are written by the engine's own test:

    PCTWIN_WRITE_CONTRACT=1 cargo test -p pctwin-app contract

They sit apart from the move's files (`*.json`, `shell/calls.json`), so the move's bridge tests stay
green until the finishing bridge is written. When `finish.ts` is written, add a test that parses
every `finish/*.json` with its parse function (as `contract.test.ts` does) and one that calls every
command in `shell/finish-calls.json` by name (as `bridge.test.ts` does). The engine's permission
test already reads both calls files.

Every word is a key: kebab-case values and `kind` tags. Names the person knows (their files, apps,
places) come as they are, and only the mover's own; other people's things are only counted. A move
is named by its My PCTwin history entry ID (`id`); none means the newest move. Errors are a
`FinishProblem` (`finish-problem.json`): `no-move`, `changed`, `not-allowed`, `busy`, `failed`.

| Command | Arguments | Returns | Task List 2.7 item (screen) |
|---|---|---|---|
| `finish_overview` | `id` | `FinishView` (`finish-view.json`) | After-move check: counts at scan time against the new laptop now; parental-control card (R01) |
| `goodbye_list` | `id` | `GoodbyeView` (`goodbye-view.json`) | Before any wipe: what is only on the old laptop, keep-until date, list saved on the new laptop (R08) |
| `goodbye_answer` | `id`, `exception`, `shown`, `answer` (`accept-loss` / `did-it`) | `GoodbyeView` | Before any wipe: each item done or accepted as lost (R08) |
| `goodbye_keep_until` | `id`, `at` (seconds since 1970) | `GoodbyeView` | Before any wipe: keep-until date (R08) |
| `catchup_arm` | `id`, `purpose` (`check` / `catch-up`) | nothing | Bring me up to date; check the old laptop again (R13). Then pair and start as for a move |
| event `finish-delta` | | `DeltaView` (`finish-delta.json`) | Bring me up to date: what changed, by item ID (R13) |
| `catchup_approve` | `id`, `digest`, `choices` | nothing | Bring me up to date: send only the reviewed changes (R13) |
| `handover_preview` | `id`, `action` (`remove-my-account` / `erase-old-laptop`) | `HandoverView` (`handover-view.json`) | Handing it on to family; safe goodbye readiness and wipe method (R08, R09) |
| `handover_confirm` | `id`, `action`, `digest` | `HandoverStarted` (`handover-started.json`) | Separate irreversible confirmation; the system's own route (R09, R10) |
| `handover_result` | `id`, `action`, `says` | `HandoverRecorded` (`handover-recorded.json`) | Wipe / handover result as observed, never a certificate (R10) |
| `doctor_run` | `role` (`old` / `new`) | `DoctorView` (`doctor-view.json`) | Full connection doctor with phone-hotspot fallback |
| `setup_checklist` | `id` | `SetupView` (`setup-view.json`) | Finish-your-setup checklist (R03) |
| `setup_mark` | `id`, `task`, `done` | `SetupView` | R03: the person's word kept apart from PCTwin's check |
| `setup_save` | `id` | `SavedSetup` (`saved-setup.json`) | Save setup profile and app list, kept in My PCTwin history (R04) |
| `undo_preview` | `id` | `UndoPreviewView` (`undo-preview.json`) | Undo preview (R05); execution is the existing `fullmove_undo` with `undoId` (R06) |
| `cleanup_preview` | `what` (`temporary-files` / `model-and-cache`) | `CleanupView` (`cleanup-view.json`) | Source cleanup and PCTwin model/cache removal, each asked separately (R07) |
| `cleanup_confirm` | `what`, `digest` | `CleanupDone` (`cleanup-done.json`) | R07 |
| `report_preview` | `id`, `disclosure` | `ReportView` (`report-view.json`) | Shareable report with redaction preview (R11) |
| `report_save` | `id`, `disclosure`, `digest`, `text` | saved path or `null` | R11: the person saves the text they read; refused if it holds anything kept out. Feedback (R12) stays in Help, Feedback, no account; nothing is sent automatically |

Confirmations: every `digest` / `shown` value is sent back exactly as received. A changed list
answers `changed`; look again and ask the person again. No countdown on any confirmation.
