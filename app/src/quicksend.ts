/**
 * Quick Send for the screens: the shapes the screens draw, which mirror the engine's library
 * (crates/pctwin-quicksend) exactly, and the few rules about them that the words depend on. The
 * commands and events are in bridge.ts (src-tauri/src/quicksend.rs).
 *
 * The flow (decided 26 October 2026): the sender picks files, pairs (showing the code), and the
 * send starts the moment the laptops are paired. The receiver sees where files will be saved
 * (Downloads, or the folder it chose this session), pairs (typing the code), and receives
 * everything offered: choosing Receive, typing the code and matching the number is the acceptance
 * (Security Design T). Both end on the Done page, which says what arrived and checked, and lists
 * only what did not.
 */
import { useLocalization } from "@fluent/react";
import type { Label } from "./pairing";

export type { Label };

/** One offered file, as both sides list it (OfferedFile). `path` uses "/" between folders. */
export type OfferedFile = { id: string; path: string; size: number };

/** How one offered file ended, on either side (Outcome). */
export type Outcome =
  /** It is in the chosen folder under `landedAs` (on the sender, the name it was sent as). */
  | { kind: "arrived"; landedAs: string }
  /** An identical file was already there, so it was not sent again. */
  | { kind: "already-there"; landedAs: string }
  /** The receiver did not accept it. */
  | { kind: "declined" }
  /** It is in the folder but not finished as a proven copy; `why` says what to do. Never counted as arrived. */
  | { kind: "needs-action"; landedAs: string; why: string; word?: NeedsActionWord }
  /** It did not arrive; `why` is the engine's own short reason. */
  | { kind: "failed"; why: string };

/**
 * The screens' own words for a needs-action line, when the engine names one: "mark-not-kept", a
 * file from another laptop on a drive that could not mark it as downloaded (open it with care).
 */
export const NEEDS_ACTION_WORDS = ["mark-not-kept"] as const;
export type NeedsActionWord = (typeof NEEDS_ACTION_WORDS)[number];

export type ReceiptLine = { path: string; outcome: Outcome };

/** What a Quick Send came to, file by file, on one side (Receipt). */
export type Receipt = { lines: ReceiptLine[] };

/** Something the sender chose on this laptop: a file or a folder. Sizes are measured once it is read. */
export type PickedItem = {
  id: string;
  name: string;
  kind: "file" | "folder";
  /** Ticked: part of what will be sent. Unticking leaves it out without losing it from the list. */
  included: boolean;
  bytes: number | null;
  /** Files inside a folder; null until counted. */
  files: number | null;
  /** What it is, in the person's words, from the system's file type ("PDF document", "8 photos"); optional. */
  description?: string;
  /**
   * The engine could not use it: not there any more (NotThere), not readable, or stored online only
   * (the original is not on this laptop yet, so there is nothing to read).
   */
  problem?: "missing" | "unreadable" | "online-only";
};

/**
 * Where a send is. "complete" is only for a send whose receipt is whole (every file in place); the
 * bridge must not report it otherwise.
 */
export type SendPhase = "copying" | "paused" | "checking" | "needs-action" | "lost" | "complete";

/** Measured progress of a send; every number is counted work, never a guess. */
export type SendProgress = {
  phase: SendPhase;
  filesDone: number;
  filesTotal: number;
  bytesDone: number;
  bytesTotal: number;
  /** The path of the file in flight, if any. */
  current: string | null;
  /** Files that did not arrive so far. */
  failed: number;
  /** Observed speed as a range in bytes per second, or null when not measured yet. */
  speed: { low: number; high: number } | null;
  /** The measured time left in whole minutes, as a range, or null while it is still being worked out. */
  minutesLeft: { low: number; high: number } | null;
};

export type MoveHistoryState = "finished" | "steps-left" | "paused" | "stopped";

/**
 * One move done on this laptop, kept locally, no account. Only the bridge says "finished". Quick Send
 * keeps nothing: what was sent or received is not remembered (decided 26 October 2026). `when` is an
 * ISO 8601 date and time; `from` and `to` are the laptops' names as the person saw them.
 */
export type HistoryEntry = { id: string; kind: "move"; when: string; from: string; to: string; state: MoveHistoryState; stepsLeft?: number };

/* ---------- Rules the words depend on ---------- */

/**
 * A name as the screens may show it: characters that reorder or hide text (right-to-left and other
 * direction overrides and isolates, direction marks) and control characters are shown as their code
 * ("[U+202E]"), so a name from another laptop can never look like something else
 * ("invoice\u202Efdp.exe" would show as "invoiceexe.pdf").
 */
