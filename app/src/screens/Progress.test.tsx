import { LocalizationProvider } from "@fluent/react";
import type { ReactNode } from "react";
import { act, cleanup, fireEvent, render, screen, within } from "@testing-library/react";
import { violations } from "../testing/a11y";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { localization } from "../i18n";
import { Dialog, MiniProgress, ProgressBar } from "../components/kit";
import {
  ALL_REASONS,
  FAILED_REASONS,
  ITEM_STATUSES,
  NEEDS_ACTION_REASONS,
  formatBytes,
  groupState,
  measuredPercent,
  minutesRange,
  parseResult,
  statusOfReason,
  WHY_MEMORY,
  whyMemoryKey,
  type Counts,
  type ItemResult,
  type MoveProgress,
  type ReportItem,
} from "../move";
import * as sample from "../moveSamples";
import { counts } from "../moveSamples";
import { FooterProvider } from "./FrameFooter";
import MiniPanel from "./MiniPanel";
import Outcome from "./Outcome";
import Paused from "./Paused";
import ProgressCompact from "./ProgressCompact";
import { TIP_SECONDS, tipsFor, type TipContext } from "./MoveTips";
import ProgressDetail from "./ProgressDetail";
import Report from "./Report";
import StopMove from "./StopMove";

const l10n = localization("en");
const text = (id: string, vars?: Record<string, string | number>) => l10n.getString(id, vars);
/** Fluent isolates variables for right-to-left safety; the marks are invisible, so tests compare without them. */
const plain = (s: string) => s.replace(/[⁦-⁩]/g, "");
const withText = (expected: string) => (content: string) => plain(content) === plain(expected);
const hasText = (expected: string) => (content: string) => plain(content).includes(plain(expected));
/** An attribute of a translated message (the progress bars' labels). */
const attribute = (id: string, name: string) => {
  const bundle = l10n.getBundle(id);
  const pattern = bundle?.getMessage(id)?.attributes[name];
  if (!bundle || !pattern) throw new Error(`no ${name} on ${id}`);
  return bundle.formatPattern(pattern);
};

let warnings: string[] = [];

beforeEach(() => {
  warnings = [];
  vi.spyOn(console, "warn").mockImplementation((...args: unknown[]) => {
    warnings.push(args.map(String).join(" "));
  });
});

afterEach(() => {
  cleanup();
  document.documentElement.dir = "";
  vi.restoreAllMocks();
});

function inApp(ui: ReactNode) {
  return render(
    <LocalizationProvider l10n={l10n}>
      <main>{ui}</main>
    </LocalizationProvider>,
  );
}

/** The window as the app draws it: content, then the footer the screens draw their actions into. */
function inFrame(ui: ReactNode) {
  return render(
    <LocalizationProvider l10n={l10n}>
      <FooterProvider>
        <main>{ui}</main>
      </FooterProvider>
    </LocalizationProvider>,
  );
}

function footerEl(): HTMLElement {
  const el = document.querySelector<HTMLElement>("footer.app-footer");
  if (!el) throw new Error("no footer");
  return el;
}

/** No string in a screen was missing, and nothing paid was said. */
function expectClean() {
  expect(warnings.filter((w) => /missing|not found|unknown/i.test(w))).toEqual([]);
  const everything = document.body.textContent;
  expect(everything).not.toMatch(/upgrade|\bpro\b|premium|price|plan\b|subscription|trial|sign in to|account|cloud|backup|storage warning/i);
}

const noop = () => undefined;

describe("the engine's results, mirrored", () => {
  it("lists the reasons in the engine's order, with the engine's names", () => {
    // crates/pctwin-types result.rs: reason_names_are_stable_translation_keys
    expect(ALL_REASONS.join(",")).toBe(
      "not-enough-space,changed-during-move,unreadable,not-verified,refused-by-safety-check,name-not-allowed,install-failed,no-equivalent," +
        "sign-in,activation,permission,online-only,manual-install,restart,finish-by-hand",
    );
    expect(FAILED_REASONS).toHaveLength(8);
    expect(NEEDS_ACTION_REASONS).toHaveLength(7);
    // crates/pctwin-types lib.rs: serialises_with_stable_names
    expect(ITEM_STATUSES.join(",")).toBe("copied,reinstalled,converted,needs-action,skipped-by-user,failed");
  });

  it("every reason belongs to exactly one status", () => {
    for (const r of FAILED_REASONS) expect(statusOfReason(r)).toBe("failed");
    for (const r of NEEDS_ACTION_REASONS) expect(statusOfReason(r)).toBe("needs-action");
  });

  it("accepts a result only in a shape the report can show honestly", () => {
    for (const r of FAILED_REASONS) expect(parseResult({ status: "failed", reason: r })).toEqual({ status: "failed", reason: r });
    for (const r of NEEDS_ACTION_REASONS) expect(parseResult({ status: "needs-action", reason: r })).toEqual({ status: "needs-action", reason: r });
    for (const s of ["copied", "reinstalled", "converted", "skipped-by-user"]) expect(parseResult({ status: s })).toEqual({ status: s });
  });

  it("refuses what the engine refuses (result.rs: saved_results_are_checked_when_read)", () => {
    const bad: unknown[] = [
      { status: "failed" },
      { status: "needs-action" },
      { status: "copied", reason: "sign-in" },
      { status: "skipped-by-user", reason: "unreadable" },
      { status: "needs-action", reason: "not-verified" },
      { status: "failed", reason: "sign-in" },
      { status: "copied", extra: 1 },
      { status: "skipped" },
      { status: "failed", reason: "no-such-reason" },
      "copied",
      null,
    ];
    for (const b of bad) expect(parseResult(b), JSON.stringify(b)).toBeNull();
    // Every status with every reason: only the fitting pairs are accepted.
    for (const status of ITEM_STATUSES)
      for (const reason of ALL_REASONS) {
        const fits = (status === "failed" || status === "needs-action") && statusOfReason(reason) === status;
        expect(parseResult({ status, reason }) !== null, `${status} ${reason}`).toBe(fits);
      }
  });

  it("the types themselves refuse a reason that does not fit", () => {
    // These lines only type-check because of the @ts-expect-error: remove the rule and `pnpm typecheck` fails.
    // @ts-expect-error a failed item must say why
    const noReason: ItemResult = { status: "failed" };
    // @ts-expect-error a finished item carries no reason
    const copiedWithReason: ItemResult = { status: "copied", reason: "sign-in" };
    // @ts-expect-error a reason that belongs to needs-action cannot be on a failed item
    const wrong: ItemResult = { status: "failed", reason: "sign-in" };
    // @ts-expect-error only a copied item carries evidence, and it must
    const copiedBare: ReportItem = { id: "x", name: "x", source: "x", status: "copied" };
    expect([noReason, copiedWithReason, wrong, copiedBare]).toHaveLength(4);
  });

  it("every status and every reason has its words", () => {
    for (const s of ITEM_STATUSES) expect(text(`status-${s}`)).not.toBe(`status-${s}`);
    for (const r of ALL_REASONS) {
      const words = text(`reason-${r}`);
      expect(words, r).not.toBe(`reason-${r}`);
      expect(words.length).toBeGreaterThan(20);
    }
  });
});

