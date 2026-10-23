import { LocalizationProvider } from "@fluent/react";
import { cleanup, fireEvent, render, screen, within } from "@testing-library/react";
import type { ReactNode } from "react";
import { violations as accessible } from "../testing/a11y";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { localization } from "../i18n";
import {
  approvalHolds,
  attentionCount,
  checkDestinations,
  chosenTotals,
  isPartial,
  leftOutByHand,
  sizeParts,
  windowModeFor,
  type Approval,
  type DestGroup,
  type FolderChoice,
  type PlanView,
  type ScanView,
} from "../plan";
import * as sample from "../planSamples";
import ChooseFolders from "./ChooseFolders";
import Destination from "./Destination";
import Overview from "./Overview";
import PlanReview from "./PlanReview";
import ScanProgress from "./ScanProgress";
import { FooterProvider } from "./FrameFooter";

const l10n = localization("en");
const text = (id: string, vars?: Record<string, string | number>) => l10n.getString(id, vars);
/** An attribute-only message (an accessible name) written out. */
function attr(id: string, name: string, vars?: Record<string, string | number>): string {
  const bundle = l10n.getBundle(id);
  const message = bundle?.getMessage(id);
  const pattern = message?.attributes[name];
  if (!bundle || !pattern) throw new Error(`no ${name} on ${id}`);
  return bundle.formatPattern(pattern, vars);
}
/** Fluent isolates variables for right-to-left safety; the marks are invisible, so tests compare without them. */
const plain = (s: string) => s.replace(/[⁦-⁩]/g, "");
const nameIs = (expected: string) => (accessible: string) => plain(accessible) === plain(expected);
const withText = (expected: string) => (content: string) => plain(content) === plain(expected);
/** What an element says, without the invisible direction marks. */
const said = (el: HTMLElement) => plain(el.textContent);

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

/** No screen in this part shows anything paid. */
function expectNothingPaid() {
  const words = document.body.textContent;
  expect(words).not.toMatch(/upgrade|\bpro\b|premium|subscri|trial|price|\$\d|cloud twin|family plan|storage quota/i);
}

function noMissingStrings() {
  expect(warnings.filter((w) => /missing|not found|unknown/i.test(w))).toEqual([]);
}

const noop = () => undefined;

/* ---------- The rules the screens stand on ---------- */

describe("plan rules", () => {
  it("sizes are written in one decimal, or whole numbers from 100", () => {
    expect(sizeParts(18_400_000_000)).toEqual({ unit: "gb", n: 18.4 });
    expect(sizeParts(120_000_000_000)).toEqual({ unit: "gb", n: 120 });
    expect(sizeParts(999)).toEqual({ unit: "b", n: 999 });
    expect(sizeParts(1_500)).toEqual({ unit: "kb", n: 1.5 });
  });

  it("an approval holds only for the same revision and the same fingerprint", () => {
    const a: Approval = { revision: 4, fingerprint: sample.planFingerprint };
    expect(approvalHolds(sample.plan, a)).toBe(true);
    expect(approvalHolds(sample.plan, null)).toBe(false);
    expect(approvalHolds(sample.plan, { ...a, revision: 3 })).toBe(false);
    // The same revision number with different content is still a different plan.
    expect(approvalHolds(sample.plan, { ...a, fingerprint: "00" })).toBe(false);
    expect(approvalHolds(sample.planNext, a)).toBe(false);
  });

  it("chosen totals leave out what is not chosen or cannot be read, and say when a part was not fully read", () => {
    const t = chosenTotals(sample.foldersDefault);
    expect(t).toEqual({ count: 2, bytes: 80_500_000_000, files: 44_224, atLeast: false });
    const withWork = sample.foldersDefault.map((f) => (f.id === "f-work" ? { ...f, selected: true } : f));
    expect(chosenTotals(withWork)).toMatchObject({ count: 3, atLeast: true });
    // A protected folder is never counted, even if it somehow arrives ticked.
    const ticked = sample.foldersProtected.map((f) => (f.id === "f-videos" ? { ...f, selected: true } : f));
    expect(chosenTotals(ticked).count).toBe(2);
  });

  it("destinations: ready, missing drive, not enough room, read-only, file too big, nothing chosen", () => {
    expect(checkDestinations(sample.groupsMapped, sample.drivesMapped).ok).toBe(true);

    const missing = checkDestinations(sample.groupsMapped, sample.drivesMissing);
    expect(missing.ok).toBe(false);
    expect(missing.groupIssues["f-pictures"]).toEqual(["drive-missing"]);

    const gone = checkDestinations(sample.groupsMapped, [sample.newLaptop]);
    expect(gone.groupIssues["f-pictures"]).toEqual(["drive-missing"]);

    const full = checkDestinations(sample.groupsMapped, sample.drivesFull);
    expect(full.ok).toBe(false);
    const load = full.loads.find((l) => l.drive.id === "travel");
    expect(load).toMatchObject({ needBytes: 62_100_000_000, shortBytes: 22_100_000_000 });

    const readOnly = checkDestinations(sample.groupsMapped, [sample.newLaptop, { ...sample.travelDisk, readOnly: true }]);
    expect(readOnly.groupIssues["f-pictures"]).toEqual(["drive-read-only"]);

    const big: DestGroup[] = sample.groupsMapped.map((g) => (g.id === "f-pictures" ? { ...g, largestFileBytes: 5_000_000_000 } : g));
    const fat = checkDestinations(big, [sample.newLaptop, { ...sample.travelDisk, largestFile: 4_294_967_295 }]);
    expect(fat.groupIssues["f-pictures"]).toEqual(["file-too-big"]);

    const none: DestGroup[] = sample.groupsMapped.map((g) => (g.id === "f-pictures" ? { ...g, to: null } : g));
    expect(checkDestinations(none, sample.drivesMapped).groupIssues["f-pictures"]).toEqual(["no-destination"]);
  });

  it("two groups on one drive share its room", () => {
    const groups: DestGroup[] = sample.groupsMapped.map((g) => ({ ...g, to: { drive: "travel", folder: g.role } }));
    const check = checkDestinations(groups, [sample.newLaptop, { ...sample.travelDisk, freeBytes: 100_000_000_000 }]);
    // 18.4 + 62.1 + 28.3 = 108.8 GB against 100 GB free.
    expect(check.loads[0]).toMatchObject({ needBytes: 108_800_000_000, shortBytes: 8_800_000_000 });
  });

  it("the scan is partial only when it has finished with places it could not read", () => {
    expect(isPartial(sample.partial)).toBe(true);
    expect(isPartial(sample.finished)).toBe(false);
    expect(isPartial({ ...sample.partial, status: "scanning" })).toBe(false);
  });

  it("the overview counts everything that still needs the person", () => {
    expect(attentionCount(sample.overview)).toBe(0);
    expect(attentionCount(sample.overviewPartial)).toBe(4);
  });

  it("only the reasons that need the person's hands are listed as steps", () => {
    expect(leftOutByHand(sample.plan).map((l) => l.reason)).toEqual(["cloud-only", "work-managed"]);
  });

  it("the review and overview use the wide window; the host sets it, the screens do not", () => {
    expect(windowModeFor("overview")).toBe("expanded");
    expect(windowModeFor("review")).toBe("expanded");
    expect(windowModeFor("scan")).toBe("everyday");
    expect(windowModeFor("files")).toBe("everyday");
    expect(windowModeFor("destination")).toBe("everyday");
  });
});

