/**
 * The flow guard, on the screens' side (Task List 2.8): the shapes the engine sends for the Phase 2
 * screens (src-tauri/src/guard.rs and guard_views.rs), read exactly or refused, and the rules for
 * which update may replace what is shown.
 *
 * - An update for another job, an older session or an older version is dropped (a late event
 *   cannot move a newer job). A gap in versions asks the engine for its view again.
 * - Before the engine has answered, nothing is shown as done or idle: "checking" first, and
 *   "status unavailable" when the engine has no job. Nothing is remembered from an earlier run.
 * - Every button sends an intent naming the job, session and plan generation it showed; the engine
 *   checks it again. The same intent is sent once while one is on its way (a double-click).
 *
 * Names (files, apps, drives, people, laptops) are kept as plain text: the screens render them as
 * text only, never as markup or links.
 */

/* ---------- Shapes ---------- */

export const DISPLAY_STATES = [
  "not-started", "preparing", "awaiting-person", "awaiting-system", "awaiting-maker", "working", "checking", "applying",
  "pause-requested", "paused", "source-unavailable", "blocked", "partial", "retrying", "reconciling", "failed", "complete", "cancelled",
] as const;
export type DisplayState = (typeof DISPLAY_STATES)[number];

export const INTENT_NAMES = [
  "approve", "leave-out-changed", "keep-local-copy", "allow-metered", "approve-offload", "choose-drive", "check-again",
  "ask-system", "not-now", "try-again", "preview-undo", "undo", "pause", "resume", "back",
] as const;
export type IntentName = (typeof INTENT_NAMES)[number];

/** What a button asks for (`IntentKind`). */
export type IntentKind =
  | { kind: "approve" | "leave-out-changed" | "allow-metered" | "approve-offload" | "check-again" | "ask-system" | "not-now" | "preview-undo" | "undo" | "pause" | "resume" | "back" }
  | { kind: "keep-local-copy"; keep: boolean }
  | { kind: "choose-drive"; drive: number }
  | { kind: "try-again"; line: number };

export type Measured = { done: number; total: number | null; unit: "bytes" | "items" };

export const CHANGE_ACTIONS = ["install", "merge", "replace", "leave-alone"] as const;
export type ChangeAction = (typeof CHANGE_ACTIONS)[number];
export const UNDO_PROMISES = ["can-reverse", "can-remove-app", "must-keep", "needs-you-service", "needs-you-maker"] as const;
export type UndoPromise = (typeof UNDO_PROMISES)[number];
export type ChangeLine = { name: string; action: ChangeAction; undo: UndoPromise };

export const SPACE_USES = ["files", "app-downloads", "unpacking", "undo-record", "partly-copied"] as const;
export type SpaceUse = (typeof SPACE_USES)[number];
export type DriveBudget = { drive: string; needed: number; free: number | null; fits: boolean | null; shortBy: number; parts: { usedFor: SpaceUse; bytes: number }[] };

export const KEEP_OFF = ["leave-in-cloud", "offload-drive", "leave-on-old-laptop"] as const;
export type KeepOff = (typeof KEEP_OFF)[number];
export type ProposalLine = { to: KeepOff; items: number; bytes: number };
export type OffloadView = { drive: string; free: number | null; approved: boolean; connected: boolean };
export type SafeInLine = { provider: string | null; files: number };

export const LOCAL_COPY_STATES = ["choosing", "working", "paused-metered", "paused-no-space", "done"] as const;
export type LocalCopyState = (typeof LOCAL_COPY_STATES)[number];

export type OwnerRow = { laptop: string; person: string; account: string | null; ownFolder: boolean };

export const CHANGED_WHY = ["changed-on-old-laptop", "app-updated", "destination-changed", "new-scope", "permission-ended"] as const;
export type ChangedWhy = (typeof CHANGED_WHY)[number];
export type ChangedLine = { name: string; why: ChangedWhy };

