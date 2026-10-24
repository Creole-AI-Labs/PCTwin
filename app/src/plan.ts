/**
 * Choosing what moves (screens M02, M03, M04, M05, M23): the view models each screen takes (props in,
 * events out) and the small pure rules that sit on top of them. The shapes follow the engine's own
 * (crates pctwin-scan, pctwin-record, pctwin-executor) so the bridge can fill them without
 * translation; where the engine has nothing yet the gap is named below.
 *
 * Engine commands and events these screens need. None exists yet; each is a request the bridge makes
 * to the Rust side, and the screens only ever see the view models.
 *
 * - `scan_start()`                        Starts scanning this laptop (`scan_this_laptop` in pctwin-scan).
 * - `scan_resume()`                       Continues a saved scan (`build_scan_resumable` with the saved `ScanState`).
 * - `scan_pause()`                        Stops at the next folder boundary (`Flow::Stop` after a folder is saved).
 * - event `scan-progress`                 Payload: [`ScanView`]. Emitted after every folder is saved, and when the
 *                                         status changes (`pausing` is sent as soon as pause is asked for).
 * - `plan_choices()`                      Returns [`FolderChoice`][] for the signed-in person's special folders (from
 *                                         `FoundFolder` + `Size::of`), including folders the person added.
 * - `plan_set_folder(id, selected)`       Includes or leaves out one folder (`Inclusion::LeftOut{ChosenByUser}`).
 * - `plan_add_folder()`                   Opens the system folder picker on this laptop; the result arrives as a new
 *                                         `FolderChoice` with `custom: true`.
 * - `plan_remove_folder(id)`              Removes a folder the person added.
 * - `plan_allow_access(id)`               Asks the system for permission to read a protected folder.
 * - `plan_destinations()`                 Returns [`DestDrive`][] (the new laptop's drives, from `list_drives` run
 *                                         there and sent over the link) and each group's [`DestChoice`].
 * - `plan_set_destination(id, choice)`    Sets where one group goes on the new laptop.
 * - `plan_review()`                       Returns [`PlanView`] for the current record revision.
 * - event `plan-changed`                  Payload: [`PlanView`]. Emitted whenever the record gets a new revision or
 *                                         its fingerprint changes (`Record::next_revision`).
 * - `plan_approve(revision, fingerprint)` The person's approval of exactly that revision (`Record::approve`, checked
 *                                         again by `Record::check` before the move starts).
 * - `plan_start(approval)`                Starts the move for the approved revision (`Plan::approved` refuses a stale one).
 *
 * Gaps (the engine cannot say these yet):
 * - Which unreadable locations need the person's permission and which are simply gone: the scan only lists
 *   paths it could not read (`unreadable`), so screens say "could not be read" and offer permission only
 *   where the bridge marks a folder `access: "needs-permission"`.
 * - The new laptop's drives, free space and the folder a group lands in: the record's `Mapping` covers people
 *   only (`PersonTarget`, `SharedTarget`); there is no files-and-folders destination in the record yet.
 * - Online-only files per folder: the scan records them item by item (`LeftOutReason::CloudOnly`), not per folder.
 * - A plan revision's list of things the person must do by hand: derived here from the left-out reasons.
 * - The window size: the host calls `setWindowMode` (see [`windowModeFor`]); the screens never do.
 */
import type { WindowMode } from "./native";

/* ---------- Names shared with the engine ---------- */

/** What a folder is for (`FolderRole`, kebab-case on the wire). */
export const FOLDER_ROLES = ["home", "desktop", "documents", "downloads", "pictures", "music", "videos", "public", "app-data", "other"] as const;
export type FolderRole = (typeof FOLDER_ROLES)[number];

/** Why an item is left out (`LeftOutReason`). */
export const LEFT_OUT_REASONS = ["chosen-by-user", "work-managed", "device-bound", "unreadable", "cloud-only", "system", "too-many-files"] as const;
export type LeftOutReason = (typeof LEFT_OUT_REASONS)[number];

export type CloudProvider = "one-drive" | "i-cloud" | "dropbox" | "google-drive" | "other";

/** Where a folder really is (`Storage`, as the engine writes it). */
export type Storage = "system-drive" | { "other-drive": { drive: string } } | { cloud: { provider: CloudProvider } };

/** What was measured for a folder (`Size`): the whole truth, at least this much, or nothing readable. */
export type Size = { kind: "known"; bytes: number; files: number } | { kind: "at-least"; bytes: number; files: number } | { kind: "unreadable" };

