/**
 * The bridge: the only file that talks to the desktop shell (src-tauri). Every command is a typed
 * function here, and everything the shell sends back (a command's reply, a rejection, an event) is
 * read by a parse function that accepts exactly the engine's shape and refuses anything else. The
 * screens get parsed values only; a value that does not parse becomes the screens' usual failure
 * (the generic "try again" problem), never shown as it came.
 *
 * The shapes are the engine's (src-tauri/src/pairing.rs and moving.rs, crates/pctwin-gate read_back.rs). The
 * engine writes one example of each into src/contract/*.json, and contract.test.ts parses every
 * example with these functions, so a change on one side that the other does not follow fails.
 *
 * What the shell lets the screens call is a closed list (src-tauri/build.rs and
 * capabilities/default.json); a command not on it is refused by the shell.
 */
import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { ITEM_STATUSES, WHY_MEMORY, parseResult, type Counts, type Evidence, type GroupName, type MoveGroup, type MoveOutcome, type MoveProgress, type Range, type ReportItem, type WhyMemory } from "./move";
import { FOLDER_ROLES, LEFT_OUT_REASONS, type FolderRole, type LeftOut, type PlanGroup, type PlanPreview, type PlanView, type Size } from "./plan";

/* ---------- Shapes ---------- */

export type Label = { kind: "picked"; colour: string; animal: string } | { kind: "named"; name: string };

export type OldEvent =
  | { kind: "label"; label: Label }
  | { kind: "code"; code: string; secondsLeft: number }
  | { kind: "wait"; secondsLeft: number }
  | { kind: "locked" }
  | { kind: "choose"; round: number; numbers: number[] }
  | { kind: "paired" }
  | { kind: "timed-out" }
  | { kind: "left" }
  /** The paired connection died before the move began: pairing is over, start again. */
  | { kind: "lost" };

export type Problem = { key: string; networks: string[] };

export type NewEvent = { kind: "paired" } | { kind: "problem"; problem: Problem } | { kind: "lost" };

export type Nearby = { index: number; label: Label; sameLabelNearby: boolean; needsUpdate: boolean };

/** A note shown before pairing starts: a question this system is about to ask (`HeadsUp`). */
export type HeadsUp = { kind: "local-network" } | { kind: "firewall" };

/** Which laptop the screens are about to start pairing as. */
export type PairRole = "old" | "new";

/** What anything unexpected becomes: the generic "something went wrong, try again" text. */
export const FAILED: Problem = { key: "pairing-failed", networks: [] };

/* ---------- Parsing: exactly the engine's shape, or null ---------- */

type Fields = Record<string, unknown>;

/** An object with exactly these keys (no more, no fewer), or null. */
function exactly(value: unknown, keys: readonly string[]): Fields | null {
  if (typeof value !== "object" || value === null || Array.isArray(value)) return null;
  const fields = value as Fields;
  const have = Object.keys(fields);
  if (have.length !== keys.length || !keys.every((k) => Object.prototype.hasOwnProperty.call(fields, k))) return null;
  return fields;
}

/** A whole number from 0 to `max` (a Rust unsigned integer that fits). */
function isCount(value: unknown, max = Number.MAX_SAFE_INTEGER): value is number {
  return typeof value === "number" && Number.isSafeInteger(value) && value >= 0 && value <= max;
}

/** A translation key or a word key: lower-case words joined by hyphens. */
function isKey(value: unknown): value is string {
  return typeof value === "string" && /^[a-z0-9]+(-[a-z0-9]+)*$/.test(value);
}

function isStringList(value: unknown): value is string[] {
  return Array.isArray(value) && value.every((v) => typeof v === "string");
}

/** A label (`LabelView`): word keys to translate, or the typed name. */
export function parseLabelView(value: unknown): Label | null {
  const tagged = typeof value === "object" && value !== null ? (value as Fields).kind : undefined;
  if (tagged === "picked") {
    const f = exactly(value, ["kind", "colour", "animal"]);
    return f && isKey(f.colour) && isKey(f.animal) ? { kind: "picked", colour: f.colour, animal: f.animal } : null;
  }
  if (tagged === "named") {
    const f = exactly(value, ["kind", "name"]);
    return f && typeof f.name === "string" ? { kind: "named", name: f.name } : null;
  }
  return null;
}

