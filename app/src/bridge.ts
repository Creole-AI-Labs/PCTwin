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
import { FILE_KINDS, ITEM_STATUSES, KINDS, REPORT_NOTES, WHY_MEMORY, parseResult, type PersonOutcome, type Counts, type Evidence, type GroupName, type MoveGroup, type MoveOutcome, type MoveProgress, type Range, type ReportItem, type WhyMemory } from "./move";
import { NEEDS_ACTION_WORDS, type OfferedFile, type Outcome as QsOutcome, type PickedItem, type Receipt, type SendProgress } from "./quicksend";
import { parseAnswer, parseProjection, parseRefusal, parseSnapshot, type Answer, type IntentKind, type Projection, type Refusal, type Snapshot } from "./guard";
import { parseAppsOpen, parseDataProgress, parseRebuildPlan, parseRebuilt, parseRebuildStep, type AppsOpen, type DataProgress, type RebuildStep } from "./fullmove";
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
/** Whether another laptop nearby is showing a code as the old laptop too (both chose Old laptop). */
export const pairingOldNearby = () => ask("pairing_old_nearby", (v) => (typeof v === "boolean" ? v : null));
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
export const MOVE_PROBLEMS = ["not-paired", "busy", "not-moving", "plan-changed", "failed", "too-many-files"] as const;
export type MoveProblem = { kind: (typeof MOVE_PROBLEMS)[number] };

/** What the move is at before files move (`Stage`): `administrator` while the system's approval prompt is up (M25). */
export type WaitStage = "scanning" | "waiting" | "moving" | "administrator" | "app-data";

/** Why the other people ticked were not moved this time (`OthersWhy`). */
export const OTHERS_WHY = ["not-approved", "failed", "not-here", "allow-in-settings"] as const;
export type OthersWhy = (typeof OTHERS_WHY)[number];

export type MoveStage = { kind: WaitStage } | { kind: "others-not-moved"; why: OthersWhy } | { kind: "rebuilding" };

/** How much of the mover's own things the scan has found so far (only ever grows). */
/** What a laptop said it is when the move started (C06, `About`). `freeBytes`: its free space, when read. */
export type About = {
  system: "windows" | "mac" | "linux" | "other";
  chip: string;
  app: string;
  power: { kind: "plugged" | "unknown" } | { kind: "battery"; percent: number };
  freeBytes: number | null;
};

/**
 * This laptop and the other one as each said when the move started (`Peer`, C06), the other's
 * pairing label when known, whether their PCTwin versions can move together, and the link the new
 * laptop measured (C07; null when not measured).
 */
export type Peer = {
  mine: About;
  theirs: About;
  other: Label | null;
  /** This laptop's own name, and the name the other laptop said it has (checked by the engine). */
  mineName: Label | null;
  theirsName: Label | null;
  versions: "same" | "update-here" | "update-there";
  link: { bytesPerSecond: number; slow: boolean } | null;
};

function parseAbout(value: unknown): About | null {
  // The name a laptop says it has travels in its greeting too; the screens read it, checked, from Peer.
  const keys = typeof value === "object" && value !== null && "label" in value ? ["system", "chip", "app", "power", "freeBytes", "label"] : ["system", "chip", "app", "power", "freeBytes"];
  const f = exactly(value, keys);
  if (!f || !oneOf(f.system, ["windows", "mac", "linux", "other"] as const) || typeof f.chip !== "string" || typeof f.app !== "string") return null;
  const kind = typeof f.power === "object" && f.power !== null ? (f.power as Fields).kind : undefined;
  let power: About["power"] | null = null;
  if (kind === "plugged" || kind === "unknown") power = exactly(f.power, ["kind"]) ? { kind } : null;
  else if (kind === "battery") {
    const b = exactly(f.power, ["kind", "percent"]);
    power = b && isCount(b.percent) && b.percent <= 100 ? { kind, percent: b.percent } : null;
  }
  if (!isCountOrNull(f.freeBytes)) return null;
  return power && { system: f.system, chip: f.chip, app: f.app, power, freeBytes: f.freeBytes };
}

export function parsePeer(value: unknown): Peer | null {
  const f = exactly(value, ["mine", "theirs", "other", "mineName", "theirsName", "versions", "link"]);
  if (!f || !oneOf(f.versions, ["same", "update-here", "update-there"] as const)) return null;
  const mine = parseAbout(f.mine);
  const theirs = parseAbout(f.theirs);
  const other = f.other === null ? null : parseLabelView(f.other);
  if (f.other !== null && other === null) return null;
  const mineName = f.mineName === null ? null : parseLabelView(f.mineName);
  const theirsName = f.theirsName === null ? null : parseLabelView(f.theirsName);
  if ((f.mineName !== null && mineName === null) || (f.theirsName !== null && theirsName === null)) return null;
  let link: Peer["link"] = null;
  if (f.link !== null) {
    const l = exactly(f.link, ["bytesPerSecond", "slow"]);
    if (!l || !isCount(l.bytesPerSecond) || typeof l.slow !== "boolean") return null;
    link = { bytesPerSecond: l.bytesPerSecond, slow: l.slow };
  }
  return mine && theirs && { mine, theirs, other, mineName, theirsName, versions: f.versions, link };
}

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

/** The words of the system window that shows a new account's temporary password: {name}, {account} and {password} are filled in there; `approve`, a Mac's administrator window, with {names}. */
export type PasswordWords = { title: string; body: string; approve: string };

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
export type EndedEarly = { kind: "link-lost"; detail: string } | { kind: "not-approved" | "update-here" | "update-there" } | { kind: "failed"; detail: string };

/** The exact plan revision the person answers. */
export type PlanKey = { revision: number; fingerprint: string };

/**
 * Keys a shape may leave out (the engine sends them only when there is something to say). Every
 * other key must be there. The contract test reads this to know which keys may be missing.
 */
