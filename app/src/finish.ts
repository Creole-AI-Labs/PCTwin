/**
 * Finishing a move (Task List 2.7), as the engine sends it (src-tauri/src/finish.rs): the hub, the
 * setup checklist, the before-wipe list, bringing the new laptop up to date, handing the old laptop
 * on, undo's preview, clean-up, the report and the connection doctor. Every value is a word key, a
 * count, or a name shown exactly as it is. The parsers are strict: an unknown key or value, a
 * missing or extra field, is refused, never guessed.
 *
 * Every `digest` and `shown` value goes back to the engine exactly as it came; a `changed` answer
 * means look again and ask the person again.
 */
import { ALL_REASONS, type Reason } from "./move";

type Fields = Record<string, unknown>;

function exactly(value: unknown, keys: readonly string[]): Fields | null {
  if (typeof value !== "object" || value === null || Array.isArray(value)) return null;
  const f = value as Fields;
  const own = Object.keys(f);
  return own.length === keys.length && keys.every((k) => own.includes(k)) ? f : null;
}

function isCount(v: unknown): v is number {
  return typeof v === "number" && Number.isSafeInteger(v) && v >= 0;
}

function oneOf<T extends string>(v: unknown, list: readonly T[]): v is T {
  return typeof v === "string" && (list as readonly string[]).includes(v);
}

function listOf<T>(v: unknown, parse: (x: unknown) => T | null): T[] | null {
  if (!Array.isArray(v)) return null;
  const out: T[] = [];
  for (const x of v) {
    const p = parse(x);
    if (p === null) return null;
    out.push(p);
  }
  return out;
}

const text = (v: unknown): string | null => (typeof v === "string" ? v : null);
const names = (v: unknown): string[] | null => listOf(v, text);

/** A hex ID or digest: exactly as the engine made it. */
function isHex(v: unknown): v is string {
  return typeof v === "string" && /^[0-9a-f]+$/.test(v);
}

/** A kebab-case word key (a message or step the engine names). */
function isKey(v: unknown): v is string {
  return typeof v === "string" && /^[a-z][a-z0-9]*(-[a-z0-9]+)*$/.test(v);
}

/* ---------- Why a finishing command was refused ---------- */

export const FINISH_PROBLEMS = ["no-move", "changed", "not-allowed", "busy", "failed"] as const;
export type FinishProblem = { kind: (typeof FINISH_PROBLEMS)[number] };

export function parseFinishProblem(value: unknown): FinishProblem | null {
  const f = exactly(value, ["kind"]);
  return f && oneOf(f.kind, FINISH_PROBLEMS) ? { kind: f.kind } : null;
}

/* ---------- The overview ---------- */

export const HEADLINES = ["ready-for-selected-work", "data-preserved", "finished-with-tasks", "handover-unresolved", "nothing-moved"] as const;
export type Headline = (typeof HEADLINES)[number];

export const STANDINGS = ["copied", "reinstalled", "converted", "needs-action", "failed", "waiting", "not-reachable", "no-result-yet", "skipped-by-user", "left-out"] as const;
export type Standing = (typeof STANDINGS)[number];

export const ACCESS_KEYS = ["usable", "key-not-checked", "not-on-new-laptop"] as const;
export type AccessKey = (typeof ACCESS_KEYS)[number];

export const AFTER_KINDS = ["photos", "videos", "music", "documents", "other-files", "apps", "settings"] as const;
export type AfterKind = (typeof AFTER_KINDS)[number];

export const FINISH_NOTES = ["parental-controls", "not-everywhere", "results-refused", "undo-open"] as const;
export type FinishNote = (typeof FINISH_NOTES)[number];

export const ITEM_KINDS = ["file", "folder", "app", "setting", "package"] as const;
export type ItemKind = (typeof ITEM_KINDS)[number];

export type Tally = {
  selected: number;
  done: number;
  needsAction: number;
  failed: number;
  waiting: number;
  notReachable: number;
  noResultYet: number;
  skipped: number;
  leftOut: number;
  /** Of those not done, how many the person chose to leave behind. */
  acceptedLoss: number;
};

