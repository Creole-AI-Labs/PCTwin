import { act, cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import App from "../App";
import { localization } from "../i18n";
import { fakeAnimations } from "../testing/animations";
import { APP_VERSION, kindFor, lastShownVersion, resetOpening } from "./openingRule";
import { resetMotion, setMotionPreference } from "./preference";

const tauri = vi.hoisted(() => ({ invoke: vi.fn(), listen: vi.fn() }));
vi.mock("@tauri-apps/api/core", () => ({ invoke: tauri.invoke }));
vi.mock("@tauri-apps/api/event", () => ({ listen: tauri.listen }));

const l10n = localization("en");
const opening = () => screen.queryByRole("dialog", { name: l10n.getString("opening-label") });
const frame = () => document.querySelector(".app") as HTMLElement;
let anim: ReturnType<typeof fakeAnimations>;

function start(): void {
  resetOpening();
  resetMotion();
  render(<App initialLanguage="en" />);
}

beforeEach(() => {
  localStorage.clear();
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
  vi.unstubAllGlobals();
  localStorage.clear();
  resetMotion();
});

describe("when the opening plays", () => {
  it("plays on the first launch and after an update only; every other start is a short restoration", () => {
    expect(kindFor({ shown: null, version: "1.0.0", mode: "full", animates: true })).toBe("wordplay");
    expect(kindFor({ shown: "0.9.0", version: "1.0.0", mode: "full", animates: true })).toBe("wordplay");
    expect(kindFor({ shown: "1.0.0", version: "1.0.0", mode: "full", animates: true })).toBe("restore");
    expect(kindFor({ shown: null, version: "1.0.0", mode: "reduced", animates: true })).toBe("still");
    expect(kindFor({ shown: null, version: "1.0.0", mode: "off", animates: true })).toBe("none");
    expect(kindFor({ shown: null, version: "1.0.0", mode: "full", animates: false })).toBe("none");
  });

  it("first launch: Wordplay over Home, which is already built underneath; the version is remembered", () => {
    start();
    expect(opening()).toBeTruthy();
    expect(lastShownVersion()).toBe(APP_VERSION);
    // Home is there from the first frame, not after the opening.
    expect(screen.getByRole("button", { name: /Start a full move/, hidden: true })).toBeTruthy();
    expect(frame().hasAttribute("inert")).toBe(true);
    // Six letters, the plain tagline (the home headline), Skip intro with focus.
    expect(document.querySelectorAll(".opening-letter")).toHaveLength(6);
    expect(screen.getByText("Send files, or move everything.")).toBeTruthy();
    expect(document.activeElement).toBe(screen.getByRole("button", { name: "Skip intro" }));
    // 4,800 ms at half speed; the mark docks onto the header's own mark.
    const clock = anim.on(".opening")[0];
    expect(clock.duration).toBe(4800);
    expect(anim.on(".opening-letter")).toHaveLength(6);
    // Transform and opacity only.
    for (const a of anim.made) expect(a.properties.every((p) => p === "transform" || p === "opacity")).toBe(true);
  });

  it("Skip intro ends it at once and Home is usable", () => {
    start();
    fireEvent.click(screen.getByRole("button", { name: "Skip intro" }));
    expect(opening()).toBeNull();
    expect(frame().hasAttribute("inert")).toBe(false);
    expect(anim.running().filter((a) => a.target.closest(".opening") ?? a.target === frame())).toHaveLength(0);
    expect(document.activeElement).toBe(document.querySelector(".app-main"));
    fireEvent.click(screen.getByRole("button", { name: /Start a full move/ }));
    expect(screen.getByRole("heading", { level: 1 }).textContent).toBe("Which laptop is this?");
  });

  it("Escape ends it at once", () => {
    start();
    fireEvent.keyDown(document.activeElement ?? document.body, { key: "Escape" });
    expect(opening()).toBeNull();
    expect(screen.getByRole("heading", { level: 1 }).textContent).toBe("Send files,or move everything.");
  });

  it("ends by itself when its clock finishes, and only removes itself", async () => {
    start();
    await act(async () => {
      anim.on(".opening")[0]?.finish();
      await Promise.resolve();
    });
    expect(opening()).toBeNull();
    expect(frame().hasAttribute("inert")).toBe(false);
    expect(screen.getByRole("button", { name: /Start a full move/ })).toBeTruthy();
  });

  it("does not play again on the next start of the same version; plays again after an update", () => {
    start();
    fireEvent.click(screen.getByRole("button", { name: "Skip intro" }));
    cleanup();
    start();
    expect(opening()).toBeNull();
    const restores = anim.on(".app");
    const restore = restores[restores.length - 1];
    expect(restore.duration).toBe(240);
    expect(restore.properties).toEqual(["opacity"]);
    cleanup();
    localStorage.setItem("pctwin.opening.shown", "0.0.1");
    start();
    expect(opening()).toBeTruthy();
  });

  it("with reduced motion it is the still identity for 560 ms: no letters, no travel", () => {
    vi.stubGlobal("matchMedia", (q: string) => ({ matches: q.includes("reduce"), media: q, addEventListener: () => undefined, removeEventListener: () => undefined }));
    start();
    expect(opening()).toBeTruthy();
    expect(document.querySelectorAll(".opening-letter")).toHaveLength(0);
    expect(anim.on(".opening")[0]?.duration).toBe(560);
    expect(anim.made.every((a) => !a.properties.includes("transform"))).toBe(true);
    fireEvent.click(screen.getByRole("button", { name: "Skip intro" }));
    expect(opening()).toBeNull();
  });

  it("with motion off nothing plays, and the first launch still counts", () => {
    setMotionPreference("off");
    start();
    expect(opening()).toBeNull();
    expect(anim.made).toHaveLength(0);
    expect(lastShownVersion()).toBe(APP_VERSION);
  });

  it("turning motion down while it plays ends it", () => {
    start();
    act(() => {
      setMotionPreference("reduced");
    });
    expect(opening()).toBeNull();
  });

  it("never hides a real problem: a move to pick up ends it at once", async () => {
    tauri.invoke.mockImplementation((command: string) => Promise.resolve(command === "recovery_check" ? { unfinished: 3 } : undefined));
    start();
    expect(await screen.findByText(l10n.getString("pickup-title"))).toBeTruthy();
    expect(opening()).toBeNull();
    expect(frame().hasAttribute("inert")).toBe(false);
  });
});