export const PERMISSION_REASONS = ["read-other-people", "make-accounts", "shared-folders", "install-for-everyone", "restore-point"] as const;
export type PermissionReason = (typeof PERMISSION_REASONS)[number];
export const PERMISSION_STAGES = ["explain", "waiting-for-system", "denied", "closed-without-answer", "given", "ended-by-sleep", "ended-other-session"] as const;
export type PermissionStage = (typeof PERMISSION_STAGES)[number];
export const NOT_MADE_WHY = ["not-windows", "protection-off", "needs-admin", "unconfirmed", "failed"] as const;
export type NotMadeWhy = (typeof NOT_MADE_WHY)[number];
export type RestorePoint = { kind: "made" | "earlier-exists"; at: string } | { kind: "not-chosen" } | { kind: "not-made"; why: NotMadeWhy };

export const LOCKS = ["bit-locker", "file-vault", "apple-chip"] as const;
export type LockKind = (typeof LOCKS)[number];
export const READ_ONLY = ["system-read-only", "pctwin-reads-only"] as const;
export type ReadOnly = (typeof READ_ONLY)[number];
export type RescueDrive = { drive: string; size: number; fileSystem: string; isTarget: boolean };
export type RescueStage =
  | { kind: "intro" }
  | { kind: "pick-drive"; drives: RescueDrive[]; refused: number | null }
  | { kind: "locked"; lock: LockKind }
  | { kind: "no-system" }
  | { kind: "salvage"; readable: number; readOnly: ReadOnly; unreadable: string[]; more: number };

export type LineWhat = { kind: "items" | "settings"; count: number } | { kind: "app"; name: string };
export const LINE_WHY = ["failed", "laptop-not-connected", "waits-for-another", "not-started"] as const;
export type LineWhy = (typeof LINE_WHY)[number];
export type ReportLine = { line: number; what: LineWhat; laptop: string; why: LineWhy };

export const UNDO_PLANS = ["put-back", "remove-app", "keep-changed", "elsewhere", "leave-alone"] as const;
export type UndoPlan = (typeof UNDO_PLANS)[number];
export type UndoPlanLine = { name: string; plan: UndoPlan };
export const UNDO_RESULTS = ["restored", "removed", "as-before", "kept-changed", "kept-other", "kept-at", "done-elsewhere", "not-done-yet"] as const;
export type UndoResultKind = (typeof UNDO_RESULTS)[number];
export type UndoResult = { kind: Exclude<UndoResultKind, "kept-at"> } | { kind: "kept-at"; at: string };
export type UndoLine = { name: string; result: UndoResult; detail: string | null };

export type ScreenView =
  | { kind: "reconciling" }
  | { kind: "changes"; changes: ChangeLine[]; more: number; downloadBytes: number | null; drives: DriveBudget[]; permissions: PermissionReason[]; restorePoint: boolean }
  | { kind: "changed"; lines: ChangedLine[]; more: number }
  | { kind: "space"; drives: DriveBudget[]; proposal: ProposalLine[]; offload: OffloadView | null; safeIn: SafeInLine[] }
  | { kind: "local-copy"; provider: string | null; files: number; downloadBytes: number; oldFreeBytes: number | null; state: LocalCopyState }
  | { kind: "owners"; rows: OwnerRow[]; more: number }
  | { kind: "permission"; stage: PermissionStage; reasons: PermissionReason[]; restorePoint: RestorePoint | null }
  | { kind: "rescue"; stage: RescueStage }
  | { kind: "report"; lines: ReportLine[]; more: number; safeIn: SafeInLine[] }
  | { kind: "undo-preview"; lines: UndoPlanLine[]; more: number }
  | { kind: "undo-done"; lines: UndoLine[]; more: number };

export type Projection = {
  job: string;
  epoch: number;
  generation: number;
  version: number;
  display: DisplayState;
  allowed: IntentName[];
  progress: Measured | null;
  screen: ScreenView;
};

export type Snapshot = { kind: "unavailable" } | { kind: "ready"; view: Projection };

export const REFUSALS = ["unavailable", "wrong-job", "old-session", "plan-changed", "not-now", "not-shown"] as const;
export type Refusal = (typeof REFUSALS)[number];
export type Answer = "taken" | "already-taken";

/* ---------- Parsing: exactly the engine's shape, or null ---------- */

type Fields = Record<string, unknown>;

function exactly(value: unknown, keys: readonly string[]): Fields | null {
  if (typeof value !== "object" || value === null || Array.isArray(value)) return null;
  const fields = value as Fields;
  const have = Object.keys(fields);
  if (have.length !== keys.length || !keys.every((k) => Object.prototype.hasOwnProperty.call(fields, k))) return null;
  return fields;
}

