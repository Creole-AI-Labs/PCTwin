/**
 * The motions every screen shares, from the Wordplay pack's six signatures and recipes:
 *
 * - Screen to screen (MC02): new content arrives from the logical trailing side over 10 px; Back
 *   reverses it; right to left mirrors both. The frame and header stay still.
 * - Paper that settles: only the first five rows of a list settle in (4 px, a short stagger); the
 *   rest are simply there. Everything is usable from the first frame.
 * - The citron bookmark: a Home choice sends a small citron mark up into the header mark.
 * - Icons with a job: the icon on a Home choice nudges on hover (fine pointers) and on keyboard
 *   activation; the text and the button never move.
 * - A calm way to leave: hiding the window stops every motion at once (no farewell); coming back
 *   is a short fade in. Screens that end something arrive without travel.
 *
 * Evidence is never delayed: a screen showing a failure or warning arrives at full opacity, rows
 * that are warnings or status are never dimmed, and nothing is ever animated on its way out (a
 * private folder that goes away is gone in the same frame).
 */
import { useLayoutEffect, useRef } from "react";
import { cancelAll, play, restoreReveal } from "./play";
import { motionMode } from "./preference";
import { DISTANCE, DURATION, STAGGER_MS, STAGGER_ROWS, at } from "./tokens";

/** Rows that settle when a screen arrives (first five only). Codes and QR are never among them. */
export const ROWS = [
  ".choice",
  ".kit-row",
  ".plan-item",
  ".plan-folder",
  ".help-row",
  ".mv-guide",
  ".mv-report tbody tr",
  ".qs-route-row",
  ".qs-rows > *",
  ".frame-step",
].join(", ");

/** Codes, QR and compared numbers (MC12, MC13): a screen showing one arrives without any motion. */
export const STILL = ".kit-code-box, .kit-code-input, .pair-tile, .pair-numbers, [data-motion-still]";

/** What counts as evidence: shown at once and at full strength. */
export const EVIDENCE = '[role="alert"], .kit-banner-attention, .qs-statuscard-warn, [data-evidence]';

let goingBack = false;

/** +1 when the reading direction runs left to right, -1 right to left. */
export function readingSign(el?: Element | null): 1 | -1 {
  const dir = el?.closest("[dir]")?.getAttribute("dir") ?? document.documentElement.dir;
  return dir === "rtl" ? -1 : 1;
}

/**
 * The horizontal start offset for arriving content: forward comes from the trailing side (the
 * right in left-to-right, the left in right-to-left); Back comes from the leading side.
 */
export function arrivalOffset(back: boolean, sign: 1 | -1): number {
  return DISTANCE.navigation * (back ? -1 : 1) * sign;
}

/** A screen arrives in `main` (the app's scrolling content). `calm`: no travel, as for an ending. */
export function settleScreen(main: HTMLElement | null, { calm = false }: { calm?: boolean } = {}): void {
  const back = goingBack;
  goingBack = false;
  // Latest wins: whatever was still settling stops now.
  cancelAll();
  if (!main) return;
  if (main.querySelector(STILL)) {
    flyBookmark();
    return;
  }
  const evidence = main.querySelector(EVIDENCE) !== null;
  const x = arrivalOffset(back, readingSign(main));
  const from: Keyframe = calm ? {} : { transform: `translateX(${String(x)}px)` };
  // A screen with a failure or warning on it is never dimmed, not even for a moment.
  if (!evidence) from.opacity = 0.6;
  if (Object.keys(from).length > 0) play(main, [from, { opacity: 1, transform: "none" }], { duration: DURATION.navigate });
  settleRows(main);
  flyBookmark();
}

/** Paper that settles: the first five rows, never evidence, never anything hidden. */
export function settleRows(root: ParentNode): void {
  const rows = [...root.querySelectorAll<HTMLElement>(ROWS)].filter((r) => !r.closest("[hidden]") && !r.matches(EVIDENCE) && !r.querySelector(EVIDENCE)).slice(0, STAGGER_ROWS);
  rows.forEach((row, i) => {
    play(row, [{ opacity: 0.6, transform: `translateY(${String(DISTANCE.row)}px)` }, { opacity: 1, transform: "none" }], { duration: DURATION.state, delay: i * STAGGER_MS });
  });
}

/** Call from a screen that changes step inside itself (`key`): the new step settles like a new screen. */
export function useSettle(key: string, options?: { calm?: boolean }): void {
  const first = useRef(true);
  const calm = options?.calm === true;
  useLayoutEffect(() => {
    if (first.current) {
      first.current = false;
      return;
    }
    settleScreen(document.querySelector<HTMLElement>(".app-main"), { calm });
  }, [key, calm]);
}

