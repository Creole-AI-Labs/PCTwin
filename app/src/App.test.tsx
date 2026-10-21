import { act, cleanup, fireEvent, render, screen, within } from "@testing-library/react";
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

  it("goes Welcome, Which laptop is this?, How will you connect?, Put both laptops together, then pairing, and back one step at a time", async () => {
    const l10n = localization("en");
    const title = () => screen.getByRole("heading", { level: 1 }).textContent;
    render(<App initialLanguage="en" />);
    fireEvent.click(screen.getByRole("button", { name: /Start a move/ }));
    expect(title()).toBe("Which laptop is this?");
    fireEvent.click(screen.getByRole("button", { name: /My old laptop/ }));
    expect(title()).toBe(l10n.getString("connect-title"));
    fireEvent.click(screen.getByRole("button", { name: l10n.getString("connect-use-wifi") }));
    expect(title()).toBe(l10n.getString("connect-prep-title"));
    fireEvent.click(screen.getByRole("button", { name: l10n.getString("connect-prep-continue") }));
    await act(async () => {});
    expect(title()).toBe(l10n.getString("pair-old-title"));
    // Back from pairing stops it and returns to the preparation step; Back again and again walks up the path.
    fireEvent.click(screen.getByRole("button", { name: l10n.getString("pair-back") }));
    expect(tauri.invoke).toHaveBeenCalledWith("pairing_stop");
    expect(title()).toBe(l10n.getString("connect-prep-title"));
    fireEvent.click(screen.getByRole("button", { name: l10n.getString("pair-back") }));
    expect(title()).toBe(l10n.getString("connect-title"));
    fireEvent.click(screen.getByRole("button", { name: l10n.getString("pair-back") }));
    expect(title()).toBe("Which laptop is this?");
    // The role chosen is kept through the new steps: the new laptop reaches its own pairing screen.
    fireEvent.click(screen.getByRole("button", { name: /My new laptop/ }));
    fireEvent.click(screen.getByRole("button", { name: l10n.getString("connect-use-wifi") }));
    fireEvent.click(screen.getByRole("button", { name: l10n.getString("connect-prep-continue") }));
    await act(async () => {});
    // The shell here gives no search reply, so the search fails, and the page says so.
    expect(title()).toBe(l10n.getString("pair-new-search-failed-title"));
    fireEvent.click(screen.getByRole("button", { name: l10n.getString("pair-back") }));
    fireEvent.click(screen.getByRole("button", { name: l10n.getString("pair-back") }));
    fireEvent.click(screen.getByRole("button", { name: l10n.getString("pair-back") }));
    fireEvent.click(screen.getByRole("button", { name: /Back/ }));
    expect(title()).toBe("Your setup.Your next laptop.");
    expect(warnings.filter((w) => /missing|not found|did not match/i.test(w))).toEqual([]);
  });

  it("the footer shows the short note and the screen action on the connection steps, and the usual footer elsewhere", () => {
    const l10n = localization("en");
    render(<App initialLanguage="en" />);
    const footer = () => within(screen.getByRole("contentinfo"));
    expect(footer().getByText(l10n.getString("footer-free"))).toBeTruthy();
    expect(footer().getByRole("button", { name: l10n.getString("footer-help") })).toBeTruthy();
    fireEvent.click(screen.getByRole("button", { name: /Start a move/ }));
    expect(footer().getByText(l10n.getString("footer-free"))).toBeTruthy();
    fireEvent.click(screen.getByRole("button", { name: /My old laptop/ }));
    expect(footer().getByText(l10n.getString("footer-note-move"))).toBeTruthy();
    expect(footer().getByRole("button", { name: l10n.getString("connect-use-wifi") })).toBeTruthy();
    expect(within(screen.getByRole("main")).queryByRole("button", { name: l10n.getString("connect-use-wifi") })).toBeNull();
    expect(footer().queryByText(l10n.getString("footer-free"))).toBeNull();
    fireEvent.click(footer().getByRole("button", { name: l10n.getString("connect-use-wifi") }));
    expect(footer().getByRole("button", { name: l10n.getString("connect-prep-continue") })).toBeTruthy();
    fireEvent.click(screen.getByRole("button", { name: l10n.getString("frame-back") }));
    fireEvent.click(screen.getByRole("button", { name: l10n.getString("frame-back") }));
    fireEvent.click(screen.getByRole("button", { name: /Back/ }));
    expect(footer().getByText(l10n.getString("footer-free"))).toBeTruthy();
    expect(footer().getByRole("button", { name: l10n.getString("footer-help") })).toBeTruthy();
  });

  it.each([
    ["Restore setup", "Restore setup"],
    ["Rescue a laptop", "Rescue a laptop"],
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

  it("a move that stopped part way is picked up first: tidy up, then pair again as the new laptop", async () => {
    const l10n = localization("en");
    tauri.invoke.mockImplementation((command: string) =>
      Promise.resolve(
        command === "recovery_check"
          ? { unfinished: 3 }
          : command === "recovery_run"
            ? { putInPlace: 2, keptToContinue: 1, notFinished: 0, leftForLater: 0 }
            : undefined,
      ),
    );
    render(<App initialLanguage="en" />);
    expect(await screen.findByText(l10n.getString("pickup-title"))).toBeTruthy();
    expect(screen.getByText((t) => t.replace(/[⁦-⁩]/g, "") === l10n.getString("pickup-unfinished-body", { count: 3 }).replace(/[⁦-⁩]/g, ""))).toBeTruthy();
    fireEvent.click(screen.getByRole("button", { name: l10n.getString("pickup-tidy") }));
    expect(await screen.findByText(l10n.getString("pickup-done"))).toBeTruthy();
    expect(tauri.invoke).toHaveBeenCalledWith("recovery_run");
    fireEvent.click(screen.getByRole("button", { name: l10n.getString("ended-pair-again") }));
    await act(async () => {});
    expect(tauri.invoke).toHaveBeenCalledWith("pairing_new_search");
    expect(warnings.filter((w) => /missing|not found/i.test(w))).toEqual([]);
  });

  it("Help in the footer opens the help page, whose guides open in place, and Back returns", () => {
    const l10n = localization("en");
    render(<App initialLanguage="en" />);
    fireEvent.click(within(screen.getByRole("contentinfo")).getByRole("button", { name: l10n.getString("footer-help") }));
    expect(screen.getByRole("heading", { level: 1 }).textContent).toBe(l10n.getString("help-home-title"));
    fireEvent.click(screen.getByText(l10n.getString("help-guide-find-title")));
    expect(screen.getByText(l10n.getString("help-guide-find-body"))).toBeTruthy();
    fireEvent.click(screen.getByRole("button", { name: l10n.getString("frame-back") }));
    expect(screen.getByRole("heading", { level: 1 }).textContent).toBe("Your setup.Your next laptop.");
    expect(warnings.filter((w) => /missing|not found/i.test(w))).toEqual([]);
  });

  it("with nothing to pick up, the app opens on Welcome", async () => {
    tauri.invoke.mockImplementation((command: string) => Promise.resolve(command === "recovery_check" ? { unfinished: 0 } : undefined));
    render(<App initialLanguage="en" />);
    await act(async () => {});
    expect(screen.getByRole("heading", { level: 1 }).textContent).toBe("Your setup.Your next laptop.");
  });

  it("Quick Send opens its home, says plainly that nothing is connected yet, and Back returns", () => {
    const l10n = localization("en");
    render(<App initialLanguage="en" />);
    fireEvent.click(screen.getByRole("button", { name: /Quick Send/ }));
    expect(screen.getByRole("heading", { level: 1 }).textContent).toBe(l10n.getString("qs-home-title"));
    expect(screen.queryByText("Coming soon")).toBeNull();
    // Send, Receive and history are real cards; until the bridge exists each one says so and changes nothing.
    for (const id of ["qs-send-title", "qs-receive-title", "qs-history-title"]) {
      fireEvent.click(screen.getByRole("button", { name: new RegExp(l10n.getString(id)) }));
      expect(screen.getByRole("status").textContent).toContain(l10n.getString("qs-soon-note"));
      expect(screen.getByRole("heading", { level: 1 }).textContent).toBe(l10n.getString("qs-home-title"));
    }
    // Nothing reaches the shell but the start-up look for a move that stopped part way.
    expect(tauri.invoke.mock.calls.map(([c]) => c as string)).toEqual(["recovery_check"]);
    // The window footer carries the free-sending note and the main action, also while the notice is showing.
    const footer = within(screen.getByRole("contentinfo"));
    expect(footer.getByText(l10n.getString("footer-note-send"))).toBeTruthy();
    expect(footer.getByRole("button", { name: l10n.getString("qs-home-action") })).toBeTruthy();
    expect(footer.queryByText(l10n.getString("footer-free"))).toBeNull();
    fireEvent.click(screen.getByRole("button", { name: l10n.getString("frame-back") }));
    expect(within(screen.getByRole("contentinfo")).getByText(l10n.getString("footer-free"))).toBeTruthy();
    expect(screen.getByRole("heading", { level: 1 }).textContent).toBe("Your setup.Your next laptop.");
    expect(warnings.filter((w) => /missing|not found/i.test(w))).toEqual([]);
  });

  it("Welcome shows only free choices: no sign-in, account, plan, price or upgrade", () => {
    render(<App initialLanguage="en" />);
    const paid = /sign[ -]?in|log[ -]?in|account|cloud twin|plan|price|upgrade|\bpro\b|premium|trial|subscri|[$€£₦]/i;
    // The main area has no paid wording at all; the footer may only promise "No account needed".
    expect(screen.getByRole("main").textContent).not.toMatch(paid);
    for (const control of [...screen.getAllByRole("button"), ...screen.queryAllByRole("link")]) {
      expect(control.textContent).not.toMatch(paid);
    }
    expect(screen.getAllByRole("button").map((b) => b.textContent)).toEqual(["PCTwin", "Start a move", "Restore setup", "Rescue a laptop", "Quick Send", "Help"]);
  });

  it("a new screen takes keyboard focus", () => {
    render(<App initialLanguage="en" />);
    fireEvent.click(screen.getByRole("button", { name: /Start a move/ }));
    expect(document.activeElement?.tagName).toBe("MAIN");
  });

  it.each(["welcome", "role", "connect", "together", "pair"])("the %s screen has no accessibility violations", async (which) => {
    render(<App initialLanguage="en" />);
    const steps = ["welcome", "role", "connect", "together", "pair"];
    const at = steps.indexOf(which);
    if (at >= 1) fireEvent.click(screen.getByRole("button", { name: /Start a move/ }));
    if (at >= 2) fireEvent.click(screen.getByRole("button", { name: /My new laptop/ }));
    if (at >= 3) fireEvent.click(screen.getByRole("button", { name: /Use Wi-Fi/ }));
    if (at >= 4) fireEvent.click(screen.getByRole("button", { name: /Continue to pairing/ }));
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

  it("every language file loads and shows its own words; English is the last resort", () => {
    for (const { code } of LANGUAGES) {
      const l10n = localization(code);
      const start = l10n.getString("welcome-start");
      expect(start).not.toBe("welcome-start");
      expect(start).not.toBe("");
      if (code === "en") expect(start).toBe("Start a move");
      else expect(start).not.toBe("Start a move");
      // An id no language has is never shown as blank: the id itself comes back.
      expect(l10n.getString("no-such-message-anywhere", undefined, "fallback text")).toBe("fallback text");
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
