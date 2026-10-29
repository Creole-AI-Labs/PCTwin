/**
 * The Phase 2 deliverables on the screens (Task List 2.9), as the engine sends them
 * (src-tauri/src/deliver/*.rs): "Can I actually work?" checks, "Fix these items", changes after
 * approval, the safe point for PCTwin's own update, and the offline recovery kit. Every value is a
 * word key, a count, or a name shown exactly as it is. The parsers are strict: an unknown key or
 * word is refused, never guessed.
 */
import { CHANGE_KINDS, parseTitle, type ChangeKind, type Title } from "./fullmove";
import { ITEM_STATUSES, parseResult, type ItemStatus, type Reason } from "./move";

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

const text = (v: unknown): string | null => (typeof v === "string" ? v : null);

/* ---------- Can I actually work? ---------- */

export const WORK_WHATS = ["linked-files", "font", "archive", "database", "task"] as const;
export type WorkWhat = (typeof WORK_WHATS)[number];
/** Who vouches for a result. Always shown. */
export const WORK_LABELS = ["machine-checked", "person-confirmed", "still-needs-attention"] as const;
export type WorkLabel = (typeof WORK_LABELS)[number];
export const WORK_STATES = ["not-run-yet", "all-there", "missing", "damaged", "not-there", "waiting-for-you", "you-said-no", "outside-project", "could-not-read", "not-supported"] as const;
export type WorkState = (typeof WORK_STATES)[number];

export type WorkCheck = {
  index: number;
  what: WorkWhat;
  name: string;
  label: WorkLabel;
  state: WorkState;
  missing: string[];
  missingMore: number;
  notChecked: string[];
  checked: number;
  of: number;
};

export type WorkChecks = { generation: number; checks: WorkCheck[]; more: number };

function parseWorkCheck(v: unknown): WorkCheck | null {
  const f = exactly(v, ["index", "what", "name", "label", "state", "missing", "missingMore", "notChecked", "checked", "of"]);
  if (!f || !isCount(f.index) || !oneOf(f.what, WORK_WHATS) || typeof f.name !== "string" || !oneOf(f.label, WORK_LABELS) || !oneOf(f.state, WORK_STATES)) return null;
  if (!isCount(f.missingMore) || !isCount(f.checked) || !isCount(f.of)) return null;
  const missing = listOf(f.missing, text);
  const notChecked = listOf(f.notChecked, text);
  return missing && notChecked ? { index: f.index, what: f.what, name: f.name, label: f.label, state: f.state, missing, missingMore: f.missingMore, notChecked, checked: f.checked, of: f.of } : null;
}

export function parseWorkChecks(v: unknown): WorkChecks | null {
  const f = exactly(v, ["generation", "checks", "more"]);
  if (!f || !isCount(f.generation) || !isCount(f.more)) return null;
  const checks = listOf(f.checks, parseWorkCheck);
  return checks ? { generation: f.generation, checks, more: f.more } : null;
}

export const WORK_PROBLEMS = ["not-chosen", "too-many", "no-such-check", "stale", "bad-name", "wrong-kind", "failed"] as const;
export type WorkProblem = (typeof WORK_PROBLEMS)[number];

/** A refused work-check command arrives as the problem's word; anything else is `failed`. */
export function parseWorkProblem(v: unknown): WorkProblem | null {
  return oneOf(v, WORK_PROBLEMS) ? v : null;
}

/** What the person adds a check for (`WorkPick`). */
export type WorkPick = { kind: "file" } | { kind: "font"; family: string } | { kind: "task"; task: string };

/* ---------- Fix these items ---------- */

export const PREREQUISITES = ["old-laptop", "internet", "administrator", "installer-tool", "space", "close-app"] as const;
export type Prerequisite = (typeof PREREQUISITES)[number];

export type RepairLine = {
  line: number;
  title: Title;
  kind: ChangeKind;
  status: ItemStatus;
  reason: Reason | null;
  /** What a retry needs that is not there now; empty means it can be tried now. */
  waitsFor: Prerequisite[];
};

export type RepairOffer = { generation: number; kept: number; lines: RepairLine[]; more: number };
export type RepairDone = RepairOffer & { fixed: number };

