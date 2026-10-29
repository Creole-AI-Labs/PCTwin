/**
 * Finishing a move: the engine's own examples (contract/finish/*.json) are each read by finish.ts
 * exactly as sent, a copy with an extra field or an unknown word is refused, and every command in
 * contract/shell/finish-calls.json is called by name with exactly those arguments.
 */
import { beforeEach, describe, expect, it, vi } from "vitest";
import calls from "./contract/shell/finish-calls.json";
import {
  catchupApprove,
  catchupArm,
  cleanupConfirm,
  cleanupPreview,
  doctorRun,
  finishOverview,
  goodbyeAnswer,
  goodbyeKeepUntil,
  goodbyeList,
  handoverConfirm,
  handoverPreview,
  handoverResult,
  listenFinishDelta,
  reportPreview,
  reportSave,
  setupChecklist,
  setupMark,
  setupSave,
  toFinishProblem,
  undoPreview,
} from "./bridge";
import * as fin from "./finish";

const tauri = vi.hoisted(() => ({
  invoke: vi.fn(),
  handlers: new Map<string, (event: { payload: unknown }) => void>(),
}));
vi.mock("@tauri-apps/api/core", () => ({ invoke: tauri.invoke }));
vi.mock("@tauri-apps/api/event", () => ({
  listen: (name: string, handler: (event: { payload: unknown }) => void) => {
    tauri.handlers.set(name, handler);
    return Promise.resolve(() => undefined);
  },
}));

beforeEach(() => {
  tauri.invoke.mockReset();
  tauri.handlers.clear();
});

const FILES = import.meta.glob<unknown[]>("./contract/finish/*.json", { import: "default", eager: true });
const examples = (name: string): Record<string, unknown>[] => (FILES[`./contract/finish/${name}.json`] ?? []) as Record<string, unknown>[];

const PARSERS: Record<string, (v: unknown) => unknown> = {
  "finish-view": fin.parseFinishView,
  "goodbye-view": fin.parseGoodbyeView,
  "finish-delta": fin.parseDeltaView,
  "handover-view": fin.parseHandoverView,
  "handover-started": fin.parseHandoverStarted,
  "handover-recorded": fin.parseHandoverRecorded,
  "doctor-view": fin.parseDoctorView,
  "setup-view": fin.parseSetupView,
  "saved-setup": fin.parseSavedSetup,
  "undo-preview": fin.parseUndoPreview,
  "cleanup-view": fin.parseCleanupView,
  "cleanup-done": fin.parseCleanupDone,
  "report-view": fin.parseReportView,
  "finish-problem": fin.parseFinishProblem,
};

describe("the engine's finishing examples", () => {
  it("are all here, each with a parse function", () => {
    const names = Object.keys(FILES).map((p) => p.replace(/^.*\/|\.json$/g, ""));
    expect(names.sort()).toEqual(Object.keys(PARSERS).sort());
  });

  it.each(Object.keys(PARSERS))("%s: every example parses to exactly what was sent", (name) => {
    const list = examples(name);
    expect(list.length).toBeGreaterThan(0);
    for (const example of list) {
      const parsed = PARSERS[name](example);
      expect(parsed, JSON.stringify(example)).not.toBeNull();
      expect(parsed).toEqual(example);
    }
  });

  it.each(Object.keys(PARSERS))("%s: a copy with an extra field, a missing field or a wrong type is refused", (name) => {
    const parse = PARSERS[name];
    for (const example of examples(name)) {
      expect(parse({ ...example, extra: 1 }), "extra field").toBeNull();
      for (const key of Object.keys(example)) {
        const missing = Object.fromEntries(Object.entries(example).filter(([k]) => k !== key));
        expect(parse(missing), `without ${key}`).toBeNull();
        expect(parse({ ...example, [key]: { wrong: true } }), `${key} of the wrong type`).toBeNull();
      }
    }
  });

  it("the examples hold every word of the closed lists the screens know", () => {
    const delta = examples("finish-delta").flatMap((e) => (e.lines as { change: string }[]).map((l) => l.change));
    expect(new Set(delta)).toEqual(new Set(fin.CHANGE_KEYS));
    const whats = examples("setup-view").flatMap((e) => (e.tasks as { what: string }[]).map((t) => t.what));
    expect(new Set(whats)).toEqual(new Set(fin.SETUP_WHATS));
    const why = examples("handover-view").flatMap((e) => (e.kind === "refused" ? [e.why] : []));
    expect(new Set(why)).toEqual(new Set(fin.REFUSAL_KEYS));
    const notes = examples("handover-view").flatMap((e) => (e.notes as string[] | undefined) ?? []);
    expect(new Set(notes)).toEqual(new Set(fin.HANDOVER_NOTES));
    const outcomes = examples("handover-recorded").map((e) => e.outcome);
    expect(new Set(outcomes)).toEqual(new Set(fin.OUTCOME_KEYS));
  });
});