/* ---------- M02 Finding your files ---------- */

describe("M02 scan progress and resume", () => {
  const view = (scan: ScanView, extra: Partial<Parameters<typeof ScanProgress>[0]> = {}) => (
    <ScanProgress scan={scan} onPause={noop} onResume={noop} onContinue={noop} {...extra} />
  );

  it("scanning: the heading has focus, counts are the measured ones, steps are words, Pause is the action", async () => {
    inApp(view(sample.scanning));
    const heading = screen.getByRole("heading", { level: 1, name: text("scan-title") });
    expect(document.activeElement).toBe(heading);
    expect(screen.getByText(withText(text("scan-found", { count: 48210, size: text("plan-size-gb", { n: 91.2 }) })))).toBeTruthy();
    expect(screen.getByText(withText(text("scan-places", { done: 3, total: 7 })))).toBeTruthy();
    const bar = screen.getByRole("progressbar", { name: attr("scan-progress", "aria-label") });
    expect(bar.getAttribute("aria-valuenow")).toBe("43");
    // Each step says where it is in words, not by colour.
    expect(within(screen.getByText(text("scan-step-usual-title")).closest("li") as HTMLElement).getByText(text("frame-step-current"))).toBeTruthy();
    expect(within(screen.getByText(text("scan-step-other-title")).closest("li") as HTMLElement).getByText(text("frame-step-todo"))).toBeTruthy();
    expect(screen.getByRole("button", { name: text("scan-pause") })).toBeTruthy();
    expect(screen.queryByRole("button", { name: text("scan-continue") })).toBeNull();
    expect(screen.queryByRole("button", { name: text("scan-resume") })).toBeNull();
    noMissingStrings();
    expectNothingPaid();
    expect(await accessible()).toEqual([]);
  });

  it("asking to pause is passed on once", () => {
    const onPause = vi.fn();
    inApp(view(sample.scanning, { onPause }));
    fireEvent.click(screen.getByRole("button", { name: text("scan-pause") }));
    expect(onPause).toHaveBeenCalledTimes(1);
  });

  it("the progress bar is the measured share of places, and never invents one when there are none to count", () => {
    inApp(view({ ...sample.scanning, places: { done: 0, total: 0 } }));
    expect(screen.getByRole("progressbar").getAttribute("aria-valuenow")).toBe("0");
  });

  it("pausing: says it is finishing the folder it is on, and offers neither Pause nor Continue", () => {
    inApp(view(sample.pausing));
    expect(said(screen.getByRole("status"))).toContain(plain(text("scan-pausing")));
    expect(screen.queryByRole("button", { name: text("scan-pause") })).toBeNull();
    expect(screen.queryByRole("button", { name: text("scan-resume") })).toBeNull();
  });

  it("paused: everything found is saved, and Continue searching carries on", async () => {
    const onResume = vi.fn();
    inApp(view(sample.paused, { onResume }));
    expect(said(screen.getByRole("status"))).toContain(plain(text("scan-paused")));
    fireEvent.click(screen.getByRole("button", { name: text("scan-resume") }));
    expect(onResume).toHaveBeenCalledTimes(1);
    expect(screen.queryByRole("button", { name: text("scan-pause") })).toBeNull();
    expect(await accessible()).toEqual([]);
  });

  it("a new state takes the focus on its new heading", () => {
    const { rerender } = inApp(view(sample.scanning));
    (document.activeElement as HTMLElement).blur();
    rerender(
      <LocalizationProvider l10n={l10n}>
        <main>{view(sample.paused)}</main>
      </LocalizationProvider>,
    );
    expect(document.activeElement).toBe(screen.getByRole("heading", { level: 1 }));
  });

  it("scanning more heard of does not pull the focus off whatever the person is on", () => {
    const { rerender } = inApp(view(sample.scanning));
    const pause = screen.getByRole("button", { name: text("scan-pause") });
    pause.focus();
    rerender(
      <LocalizationProvider l10n={l10n}>
        <main>{view({ ...sample.scanning, places: { done: 4, total: 7 }, found: { files: 50_000, bytes: 95_000_000_000 } })}</main>
      </LocalizationProvider>,
    );
    expect(document.activeElement).toBe(screen.getByRole("button", { name: text("scan-pause") }));
  });

  it("resumed: says it picked up where it stopped", () => {
    inApp(view(sample.resumed));
    expect(said(screen.getByRole("status"))).toContain(plain(text("scan-resumed")));
  });

  it("partial: places that could not be read stay listed as not searched, and say how many are not counted", async () => {
    const onAllowAccess = vi.fn();
    const onContinue = vi.fn();
    inApp(view(sample.partial, { onAllowAccess, onContinue }));
    expect(screen.getByRole("heading", { level: 1, name: text("scan-title") })).toBeTruthy();
    expect(said(screen.getByRole("alert"))).toContain(plain(text("scan-partial", { count: 2 })));
    expect(screen.getByText(text("scan-unreadable-note"))).toBeTruthy();
    expect(screen.getAllByText(text("scan-status-not-searched"))).toHaveLength(2);
    expect(screen.getByText("D:\\Archive\\Locked")).toBeTruthy();
    // The protected step is still to come, in words.
    expect(within(screen.getByText(text("scan-step-protected-title")).closest("li") as HTMLElement).getByText(text("frame-step-todo"))).toBeTruthy();
    fireEvent.click(screen.getAllByRole("button", { name: text("scan-allow") })[1]);
    expect(onAllowAccess).toHaveBeenCalledWith("D:\\Archive\\Locked");
    fireEvent.click(screen.getByRole("button", { name: text("scan-continue") }));
    expect(onContinue).toHaveBeenCalledTimes(1);
    noMissingStrings();
    expectNothingPaid();
    expect(await accessible()).toEqual([]);
  });

  it("finished with nothing unreadable: no warning, and Continue is the action", () => {
    inApp(view(sample.finished));
    expect(screen.queryByRole("alert")).toBeNull();
    expect(screen.queryByText(text("scan-status-not-searched"))).toBeNull();
    expect(screen.getByRole("button", { name: text("scan-continue") })).toBeTruthy();
  });

  it("without a way to ask for permission there is no Allow button, only the honest list", () => {
    inApp(view(sample.partial));
    expect(screen.queryByRole("button", { name: text("scan-allow") })).toBeNull();
    expect(screen.getAllByText(text("scan-status-not-searched"))).toHaveLength(2);
  });
});

