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
import { ITEM_STATUSES, WHY_MEMORY, parseResult, type PersonOutcome, type Counts, type Evidence, type GroupName, type MoveGroup, type MoveOutcome, type MoveProgress, type Range, type ReportItem, type WhyMemory } from "./move";
import type { OfferedFile, Outcome as QsOutcome, PickedItem, Receipt, SendProgress } from "./quicksend";
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

/** What the move is at before files move (`Stage`): `administrator` while the system's approval prompt is up (M25). */
export type WaitStage = "scanning" | "waiting" | "moving" | "administrator";

/** Why the other people ticked were not moved this time (`OthersWhy`). */
export const OTHERS_WHY = ["not-approved", "failed", "not-here"] as const;
export type OthersWhy = (typeof OTHERS_WHY)[number];

export type MoveStage = { kind: WaitStage } | { kind: "others-not-moved"; why: OthersWhy };

/** How much of the mover's own things the scan has found so far (only ever grows). */
export function parseFound(value: unknown): { bytes: number } | null {
  const f = exactly(value, ["bytes"]);
  return f && isCount(f.bytes) ? { bytes: f.bytes } : null;
}

/** A Mac kept this many of the mover's places private (the Full Disk Access guide). */
export type Privacy = { places: number };

export function parsePrivacy(value: unknown): Privacy | null {
  const f = exactly(value, ["places"]);
  return f && isCount(f.places) && f.places > 0 ? { places: f.places } : null;
}

export const movePrivacyAnswer = (again: boolean) => run("move_privacy_answer", { again });
export const openPrivacySettings = () => run("open_privacy_settings");

/** Where one other person's things go on the new laptop. */
export type PlaceChoice = { kind: "existing"; accountId: string } | { kind: "new" } | { kind: "leave-out" };

/** The other people to place on the new laptop (M06): names and counts only, and this laptop's accounts. */
export type PeopleToPlace = {
  people: { id: string; name: string; files: number; bytes: number }[];
  accounts: { id: string; name: string }[];
  canPlace: boolean;
};

/** The words of the system window that shows a new account's temporary password: {name}, {account} and {password} are filled in there. */
export type PasswordWords = { title: string; body: string };

export function parsePeopleToPlace(value: unknown): PeopleToPlace | null {
  const f = exactly(value, ["people", "accounts", "canPlace"]);
  if (!f || typeof f.canPlace !== "boolean") return null;
  const people = listOf(f.people, (v) => {
    const p = exactly(v, ["id", "name", "files", "bytes"]);
    return p && typeof p.id === "string" && typeof p.name === "string" && isCount(p.files) && isCount(p.bytes) ? { id: p.id, name: p.name, files: p.files, bytes: p.bytes } : null;
  });
  const accounts = listOf(f.accounts, (v) => {
    const a = exactly(v, ["id", "name"]);
    return a && typeof a.id === "string" && typeof a.name === "string" ? { id: a.id, name: a.name } : null;
  });
  return people && accounts ? { people, accounts, canPlace: f.canPlace } : null;
}

/** The people on the old laptop (M06), and whether others can be moved from this system. */
export type PeopleHere = { people: { id: string; name: string; isYou: boolean }[]; canMoveOthers: boolean };

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
  "qs-picked": ["problem"],
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
  const kind = typeof value === "object" && value !== null ? (value as Fields).kind : undefined;
  if (kind === "others-not-moved") {
    const f = exactly(value, ["kind", "why"]);
    return f && oneOf(f.why, OTHERS_WHY) ? { kind, why: f.why } : null;
  }
  return parseKindOnly(value, ["scanning", "waiting", "moving", "administrator"] as const);
}

