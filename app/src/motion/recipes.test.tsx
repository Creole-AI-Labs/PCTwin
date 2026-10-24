import { LocalizationProvider } from "@fluent/react";
import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import App from "../App";
import { ProgressBar } from "../components/kit";
import { localization } from "../i18n";
import Paired from "../screens/Paired";
import { fakeAnimations, type FakeAnimation } from "../testing/animations";
import { resetOpening } from "./openingRule";
import { resetMotion } from "./preference";
import { arrivalOffset, resetScreenMotion, settleScreen } from "./screen";

const tauri = vi.hoisted(() => ({ invoke: vi.fn(), listen: vi.fn() }));
vi.mock("@tauri-apps/api/core", () => ({ invoke: tauri.invoke }));
vi.mock("@tauri-apps/api/event", () => ({ listen: tauri.listen }));

let anim: ReturnType<typeof fakeAnimations>;

function startApp(language: "en" | "ar" = "en"): void {
  localStorage.setItem("pctwin.opening.shown", "seen");
  resetOpening();
  render(<App initialLanguage={language} />);
}

/** The latest screen arrival on the app's content. */
function arrival(): FakeAnimation {
  const all = anim.on(".app-main");
  return all[all.length - 1];
}
const startX = (a: FakeAnimation) => String(a.keyframes[0].transform ?? "");

function page(html: string): HTMLElement {
  const main = document.createElement("main");
  main.innerHTML = html;
  document.body.append(main);
  return main;
}

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
  document.querySelectorAll("main").forEach((m) => {
    m.remove();
  });
});

describe("screen to screen (MC02)", () => {
  it("forward arrives from the trailing side, Back from the leading side, over 10 px in 440 ms", () => {
    startApp();
    fireEvent.click(screen.getByRole("button", { name: /Start a full move/ }));
    expect(startX(arrival())).toBe("translateX(10px)");
    expect(arrival().duration).toBe(440);
    fireEvent.click(screen.getByRole("button", { name: /Back/ }));
    expect(startX(arrival())).toBe("translateX(-10px)");
  });

  it("right to left mirrors the direction", () => {
    expect(arrivalOffset(false, -1)).toBe(-10);
    expect(arrivalOffset(true, -1)).toBe(10);
    startApp("ar");
    const l10n = localization("ar");
    fireEvent.click(screen.getByRole("button", { name: new RegExp(l10n.getString("welcome-start")) }));
    expect(startX(arrival())).toBe("translateX(-10px)");
    fireEvent.click(screen.getByRole("button", { name: new RegExp(l10n.getString("frame-back")) }));
    expect(startX(arrival())).toBe("translateX(10px)");
  });

  it("latest wins: a new screen stops what the last one was still settling", () => {
    const a = page('<ul><li class="kit-row"></li></ul>');
    settleScreen(a);
    const first = anim.running();
    expect(first.length).toBeGreaterThan(0);
    settleScreen(page('<ul><li class="kit-row"></li></ul>'));
    expect(first.every((x) => x.cancelled)).toBe(true);
  });
});

describe("pairing and codes (MC12 to MC14)", () => {
  it("a screen showing a code or the numbers to compare does not move at all", () => {
    settleScreen(page('<div class="kit-code-box"><span>123456</span></div><ul><li class="kit-row"></li></ul>'));
    settleScreen(page('<ul class="pair-numbers"><li class="pair-tile">42</li></ul>'));
    expect(anim.made).toHaveLength(0);
  });

  it("a confirmed pairing gets one small accent on its icon; the words stay still", () => {
    render(
      <LocalizationProvider l10n={localization("en")}>
        <Paired />
      </LocalizationProvider>,
    );
    expect(anim.on(".motion-accent")).toHaveLength(1);
    expect(anim.made.filter((a) => !a.target.matches(".motion-accent"))).toHaveLength(0);
  });
});

describe("failure, warnings and evidence (MC23)", () => {
  it("a screen with a failure on it arrives at full strength; the failure is never dimmed or moved on its own", () => {
    const main = page('<div class="kit-banner kit-banner-attention" role="alert">Not copied</div><ul><li class="kit-row" role="alert"></li><li class="kit-row"></li></ul>');
    settleScreen(main);
    const own = anim.on(main);
    expect(own).toHaveLength(1);
    expect(own[0].keyframes.some((k) => "opacity" in k && k.opacity !== 1)).toBe(false);
    expect(anim.on('[role="alert"]')).toHaveLength(0);
    expect(anim.on(".kit-row")).toHaveLength(1);
  });
});

describe("measured progress (MC16, MC21)", () => {
  it("the bar is scaled to the observed value (transform only) and holds nothing of its own", () => {
    const { container } = render(<ProgressBar value={0.42} label="Moving" />);
    const fill = container.querySelector<HTMLElement>(".kit-progress-fill");
    expect(fill?.style.transform).toBe("scaleX(0.42)");
    expect(fill?.style.inlineSize).toBe("");
    expect(anim.made).toHaveLength(0);
  });
});