/* ---------- The citron bookmark ---------- */

let bookmarkFrom: DOMRect | null = null;

function flyBookmark(): void {
  const from = bookmarkFrom;
  bookmarkFrom = null;
  if (!from || motionMode() !== "full") return;
  const target = document.querySelector(".app-header .brand-mark")?.getBoundingClientRect();
  if (!target || target.width === 0) return;
  const mark = document.createElement("i");
  mark.className = "motion-bookmark";
  mark.setAttribute("aria-hidden", "true");
  const x = from.left + from.width / 2 - 4.5;
  const y = from.top + from.height / 2 - 4.5;
  mark.style.left = `${String(x)}px`;
  mark.style.top = `${String(y)}px`;
  document.body.append(mark);
  const dx = target.left + target.width / 2 - 4.5 - x;
  const dy = target.top + target.height / 2 - 4.5 - y;
  const a = play(
    mark,
    [
      { opacity: 0, transform: "translate(0,0) rotate(-12deg) scale(.7)" },
      { opacity: 1, transform: `translate(${String(dx * 0.25)}px,${String(dy * 0.7 - 14)}px) rotate(20deg) scale(1)`, offset: 0.4 },
      { opacity: 0, transform: `translate(${String(dx)}px,${String(dy)}px) rotate(70deg) scale(.5)` },
    ],
    { duration: DURATION.signature },
  );
  const remove = () => {
    mark.remove();
  };
  if (a) a.finished.then(remove, remove);
  else remove();
}

/* ---------- Icons with a job ---------- */

/** Each job's nudge, at its furthest point (direction-aware where it points along the reading line). */
function nudge(job: string, sign: 1 | -1): string | null {
  switch (job) {
    case "send":
      return `translate(${String(3 * sign)}px,-2px) rotate(${String(-8 * sign)}deg)`;
    case "move":
      return `translateX(${String(3 * sign)}px)`;
    case "restore":
      return "rotate(-12deg)";
    case "rescue":
      return "translateY(-2px) scale(1.06)";
    default:
      return null;
  }
}

export function iconJob(button: Element): Animation | null {
  const job = button.getAttribute("data-icon-job");
  const icon = button.querySelector("svg");
  if (!job || !icon) return null;
  const to = nudge(job, readingSign(button));
  if (!to) return null;
  // Keep the icon's own transform (a mirrored arrow stays mirrored).
  const own = getComputedStyle(icon).transform;
  const base = own && own !== "none" ? ` ${own}` : "";
  return play(icon, [{ transform: base.trim() || "none" }, { transform: `${to}${base}`, offset: 0.42 }, { transform: base.trim() || "none" }], { duration: at(230) });
}

/* ---------- Listening, once, on the document ---------- */

let installed = false;

/** Install the shared listeners once: Back cues, Home choices, icons, and leaving and coming back. */
export function installScreenMotion(): void {
  if (installed || typeof document === "undefined") return;
  installed = true;
  document.addEventListener(
    "click",
    (e) => {
      const el = e.target instanceof Element ? e.target : null;
      const button = el?.closest("button, a");
      if (!button) return;
      goingBack = button.matches('.back, [data-nav="back"]');
      if (button.matches("[data-bookmark]")) bookmarkFrom = (button.querySelector("svg") ?? button).getBoundingClientRect();
    },
    true,
  );
  const fine = () => typeof window.matchMedia === "function" && window.matchMedia("(hover: hover) and (pointer: fine)").matches;
  document.addEventListener("pointerover", (e) => {
    const el = e.target instanceof Element ? e.target : null;
    const button = el?.closest("[data-icon-job]");
    if (!button || (e.relatedTarget instanceof Node && button.contains(e.relatedTarget)) || !fine()) return;
    iconJob(button);
  });
  document.addEventListener("keydown", (e) => {
    if (e.key !== "Enter" && e.key !== " ") return;
    const el = e.target instanceof Element ? e.target : null;
    const button = el?.closest("[data-icon-job]");
    if (button) iconJob(button);
  });
  document.addEventListener("visibilitychange", () => {
    if (document.hidden) cancelAll();
    else restoreReveal(document.querySelector(".app"));
  });
}

/** Tests: forget a pending Back or bookmark. */
export function resetScreenMotion(): void {
  goingBack = false;
  bookmarkFrom = null;
}
