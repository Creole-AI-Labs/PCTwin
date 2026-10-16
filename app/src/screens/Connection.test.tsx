import { LocalizationProvider } from "@fluent/react";
import type { ReactNode } from "react";
import { cleanup, fireEvent, render, screen, within } from "@testing-library/react";
import axe from "axe-core";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { ConnectionCheck, PeerReadiness, WaitingState } from "../connection";
import { localization } from "../i18n";
import CheckConnection from "./CheckConnection";
import ConnectionEnded from "./ConnectionEnded";
import ConnectionRoute from "./ConnectionRoute";
import MeetLaptop from "./MeetLaptop";
import PutTogether from "./PutTogether";
import WaitingForLaptop from "./WaitingForLaptop";
import Welcome from "./Welcome";

const l10n = localization("en");
const text = (id: string, vars?: Record<string, string | number | Date>) => l10n.getString(id, vars);
/** Fluent isolates variables for right-to-left safety; the marks are invisible, so tests compare without them. */
const plain = (s: string) => s.replace(/[⁦-⁩]/g, "");

/** Anything paid, or any promise the app cannot keep, must never appear on these screens. */
const PAID = /sign[ -]?in|log[ -]?in|account|cloud|plans|price|billing|payment|paid|upgrade|\bpro\b|premium|trial|subscri|[$€£₦]/i;
const PROMISE = /guarantee|will finish in|exactly|promise to|standard or smart/i;

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

function inApp(ui: ReactNode) {
  return render(
    <LocalizationProvider l10n={l10n}>
      <main>{ui}</main>
    </LocalizationProvider>,
  );
}

async function violations(): Promise<string[]> {
  const result = await axe.run(document.body, { rules: { "color-contrast": { enabled: false } } });
  return result.violations.map((v) => `${v.id}: ${v.nodes.map((n) => n.target.join(" ")).join(", ")}`);
}

const noStringsMissing = () => {
  expect(warnings.filter((w) => /missing|not found|did not match|unknown/i.test(w))).toEqual([]);
};
const heading = () => screen.getByRole("heading", { level: 1 });
const nothing = () => undefined;

/* ---------------------------------------------------------------- F01 Resume your move */

describe("F01 Welcome with an unfinished move", () => {
  it("is unchanged when no move is waiting: Start a move, no Resume", () => {
    inApp(<Welcome onStart={nothing} onSoon={nothing} onQuick={nothing} />);
    expect(screen.getByRole("button", { name: text("welcome-start") })).toBeTruthy();
    expect(screen.queryByRole("button", { name: text("resume-open") })).toBeNull();
  });

  it("the primary button reads Resume your move, not Start a move, and reopens the saved move", () => {
    const onStart = vi.fn();
    const onResume = vi.fn();
    inApp(<Welcome onStart={onStart} onSoon={nothing} onQuick={nothing} unfinished onResume={onResume} />);
    const button = screen.getByRole("button", { name: text("resume-open") });
    expect(screen.queryByRole("button", { name: text("welcome-start") })).toBeNull();
    button.focus();
    expect(document.activeElement).toBe(button);
    fireEvent.click(button);
    expect(onResume).toHaveBeenCalledTimes(1);
    expect(onStart).not.toHaveBeenCalled();
    noStringsMissing();
  });

  it("changes nothing else: same picture, headline and secondary choices, nothing paid, no accessibility violations", async () => {
    inApp(<Welcome onStart={nothing} onSoon={nothing} onQuick={nothing} unfinished onResume={nothing} />);
    expect(heading().textContent).toBe("Your setup.Your next laptop.");
    expect(document.querySelector(".illustration")).toBeTruthy();
    expect(screen.getAllByRole("button").map((b) => b.textContent)).toEqual(["Resume your move", "Restore setup", "Rescue a laptop", "Quick Send"]);
    expect(screen.getByRole("main").textContent).not.toMatch(PAID);
    expect(await violations()).toEqual([]);
  });
});