const TALLY_KEYS = ["selected", "done", "needsAction", "failed", "waiting", "notReachable", "noResultYet", "skipped", "leftOut", "acceptedLoss"] as const;

function parseTally(v: unknown): Tally | null {
  const f = exactly(v, TALLY_KEYS);
  if (!f || !TALLY_KEYS.every((k) => isCount(f[k]))) return null;
  return Object.fromEntries(TALLY_KEYS.map((k) => [k, f[k]])) as Tally;
}

export type ItemLine = { id: string; label: string; kind: ItemKind; standing: Standing; reason: Reason | null; access: AccessKey };

function parseReason(v: unknown): Reason | null | undefined {
  if (v === null) return null;
  return oneOf(v, ALL_REASONS) ? v : undefined;
}

function parseItemLine(v: unknown): ItemLine | null {
  const f = exactly(v, ["id", "label", "kind", "standing", "reason", "access"]);
  if (!f || !isHex(f.id) || typeof f.label !== "string" || !oneOf(f.kind, ITEM_KINDS) || !oneOf(f.standing, STANDINGS) || !oneOf(f.access, ACCESS_KEYS)) return null;
  const reason = parseReason(f.reason);
  return reason === undefined ? null : { id: f.id, label: f.label, kind: f.kind, standing: f.standing, reason, access: f.access };
}

export type AfterGroup = { group: AfterKind; atScan: number; onNewLaptop: number };

/** Where PCTwin looked on the old laptop, and where it did not. */
export type Search = { searched: string[]; notSearched: string[]; foundEverything: boolean };

function parseSearch(v: unknown): Search | null {
  const f = exactly(v, ["searched", "notSearched", "foundEverything"]);
  const searched = f && names(f.searched);
  const notSearched = f && names(f.notSearched);
  return f && searched && notSearched && typeof f.foundEverything === "boolean" ? { searched, notSearched, foundEverything: f.foundEverything } : null;
}

export type FinishView = {
  id: string;
  headline: Headline;
  tally: Tally;
  /** Other people's items, counted only. */
  others: number;
  afterMove: AfterGroup[];
  notes: FinishNote[];
  notFinished: ItemLine[];
  more: number;
  search: Search;
  /** Seconds since 1970; null: not checked since the move. */
  oldLaptopCheckedAt: number | null;
};

export function parseFinishView(v: unknown): FinishView | null {
  const f = exactly(v, ["id", "headline", "tally", "others", "afterMove", "notes", "notFinished", "more", "search", "oldLaptopCheckedAt"]);
  if (!f || typeof f.id !== "string" || !oneOf(f.headline, HEADLINES) || !isCount(f.others) || !isCount(f.more)) return null;
  if (f.oldLaptopCheckedAt !== null && !isCount(f.oldLaptopCheckedAt)) return null;
  const tally = parseTally(f.tally);
  const afterMove = listOf(f.afterMove, (x) => {
    const g = exactly(x, ["group", "atScan", "onNewLaptop"]);
    return g && oneOf(g.group, AFTER_KINDS) && isCount(g.atScan) && isCount(g.onNewLaptop) ? { group: g.group, atScan: g.atScan, onNewLaptop: g.onNewLaptop } : null;
  });
  const notes = listOf(f.notes, (x) => (oneOf(x, FINISH_NOTES) ? x : null));
  const notFinished = listOf(f.notFinished, parseItemLine);
  const search = parseSearch(f.search);
  if (!tally || !afterMove || !notes || !notFinished || !search) return null;
  return { id: f.id, headline: f.headline, tally, others: f.others, afterMove, notes, notFinished, more: f.more, search, oldLaptopCheckedAt: f.oldLaptopCheckedAt };
}

/* ---------- Before any wipe ---------- */

export const EXCEPTION_KEYS = [
  "only-on-old-laptop",
  "key-not-checked",
  "tied-to-old-laptop",
  "do-first-on-old-laptop",
  "steps-not-finished",
  "not-searched",
  "old-laptop-not-reachable",
  "special-sign-in",
  "special-relink",
  "special-export-first",
  "special-look-at-it",
] as const;
export type ExceptionKey = (typeof EXCEPTION_KEYS)[number];

