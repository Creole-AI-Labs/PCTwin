/**
 * Screen samples: each screen in each of its states, drawn from made-up data, for review, for the
 * screens page (screens.html) and for the accessibility check every sample gets in the tests.
 * Each group of screens adds one file in ./samples that exports `samples`; nothing else to register.
 */
import type { ReactNode } from "react";
import type { WindowMode } from "../native";

export type Sample = {
  /** Stable and unique: the mockup frame and the state, like "C02-default" or "R02-partial". */
  id: string;
  /** What a reviewer sees in the picker (English; the review page is not translated). */
  title: string;
  /** The window size the screen is drawn for (the design system's three sizes). */
  size: WindowMode;
  render: () => ReactNode;
};

const FILES = import.meta.glob<{ samples: Sample[] }>("./samples/*.tsx", { eager: true });

/** Each group file's name ("quicksend.tsx") and how many samples it gave, in file order. */
export const GROUPS: { file: string; count: number }[] = Object.keys(FILES)
  .sort()
  .map((key) => ({ file: key.split("/").pop() ?? key, count: FILES[key].samples.length }));

/** Every sample from every group, in file then listed order. */
export const SAMPLES: Sample[] = Object.keys(FILES)
  .sort()
  .flatMap((file) => FILES[file].samples);

/** The logical window size of each mode, in CSS pixels (as set_window_mode uses). */
export const FRAME: Record<WindowMode, { width: number; height: number }> = {
  everyday: { width: 480, height: 620 },
  expanded: { width: 760, height: 620 },
  mini: { width: 360, height: 240 },
};
