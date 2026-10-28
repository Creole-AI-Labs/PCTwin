/**
 * Made-up views of the Phase 2 screens, for the screens page and the tests. They include hostile
 * and unusually long names, right-to-left and mixed-script names, partial results and mixed states
 * that look contradictory but are legitimate. A review tool: the app never shows these as real.
 */
import type { DisplayState, IntentName, Measured, Projection, ScreenView } from "./guard";

/** A name that would be markup, a script or a link if it were not shown as text. */
export const HOSTILE = `<img src=x onerror="alert(1)">javascript:alert(2)<a href="https://evil.example">x</a>`;
export const LONG = `${"Quarterly report final FINAL really final ".repeat(5)}.docx`;
export const RTL = "تقرير المشروع.pdf";
export const MIXED = "Фото 東京 2024 — résumé.jpg";

export function view(screen: ScreenView, display: DisplayState, allowed: IntentName[], progress: Measured | null = null, generation = 3): Projection {
  return { job: "job-sample", epoch: 2, generation, version: 7, display, allowed, progress, screen };
}

export const changes = view(
  {
    kind: "changes",
    changes: [
      { name: "VLC media player", action: "install", undo: "can-remove-app" },
      { name: "Photos 2024", action: "merge", undo: "can-reverse" },
      { name: "Dark mode", action: "replace", undo: "can-reverse" },
      { name: "Mail account (Outlook)", action: "merge", undo: "needs-you-service" },
      { name: "Adobe licence", action: "install", undo: "needs-you-maker" },
      { name: "Documents already on this laptop", action: "leave-alone", undo: "must-keep" },
      { name: HOSTILE, action: "merge", undo: "can-reverse" },
      { name: RTL, action: "merge", undo: "can-reverse" },
    ],
    more: 12,
    downloadBytes: 1_840_000_000,
    drives: [
      { drive: "C:", needed: 182_000_000_000, free: 240_000_000_000, fits: true, shortBy: 0, parts: [{ usedFor: "files", bytes: 170_000_000_000 }, { usedFor: "undo-record", bytes: 2_000_000_000 }] },
      { drive: "D: Data", needed: 40_000_000_000, free: null, fits: null, shortBy: 0, parts: [{ usedFor: "files", bytes: 40_000_000_000 }] },
    ],
    permissions: ["install-for-everyone", "restore-point"],
    restorePoint: true,
  },
  "awaiting-person",
  ["approve", "back"],
);

export const changesNothingToDownload = view(
  { kind: "changes", changes: [{ name: "Wallpaper", action: "replace", undo: "can-reverse" }], more: 0, downloadBytes: 0, drives: [], permissions: [], restorePoint: false },
  "awaiting-person",
  ["approve", "back"],
);

export const changed = view(
  {
    kind: "changed",
    lines: [
      { name: "Thesis.docx", why: "changed-on-old-laptop" },
      { name: "Spotify", why: "app-updated" },
      { name: "Pictures", why: "destination-changed" },
    ],
    more: 0,
  },
  "awaiting-person",
  ["approve", "leave-out-changed"],
  null,
  4,
);

export const spaceShort = view(
  {
    kind: "space",
    drives: [{ drive: "C:", needed: 300_000_000_000, free: 256_000_000_000, fits: false, shortBy: 45_000_000_000, parts: [{ usedFor: "files", bytes: 290_000_000_000 }, { usedFor: "app-downloads", bytes: 6_000_000_000 }, { usedFor: "undo-record", bytes: 4_000_000_000 }] }],
    proposal: [
      { to: "leave-in-cloud", items: 1204, bytes: 30_000_000_000 },
      { to: "offload-drive", items: 88, bytes: 16_000_000_000 },
    ],
    offload: { drive: "Backup Plus", free: 900_000_000_000, approved: false, connected: true },
    safeIn: [{ provider: "OneDrive", files: 1204 }],
  },
  "awaiting-person",
  ["approve", "approve-offload", "back"],
);

export const spaceOffloadAway = view(
  {
    kind: "space",
    drives: [{ drive: "C:", needed: 200_000_000_000, free: 256_000_000_000, fits: true, shortBy: 0, parts: [{ usedFor: "files", bytes: 200_000_000_000 }] }],
    proposal: [{ to: "offload-drive", items: 88, bytes: 16_000_000_000 }],
    offload: { drive: "Backup Plus", free: null, approved: true, connected: false },
    safeIn: [],
  },
  "blocked",
  ["back"],
);

const local = (state: "choosing" | "working" | "paused-metered" | "paused-no-space" | "done", display: DisplayState, allowed: IntentName[], progress: Measured | null = null) =>
  view({ kind: "local-copy", provider: "OneDrive", files: 342, downloadBytes: 12_400_000_000, oldFreeBytes: 9_000_000_000, state }, display, allowed, progress);

export const localChoosing = local("choosing", "awaiting-person", ["keep-local-copy"]);
export const localWorking = local("working", "working", ["pause"], { done: 3_100_000_000, total: 12_400_000_000, unit: "bytes" });
export const localMetered = local("paused-metered", "paused", ["allow-metered", "resume"]);
export const localNoSpace = local("paused-no-space", "paused", ["keep-local-copy"]);

