/**
 * The motion rules that hold for every screen (Task List 1.10, motion tests):
 * - motion off and reduced change nothing a person can do;
 * - no screen waits for an animation;
 * - evidence (a failure, a private folder going away) is never delayed by motion.
 * The opening's own rules (first launch and update only, Skip intro, Escape) are in
 * Opening.test.tsx; right to left mirroring is in recipes.test.tsx.
 */
import { LocalizationProvider } from "@fluent/react";
import { cleanup, fireEvent, render, screen, within } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import App from "../App";
import { Banner, Row, RowList } from "../components/kit";
import { localization } from "../i18n";
import { fakeAnimations } from "../testing/animations";
import { APP_VERSION, resetOpening } from "./openingRule";
import { resetMotion, setMotionPreference, type MotionPreference } from "./preference";
import { resetScreenMotion, settleRows, settleScreen } from "./screen";

const tauri = vi.hoisted(() => ({ invoke: vi.fn(), listen: vi.fn() }));
vi.mock("@tauri-apps/api/core", () => ({ invoke: tauri.invoke }));
vi.mock("@tauri-apps/api/event", () => ({ listen: tauri.listen }));

const l10n = localization("en");
let anim: ReturnType<typeof fakeAnimations>;

beforeEach(() => {
  localStorage.clear();
  resetMotion();
  resetScreenMotion();
  tauri.invoke.mockReset();
  tauri.invoke.mockResolvedValue(undefined);
  tauri.listen.mockReset();
  tauri.listen.mockResolvedValue(() => undefined);
  anim = fakeAnimations();
});

afterEach(() => {
  cleanup();
  anim.restore();
  vi.restoreAllMocks();
  localStorage.clear();
  resetMotion();
});

/** Everything a person can read and do on the screen now: headings, controls, their names and states. */
function whatAPersonCanDo(): string[] {
  const main = within(document.querySelector(".app") as HTMLElement);
  return [
    ...main.getAllByRole("heading").map((h) => `heading:${h.textContent}`),
    ...main.getAllByRole("button").map((b) => `button:${b.textContent}:${b.hasAttribute("disabled") || b.getAttribute("aria-disabled") === "true" ? "off" : "on"}`),
    ...main.queryAllByRole("combobox").map((c) => `choice:${c.getAttribute("aria-label") ?? c.closest("label")?.textContent ?? ""}`),
    `focus-inside-main:${String(document.querySelector(".app-main")?.contains(document.activeElement) ?? false)}`,
    `inert:${String(document.querySelector("[inert]") !== null)}`,
  ];
}

/** Home, then three steps into a move and one back, recording what can be done at each. */
function walk(preference: MotionPreference): string[] {
  localStorage.setItem("pctwin.opening.shown", APP_VERSION);
  resetOpening();
  resetMotion();
  setMotionPreference(preference);
  render(<App initialLanguage="en" />);
  const seen: string[] = [];
  const step = (name: RegExp | string) => {
    fireEvent.click(screen.getByRole("button", { name }));
    seen.push(...whatAPersonCanDo());
  };
  seen.push(...whatAPersonCanDo());
  step(/Start a full move/);
  step(/This is my old laptop/);
  step(l10n.getString("connect-use-wifi"));
  step(/Back/);
  cleanup();
  return seen.map((s) => s.replace(/Motion: (follow system|reduced|off)/g, "Motion: <choice>"));
}

describe("motion off and reduced change nothing a person can do", () => {
  it("the same headings, controls, states and focus on every step, whatever the motion setting", () => {
    const full = walk("system");
    anim.made.length = 0;
    const reduced = walk("reduced");
    expect(anim.made.every((a) => !a.properties.includes("transform") && a.delay === 0)).toBe(true);
    anim.made.length = 0;
    const off = walk("off");
    expect(anim.made).toHaveLength(0);
    expect(reduced).toEqual(full);
    expect(off).toEqual(full);
  });
});

describe("no screen waits for an animation", () => {
  it("with every motion still playing (none ever finishes), a person goes straight through the screens", () => {
    localStorage.setItem("pctwin.opening.shown", APP_VERSION);
    resetOpening();
    render(<App initialLanguage="en" />);
    fireEvent.click(screen.getByRole("button", { name: /Start a full move/ }));
    fireEvent.click(screen.getByRole("button", { name: /This is my old laptop/ }));
    fireEvent.click(screen.getByRole("button", { name: l10n.getString("connect-use-wifi") }));
    expect(screen.getByRole("heading", { level: 1 }).textContent).toBe(l10n.getString("connect-prep-title"));
    expect(anim.made.length).toBeGreaterThan(0);
    expect(anim.made.every((a) => !a.done)).toBe(true);
    // Nothing is held back while motion plays: no inert content, no hidden controls.
    expect(document.querySelector("[inert]")).toBeNull();
    // No motion holds a state of its own after it ends (it only ever fills backwards, before it starts).
    expect(anim.made.every((a) => a.options.fill === "backwards")).toBe(true);
  });

  it("the first launch's opening never holds Home back: Home is built under it, and usable the moment it is skipped", () => {
    resetOpening();
    render(<App initialLanguage="en" />);
    expect(screen.getByRole("dialog", { name: l10n.getString("opening-label") })).toBeTruthy();
    expect(document.querySelector(".app-main h1")?.textContent).toBe("Send files,or move everything.");
    fireEvent.keyDown(document.body, { key: "Escape" });
    fireEvent.click(screen.getByRole("button", { name: /Start a full move/ }));
    expect(screen.getByRole("heading", { level: 1 }).textContent).toBe("Which laptop is this?");
  });
});

describe("evidence is never delayed by motion", () => {
  it("a failure banner is there at once, at full strength, and nothing animates it", () => {
    const main = document.createElement("main");
    document.body.append(main);
    render(
      <LocalizationProvider l10n={l10n}>
        <Banner tone="attention" title="Not copied">
          <p>The drive was removed.</p>
        </Banner>
        <RowList>
          <Row title="Photos" />
        </RowList>
      </LocalizationProvider>,
      { container: main },
    );
    settleScreen(main);
    expect(screen.getByRole("alert").textContent).toContain("Not copied");
    const banner = screen.getByRole("alert");
    expect(anim.made.filter((a) => a.target === banner || banner.contains(a.target))).toHaveLength(0);
    expect(anim.on(main).every((a) => a.keyframes.every((k) => !("opacity" in k) || k.opacity === 1))).toBe(true);
    main.remove();
  });

  it("a private folder going away is gone in the same frame, even while its list is still settling", () => {
    const Places = ({ names }: { names: string[] }) => (
      <LocalizationProvider l10n={l10n}>
        <RowList>
          {names.map((n) => (
            <Row key={n} title={n} />
          ))}
        </RowList>
      </LocalizationProvider>
    );
    const { container, rerender } = render(<Places names={["Documents", "Maya's private folder", "Music"]} />);
    settleRows(container);
    expect(anim.running().length).toBe(3);
    const before = anim.made.length;
    rerender(<Places names={["Documents", "Music"]} />);
    expect(screen.queryByText("Maya's private folder")).toBeNull();
    // No way-out animation was made for it.
    expect(anim.made.length).toBe(before);
  });
});