describe("measured numbers", () => {
  const unfinishedCounts = counts({ done: 99, checking: 1 });

  it("the percent never reaches 100 while anything is waiting, copying or checking", () => {
    for (const kind of ["waiting", "copying", "checking"] as const) {
      const c = counts({ done: 99, [kind]: 1 });
      expect(measuredPercent({ bytes: { checked: 1000, total: 1000 }, counts: c })).toBe(99);
    }
  });

  it("rounds down, never up", () => {
    expect(measuredPercent({ bytes: { checked: 996, total: 1000 }, counts: counts({ done: 1 }) })).toBe(99);
    expect(measuredPercent({ bytes: { checked: 689, total: 1000 }, counts: unfinishedCounts })).toBe(68);
  });

  it("is 100 only when nothing is left, and says no number when the plan has no size", () => {
    expect(measuredPercent({ bytes: { checked: 1000, total: 1000 }, counts: counts({ done: 10 }) })).toBe(100);
    expect(measuredPercent({ bytes: { checked: 0, total: 0 }, counts: unfinishedCounts })).toBeNull();
  });

  it("formats sizes in the reader's language", () => {
    expect(formatBytes(81_600_000_000, "en")).toBe("81.6 GB");
    expect(formatBytes(120_000_000_000, "en")).toBe("120 GB");
  });

  it("the progress bar says what it is given, and nothing when nothing is measured", () => {
    inApp(
      <>
        <ProgressBar value={0.68} label="Measured" />
        <ProgressBar value={null} label="Unmeasured" />
      </>,
    );
    expect(screen.getByRole("progressbar", { name: "Measured" }).getAttribute("aria-valuenow")).toBe("68");
    const none = screen.getByRole("progressbar", { name: "Unmeasured" });
    expect(none.hasAttribute("aria-valuenow")).toBe(false);
    expect(none.querySelector(".kit-progress-fill")).toBeNull();
  });
});

describe("a group's state", () => {
  const c = (p: Partial<Counts>) => counts(p);
  it("is copied only when nothing in it is waiting, copying or checking", () => {
    expect(groupState(c({ done: 10 }))).toBe("copied");
    expect(groupState(c({ done: 10, checking: 1 }))).toBe("checking");
    expect(groupState(c({ done: 10, copying: 1 }))).toBe("copying");
    expect(groupState(c({ done: 10, waiting: 1 }))).toBe("copying");
    expect(groupState(c({ waiting: 10 }))).toBe("waiting");
    expect(groupState(c({ done: 9, checking: 1, failed: 1 }))).toBe("checking");
  });

  it("puts failed before needs-action before copied once nothing is unfinished", () => {
    expect(groupState(c({ done: 5, needsAction: 1 }))).toBe("needs-action");
    expect(groupState(c({ done: 5, needsAction: 1, failed: 1 }))).toBe("failed");
  });
});