export const OPTIONAL_KEYS: Partial<Record<string, readonly string[]>> = {
  "plan-view": ["rebuild", "space"],
  "move-outcome": ["rebuilt", "landed"],
  "move-progress": ["current", "speed", "minutesLeft", "notice"],
  "report-item": ["destination", "reason", "evidence", "note", "kind", "bytes"],
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
  if (kind === "rebuilding") return exactly(value, ["kind"]) ? { kind } : null;
  return parseKindOnly(value, ["scanning", "waiting", "moving", "administrator", "app-data"] as const);
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
  if (kind === "not-approved" || kind === "update-here" || kind === "update-there") return exactly(value, ["kind"]) ? { kind } : null;
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
  const f = exactly(value, ["fits", "shortBytes", "minutes", "renamed", "tooBig", "refused", "runsAtSignIn", "estimatedBytes", "synced"]);
  if (!f || !(f.fits === null || typeof f.fits === "boolean")) return null;
  const minutes = f.minutes === null ? null : parseRange(f.minutes);
  if (f.minutes !== null && minutes === null) return null;
  const { shortBytes, renamed, tooBig, refused, runsAtSignIn } = f;
  if (!isCount(shortBytes) || !isCount(renamed) || !isCount(tooBig) || !isCount(refused) || !isCount(runsAtSignIn) || !isCount(f.estimatedBytes)) return null;
  const synced = listOf(f.synced, (s) => {
    const g = exactly(s, ["folder", "service"]);
    return g && typeof g.folder === "string" && typeof g.service === "string" ? { folder: g.folder, service: g.service } : null;
  });
  if (!synced) return null;
  return { fits: f.fits, shortBytes, minutes, renamed, tooBig, refused, runsAtSignIn, estimatedBytes: f.estimatedBytes, synced };
}

/** The plan for the person to approve (`PlanView`, event `move-plan`). */
export function parsePlanView(value: unknown): PlanView | null {
  const f = shaped(value, ["revision", "fingerprint", "groups", "leftOut", "preview", "first"], OPTIONAL_KEYS["plan-view"] ?? []);
  if (!f || !isCount(f.revision) || typeof f.fingerprint !== "string" || !/^[0-9a-f]{64}$/.test(f.fingerprint)) return null;
  const groups = listOf(f.groups, parsePlanGroup);
  const leftOut = listOf(f.leftOut, parseLeftOut);
  const preview = parsePlanPreview(f.preview);
  const first = listOf(f.first, (r) => (oneOf(r, FOLDER_ROLES) ? r : null));
  if (first && new Set(first).size !== first.length) return null;
  if (!groups || !leftOut || !preview || !first) return null;
  const plan: PlanView = { revision: f.revision, fingerprint: f.fingerprint, groups, leftOut, preview, first };
  if ("rebuild" in f) {
    const rebuild = parseRebuildPlan(f.rebuild);
    if (!rebuild) return null;
    plan.rebuild = rebuild;
  }
  if ("space" in f) {
    const s = exactly(f.space, ["files", "bytes", "stillShortBytes"]);
    if (!s || !isCount(s.files) || !isCount(s.bytes) || !isCount(s.stillShortBytes) || s.files === 0) return null;
    plan.space = { files: s.files, bytes: s.bytes, stillShortBytes: s.stillShortBytes };
  }
  return plan;
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
  if (kind === "may-sleep") return exactly(value, ["kind"]) ? { kind } : null;
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
  const f = shaped(value, ["people", "byStatus", "notCopied", "bytesChecked", "stopped", "brokeOff", "checkedFromMemory", "afterMove"], OPTIONAL_KEYS["move-outcome"] ?? []);
  if (!f || !isCount(f.notCopied) || !isCount(f.bytesChecked) || typeof f.stopped !== "boolean" || typeof f.brokeOff !== "boolean" || !isCount(f.checkedFromMemory)) return null;
  const by = exactly(f.byStatus, ITEM_STATUSES);
  if (!by || !ITEM_STATUSES.every((s) => isCount(by[s]))) return null;
  const byStatus = Object.fromEntries(ITEM_STATUSES.map((s) => [s, by[s]])) as MoveOutcome["byStatus"];
  const people = listOf(f.people, parsePersonOutcome);
  if (!people) return null;
  const afterMove = parseAfterMove(f.afterMove);
  if (!afterMove) return null;
  const outcome: MoveOutcome = { people, byStatus, notCopied: f.notCopied, bytesChecked: f.bytesChecked, stopped: f.stopped, brokeOff: f.brokeOff, checkedFromMemory: f.checkedFromMemory, afterMove };
  if ("rebuilt" in f) {
    const rebuilt = parseRebuilt(f.rebuilt);
    if (!rebuilt) return null;
    outcome.rebuilt = rebuilt;
  }
  if ("landed" in f) {
    const landed = listOf(f.landed, (x) => {
      const l = exactly(x, ["name", "folder"]);
      const name = l && parseGroupName(l.name);
      return name && typeof l.folder === "string" ? { name, folder: l.folder } : null;
    });
    if (!landed) return null;
    outcome.landed = landed;
  }
  return outcome;
}

/** The after-move check: every kind, each with whole counts, and never more copied than planned. */
function parseAfterMove(value: unknown): MoveOutcome["afterMove"] | null {
  const f = exactly(value, KINDS);
  if (!f) return null;
  const counts = KINDS.map((k) => {
    const c = exactly(f[k], ["planned", "copied"]);
    return c && isCount(c.planned) && isCount(c.copied) && c.copied <= c.planned ? { planned: c.planned, copied: c.copied } : null;
  });
  if (counts.some((c) => c === null)) return null;
  return Object.fromEntries(KINDS.map((k, i) => [k, counts[i]])) as NonNullable<MoveOutcome["afterMove"]>;
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
  if ("note" in f && !oneOf(f.note, REPORT_NOTES)) return null;
  if ("kind" in f && !oneOf(f.kind, FILE_KINDS)) return null;
  if ("bytes" in f && !(typeof f.bytes === "number" && Number.isSafeInteger(f.bytes) && f.bytes >= 0)) return null;
  const line = {
    id: f.id,
    name: f.name,
    source: f.source,
    ...(typeof f.destination === "string" ? { destination: f.destination } : {}),
    ...(oneOf(f.note, REPORT_NOTES) ? { note: f.note } : {}),
    ...(oneOf(f.kind, FILE_KINDS) ? { kind: f.kind } : {}),
    ...(typeof f.bytes === "number" ? { bytes: f.bytes } : {}),
  };
  if (result.status === "copied") {
    const evidence = parseReportEvidence(f.evidence);
    return evidence ? { ...line, status: "copied", evidence } : null;
  }
  if ("evidence" in f) return null;
  return { ...line, ...result } as ReportItem;
}