/* ---------- M03 Everything you chose, in view ---------- */

describe("M03 overview", () => {
  const view = (v = sample.overview, extra: Partial<Parameters<typeof Overview>[0]> = {}) => <Overview view={v} onOpen={noop} onContinue={noop} {...extra} />;

  it("shows the measured totals, the heading has focus, and nothing from a later phase appears", async () => {
    inApp(view());
    expect(document.activeElement).toBe(screen.getByRole("heading", { level: 1, name: text("overview-title") }));
    expect(screen.getByText(withText(text("plan-size-gb", { n: 109 })))).toBeTruthy();
    expect(screen.getByText(text("overview-size-label"))).toBeTruthy();
    expect(screen.getByText("44,224")).toBeTruthy();
    expect(screen.getByText(text("overview-items-label", { count: 44224 }))).toBeTruthy();
    // No apps, settings or profile teasers.
    expect(document.body.textContent).not.toMatch(/\bapps?\b|settings|browser|games/i);
    noMissingStrings();
    expectNothingPaid();
    expect(await accessible()).toEqual([]);
  });

  it("each row opens its own screen", () => {
    const onOpen = vi.fn();
    inApp(view(sample.overview, { onOpen }));
    const row = (title: string) => screen.getByRole("button", { name: new RegExp(`${text("overview-review")}.*${title}`) });
    fireEvent.click(row(text("overview-files-title")));
    fireEvent.click(row(text("overview-dest-title")));
    fireEvent.click(row(text("overview-people-title")));
    expect(onOpen.mock.calls).toEqual([["files"], ["destinations"], ["people"]]);
  });

  it("with a partial scan it says so, says those files are not included, and offers to allow access", () => {
    const onAllowAccess = vi.fn();
    inApp(view(sample.overviewPartial, { onAllowAccess }));
    expect(said(screen.getByRole("alert"))).toContain(plain(text("overview-partial", { count: 2 })));
    fireEvent.click(screen.getByRole("button", { name: text("overview-allow") }));
    expect(onAllowAccess).toHaveBeenCalledTimes(1);
  });

  it("counts what needs attention and marks the rows that need it, with words", () => {
    inApp(view(sample.overviewPartial));
    expect(screen.getByText("4")).toBeTruthy();
    expect(screen.getByText(text("overview-attention-label", { count: 4 }))).toBeTruthy();
    // Files (unreadable), destinations and people each carry the mark.
    expect(screen.getAllByText(text("overview-row-attention"))).toHaveLength(3);
  });

  it("with nothing to fix there is no warning and no row mark", () => {
    inApp(view());
    expect(screen.queryByRole("alert")).toBeNull();
    expect(screen.queryByText(text("overview-row-attention"))).toBeNull();
  });

  it("a total with a part that was not fully read says at least", () => {
    inApp(view({ ...sample.overview, chosen: { ...sample.overview.chosen, atLeast: true } }));
    expect(screen.getByText(withText(text("plan-size-at-least", { size: text("plan-size-gb", { n: 109 }) })))).toBeTruthy();
  });

  it("Continue is passed on", () => {
    const onContinue = vi.fn();
    inApp(view(sample.overview, { onContinue }));
    fireEvent.click(screen.getByRole("button", { name: text("overview-continue") }));
    expect(onContinue).toHaveBeenCalledTimes(1);
  });
});