describe("X01 Move progress, compact", () => {
  const groupRow = (name: string) => screen.getAllByRole("listitem").find((li) => li.textContent.includes(name)) as HTMLElement;

  it("shows what is measured: the bar, sizes, the two laptops and each group", async () => {
    inApp(<ProgressCompact progress={sample.running} onPause={noop} onResume={noop} onMinimise={noop} onSeeNeedsAction={noop} />);
    const heading = screen.getByRole("heading", { level: 1 });
    expect(heading.textContent).toBe(text("progress-title"));
    expect(document.activeElement).toBe(heading);
    const bar = screen.getByRole("progressbar", { name: attribute("progress-percent", "aria-label") });
    expect(bar.getAttribute("aria-valuenow")).toBe("68"); // 81.6 of 120 GB
    expect(screen.getByText(hasText("81.6 GB of 120 GB"))).toBeTruthy();
    // The route: the old laptop by the name pairing gave it, the new one by its part (it has no
    // label of its own), then each group with its own state.
    expect(screen.getByText("Ada’s old laptop")).toBeTruthy();
    expect(screen.getByText(text("progress-route-old"))).toBeTruthy();
    expect(screen.getByText(text("progress-route-new"))).toBeTruthy();
    expect(screen.getByText(text("progress-paired"))).toBeTruthy();
    expect(within(groupRow("Documents")).getByText(text("progress-state-verified"))).toBeTruthy();
    expect(within(groupRow("Pictures")).getByText(text("progress-state-checking"))).toBeTruthy();
    expect(within(groupRow("Tax 2023")).getByText(text("progress-state-waiting"))).toBeTruthy();
    expect(screen.getByText(withText(text("progress-current-copying", { name: "Holiday 2024 / IMG_2043.jpg" })))).toBeTruthy();
    expect(screen.getByText(text("progress-old-safe"))).toBeTruthy();
    expectClean();
    expect(await violations()).toEqual([]);
  });

  describe("T14 one tip under the bar", () => {
    const calm: TipContext = { old: false, onBattery: false, slow: false, batteryWarning: false };
    const compact = (p: MoveProgress, tips: Omit<TipContext, "batteryWarning">) =>
      inApp(<ProgressCompact progress={p} tips={tips} onPause={noop} onResume={noop} onMinimise={noop} onSeeNeedsAction={noop} />);
    const tipText = () => document.querySelector(".mv-tip-text")?.textContent;

    afterEach(() => {
      vi.useRealTimers();
    });

    it("each laptop's tips are about itself: battery only on battery, the router only when slow, keep using only on the old one", () => {
      expect(tipsFor(calm)).toEqual(["tip-lids", "tip-pause", "tip-power-cut"]);
      expect(tipsFor({ ...calm, old: true })).toEqual(["tip-lids", "tip-keep-using", "tip-pause", "tip-power-cut"]);
      expect(tipsFor({ ...calm, onBattery: true, slow: true })).toEqual(["tip-battery", "tip-closer", "tip-lids", "tip-pause", "tip-power-cut"]);
      // A pinned battery warning is not repeated as a tip.
      expect(tipsFor({ ...calm, onBattery: true, batteryWarning: true })).not.toContain("tip-battery");
    });

    it("changes about every 12 seconds, never as a live announcement, and Pause tips holds it", () => {
      vi.useFakeTimers();
      compact(sample.running, calm);
      const panel = screen.getByRole("region", { name: text("tips-label") });
      expect(panel.closest("[aria-live]")).toBeNull();
      expect(panel.querySelector("[aria-live], [role=status], [role=alert]")).toBeNull();
      expect(tipText()).toBe(text("tip-lids"));
      act(() => {
        vi.advanceTimersByTime(TIP_SECONDS * 1000 - 1);
      });
      expect(tipText()).toBe(text("tip-lids"));
      act(() => {
        vi.advanceTimersByTime(1);
      });
      expect(tipText()).toBe(text("tip-pause"));
      fireEvent.click(within(panel).getByRole("button", { name: text("tips-pause") }));
      act(() => {
        vi.advanceTimersByTime(TIP_SECONDS * 5000);
      });
      expect(tipText()).toBe(text("tip-pause"));
      // Paused, Next tip still moves on by hand, and the control says it can start again.
      fireEvent.click(within(panel).getByRole("button", { name: text("tips-next") }));
      expect(tipText()).toBe(text("tip-power-cut"));
      fireEvent.click(within(panel).getByRole("button", { name: text("tips-resume") }));
      act(() => {
        vi.advanceTimersByTime(TIP_SECONDS * 1000);
      });
      expect(tipText()).toBe(text("tip-lids"));
    });

    it("with reduced motion the tip stays still: no Pause tips, and Next tip by hand", () => {
      vi.useFakeTimers();
      vi.stubGlobal("matchMedia", (query: string) => ({ matches: query.includes("reduce"), media: query, addEventListener: noop, removeEventListener: noop }));
      try {
        compact(sample.running, calm);
        act(() => {
          vi.advanceTimersByTime(TIP_SECONDS * 3000);
        });
        expect(tipText()).toBe(text("tip-lids"));
        expect(screen.queryByRole("button", { name: text("tips-pause") })).toBeNull();
        fireEvent.click(screen.getByRole("button", { name: text("tips-next") }));
        expect(tipText()).toBe(text("tip-pause"));
      } finally {
        vi.unstubAllGlobals();
      }
    });

    it("the battery warning stays pinned in the notices while the tips go on without a battery tip", () => {
      compact(sample.lowBattery, { old: false, onBattery: true, slow: false });
      expect(screen.getByText(text("progress-low-battery"))).toBeTruthy();
      expect(tipText()).toBe(text("tip-lids"));
    });

    it("has every string and no accessibility problems, in all its states", async () => {
      compact(sample.running, { old: true, onBattery: true, slow: true });
      expectClean();
      expect(await violations()).toEqual([]);
    });

    it("without tips (the host gives none), there is no tip panel", () => {
      inApp(<ProgressCompact progress={sample.running} onPause={noop} onResume={noop} onMinimise={noop} onSeeNeedsAction={noop} />);
      expect(document.querySelector(".mv-tip")).toBeNull();
    });
  });

  it("time left and speed are ranges, and only when measured", () => {
    const { unmount } = inApp(<ProgressCompact progress={sample.running} onPause={noop} onResume={noop} onMinimise={noop} onSeeNeedsAction={noop} />);
    const range = text("progress-minutes", { low: 15, high: 20 });
    expect(screen.getByText(withText(text("progress-time-left", { range })))).toBeTruthy();
    expect(screen.getByText(withText(text("progress-speed", { speed: text("progress-speed-range", { low: 58, high: 66, unit: "MB/s" }) })))).toBeTruthy();
    unmount();
    inApp(<ProgressCompact progress={sample.starting} onPause={noop} onResume={noop} onMinimise={noop} onSeeNeedsAction={noop} />);
    expect(screen.getByText(text("progress-time-measuring"))).toBeTruthy();
    expect(screen.queryByText(hasText("MB/s"))).toBeNull();
    expect(screen.queryByText(hasText("About"))).toBeNull();
    expect(document.querySelector(".mv-speed")).toBeNull();
  });

  it("a measured time is never widened: one number reads as about that many minutes", () => {
    const p: MoveProgress = { ...sample.running, minutesLeft: { low: 7, high: 7 } };
    const { unmount } = inApp(<ProgressCompact progress={p} onPause={noop} onResume={noop} onMinimise={noop} onSeeNeedsAction={noop} />);
    const one = text("progress-minutes-one", { minutes: 7 });
    expect(plain(one)).toBe("7 minutes");
    expect(screen.getByText(withText(text("progress-time-left", { range: one })))).toBeTruthy();
    expect(screen.queryByText(hasText(text("progress-minutes", { low: 7, high: 8 })))).toBeNull();
    unmount();
    inApp(<ProgressDetail progress={p} onCollapse={noop} onSeeNeedsAction={noop} />);
    expect(screen.getByText(withText(one))).toBeTruthy();
  });

  it("minutes are rounded and otherwise left as measured", () => {
    expect(minutesRange({ low: 6.6, high: 7.4 })).toEqual({ low: 7, high: 7 });
    expect(minutesRange({ low: 14.6, high: 20.2 })).toEqual({ low: 15, high: 20 });
    expect(minutesRange({ low: -0.4, high: 0.3 })).toEqual({ low: 0, high: 0 });
    expect(plain(text("progress-minutes-one", { minutes: 1 }))).toBe("1 minute");
  });

  it("never says 100 or done while anything is still being checked", () => {
    inApp(<ProgressCompact progress={sample.almost} onPause={noop} onResume={noop} onMinimise={noop} onSeeNeedsAction={noop} />);
    const bar = screen.getByRole("progressbar");
    expect(bar.getAttribute("aria-valuenow")).toBe("99");
    expect(document.body.textContent).not.toMatch(/100\s*%/);
    expect(document.body.textContent).not.toMatch(/\b(done|complete|completed|finished|all set)\b/i);
    // The group with files still being checked is never called verified.
    expect(within(groupRow("Pictures")).queryByText(text("progress-state-verified"))).toBeNull();
    expect(within(groupRow("Pictures")).getByText(text("progress-state-checking"))).toBeTruthy();
    expect(within(groupRow("Documents")).getByText(text("progress-state-verified"))).toBeTruthy();
  });

  it("with nothing measured the bar is empty and says no number", () => {
    const p: MoveProgress = { ...sample.starting, bytes: { checked: 0, total: 0 } };
    inApp(<ProgressCompact progress={p} onPause={noop} onResume={noop} onMinimise={noop} onSeeNeedsAction={noop} />);
    expect(screen.getByRole("progressbar").hasAttribute("aria-valuenow")).toBe(false);
    expect(screen.getByText(text("progress-percent-unknown"))).toBeTruthy();
    expect(document.body.textContent).not.toMatch(/\d\s*%/);
  });

  it("items that need the person are counted apart and do not stop the rest", () => {
    const onSee = vi.fn();
    inApp(<ProgressCompact progress={sample.withNeedsAction} onPause={noop} onResume={noop} onMinimise={noop} onSeeNeedsAction={onSee} />);
    const docs = groupRow("Documents");
    expect(within(docs).getByText(withText(text("progress-count-needs-action", { count: 3 })))).toBeTruthy();
    // Nothing unfinished in it, so it is shown as needing the person, not as verified.
    expect(within(docs).getByText(text("status-needs-action"))).toBeTruthy();
    expect(within(docs).queryByText(text("progress-state-verified"))).toBeNull();
    expect(screen.getByText(withText(text("progress-needs-action", { count: 3 })))).toBeTruthy();
    // The other groups are still there.
    expect(within(groupRow("Pictures")).getByText(text("progress-state-checking"))).toBeTruthy();
    fireEvent.click(screen.getByRole("button", { name: text("progress-see-needs-action") }));
    expect(onSee).toHaveBeenCalledOnce();
  });

  it("items that could not be copied stay visible", () => {
    inApp(<ProgressCompact progress={sample.withFailures} onPause={noop} onResume={noop} onMinimise={noop} onSeeNeedsAction={noop} />);
    const pictures = groupRow("Pictures");
    expect(within(pictures).getByText(withText(text("progress-count-failed", { count: 1 })))).toBeTruthy();
    // Still being worked on, so its state is a working one; the failure is counted under its name.
    expect(within(pictures).getByText(text("progress-state-checking"))).toBeTruthy();
  });

  it("no line for needs-action or failed when there are none", () => {
    inApp(<ProgressCompact progress={sample.running} onPause={noop} onResume={noop} onMinimise={noop} onSeeNeedsAction={noop} />);
    expect(screen.queryByText(hasText("need your action"))).toBeNull();
    expect(screen.queryByText(hasText("could not be copied"))).toBeNull();
  });

  it("says when the connection dropped, and when a laptop is low on battery", () => {
    const { unmount } = inApp(<ProgressCompact progress={sample.linkLost} onPause={noop} onResume={noop} onMinimise={noop} onSeeNeedsAction={noop} />);
    expect(screen.getByText(text("progress-interrupted"))).toBeTruthy();
    unmount();
    inApp(<ProgressCompact progress={sample.lowBattery} onPause={noop} onResume={noop} onMinimise={noop} onSeeNeedsAction={noop} />);
    expect(screen.getByText(text("progress-low-battery"))).toBeTruthy();
  });

  it("its footer, as in the mockup, holds Pause move and Keep compact and nothing else is a button", () => {
    const h = { onPause: vi.fn(), onResume: vi.fn(), onMinimise: vi.fn() };
    inFrame(<ProgressCompact progress={sample.running} {...h} onSeeNeedsAction={noop} />);
    const footer = footerEl();
    fireEvent.click(within(footer).getByRole("button", { name: text("progress-pause") }));
    fireEvent.click(within(footer).getByRole("button", { name: text("progress-compact") }));
    expect(h.onPause).toHaveBeenCalledOnce();
    expect(h.onMinimise).toHaveBeenCalledOnce();
    expect(h.onResume).not.toHaveBeenCalled();
    // No other buttons in the content: no "see the whole move", no stop.
    const inContent = within(screen.getByRole("main")).queryAllByRole("button");
    expect(inContent.filter((b) => !footer.contains(b))).toEqual([]);
    expect(screen.queryByRole("button", { name: text("progress-expand") })).toBeNull();
    expect(screen.queryByRole("button", { name: text("progress-stop") })).toBeNull();
    // No Back strip on X01.
    expect(screen.queryByRole("button", { name: text("frame-back") })).toBeNull();
  });

  it("a paused move shows the paused screen, not a running one", () => {
    inApp(<ProgressCompact progress={sample.paused} onPause={noop} onResume={noop} onMinimise={noop} onSeeNeedsAction={noop} />);
    expect(screen.getByRole("heading", { level: 1 }).textContent).toBe(text("pause-title"));
    expect(screen.queryByRole("button", { name: text("progress-pause") })).toBeNull();
  });

  it("works right to left", async () => {
    document.documentElement.dir = "rtl";
    inApp(<ProgressCompact progress={sample.withFailures} onPause={noop} onResume={noop} onMinimise={noop} onSeeNeedsAction={noop} />);
    expect(await violations()).toEqual([]);
  });
});

