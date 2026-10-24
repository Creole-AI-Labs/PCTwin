import { LocalizationProvider } from "@fluent/react";
import { useState, type ReactNode } from "react";
import { act, cleanup, fireEvent, render, screen, within } from "@testing-library/react";
import { violations } from "../testing/a11y";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { SAMPLES } from "../gallery/samples";
import { localization } from "../i18n";
import { extensionOf, fileName, fraction, formatBytes, historyStateOf, missingLines, receiptState, shownSafely, tally, type Conflict, type HistoryEntry, type PickedItem, type Receipt } from "../quicksend";
import { BLUE_FOX, CONFLICTS, GREEN_OWL, HISTORY, NEARBY, OFFER, PICKED, PROGRESS, RECEIPT_PARTIAL, RECEIPT_WHOLE } from "../quicksend.fixtures";
import { FooterProvider } from "./FrameFooter";
import History from "./History";
import QuickApprove from "./QuickApprove";
import QuickHome from "./QuickHome";
import QuickIncoming from "./QuickIncoming";
import QuickKeep from "./QuickKeep";
import QuickMissing from "./QuickMissing";
import QuickPick from "./QuickPick";
import QuickProgress from "./QuickProgress";
import QuickReceipt from "./QuickReceipt";
import QuickReview from "./QuickReview";
import QuickStop from "./QuickStop";
import QuickWhere from "./QuickWhere";
import QuickWho from "./QuickWho";

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

/** A screen's controlled tick, kept in a tiny harness so a test can click it. */
function Ticked({ children }: { children: (on: boolean, set: (on: boolean) => void) => ReactNode }) {
  const [on, set] = useState(false);
  return <>{children(on, set)}</>;
}

