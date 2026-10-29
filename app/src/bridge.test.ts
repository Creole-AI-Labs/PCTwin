/**
 * The bridge is the only way to the shell, and nothing it has not checked reaches a screen.
 */
import { beforeEach, describe, expect, it, vi } from "vitest";
import calls from "./contract/shell/calls.json";
import rescueDriveLists from "./contract/rescue-drives.json";
import {
  FAILED,
  HELP_STEPS,
  HELP_TOPICS,
  helpCode,
  fullmoveUndo,
  moveAppdataAnswer,
  moveChooseWhat,
  listenUndoClosed,
  listenSafeStop,
  workChecks,
  workCheckAdd,
  workCheckRun,
  workCheckAnswer,
  workCheckRemove,
  repairOffer,
  repairRun,
  moveChanges,
  moveReapprove,
  safeStop,
  recoveryKitSave,
  listenMove,
  recoveryCheck,
  recoveryRun,
  thingsReady,
  listenPairingNew,
  listenPairingOld,
  moveApprove,
  moveBegin,
  parseRescueDrive,
  rescueBegin,
  rescueDrives,
  moveChoosePlace,
  moveUsualPlace,
  movePeopleHere,
  movePlacePeople,
  movePrivacyAnswer,
  listenQuickSend,
  qsChooseFolder,
  qsSaveHere,
  qsDeclineHere,
  qsFolder,
  qsKeepOriginalDates,
  qsOpenFolder,
  qsClear,
  qsAcceptDrops,
  qsOffer,
  qsOfferMissing,
  qsPause,
  qsPick,
  qsReceive,
  qsResume,
  qsSend,
  qsStop,
  qsSendNote,
  qsOpenLink,
  qsLaunchFiles,
  knownConnections,
  knownRemember,
  qsSessionAdd,
  qsSessionEnd,
  qsSessions,
  qsSends,
  qsSendPause,
  qsSendResume,
  qsSendCancel,
  suggestReplacement,
  knownForget,
  knownRename,
  knownAllowFolder,
  knownRevokeFolder,
  knownRenewFolder,
  knownNextFolder,
  knownClearNextFolder,
  knownSaveIncoming,
  knownDeclineIncoming,
  knownSend,
  waitingPrepare,
  waitingSends,
  waitingRenew,
  waitingCancel,
  listenKnown,
  laptopName,
  laptopNameShuffle,
  laptopNameType,
  laptopNameUse,
  updateCheck,
  updateInstall,
  historyExport,
  historyList,
  historyOpen,
  openPrivacySettings,
  movePause,
  moveReport,
  moveResume,
  moveRetry,
  moveStop,
  pairingHeadsUp,
  pairingNewConnect,
  pairingNewSearch,
  pairingOldDecline,
  pairingOldPick,
  pairingOldRename,
  pairingOldShuffle,
  pairingOldStart,
  pairingOldStartAgain,
  pairingOldNearby,
  pairingStop,
  setTrayLabels,
  setWindowMode,
  windowDrag,
  toProblem,
  guardIntent,
  guardView,
  listenGuard,
  type NewEvent,
  type OldEvent,
} from "./bridge";

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

const SOURCES = import.meta.glob<string>(["./**/*.ts", "./**/*.tsx", "!./**/*.test.ts", "!./**/*.test.tsx"], { query: "?raw", import: "default", eager: true });

describe("the only way to the shell", () => {
  it("no screen or module but bridge.ts reaches Tauri", () => {
    const reaching = Object.entries(SOURCES)
      .filter(([, text]) => text.includes("@tauri-apps/") || text.includes("__TAURI"))
      .map(([path]) => path);
    expect(reaching).toEqual(["./bridge.ts"]);
    expect(Object.keys(SOURCES).length).toBeGreaterThan(20);
  });

  it("outside the app, the window and tray requests do nothing", async () => {
    await setWindowMode("mini");
    await setTrayLabels("Show", "Quit");
    expect(tauri.invoke).not.toHaveBeenCalled();
  });

  it("a command with no arguments is sent with none", async () => {
    tauri.invoke.mockResolvedValue(null);
    await pairingStop();
    expect(tauri.invoke).toHaveBeenCalledWith("pairing_stop");
    expect(tauri.invoke.mock.calls[0]).toHaveLength(1);
  });
});