describe("X02 Move progress, expanded", () => {
  const props = { progress: sample.running, onCollapse: noop, onSeeNeedsAction: noop };
  const rowOf = (name: string) => screen.getAllByRole("row").find((r) => r.textContent.includes(name)) as HTMLElement;

  it("shows one table row for each group, with its source and its state in words", async () => {
    inApp(<ProgressDetail {...props} />);
    const heading = screen.getByRole("heading", { level: 1 });
    expect(heading.textContent).toBe(text("progress-all-title"));
    expect(document.activeElement).toBe(heading);
    expect(screen.getAllByRole("row")).toHaveLength(sample.running.groups.length + 1); // plus the header row
    expect(screen.getByRole("columnheader", { name: text("progress-col-item") })).toBeTruthy();
    expect(screen.getByRole("columnheader", { name: text("progress-col-from") })).toBeTruthy();
    expect(screen.getByRole("columnheader", { name: text("progress-col-status") })).toBeTruthy();
    const docs = rowOf(text("plan-role-documents"));
    expect(within(docs).getByText("Ada’s old laptop")).toBeTruthy();
    expect(within(docs).getByText(text("progress-state-verified"))).toBeTruthy();
    expect(within(rowOf(text("plan-role-pictures"))).getByText(text("progress-state-checking"))).toBeTruthy();
    expect(within(rowOf("Tax 2023")).getByText(text("progress-state-waiting"))).toBeTruthy();
    expect(screen.getByText(text("progress-checked-note"))).toBeTruthy();
    expectClean();
    expect(await violations()).toEqual([]);
  });

  it("a group still being checked is never shown as verified", () => {
    inApp(<ProgressDetail {...props} progress={sample.almost} />);
    const pictures = rowOf(text("plan-role-pictures"));
    expect(within(pictures).queryByText(text("progress-state-verified"))).toBeNull();
    expect(within(pictures).getByText(text("progress-state-checking"))).toBeTruthy();
    expect(within(rowOf(text("plan-role-documents"))).getByText(text("progress-state-verified"))).toBeTruthy();
  });

  it("items that need the person or failed are counted under their group", () => {
    inApp(<ProgressDetail {...props} progress={sample.withFailures} />);
    const docs = rowOf(text("plan-role-documents"));
    expect(within(docs).getByText(withText(text("progress-count-needs-action", { count: 3 })))).toBeTruthy();
    expect(within(docs).getByText(text("status-needs-action"))).toBeTruthy();
    expect(within(rowOf(text("plan-role-pictures"))).getByText(withText(text("progress-count-failed", { count: 1 })))).toBeTruthy();
  });

  it("waiting groups read Paused while the move is paused", () => {
    inApp(<ProgressDetail {...props} progress={{ ...sample.paused, groups: sample.running.groups }} />);
    const row = rowOf("Tax 2023");
    expect(within(row).getByText(text("progress-state-paused"))).toBeTruthy();
    expect(within(row).queryByText(text("progress-state-waiting"))).toBeNull();
  });

  it("says so when there are no groups", () => {
    inApp(<ProgressDetail {...props} progress={{ ...sample.running, groups: [] }} />);
    expect(screen.getByText(text("progress-queue-empty"))).toBeTruthy();
    expect(screen.queryByRole("table")).toBeNull();
  });

  it("has the approved-job strip with Back to the compact view, and no footer action", () => {
    const onCollapse = vi.fn();
    inFrame(<ProgressDetail {...props} onCollapse={onCollapse} />);
    expect(screen.getByText(text("progress-approved-job"))).toBeTruthy();
    fireEvent.click(screen.getByRole("button", { name: text("frame-back") }));
    expect(onCollapse).toHaveBeenCalledOnce();
    // The footer is the usual one: no screen action is drawn into it.
    expect(within(footerEl()).queryByRole("button", { name: text("progress-pause") })).toBeNull();
    expect(within(footerEl()).getByText(text("footer-free"))).toBeTruthy();
  });

  it("shows three measured numbers, never 100 while something is checking", () => {
    inApp(<ProgressDetail {...props} progress={sample.almost} />);
    expect(screen.getByText("99%")).toBeTruthy();
    expect(screen.getByText(text("progress-metric-overall"))).toBeTruthy();
    expect(screen.getByText("120 GB")).toBeTruthy(); // 119.97 GB, shown to one decimal at most
    expect(document.body.textContent).not.toMatch(/100\s*%/);
  });

  it("with nothing measured the numbers say none, not zero", () => {
    inApp(<ProgressDetail {...props} progress={{ ...sample.starting, bytes: { checked: 0, total: 0 } }} />);
    expect(screen.getAllByText("—").length).toBeGreaterThanOrEqual(2);
    expect(document.body.textContent).not.toMatch(/\d\s*%/);
  });
});