/** `approve`: the words of a Mac's administrator window, with {names} filled in there with the people ticked. */
export const moveBegin = (fallback: Partial<Record<FolderRole, string>>, others: string[], approve: string) => run("move_begin", { fallback, others, approve });
export const movePeopleHere = () => ask("move_people_here", parsePeopleHere);

/** R13 Rescue a laptop: a drive connected to this laptop that a rescue can read (2.6). */
export type RescueDrive = { id: string; name: string; fileSystem: string; totalBytes: number; looksLikeALaptop: boolean };

export function parseRescueDrive(value: unknown): RescueDrive | null {
  const f = exactly(value, ["id", "name", "fileSystem", "totalBytes", "looksLikeALaptop"]);
  if (!f || typeof f.id !== "string" || f.id === "" || typeof f.name !== "string" || typeof f.fileSystem !== "string") return null;
  if (typeof f.totalBytes !== "number" || !Number.isSafeInteger(f.totalBytes) || f.totalBytes < 0 || typeof f.looksLikeALaptop !== "boolean") return null;
  return { id: f.id, name: f.name, fileSystem: f.fileSystem, totalBytes: f.totalBytes, looksLikeALaptop: f.looksLikeALaptop };
}

/** The drives a rescue could read now. */
export const rescueDrives = () => ask("rescue_drives", (v) => listOf(v, parseRescueDrive));
/** Starts a rescue from drive `id` (one rescueDrives gave): the move's own events follow, as on a new laptop. */
export const rescueBegin = (id: string, fallback: Partial<Record<FolderRole, string>>) => run("rescue_begin", { id, fallback });
export const movePlacePeople = (choices: { id: string; to: PlaceChoice }[], words: PasswordWords) => run("move_place_people", { choices, words });
/** What came of choosing another place for a group of folders (M05). */
export type PlaceAnswer = { kind: "chosen"; path: string; freeBytes: number | null } | { kind: "not-chosen" | "no-safe-name" | "cannot-open" | "locked" };

export function parsePlaceAnswer(value: unknown): PlaceAnswer | null {
  if (typeof value !== "object" || value === null) return null;
  const kind = (value as { kind?: unknown }).kind;
  if (kind === "chosen") {
    const f = exactly(value, ["kind", "path", "freeBytes"]);
    return f && typeof f.path === "string" && isCountOrNull(f.freeBytes) ? { kind, path: f.path, freeBytes: f.freeBytes } : null;
  }
  const f = exactly(value, ["kind"]);
  return f && oneOf(f.kind, ["not-chosen", "no-safe-name", "cannot-open", "locked"] as const) ? { kind: f.kind } : null;
}

/** While the plan waits: the person chooses another folder for the folders of `role`, in the system's picker. */
export const moveChoosePlace = (role: FolderRole) => ask("move_choose_place", parsePlaceAnswer, { role });
/** The folders of `role` go to their usual place again. */
export const moveUsualPlace = (role: FolderRole) => run("move_usual_place", { role });
/** The answer to the plan on screen; `leftOut`: the folders the person unticked (none of them moves). */
/** `secrets`: the secret lines of app data the person ticked, one by one (none unless ticked). */
/** `leaveBehind`: the person accepted the plan's space proposal (leave those files on the old laptop). */
export const moveApprove = (plan: PlanKey, approve: boolean, leftOut: FolderRole[] = [], secrets: number[] = [], updates: number[] = [], leaveBehind = false) =>
  run("move_approve", { plan, approve, leftOut, secrets, updates, leaveBehind });
export const movePause = () => run("move_pause");
export const moveResume = () => run("move_resume");
export const moveStop = () => run("move_stop");
/** "Try now" while waiting for the other laptop after a drop (C09). */
export const moveRetry = () => run("move_retry");

/**
 * The connection during the move (`ConnectionView`, event `move-link`): connected again, waiting
 * for the other laptop after a drop (C09; `nothingNew` once tries keep bringing nothing), or ended
 * because the laptops could not find each other again (C10). Sizes are measured bytes.
 */
export type ConnectionView =
  | { kind: "connected" }
  | { kind: "waiting"; nothingNew: boolean; lastSeenSeconds: number; savedBytes: number; totalBytes: number }
  | { kind: "ended"; movedSomething: boolean; savedBytes: number; totalBytes: number };

export function parseConnectionView(value: unknown): ConnectionView | null {
  const kind = typeof value === "object" && value !== null ? (value as Fields).kind : undefined;
  if (kind === "connected") return exactly(value, ["kind"]) ? { kind } : null;
  if (kind === "waiting") {
    const f = exactly(value, ["kind", "nothingNew", "lastSeenSeconds", "savedBytes", "totalBytes"]);
    if (!f || typeof f.nothingNew !== "boolean" || !isCount(f.lastSeenSeconds) || !isCount(f.savedBytes) || !isCount(f.totalBytes)) return null;
    return { kind, nothingNew: f.nothingNew, lastSeenSeconds: f.lastSeenSeconds, savedBytes: f.savedBytes, totalBytes: f.totalBytes };
  }
  if (kind === "ended") {
    const f = exactly(value, ["kind", "movedSomething", "savedBytes", "totalBytes"]);
    if (!f || typeof f.movedSomething !== "boolean" || !isCount(f.savedBytes) || !isCount(f.totalBytes)) return null;
    return { kind, movedSomething: f.movedSomething, savedBytes: f.savedBytes, totalBytes: f.totalBytes };
  }
  return null;
}
export const moveReport = () => ask("move_report", (v) => listOf(v, parseReportItem));
/** Opens undo for one move's apps and settings (its ID from that move's result; none: the newest move): the flow guard shows the preview (R05), saying which move, then the result (R06). */
export const fullmoveUndo = (id: string | null) => run("fullmove_undo", { id });
/** The old laptop: the person's answer about the apps to close (true: try again). */
export const moveAppdataAnswer = (retry: boolean) => run("move_appdata_answer", { retry });