/** A problem (`ProblemView`), as a rejected command or a `problem` event carries it. */
export function parseProblemView(value: unknown): Problem | null {
  const f = exactly(value, ["key", "networks"]);
  return f && isKey(f.key) && isStringList(f.networks) ? { key: f.key, networks: [...f.networks] } : null;
}

/** An update for the old laptop's screen (`OldView`, event `pairing-old`). */
export function parseOldView(value: unknown): OldEvent | null {
  const kind = typeof value === "object" && value !== null ? (value as Fields).kind : undefined;
  switch (kind) {
    case "label": {
      const f = exactly(value, ["kind", "label"]);
      const label = f && parseLabelView(f.label);
      return label ? { kind, label } : null;
    }
    case "code": {
      const f = exactly(value, ["kind", "code", "secondsLeft"]);
      return f && typeof f.code === "string" && isCount(f.secondsLeft) ? { kind, code: f.code, secondsLeft: f.secondsLeft } : null;
    }
    case "wait": {
      const f = exactly(value, ["kind", "secondsLeft"]);
      return f && isCount(f.secondsLeft) ? { kind, secondsLeft: f.secondsLeft } : null;
    }
    case "choose": {
      const f = exactly(value, ["kind", "round", "numbers"]);
      if (!f || !isCount(f.round) || !Array.isArray(f.numbers) || f.numbers.length !== 3) return null;
      const numbers: unknown[] = f.numbers;
      return numbers.every((n) => isCount(n, 255)) ? { kind, round: f.round, numbers: [...numbers] } : null;
    }
    case "locked":
    case "paired":
    case "timed-out":
    case "left":
    case "lost":
      return exactly(value, ["kind"]) ? { kind } : null;
    default:
      return null;
  }
}

/** What the new laptop's screen hears after its number is shown (`NewView`, event `pairing-new`). */
export function parseNewView(value: unknown): NewEvent | null {
  const kind = typeof value === "object" && value !== null ? (value as Fields).kind : undefined;
  switch (kind) {
    case "problem": {
      const f = exactly(value, ["kind", "problem"]);
      const problem = f && parseProblemView(f.problem);
      return problem ? { kind, problem } : null;
    }
    case "paired":
    case "lost":
      return exactly(value, ["kind"]) ? { kind } : null;
    default:
      return null;
  }
}

/** An old laptop nearby (`NearbyView`). */
export function parseNearbyView(value: unknown): Nearby | null {
  const f = exactly(value, ["index", "label", "sameLabelNearby", "needsUpdate"]);
  if (!f || !isCount(f.index) || typeof f.sameLabelNearby !== "boolean" || typeof f.needsUpdate !== "boolean") return null;
  const label = parseLabelView(f.label);
  return label ? { index: f.index, label, sameLabelNearby: f.sameLabelNearby, needsUpdate: f.needsUpdate } : null;
}

/** The search's reply: every entry must parse, or none is used. */
export function parseNearbyList(value: unknown): Nearby[] | null {
  if (!Array.isArray(value)) return null;
  const list = (value as unknown[]).map(parseNearbyView);
  return list.every((n): n is Nearby => n !== null) ? list : null;
}

/** One note to show before pairing starts (`HeadsUp`). */
export function parseHeadsUp(value: unknown): HeadsUp | null {
  const kind = typeof value === "object" && value !== null ? (value as Fields).kind : undefined;
  return (kind === "local-network" || kind === "firewall") && exactly(value, ["kind"]) ? { kind } : null;
}

/** The notes' reply: every entry must parse, or none is used. */
export function parseHeadsUpList(value: unknown): HeadsUp[] | null {
  if (!Array.isArray(value)) return null;
  const list = (value as unknown[]).map(parseHeadsUp);
  return list.every((n): n is HeadsUp => n !== null) ? list : null;
}

/** The number the new laptop shows (a Rust `u8`). */
export function parseShownNumber(value: unknown): number | null {
  return isCount(value, 255) ? value : null;
}

/** How a copied file was checked (the gate's `CheckedFrom`): from the drive, or from memory and why. */
export function parseEvidence(value: unknown): Evidence | null {
  const from = typeof value === "object" && value !== null ? (value as Fields).from : undefined;
  if (from === "drive") return exactly(value, ["from"]) ? { from } : null;
  if (from === "memory") {
    const f = exactly(value, ["from", "why"]);
    return f && typeof f.why === "string" && (WHY_MEMORY as readonly string[]).includes(f.why) ? { from, why: f.why as WhyMemory } : null;
  }
  return null;
}