describe("F09 Mini panel", () => {
  it("shows a move: what is moving, the measured percent, Pause and Open", async () => {
    const h = { onPause: vi.fn(), onResume: vi.fn(), onOpen: vi.fn() };
    inApp(<MiniPanel progress={sample.running} {...h} />);
    const group = screen.getByRole("group", { name: text("mini-title") });
    expect(group.getAttribute("data-kind")).toBe("move");
    expect(within(group).getByText(text("mini-what"))).toBeTruthy();
    expect(screen.getByRole("progressbar", { name: attribute("mini-progress", "aria-label") }).getAttribute("aria-valuenow")).toBe("68");
    expect(within(group).getByText("68%")).toBeTruthy();
    fireEvent.click(screen.getByRole("button", { name: text("mini-pause") }));
    fireEvent.click(screen.getByRole("button", { name: text("mini-open") }));
    expect(h.onPause).toHaveBeenCalledOnce();
    expect(h.onOpen).toHaveBeenCalledOnce();
    expectClean();
    expect(await violations()).toEqual([]);
  });

  it("shows a Quick Send with its own words", () => {
    inApp(<MiniPanel kind="send" progress={sample.running} onPause={noop} onResume={noop} onOpen={noop} />);
    const group = screen.getByRole("group", { name: text("mini-title-send") });
    expect(group.getAttribute("data-kind")).toBe("send");
    expect(within(group).getByText(text("mini-what-send"))).toBeTruthy();
    expect(screen.getByRole("button", { name: text("mini-open-send") })).toBeTruthy();
    expect(screen.getByRole("progressbar", { name: attribute("mini-progress-send", "aria-label") })).toBeTruthy();
    expect(screen.queryByText(text("mini-title"))).toBeNull();
  });

  it("never reaches 100 while anything is checking", () => {
    inApp(<MiniPanel progress={sample.almost} onPause={noop} onResume={noop} onOpen={noop} />);
    expect(screen.getByRole("progressbar").getAttribute("aria-valuenow")).toBe("99");
    expect(document.body.textContent).not.toMatch(/100\s*%/);
  });

  it("with nothing measured it shows no percent", () => {
    inApp(<MiniPanel progress={{ ...sample.starting, bytes: { checked: 0, total: 0 } }} onPause={noop} onResume={noop} onOpen={noop} />);
    expect(screen.getByRole("progressbar").hasAttribute("aria-valuenow")).toBe(false);
    expect(document.body.textContent).not.toMatch(/\d\s*%/);
  });

  it("a paused move offers Resume instead of Pause", () => {
    const onResume = vi.fn();
    inApp(<MiniPanel progress={sample.paused} onPause={noop} onResume={onResume} onOpen={noop} />);
    expect(screen.getByRole("group", { name: text("mini-title-paused") })).toBeTruthy();
    expect(screen.queryByRole("button", { name: text("mini-pause") })).toBeNull();
    fireEvent.click(screen.getByRole("button", { name: text("mini-resume") }));
    expect(onResume).toHaveBeenCalledOnce();
  });

  it("a move that is pausing says it is finishing the file first", () => {
    inApp(<MiniPanel progress={sample.pausing} onPause={noop} onResume={noop} onOpen={noop} />);
    expect(screen.getByRole("group", { name: text("pause-title-finishing") })).toBeTruthy();
  });

  it("its Open button sits in the footer with the free note", () => {
    const onOpen = vi.fn();
    inFrame(<MiniPanel progress={sample.running} onPause={noop} onResume={noop} onOpen={onOpen} />);
    const footer = footerEl();
    expect(within(footer).getByText(text("footer-note-move"))).toBeTruthy();
    fireEvent.click(within(footer).getByRole("button", { name: text("mini-open") }));
    expect(onOpen).toHaveBeenCalledOnce();
    // The panel itself holds only Pause.
    expect(within(screen.getByRole("group", { name: text("mini-title") })).getAllByRole("button")).toHaveLength(1);
  });

  it("has none of the paid states", () => {
    inApp(<MiniPanel progress={sample.running} onPause={noop} onResume={noop} onOpen={noop} />);
    expect(document.body.textContent).not.toMatch(/cloud|storage|backup|saved|protection/i);
  });

  it("the kit's MiniProgress takes a kind and an unmeasured value", () => {
    inApp(<MiniProgress kind="send" title="Sending" value={null} label="Send" pause={<button>p</button>} open={<button>o</button>} />);
    const group = screen.getByRole("group", { name: "Sending" });
    expect(group.getAttribute("data-kind")).toBe("send");
    expect(screen.getByRole("progressbar").hasAttribute("aria-valuenow")).toBe(false);
  });
});

