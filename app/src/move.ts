/**
 * The move in progress and its results, as the screens see them (X01, X02, F09, X05, X10, R01, R02).
 *
 * Every screen here is a view model: props in, events out. The shell's commands and events, called
 * and parsed in bridge.ts (src-tauri/src/moving.rs):
 *
 * Commands (Tauri `invoke`):
 * - `move_begin(fallback)`: start this laptop's side with the paired link; `fallback` names, in the
 *   person's language, the folder in the home that stands for one the new laptop lacks.
 * - `move_approve(plan, approve)`: the person's answer to exactly the plan revision on screen (M23).
 * - `move_pause()`: ask the move to pause at a safe point, which is a block boundary, not the end
 *   of a file: only what is already on its way finishes first (at most 32 MiB, the transfer's
 *   in-flight limit, IN_FLIGHT_BYTES in crates/pctwin-transfer); the file being written does not.
 *   Replies at once; the state then reads "pausing" until `move-state` says "paused".
 * - `move_resume()`: continue a paused move, or withdraw a pause that has not completed.
 * - `move_stop()`: stop the rest of the move. What is already copied and checked stays. Not undo.
 * - `move_report()`: the finished move's items, as `ReportItem[]` (the screen shows a page at a time).
 * - `set_window_mode("mini" | "everyday" | "expanded")`: exists already (native.ts).
 *
 * Events (Tauri `listen`):
 * - `move-stage`: what the move is at before files move (scanning, waiting for the other laptop, moving).
 * - `move-plan`: the plan for the person to approve (`PlanView`, new laptop only).
 * - `move-progress`: a `MoveProgress` (the old laptop's label, each group's measured counts, the totals, the run state), whenever a measured number changes (a few times a second at most).
 * - `move-finished`: `MoveOutcome` once nothing is waiting, copying or checking.
 * - `move-problem`: the move could not go on (the connection was lost, the plan was not approved, or something else).
 *
 * Mirrors: `ItemStatus` and `Reason` in crates/pctwin-types (result.rs), the write steps in
 * crates/pctwin-journal (State), and `CheckedFrom` / `WhyMemory` in the gate's read-back
 * (crates/pctwin-gate, read_back.rs). Names are the engine's kebab-case names, which are also the
 * keys of their translated text. The evidence shape is checked against the engine's own output
 * (src/contract/evidence.json, parsed by bridge.ts `parseEvidence`).
 */

import type { Rebuilt } from "./fullmove";
import type { Label } from "./pairing";
import type { FolderRole } from "./plan";

/* ---------- Results: the engine's ItemStatus and Reason ---------- */

/** Finished and verified. */
export type DoneStatus = "copied" | "reinstalled" | "converted";

/** The reasons that belong to `failed` (PCTwin could not finish it). */
export const FAILED_REASONS = [
  "not-enough-space",
  "changed-during-move",
  "unreadable",
  "not-verified",
  "refused-by-safety-check",
  "name-not-allowed",
  "install-failed",
  "no-equivalent",
  "drive-locked",
] as const;
export type FailedReason = (typeof FAILED_REASONS)[number];

/** The reasons that belong to `needs-action` (the person must finish a step). */
export const NEEDS_ACTION_REASONS = ["sign-in", "activation", "permission", "online-only", "manual-install", "restart", "finish-by-hand", "parts-missing", "moved-file-missing"] as const;
export type NeedsActionReason = (typeof NEEDS_ACTION_REASONS)[number];

export type Reason = FailedReason | NeedsActionReason;

/** Every reason, in report order (as `Reason::ALL`). */
export const ALL_REASONS: readonly Reason[] = [...FAILED_REASONS, ...NEEDS_ACTION_REASONS];

/** The one status a reason belongs to (as `Reason::status`). */
export function statusOfReason(reason: Reason): "failed" | "needs-action" {
  return (FAILED_REASONS as readonly string[]).includes(reason) ? "failed" : "needs-action";
}

/** The six ways an item can end, in report order (as `ItemStatus::ALL`). */
export const ITEM_STATUSES = ["copied", "reinstalled", "converted", "needs-action", "skipped-by-user", "failed"] as const;
export type ItemStatus = (typeof ITEM_STATUSES)[number];

