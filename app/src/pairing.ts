/**
 * Pairing for the screens (C03 to C05): the engine's commands and events (src-tauri/src/pairing.rs)
 * come through bridge.ts, which checks every value's shape. The engine sends plain data only: word
 * keys for a label, numbers, and problems as translation keys. All wording comes from the
 * translation files.
 */
import { useLocalization, type ReactLocalization } from "@fluent/react";
import {
  FAILED,
  listenPairingNew,
  listenPairingOld,
  pairingHeadsUp,
  pairingNewConnect,
  pairingNewSearch,
  pairingOldDecline,
  pairingOldPick,
  pairingOldRename,
  pairingOldShuffle,
  pairingOldStart,
  pairingOldStartAgain,
  pairingStop,
  toProblem,
  type HeadsUp,
  type Label,
  type Nearby,
  type NewEvent,
  type OldEvent,
  type PairRole,
  type Problem,
} from "./bridge";

export { FAILED, toProblem };
export type { HeadsUp, Label, Nearby, NewEvent, OldEvent, PairRole, Problem };

/** What a lost connection shows: the connection dropped, nothing was shared, try again. */
export const LOST: Problem = { key: "pairing-connection-lost", networks: [] };

/** The same on the new laptop, which looks for the old laptop again by itself (no Try again there). */
export const LOST_NEW: Problem = { key: "pairing-connection-lost-new", networks: [] };

/** A problem with no words of its own, on the new laptop, which looks again by itself (no Try again there). */
export const FAILED_NEW: Problem = { key: "pairing-failed-new", networks: [] };

/** A label as words: "Blue Fox" in the chosen language, or the typed name (or "unnamed"). */
export function labelText(l10n: ReactLocalization, label: Label): string {
  if (label.kind === "picked") {
    // Some languages make the colour word agree with the animal word ("Renard bleu", "Panthère
    // noire"). Those files carry a `label-gender-<animal>` message; the colour words read it as $gender.
    const genderId = `label-gender-${label.animal}`;
    const gender = l10n.getBundle(genderId) ? l10n.getString(genderId) : "";
    return l10n.getString("label-picked", {
      colour: l10n.getString(`label-colour-${label.colour}`, { gender }),
      animal: l10n.getString(`label-animal-${label.animal}`),
    });
  }
  return label.name.trim() === "" ? l10n.getString("label-unnamed") : label.name;
}

export function useLabelText(label: Label | null): string {
  const { l10n } = useLocalization();
  return label ? labelText(l10n, label) : "";
}

/* ---------- Commands and events (bridge.ts) ---------- */

export const oldStart = pairingOldStart;
export const oldPick = pairingOldPick;
export const oldDecline = pairingOldDecline;
export const oldShuffle = pairingOldShuffle;
export const oldRename = pairingOldRename;
export const oldStartAgain = pairingOldStartAgain;
export const stop = pairingStop;
export const newSearch = pairingNewSearch;
export const newConnect = pairingNewConnect;
export const listenOld = listenPairingOld;
export const listenNew = listenPairingNew;

/* ---------- Notes before pairing starts ---------- */

/** What each note says. */
export const HEADS_UP_TEXT: Record<HeadsUp["kind"], string> = {
  "local-network": "pair-heads-up-local-network",
  firewall: "pair-heads-up-firewall",
};

const SEEN_KEY = "pctwin.headsUpSeen";

/** The notes already shown on this laptop (the system asks its question once). */
function seenNotes(): string[] {
  try {
    const v: unknown = JSON.parse(localStorage.getItem(SEEN_KEY) ?? "[]");
    return Array.isArray(v) ? v.filter((k): k is string => typeof k === "string") : [];
  } catch {
    return [];
  }
}

/** The person read these notes and went on. */
export function markHeadsUpSeen(notes: HeadsUp[]): void {
  try {
    localStorage.setItem(SEEN_KEY, JSON.stringify([...new Set([...seenNotes(), ...notes.map((n) => n.kind)])]));
  } catch {
    // Not kept: the note shows again next time, which does no harm.
  }
}

/**
 * The notes to show before pairing starts as `role` that this laptop has not shown before. When
 * asking fails there are none: the system's own question still says what it is about.
 */
export async function unseenHeadsUp(role: PairRole): Promise<HeadsUp[]> {
  try {
    const seen = seenNotes();
    return (await pairingHeadsUp(role)).filter((n) => !seen.includes(n.kind));
  } catch {
    return [];
  }
}