export type Resolution = { kind: "open" } | { kind: "person-did-it"; at: number } | { kind: "accepted-loss"; at: number };

function parseResolution(v: unknown): Resolution | null {
  const kind = typeof v === "object" && v !== null ? (v as Fields).kind : undefined;
  if (kind === "open") return exactly(v, ["kind"]) ? { kind } : null;
  if (kind === "person-did-it" || kind === "accepted-loss") {
    const f = exactly(v, ["kind", "at"]);
    return f && isCount(f.at) ? { kind, at: f.at } : null;
  }
  return null;
}

export type NamedItem = { id: string; label: string; kind: ItemKind };

export type ExceptionLine = {
  id: string;
  key: ExceptionKey;
  count: number;
  items: NamedItem[];
  more: number;
  places: string[];
  names: string[];
  resolution: Resolution;
  canAccept: boolean;
  canMarkDone: boolean;
  /** Binds an answer to exactly what is shown now; sent back with the answer. */
  shown: string;
};

function parseException(v: unknown): ExceptionLine | null {
  const f = exactly(v, ["id", "key", "count", "items", "more", "places", "names", "resolution", "canAccept", "canMarkDone", "shown"]);
  if (!f || !isHex(f.id) || !oneOf(f.key, EXCEPTION_KEYS) || !isCount(f.count) || !isCount(f.more) || typeof f.canAccept !== "boolean" || typeof f.canMarkDone !== "boolean" || !isHex(f.shown)) return null;
  const items = listOf(f.items, (x) => {
    const n = exactly(x, ["id", "label", "kind"]);
    return n && isHex(n.id) && typeof n.label === "string" && oneOf(n.kind, ITEM_KINDS) ? { id: n.id, label: n.label, kind: n.kind } : null;
  });
  const places = names(f.places);
  const named = names(f.names);
  const resolution = parseResolution(f.resolution);
  if (!items || !places || !named || !resolution) return null;
  return { id: f.id, key: f.key, count: f.count, items, more: f.more, places, names: named, resolution, canAccept: f.canAccept, canMarkDone: f.canMarkDone, shown: f.shown };
}

export type OldLaptop = { kind: "checked"; at: number } | { kind: "not-checked" };

function parseOldLaptop(v: unknown): OldLaptop | null {
  const kind = typeof v === "object" && v !== null ? (v as Fields).kind : undefined;
  if (kind === "not-checked") return exactly(v, ["kind"]) ? { kind } : null;
  if (kind === "checked") {
    const f = exactly(v, ["kind", "at"]);
    return f && isCount(f.at) ? { kind, at: f.at } : null;
  }
  return null;
}

export type GoodbyeView = {
  id: string;
  madeAt: number;
  suggestedKeepUntil: number;
  keepUntil: number | null;
  undoOpen: boolean;
  oldLaptop: OldLaptop;
  /** Every thing resolved or accepted, and the old laptop checked. */
  clear: boolean;
  search: Search;
  exceptions: ExceptionLine[];
  /** The list is saved on the new laptop. */
  saved: boolean;
};

export function parseGoodbyeView(v: unknown): GoodbyeView | null {
  const f = exactly(v, ["id", "madeAt", "suggestedKeepUntil", "keepUntil", "undoOpen", "oldLaptop", "clear", "search", "exceptions", "saved"]);
  if (!f || typeof f.id !== "string" || !isCount(f.madeAt) || !isCount(f.suggestedKeepUntil) || (f.keepUntil !== null && !isCount(f.keepUntil))) return null;
  if (typeof f.undoOpen !== "boolean" || typeof f.clear !== "boolean" || typeof f.saved !== "boolean") return null;
  const oldLaptop = parseOldLaptop(f.oldLaptop);
  const search = parseSearch(f.search);
  const exceptions = listOf(f.exceptions, parseException);
  if (!oldLaptop || !search || !exceptions) return null;
  return { id: f.id, madeAt: f.madeAt, suggestedKeepUntil: f.suggestedKeepUntil, keepUntil: f.keepUntil, undoOpen: f.undoOpen, oldLaptop, clear: f.clear, search, exceptions, saved: f.saved };
}