/** The old laptop: what it found, for the person to choose what moves (`ChoiceView`, event `move-choose`). */
export type ChoiceGroup = { role: FolderRole; files: number; bytes: number };
export type LeftBehind = { why: (typeof LEFT_OUT_REASONS)[number]; items: number; bytes: number };
/** One folder of the tree (`parts` empty for the group itself): the files counted in it. */
export type ChoiceFolder = { role: FolderRole; parts: string[]; files: number; bytes: number };
export type ChoiceView = { groups: ChoiceGroup[]; folders: ChoiceFolder[]; apps: number; leftBehind: LeftBehind[] };
/** The person's choice: the folders whose files move, and whether apps, settings and app data do. */
export type Choice = { folders: { role: FolderRole; parts: string[] }[]; apps: boolean };

export function parseChoiceView(v: unknown): ChoiceView | null {
  const f = exactly(v, ["groups", "folders", "apps", "leftBehind"]);
  if (!f || !isCount(f.apps)) return null;
  const groups = listOf(f.groups, (x) => {
    const g = exactly(x, ["role", "files", "bytes"]);
    return g && oneOf(g.role, FOLDER_ROLES) && isCount(g.files) && isCount(g.bytes) ? { role: g.role, files: g.files, bytes: g.bytes } : null;
  });
  const leftBehind = listOf(f.leftBehind, (x) => {
    const l = exactly(x, ["why", "items", "bytes"]);
    return l && oneOf(l.why, LEFT_OUT_REASONS) && isCount(l.items) && isCount(l.bytes) ? { why: l.why, items: l.items, bytes: l.bytes } : null;
  });
  const folders = listOf(f.folders, (x) => {
    const d = exactly(x, ["role", "parts", "files", "bytes"]);
    const parts = d ? listOf(d.parts, (p) => (typeof p === "string" ? p : null)) : null;
    return d && parts && oneOf(d.role, FOLDER_ROLES) && isCount(d.files) && isCount(d.bytes) ? { role: d.role, parts, files: d.files, bytes: d.bytes } : null;
  });
  return groups && folders && leftBehind ? { groups, folders, apps: f.apps, leftBehind } : null;
}
/** The old laptop: the person's choice of what moves. */
export const moveChooseWhat = (choice: Choice) => run("move_choose_what", { choice });
/** The undo the screens opened is over (the person went back). */
export const listenUndoClosed = (handler: () => void): Promise<UnlistenFn> =>
  listen<unknown>("move-undo-closed", () => {
    handler();
  });

/** Every move event in one place. A payload that does not parse ends the move on screen as a problem, never shown as sent. */
export type MoveEvent =
  | { kind: "stage"; stage: MoveStage }
  | { kind: "plan"; plan: PlanView }
  | { kind: "people"; toPlace: PeopleToPlace }
  | { kind: "privacy"; privacy: Privacy }
  | { kind: "found"; bytes: number }
  | { kind: "peer"; peer: Peer }
  | { kind: "progress"; progress: MoveProgress }
  | { kind: "finished"; outcome: MoveOutcome }
  | { kind: "problem"; problem: EndedEarly }
  | { kind: "link"; link: ConnectionView }
  | { kind: "rebuild"; step: RebuildStep }
  | { kind: "appdata"; progress: DataProgress }
  | { kind: "appdata-open"; open: AppsOpen }
  | { kind: "choose"; choice: ChoiceView };

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
    on("move-peer", parsePeer, (peer) => ({ kind: "peer", peer })),
    on("move-progress", parseMoveProgress, (progress) => ({ kind: "progress", progress })),
    on("move-finished", parseMoveOutcome, (outcome) => ({ kind: "finished", outcome })),
    on("move-problem", parseEndedEarly, (problem) => ({ kind: "problem", problem })),
    on("move-link", parseConnectionView, (link) => ({ kind: "link", link })),
    on("move-rebuild", parseRebuildStep, (step) => ({ kind: "rebuild", step })),
    on("move-appdata", parseDataProgress, (progress) => ({ kind: "appdata", progress })),
    on("move-appdata-open", parseAppsOpen, (open) => ({ kind: "appdata-open", open })),
    on("move-choose", parseChoiceView, (choice) => ({ kind: "choose", choice })),
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

/* ---------- This laptop's name (src-tauri/src/laptop_name.rs) ---------- */

/** The name this laptop keeps (null before one was chosen), and the one the naming page suggests. */
export type NameView = { kept: Label | null; suggested: Label };

export function parseNameView(value: unknown): NameView | null {
  const f = exactly(value, ["kept", "suggested"]);
  if (!f) return null;
  const kept = f.kept === null ? null : parseLabelView(f.kept);
  const suggested = parseLabelView(f.suggested);
  return suggested && (f.kept === null || kept) ? { kept, suggested } : null;
}

export const laptopName = () => ask("laptop_name", parseNameView);
/** Choose another name: a new random colour and animal, shown in place of the suggestion. */
export const laptopNameShuffle = () => ask("laptop_name_shuffle", parseLabelView);
/** Use this name: keeps the name shown. */
export const laptopNameUse = () => ask("laptop_name_use", parseLabelView);
/** Keeps a name the person typed; a name that cannot be used is refused as `pairing-name-invalid`. */
export const laptopNameType = (name: string) => ask("laptop_name_type", parseLabelView, { name });

/* ---------- Updates (src-tauri/src/updates.rs) ---------- */

/** A newer build of PCTwin: its version. */
export type Update = { version: string };

export function parseUpdate(value: unknown): Update | null {
  const f = exactly(value, ["version"]);
  return f && typeof f.version === "string" && f.version !== "" ? { version: f.version } : null;
}