/* ---------- M04 Keep what matters to you ---------- */

describe("M04 files and folders", () => {
  const view = (folders: FolderChoice[], extra: Partial<Parameters<typeof ChooseFolders>[0]> = {}) => (
    <ChooseFolders folders={folders} onToggle={noop} onAddFolder={noop} onRemove={noop} onAllowAccess={noop} onContinue={noop} onBack={noop} {...extra} />
  );
  /** Two folders are called Documents here (personal and work), so each is named by its full place. */
  const DOCS = "C:\\Users\\Ada\\Documents";
  const box = (name: string) => screen.getByRole("checkbox", { name: nameIs(attr("files-select", "aria-label", { name })) });

  it("default: folders with sizes and checkboxes, the chosen total, and the no-limit line", async () => {
    inApp(view(sample.foldersDefault));
    expect(document.activeElement).toBe(screen.getByRole("heading", { level: 1, name: text("files-title") }));
    expect((box(DOCS) as HTMLInputElement).checked).toBe(true);
    expect((box("Downloads") as HTMLInputElement).checked).toBe(false);
    expect(screen.getByText(withText(text("files-folder-detail", { size: text("plan-size-gb", { n: 18.4 }), count: 12404 })))).toBeTruthy();
    expect(said(screen.getByRole("status"))).toContain(plain(text("files-total", { count: 2, size: text("plan-size-gb", { n: 80.5 }) })));
    expect(screen.getByText(text("files-no-limit"))).toBeTruthy();
    expect(screen.getByText(withText(text("files-online-only", { count: 12 })))).toBeTruthy();
    noMissingStrings();
    expectNothingPaid();
    expect(await accessible()).toEqual([]);
  });

  it("ticking and unticking is passed on with the folder and the new answer", () => {
    const onToggle = vi.fn();
    inApp(view(sample.foldersDefault, { onToggle }));
    fireEvent.click(box("Downloads"));
    fireEvent.click(box(DOCS));
    expect(onToggle.mock.calls).toEqual([
      ["f-downloads", true],
      ["f-documents", false],
    ]);
  });

  it("a work or school folder is left out and says so, and a part-read size says at least", () => {
    inApp(view(sample.foldersDefault));
    expect(screen.getByText(text("files-status-work"))).toBeTruthy();
    expect(screen.getByText(text("files-work-body"))).toBeTruthy();
    expect(screen.getByText(withText(text("files-folder-detail", { size: text("plan-size-at-least", { size: text("plan-size-gb", { n: 4.3 }) }), count: 2050 })))).toBeTruthy();
    expect((box(DOCS) as HTMLInputElement).checked).toBe(true);
  });

  it("a protected folder cannot be ticked, is not counted, and offers to allow access", async () => {
    const onAllowAccess = vi.fn();
    inApp(view(sample.foldersProtected, { onAllowAccess }));
    expect((box("Videos") as HTMLInputElement).disabled).toBe(true);
    expect(screen.getByText(text("files-status-protected"))).toBeTruthy();
    expect(screen.getByText(text("files-protected-body"))).toBeTruthy();
    expect(said(screen.getByRole("status"))).toContain(plain(text("files-total", { count: 2, size: text("plan-size-gb", { n: 80.5 }) })));
    fireEvent.click(screen.getByRole("button", { name: text("files-allow") }));
    expect(onAllowAccess).toHaveBeenCalledWith("f-videos");
    expect(await accessible()).toEqual([]);
  });

  it("a folder the person added is marked, can be removed by name, and counts", async () => {
    const onRemove = vi.fn();
    const onAddFolder = vi.fn();
    inApp(view(sample.foldersCustom, { onRemove, onAddFolder }));
    expect(screen.getByText(text("files-status-added"))).toBeTruthy();
    expect(screen.getByText("Projects")).toBeTruthy();
    expect(said(screen.getByRole("status"))).toContain(plain(text("files-total", { count: 3, size: text("plan-size-gb", { n: 109 }) })));
    fireEvent.click(screen.getByRole("button", { name: nameIs(attr("files-remove", "aria-label", { name: "Projects" })) }));
    expect(onRemove).toHaveBeenCalledWith("f-projects");
    // Only added folders can be removed.
    expect(screen.getAllByRole("button", { name: /^Remove / })).toHaveLength(1);
    fireEvent.click(screen.getByRole("button", { name: text("files-add") }));
    expect(onAddFolder).toHaveBeenCalledTimes(1);
    expect(await accessible()).toEqual([]);
  });

  it("finds folders by name, and says when none match", () => {
    inApp(view(sample.foldersDefault));
    const search = screen.getByRole("searchbox", { name: text("files-search-label") });
    fireEvent.change(search, { target: { value: "pic" } });
    expect(screen.queryByRole("checkbox", { name: nameIs(attr("files-select", "aria-label", { name: DOCS })) })).toBeNull();
    expect(box("Pictures")).toBeTruthy();
    fireEvent.change(search, { target: { value: "zzz" } });
    expect(screen.getByText(text("files-no-match"))).toBeTruthy();
    // The chosen total still counts what is chosen, not only what is shown.
    expect(said(screen.getByRole("status"))).toContain(plain(text("files-total", { count: 2, size: text("plan-size-gb", { n: 80.5 }) })));
  });

  it("with nothing chosen it says so and does not continue", () => {
    const onContinue = vi.fn();
    const none = sample.foldersDefault.map((f) => ({ ...f, selected: false }));
    inApp(view(none, { onContinue }));
    expect(said(screen.getByRole("status"))).toContain(plain(text("files-none")));
    const go = screen.getByRole<HTMLButtonElement>("button", { name: text("files-continue") });
    expect(go.disabled).toBe(true);
    fireEvent.click(go);
    expect(onContinue).not.toHaveBeenCalled();
  });

  it("with something chosen it continues, and Back goes back", () => {
    const onContinue = vi.fn();
    const onBack = vi.fn();
    inApp(view(sample.foldersDefault, { onContinue, onBack }));
    fireEvent.click(screen.getByRole("button", { name: text("files-continue") }));
    fireEvent.click(screen.getByRole("button", { name: text("plan-back") }));
    expect(onContinue).toHaveBeenCalledTimes(1);
    expect(onBack).toHaveBeenCalledTimes(1);
  });

  it("there is no cap: a very large number of folders and bytes are listed and counted", () => {
    const many: FolderChoice[] = Array.from({ length: 40 }, (_, i) => ({
      ...sample.foldersDefault[0],
      id: `f-${String(i)}`,
      path: `C:\\Big\\Folder ${String(i)}`,
      role: "other",
      size: { kind: "known", bytes: 5_000_000_000_000, files: 1_000_000 },
    }));
    inApp(view(many));
    expect(screen.getAllByRole("checkbox")).toHaveLength(40);
    expect(said(screen.getByRole("status"))).toContain(plain(text("files-total", { count: 40, size: text("plan-size-tb", { n: 200 }) })));
  });
});