export const GOODBYE_ANSWERS = ["accept-loss", "did-it"] as const;
export type GoodbyeAnswer = (typeof GOODBYE_ANSWERS)[number];

/* ---------- Bring me up to date ---------- */

export const CATCHUP_PURPOSES = ["check", "catch-up"] as const;
export type CatchupPurpose = (typeof CATCHUP_PURPOSES)[number];

export const CHANGE_KEYS = ["new", "updated", "both-changed", "gone-on-new-laptop", "gone-on-old-laptop", "cannot-tell"] as const;
export type ChangeKey = (typeof CHANGE_KEYS)[number];

export const CATCHUP_CHOICES = ["send", "keep-both", "keep-new-laptop-version"] as const;
export type CatchupChoice = (typeof CATCHUP_CHOICES)[number];

export type DeltaLine = { id: string; label: string; change: ChangeKey; needsChoice: boolean };

export type DeltaView = {
  id: string;
  purpose: CatchupPurpose;
  /** Binds the approval to exactly this list. */
  digest: string;
  lines: DeltaLine[];
  more: number;
  others: number;
};

export function parseDeltaView(v: unknown): DeltaView | null {
  const f = exactly(v, ["id", "purpose", "digest", "lines", "more", "others"]);
  if (!f || typeof f.id !== "string" || !oneOf(f.purpose, CATCHUP_PURPOSES) || !isHex(f.digest) || !isCount(f.more) || !isCount(f.others)) return null;
  const lines = listOf(f.lines, (x) => {
    const l = exactly(x, ["id", "label", "change", "needsChoice"]);
    return l && isHex(l.id) && typeof l.label === "string" && oneOf(l.change, CHANGE_KEYS) && typeof l.needsChoice === "boolean" ? { id: l.id, label: l.label, change: l.change, needsChoice: l.needsChoice } : null;
  });
  return lines ? { id: f.id, purpose: f.purpose, digest: f.digest, lines, more: f.more, others: f.others } : null;
}

/** What each kind of change allows the person to choose (the engine's `ChangeKey::needs_choice` is the rule for who must). */
export const CHOICES_FOR: Record<ChangeKey, readonly CatchupChoice[]> = {
  new: ["send"],
  updated: ["send"],
  "both-changed": ["keep-both", "keep-new-laptop-version"],
  "gone-on-new-laptop": ["send", "keep-new-laptop-version"],
  "gone-on-old-laptop": [],
  "cannot-tell": ["keep-both", "keep-new-laptop-version"],
};

export type ChoiceIn = { id: string; choice: CatchupChoice };

/* ---------- Handing on, safe goodbye ---------- */

export const HANDOVER_ACTIONS = ["remove-my-account", "erase-old-laptop"] as const;
export type HandoverAction = (typeof HANDOVER_ACTIONS)[number];

export const REFUSAL_KEYS = [
  "wrong-device",
  "undo-not-closed",
  "old-laptop-not-checked",
  "list-out-of-date",
  "list-not-clear",
  "no-write-exclusion",
  "no-such-account",
  "other-people-use-it",
  "not-qualified",
  "no-fresh-confirmation",
  "wrong-confirmation",
  "changed-since-confirmed",
  "expired",
  "already-used",
  "nothing-to-remove",
] as const;
export type RefusalKey = (typeof REFUSAL_KEYS)[number];

export const HANDOVER_NOTES = [
  "ssd-not-secure-wipe",
  "does-not-cancel-subscriptions",
  "others-stay",
  "cannot-be-undone",
  "closes-undo",
  "sign-in-as-another-administrator",
  "family-group-first",
  "not-a-certificate",
  "encrypt-first-helps",
] as const;
export type HandoverNote = (typeof HANDOVER_NOTES)[number];