/** What the scan counts for the after-move check (`Counts`). */
export interface Counts {
  photos: number;
  videos: number;
  music: number;
  documents: number;
  other: number;
}

/** A plan approval (`Approval`): one revision, and the fingerprint of exactly what that revision said. */
export interface Approval {
  revision: number;
  fingerprint: string;
}

/** Which window size each screen is drawn for. The host sets it when it shows the screen. */
export const WINDOW_MODE: Record<"scan" | "overview" | "files" | "destination" | "review", WindowMode> = {
  scan: "everyday",
  overview: "expanded",
  files: "everyday",
  destination: "everyday",
  review: "expanded",
};

export function windowModeFor(screen: keyof typeof WINDOW_MODE): WindowMode {
  return WINDOW_MODE[screen];
}

/* ---------- Sizes ---------- */

export type SizeUnit = "b" | "kb" | "mb" | "gb" | "tb";

/**
 * A byte count as a number and a unit (decimal: 1 GB is 1,000,000,000 bytes). The number is rounded
 * to one decimal, or to a whole number from 100 up; the words come from the strings (`plan-size-<unit>`).
 */
export function sizeParts(bytes: number): { unit: SizeUnit; n: number } {
  const units: SizeUnit[] = ["b", "kb", "mb", "gb", "tb"];
  let value = Math.max(0, bytes);
  let i = 0;
  while (value >= 1000 && i < units.length - 1) {
    value /= 1000;
    i++;
  }
  return { unit: units[i], n: value >= 100 || i === 0 ? Math.round(value) : Math.round(value * 10) / 10 };
}

/* ---------- M02 Finding your files ---------- */

/**
 * The three steps the person sees: the usual folders, other folders and drives, and the protected
 * places that could not be opened. The protected step is listed only when there are such places.
 */
export type ScanStepKey = "usual" | "other" | "protected";

export type ScanStepState = "done" | "current" | "todo";

export interface ScanStep {
  key: ScanStepKey;
  state: ScanStepState;
}

/** A place the scan could not read. It stays on the screen and is never counted as scanned. */
export interface UnreadablePlace {
  /** The path as the system reported it. */
  path: string;
  role?: FolderRole;
}

/** `pausing`: pause was asked for and the scan is finishing the folder it is in (it stops after a folder is saved). */
export type ScanStatus = "scanning" | "pausing" | "paused" | "finished";

export interface ScanView {
  status: ScanStatus;
  /** The scan carried on from a saved one (after a pause or a restart), so finished folders were not read again. */
  resumed: boolean;
  steps: ScanStep[];
  /** Folders looked at, out of the folders to look at: both measured, never estimated. */
  places: { done: number; total: number };
  /** Counted so far from the folder listings alone. Nothing is opened. */
  found: { files: number; bytes: number };
  unreadable: UnreadablePlace[];
}

/** The scan ended but some places could not be read (the "partial" state). */
export function isPartial(scan: ScanView): boolean {
  return scan.status === "finished" && scan.unreadable.length > 0;
}

/* ---------- M03 Everything you chose, in view ---------- */

/** The rows that open another screen. */
export type OverviewRow = "files" | "destinations" | "people";

export interface OverviewView {
  /** What is chosen to move (from the same sums as [`chosenTotals`]). */
  chosen: { folders: number; files: number; bytes: number; atLeast: boolean };
  /** Places the scan could not read: not in the totals. */
  unreadable: number;
  /** Groups whose destination is missing, full or not chosen (see [`checkDestinations`]). */
  destinationProblems: number;
  /** People whose owner has not signed in and approved yet. */
  peopleWaiting: number;
  /** Files that will not go as they are (too big for the drive, refused, or would run at sign-in): the plan lists each. */
  filesNeedingLook: number;
}

/**
 * The overview (M03) of a plan the new laptop received: what is chosen, from the plan's own sums;
 * a destination problem when the plan does not fit or its free space could not be read; files that
 * will not go as they are; people waiting when the plan is held for them. Places the old laptop
 * could not search are known only there, so none are counted. The attention rule is the plan
 * page's own ([`planAttention`]).
 */
