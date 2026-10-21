/**
 * Made-up data for the progress and result screens: used by the screens page and by the tests. Every
 * number is invented. The shapes are the real ones (move.ts), so a sample that does not fit the
 * engine's rules does not type-check.
 */
import type { Label } from "./pairing";
import { noStatuses, type Counts, type MoveOutcome, type MoveProgress, type ReportItem } from "./move";

const GB = 1_000_000_000;
const MB = 1_000_000;

export function counts(partial: Partial<Counts>): Counts {
  const c = { total: 0, waiting: 0, copying: 0, checking: 0, done: 0, needsAction: 0, failed: 0, skippedByUser: 0, ...partial };
  return { ...c, total: partial.total ?? c.waiting + c.copying + c.checking + c.done + c.needsAction + c.failed + c.skippedByUser };
}

const old: Label = { kind: "named", name: "Ada’s old laptop" };
const laptops = { old };

/** Copying, mid-move: some done, some being checked, some waiting. */
export const running: MoveProgress = {
  state: "running",
  laptops,
  groups: [
    { id: "g1", name: { kind: "role", role: "documents" }, counts: counts({ done: 1800 }) },
    { id: "g2", name: { kind: "role", role: "pictures" }, counts: counts({ done: 2410, copying: 1, checking: 3, waiting: 1286 }) },
    { id: "g3", name: { kind: "folder", name: "Tax 2023" }, counts: counts({ waiting: 500 }) },
  ],
  counts: counts({ done: 4210, copying: 1, checking: 3, waiting: 1786 }),
  bytes: { checked: 81.6 * GB, total: 120 * GB },
  current: { name: "Holiday 2024 / IMG_2043.jpg", step: "copying" },
  speed: { low: 58 * MB, high: 66 * MB },
  minutesLeft: { low: 15, high: 20 },
};

/** Just started: nothing measured yet, so no speed and no time. */
export const starting: MoveProgress = {
  state: "running",
  laptops,
  groups: [
    { id: "g1", name: { kind: "role", role: "documents" }, counts: counts({ waiting: 2000 }) },
    { id: "g2", name: { kind: "role", role: "pictures" }, counts: counts({ waiting: 4000 }) },
  ],
  counts: counts({ waiting: 6000 }),
  bytes: { checked: 0, total: 120 * GB },
};

/** Every byte is checked but the last files are still being read back: never 100. */
export const almost: MoveProgress = {
  state: "running",
  laptops,
  groups: [
    { id: "g1", name: { kind: "role", role: "documents" }, counts: counts({ done: 2000 }) },
    { id: "g2", name: { kind: "role", role: "pictures" }, counts: counts({ done: 3997, checking: 3 }) },
  ],
  counts: counts({ done: 5997, checking: 3 }),
  bytes: { checked: 119.97 * GB, total: 120 * GB },
  current: { name: "Tax / 2023 return.pdf", step: "checking" },
};

export const withNeedsAction: MoveProgress = {
  ...running,
  groups: [
    { id: "g1", name: { kind: "role", role: "documents" }, counts: counts({ done: 1797, needsAction: 3 }) },
    ...running.groups.slice(1),
  ],
  counts: counts({ done: 4210, copying: 1, checking: 3, waiting: 1783, needsAction: 3 }),
};

export const withFailures: MoveProgress = {
  ...running,
  groups: [
    { id: "g1", name: { kind: "role", role: "documents" }, counts: counts({ done: 1797, needsAction: 3 }) },
    { id: "g2", name: { kind: "role", role: "pictures" }, counts: counts({ done: 2410, copying: 1, checking: 3, waiting: 1285, failed: 1 }) },
    running.groups[2],
  ],
  counts: counts({ done: 4210, copying: 1, checking: 3, waiting: 1782, needsAction: 3, failed: 1 }),
};

export const linkLost: MoveProgress = { ...running, notice: { kind: "link-lost" } };
export const lowBattery: MoveProgress = { ...running, notice: { kind: "low-battery", label: "Blue Fox" } };

export const pausing: MoveProgress = { ...running, state: "pausing", counts: counts({ done: 4210, copying: 1, waiting: 1789 }) };
export const paused: MoveProgress = { ...running, state: "paused", counts: counts({ done: 4211, waiting: 1789 }), current: undefined, speed: undefined, minutesLeft: undefined };

/** A move that ended with a mix. */
export const finishedMixed: MoveOutcome = {
  byStatus: { ...noStatuses(), copied: 5900, reinstalled: 0, converted: 90, "needs-action": 3, failed: 2, "skipped-by-user": 5 },
  notCopied: 0,
  bytesChecked: 118 * GB,
  stopped: false,
  checkedFromMemory: 0,
};

export const finishedClean: MoveOutcome = {
  byStatus: { ...noStatuses(), copied: 6000 },
  notCopied: 0,
  bytesChecked: 120 * GB,
  stopped: false,
  checkedFromMemory: 0,
};

export const finishedMemory: MoveOutcome = { ...finishedClean, checkedFromMemory: 6000 };

export const finishedStopped: MoveOutcome = {
  byStatus: { ...noStatuses(), copied: 3100 },
  notCopied: 2900,
  bytesChecked: 62 * GB,
  stopped: true,
  checkedFromMemory: 0,
};

/** The report: one of each status, with both kinds of evidence. */
export const report: ReportItem[] = [
  { id: "r1", name: "Invoice March.pdf", source: "C:\\Users\\Ada\\Documents\\Invoice March.pdf", destination: "C:\\Users\\Ada\\Documents\\Invoice March.pdf", status: "copied", evidence: { from: "drive" } },
  { id: "r2", name: "Holiday 2024", source: "C:\\Users\\Ada\\Pictures\\Holiday 2024", destination: "C:\\Users\\Ada\\Pictures\\Holiday 2024", status: "copied", evidence: { from: "memory", why: "cannot-skip-cache" } },
  { id: "r3", name: "Photo editor", source: "Installed app", destination: "Installed from its maker’s catalog", status: "reinstalled" },
  { id: "r4", name: "Old notes.rtf", source: "C:\\Users\\Ada\\Documents\\Old notes.rtf", destination: "C:\\Users\\Ada\\Documents\\Old notes.docx", status: "converted" },
  { id: "r5", name: "Mail export.pst", source: "C:\\Users\\Ada\\Documents\\Mail export.pst", status: "needs-action", reason: "online-only" },
  { id: "r6", name: "Games", source: "D:\\Games", status: "skipped-by-user" },
  { id: "r7", name: "Archive.zip", source: "D:\\Archive\\Archive.zip", status: "failed", reason: "not-enough-space" },
];

/** A long report, to try the paging. */
export function longReport(n: number): ReportItem[] {
  return Array.from({ length: n }, (_, i) => ({
    id: `l${i.toString()}`,
    name: `File ${i.toString()}.txt`,
    source: `C:\\Users\\Ada\\Documents\\File ${i.toString()}.txt`,
    destination: `C:\\Users\\Ada\\Documents\\File ${i.toString()}.txt`,
    status: "copied" as const,
    evidence: { from: "drive" as const },
  }));
}