describe("X05 Paused at a safe point", () => {
  it("says what is kept and what waits", async () => {
    inApp(<Paused progress={sample.paused} onResume={noop} />);
    const heading = screen.getByRole("heading", { level: 1 });
    expect(heading.textContent).toBe(text("pause-title"));
    expect(document.activeElement).toBe(heading);
    expect(screen.getByText(text("pause-body"))).toBeTruthy();
    expect(screen.getByText(withText(text("pause-kept-body", { count: 4211 })))).toBeTruthy();
    expect(screen.getByText(withText(text("pause-waiting-body", { count: 1789 })))).toBeTruthy();
    expect(screen.getByText(text("pause-note"))).toBeTruthy();
    expect(screen.getByText(text("pause-status-kept"))).toBeTruthy();
    expect(screen.getAllByText(text("pause-status-waiting"))).toHaveLength(2);
    expectClean();
    expect(await violations()).toEqual([]);
  });

  it("while the current file finishes it says so, and is not yet paused", () => {
    inApp(<Paused progress={sample.pausing} onResume={noop} />);
    expect(screen.getByRole("heading", { level: 1 }).textContent).toBe(text("pause-title-finishing"));
    expect(screen.getByText(withText(text("pause-finishing", { name: "Holiday 2024 / IMG_2043.jpg" })))).toBeTruthy();
    expect(screen.queryByText(text("pause-title"))).toBeNull();
    // The file still finishing is not counted as kept or as waiting.
    expect(screen.getByText(withText(text("pause-kept-body", { count: 4210 })))).toBeTruthy();
    expect(screen.getByText(withText(text("pause-waiting-body", { count: 1789 })))).toBeTruthy();
  });

  it("focus moves to the new heading when the pause completes", () => {
    const view = inApp(<Paused progress={sample.pausing} onResume={noop} />);
    view.rerender(
      <LocalizationProvider l10n={l10n}>
        <main>
          <Paused progress={sample.paused} onResume={noop} />
        </main>
      </LocalizationProvider>,
    );
    const heading = screen.getByRole("heading", { level: 1 });
    expect(heading.textContent).toBe(text("pause-title"));
    expect(document.activeElement).toBe(heading);
  });

  it("items that need the person are still counted", () => {
    inApp(<Paused progress={{ ...sample.paused, counts: counts({ done: 10, waiting: 5, needsAction: 2 }) }} onResume={noop} />);
    expect(screen.getByText(withText(text("progress-count-needs-action", { count: 2 })))).toBeTruthy();
  });

  it("its one action, Resume, sits in the footer", () => {
    const onResume = vi.fn();
    inFrame(<Paused progress={sample.paused} onResume={onResume} />);
    expect(screen.getByText(text("progress-approved-job"))).toBeTruthy();
    const footer = footerEl();
    fireEvent.click(within(footer).getByRole("button", { name: text("pause-resume") }));
    expect(onResume).toHaveBeenCalledOnce();
    expect(within(screen.getByRole("main")).queryAllByRole("button").filter((b) => !footer.contains(b))).toEqual([]);
  });

});

describe("X10 Stop the rest of the move?", () => {
  function open(extra: { onStop?: () => void; onKeep?: () => void } = {}) {
    return inApp(<StopMove open onStop={extra.onStop ?? noop} onKeep={extra.onKeep ?? noop} />);
  }

  it("says what stays, what does not get copied and that the old laptop is untouched", async () => {
    open();
    const dialog = screen.getByRole("dialog", { name: text("cancel-title") });
    expect(within(dialog).getByText(text("cancel-body"))).toBeTruthy();
    for (const k of ["cancel-copied-body", "cancel-rest-body", "cancel-partial-body", "cancel-old-body"]) expect(within(dialog).getByText(text(k))).toBeTruthy();
    expectClean();
    expect(await violations()).toEqual([]);
  });

  it("the stop button stays off until the person ticks the box", () => {
    const onStop = vi.fn();
    open({ onStop });
    const stop = screen.getByRole("button", { name: text("cancel-stop") });
    expect((stop as HTMLButtonElement).disabled).toBe(true);
    fireEvent.click(stop);
    expect(onStop).not.toHaveBeenCalled();
    const box = screen.getByRole("checkbox", { name: text("cancel-ack") });
    fireEvent.click(box);
    expect((stop as HTMLButtonElement).disabled).toBe(false);
    fireEvent.click(stop);
    expect(onStop).toHaveBeenCalledOnce();
    // Unticking turns it off again.
    fireEvent.click(box);
    expect((stop as HTMLButtonElement).disabled).toBe(true);
  });

  it("the off button says why", () => {
    open();
    const stop = screen.getByRole("button", { name: text("cancel-stop") });
    const hint = document.getElementById(stop.getAttribute("aria-describedby") ?? "none");
    expect(hint?.textContent).toBe(text("cancel-ack-hint"));
  });

  it("keeps the move going from the button and from Escape", () => {
    const onKeep = vi.fn();
    open({ onKeep });
    fireEvent.click(screen.getByRole("button", { name: text("cancel-keep") }));
    fireEvent.keyDown(screen.getByRole("dialog"), { key: "Escape" });
    expect(onKeep).toHaveBeenCalledTimes(2);
  });

  it("starts on the safe choice, not on stop", () => {
    open();
    expect(document.activeElement).toBe(screen.getByRole("button", { name: text("cancel-keep") }));
  });

  it("forgets the tick each time it opens", () => {
    const ui = (isOpen: boolean) => (
      <LocalizationProvider l10n={l10n}>
        <main>
          <StopMove open={isOpen} onStop={noop} onKeep={noop} />
        </main>
      </LocalizationProvider>
    );
    const view = render(ui(true));
    fireEvent.click(screen.getByRole("checkbox", { name: text("cancel-ack") }));
    view.rerender(ui(false));
    view.rerender(ui(true));
    expect(screen.getByRole<HTMLInputElement>("checkbox", { name: text("cancel-ack") }).checked).toBe(false);
    expect(screen.getByRole<HTMLButtonElement>("button", { name: text("cancel-stop") }).disabled).toBe(true);
  });

  it("the kit dialog takes its own dismiss words", () => {
    inApp(
      <Dialog open title="T" onClose={noop} closeLabel="Keep going">
        body
      </Dialog>,
    );
    expect(screen.getByRole("button", { name: "Keep going" })).toBeTruthy();
    expect(screen.queryByRole("button", { name: text("dialog-close") })).toBeNull();
  });
});

