/**
 * The full move's apps, settings and special categories, as the engine sends them
 * (src-tauri/src/fullmove.rs): the plan section of the approval screen (M23), the progress after the
 * files, and the outcome section (R01). Every value is a word key, a count, or a name shown exactly as
 * it is. The parsers are strict: an unknown key or value is refused, never guessed.
 */
import { CHANGE_ACTIONS, UNDO_PROMISES, type ChangeAction, type UndoPromise } from "./guard";
import { ITEM_STATUSES, parseResult, type ItemStatus, type Reason } from "./move";

/** PCTwin's own lines, by key. */
export const OWN_TITLES = ["fullmove-special-not-yet", "fullmove-special-not-checked", "fullmove-extras-refused", "fullmove-left-by-person"] as const;
export type Title = { kind: "name"; value: string } | { kind: "key"; value: (typeof OWN_TITLES)[number] };

export const CHANGE_KINDS = ["app", "settings", "special"] as const;
export type ChangeKind = (typeof CHANGE_KINDS)[number];

export const SPECIAL_STEPS = ["copy", "export-first", "sign-in", "relink", "not-yet", "look-at-it"] as const;
export type SpecialStep = (typeof SPECIAL_STEPS)[number];

export const NOT_MOVED_GROUPS = ["private", "could-run", "old-laptop-only", "system-own", "not-read"] as const;
export type NotMovedGroup = (typeof NOT_MOVED_GROUPS)[number];

export type ChangeView = {
  title: Title;
  kind: ChangeKind;
  action: ChangeAction;
  undo: UndoPromise;
  count: number;
  needs: Reason | null;
  special: SpecialStep | null;
  files: number;
  /** A secret the person may tick to bring: its line (none moves unless ticked). */
  secret: number | null;
  /** An older version is on this laptop and PCTwin can update it if ticked: the app's place in the plan. */
  update: number | null;
  /** Where PCTwin gets an app it installs: the package's own ID in its catalog. */
  source: string | null;
  /** What else the person should know: an older version already here, or saved sign-ins to websites. */
  note: ChangeNote | null;
};

export type ChangeNote = { note: "older-here"; version: string } | { note: "sites" };

/** Why an app's data needs the person (`DataWhy`). */
export const DATA_WHYS = ["lost", "did-not-arrive", "app-open", "changing", "unreadable", "not-read", "not-whole", "not-landed", "beside", "placed"] as const;
export type DataWhy = (typeof DATA_WHYS)[number];

/** One thing to know or do for an outcome line (`LineNote`). */
export type LineNote =
  | { note: "sign-in" }
  | { note: "licence-key" }
  | { note: "registration"; owner: string | null; company: string | null; productId: string | null }
  | { note: "extras"; apps: string[] }
  | { note: "sites" }
  | { note: "data"; why: DataWhy; folder?: string };

/** An app already on this laptop, left as it is (`HereLine`). */
export type HereLine = { name: string; older: string | null };

export type RebuildPlan = {
  changes: ChangeView[];
  more: number;
  notMoved: { group: NotMovedGroup; count: number }[];
  specialNotMoved: number;
  /** Shortcuts (links and junctions) among the person's files: never followed, never moved. */
  links: number;
  prompt: boolean;
  restorePoint: boolean;
};

export type RebuiltLine = { title: Title; kind: ChangeKind; count: number; status: ItemStatus; reason: Reason | null; notes: LineNote[] };

export const RESTORE_WORDS = ["made", "earlier-exists", "not-made", "not-windows", "not-asked"] as const;
export type RestoreWord = (typeof RESTORE_WORDS)[number];

export type Rebuilt = {
  lines: RebuiltLine[];
  more: number;
  /** Apps already on this laptop, left as they are: not items of the move, so outside the result words. */
  here: HereLine[];
  hereMore: number;
  /** PCTwin installed at least one app (fresh installs start signed out). */
  fresh: boolean;
  restorePoint: RestoreWord;
  stopped: boolean;
  canUndo: boolean;
  /** The move this outcome's undo book belongs to. */
  undoId: string | null;
};

export type RebuildStep = { step: "permission" } | { step: "apps" | "settings"; done: number; total: number };

type Fields = Record<string, unknown>;

