/**
 * Help, then Feedback (R12), and the Help routes to the update-or-remove page and the safe-stop page:
 * nothing is sent by PCTwin, Copy says "copied" only once the clipboard accepted the text, the shared
 * text holds only the rating and words the person chose, and each page is free of violations.
 */
import { LocalizationProvider } from "@fluent/react";
import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import type { ReactNode } from "react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import safeStops from "../contract/safe-stop.json";
import { localization } from "../i18n";
import { violations } from "../testing/a11y";
import { FooterProvider } from "./FrameFooter";
import Feedback from "./Feedback";
import HelpHome from "./HelpHome";
import Welcome from "./Welcome";

const api = vi.hoisted(() => ({ safeStop: vi.fn(), listenSafeStop: vi.fn(), openInBrowser: vi.fn() }));
vi.mock("../bridge", async (original) => ({ ...(await original<typeof import("../bridge")>()), ...api }));

const l10n = localization("en");
const text = (id: string, vars?: Record<string, string | number>) => l10n.getString(id, vars);
let warnings: string[] = [];

beforeEach(() => {
  warnings = [];
  vi.spyOn(console, "warn").mockImplementation((...args: unknown[]) => {
    warnings.push(args.map(String).join(" "));
  });
  for (const f of Object.values(api)) f.mockReset();
  api.safeStop.mockResolvedValue(safeStops[1]);
  api.listenSafeStop.mockResolvedValue(() => undefined);
  api.openInBrowser.mockResolvedValue(undefined);
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

const clipboard = (writeText: ((t: string) => Promise<void>) | undefined) => {
  Object.defineProperty(navigator, "clipboard", { value: writeText ? { writeText } : undefined, configurable: true });
};

describe("Feedback", () => {
  it("says it is optional, sends nothing by itself and needs no account", async () => {
    show(<Feedback onBack={() => undefined} />);
    expect(screen.getByRole("heading", { level: 1, name: text("feedback-title") })).toBeTruthy();
    expect(screen.getByText(text("feedback-body"))).toBeTruthy();
    expect(screen.getByText(text("feedback-sends-nothing"))).toBeTruthy();
    expect(screen.getByText(text("feedback-preview-empty"))).toBeTruthy();
    expect(screen.getByRole<HTMLButtonElement>("button", { name: text("feedback-copy") }).disabled).toBe(true);
    expect(await violations()).toEqual([]);
    document.documentElement.dir = "rtl";
    expect(await violations()).toEqual([]);
    document.documentElement.dir = "ltr";
  });

  it("previews exactly the rating and the words, and copies exactly that, saying copied only after the clipboard took it", async () => {
    let accept: () => void = () => undefined;
    const writeText = vi.fn(
      () =>
        new Promise<void>((resolve) => {
          accept = resolve;
        }),
    );
    clipboard(writeText);
    show(<Feedback onBack={() => undefined} />);
    fireEvent.click(screen.getByRole("radio", { name: text("feedback-rating-good") }));
    fireEvent.change(screen.getByLabelText(text("feedback-words-label")), { target: { value: "  Clearer steps.  " } });
    const expected = `${text("feedback-preview-rating", { rating: text("feedback-rating-good") })}\nClearer steps.`;
    expect(screen.getByText((_, el) => el?.className === "fb-preview-text" && el.textContent === expected)).toBeTruthy();
    fireEvent.click(screen.getByRole("button", { name: text("feedback-copy") }));
    expect(writeText).toHaveBeenCalledWith(expected);
    // Not claimed while the clipboard has not answered.
    expect(screen.queryByText(text("feedback-copied"))).toBeNull();
    accept();
    expect(await screen.findByText(text("feedback-copied"))).toBeTruthy();
  });

  it("says it could not copy when the clipboard refuses or is missing", async () => {
    clipboard(() => Promise.reject(new Error("no")));
    show(<Feedback onBack={() => undefined} />);
    fireEvent.click(screen.getByRole("radio", { name: text("feedback-rating-needs-work") }));
    fireEvent.click(screen.getByRole("button", { name: text("feedback-copy") }));
    expect(await screen.findByText(text("feedback-copy-failed"))).toBeTruthy();
    expect(screen.queryByText(text("feedback-copied"))).toBeNull();
    clipboard(undefined);
    fireEvent.click(screen.getByRole("button", { name: text("feedback-copy") }));
    expect(await screen.findByText(text("feedback-copy-failed"))).toBeTruthy();
  });

  it("opens only getpctwin.com", async () => {
    show(<Feedback onBack={() => undefined} />);
    fireEvent.click(screen.getByRole("button", { name: text("feedback-open-site") }));
    await waitFor(() => {
      expect(api.openInBrowser).toHaveBeenCalledWith("https://getpctwin.com/");
    });
  });
});

describe("Help home routes", () => {
  it("opens Feedback and returns to the guides with Back", () => {
    show(<HelpHome onBack={() => undefined} />);
    fireEvent.click(screen.getByRole("button", { name: text("help-home-feedback") }));
    expect(screen.getByRole("heading", { level: 1, name: text("feedback-title") })).toBeTruthy();
    fireEvent.click(screen.getByRole("button", { name: text("frame-back") }));
    expect(screen.getByRole("heading", { level: 1, name: text("help-home-title") })).toBeTruthy();
  });

  it("opens the safe-stop page, listing what it waits for, with no violations", async () => {
    show(<HelpHome onBack={() => undefined} />);
    fireEvent.click(screen.getByRole("button", { name: text("help-home-update") }));
    expect(screen.getByRole("heading", { level: 1, name: text("safe-stop-title") })).toBeTruthy();
    expect(await screen.findByText(text("safe-stop-busy-moving"))).toBeTruthy();
    expect(await violations()).toEqual([]);
    fireEvent.click(screen.getByRole("button", { name: text("frame-back") }));
    expect(screen.getByRole("heading", { level: 1, name: text("help-home-title") })).toBeTruthy();
  });
});

describe("Home, an update refused because PCTwin is busy", () => {
  const props = { onStart: vi.fn(), onSoon: vi.fn(), onQuick: vi.fn(), onHistory: vi.fn(), onUpdate: vi.fn() };
  it("offers to see what it is waiting for", () => {
    const onSafeStop = vi.fn();
    show(<Welcome {...props} update={{ version: "0.2.0", state: "busy" }} onSafeStop={onSafeStop} />);
    fireEvent.click(screen.getByRole("button", { name: text("safe-stop-see") }));
    expect(onSafeStop).toHaveBeenCalledOnce();
  });
  it("does not offer it in other update states", () => {
    show(<Welcome {...props} update={{ version: "0.2.0", state: "ready" }} onSafeStop={vi.fn()} />);
    expect(screen.queryByRole("button", { name: text("safe-stop-see") })).toBeNull();
  });
});