/* ---------------------------------------------------------------- C01 */

describe("C01 How will you connect?", () => {
  it("offers Wi-Fi as the only way, and Use Wi-Fi is the action", () => {
    const onWifi = vi.fn();
    inApp(<ConnectionRoute onBack={nothing} onWifi={onWifi} />);
    expect(heading().textContent).toBe(text("connect-title"));
    expect(screen.getAllByRole("button")).toHaveLength(2); // Back and Use Wi-Fi
    expect(screen.getByText(text("connect-wifi-title"))).toBeTruthy();
    expect(screen.getByText(text("connect-wifi-body"))).toBeTruthy();
    fireEvent.click(screen.getByRole("button", { name: text("connect-use-wifi") }));
    expect(onWifi).toHaveBeenCalledTimes(1);
    noStringsMissing();
  });

  it("has no disabled, greyed or locked card for the later routes, only one quiet plain line", () => {
    inApp(<ConnectionRoute onBack={nothing} onWifi={nothing} />);
    expect(document.querySelectorAll("[disabled], [aria-disabled='true']")).toHaveLength(0);
    expect(screen.getByText(text("connect-later"))).toBeTruthy();
    expect(screen.getAllByRole("heading", { level: 2 })).toHaveLength(1);
    expect(screen.getByRole("main").textContent).not.toMatch(PAID);
  });

  it("keeps the note that an ordinary USB cable cannot connect two laptops", () => {
    inApp(<ConnectionRoute onBack={nothing} onWifi={nothing} />);
    expect(screen.getByText(text("connect-usb-note"))).toBeTruthy();
  });

  it("puts keyboard focus on the heading and Back works", () => {
    const onBack = vi.fn();
    inApp(<ConnectionRoute onBack={onBack} onWifi={nothing} />);
    expect(document.activeElement).toBe(heading());
    fireEvent.click(screen.getByRole("button", { name: text("pair-back") }));
    expect(onBack).toHaveBeenCalled();
  });

  it("has no accessibility violations", async () => {
    inApp(<ConnectionRoute onBack={nothing} onWifi={nothing} />);
    expect(await violations()).toEqual([]);
  });
});

/* ---------------------------------------------------------------- C02 */

