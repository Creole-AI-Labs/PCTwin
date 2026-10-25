/**
 * When the Wordplay opening plays. The full opening plays on the first launch and after an update
 * only (decided 24 October 2026); every other start gets a short restoration. The version last
 * shown is kept on this laptop (localStorage, wrapped: a blocked store only means the opening may
 * play again).
 *
 * The opening is presentation. It is decided once per start, never waits on the app being ready
 * and never stands for it: the app is built underneath from the first frame.
 */
import { version as APP_VERSION } from "../../src-tauri/tauri.conf.json";
import { motionMode } from "./preference";

export { APP_VERSION };

/**
 * - wordplay: the full opening (motion full, first launch or update).
 * - still: the short still identity instead (reduced motion, first launch or update).
 * - restore: a short fade back in (every other start).
 * - none: nothing (motion off, or no Web Animations to draw it with).
 */
export type OpeningKind = "wordplay" | "still" | "restore" | "none";

const STORE_KEY = "pctwin.opening.shown";

export function lastShownVersion(): string | null {
  try {
    return localStorage.getItem(STORE_KEY);
  } catch {
    return null;
  }
}

function recordShown(version: string): void {
  try {
    localStorage.setItem(STORE_KEY, version);
  } catch {
    // Not kept: the opening may play again next start.
  }
}

/** Whether this page can draw with the Web Animations API, as the pack does. */
export function canAnimate(): boolean {
  return typeof Element !== "undefined" && typeof Element.prototype.animate === "function";
}

/** The decision, from its inputs. */
export function kindFor({ shown, version, mode, animates }: { shown: string | null; version: string; mode: "full" | "reduced" | "off"; animates: boolean }): OpeningKind {
  if (!animates || mode === "off") return "none";
  if (shown !== version) return mode === "full" ? "wordplay" : "still";
  return "restore";
}

let decided: OpeningKind | null = null;

/** This start's opening, decided once (a re-render or a strict-mode double call asks again safely). */
export function openingKind(version: string = APP_VERSION): OpeningKind {
  if (decided === null) {
    decided = kindFor({ shown: lastShownVersion(), version, mode: motionMode(), animates: canAnimate() });
    recordShown(version);
  }
  return decided;
}

/** A fresh start, for tests. */
export function resetOpening(): void {
  decided = null;
}
