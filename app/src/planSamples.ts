/**
 * Made-up data for the plan screens (M02 to M23): the samples page and the tests both draw on it.
 * Nothing here is real: the names, sizes and counts only show how each state looks.
 */
import type { DestDrive, DestGroup, FolderChoice, OverviewView, PlanView, ScanView } from "./plan";

const GB = 1_000_000_000;

/** Scanning, partway: the usual folders are being read. */
export const scanning: ScanView = {
  status: "scanning",
  resumed: false,
  steps: [
    { key: "usual", state: "current" },
    { key: "other", state: "todo" },
  ],
  places: { done: 3, total: 7 },
  found: { files: 48_210, bytes: 91_200_000_000 },
  unreadable: [],
};

export const pausing: ScanView = { ...scanning, status: "pausing" };

export const paused: ScanView = { ...scanning, status: "paused" };

/** Carried on from a saved scan: the first folders were not read again. */
export const resumed: ScanView = { ...scanning, resumed: true, places: { done: 4, total: 7 } };

/** Finished, but two places could not be opened: they stay listed and are not counted. */
export const partial: ScanView = {
  status: "finished",
  resumed: false,
  steps: [
    { key: "usual", state: "done" },
    { key: "other", state: "done" },
    { key: "protected", state: "todo" },
  ],
  places: { done: 7, total: 7 },
  found: { files: 61_402, bytes: 118_700_000_000 },
  unreadable: [
    { path: "C:\\Users\\Ada\\AppData\\Local\\Private", role: "app-data" },
    { path: "D:\\Archive\\Locked", role: "other" },
  ],
};

export const finished: ScanView = {
  status: "finished",
  resumed: false,
  steps: [
    { key: "usual", state: "done" },
    { key: "other", state: "done" },
  ],
  places: { done: 7, total: 7 },
  found: { files: 61_402, bytes: 118_700_000_000 },
  unreadable: [],
};

export const overview: OverviewView = {
  chosen: { folders: 3, files: 44_224, bytes: 108_700_000_000, atLeast: false },
  unreadable: 0,
  destinationProblems: 0,
  filesNeedingLook: 0,
  peopleWaiting: 0,
};

export const overviewPartial: OverviewView = { ...overview, unreadable: 2, destinationProblems: 1, peopleWaiting: 1 };

const documents: FolderChoice = {
  id: "f-documents",
  role: "documents",
  path: "C:\\Users\\Ada\\Documents",
  storage: "system-drive",
  size: { kind: "known", bytes: 18_400_000_000, files: 12_404 },
  managedBy: "personal",
  selected: true,
  access: "ok",
};

const pictures: FolderChoice = {
  id: "f-pictures",
  role: "pictures",
  path: "C:\\Users\\Ada\\Pictures",
  storage: "system-drive",
  size: { kind: "known", bytes: 62_100_000_000, files: 31_820 },
  onlineOnlyFiles: 12,
  managedBy: "personal",
  selected: true,
  access: "ok",
};

const downloads: FolderChoice = {
  id: "f-downloads",
  role: "downloads",
  path: "C:\\Users\\Ada\\Downloads",
  storage: "system-drive",
  size: { kind: "known", bytes: 11_200_000_000, files: 640 },
  managedBy: "personal",
  selected: false,
  access: "ok",
};

const work: FolderChoice = {
  id: "f-work",
  role: "documents",
  path: "C:\\Users\\Ada\\OneDrive - Contoso\\Documents",
  storage: { cloud: { provider: "one-drive" } },
  size: { kind: "at-least", bytes: 4_300_000_000, files: 2_050 },
  managedBy: "organisation",
  selected: false,
  access: "ok",
};

const videos: FolderChoice = {
  id: "f-videos",
  role: "videos",
  path: "C:\\Users\\Ada\\Videos",
  storage: "system-drive",
  size: { kind: "unreadable" },
  managedBy: "personal",
  selected: false,
  access: "needs-permission",
};

const projects: FolderChoice = {
  id: "f-projects",
  role: "other",
  path: "D:\\Projects",
  storage: { "other-drive": { drive: "D" } },
  size: { kind: "known", bytes: 28_300_000_000, files: 9_120 },
  managedBy: "personal",
  selected: true,
  access: "ok",
  custom: true,
};

export const foldersDefault: FolderChoice[] = [documents, pictures, downloads, work];
export const foldersProtected: FolderChoice[] = [documents, pictures, downloads, videos];
export const foldersCustom: FolderChoice[] = [documents, pictures, downloads, projects];

export const newLaptop: DestDrive = {
  id: "new",
  name: "New laptop",
  removable: false,
  readOnly: false,
  isSystem: true,
  connected: true,
  freeBytes: 500 * GB,
  totalBytes: 1000 * GB,
  largestFile: null,
};

export const travelDisk: DestDrive = {
  id: "travel",
  name: "Travel disk",
  removable: true,
  readOnly: false,
  isSystem: false,
  connected: true,
  freeBytes: 120 * GB,
  totalBytes: 500 * GB,
  largestFile: null,
};

export const groupsMapped: DestGroup[] = [
  { id: "f-documents", role: "documents", path: documents.path, size: documents.size, to: { drive: "new", folder: "Documents" } },
  { id: "f-pictures", role: "pictures", path: pictures.path, size: pictures.size, to: { drive: "travel", folder: "Photos" } },
  { id: "f-projects", role: "other", path: projects.path, size: projects.size, to: { drive: "new", folder: "Projects" } },
];

export const drivesMapped: DestDrive[] = [newLaptop, travelDisk];
export const drivesMissing: DestDrive[] = [newLaptop, { ...travelDisk, connected: false }];
export const drivesFull: DestDrive[] = [newLaptop, { ...travelDisk, freeBytes: 40 * GB }];

export const planFingerprint = "9f3ac27be410d5568a1c0be2f7d3a94c61e08b57d2c4f9a03b6e1d85c7a24f10";

export const plan: PlanView = {
  revision: 4,
  fingerprint: planFingerprint,
  groups: [
    { id: "f-documents", role: "documents", path: documents.path, size: documents.size, toDrive: "New laptop", toFolder: "Documents" },
    { id: "f-pictures", role: "pictures", path: pictures.path, size: pictures.size, toDrive: "Travel disk", toFolder: "Photos" },
    { id: "f-projects", role: "other", path: projects.path, size: projects.size, toDrive: "New laptop", toFolder: "Projects" },
  ],
  leftOut: [
    { reason: "cloud-only", count: 12 },
    { reason: "work-managed", count: 340 },
    { reason: "chosen-by-user", count: 1 },
    { reason: "system", count: 52 },
  ],
};

/** A later revision of the same plan (something in it changed). */
export const planNext: PlanView = { ...plan, revision: 5, fingerprint: "1b7e55a0d3c9486f2a07e91c5d40b8836fa2c71e9d05b4a83c6f0e27d19a58c3" };