/**
 * An item's result. The shape is the engine's rule (`ItemResult`): a failed or needs-action item
 * always has a reason that fits its status; a finished or skipped one has none. A wrong pairing
 * does not type-check, and `parseResult` refuses it at the boundary.
 */
export type ItemResult =
  | { status: DoneStatus | "skipped-by-user"; reason?: never }
  | { status: "failed"; reason: FailedReason }
  | { status: "needs-action"; reason: NeedsActionReason };

/** Reads a result sent by the shell, refusing any status and reason that do not fit (as the engine does). */
export function parseResult(value: unknown): ItemResult | null {
  if (typeof value !== "object" || value === null) return null;
  const { status, reason, ...extra } = value as Record<string, unknown>;
  if (Object.keys(extra).length > 0) return null;
  if (typeof status !== "string" || !(ITEM_STATUSES as readonly string[]).includes(status)) return null;
  if (status === "failed" || status === "needs-action") {
    if (typeof reason !== "string" || !(ALL_REASONS as readonly string[]).includes(reason)) return null;
    if (statusOfReason(reason as Reason) !== status) return null;
    return { status, reason } as ItemResult;
  }
  return reason === undefined ? ({ status } as ItemResult) : null;
}

/* ---------- Evidence: where a check read from ---------- */

/**
 * Why a check could not read past this laptop's cache (the gate's `WhyMemory`, in its order): the
 * drive refused, the drive did not show the read reached it, this system has no way, the file is
 * stored specially (compressed, encrypted, inline), or the file is too small. Each has its own
 * words in "more details" (`why-memory-<reason>`).
 */
export const WHY_MEMORY = ["cannot-skip-cache", "not-shown", "no-way", "stored-specially", "too-small"] as const;
export type WhyMemory = (typeof WHY_MEMORY)[number];

/** The words for one reason, shown under "more details" with the general explanation. */
export function whyMemoryKey(why: WhyMemory): string {
  return `why-memory-${why}`;
}

/**
 * How a copied file is known to be a true copy. Read back from the drive, or through this laptop's
 * memory (the gate's `CheckedFrom`, and why); an identical file already on the new laptop; checked
 * before a restart and proven again after; or, on the old laptop, checked by the new laptop. Each is
 * its own line; none is shown as another.
 */
export type Evidence =
  | { from: "drive" }
  | { from: "memory"; why: WhyMemory }
  | { from: "already-there" }
  | { from: "recovered" }
  | { from: "new-laptop" };

/**
 * A finished item and how it was checked. Only a copied file carries evidence: it was read back,
 * either from the drive or from this laptop's memory, and the report must say which.
 */
export type Outcome =
  | { status: "copied"; evidence: Evidence; reason?: never }
  | { status: "reinstalled" | "converted" | "skipped-by-user"; evidence?: never; reason?: never }
  | { status: "failed"; reason: FailedReason; evidence?: never }
  | { status: "needs-action"; reason: NeedsActionReason; evidence?: never };

/**
 * What the report adds beside a line's result (the shell's `ReportNote`): it changed on the old
 * laptop after the scan and was sent as it is now, or it was gone before it could be sent.
 */
export const REPORT_NOTES = ["changed-since-scan", "no-longer-there", "time-not-kept"] as const;
export type ReportNote = (typeof REPORT_NOTES)[number];

/** The groups the report shows files in, like a phone's storage page (as the scan counts kinds). */
export const FILE_KINDS = ["photos", "videos", "music", "documents", "other"] as const;
export type FileKind = (typeof FILE_KINDS)[number];

/**
 * One line of the detailed report (R02). Names and paths are the person's own text. Its kind and
 * size are absent only in a report kept by an older version (shown under other files).
 */
export type ReportItem = { id: string; name: string; source: string; destination?: string; note?: ReportNote; kind?: FileKind; bytes?: number } & Outcome;

/* ---------- Progress: measured numbers only ---------- */

