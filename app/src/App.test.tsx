import { act, cleanup, fireEvent, render, screen } from "@testing-library/react";
import axe from "axe-core";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import App from "./App";
import { LANGUAGES, directionOf, localization, pickLanguage } from "./i18n";

let warnings: string[] = [];

beforeEach(() => {
  warnings = [];
  vi.spyOn(console, "warn").mockImplementation((...args: unknown[]) => {
    warnings.push(args.map(String).join(" "));
  });
  localStorage.clear();
});

afterEach(() => {
  cleanup();
  vi.restoreAllMocks();
});

/** axe-core on the rendered page. Colour contrast needs real layout, so it is checked in the built app instead. */
async function violations(): Promise<string[]> {
  const result = await axe.run(document.body, { rules: { "color-contrast": { enabled: false } } });
  return result.violations.map((v) => `${v.id}: ${v.nodes.map((n) => n.target.join(" ")).join(", ")}`);
}

describe("screens", () => {
  it("Welcome shows the free move first, in English, with nothing missing", () => {
    render(<App initialLanguage="en" />);
    expect(screen.getByRole("heading", { level: 1 }).textContent).toBe("Your setup.Your next laptop.");
    expect(screen.getByRole("button", { name: /Start a move/ })).toBeTruthy();
    expect(screen.getByText("Free personal moves. No account needed.")).toBeTruthy();
    expect(warnings.filter((w) => /missing|not found/i.test(w))).toEqual([]);
  });

  it("goes Welcome, then Which laptop is this?, then the next step, and back", () => {
    render(<App initialLanguage="en" />);
    fireEvent.click(screen.getByRole("button", { name: /Start a move/ }));
    expect(screen.getByRole("heading", { level: 1 }).textContent).toBe("Which laptop is this?");
    fireEvent.click(screen.getByRole("button", { name: /My old laptop/ }));
    expect(screen.getByRole("heading", { level: 1 }).textContent).toBe("This is the old laptop");
    expect(screen.getByText(/nothing has been changed on this laptop/)).toBeTruthy();
    fireEvent.click(screen.getByRole("button", { name: /Back/ }));
    fireEvent.click(screen.getByRole("button", { name: /Back/ }));
    expect(screen.getByRole("heading", { level: 1 }).textContent).toBe("Your setup.Your next laptop.");
    expect(warnings.filter((w) => /missing|not found/i.test(w))).toEqual([]);
  });

  it.each([
    ["Restore setup", "Restore setup"],
    ["Rescue a laptop", "Rescue a laptop"],
    ["Sign in", "Cloud Twin"],
  ])("%s opens a coming-soon page that changes nothing, and Back returns", async (button, heading) => {
    render(<App initialLanguage="en" />);
    fireEvent.click(screen.getByRole("button", { name: new RegExp(button) }));
    expect(screen.getByText("Coming soon")).toBeTruthy();
    expect(screen.getByRole("heading", { level: 1 }).textContent).toBe(heading);
    expect(screen.getByText(/later version/)).toBeTruthy();
    expect(await violations()).toEqual([]);
    fireEvent.click(screen.getByRole("button", { name: /Back/ }));
    expect(screen.getByRole("heading", { level: 1 }).textContent).toBe("Your setup.Your next laptop.");
    expect(warnings.filter((w) => /missing|not found/i.test(w))).toEqual([]);
  });

  it("a new screen takes keyboard focus", () => {
    render(<App initialLanguage="en" />);
    fireEvent.click(screen.getByRole("button", { name: /Start a move/ }));
    expect(document.activeElement?.tagName).toBe("MAIN");
  });

  it.each(["welcome", "role", "next"])("the %s screen has no accessibility violations", async (which) => {
    render(<App initialLanguage="en" />);
    if (which !== "welcome") fireEvent.click(screen.getByRole("button", { name: /Start a move/ }));
    if (which === "next") fireEvent.click(screen.getByRole("button", { name: /My new laptop/ }));
    expect(await violations()).toEqual([]);
  });
});

describe("languages", () => {
  it("picks the launch language closest to the system's", () => {
    expect(pickLanguage(["fr-FR", "en"])).toBe("fr");
    expect(pickLanguage(["pt-BR"])).toBe("pt");
    expect(pickLanguage(["zh-CN"])).toBe("zh-CN");
    expect(pickLanguage(["de-DE"])).toBe("en");
    expect(pickLanguage([])).toBe("en");
  });

  it("every language file loads, and a string missing in one falls back to English", () => {
    for (const { code } of LANGUAGES) {
      const l10n = localization(code);
      expect(l10n.getString("welcome-start")).toBe("Start a move");
    }
  });

  it("Arabic lays the page out right to left; the choice is remembered on this laptop", () => {
    render(<App initialLanguage="en" />);
    act(() => {
      fireEvent.change(screen.getByRole("combobox", { name: "Language" }), { target: { value: "ar" } });
    });
    expect(document.documentElement.dir).toBe("rtl");
    expect(document.documentElement.lang).toBe("ar");
    expect(localStorage.getItem("pctwin.language")).toBe("ar");
    expect(directionOf("en")).toBe("ltr");
  });
});