describe("C02 Put both laptops together", () => {
  const stepItems = () => screen.getAllByRole("listitem").filter((li) => li.matches(".frame-step"));

  it("lists three steps in order, each with its state in words", () => {
    inApp(<PutTogether onBack={nothing} onContinue={nothing} />);
    const steps = stepItems();
    expect(steps).toHaveLength(3);
    expect(within(steps[0]).getByText(text("connect-prep-step-1-title"))).toBeTruthy();
    expect(within(steps[1]).getByText(text("connect-prep-step-2-title"))).toBeTruthy();
    expect(within(steps[2]).getByText(text("connect-prep-step-3-title"))).toBeTruthy();
    expect(within(steps[0]).getByText(text("frame-step-current"))).toBeTruthy();
    expect(within(steps[1]).getByText(text("frame-step-todo"))).toBeTruthy();
    expect(within(steps[2]).getByText(text("frame-step-todo"))).toBeTruthy();
    noStringsMissing();
  });

  it("shows done, current and to-do each with its own word, a step position for screen readers, and marks the current step", () => {
    inApp(<PutTogether steps={["done", "current", "todo"]} onBack={nothing} onContinue={nothing} />);
    const steps = stepItems();
    expect(within(steps[0]).getByText(text("frame-step-done"))).toBeTruthy();
    expect(within(steps[1]).getByText(text("frame-step-current"))).toBeTruthy();
    expect(within(steps[2]).getByText(text("frame-step-todo"))).toBeTruthy();
    expect(steps[1].getAttribute("aria-current")).toBe("step");
    expect(steps[0].getAttribute("aria-current")).toBeNull();
    expect(steps[2].getAttribute("aria-current")).toBeNull();
    expect(plain(steps[1].textContent)).toContain(plain(text("frame-step-position", { number: 2, total: 3 })));
  });

  it("names the Wi-Fi this laptop is on only when it is known", () => {
    const { rerender } = inApp(<PutTogether onBack={nothing} onContinue={nothing} />);
    expect(screen.queryByText((c) => plain(c).includes("Home Wi-Fi"))).toBeNull();
    rerender(
      <LocalizationProvider l10n={l10n}>
        <main>
          <PutTogether network="Home Wi-Fi" onBack={nothing} onContinue={nothing} />
        </main>
      </LocalizationProvider>,
    );
    expect(screen.getByText((c) => plain(c) === plain(text("connect-prep-step-1-network", { network: "Home Wi-Fi" })))).toBeTruthy();
  });

  it("explains guest Wi-Fi and networks that keep devices apart", () => {
    inApp(<PutTogether onBack={nothing} onContinue={nothing} />);
    expect(screen.getByText(text("connect-prep-note"))).toBeTruthy();
  });

  it("continues to pairing, and Back goes back", () => {
    const onContinue = vi.fn();
    const onBack = vi.fn();
    inApp(<PutTogether onBack={onBack} onContinue={onContinue} />);
    expect(document.activeElement).toBe(heading());
    fireEvent.click(screen.getByRole("button", { name: text("connect-prep-continue") }));
    expect(onContinue).toHaveBeenCalledTimes(1);
    fireEvent.click(screen.getByRole("button", { name: text("pair-back") }));
    expect(onBack).toHaveBeenCalledTimes(1);
  });

  it("is free, and has no accessibility violations", async () => {
    inApp(<PutTogether steps={["done", "current", "todo"]} onBack={nothing} onContinue={nothing} />);
    expect(screen.getByRole("main").textContent).not.toMatch(PAID);
    expect(await violations()).toEqual([]);
  });
});

/* ---------------------------------------------------------------- C06 */