describe("nothing unchecked reaches a screen", () => {
  it("an old-laptop update that does not parse ends pairing on screen, never shown as sent", async () => {
    const seen: OldEvent[] = [];
    await listenPairingOld((e) => seen.push(e));
    const send = (payload: unknown) => {
      tauri.handlers.get("pairing-old")?.({ payload });
    };
    send({ kind: "code", code: "123456", secondsLeft: 9 });
    send({ kind: "code", code: "<img src=x>", secondsLeft: "soon" });
    send({ kind: "something-new" });
    expect(seen).toEqual([{ kind: "code", code: "123456", secondsLeft: 9 }, { kind: "lost" }, { kind: "lost" }]);
  });

  it("a new-laptop update that does not parse is the generic problem", async () => {
    const seen: NewEvent[] = [];
    await listenPairingNew((e) => seen.push(e));
    const send = (payload: unknown) => {
      tauri.handlers.get("pairing-new")?.({ payload });
    };
    send({ kind: "paired" });
    send({ kind: "problem", problem: { key: "pairing-failed" } });
    send("paired");
    expect(seen).toEqual([{ kind: "paired" }, { kind: "problem", problem: FAILED }, { kind: "problem", problem: FAILED }]);
  });

  /** What `reply` was rejected with; a reply that resolves fails the test. */
  async function rejectionOf(reply: Promise<unknown>): Promise<unknown> {
    const outcome = await reply.then(
      (value: unknown) => ({ resolved: true, value }),
      (error: unknown) => ({ resolved: false, value: error }),
    );
    expect(outcome.resolved, `resolved with ${JSON.stringify(outcome.value)}`).toBe(false);
    return outcome.value;
  }

  it("a search reply that does not parse is the generic problem, and so is a wrong number", async () => {
    tauri.invoke.mockResolvedValueOnce([{ index: 0, label: { kind: "named", name: "Ada" }, sameLabelNearby: false, needsUpdate: false }]);
    await expect(pairingNewSearch()).resolves.toEqual([{ index: 0, label: { kind: "named", name: "Ada" }, sameLabelNearby: false, needsUpdate: false }]);
    tauri.invoke.mockResolvedValueOnce([{ index: 0, label: { kind: "named", name: "Ada" } }]);
    expect(toProblem(await rejectionOf(pairingNewSearch()))).toEqual(FAILED);
    tauri.invoke.mockResolvedValueOnce(42);
    await expect(pairingNewConnect(0, "123456")).resolves.toBe(42);
    tauri.invoke.mockResolvedValueOnce(4200);
    expect(toProblem(await rejectionOf(pairingNewConnect(0, "123456")))).toEqual(FAILED);
  });

  it("a notes reply is used only when every note has the engine's shape", async () => {
    tauri.invoke.mockResolvedValueOnce([{ kind: "local-network" }, { kind: "firewall" }]);
    await expect(pairingHeadsUp("new")).resolves.toEqual([{ kind: "local-network" }, { kind: "firewall" }]);
    expect(tauri.invoke).toHaveBeenLastCalledWith("pairing_heads_up", { role: "new" });
    for (const wrong of [[{ kind: "local-network" }, { kind: "something-new" }], [{ kind: "firewall", more: 1 }], { kind: "firewall" }, null]) {
      tauri.invoke.mockResolvedValueOnce(wrong);
      expect(toProblem(await rejectionOf(pairingHeadsUp("old"))), JSON.stringify(wrong)).toEqual(FAILED);
    }
  });

  it("the shown number is refused above 255, the largest the engine can send", async () => {
    tauri.invoke.mockResolvedValueOnce(255);
    await expect(pairingNewConnect(0, "123456")).resolves.toBe(255);
    for (const wrong of [256, -1, 4.5, "42", null]) {
      tauri.invoke.mockResolvedValueOnce(wrong);
      await expect(pairingNewConnect(0, "123456"), String(wrong)).rejects.toThrow("did not have the engine's shape");
    }
  });

  it("a rejection is the engine's problem only when it has the engine's shape", () => {
    expect(toProblem({ key: "pairing-code-wrong", networks: [] })).toEqual({ key: "pairing-code-wrong", networks: [] });
    expect(toProblem({ key: "pairing-choose-network", networks: ["Wi-Fi"] })).toEqual({ key: "pairing-choose-network", networks: ["Wi-Fi"] });
    expect(toProblem("pairing_stop not allowed. Command not found")).toEqual(FAILED);
    expect(toProblem(new Error("boom"))).toEqual(FAILED);
    expect(toProblem({ key: "pairing-code-wrong", networks: [], more: "html" })).toEqual(FAILED);
  });
});