describe("R01 Welcome to your next laptop", () => {
  const props = { onReport: noop };
  const chip = () => screen.getByRole("listitem");

  it("a clean move shows two numbers and one Selected files row marked verified", async () => {
    inApp(<Outcome outcome={sample.finishedClean} {...props} />);
    const heading = screen.getByRole("heading", { level: 1 });
    expect(heading.textContent).toBe(text("outcome-title"));
    expect(document.activeElement).toBe(heading);
    expect(screen.getByText("120 GB")).toBeTruthy();
    expect(screen.getByText(text("outcome-metric-size"))).toBeTruthy();
    expect(screen.getByText("0")).toBeTruthy();
    expect(screen.getByText(text("outcome-metric-action"))).toBeTruthy();
    expect(screen.getAllByRole("listitem")).toHaveLength(1);
    expect(within(chip()).getByText(text("outcome-files-title"))).toBeTruthy();
    expect(within(chip()).getByText(withText(text("outcome-files-summary", { done: 6000, total: 6000 })))).toBeTruthy();
    expect(within(chip()).getByText(text("progress-state-verified"))).toBeTruthy();
    expect(screen.queryByText(text("outcome-status-partial"))).toBeNull();
    expect(screen.queryByText(text("outcome-old-safe"))).toBeNull();
    expectClean();
    expect(await violations()).toEqual([]);
  });

  it("the after-move check: each kind the plan held, with its counts; one short says Partial, a kind not in the plan is left out", async () => {
    inApp(<Outcome outcome={sample.finishedChecked} {...props} />);
    const check = screen.getByRole("region", { name: text("outcome-kinds-title") });
    const kindRow = (k: string) => within(check).getByText(text(`outcome-kind-${k}`)).closest("li") as HTMLElement;
    expect(plain(kindRow("photos").textContent)).toContain(plain(text("outcome-kind-count", { copied: 4208, planned: 4210 })));
    expect(within(kindRow("photos")).getByText(text("outcome-status-partial"))).toBeTruthy();
    expect(within(kindRow("photos")).queryByText(text("progress-state-verified"))).toBeNull();
    expect(within(kindRow("videos")).getByText(text("progress-state-verified"))).toBeTruthy();
    expect(within(kindRow("documents")).getByText(text("progress-state-verified"))).toBeTruthy();
    expect(within(check).queryByText(text("outcome-kind-music"))).toBeNull();
    expectClean();
    expect(await violations()).toEqual([]);
  });

  it("without the after-move check (nothing of those kinds), there is no such section", () => {
    inApp(<Outcome outcome={{ ...sample.finishedClean, afterMove: { photos: { planned: 0, copied: 0 }, videos: { planned: 0, copied: 0 }, music: { planned: 0, copied: 0 }, documents: { planned: 0, copied: 0 } } }} {...props} />);
    expect(screen.queryByText(text("outcome-kinds-title"))).toBeNull();
  });

  it("anything that failed or needs the person is Partial, never verified, and the warning shows", () => {
    inApp(<Outcome outcome={sample.finishedMixed} {...props} />);
    expect(within(chip()).getByText(text("outcome-status-partial"))).toBeTruthy();
    expect(within(chip()).queryByText(text("progress-state-verified"))).toBeNull();
    expect(screen.getByText(text("outcome-old-safe"))).toBeTruthy();
    expect(screen.getByText("3")).toBeTruthy(); // need your action
    expect(within(chip()).getByText(withText(text("outcome-files-summary", { done: 5990, total: 5995 })))).toBeTruthy();
  });

  it("a single failure among many successes still makes it Partial", () => {
    const one = { ...sample.finishedClean, byStatus: { ...sample.finishedClean.byStatus, copied: 99999, failed: 1 } };
    inApp(<Outcome outcome={one} {...props} />);
    expect(within(chip()).getByText(text("outcome-status-partial"))).toBeTruthy();
    expect(within(chip()).queryByText(text("progress-state-verified"))).toBeNull();
  });

  it("has no per-status rows and no list of next steps", () => {
    inApp(<Outcome outcome={sample.finishedMixed} {...props} />);
    expect(screen.getAllByRole("listitem")).toHaveLength(1);
    expect(screen.queryByText(text("outcome-next-title"))).toBeNull();
    expect(screen.queryByText(text("status-failed"))).toBeNull();
  });

  it("a stopped move has the same layout with its own heading and is Partial", () => {
    inApp(<Outcome outcome={sample.finishedStopped} {...props} />);
    expect(screen.getByRole("heading", { level: 1 }).textContent).toBe(text("outcome-stopped-title"));
    expect(screen.getByText(text("outcome-stopped-body"))).toBeTruthy();
    expect(within(chip()).getByText(withText(text("outcome-files-summary", { done: 3100, total: 6000 })))).toBeTruthy();
    expect(within(chip()).getByText(text("outcome-status-partial"))).toBeTruthy();
    expect(screen.queryByText(text("outcome-title"))).toBeNull();
  });

  it("files checked from this laptop's memory are named in the row's own text", () => {
    inApp(<Outcome outcome={sample.finishedMemory} {...props} />);
    expect(within(chip()).getByText(hasText(text("outcome-files-memory", { count: 6000 })))).toBeTruthy();
    expect(within(chip()).getByText(hasText("not the drive"))).toBeTruthy();
    // Still complete: it was checked, and the line says how.
    expect(within(chip()).getByText(text("progress-state-verified"))).toBeTruthy();
  });

  it("no memory line when every file was read back from the drive", () => {
    inApp(<Outcome outcome={sample.finishedClean} {...props} />);
    expect(screen.queryByText(hasText("not the drive"))).toBeNull();
  });

  it("its action, View complete report, sits in the footer, and there is no Back to start", () => {
    const onReport = vi.fn();
    inFrame(<Outcome outcome={sample.finishedMixed} onReport={onReport} />);
    expect(screen.getByText(text("outcome-eyebrow"))).toBeTruthy();
    const footer = footerEl();
    fireEvent.click(within(footer).getByRole("button", { name: text("outcome-report") }));
    expect(onReport).toHaveBeenCalledOnce();
    expect(screen.queryByRole("button", { name: text("outcome-home") })).toBeNull();
    expect(within(screen.getByRole("main")).queryAllByRole("button").filter((b) => !footer.contains(b))).toEqual([]);
  });

  it("every state is accessible, also right to left", async () => {
    document.documentElement.dir = "rtl";
    for (const o of [sample.finishedClean, sample.finishedMixed, sample.finishedStopped, sample.finishedMemory]) {
      const { unmount } = inApp(<Outcome outcome={o} {...props} />);
      expect(await violations()).toEqual([]);
      unmount();
    }
  });
});