describe("C06 Meet your other laptop", () => {
  const old = { label: "Blue Fox", system: "Windows 11", version: "1.4.2" };
  const fresh = { label: "Green Owl", system: "macOS 15", version: "1.4.2" };
  const ready: PeerReadiness = { state: "ready", sending: old, receiving: fresh };
  const differ: PeerReadiness = { state: "versions-differ", sending: old, receiving: { ...fresh, version: "1.5.0" }, updateOn: "sending" };
  const unsupported: PeerReadiness = { state: "direction-unsupported", sending: old, receiving: fresh };
  const handlers = () => ({ onContinue: vi.fn(), onBack: vi.fn() });
  const action = () => screen.getByRole("button", { name: text("peer-continue") });

  it("ready: shows both laptops side by side with name, system and version, and the way on", () => {
    const h = handlers();
    inApp(<MeetLaptop readiness={ready} {...h} />);
    expect(heading().textContent).toBe(text("peer-title"));
    const sending = screen.getByRole("region", { name: text("peer-sending") });
    const receiving = screen.getByRole("region", { name: text("peer-receiving") });
    expect(sending.textContent).toContain(text("peer-old"));
    expect(sending.textContent).toContain("Blue Fox");
    expect(sending.textContent).toContain("Windows 11");
    expect(sending.textContent).toContain(text("peer-version-label"));
    expect(plain(sending.textContent)).toContain(plain(text("peer-version", { version: "1.4.2" })));
    expect(receiving.textContent).toContain(text("peer-new"));
    expect(receiving.textContent).toContain("Green Owl");
    expect(receiving.textContent).toContain("macOS 15");
    expect(screen.getByText(text("peer-ready-note"))).toBeTruthy();
    expect(action()).toHaveProperty("disabled", false);
    fireEvent.click(action());
    expect(h.onContinue).toHaveBeenCalledTimes(1);
    noStringsMissing();
  });

  it("different versions: says so, shows which laptop is blocked and which waits, and the way to update; the action stays off", () => {
    const h = handlers();
    inApp(<MeetLaptop readiness={differ} {...h} />);
    expect(screen.getByRole("alert").textContent).toContain(text("peer-version-differs"));
    const rows = screen.getAllByRole("listitem");
    expect(rows).toHaveLength(3);
    expect(plain(rows[0].textContent)).toContain(plain(text("peer-version-old-body", { version: "1.4.2" })));
    expect(within(rows[0]).getByText(text("peer-status-update"))).toBeTruthy();
    expect(rows[1].textContent).toContain(text("peer-version-new-body"));
    expect(within(rows[1]).getByText(text("peer-status-unsupported"))).toBeTruthy();
    expect(rows[2].textContent).toContain(text("peer-update-title"));
    expect(rows[2].textContent).toContain(text("peer-update-body"));
    expect(screen.queryByText(text("peer-ready-note"))).toBeNull();
    expect(action()).toHaveProperty("disabled", true);
    fireEvent.click(action());
    expect(h.onContinue).not.toHaveBeenCalled();
  });

  it("different versions: the receiving laptop can be the one with the incompatible build", () => {
    inApp(<MeetLaptop readiness={{ ...differ, updateOn: "receiving" }} {...handlers()} />);
    const rows = screen.getAllByRole("listitem");
    expect(plain(rows[1].textContent)).toContain(plain(text("peer-version-old-body", { version: "1.5.0" })));
    expect(rows[0].textContent).toContain(text("peer-version-new-body"));
  });

  it("direction not supported yet: says so, keeps both laptops in view, and the action stays off", () => {
    const h = handlers();
    inApp(<MeetLaptop readiness={unsupported} {...h} />);
    expect(screen.getByRole("alert").textContent).toContain(text("peer-direction-unsupported"));
    expect(screen.getByRole("region", { name: text("peer-sending") })).toBeTruthy();
    expect(screen.getByRole("region", { name: text("peer-receiving") })).toBeTruthy();
    expect(screen.queryByText(text("peer-ready-note"))).toBeNull();
    expect(action()).toHaveProperty("disabled", true);
    fireEvent.click(action());
    expect(h.onContinue).not.toHaveBeenCalled();
  });

  it("Back goes back", () => {
    const h = handlers();
    inApp(<MeetLaptop readiness={ready} {...h} />);
    fireEvent.click(screen.getByRole("button", { name: text("pair-back") }));
    expect(h.onBack).toHaveBeenCalledTimes(1);
  });

  it("moves focus to a new heading when the state changes", () => {
    const h = handlers();
    const { rerender } = inApp(<MeetLaptop readiness={ready} {...h} />);
    expect(document.activeElement).toBe(heading());
    if (document.activeElement instanceof HTMLElement) document.activeElement.blur();
    rerender(
      <LocalizationProvider l10n={l10n}>
        <main>
          <MeetLaptop readiness={differ} {...h} />
        </main>
      </LocalizationProvider>,
    );
    expect(document.activeElement).toBe(heading());
  });

  it.each([
    ["ready", ready],
    ["versions-differ", differ],
    ["direction-unsupported", unsupported],
  ])("%s: free, no promises, no accessibility violations", async (_n, readiness) => {
    inApp(<MeetLaptop readiness={readiness} {...handlers()} />);
    expect(screen.getByRole("main").textContent).not.toMatch(PAID);
    expect(screen.getByRole("main").textContent).not.toMatch(PROMISE);
    expect(await violations()).toEqual([]);
  });
});

/* ---------------------------------------------------------------- C07 */