/* ---------- M05 Give every file a home ---------- */

describe("M05 destination", () => {
  const view = (groups: DestGroup[], drives = sample.drivesMapped, extra: Partial<Parameters<typeof Destination>[0]> = {}) => (
    <Destination groups={groups} drives={drives} onSetDestination={noop} onSetAll={noop} onChooseFolder={noop} onContinue={noop} onBack={noop} {...extra} />
  );
  const pick = (name: string) => screen.getByRole("combobox", { name: nameIs(attr("dest-select", "aria-label", { name })) });

  it("all mapped: each folder has a place, every row says Ready, and the next step is open", async () => {
    inApp(view(sample.groupsMapped));
    expect(document.activeElement).toBe(screen.getByRole("heading", { level: 1, name: text("dest-title") }));
    expect(screen.getAllByText(text("dest-status-ready"))).toHaveLength(3);
    expect(screen.getByText(text("dest-all-ready"))).toBeTruthy();
    expect((pick("Documents") as HTMLSelectElement).value).toBe("new");
    expect((pick("Pictures") as HTMLSelectElement).value).toBe("travel");
    expect((screen.getByRole<HTMLButtonElement>("button", { name: text("dest-continue") })).disabled).toBe(false);
    noMissingStrings();
    expectNothingPaid();
    expect(await accessible()).toEqual([]);
  });

  it("the new laptop and a connected drive are told apart in plain words", () => {
    inApp(view(sample.groupsMapped));
    const options = within(pick("Pictures")).getAllByRole("option").map((o) => plain(o.textContent));
    expect(options).toContain(plain(text("dest-option-folder", { folder: "Pictures" })));
    expect(options).toContain(plain(text("dest-option-drive", { drive: "Travel disk" })));
    expect(options).toContain(plain(text("dest-option-choose")));
  });

  it("choosing another place is passed on with the folder and the drive", () => {
    const onSetDestination = vi.fn();
    inApp(view(sample.groupsMapped, sample.drivesMapped, { onSetDestination }));
    fireEvent.change(pick("Pictures"), { target: { value: "new" } });
    expect(onSetDestination).toHaveBeenCalledWith("f-pictures", "new");
  });

  it("Choose a folder asks for the folder picker instead of changing the place", () => {
    const onSetDestination = vi.fn();
    const onChooseFolder = vi.fn();
    inApp(view(sample.groupsMapped, sample.drivesMapped, { onSetDestination, onChooseFolder }));
    fireEvent.change(pick("Documents"), { target: { value: "__choose__" } });
    expect(onChooseFolder).toHaveBeenCalledWith("f-documents");
    expect(onSetDestination).not.toHaveBeenCalled();
  });

  it("one usual place for everything is passed on", () => {
    const onSetAll = vi.fn();
    inApp(view(sample.groupsMapped, sample.drivesMapped, { onSetAll }));
    fireEvent.change(screen.getByRole("combobox", { name: text("dest-default-label") }), { target: { value: "travel" } });
    expect(onSetAll).toHaveBeenCalledWith("travel");
  });

  it("a drive is missing: the row says so, the message names the drive, and the next step is closed", async () => {
    const onContinue = vi.fn();
    inApp(view(sample.groupsMapped, sample.drivesMissing, { onContinue }));
    expect(screen.getByText(text("dest-status-missing"))).toBeTruthy();
    expect(screen.getAllByText(text("dest-status-ready"))).toHaveLength(2);
    expect(said(screen.getByRole("alert"))).toContain(plain(text("dest-missing", { drive: "Travel disk" })));
    expect(screen.queryByText(text("dest-all-ready"))).toBeNull();
    const go = screen.getByRole<HTMLButtonElement>("button", { name: text("dest-continue") });
    expect(go.disabled).toBe(true);
    fireEvent.click(go);
    expect(onContinue).not.toHaveBeenCalled();
    expect(await accessible()).toEqual([]);
  });

  it("Change in a message takes the person to the place to change", () => {
    inApp(view(sample.groupsMapped, sample.drivesMissing));
    fireEvent.click(screen.getByRole("button", { name: text("dest-change") }));
    expect(document.activeElement).toBe(pick("Pictures"));
  });

  it("not enough space: the numbers are shown and what to do", async () => {
    inApp(view(sample.groupsMapped, sample.drivesFull));
    expect(screen.getByText(text("dest-status-space"))).toBeTruthy();
    expect(said(screen.getByRole("alert"))).toContain(plain(text("dest-space", { place: "Travel disk", free: text("plan-size-gb", { n: 40 }), needed: text("plan-size-gb", { n: 62.1 }) })));
    expect((screen.getByRole<HTMLButtonElement>("button", { name: text("dest-continue") })).disabled).toBe(true);
    expect(await accessible()).toEqual([]);
  });

  it("a size that was only partly read says at least when it is the need", () => {
    const partlyRead = sample.groupsMapped.map((g) => (g.id === "f-pictures" ? { ...g, size: { kind: "at-least" as const, bytes: 62_100_000_000, files: 31_820 } } : g));
    inApp(view(partlyRead, sample.drivesFull));
    expect(said(screen.getByRole("alert"))).toContain(plain(text("plan-size-at-least", { size: text("plan-size-gb", { n: 62.1 }) })));
  });

  it("no place chosen, read-only drive, and a file too big for the drive each say so", () => {
    const none: DestGroup[] = sample.groupsMapped.map((g) => (g.id === "f-projects" ? { ...g, to: null } : g));
    const { unmount } = inApp(view(none));
    expect(screen.getByText(text("dest-status-none"))).toBeTruthy();
    expect(said(screen.getByRole("alert"))).toContain(plain(text("dest-none", { name: "Projects" })));
    unmount();

    const { unmount: u2 } = inApp(view(sample.groupsMapped, [sample.newLaptop, { ...sample.travelDisk, readOnly: true }]));
    expect(screen.getByText(text("dest-status-readonly"))).toBeTruthy();
    expect(said(screen.getByRole("alert"))).toContain(plain(text("dest-readonly", { place: "Travel disk" })));
    u2();

    const big = sample.groupsMapped.map((g) => (g.id === "f-pictures" ? { ...g, largestFileBytes: 5_000_000_000 } : g));
    inApp(view(big, [sample.newLaptop, { ...sample.travelDisk, largestFile: 4_294_967_295 }]));
    expect(screen.getByText(text("dest-status-format"))).toBeTruthy();
    expect(said(screen.getByRole("alert"))).toContain(plain(text("dest-format", { place: "Travel disk" })));
  });

  it("Back goes back to the folders", () => {
    const onBack = vi.fn();
    inApp(view(sample.groupsMapped, sample.drivesMapped, { onBack }));
    fireEvent.click(screen.getByRole("button", { name: text("plan-back") }));
    expect(onBack).toHaveBeenCalledTimes(1);
  });
});