describe("R02 Your move, item by item", () => {
  const detailsOf = (row: HTMLElement): HTMLDetailsElement => {
    const el = within(row).getByText(text("report-more-details")).closest<HTMLDetailsElement>("details");
    if (!el) throw new Error("no details element");
    return el;
  };
  const rowOf = (name: string) => screen.getAllByRole("row").find((r) => r.textContent.includes(name)) as HTMLElement;

  it("lists every item with its status chip and its reason", async () => {
    inApp(<Report items={sample.report} onBack={noop} />);
    const heading = screen.getByRole("heading", { level: 1 });
    expect(heading.textContent).toBe(text("report-title"));
    expect(document.activeElement).toBe(heading);
    expect(screen.getAllByRole("row")).toHaveLength(sample.report.length + 1);
    expect(within(rowOf("Invoice March.pdf")).getByText(text("status-copied"))).toBeTruthy();
    expect(within(rowOf("Photo editor")).getByText(text("status-reinstalled"))).toBeTruthy();
    expect(within(rowOf("Old notes.rtf")).getByText(text("status-converted"))).toBeTruthy();
    expect(within(rowOf("Mail export.pst")).getByText(text("status-needs-action"))).toBeTruthy();
    expect(within(rowOf("Games")).getByText(text("status-skipped-by-user"))).toBeTruthy();
    expect(within(rowOf("Archive.zip")).getByText(text("status-failed"))).toBeTruthy();
    expect(within(rowOf("Archive.zip")).getByText(text("reason-not-enough-space"))).toBeTruthy();
    expect(within(rowOf("Mail export.pst")).getByText(text("reason-online-only"))).toBeTruthy();
    // Only items that did not finish carry a reason.
    expect(within(rowOf("Invoice March.pdf")).queryByText(hasText("laptop"))).toBeNull();
    expectClean();
    expect(await violations()).toEqual([]);
  });

  it("a drive read-back and a check from memory are two different lines", () => {
    inApp(<Report items={sample.report} onBack={noop} />);
    const fromDrive = rowOf("Invoice March.pdf");
    const fromMemory = rowOf("Holiday 2024");
    expect(within(fromDrive).getByText(text("evidence-read-back"))).toBeTruthy();
    expect(within(fromDrive).queryByText(text("evidence-memory"))).toBeNull();
    expect(within(fromMemory).getByText(text("evidence-memory"))).toBeTruthy();
    expect(within(fromMemory).queryByText(text("evidence-read-back"))).toBeNull();
    expect(text("evidence-memory")).not.toBe(text("evidence-read-back"));
    expect(text("evidence-memory")).toMatch(/memory, not the drive/);
  });

  it("explains a memory check behind More details, not in the main line", () => {
    inApp(<Report items={sample.report} onBack={noop} />);
    const row = rowOf("Holiday 2024");
    const details = detailsOf(row);
    expect(details.open).toBe(false);
    expect(within(details).getByText(text("evidence-memory-explain"))).toBeTruthy();
    // Under it, why this file was checked from memory, in plain words.
    expect(within(details).getByText(text("why-memory-cannot-skip-cache"))).toBeTruthy();
    // The explanation is not the evidence line itself.
    expect(within(row).getAllByText(text("evidence-memory"))).toHaveLength(1);
    // A file read back from the drive has neither.
    const drive = detailsOf(rowOf("Invoice March.pdf"));
    expect(within(drive).queryByText(text("evidence-memory-explain"))).toBeNull();
    expect(drive.querySelector("[data-why]")).toBeNull();
  });

  it.each(WHY_MEMORY)("a check from memory because of %s says why in its own words", (why) => {
    const item: ReportItem = { id: "m", name: "Notes.txt", source: "Notes.txt", status: "copied", evidence: { from: "memory", why } };
    inApp(<Report items={[item]} onBack={noop} />);
    const details = detailsOf(rowOf("Notes.txt"));
    const line = details.querySelector(`[data-why="${why}"]`);
    expect(line?.textContent).toBe(text(whyMemoryKey(why)));
  });

  it("every reason a check came from memory has its words, different from the others", () => {
    const words = WHY_MEMORY.map((why) => {
      expect(l10n.getBundle(whyMemoryKey(why)), whyMemoryKey(why)).not.toBeNull();
      return text(whyMemoryKey(why));
    });
    expect(new Set(words).size).toBe(WHY_MEMORY.length);
    expect(WHY_MEMORY).toEqual(["cannot-skip-cache", "not-shown", "no-way", "stored-specially", "too-small"]);
  });

  it("shows where it came from and where it went behind More details", () => {
    inApp(<Report items={sample.report} onBack={noop} />);
    const details = detailsOf(rowOf("Invoice March.pdf"));
    const path = String.raw`C:\Users\Ada\Documents\Invoice March.pdf`;
    expect(plain(details.textContent)).toContain(plain(text("report-from", { path })));
    expect(plain(details.textContent)).toContain(plain(text("report-to", { path })));
  });

  it("a file changed since the scan says so beside its result, which stays copied and checked", async () => {
    const changed: ReportItem = { id: "c", name: "Budget.xlsx", source: "Budget.xlsx", status: "copied", evidence: { from: "drive" }, note: "changed-since-scan" };
    const gone: ReportItem = { id: "g", name: "Draft.docx", source: "Draft.docx", status: "failed", reason: "unreadable", note: "no-longer-there" };
    const plainFailed: ReportItem = { id: "u", name: "Locked.pst", source: "Locked.pst", status: "failed", reason: "unreadable" };
    inApp(<Report items={[changed, gone, plainFailed]} onBack={noop} />);
    const row = rowOf("Budget.xlsx");
    expect(within(row).getByText(text("report-note-changed-since-scan"))).toBeTruthy();
    expect(within(row).getByText(text("evidence-read-back"))).toBeTruthy();
    // Gone before it could be sent: that is the why, said in place of "could not be read".
    const goneRow = rowOf("Draft.docx");
    expect(within(goneRow).getByText(text("report-note-no-longer-there"))).toBeTruthy();
    expect(within(goneRow).queryByText(text("reason-unreadable"))).toBeNull();
    expect(within(goneRow).getByText(text("evidence-not-checked"))).toBeTruthy();
    // A line with no note has none.
    expect(rowOf("Locked.pst").querySelector(".mv-note")).toBeNull();
    expect(within(rowOf("Locked.pst")).getByText(text("reason-unreadable"))).toBeTruthy();
    expectClean();
    expect(await violations()).toEqual([]);
  });

  it("an item that did not finish is marked as not checked", () => {
    inApp(<Report items={sample.report} onBack={noop} />);
    expect(within(rowOf("Archive.zip")).getByText(text("evidence-not-checked"))).toBeTruthy();
    expect(within(rowOf("Games")).queryByText(text("evidence-not-checked"))).toBeNull();
    expect(within(rowOf("Games")).queryByText(text("evidence-read-back"))).toBeNull();
  });

  it("has no filter buttons and no count line", () => {
    inApp(<Report items={sample.report} onBack={noop} />);
    expect(screen.queryByRole("group", { name: text("report-filter-label") })).toBeNull();
    expect(screen.queryByRole("status")).toBeNull();
    expect(screen.queryByText(hasText("not the drive."), { selector: "p.eyebrow, .kit-banner p" })).toBeNull();
  });

  it("has the move-results strip with Back and no footer action", () => {
    const onBack = vi.fn();
    inFrame(<Report items={sample.report} onBack={onBack} />);
    expect(screen.getByText(text("outcome-eyebrow"))).toBeTruthy();
    fireEvent.click(screen.getByRole("button", { name: text("frame-back") }));
    expect(onBack).toHaveBeenCalledOnce();
    expect(within(footerEl()).getByText(text("footer-free"))).toBeTruthy();
  });

  it("a report with no items says so", () => {
    inApp(<Report items={[]} onBack={noop} />);
    expect(screen.getByText(text("report-none"))).toBeTruthy();
  });

  it("shows a long report a page at a time and says how much is shown", () => {
    inApp(<Report items={sample.longReport(250)} onBack={noop} />);
    // (Counting rows straight from the page: asking for them by role is slow with hundreds.)
    const rows = () => document.querySelectorAll("tbody tr").length;
    expect(rows()).toBe(100);
    expect(screen.getByText(withText(text("report-showing", { shown: 100, total: 250 })))).toBeTruthy();
    fireEvent.click(screen.getByText(text("report-show-more")));
    expect(rows()).toBe(200);
    fireEvent.click(screen.getByText(text("report-show-more")));
    expect(rows()).toBe(250);
    expect(screen.queryByText(text("report-show-more"))).toBeNull();
  });

  it("goes back", () => {
    const onBack = vi.fn();
    inApp(<Report items={sample.report} onBack={onBack} />);
    fireEvent.click(screen.getByRole("button", { name: text("frame-back") }));
    expect(onBack).toHaveBeenCalledOnce();
  });

  it("is accessible, also right to left", async () => {
    document.documentElement.dir = "rtl";
    inApp(<Report items={sample.report} onBack={noop} />);
    expect(await violations()).toEqual([]);
  });
});

describe("the kit's result chip", () => {
  it("is called skipped-by-user, as the engine calls it", async () => {
    const { ResultChip } = await import("../components/kit");
    inApp(<ResultChip result="skipped-by-user" />);
    expect(screen.getByText(text("status-skipped-by-user"))).toBeTruthy();
    expectClean();
  });
});