export function shownSafely(name: string): string {
  const hidden = (c: number) =>
    c <= 0x1f || (c >= 0x7f && c <= 0x9f) || c === 0x61c || c === 0x200e || c === 0x200f || (c >= 0x202a && c <= 0x202e) || (c >= 0x2066 && c <= 0x2069);
  let out = "";
  for (const ch of name) {
    const c = ch.codePointAt(0) ?? 0;
    out += hidden(c) ? `[U+${c.toString(16).toUpperCase().padStart(4, "0")}]` : ch;
  }
  return out;
}

export function fileName(path: string): string {
  const parts = path.split("/").filter(Boolean);
  return shownSafely(parts.length === 0 ? path : parts[parts.length - 1]);
}

/** The real type of a file by its name, as the system will treat it: what follows its last dot ("exe"), or null. */
export function extensionOf(path: string): string | null {
  const name = path.split("/").filter(Boolean).pop() ?? path;
  const at = name.lastIndexOf(".");
  return at > 0 && at < name.length - 1 ? shownSafely(name.slice(at + 1)) : null;
}

export function totalBytes(files: readonly { size: number }[]): number {
  return files.reduce((sum, f) => sum + f.size, 0);
}

export type Tally = { arrived: number; alreadyThere: number; declined: number; failed: number; total: number };

/** Counts per outcome, measured from the lines. */
export function tally(receipt: Receipt): Tally {
  const t: Tally = { arrived: 0, alreadyThere: 0, declined: 0, failed: 0, total: receipt.lines.length };
  for (const { outcome } of receipt.lines) {
    if (outcome.kind === "arrived") t.arrived++;
    else if (outcome.kind === "already-there") t.alreadyThere++;
    else if (outcome.kind === "declined") t.declined++;
    else t.failed++;
  }
  return t;
}

/**
 * Whether a handover is whole. Only when there is at least one line and every line ended with the
 * file in place (arrived, or an identical file already there). A declined or failed line, or no lines
 * at all, is never "complete".
 */
export function receiptState(receipt: Receipt): "complete" | "partial" | "empty" {
  const t = tally(receipt);
  if (t.total === 0) return "empty";
  return t.declined === 0 && t.failed === 0 ? "complete" : "partial";
}

/** The lines that still need to go: failed or not accepted. */
export function missingLines(receipt: Receipt): ReceiptLine[] {
  return receipt.lines.filter((l) => l.outcome.kind === "failed" || l.outcome.kind === "declined");
}

/** A fraction from 0 to 1 of counted bytes; 0 when nothing is to be sent. */
export function fraction(done: number, total: number): number {
  if (total <= 0) return 0;
  return Math.max(0, Math.min(1, done / total));
}

/* ---------- Sizes ---------- */

const UNITS = ["byte", "kilobyte", "megabyte", "gigabyte", "terabyte"] as const;

/** "24.8 MB" in the reader's language: powers of 1000, one decimal at most. */
export function formatBytes(bytes: number, language: string, perSecond = false): string {
  let value = Math.max(0, bytes);
  let unit = 0;
  while (value >= 1000 && unit < UNITS.length - 1) {
    value /= 1000;
    unit++;
  }
  try {
    return new Intl.NumberFormat(language || "en", {
      style: "unit",
      unit: perSecond ? `${UNITS[unit]}-per-second` : UNITS[unit],
      unitDisplay: "short",
      maximumFractionDigits: unit === 0 ? 0 : 1,
    }).format(value);
  } catch {
    return `${String(Math.round(value))} ${UNITS[unit]}`;
  }
}

/** "2 MB/s – 4 MB/s": a measured speed range, in the reader's language. */
export function formatRate(low: number, high: number, language: string): string {
  return `${formatBytes(low, language, true)} – ${formatBytes(high, language, true)}`;
}

/** A formatter for the screens: sizes in the language the page is in. */
export function useBytes(): (bytes: number) => string {
  // Reading the localization keeps the screen re-rendering when the language changes.
  useLocalization();
  const language = typeof document === "undefined" ? "en" : document.documentElement.lang;
  return (bytes) => formatBytes(bytes, language);
}

/** A date and time in the reader's language, or the text as given when it is not a date. */
export function formatWhen(iso: string, language: string): string {
  const date = new Date(iso);
  if (Number.isNaN(date.getTime())) return iso;
  try {
    return new Intl.DateTimeFormat(language || "en", { dateStyle: "medium", timeStyle: "short" }).format(date);
  } catch {
    return iso;
  }
}