describe("C07 Let's check the connection", () => {
  const good: ConnectionCheck = { oldLabel: "Blue Fox", newLabel: "Green Owl", speedMbPerSecond: 62, link: "good", freeSpaceGb: 410, powerOld: "plugged-in", powerNew: "plugged-in", internet: "online" };
  const weak: ConnectionCheck = { ...good, speedMbPerSecond: 4, link: "weak" };
  const offline: ConnectionCheck = { ...good, internet: "offline" };
  const handlers = () => ({ onContinue: vi.fn(), onRecheck: vi.fn(), onBack: vi.fn() });
  /** The row (list item) that starts with this title. */
  const row = (title: string) => {
    const el = screen.getByText(title).closest("li");
    if (!el) throw new Error(`no row ${title}`);
    return el;
  };
  const unit = (id: string, value: number) => plain(text(id, { value }));

  it("good link: shows the measured speed and space, a word for power and the internet, and no warning", () => {
    const h = handlers();
    inApp(<CheckConnection check={good} {...h} />);
    expect(heading().textContent).toBe(text("check-title"));
    expect(plain(screen.getByText(text("check-speed-label")).parentElement?.textContent ?? "")).toContain(unit("connect-unit-mbps", 62));
    expect(plain(screen.getByText(text("check-space-label")).parentElement?.textContent ?? "")).toContain(unit("connect-unit-gb", 410));
    const power = row(text("check-power-title"));
    expect(within(power).getByText(text("check-status-ready"))).toBeTruthy();
    expect(power.textContent).toContain(text("check-power-both"));
    expect(row(text("check-internet-title")).textContent).toContain(text("check-internet-on"));
    expect(within(row(text("check-internet-title"))).getByText(text("check-status-connected"))).toBeTruthy();
    expect(screen.queryByRole("alert")).toBeNull();
    fireEvent.click(screen.getByRole("button", { name: text("check-continue") }));
    expect(h.onContinue).toHaveBeenCalledTimes(1);
    noStringsMissing();
  });

  it("weak link: warns in words that moving may be slow and what helps, but still lets the person go on", () => {
    inApp(<CheckConnection check={{ ...good, speedMbPerSecond: 4, link: "weak" }} {...handlers()} />);
    expect(screen.getByRole("alert").textContent).toContain(text("check-link-weak"));
    expect(plain(screen.getByText(text("check-speed-label")).parentElement?.textContent ?? "")).toContain(unit("connect-unit-mbps", 4));
    expect(screen.getByRole("button", { name: text("check-continue") })).toHaveProperty("disabled", false);
  });

  it("offline: says that is fine, because copying files does not need the internet", () => {
    inApp(<CheckConnection check={{ ...good, internet: "offline" }} {...handlers()} />);
    const net = row(text("check-internet-title"));
    expect(within(net).getByText(text("check-status-not-needed"))).toBeTruthy();
    expect(net.textContent).toContain(text("check-internet-off"));
    expect(within(net).queryByText(text("check-status-connected"))).toBeNull();
    expect(screen.getByRole("button", { name: text("check-continue") })).toHaveProperty("disabled", false);
  });

  it("a laptop on battery is named and asked to be plugged in", () => {
    inApp(<CheckConnection check={{ ...good, powerOld: "on-battery" }} {...handlers()} />);
    const power = row(text("check-power-title"));
    expect(within(power).getByText(text("check-status-plug-in"))).toBeTruthy();
    expect(plain(power.textContent)).toContain(plain(text("check-power-battery", { label: "Blue Fox" })));
    expect(power.textContent).not.toContain(text("check-power-both"));
    expect(within(power).queryByText(text("check-status-ready"))).toBeNull();
  });

  it("what could not be read says so and is never shown as ready", () => {
    inApp(<CheckConnection check={{ ...good, speedMbPerSecond: null, freeSpaceGb: null, powerOld: "unknown", powerNew: "unknown", internet: "unknown" }} {...handlers()} />);
    expect(screen.getByRole("alert").textContent).toContain(text("check-speed-unknown"));
    expect(row(text("check-space-title")).textContent).toContain(text("check-space-unknown"));
    expect(plain(row(text("check-power-title")).textContent)).toContain(plain(text("check-power-unknown", { label: "Blue Fox" })));
    expect(row(text("check-internet-title")).textContent).toContain(text("check-internet-unknown"));
    expect(screen.queryAllByText(text("check-status-ready"))).toHaveLength(0);
    expect(screen.queryAllByText(text("check-status-connected"))).toHaveLength(0);
    expect(screen.getAllByText(text("check-status-unknown")).length).toBeGreaterThan(0);
  });

  it("while measuring there are no numbers, only that it is measuring, and nothing to continue with", () => {
    inApp(<CheckConnection check={null} {...handlers()} />);
    expect(screen.getByRole("status").textContent).toContain(text("check-measuring"));
    expect(screen.queryByText(text("check-power-title"))).toBeNull();
    expect(screen.queryByRole("button", { name: text("check-continue") })).toBeNull();
    noStringsMissing();
  });

  it("gives time only as a range based on the measured speed, never a promise, and offers no Standard or Smart choice", () => {
    inApp(<CheckConnection check={good} {...handlers()} />);
    expect(screen.getByText(text("check-estimate-note"))).toBeTruthy();
    expect(screen.getByRole("main").textContent).not.toMatch(PROMISE);
    expect(screen.getByRole("main").textContent).not.toMatch(/standard|smart/i);
  });

  it("Check again and Back call out", () => {
    const h = handlers();
    inApp(<CheckConnection check={good} {...h} />);
    fireEvent.click(screen.getByRole("button", { name: text("check-again") }));
    fireEvent.click(screen.getByRole("button", { name: text("pair-back") }));
    expect(h.onRecheck).toHaveBeenCalledTimes(1);
    expect(h.onBack).toHaveBeenCalledTimes(1);
  });

  it.each([
    ["good", good],
    ["weak", weak],
    ["offline", offline],
    ["measuring", null],
  ])("%s: free and no accessibility violations", async (_n, check) => {
    inApp(<CheckConnection check={check} {...handlers()} />);
    expect(screen.getByRole("main").textContent).not.toMatch(PAID);
    expect(await violations()).toEqual([]);
  });
});

