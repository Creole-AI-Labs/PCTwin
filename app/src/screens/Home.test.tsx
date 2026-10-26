import { LocalizationProvider } from "@fluent/react";
import type { ReactNode } from "react";
import { cleanup, fireEvent, render, screen, within } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { localization, type LanguageCode } from "../i18n";
import { violations } from "../testing/a11y";
import Welcome from "./Welcome";

/*
 * F01 Home, as approved in the Home and Quick Send mockup (revision 3): the headline with the small
 * two-laptop picture beside it, four choice cards (Quick Send first and filled), then My history.
 * Its four states: Returning, First use, Interrupted move and Waiting files.
 */

const en = localization("en");
const text = (id: string, vars?: Record<string, string | number>) => en.getString(id, vars);
/** Fluent isolates variables for right-to-left safety; the marks are invisible, so tests compare without them. */
const plain = (s: string | null | undefined) => (s ?? "").replace(/[⁦-⁩]/g, "");
const PAID = /sign[ -]?in|log[ -]?in|account|cloud|plans|price|billing|payment|paid|upgrade|\bpro\b|premium|trial|subscri|[$€£₦]/i;
const nothing = () => undefined;

let warnings: string[] = [];

beforeEach(() => {
  warnings = [];
  vi.spyOn(console, "warn").mockImplementation((...args: unknown[]) => {
    warnings.push(args.map(String).join(" "));
  });
});

afterEach(() => {
  cleanup();
  vi.restoreAllMocks();
});

function inApp(ui: ReactNode, language: LanguageCode = "en") {
  return render(
    <LocalizationProvider l10n={localization(language)}>
      <main dir={language === "ar" ? "rtl" : "ltr"}>{ui}</main>
    </LocalizationProvider>,
  );
}

function home(props: Partial<Parameters<typeof Welcome>[0]> = {}, language: LanguageCode = "en") {
  const calls = { start: vi.fn(), soon: vi.fn(), quick: vi.fn(), history: vi.fn(), resume: vi.fn(), waiting: vi.fn() };
  inApp(
    <Welcome
      onStart={calls.start}
      onSoon={calls.soon}
      onQuick={calls.quick}
      onHistory={calls.history}
      {...props}
    />,
    language,
  );
  return calls;
}

const cards = () => [...document.querySelectorAll(".home-grid > button")];
const noStringsMissing = () => {
  expect(warnings.filter((w) => /missing|not found|did not match|unknown/i.test(w))).toEqual([]);
};

describe("F01 Home: Returning and First use", () => {
  it("shows the headline, the small picture beside it, and the line under it", () => {
    home();
    expect(screen.getByRole("heading", { level: 1 }).textContent).toBe("Send files,or move everything.");
    const art = document.querySelector(".home-headline .home-art");
    expect(art).toBeTruthy();
    expect(art?.getAttribute("aria-hidden")).toBe("true");
    expect(screen.getByText(text("welcome-body"))).toBeTruthy();
    noStringsMissing();
  });

  it("offers four cards in the mockup's order, each a title and one plain line, Quick Send filled", () => {
    home();
    expect(cards().map((c) => [c.querySelector("strong")?.textContent, c.querySelector("small")?.textContent])).toEqual([
      ["Quick Send", "Send or receive files"],
      ["Start a full move", "Move your setup"],
      ["Restore setup", "Bring it back"],
      ["Rescue a laptop", "Save what matters"],
    ]);
    expect(cards()[0].classList.contains("filled")).toBe(true);
    expect(cards().slice(1).some((c) => c.classList.contains("filled"))).toBe(false);
    noStringsMissing();
  });

  it("keeps the motion hooks: every card is a bookmark with its icon job", () => {
    home();
    expect(cards().map((c) => c.getAttribute("data-icon-job"))).toEqual(["send", "move", "restore", "rescue"]);
    for (const c of cards()) {
      expect(c.hasAttribute("data-bookmark")).toBe(true);
      expect(c.querySelector("svg")).toBeTruthy();
    }
    expect(screen.getByRole("button", { name: /My history/ }).hasAttribute("data-bookmark")).toBe(true);
  });

  it("each card and the history row does its own job", () => {
    const calls = home();
    fireEvent.click(screen.getByRole("button", { name: /Quick Send/ }));
    expect(calls.quick).toHaveBeenCalledTimes(1);
    fireEvent.click(screen.getByRole("button", { name: /Start a full move/ }));
    expect(calls.start).toHaveBeenCalledTimes(1);
    fireEvent.click(screen.getByRole("button", { name: /Restore setup/ }));
    expect(calls.soon).toHaveBeenLastCalledWith("restore");
    fireEvent.click(screen.getByRole("button", { name: /Rescue a laptop/ }));
    expect(calls.soon).toHaveBeenLastCalledWith("rescue");
    const history = screen.getByRole("button", { name: /My history/ });
    expect(history.textContent).toContain("Your moves and their reports");
    fireEvent.click(history);
    expect(calls.history).toHaveBeenCalledTimes(1);
  });

  it("first use and returning are the same calm page: no resume banner, nothing waiting", () => {
    home();
    expect(screen.queryByRole("button", { name: text("resume-open") })).toBeNull();
    expect(document.querySelector(".home-resume")).toBeNull();
    expect(document.querySelector(".home-waiting")).toBeNull();
    expect(screen.getAllByRole("button").map((b) => b.querySelector("strong")?.textContent)).toEqual([
      "Quick Send",
      "Start a full move",
      "Restore setup",
      "Rescue a laptop",
      "My history",
    ]);
  });

  it("shows only free choices and has no accessibility violations", async () => {
    home();
    expect(screen.getByRole("main").textContent).not.toMatch(PAID);
    expect(await violations()).toEqual([]);
  });
});

