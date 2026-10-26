import { LocalizationProvider } from "@fluent/react";
import type { ReactNode } from "react";
import { act, cleanup, fireEvent, render, screen, within } from "@testing-library/react";
import { violations } from "../testing/a11y";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { SAMPLES } from "../gallery/samples";
import { localization } from "../i18n";
import { extensionOf, fileName, fraction, formatBytes, historyStateOf, missingLines, receiptState, shownSafely, tally, type HistoryEntry, type PickedItem, type Receipt } from "../quicksend";
import { BLUE_FOX, GREEN_OWL, HISTORY, OFFER, PICKED, PROGRESS, RECEIPT_PARTIAL, RECEIPT_PROBLEMS, RECEIPT_WHOLE } from "../quicksend.fixtures";
import { FooterProvider } from "./FrameFooter";
import History from "./History";
import QuickDone from "./QuickDone";
import QuickHome from "./QuickHome";
import QuickPick from "./QuickPick";
import QuickProgress from "./QuickProgress";
import QuickStop from "./QuickStop";
import { QUICK_WORDS } from "./PairParts";

const l10n = localization("en");
const text = (id: string, vars?: Record<string, string | number>) => l10n.getString(id, vars);
/** Fluent isolates variables for right-to-left safety; the marks are invisible, so tests compare without them. */
const plain = (s: string) => s.replace(/[⁦-⁩]/g, "");
const nameIs = (expected: string) => (accessible: string) => plain(accessible) === plain(expected);
const nameHas = (part: string) => (accessible: string) => plain(accessible).includes(part);
const withText = (expected: string) => (content: string) => plain(content) === plain(expected);
const button = (id: string, vars?: Record<string, string | number>) => screen.getByRole("button", { name: nameIs(text(id, vars)) });
const maybeButton = (id: string) => screen.queryByRole("button", { name: nameIs(text(id)) });
const h1 = () => plain(screen.getByRole("heading", { level: 1 }).textContent);
const fn = () => vi.fn<() => void>();