/* ---------------------------------------------------------------- C09 */

describe("C09 Waiting for your other laptop", () => {
  const base: WaitingState = { kind: "reconnecting", otherLaptop: "Blue Fox", savedGb: 81.6, totalGb: 120, lastSeenSeconds: 45 };
  const handlers = () => ({ onRetry: vi.fn(), onPairAgain: vi.fn() });
  const withState = (waiting: WaitingState, h = handlers()) => (
    <LocalizationProvider l10n={l10n}>
      <main>
        <WaitingForLaptop waiting={waiting} {...h} />
      </main>
    </LocalizationProvider>
  );

  it("reconnecting: says it is trying again by itself, and shows what is saved as measured", () => {
    const h = handlers();
    inApp(<WaitingForLaptop waiting={base} {...h} />);
    expect(heading().textContent).toBe(text("reconnect-title"));
    expect(screen.getByText(text("reconnect-body"))).toBeTruthy();
    expect(screen.getByRole("status").textContent).toContain(text("reconnect-trying"));
    expect(screen.getByText(text("reconnect-status-trying"))).toBeTruthy();
    const bar = screen.getByRole("progressbar", { name: "Move paused by connection" });
    expect(bar.getAttribute("aria-valuenow")).toBe("68");
    const saved = text("reconnect-saved", { done: plain(text("connect-unit-gb", { value: 81.6 })), total: plain(text("connect-unit-gb", { value: 120 })) });
    expect(screen.getByText((c) => plain(c) === plain(saved))).toBeTruthy();
    expect(screen.getByText((c) => plain(c).includes("45 seconds ago"))).toBeTruthy();
    fireEvent.click(screen.getByRole("button", { name: text("reconnect-try-now") }));
    expect(h.onRetry).toHaveBeenCalledTimes(1);
    expect(screen.queryByRole("button", { name: text("reconnect-pair-again") })).toBeNull();
    noStringsMissing();
  });

  it("last seen is told in minutes after a while, and left out when there is none", () => {
    const { rerender } = inApp(<WaitingForLaptop waiting={{ ...base, lastSeenSeconds: 300 }} {...handlers()} />);
    expect(screen.getByText((c) => plain(c).includes("5 minutes ago"))).toBeTruthy();
    rerender(withState({ ...base, lastSeenSeconds: null }));
    expect(screen.queryByText((c) => /Last seen/.test(c))).toBeNull();
  });

  it("pairing expired: says so, and the action is to pair again, not to retry", () => {
    const h = handlers();
    inApp(<WaitingForLaptop waiting={{ ...base, kind: "pairing-expired" }} {...h} />);
    expect(screen.getByRole("alert").textContent).toContain(text("reconnect-expired"));
    expect(screen.getByText(text("reconnect-status-offline"))).toBeTruthy();
    expect(screen.queryByRole("button", { name: text("reconnect-try-now") })).toBeNull();
    expect(screen.queryByText(text("reconnect-trying"))).toBeNull();
    fireEvent.click(screen.getByRole("button", { name: text("reconnect-pair-again") }));
    expect(h.onPairAgain).toHaveBeenCalledTimes(1);
  });

  it("not reachable: names the laptop and what to check, and offers Try now", () => {
    const h = handlers();
    inApp(<WaitingForLaptop waiting={{ ...base, kind: "unreachable" }} {...h} />);
    expect(plain(screen.getByRole("alert").textContent)).toContain(plain(text("reconnect-unavailable", { label: "Blue Fox" })));
    expect(screen.getByText(text("reconnect-status-offline"))).toBeTruthy();
    fireEvent.click(screen.getByRole("button", { name: text("reconnect-try-now") }));
    expect(h.onRetry).toHaveBeenCalledTimes(1);
    expect(screen.queryByRole("button", { name: text("reconnect-pair-again") })).toBeNull();
  });

  it("shows the resume check as a row, pending while it waits, and not pending once pairing has expired", () => {
    const { rerender } = inApp(<WaitingForLaptop waiting={base} {...handlers()} />);
    const resume = screen.getByText(text("reconnect-details")).closest("li");
    expect(resume?.textContent).toContain(text("reconnect-details-body"));
    expect(within(resume as HTMLElement).getByText(text("reconnect-status-pending"))).toBeTruthy();
    rerender(withState({ ...base, kind: "pairing-expired" }));
    expect(screen.queryByText(text("reconnect-status-pending"))).toBeNull();
  });

  it("saved progress is measured: nothing saved reads 0, everything saved reads 100, and it is clamped", () => {
    const { rerender } = inApp(<WaitingForLaptop waiting={{ ...base, savedGb: 0 }} {...handlers()} />);
    expect(screen.getByRole("progressbar").getAttribute("aria-valuenow")).toBe("0");
    rerender(withState({ ...base, savedGb: 120 }));
    expect(screen.getByRole("progressbar").getAttribute("aria-valuenow")).toBe("100");
    rerender(withState({ ...base, savedGb: 130 }));
    expect(screen.getByRole("progressbar").getAttribute("aria-valuenow")).toBe("100");
    rerender(withState({ ...base, savedGb: 0, totalGb: 0 }));
    expect(screen.getByRole("progressbar").getAttribute("aria-valuenow")).toBe("0");
  });

  it("never says the move is finished while it waits", () => {
    inApp(<WaitingForLaptop waiting={base} {...handlers()} />);
    expect(screen.getByRole("main").textContent).not.toMatch(/\b(finished|completed?|done|success)/i);
  });

  it("moves focus to a new heading when the state changes", () => {
    const { rerender } = inApp(<WaitingForLaptop waiting={base} {...handlers()} />);
    if (document.activeElement instanceof HTMLElement) document.activeElement.blur();
    rerender(withState({ ...base, kind: "pairing-expired" }));
    expect(document.activeElement).toBe(heading());
  });

  it.each(["reconnecting", "pairing-expired", "unreachable"] as const)("%s: free and no accessibility violations", async (kind) => {
    inApp(<WaitingForLaptop waiting={{ ...base, kind }} {...handlers()} />);
    expect(screen.getByRole("main").textContent).not.toMatch(PAID);
    expect(await violations()).toEqual([]);
  });
});

