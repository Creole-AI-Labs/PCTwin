import { LocalizationProvider } from "@fluent/react";
import type { ReactNode } from "react";
import { act, cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import offeredExamples from "../contract/qs-offered.json";
import receipts from "../contract/qs-receipt.json";
import { localization } from "../i18n";
import QuickHost from "./QuickHost";

const tauri = vi.hoisted(() => ({
  invoke: vi.fn(),
  handlers: new Map<string, (event: { payload: unknown }) => void>(),
}));

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
  expect(warnings.filter((w) => /missing|not found/i.test(w))).toEqual([]);
  cleanup();
  vi.restoreAllMocks();
});

function inApp(ui: ReactNode) {
  return render(
    <LocalizationProvider l10n={l10n}>
      <main>{ui}</main>
    </LocalizationProvider>,
  );
}

function send(event: string, payload: unknown) {
  act(() => {
    tauri.handlers.get(event)?.({ payload });
  });
}

const heading = () => screen.getByRole("heading", { level: 1 }).textContent;

describe("sending", () => {
  it("adds what the person picks in the system's picker, keeps their ticks, and offers only the ticked things", async () => {
    replies.quicksend_pick = () => [
      { id: "p1", name: "letter.txt", kind: "file", included: true, bytes: 1200, files: 1 },
      { id: "p2", name: "Holiday", kind: "folder", included: true, bytes: null, files: null },
    ];
    replies.quicksend_offer = () => offeredExamples;
    inApp(<QuickHost mode="send" onHome={vi.fn()} />);
    expect(heading()).toBe(text("qs-choose-title"));
    fireEvent.click(screen.getByRole("button", { name: new RegExp(text("qs-add-files")) }));
    await waitFor(() => {
      expect(screen.getByText("letter.txt")).toBeTruthy();
    });
    expect(tauri.invoke).toHaveBeenCalledWith("quicksend_pick", { folders: false });
    fireEvent.click(screen.getByRole("checkbox", { name: (n) => n.includes("Holiday") }));
    fireEvent.click(screen.getByRole("button", { name: new RegExp(text("qs-choose-continue")) }));
    await waitFor(() => {
      expect(tauri.invoke).toHaveBeenCalledWith("quicksend_offer", { ids: ["p1"] });
    });
    await waitFor(() => {
      expect(heading()).toBe(text("qs-check-title"));
    });
  });
});

describe("receiving", () => {
  it("shows what is offered, takes the folder from the system's picker, accepts, and ends on its receipt", async () => {
    replies.quicksend_choose_folder = () => ({ path: "C:/Users/Ada/Downloads", freeBytes: 80_000_000_000 });
    inApp(<QuickHost mode="receive" onHome={vi.fn()} />);
    // Pairing first: the new laptop's search.
    await waitFor(() => {
      expect(tauri.invoke).toHaveBeenCalledWith("pairing_new_search");
    });
    send("quicksend-incoming", { files: offeredExamples });
    expect(heading()).toBe(text("qs-offer-title"));
    fireEvent.click(screen.getByRole("button", { name: new RegExp(text("qs-offer-accept")) }));
    expect(heading()).toBe(text("qs-land-title"));
    fireEvent.click(screen.getByRole("button", { name: new RegExp(text("qs-land-choose")) }));
    await waitFor(() => {
      expect(screen.getByText("C:/Users/Ada/Downloads")).toBeTruthy();
    });
    fireEvent.click(screen.getByRole("checkbox", { name: (n) => n.includes(text("qs-land-confirm")) }));
    fireEvent.click(screen.getByRole("button", { name: new RegExp(text("qs-land-use")) }));
    await waitFor(() => {
      expect(tauri.invoke).toHaveBeenCalledWith("quicksend_answer", { answer: { kind: "accept", ids: null } });
    });
    send("quicksend-receipt", receipts[0]);
    expect(heading()).toBe(text("qs-receipt-title"));
    // A file there but not finished is never shown as arrived.
    expect(screen.getByText((t) => t.includes("d.jpg") && t.includes("not finished"))).toBeTruthy();
  });

  it("declining the offer tells the engine, and a lost connection says so", async () => {
    inApp(<QuickHost mode="receive" onHome={vi.fn()} />);
    await waitFor(() => {
      expect(tauri.invoke).toHaveBeenCalledWith("pairing_new_search");
    });
    send("quicksend-incoming", { files: offeredExamples });
    fireEvent.click(screen.getByRole("button", { name: new RegExp(text("qs-offer-decline")) }));
    await waitFor(() => {
      expect(tauri.invoke).toHaveBeenCalledWith("quicksend_answer", { answer: { kind: "decline" } });
    });
    send("quicksend-problem", { kind: "link-lost", detail: "the connection dropped" });
    expect(heading()).toBe(text("qs-problem-lost-title"));
  });

  it("a word from the engine that does not parse ends on the problem page", async () => {
    inApp(<QuickHost mode="receive" onHome={vi.fn()} />);
    await waitFor(() => {
      expect(tauri.invoke).toHaveBeenCalledWith("pairing_new_search");
    });
    send("quicksend-incoming", { files: [{ id: 1 }] });
    expect(heading()).toBe(text("qs-problem-failed-title"));
  });
});
