import { LocalizationProvider } from "@fluent/react";
import type { ReactNode } from "react";
import { cleanup, render, screen, waitFor } from "@testing-library/react";
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
  replies = {};
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

  it("the sender picks, offers and checks with the keyboard only", async () => {
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
      expect(heading()).toBe(text("qs-check-title"));
    });
    expect(tauri.invoke).toHaveBeenCalledWith("quicksend_offer", { ids: ["p1"] });
  });

  it("the receiver sees the offer, accepts and chooses a folder with the keyboard only", async () => {
    replies.quicksend_choose_folder = () => ({ path: "C:/Users/Ada/Downloads", freeBytes: 80_000_000_000 });
    const user = userEvent.setup();
    inApp(<QuickHost mode="receive" onHome={vi.fn()} />);
    await waitFor(() => {
      expect(tauri.invoke).toHaveBeenCalledWith("pairing_new_search");
    });
    tauri.handlers.get("quicksend-incoming")?.({ payload: { files: offeredExamples } });
    await waitFor(() => {
      expect(heading()).toBe(text("qs-offer-title"));
    });
    await tabTo(user, "button", named("qs-offer-accept"));
    await user.keyboard("{Enter}");
    expect(heading()).toBe(text("qs-land-title"));
    await tabTo(user, "button", named("qs-land-choose"));
    await user.keyboard("{Enter}");
    await waitFor(() => {
      expect(screen.getByText("C:/Users/Ada/Downloads")).toBeTruthy();
    });
    await tabTo(user, "checkbox", named("qs-land-confirm"));
    await user.keyboard(" ");
    await tabTo(user, "button", named("qs-land-use"));
    await user.keyboard("{Enter}");
    await waitFor(() => {
      expect(tauri.invoke).toHaveBeenCalledWith("quicksend_answer", { answer: { kind: "accept", ids: null } });
    });
  });
});