/** Whether a newer build is out; null when there is none or it cannot be told. */
export const updateCheck = () =>
  ask("update_check", (v) => (v === null ? { update: null } : (() => { const u = parseUpdate(v); return u ? { update: u } : null; })())).then((r) => r.update);
/** Downloads, checks, installs the newer build and opens PCTwin again. Refused during a move. */
export const updateInstall = () => run("update_install");

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
    // The screens' own words come only as one of the known ones.
    const worded = exactly(value, ["kind", "landedAs", "why", "word"]);
    const f = worded ?? exactly(value, ["kind", "landedAs", "why"]);
    if (!f || typeof f.landedAs !== "string" || typeof f.why !== "string") return null;
    if (worded) return oneOf(f.word, NEEDS_ACTION_WORDS) ? { kind, landedAs: f.landedAs, why: f.why, word: f.word } : null;
    return { kind, landedAs: f.landedAs, why: f.why };
  }
  if (kind === "failed") {
    const f = exactly(value, ["kind", "why"]);
    return f && typeof f.why === "string" ? { kind, why: f.why } : null;
  }
  return null;
}

/** How a Quick Send ended, on one side. */
/** `newDates`: on the receiver, how many arrived files now show the arrival time (0 on the sender). */
export type QsReceipt = { side: "sender" | "receiver"; lines: Receipt["lines"]; stopped: boolean; newDates: number };

export function parseQsReceipt(value: unknown): QsReceipt | null {
  const f = exactly(value, ["side", "lines", "stopped", "newDates"]);
  if (!f || !oneOf(f.side, ["sender", "receiver"] as const) || typeof f.stopped !== "boolean" || !isCount(f.newDates)) return null;
  const lines = listOf(f.lines, (v) => {
    const l = exactly(v, ["path", "outcome"]);
    const outcome = l && parseQsOutcome(l.outcome);
    return l && typeof l.path === "string" && outcome ? { path: l.path, outcome } : null;
  });
  return lines ? { side: f.side, lines, stopped: f.stopped, newDates: f.newDates } : null;
}

const QS_PHASES = ["copying", "paused", "checking", "needs-action", "lost", "complete"] as const;

export function parseQsProgress(value: unknown): SendProgress | null {
  const f = exactly(value, ["phase", "filesDone", "filesTotal", "bytesDone", "bytesTotal", "current", "failed", "speed", "minutesLeft"]);
  if (!f || !oneOf(f.phase, QS_PHASES)) return null;
  const { filesDone, filesTotal, bytesDone, bytesTotal, failed } = f;
  if (!isCount(filesDone) || !isCount(filesTotal) || !isCount(bytesDone) || !isCount(bytesTotal) || !isCount(failed)) return null;
  if (!(f.current === null || typeof f.current === "string")) return null;
  const speed = f.speed === null ? null : parseRange(f.speed);
  if (f.speed !== null && speed === null) return null;
  const minutesLeft = f.minutesLeft === null ? null : parseRange(f.minutesLeft);
  if (f.minutesLeft !== null && minutesLeft === null) return null;
  return { phase: f.phase, filesDone, filesTotal, bytesDone, bytesTotal, current: f.current, failed, speed, minutesLeft };
}

/** Where received files go: the folder's own name, the whole folder (for More details), and its free space. */
export type QsFolder = { name: string; path: string; freeBytes: number | null };

export function parseQsFolder(value: unknown): QsFolder | null {
  const f = exactly(value, ["name", "path", "freeBytes"]);
  return f && typeof f.name === "string" && typeof f.path === "string" && isCountOrNull(f.freeBytes) ? { name: f.name, path: f.path, freeBytes: f.freeBytes } : null;
}

/** Why a Quick Send could not go on; "no-room": the receiving folder had no room, so the offer was declined. */
export type QsProblem = { kind: "link-lost" | "failed"; detail: string } | { kind: "no-room"; needed: number; room: number };

export function parseQsProblem(value: unknown): QsProblem | null {
  const room = exactly(value, ["kind", "needed", "room"]);
  if (room) return room.kind === "no-room" && isCount(room.needed) && isCount(room.room) ? { kind: "no-room", needed: room.needed, room: room.room } : null;
  const f = exactly(value, ["kind", "detail"]);
  return f && oneOf(f.kind, ["link-lost", "failed"] as const) && typeof f.detail === "string" ? { kind: f.kind, detail: f.detail } : null;
}

export const qsPick = (folders: boolean) => ask("quicksend_pick", (v) => listOf(v, parsePicked), { folders });
export const qsClear = () => run("quicksend_clear");
/** The pick page is open (`on`) or closed: things dropped on the window are added only while it is. */
export const qsAcceptDrops = (on: boolean) => run("quicksend_accept_drops", { on });
export const qsOffer = (ids: string[]) => ask("quicksend_offer", (v) => listOf(v, parseOffered), { ids });
/** Q19: the same chosen things offered again, only what the last receipt says did not arrive. */
export const qsOfferMissing = () => ask("quicksend_offer_missing", (v) => listOf(v, parseOffered));
export const qsSend = () => run("quicksend_send");
export const qsReceive = () => run("quicksend_receive");
/** The folder the next receive saves into, held from now: the one chosen this session, or Downloads. */
export const qsFolder = () => ask("quicksend_folder", parseQsFolder);
/** Change: a folder from the system's picker, used from now on this session ({ none } when none was chosen). */
export const qsChooseFolder = () => ask("quicksend_choose_folder", (v) => (v === null ? { none: true as const } : parseQsFolder(v)));
/** Save: what arrived goes into the folder shown now (Downloads, or the one chosen); nothing is saved before this. */
export const qsSaveHere = () => run("quicksend_save_here");
/** Opens the folder the last receive saved into, in the system's file manager (never a file). */
export const qsOpenFolder = () => run("quicksend_open_folder");
/** Keep original dates: how many of the last receive's files got their original dates back. */
export const qsKeepOriginalDates = () => ask("quicksend_keep_original_dates", (v) => (isCount(v) ? v : null));
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

/* ---------- Quick Send stage 1: notes, remembered connections, waiting sends (src-tauri/src/remembered.rs) ---------- */

