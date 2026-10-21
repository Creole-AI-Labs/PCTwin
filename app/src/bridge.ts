/**
 * The bridge: the only file that talks to the desktop shell (src-tauri). Every command is a typed
 * function here, and everything the shell sends back (a command's reply, a rejection, an event) is
 * read by a parse function that accepts exactly the engine's shape and refuses anything else. The
 * screens get parsed values only; a value that does not parse becomes the screens' usual failure
 * (the generic "try again" problem), never shown as it came.
 *
 * The shapes are the engine's (src-tauri/src/pairing.rs, crates/pctwin-gate read_back.rs). The
 * engine writes one example of each into src/contract/*.json, and contract.test.ts parses every
 * example with these functions, so a change on one side that the other does not follow fails.
 *
 * What the shell lets the screens call is a closed list (src-tauri/build.rs and
 * capabilities/default.json); a command not on it is refused by the shell.
 */
import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { WHY_MEMORY, type Evidence, type WhyMemory } from "./move";

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