function parseRepairLine(v: unknown): RepairLine | null {
  const f = exactly(v, ["line", "title", "kind", "status", "reason", "waitsFor"]);
  if (!f || !isCount(f.line) || !oneOf(f.kind, CHANGE_KINDS) || !oneOf(f.status, ITEM_STATUSES)) return null;
  const title = parseTitle(f.title);
  const result = parseResult(f.reason === null ? { status: f.status } : { status: f.status, reason: f.reason });
  const waitsFor = listOf(f.waitsFor, (x) => (oneOf(x, PREREQUISITES) ? x : null));
  if (!title || !result || !waitsFor) return null;
  return { line: f.line, title, kind: f.kind, status: result.status, reason: "reason" in result ? (result.reason ?? null) : null, waitsFor };
}

export function parseRepairOffer(v: unknown): RepairOffer | null {
  const f = exactly(v, ["generation", "kept", "lines", "more"]);
  if (!f || !isCount(f.generation) || !isCount(f.kept) || !isCount(f.more)) return null;
  const lines = listOf(f.lines, parseRepairLine);
  return lines ? { generation: f.generation, kept: f.kept, lines, more: f.more } : null;
}

export function parseRepairDone(v: unknown): RepairDone | null {
  const f = exactly(v, ["generation", "kept", "lines", "more", "fixed"]);
  if (!f || !isCount(f.fixed)) return null;
  const offer = parseRepairOffer({ generation: f.generation, kept: f.kept, lines: f.lines, more: f.more });
  return offer ? { ...offer, fixed: f.fixed } : null;
}

export const REPAIR_PROBLEMS = ["nothing-to-repair", "stale", "busy", "failed"] as const;
export type RepairProblem = (typeof REPAIR_PROBLEMS)[number];

export function parseRepairProblem(v: unknown): RepairProblem | null {
  return oneOf(v, REPAIR_PROBLEMS) ? v : null;
}

/* ---------- Changes during the move ---------- */

export const CHANGED_WHYS = ["app-updated", "app-removed", "destination-changed", "project-edited"] as const;
export type ChangedWhy = (typeof CHANGED_WHYS)[number];
export const CHANGED_STEPS = ["install", "settings", "app-data", "files"] as const;
export type ChangedStep = (typeof CHANGED_STEPS)[number];

export type ChangedAfter = { name: string; why: ChangedWhy; steps: ChangedStep[] };
/** What changed after approval; `held` is how many steps wait for the person's new yes. */
export type ChangesView = { generation: number; held: number; lines: ChangedAfter[]; more: number };

export function parseChangesView(v: unknown): ChangesView | null {
  const f = exactly(v, ["generation", "held", "lines", "more"]);
  if (!f || !isCount(f.generation) || !isCount(f.held) || !isCount(f.more)) return null;
  const lines = listOf(f.lines, (x) => {
    const l = exactly(x, ["name", "why", "steps"]);
    const steps = l ? listOf(l.steps, (s) => (oneOf(s, CHANGED_STEPS) ? s : null)) : null;
    return l && typeof l.name === "string" && oneOf(l.why, CHANGED_WHYS) && steps ? { name: l.name, why: l.why, steps } : null;
  });
  return lines ? { generation: f.generation, held: f.held, lines, more: f.more } : null;
}

/* ---------- PCTwin's own update waits for a safe point ---------- */

export const BUSY_WITHS = ["moving", "rebuilding", "app-data", "undoing"] as const;
export type BusyWith = (typeof BUSY_WITHS)[number];
export type SafeStopView = { safe: boolean; busy: BusyWith[] };

export function parseSafeStop(v: unknown): SafeStopView | null {
  const f = exactly(v, ["safe", "busy"]);
  if (!f || typeof f.safe !== "boolean") return null;
  const busy = listOf(f.busy, (x) => (oneOf(x, BUSY_WITHS) ? x : null));
  return busy ? { safe: f.safe, busy } : null;
}

/* ---------- The offline recovery kit ---------- */

export const KIT_PARTS = ["move-record", "results", "before-wipe-list", "file-data", "setup-steps", "recovery-tool"] as const;
export type KitPart = (typeof KIT_PARTS)[number];
export type KitSaved = { folder: string; files: string[]; holds: KitPart[]; lacks: KitPart[] };

export function parseKitSaved(v: unknown): KitSaved | null {
  const f = exactly(v, ["folder", "files", "holds", "lacks"]);
  if (!f || typeof f.folder !== "string") return null;
  const files = listOf(f.files, text);
  const part = (x: unknown) => (oneOf(x, KIT_PARTS) ? x : null);
  const holds = listOf(f.holds, part);
  const lacks = listOf(f.lacks, part);
  return files && holds && lacks ? { folder: f.folder, files, holds, lacks } : null;
}
