/**
 * Pairing for the screens (C03 to C05): typed wrappers over the engine's commands and events
 * (src-tauri/src/pairing.rs). The engine sends plain data only: word keys for a label, numbers,
 * and problems as translation keys. All wording comes from the translation files.
 */
import { useLocalization, type ReactLocalization } from "@fluent/react";
import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";

export type Label = { kind: "picked"; colour: string; animal: string } | { kind: "named"; name: string };

export type OldEvent =
  | { kind: "label"; label: Label }
  | { kind: "code"; code: string; secondsLeft: number }
  | { kind: "wait"; secondsLeft: number }
  | { kind: "locked" }
  | { kind: "choose"; round: number; numbers: number[] }
  | { kind: "paired" }
  | { kind: "timed-out" }
  | { kind: "left" };

export type Problem = { key: string; networks: string[] };

export type NewEvent = { kind: "paired" } | { kind: "problem"; problem: Problem };

export type Nearby = { index: number; label: Label; sameLabelNearby: boolean };

/** What anything unexpected becomes: the generic "something went wrong, try again" text. */
export const FAILED: Problem = { key: "pairing-failed", networks: [] };

/** A rejected command is the engine's `{key, networks}`; anything else is the generic problem. */
export function toProblem(error: unknown): Problem {
  if (typeof error === "object" && error !== null && "key" in error && typeof error.key === "string" && error.key !== "") {
    const networks = "networks" in error && Array.isArray(error.networks) ? error.networks.filter((n): n is string => typeof n === "string") : [];
    return { key: error.key, networks };
  }
  return FAILED;
}

/** A label as words: "Blue Fox" in the chosen language, or the typed name (or "unnamed"). */
export function labelText(l10n: ReactLocalization, label: Label): string {
  if (label.kind === "picked") {
    return l10n.getString("label-picked", {
      colour: l10n.getString(`label-colour-${label.colour}`),
      animal: l10n.getString(`label-animal-${label.animal}`),
    });
  }
  return label.name.trim() === "" ? l10n.getString("label-unnamed") : label.name;
}

export function useLabelText(label: Label | null): string {
  const { l10n } = useLocalization();
  return label ? labelText(l10n, label) : "";
}

/* ---------- Commands ---------- */

export const oldStart = (network: string | null) => invoke<undefined>("pairing_old_start", { network });
/** A pick or a decline names the round of numbers it answers, so a late one never applies to the next. */
export const oldPick = (round: number, number: number) => invoke<undefined>("pairing_old_pick", { round, number });
export const oldDecline = (round: number) => invoke<undefined>("pairing_old_decline", { round });
export const oldShuffle = () => invoke<undefined>("pairing_old_shuffle");
export const oldRename = (name: string) => invoke<undefined>("pairing_old_rename", { name });
export const oldStartAgain = () => invoke<undefined>("pairing_old_start_again");
export const stop = () => invoke<undefined>("pairing_stop");
export const newSearch = () => invoke<Nearby[]>("pairing_new_search");
export const newConnect = (index: number, code: string) => invoke<number>("pairing_new_connect", { index, code });

/* ---------- Events ---------- */

export const listenOld = (handler: (event: OldEvent) => void): Promise<UnlistenFn> =>
  listen<OldEvent>("pairing-old", (e) => {
    handler(e.payload);
  });

export const listenNew = (handler: (event: NewEvent) => void): Promise<UnlistenFn> =>
  listen<NewEvent>("pairing-new", (e) => {
    handler(e.payload);
  });
