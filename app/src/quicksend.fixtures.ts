/** Made-up data for the Quick Send screens: used by the screens page and by the tests. Nothing here is real. */
import type { Label } from "./pairing";
import type { Conflict, HistoryEntry, NearbyLaptop, OfferedFile, PickedItem, Receipt, SendProgress } from "./quicksend";

export const BLUE_FOX: Label = { kind: "picked", colour: "blue", animal: "fox" };
export const GREEN_OWL: Label = { kind: "picked", colour: "green", animal: "owl" };

export const PICKED: PickedItem[] = [
  { id: "p1", name: "Project brief.pdf", kind: "file", included: true, bytes: 1_200_000, files: null, description: "PDF document" },
  { id: "p2", name: "Budget.xlsx", kind: "file", included: true, bytes: 600_000, files: null, description: "spreadsheet" },
  { id: "p3", name: "Reference photos", kind: "folder", included: true, bytes: 23_000_000, files: 8, description: "8 photos" },
];

export const OFFER: OfferedFile[] = [
  { id: "f1", path: "Budget.xlsx", size: 600_000 },
  { id: "f2", path: "Project brief.pdf", size: 1_200_000 },
  { id: "f3", path: "Reference photos/beach.jpg", size: 2_900_000 },
  { id: "f4", path: "Reference photos/market.jpg", size: 3_100_000 },
];

export const NEARBY: NearbyLaptop[] = [
  { index: 0, label: GREEN_OWL, sameLabelNearby: false, needsUpdate: false },
  { index: 1, label: { kind: "named", name: "Sam’s laptop" }, sameLabelNearby: false, needsUpdate: false },
];

export const CONFLICTS: Conflict[] = [
  {
    path: "Budget.xlsx",
    existingBytes: 590_000,
    offeredBytes: 600_000,
    newName: "Budget (2).xlsx",
    existingModified: "2026-10-14T09:30:00Z",
    choice: "keep-both",
  },
  {
    path: "Reference photos/beach.jpg",
    existingBytes: 2_900_000,
    offeredBytes: 2_900_000,
    newName: "beach (2).jpg",
    existingModified: null,
    choice: null,
  },
];

export const PROGRESS: SendProgress = {
  phase: "copying",
  filesDone: 2,
  filesTotal: 4,
  bytesDone: 2_000_000,
  bytesTotal: 7_800_000,
  current: "Reference photos/beach.jpg",
  failed: 0,
  speed: { low: 1_900_000, high: 3_100_000 },
};

export const RECEIPT_WHOLE: Receipt = {
  lines: [
    { path: "Budget.xlsx", outcome: { kind: "arrived", landedAs: "Budget.xlsx" } },
    { path: "Project brief.pdf", outcome: { kind: "already-there", landedAs: "Project brief.pdf" } },
    { path: "Reference photos/beach.jpg", outcome: { kind: "arrived", landedAs: "Reference photos/beach.jpg" } },
    { path: "Reference photos/market.jpg", outcome: { kind: "arrived", landedAs: "Reference photos/market.jpg" } },
  ],
};

export const RECEIPT_PARTIAL: Receipt = {
  lines: [
    { path: "Budget.xlsx", outcome: { kind: "arrived", landedAs: "Budget (2).xlsx" } },
    { path: "Project brief.pdf", outcome: { kind: "declined" } },
    { path: "Reference photos/beach.jpg", outcome: { kind: "failed", why: "the connection was lost before it finished" } },
    { path: "Reference photos/market.jpg", outcome: { kind: "arrived", landedAs: "Reference photos/market.jpg" } },
  ],
};

export const HISTORY: HistoryEntry[] = [
  { id: "h1", kind: "send", when: "2026-10-15T14:05:00Z", peer: "Green Owl", state: "finished" },
  { id: "h2", kind: "receive", when: "2026-10-14T10:20:00Z", peer: "Sam’s laptop", state: "partial" },
  { id: "h3", kind: "move", when: "2026-10-12T18:00:00Z", from: "Old laptop", to: "New laptop", state: "paused" },
  { id: "h4", kind: "move", when: "2026-09-28T09:00:00Z", from: "Studio PC", to: "Studio laptop", state: "steps-left", stepsLeft: 2 },
  { id: "h5", kind: "send", when: "2026-09-20T16:40:00Z", peer: "Blue Fox", state: "stopped" },
];
