import { LocalizationProvider } from "@fluent/react";
import type { ReactNode } from "react";
import { cleanup, fireEvent, render, screen, within } from "@testing-library/react";
import axe from "axe-core";
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
  parseResult,
  statusOfReason,
  type Counts,
  type ItemResult,
  type MoveProgress,
  type ReportItem,
} from "../move";
import * as sample from "../moveSamples";
import { counts } from "../moveSamples";
import MiniPanel from "./MiniPanel";
import Outcome from "./Outcome";
import Paused from "./Paused";
import ProgressCompact from "./ProgressCompact";
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

async function violations(): Promise<string[]> {
  const result = await axe.run(document.body, { rules: { "color-contrast": { enabled: false } } });
  return result.violations.map((v) => `${v.id}: ${v.nodes.map((n) => n.target.join(" ")).join(", ")}`);
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
    inApp(<ProgressCompact progress={sample.running} onPause={noop} onResume={noop} onExpand={noop} onMinimise={noop} onStop={noop} onSeeNeedsAction={noop} />);
    const heading = screen.getByRole("heading", { level: 1 });
    expect(heading.textContent).toBe(text("progress-title"));
    expect(document.activeElement).toBe(heading);
    const bar = screen.getByRole("progressbar", { name: attribute("progress-percent", "aria-label") });
    expect(bar.getAttribute("aria-valuenow")).toBe("68"); // 81.6 of 120 GB
    expect(screen.getByText(hasText("81.6 GB of 120 GB"))).toBeTruthy();
    // The route: both laptops by the names pairing gave them, then each group with its own state.
    expect(screen.getByText("Ada’s old laptop")).toBeTruthy();
    expect(screen.getByText(withText(text("label-picked", { colour: text("label-colour-blue"), animal: text("label-animal-fox") })))).toBeTruthy();
    expect(screen.getByText(text("progress-route-old"))).toBeTruthy();
    expect(screen.getByText(text("progress-paired"))).toBeTruthy();
    expect(within(groupRow("Documents")).getByText(text("progress-state-verified"))).toBeTruthy();
    expect(within(groupRow("Pictures")).getByText(text("progress-state-checking"))).toBeTruthy();
    expect(within(groupRow("Tax 2023")).getByText(text("progress-state-waiting"))).toBeTruthy();
    expect(screen.getByText(withText(text("progress-current-copying", { name: "Holiday 2024 / IMG_2043.jpg" })))).toBeTruthy();
    expect(screen.getByText(text("progress-old-safe"))).toBeTruthy();
    expectClean();
    expect(await violations()).toEqual([]);
  });

  it("time left and speed are ranges, and only when measured", () => {
    const { unmount } = inApp(<ProgressCompact progress={sample.running} onPause={noop} onResume={noop} onExpand={noop} onMinimise={noop} onStop={noop} onSeeNeedsAction={noop} />);
    const range = text("progress-minutes", { low: 15, high: 20 });
    expect(screen.getByText(withText(text("progress-time-left", { range })))).toBeTruthy();
    expect(screen.getByText(withText(text("progress-speed", { speed: text("progress-speed-range", { low: 58, high: 66, unit: "MB/s" }) })))).toBeTruthy();
    unmount();
    inApp(<ProgressCompact progress={sample.starting} onPause={noop} onResume={noop} onExpand={noop} onMinimise={noop} onStop={noop} onSeeNeedsAction={noop} />);
    expect(screen.getByText(text("progress-time-measuring"))).toBeTruthy();
    expect(screen.queryByText(hasText("MB/s"))).toBeNull();
    expect(screen.queryByText(hasText("About"))).toBeNull();
    expect(document.querySelector(".mv-speed")).toBeNull();
  });

  it("a range of one number still reads as a range", () => {
    const p: MoveProgress = { ...sample.running, minutesLeft: { low: 7, high: 7 } };
    inApp(<ProgressCompact progress={p} onPause={noop} onResume={noop} onExpand={noop} onMinimise={noop} onStop={noop} onSeeNeedsAction={noop} />);
    expect(screen.getByText(hasText(text("progress-minutes", { low: 7, high: 8 })))).toBeTruthy();
  });

  it("never says 100 or done while anything is still being checked", () => {
    inApp(<ProgressCompact progress={sample.almost} onPause={noop} onResume={noop} onExpand={noop} onMinimise={noop} onStop={noop} onSeeNeedsAction={noop} />);
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
    inApp(<ProgressCompact progress={p} onPause={noop} onResume={noop} onExpand={noop} onMinimise={noop} onStop={noop} onSeeNeedsAction={noop} />);
    expect(screen.getByRole("progressbar").hasAttribute("aria-valuenow")).toBe(false);
    expect(screen.getByText(text("progress-percent-unknown"))).toBeTruthy();
    expect(document.body.textContent).not.toMatch(/\d\s*%/);
  });

  it("items that need the person are counted apart and do not stop the rest", () => {
    const onSee = vi.fn();
    inApp(<ProgressCompact progress={sample.withNeedsAction} onPause={noop} onResume={noop} onExpand={noop} onMinimise={noop} onStop={noop} onSeeNeedsAction={onSee} />);
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
    inApp(<ProgressCompact progress={sample.withFailures} onPause={noop} onResume={noop} onExpand={noop} onMinimise={noop} onStop={noop} onSeeNeedsAction={noop} />);
    const pictures = groupRow("Pictures");
    expect(within(pictures).getByText(withText(text("progress-count-failed", { count: 1 })))).toBeTruthy();
    // Still being worked on, so its state is a working one; the failure is counted under its name.
    expect(within(pictures).getByText(text("progress-state-checking"))).toBeTruthy();
  });

  it("no line for needs-action or failed when there are none", () => {
    inApp(<ProgressCompact progress={sample.running} onPause={noop} onResume={noop} onExpand={noop} onMinimise={noop} onStop={noop} onSeeNeedsAction={noop} />);
    expect(screen.queryByText(hasText("need your action"))).toBeNull();
    expect(screen.queryByText(hasText("could not be copied"))).toBeNull();
  });

  it("says when the connection dropped, and when a laptop is low on battery", () => {
    const { unmount } = inApp(<ProgressCompact progress={sample.linkLost} onPause={noop} onResume={noop} onExpand={noop} onMinimise={noop} onStop={noop} onSeeNeedsAction={noop} />);
    expect(screen.getByText(text("progress-interrupted"))).toBeTruthy();
    unmount();
    inApp(<ProgressCompact progress={sample.lowBattery} onPause={noop} onResume={noop} onExpand={noop} onMinimise={noop} onStop={noop} onSeeNeedsAction={noop} />);
    expect(screen.getByText(withText(text("progress-low-battery", { label: "Blue Fox" })))).toBeTruthy();
  });

  it("its buttons do what they say", () => {
    const h = { onPause: vi.fn(), onResume: vi.fn(), onExpand: vi.fn(), onMinimise: vi.fn(), onStop: vi.fn() };
    inApp(<ProgressCompact progress={sample.running} {...h} onSeeNeedsAction={noop} />);
    fireEvent.click(screen.getByRole("button", { name: text("progress-pause") }));
    fireEvent.click(screen.getByRole("button", { name: text("progress-expand") }));
    fireEvent.click(screen.getByRole("button", { name: text("progress-compact") }));
    fireEvent.click(screen.getByRole("button", { name: text("progress-stop") }));
    expect(h.onPause).toHaveBeenCalledOnce();
    expect(h.onExpand).toHaveBeenCalledOnce();
    expect(h.onMinimise).toHaveBeenCalledOnce();
    expect(h.onStop).toHaveBeenCalledOnce();
    expect(h.onResume).not.toHaveBeenCalled();
    expect(screen.queryByRole("button", { name: text("progress-resume") })).toBeNull();
  });

  it("a paused move shows the paused screen, not a running one", () => {
    inApp(<ProgressCompact progress={sample.paused} onPause={noop} onResume={noop} onExpand={noop} onMinimise={noop} onStop={noop} onSeeNeedsAction={noop} />);
    expect(screen.getByRole("heading", { level: 1 }).textContent).toBe(text("pause-title"));
    expect(screen.queryByRole("button", { name: text("progress-pause") })).toBeNull();
  });

  it("works right to left", async () => {
    document.documentElement.dir = "rtl";
    inApp(<ProgressCompact progress={sample.withFailures} onPause={noop} onResume={noop} onExpand={noop} onMinimise={noop} onStop={noop} onSeeNeedsAction={noop} />);
    expect(await violations()).toEqual([]);
  });
});

