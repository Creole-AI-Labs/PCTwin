import { act, cleanup, fireEvent, render, screen } from "@testing-library/react";
import axe from "axe-core";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import App from "./App";
import { LANGUAGES, directionOf, localization, pickLanguage } from "./i18n";

// Pairing talks to the desktop shell; these tests only look at the screens, so it answers with nothing.
const tauri = vi.hoisted(() => ({ invoke: vi.fn(), listen: vi.fn() }));
vi.mock("@tauri-apps/api/core", () => ({ invoke: tauri.invoke }));
vi.mock("@tauri-apps/api/event", () => ({ listen: tauri.listen }));

let warnings: string[] = [];

beforeEach(() => {
  warnings = [];
  vi.spyOn(console, "warn").mockImplementation((...args: unknown[]) => {
    warnings.push(args.map(String).join(" "));
  });
  localStorage.clear();
  tauri.invoke.mockReset();
  tauri.invoke.mockResolvedValue(undefined);
  tauri.listen.mockReset();
  tauri.listen.mockResolvedValue(() => undefined);
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

  it("goes Welcome, then Which laptop is this?, then pairing, and back", async () => {
    const l10n = localization("en");
    render(<App initialLanguage="en" />);
    fireEvent.click(screen.getByRole("button", { name: /Start a move/ }));
    expect(screen.getByRole("heading", { level: 1 }).textContent).toBe("Which laptop is this?");
    fireEvent.click(screen.getByRole("button", { name: /My old laptop/ }));
    await act(async () => {});
    expect(screen.getByRole("heading", { level: 1 }).textContent).toBe(l10n.getString("pair-old-title"));
    fireEvent.click(screen.getByRole("button", { name: l10n.getString("pair-back") }));
    expect(tauri.invoke).toHaveBeenCalledWith("pairing_stop");
    expect(screen.getByRole("heading", { level: 1 }).textContent).toBe("Which laptop is this?");
    fireEvent.click(screen.getByRole("button", { name: /My new laptop/ }));
    await act(async () => {});
    expect(screen.getByRole("heading", { level: 1 }).textContent).toBe(l10n.getString("pair-new-searching-title"));
    fireEvent.click(screen.getByRole("button", { name: l10n.getString("pair-back") }));
    fireEvent.click(screen.getByRole("button", { name: /Back/ }));
    expect(screen.getByRole("heading", { level: 1 }).textContent).toBe("Your setup.Your next laptop.");
    expect(warnings.filter((w) => /missing|not found|did not match/i.test(w))).toEqual([]);
  });

  it.each([
    ["Restore setup", "Restore setup"],
    ["Rescue a laptop", "Rescue a laptop"],
    ["Sign in", "Cloud Twin"],
    ["Quick Send", "Quick Send"],
  ])("%s opens a coming-soon page that changes nothing, and Back returns", async (button, heading) => {
    render(<App initialLanguage="en" />);
    fireEvent.click(screen.getByRole("button", { name: new RegExp(button) }));
    expect(screen.getByText("Coming soon")).toBeTruthy();
    expect(screen.getByRole("heading", { level: 1 }).textContent).toBe(heading);
    expect(screen.getByText(/later version|first release/)).toBeTruthy();
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

  it.each(["welcome", "role", "pair"])("the %s screen has no accessibility violations", async (which) => {
    render(<App initialLanguage="en" />);
    if (which !== "welcome") fireEvent.click(screen.getByRole("button", { name: /Start a move/ }));
    if (which === "pair") fireEvent.click(screen.getByRole("button", { name: /My new laptop/ }));
    await act(async () => {});
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