export function parsePeopleHere(value: unknown): PeopleHere | null {
  const f = exactly(value, ["people", "canMoveOthers"]);
  if (!f || typeof f.canMoveOthers !== "boolean") return null;
  const people = listOf(f.people, (v) => {
    const p = exactly(v, ["id", "name", "isYou"]);
    return p && typeof p.id === "string" && typeof p.name === "string" && typeof p.isYou === "boolean" ? { id: p.id, name: p.name, isYou: p.isYou } : null;
  });
  // Exactly one person is the mover.
  if (!people || people.filter((p) => p.isYou).length !== 1) return null;
  return { people, canMoveOthers: f.canMoveOthers };
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

/** Something the person may need to act on during the move (`Notice`). */
function parseNotice(value: unknown): MoveProgress["notice"] | null {
  const kind = typeof value === "object" && value !== null ? (value as Fields).kind : undefined;
  if (kind === "drive-problem") {
    const n = exactly(value, ["kind", "place"]);
    const place = n && parseGroupName(n.place);
    return place ? { kind, place } : null;
  }
  if (kind === "low-battery") {
    const n = exactly(value, ["kind", "paused"]);
    return n && typeof n.paused === "boolean" ? { kind, paused: n.paused } : null;
  }
  return null;
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
    const notice = parseNotice(f.notice);
    if (!notice) return null;
    progress.notice = notice;
  }
  return progress;
}

/** How the move ended (`MoveOutcome`, event `move-finished`). */
function parsePersonOutcome(value: unknown): PersonOutcome | null {
  const f = exactly(value, ["name", "copied", "needsAction", "failed", "atSignIn"]);
  if (!f || typeof f.name !== "string" || !isCount(f.copied) || !isCount(f.needsAction) || !isCount(f.failed) || typeof f.atSignIn !== "boolean") return null;
  return { name: f.name, copied: f.copied, needsAction: f.needsAction, failed: f.failed, atSignIn: f.atSignIn };
}

export function parseMoveOutcome(value: unknown): MoveOutcome | null {
  const f = exactly(value, ["people", "byStatus", "notCopied", "bytesChecked", "stopped", "checkedFromMemory"]);
  if (!f || !isCount(f.notCopied) || !isCount(f.bytesChecked) || typeof f.stopped !== "boolean" || !isCount(f.checkedFromMemory)) return null;
  const by = exactly(f.byStatus, ITEM_STATUSES);
  if (!by || !ITEM_STATUSES.every((s) => isCount(by[s]))) return null;
  const byStatus = Object.fromEntries(ITEM_STATUSES.map((s) => [s, by[s]])) as MoveOutcome["byStatus"];
  const people = listOf(f.people, parsePersonOutcome);
  if (!people) return null;
  return { people, byStatus, notCopied: f.notCopied, bytesChecked: f.bytesChecked, stopped: f.stopped, checkedFromMemory: f.checkedFromMemory };
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

export const moveBegin = (fallback: Partial<Record<FolderRole, string>>, others: string[]) => run("move_begin", { fallback, others });
export const movePeopleHere = () => ask("move_people_here", parsePeopleHere);
export const movePlacePeople = (choices: { id: string; to: PlaceChoice }[], words: PasswordWords) => run("move_place_people", { choices, words });
/** The answer to the plan on screen; `leftOut`: the folders the person unticked (none of them moves). */
export const moveApprove = (plan: PlanKey, approve: boolean, leftOut: FolderRole[] = []) => run("move_approve", { plan, approve, leftOut });
export const movePause = () => run("move_pause");
export const moveResume = () => run("move_resume");
export const moveStop = () => run("move_stop");
export const moveReport = () => ask("move_report", (v) => listOf(v, parseReportItem));

/** Every move event in one place. A payload that does not parse ends the move on screen as a problem, never shown as sent. */
export type MoveEvent =
  | { kind: "stage"; stage: MoveStage }
  | { kind: "plan"; plan: PlanView }
  | { kind: "people"; toPlace: PeopleToPlace }
  | { kind: "privacy"; privacy: Privacy }
  | { kind: "found"; bytes: number }
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
    on("move-people", parsePeopleToPlace, (toPlace) => ({ kind: "people", toPlace })),
    on("move-privacy", parsePrivacy, (privacy) => ({ kind: "privacy", privacy })),
    on("move-found", parseFound, (f) => ({ kind: "found", bytes: f.bytes })),
    on("move-progress", parseMoveProgress, (progress) => ({ kind: "progress", progress })),
    on("move-finished", parseMoveOutcome, (outcome) => ({ kind: "finished", outcome })),
    on("move-problem", parseEndedEarly, (problem) => ({ kind: "problem", problem })),
  ]);
  return () => {
    for (const stop of all) stop();
  };
}