/** A rejected command is the engine's problem; anything else is the generic problem. */
export function toProblem(error: unknown): Problem {
  return parseProblemView(error) ?? FAILED;
}

/* ---------- Commands ---------- */

/** The shell's invoke, with arguments only when the command takes some. */
const send = (command: string, args?: Record<string, unknown>): Promise<unknown> => (args === undefined ? invoke(command) : invoke(command, args));

/** Runs a command whose reply carries nothing; a rejection reaches the caller as it came. */
async function run(command: string, args?: Record<string, unknown>): Promise<undefined> {
  await send(command, args);
  return undefined;
}

/**
 * Runs a command and parses its reply. A reply that does not parse is refused with an error that is
 * not a problem, so `toProblem` makes it the generic problem.
 */
async function ask<T>(command: string, parse: (value: unknown) => T | null, args?: Record<string, unknown>): Promise<T> {
  const reply = parse(await send(command, args));
  if (reply === null) throw new Error(`the reply to ${command} did not have the engine's shape`);
  return reply;
}

export const pairingOldStart = (network: string | null) => run("pairing_old_start", { network });
/** A pick or a decline names the round of numbers it answers, so a late one never applies to the next. */
export const pairingOldPick = (round: number, number: number) => run("pairing_old_pick", { round, number });
export const pairingOldDecline = (round: number) => run("pairing_old_decline", { round });
export const pairingOldShuffle = () => run("pairing_old_shuffle");
export const pairingOldRename = (name: string) => run("pairing_old_rename", { name });
export const pairingOldStartAgain = () => run("pairing_old_start_again");
export const pairingStop = () => run("pairing_stop");
export const pairingHeadsUp = (role: PairRole) => ask("pairing_heads_up", parseHeadsUpList, { role });
export const pairingNewSearch = () => ask("pairing_new_search", parseNearbyList);
export const pairingNewConnect = (index: number, code: string) => ask("pairing_new_connect", parseShownNumber, { index, code });

/* ---------- Events ---------- */

/** Old laptop updates. One that does not parse ends pairing on screen as a lost connection (start again). */
export const listenPairingOld = (handler: (event: OldEvent) => void): Promise<UnlistenFn> =>
  listen<unknown>("pairing-old", (e) => {
    handler(parseOldView(e.payload) ?? { kind: "lost" });
  });

/** New laptop updates. One that does not parse is the generic problem (the screen looks again). */
export const listenPairingNew = (handler: (event: NewEvent) => void): Promise<UnlistenFn> =>
  listen<unknown>("pairing-new", (e) => {
    handler(parseNewView(e.payload) ?? { kind: "problem", problem: FAILED });
  });

/* ---------- The move (src-tauri/src/moving.rs) ---------- */

/** Why a move command was refused (`MoveProblem`). */
export const MOVE_PROBLEMS = ["not-paired", "busy", "not-moving", "plan-changed", "failed"] as const;
export type MoveProblem = { kind: (typeof MOVE_PROBLEMS)[number] };

/** What the move is at before files move (`Stage`). */
export type MoveStage = { kind: "scanning" | "waiting" | "moving" };

/** Why a move ended before it finished (`EndedEarly`). `detail` is the engine's words, for "more details" only. */
export type EndedEarly = { kind: "link-lost"; detail: string } | { kind: "not-approved" } | { kind: "failed"; detail: string };

/** The exact plan revision the person answers. */
export type PlanKey = { revision: number; fingerprint: string };

/**
 * Keys a shape may leave out (the engine sends them only when there is something to say). Every
 * other key must be there. The contract test reads this to know which keys may be missing.
 */
export const OPTIONAL_KEYS: Partial<Record<string, readonly string[]>> = {
  "move-progress": ["current", "speed", "minutesLeft", "notice"],
  "report-item": ["destination", "reason", "evidence"],
};

/** An object with every key in `required`, any of `optional`, and nothing else; or null. */
function shaped(value: unknown, required: readonly string[], optional: readonly string[]): Fields | null {
  if (typeof value !== "object" || value === null || Array.isArray(value)) return null;
  const fields = value as Fields;
  const has = (k: string) => Object.prototype.hasOwnProperty.call(fields, k);
  if (!required.every(has)) return null;
  return Object.keys(fields).every((k) => required.includes(k) || optional.includes(k)) ? fields : null;
}

