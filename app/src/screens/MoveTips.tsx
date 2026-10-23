import { useLocalization } from "@fluent/react";
import { useEffect, useState } from "react";
import { Info } from "../icons";

/** What this laptop knows about itself that changes which tips fit (each laptop's tips are about itself). */
export type TipContext = {
  /** This is the old laptop (it is only read, so the person can keep using it). */
  old: boolean;
  /** This laptop said it was on battery when the move started. */
  onBattery: boolean;
  /** The connection measured as the laptops greeted was slow. */
  slow: boolean;
  /** A battery warning is pinned on screen now: the battery tip would only repeat it. */
  batteryWarning: boolean;
};

/** About every 12 seconds, as the design says (T14). */
export const TIP_SECONDS = 12;

/**
 * The tips that fit right now, most useful first: what this laptop's own state calls for (its
 * battery, a slow connection), then the ones that always hold. Each is a string id.
 */
export function tipsFor(c: TipContext): string[] {
  const tips: string[] = [];
  if (c.onBattery && !c.batteryWarning) tips.push("tip-battery");
  if (c.slow) tips.push("tip-closer");
  tips.push("tip-lids");
  if (c.old) tips.push("tip-keep-using");
  tips.push("tip-pause", "tip-power-cut");
  return tips;
}

/** Whether the person asked their system for less motion. Unknown counts as no. */
function prefersReducedMotion(): boolean {
  return typeof window !== "undefined" && typeof window.matchMedia === "function" && window.matchMedia("(prefers-reduced-motion: reduce)").matches;
}

/**
 * T14 One contextual tip under the progress bar. It changes about every 12 seconds; Pause tips
 * holds the one on screen, and Next tip shows the next. With reduced motion it never changes by
 * itself. The tip is never announced as it changes (no live region): a screen reader reads it
 * where it sits. Warnings are not tips: they stay pinned in the notices and are never rotated.
 */
export default function MoveTips({ context }: { context: TipContext }) {
  const { l10n } = useLocalization();
  const tips = tipsFor(context);
  const [at, setAt] = useState(0);
  const [paused, setPaused] = useState(false);
  const [still] = useState(prefersReducedMotion);
  const turning = !paused && !still && tips.length > 1;

  useEffect(() => {
    if (!turning) return;
    const timer = setInterval(() => {
      setAt((n) => n + 1);
    }, TIP_SECONDS * 1000);
    return () => {
      clearInterval(timer);
    };
  }, [turning]);

  const tip = tips[at % tips.length];
  return (
    <section className="note mv-tip" aria-label={l10n.getString("tips-label")}>
      <Info />
      <div className="mv-tip-body">
        <p className="mv-tip-text">{l10n.getString(tip)}</p>
        <div className="mv-tip-actions">
          {!still && tips.length > 1 && (
            <button
              type="button"
              className="mv-link"
              onClick={() => {
                setPaused((p) => !p);
              }}
            >
              {l10n.getString(paused ? "tips-resume" : "tips-pause")}
            </button>
          )}
          {tips.length > 1 && (
            <button
              type="button"
              className="mv-link"
              onClick={() => {
                setAt((n) => n + 1);
              }}
            >
              {l10n.getString("tips-next")}
            </button>
          )}
        </div>
      </div>
    </section>
  );
}