/* ---------- M23 Approve the move you want ---------- */

describe("M23 plan review and approval", () => {
  const good: Approval = { revision: sample.plan.revision, fingerprint: sample.plan.fingerprint };
  const view = (plan: PlanView, approval: Approval | null, extra: Partial<Parameters<typeof PlanReview>[0]> = {}) => (
    <PlanReview plan={plan} approval={approval} onApprovalChange={noop} onStart={noop} onEdit={noop} {...extra} />
  );
  const box = () => screen.getByRole("checkbox", { name: text("plan-approve") });
  const start = () => screen.getByRole<HTMLButtonElement>("button", { name: text("plan-start") });

  it("review: what will move, where, what to do by hand, and the old laptop stays unchanged", async () => {
    inApp(view(sample.plan, null));
    expect(document.activeElement).toBe(screen.getByRole("heading", { level: 1, name: text("plan-title") }));
    expect(screen.getByText(text("plan-body"))).toBeTruthy();
    expect(screen.getByText(withText(text("plan-what-body", { count: 3, size: text("plan-size-gb", { n: 109 }) })))).toBeTruthy();
    expect(screen.getByText(withText(text("plan-where-body", { count: 3 })))).toBeTruthy();
    expect(screen.getAllByText(withText(text("plan-group-to", { place: text("plan-place", { folder: "Documents", drive: "New laptop" }) }))).length).toBeGreaterThan(0);
    expect(screen.getByText(withText(text("plan-hand-body", { count: 2 })))).toBeTruthy();
    expect(screen.getByText(withText(text("plan-hand-cloud-only", { count: 12 })))).toBeTruthy();
    expect(screen.getByText(withText(text("plan-hand-work-managed", { count: 340 })))).toBeTruthy();
    // Things only the system or the person left out are not made into homework.
    expect(screen.queryByText(withText(text("plan-hand-unreadable", { count: 1 })))).toBeNull();
    expect(screen.getByText(text("plan-old-safe"))).toBeTruthy();
    noMissingStrings();
    expectNothingPaid();
    expect(await accessible()).toEqual([]);
  });

  it("with nothing to do by hand it says so", () => {
    inApp(view({ ...sample.plan, leftOut: [{ reason: "system", count: 4 }] }, null));
    expect(screen.getByText(text("plan-hand-none"))).toBeTruthy();
  });

  it("not approved yet: the box is empty, the start button is closed and says why", () => {
    const onStart = vi.fn();
    inApp(view(sample.plan, null, { onStart }));
    expect((box() as HTMLInputElement).checked).toBe(false);
    expect(start().disabled).toBe(true);
    expect(screen.getByText(text("plan-not-approved"))).toBeTruthy();
    expect(screen.queryByRole("alert")).toBeNull();
    fireEvent.click(start());
    expect(onStart).not.toHaveBeenCalled();
  });

  it("ticking approves exactly the revision and fingerprint on screen", () => {
    const onApprovalChange = vi.fn();
    inApp(view(sample.plan, null, { onApprovalChange }));
    fireEvent.click(box());
    expect(onApprovalChange).toHaveBeenCalledWith({ revision: 4, fingerprint: sample.planFingerprint });
  });

  it("unticking withdraws the approval", () => {
    const onApprovalChange = vi.fn();
    inApp(view(sample.plan, good, { onApprovalChange }));
    fireEvent.click(box());
    expect(onApprovalChange).toHaveBeenCalledWith(null);
  });

  it("approved: the box is ticked and Start the move is open, and it starts with that approval", () => {
    const onStart = vi.fn();
    inApp(view(sample.plan, good, { onStart }));
    expect((box() as HTMLInputElement).checked).toBe(true);
    expect(start().disabled).toBe(false);
    expect(screen.queryByText(text("plan-not-approved"))).toBeNull();
    fireEvent.click(start());
    expect(onStart).toHaveBeenCalledWith(good);
  });

  it("an approval for an older revision is cleared, said so out loud, and cannot start the move", async () => {
    const onApprovalChange = vi.fn();
    const onStart = vi.fn();
    inApp(view(sample.planNext, good, { onApprovalChange, onStart }));
    expect((box() as HTMLInputElement).checked).toBe(false);
    expect(start().disabled).toBe(true);
    expect(said(screen.getByRole("alert"))).toContain(plain(text("plan-changed")));
    expect(onApprovalChange).toHaveBeenCalledWith(null);
    fireEvent.click(start());
    expect(onStart).not.toHaveBeenCalled();
    expect(await accessible()).toEqual([]);
  });

  it("the same revision number with different content is a changed plan too", () => {
    const onApprovalChange = vi.fn();
    inApp(view({ ...sample.plan, fingerprint: sample.planNext.fingerprint }, good, { onApprovalChange }));
    expect(said(screen.getByRole("alert"))).toContain(plain(text("plan-changed")));
    expect(start().disabled).toBe(true);
    expect(onApprovalChange).toHaveBeenCalledWith(null);
  });

  it("when the plan changes under an approval the person had given, the approval goes and the change is announced", () => {
    const { rerender } = inApp(view(sample.plan, good));
    expect(start().disabled).toBe(false);
    expect(screen.queryByRole("alert")).toBeNull();
    // The host clears the approval at the same moment it hands over the new revision.
    rerender(
      <LocalizationProvider l10n={l10n}>
        <main>{view(sample.planNext, null)}</main>
      </LocalizationProvider>,
    );
    expect(said(screen.getByRole("alert"))).toContain(plain(text("plan-changed")));
    expect(start().disabled).toBe(true);
    expect((box() as HTMLInputElement).checked).toBe(false);
  });

  it("approving the new version takes the notice away and opens Start", () => {
    const { rerender } = inApp(view(sample.planNext, good));
    expect(screen.getByRole("alert")).toBeTruthy();
    const fresh: Approval = { revision: 5, fingerprint: sample.planNext.fingerprint };
    rerender(
      <LocalizationProvider l10n={l10n}>
        <main>{view(sample.planNext, fresh)}</main>
      </LocalizationProvider>,
    );
    expect(screen.queryByRole("alert")).toBeNull();
    expect(start().disabled).toBe(false);
  });

  it("a plan that cannot be approved explains why, offers no box, and cannot start", () => {
    const onFixPeople = vi.fn();
    inApp(view({ ...sample.plan, blocked: "someone-unmapped" }, null, { onFixPeople }));
    expect(screen.queryByRole("checkbox")).toBeNull();
    expect(said(screen.getByRole("alert"))).toContain(plain(text("plan-blocked-someone-unmapped")));
    expect(start().disabled).toBe(true);
    fireEvent.click(screen.getByRole("button", { name: text("plan-blocked-fix") }));
    expect(onFixPeople).toHaveBeenCalledTimes(1);
  });

  it("even a held approval cannot start a plan that is blocked", () => {
    inApp(view({ ...sample.plan, blocked: "invalid" }, good));
    expect(start().disabled).toBe(true);
  });

  it("the version is shown plainly, and the exact reference is behind More details", () => {
    inApp(view(sample.plan, null));
    expect(screen.getByText(text("plan-version", { revision: 4 }))).toBeTruthy();
    const details = screen.getByText(text("plan-details")).closest("details") as HTMLDetailsElement;
    expect(details.open).toBe(false);
    expect(within(details).getByText(withText(text("plan-reference", { reference: sample.planFingerprint.slice(0, 8).toUpperCase() })))).toBeTruthy();
  });

  it("Change the plan goes back to change it", () => {
    const onEdit = vi.fn();
    inApp(view(sample.plan, good, { onEdit }));
    fireEvent.click(screen.getByRole("button", { name: text("plan-edit") }));
    expect(onEdit).toHaveBeenCalledTimes(1);
  });
});