/* ---------------------------------------------------------------- C10 */

describe("C10 The connection has ended", () => {
  const handlers = () => ({ onPairAgain: vi.fn(), onSeeMoved: vi.fn(), onHome: vi.fn() });
  const action = (id: string) => screen.getByRole("button", { name: new RegExp(`^${text(id)}`) });

  it("is a dialog that says what is kept and what is not", () => {
    inApp(<ConnectionEnded {...handlers()} />);
    const dialog = screen.getByRole("dialog", { name: text("ended-title") });
    expect(within(dialog).getByText(text("ended-body"))).toBeTruthy();
    expect(within(dialog).getByText(text("ended-kept-title"))).toBeTruthy();
    expect(within(dialog).getByText(text("ended-kept-body"))).toBeTruthy();
    expect(within(dialog).getByText(text("ended-not-kept-title"))).toBeTruthy();
    expect(within(dialog).getByText(text("ended-not-kept-body"))).toBeTruthy();
    noStringsMissing();
  });

  it("offers pair again, see what has moved, and return home", () => {
    const h = handlers();
    inApp(<ConnectionEnded {...h} />);
    fireEvent.click(action("ended-pair-again"));
    fireEvent.click(action("ended-see-moved"));
    fireEvent.click(screen.getByRole("button", { name: text("ended-home") }));
    expect(h.onPairAgain).toHaveBeenCalledTimes(1);
    expect(h.onSeeMoved).toHaveBeenCalledTimes(1);
    expect(h.onHome).toHaveBeenCalledTimes(1);
  });

  it("each action says what it does", () => {
    inApp(<ConnectionEnded {...handlers()} />);
    expect(action("ended-pair-again").textContent).toContain(text("ended-pair-again-body"));
    expect(action("ended-see-moved").textContent).toContain(text("ended-see-moved-body"));
  });

  it("Escape returns home, the safe way out, and nothing else runs", () => {
    const h = handlers();
    inApp(<ConnectionEnded {...h} />);
    fireEvent.keyDown(screen.getByRole("dialog"), { key: "Escape" });
    expect(h.onHome).toHaveBeenCalledTimes(1);
    expect(h.onPairAgain).not.toHaveBeenCalled();
    expect(h.onSeeMoved).not.toHaveBeenCalled();
  });

  it("moves focus into the dialog, onto a button", () => {
    inApp(<ConnectionEnded {...handlers()} />);
    expect(screen.getByRole("dialog").contains(document.activeElement)).toBe(true);
    expect(document.activeElement?.tagName).toBe("BUTTON");
  });

  it("when nothing had moved yet, does not offer to see what has moved", () => {
    inApp(<ConnectionEnded movedSomething={false} {...handlers()} />);
    expect(screen.queryByRole("button", { name: new RegExp(`^${text("ended-see-moved")}`) })).toBeNull();
    expect(action("ended-pair-again")).toBeTruthy();
  });

  it("is free and has no accessibility violations", async () => {
    inApp(<ConnectionEnded {...handlers()} />);
    expect(screen.getByRole("dialog").textContent).not.toMatch(PAID);
    expect(await violations()).toEqual([]);
  });
});
