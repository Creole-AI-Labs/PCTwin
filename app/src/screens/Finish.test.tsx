/**
 * Finishing a move (Task List 2.7): the hub and the pages it opens, against the engine's own
 * examples. Words, not colour, tell the state; nothing irreversible is pre-ticked; every digest and
 * `shown` value goes back exactly as received; and each page is free of accessibility violations
 * in both directions.
 */
import { LocalizationProvider } from "@fluent/react";
import { act, cleanup, render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import type { ReactNode } from "react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import cleanupJson from "../contract/finish/cleanup-view.json";
import deltaJson from "../contract/finish/finish-delta.json";
import doctorJson from "../contract/finish/doctor-view.json";
import goodbyeJson from "../contract/finish/goodbye-view.json";
import handoverJson from "../contract/finish/handover-view.json";
import startedJson from "../contract/finish/handover-started.json";
import recordedJson from "../contract/finish/handover-recorded.json";
import reportJson from "../contract/finish/report-view.json";
import setupJson from "../contract/finish/setup-view.json";
import undoJson from "../contract/finish/undo-preview.json";
import viewJson from "../contract/finish/finish-view.json";
import * as fin from "../finish";
import { localization } from "../i18n";
import { violations } from "../testing/a11y";
import { FooterProvider } from "./FrameFooter";
import FinishHost from "./FinishHost";
import FinishCatchup from "./FinishCatchup";

const api = vi.hoisted(() => ({
  finishOverview: vi.fn(),
  goodbyeList: vi.fn(),
  goodbyeAnswer: vi.fn(),
  goodbyeKeepUntil: vi.fn(),
  catchupArm: vi.fn(),
  catchupApprove: vi.fn(),
  handoverPreview: vi.fn(),
  handoverConfirm: vi.fn(),
  handoverResult: vi.fn(),
  doctorRun: vi.fn(),
  setupChecklist: vi.fn(),
  setupMark: vi.fn(),
  setupSave: vi.fn(),
  undoPreview: vi.fn(),
  cleanupPreview: vi.fn(),
  cleanupConfirm: vi.fn(),
  reportPreview: vi.fn(),
  reportSave: vi.fn(),
  listenFinishDelta: vi.fn(),
  fullmoveUndo: vi.fn(),
}));
vi.mock("../bridge", async (original) => ({ ...(await original<typeof import("../bridge")>()), ...api }));
// The existing undo flow is its own screen set: here only that it is handed over to.
vi.mock("./UndoHost", () => ({ default: () => <h1>undo flow</h1> }));

const l10n = localization("en");
const text = (id: string, vars?: Record<string, string | number>) => l10n.getString(id, vars);
const plain = (s: string) => s.replace(/[⁦-⁩]/g, "");
let warnings: string[] = [];
let deltaHandler: ((d: fin.DeltaView | null) => void) | null = null;

const first = <T,>(parse: (v: unknown) => T | null, list: unknown[], pick: (x: T) => boolean = () => true): T => {
  for (const e of list) {
    const p = parse(e);
    if (p !== null && pick(p)) return p;
  }
  throw new Error("no example");
};
const view = (headline: fin.Headline = "ready-for-selected", notes?: fin.FinishNote[]): fin.FinishView => ({ ...first(fin.parseFinishView, viewJson), headline, ...(notes ? { notes } : {}) });
const goodbye = (over: Partial<fin.GoodbyeView> = {}): fin.GoodbyeView => ({ ...first(fin.parseGoodbyeView, goodbyeJson), ...over });
const ready = first(fin.parseHandoverView, handoverJson, (v) => v.kind === "ready");

beforeEach(() => {
  warnings = [];
  vi.spyOn(console, "warn").mockImplementation((...args: unknown[]) => {
    warnings.push(args.map(String).join(" "));
  });
  for (const f of Object.values(api)) f.mockReset();
  deltaHandler = null;
  api.listenFinishDelta.mockImplementation((h: (d: fin.DeltaView | null) => void) => {
    deltaHandler = h;
    return Promise.resolve(() => undefined);
  });
  api.finishOverview.mockResolvedValue(view());
  api.goodbyeList.mockResolvedValue(goodbye());
  api.setupChecklist.mockResolvedValue(first(fin.parseSetupView, setupJson));
  api.undoPreview.mockResolvedValue(first(fin.parseUndoPreview, undoJson));
  api.cleanupPreview.mockResolvedValue(first(fin.parseCleanupView, cleanupJson));
  api.doctorRun.mockResolvedValue(first(fin.parseDoctorView, doctorJson));
  api.reportPreview.mockResolvedValue(first(fin.parseReportView, reportJson));
  api.handoverPreview.mockResolvedValue(ready);
});

afterEach(() => {
  expect(warnings).toEqual([]);
  cleanup();
  vi.restoreAllMocks();
  document.documentElement.dir = "ltr";
});

const show = (ui: ReactNode) =>
  render(
    <LocalizationProvider l10n={l10n}>
      <FooterProvider>
        <main>{ui}</main>
      </FooterProvider>
    </LocalizationProvider>,
  );
const host = (id: string | null = "1792627200000-0") => show(<FinishHost id={id} onBack={() => undefined} />);
const heading = (name: string) => screen.findByRole("heading", { level: 1, name });
const open = async (user: ReturnType<typeof userEvent.setup>, stepKey: string) => {
  await user.click(await screen.findByRole("button", { name: text("finish-open-name", { title: text(`finish-step-${stepKey}`) }) }));
};

/** Every page is free of violations, left to right and right to left. */
const accessible = async () => {
  expect(await violations()).toEqual([]);
  document.documentElement.dir = "rtl";
  expect(await violations()).toEqual([]);
};

describe("Finish your move (the hub)", () => {
  it("names the move, tells its state in words, and takes focus to the heading", async () => {
    host(null);
    const h1 = await heading(text("finish-title"));
    expect(document.activeElement).toBe(h1);
    expect(api.finishOverview).toHaveBeenCalledWith(null);
    expect(screen.getByRole("heading", { level: 2, name: text("finish-headline-ready-for-selected") })).toBeTruthy();
    expect(screen.getByText(plain(text("finish-others", { count: "120" })))).toBeTruthy();
    await accessible();
  });

  it.each(["finished-with-tasks", "data-preserved-access-unresolved", "handover-unresolved", "nothing-moved"] as const)("a partial result (%s) says so and never says everything is there", async (headline) => {
    api.finishOverview.mockResolvedValue(view(headline));
    host();
    expect(await screen.findByRole("heading", { level: 2, name: text(`finish-headline-${headline}`) })).toBeTruthy();
    expect(screen.queryByText(text("finish-headline-ready-for-selected"))).toBeNull();
    expect(document.querySelector(".seal, [data-confetti]")).toBeNull();
  });

  it("shows what is not finished with its reason, the parental-control reminder, and where PCTwin did not look", async () => {
    host();
    await screen.findByText(text("finish-note-parental-controls"));
    expect(screen.getAllByText(text("reason-parts-missing")).length).toBeGreaterThan(0);
    expect(screen.getByText(text("finish-search-not-searched", { places: "E:\\" }))).toBeTruthy();
    expect(screen.getByText(plain(text("finish-more-not-shown", { count: 9 })))).toBeTruthy();
  });

  it("offers undo only while undo is open, and every step opens its own page with Back to the list", async () => {
    api.finishOverview.mockResolvedValue(view("ready-for-selected", []));
    const user = userEvent.setup();
    host();
    await screen.findByRole("heading", { level: 2, name: text("finish-headline-ready-for-selected") });
    expect(screen.queryByRole("button", { name: text("finish-open-name", { title: text("finish-step-undo") }) })).toBeNull();
    await open(user, "doctor");
    expect(document.activeElement).toBe(await heading(text("finish-doctor-title")));
    await user.click(screen.getByRole("button", { name: text("frame-back") }));
    expect(await heading(text("finish-title"))).toBeTruthy();
  });

  it("says what went wrong in words when the engine refuses", async () => {
    api.finishOverview.mockRejectedValue({ kind: "no-move" });
    host();
    expect(await screen.findByText(text("finish-problem-no-move"))).toBeTruthy();
  });
});

describe("R03 Finish setting up", () => {
  it("keeps the person's tick apart from PCTwin's own check", async () => {
    const user = userEvent.setup();
    host();
    await open(user, "setup");
    await heading(text("finish-setup-title"));
    await screen.findByText(text("finish-setup-licence-key", { name: "Mail" }));
    // PCTwin's check: a badge and no box. The person's word: a box.
    expect(screen.getAllByText(text("finish-setup-mark-machine-checked")).length).toBeGreaterThan(0);
    expect(screen.queryByRole("checkbox", { name: new RegExp(text("finish-setup-licence-key", { name: "Mail" })) })).toBeNull();
    const box = screen.getByRole("checkbox", { name: new RegExp(text("finish-setup-sign-in", { name: "Mail" })) });
    expect((box as HTMLInputElement).checked || box.getAttribute("aria-checked") === "true").toBe(false);
    const next = { ...first(fin.parseSetupView, setupJson) };
    api.setupMark.mockResolvedValue(next);
    await user.click(box);
    expect(api.setupMark).toHaveBeenCalledWith("1792627200000-0", "0000000000000000000000000000000000000000000000000000000000000001", true);
    await accessible();
  });

  it("R04 saves the list in My PCTwin history and says it is a list, not files, with no account option", async () => {
    api.setupSave.mockResolvedValue({ historyId: "1", apps: 12, settings: 40 });
    const user = userEvent.setup();
    host();
    await open(user, "setup");
    await user.click(await screen.findByRole("button", { name: text("finish-open-name", { title: text("finish-setup-save-row") }) }));
    await heading(text("finish-save-title"));
    expect(screen.getByText(text("finish-save-body"))).toBeTruthy();
    expect(screen.queryByText(/account|sign in/i)).toBeNull();
    await user.click(screen.getByRole("button", { name: text("finish-save-button") }));
    expect(await screen.findByText(plain(text("finish-save-saved", { apps: 12, settings: 40 })))).toBeTruthy();
    await accessible();
  });
});

describe("R08 Before you leave the old laptop", () => {
  it("starts with nothing answered, sends the answer with the exact `shown` value, and keeps the handover blocked in words", async () => {
    const open0 = goodbye().exceptions.map((x) => ({ ...x, resolution: { kind: "open" } as const }));
    api.goodbyeList.mockResolvedValue(goodbye({ exceptions: open0 }));
    const user = userEvent.setup();
    host();
    await open(user, "goodbye");
    await heading(text("finish-goodbye-title"));
    const g = goodbye({ exceptions: open0 });
    const e = g.exceptions[0];
    expect(screen.getAllByRole("button", { name: text("finish-goodbye-did-it") }).every((b) => b.getAttribute("aria-pressed") === "false")).toBe(true);
    expect(screen.getByText(plain(text("finish-goodbye-blocked", { count: open0.length })))).toBeTruthy();
    expect(screen.getByRole<HTMLButtonElement>("button", { name: text("finish-goodbye-handover") }).disabled).toBe(true);
    api.goodbyeAnswer.mockResolvedValue(g);
    // The first open thing with an "I accept it is lost" button.
    await user.click(screen.getAllByRole("button", { name: text("finish-goodbye-accept") })[0]);
    expect(api.goodbyeAnswer).toHaveBeenCalledWith("1792627200000-0", e.id, e.shown, "accept-loss");
    await accessible();
  });

  it("a changed list is looked at again", async () => {
    const user = userEvent.setup();
    host();
    await open(user, "goodbye");
    await heading(text("finish-goodbye-title"));
    api.goodbyeAnswer.mockRejectedValue({ kind: "changed" });
    await user.click(screen.getAllByRole("button", { name: text("finish-goodbye-accept") })[0]);
    expect(await screen.findByText(text("finish-problem-changed"))).toBeTruthy();
    await waitFor(() => {
      expect(api.goodbyeList.mock.calls.length).toBeGreaterThan(1);
    });
  });

  it("opens the handover only when everything is answered and the old laptop was checked", async () => {
    api.goodbyeList.mockResolvedValue(goodbye({ clear: true, exceptions: [] }));
    const user = userEvent.setup();
    host();
    await open(user, "goodbye");
    await screen.findByText(text("finish-goodbye-clear"));
    await user.click(screen.getByRole("button", { name: text("finish-goodbye-handover") }));
    expect(await heading(text("finish-handover-title"))).toBeTruthy();
  });

  it("saves the keep-until date the person picks, or the suggested one", async () => {
    api.goodbyeKeepUntil.mockResolvedValue(goodbye({ keepUntil: 1_796_000_000 }));
    const user = userEvent.setup();
    host();
    await open(user, "goodbye");
    await user.click(await screen.findByRole("button", { name: text("finish-goodbye-keep-use") }));
    expect(api.goodbyeKeepUntil).toHaveBeenCalledWith("1792627200000-0", goodbye().suggestedKeepUntil);
  });
});

describe("R13 Bring me up to date", () => {
  const delta = (purpose: fin.CatchupPurpose) => first(fin.parseDeltaView, deltaJson, (d) => d.purpose === purpose);

  it("arms the old laptop, shows the changes by item, and sends only what was chosen, with the digest as received", async () => {
    api.catchupArm.mockResolvedValue(undefined);
    api.catchupApprove.mockResolvedValue(undefined);
    const user = userEvent.setup();
    show(<FinishCatchup id="m" onBack={() => undefined} onPair={() => undefined} />);
    await heading(text("finish-catchup-title"));
    await user.click(screen.getByRole("button", { name: text("finish-open-name", { title: text("finish-catchup-purpose-catch-up") }) }));
    expect(api.catchupArm).toHaveBeenCalledWith("m", "catch-up");
    expect(await screen.findByText(text("finish-catchup-armed"))).toBeTruthy();
    act(() => {
      deltaHandler?.(delta("catch-up"));
    });
    await screen.findByRole("heading", { name: text("finish-catchup-changes-title") });
    const approve = screen.getByRole("button", { name: text("finish-catchup-approve", { count: "0" }) });
    expect((approve as HTMLButtonElement).disabled).toBe(true);
    // Both changed: the person must choose; the choices are the ones the contract allows.
    const choose = screen.getAllByRole("combobox")[0];
    expect(within(choose).getAllByRole("option").map((o) => o.textContent)).toEqual([text("finish-choice-none"), text("finish-choice-keep-both"), text("finish-choice-keep-new-laptop-version")]);
    await user.selectOptions(choose, "keep-both");
    expect(screen.getByText(plain(text("finish-catchup-others", { count: 3 })))).toBeTruthy();
    await user.click(screen.getByRole("button", { name: text("finish-catchup-approve", { count: "1" }) }));
    expect(api.catchupApprove).toHaveBeenCalledWith("m", delta("catch-up").digest, [{ id: delta("catch-up").lines[2].id, choice: "keep-both" }]);
    expect(await screen.findByText(text("finish-catchup-sent"))).toBeTruthy();
    await accessible();
  });

  it("only looking sends nothing and offers no approval", async () => {
    api.catchupArm.mockResolvedValue(undefined);
    const user = userEvent.setup();
    show(<FinishCatchup id={null} onBack={() => undefined} />);
    await user.click(await screen.findByRole("button", { name: text("finish-open-name", { title: text("finish-catchup-purpose-check") }) }));
    act(() => {
      deltaHandler?.(delta("check"));
    });
    await screen.findByText(text("finish-catchup-check-only"));
    expect(screen.queryByRole("button", { name: /^Send the changes/ })).toBeNull();
    expect(screen.queryByRole("combobox")).toBeNull();
  });

  it("says so when the old laptop's answer cannot be read", async () => {
    api.catchupArm.mockResolvedValue(undefined);
    const user = userEvent.setup();
    show(<FinishCatchup id={null} onBack={() => undefined} />);
    await user.click(await screen.findByRole("button", { name: text("finish-open-name", { title: text("finish-catchup-purpose-check") }) }));
    act(() => {
      deltaHandler?.(null);
    });
    expect(await screen.findByText(text("finish-catchup-unreadable"))).toBeTruthy();
  });
});

describe("Hand the old laptop on (R08 to R10)", () => {
  const toPreview = async (user: ReturnType<typeof userEvent.setup>, which: "remove" | "erase") => {
    api.goodbyeList.mockResolvedValue(goodbye({ clear: true, exceptions: [] }));
    host();
    await open(user, "handover");
    await heading(text("finish-handover-title"));
    await user.click(screen.getByRole("button", { name: text("finish-open-name", { title: text(`finish-handover-${which}-title`) }) }));
  };

  it("previews exactly what is removed and kept, the system's route, and the plain SSD note", async () => {
    const user = userEvent.setup();
    await toPreview(user, "erase");
    await heading(text("finish-handover-preview-erase"));
    expect(screen.getByText("Samsung SSD 980")).toBeTruthy();
    expect(screen.getByText(text("finish-handover-keeps-none"))).toBeTruthy();
    expect(screen.getByText(text("finish-route-windows-clean-drive"))).toBeTruthy();
    expect(screen.getByText(text("finish-hnote-ssd-not-secure-wipe"))).toBeTruthy();
    expect(api.handoverPreview).toHaveBeenCalledWith("1792627200000-0", "erase-old-laptop");
    await accessible();
  });

  it("confirms on its own page: unticked, no typed phrase, no countdown, and a separate button that waits for the tick", async () => {
    api.handoverConfirm.mockResolvedValue(first(fin.parseHandoverStarted, startedJson));
    const user = userEvent.setup();
    await toPreview(user, "erase");
    await user.click(await screen.findByRole("button", { name: text("finish-handover-continue") }));
    await heading(text("finish-confirm-title-erase-old-laptop"));
    expect(screen.queryByRole("textbox")).toBeNull();
    const box = screen.getByRole("checkbox", { name: new RegExp("Samsung SSD 980") });
    expect(box.getAttribute("aria-checked") === "true" || (box as HTMLInputElement).checked).toBe(false);
    const go = screen.getByRole<HTMLButtonElement>("button", { name: text("finish-confirm-go") });
    expect(go.disabled).toBe(true);
    await accessible();
    document.documentElement.dir = "ltr";
    await user.click(box);
    expect(go.disabled).toBe(false);
    await user.click(go);
    expect(api.handoverConfirm).toHaveBeenCalledWith("1792627200000-0", "erase-old-laptop", ready.kind === "ready" ? ready.digest : "");
    // The system's own route, then what the person saw: recorded as their word, never a certificate.
    await heading(text("finish-started-title"));
    expect(screen.getByText(text("finish-started-undo-closed"))).toBeTruthy();
    api.handoverResult.mockResolvedValue(first(fin.parseHandoverRecorded, recordedJson));
    await user.selectOptions(screen.getByRole("combobox"), "cannot-tell");
    await user.click(screen.getByRole("button", { name: text("finish-says-record") }));
    expect(api.handoverResult).toHaveBeenCalledWith("1792627200000-0", "erase-old-laptop", "cannot-tell");
    expect(await screen.findByText(text("finish-recorded-person-says-done"))).toBeTruthy();
    expect(screen.getByText(text("finish-hnote-not-a-certificate"))).toBeTruthy();
  });

  it("a refusal says why and that nothing was done, and points back to the list", async () => {
    api.handoverPreview.mockResolvedValue({ kind: "refused", action: "remove-my-account", why: "list-not-clear", open: 2 });
    const user = userEvent.setup();
    await toPreview(user, "remove");
    expect(await screen.findByText(new RegExp(text("finish-refused-list-not-clear")))).toBeTruthy();
    expect(screen.getByText(plain(text("finish-handover-refused-open", { count: 2 })))).toBeTruthy();
    expect(screen.queryByRole("button", { name: text("finish-handover-continue") })).toBeNull();
    await accessible();
  });

  it("a confirmation that no longer matches goes back to look again", async () => {
    api.handoverConfirm.mockRejectedValue({ kind: "changed" });
    const user = userEvent.setup();
    await toPreview(user, "erase");
    await user.click(await screen.findByRole("button", { name: text("finish-handover-continue") }));
    await user.click(await screen.findByRole("checkbox"));
    await user.click(screen.getByRole("button", { name: text("finish-confirm-go") }));
    await heading(text("finish-handover-preview-erase"));
    expect(api.handoverPreview.mock.calls.length).toBeGreaterThan(1);
  });
});

describe("R05 Review before undo", () => {
  it("shows what undo removes and keeps, then hands over to the existing undo with the undo ID", async () => {
    api.fullmoveUndo.mockResolvedValue(undefined);
    const user = userEvent.setup();
    host();
    await open(user, "undo");
    await heading(text("finish-undo-title"));
    expect(screen.getByText(/7-Zip/)).toBeTruthy();
    expect(screen.getByText(/Chromey/)).toBeTruthy();
    expect(screen.getByText(text("finish-undo-keeps"))).toBeTruthy();
    await accessible();
    document.documentElement.dir = "ltr";
    await user.click(screen.getByRole("button", { name: text("finish-undo-start") }));
    expect(api.fullmoveUndo).toHaveBeenCalledWith("01010101010101010101010101010101");
    expect(await screen.findByText("undo flow")).toBeTruthy();
  });

  it("cannot be started once undo is closed", async () => {
    api.undoPreview.mockResolvedValue({ ...first(fin.parseUndoPreview, undoJson), open: false });
    const user = userEvent.setup();
    host();
    await open(user, "undo");
    await screen.findByText(text("finish-undo-closed"));
    expect(screen.getByRole<HTMLButtonElement>("button", { name: text("finish-undo-start") }).disabled).toBe(true);
  });
});

describe("R07 Clean up", () => {
  it("asks each question on its own page and removes nothing until it is ticked and confirmed, with the digest as received", async () => {
    api.cleanupConfirm.mockResolvedValue({ what: "temporary-files", removed: 3, kept: 1, failed: 0 });
    const user = userEvent.setup();
    host();
    await open(user, "cleanup");
    await heading(text("finish-cleanup-title"));
    expect(api.cleanupPreview).not.toHaveBeenCalled();
    await user.click(screen.getByRole("button", { name: text("finish-open-name", { title: text("finish-cleanup-temp") }) }));
    await heading(text("finish-cleanup-temp"));
    expect(screen.getByText("pctwin-installs-0011223344556677")).toBeTruthy();
    expect(screen.getByText("pctwin-notes")).toBeTruthy();
    const go = screen.getByRole<HTMLButtonElement>("button", { name: text("finish-cleanup-confirm") });
    expect(go.disabled).toBe(true);
    await accessible();
    document.documentElement.dir = "ltr";
    await user.click(screen.getByRole("checkbox"));
    await user.click(go);
    expect(api.cleanupConfirm).toHaveBeenCalledWith("temporary-files", "000000000000000000000000000000000000000000000000000000000000000b");
    expect(await screen.findByText(plain(text("finish-cleanup-done", { removed: 3, kept: 1, failed: 0 })))).toBeTruthy();
  });
});

describe("Connection doctor", () => {
  it("says what it found in plain words, the phone hotspot, and what it could not check", async () => {
    const user = userEvent.setup();
    host();
    await open(user, "doctor");
    await heading(text("finish-doctor-title"));
    await user.click(screen.getByRole("button", { name: text("finish-doctor-run") }));
    expect(api.doctorRun).toHaveBeenCalledWith("new");
    await screen.findByText(plain(text("doctor-windows-public-network", { names: "Wi-Fi" })));
    expect(screen.getByText(text("finish-doctor-hotspot-first"))).toBeTruthy();
    expect(screen.getByText(text("finish-notchecked-doctor-probe-hears-itself"))).toBeTruthy();
    await accessible();
  });
});

describe("R11 Share the report", () => {
  it("includes nothing until chosen, previews the exact text, and saves the text the person read", async () => {
    api.reportSave.mockResolvedValue("C:/Users/Ada/report.txt");
    const user = userEvent.setup();
    host();
    await open(user, "report");
    await heading(text("finish-report-title"));
    for (const k of ["file-names", "app-names", "account-id"]) {
      expect(screen.getByRole("checkbox", { name: text(`finish-report-${k}`) }).getAttribute("aria-checked") === "true").toBe(false);
    }
    expect(api.reportPreview).toHaveBeenCalledWith("1792627200000-0", { fileNames: false, appNames: false, accountId: false });
    await screen.findByText(text("finish-report-save"));
    const preview = document.querySelector("pre.fin-text") as HTMLElement;
    expect(preview.textContent).toContain(text("finish-report-text-title"));
    expect(preview.textContent).toContain(text("finish-report-text-unnamed-file"));
    await user.click(screen.getByRole("checkbox", { name: text("finish-report-app-names") }));
    await waitFor(() => {
      expect(api.reportPreview).toHaveBeenLastCalledWith("1792627200000-0", { fileNames: false, appNames: true, accountId: false });
    });
    await screen.findByText(text("finish-report-save"));
    const shown = (document.querySelector("pre.fin-text") as HTMLElement).textContent;
    await user.click(screen.getByRole("button", { name: text("finish-report-save") }));
    expect(api.reportSave).toHaveBeenCalledWith("1792627200000-0", { fileNames: false, appNames: true, accountId: false }, "000000000000000000000000000000000000000000000000000000000000000d", shown);
    expect(await screen.findByText(plain(text("finish-report-saved", { path: "C:/Users/Ada/report.txt" })))).toBeTruthy();
    await accessible();
  });

  it("offers no professional, PDF or signed report and no account", async () => {
    const user = userEvent.setup();
    host();
    await open(user, "report");
    await heading(text("finish-report-title"));
    expect(screen.queryByText(/\b(pdf|signed|professional|upgrade)\b/i)).toBeNull();
  });
});
