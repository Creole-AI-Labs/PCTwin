/**
 * Playing a motion: the one door every screen's motion goes through. Presentation only.
 *
 * - Nothing plays when motion is off, the page is hidden, or the Web Animations API is missing.
 * - Reduced motion keeps opacity only: no travel, scale or rotation, and no stagger delay.
 * - Only transform and opacity are ever animated, and an animation never holds a final state
 *   (`fill` is never "forwards"), so cancelling one leaves the screen exactly as it is.
 * - Nothing ever waits for an animation: no business action runs on `finished`.
 */
import { motionMode } from "./preference";
import { DURATION, EASING } from "./tokens";

export interface PlayOptions {
  duration: number;
  delay?: number;
  easing?: string;
}

const active = new Set<Animation>();
const ALLOWED = new Set(["opacity", "transform", "offset", "easing", "composite"]);

/** Play keyframes on an element under the motion rules; null when nothing plays. */
export function play(el: Element | null | undefined, frames: Keyframe[], { duration, delay = 0, easing = EASING.settle }: PlayOptions): Animation | null {
  const mode = motionMode();
  if (!el || mode === "off" || typeof el.animate !== "function" || document.hidden) return null;
  let keyframes = frames;
  if (mode === "reduced") {
    keyframes = frames.map((k) => Object.fromEntries(Object.entries(k).filter(([p]) => p !== "transform")));
    if (!keyframes.some((k) => "opacity" in k)) return null;
    delay = 0;
  }
  for (const k of keyframes) {
    for (const p of Object.keys(k)) if (!ALLOWED.has(p)) throw new Error(`motion animates transform and opacity only, not ${p}`);
  }
  const a = el.animate(keyframes, { duration, delay, easing, fill: "backwards" });
  active.add(a);
  const forget = () => active.delete(a);
  a.finished.then(forget, forget);
  return a;
}

/** Stop every motion now (a new screen, a hidden window, a private thing going away). */
export function cancelAll(): void {
  for (const a of active) {
    try {
      a.cancel();
    } catch {
      // Already gone.
    }
  }
  active.clear();
}

/** How many motions are playing (tests and checks). */
export function playing(): number {
  return active.size;
}

/** The short restoration on an ordinary start or a return to the window: a quick fade in. */
export function restoreReveal(el: Element | null): Animation | null {
  return play(el, [{ opacity: 0 }, { opacity: 1 }], { duration: DURATION.quick });
}
