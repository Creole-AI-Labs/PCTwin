/**
 * Quick Send for the screens (Q01 to Q20 and the local history): the shapes the screens draw, which
 * mirror the engine's library (crates/pctwin-quicksend) exactly, and the few rules about them that the
 * words depend on. There is no Tauri command for Quick Send yet, so every screen is a view model: data
 * in as props, events out as callbacks, and the bridge fills them in later.
 *
 * Commands the bridge needs (names are proposals; nothing here calls them):
 *  - quicksend_pick { kind: "files" | "folders" } -> PickedItem[]
 *      Opens the system picker. Raised by Q02 "Add files" / "Add folders". Dropped paths do not come
 *      from the screen: the shell reads them from the window's drag-drop event and calls
 *      quicksend_add { paths: string[] }; it also tells Q02 when a drag is over the window (`dragging`).
 *  - Untick in Q02 is local: the bridge keeps the list and sends only the ticked items to quicksend_offer.
 *  - quicksend_offer () -> { files: OfferedFile[], issues: SourceIssue[] }
 *      Offer::of(chosen) then Offer::files(). Q03 shows it; issues are files that changed, went missing
 *      or cannot be read since they were picked.
 *  - Pairing: Send uses the existing PairOld screen (it shows the code, then the three numbers) and
 *      Receive uses PairNew (type the code, then the number is shown). The bridge starts pairing in
 *      Quick Send mode, and on the `paired` event moves to Q08 (receiver) or Q11 (sender) instead of the
 *      Paired page. This needs a small change in PairOld/PairNew: an optional `onPaired` prop and a
 *      `variant` that picks quick-send wording ("the other laptop" rather than "old"/"new").
 *  - quicksend_incoming event { from: Label, files: OfferedFile[] } -> Q08   (receive_offer)
 *  - quicksend_accept { ids: string[] | null, folder } -> Receipt   Q08 + Q09 + Q10 (Incoming::accept;
 *      `ids` is the subset the person kept ticked, null for all). quicksend_decline {} -> void
 *      (Incoming::decline).
 *  - quicksend_choose_folder () -> { path, freeBytes | null }    Q09 "Choose a folder".
 *  - quicksend_conflicts { folder, ids } -> Conflict[]            Q10 (what is already in the folder).
 *      The engine today only keeps both (a taken name gets "(2)"); Skip is "do not tick the file" in Q08;
 *      Replace does not exist: the safety gate never overwrites, so Q10 offers only Keep both and Skip.
 *  - quicksend_send { offer } -> Receipt   Q11 Start (send(ch, offer)). The sender's approval and the
 *      receiver's approval are separate; changing the selection clears the receiver's.
 *  - quicksend_progress event { phase, filesDone, filesTotal, bytesDone, bytesTotal, current, failed }
 *      -> Q12. quicksend_pause / quicksend_resume / quicksend_stop {} -> void.
 *  - quicksend_receipt event/query { side, receipt: Receipt, stopped } -> Q14, Q19, History.
 *  - quicksend_history () -> HistoryEntry[]; quicksend_history_export () -> { savedTo } -> History.
 */
import { useLocalization } from "@fluent/react";
import type { Label, Nearby } from "./pairing";

export type { Label };

/** A laptop nearby that can receive: the pairing list's entry, and whether it is busy with another send. */
export type NearbyLaptop = Nearby & { busy?: boolean };

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
  | { kind: "needs-action"; landedAs: string; why: string }
  /** It did not arrive; `why` is the engine's own short reason. */
  | { kind: "failed"; why: string };

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

/** A thing in the offer that is not as it was when the person looked (source files change). */
export type SourceIssue = { path: string; kind: "changed" | "missing" | "unreadable" };

/** A file the receiving folder already has under the offered name. */
export type Conflict = {
  path: string;
  existingBytes: number;
  offeredBytes: number;
  /** What Keep both would save the new file as ("notes (2).txt"), worked out the way the engine does. */
  newName: string;
  /** When the file already in the folder was last changed (ISO 8601), or null if unknown. */
  existingModified: string | null;
  /** Null until chosen, and again whenever the file in the folder changed since the person looked. */
  choice: "keep-both" | "skip" | null;
  /** The file in the folder changed (or could not be read) since the person looked. */
  changed?: boolean;
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
};

export type MoveHistoryState = "finished" | "steps-left" | "paused" | "stopped";
export type SendHistoryState = "finished" | "stopped" | "partial";

/**
 * One thing done on this laptop: a move, or a Quick Send either way. Kept locally, no account. Only the
 * bridge says "finished", and for a Quick Send only when the receipt is whole (see historyStateOf).
 * `when` is an ISO 8601 date and time; `from`, `to` and `peer` are the laptops' names as the person saw them.
 */
export type HistoryEntry =
  | { id: string; kind: "move"; when: string; from: string; to: string; state: MoveHistoryState; stepsLeft?: number }
  | { id: string; kind: "send" | "receive"; when: string; peer: string; state: SendHistoryState };

/* ---------- Rules the words depend on ---------- */

export function fileName(path: string): string {
  const parts = path.split("/").filter(Boolean);
  return parts.length === 0 ? path : parts[parts.length - 1];
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

/** The history state a finished Quick Send gets: never "finished" unless the receipt is whole. */
export function historyStateOf(receipt: Receipt, stopped: boolean): SendHistoryState {
  if (receiptState(receipt) === "complete") return "finished";
  return stopped ? "stopped" : "partial";
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