export const owners = view(
  {
    kind: "owners",
    rows: [
      { laptop: "Kitchen laptop", person: "Ada", account: "ada", ownFolder: false },
      { laptop: "Kitchen laptop", person: "Bea", account: null, ownFolder: false },
      { laptop: "Old work laptop", person: "Ada", account: "ada", ownFolder: true },
      { laptop: RTL, person: MIXED, account: null, ownFolder: true },
    ],
    more: 0,
  },
  "awaiting-person",
  ["approve", "back"],
);

const permission = (stage: "explain" | "waiting-for-system" | "denied" | "closed-without-answer" | "given" | "ended-by-sleep" | "ended-other-session", allowed: IntentName[], display: DisplayState, restorePoint: Extract<ScreenView, { kind: "permission" }>["restorePoint"] = null) =>
  view({ kind: "permission", stage, reasons: ["read-other-people", "make-accounts", "restore-point"], restorePoint }, display, allowed);

export const permissionExplain = permission("explain", ["ask-system", "not-now"], "awaiting-person");
export const permissionWaiting = permission("waiting-for-system", [], "awaiting-system");
export const permissionClosed = permission("closed-without-answer", ["ask-system", "not-now"], "awaiting-person");
export const permissionSlept = permission("ended-by-sleep", ["ask-system", "not-now"], "blocked", { kind: "made", at: "27 October 2026, 10:04" });
export const permissionOtherSession = permission("ended-other-session", ["ask-system", "not-now"], "blocked", { kind: "earlier-exists", at: "27 October 2026, 08:15" });
export const permissionDenied = permission("denied", [], "working", { kind: "not-chosen" });
export const permissionGiven = permission("given", [], "working", { kind: "not-made", why: "protection-off" });

export const rescueIntro = view({ kind: "rescue", stage: { kind: "intro" } }, "awaiting-person", ["check-again", "back"]);
export const rescuePick = view(
  {
    kind: "rescue",
    stage: {
      kind: "pick-drive",
      drives: [
        { drive: "Old laptop drive (E:)", size: 512_000_000_000, fileSystem: "NTFS", isTarget: false },
        { drive: "This laptop (C:)", size: 1_000_000_000_000, fileSystem: "NTFS", isTarget: true },
      ],
      refused: 1,
    },
  },
  "awaiting-person",
  ["choose-drive", "back"],
);
export const rescueBitLocker = view({ kind: "rescue", stage: { kind: "locked", lock: "bit-locker" } }, "blocked", ["check-again", "back"]);
export const rescueAppleChip = view({ kind: "rescue", stage: { kind: "locked", lock: "apple-chip" } }, "blocked", ["back"]);
export const rescueSalvage = view(
  { kind: "rescue", stage: { kind: "salvage", readable: 48_210, readOnly: "pctwin-reads-only", unreadable: ["Users/ada/Videos/holiday.mp4", LONG, HOSTILE], more: 31 } },
  "partial",
  ["approve", "back"],
);

export const report = view(
  {
    kind: "report",
    lines: [
      { line: 0, what: { kind: "items", count: 14 }, laptop: "Kitchen laptop", why: "laptop-not-connected" },
      { line: 1, what: { kind: "app", name: "Zoom" }, laptop: "Kitchen laptop", why: "failed" },
      { line: 2, what: { kind: "settings", count: 1 }, laptop: RTL, why: "waits-for-another" },
    ],
    more: 0,
    safeIn: [{ provider: "Google Drive", files: 77 }, { provider: null, files: 3 }],
  },
  "partial",
  ["try-again"],
);

export const undoPreview = view(
  {
    kind: "undo-preview",
    lines: [
      { name: "Photos 2024", plan: "put-back" },
      { name: "VLC media player", plan: "remove-app" },
      { name: "Thesis.docx", plan: "keep-changed" },
      { name: "Mail account (Outlook)", plan: "elsewhere" },
      { name: "Documents already on this laptop", plan: "leave-alone" },
    ],
    more: 0,
    when: 1793000000,
  },
  "awaiting-person",
  ["undo", "back"],
);

export const undoDone = view(
  {
    kind: "undo-done",
    lines: [
      { name: "Dark mode", result: { kind: "restored" } },
      { name: "VLC media player", result: { kind: "removed" } },
      { name: "Thesis.docx", result: { kind: "kept-changed" } },
      { name: "Budget.xlsx", result: { kind: "kept-at", at: "Budget (kept by PCTwin undo).xlsx" } },
      { name: "Mail account (Outlook)", result: { kind: "done-elsewhere" } },
      { name: "Notes.txt", result: { kind: "not-done-yet" } },
    ].map((l) => ({ ...l, detail: l.result.kind === "kept-changed" ? "it changed since the move" : l.result.kind === "not-done-yet" ? "it was in use" : null })) as Extract<ScreenView, { kind: "undo-done" }>["lines"],
    more: 0,
  },
  "partial",
  [],
);

export const undoRunning = view({ kind: "undo-done", lines: [{ name: "Dark mode", result: { kind: "restored" }, detail: null }], more: 0 }, "applying", ["pause"], { done: 3, total: null, unit: "items" });

export const reconciling = view({ kind: "reconciling" }, "reconciling", [], { done: 120, total: null, unit: "items" }, 0);