describe("shapes the engine never sends are refused", () => {
  const view = examples("finish-view")[0] ?? {};
  const goodbye = examples("goodbye-view")[0] ?? {};
  const ready = examples("handover-view").find((e) => e.kind === "ready") ?? {};
  const line = (view.notFinished as Record<string, unknown>[])[0] ?? {};
  const task = (examples("setup-view")[0]?.tasks as Record<string, unknown>[])[0] ?? {};
  const delta = examples("finish-delta")[0] ?? {};
  const exception = (goodbye.exceptions as Record<string, unknown>[])[0] ?? {};

  it.each([
    ["an unknown headline", fin.parseFinishView, { ...view, headline: "all-done" }],
    ["a headline in the wrong case", fin.parseFinishView, { ...view, headline: "Ready-For-Selected" }],
    ["an unknown note", fin.parseFinishView, { ...view, notes: ["celebrate"] }],
    ["a negative count", fin.parseFinishView, { ...view, others: -1 }],
    ["a fractional count", fin.parseFinishView, { ...view, more: 1.5 }],
    ["an unknown standing", fin.parseFinishView, { ...view, notFinished: [{ ...line, standing: "mostly" }] }],
    ["a reason that is not one of the engine's", fin.parseFinishView, { ...view, notFinished: [{ ...line, reason: "bad-luck" }] }],
    ["an unknown group in the after-move check", fin.parseFinishView, { ...view, afterMove: [{ group: "games", atScan: 1, onNewLaptop: 1 }] }],
    ["an unknown exception key", fin.parseGoodbyeView, { ...goodbye, exceptions: [{ ...exception, key: "mystery" }] }],
    ["an answer that is not a resolution", fin.parseGoodbyeView, { ...goodbye, exceptions: [{ ...exception, resolution: { kind: "forgiven", at: 1 } }] }],
    ["an unknown change", fin.parseDeltaView, { ...delta, lines: [{ id: "01", label: "a", change: "vanished", needsChoice: false }] }],
    ["an unknown purpose", fin.parseDeltaView, { ...delta, purpose: "everything" }],
    ["a handover action that is not offered", fin.parseHandoverView, { ...ready, action: "format-everything" }],
    ["an unknown handover note", fin.parseHandoverView, { ...ready, notes: ["totally-safe"] }],
    ["an unknown refusal", fin.parseHandoverView, { kind: "refused", action: "erase-old-laptop", why: "because", open: 0 }],
    ["an unknown system", fin.parseHandoverStarted, { action: "erase-old-laptop", undoClosed: true, opened: true, route: { system: "beos", route: "a", steps: [], canOpen: true } }],
    ["a certificate for a handover", fin.parseHandoverRecorded, { action: "erase-old-laptop", outcome: "certified-erased", at: 1 }],
    ["an unknown mark on a setup task", fin.parseSetupView, { id: "a", saved: [], tasks: [{ ...task, mark: "verified-by-me" }] }],
    ["a setup task about nothing the engine names", fin.parseSetupView, { id: "a", saved: [], tasks: [{ ...task, what: "do-anything" }] }],
    ["a clean-up of something else", fin.parseCleanupView, { what: "documents", digest: "0b", removes: [], keeps: [] }],
    ["a message key with markup", fin.parseDoctorView, { findings: [{ cause: "<b>x</b>", fix: "doctor-x-fix", sure: true, names: [] }], hotspot: { suggested: false }, notChecked: [] }],
    ["a report goodbye with three numbers", fin.parseReportView, { ...(examples("report-view")[0] ?? {}), goodbye: [1, 2, 3] }],
    ["an undo book that is not a hex ID", fin.parseUndoPreview, { ...(examples("undo-preview")[0] ?? {}), undoId: "../../etc" }],
    ["a problem the engine does not know", fin.parseFinishProblem, { kind: "oops" }],
  ])("%s", (_name, parse, value) => {
    expect(parse(value)).toBeNull();
  });

  it("a rejected command is the engine's problem; anything else is the generic problem", () => {
    expect(toFinishProblem({ kind: "changed" })).toEqual({ kind: "changed" });
    expect(toFinishProblem({ kind: "changed", extra: 1 })).toEqual({ kind: "failed" });
    expect(toFinishProblem(new Error("boom"))).toEqual({ kind: "failed" });
    expect(toFinishProblem("no-move")).toEqual({ kind: "failed" });
  });
});