/* ---------- The main action sits in the window footer ---------- */

describe("the primary action is in the footer bar", () => {
  function inFrame(ui: ReactNode) {
    return render(
      <LocalizationProvider l10n={l10n}>
        <FooterProvider>
          <main>{ui}</main>
        </FooterProvider>
      </LocalizationProvider>,
    );
  }
  const footer = () => within(document.querySelector("footer") as HTMLElement);

  it("M02 (finished): Preview inventory; the current step is marked", () => {
    inFrame(<ScanProgress scan={sample.finished} onPause={noop} onResume={noop} onContinue={noop} />);
    expect(footer().getByRole("button", { name: text("scan-continue") })).toBeTruthy();
  });

  it("M02 (scanning): the current step has aria-current", () => {
    inFrame(<ScanProgress scan={sample.scanning} onPause={noop} onResume={noop} onContinue={noop} />);
    expect(document.querySelector('[aria-current="step"]')?.textContent).toContain(text("scan-step-usual-title"));
    expect(footer().getByRole("button", { name: text("scan-pause") })).toBeTruthy();
  });

  it("M03, M04, M05 keep Continue in the footer", () => {
    const a = inFrame(<Overview view={sample.overview} onOpen={noop} onContinue={noop} />);
    expect(footer().getByRole("button", { name: text("overview-continue") })).toBeTruthy();
    a.unmount();
    const b = inFrame(<ChooseFolders folders={sample.foldersDefault} onToggle={noop} onAddFolder={noop} onRemove={noop} onAllowAccess={noop} onContinue={noop} onBack={noop} />);
    expect(footer().getByRole("button", { name: text("files-continue") })).toBeTruthy();
    b.unmount();
    inFrame(<Destination groups={sample.groupsMapped} drives={sample.drivesMapped} onSetDestination={noop} onSetAll={noop} onChooseFolder={noop} onContinue={noop} onBack={noop} />);
    expect(footer().getByRole("button", { name: text("dest-continue") })).toBeTruthy();
  });

  it("M23: Start the move is in the footer and stays closed until the matching approval", () => {
    const good = { revision: sample.plan.revision, fingerprint: sample.plan.fingerprint };
    const { unmount } = inFrame(<PlanReview plan={sample.plan} approval={null} onApprovalChange={noop} onStart={noop} onEdit={noop} />);
    expect(footer().getByRole<HTMLButtonElement>("button", { name: text("plan-start") }).disabled).toBe(true);
    unmount();
    inFrame(<PlanReview plan={sample.plan} approval={good} onApprovalChange={noop} onStart={noop} onEdit={noop} />);
    expect(footer().getByRole<HTMLButtonElement>("button", { name: text("plan-start") }).disabled).toBe(false);
  });
});