function exactly(value: unknown, keys: readonly string[]): Fields | null {
  if (typeof value !== "object" || value === null || Array.isArray(value)) return null;
  const f = value as Fields;
  const own = Object.keys(f);
  return own.length === keys.length && keys.every((k) => own.includes(k)) ? f : null;
}

function isCount(v: unknown): v is number {
  return typeof v === "number" && Number.isSafeInteger(v) && v >= 0;
}

function oneOf<T extends string>(v: unknown, list: readonly T[]): v is T {
  return typeof v === "string" && (list as readonly string[]).includes(v);
}

function listOf<T>(v: unknown, parse: (x: unknown) => T | null): T[] | null {
  if (!Array.isArray(v)) return null;
  const out: T[] = [];
  for (const x of v) {
    const p = parse(x);
    if (p === null) return null;
    out.push(p);
  }
  return out;
}

function parseTitle(v: unknown): Title | null {
  const f = exactly(v, ["kind", "value"]);
  if (!f) return null;
  if (f.kind === "name" && typeof f.value === "string") return { kind: "name", value: f.value };
  if (f.kind === "key" && oneOf(f.value, OWN_TITLES)) return { kind: "key", value: f.value };
  return null;
}

function nameOrNull(v: unknown): v is string | null {
  return v === null || typeof v === "string";
}

function parseChangeNote(v: unknown): ChangeNote | null | undefined {
  if (v === null) return null;
  const kind = typeof v === "object" ? (v as Fields).note : undefined;
  if (kind === "sites") return exactly(v, ["note"]) ? { note: kind } : undefined;
  if (kind === "older-here") {
    const f = exactly(v, ["note", "version"]);
    return f && typeof f.version === "string" ? { note: kind, version: f.version } : undefined;
  }
  return undefined;
}

function parseLineNote(v: unknown): LineNote | null {
  const kind = typeof v === "object" && v !== null ? (v as Fields).note : undefined;
  if (kind === "sign-in" || kind === "licence-key" || kind === "sites") return exactly(v, ["note"]) ? { note: kind } : null;
  if (kind === "registration") {
    const f = exactly(v, ["note", "owner", "company", "productId"]);
    return f && nameOrNull(f.owner) && nameOrNull(f.company) && nameOrNull(f.productId) ? { note: kind, owner: f.owner, company: f.company, productId: f.productId } : null;
  }
  if (kind === "extras") {
    const f = exactly(v, ["note", "apps"]);
    const apps = f ? listOf(f.apps, (x) => (typeof x === "string" ? x : null)) : null;
    return apps ? { note: kind, apps } : null;
  }
  if (kind === "data") {
    const why = (v as Fields).why;
    if (why === "beside" || why === "placed") {
      const f = exactly(v, ["note", "why", "folder"]);
      return f && typeof f.folder === "string" ? { note: kind, why, folder: f.folder } : null;
    }
    return oneOf(why, DATA_WHYS) && exactly(v, ["note", "why"]) ? { note: kind, why } : null;
  }
  return null;
}

function parseChange(v: unknown): ChangeView | null {
  const f = exactly(v, ["title", "kind", "action", "undo", "count", "needs", "special", "files", "secret", "update", "source", "note"]);
  if (!f) return null;
  const title = parseTitle(f.title);
  if (!title || !oneOf(f.kind, CHANGE_KINDS) || !oneOf(f.action, CHANGE_ACTIONS) || !oneOf(f.undo, UNDO_PROMISES)) return null;
  if (!isCount(f.count) || !isCount(f.files)) return null;
  if (f.special !== null && !oneOf(f.special, SPECIAL_STEPS)) return null;
  if (f.secret !== null && !isCount(f.secret)) return null;
  if (f.update !== null && !isCount(f.update)) return null;
  if (f.source !== null && typeof f.source !== "string") return null;
  const note = parseChangeNote(f.note);
  if (note === undefined) return null;
  let needs: Reason | null = null;
  if (f.needs !== null) {
    const r = parseResult({ status: "needs-action", reason: f.needs });
    if (!r || !("reason" in r) || r.reason === undefined) return null;
    needs = r.reason;
  }
  return { title, kind: f.kind, action: f.action, undo: f.undo, count: f.count, needs, special: f.special, files: f.files, secret: f.secret, update: f.update, source: f.source, note };
}