/* ---------- Picking up after a restart (src-tauri/src/recovery.rs) ---------- */

/** What a move that stopped part way left on this laptop (`Interrupted`). */
export type Interrupted = { unfinished: number };

/** What tidying up did, counted (`Tidied`). */
export type Tidied = { putInPlace: number; keptToContinue: number; notFinished: number; leftForLater: number };

export function parseInterrupted(value: unknown): Interrupted | null {
  const f = exactly(value, ["unfinished"]);
  return f && isCount(f.unfinished) ? { unfinished: f.unfinished } : null;
}

const TIDIED_KEYS = ["putInPlace", "keptToContinue", "notFinished", "leftForLater"] as const;

export function parseTidied(value: unknown): Tidied | null {
  const f = exactly(value, TIDIED_KEYS);
  if (!f || !TIDIED_KEYS.every((k) => isCount(f[k]))) return null;
  return Object.fromEntries(TIDIED_KEYS.map((k) => [k, f[k]])) as Tidied;
}

export const recoveryCheck = () => ask("recovery_check", parseInterrupted);

/** "Your things are ready" (M32): what a hand-over at this person's sign-in did, counted. */
export type ReadyNote = { landed: number; kept: number; failed: number };

export function parseReadyNote(value: unknown): ReadyNote | null {
  const f = exactly(value, ["landed", "kept", "failed"]);
  return f && isCount(f.landed) && isCount(f.kept) && isCount(f.failed) ? { landed: f.landed, kept: f.kept, failed: f.failed } : null;
}

/** The note, once (null: there is none). */
export const thingsReady = () => ask("things_ready", (v) => (v === null ? { none: true as const } : parseReadyNote(v)));
export const recoveryRun = () => ask("recovery_run", parseTidied);

/* ---------- Quick Send (src-tauri/src/quicksend.rs) ---------- */

const QS_PROBLEMS = ["missing", "unreadable", "online-only"] as const;

function isCountOrNull(v: unknown): v is number | null {
  return v === null || isCount(v);
}

/** One thing chosen in the system's picker (the screens never see its path). */
export function parsePicked(value: unknown): PickedItem | null {
  const f = shaped(value, ["id", "name", "kind", "included", "bytes", "files"], ["problem"]);
  if (!f || typeof f.id !== "string" || typeof f.name !== "string" || !oneOf(f.kind, ["file", "folder"] as const)) return null;
  if (typeof f.included !== "boolean" || !isCountOrNull(f.bytes) || !isCountOrNull(f.files)) return null;
  if ("problem" in f && !oneOf(f.problem, QS_PROBLEMS)) return null;
  const item: PickedItem = { id: f.id, name: f.name, kind: f.kind, included: f.included, bytes: f.bytes, files: f.files };
  if (oneOf(f.problem, QS_PROBLEMS)) item.problem = f.problem;
  return item;
}

export function parseOffered(value: unknown): OfferedFile | null {
  const f = exactly(value, ["id", "path", "size"]);
  return f && typeof f.id === "string" && typeof f.path === "string" && isCount(f.size) ? { id: f.id, path: f.path, size: f.size } : null;
}

export function parseQsIncoming(value: unknown): { files: OfferedFile[] } | null {
  const f = exactly(value, ["files"]);
  const files = f && listOf(f.files, parseOffered);
  return files ? { files } : null;
}

function parseQsOutcome(value: unknown): QsOutcome | null {
  const kind = typeof value === "object" && value !== null ? (value as Fields).kind : undefined;
  if (kind === "declined") return exactly(value, ["kind"]) ? { kind } : null;
  if (kind === "arrived" || kind === "already-there") {
    const f = exactly(value, ["kind", "landedAs"]);
    return f && typeof f.landedAs === "string" ? { kind, landedAs: f.landedAs } : null;
  }
  if (kind === "needs-action") {
    const f = exactly(value, ["kind", "landedAs", "why"]);
    return f && typeof f.landedAs === "string" && typeof f.why === "string" ? { kind, landedAs: f.landedAs, why: f.why } : null;
  }
  if (kind === "failed") {
    const f = exactly(value, ["kind", "why"]);
    return f && typeof f.why === "string" ? { kind, why: f.why } : null;
  }
  return null;
}

