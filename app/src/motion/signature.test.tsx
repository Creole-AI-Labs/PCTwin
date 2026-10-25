import { LocalizationProvider } from "@fluent/react";
import { act, cleanup, fireEvent, render, screen } from "@testing-library/react";
import type { ReactNode } from "react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import App from "../App";
import { localization } from "../i18n";
import { finishedClean, finishedMixed, finishedStopped } from "../moveSamples";
import { BLUE_FOX, GREEN_OWL, OFFER, RECEIPT_PARTIAL, RECEIPT_WHOLE } from "../quicksend.fixtures";
import Outcome from "../screens/Outcome";
import QuickReceipt from "../screens/QuickReceipt";
import { fakeAnimations } from "../testing/animations";
import { APP_VERSION, resetOpening } from "./openingRule";
import { resetMotion } from "./preference";
import { resetScreenMotion, settleRows } from "./screen";

const tauri = vi.hoisted(() => ({ invoke: vi.fn(), listen: vi.fn() }));
vi.mock("@tauri-apps/api/core", () => ({ invoke: tauri.invoke }));
vi.mock("@tauri-apps/api/event", () => ({ listen: tauri.listen }));

const l10n = localization("en");
let anim: ReturnType<typeof fakeAnimations>;

/** jsdom lays nothing out; give every element a small box so motion has somewhere to go. */
function withLayout(): void {
  vi.spyOn(Element.prototype, "getBoundingClientRect").mockImplementation(function (this: Element) {
    const top = this.closest(".app-header") ? 10 : 300;
    return { x: 20, y: top, left: 20, top, width: 27, height: 22, right: 47, bottom: top + 22, toJSON: () => ({}) };
  });
}

/** Start the app past its opening (an ordinary start, already shown this version). */
function startApp(): void {
  localStorage.setItem("pctwin.opening.shown", APP_VERSION);
  resetOpening();
  render(<App initialLanguage="en" />);
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
  resetMotion();
  document.querySelectorAll(".motion-bookmark").forEach((n) => {
    n.remove();
  });
});

describe("the six signature motions", () => {
  it("the citron bookmark: a Home choice sends a small citron mark into the header mark, then it is gone", async () => {
    withLayout();
    startApp();
    // Not a restored start here: the first launch would have the opening instead.
    expect(localStorage.getItem("pctwin.opening.shown")).not.toBeNull();
    fireEvent.click(screen.getByRole("button", { name: /Start a full move/ }));
    const mark = document.querySelector(".motion-bookmark");
    expect(mark?.getAttribute("aria-hidden")).toBe("true");
    const flight = anim.on(".motion-bookmark")[0];
    expect(flight.duration).toBe(600);
    expect(flight.properties.sort()).toEqual(["opacity", "transform"]);
    // The next screen is already there and working while it flies.
    expect(screen.getByRole("heading", { level: 1 }).textContent).toBe("Which laptop is this?");
    await act(async () => {
      flight.finish();
      await Promise.resolve();
    });
    expect(document.querySelector(".motion-bookmark")).toBeNull();
  });

  it("icons with a job: the icon nudges on keyboard activation; the button and its words stay still", () => {
    startApp();
    const button = screen.getByRole("button", { name: /Restore setup/ });
    fireEvent.keyDown(button, { key: "Enter" });
    const icon = button.querySelector("svg");
    expect(anim.on(icon as Element)).toHaveLength(1);
    expect(anim.on(icon as Element)[0].duration).toBe(460);
    expect(anim.on(button)).toHaveLength(0);
    expect(anim.made.some((a) => a.target.matches("span") && button.contains(a.target))).toBe(false);
  });

  it("paper that settles: only the first five rows, each a little after the last", () => {
    const list = document.createElement("ul");
    for (let i = 0; i < 9; i++) {
      const li = document.createElement("li");
      li.className = "kit-row";
      list.append(li);
    }
    document.body.append(list);
    settleRows(list);
    const rows = anim.on(".kit-row");
    expect(rows).toHaveLength(5);
    expect(rows.map((a) => a.delay)).toEqual([0, 36, 72, 108, 144]);
    expect(rows.every((a) => a.duration === 334)).toBe(true);
    list.remove();
  });

  it("the receipt seal: only on a result verified as a whole", () => {
    const wrap = (ui: ReactNode) => render(<LocalizationProvider l10n={l10n}>{ui}</LocalizationProvider>);
    wrap(<Outcome outcome={finishedClean} onReport={() => undefined} />);
    expect(anim.on(".motion-seal")).toHaveLength(1);
    expect(anim.on(".motion-seal")[0].duration).toBe(600);
    expect(anim.on(".motion-seal-trace")).toHaveLength(1);
    cleanup();
    for (const outcome of [finishedMixed, finishedStopped]) {
      anim.made.length = 0;
      wrap(<Outcome outcome={outcome} onReport={() => undefined} />);
      expect(anim.on(".motion-seal")).toHaveLength(0);
      cleanup();
    }
    anim.made.length = 0;
    wrap(<QuickReceipt side="receive" receipt={RECEIPT_WHOLE} from={BLUE_FOX} to={GREEN_OWL} files={OFFER} onHistory={() => undefined} onDone={() => undefined} />);
    expect(anim.on(".motion-seal")).toHaveLength(1);
    cleanup();
    anim.made.length = 0;
    wrap(<QuickReceipt side="receive" receipt={RECEIPT_PARTIAL} from={BLUE_FOX} to={GREEN_OWL} files={OFFER} onHistory={() => undefined} onDone={() => undefined} />);
    wrap(<QuickReceipt side="send" receipt={RECEIPT_WHOLE} from={BLUE_FOX} to={GREEN_OWL} files={OFFER} stopped onHistory={() => undefined} onDone={() => undefined} />);
    expect(anim.on(".motion-seal")).toHaveLength(0);
  });

  it("a calm way to leave: hiding the window stops every motion at once; coming back is a short fade", () => {
    startApp();
    fireEvent.click(screen.getByRole("button", { name: /Start a full move/ }));
    expect(anim.running().length).toBeGreaterThan(0);
    const hidden = vi.spyOn(document, "hidden", "get").mockReturnValue(true);
    document.dispatchEvent(new Event("visibilitychange"));
    expect(anim.running()).toHaveLength(0);
    hidden.mockReturnValue(false);
    document.dispatchEvent(new Event("visibilitychange"));
    const back = anim.on(".app");
    expect(back[back.length - 1].duration).toBe(240);
    expect(back[back.length - 1].properties).toEqual(["opacity"]);
  });
});