const isCount = (v: unknown): v is number => typeof v === "number" && Number.isSafeInteger(v) && v >= 0;
const isText = (v: unknown): v is string => typeof v === "string";
const isBool = (v: unknown): v is boolean => typeof v === "boolean";
const orNull = <T>(v: unknown, is: (x: unknown) => x is T): v is T | null => v === null || is(v);
function oneOf<T extends string>(list: readonly T[]) {
  return (v: unknown): v is T => typeof v === "string" && (list as readonly string[]).includes(v);
}
/** A `{ kind }` tag with nothing else, from `list`. */
function tag<T extends string>(v: unknown, list: readonly T[]): T | null {
  const f = exactly(v, ["kind"]);
  return f && oneOf(list)(f.kind) ? f.kind : null;
}
/** A plain word from `list` (an enum with no fields), or null. */
function word<T extends string>(v: unknown, list: readonly T[]): T | null {
  return oneOf(list)(v) ? v : null;
}

/** Every element parsed, or null. */
function all<T>(v: unknown, parse: (x: unknown) => T | null): T[] | null {
  if (!Array.isArray(v)) return null;
  const out: T[] = [];
  for (const x of v) {
    const p = parse(x);
    if (p === null) return null;
    out.push(p);
  }
  return out;
}

function parseChange(v: unknown): ChangeLine | null {
  const f = exactly(v, ["name", "action", "undo"]);
  if (!f || !isText(f.name)) return null;
  const action = word(f.action, CHANGE_ACTIONS);
  const undo = word(f.undo, UNDO_PROMISES);
  return action && undo ? { name: f.name, action, undo } : null;
}

function parseBudget(v: unknown): DriveBudget | null {
  const f = exactly(v, ["drive", "needed", "free", "fits", "shortBy", "parts"]);
  if (!f || !isText(f.drive) || !isCount(f.needed) || !orNull(f.free, isCount) || !orNull(f.fits, isBool) || !isCount(f.shortBy)) return null;
  const parts = all(f.parts, (p) => {
    const g = exactly(p, ["usedFor", "bytes"]);
    const usedFor = g ? word(g.usedFor, SPACE_USES) : null;
    return g && usedFor && isCount(g.bytes) ? { usedFor, bytes: g.bytes } : null;
  });
  return parts ? { drive: f.drive, needed: f.needed, free: f.free, fits: f.fits, shortBy: f.shortBy, parts } : null;
}

function parseSafeIn(v: unknown): SafeInLine | null {
  const f = exactly(v, ["provider", "files"]);
  return f && orNull(f.provider, isText) && isCount(f.files) ? { provider: f.provider, files: f.files } : null;
}

function parseRestorePoint(v: unknown): RestorePoint | null {
  const kind = (v as Fields | null)?.kind;
  if (kind === "made" || kind === "earlier-exists") {
    const f = exactly(v, ["kind", "at"]);
    return f && isText(f.at) ? { kind, at: f.at } : null;
  }
  if (kind === "not-chosen") return exactly(v, ["kind"]) ? { kind } : null;
  if (kind === "not-made") {
    const f = exactly(v, ["kind", "why"]);
    const why = f ? word(f.why, NOT_MADE_WHY) : null;
    return why ? { kind, why } : null;
  }
  return null;
}

function parseRescueDrive(v: unknown): RescueDrive | null {
  const f = exactly(v, ["drive", "size", "fileSystem", "isTarget"]);
  return f && isText(f.drive) && isCount(f.size) && isText(f.fileSystem) && isBool(f.isTarget) ? { drive: f.drive, size: f.size, fileSystem: f.fileSystem, isTarget: f.isTarget } : null;
}