describe("F01 Home: Interrupted move", () => {
  it("puts Resume your move at the top, says progress is saved, and reopens the saved move", () => {
    const calls = home({ unfinished: true, onResume: vi.fn() });
    const banner = document.querySelector(".home-resume");
    expect(banner).toBeTruthy();
    expect(banner?.textContent).toContain(text("resume-open"));
    expect(banner?.textContent).toContain(text("home-resume-body"));
    // Before the headline, as in the mockup.
    const heading = screen.getByRole("heading", { level: 1 });
    expect(banner && banner.compareDocumentPosition(heading) & Node.DOCUMENT_POSITION_FOLLOWING).toBeTruthy();
    const button = screen.getByRole("button", { name: text("resume-open") });
    expect(button.textContent).toBe(text("home-resume-action"));
    expect(button.hasAttribute("data-bookmark")).toBe(true);
    expect(calls.start).not.toHaveBeenCalled();
    noStringsMissing();
  });

  it("calls only Resume when Resume is chosen; the four cards stay", () => {
    const onResume = vi.fn();
    const calls = home({ unfinished: true, onResume });
    fireEvent.click(screen.getByRole("button", { name: text("resume-open") }));
    expect(onResume).toHaveBeenCalledTimes(1);
    expect(calls.start).not.toHaveBeenCalled();
    expect(cards()).toHaveLength(4);
  });

  it("uses the smaller headline and leaves the picture out so the cards stay in view", () => {
    home({ unfinished: true, onResume: nothing });
    expect(document.querySelector(".home-headline")?.classList.contains("compact")).toBe(true);
    expect(document.querySelector(".home-art")).toBeNull();
  });

  it("states no percentage it cannot measure, and has no accessibility violations", async () => {
    home({ unfinished: true, onResume: nothing });
    expect(document.querySelector(".home-resume")?.textContent).not.toMatch(/%|\d/);
    expect(await violations()).toEqual([]);
  });

  it("without a way to resume, shows no banner", () => {
    home({ unfinished: true });
    expect(document.querySelector(".home-resume")).toBeNull();
  });
});

describe("F01 Home: Waiting files", () => {
  it("adds a quiet line under My history that names how many files wait and for which laptop", () => {
    const onWaiting = vi.fn();
    home({ waiting: { count: 2, to: "Work laptop" }, onWaiting });
    const line = screen.getByRole("button", { name: (n) => plain(n) === "2 files waiting for Work laptop" });
    const history = screen.getByRole("button", { name: /My history/ });
    expect(history.compareDocumentPosition(line) & Node.DOCUMENT_POSITION_FOLLOWING).toBeTruthy();
    fireEvent.click(line);
    expect(onWaiting).toHaveBeenCalledTimes(1);
    noStringsMissing();
  });

  it("says one file in the singular", () => {
    home({ waiting: { count: 1, to: "Work laptop" }, onWaiting: nothing });
    expect(plain(document.querySelector(".home-waiting")?.textContent)).toBe("1 file waiting for Work laptop");
  });

  it("shows nothing when no files wait", () => {
    home({ waiting: { count: 0, to: "Work laptop" }, onWaiting: nothing });
    expect(document.querySelector(".home-waiting")).toBeNull();
  });

  it("has no accessibility violations", async () => {
    home({ waiting: { count: 2, to: "Work laptop" }, onWaiting: nothing });
    expect(await violations()).toEqual([]);
  });
});

describe("F01 Home in every language", () => {
  it.each(["en", "fr", "es", "pt", "ar", "hi", "zh-CN"] as const)("%s: every Home string is there in its own words", (code) => {
    home({ unfinished: true, onResume: nothing, waiting: { count: 2, to: "Work laptop" }, onWaiting: nothing }, code);
    noStringsMissing();
    const l10n = localization(code);
    for (const id of ["home-quick-send-body", "home-move-body", "home-restore-body", "home-rescue-body", "home-history", "home-history-body", "home-resume-body", "home-resume-action"]) {
      const value = l10n.getString(id);
      expect(value).not.toBe(id);
      if (code !== "en") expect(value).not.toBe(en.getString(id));
    }
  });

  it("Arabic keeps the order and reads right to left; the chevron points along the reading direction", () => {
    home({}, "ar");
    const ar = localization("ar");
    expect(within(screen.getByRole("main")).getByRole("heading", { level: 1 }).textContent).toBe(`${ar.getString("welcome-title-line1")}${ar.getString("welcome-title-line2")}`);
    expect(cards()[0].querySelector("strong")?.textContent).toBe(ar.getString("welcome-quick-send"));
    expect(document.querySelector(".home-history .chevron")?.classList.contains("flip")).toBe(true);
    expect(document.querySelector(".home-art svg")?.classList.contains("flip")).toBe(true);
  });
});
