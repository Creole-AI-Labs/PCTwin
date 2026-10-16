/**
 * The bridge is the only way to the shell, and nothing it has not checked reaches a screen.
 */
import { beforeEach, describe, expect, it, vi } from "vitest";
import { FAILED, listenPairingNew, listenPairingOld, pairingNewConnect, pairingNewSearch, pairingStop, setTrayLabels, setWindowMode, toProblem, type NewEvent, type OldEvent } from "./bridge";

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

  it("a search reply that does not parse is the generic problem, and so is a wrong number", async () => {
    tauri.invoke.mockResolvedValueOnce([{ index: 0, label: { kind: "named", name: "Ada" }, sameLabelNearby: false }]);
    await expect(pairingNewSearch()).resolves.toEqual([{ index: 0, label: { kind: "named", name: "Ada" }, sameLabelNearby: false }]);
    tauri.invoke.mockResolvedValueOnce([{ index: 0, label: { kind: "named", name: "Ada" } }]);
    expect(toProblem(await pairingNewSearch().catch((e: unknown) => e))).toEqual(FAILED);
    tauri.invoke.mockResolvedValueOnce(42);
    await expect(pairingNewConnect(0, "123456")).resolves.toBe(42);
    tauri.invoke.mockResolvedValueOnce(4200);
    expect(toProblem(await pairingNewConnect(0, "123456").catch((e: unknown) => e))).toEqual(FAILED);
  });

  it("a rejection is the engine's problem only when it has the engine's shape", () => {
    expect(toProblem({ key: "pairing-code-wrong", networks: [] })).toEqual({ key: "pairing-code-wrong", networks: [] });
    expect(toProblem({ key: "pairing-choose-network", networks: ["Wi-Fi"] })).toEqual({ key: "pairing-choose-network", networks: ["Wi-Fi"] });
    expect(toProblem("pairing_stop not allowed. Command not found")).toEqual(FAILED);
    expect(toProblem(new Error("boom"))).toEqual(FAILED);
    expect(toProblem({ key: "pairing-code-wrong", networks: [], more: "html" })).toEqual(FAILED);
  });
});