function parseRescueStage(v: unknown): RescueStage | null {
  const kind = (v as Fields | null)?.kind;
  if (kind === "intro" || kind === "no-system") return exactly(v, ["kind"]) ? { kind } : null;
  if (kind === "pick-drive") {
    const f = exactly(v, ["kind", "drives", "refused"]);
    const drives = f ? all(f.drives, parseRescueDrive) : null;
    return f && drives && orNull(f.refused, isCount) ? { kind, drives, refused: f.refused } : null;
  }
  if (kind === "locked") {
    const f = exactly(v, ["kind", "lock"]);
    const lock = f ? word(f.lock, LOCKS) : null;
    return lock ? { kind, lock } : null;
  }
  if (kind === "salvage") {
    const f = exactly(v, ["kind", "readable", "readOnly", "unreadable", "more"]);
    const readOnly = f ? word(f.readOnly, READ_ONLY) : null;
    const unreadable = f ? all(f.unreadable, (x) => (isText(x) ? x : null)) : null;
    return f && readOnly && unreadable && isCount(f.readable) && isCount(f.more) ? { kind, readable: f.readable, readOnly, unreadable, more: f.more } : null;
  }
  return null;
}

function parseLineWhat(v: unknown): LineWhat | null {
  const kind = (v as Fields | null)?.kind;
  if (kind === "items" || kind === "settings") {
    const f = exactly(v, ["kind", "count"]);
    return f && isCount(f.count) ? { kind, count: f.count } : null;
  }
  if (kind === "app") {
    const f = exactly(v, ["kind", "name"]);
    return f && isText(f.name) ? { kind, name: f.name } : null;
  }
  return null;
}

function parseUndoResult(v: unknown): UndoResult | null {
  const kind = (v as Fields | null)?.kind;
  if (kind === "kept-at") {
    const f = exactly(v, ["kind", "at"]);
    return f && isText(f.at) ? { kind, at: f.at } : null;
  }
  const t = tag(v, UNDO_RESULTS);
  return t && t !== "kept-at" ? { kind: t } : null;
}