/** How a Quick Send ended, on one side. */
export type QsReceipt = { side: "sender" | "receiver"; lines: Receipt["lines"]; stopped: boolean };

export function parseQsReceipt(value: unknown): QsReceipt | null {
  const f = exactly(value, ["side", "lines", "stopped"]);
  if (!f || !oneOf(f.side, ["sender", "receiver"] as const) || typeof f.stopped !== "boolean") return null;
  const lines = listOf(f.lines, (v) => {
    const l = exactly(v, ["path", "outcome"]);
    const outcome = l && parseQsOutcome(l.outcome);
    return l && typeof l.path === "string" && outcome ? { path: l.path, outcome } : null;
  });
  return lines ? { side: f.side, lines, stopped: f.stopped } : null;
}

const QS_PHASES = ["copying", "paused", "checking", "needs-action", "lost", "complete"] as const;

export function parseQsProgress(value: unknown): SendProgress | null {
  const f = exactly(value, ["phase", "filesDone", "filesTotal", "bytesDone", "bytesTotal", "current", "failed", "speed"]);
  if (!f || !oneOf(f.phase, QS_PHASES)) return null;
  const { filesDone, filesTotal, bytesDone, bytesTotal, failed } = f;
  if (!isCount(filesDone) || !isCount(filesTotal) || !isCount(bytesDone) || !isCount(bytesTotal) || !isCount(failed)) return null;
  if (!(f.current === null || typeof f.current === "string")) return null;
  const speed = f.speed === null ? null : parseRange(f.speed);
  if (f.speed !== null && speed === null) return null;
  return { phase: f.phase, filesDone, filesTotal, bytesDone, bytesTotal, current: f.current, failed, speed };
}

export type QsFolder = { path: string; freeBytes: number | null };

export function parseQsFolder(value: unknown): QsFolder | null {
  const f = exactly(value, ["path", "freeBytes"]);
  return f && typeof f.path === "string" && isCountOrNull(f.freeBytes) ? { path: f.path, freeBytes: f.freeBytes } : null;
}

export type QsProblem = { kind: "link-lost" | "failed"; detail: string };

export function parseQsProblem(value: unknown): QsProblem | null {
  const f = exactly(value, ["kind", "detail"]);
  return f && oneOf(f.kind, ["link-lost", "failed"] as const) && typeof f.detail === "string" ? { kind: f.kind, detail: f.detail } : null;
}

export const qsPick = (folders: boolean) => ask("quicksend_pick", (v) => listOf(v, parsePicked), { folders });
export const qsClear = () => run("quicksend_clear");
/** The pick page is open (`on`) or closed: things dropped on the window are added only while it is. */
export const qsAcceptDrops = (on: boolean) => run("quicksend_accept_drops", { on });
export const qsOffer = (ids: string[]) => ask("quicksend_offer", (v) => listOf(v, parseOffered), { ids });
export const qsSend = () => run("quicksend_send");
export const qsReceive = () => run("quicksend_receive");
export const qsChooseFolder = () => ask("quicksend_choose_folder", (v) => (v === null ? { none: true as const } : parseQsFolder(v)));
export const qsAnswer = (answer: { kind: "accept"; ids: string[] | null } | { kind: "decline" }) => run("quicksend_answer", { answer });
export const qsPause = () => run("quicksend_pause");
export const qsResume = () => run("quicksend_resume");
export const qsStop = () => run("quicksend_stop");

export type QsEvent =
  | { kind: "progress"; progress: SendProgress }
  | { kind: "incoming"; files: OfferedFile[] }
  | { kind: "receipt"; receipt: QsReceipt }
  | { kind: "problem"; problem: QsProblem }
  | { kind: "picked"; items: PickedItem[] }
  | { kind: "dragging"; over: boolean };

const QS_UNREADABLE: QsEvent = { kind: "problem", problem: { kind: "failed", detail: "" } };