/** What the move is doing now. */
export type RunState = "running" | "pausing" | "paused";

/**
 * Counts of items by what is happening to them now. All measured by the engine and all separate: a
 * file being written, a file being read back, a finished one and one that needs the person are
 * never merged. They add up to `total`.
 */
export type Counts = {
  total: number;
  /** Not started. */
  waiting: number;
  /** Being written on the new laptop (journal step staged). */
  copying: number;
  /** Being read back and checked. */
  checking: number;
  /** Copied (or reinstalled or converted) and checked. */
  done: number;
  needsAction: number;
  failed: number;
  skippedByUser: number;
};

export type Range = { low: number; high: number };

/** A group of the move: a folder by its role (named as the plan names it), or a folder the person added, by its name. */
export type GroupName = { kind: "role"; role: FolderRole } | { kind: "folder"; name: string };

/** Where a group stands. "copied" means every item in it is copied and checked, and nothing else. */
export type GroupState = "waiting" | "copying" | "checking" | "copied" | "needs-action" | "failed";

/** One group's measured counts (the same separate counts as the whole move). */
export type MoveGroup = { id: string; name: GroupName; counts: Counts };

/**
 * A group's state, derived from its measured counts so it cannot be told otherwise: while anything
 * in it is waiting, copying or checking it is not "copied"; when nothing is unfinished, failed
 * comes before needs-action, which comes before copied.
 */
export function groupState(c: Counts): GroupState {
  if (c.checking > 0) return "checking";
  if (c.copying > 0) return "copying";
  if (c.waiting > 0) return c.done + c.needsAction + c.failed > 0 ? "copying" : "waiting";
  if (c.failed > 0) return "failed";
  if (c.needsAction > 0) return "needs-action";
  return "copied";
}

export type MoveProgress = {
  state: RunState;
  /** The old laptop's label as pairing showed it (the new laptop has none of its own); null when not known. */
  /** The two laptops by their names; `new` is filled by the screens from the greeting, when known. */
  laptops: { old: Label | null; new?: Label | null };
  /** The groups of the plan, each with its own measured counts. */
  groups: MoveGroup[];
  counts: Counts;
  /** Bytes written and read back so far, and bytes in the approved plan. Never estimated. */
  bytes: { checked: number; total: number };
  /** The item being worked on, when one is. */
  current?: { name: string; step: "copying" | "checking" };
  /** Measured, as a range in bytes a second; absent until there is enough to measure. */
  speed?: Range;
  /** Whole minutes left as a range; absent until it can be measured. Never a promise. */
  minutesLeft?: Range;
  /**
   * Something the person may need to act on: this laptop is low on battery (`paused`: at the safe
   * limit, so the move was paused for it), the connection dropped, or a drive is not keeping files.
   */
  notice?: { kind: "low-battery"; paused: boolean } | { kind: "may-sleep" } | { kind: "link-lost" } | { kind: "drive-problem"; place: GroupName };
};

/** A count of zero for every status, to fill in. */
export function noStatuses(): Record<ItemStatus, number> {
  return { copied: 0, reinstalled: 0, converted: 0, "needs-action": 0, "skipped-by-user": 0, failed: 0 };
}

/** Items that are still waiting, being copied or being checked. */
export function unfinished(c: Counts): number {
  return c.waiting + c.copying + c.checking;
}

/** Items that ended some way other than done: they stay visible everywhere. */
export function notDone(c: Counts): number {
  return c.needsAction + c.failed;
}

/**
 * The percent to show, from bytes written and checked, rounded down. It is never 100 while anything
 * is waiting, copying or checking, and it is `null` (no number at all) when the plan has no size.
 */
export function measuredPercent(p: Pick<MoveProgress, "bytes" | "counts">): number | null {
  const { checked, total } = p.bytes;
  if (!(total > 0)) return null;
  const pct = Math.floor((Math.max(0, checked) / total) * 100);
  if (unfinished(p.counts) > 0) return Math.min(pct, 99);
  return Math.min(pct, 100);
}

