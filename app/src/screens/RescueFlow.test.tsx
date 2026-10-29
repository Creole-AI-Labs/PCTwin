/**
 * Rescue a laptop (2.6) after a drive is chosen: the new laptop's move pages, started with
 * `rescue_begin` for that drive, with words that fit a drive (nobody "sends" from it, nothing on
 * it changes) and no pages about meeting another laptop.
 */
import { LocalizationProvider } from "@fluent/react";
import { act, cleanup, render, screen, waitFor } from "@testing-library/react";
import type { ReactNode } from "react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import outcomes from "../contract/move-outcome.json";
import plans from "../contract/plan-view.json";
import progresses from "../contract/move-progress.json";
import { localization } from "../i18n";
import { violations } from "../testing/a11y";
import MoveHost from "./MoveHost";

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

beforeEach(() => {
  warnings = [];
  tauri.handlers.clear();
  tauri.invoke.mockReset();
  tauri.invoke.mockImplementation(() => Promise.resolve(null));
  vi.spyOn(console, "warn").mockImplementation((...args: unknown[]) => {
    warnings.push(args.map(String).join(" "));
  });
});

afterEach(() => {
  expect(warnings).toEqual([]);
  cleanup();
  vi.restoreAllMocks();
  document.documentElement.dir = "ltr";
});

const drive = { id: "2f1c9e3a-7b44-4c1e-9a7d-0e5b6c8d9f10", name: "Old laptop drive (E:\\)", fileSystem: "NTFS", totalBytes: 256000000000, looksLikeALaptop: true };

function send(event: string, payload: unknown) {
  act(() => {
    tauri.handlers.get(event)?.({ payload });
  });
}

async function start() {
  const ui: ReactNode = <MoveHost old={false} rescue={drive} onPairAgain={vi.fn()} onHome={vi.fn()} />;
  render(
    <LocalizationProvider l10n={l10n}>
      <main>{ui}</main>
    </LocalizationProvider>,
  );
  await waitFor(() => {
    expect(tauri.invoke).toHaveBeenCalledWith("rescue_begin", expect.anything());
  });
}

const h1 = () => screen.getByRole("heading", { level: 1 }).textContent;

describe("a rescue's move pages", () => {
  it("starts the rescue for the chosen drive, never an ordinary move", async () => {
    await start();
    const args = tauri.invoke.mock.calls.find(([c]) => c === "rescue_begin")?.[1] as { id: string; fallback: Record<string, string> };
    expect(args.id).toBe(drive.id);
    expect(Object.keys(args.fallback).length).toBeGreaterThan(0);
    expect(tauri.invoke).not.toHaveBeenCalledWith("move_begin", expect.anything());
    // Waiting: the drive is being read; nothing on it changes.
    expect(h1()).toBe(text("rescue-wait-title", { drive: drive.name }));
    expect(screen.getByText(text("rescue-safe"))).toBeTruthy();
    expect(await violations()).toEqual([]);
  });

  it("says files were found on the drive, not that someone wants to send them", async () => {
    await start();
    send("move-plan", plans[0]);
    expect(h1()).toBe(text("rescue-accept-title", { drive: drive.name }));
    expect(screen.queryByText(text("accept-body"))).toBeNull();
    expect(screen.getByText(text("rescue-accept-body"))).toBeTruthy();
  });

  it("copies from the drive, and its results name the drive", async () => {
    await start();
    const running = Object.fromEntries(Object.entries(progresses[0]).filter(([key]) => key !== "notice"));
    send("move-progress", running);
    expect(h1()).toBe(text("rescue-progress-title", { drive: drive.name }));
    send("move-finished", outcomes[0]);
    const title = h1();
    // A stopped or broken-off outcome keeps its own words; an ordinary end names the drive.
    expect([text("rescue-outcome-title", { drive: drive.name }), text("outcome-stopped-title"), text("outcome-broke-title")]).toContain(title);
  });

  it("is accessible right to left", async () => {
    document.documentElement.dir = "rtl";
    await start();
    send("move-plan", plans[0]);
    expect(await violations()).toEqual([]);
  });
});
