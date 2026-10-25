/**
 * The motion setting: the person's choice (follow the system, reduced, off) and what it comes to
 * with the system's own reduced-motion setting. The system asking for less motion always wins over
 * "follow the system"; reduced and off are never overridden by the system.
 *
 * - full: the designed motion.
 * - reduced: no travel, scale, rotation or stagger; a short fade at most.
 * - off: no optional motion at all.
 *
 * Every mode shows the same words, controls, status and focus: motion is presentation only.
 * The choice is kept on this laptop (localStorage, wrapped: a blocked store means "follow the system").
 */
import { useSyncExternalStore } from "react";
import { cssVariables } from "./tokens";

export const PREFERENCES = ["system", "reduced", "off"] as const;
export type MotionPreference = (typeof PREFERENCES)[number];
export type MotionMode = "full" | "reduced" | "off";

const STORE_KEY = "pctwin.motion";
const QUERY = "(prefers-reduced-motion: reduce)";

/** What a preference and the system setting come to. */
export function modeFor(preference: MotionPreference, systemReduced: boolean): MotionMode {
  if (preference === "off") return "off";
  if (preference === "reduced" || systemReduced) return "reduced";
  return "full";
}

function media(): MediaQueryList | null {
  try {
    return typeof window.matchMedia === "function" ? window.matchMedia(QUERY) : null;
  } catch {
    return null;
  }
}

/** Whether the system asks for reduced motion (false where it cannot be read). */
export function systemReduced(): boolean {
  return media()?.matches === true;
}

export function savedPreference(): MotionPreference {
  try {
    const v = localStorage.getItem(STORE_KEY);
    return PREFERENCES.find((p) => p === v) ?? "system";
  } catch {
    return "system";
  }
}

let preference: MotionPreference | null = null;
let reducedBySystem = false;
let listening = false;
const listeners = new Set<() => void>();

function current(): MotionPreference {
  if (preference === null) {
    preference = savedPreference();
    reducedBySystem = systemReduced();
  }
  return preference;
}

/** The mode now. Read before any decorative frame. */
export function motionMode(): MotionMode {
  return modeFor(current(), reducedBySystem);
}

export function motionPreference(): MotionPreference {
  return current();
}

export function setMotionPreference(next: MotionPreference): void {
  preference = next;
  try {
    localStorage.setItem(STORE_KEY, next);
  } catch {
    // Not kept: the system setting is followed next time.
  }
  changed();
}

/** Forget the cached choice and read the store and the system again (tests, and a fresh start). */
export function resetMotion(): void {
  preference = null;
  changed();
}

function changed(): void {
  applyToDocument();
  listeners.forEach((l) => {
    l();
  });
}

function subscribe(listener: () => void): () => void {
  listeners.add(listener);
  if (!listening) {
    listening = true;
    media()?.addEventListener("change", (e) => {
      reducedBySystem = e.matches;
      changed();
    });
  }
  return () => {
    listeners.delete(listener);
  };
}

/**
 * Write the mode and the tokens onto the page: `data-motion` on the root element, and every
 * `--motion-*` duration (zero when the mode is not full, so CSS transitions and animations stop).
 */
export function applyToDocument(): void {
  if (typeof document === "undefined") return;
  const root = document.documentElement;
  const mode = motionMode();
  root.dataset.motion = mode;
  for (const [name, value] of Object.entries(cssVariables())) {
    root.style.setProperty(name, mode === "full" || !value.endsWith("ms") ? value : "0ms");
  }
}

/** The motion setting, for a component: re-renders when the choice or the system setting changes. */
export function useMotion(): { mode: MotionMode; preference: MotionPreference; setPreference: (p: MotionPreference) => void } {
  const mode = useSyncExternalStore(subscribe, motionMode, motionMode);
  const pref = useSyncExternalStore(subscribe, motionPreference, motionPreference);
  return { mode, preference: pref, setPreference: setMotionPreference };
}