export const SYSTEM_KEYS = ["windows", "macos", "linux"] as const;
export type SystemKey = (typeof SYSTEM_KEYS)[number];

/** `system` is null when PCTwin cannot tell which system the old laptop runs: the steps are then the general ones. */
export type Route = { system: SystemKey | null; route: string; steps: string[]; canOpen: boolean };

function parseRoute(v: unknown): Route | null {
  const f = exactly(v, ["system", "route", "steps", "canOpen"]);
  const steps = f && listOf(f.steps, (x) => (isKey(x) ? x : null));
  return f && steps && (f.system === null || oneOf(f.system, SYSTEM_KEYS)) && isKey(f.route) && typeof f.canOpen === "boolean" ? { system: f.system, route: f.route, steps, canOpen: f.canOpen } : null;
}

export type HandoverView =
  /** `ticket`: this preview's own mark, sent back exactly with the confirmation (the engine takes it once, between 2 seconds and 10 minutes after the preview). */
  | { kind: "ready"; action: HandoverAction; digest: string; ticket: string; removes: string[]; keeps: string[]; notes: HandoverNote[]; route: Route }
  | { kind: "refused"; action: HandoverAction; why: RefusalKey; open: number };

export function parseHandoverView(v: unknown): HandoverView | null {
  const kind = typeof v === "object" && v !== null ? (v as Fields).kind : undefined;
  if (kind === "ready") {
    const f = exactly(v, ["kind", "action", "digest", "ticket", "removes", "keeps", "notes", "route"]);
    if (!f || !oneOf(f.action, HANDOVER_ACTIONS) || !isHex(f.digest) || !(isHex(f.ticket) && f.ticket.length === 32)) return null;
    const removes = names(f.removes);
    const keeps = names(f.keeps);
    const notes = listOf(f.notes, (x) => (oneOf(x, HANDOVER_NOTES) ? x : null));
    const route = parseRoute(f.route);
    return removes && keeps && notes && route ? { kind, action: f.action, digest: f.digest, ticket: f.ticket, removes, keeps, notes, route } : null;
  }
  if (kind === "refused") {
    const f = exactly(v, ["kind", "action", "why", "open"]);
    return f && oneOf(f.action, HANDOVER_ACTIONS) && oneOf(f.why, REFUSAL_KEYS) && isCount(f.open) ? { kind, action: f.action, why: f.why, open: f.open } : null;
  }
  return null;
}

export type HandoverStarted = { action: HandoverAction; undoClosed: boolean; opened: boolean; route: Route };

export function parseHandoverStarted(v: unknown): HandoverStarted | null {
  const f = exactly(v, ["action", "undoClosed", "opened", "route"]);
  const route = f && parseRoute(f.route);
  return f && route && oneOf(f.action, HANDOVER_ACTIONS) && typeof f.undoClosed === "boolean" && typeof f.opened === "boolean" ? { action: f.action, undoClosed: f.undoClosed, opened: f.opened, route } : null;
}

export const PERSON_SAYS = ["done", "partly", "cancelled-part-way", "not-started", "cannot-tell"] as const;
export type PersonSays = (typeof PERSON_SAYS)[number];

export const OUTCOME_KEYS = ["person-says-done", "partial", "interrupted", "cancelled-after-some-effects", "not-started", "inconclusive"] as const;
export type OutcomeKey = (typeof OUTCOME_KEYS)[number];

export type HandoverRecorded = { action: HandoverAction; outcome: OutcomeKey; at: number };

export function parseHandoverRecorded(v: unknown): HandoverRecorded | null {
  const f = exactly(v, ["action", "outcome", "at"]);
  return f && oneOf(f.action, HANDOVER_ACTIONS) && oneOf(f.outcome, OUTCOME_KEYS) && isCount(f.at) ? { action: f.action, outcome: f.outcome, at: f.at } : null;
}

/* ---------- Connection doctor ---------- */

export const DOCTOR_ROLES = ["old", "new"] as const;
export type DoctorRole = (typeof DOCTOR_ROLES)[number];