export async function listenQuickSend(handler: (event: QsEvent) => void): Promise<UnlistenFn> {
  const on = <T,>(name: string, parse: (v: unknown) => T | null, wrap: (v: T) => QsEvent) =>
    listen<unknown>(name, (e) => {
      const parsed = parse(e.payload);
      handler(parsed === null ? QS_UNREADABLE : wrap(parsed));
    });
  const all = await Promise.all([
    on("quicksend-progress", parseQsProgress, (progress) => ({ kind: "progress", progress })),
    on("quicksend-incoming", parseQsIncoming, (i) => ({ kind: "incoming", files: i.files })),
    on("quicksend-receipt", parseQsReceipt, (receipt) => ({ kind: "receipt", receipt })),
    on("quicksend-problem", parseQsProblem, (problem) => ({ kind: "problem", problem })),
    on("quicksend-picked", (v) => listOf(v, parsePicked), (items) => ({ kind: "picked", items })),
    on("quicksend-dragging", (v) => (typeof v === "boolean" ? { over: v } : null), (d) => ({ kind: "dragging", over: d.over })),
  ]);
  return () => {
    for (const stop of all) stop();
  };
}

/* ---------- History (src-tauri/src/history.rs) ---------- */

/** One thing done on this laptop, as the app recorded it when it ended. */
export type HistoryEntryRaw = {
  id: string;
  kind: "move" | "send" | "receive";
  when: string;
  other: Label | null;
  state: "finished" | "steps-left" | "stopped" | "partial";
  stepsLeft: number;
};

export function parseHistoryEntry(value: unknown): HistoryEntryRaw | null {
  const f = exactly(value, ["id", "kind", "when", "other", "state", "stepsLeft"]);
  if (!f || typeof f.id !== "string" || typeof f.when !== "string" || !isCount(f.stepsLeft)) return null;
  if (!oneOf(f.kind, ["move", "send", "receive"] as const) || !oneOf(f.state, ["finished", "steps-left", "stopped", "partial"] as const)) return null;
  const other = f.other === null ? null : parseLabelView(f.other);
  if (f.other !== null && other === null) return null;
  return { id: f.id, kind: f.kind, when: f.when, other, state: f.state, stepsLeft: f.stepsLeft };
}

/** What an entry kept: a move's report or a send's receipt. */
export type Kept = { kind: "report"; items: ReportItem[] } | { kind: "receipt"; receipt: QsReceipt };

export function parseKept(value: unknown): Kept | null {
  const kind = typeof value === "object" && value !== null ? (value as Fields).kind : undefined;
  if (kind === "report") {
    const f = exactly(value, ["kind", "items"]);
    const items = f && listOf(f.items, parseReportItem);
    return items ? { kind, items } : null;
  }
  if (kind === "receipt") {
    const f = exactly(value, ["kind", "receipt"]);
    const receipt = f && parseQsReceipt(f.receipt);
    return receipt ? { kind, receipt } : null;
  }
  return null;
}

export const historyList = () => ask("history_list", (v) => listOf(v, parseHistoryEntry));
export const historyOpen = (id: string) => ask("history_open", parseKept, { id });
/** The path the copy was saved to, or null when the person chose no folder. */
export const historyExport = () => ask("history_export", (v) => (v === null ? { path: null } : typeof v === "string" ? { path: v } : null)).then((r) => r.path);

/* ---------- Get help (src-tauri/src/help.rs) ---------- */

/** The kinds of problem a help code can name (`HelpTopic`), in the engine's order. */
export const HELP_TOPICS = ["pairing-failed", "search-failed", "connection-lost", "not-approved", "move-failed", "move-refused", "drive-problem", "low-battery"] as const;
export type HelpTopic = (typeof HELP_TOPICS)[number];

/** How far the task got (`HelpStep`). */
export const HELP_STEPS = ["pairing", "finding", "approving", "moving", "finished"] as const;
export type HelpStep = (typeof HELP_STEPS)[number];

/** A problem code as the engine makes it: three groups of four from Crockford's Base32. */
export function parseHelpCode(value: unknown): string | null {
  return typeof value === "string" && /^[0-9A-HJKMNP-TV-Z]{4}-[0-9A-HJKMNP-TV-Z]{4}-[0-9A-HJKMNP-TV-Z]{4}$/.test(value) ? value : null;
}

export const helpCode = (topic: HelpTopic, step: HelpStep) => ask("help_code", parseHelpCode, { topic, step });

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
