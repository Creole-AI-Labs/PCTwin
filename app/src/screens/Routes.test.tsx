/**
 * Routes to finish a move from where the person is: Fix what did not finish from the results page and
 * from My history (the newest move only), and Bring me up to date after pairing (the catch-up review
 * on the new laptop, kept until the approved changes start moving).
 */
import { LocalizationProvider } from "@fluent/react";
import { act, cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import type { ReactNode } from "react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import deltas from "../contract/finish/finish-delta.json";
import outcomes from "../contract/move-outcome.json";
import progresses from "../contract/move-progress.json";
import offers from "../contract/repair-offer.json";
import { localization } from "../i18n";
import type { HistoryEntry } from "../quicksend";
import { violations } from "../testing/a11y";
import { FooterProvider } from "./FrameFooter";
import History from "./History";
import MoveHost, { next } from "./MoveHost";
import Outcome from "./Outcome";

const tauri = vi.hoisted(() => ({ invoke: vi.fn(), handlers: new Map<string, (event: { payload: unknown }) => void>() }));
vi.mock("@tauri-apps/api/core", () => ({ invoke: tauri.invoke }));
vi.mock("@tauri-apps/api/event", () => ({
  listen: (name: string, handler: (event: { payload: unknown }) => void) => {
    tauri.handlers.set(name, handler);
    return Promise.resolve(() => {
      tauri.handlers.delete(name);
    });
  },
}));

const l10n = localization("en");
const text = (id: string, vars?: Record<string, string | number>) => l10n.getString(id, vars);
const plain = (s: string) => s.replace(/[⁦-⁩]/g, "");
let warnings: string[] = [];
let replies: Partial<Record<string, (args: unknown) => unknown>> = {};

beforeEach(() => {
  warnings = [];
  replies = {};
  tauri.handlers.clear();
  tauri.invoke.mockReset();
  tauri.invoke.mockImplementation((command: string, args?: unknown) => Promise.resolve().then(() => replies[command]?.(args)));
  vi.spyOn(console, "warn").mockImplementation((...args: unknown[]) => {
    warnings.push(args.map(String).join(" "));
  });
});

afterEach(() => {
  expect(warnings).toEqual([]);
  cleanup();
  vi.restoreAllMocks();
});

const show = (ui: ReactNode) =>
  render(
    <LocalizationProvider l10n={l10n}>
      <FooterProvider>
        <main>{ui}</main>
      </FooterProvider>
    </LocalizationProvider>,
  );
const send = (event: string, payload: unknown) => {
  act(() => {
    tauri.handlers.get(event)?.({ payload });
  });
};
const h1 = () => screen.getByRole("heading", { level: 1 }).textContent;
const outcome = outcomes[0];
const clean = { ...outcome, byStatus: { ...outcome.byStatus, failed: 0, "needs-action": 0 } };
const begun = () => waitFor(() => { expect(tauri.invoke).toHaveBeenCalledWith("move_begin", expect.anything()); });

describe("Fix what did not finish, from the results page", () => {
  it("is offered only when something failed or needs action", () => {
    const onRepair = vi.fn();
    const { unmount } = show(<Outcome outcome={outcome as never} onReport={() => undefined} onRepair={onRepair} />);
    fireEvent.click(screen.getByRole("button", { name: text("repair-strip") }));
    expect(onRepair).toHaveBeenCalledOnce();
    unmount();
    show(<Outcome outcome={clean as never} onReport={() => undefined} onRepair={onRepair} />);
    expect(screen.queryByRole("button", { name: text("repair-strip") })).toBeNull();
  });

  it("on the new laptop opens the repair page, and Back returns to the results", async () => {
    replies.repair_offer = () => offers[0];
    show(<MoveHost old={false} onPairAgain={() => undefined} onHome={() => undefined} />);
    await begun();
    send("move-finished", outcome);
    fireEvent.click(screen.getByRole("button", { name: text("repair-strip") }));
    expect(await screen.findByText(plain(text("repair-body", { kept: offers[0].kept })))).toBeTruthy();
    fireEvent.click(screen.getByRole("button", { name: text("frame-back") }));
    expect(h1()).toBe(text("outcome-title"));
  });

  it("the old laptop has no such button", async () => {
    show(<MoveHost old={true} onPairAgain={() => undefined} onHome={() => undefined} />);
    await begun();
    send("move-finished", outcome);
    expect(screen.queryByRole("button", { name: text("repair-strip") })).toBeNull();
  });
});

describe("Fix items, from My history", () => {
  const entry = (id: string, when: string, state: HistoryEntry["state"]): HistoryEntry => ({ id, kind: "move", when, from: "Old", to: "New", state, stepsLeft: 1 });
  const list = (entries: HistoryEntry[], onRepair = vi.fn()) =>
    show(<History entries={entries} onOpen={() => undefined} onContinue={() => undefined} onRepair={onRepair} onExport={() => undefined} onStart={() => undefined} onBack={() => undefined} />);
  const repairButtons = () => screen.queryAllByRole("button", { name: new RegExp(`^${text("history-repair")}`) });

  it("is only on the newest move", () => {
    const onRepair = vi.fn();
    list([entry("a", "2026-10-20T00:00:00Z", "finished"), entry("b", "2026-10-22T00:00:00Z", "steps-left"), entry("c", "2026-10-21T00:00:00Z", "stopped")], onRepair);
    const buttons = repairButtons();
    expect(buttons).toHaveLength(1);
    expect(buttons[0].getAttribute("aria-label")).toContain(text("history-row-move", { from: "Old", to: "New" }));
    fireEvent.click(buttons[0]);
    expect(onRepair).toHaveBeenCalledOnce();
  });

  it("is not shown for a paused newest move", () => {
    list([entry("a", "2026-10-22T00:00:00Z", "paused")]);
    expect(repairButtons()).toHaveLength(0);
  });
});

describe("Bring me up to date, after pairing", () => {
  it("the new laptop shows the changes to review instead of a plan, and keeps them until files move", async () => {
    show(<MoveHost old={false} onPairAgain={() => undefined} onHome={() => undefined} />);
    await begun();
    send("finish-delta", deltas[0]);
    expect(h1()).toBe(text("finish-catchup-title"));
    expect(screen.getByRole("heading", { level: 2, name: text("finish-catchup-changes-title") })).toBeTruthy();
    // A stage word does not take the review away.
    send("move-stage", { kind: "moving" });
    expect(h1()).toBe(text("finish-catchup-title"));
    expect(await violations({ region: { enabled: false } })).toEqual([]);
    send("move-progress", Object.fromEntries(Object.entries(progresses[0]).filter(([key]) => key !== "notice")));
    expect(h1()).toBe(text("progress-title"));
  });

  it("a list that cannot be read says so, and Back goes home", async () => {
    const onHome = vi.fn();
    show(<MoveHost old={false} onPairAgain={() => undefined} onHome={onHome} />);
    await begun();
    send("finish-delta", { nonsense: true });
    expect(screen.getByText(text("finish-catchup-unreadable"))).toBeTruthy();
    fireEvent.click(screen.getByRole("button", { name: text("frame-back") }));
    expect(onHome).toHaveBeenCalledOnce();
  });

  it("the old laptop never listens for it", async () => {
    show(<MoveHost old={true} onPairAgain={() => undefined} onHome={() => undefined} />);
    await waitFor(() => { expect(tauri.invoke).toHaveBeenCalled(); });
    expect(tauri.handlers.has("finish-delta")).toBe(false);
  });

  it("only a progress word, the outcome or a problem leaves the review", () => {
    const view = { kind: "catchup", delta: null } as const;
    expect(next(view, { kind: "stage", stage: { kind: "waiting" } })).toBe(view);
    expect(next(view, { kind: "finished", outcome: outcome as never }).kind).toBe("outcome");
    expect(next(view, { kind: "problem", problem: { kind: "not-approved" } }).kind).toBe("end");
  });
});