export type Finding = { cause: string; fix: string; sure: boolean; names: string[] };
export type DoctorView = { findings: Finding[]; hotspot: { suggested: boolean }; notChecked: string[] };

export function parseDoctorView(v: unknown): DoctorView | null {
  const f = exactly(v, ["findings", "hotspot", "notChecked"]);
  if (!f) return null;
  const findings = listOf(f.findings, (x) => {
    const g = exactly(x, ["cause", "fix", "sure", "names"]);
    const n = g && names(g.names);
    return g && n && isKey(g.cause) && isKey(g.fix) && typeof g.sure === "boolean" ? { cause: g.cause, fix: g.fix, sure: g.sure, names: n } : null;
  });
  const hot = exactly(f.hotspot, ["suggested"]);
  const notChecked = listOf(f.notChecked, (x) => (isKey(x) ? x : null));
  return findings && hot && typeof hot.suggested === "boolean" && notChecked ? { findings, hotspot: { suggested: hot.suggested }, notChecked } : null;
}

/* ---------- Finish your setup, saved setup ---------- */

export const SETUP_WHATS = ["sign-in", "licence-key", "set-up-again", "download-from-maker", "permission", "restart", "finish-by-hand", "parts-missing", "export-first", "relink", "look-at-it", "screens"] as const;
export type SetupWhat = (typeof SETUP_WHATS)[number];

export const SETUP_MARKS = ["open", "machine-checked", "you-did-it"] as const;
export type SetupMark = (typeof SETUP_MARKS)[number];

export type SetupTask = { id: string; what: SetupWhat; name: string; mark: SetupMark };
export type SavedSetupEntry = { historyId: string; when: string };
export type SetupView = { id: string; tasks: SetupTask[]; saved: SavedSetupEntry[] };

export function parseSetupView(v: unknown): SetupView | null {
  const f = exactly(v, ["id", "tasks", "saved"]);
  if (!f || typeof f.id !== "string") return null;
  const tasks = listOf(f.tasks, (x) => {
    const t = exactly(x, ["id", "what", "name", "mark"]);
    return t && isHex(t.id) && oneOf(t.what, SETUP_WHATS) && typeof t.name === "string" && oneOf(t.mark, SETUP_MARKS) ? { id: t.id, what: t.what, name: t.name, mark: t.mark } : null;
  });
  const saved = listOf(f.saved, (x) => {
    const s = exactly(x, ["historyId", "when"]);
    return s && typeof s.historyId === "string" && typeof s.when === "string" ? { historyId: s.historyId, when: s.when } : null;
  });
  return tasks && saved ? { id: f.id, tasks, saved } : null;
}

export type SavedSetup = { historyId: string; apps: number; settings: number };

export function parseSavedSetup(v: unknown): SavedSetup | null {
  const f = exactly(v, ["historyId", "apps", "settings"]);
  return f && typeof f.historyId === "string" && isCount(f.apps) && isCount(f.settings) ? { historyId: f.historyId, apps: f.apps, settings: f.settings } : null;
}

/* ---------- Undo preview ---------- */

export type UndoPreview = {
  id: string;
  /** The move's undo book, for `fullmoveUndo` (null: nothing of apps or settings to undo). */
  undoId: string | null;
  appsRemoved: string[];
  appsYours: string[];
  settings: number;
  appData: string[];
  files: number;
  open: boolean;
};

export function parseUndoPreview(v: unknown): UndoPreview | null {
  const f = exactly(v, ["id", "undoId", "appsRemoved", "appsYours", "settings", "appData", "files", "open"]);
  if (!f || typeof f.id !== "string" || !isCount(f.settings) || !isCount(f.files) || typeof f.open !== "boolean") return null;
  if (f.undoId !== null && !(typeof f.undoId === "string" && /^[0-9a-f]{32}$/.test(f.undoId))) return null;
  const appsRemoved = names(f.appsRemoved);
  const appsYours = names(f.appsYours);
  const appData = names(f.appData);
  return appsRemoved && appsYours && appData ? { id: f.id, undoId: f.undoId, appsRemoved, appsYours, settings: f.settings, appData, files: f.files, open: f.open } : null;
}