describe("X02 Move progress, expanded", () => {
  const props = { progress: sample.running, queue: sample.queue, onPause: noop, onResume: noop, onCollapse: noop, onStop: noop, onSeeNeedsAction: noop };

  it("lists every item with its own state, source and status in words", async () => {
    inApp(<ProgressDetail {...props} />);
    const heading = screen.getByRole("heading", { level: 1 });
    expect(heading.textContent).toBe(text("progress-all-title"));
    expect(document.activeElement).toBe(heading);
    const table = screen.getByRole("table");
    const rows = within(table).getAllByRole("row");
    expect(rows).toHaveLength(sample.queue.length + 1); // plus the header row
    const row = (name: string) => rows.find((r) => r.textContent.includes(name)) as HTMLElement;
    expect(within(row("IMG_2043.jpg")).getByText(text("progress-state-copying"))).toBeTruthy();
    expect(within(row("IMG_2042.jpg")).getByText(text("progress-state-checking"))).toBeTruthy();
    expect(within(row("Budget.xlsx")).getByText(text("progress-state-placing"))).toBeTruthy();
    expect(within(row("Notes.txt")).getByText(text("progress-state-existing"))).toBeTruthy();
    expect(within(row("Wedding video.mp4")).getByText(text("progress-state-waiting"))).toBeTruthy();
    expect(within(row("Invoice March.pdf")).getByText(text("status-copied"))).toBeTruthy();
    expect(within(row("Archive.zip")).getByText(text("status-failed"))).toBeTruthy();
    expect(within(row("Mail export.pst")).getByText(text("status-needs-action"))).toBeTruthy();
    expect(screen.getByText(text("progress-checked-note"))).toBeTruthy();
    expectClean();
    expect(await violations()).toEqual([]);
  });

  it("an item that did not finish says why and what to do", () => {
    inApp(<ProgressDetail {...props} />);
    expect(screen.getByText(text("reason-not-enough-space"))).toBeTruthy();
    expect(screen.getByText(text("reason-online-only"))).toBeTruthy();
  });

  it("an item is called copied only once it has ended as copied", () => {
    inApp(<ProgressDetail {...props} />);
    expect(screen.getAllByText(text("status-copied"))).toHaveLength(1);
  });

  it("waiting items read Paused while the move is paused", () => {
    inApp(<ProgressDetail {...props} progress={sample.paused} />);
    const row = screen.getAllByRole("row").find((r) => r.textContent.includes("Wedding video.mp4")) as HTMLElement;
    expect(within(row).getByText(text("progress-state-paused"))).toBeTruthy();
    expect(within(row).queryByText(text("progress-state-waiting"))).toBeNull();
  });

  it("says so when nothing has started", () => {
    inApp(<ProgressDetail {...props} queue={[]} />);
    expect(screen.getByText(text("progress-queue-empty"))).toBeTruthy();
    expect(screen.queryByRole("table")).toBeNull();
  });

  it("can go back to the compact view and still pause or stop", () => {
    const h = { onCollapse: vi.fn(), onPause: vi.fn(), onStop: vi.fn() };
    inApp(<ProgressDetail {...props} {...h} />);
    fireEvent.click(screen.getByRole("button", { name: text("report-back") }));
    fireEvent.click(screen.getByRole("button", { name: text("progress-pause") }));
    fireEvent.click(screen.getByRole("button", { name: text("progress-stop") }));
    expect(h.onCollapse).toHaveBeenCalledOnce();
    expect(h.onPause).toHaveBeenCalledOnce();
    expect(h.onStop).toHaveBeenCalledOnce();
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
    inApp(<Paused progress={sample.paused} onResume={noop} onStop={noop} />);
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
    inApp(<Paused progress={sample.pausing} onResume={noop} onStop={noop} />);
    expect(screen.getByRole("heading", { level: 1 }).textContent).toBe(text("pause-title-finishing"));
    expect(screen.getByText(withText(text("pause-finishing", { name: "Holiday 2024 / IMG_2043.jpg" })))).toBeTruthy();
    expect(screen.queryByText(text("pause-title"))).toBeNull();
    // The file still finishing is not counted as kept or as waiting.
    expect(screen.getByText(withText(text("pause-kept-body", { count: 4210 })))).toBeTruthy();
    expect(screen.getByText(withText(text("pause-waiting-body", { count: 1789 })))).toBeTruthy();
  });

  it("focus moves to the new heading when the pause completes", () => {
    const view = inApp(<Paused progress={sample.pausing} onResume={noop} onStop={noop} />);
    view.rerender(
      <LocalizationProvider l10n={l10n}>
        <main>
          <Paused progress={sample.paused} onResume={noop} onStop={noop} />
        </main>
      </LocalizationProvider>,
    );
    const heading = screen.getByRole("heading", { level: 1 });
    expect(heading.textContent).toBe(text("pause-title"));
    expect(document.activeElement).toBe(heading);
  });

  it("items that need the person are still counted", () => {
    inApp(<Paused progress={{ ...sample.paused, counts: counts({ done: 10, waiting: 5, needsAction: 2 }) }} onResume={noop} onStop={noop} />);
    expect(screen.getByText(withText(text("progress-count-needs-action", { count: 2 })))).toBeTruthy();
  });

  it("continues or stops", () => {
    const h = { onResume: vi.fn(), onStop: vi.fn() };
    inApp(<Paused progress={sample.paused} {...h} />);
    fireEvent.click(screen.getByRole("button", { name: text("pause-resume") }));
    fireEvent.click(screen.getByRole("button", { name: text("pause-stop") }));
    expect(h.onResume).toHaveBeenCalledOnce();
    expect(h.onStop).toHaveBeenCalledOnce();
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
  const props = { onReport: noop, onHome: noop };

  it("a clean move says every item was copied and checked, with the size", async () => {
    inApp(<Outcome outcome={sample.finishedClean} {...props} />);
    const heading = screen.getByRole("heading", { level: 1 });
    expect(heading.textContent).toBe(text("outcome-title"));
    expect(document.activeElement).toBe(heading);
    expect(screen.getByText(text("outcome-all-done"))).toBeTruthy();
    expect(screen.getByText("120 GB")).toBeTruthy();
    expect(screen.getByText(text("outcome-metric-size"))).toBeTruthy();
    expect(screen.getByText("6,000")).toBeTruthy();
    expect(screen.getByText(withText(text("outcome-copied", { count: 6000 })))).toBeTruthy();
    expect(screen.getByText(text("outcome-old-safe"))).toBeTruthy();
    expect(screen.queryByText(hasText("did not finish"))).toBeNull();
    expectClean();
    expect(await violations()).toEqual([]);
  });

  it("a mixed move shows each status with its own count and never says everything worked", () => {
    inApp(<Outcome outcome={sample.finishedMixed} {...props} />);
    expect(screen.queryByText(text("outcome-all-done"))).toBeNull();
    expect(screen.getByText(withText(text("outcome-copied", { count: 5900 })))).toBeTruthy();
    expect(screen.getByText(withText(text("outcome-converted", { count: 90 })))).toBeTruthy();
    expect(screen.getByText(withText(text("outcome-needs-action", { count: 3 })))).toBeTruthy();
    expect(screen.getByText(withText(text("outcome-failed", { count: 2 })))).toBeTruthy();
    expect(screen.getByText(withText(text("outcome-skipped-by-user", { count: 5 })))).toBeTruthy();
    expect(screen.getByText(withText(text("outcome-unfinished", { count: 5 })))).toBeTruthy();
    expect(screen.getByText(text("outcome-next-needs-action"))).toBeTruthy();
    expect(screen.getByText(text("outcome-next-failed"))).toBeTruthy();
    // Each count sits beside its status word.
    expect(screen.getByText(text("status-failed"))).toBeTruthy();
    expect(screen.getByText(text("status-needs-action"))).toBeTruthy();
  });

  it("leaves out the statuses nothing ended as", () => {
    inApp(<Outcome outcome={sample.finishedClean} {...props} />);
    expect(screen.queryByText(text("status-failed"))).toBeNull();
    expect(screen.queryByText(text("status-needs-action"))).toBeNull();
    expect(screen.queryByText(text("outcome-next-needs-action"))).toBeNull();
    expect(screen.queryByText(text("outcome-next-failed"))).toBeNull();
  });

  it("a single failure is not hidden by a long list of successes", () => {
    const one = { ...sample.finishedClean, byStatus: { ...sample.finishedClean.byStatus, copied: 99999, failed: 1 } };
    inApp(<Outcome outcome={one} {...props} />);
    expect(screen.queryByText(text("outcome-all-done"))).toBeNull();
    expect(screen.getByText(withText(text("outcome-failed", { count: 1 })))).toBeTruthy();
  });

  it("a move stopped by the person says so and counts what was not copied", () => {
    inApp(<Outcome outcome={sample.finishedStopped} {...props} />);
    expect(screen.getByRole("heading", { level: 1 }).textContent).toBe(text("outcome-stopped-title"));
    expect(screen.getByText(text("outcome-stopped-body"))).toBeTruthy();
    expect(screen.getByText(withText(text("outcome-not-copied", { count: 2900 })))).toBeTruthy();
    expect(screen.queryByText(text("outcome-all-done"))).toBeNull();
    expect(screen.queryByText(text("outcome-title"))).toBeNull();
  });

  it("files checked from this laptop's memory are called out in their own line", () => {
    inApp(<Outcome outcome={sample.finishedMemory} {...props} />);
    expect(screen.getByText(withText(text("outcome-checked-memory", { count: 6000 })))).toBeTruthy();
    expect(screen.getByText(hasText("not the drive"))).toBeTruthy();
  });

  it("no memory line when every file was read back from the drive", () => {
    inApp(<Outcome outcome={sample.finishedClean} {...props} />);
    expect(screen.queryByText(hasText("not the drive"))).toBeNull();
  });

  it("opens the report or goes home", () => {
    const h = { onReport: vi.fn(), onHome: vi.fn() };
    inApp(<Outcome outcome={sample.finishedMixed} {...h} />);
    fireEvent.click(screen.getByRole("button", { name: text("outcome-report") }));
    fireEvent.click(screen.getByRole("button", { name: text("outcome-home") }));
    expect(h.onReport).toHaveBeenCalledOnce();
    expect(h.onHome).toHaveBeenCalledOnce();
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
    // The explanation is not the evidence line itself.
    expect(within(row).getAllByText(text("evidence-memory"))).toHaveLength(1);
  });

  it("shows where it came from and where it went behind More details", () => {
    inApp(<Report items={sample.report} onBack={noop} />);
    const details = detailsOf(rowOf("Invoice March.pdf"));
    const path = String.raw`C:\Users\Ada\Documents\Invoice March.pdf`;
    expect(plain(details.textContent)).toContain(plain(text("report-from", { path })));
    expect(plain(details.textContent)).toContain(plain(text("report-to", { path })));
  });

  it("an item that did not finish is marked as not checked", () => {
    inApp(<Report items={sample.report} onBack={noop} />);
    expect(within(rowOf("Archive.zip")).getByText(text("evidence-not-checked"))).toBeTruthy();
    expect(within(rowOf("Games")).queryByText(text("evidence-not-checked"))).toBeNull();
    expect(within(rowOf("Games")).queryByText(text("evidence-read-back"))).toBeNull();
  });

  it("summarises how many files were checked from memory", () => {
    inApp(<Report items={sample.report} onBack={noop} />);
    expect(screen.getByText(withText(text("report-memory-summary", { count: 1 })))).toBeTruthy();
  });

  it("no memory summary when every check read the drive", () => {
    inApp(<Report items={sample.report.filter((r) => r.id !== "r2")} onBack={noop} />);
    expect(screen.queryByText(hasText("not the drive."))).toBeNull();
    expect(screen.queryByText(text("evidence-memory"))).toBeNull();
  });

  it("filters by status with buttons that say which is on", () => {
    inApp(<Report items={sample.report} onBack={noop} />);
    const group = screen.getByRole("group", { name: text("report-filter-label") });
    const all = within(group).getByRole("button", { name: hasText(text("report-filter-all")) });
    expect(all.getAttribute("aria-pressed")).toBe("true");
    const failed = within(group).getByRole("button", { name: hasText(text("status-failed")) });
    fireEvent.click(failed);
    expect(failed.getAttribute("aria-pressed")).toBe("true");
    expect(all.getAttribute("aria-pressed")).toBe("false");
    expect(screen.getAllByRole("row")).toHaveLength(2);
    expect(screen.getByText("Archive.zip")).toBeTruthy();
    expect(screen.queryByText("Invoice March.pdf")).toBeNull();
    fireEvent.click(all);
    expect(screen.getAllByRole("row")).toHaveLength(sample.report.length + 1);
  });

  it("each filter button shows how many items it has", () => {
    inApp(<Report items={sample.report} onBack={noop} />);
    const group = screen.getByRole("group", { name: text("report-filter-label") });
    const copied = within(group).getByRole("button", { name: hasText(text("status-copied")) });
    expect(copied.textContent).toContain("2");
    const all = within(group).getByRole("button", { name: hasText(text("report-filter-all")) });
    expect(all.textContent).toContain("7");
  });

  it("a filter with no items says so, and the count is announced", () => {
    inApp(<Report items={sample.report.filter((r) => r.status !== "failed")} onBack={noop} />);
    fireEvent.click(screen.getByRole("button", { name: hasText(text("status-failed")) }));
    expect(screen.getByText(text("report-none"))).toBeTruthy();
    expect(screen.queryByRole("table")).toBeNull();
    expect(plain(screen.getByRole("status").textContent)).toContain(plain(text("report-count", { count: 0 })));
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

  it("changing the filter starts again at the first page", () => {
    const items = [...sample.longReport(150), ...sample.report.filter((r) => r.status === "failed")];
    inApp(<Report items={items} onBack={noop} />);
    const rows = () => document.querySelectorAll("tbody tr").length;
    fireEvent.click(screen.getByText(text("report-show-more")));
    expect(rows()).toBe(151);
    fireEvent.click(screen.getByRole("button", { name: hasText(text("status-failed")) }));
    fireEvent.click(screen.getByRole("button", { name: hasText(text("report-filter-all")) }));
    expect(rows()).toBe(100);
  });

  it("goes back", () => {
    const onBack = vi.fn();
    inApp(<Report items={sample.report} onBack={onBack} />);
    fireEvent.click(screen.getByRole("button", { name: text("report-back") }));
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