/** Why a Quick Send stage 1 command was refused (`QsRefusal`), as a translation key. */
export const QS_REFUSALS = [
  "not-paired",
  "not-waiting",
  "failed",
  "note-empty",
  "note-too-long",
  "note-too-many-lines",
  "note-hidden",
  "no-link",
  "not-found",
  "restored",
  "bad-name",
  "store-unavailable",
  "too-many-connections",
  "folder-runs-at-sign-in",
  "folder-shell-start-up",
  "folder-holds-keys",
  "folder-system",
  "folder-not-plain",
  "folder-not-the-same",
  "not-nearby",
  "over-cap",
  "no-room",
  "bad-days",
  "bad-recipients",
  "too-many-files",
  "changed-while-copying",
] as const;
export type QsRefusal = { kind: (typeof QS_REFUSALS)[number] };

export function parseQsRefusal(value: unknown): QsRefusal | null {
  return parseKindOnly(value, QS_REFUSALS);
}

/** A rejected Quick Send stage 1 command: the engine's reason, or "failed" for anything else. */
export function toQsRefusal(error: unknown): QsRefusal {
  return parseQsRefusal(error) ?? { kind: "failed" };
}

function isTextOrNull(v: unknown): v is string | null {
  return v === null || typeof v === "string";
}

/**
 * A note or link that arrived: shown as text with Copy; a link has its site, and opens only by the person (Open, which
 * names this note's `id`: a later note never takes its place under the cursor).
 */
export type QsNote = { id: string; text: string; kind: "text" | "link"; host: string | null; from: string | null };

export function parseQsNote(value: unknown): QsNote | null {
  const f = exactly(value, ["id", "text", "kind", "host", "from"]);
  if (!f || typeof f.id !== "string" || typeof f.text !== "string" || !oneOf(f.kind, ["text", "link"] as const) || !isTextOrNull(f.host) || !isTextOrNull(f.from)) return null;
  // A link always names its site; plain text never does.
  if ((f.kind === "link") !== (typeof f.host === "string")) return null;
  return { id: f.id, text: f.text, kind: f.kind, host: f.host, from: f.from };
}

/** The other laptop took the note (it arrived and passed its checks; never that it was read). */
export function parseQsNoteSent(value: unknown): { taken: true } | null {
  const f = exactly(value, ["taken"]);
  return f && f.taken === true ? { taken: true } : null;
}

/** A connection already remembered, that a new one may replace. */
export type RememberChoice = { id: string; label: string; lastUsed: number };

/**
 * "Remember this connection" may be offered; `fallbackStore`: the keys would be kept in a file (Linux), said plainly.
 * `connections`: those already remembered, newest use first. Pairing the same laptop again makes fresh keys, so only
 * the person can say which one this replaces ([`suggestReplacement`]).
 */
export type QsRemember = { fallbackStore: boolean; connections: RememberChoice[] };

export function parseQsRemember(value: unknown): QsRemember | null {
  const f = exactly(value, ["fallbackStore", "connections"]);
  if (!f || typeof f.fallbackStore !== "boolean") return null;
  const connections = listOf(f.connections, (v) => {
    const c = exactly(v, ["id", "label", "lastUsed"]);
    return c && typeof c.id === "string" && typeof c.label === "string" && isCount(c.lastUsed) ? { id: c.id, label: c.label, lastUsed: c.lastUsed } : null;
  });
  return connections ? { fallbackStore: f.fallbackStore, connections } : null;
}

/** The remembered connection to suggest replacing: the one with the name the person typed (case and spaces aside). */
export function suggestReplacement(connections: RememberChoice[], typed: string): RememberChoice | null {
  const name = typed.trim().toLowerCase();
  if (name === "") return null;
  return connections.find((c) => c.label.trim().toLowerCase() === name) ?? null;
}

export type KnownGrant = { name: string; path: string; endsAt: number; ended: boolean };
export const KNOWN_DEVICES = ["same", "restored", "cannot-tell"] as const;
export type KnownDevice = (typeof KNOWN_DEVICES)[number];
export type KnownConnection = {
  id: string;
  label: string;
  madeAt: number;
  lastUsed: number;
  expiresAt: number;
  /** `same`: the same laptop as before; `restored`: restored or copied (connect again); `cannot-tell`: said plainly, never "same device". */
  device: KnownDevice;
  folder: KnownGrant | null;
  nextFolder: { name: string; path: string } | null;
};

function parseKnownGrant(value: unknown): KnownGrant | null {
  const f = exactly(value, ["name", "path", "endsAt", "ended"]);
  return f && typeof f.name === "string" && typeof f.path === "string" && isCount(f.endsAt) && typeof f.ended === "boolean"
    ? { name: f.name, path: f.path, endsAt: f.endsAt, ended: f.ended }
    : null;
}

function parseNamedFolder(value: unknown): { name: string; path: string } | null {
  const f = exactly(value, ["name", "path"]);
  return f && typeof f.name === "string" && typeof f.path === "string" ? { name: f.name, path: f.path } : null;
}

/** A remembered connection: "the same device as before", never a proven person. */
export function parseKnownConnection(value: unknown): KnownConnection | null {
  const f = exactly(value, ["id", "label", "madeAt", "lastUsed", "expiresAt", "device", "folder", "nextFolder"]);
  if (!f || typeof f.id !== "string" || typeof f.label !== "string" || !oneOf(f.device, KNOWN_DEVICES)) return null;
  if (!isCount(f.madeAt) || !isCount(f.lastUsed) || !isCount(f.expiresAt)) return null;
  const folder = f.folder === null ? null : parseKnownGrant(f.folder);
  if (f.folder !== null && folder === null) return null;
  const nextFolder = f.nextFolder === null ? null : parseNamedFolder(f.nextFolder);
  if (f.nextFolder !== null && nextFolder === null) return null;
  return { id: f.id, label: f.label, madeAt: f.madeAt, lastUsed: f.lastUsed, expiresAt: f.expiresAt, device: f.device, folder, nextFolder };
}

export type KnownIncoming = { connection: string; label: string; files: OfferedFile[]; bytes: number; folder: QsFolder | null };