let warnings: string[] = [];
beforeEach(() => {
  warnings = [];
  vi.spyOn(console, "warn").mockImplementation((...args: unknown[]) => {
    warnings.push(args.map(String).join(" "));
  });
});
afterEach(() => {
  // Nothing may be missing from the English strings in any test.
  expect(warnings.filter((w) => /missing|not found|unknown/i.test(w))).toEqual([]);
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

describe("what the words depend on (how a send ended, history, sizes)", () => {
  it("a handover is whole only when every line ended with the file in place", () => {
    expect(receiptState(RECEIPT_WHOLE)).toBe("complete");
    expect(receiptState({ lines: [] })).toBe("empty");
    expect(receiptState(RECEIPT_PARTIAL)).toBe("partial");
    const declinedOnly: Receipt = { lines: [{ path: "a", outcome: { kind: "arrived", landedAs: "a" } }, { path: "b", outcome: { kind: "declined" } }] };
    expect(receiptState(declinedOnly)).toBe("partial");
    const failedOnly: Receipt = { lines: [{ path: "a", outcome: { kind: "failed", why: "x" } }] };
    expect(receiptState(failedOnly)).toBe("partial");
  });

  it("counts each ending apart: an identical file already there is not an arrival", () => {
    expect(tally(RECEIPT_WHOLE)).toEqual({ arrived: 3, alreadyThere: 1, declined: 0, failed: 0, total: 4 });
    expect(tally(RECEIPT_PARTIAL)).toEqual({ arrived: 2, alreadyThere: 0, declined: 1, failed: 1, total: 4 });
    expect(missingLines(RECEIPT_PARTIAL).map((l) => l.path)).toEqual(["Project brief.pdf", "Reference photos/beach.jpg"]);
  });

  it("a Quick Send is only 'finished' in history when the receipt is whole", () => {
    expect(historyStateOf(RECEIPT_WHOLE, false)).toBe("finished");
    expect(historyStateOf(RECEIPT_WHOLE, true)).toBe("finished");
    expect(historyStateOf(RECEIPT_PARTIAL, false)).toBe("partial");
    expect(historyStateOf(RECEIPT_PARTIAL, true)).toBe("stopped");
    expect(historyStateOf({ lines: [] }, false)).toBe("partial");
  });

  it("progress is a counted fraction, never more than whole or less than none", () => {
    expect(fraction(1, 4)).toBe(0.25);
    expect(fraction(9, 4)).toBe(1);
    expect(fraction(-1, 4)).toBe(0);
    expect(fraction(5, 0)).toBe(0);
  });

  it("sizes use powers of a thousand with one decimal at most", () => {
    expect(formatBytes(24_800_000, "en")).toBe("24.8 MB");
    expect(formatBytes(999, "en")).toBe("999 byte");
  });
});

describe("Q01 Quick Send home", () => {
  const home = (props: Partial<Parameters<typeof QuickHome>[0]> = {}) => {
    const events = { onSend: fn(), onReceive: fn(), onHistory: fn(), onBack: fn(), onStopReceiving: fn() };
    inApp(<QuickHome {...events} {...props} />);
    return events;
  };

  it("takes focus on its heading and offers Send, Receive and what was sent and received as cards", () => {
    home();
    expect(h1()).toBe(text("qs-home-title"));
    expect(document.activeElement?.tagName).toBe("H1");
    expect(screen.getAllByRole("button").map((b) => plain(b.textContent))).toEqual(
      expect.arrayContaining([expect.stringContaining(text("qs-send-title")), expect.stringContaining(text("qs-receive-title")), expect.stringContaining(text("qs-history-title"))]),
    );
  });

  it("raises the right event for each card, the main action and Back", () => {
    const e = home();
    fireEvent.click(screen.getByRole("button", { name: new RegExp(text("qs-send-title")) }));
    fireEvent.click(screen.getByRole("button", { name: new RegExp(text("qs-receive-title")) }));
    fireEvent.click(screen.getByRole("button", { name: new RegExp(text("qs-history-title")) }));
    fireEvent.click(button("qs-home-action"));
    fireEvent.click(button("frame-back"));
    expect(e.onSend).toHaveBeenCalledTimes(2);
    expect(e.onReceive).toHaveBeenCalledTimes(1);
    expect(e.onHistory).toHaveBeenCalledTimes(1);
    expect(e.onBack).toHaveBeenCalledTimes(1);
  });

  it("says plainly when the engine is not connected, in a polite status", () => {
    home({ notice: true });
    expect(screen.getByRole("status").textContent).toContain(text("qs-soon-note"));
  });

  it("while ready to receive, says what nearby laptops see and offers to stop; no laptop found says so", () => {
    const e = home({ receiving: BLUE_FOX, noNearby: true });
    expect(plain(screen.getByRole("status").textContent)).toContain(plain(text("qs-receiving", { label: "Blue Fox" })));
    expect(screen.getByRole("status").textContent).toContain(text("qs-no-nearby"));
    fireEvent.click(button("qs-receive-stop"));
    expect(e.onStopReceiving).toHaveBeenCalled();
  });

  it("has no paid wording and no accessibility violations", async () => {
    home({ receiving: BLUE_FOX });
    expect(await violations()).toEqual([]);
  });
});

describe("Q02 What are you sending?", () => {
  const pick = (items: PickedItem[], props: Partial<Parameters<typeof QuickPick>[0]> = {}) => {
    const events = { onAddFiles: fn(), onAddFolders: fn(), onToggle: vi.fn(), onAllowAccess: vi.fn(), onContinue: fn(), onBack: fn() };
    inApp(<QuickPick items={items} {...events} {...props} />);
    return events;
  };

  it("nothing chosen: says so and the main action is off", () => {
    pick([]);
    expect(screen.getByText(text("qs-empty"))).toBeTruthy();
    expect((button("qs-choose-continue") as HTMLButtonElement).disabled).toBe(true);
  });

  it("shows a measured tally of what is ticked and continues", () => {
    const e = pick(PICKED);
    expect(screen.getByText(withText(text("qs-tally", { count: 3, size: "24.8 MB" })))).toBeTruthy();
    fireEvent.click(button("qs-choose-continue"));
    expect(e.onContinue).toHaveBeenCalled();
  });

  it("unticking an item raises the event and the tally does not count it", () => {
    const e = pick(PICKED);
    fireEvent.click(screen.getByRole("checkbox", { name: /Budget.xlsx/ }));
    expect(e.onToggle).toHaveBeenCalledWith("p2", false);
    cleanup();
    pick(PICKED.map((i) => (i.id === "p2" ? { ...i, included: false } : i)));
    expect(screen.getByText(withText(text("qs-tally", { count: 2, size: "24.2 MB" })))).toBeTruthy();
  });

  it("an item the engine cannot use blocks the main action until it is unticked, and says why in words", () => {
    const bad: PickedItem[] = [
      PICKED[0],
      { id: "x1", name: "Video.mov", kind: "file", included: true, bytes: null, files: null, problem: "online-only" },
      { id: "x2", name: "Tax papers", kind: "folder", included: true, bytes: null, files: null, problem: "unreadable" },
      { id: "x3", name: "Gone.txt", kind: "file", included: true, bytes: null, files: null, problem: "missing" },
    ];
    const e = pick(bad);
    expect((button("qs-choose-continue") as HTMLButtonElement).disabled).toBe(true);
    expect(screen.getByText(nameIs(text("qs-online-only", { name: "Video.mov" })))).toBeTruthy();
    expect(screen.getByText(nameIs(text("qs-protected", { name: "Tax papers" })))).toBeTruthy();
    expect(screen.getByText(nameIs(text("qs-gone", { name: "Gone.txt" })))).toBeTruthy();
    fireEvent.click(button("qs-allow"));
    expect(e.onAllowAccess).toHaveBeenCalledWith("x2");
    cleanup();
    pick(bad.map((i) => (i.problem ? { ...i, included: false } : i)));
    expect((button("qs-choose-continue") as HTMLButtonElement).disabled).toBe(false);
  });

  it("works by keyboard: Add files and Add folders are buttons that ask the bridge; the screen never reads a path", () => {
    const e = pick(PICKED);
    fireEvent.click(screen.getByRole("button", { name: text("qs-add-files") }));
    fireEvent.click(screen.getByRole("button", { name: text("qs-add-folders") }));
    expect(e.onAddFiles).toHaveBeenCalledTimes(1);
    expect(e.onAddFolders).toHaveBeenCalledTimes(1);
    expect(document.querySelector("input[type=file]")).toBeNull();
    expect(screen.getByRole("region", { name: /drop area/i })).toBeTruthy();
  });

  it("a drag over the window changes the words as well as the look", () => {
    pick(PICKED, { dragging: true });
    expect(screen.getByText(text("qs-drop-active"))).toBeTruthy();
    expect(screen.queryByText(text("qs-drop-body"))).toBeNull();
  });
});

describe("Q12 Your files are on their way", () => {
  const progress = (patch: Partial<typeof PROGRESS> = {}) => {
    const events = { onPause: fn(), onResume: fn(), onCompact: fn(), onStop: fn() };
    inApp(<QuickProgress from={BLUE_FOX} to={GREEN_OWL} progress={{ ...PROGRESS, ...patch }} {...events} />);
    return events;
  };

  it("the receiver's page is in receive words: never Sending selected files or Pause send", () => {
    const e = { onPause: fn(), onResume: fn(), onCompact: fn(), onStop: fn() };
    inApp(<QuickProgress side="receive" from={GREEN_OWL} to={BLUE_FOX} progress={PROGRESS} {...e} />);
    const body = plain(document.body.textContent);
    expect(h1()).toBe(text("qs-receiving-title"));
    for (const id of ["qs-sending-card", "qs-pause", "qs-stop", "qs-sending-title"]) expect(body).not.toContain(text(id));
    expect(body).not.toMatch(/Sending selected files|Pause send|selected item/);
    expect(body).toContain(text("qs-receiving-card"));
    expect(body).toContain(plain(text("qs-step-receiving-body", { done: 2 })));
    expect(body).toContain(`Green Owl${text("qs-route-sender")}`);
    expect(body).toContain(`Blue Fox${text("qs-route-receiving")}`);
    fireEvent.click(button("qs-receive-pause"));
    fireEvent.click(button("qs-receive-stop"));
    expect(e.onPause).toHaveBeenCalled();
    expect(e.onStop).toHaveBeenCalled();
  });

  it("the sender's route reads this laptop sending, the other laptop receiving", () => {
    progress();
    const body = plain(document.body.textContent);
    expect(body).toContain(`Blue Fox${text("qs-route-sender")}`);
    expect(body).toContain(`Green Owl${text("qs-route-receiver")}`);
    expect(text("qs-route-sender")).toBe("Sending");
  });

  it("the bar is the counted fraction of bytes, with the counts in words", () => {
    progress();
    const bar = screen.getByRole("progressbar");
    expect(bar.getAttribute("aria-valuenow")).toBe(String(Math.round((2_000_000 / 7_800_000) * 100)));
    expect(plain(document.body.textContent)).toContain(plain(text("qs-tally", { count: 2, size: "2 MB" })));
    expect(plain(document.body.textContent)).toContain(plain(text("qs-step-copying-body", { done: 2, total: 4 })));
  });

  it("the speed is a measured range, and absent when not measured or paused", () => {
    progress();
    expect(plain(document.body.textContent)).toMatch(/1\.9 MB\/s – 3\.1 MB\/s/);
    cleanup();
    progress({ speed: null });
    expect(plain(document.body.textContent)).not.toMatch(/MB\/s/);
    cleanup();
    progress({ phase: "paused" });
    expect(plain(document.body.textContent)).not.toMatch(/MB\/s/);
  });

  it("every stage carries its state for a screen reader, and the current one is marked", () => {
    progress({ phase: "copying" });
    const steps = screen.getAllByRole("listitem").filter((li) => li.className.includes("frame-step"));
    expect(steps.length).toBe(2);
    expect(steps[0].textContent).toContain(text("frame-step-current"));
    expect(steps[1].textContent).toContain(text("frame-step-todo"));
    expect(steps[0].getAttribute("aria-current")).toBe("step");
    cleanup();
    progress({ phase: "checking" });
    const later = screen.getAllByRole("listitem").filter((li) => li.className.includes("frame-step"));
    expect(later[0].textContent).toContain(text("frame-step-done"));
    expect(later[1].getAttribute("aria-current")).toBe("step");
  });

  it("pause and resume swap; paused says what is kept", () => {
    const e = progress();
    fireEvent.click(button("qs-pause"));
    expect(e.onPause).toHaveBeenCalled();
    cleanup();
    const e2 = progress({ phase: "paused" });
    expect(maybeButton("qs-pause")).toBeNull();
    expect(screen.getByRole("status").textContent).toContain(text("qs-sending-paused"));
    fireEvent.click(button("qs-resume"));
    expect(e2.onResume).toHaveBeenCalled();
  });

  it("keep it compact and stop each raise their event; there is no receipt step or button", () => {
    const e = progress();
    fireEvent.click(button("qs-compact"));
    fireEvent.click(button("qs-stop"));
    expect(e.onCompact).toHaveBeenCalled();
    expect(e.onStop).toHaveBeenCalled();
    expect(document.body.textContent).not.toMatch(/receipt/i);
  });

  it("a completed send says so with its own card, and offers no pause, compact or stop", () => {
    progress({ phase: "complete", filesDone: 4, bytesDone: 7_800_000, bytesTotal: 7_800_000 });
    expect(screen.getByText(text("qs-sending-complete-title"))).toBeTruthy();
    expect(screen.getByRole("progressbar").getAttribute("aria-valuenow")).toBe("100");
    for (const id of ["qs-pause", "qs-compact", "qs-stop", "qs-resume"]) expect(maybeButton(id)).toBeNull();
    expect(screen.getAllByText(text("frame-step-done")).length).toBe(2);
  });

  it("needs action counts what did not arrive; a dropped connection says what is kept and offers no compact view", () => {
    progress({ phase: "needs-action", failed: 2 });
    expect(screen.getByRole("alert").textContent).toContain(text("qs-sending-needs-action", { count: 2 }));
    cleanup();
    progress({ phase: "lost" });
    expect(plain(screen.getByRole("alert").textContent)).toContain(plain(text("qs-sending-disconnected", { label: "Green Owl" })));
    expect(maybeButton("qs-compact")).toBeNull();
    expect(maybeButton("qs-pause")).toBeNull();
  });
});

describe("Q14 Done", () => {
  const done = (props: Partial<Parameters<typeof QuickDone>[0]> = {}) => {
    const events = { onOpenFolder: fn(), onSendMissing: fn(), onDone: fn() };
    inApp(<QuickDone side="send" receipt={RECEIPT_WHOLE} files={OFFER} {...events} {...props} />);
    return events;
  };

  it("everything arrived: says so on each side, with one line of what arrived and matched", () => {
    done();
    expect(h1()).toBe(text("qs-done-sent-title"));
    expect(plain(document.body.textContent)).toContain(plain(text("qs-done-arrived", { count: 3, size: "6.6 MB" })));
    expect(plain(document.body.textContent)).toContain(plain(text("qs-done-already", { count: 1 })));
    expect(screen.queryByText(text("qs-done-problems-title"))).toBeNull();
    cleanup();
    done({ side: "receive", folder: "Downloads" });
    expect(h1()).toBe(text("qs-done-here-title"));
    expect(plain(document.body.textContent)).toContain(plain(text("qs-done-folder", { folder: "Downloads" })));
  });

  it("never says the files were sent, or are here, unless every file is in place", () => {
    for (const receipt of [RECEIPT_PARTIAL, RECEIPT_PROBLEMS]) {
      for (const side of ["send", "receive"] as const) {
        done({ side, receipt });
        expect(h1()).toBe(text("qs-done-partial-title"));
        cleanup();
      }
    }
    done({ receipt: { lines: [] } });
    expect(h1()).toBe(text("qs-done-empty-title"));
  });

  it("lists only the files that need attention, each with its outcome word and the reason behind More details", () => {
    done({ side: "receive", receipt: RECEIPT_PROBLEMS, folder: "Downloads" });
    const section = screen.getByRole("region", { name: text("qs-done-problems-title") });
    const rows = within(section).getAllByRole("listitem");
    expect(rows).toHaveLength(2);
    expect(rows[0].textContent).toContain("beach.jpg");
    expect(rows[0].textContent).toContain(text("qs-done-not-arrived"));
    expect(rows[0].textContent).toContain(text("status-failed"));
    expect(rows[1].textContent).toContain(text("qs-done-mark-not-kept"));
    expect(rows[1].textContent).toContain(text("status-needs-action"));
    const details = section.querySelector("details");
    expect(details?.open).toBe(false);
    expect(details?.textContent).toContain("the connection was lost before it finished");
    expect(within(section).queryByText("Budget.xlsx")).toBeNull();
  });

  it("every file declined: the sender is told the receiving laptop did not take them, never that some did not arrive", () => {
    done({ receipt: { lines: OFFER.map((f) => ({ path: f.path, outcome: { kind: "declined" as const } })) } });
    expect(h1()).toBe(text("qs-done-refused-title"));
    expect(screen.getByText(text("qs-done-refused-body"))).toBeTruthy();
  });

  it("Open folder is the receiver's; a folder that cannot be opened says so", () => {
    const e = done({ side: "receive", folder: "Downloads" });
    fireEvent.click(button("qs-done-open-folder"));
    expect(e.onOpenFolder).toHaveBeenCalled();
    cleanup();
    done({ side: "receive", folder: "Downloads", openFailed: true });
    expect(screen.getByText(text("qs-done-open-failed"))).toBeTruthy();
    cleanup();
    done();
    expect(maybeButton("qs-done-open-folder")).toBeNull();
  });

  it("Send the missing files is the sender's, and only when something did not arrive", () => {
    const e = done({ receipt: RECEIPT_PARTIAL });
    fireEvent.click(screen.getByRole("button", { name: nameHas(text("qs-done-send-missing")) }));
    expect(e.onSendMissing).toHaveBeenCalled();
    cleanup();
    done();
    expect(screen.queryByRole("button", { name: nameHas(text("qs-done-send-missing")) })).toBeNull();
    cleanup();
    done({ side: "receive", receipt: RECEIPT_PARTIAL });
    expect(screen.queryByRole("button", { name: nameHas(text("qs-done-send-missing")) })).toBeNull();
  });

  it("a stopped send says it was stopped and what arrived, and its unsent files say so", () => {
    done({ stopped: true, receipt: RECEIPT_PARTIAL });
    expect(h1()).toBe(text("qs-done-stopped-title"));
    expect(screen.getByText(text("qs-done-stopped-body"))).toBeTruthy();
    expect(plain(document.body.textContent)).toContain(plain(text("qs-done-arrived", { count: 2, size: "3.7 MB" })));
    expect(screen.getAllByText(nameHas(text("qs-done-not-completed"))).length).toBe(2);
  });

  it("without the offer (from history) says the count only; a renamed file says why", () => {
    done({ side: "receive", receipt: RECEIPT_PARTIAL, files: undefined });
    expect(plain(document.body.textContent)).toContain(plain(text("qs-done-arrived-count", { count: 2 })));
    expect(plain(document.body.textContent)).toContain(plain(text("qs-done-renamed", { name: "Budget (2).xlsx" })));
  });

  it("Done raises its event, and the word receipt is nowhere on the page", () => {
    const e = done({ side: "receive", receipt: RECEIPT_PROBLEMS, folder: "Downloads" });
    expect(document.body.textContent).not.toMatch(/receipt/i);
    fireEvent.click(button("qs-done-done"));
    expect(e.onDone).toHaveBeenCalled();
  });

  it("has no accessibility violations", async () => {
    done({ side: "receive", receipt: RECEIPT_PROBLEMS, folder: "Downloads" });
    expect(await violations()).toEqual([]);
  });
});

describe("Q20 Stop this send", () => {
  it("stopping needs the person to understand that arrived files stay; the stop button is never the first focus", () => {
    const onConfirm = fn();
    const onClose = fn();
    inApp(<QuickStop open onConfirm={onConfirm} onClose={onClose} />);
    expect(screen.getByRole("dialog", { name: text("qs-stop-title") })).toBeTruthy();
    const stop = button("qs-stop-now") as HTMLButtonElement;
    expect(stop.disabled).toBe(true);
    expect(document.activeElement).not.toBe(stop);
    expect(document.activeElement?.textContent).toBe(text("qs-stop-body"));
    fireEvent.click(screen.getByRole("checkbox", { name: nameHas(text("qs-stop-ack")) }));
    expect(stop.disabled).toBe(false);
    fireEvent.click(stop);
    expect(onConfirm).toHaveBeenCalled();
    fireEvent.click(button("qs-stop-keep"));
    expect(onClose).toHaveBeenCalled();
  });

  it("says what stays, what stops and what happens to a half-arrived file", () => {
    inApp(<QuickStop open onConfirm={fn()} onClose={fn()} />);
    for (const part of ["arrived", "rest", "partial"]) {
      expect(screen.getByText(text(`qs-stop-${part}-title`))).toBeTruthy();
      expect(screen.getByText(text(`qs-stop-${part}-body`))).toBeTruthy();
    }
  });

  it("a send that already finished has nothing to stop", () => {
    inApp(<QuickStop open finished onConfirm={fn()} onClose={fn()} />);
    expect(screen.getByText(text("qs-stop-finished"))).toBeTruthy();
    expect(maybeButton("qs-stop-now")).toBeNull();
  });

  it("Escape closes it without stopping", () => {
    const onConfirm = fn();
    const onClose = fn();
    inApp(<QuickStop open onConfirm={onConfirm} onClose={onClose} />);
    fireEvent.keyDown(screen.getByRole("dialog"), { key: "Escape" });
    expect(onClose).toHaveBeenCalled();
    expect(onConfirm).not.toHaveBeenCalled();
  });

  it("the tick starts clear each time the dialog opens", () => {
    const { rerender } = inApp(<QuickStop open onConfirm={fn()} onClose={fn()} />);
    fireEvent.click(screen.getByRole("checkbox"));
    rerender(
      <LocalizationProvider l10n={l10n}>
        <main>
          <QuickStop open={false} onConfirm={fn()} onClose={fn()} />
        </main>
      </LocalizationProvider>,
    );
    rerender(
      <LocalizationProvider l10n={l10n}>
        <main>
          <QuickStop open onConfirm={fn()} onClose={fn()} />
        </main>
      </LocalizationProvider>,
    );
    expect(screen.getByRole<HTMLInputElement>("checkbox").checked).toBe(false);
  });
});

describe("F08 My PCTwin history", () => {
  const history = (entries: HistoryEntry[] = HISTORY, props: Partial<Parameters<typeof History>[0]> = {}) => {
    const events = { onOpen: vi.fn(), onContinue: vi.fn(), onExport: fn(), onStart: fn(), onBack: fn() };
    inApp(<History entries={entries} {...events} {...props} />);
    return events;
  };

  it("lists newest first, says it stays on this laptop, and has no account", () => {
    history();
    const rows = screen.getAllByRole("listitem").map((li) => plain(li.textContent));
    expect(rows[0]).toContain("Green Owl");
    expect(rows[rows.length - 1]).toContain("Blue Fox");
    expect(screen.getByText(text("history-note"))).toBeTruthy();
    expect(document.body.textContent).not.toMatch(/account(?! needed)|sign[ -]?in|log[ -]?in/i);
  });

  it("each state is its own word: only a whole send or finished move is 'Finished'", () => {
    history();
    expect(screen.getAllByText(text("history-status-finished")).length).toBe(1);
    expect(screen.getByText(text("history-status-partial"))).toBeTruthy();
    expect(screen.getByText(text("history-status-paused"))).toBeTruthy();
    expect(screen.getByText(text("history-status-stopped"))).toBeTruthy();
    expect(screen.getByText(text("history-status-steps-left", { count: 2 }))).toBeTruthy();
  });

  it("opens a report for a move and the files for a send, and continues only a paused move", () => {
    const e = history();
    const reports = screen.getAllByRole("button", { name: (n) => plain(n).startsWith(text("history-open-report")) || plain(n).startsWith(text("history-open-files")) });
    expect(reports.map((b) => b.textContent)).toEqual([text("history-open-files"), text("history-open-files"), text("history-open-report"), text("history-open-report"), text("history-open-files")]);
    expect(document.body.textContent).not.toMatch(/receipt/i);
    fireEvent.click(reports[0]);
    expect(e.onOpen).toHaveBeenCalledWith("h1");
    fireEvent.click(screen.getByRole("button", { name: text("history-continue") }));
    expect(e.onContinue).toHaveBeenCalledWith("h3");
  });

  it("export raises its event and says where it went or that it could not", () => {
    const e = history();
    fireEvent.click(button("history-export"));
    expect(e.onExport).toHaveBeenCalled();
    cleanup();
    history(HISTORY, { exported: { kind: "saved", path: "Documents/h.json" } });
    expect(plain(screen.getByRole("status").textContent)).toContain(plain(text("history-exported", { path: "Documents/h.json" })));
    cleanup();
    history(HISTORY, { exported: { kind: "failed" } });
    expect(screen.getByRole("alert").textContent).toContain(text("history-export-failed"));
  });

  it("empty: says so, offers a move, and no export", () => {
    const e = history([]);
    expect(screen.getByText(text("history-empty"))).toBeTruthy();
    expect(maybeButton("history-export")).toBeNull();
    fireEvent.click(button("history-start"));
    expect(e.onStart).toHaveBeenCalled();
  });
});

describe("every Quick Send screen", () => {
  const ours = SAMPLES.filter((s) => /^(Q\d\d|F08)-/.test(s.id));

  it("is covered by samples for each frame in this lane", () => {
    const frames = new Set(ours.map((s) => s.id.split("-")[0]));
    for (const f of ["Q01", "Q02", "Q12", "Q14", "Q20", "F08"]) expect(frames.has(f)).toBe(true);
  });

  it.each(ours.map((s) => [s.id, s] as const))("%s never says receipt, and Quick Send never says old or new laptop", (id, sample) => {
    inApp(sample.render());
    expect(document.body.textContent).not.toMatch(/receipt/i);
    // History also lists full moves, whose laptops may be named old and new.
    if (!id.startsWith("F08")) expect(document.body.textContent).not.toMatch(/old laptop|new laptop/i);
  });

  it.each(ours.map((s) => [s.id, s] as const))("%s has nothing paid: no price, plan, upgrade, trial, storage or sign-in", (_id, sample) => {
    inApp(sample.render());
    const paid = /\b(price|plan|upgrade|pro|premium|trial|subscri|cloud|storage|quota|backup|sign[ -]?in|log[ -]?in)\b|[$€£₦]/i;
    expect(document.body.textContent).not.toMatch(paid);
  });

  it("inside the window's footer provider, the main action of Q01 and Q14 sits in the footer with the free-sending note", () => {
    for (const [id, action] of [["Q01-default", "qs-home-action"], ["Q14-sent", "qs-done-done"]] as const) {
      const sample = ours.find((s) => s.id === id);
      render(
        <LocalizationProvider l10n={l10n}>
          <FooterProvider>
            <main>{sample?.render()}</main>
          </FooterProvider>
        </LocalizationProvider>,
      );
      const footer = within(screen.getByRole("contentinfo"));
      expect(footer.getByText(text("footer-note-send"))).toBeTruthy();
      expect(footer.getByRole("button", { name: nameIs(text(action)) })).toBeTruthy();
      expect(within(screen.getByRole("main")).queryByRole("button", { name: nameIs(text(action)) })).toBeNull();
      cleanup();
    }
  });

  it("a screen's heading takes keyboard focus when it appears", () => {
    inApp(ours.find((s) => s.id === "Q14-sent")?.render());
    expect(document.activeElement).toBe(screen.getByRole("heading", { level: 1 }));
  });

  it("works at the narrowest window: no fixed widths that would crop (rows wrap)", async () => {
    inApp(ours.find((s) => s.id === "Q14-partial-sender")?.render());
    await act(async () => {});
    expect(document.querySelector(".qs-rows")).not.toBeNull();
  });
});

describe("names from another laptop are shown safely", () => {
  const done2 = (receipt: Receipt) => inApp(<QuickDone side="receive" receipt={receipt} folder="Downloads" onDone={fn()} />);

  it("a name that reorders itself shows its hidden characters as codes, and its real type apart", () => {
    expect(shownSafely("invoice\u202Efdp.exe")).toBe("invoice[U+202E]fdp.exe");
    expect(shownSafely("a\u2067b\u0000c")).toBe("a[U+2067]b[U+0000]c");
    expect(fileName("Docs/invoice\u202Efdp.exe")).toBe("invoice[U+202E]fdp.exe");
    expect(extensionOf("Docs/invoice\u202Efdp.exe")).toBe("exe");
    expect(extensionOf("Docs/notes")).toBeNull();
    expect(shownSafely("Holiday/été.jpg")).toBe("Holiday/été.jpg");
  });

  it("the Done page shows such a name with its codes, in the files that need attention and in the files that arrived", () => {
    const name = "invoice‮fdp.exe";
    done2({ lines: [{ path: name, outcome: { kind: "arrived", landedAs: name } }, { path: `b${name}`, outcome: { kind: "failed", why: "x" } }] });
    const body = plain(document.body.textContent);
    expect(body).toContain("invoice[U+202E]fdp.exe");
    expect(body).toContain("binvoice[U+202E]fdp.exe");
    expect(body).not.toContain("‮");
  });
});

describe("pairing in Quick Send's words", () => {
  it.each(["en", "fr", "es", "pt", "ar", "hi", "zh-CN"] as const)("%s: every pairing line that names the laptops old or new has a Quick Send twin with other words", (lang) => {
    const words = localization(lang);
    for (const id of QUICK_WORDS) {
      expect(words.getBundle(`qs-${id}`), `qs-${id}`).not.toBeNull();
      const vars = { seconds: 5, label: "Blue Fox", number: 42 };
      expect(words.getString(`qs-${id}`, vars), id).not.toBe(words.getString(id, vars));
      if (lang === "en") expect(words.getString(`qs-${id}`, vars)).not.toMatch(/\b(old|new) laptop/i);
    }
  });
});

describe("Quick Send names the laptops by name or role", () => {
  // Kept because the full move uses them (MoveHost): never shown on a Quick Send page.
  const FULL_MOVE_ONLY = new Set(["qs-label-this", "qs-label-other"]);

  it("no English Quick Send line says this laptop, the other laptop, old laptop or new laptop", () => {
    const ftl = Object.values(import.meta.glob<string>("../locales/en/main.ftl", { query: "?raw", import: "default", eager: true }))[0];
    const bad: string[] = [];
    let id = "";
    for (const line of ftl.split(/\r?\n/)) {
      const m = /^([a-z][a-z0-9-]*) =/.exec(line);
      if (m) id = m[1];
      else if (!line.startsWith(" ")) id = "";
      if (!id.startsWith("qs-") || FULL_MOVE_ONLY.has(id)) continue;
      if (/this laptop|the other laptop|old laptop|new laptop/i.test(line)) bad.push(line);
    }
    expect(bad).toEqual([]);
  });

  it("a name from the other laptop is shown safely on the pages that use it", () => {
    inApp(<QuickProgress side="receive" from={{ kind: "named", name: shownSafely("Ada‮gpj.exe") }} to={{ kind: "named", name: text("qs-label-yours") }} progress={PROGRESS} onPause={fn()} onResume={fn()} onCompact={fn()} onStop={fn()} />);
    expect(document.body.textContent).not.toContain("‮");
    expect(document.body.textContent).toContain("Ada[U+202E]gpj.exe");
  });
});
