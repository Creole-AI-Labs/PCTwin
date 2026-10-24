/**
 * Motion tokens: the one place every screen reads its timings, distances and easings from.
 *
 * `BLUEPRINT` is the Wordplay pack's token file (mockups/PCTwin_Motion_Wordplay/source/
 * PCTwin_Motion_Tokens.json) as written. PCTwin ships every motion at half that speed, for everyone
 * (decided 24 October 2026: every duration and delay doubled), so the shipped values below are the
 * blueprint's divided by `SPEED`. Distances and easings do not change with speed.
 *
 * No duration here is a wait: nothing a person can do ever waits for one to pass.
 */

/** The pack's tokens, unchanged. */
export const BLUEPRINT = {
  durationMs: {
    instant: 0,
    acknowledge: 83,
    quick: 120,
    state: 167,
    navigate: 220,
    surface: 250,
    signature: 300,
    emphasisMax: 333,
    wordplay: 2400,
    reducedOpening: 280,
  },
  distancePx: { press: 1, hover: 2, row: 4, navigation: 10, layer: 12 },
  staggerMs: 18,
  staggerVisibleRowMax: 5,
  easing: { settle: "cubic-bezier(.2,0,0,1)", measured: "linear", dock: "cubic-bezier(.65,0,.2,1)" },
} as const;

/** Half the blueprint's speed: every duration and delay is doubled. */
export const SPEED = 0.5;

export type DurationName = keyof typeof BLUEPRINT.durationMs;

const scaled = <K extends string>(values: Record<K, number>): Record<K, number> =>
  Object.fromEntries(Object.entries<number>(values).map(([k, v]) => [k, Math.round(v / SPEED)])) as Record<K, number>;

/** Shipped durations in milliseconds (the blueprint's, doubled). */
export const DURATION: Readonly<Record<DurationName, number>> = scaled(BLUEPRINT.durationMs);

/** A delay or a time inside a timeline, written at the blueprint's speed, as shipped. */
export const at = (blueprintMs: number): number => Math.round(blueprintMs / SPEED);

/** Shipped stagger between the first visible rows of a list, and how many rows take part. */
export const STAGGER_MS = at(BLUEPRINT.staggerMs);
export const STAGGER_ROWS = BLUEPRINT.staggerVisibleRowMax;

export const DISTANCE = BLUEPRINT.distancePx;
export const EASING = BLUEPRINT.easing;

/** The CSS custom properties every stylesheet reads (`--motion-*`), set on the root by the motion store. */
export function cssVariables(): Record<string, string> {
  const vars: Record<string, string> = {};
  for (const [name, ms] of Object.entries(DURATION)) vars[`--motion-${kebab(name)}`] = `${String(ms)}ms`;
  for (const [name, px] of Object.entries(DISTANCE)) vars[`--motion-distance-${name}`] = `${String(px)}px`;
  vars["--motion-ease-settle"] = EASING.settle;
  vars["--motion-ease-measured"] = EASING.measured;
  vars["--motion-ease-dock"] = EASING.dock;
  // The names the stylesheets used before the pack, kept as aliases of the pack's tokens.
  vars["--motion-feedback"] = `${String(DURATION.quick)}ms`;
  vars["--motion-page"] = `${String(DURATION.navigate)}ms`;
  vars["--motion-resize"] = `${String(DURATION.state)}ms`;
  vars["--motion-easing"] = EASING.settle;
  return vars;
}

function kebab(s: string): string {
  return s.replace(/[A-Z]/g, (c) => `-${c.toLowerCase()}`);
}