/** Files from a remembered laptop, waiting for Save or Decline. */
export function parseKnownIncoming(value: unknown): KnownIncoming | null {
  const f = exactly(value, ["connection", "label", "files", "bytes", "folder"]);
  if (!f || typeof f.connection !== "string" || typeof f.label !== "string" || !isCount(f.bytes)) return null;
  const files = listOf(f.files, parseOffered);
  const folder = f.folder === null ? null : parseQsFolder(f.folder);
  if (!files || (f.folder !== null && folder === null)) return null;
  return { connection: f.connection, label: f.label, files, bytes: f.bytes, folder };
}

export type KnownDelivered = { connection: string; label: string; folder: string; lines: Receipt["lines"]; tell: boolean };

/** Files that landed under "allow future sends into this folder"; `tell`: the first after a quiet spell. */
export function parseKnownDelivered(value: unknown): KnownDelivered | null {
  const f = exactly(value, ["connection", "label", "folder", "lines", "tell"]);
  if (!f || typeof f.connection !== "string" || typeof f.label !== "string" || typeof f.folder !== "string" || typeof f.tell !== "boolean") return null;
  const lines = listOf(f.lines, (v) => {
    const l = exactly(v, ["path", "outcome"]);
    const outcome = l && parseQsOutcome(l.outcome);
    return l && typeof l.path === "string" && outcome ? { path: l.path, outcome } : null;
  });
  return lines ? { connection: f.connection, label: f.label, folder: f.folder, lines, tell: f.tell } : null;
}

export type WaitingDelivery =
  | { kind: "waiting" }
  | { kind: "delivered"; arrived: number; needsAction: number; at: number }
  | { kind: "declined"; at: number }
  | { kind: "failed"; failed: number; why: string; at: number };

function parseWaitingDelivery(value: unknown): WaitingDelivery | null {
  const kind = typeof value === "object" && value !== null ? (value as Fields).kind : undefined;
  if (kind === "waiting") return exactly(value, ["kind"]) ? { kind } : null;
  if (kind === "delivered") {
    const f = exactly(value, ["kind", "arrived", "needsAction", "at"]);
    return f && isCount(f.arrived) && isCount(f.needsAction) && isCount(f.at) ? { kind, arrived: f.arrived, needsAction: f.needsAction, at: f.at } : null;
  }
  if (kind === "declined") {
    const f = exactly(value, ["kind", "at"]);
    return f && isCount(f.at) ? { kind, at: f.at } : null;
  }
  if (kind === "failed") {
    const f = exactly(value, ["kind", "failed", "why", "at"]);
    return f && isCount(f.failed) && typeof f.why === "string" && isCount(f.at) ? { kind, failed: f.failed, why: f.why, at: f.at } : null;
  }
  return null;
}

export type WaitingSend = {
  id: string;
  madeAt: number;
  deadline: number;
  files: number;
  bytes: number;
  recipients: { connection: string; label: string | null; delivery: WaitingDelivery }[];
  newer: string[];
};

/** A waiting send: each recipient's own result; `newer`: files changed since approval ("review again"). */
export function parseWaitingSend(value: unknown): WaitingSend | null {
  const f = exactly(value, ["id", "madeAt", "deadline", "files", "bytes", "recipients", "newer"]);
  if (!f || typeof f.id !== "string" || !isCount(f.madeAt) || !isCount(f.deadline) || !isCount(f.files) || !isCount(f.bytes)) return null;
  const recipients = listOf(f.recipients, (v) => {
    const r = exactly(v, ["connection", "label", "delivery"]);
    const delivery = r && parseWaitingDelivery(r.delivery);
    return r && typeof r.connection === "string" && isTextOrNull(r.label) && delivery ? { connection: r.connection, label: r.label, delivery } : null;
  });
  const newer = listOf(f.newer, (v) => (typeof v === "string" ? v : null));
  return recipients && newer ? { id: f.id, madeAt: f.madeAt, deadline: f.deadline, files: f.files, bytes: f.bytes, recipients, newer } : null;
}

const knownList = (v: unknown) => listOf(v, parseKnownConnection);
const waitingList = (v: unknown) => listOf(v, parseWaitingSend);

/** Sends a note or link over the paired link; `quicksend-note-sent` says it was taken. */
export const qsSendNote = (text: string) => run("quicksend_send_note", { text });
/** Open, on a received link: the system's browser opens the note shown as `id`, once (the screens never name the address). */
export const qsOpenLink = (id: string) => run("quicksend_open_link", { id });
/** Files the system handed PCTwin to send (Send to, a file manager entry, the Mac open action): the whole chosen list. */
export const qsLaunchFiles = () => ask("quicksend_launch_files", (v) => listOf(v, parsePicked));
export const knownConnections = () => ask("known_list", knownList);
/**
 * Remember this connection, by the name the person gives it, from the last six-digit exchange. `replaces`: the
 * connection the person says this laptop was before, forgotten first (its key, permissions and waiting deliveries).
 */
export const knownRemember = (label: string, replaces: string | null = null) => ask("known_remember", knownList, { label, replaces });
/** Forget: its key and every permission deleted, its waiting deliveries and transfers ended. */
export const knownForget = (id: string) => ask("known_forget", knownList, { id });
export const knownRename = (id: string, label: string) => ask("known_rename", knownList, { id, label });
/** Allow future sends into a folder chosen in the system's picker ({ none } when none was chosen). */
export const knownAllowFolder = (id: string) => ask("known_allow_folder", (v) => (v === null ? { none: true as const } : knownList(v)), { id });
export const knownRevokeFolder = (id: string) => ask("known_revoke_folder", knownList, { id });
export const knownRenewFolder = (id: string) => ask("known_renew_folder", knownList, { id });
/** Use the folder the last receive saved into for this connection next time. */
export const knownNextFolder = (id: string) => ask("known_next_folder", knownList, { id });
export const knownClearNextFolder = (id: string) => ask("known_clear_next_folder", knownList, { id });
/** Save files from a remembered laptop into the folder held now; `nextTime`: remember that folder for it. */
export const knownSaveIncoming = (nextTime: boolean) => run("known_save_incoming", { nextTime });
export const knownDeclineIncoming = () => run("known_decline_incoming");
/** Sends the offer made on the pick page to a remembered laptop nearby, now. */
export const knownSend = (id: string) => run("known_send", { id });
/** Approve this delivery only: the offer made on the pick page, for these connections, for `days` days. */
export const waitingPrepare = (ids: string[], days: number) => ask("waiting_prepare", waitingList, { ids, days });
export const waitingSends = () => ask("waiting_list", waitingList);
export const waitingRenew = (id: string, days: number) => ask("waiting_renew", waitingList, { id, days });
/** Cancel a waiting send: its kept copy is deleted. */
export const waitingCancel = (id: string) => ask("waiting_cancel", waitingList, { id });