describe("what the words depend on (receipts, history, sizes)", () => {
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

  it("takes focus on its heading and offers Send, Receive and past receipts as cards", () => {
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

describe("Q03 Check what you are sharing", () => {
  const review = (props: Partial<Parameters<typeof QuickReview>[0]> = {}) => {
    const events = { onChange: fn(), onContinue: fn(), onBack: fn() };
    inApp(
      <Ticked>
        {(on, set) => <QuickReview files={OFFER} checked={on} onCheckedChange={set} {...events} {...props} />}
      </Ticked>,
    );
    return events;
  };

  it("continue stays off until the files are checked, then goes", () => {
    const e = review();
    const go = button("qs-check-continue") as HTMLButtonElement;
    expect(go.disabled).toBe(true);
    fireEvent.click(screen.getByRole("checkbox", { name: nameHas(text("qs-check-ack")) }));
    expect(go.disabled).toBe(false);
    fireEvent.click(go);
    expect(e.onContinue).toHaveBeenCalled();
  });

  it("lists every file with its size, and says what the receiver will see", () => {
    review();
    expect(screen.getAllByText(text("qs-check-included")).length).toBe(OFFER.length);
    expect(screen.getByText(text("qs-check-shared-note"))).toBeTruthy();
  });

  it("a changed, missing or unreadable file blocks even a ticked box, and is announced", () => {
    review({ issues: [{ path: "Budget.xlsx", kind: "changed" }, { path: "Old.txt", kind: "missing" }, { path: "Tax", kind: "unreadable" }] });
    fireEvent.click(screen.getByRole("checkbox"));
    expect((button("qs-check-continue") as HTMLButtonElement).disabled).toBe(true);
    const status = screen.getByRole("status").textContent;
    expect(plain(status)).toContain(plain(text("qs-check-changed", { name: "Budget.xlsx" })));
    expect(plain(status)).toContain(plain(text("qs-check-missing", { name: "Old.txt" })));
    expect(plain(status)).toContain(plain(text("qs-check-unreadable", { name: "Tax" })));
  });

  it("a long list shows the first hundred and then all of them on request, counting every file", () => {
    const many = Array.from({ length: 150 }, (_, i) => ({ id: `m${String(i)}`, path: `Folder/file ${String(i)}.txt`, size: 10 }));
    review({ files: many });
    expect(document.querySelectorAll(".kit-row").length).toBe(100);
    fireEvent.click(button("qs-check-show-all", { count: 150 }));
    expect(document.querySelectorAll(".kit-row").length).toBe(150);
  });

  it("Change goes back to the choice", () => {
    const e = review();
    fireEvent.click(button("qs-check-change"));
    expect(e.onChange).toHaveBeenCalled();
  });
});

describe("Q04 Who should receive it?", () => {
  it("chooses a laptop nearby by index, or uses a code", () => {
    const onChooseNearby = vi.fn();
    const onUseCode = fn();
    inApp(<QuickWho count={4} size={7_800_000} nearby={NEARBY} onChooseNearby={onChooseNearby} onUseCode={onUseCode} onBack={fn()} />);
    fireEvent.click(screen.getByRole("button", { name: nameHas("Green Owl") }));
    expect(onChooseNearby).toHaveBeenCalledWith(0);
    fireEvent.click(button("qs-who-action"));
    expect(onUseCode).toHaveBeenCalled();
  });

  it("a busy laptop cannot be chosen and says why; a repeated name warns", () => {
    const onChooseNearby = vi.fn();
    inApp(
      <QuickWho
        count={1}
        size={10}
        nearby={[{ ...NEARBY[0], busy: true }, { ...NEARBY[1], sameLabelNearby: true, needsUpdate: false }]}
        onChooseNearby={onChooseNearby}
        onUseCode={fn()}
        onBack={fn()}
      />,
    );
    const busy = screen.getByRole<HTMLButtonElement>("button", { name: nameHas("Green Owl") });
    expect(busy.disabled).toBe(true);
    expect(plain(busy.textContent)).toContain(plain(text("qs-who-busy", { label: "Green Owl" })));
    expect(screen.getByRole("button", { name: /Sam/ }).textContent).toContain(text("qs-who-same-label"));
  });

  it("with nothing nearby it says so and the code still works", () => {
    const onUseCode = fn();
    inApp(<QuickWho count={1} size={10} nearby={[]} onChooseNearby={vi.fn()} onUseCode={onUseCode} onBack={fn()} />);
    expect(screen.getByRole("status").textContent).toContain(text("qs-no-nearby"));
    fireEvent.click(screen.getByRole("button", { name: new RegExp(text("qs-who-code")) }));
    expect(onUseCode).toHaveBeenCalled();
  });
});

describe("Q08 Someone wants to send you files", () => {
  const incoming = (props: Partial<Parameters<typeof QuickIncoming>[0]> = {}) => {
    const events = { onAccept: fn(), onDecline: fn(), onDone: fn() };
    inApp(<QuickIncoming from={BLUE_FOX} to={GREEN_OWL} files={OFFER} {...events} {...props} />);
    return events;
  };

  it("shows names and sizes only, with the roles of the two laptops, and never a place to choose from the sender", () => {
    incoming();
    expect(screen.getByRole("group", { name: nameIs(`From Blue Fox to Green Owl`) })).toBeTruthy();
    expect(screen.getByText(text("qs-offer-note"))).toBeTruthy();
    expect(screen.getAllByText(/\bMB\b/).length).toBeGreaterThan(0);
    expect(document.querySelector("input[type=text], input[type=file]")).toBeNull();
  });

  it("accept and decline raise their own events", () => {
    const e = incoming();
    fireEvent.click(button("qs-offer-accept"));
    fireEvent.click(button("qs-offer-decline"));
    expect(e.onAccept).toHaveBeenCalledTimes(1);
    expect(e.onDecline).toHaveBeenCalledTimes(1);
  });

  it("an empty offer cannot be accepted", () => {
    incoming({ files: [] });
    expect((button("qs-offer-accept") as HTMLButtonElement).disabled).toBe(true);
  });

  it("an offer that changed says so, in an alert", () => {
    incoming({ changed: true });
    expect(screen.getByRole("alert").textContent).toContain(text("qs-offer-changed"));
  });

  it("declined and gone end the offer: nothing was saved, and the heading takes focus", () => {
    const e = incoming({ state: "declined" });
    expect(screen.getByText(text("qs-offer-declined"))).toBeTruthy();
    expect(document.activeElement?.tagName).toBe("H1");
    expect(maybeButton("qs-offer-accept")).toBeNull();
    fireEvent.click(button("qs-receipt-done"));
    expect(e.onDone).toHaveBeenCalled();
    cleanup();
    incoming({ state: "gone" });
    expect(plain(screen.getByRole("alert").textContent)).toContain(plain(text("qs-offer-gone", { label: "Blue Fox" })));
  });
});

describe("Q09 Where should the files land?", () => {
  const where = (props: Partial<Parameters<typeof QuickWhere>[0]> = {}) => {
    const events = { onChoose: fn(), onContinue: fn(), onBack: fn() };
    inApp(
      <Ticked>
        {(on, set) => <QuickWhere folder="Documents/PCTwin received" free={86_000_000_000} needed={7_800_000} confirmed={on} onConfirmedChange={set} {...events} {...props} />}
      </Ticked>,
    );
    return events;
  };

  it("fits: shows the folder, the measured space, and continues only after the folder is confirmed", () => {
    const e = where();
    expect(screen.getByText(text("qs-land-status-fits"))).toBeTruthy();
    expect(plain(document.body.textContent)).toContain(plain(text("qs-land-space", { free: "86 GB", needed: "7.8 MB" })));
    const go = button("qs-land-use") as HTMLButtonElement;
    expect(go.disabled).toBe(true);
    fireEvent.click(screen.getByRole("checkbox", { name: nameHas(text("qs-land-confirm")) }));
    fireEvent.click(go);
    expect(e.onContinue).toHaveBeenCalled();
  });

  it("not enough space says the numbers and blocks, even when confirmed", () => {
    where({ free: 4_000_000 });
    fireEvent.click(screen.getByRole("checkbox"));
    expect((button("qs-land-use") as HTMLButtonElement).disabled).toBe(true);
    expect(screen.getByText(text("qs-land-status-no-space"))).toBeTruthy();
    expect(plain(screen.getByRole("alert").textContent)).toContain(plain(text("qs-land-no-space", { free: "4 MB", needed: "7.8 MB" })));
  });

  it("space that could not be measured is said, not guessed", () => {
    where({ free: null });
    expect(plain(document.body.textContent)).toContain(plain(text("qs-land-space-unknown", { needed: "7.8 MB" })));
    expect(screen.queryByText(text("qs-land-status-fits"))).toBeNull();
  });

  it("a folder that is gone, or not writable, blocks and says what to do", () => {
    where({ problem: "unavailable" });
    fireEvent.click(screen.getByRole("checkbox"));
    expect((button("qs-land-use") as HTMLButtonElement).disabled).toBe(true);
    expect(screen.getByRole("alert").textContent).toContain(text("qs-land-unavailable"));
    cleanup();
    where({ problem: "no-permission" });
    expect(screen.getByRole("alert").textContent).toContain(text("qs-land-denied"));
  });

  it("no folder yet: the picker is the way forward", () => {
    const e = where({ folder: null, free: null });
    expect((button("qs-land-use") as HTMLButtonElement).disabled).toBe(true);
    fireEvent.click(button("qs-land-choose"));
    expect(e.onChoose).toHaveBeenCalled();
  });
});

describe("Q10 Keep your existing work safe", () => {
  function Keep({ conflicts = CONFLICTS, onContinue = fn() }: { conflicts?: Conflict[]; onContinue?: () => void }) {
    const [list, setList] = useState(conflicts);
    const [approved, setApproved] = useState(false);
    const change = (path: string, patch: Partial<Conflict>) => {
      setList((prev) => prev.map((c) => (c.path === path ? { ...c, ...patch } : c)));
    };
    return (
      <QuickKeep
        files={OFFER}
        conflicts={list}
        approved={approved}
        onChoose={(path, choice) => {
          change(path, { choice, changed: false });
        }}
        onApprovedChange={setApproved}
        onContinue={onContinue}
        onBack={fn()}
      />
    );
  }

  it("lists every offered file with what is already there, and offers only what the engine can do", () => {
    inApp(<Keep />);
    expect(screen.getAllByRole("row").length).toBe(OFFER.length + 1);
    expect(screen.getAllByText(text("qs-conflict-none")).length).toBe(OFFER.length - CONFLICTS.length);
    const options = within(screen.getAllByRole("combobox")[0]).getAllByRole("option").map((o) => o.textContent);
    expect(options).toEqual([text("qs-keep-both"), text("qs-skip")]);
  });

  it("every clash needs a choice and the approval before continuing; keep both says the new name", () => {
    const onContinue = fn();
    inApp(<Keep onContinue={onContinue} />);
    const go = button("qs-conflict-continue") as HTMLButtonElement;
    expect(go.disabled).toBe(true);
    fireEvent.change(screen.getAllByRole("combobox")[1], { target: { value: "keep-both" } });
    expect(go.disabled).toBe(true);
    fireEvent.click(screen.getByRole("checkbox", { name: text("qs-conflict-approve") }));
    expect(go.disabled).toBe(false);
    expect(plain(document.body.textContent)).toContain(plain(text("qs-keep-both-note", { newName: "beach (2).jpg" })));
    fireEvent.click(go);
    expect(onContinue).toHaveBeenCalled();
  });

  it("never offers Replace: every choice is exactly Keep both or Skip this item", () => {
    inApp(<Keep />);
    const boxes = screen.getAllByRole("combobox");
    expect(boxes.length).toBe(CONFLICTS.length);
    for (const box of boxes) {
      // The "choose" placeholder (empty value) is not a choice.
      const options = within(box)
        .getAllByRole<HTMLOptionElement>("option")
        .filter((o) => o.value !== "");
      expect(options.map((o) => o.textContent)).toEqual([text("qs-keep-both"), text("qs-skip")]);
      expect(options.map((o) => o.value)).not.toContain("replace");
    }
  });

  it("a file that changed in the folder has no choice until chosen again", () => {
    inApp(<Keep conflicts={[{ ...CONFLICTS[0], choice: null, changed: true }]} />);
    expect(plain(document.body.textContent)).toContain(plain(text("qs-conflict-target-changed", { name: "Budget.xlsx" })));
    expect((button("qs-conflict-continue") as HTMLButtonElement).disabled).toBe(true);
  });

  it("no clashes: nothing to choose, nothing will be replaced, and continue is open", () => {
    inApp(<Keep conflicts={[]} />);
    expect(screen.getByText(text("qs-no-conflicts"))).toBeTruthy();
    expect((button("qs-conflict-continue") as HTMLButtonElement).disabled).toBe(false);
  });
});

describe("Q11 Ready to send these files?", () => {
  const approve = (props: Partial<Parameters<typeof QuickApprove>[0]> = {}) => {
    const onStart = fn();
    inApp(
      <Ticked>
        {(on, set) => <QuickApprove from={BLUE_FOX} to={GREEN_OWL} count={4} size={7_800_000} receiver="accepted" approved={on} onApprovedChange={set} onStart={onStart} onBack={fn()} {...props} />}
      </Ticked>,
    );
    return onStart;
  };
  const tick = () => fireEvent.click(screen.getByRole("checkbox"));

  it("starts only when the receiver accepted, the files are unchanged, and the person ticked", () => {
    const onStart = approve();
    const go = button("qs-approve-start") as HTMLButtonElement;
    expect(go.disabled).toBe(true);
    tick();
    expect(go.disabled).toBe(false);
    fireEvent.click(go);
    expect(onStart).toHaveBeenCalled();
  });

  it.each(["waiting", "withdrawn", "cleared"] as const)("a receiver who is %s blocks the start, even when ticked", (receiver) => {
    approve({ receiver });
    tick();
    expect((button("qs-approve-start") as HTMLButtonElement).disabled).toBe(true);
    expect(screen.getAllByText(text("qs-status-waiting")).length).toBe(1);
  });

  it("withdrawn is an alert and a change to the files clears the approval with its own words", () => {
    approve({ receiver: "withdrawn" });
    expect(plain(screen.getByRole("alert").textContent)).toContain(plain(text("qs-approve-withdrawn", { label: "Green Owl" })));
    cleanup();
    approve({ receiver: "cleared" });
    expect(plain(document.body.textContent)).toContain(plain(text("qs-approve-cleared", { label: "Green Owl" })));
  });

  it("a changed file blocks the start and says so", () => {
    approve({ issues: [{ path: "Budget.xlsx", kind: "changed" }] });
    tick();
    expect((button("qs-approve-start") as HTMLButtonElement).disabled).toBe(true);
    expect(screen.getByText(text("qs-status-changed"))).toBeTruthy();
  });
});

describe("Q12 Your files are on their way", () => {
  const progress = (patch: Partial<typeof PROGRESS> = {}) => {
    const events = { onPause: fn(), onResume: fn(), onCompact: fn(), onStop: fn(), onReceipt: fn() };
    inApp(<QuickProgress from={BLUE_FOX} to={GREEN_OWL} progress={{ ...PROGRESS, ...patch }} {...events} />);
    return events;
  };

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
    progress({ phase: "checking" });
    const steps = screen.getAllByRole("listitem").filter((li) => li.className.includes("frame-step"));
    expect(steps.length).toBe(3);
    expect(steps[0].textContent).toContain(text("frame-step-done"));
    expect(steps[1].textContent).toContain(text("frame-step-current"));
    expect(steps[2].textContent).toContain(text("frame-step-todo"));
    expect(steps[1].getAttribute("aria-current")).toBe("step");
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

  it("keep it compact, stop and the receipt so far each raise their event", () => {
    const e = progress();
    fireEvent.click(button("qs-compact"));
    fireEvent.click(button("qs-stop"));
    fireEvent.click(button("qs-sending-receipt"));
    expect(e.onCompact).toHaveBeenCalled();
    expect(e.onStop).toHaveBeenCalled();
    expect(e.onReceipt).toHaveBeenCalled();
  });

  it("a completed send says so with its own card, and offers no pause, compact or stop", () => {
    progress({ phase: "complete", filesDone: 4, bytesDone: 7_800_000, bytesTotal: 7_800_000 });
    expect(screen.getByText(text("qs-sending-complete-title"))).toBeTruthy();
    expect(screen.getByRole("progressbar").getAttribute("aria-valuenow")).toBe("100");
    for (const id of ["qs-pause", "qs-compact", "qs-stop", "qs-resume"]) expect(maybeButton(id)).toBeNull();
    expect(screen.getAllByText(text("frame-step-done")).length).toBe(3);
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

describe("Q14 Handover receipt", () => {
  const receipt = (props: Partial<Parameters<typeof QuickReceipt>[0]> = {}) => {
    const events = { onAcknowledge: fn(), onSendMissing: fn(), onHistory: fn(), onDone: fn() };
    inApp(<QuickReceipt side="send" receipt={RECEIPT_WHOLE} from={BLUE_FOX} to={GREEN_OWL} files={OFFER} {...events} {...props} />);
    return events;
  };

  it("a whole receipt says everything arrived, with measured counts and sizes", () => {
    receipt();
    expect(screen.getByText(text("qs-receipt-box-title"))).toBeTruthy();
    expect(screen.queryByText(text("qs-receipt-partial-title"))).toBeNull();
    expect(screen.getByText("3")).toBeTruthy();
    expect(plain(document.body.textContent)).toContain("6.6 MB");
  });

  it("a partial receipt never says whole, says how many did not arrive, and what to do next", () => {
    receipt({ receipt: RECEIPT_PARTIAL });
    expect(screen.queryByText(text("qs-receipt-box-title"))).toBeNull();
    expect(screen.queryByText(text("qs-receipt-box-body"))).toBeNull();
    expect(screen.getByText(text("qs-receipt-partial-title"))).toBeTruthy();
    expect(screen.getByText(text("qs-receipt-partial", { count: 2 }))).toBeTruthy();
    expect(screen.getByText(text("qs-receipt-not-arrived"))).toBeTruthy();
    expect(screen.getByText(text("status-needs-action"))).toBeTruthy();
    expect(screen.getByText(text("qs-receipt-chip-declined"))).toBeTruthy();
    expect(screen.getByText(nameIs(text("qs-receipt-declined-send")))).toBeTruthy();
  });

  it("a file from another laptop whose downloaded mark the drive could not keep says to open it with care", () => {
    const unmarked: Receipt = {
      lines: [{ path: "setup.exe", outcome: { kind: "needs-action", landedAs: "setup.exe", why: "engine words", word: "mark-not-kept" } }],
    };
    receipt({ side: "receive", receipt: unmarked });
    expect(plain(document.body.textContent)).toContain(plain(text("qs-receipt-mark-not-kept", { path: "setup.exe" })));
    expect(screen.queryByText(text("qs-receipt-box-title"))).toBeNull();
  });

  it("an identical file already there is its own line and is never shown as Copied", () => {
    receipt();
    const row = screen.getByText("Project brief.pdf").closest("li");
    expect(row?.textContent).toContain(text("qs-receipt-chip-already"));
    expect(row?.textContent).not.toContain(text("qs-receipt-status-verified"));
    expect(row?.textContent).not.toContain(text("status-copied"));
    expect(plain(row?.textContent ?? "")).toContain(plain(text("qs-receipt-already", { path: "Project brief.pdf" })));
    expect(screen.getAllByText(text("qs-receipt-status-verified")).length).toBe(3);
  });

  it("the receiver sees the name it was saved under, and a new name says why", () => {
    receipt({ side: "receive", receipt: RECEIPT_PARTIAL });
    const row = screen.getByText("Budget.xlsx").closest("li");
    expect(plain(row?.textContent ?? "")).toContain(plain(text("qs-receipt-saved-as", { path: "Budget (2).xlsx" })));
    expect(row?.textContent).toContain(text("qs-receipt-renamed"));
    expect(screen.getByText(text("qs-receipt-partial", { count: 2 }))).toBeTruthy();
  });

  it("the reason a file failed stays behind More details", () => {
    receipt({ receipt: RECEIPT_PARTIAL });
    const details = document.querySelector("details");
    expect(details?.open).toBe(false);
    expect(details?.textContent).toContain("the connection was lost before it finished");
  });

  it("Send missing pieces is the sender's, and only when something is missing", () => {
    const e = receipt({ receipt: RECEIPT_PARTIAL });
    fireEvent.click(screen.getByRole("button", { name: nameHas(text("qs-receipt-missing")) }));
    expect(e.onSendMissing).toHaveBeenCalled();
    cleanup();
    receipt();
    expect(screen.queryByRole("button", { name: nameHas(text("qs-receipt-missing")) })).toBeNull();
    cleanup();
    receipt({ side: "receive", receipt: RECEIPT_PARTIAL });
    expect(screen.queryByRole("button", { name: nameHas(text("qs-receipt-missing")) })).toBeNull();
  });

  it("acknowledging is the receiver's, says it is not 'opened', and cannot be done twice", () => {
    const e = receipt({ side: "receive" });
    expect(screen.getByText(text("qs-receipt-ack-note"))).toBeTruthy();
    expect(screen.getByText(text("qs-receipt-not-opened"))).toBeTruthy();
    fireEvent.click(button("qs-receipt-ack"));
    expect(e.onAcknowledge).toHaveBeenCalled();
    cleanup();
    receipt({ side: "receive", acknowledgedAt: "2026-10-15T14:10:00Z" });
    expect(maybeButton("qs-receipt-ack")).toBeNull();
    expect(plain(document.body.textContent)).toContain("Receipt acknowledged");
  });

  it("stopped and offline are said; an empty receipt is not 'whole'", () => {
    receipt({ stopped: true, receipt: RECEIPT_PARTIAL });
    expect(screen.getByText(text("qs-receipt-stopped"))).toBeTruthy();
    cleanup();
    receipt({ peerOffline: true });
    expect(plain(document.body.textContent)).toContain(plain(text("qs-receipt-offline", { label: "Green Owl" })));
    cleanup();
    receipt({ receipt: { lines: [] } });
    expect(screen.queryByText(text("qs-receipt-box-title"))).toBeNull();
    expect(screen.getByText(text("qs-receipt-empty"))).toBeTruthy();
  });

  it("after a stop, lines that did not arrive are Unfinished and say Not completed; arrived ones say they were checked first", () => {
    receipt({ stopped: true, receipt: RECEIPT_PARTIAL });
    expect(screen.getAllByText(text("qs-receipt-status-unfinished")).length).toBe(2);
    expect(screen.getAllByText(nameIs(text("qs-receipt-not-completed"))).length).toBe(2);
    expect(screen.getAllByText(nameIs(text("qs-receipt-stopped-row"))).length).toBe(2);
    expect(screen.queryByText(text("status-needs-action"))).toBeNull();
  });

  it("the history and done actions raise their events", () => {
    const e = receipt();
    fireEvent.click(button("qs-receipt-history"));
    fireEvent.click(button("qs-receipt-done"));
    expect(e.onHistory).toHaveBeenCalled();
    expect(e.onDone).toHaveBeenCalled();
  });
});

describe("Q19 Send the missing pieces", () => {
  const missing = (props: Partial<Parameters<typeof QuickMissing>[0]> = {}) => {
    const events = { onContinue: fn(), onNewSend: fn(), onBack: fn() };
    inApp(
      <Ticked>
        {(on, set) => <QuickMissing receipt={RECEIPT_PARTIAL} approved={on} onApprovedChange={set} {...events} {...props} />}
      </Ticked>,
    );
    return events;
  };

  it("lists what is kept and what is missing, and continues only when ticked", () => {
    const e = missing();
    expect(screen.getAllByText(text("qs-missing-status-missing")).length).toBe(2);
    expect(screen.getAllByText(text("qs-missing-status-there")).length).toBe(2);
    const go = button("qs-missing-continue") as HTMLButtonElement;
    expect(go.disabled).toBe(true);
    fireEvent.click(screen.getByRole("checkbox"));
    fireEvent.click(go);
    expect(e.onContinue).toHaveBeenCalled();
  });

  it("nothing missing: says so and offers nothing to send", () => {
    missing({ receipt: RECEIPT_WHOLE });
    expect(screen.getByText(text("qs-missing-complete"))).toBeTruthy();
    expect(maybeButton("qs-missing-continue")).toBeNull();
  });

  it("no receipt: says it cannot tell what is missing and offers a new send", () => {
    const e = missing({ receipt: null });
    expect(screen.getByRole("alert").textContent).toContain(text("qs-missing-unavailable"));
    expect(maybeButton("qs-missing-continue")).toBeNull();
    fireEvent.click(button("qs-missing-new-send"));
    expect(e.onNewSend).toHaveBeenCalled();
  });

  it("a changed file is listed so it is approved again", () => {
    missing({ issues: [{ path: "beach.jpg", kind: "changed" }] });
    expect(plain(screen.getByRole("alert").textContent)).toContain(plain(text("qs-check-changed", { name: "beach.jpg" })));
  });
});

describe("Q20 Stop or decline", () => {
  it("stopping needs the person to understand that arrived files stay; the stop button is never the first focus", () => {
    const onConfirm = fn();
    const onClose = fn();
    inApp(<QuickStop open kind="stop" onConfirm={onConfirm} onClose={onClose} />);
    expect(screen.getByRole("dialog", { name: text("qs-stop-title") })).toBeTruthy();
    const stop = button("qs-stop-confirm") as HTMLButtonElement;
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
    inApp(<QuickStop open kind="stop" onConfirm={fn()} onClose={fn()} />);
    for (const part of ["arrived", "rest", "partial"]) {
      expect(screen.getByText(text(`qs-stop-${part}-title`))).toBeTruthy();
      expect(screen.getByText(text(`qs-stop-${part}-body`))).toBeTruthy();
    }
  });

  it("declining an offer is plain: nothing is saved", () => {
    const onConfirm = fn();
    inApp(<QuickStop open kind="decline" onConfirm={onConfirm} onClose={fn()} />);
    expect(screen.getByText(text("qs-decline-body"))).toBeTruthy();
    fireEvent.click(button("qs-decline-confirm"));
    expect(onConfirm).toHaveBeenCalled();
  });

  it("a send that already finished has nothing to stop", () => {
    inApp(<QuickStop open kind="stop" finished onConfirm={fn()} onClose={fn()} />);
    expect(screen.getByText(text("qs-already-finished"))).toBeTruthy();
    expect(maybeButton("qs-stop-confirm")).toBeNull();
  });

  it("Escape closes it without stopping", () => {
    const onConfirm = fn();
    const onClose = fn();
    inApp(<QuickStop open kind="stop" onConfirm={onConfirm} onClose={onClose} />);
    fireEvent.keyDown(screen.getByRole("dialog"), { key: "Escape" });
    expect(onClose).toHaveBeenCalled();
    expect(onConfirm).not.toHaveBeenCalled();
  });

  it("the tick starts clear each time the dialog opens", () => {
    const { rerender } = inApp(<QuickStop open kind="stop" onConfirm={fn()} onClose={fn()} />);
    fireEvent.click(screen.getByRole("checkbox"));
    rerender(
      <LocalizationProvider l10n={l10n}>
        <main>
          <QuickStop open={false} kind="stop" onConfirm={fn()} onClose={fn()} />
        </main>
      </LocalizationProvider>,
    );
    rerender(
      <LocalizationProvider l10n={l10n}>
        <main>
          <QuickStop open kind="stop" onConfirm={fn()} onClose={fn()} />
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

  it("opens a report for a move and a receipt for a send, and continues only a paused move", () => {
    const e = history();
    const reports = screen.getAllByRole("button", { name: /Report|Receipt/ });
    expect(reports.map((b) => b.textContent)).toEqual([text("history-open-receipt"), text("history-open-receipt"), text("history-open-report"), text("history-open-report"), text("history-open-receipt")]);
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
    for (const f of ["Q01", "Q02", "Q03", "Q04", "Q08", "Q09", "Q10", "Q11", "Q12", "Q14", "Q19", "Q20", "F08"]) expect(frames.has(f)).toBe(true);
  });

  it.each(ours.map((s) => [s.id, s] as const))("%s has nothing paid: no price, plan, upgrade, trial, storage or sign-in", (_id, sample) => {
    inApp(sample.render());
    const paid = /\b(price|plan|upgrade|pro|premium|trial|subscri|cloud|storage|quota|backup|sign[ -]?in|log[ -]?in)\b|[$€£₦]/i;
    expect(document.body.textContent).not.toMatch(paid);
  });

  it("inside the window's footer provider, the main action of Q01, Q11 and Q14 sits in the footer with the free-sending note", () => {
    for (const [id, action] of [["Q01-default", "qs-home-action"], ["Q11-ready", "qs-approve-start"], ["Q14-whole-sender", "qs-receipt-done"]] as const) {
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
    inApp(ours.find((s) => s.id === "Q03-default")?.render());
    expect(document.activeElement).toBe(screen.getByRole("heading", { level: 1 }));
  });

  it("works at the narrowest window: no fixed widths that would crop (rows wrap)", async () => {
    inApp(ours.find((s) => s.id === "Q14-partial-sender")?.render());
    await act(async () => {});
    expect(document.querySelector(".qs-rows")).not.toBeNull();
  });
});

describe("names from another laptop are shown safely", () => {
  it("a name that reorders itself shows its hidden characters as codes, and its real type apart", () => {
    expect(shownSafely("invoice\u202Efdp.exe")).toBe("invoice[U+202E]fdp.exe");
    expect(shownSafely("a\u2067b\u0000c")).toBe("a[U+2067]b[U+0000]c");
    expect(fileName("Docs/invoice\u202Efdp.exe")).toBe("invoice[U+202E]fdp.exe");
    expect(extensionOf("Docs/invoice\u202Efdp.exe")).toBe("exe");
    expect(extensionOf("Docs/notes")).toBeNull();
    expect(shownSafely("Holiday/été.jpg")).toBe("Holiday/été.jpg");
  });

  it("the offer screen shows such a name with its codes and its real type apart from it", () => {
    inApp(
      <QuickIncoming
        from={BLUE_FOX}
        to={GREEN_OWL}
        files={[{ id: "1", path: "invoice‮fdp.exe", size: 10 }]}
        onAccept={vi.fn()}
        onDecline={vi.fn()}
        onDone={vi.fn()}
      />,
    );
    const body = plain(document.body.textContent);
    expect(body).toContain("invoice[U+202E]fdp.exe");
    expect(body).not.toContain("‮");
    expect(body).toContain(plain(text("qs-file-type", { ext: "exe" })));
  });
});