describe("the engine's names (contract/shell/calls.json, written by the engine)", () => {
  it("every command is called by its own name with exactly its own arguments", async () => {
    const delivered: Record<string, unknown> = {
      work_checks: { generation: 0, checks: [], more: 0 },
      work_check_add: { generation: 0, checks: [], more: 0 },
      work_check_run: { generation: 0, checks: [], more: 0 },
      work_check_answer: { generation: 0, checks: [], more: 0 },
      work_check_remove: { generation: 0, checks: [], more: 0 },
      repair_offer: { generation: 0, kept: 0, lines: [], more: 0 },
      repair_run: { generation: 0, kept: 0, lines: [], more: 0, fixed: 0, undoId: null },
      move_reapprove: { generation: 0, kept: 0, lines: [], more: 0, fixed: 0, undoId: null },
      safe_stop: { safe: true, busy: [] },
    };
    tauri.invoke.mockImplementation((command: string) =>
      Promise.resolve(
        command in delivered
          ? delivered[command]
          : command === "pairing_new_search" || command === "pairing_heads_up" || command === "move_report"
          ? []
          : command === "pairing_new_connect"
            ? 42
            : command === "help_code"
              ? "K3QD-8XWA-2M7P"
              : command === "quicksend_pick" || command === "quicksend_offer" || command === "quicksend_offer_missing" || command === "history_list" || command === "quicksend_launch_files" || command.startsWith("known_") || command.startsWith("waiting_") || command.startsWith("quicksend_session") || command.startsWith("quicksend_send")
                ? []
                : command === "recovery_check"
                ? { unfinished: 0 }
                : command === "recovery_run"
                  ? { putInPlace: 0, keptToContinue: 0, notFinished: 0, leftForLater: 0 }
                  : null,
      ),
    );
    (window as unknown as Record<string, unknown>).__TAURI_INTERNALS__ = {};
    try {
      await setWindowMode("mini");
      await windowDrag();
      await setTrayLabels("Show", "Quit");
      await pairingOldStart(null);
      await pairingOldPick(1, 42);
      await pairingOldDecline(1);
      await pairingOldShuffle();
      await pairingOldRename("Kitchen laptop");
      await pairingOldStartAgain();
      await pairingOldNearby().catch(() => undefined);
      await pairingHeadsUp("old");
      await pairingStop();
      await pairingNewSearch();
      await pairingNewConnect(0, "123456");
      await moveBegin({ music: "From your old laptop/Music" }, ["1001"], "Approve reading the files of {names}.");
      await movePeopleHere().catch(() => undefined);
      await rescueDrives().catch(() => undefined);
      await rescueBegin("2f1c9e3a-7b44-4c1e-9a7d-0e5b6c8d9f10", { music: "From your old laptop/Music" });
      await movePrivacyAnswer(true);
      await openPrivacySettings();
      await qsPick(false);
      await qsClear();
      await qsAcceptDrops(true);
      await qsOffer(["p1"]);
      await qsOfferMissing();
      await qsSend();
      await qsReceive();
      await qsChooseFolder(1);
      await qsSaveHere(1);
      await qsDeclineHere(1);
      await qsFolder().catch(() => undefined);
      await qsOpenFolder();
      await qsKeepOriginalDates().catch(() => undefined);
      await qsPause();
      await qsResume();
      await qsStop();
      await qsSendNote("Wi-Fi: Home");
      await qsOpenLink("ffeeddccbbaa99887766554433221100");
      await qsLaunchFiles();
      await knownConnections();
      await knownRemember("Ada's laptop", "0011223344556677");
      await knownForget("0011223344556677");
      await knownRename("0011223344556677", "Ada (work)");
      await knownAllowFolder("0011223344556677");
      await knownRevokeFolder("0011223344556677");
      await knownRenewFolder("0011223344556677");
      await knownNextFolder("0011223344556677");
      await knownClearNextFolder("0011223344556677");
      await knownSaveIncoming(4, true);
      await knownDeclineIncoming(4);
      await knownSend("0011223344556677");
      await waitingPrepare(["0011223344556677"], 7);
      await waitingSends();
      await waitingRenew("00112233445566778899aabbccddeeff", 7);
      await waitingCancel("00112233445566778899aabbccddeeff");
      await qsSessionAdd("00112233445566778899aabbccddeeff");
      await qsSessionEnd("00112233445566778899aabbccddeeff");
      await qsSessions();
      await qsSends();
      await qsSendPause(2);
      await qsSendResume(2);
      await qsSendCancel(2);
      await updateCheck();
      await updateInstall();
      await laptopName().catch(() => undefined);
      await laptopNameShuffle().catch(() => undefined);
      await laptopNameUse().catch(() => undefined);
      await laptopNameType("Ezzy Laptop").catch(() => undefined);
      await historyList();
      await historyOpen("1792627200000-0").catch(() => undefined);
      await historyExport();
      await movePlacePeople([{ id: "S-2", to: { kind: "new" } }], { title: "New account", body: "{name}: {account} / {password}", approve: "Approve reading the files of {names}." });
      await moveChoosePlace("pictures").catch(() => undefined);
      await moveUsualPlace("pictures");
      await moveApprove({ revision: 2, fingerprint: "9f" }, true, ["music"], [2], [0], true, "From Bea's laptop", true, true);
      await movePause();
      await moveResume();
      await moveStop();
      await moveRetry();
      await moveReport();
      await helpCode("connection-lost", "moving");
      await recoveryCheck();
      await recoveryRun();
      await thingsReady();
      await guardView();
      await guardIntent({ job: "job-1", epoch: 2, generation: 3, press: "p-1", intent: { kind: "try-again", line: 0 } });
      await fullmoveUndo("01010101010101010101010101010101");
      await moveAppdataAnswer(true);
      await moveChooseWhat({ folders: [{ role: "documents", parts: ["Work"] }], apps: true });
      await workChecks();
      await workCheckAdd({ kind: "font", family: "Inter" });
      await workCheckRun();
      await workCheckAnswer(0, 1, true);
      await workCheckRemove(0, 1);
      await repairOffer();
      await repairRun(1, [0, 2]);
      await moveChanges();
      await moveReapprove(1, true);
      await safeStop();
      await recoveryKitSave("1792627200000-0");
    } finally {
      delete (window as unknown as Record<string, unknown>).__TAURI_INTERNALS__;
    }
    const sent = tauri.invoke.mock.calls.map(([command, args]) => ({ name: command as string, args: Object.keys((args ?? {}) as object).sort() }));
    const engine = calls.commands.map((c) => ({ name: c.name, args: c.args.map(([key]) => key as string).sort() }));
    const byName = (a: { name: string }, b: { name: string }) => a.name.localeCompare(b.name);
    expect(sent.sort(byName)).toEqual(engine.sort(byName));
  });

  it("the screens name help topics and steps from the engine's own lists", () => {
    expect([...HELP_TOPICS]).toEqual(calls.helpTopics);
    expect([...HELP_STEPS]).toEqual(calls.helpSteps);
  });

  it("the screens listen to the engine's own events", async () => {
    await listenPairingOld(() => undefined);
    await listenPairingNew(() => undefined);
    await listenMove(() => undefined);
    await listenQuickSend(() => undefined);
    await listenKnown(() => undefined);
    await listenGuard(() => undefined);
    await listenUndoClosed(() => undefined);
    await listenSafeStop(() => undefined);
    expect([...tauri.handlers.keys()].sort()).toEqual(Object.values(calls.events).sort());
  });
});