export type KnownEvent =
  | { kind: "note"; note: QsNote }
  | { kind: "note-sent" }
  | { kind: "remember"; fallbackStore: boolean; connections: RememberChoice[] }
  | { kind: "launch"; items: PickedItem[] }
  | { kind: "incoming"; incoming: KnownIncoming }
  | { kind: "delivered"; delivered: KnownDelivered }
  | { kind: "connections"; connections: KnownConnection[] }
  | { kind: "waiting"; sends: WaitingSend[] }
  | { kind: "unreadable" };

export async function listenKnown(handler: (event: KnownEvent) => void): Promise<UnlistenFn> {
  const on = <T,>(name: string, parse: (v: unknown) => T | null, wrap: (v: T) => KnownEvent) =>
    listen<unknown>(name, (e) => {
      const parsed = parse(e.payload);
      handler(parsed === null ? { kind: "unreadable" } : wrap(parsed));
    });
  const all = await Promise.all([
    on("quicksend-note", parseQsNote, (note) => ({ kind: "note", note })),
    on("quicksend-note-sent", parseQsNoteSent, () => ({ kind: "note-sent" })),
    on("quicksend-remember", parseQsRemember, (r) => ({ kind: "remember", fallbackStore: r.fallbackStore, connections: r.connections })),
    on("quicksend-launch", (v) => listOf(v, parsePicked), (items) => ({ kind: "launch", items })),
    on("known-incoming", parseKnownIncoming, (incoming) => ({ kind: "incoming", incoming })),
    on("known-delivered", parseKnownDelivered, (delivered) => ({ kind: "delivered", delivered })),
    on("known-changed", knownList, (connections) => ({ kind: "connections", connections })),
    on("waiting-changed", waitingList, (sends) => ({ kind: "waiting", sends })),
  ]);
  return () => {
    for (const stop of all) stop();
  };
}

/* ---------- History (src-tauri/src/history.rs) ---------- */

/** One move done on this laptop, as the app recorded it when it ended. Quick Send keeps nothing. */
export type HistoryEntryRaw = {
  id: string;
  kind: "move";
  when: string;
  other: Label | null;
  /** The new laptop's name, when known. */
  newName: Label | null;
  /** The move's undo book, when its apps and settings can be undone. */
  undoId: string | null;
  state: "finished" | "steps-left" | "stopped";
  stepsLeft: number;
};

export function parseHistoryEntry(value: unknown): HistoryEntryRaw | null {
  const f = exactly(value, ["id", "kind", "when", "other", "newName", "undoId", "state", "stepsLeft"]);
  if (!f || typeof f.id !== "string" || typeof f.when !== "string" || !isCount(f.stepsLeft)) return null;
  if (!oneOf(f.kind, ["move"] as const) || !oneOf(f.state, ["finished", "steps-left", "stopped"] as const)) return null;
  const other = f.other === null ? null : parseLabelView(f.other);
  if (f.other !== null && other === null) return null;
  const newName = f.newName === null ? null : parseLabelView(f.newName);
  if (f.newName !== null && newName === null) return null;
  if (f.undoId !== null && !(typeof f.undoId === "string" && /^[0-9a-f]{32}$/.test(f.undoId))) return null;
  return { id: f.id, kind: f.kind, when: f.when, other, newName, undoId: f.undoId, state: f.state, stepsLeft: f.stepsLeft };
}

/** What an entry kept: the move's report. */
export type Kept = { kind: "report"; items: ReportItem[] };

export function parseKept(value: unknown): Kept | null {
  const kind = typeof value === "object" && value !== null ? (value as Fields).kind : undefined;
  if (kind === "report") {
    const f = exactly(value, ["kind", "items"]);
    const items = f && listOf(f.items, parseReportItem);
    return items ? { kind, items } : null;
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
/** The mini box has no frame: dragging its body moves it (the app starts the drag). */
export const windowDrag = (): Promise<void> => runInApp("window_drag", {});

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

/* ---------- The flow guard (src-tauri/src/guard.rs): the Phase 2 screens ---------- */

/** What the engine shows for its job now. A reply that does not parse is "status unavailable". */
export async function guardView(): Promise<Snapshot> {
  try {
    return parseSnapshot(await send("guard_view")) ?? { kind: "unavailable" };
  } catch {
    return { kind: "unavailable" };
  }
}

/**
 * Sends one intent, bound to the job, session and plan generation shown; the engine checks it again.
 * A refusal comes back as its key only; anything unexpected is "status unavailable". Nothing is logged.
 */
export async function guardIntent(intent: { job: string; epoch: number; generation: number; press: string; intent: IntentKind }): Promise<{ ok: true; answer: Answer } | { ok: false; refusal: Refusal }> {
  try {
    const answer = parseAnswer(await send("guard_intent", { intent }));
    return answer ? { ok: true, answer } : { ok: false, refusal: "unavailable" };
  } catch (error: unknown) {
    return { ok: false, refusal: parseRefusal(error) ?? "unavailable" };
  }
}

/** The engine's updates for the Phase 2 screens. One that does not parse asks for the view again. */
export const listenGuard = (handler: (view: Projection | null) => void): Promise<UnlistenFn> =>
  listen<unknown>("guard-view", (e) => {
    handler(parseProjection(e.payload));
  });