/* ---------- Clean-up ---------- */

export const CLEANUP_WHATS = ["temporary-files", "model-and-cache"] as const;
export type CleanupWhat = (typeof CLEANUP_WHATS)[number];

export type CleanupEntry = { name: string; bytes: number };
export type CleanupView = { what: CleanupWhat; digest: string; removes: CleanupEntry[]; keeps: CleanupEntry[] };

const parseEntry = (x: unknown): CleanupEntry | null => {
  const e = exactly(x, ["name", "bytes"]);
  return e && typeof e.name === "string" && isCount(e.bytes) ? { name: e.name, bytes: e.bytes } : null;
};

export function parseCleanupView(v: unknown): CleanupView | null {
  const f = exactly(v, ["what", "digest", "removes", "keeps"]);
  if (!f || !oneOf(f.what, CLEANUP_WHATS) || !isHex(f.digest)) return null;
  const removes = listOf(f.removes, parseEntry);
  const keeps = listOf(f.keeps, parseEntry);
  return removes && keeps ? { what: f.what, digest: f.digest, removes, keeps } : null;
}

export type CleanupDone = { what: CleanupWhat; removed: number; kept: number; failed: number };

export function parseCleanupDone(v: unknown): CleanupDone | null {
  const f = exactly(v, ["what", "removed", "kept", "failed"]);
  return f && oneOf(f.what, CLEANUP_WHATS) && isCount(f.removed) && isCount(f.kept) && isCount(f.failed) ? { what: f.what, removed: f.removed, kept: f.kept, failed: f.failed } : null;
}

/* ---------- The report ---------- */

export type Disclosure = { fileNames: boolean; appNames: boolean; accountId: boolean };

export type ReportLine = { name: string | null; isApp: boolean; standing: Standing; reason: Reason | null };

export type ReportView = {
  id: string;
  headline: Headline;
  tally: Tally;
  accountId: string | null;
  lines: ReportLine[];
  othersNotFinished: number;
  /** The before-wipe list as sent: [how many things, how many still open] (null: no list yet). */
  goodbye: [number, number] | null;
  withheld: { names: number; secrets: number; accountIds: number };
  /** Binds a save to exactly this report. */
  digest: string;
};

export function parseReportView(v: unknown): ReportView | null {
  const f = exactly(v, ["id", "headline", "tally", "accountId", "lines", "othersNotFinished", "goodbye", "withheld", "digest"]);
  if (!f || typeof f.id !== "string" || !oneOf(f.headline, HEADLINES) || !isCount(f.othersNotFinished) || !isHex(f.digest)) return null;
  if (f.accountId !== null && typeof f.accountId !== "string") return null;
  const tally = parseTally(f.tally);
  const lines = listOf(f.lines, (x) => {
    const l = exactly(x, ["name", "isApp", "standing", "reason"]);
    if (!l || (l.name !== null && typeof l.name !== "string") || typeof l.isApp !== "boolean" || !oneOf(l.standing, STANDINGS)) return null;
    const reason = parseReason(l.reason);
    return reason === undefined ? null : { name: l.name, isApp: l.isApp, standing: l.standing, reason };
  });
  let goodbye: ReportView["goodbye"] = null;
  if (f.goodbye !== null) {
    if (!Array.isArray(f.goodbye) || f.goodbye.length !== 2 || !isCount(f.goodbye[0]) || !isCount(f.goodbye[1])) return null;
    goodbye = [f.goodbye[0], f.goodbye[1]];
  }
  const w = exactly(f.withheld, ["names", "secrets", "accountIds"]);
  if (!tally || !lines || !w || !isCount(w.names) || !isCount(w.secrets) || !isCount(w.accountIds)) return null;
  return { id: f.id, headline: f.headline, tally, accountId: f.accountId, lines, othersNotFinished: f.othersNotFinished, goodbye, withheld: { names: w.names, secrets: w.secrets, accountIds: w.accountIds }, digest: f.digest };
}