/** The plan section (`RebuildPlanView`). */
export function parseRebuildPlan(v: unknown): RebuildPlan | null {
  const f = exactly(v, ["changes", "more", "notMoved", "specialNotMoved", "links", "prompt", "restorePoint"]);
  if (!f || !isCount(f.more) || !isCount(f.specialNotMoved) || !isCount(f.links) || typeof f.prompt !== "boolean" || typeof f.restorePoint !== "boolean") return null;
  const changes = listOf(f.changes, parseChange);
  const notMoved = listOf(f.notMoved, (x) => {
    const n = exactly(x, ["group", "count"]);
    return n && oneOf(n.group, NOT_MOVED_GROUPS) && isCount(n.count) ? { group: n.group, count: n.count } : null;
  });
  return changes && notMoved ? { changes, more: f.more, notMoved, specialNotMoved: f.specialNotMoved, links: f.links, prompt: f.prompt, restorePoint: f.restorePoint } : null;
}

function parseLine(v: unknown): RebuiltLine | null {
  const f = exactly(v, ["title", "kind", "count", "status", "reason", "notes"]);
  if (!f || !oneOf(f.kind, CHANGE_KINDS) || !isCount(f.count) || !oneOf(f.status, ITEM_STATUSES)) return null;
  const title = parseTitle(f.title);
  const result = parseResult(f.reason === null ? { status: f.status } : { status: f.status, reason: f.reason });
  const notes = listOf(f.notes, parseLineNote);
  if (!title || !result || !notes) return null;
  return { title, kind: f.kind, count: f.count, status: result.status, reason: "reason" in result ? (result.reason ?? null) : null, notes };
}

/** The outcome section (`RebuiltView`). */
export function parseRebuilt(v: unknown): Rebuilt | null {
  const f = exactly(v, ["lines", "more", "here", "hereMore", "fresh", "restorePoint", "stopped", "canUndo", "undoId"]);
  if (!f || !isCount(f.more) || !isCount(f.hereMore) || typeof f.fresh !== "boolean" || !oneOf(f.restorePoint, RESTORE_WORDS) || typeof f.stopped !== "boolean" || typeof f.canUndo !== "boolean") return null;
  if (f.undoId !== null && !(typeof f.undoId === "string" && /^[0-9a-f]{32}$/.test(f.undoId))) return null;
  const lines = listOf(f.lines, parseLine);
  const here = listOf(f.here, (x) => {
    const h = exactly(x, ["name", "older"]);
    return h && typeof h.name === "string" && nameOrNull(h.older) ? { name: h.name, older: h.older } : null;
  });
  return lines && here ? { lines, more: f.more, here, hereMore: f.hereMore, fresh: f.fresh, restorePoint: f.restorePoint, stopped: f.stopped, canUndo: f.canUndo, undoId: f.undoId } : null;
}

/** Where rebuilding is (`Progress`, event `move-rebuild`). */
export function parseRebuildStep(v: unknown): RebuildStep | null {
  if (typeof v !== "object" || v === null) return null;
  const step = (v as Fields).step;
  if (step === "permission") return exactly(v, ["step"]) ? { step } : null;
  if (step === "apps" || step === "settings") {
    const f = exactly(v, ["step", "done", "total"]);
    return f && isCount(f.done) && isCount(f.total) && f.done <= f.total ? { step, done: f.done, total: f.total } : null;
  }
  return null;
}

/** The old laptop's progress with the app data (`DataProgress`, event `move-appdata`). */
export type DataProgress = { done: number; total: number; waitingFor: string[] };

export function parseDataProgress(v: unknown): DataProgress | null {
  const f = exactly(v, ["done", "total", "waitingFor"]);
  if (!f || !isCount(f.done) || !isCount(f.total) || f.done > f.total) return null;
  const waitingFor = listOf(f.waitingFor, (x) => (typeof x === "string" ? x : null));
  return waitingFor ? { done: f.done, total: f.total, waitingFor } : null;
}

/** The apps to close on the old laptop before their data can be read (`AppsOpen`, event `move-appdata-open`). */
export type AppsOpen = { apps: string[] };

export function parseAppsOpen(v: unknown): AppsOpen | null {
  const f = exactly(v, ["apps"]);
  const apps = f ? listOf(f.apps, (x) => (typeof x === "string" ? x : null)) : null;
  return apps ? { apps } : null;
}