export function overviewOf(plan: PlanView): OverviewView {
  const files = plan.groups.reduce((sum, g) => sum + (g.size.kind === "unreadable" ? 0 : g.size.files), 0);
  const bytes = plan.groups.reduce((sum, g) => sum + (g.size.kind === "unreadable" ? 0 : g.size.bytes), 0);
  return {
    chosen: { folders: plan.groups.length, files, bytes, atLeast: plan.groups.some((g) => g.size.kind !== "known") },
    unreadable: 0,
    destinationProblems: planAttention(plan).space ? 1 : 0,
    peopleWaiting: plan.blocked && plan.blocked !== "invalid" ? 1 : 0,
    filesNeedingLook: planAttention(plan).files,
  };
}

/** Everything in the overview that still needs the person. */
export function attentionCount(view: OverviewView): number {
  return view.unreadable + view.destinationProblems + view.peopleWaiting + view.filesNeedingLook;
}

/** What on a plan needs the person before it starts, by the one rule the plan page and the overview share. */
export function planAttention(plan: PlanView): { files: number; space: boolean } {
  const p = plan.preview;
  return {
    files: p ? p.tooBig + p.refused + p.runsAtSignIn : 0,
    // Not enough space, or space that could not be read, is never shown as fine.
    space: p ? p.fits !== true : false,
  };
}

/* ---------- M04 Keep what matters to you ---------- */

export interface FolderChoice {
  /** Stable for the folder: the same on every scan (`ItemId` of the top item, as text). */
  id: string;
  role: FolderRole;
  /** The folder as the system reports it. */
  path: string;
  storage: Storage;
  size: Size;
  /** Online-only files inside: counted, never downloaded, so never copied. */
  onlineOnlyFiles?: number;
  /** Managed by an employer or school: left out until the person chooses to include it. */
  managedBy: "personal" | "organisation";
  selected: boolean;
  /** `needs-permission`: the folder cannot be read until the person allows it. */
  access: "ok" | "needs-permission";
  /** Added by the person, not found by the scan. */
  custom?: boolean;
}

/** What the chosen folders add up to. A total with any part that was not fully read is "at least". */
export function chosenTotals(folders: readonly FolderChoice[]): { count: number; bytes: number; files: number; atLeast: boolean } {
  let bytes = 0;
  let files = 0;
  let atLeast = false;
  let count = 0;
  for (const f of folders) {
    if (!f.selected || f.access !== "ok") continue;
    count++;
    if (f.size.kind === "unreadable") {
      atLeast = true;
      continue;
    }
    bytes += f.size.bytes;
    files += f.size.files;
    if (f.size.kind === "at-least") atLeast = true;
  }
  return { count, bytes, files, atLeast };
}

/* ---------- M05 Give every file a home ---------- */

/** A drive on the new laptop (`Drive`, with `Keeps.largest_file`). */
export interface DestDrive {
  id: string;
  name: string;
  removable: boolean;
  readOnly: boolean;
  /** The one the new laptop runs from. */
  isSystem: boolean;
  /** False for a drive that was chosen and is no longer plugged in. */
  connected: boolean;
  freeBytes: number;
  totalBytes: number;
  /** The largest single file this drive's format can hold, or null for no practical limit (`Keeps.largest_file`). */
  largestFile: number | null;
}

/** A place on the new laptop: a drive and a folder on it. */
export interface DestChoice {
  drive: string;
  /** The folder on that drive, as text ("Documents"); empty for the top of the drive. */
  folder: string;
}

/** One selected group of files and where it goes. */
export interface DestGroup {
  /** The `FolderChoice.id` it came from. */
  id: string;
  role: FolderRole;
  path: string;
  size: Size;
  /** The biggest single file in the group, when known (to check a drive's largest-file limit). */
  largestFileBytes?: number;
  to: DestChoice | null;
}

export type GroupIssue = "no-destination" | "drive-missing" | "drive-read-only" | "file-too-big";

export interface DriveLoad {
  drive: DestDrive;
  /** What the groups going to this drive add up to. */
  needBytes: number;
  /** True when a group's size was only partly read, so the real need is at least this. */
  needAtLeast: boolean;
  /** How much more room is needed (0 when it fits). */
  shortBytes: number;
}

export interface DestinationCheck {
  groupIssues: Record<string, GroupIssue[]>;
  loads: DriveLoad[];
  /** Nothing to fix: every group has a connected, writable place with room. */
  ok: boolean;
}

function groupBytes(size: Size): number {
  return size.kind === "unreadable" ? 0 : size.bytes;
}