describe("the engine's names (contract/shell/finish-calls.json, written by the engine)", () => {
  const REPLIES: Record<string, string> = {
    finish_overview: "finish-view",
    goodbye_list: "goodbye-view",
    goodbye_answer: "goodbye-view",
    goodbye_keep_until: "goodbye-view",
    handover_preview: "handover-view",
    handover_confirm: "handover-started",
    handover_result: "handover-recorded",
    doctor_run: "doctor-view",
    setup_checklist: "setup-view",
    setup_mark: "setup-view",
    setup_save: "saved-setup",
    undo_preview: "undo-preview",
    cleanup_preview: "cleanup-view",
    cleanup_confirm: "cleanup-done",
    report_preview: "report-view",
  };
  const reply = (name: string): unknown => {
    if (name === "catchup_arm" || name === "catchup_approve") return undefined;
    if (name === "report_save") return "C:/Users/Ada/Documents/move-report.txt";
    return examples(REPLIES[name] ?? "")[0];
  };

  type Args = Record<string, unknown>;
  type Disc = { fileNames: boolean; appNames: boolean; accountId: boolean };
  const CALLS: Record<string, (a: Args) => Promise<unknown>> = {
    finish_overview: (a) => finishOverview(a.id as string),
    goodbye_list: (a) => goodbyeList(a.id as string),
    goodbye_answer: (a) => goodbyeAnswer(a.id as string, a.exception as string, a.shown as string, a.answer as "accept-loss"),
    goodbye_keep_until: (a) => goodbyeKeepUntil(a.id as string, a.at as number),
    catchup_arm: (a) => catchupArm(a.id as string, a.purpose as "check"),
    catchup_approve: (a) => catchupApprove(a.id as string, a.digest as string, a.choices as { id: string; choice: "keep-both" }[]),
    handover_preview: (a) => handoverPreview(a.id as string, a.action as "remove-my-account"),
    handover_confirm: (a) => handoverConfirm(a.id as string, a.action as "erase-old-laptop", a.digest as string),
    handover_result: (a) => handoverResult(a.id as string, a.action as "erase-old-laptop", a.says as "cannot-tell"),
    doctor_run: (a) => doctorRun(a.role as "new"),
    setup_checklist: (a) => setupChecklist(a.id as string),
    setup_mark: (a) => setupMark(a.id as string, a.task as string, a.done as boolean),
    setup_save: (a) => setupSave(a.id as string),
    undo_preview: (a) => undoPreview(a.id as string),
    cleanup_preview: (a) => cleanupPreview(a.what as "temporary-files"),
    cleanup_confirm: (a) => cleanupConfirm(a.what as "model-and-cache", a.digest as string),
    report_preview: (a) => reportPreview(a.id as string, a.disclosure as Disc),
    report_save: (a) => reportSave(a.id as string, a.disclosure as Disc, a.digest as string, a.text as string),
  };

  it("every command is called by name with exactly the engine's arguments", async () => {
    expect(calls.commands.map((c) => c.name).sort()).toEqual(Object.keys(CALLS).sort());
    for (const c of calls.commands) {
      tauri.invoke.mockReset();
      tauri.invoke.mockResolvedValue(reply(c.name));
      const args = Object.fromEntries(c.args.map(([k, v]) => [k, v])) as Args;
      await CALLS[c.name](args);
      expect(tauri.invoke, c.name).toHaveBeenCalledTimes(1);
      expect(tauri.invoke).toHaveBeenCalledWith(c.name, args);
    }
  });

  it("a move named by none is sent as none (the newest move)", async () => {
    tauri.invoke.mockResolvedValue(examples("finish-view")[0]);
    await finishOverview(null);
    expect(tauri.invoke).toHaveBeenCalledWith("finish_overview", { id: null });
  });

  it("a reply that does not have the engine's shape is refused, never handed to a screen", async () => {
    tauri.invoke.mockResolvedValue({ ...examples("finish-view")[0], headline: "all-done" });
    await expect(finishOverview(null)).rejects.toThrow(/did not have the engine's shape/);
    tauri.invoke.mockResolvedValue(42);
    await expect(reportSave(null, { fileNames: false, appNames: false, accountId: false }, "0a", "x")).rejects.toThrow();
  });

  it("a report the person chose nowhere to save is null", async () => {
    tauri.invoke.mockResolvedValue(null);
    await expect(reportSave(null, { fileNames: false, appNames: false, accountId: false }, "0a", "x")).resolves.toBeNull();
  });

  it("the delta event is the one the engine names, and one that does not parse reaches the screen as null", async () => {
    const seen: unknown[] = [];
    await listenFinishDelta((d) => seen.push(d));
    expect([...tauri.handlers.keys()]).toEqual([calls.events.finishDelta]);
    const send = (payload: unknown) => tauri.handlers.get("finish-delta")?.({ payload });
    send(examples("finish-delta")[0]);
    send({ nonsense: true });
    expect(seen).toEqual([examples("finish-delta")[0], null]);
  });
});