function oneOf<T extends string>(value: unknown, list: readonly T[]): value is T {
  return typeof value === "string" && (list as readonly string[]).includes(value);
}

/** Every entry parses, or the whole list is refused. */
function listOf<T>(value: unknown, parse: (v: unknown) => T | null): T[] | null {
  if (!Array.isArray(value)) return null;
  const list = (value as unknown[]).map(parse);
  return list.every((v): v is T => v !== null) ? list : null;
}

function parseRange(value: unknown): Range | null {
  const f = exactly(value, ["low", "high"]);
  return f && isCount(f.low) && isCount(f.high) ? { low: f.low, high: f.high } : null;
}

/** A tagged value whose only field is `kind`, from `kinds`. */
function parseKindOnly<T extends string>(value: unknown, kinds: readonly T[]): { kind: T } | null {
  const f = exactly(value, ["kind"]);
  return f && oneOf(f.kind, kinds) ? { kind: f.kind } : null;
}

export function parseMoveProblem(value: unknown): MoveProblem | null {
  return parseKindOnly(value, MOVE_PROBLEMS);
}

/** A rejected move command: the engine's reason, or "failed" for anything else. */
export function toMoveProblem(error: unknown): MoveProblem {
  return parseMoveProblem(error) ?? { kind: "failed" };
}

export function parseMoveStage(value: unknown): MoveStage | null {
  return parseKindOnly(value, ["scanning", "waiting", "moving"] as const);
}

export function parseEndedEarly(value: unknown): EndedEarly | null {
  const kind = typeof value === "object" && value !== null ? (value as Fields).kind : undefined;
  if (kind === "not-approved") return exactly(value, ["kind"]) ? { kind } : null;
  if (kind === "link-lost" || kind === "failed") {
    const f = exactly(value, ["kind", "detail"]);
    return f && typeof f.detail === "string" ? { kind, detail: f.detail } : null;
  }
  return null;
}

function parseSize(value: unknown): Size | null {
  const kind = typeof value === "object" && value !== null ? (value as Fields).kind : undefined;
  if (kind === "unreadable") return exactly(value, ["kind"]) ? { kind } : null;
  if (kind === "known" || kind === "at-least") {
    const f = exactly(value, ["kind", "bytes", "files"]);
    return f && isCount(f.bytes) && isCount(f.files) ? { kind, bytes: f.bytes, files: f.files } : null;
  }
  return null;
}

function parsePlanGroup(value: unknown): PlanGroup | null {
  const f = exactly(value, ["id", "role", "path", "size", "toDrive", "toFolder"]);
  if (!f || typeof f.id !== "string" || !oneOf(f.role, FOLDER_ROLES) || typeof f.path !== "string") return null;
  if (typeof f.toDrive !== "string" || typeof f.toFolder !== "string") return null;
  const size = parseSize(f.size);
  return size ? { id: f.id, role: f.role, path: f.path, size, toDrive: f.toDrive, toFolder: f.toFolder } : null;
}

function parseLeftOut(value: unknown): LeftOut | null {
  const f = exactly(value, ["reason", "count"]);
  return f && oneOf(f.reason, LEFT_OUT_REASONS) && isCount(f.count) ? { reason: f.reason, count: f.count } : null;
}

function parsePlanPreview(value: unknown): PlanPreview | null {
  const f = exactly(value, ["fits", "shortBytes", "minutes", "renamed", "tooBig", "refused", "runsAtSignIn"]);
  if (!f || !(f.fits === null || typeof f.fits === "boolean")) return null;
  const minutes = f.minutes === null ? null : parseRange(f.minutes);
  if (f.minutes !== null && minutes === null) return null;
  const { shortBytes, renamed, tooBig, refused, runsAtSignIn } = f;
  if (!isCount(shortBytes) || !isCount(renamed) || !isCount(tooBig) || !isCount(refused) || !isCount(runsAtSignIn)) return null;
  return { fits: f.fits, shortBytes, minutes, renamed, tooBig, refused, runsAtSignIn };
}

/** The plan for the person to approve (`PlanView`, event `move-plan`). */
export function parsePlanView(value: unknown): PlanView | null {
  const f = exactly(value, ["revision", "fingerprint", "groups", "leftOut", "preview"]);
  if (!f || !isCount(f.revision) || typeof f.fingerprint !== "string" || !/^[0-9a-f]{64}$/.test(f.fingerprint)) return null;
  const groups = listOf(f.groups, parsePlanGroup);
  const leftOut = listOf(f.leftOut, parseLeftOut);
  const preview = parsePlanPreview(f.preview);
  return groups && leftOut && preview ? { revision: f.revision, fingerprint: f.fingerprint, groups, leftOut, preview } : null;
}