/** Checks every group against the drives. Missing places and full drives block; nothing is guessed. */
export function checkDestinations(groups: readonly DestGroup[], drives: readonly DestDrive[]): DestinationCheck {
  const byId = new Map(drives.map((d) => [d.id, d]));
  const groupIssues: Record<string, GroupIssue[]> = {};
  const need = new Map<string, { bytes: number; atLeast: boolean }>();
  for (const g of groups) {
    const issues: GroupIssue[] = [];
    if (!g.to) {
      issues.push("no-destination");
    } else {
      const drive = byId.get(g.to.drive);
      if (!drive?.connected) {
        issues.push("drive-missing");
      } else {
        if (drive.readOnly) issues.push("drive-read-only");
        if (drive.largestFile !== null && g.largestFileBytes !== undefined && g.largestFileBytes > drive.largestFile) issues.push("file-too-big");
        const entry = need.get(drive.id) ?? { bytes: 0, atLeast: false };
        entry.bytes += groupBytes(g.size);
        entry.atLeast ||= g.size.kind !== "known";
        need.set(drive.id, entry);
      }
    }
    if (issues.length > 0) groupIssues[g.id] = issues;
  }
  const loads: DriveLoad[] = [];
  for (const [id, n] of need) {
    const drive = byId.get(id);
    if (!drive) continue;
    loads.push({ drive, needBytes: n.bytes, needAtLeast: n.atLeast, shortBytes: Math.max(0, n.bytes - drive.freeBytes) });
  }
  const ok = Object.keys(groupIssues).length === 0 && loads.every((l) => l.shortBytes === 0);
  return { groupIssues, loads, ok };
}

/* ---------- M23 Approve the move you want ---------- */

/** What the plan lists for one group: from where, to where. */
export interface PlanGroup {
  id: string;
  role: FolderRole;
  path: string;
  size: Size;
  /** Where it lands, by the drive's name and the folder on it. */
  toDrive: string;
  toFolder: string;
}

/** What stays behind, and how many items (`Inclusion::LeftOut`), counted by reason. */
export interface LeftOut {
  reason: LeftOutReason;
  count: number;
}

/** Why the plan cannot be approved yet (the errors of `Record::approve`). */
export type PlanBlock = "someone-unmapped" | "account-used-twice" | "invalid";

/**
 * What the new laptop worked out before anything moves (the engine's `Preview`): whether it fits
 * (null: the free space could not be read), about how long in whole minutes (null: the link was not
 * measured), and how many files will not go as they are.
 */
export interface PlanPreview {
  fits: boolean | null;
  shortBytes: number;
  minutes: { low: number; high: number } | null;
  /** Land under a name changed to fit the drive. */
  renamed: number;
  /** Bigger than the drive holds in one file: not copied there. */
  tooBig: number;
  /** Paths the new laptop refuses: not copied. */
  refused: number;
  /** Would land where the new laptop starts programs by itself at sign-in. */
  runsAtSignIn: number;
}

/** One revision of the plan, exactly as it will run. Anything that changes it is a new revision and a new fingerprint. */
export interface PlanView {
  revision: number;
  /** The fingerprint of this revision (`Record::fingerprint`, as hex). */
  fingerprint: string;
  groups: PlanGroup[];
  leftOut: LeftOut[];
  /** Set when `Record::approve` would refuse. */
  blocked?: PlanBlock;
  /** Sent by the engine with every plan; absent only in the screens' own samples. */
  preview?: PlanPreview;
  /**
   * The groups whose things move first ("we'll move these first": what the person asked for, then
   * their essentials), in the order they start, from the very order the move sends in. Absent only
   * in the screens' own samples.
   */
  first?: FolderRole[];
}

/** The groups that move first and are still in the plan (one the person left out moves nothing). */
export function movesFirst(plan: PlanView, leftOut: readonly FolderRole[]): FolderRole[] {
  return (plan.first ?? []).filter((role) => !leftOut.includes(role) && plan.groups.some((g) => g.role === role));
}

/** Whether the approval the person gave is for exactly this revision, unchanged. */
export function approvalHolds(plan: PlanView, approval: Approval | null): boolean {
  return approval !== null && approval.revision === plan.revision && approval.fingerprint === plan.fingerprint;
}

/** What the plan lists that needs the person to do something by hand, from the left-out reasons. */
export const BY_HAND: readonly LeftOutReason[] = ["cloud-only", "unreadable", "work-managed", "too-many-files"];

export function leftOutByHand(plan: PlanView): LeftOut[] {
  return plan.leftOut.filter((l) => l.count > 0 && BY_HAND.includes(l.reason));
}