/** The write step each phase shows, as the journal names them (crates/pctwin-journal State). */
export type Phase = "waiting" | "copying" | "checking" | "putting-in-place" | "already-there";
export const PHASE_STEP: Record<Phase, string> = {
  waiting: "planned",
  copying: "staged",
  checking: "staged",
  "putting-in-place": "verified or applied",
  "already-there": "existing",
};

/** What R01 shows when a move ends. `stopped` means the person stopped it early. */
/** Another person in the move: their things, counted (never named file by file). `atSignIn`: handed over at their next sign-in (M32). */
export type PersonOutcome = { name: string; copied: number; needsAction: number; failed: number; atSignIn: boolean };

export type MoveOutcome = {
  /** The other people in the move, one line each (absent in the screens' own samples). */
  people?: PersonOutcome[];
  /** Items by how they ended: one count for each of the engine's six statuses. */
  byStatus: Record<ItemStatus, number>;
  /** Items that never started or never finished because the move was stopped. Zero unless `stopped`. */
  notCopied: number;
  /** Bytes written and read back. */
  bytesChecked: number;
  stopped: boolean;
  /** It ended early because something went wrong between the laptops, not because a person stopped it (absent in the screens' own samples). */
  brokeOff?: boolean;
  /** Copied files checked from this laptop's memory, not the drive. */
  checkedFromMemory: number;
  /** The after-move check (absent in the screens' own samples that predate it). */
  afterMove?: AfterMove;
  /** The full move's apps, settings and special categories, each with its result. */
  rebuilt?: Rebuilt;
  /** The new laptop: where each group of the person's things is now (the folder as the person reads it). */
  landed?: { name: GroupName; folder: string }[];
};

/** One kind of file after the move: how many the plan held, and how many are copied and checked. */
export type KindCount = { planned: number; copied: number };

/** The kinds the after-move check counts, in the order it lists them (as the scan counts them). */
export const KINDS = ["photos", "videos", "music", "documents"] as const;
export type Kind = (typeof KINDS)[number];

/** The mover's photos, videos, music and documents in the plan, each with how many arrived checked. */
export type AfterMove = Record<Kind, KindCount>;

/* ---------- Formatting ---------- */

const UNITS = ["byte", "kilobyte", "megabyte", "gigabyte", "terabyte"] as const;

/** A size in the reader's language: 1.2 GB. Decimal units, one decimal at most. */
export function formatBytes(bytes: number, locale: string): string {
  let n = Math.max(0, bytes);
  let u = 0;
  // Under a kilobyte reads as a share of one ("0.3 kB", "0 kB"), never "0 byte".
  if (n < 1000) {
    n /= 1000;
    u = 1;
  }
  while (n >= 1000 && u < UNITS.length - 1) {
    n /= 1000;
    u++;
  }
  return new Intl.NumberFormat(locale, { style: "unit", unit: UNITS[u], unitDisplay: "short", maximumFractionDigits: u === 0 ? 0 : 1 }).format(n);
}

/** A speed range's two ends in one unit, so the unit is said once: "40" "55" and "MB/s". */
export function formatSpeed(r: Range, locale: string): { low: string; high: string; unit: string } {
  let u = 0;
  let n = Math.max(r.low, r.high);
  while (n >= 1000 && u < UNITS.length - 1) {
    n /= 1000;
    u++;
  }
  const f = new Intl.NumberFormat(locale, { maximumFractionDigits: u === 0 ? 0 : 1 });
  const unit = ["B/s", "kB/s", "MB/s", "GB/s", "TB/s"][u];
  return { low: f.format(r.low / 1000 ** u), high: f.format(r.high / 1000 ** u), unit };
}

/**
 * A measured range of minutes, rounded to whole minutes and otherwise as measured: never widened.
 * When both ends round to the same number the screens say that one number ("about 7 minutes").
 */
export function minutesRange(r: Range): { low: number; high: number } {
  return { low: Math.max(0, Math.round(r.low)), high: Math.max(0, Math.round(r.high)) };
}

/** The language the page is in (set by the app), for number formats. */
export function pageLocale(): string {
  return typeof document === "undefined" ? "en" : document.documentElement.lang || "en";
}