const COUNT_KEYS = ["total", "waiting", "copying", "checking", "done", "needsAction", "failed", "skippedByUser"] as const;

function parseCounts(value: unknown): Counts | null {
  const f = exactly(value, COUNT_KEYS);
  if (!f || !COUNT_KEYS.every((k) => isCount(f[k]))) return null;
  return Object.fromEntries(COUNT_KEYS.map((k) => [k, f[k]])) as Counts;
}

function parseGroupName(value: unknown): GroupName | null {
  const kind = typeof value === "object" && value !== null ? (value as Fields).kind : undefined;
  if (kind === "role") {
    const f = exactly(value, ["kind", "role"]);
    return f && oneOf(f.role, FOLDER_ROLES) ? { kind, role: f.role } : null;
  }
  if (kind === "folder") {
    const f = exactly(value, ["kind", "name"]);
    return f && typeof f.name === "string" ? { kind, name: f.name } : null;
  }
  return null;
}

function parseMoveGroup(value: unknown): MoveGroup | null {
  const f = exactly(value, ["id", "name", "counts"]);
  if (!f || typeof f.id !== "string") return null;
  const name = parseGroupName(f.name);
  const counts = parseCounts(f.counts);
  return name && counts ? { id: f.id, name, counts } : null;
}

/** The measured progress (`MoveProgress`, event `move-progress`). */
export function parseMoveProgress(value: unknown): MoveProgress | null {
  const f = shaped(value, ["state", "laptops", "groups", "counts", "bytes"], OPTIONAL_KEYS["move-progress"] ?? []);
  if (!f || !oneOf(f.state, ["running", "pausing", "paused"] as const)) return null;
  const laptops = exactly(f.laptops, ["old"]);
  if (!laptops) return null;
  const old = laptops.old === null ? null : parseLabelView(laptops.old);
  if (laptops.old !== null && old === null) return null;
  const groups = listOf(f.groups, parseMoveGroup);
  const counts = parseCounts(f.counts);
  const bytes = exactly(f.bytes, ["checked", "total"]);
  if (!groups || !counts || !bytes || !isCount(bytes.checked) || !isCount(bytes.total)) return null;
  const progress: MoveProgress = { state: f.state, laptops: { old }, groups, counts, bytes: { checked: bytes.checked, total: bytes.total } };
  if ("current" in f) {
    const c = exactly(f.current, ["name", "step"]);
    if (!c || typeof c.name !== "string" || !oneOf(c.step, ["copying", "checking"] as const)) return null;
    progress.current = { name: c.name, step: c.step };
  }
  for (const key of ["speed", "minutesLeft"] as const) {
    if (!(key in f)) continue;
    const range = parseRange(f[key]);
    if (!range) return null;
    progress[key] = range;
  }
  if ("notice" in f) {
    const n = exactly(f.notice, ["kind", "place"]);
    const place = n?.kind === "drive-problem" ? parseGroupName(n.place) : null;
    if (!place) return null;
    progress.notice = { kind: "drive-problem", place };
  }
  return progress;
}

/** How the move ended (`MoveOutcome`, event `move-finished`). */
export function parseMoveOutcome(value: unknown): MoveOutcome | null {
  const f = exactly(value, ["byStatus", "notCopied", "bytesChecked", "stopped", "checkedFromMemory"]);
  if (!f || !isCount(f.notCopied) || !isCount(f.bytesChecked) || typeof f.stopped !== "boolean" || !isCount(f.checkedFromMemory)) return null;
  const by = exactly(f.byStatus, ITEM_STATUSES);
  if (!by || !ITEM_STATUSES.every((s) => isCount(by[s]))) return null;
  const byStatus = Object.fromEntries(ITEM_STATUSES.map((s) => [s, by[s]])) as MoveOutcome["byStatus"];
  return { byStatus, notCopied: f.notCopied, bytesChecked: f.bytesChecked, stopped: f.stopped, checkedFromMemory: f.checkedFromMemory };
}