/** One screen's content (`ScreenView`), or null. */
export function parseScreenView(v: unknown): ScreenView | null {
  const kind = (v as Fields | null)?.kind;
  switch (kind) {
    case "reconciling":
      return exactly(v, ["kind"]) ? { kind } : null;
    case "changes": {
      const f = exactly(v, ["kind", "changes", "more", "downloadBytes", "drives", "permissions", "restorePoint"]);
      if (!f) return null;
      const changes = all(f.changes, parseChange);
      const drives = all(f.drives, parseBudget);
      const permissions = all(f.permissions, (x) => word(x, PERMISSION_REASONS));
      return changes && drives && permissions && isCount(f.more) && orNull(f.downloadBytes, isCount) && isBool(f.restorePoint)
        ? { kind, changes, more: f.more, downloadBytes: f.downloadBytes, drives, permissions, restorePoint: f.restorePoint }
        : null;
    }
    case "changed": {
      const f = exactly(v, ["kind", "lines", "more"]);
      const lines = f
        ? all(f.lines, (x) => {
            const g = exactly(x, ["name", "why"]);
            const why = g ? word(g.why, CHANGED_WHY) : null;
            return g && why && isText(g.name) ? { name: g.name, why } : null;
          })
        : null;
      return f && lines && isCount(f.more) ? { kind, lines, more: f.more } : null;
    }
    case "space": {
      const f = exactly(v, ["kind", "drives", "proposal", "offload", "safeIn"]);
      if (!f) return null;
      const drives = all(f.drives, parseBudget);
      const proposal = all(f.proposal, (x) => {
        const g = exactly(x, ["to", "items", "bytes"]);
        const to = g ? word(g.to, KEEP_OFF) : null;
        return g && to && isCount(g.items) && isCount(g.bytes) ? { to, items: g.items, bytes: g.bytes } : null;
      });
      const safeIn = all(f.safeIn, parseSafeIn);
      let offload: OffloadView | null = null;
      if (f.offload !== null) {
        const g = exactly(f.offload, ["drive", "free", "approved", "connected"]);
        if (!g || !isText(g.drive) || !orNull(g.free, isCount) || !isBool(g.approved) || !isBool(g.connected)) return null;
        offload = { drive: g.drive, free: g.free, approved: g.approved, connected: g.connected };
      }
      return drives && proposal && safeIn ? { kind, drives, proposal, offload, safeIn } : null;
    }
    case "local-copy": {
      const f = exactly(v, ["kind", "provider", "files", "downloadBytes", "oldFreeBytes", "state"]);
      const state = f ? word(f.state, LOCAL_COPY_STATES) : null;
      return f && state && orNull(f.provider, isText) && isCount(f.files) && isCount(f.downloadBytes) && orNull(f.oldFreeBytes, isCount)
        ? { kind, provider: f.provider, files: f.files, downloadBytes: f.downloadBytes, oldFreeBytes: f.oldFreeBytes, state }
        : null;
    }
    case "owners": {
      const f = exactly(v, ["kind", "rows", "more"]);
      const rows = f
        ? all(f.rows, (x) => {
            const g = exactly(x, ["laptop", "person", "account", "ownFolder"]);
            return g && isText(g.laptop) && isText(g.person) && orNull(g.account, isText) && isBool(g.ownFolder) ? { laptop: g.laptop, person: g.person, account: g.account, ownFolder: g.ownFolder } : null;
          })
        : null;
      return f && rows && isCount(f.more) ? { kind, rows, more: f.more } : null;
    }
    case "permission": {
      const f = exactly(v, ["kind", "stage", "reasons", "restorePoint"]);
      if (!f) return null;
      const stage = word(f.stage, PERMISSION_STAGES);
      const reasons = all(f.reasons, (x) => word(x, PERMISSION_REASONS));
      const restorePoint = f.restorePoint === null ? null : parseRestorePoint(f.restorePoint);
      if (f.restorePoint !== null && restorePoint === null) return null;
      return stage && reasons ? { kind, stage, reasons, restorePoint } : null;
    }
    case "rescue": {
      const f = exactly(v, ["kind", "stage"]);
      const stage = f ? parseRescueStage(f.stage) : null;
      return stage ? { kind, stage } : null;
    }
    case "report": {
      const f = exactly(v, ["kind", "lines", "more", "safeIn"]);
      if (!f) return null;
      const lines = all(f.lines, (x) => {
        const g = exactly(x, ["line", "what", "laptop", "why"]);
        const what = g ? parseLineWhat(g.what) : null;
        const why = g ? word(g.why, LINE_WHY) : null;
        return g && what && why && isCount(g.line) && isText(g.laptop) ? { line: g.line, what, laptop: g.laptop, why } : null;
      });
      const safeIn = all(f.safeIn, parseSafeIn);
      return lines && safeIn && isCount(f.more) ? { kind, lines, more: f.more, safeIn } : null;
    }
    case "undo-preview": {
      const f = exactly(v, ["kind", "lines", "more"]);
      const lines = f
        ? all(f.lines, (x) => {
            const g = exactly(x, ["name", "plan"]);
            const plan = g ? word(g.plan, UNDO_PLANS) : null;
            return g && plan && isText(g.name) ? { name: g.name, plan } : null;
          })
        : null;
      return f && lines && isCount(f.more) ? { kind, lines, more: f.more } : null;
    }
    case "undo-done": {
      const f = exactly(v, ["kind", "lines", "more"]);
      const lines = f
        ? all(f.lines, (x) => {
            const g = exactly(x, ["name", "result", "detail"]);
            const result = g ? parseUndoResult(g.result) : null;
            return g && result && isText(g.name) && orNull(g.detail, isText) ? { name: g.name, result, detail: g.detail } : null;
          })
        : null;
      return f && lines && isCount(f.more) ? { kind, lines, more: f.more } : null;
    }
    default:
      return null;
  }
}

function parseMeasured(v: unknown): Measured | null {
  const f = exactly(v, ["done", "total", "unit"]);
  return f && isCount(f.done) && orNull(f.total, isCount) && (f.unit === "bytes" || f.unit === "items") ? { done: f.done, total: f.total, unit: f.unit } : null;
}

/** What the screens are shown of one job (`Projection`), or null. */
export function parseProjection(v: unknown): Projection | null {
  const f = exactly(v, ["job", "epoch", "generation", "version", "display", "allowed", "progress", "screen"]);
  if (!f || !isText(f.job) || !isCount(f.epoch) || !isCount(f.generation) || !isCount(f.version)) return null;
  const display = word(f.display, DISPLAY_STATES);
  const allowed = all(f.allowed, (x) => (oneOf(INTENT_NAMES)(x) ? x : null));
  const progress = f.progress === null ? null : parseMeasured(f.progress);
  if (f.progress !== null && progress === null) return null;
  const screen = parseScreenView(f.screen);
  return display && allowed && screen ? { job: f.job, epoch: f.epoch, generation: f.generation, version: f.version, display, allowed, progress, screen } : null;
}

