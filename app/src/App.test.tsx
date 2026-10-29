import { act, cleanup, fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { violations } from "./testing/a11y";
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

describe("screens", () => {
  it("Welcome shows the free move first, in English, with nothing missing", () => {
    render(<App initialLanguage="en" />);
    expect(screen.getByRole("heading", { level: 1 }).textContent).toBe("Send files,or move everything.");
    expect(screen.getByRole("button", { name: /Start a full move/ })).toBeTruthy();
    expect(screen.getByText("Free. No account needed.")).toBeTruthy();
    expect(warnings.filter((w) => /missing|not found/i.test(w))).toEqual([]);
  });

  it("goes Welcome, Which laptop is this?, How will you connect?, Put both laptops together, then pairing, and back one step at a time", async () => {
    const l10n = localization("en");
    const title = () => screen.getByRole("heading", { level: 1 }).textContent;
    render(<App initialLanguage="en" />);
    fireEvent.click(screen.getByRole("button", { name: /Start a full move/ }));
    expect(title()).toBe("Which laptop is this?");
    fireEvent.click(screen.getByRole("button", { name: /This is my old laptop/ }));
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
    fireEvent.click(screen.getByRole("button", { name: /This is my new laptop/ }));
    fireEvent.click(screen.getByRole("button", { name: l10n.getString("connect-use-wifi") }));
    fireEvent.click(screen.getByRole("button", { name: l10n.getString("connect-prep-continue") }));
    await act(async () => {});
    // The shell here gives no search reply, so the search fails, and the page says so.
    expect(title()).toBe(l10n.getString("pair-new-search-failed-title"));
    fireEvent.click(screen.getByRole("button", { name: l10n.getString("pair-back") }));
    fireEvent.click(screen.getByRole("button", { name: l10n.getString("pair-back") }));
    fireEvent.click(screen.getByRole("button", { name: l10n.getString("pair-back") }));
    fireEvent.click(screen.getByRole("button", { name: /Back/ }));
    expect(title()).toBe("Send files,or move everything.");
    expect(warnings.filter((w) => /missing|not found|did not match/i.test(w))).toEqual([]);
  });

  it("the footer shows the short note and the screen action on the connection steps, and the usual footer elsewhere", () => {
    const l10n = localization("en");
    render(<App initialLanguage="en" />);
    const footer = () => within(screen.getByRole("contentinfo"));
    expect(footer().getByText(l10n.getString("footer-free"))).toBeTruthy();
    expect(footer().getByRole("button", { name: l10n.getString("footer-help") })).toBeTruthy();
    fireEvent.click(screen.getByRole("button", { name: /Start a full move/ }));
    expect(footer().getByText(l10n.getString("footer-free"))).toBeTruthy();
    fireEvent.click(screen.getByRole("button", { name: /This is my old laptop/ }));
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

  it("Rescue a laptop opens the rescue page, which lists the drives connected here, and Back returns", async () => {
    tauri.invoke.mockImplementation((command: string) => Promise.resolve(command === "rescue_drives" ? [] : undefined));
    render(<App initialLanguage="en" />);
    fireEvent.click(screen.getByRole("button", { name: /Rescue a laptop/ }));
    expect(screen.queryByText("Coming soon")).toBeNull();
    expect(screen.getByRole("heading", { level: 1 }).textContent).toBe("Rescue a laptop");
    await waitFor(() => {
      expect(tauri.invoke.mock.calls.some(([c]) => c === "rescue_drives")).toBe(true);
    });
    fireEvent.click(screen.getByRole("button", { name: /Back/ }));
    expect(screen.getByRole("heading", { level: 1 }).textContent).toBe("Send files,or move everything.");
  });

  it.each([["Restore setup", "Restore setup"]])("%s opens a coming-soon page that changes nothing, and Back returns", async (button, heading) => {
    render(<App initialLanguage="en" />);
    fireEvent.click(screen.getByRole("button", { name: new RegExp(button) }));
    expect(screen.getByText("Coming soon")).toBeTruthy();
    expect(screen.getByRole("heading", { level: 1 }).textContent).toBe(heading);
    expect(screen.getByText(/later version|first release/)).toBeTruthy();
    expect(await violations()).toEqual([]);
    fireEvent.click(screen.getByRole("button", { name: /Back/ }));
    expect(screen.getByRole("heading", { level: 1 }).textContent).toBe("Send files,or move everything.");
    expect(warnings.filter((w) => /missing|not found/i.test(w))).toEqual([]);
  });

  it("a move that stopped part way shows Resume your move on Home; Resume opens the pick-up: tidy up, then pair again as the new laptop", async () => {
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
    const resume = await screen.findByRole("button", { name: l10n.getString("resume-open") });
    // Home stays in front, with the four cards; the pick-up opens only when asked.
    expect(screen.getByRole("heading", { level: 1 }).textContent).toBe("Send files,or move everything.");
    expect(screen.getByRole("button", { name: /Quick Send/ })).toBeTruthy();
    fireEvent.click(resume);
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
    expect(screen.getByRole("heading", { level: 1 }).textContent).toBe("Send files,or move everything.");
    expect(warnings.filter((w) => /missing|not found/i.test(w))).toEqual([]);
  });

  it("Quick Send's Send opens the file choice (nothing is picked by itself), and Back returns to its home", () => {
    const l10n = localization("en");
    render(<App initialLanguage="en" />);
    fireEvent.click(screen.getByRole("button", { name: /Quick Send/ }));
    fireEvent.click(screen.getByRole("button", { name: new RegExp(l10n.getString("qs-send-title")) }));
    expect(screen.getByRole("heading", { level: 1 }).textContent).toBe(l10n.getString("qs-choose-title"));
    expect(tauri.invoke.mock.calls.map(([c]) => c as string).filter((c) => c.startsWith("quicksend_pick"))).toEqual([]);
    fireEvent.click(screen.getByRole("button", { name: l10n.getString("frame-back") }));
    expect(screen.getByRole("heading", { level: 1 }).textContent).toBe(l10n.getString("qs-home-title"));
  });

  it("a person whose things were handed over at their sign-in sees them counted once, then Welcome", async () => {
    const l10n = localization("en");
    tauri.invoke.mockImplementation((command: string) =>
      Promise.resolve(command === "things_ready" ? { landed: 120, kept: 2, failed: 0 } : command === "recovery_check" ? { unfinished: 0 } : undefined),
    );
    render(<App initialLanguage="en" />);
    expect(await screen.findByText(l10n.getString("ready-title"))).toBeTruthy();
    expect(screen.getByText("120")).toBeTruthy();
    fireEvent.click(screen.getByRole("button", { name: l10n.getString("ready-done") }));
    expect(screen.getByRole("heading", { level: 1 }).textContent).toBe("Send files,or move everything.");
  });

  it("Quick Send home offers no history: what was sent or received is not remembered", () => {
    render(<App initialLanguage="en" />);
    fireEvent.click(screen.getByRole("button", { name: /Quick Send/ }));
    expect(screen.queryByRole("button", { name: /history|sent and received/i })).toBeNull();
  });

  it("My history on Home opens the history, and Back returns to Home", async () => {
    const l10n = localization("en");
    tauri.invoke.mockImplementation((command: string) => Promise.resolve(command === "history_list" ? [] : command === "recovery_check" ? { unfinished: 0 } : undefined));
    render(<App initialLanguage="en" />);
    fireEvent.click(screen.getByRole("button", { name: /My history/ }));
    expect(screen.getByRole("heading", { level: 1 }).textContent).toBe(l10n.getString("history-title"));
    await act(async () => {});
    fireEvent.click(screen.getByRole("button", { name: l10n.getString("frame-back") }));
    expect(screen.getByRole("heading", { level: 1 }).textContent).toBe("Send files,or move everything.");
  });

  it("the header's language control opens the languages straight away; choosing one applies it and closes the list", () => {
    render(<App initialLanguage="en" />);
    const header = within(screen.getByRole("banner"));
    expect(header.getByText("Your laptop")).toBeTruthy();
    const language = header.getByRole("button", { name: "Language" });
    expect(language.textContent).toBe("EN");
    expect(language.getAttribute("aria-expanded")).toBe("false");
    fireEvent.click(language);
    expect(language.getAttribute("aria-expanded")).toBe("true");
    const list = within(screen.getByRole("list", { name: "Language" }));
    // Every language at once, with no motion choice mixed in; the current one is marked.
    expect(list.getAllByRole("button").length).toBe(7);
    expect(list.getByRole("button", { name: "English" }).getAttribute("aria-pressed")).toBe("true");
    expect(screen.queryByText(/Motion:/)).toBeNull();
    act(() => {
      fireEvent.click(list.getByRole("button", { name: "Français" }));
    });
    expect(screen.queryByRole("list", { name: /Lang/ })).toBeNull();
    expect(header.getByRole("button", { name: "Langue" }).textContent).toBe("FR");
  });

  it("the motion icon sits where Help was and opens the three motion choices; Escape closes it", () => {
    render(<App initialLanguage="en" />);
    const header = within(screen.getByRole("banner"));
    expect(header.queryByRole("button", { name: "Help" })).toBeNull();
    const motion = header.getByRole("button", { name: "Motion" });
    expect(motion.textContent).toBe("");
    expect(motion.querySelector("svg")).toBeTruthy();
    fireEvent.click(motion);
    const list = within(screen.getByRole("list", { name: "Motion" }));
    expect(list.getAllByRole("button").map((b) => b.textContent)).toEqual(["Motion: follow system", "Motion: reduced", "Motion: off"]);
    expect(list.getByRole("button", { name: "Motion: follow system" }).getAttribute("aria-pressed")).toBe("true");
    fireEvent.click(list.getByRole("button", { name: "Motion: off" }));
    expect(screen.queryByRole("list", { name: "Motion" })).toBeNull();
    expect(document.activeElement).toBe(motion);
    fireEvent.click(motion);
    expect(screen.getByRole("button", { name: "Motion: off" }).getAttribute("aria-pressed")).toBe("true");
    fireEvent.keyDown(screen.getByRole("button", { name: "Motion: off" }), { key: "Escape" });
    expect(screen.queryByRole("list", { name: "Motion" })).toBeNull();
    expect(document.activeElement).toBe(motion);
  });

  it("with no name chosen yet, the app opens on Name this laptop; keeping the suggestion goes Home", async () => {
    const l10n = localization("en");
    tauri.invoke.mockImplementation((command: string) =>
      Promise.resolve(
        command === "laptop_name"
          ? { kept: null, suggested: { kind: "named", name: "Ezzy Laptop" } }
          : command === "laptop_name_shuffle"
            ? { kind: "picked", colour: "blue", animal: "fox" }
            : command === "laptop_name_use"
              ? { kind: "picked", colour: "blue", animal: "fox" }
              : command === "recovery_check"
                ? { unfinished: 0 }
                : undefined,
      ),
    );
    render(<App initialLanguage="en" />);
    await act(async () => {});
    expect(screen.getByRole("heading", { level: 1 }).textContent).toBe(l10n.getString("name-title"));
    expect(screen.getByText("Ezzy Laptop")).toBeTruthy();
    // Anyone on the same Wi-Fi can see it, and the page says so.
    expect(screen.getByText(l10n.getString("pair-old-rename-note"))).toBeTruthy();
    expect(await violations()).toEqual([]);
    fireEvent.click(screen.getByRole("button", { name: l10n.getString("pair-old-shuffle") }));
    await act(async () => {});
    expect(screen.queryByText("Ezzy Laptop")).toBeNull();
    fireEvent.click(screen.getByRole("button", { name: l10n.getString("name-use") }));
    await act(async () => {});
    expect(tauri.invoke.mock.calls.map(([c]) => c as string)).toContain("laptop_name_use");
    expect(screen.getByRole("heading", { level: 1 }).textContent).toBe("Send files,or move everything.");
  });

  it("a typed name that cannot be used is refused in plain words; a good one is kept and goes Home", async () => {
    const l10n = localization("en");
    tauri.invoke.mockImplementation((command: string, args?: { name?: string }) =>
      command === "laptop_name"
        ? Promise.resolve({ kept: null, suggested: { kind: "picked", colour: "green", animal: "owl" } })
        : command === "laptop_name_type"
          ? args?.name === "Ezzy Laptop"
            ? Promise.resolve({ kind: "named", name: "Ezzy Laptop" })
            : // The engine refuses with its problem as plain data, as Tauri delivers it, not an Error.
              // eslint-disable-next-line @typescript-eslint/prefer-promise-reject-errors
              Promise.reject({ key: "pairing-name-invalid", networks: [] })
          : Promise.resolve(command === "recovery_check" ? { unfinished: 0 } : undefined),
    );
    render(<App initialLanguage="en" />);
    await act(async () => {});
    fireEvent.click(screen.getByRole("button", { name: l10n.getString("pair-old-rename") }));
    const field = screen.getByRole("textbox", { name: l10n.getString("pair-old-rename-label") });
    fireEvent.change(field, { target: { value: "‮bad" } });
    fireEvent.click(screen.getByRole("button", { name: l10n.getString("pair-old-rename-save") }));
    await act(async () => {});
    expect(screen.getByText(l10n.getString("pairing-name-invalid"))).toBeTruthy();
    expect(screen.getByRole("heading", { level: 1 }).textContent).toBe(l10n.getString("name-title"));
    fireEvent.change(field, { target: { value: "Ezzy Laptop" } });
    fireEvent.click(screen.getByRole("button", { name: l10n.getString("pair-old-rename-save") }));
    await act(async () => {});
    expect(screen.getByRole("heading", { level: 1 }).textContent).toBe("Send files,or move everything.");
  });

  it("a laptop that has a name opens on Home, with no naming page", async () => {
    tauri.invoke.mockImplementation((command: string) =>
      Promise.resolve(
        command === "laptop_name"
          ? { kept: { kind: "named", name: "Ezzy Laptop" }, suggested: { kind: "named", name: "Ezzy Laptop" } }
          : command === "recovery_check"
            ? { unfinished: 0 }
            : undefined,
      ),
    );
    render(<App initialLanguage="en" />);
    await act(async () => {});
    expect(screen.getByRole("heading", { level: 1 }).textContent).toBe("Send files,or move everything.");
  });

  it("with nothing to pick up, the app opens on Welcome", async () => {
    tauri.invoke.mockImplementation((command: string) => Promise.resolve(command === "recovery_check" ? { unfinished: 0 } : undefined));
    render(<App initialLanguage="en" />);
    await act(async () => {});
    expect(screen.getByRole("heading", { level: 1 }).textContent).toBe("Send files,or move everything.");
  });

  it("Quick Send opens its home, says plainly that nothing is connected yet, and Back returns", () => {
    const l10n = localization("en");
    render(<App initialLanguage="en" />);
    fireEvent.click(screen.getByRole("button", { name: /Quick Send/ }));
    expect(screen.getByRole("heading", { level: 1 }).textContent).toBe(l10n.getString("qs-home-title"));
    expect(screen.queryByText("Coming soon")).toBeNull();
    // Nothing reaches the shell but the start-up looks (a move that stopped part way, this
    // laptop's name, a newer version, a hand-over note).
    expect(tauri.invoke.mock.calls.map(([c]) => c as string)).toEqual(["recovery_check", "laptop_name", "update_check", "things_ready"]);
    // The window footer carries the free-sending note and the main action, also while the notice is showing.
    const footer = within(screen.getByRole("contentinfo"));
    expect(footer.getByText(l10n.getString("footer-note-send"))).toBeTruthy();
    expect(footer.getByRole("button", { name: l10n.getString("qs-home-action") })).toBeTruthy();
    expect(footer.queryByText(l10n.getString("footer-free"))).toBeNull();
    fireEvent.click(screen.getByRole("button", { name: l10n.getString("frame-back") }));
    expect(within(screen.getByRole("contentinfo")).getByText(l10n.getString("footer-free"))).toBeTruthy();
    expect(screen.getByRole("heading", { level: 1 }).textContent).toBe("Send files,or move everything.");
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
    expect(within(screen.getByRole("main")).getAllByRole("button").map((b) => b.querySelector("strong")?.textContent)).toEqual(["Quick Send", "Start a full move", "Restore setup", "Rescue a laptop", "My history"]);
    expect(within(screen.getByRole("banner")).getAllByRole("button").map((b) => b.getAttribute("aria-label") ?? b.textContent)).toEqual(["PCTwin home", "Language", "Motion"]);
    expect(within(screen.getByRole("contentinfo")).getAllByRole("button").map((b) => b.textContent)).toEqual(["Help"]);
  });

  it("Start a full move opens the move in every build, with no coming-soon page", () => {
    vi.stubEnv("VITE_PCTWIN_FULL_MOVE", "");
    try {
      render(<App initialLanguage="en" />);
      fireEvent.click(screen.getByRole("button", { name: /Start a full move/ }));
      expect(screen.getByRole("heading", { level: 1 }).textContent).toBe("Which laptop is this?");
      expect(screen.queryByText(/use Quick Send to send files/)).toBeNull();
    } finally {
      vi.unstubAllEnvs();
    }
  });

  it("a new screen takes keyboard focus", () => {
    render(<App initialLanguage="en" />);
    fireEvent.click(screen.getByRole("button", { name: /Start a full move/ }));
    expect(document.activeElement?.tagName).toBe("MAIN");
  });

  it.each(["welcome", "role", "connect", "together", "pair"])("the %s screen has no accessibility violations", async (which) => {
    render(<App initialLanguage="en" />);
    const steps = ["welcome", "role", "connect", "together", "pair"];
    const at = steps.indexOf(which);
    if (at >= 1) fireEvent.click(screen.getByRole("button", { name: /Start a full move/ }));
    if (at >= 2) fireEvent.click(screen.getByRole("button", { name: /This is my new laptop/ }));
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
      if (code === "en") expect(start).toBe("Start a full move");
      else expect(start).not.toBe("Start a full move");
      // An id no language has is never shown as blank: the id itself comes back.
      expect(l10n.getString("no-such-message-anywhere", undefined, "fallback text")).toBe("fallback text");
    }
  });

  it("Arabic lays the page out right to left; the choice is remembered on this laptop", () => {
    render(<App initialLanguage="en" />);
    fireEvent.click(screen.getByRole("button", { name: "Language" }));
    act(() => {
      fireEvent.click(screen.getByRole("button", { name: "العربية" }));
    });
    expect(document.documentElement.dir).toBe("rtl");
    expect(document.documentElement.lang).toBe("ar");
    expect(localStorage.getItem("pctwin.language")).toBe("ar");
    expect(directionOf("en")).toBe("ltr");
  });
});
