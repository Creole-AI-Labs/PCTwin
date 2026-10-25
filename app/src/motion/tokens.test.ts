import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { BLUEPRINT, DURATION, SPEED, STAGGER_MS, STAGGER_ROWS, at, cssVariables } from "./tokens";
import { applyToDocument, modeFor, motionMode, resetMotion, setMotionPreference } from "./preference";

describe("motion tokens", () => {
  it("ship every duration at half the blueprint's speed (doubled), the opening at 4,800 ms", () => {
    expect(SPEED).toBe(0.5);
    for (const [name, ms] of Object.entries(BLUEPRINT.durationMs)) expect(DURATION[name as keyof typeof DURATION]).toBe(ms * 2);
    expect(DURATION.wordplay).toBe(4800);
    expect(DURATION.reducedOpening).toBe(560);
    expect(STAGGER_MS).toBe(36);
    expect(STAGGER_ROWS).toBe(5);
    expect(at(1740)).toBe(3480);
  });

  it("give the stylesheets every token as a --motion-* property, the older names as aliases", () => {
    const vars = cssVariables();
    expect(vars["--motion-navigate"]).toBe("440ms");
    expect(vars["--motion-signature"]).toBe("600ms");
    expect(vars["--motion-distance-navigation"]).toBe("10px");
    expect(vars["--motion-feedback"]).toBe("240ms");
    expect(vars["--motion-page"]).toBe("440ms");
    expect(vars["--motion-ease-settle"]).toBe("cubic-bezier(.2,0,0,1)");
  });
});

describe("motion setting", () => {
  beforeEach(() => {
    localStorage.clear();
    resetMotion();
  });
  afterEach(() => {
    vi.unstubAllGlobals();
    localStorage.clear();
    resetMotion();
  });

  it("follows the system by default; the system asking for less always wins over following it", () => {
    expect(modeFor("system", false)).toBe("full");
    expect(modeFor("system", true)).toBe("reduced");
    expect(modeFor("reduced", false)).toBe("reduced");
    expect(modeFor("off", true)).toBe("off");
    expect(modeFor("off", false)).toBe("off");
  });

  it("reads the system's reduced-motion setting", () => {
    vi.stubGlobal("matchMedia", (q: string) => ({ matches: q.includes("reduce"), media: q, addEventListener: () => undefined, removeEventListener: () => undefined }));
    resetMotion();
    expect(motionMode()).toBe("reduced");
  });

  it("keeps the choice on this laptop and survives a blocked store", () => {
    setMotionPreference("off");
    expect(localStorage.getItem("pctwin.motion")).toBe("off");
    resetMotion();
    expect(motionMode()).toBe("off");
    vi.spyOn(Storage.prototype, "getItem").mockImplementation(() => {
      throw new Error("blocked");
    });
    resetMotion();
    expect(motionMode()).toBe("full");
  });

  it("writes the mode on the page and stops every CSS duration when reduced or off", () => {
    applyToDocument();
    const root = document.documentElement;
    expect(root.dataset.motion).toBe("full");
    expect(root.style.getPropertyValue("--motion-navigate")).toBe("440ms");
    setMotionPreference("reduced");
    expect(root.dataset.motion).toBe("reduced");
    expect(root.style.getPropertyValue("--motion-navigate")).toBe("0ms");
    expect(root.style.getPropertyValue("--motion-distance-row")).toBe("4px");
    setMotionPreference("off");
    expect(root.dataset.motion).toBe("off");
    expect(root.style.getPropertyValue("--motion-feedback")).toBe("0ms");
  });
});