/** The engine's answer to "what is shown now" (`Snapshot`), or null. */
export function parseSnapshot(v: unknown): Snapshot | null {
  const kind = (v as Fields | null)?.kind;
  if (kind === "unavailable") return exactly(v, ["kind"]) ? { kind } : null;
  if (kind === "ready") {
    const f = exactly(v, ["kind", "view"]);
    const view = f ? parseProjection(f.view) : null;
    return view ? { kind, view } : null;
  }
  return null;
}

/** Why an intent was refused (`Refused`), or null. */
export function parseRefusal(v: unknown): Refusal | null {
  return tag(v, REFUSALS);
}

/** What came of an intent (`Answer`), or null. */
export function parseAnswer(v: unknown): Answer | null {
  return tag(v, ["taken", "already-taken"] as const);
}

/* ---------- Which update may replace what is shown ---------- */

/** What the screens hold: nothing yet (checking), the engine has no job, or a job's view. */
export type Shown = { kind: "checking" } | { kind: "unavailable" } | { kind: "ready"; view: Projection };

export type Arrival = { shown: Shown; askAgain: boolean };

/**
 * Whether an event replaces what is shown. Dropped: another job while one is shown, an older
 * session, an older or equal version. A version that skips some asks the engine again (an update
 * may have been missed), and is still shown, since it is the newest the engine sent.
 */
export function arrive(shown: Shown, event: Projection): Arrival {
  if (shown.kind !== "ready") {
    // Before the engine's own answer, an event is not trusted to be the whole truth: ask again.
    return { shown: { kind: "ready", view: event }, askAgain: shown.kind === "checking" };
  }
  const now = shown.view;
  if (event.job !== now.job) {
    // A new job begins with a new session; anything else is a stray event.
    return event.epoch > now.epoch ? { shown: { kind: "ready", view: event }, askAgain: true } : { shown, askAgain: false };
  }
  if (event.epoch < now.epoch) return { shown, askAgain: false };
  if (event.epoch === now.epoch && event.version <= now.version) return { shown, askAgain: false };
  const gap = event.epoch === now.epoch && event.version > now.version + 1;
  return { shown: { kind: "ready", view: event }, askAgain: gap || event.epoch > now.epoch };
}

/** What the engine's snapshot makes the screens show. */
export function fromSnapshot(s: Snapshot): Shown {
  return s.kind === "ready" ? { kind: "ready", view: s.view } : { kind: "unavailable" };
}

/** The intent a button sends, bound to what was shown. */
export function bind(view: Projection, intent: IntentKind) {
  return { job: view.job, epoch: view.epoch, generation: view.generation, intent };
}

/** One key per intent and generation: the same key while one is on its way is not sent again. */
export function intentKey(view: Projection, intent: IntentKind): string {
  return `${view.job}:${String(view.epoch)}:${String(view.generation)}:${JSON.stringify(intent)}`;
}

export function allows(view: Projection, name: IntentName): boolean {
  return view.allowed.includes(name);
}

/* ---------- Announcements: bounded, never every byte ---------- */

/** The quarter marks announced, and the least time between two progress announcements. */
export const MARKS = [25, 50, 75, 100] as const;
export const QUIET_MS = 30_000;

export type Spoken = { mark: number; at: number };

/**
 * Whether measured progress is worth announcing now: only when it passes a new quarter mark, and
 * not sooner than `QUIET_MS` after the last one (except reaching 100). Unknown totals are never
 * announced as numbers.
 */
export function shouldAnnounce(progress: Measured | null, last: Spoken, now: number): { say: boolean; mark: number } {
  if (!progress || progress.total === null || progress.total === 0) return { say: false, mark: last.mark };
  const pct = Math.min(100, Math.floor((progress.done / progress.total) * 100));
  const mark = [...MARKS].reverse().find((m) => pct >= m) ?? 0;
  if (mark <= last.mark) return { say: false, mark: last.mark };
  if (mark < 100 && now - last.at < QUIET_MS) return { say: false, mark: last.mark };
  return { say: true, mark };
}
