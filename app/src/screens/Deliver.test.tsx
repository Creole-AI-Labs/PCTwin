/**
 * The Phase 2 deliverables on the screens (Task List 2.9), against the engine's own examples:
 * what the plan says about the internet and room, the undo words, the compatibility record, the
 * "Can I actually work?" checks, "Fix these items", changes during the move, the safe stop and the
 * recovery kit. Words, not colour, tell the state; ids and generations go back as received; each
 * page is free of accessibility violations in both directions; and no message is missing.
 */
import { LocalizationProvider } from "@fluent/react";
import { act, cleanup, render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import type { ReactNode } from "react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import changedJson from "../contract/move-changed.json";
import kitJson from "../contract/kit-saved.json";
import offerJson from "../contract/repair-offer.json";
import doneJson from "../contract/repair-done.json";
import planJson from "../contract/plan-view.json";
import safeJson from "../contract/safe-stop.json";
import workJson from "../contract/work-checks.json";
import { parseChangesView, parseKitSaved, parseRepairDone, parseRepairOffer, parseSafeStop, parseWorkChecks } from "../deliver";
import { parsePlanView } from "../bridge";
import { UNDO_PROMISES } from "../guard";
import { localization } from "../i18n";
import { violations } from "../testing/a11y";
import { FooterProvider } from "./FrameFooter";
import { PromiseBadge } from "./ChangesPreview";
import { RebuildPlanSection } from "./FullMoveParts";
import PlanReview from "./PlanReview";
import MoveChanged from "./MoveChanged";
import RecoveryKit from "./RecoveryKit";
import RepairItems from "./RepairItems";
import SafeStop from "./SafeStop";
import WorkChecks from "./WorkChecks";
import { next } from "./MoveHost";

const api = vi.hoisted(() => ({
  workChecks: vi.fn(),
  workCheckAdd: vi.fn(),
  workCheckRun: vi.fn(),
  workCheckAnswer: vi.fn(),
  workCheckRemove: vi.fn(),
  repairOffer: vi.fn(),
  repairRun: vi.fn(),
  moveReapprove: vi.fn(),
  safeStop: vi.fn(),
  listenSafeStop: vi.fn(),
  recoveryKitSave: vi.fn(),
  historyList: vi.fn(),
  openStorePage: vi.fn(),
}));
vi.mock("../bridge", async (original) => ({ ...(await original<typeof import("../bridge")>()), ...api }));

const l10n = localization("en");
const text = (id: string, vars?: Record<string, string | number>) => l10n.getString(id, vars);
const plain = (s: string) => s.replace(/[⁦-⁩]/g, "");
let warnings: string[] = [];

const must = <T,>(v: T | null): T => {
  if (v === null) throw new Error("the engine's example did not parse");
  return v;
};
const work = (i: number) => must(parseWorkChecks(workJson[i]));
const offer = (i: number) => must(parseRepairOffer(offerJson[i]));
const rebuild = () => {
  const plan = must(parsePlanView(planJson[1]));
  if (!plan.rebuild) throw new Error("the example has a full move section");
  return { plan, rebuild: plan.rebuild };
};

beforeEach(() => {
  warnings = [];
  vi.spyOn(console, "warn").mockImplementation((...args: unknown[]) => {
    warnings.push(args.map(String).join(" "));
  });
  for (const f of Object.values(api)) f.mockReset();
  api.listenSafeStop.mockResolvedValue(() => undefined);
});

afterEach(() => {
  expect(warnings).toEqual([]);
  cleanup();
  vi.restoreAllMocks();
  document.documentElement.dir = "ltr";
});

const inApp = (node: ReactNode) =>
  render(
    <LocalizationProvider l10n={l10n}>
      <FooterProvider>
        <main>{node}</main>
      </FooterProvider>
    </LocalizationProvider>,
  );
const noop = () => undefined;
const axeBoth = async (ui: ReactNode) => {
  const first = inApp(ui);
  expect(await violations({ region: { enabled: false } })).toEqual([]);
  first.unmount();
  document.documentElement.dir = "rtl";
  inApp(ui);
  expect(await violations({ region: { enabled: false } })).toEqual([]);
};

describe("before PCTwin changes anything (item 1, 5)", () => {
  it("says what needs the internet and the room beyond files, with 'about'", () => {
    const { rebuild: r } = rebuild();
    inApp(<RebuildPlanSection rebuild={r} />);
    expect(screen.getByText(text("plan-net-title"))).toBeTruthy();
    expect(screen.getByText(text("plan-net-downloads", { count: r.internet.downloads }))).toBeTruthy();
    expect(screen.getByText(text("plan-net-signins", { count: r.internet.signIns }))).toBeTruthy();
    expect(screen.getByText(text("plan-net-sizes"))).toBeTruthy();
    expect(screen.getByText(/about/)).toBeTruthy();
    expect(screen.getByText(text("plan-room-title"))).toBeTruthy();
  });

  it("says the move needs no internet when nothing does", () => {
    const { rebuild: r } = rebuild();
    inApp(<RebuildPlanSection rebuild={{ ...r, internet: { downloads: 0, signIns: 0 }, room: { installs: 0, appData: 0 } }} />);
    expect(screen.getByText(text("plan-net-none"))).toBeTruthy();
    expect(screen.queryByText(text("plan-room-title"))).toBeNull();
  });

  it("shows every undo word, and 'keeps-newer' next to Update it", () => {
    const { rebuild: r } = rebuild();
    inApp(
      <>
        {UNDO_PROMISES.map((p) => (
          <PromiseBadge key={p} promise={p} />
        ))}
        <RebuildPlanSection rebuild={r} updates={[]} onUpdate={noop} />
      </>,
    );
    for (const p of UNDO_PROMISES) expect(screen.getAllByText(text(`guard-undo-${p}`)).length).toBeGreaterThan(0);
    const box = screen.getByRole("checkbox", { name: new RegExp(text("fullmove-update-it", { name: "Notes" })) });
    expect(box.closest("li")?.textContent).toContain(text("guard-undo-keeps-newer"));
  });

  it("is free of accessibility violations", async () => {
    const { rebuild: r } = rebuild();
    await axeBoth(<RebuildPlanSection rebuild={r} onCompat={noop} updates={[]} onUpdate={noop} />);
  });
});

describe("the compatibility record (item 8)", () => {
  it("lists each app's outcome in words and opens its own page, Back returns", async () => {
    const { plan } = rebuild();
    const user = userEvent.setup();
    inApp(<PlanReview plan={plan} approval={null} onApprovalChange={noop} onStart={noop} />);
    expect(screen.getAllByText(text("compat-outcome-works-with-steps")).length).toBeGreaterThan(0);
    expect(screen.getAllByText(text("compat-outcome-not-tested")).length).toBeGreaterThan(0);
    await user.click(screen.getByRole("button", { name: text("compat-open-name", { name: "Writer" }) }));
    const h1 = screen.getByRole("heading", { level: 1 });
    expect(plain(h1.textContent)).toContain("Writer");
    expect(document.activeElement).toBe(h1);
    for (const a of ["install", "data-import", "settings", "launch", "undo"]) expect(screen.getByText(text(`compat-aspect-${a}`))).toBeTruthy();
    expect(screen.getByText(text("compat-step-sign-in"))).toBeTruthy();
    expect(await violations({ region: { enabled: false } })).toEqual([]);
    await user.click(screen.getByRole("button", { name: text("frame-back") }));
    expect(screen.getByRole("heading", { level: 1 }).textContent).toBe(text("plan-title"));
  });

  it("says plainly when an app was never tested", async () => {
    const { plan } = rebuild();
    const user = userEvent.setup();
    inApp(<PlanReview plan={plan} approval={null} onApprovalChange={noop} onStart={noop} />);
    await user.click(screen.getByRole("button", { name: text("compat-open-name", { name: "Sketch Tool" }) }));
    expect(screen.getByText(text("compat-never-checked"))).toBeTruthy();
    expect(screen.queryByText(text("compat-checked-on", { date: "x" }))).toBeNull();
  });
});

describe("one app's detail (M11)", () => {
  it("says every place an app can come from and who made it", async () => {
    const { plan } = rebuild();
    const user = userEvent.setup();
    inApp(<PlanReview plan={plan} approval={null} onApprovalChange={noop} onStart={noop} />);
    await user.click(screen.getByRole("button", { name: text("compat-open-name", { name: "Writer" }) }));
    for (const k of ["winget", "microsoft-store", "homebrew", "mac-app-store", "linux-package", "flatpak", "snap", "makers-site", "old-installer", "sign-in-page"]) {
      expect(screen.getByText(text(`app-from-${k}`))).toBeTruthy();
    }
    expect(screen.getByText(text("app-maker-signed", { maker: "Example Ltd" }))).toBeTruthy();
    // PCTwin installs it itself: no store page to open.
    expect(screen.queryByRole("button", { name: text("app-store-microsoft") })).toBeNull();
    expect(await violations({ region: { enabled: false } })).toEqual([]);
  });

  it("opens the App Store's own page for an app the person installs, and claims nothing", async () => {
    const { plan } = rebuild();
    const user = userEvent.setup();
    api.openStorePage.mockResolvedValue(undefined);
    inApp(<PlanReview plan={plan} approval={null} onApprovalChange={noop} onStart={noop} />);
    await user.click(screen.getByRole("button", { name: text("compat-open-name", { name: "Sketch Tool" }) }));
    expect(screen.getByText(text("app-maker-unsigned"))).toBeTruthy();
    expect(screen.getByText(text("app-store-body"))).toBeTruthy();
    await user.click(screen.getByRole("button", { name: text("app-store-apple") }));
    expect(api.openStorePage).toHaveBeenCalledWith("macappstore://apps.apple.com/app/id409183694");
    expect(await violations({ region: { enabled: false } })).toEqual([]);
  });

  it("refuses a detail whose store page is not exactly a store's", () => {
    const good = structuredClone(planJson[1]) as unknown as { rebuild: { changes: { detail: { store: { page: string } | null } | null }[] } };
    expect(parsePlanView(good)).not.toBeNull();
    const d = good.rebuild.changes.find((c) => c.detail?.store)?.detail;
    expect(d?.store).toBeTruthy();
    if (d?.store) d.store.page = "https://example.com/app";
    expect(parsePlanView(good)).toBeNull();
  });
});

describe("Can I actually work? (item 2)", () => {
  it("gives every result exactly one label in words, as the engine sent it", async () => {
    api.workChecks.mockResolvedValue(work(1));
    inApp(<WorkChecks onBack={noop} />);
    await screen.findByText("Garden plan.svg");
    const rows = screen.getAllByRole("listitem");
    expect(rows).toHaveLength(work(1).checks.length);
    for (const c of work(1).checks) {
      const row = rows[c.index];
      const labels = ["machine-checked", "person-confirmed", "still-needs-attention"].filter((l) => within(row).queryByText(text(`work-label-${l}`)));
      expect(labels).toEqual([c.label]);
    }
    // A person's yes is never a PCTwin check.
    const mine = rows[7];
    expect(within(mine).getByText(text("work-label-person-confirmed"))).toBeTruthy();
    expect(within(mine).queryByText(text("work-label-machine-checked"))).toBeNull();
    // Missing names are shown as text.
    expect(within(rows[2]).getByText("images/front.jpg")).toBeTruthy();
  });

  it("sends an answer with the generation exactly as received, and shows the new list", async () => {
    api.workChecks.mockResolvedValue(work(1));
    api.workCheckAnswer.mockResolvedValue({ ...work(1), generation: 8 });
    api.workCheckRemove.mockResolvedValue({ ...work(1), generation: 9 });
    const user = userEvent.setup();
    inApp(<WorkChecks onBack={noop} />);
    await screen.findByText("Print my boarding pass");
    await user.click(screen.getByRole("button", { name: text("work-answer-worked-name", { name: "Print my boarding pass" }) }));
    expect(api.workCheckAnswer).toHaveBeenCalledWith(6, 7, true);
    await user.click(screen.getByRole("button", { name: text("work-remove-name", { name: "Album.m3u" }) }));
    await waitFor(() => {
      expect(api.workCheckRemove).toHaveBeenCalledWith(9, 8);
    });
  });

  it("reloads on a stale answer and says so", async () => {
    api.workChecks.mockResolvedValue(work(1));
    api.workCheckAnswer.mockRejectedValue("stale");
    const user = userEvent.setup();
    inApp(<WorkChecks onBack={noop} />);
    await screen.findByText("Print my boarding pass");
    await user.click(screen.getByRole("button", { name: text("work-answer-not-name", { name: "Print my boarding pass" }) }));
    expect(await screen.findByText(text("work-problem-stale"))).toBeTruthy();
    await waitFor(() => {
      expect(api.workChecks).toHaveBeenCalledTimes(2);
    });
  });

  it("adds a file, a font and a task, and says when no file was chosen", async () => {
    api.workChecks.mockResolvedValue(work(0));
    const user = userEvent.setup();
    inApp(<WorkChecks onBack={noop} />);
    expect(await screen.findByText(text("work-empty"))).toBeTruthy();
    api.workCheckAdd.mockRejectedValueOnce("not-chosen");
    await user.click(screen.getByRole("button", { name: text("work-add-file") }));
    expect(await screen.findByText(text("work-problem-not-chosen"))).toBeTruthy();
    expect(api.workCheckAdd).toHaveBeenLastCalledWith({ kind: "file" });
    api.workCheckAdd.mockResolvedValueOnce(work(1));
    await user.type(screen.getByLabelText(text("work-font-label")), "Inter");
    await user.click(screen.getByRole("button", { name: text("work-add-font") }));
    expect(api.workCheckAdd).toHaveBeenLastCalledWith({ kind: "font", family: "Inter" });
    await screen.findByText("Garden plan.svg");
    api.workCheckAdd.mockResolvedValueOnce(work(1));
    await user.type(screen.getByLabelText(text("work-task-label")), "Print it");
    await user.click(screen.getByRole("button", { name: text("work-add-task") }));
    expect(api.workCheckAdd).toHaveBeenLastCalledWith({ kind: "task", task: "Print it" });
  });

  it("is free of accessibility violations", async () => {
    api.workChecks.mockResolvedValue(work(1));
    const first = inApp(<WorkChecks onBack={noop} />);
    await screen.findByText("Garden plan.svg");
    expect(await violations({ region: { enabled: false } })).toEqual([]);
    first.unmount();
    document.documentElement.dir = "rtl";
    inApp(<WorkChecks onBack={noop} />);
    await screen.findByText("Garden plan.svg");
    expect(await violations({ region: { enabled: false } })).toEqual([]);
  });
});

describe("Fix these items (item 4)", () => {
  it("lists what failed and what each waits for; only what can be tried now is ticked", async () => {
    api.repairOffer.mockResolvedValue(offer(0));
    inApp(<RepairItems onBack={noop} />);
    expect((await screen.findByRole("heading", { level: 1 })).textContent).toBe(text("repair-title", { count: 4 }));
    expect(screen.getByText(new RegExp(text("repair-waits-installer-tool")))).toBeTruthy();
    const boxes = screen.getAllByRole<HTMLInputElement>("checkbox");
    expect(boxes.map((b) => b.checked)).toEqual([true, false, false, false]);
    expect(screen.getByRole("button", { name: text("repair-run", { count: 1 }) })).toBeTruthy();
  });

  it("tries only the chosen lines with the generation as received, and keeps what worked", async () => {
    api.repairOffer.mockResolvedValue(offer(0));
    api.repairRun.mockResolvedValue(must(parseRepairDone(doneJson[0])));
    const user = userEvent.setup();
    inApp(<RepairItems onBack={noop} />);
    await screen.findAllByRole("checkbox");
    await user.click(screen.getByRole("checkbox", { name: /Mail/ }));
    await user.click(screen.getByRole("button", { name: text("repair-run", { count: 2 }) }));
    expect(api.repairRun).toHaveBeenCalledWith(3, [0, 3]);
    expect(await screen.findByText(text("repair-fixed", { count: 2 }))).toBeTruthy();
    expect(screen.getByText(text("repair-body", { kept: 43 }))).toBeTruthy();
  });

  it("says in plain words when the list changed, and when nothing needs fixing", async () => {
    api.repairOffer.mockResolvedValue(offer(0));
    api.repairRun.mockRejectedValue("stale");
    const user = userEvent.setup();
    inApp(<RepairItems onBack={noop} />);
    await screen.findAllByRole("checkbox");
    await user.click(screen.getByRole("button", { name: text("repair-run", { count: 1 }) }));
    expect(await screen.findByText(text("repair-problem-stale"))).toBeTruthy();
    cleanup();
    api.repairOffer.mockResolvedValue(offer(1));
    inApp(<RepairItems onBack={noop} />);
    expect(await screen.findByText(text("repair-nothing"))).toBeTruthy();
  });

  it("is free of accessibility violations", async () => {
    api.repairOffer.mockResolvedValue(offer(0));
    const first = inApp(<RepairItems onBack={noop} />);
    await screen.findAllByRole("checkbox");
    expect(await violations({ region: { enabled: false } })).toEqual([]);
    first.unmount();
    document.documentElement.dir = "rtl";
    inApp(<RepairItems onBack={noop} />);
    await screen.findAllByRole("checkbox");
    expect(await violations({ region: { enabled: false } })).toEqual([]);
  });
});

describe("Changes during the move (item 6)", () => {
  const changes = () => must(parseChangesView(changedJson[0]));

  it("shows what changed and which steps it affects, and answers with the generation", async () => {
    // The engine answers with what was installed now (a repair result).
    api.moveReapprove.mockResolvedValue(must(parseRepairDone(doneJson[0])));
    const onDone = vi.fn();
    const user = userEvent.setup();
    inApp(<MoveChanged changes={changes()} onDone={onDone} />);
    expect(screen.getByRole("heading", { level: 1 }).textContent).toBe(text("changed-title", { count: 4 }));
    expect(screen.getByText(new RegExp(text("changed-why-app-removed")))).toBeTruthy();
    expect(screen.getByText(text("changed-held", { count: 6 }))).toBeTruthy();
    await user.click(screen.getByRole("button", { name: text("changed-approve") }));
    expect(api.moveReapprove).toHaveBeenCalledWith(2, true);
    await waitFor(() => {
      expect(onDone).toHaveBeenCalled();
    });
  });

  it("can leave them out, and says why when the engine refuses", async () => {
    api.moveReapprove.mockRejectedValue("stale");
    const onDone = vi.fn();
    const user = userEvent.setup();
    inApp(<MoveChanged changes={changes()} onDone={onDone} />);
    await user.click(screen.getByRole("button", { name: text("changed-leave-out") }));
    expect(api.moveReapprove).toHaveBeenCalledWith(2, false);
    expect(await screen.findByText(text("repair-problem-stale"))).toBeTruthy();
    expect(onDone).not.toHaveBeenCalled();
  });

  it("takes the move page when it arrives and is not left by progress", () => {
    const view = next({ kind: "wait", stage: "moving" }, { kind: "changed", changes: changes() });
    expect(view.kind).toBe("changed");
    const same = next(view, { kind: "found", bytes: 1 });
    expect(same.kind).toBe("changed");
  });

  it("is free of accessibility violations", async () => {
    await axeBoth(<MoveChanged changes={changes()} onDone={noop} />);
  });
});

describe("PCTwin's own update waits for a safe point (item 6)", () => {
  it("says what it is waiting for in words, with no percentage", async () => {
    api.safeStop.mockResolvedValue(must(parseSafeStop(safeJson[1])));
    inApp(<SafeStop onBack={noop} />);
    for (const b of ["moving", "rebuilding", "app-data", "undoing"]) expect(await screen.findByText(text(`safe-stop-busy-${b}`))).toBeTruthy();
    expect(document.body.textContent).not.toMatch(/%/);
  });

  it("says it is safe when nothing is going on, and follows the engine's event", async () => {
    api.safeStop.mockResolvedValue(must(parseSafeStop(safeJson[1])));
    let push: (v: unknown) => void = noop;
    api.listenSafeStop.mockImplementation((h: (v: unknown) => void) => {
      push = h;
      return Promise.resolve(() => undefined);
    });
    inApp(<SafeStop onBack={noop} />);
    await screen.findByText(text("safe-stop-busy-moving"));
    act(() => {
      push(must(parseSafeStop(safeJson[0])));
    });
    expect(await screen.findByText(text("safe-stop-ready"))).toBeTruthy();
  });

  it("is free of accessibility violations", async () => {
    api.safeStop.mockResolvedValue(must(parseSafeStop(safeJson[1])));
    const first = inApp(<SafeStop onBack={noop} />);
    await screen.findByText(text("safe-stop-busy-moving"));
    expect(await violations({ region: { enabled: false } })).toEqual([]);
    first.unmount();
    document.documentElement.dir = "rtl";
    inApp(<SafeStop onBack={noop} />);
    await screen.findByText(text("safe-stop-busy-moving"));
    expect(await violations({ region: { enabled: false } })).toEqual([]);
  });
});

describe("The recovery kit (item 7)", () => {
  it("says before saving that it holds no files, then shows what it holds and lacks", async () => {
    api.recoveryKitSave.mockResolvedValue(must(parseKitSaved(kitJson[0])));
    const user = userEvent.setup();
    inApp(<RecoveryKit id="1792627200000-0" onBack={noop} />);
    expect(screen.getByText(text("kit-lacks-sentence"))).toBeTruthy();
    await user.click(screen.getByRole("button", { name: text("kit-save") }));
    expect(api.recoveryKitSave).toHaveBeenCalledWith("1792627200000-0");
    expect(await screen.findByText(text("kit-part-before-wipe-list"))).toBeTruthy();
    expect(screen.getByText(text("kit-part-file-data"))).toBeTruthy();
    expect(screen.getByText(text("kit-lacks-title"))).toBeTruthy();
  });

  it("saves nothing and says so when no folder was chosen; the newest move is used for none", async () => {
    api.recoveryKitSave.mockResolvedValue(null);
    api.historyList.mockResolvedValue([
      { id: "old", when: "2026-01-01T00:00:00Z" },
      { id: "new", when: "2026-02-01T00:00:00Z" },
    ]);
    const user = userEvent.setup();
    inApp(<RecoveryKit id={null} onBack={noop} />);
    await user.click(screen.getByRole("button", { name: text("kit-save") }));
    expect(await screen.findByText(text("kit-not-saved"))).toBeTruthy();
    expect(api.recoveryKitSave).toHaveBeenCalledWith("new");
  });

  it("is free of accessibility violations", async () => {
    await axeBoth(<RecoveryKit id="x" onBack={noop} />);
  });
});