describe("a rescue's drives (2.6)", () => {
  it("reads the engine's example drives and refuses anything else", () => {
    for (const list of rescueDriveLists) for (const d of list) expect(parseRescueDrive(d)).not.toBeNull();
    const good = rescueDriveLists[0][0];
    expect(parseRescueDrive({ ...good, id: "" })).toBeNull();
    expect(parseRescueDrive({ ...good, totalBytes: -1 })).toBeNull();
    expect(parseRescueDrive({ ...good, path: "C:\\" })).toBeNull();
    const missing: Record<string, unknown> = { ...good };
    delete missing.looksLikeALaptop;
    expect(parseRescueDrive(missing)).toBeNull();
  });
});

describe("remembering a laptop paired again", () => {
  it("suggests the earlier connection with the name typed, and nothing else", () => {
    const connections = [
      { id: "0011223344556677", label: "Ada's laptop", lastUsed: 2 },
      { id: "8899aabbccddeeff", label: "Kitchen laptop", lastUsed: 1 },
    ];
    expect(suggestReplacement(connections, "  ada's LAPTOP ")?.id).toBe("0011223344556677");
    expect(suggestReplacement(connections, "Ada")).toBeNull();
    expect(suggestReplacement(connections, "   ")).toBeNull();
    expect(suggestReplacement([], "Ada's laptop")).toBeNull();
  });
});
