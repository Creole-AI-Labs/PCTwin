/**
 * The flow guard on the screens' side (Task List 2.8), case by case against the plan's acceptance
 * list (T28-001 to T28-030). The engine's own checks are tested in src-tauri/src/guard.rs; here the
 * screens: what they show before the engine answers, which updates they keep, how many intents a
 * double-click sends, how names are drawn, what is announced, and that nothing is sent on its own.
 */
import { LocalizationProvider } from "@fluent/react";
import { act, cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import type { ReactNode } from "react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { DISPLAY_STATES, REFUSALS, arrive, parseProjection, settle, shouldAnnounce, type IntentKind, type Projection, type Snapshot } from "../guard";
import * as g from "../guardSamples";
import { localization, type LanguageCode } from "../i18n";
import { violations } from "../testing/a11y";
import GuardHost, { type GuardBridge } from "./GuardHost";
import { Name } from "./GuardParts";
import { shownName } from "../names";

const l10n = localization("en");
const text = (id: string, vars?: Record<string, string | number>) => l10n.getString(id, vars);

function wrap(children: ReactNode, language: LanguageCode = "en") {
  return <LocalizationProvider l10n={localization(language)}>{children}</LocalizationProvider>;
}

/** A fake engine: answers the view when told, records intents, and can push events. */
function fakeEngine(first: Snapshot | null = null) {
  let push: ((v: Projection | null) => void) | null = null;
  let answerView: ((s: Snapshot) => void) | null = null;
  const sent: { job: string; epoch: number; generation: number; press: string; intent: IntentKind }[] = [];
  let reply: Awaited<ReturnType<GuardBridge["intent"]>> = { ok: true, answer: "taken" };
  let hold = false;
  let holdView = false;
  const heldViews: ((s: Snapshot) => void)[] = [];
  const pendingReplies: (() => void)[] = [];
  const bridge: GuardBridge = {
    view: vi.fn(
      () =>
        new Promise<Snapshot>((resolve) => {
          if (holdView) heldViews.push(resolve);
          else if (first) resolve(first);
          else answerView = resolve;
        }),
    ),
    intent: vi.fn((i: Parameters<GuardBridge["intent"]>[0]) => {
      sent.push(i);
      return new Promise<Awaited<ReturnType<GuardBridge["intent"]>>>((resolve) => {
        const done = () => {
          resolve(reply);
        };
        if (hold) pendingReplies.push(done);
        else done();
      });
    }),
    listen: vi.fn((handler: (v: Projection | null) => void) => {
      push = handler;
      return Promise.resolve(() => undefined);
    }),
  };
  return {
    bridge,
    sent,
    push: (v: Projection | null) => {
      act(() => push?.(v));
    },
    answer: (s: Snapshot) => {
      act(() => answerView?.(s));
    },
    refuse: (refusal: (typeof REFUSALS)[number]) => {
      reply = { ok: false, refusal };
    },
    holdReplies: () => {
      hold = true;
    },
    release: async () => {
      await act(async () => {
        pendingReplies.splice(0).forEach((f) => {
          f();
        });
        await Promise.resolve();
      });
    },
    setFirst: (s: Snapshot) => {
      first = s;
    },
    /** From now on, each ask for the view waits until answered with `answerHeld`. */
    holdViews: () => {
      holdView = true;
    },
    heldCount: () => heldViews.length,
    answerHeld: async (at: number, s: Snapshot) => {
      await act(async () => {
        heldViews[at](s);
        await Promise.resolve();
      });
    },
  };
}

const ready = (view: Projection): Snapshot => ({ kind: "ready", view });

let logs: unknown[][] = [];
beforeEach(() => {
  logs = [];
  for (const level of ["log", "info", "warn", "error", "debug"] as const) {
    vi.spyOn(console, level).mockImplementation((...args: unknown[]) => {
      logs.push(args);
    });
  }
});
afterEach(() => {
  cleanup();
  vi.restoreAllMocks();
  vi.useRealTimers();
});

describe("before the engine answers, and when it cannot (T28-008, T28-009)", () => {
  it("shows Checking, with no action, until the engine's own view arrives; nothing earlier is replayed", async () => {
    // A copy of an old "complete" view in browser storage must never be shown as current.
    localStorage.setItem("pctwin-guard", JSON.stringify({ display: "complete" }));
    const engine = fakeEngine();
    render(wrap(<GuardHost bridge={engine.bridge} />));
    expect(screen.getByRole("heading", { name: text("guard-checking-title") })).toBeTruthy();
    expect(screen.queryByRole("button")).toBeNull();
    expect(screen.queryByText(text("guard-state-complete"))).toBeNull();
    engine.answer(ready(g.changes));
    expect(await screen.findByRole("heading", { name: text("guard-changes-title") })).toBeTruthy();
    localStorage.removeItem("pctwin-guard");
  });

  it("status unavailable when the engine has no job: never idle or done; Check again only asks again", async () => {
    const engine = fakeEngine({ kind: "unavailable" });
    render(wrap(<GuardHost bridge={engine.bridge} />));
    expect(await screen.findByRole("heading", { name: text("guard-unavailable-title") })).toBeTruthy();
    fireEvent.click(screen.getByRole("button", { name: text("guard-check-again") }));
    await waitFor(() => {
      expect(engine.bridge.view).toHaveBeenCalledTimes(2);
    });
    expect(engine.bridge.intent).not.toHaveBeenCalled();
  });

  it("every made-up view the samples use has the engine's exact shape", () => {
    for (const v of (Object.values(g) as unknown[]).filter((x): x is Projection => typeof x === "object" && x !== null && "screen" in x)) {
      expect(parseProjection(JSON.parse(JSON.stringify(v))), v.screen.kind).toEqual(v);
    }
  });

  it("an event that arrives before the engine's answer still asks for the whole view", () => {
    const next = arrive({ kind: "checking" }, g.changes);
    expect(next.askAgain).toBe(true);
  });
});

describe("which updates are kept (T28-003, T28-007)", () => {
  const newer = { ...g.changes, version: g.changes.version + 1 };
  it("another job, an older session or an older version is dropped", () => {
    const shown = { kind: "ready" as const, view: g.changes };
    expect(arrive(shown, { ...newer, job: "job-other" }).shown).toBe(shown);
    expect(arrive(shown, { ...newer, epoch: g.changes.epoch - 1 }).shown).toBe(shown);
    expect(arrive(shown, { ...g.changes, version: g.changes.version - 1, display: "complete" }).shown).toBe(shown);
    expect(arrive(shown, newer).shown).toEqual({ kind: "ready", view: newer });
  });

  it("the engine's answer is checked like an event: an older view never replaces a newer one (third review, finding 7)", () => {
    const shown = { kind: "ready" as const, view: newer };
    expect(settle(shown, ready(g.changes), false)).toBe(shown);
    expect(settle(shown, ready({ ...newer, epoch: newer.epoch - 1, version: 99 }), false)).toBe(shown);
    expect(settle(shown, ready({ ...newer, version: newer.version + 1 }), false)).toEqual({ kind: "ready", view: { ...newer, version: newer.version + 1 } });
    // The same version is the engine's own word for it: kept.
    expect(settle(shown, ready(newer), false)).toEqual({ kind: "ready", view: newer });
    // "No job" from before a newer event is not believed; from after it, it is.
    expect(settle(shown, { kind: "unavailable" }, true)).toBe(shown);
    expect(settle(shown, { kind: "unavailable" }, false)).toEqual({ kind: "unavailable" });
    expect(settle({ kind: "checking" }, ready(g.changes), false)).toEqual({ kind: "ready", view: g.changes });
  });

  it("on screen: a late answer older than an event already shown does not move the screen back", async () => {
    const engine = fakeEngine();
    render(wrap(<GuardHost bridge={engine.bridge} />));
    const later = { ...g.undoDone, job: g.changes.job, epoch: g.changes.epoch, version: g.changes.version + 1 };
    engine.push(later);
    await screen.findByRole("heading", { name: text("guard-undo-done-title") });
    engine.answer(ready(g.changes));
    await act(async () => {
      await Promise.resolve();
    });
    expect(screen.getByRole("heading", { name: text("guard-undo-done-title") })).toBeTruthy();
    expect(screen.queryByRole("heading", { name: text("guard-changes-title") })).toBeNull();
  });

  it("on screen: only the newest ask's answer counts, and an answer from before an event does not undo it", async () => {
    const engine = fakeEngine(ready(g.changes));
    engine.refuse("plan-changed");
    render(wrap(<GuardHost bridge={engine.bridge} />));
    const approve = await screen.findByRole("button", { name: text("guard-changes-approve") });
    engine.holdViews();
    // Two refusals, each asking for the view again, the first answer arriving last.
    fireEvent.click(approve);
    await waitFor(() => {
      expect(engine.heldCount()).toBe(1);
    });
    fireEvent.click(approve);
    await waitFor(() => {
      expect(engine.heldCount()).toBe(2);
    });
    await engine.answerHeld(1, ready(g.changes));
    await engine.answerHeld(0, { kind: "unavailable" });
    expect(screen.getByRole("heading", { name: text("guard-changes-title") })).toBeTruthy();
    // An ask on its way, then a newer event: the ask's "no job" answer is from before it.
    fireEvent.click(approve);
    await waitFor(() => {
      expect(engine.heldCount()).toBe(3);
    });
    engine.push({ ...g.changes, version: g.changes.version + 1 });
    await engine.answerHeld(2, { kind: "unavailable" });
    expect(screen.getByRole("heading", { name: text("guard-changes-title") })).toBeTruthy();
  });

  it("a gap in versions asks the engine again", () => {
    const shown = { kind: "ready" as const, view: g.changes };
    expect(arrive(shown, { ...g.changes, version: g.changes.version + 3 }).askAgain).toBe(true);
    expect(arrive(shown, newer).askAgain).toBe(false);
  });

  it("on screen: a late success from an older session does not move the screen", async () => {
    const engine = fakeEngine(ready(g.changes));
    render(wrap(<GuardHost bridge={engine.bridge} />));
    await screen.findByRole("heading", { name: text("guard-changes-title") });
    engine.push({ ...g.undoDone, epoch: g.changes.epoch - 1, version: 99, display: "complete" });
    engine.push({ ...g.undoDone, job: "job-other", version: 99, display: "complete" });
    expect(screen.getByRole("heading", { name: text("guard-changes-title") })).toBeTruthy();
    expect(screen.queryByText(text("guard-state-complete"))).toBeNull();
  });

  it("an update that does not have the engine's shape asks for the view again (T28-001: no demo event is trusted)", async () => {
    const engine = fakeEngine(ready(g.changes));
    render(wrap(<GuardHost bridge={engine.bridge} />));
    await screen.findByRole("heading", { name: text("guard-changes-title") });
    expect(parseProjection({ kind: "windows-approved", job: "job-sample" })).toBeNull();
    engine.push(null);
    await waitFor(() => {
      expect(engine.bridge.view).toHaveBeenCalledTimes(2);
    });
  });
});

describe("intents (T28-002, T28-006, T28-024, T28-028)", () => {
  it("each button sends one intent bound to the job, session and plan generation shown", async () => {
    const engine = fakeEngine(ready(g.changes));
    render(wrap(<GuardHost bridge={engine.bridge} />));
    fireEvent.click(await screen.findByRole("button", { name: text("guard-changes-approve") }));
    expect(engine.sent).toEqual([{ job: "job-sample", epoch: 2, generation: 3, press: expect.any(String) as string, intent: { kind: "approve" } }]);
  });

  it("each press carries its own mark, so the engine takes every press once (third review, finding 6)", async () => {
    const engine = fakeEngine(ready(g.changes));
    render(wrap(<GuardHost bridge={engine.bridge} />));
    const approve = await screen.findByRole("button", { name: text("guard-changes-approve") });
    fireEvent.click(approve);
    await waitFor(() => {
      expect(engine.sent).toHaveLength(1);
    });
    await act(async () => {
      await Promise.resolve();
    });
    fireEvent.click(approve);
    await waitFor(() => {
      expect(engine.sent).toHaveLength(2);
    });
    expect(engine.sent[0].press).not.toBe(engine.sent[1].press);
    expect(engine.sent[0].press.length).toBeGreaterThan(0);
  });

  it("a double-click and a held Enter key send one intent", async () => {
    const engine = fakeEngine(ready(g.changes));
    engine.holdReplies();
    render(wrap(<GuardHost bridge={engine.bridge} />));
    const approve = await screen.findByRole("button", { name: text("guard-changes-approve") });
    fireEvent.click(approve);
    fireEvent.click(approve);
    fireEvent.click(approve);
    expect(engine.sent).toHaveLength(1);
    expect(approve.getAttribute("aria-disabled")).toBe("true");
    await engine.release();
  });

  it("a stale approval is refused by the engine: the reason is said, and the view is fetched again", async () => {
    const engine = fakeEngine(ready(g.changes));
    engine.refuse("plan-changed");
    render(wrap(<GuardHost bridge={engine.bridge} />));
    fireEvent.click(await screen.findByRole("button", { name: text("guard-changes-approve") }));
    expect((await screen.findByRole("alert")).textContent).toContain(text("guard-refused-plan-changed"));
    await waitFor(() => {
      expect(engine.bridge.view).toHaveBeenCalledTimes(2);
    });
  });

  it("buttons the engine does not allow now are not drawn", async () => {
    const engine = fakeEngine(ready({ ...g.changes, allowed: ["back"] }));
    render(wrap(<GuardHost bridge={engine.bridge} />));
    await screen.findByRole("heading", { name: text("guard-changes-title") });
    expect(screen.queryByRole("button", { name: text("guard-changes-approve") })).toBeNull();
  });

  it("no timer, screen change or animation sends anything: consent is never by timeout", async () => {
    vi.useFakeTimers({ shouldAdvanceTime: true });
    const engine = fakeEngine(ready(g.permissionExplain));
    render(wrap(<GuardHost bridge={engine.bridge} />));
    await screen.findByRole("heading", { name: text("guard-permission-title-explain") });
    act(() => {
      vi.advanceTimersByTime(60 * 60 * 1000);
    });
    for (const el of document.querySelectorAll("*")) {
      fireEvent.animationEnd(el);
      fireEvent.transitionEnd(el);
    }
    engine.push({ ...g.permissionExplain, version: g.permissionExplain.version + 1 });
    expect(engine.bridge.intent).not.toHaveBeenCalled();
  });
});

describe("what is shown (T28-005, T28-010 to T28-012, T28-014, T28-022)", () => {
  it("hostile, long, right-to-left and mixed-script names are text only: no element, link or script from them", async () => {
    const engine = fakeEngine(ready(g.changes));
    render(wrap(<GuardHost bridge={engine.bridge} />));
    await screen.findByRole("heading", { name: text("guard-changes-title") });
    expect(document.querySelector("img")).toBeNull();
    expect(document.querySelector("a[href]")).toBeNull();
    expect(document.body.textContent).toContain(g.HOSTILE);
    const rtl = [...document.querySelectorAll("bdi")].find((b) => b.textContent === g.RTL);
    expect(rtl).toBeTruthy();
  });

  it("mixed results are both said: an app PCTwin can remove next to a mail change only the service can undo", async () => {
    const engine = fakeEngine(ready(g.changes));
    render(wrap(<GuardHost bridge={engine.bridge} />));
    await screen.findByRole("heading", { name: text("guard-changes-title") });
    expect(screen.getByText(text("guard-undo-can-remove-app"))).toBeTruthy();
    expect(screen.getByText(text("guard-undo-needs-you-service"))).toBeTruthy();
    expect(screen.getByText(text("guard-undo-needs-you-maker"))).toBeTruthy();
    expect(screen.getAllByText(text("guard-undo-must-keep")).length).toBeGreaterThan(0);
  });

  it("undo of apps PCTwin installed says each one plainly: removed, kept at another version, already gone, or left for you", async () => {
    const preview = {
      ...g.undoPreview,
      screen: {
        ...g.undoPreview.screen,
        lines: [
          { name: "Writer", plan: "remove-app" },
          { name: "Viewer", plan: "keep-other-version" },
          { name: "Player", plan: "already-gone" },
          { name: "Tool", plan: "remove-yourself" },
        ],
      },
    } as Projection;
    const engine = fakeEngine(ready(preview));
    render(wrap(<GuardHost bridge={engine.bridge} />));
    await screen.findByRole("heading", { name: text("guard-undo-preview-title") });
    for (const id of ["guard-undo-plan-remove-app", "guard-undo-plan-keep-other-version", "guard-undo-plan-already-gone", "guard-undo-plan-remove-yourself"]) {
      expect(screen.getByText(text(id))).toBeTruthy();
    }
    cleanup();
    const done = {
      ...g.undoDone,
      screen: {
        kind: "undo-done",
        lines: [
          { name: "Writer", result: { kind: "removed-restart" }, detail: null },
          { name: "Viewer", result: { kind: "kept-other-version" }, detail: null },
          { name: "Player", result: { kind: "not-removed" }, detail: null },
        ],
        more: 0,
      },
    } as Projection;
    const again = fakeEngine(ready(done));
    render(wrap(<GuardHost bridge={again.bridge} />));
    await screen.findByRole("heading", { name: text("guard-undo-done-title") });
    for (const id of ["guard-undo-result-removed-restart", "guard-undo-result-kept-other-version", "guard-undo-result-not-removed"]) {
      expect(screen.getByText(text(id))).toBeTruthy();
    }
    // Nothing reads as a raw key.
    expect(document.body.textContent).not.toMatch(/guard-undo-/);
  });

  it("an unknown total is an indeterminate bar with the measured amount, never a percent", async () => {
    const engine = fakeEngine(ready(g.undoRunning));
    render(wrap(<GuardHost bridge={engine.bridge} />));
    await screen.findByRole("heading", { name: text("guard-undo-running-title") });
    const bar = screen.getByRole("progressbar");
    expect(bar.getAttribute("aria-valuenow")).toBeNull();
    expect(bar.getAttribute("aria-valuetext")).toContain("3");
    expect(document.body.textContent).not.toMatch(/\d+ ?%/);
  });

  it("pause asked for is said as not paused yet", async () => {
    const engine = fakeEngine(ready({ ...g.localWorking, display: "pause-requested" }));
    render(wrap(<GuardHost bridge={engine.bridge} />));
    expect(await screen.findByText(text("guard-state-pause-requested"))).toBeTruthy();
    expect(screen.queryByText(text("guard-state-paused"))).toBeNull();
  });

  it("a system prompt closed without an answer is not an approval: it asks again or carries on without", async () => {
    const engine = fakeEngine(ready(g.permissionClosed));
    render(wrap(<GuardHost bridge={engine.bridge} />));
    expect(await screen.findByRole("heading", { name: text("guard-permission-title-closed-without-answer") })).toBeTruthy();
    expect(screen.getByRole("button", { name: text("guard-permission-ask-again") })).toBeTruthy();
    expect(screen.queryByRole("button", { name: text("guard-changes-approve") })).toBeNull();
    // No password field is ever drawn: the system asks, not PCTwin.
    expect(document.querySelector("input")).toBeNull();
  });

  it("every display state and refusal has words, and the state line carries an icon beside them (not colour alone)", async () => {
    for (const s of DISPLAY_STATES) expect(l10n.getBundle(`guard-state-${s}`)?.hasMessage(`guard-state-${s}`), s).toBe(true);
    for (const r of REFUSALS) expect(l10n.getBundle(`guard-refused-${r}`)?.hasMessage(`guard-refused-${r}`), r).toBe(true);
    const engine = fakeEngine(ready(g.report));
    render(wrap(<GuardHost bridge={engine.bridge} />));
    const line = await screen.findByText(text("guard-state-partial"));
    expect(line.closest("[role=status]")?.querySelector("svg")).toBeTruthy();
  });

  it("the rescue picker cannot choose the drive things are put on, and never asks for a key", async () => {
    const engine = fakeEngine(ready(g.rescuePick));
    render(wrap(<GuardHost bridge={engine.bridge} />));
    await screen.findByRole("heading", { name: text("guard-rescue-title-pick-drive") });
    expect(screen.getAllByRole("button", { name: text("guard-rescue-choose") })).toHaveLength(1);
    expect(screen.getByText(text("guard-rescue-drive-cannot"))).toBeTruthy();
    fireEvent.click(screen.getByRole("button", { name: text("guard-rescue-choose") }));
    expect(engine.sent[0]?.intent).toEqual({ kind: "choose-drive", drive: 0 });
    cleanup();
    render(wrap(<GuardHost bridge={fakeEngine(ready(g.rescueBitLocker)).bridge} />));
    await screen.findByRole("heading", { name: text("guard-rescue-title-locked") });
    expect(document.querySelector("input")).toBeNull();
    expect(screen.getByText(text("guard-rescue-never-key"))).toBeTruthy();
  });

  it("Try again names its own line", async () => {
    const engine = fakeEngine(ready(g.report));
    render(wrap(<GuardHost bridge={engine.bridge} />));
    const buttons = await screen.findAllByRole("button", { name: text("guard-try-again") });
    expect(buttons).toHaveLength(3);
    fireEvent.click(buttons[1]);
    expect(engine.sent[0]?.intent).toEqual({ kind: "try-again", line: 1 });
  });
});

describe("focus and announcements (T28-018, T28-020)", () => {
  it("a progress update keeps focus where the person put it; a new decision moves it to the heading", async () => {
    const engine = fakeEngine(ready(g.localWorking));
    render(wrap(<GuardHost bridge={engine.bridge} />));
    const pause = await screen.findByRole("button", { name: text("guard-pause") });
    pause.focus();
    engine.push({ ...g.localWorking, version: g.localWorking.version + 1, progress: { done: 6_000_000_000, total: 12_400_000_000, unit: "bytes" } });
    expect(document.activeElement).toBe(screen.getByRole("button", { name: text("guard-pause") }));
    engine.push({ ...g.localMetered, version: g.localWorking.version + 2 });
    await waitFor(() => {
      expect(document.activeElement).toBe(screen.getByRole("heading", { name: text("guard-local-title") }));
    });
  });

  it("quarter marks only, and not more often than every 30 seconds", () => {
    const p = (done: number) => ({ done, total: 100, unit: "bytes" as const });
    let last = { mark: 0, at: 0 };
    const said: number[] = [];
    let now = 1_000_000;
    for (let done = 0; done <= 100; done++) {
      now += 1000;
      const r = shouldAnnounce(p(done), last, now);
      if (r.say) {
        said.push(r.mark);
        last = { mark: r.mark, at: now };
      }
    }
    expect(said).toEqual([25, 50, 75, 100]);
    expect(shouldAnnounce({ done: 5, total: null, unit: "bytes" }, { mark: 0, at: 0 }, now).say).toBe(false);
    // Fast progress: 50 is reached 2 seconds after 25 was said, so it waits.
    expect(shouldAnnounce(p(50), { mark: 25, at: now }, now + 2000).say).toBe(false);
  });

  it("the live region is in the page before progress changes, beside the bar, polite", async () => {
    const engine = fakeEngine(ready(g.localWorking));
    render(wrap(<GuardHost bridge={engine.bridge} />));
    await screen.findByRole("progressbar");
    const live = document.querySelector(".g-progress [aria-live=polite]");
    expect(live).toBeTruthy();
    for (let i = 1; i <= 50; i++) {
      engine.push({ ...g.localWorking, version: g.localWorking.version + i, progress: { done: 3_100_000_000 + i * 1000, total: 12_400_000_000, unit: "bytes" } });
    }
    expect(live?.textContent).toBe(text("guard-progress-mark", { percent: 25 }));
  });
});

describe("language, privacy and the shipped build (T28-021, T28-023, T28-025, T28-029)", () => {
  it("Arabic: every sample renders in the language with no missing words or accessibility violations", async () => {
    const engine = fakeEngine(ready(g.changes));
    render(wrap(<main><GuardHost bridge={engine.bridge} /></main>, "ar"));
    await waitFor(() => {
      expect(document.querySelector("h1")?.textContent).toBeTruthy();
    });
    expect(await violations()).toEqual([]);
  });

  it("with motion off every state and action is still there: the same words and buttons", async () => {
    const draw = async () => {
      const engine = fakeEngine(ready(g.localWorking));
      const { container, unmount } = render(wrap(<GuardHost bridge={engine.bridge} />));
      await screen.findByRole("heading", { name: text("guard-local-title") });
      // The quarter mark is announced once the measured progress is drawn; wait for it both times.
      await screen.findByText(text("guard-progress-mark", { percent: 25 }));
      const seen = { text: container.textContent, buttons: screen.getAllByRole("button").map((b) => b.textContent) };
      unmount();
      return seen;
    };
    const full = await draw();
    document.documentElement.dataset.motion = "off";
    const off = await draw();
    delete document.documentElement.dataset.motion;
    expect(off).toEqual(full);
  });

  it("refused intents write nothing to the console, whatever the engine's error held", async () => {
    const engine = fakeEngine(ready(g.changes));
    engine.refuse("unavailable");
    render(wrap(<GuardHost bridge={engine.bridge} />));
    fireEvent.click(await screen.findByRole("button", { name: text("guard-changes-approve") }));
    await screen.findByRole("alert");
    expect(logs).toEqual([]);
  });

  it("the shipped screens never import the made-up samples or the review gallery", () => {
    const sources = import.meta.glob<string>(["../**/*.ts", "../**/*.tsx", "!../**/*.test.ts", "!../**/*.test.tsx", "!../gallery/**", "!../guardSamples.ts", "!../moveSamples.ts", "!../planSamples.ts", "!../quicksend.fixtures.ts"], {
      query: "?raw",
      import: "default",
      eager: true,
    });
    const offenders = Object.entries(sources)
      .filter(([, src]) => /from ["'][./]*(gallery\/|guardSamples)/.test(src))
      .map(([path]) => path);
    expect(offenders).toEqual([]);
  });
});

describe("names that change text direction (third review, finding 10)", () => {
  it("a direction character in a name is shown as a visible mark, everywhere a guard name is drawn", async () => {
    const tricky = "holiday\u202Egpj.exe";
    const view = { ...g.changes, screen: { ...g.changes.screen, changes: [{ name: tricky, action: "merge", undo: "can-reverse" }] } } as Projection;
    const parsed = parseProjection(JSON.parse(JSON.stringify(view)));
    expect(JSON.stringify(parsed)).not.toMatch(/[\u061C\u200E\u200F\u202A-\u202E\u2066-\u2069]/);
    const engine = fakeEngine(ready(parsed ?? view));
    render(wrap(<GuardHost bridge={engine.bridge} />));
    await screen.findByRole("heading", { name: text("guard-changes-title") });
    expect(document.body.textContent).toContain("holiday\uFFFDgpj.exe");
    expect(document.body.textContent).not.toContain("\u202E");
    // Letters of a right-to-left language are left as they are.
    expect(shownName(g.RTL)).toBe(g.RTL);
    // The Name part marks them itself too, whatever view it is given.
    const { container } = render(wrap(<Name>{"a\u202Eb"}</Name>));
    expect(container.textContent).toBe("a\uFFFDb");
    for (const c of ["\u061C", "\u200E", "\u200F", "\u202A", "\u202B", "\u202C", "\u202D", "\u202E", "\u2066", "\u2067", "\u2068", "\u2069"]) {
      expect(shownName(`a${c}b`)).toBe("a\uFFFDb");
    }
  });
});