/** How a copied line in the report was checked: the gate's two, or one of the move's own. */
function parseReportEvidence(value: unknown): Evidence | null {
  const from = typeof value === "object" && value !== null ? (value as Fields).from : undefined;
  if (from === "already-there" || from === "recovered" || from === "new-laptop") return exactly(value, ["from"]) ? { from } : null;
  return parseEvidence(value);
}

/**
 * One line of the report (`ReportItem`). Status and reason must fit each other (`parseResult`), and
 * exactly a copied line says how it was checked.
 */
export function parseReportItem(value: unknown): ReportItem | null {
  const f = shaped(value, ["id", "name", "source", "status"], OPTIONAL_KEYS["report-item"] ?? []);
  if (!f || typeof f.id !== "string" || typeof f.name !== "string" || typeof f.source !== "string") return null;
  if ("destination" in f && typeof f.destination !== "string") return null;
  const result = parseResult("reason" in f ? { status: f.status, reason: f.reason } : { status: f.status });
  if (!result) return null;
  const line = { id: f.id, name: f.name, source: f.source, ...(typeof f.destination === "string" ? { destination: f.destination } : {}) };
  if (result.status === "copied") {
    const evidence = parseReportEvidence(f.evidence);
    return evidence ? { ...line, status: "copied", evidence } : null;
  }
  if ("evidence" in f) return null;
  return { ...line, ...result } as ReportItem;
}

export const moveBegin = (fallback: Partial<Record<FolderRole, string>>) => run("move_begin", { fallback });
export const moveApprove = (plan: PlanKey, approve: boolean) => run("move_approve", { plan, approve });
export const movePause = () => run("move_pause");
export const moveResume = () => run("move_resume");
export const moveStop = () => run("move_stop");
export const moveReport = () => ask("move_report", (v) => listOf(v, parseReportItem));

/** Every move event in one place. A payload that does not parse ends the move on screen as a problem, never shown as sent. */
export type MoveEvent =
  | { kind: "stage"; stage: MoveStage }
  | { kind: "plan"; plan: PlanView }
  | { kind: "progress"; progress: MoveProgress }
  | { kind: "finished"; outcome: MoveOutcome }
  | { kind: "problem"; problem: EndedEarly };

const UNREADABLE: MoveEvent = { kind: "problem", problem: { kind: "failed", detail: "" } };

export async function listenMove(handler: (event: MoveEvent) => void): Promise<UnlistenFn> {
  const on = <T,>(name: string, parse: (v: unknown) => T | null, wrap: (v: T) => MoveEvent) =>
    listen<unknown>(name, (e) => {
      const parsed = parse(e.payload);
      handler(parsed === null ? UNREADABLE : wrap(parsed));
    });
  const all = await Promise.all([
    on("move-stage", parseMoveStage, (stage) => ({ kind: "stage", stage })),
    on("move-plan", parsePlanView, (plan) => ({ kind: "plan", plan })),
    on("move-progress", parseMoveProgress, (progress) => ({ kind: "progress", progress })),
    on("move-finished", parseMoveOutcome, (outcome) => ({ kind: "finished", outcome })),
    on("move-problem", parseEndedEarly, (problem) => ({ kind: "problem", problem })),
  ]);
  return () => {
    for (const stop of all) stop();
  };
}

/* ---------- The window and the tray: only inside the app ---------- */

/**
 * Inside the app these reach the shell; outside it (tests, a browser preview) they do nothing.
 * Loaded on demand so a browser preview never needs the shell's script.
 */
const inApp = () => "__TAURI_INTERNALS__" in window;

async function runInApp(command: string, args: Record<string, unknown>): Promise<void> {
  if (!inApp()) return;
  const core = await import("@tauri-apps/api/core");
  await core.invoke(command, args);
}

/** The three window sizes from the design system. */
export type WindowMode = "everyday" | "mini" | "expanded";

export const setWindowMode = (mode: WindowMode): Promise<void> => runInApp("set_window_mode", { mode });

/** The tray menu speaks the person's language too. */
export const setTrayLabels = (show: string, quit: string): Promise<void> => runInApp("set_tray_labels", { show, quit });

/** Our own site in the person's browser (the shell allows https://getpctwin.com/ only). Outside the app, a new tab. */
export async function openInBrowser(url: string): Promise<void> {
  if (inApp()) {
    const { openUrl } = await import("@tauri-apps/plugin-opener");
    await openUrl(url);
  } else {
    window.open(url, "_blank", "noopener,noreferrer");
  }
}
