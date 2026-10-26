import { LocalizationProvider } from "@fluent/react";
import type { ReactNode } from "react";
import { act, cleanup, render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import offeredExamples from "../contract/qs-offered.json";
import { localization } from "../i18n";
import QuickHost from "./QuickHost";

/*
 * Quick Send acceptance QS08: the whole flow by keyboard alone, in all seven languages. Every step
 * is reached with Tab and taken with Enter or Space, the way a person who cannot use a mouse does;
 * nothing is clicked.
 */

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

const LANGUAGES = ["en", "fr", "es", "pt", "ar", "hi", "zh-CN"] as const;

let replies: Partial<Record<string, (args: unknown) => unknown>> = {};

beforeEach(() => {
  replies = { pairing_heads_up: () => [], pairing_new_search: () => [] };
  tauri.handlers.clear();
  tauri.invoke.mockReset();
  tauri.invoke.mockImplementation((command: string, args?: unknown) => Promise.resolve().then(() => replies[command]?.(args)));
});

afterEach(() => {
  cleanup();
});

/** Presses Tab until the focused control is the one named `name`, as a person would. */
async function tabTo(user: ReturnType<typeof userEvent.setup>, role: "button" | "checkbox", name: RegExp | ((n: string) => boolean)) {
  for (let n = 0; n < 60; n++) {
    const focused = document.activeElement;
    const matches = screen.queryAllByRole(role, { name }).some((el) => el === focused);
    if (matches) return;
    await user.tab();
  }
  throw new Error(`could not reach the ${role} ${String(name)} with the keyboard`);
}

const heading = () => screen.getByRole("heading", { level: 1 }).textContent;

describe.each(LANGUAGES)("Quick Send by keyboard alone (QS08), %s", (lang) => {
  const l10n = localization(lang);
  const text = (id: string) => l10n.getString(id);
  const inApp = (ui: ReactNode) =>
    render(
      <LocalizationProvider l10n={l10n}>
        <main>{ui}</main>
      </LocalizationProvider>,
    );
  // A label may sit inside a longer accessible name (an icon's words, a count).
  const named = (id: string) => (n: string) => n.includes(text(id));

  it("the sender picks and offers with the keyboard only, and lands on pairing in Quick Send's words", async () => {
    replies.quicksend_pick = () => [{ id: "p1", name: "letter.txt", kind: "file", included: true, bytes: 1200, files: 1 }];
    replies.quicksend_offer = () => offeredExamples;
    const user = userEvent.setup();
    inApp(<QuickHost mode="send" onHome={vi.fn()} />);
    expect(heading()).toBe(text("qs-choose-title"));
    await tabTo(user, "button", named("qs-add-files"));
    await user.keyboard("{Enter}");
    await waitFor(() => {
      expect(screen.getByText("letter.txt")).toBeTruthy();
    });
    await tabTo(user, "button", named("qs-choose-continue"));
    await user.keyboard("{Enter}");
    await waitFor(() => {
      expect(tauri.invoke).toHaveBeenCalledWith("pairing_old_start", { network: null });
    });
    expect(tauri.invoke).toHaveBeenCalledWith("quicksend_offer", { ids: ["p1"] });
    act(() => {
      tauri.handlers.get("pairing-old")?.({ payload: { kind: "code", code: "123456", secondsLeft: 50 } });
    });
    const words = document.body.textContent;
    expect(words).toContain(text("qs-pair-old-body"));
    // Never the full move's words for the laptops.
    for (const id of ["pair-old-eyebrow", "pair-old-body", "pair-old-waiting", "pair-old-steps-title"]) expect(words).not.toContain(text(id));
  });

  it("the receiver changes the folder with the keyboard only, on a pairing page in Quick Send's words", async () => {
    replies.quicksend_folder = () => ({ name: "Downloads", path: "C:/Users/Ada/Downloads", freeBytes: 80_000_000_000 });
    replies.quicksend_choose_folder = () => ({ name: "Work", path: "D:/Work", freeBytes: null });
    const user = userEvent.setup();
    inApp(<QuickHost mode="receive" onHome={vi.fn()} />);
    await waitFor(() => {
      expect(tauri.invoke).toHaveBeenCalledWith("pairing_new_search");
    });
    await tabTo(user, "button", named("qs-folder-change"));
    await user.keyboard("{Enter}");
    await waitFor(() => {
      expect(tauri.invoke.mock.calls.some((c: unknown[]) => c[0] === "quicksend_choose_folder")).toBe(true);
    });
    const words = document.body.textContent;
    for (const id of ["pair-new-eyebrow", "pair-new-none-title", "pair-new-none-body", "pair-new-none-same-role", "pair-new-switch"]) expect(words).not.toContain(text(id));
    expect(words).toContain(text("qs-pair-new-none-title"));
  });
});
