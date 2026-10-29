/**
 * Choosing on the old laptop and accepting on the new one (decided 28 October 2026): the old
 * laptop's person picks Everything, Everything except or Only what I choose; the new laptop shows
 * what is coming and accepts it with one tap.
 */
import { LocalizationProvider } from "@fluent/react";
import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import chooses from "../contract/move-choose.json";
import plans from "../contract/plan-view.json";
import { parseChoiceView, parsePlanView, type ChoiceView } from "../bridge";
import { localization } from "../i18n";
import { violations } from "../testing/a11y";
import AcceptMove from "./AcceptMove";
import { RouteStrip } from "./ProgressParts";
import Outcome from "./Outcome";
import History from "./History";
import { RebuildPlanSection } from "./FullMoveParts";
import plansFull from "../contract/plan-view.json";
import outcomes from "../contract/move-outcome.json";
import { parseMoveOutcome } from "../bridge";
import ChooseWhat, { choiceOf, keyOf, stateOf } from "./ChooseWhat";
import { next } from "./MoveHost";

const l10n = localization("en");
const text = (id: string, vars?: Record<string, string | number>) => l10n.getString(id, vars);
let warnings: string[] = [];

beforeEach(() => {
  warnings = [];
  vi.spyOn(console, "warn").mockImplementation((...args: unknown[]) => {
    warnings.push(args.map(String).join(" "));
  });
});

afterEach(() => {
  expect(warnings).toEqual([]);
  cleanup();
  vi.restoreAllMocks();
});

const inApp = (node: React.ReactNode) =>
  render(
    <LocalizationProvider l10n={l10n}>
      <main>{node}</main>
    </LocalizationProvider>,
  );

const found = (): ChoiceView => {
  const v = parseChoiceView(chooses[0]);
  if (!v) throw new Error("the engine's example parses");
  return v;
};

describe("choosing what moves, on the old laptop", () => {
  it("every example the engine writes parses, and anything else is refused", () => {
    for (const c of chooses) expect(parseChoiceView(c)).not.toBeNull();
    expect(parseChoiceView({ groups: [], apps: 1 })).toBeNull();
    expect(parseChoiceView({ groups: [], folders: [{ role: "documents", parts: [1], files: 1, bytes: 1 }], apps: 0, leftBehind: [] })).toBeNull();
    expect(parseChoiceView({ groups: [{ role: "nowhere", files: 1, bytes: 1 }], apps: 0, leftBehind: [] })).toBeNull();
    // Online-only files (2.6): offered only where this laptop can keep local copies, each cloud by a plain ID.
    const ok = { groups: [], folders: [], apps: 0, leftBehind: [], keepLocalHere: true, onlineOnly: [{ provider: "one-drive", name: "OneDrive", files: 2, bytes: 9 }] };
    expect(parseChoiceView(ok)?.onlineOnly).toEqual(ok.onlineOnly);
    expect(parseChoiceView({ ...ok, keepLocalHere: false })).toBeNull();
    expect(parseChoiceView({ ...ok, onlineOnly: [{ provider: "One Drive", name: null, files: 2, bytes: 9 }] })).toBeNull();
    expect(parseChoiceView({ ...ok, onlineOnly: [{ provider: "one-drive", name: "", files: 2, bytes: 9 }] })).toBeNull();
    expect(parseChoiceView({ ...ok, onlineOnly: [{ provider: "one-drive", name: null, files: 0, bytes: 0 }] })).toBeNull();
  });

  it("Everything moves every folder and the apps; the others move exactly the ticked folders", () => {
    const v = found();
    const all = new Set(v.folders.map(keyOf));
    expect(choiceOf(v, "everything", new Set(), false)).toEqual({ folders: v.folders.map((f) => ({ role: f.role, parts: f.parts })), apps: true });
    const work = keyOf({ role: "documents", parts: ["Work"] });
    expect(choiceOf(v, "only", new Set([work]), false)).toEqual({ folders: [{ role: "documents", parts: ["Work"] }], apps: false });
    all.delete(work);
    expect(choiceOf(v, "except", all, true).folders).not.toContainEqual({ role: "documents", parts: ["Work"] });
  });

  it("a folder with no files of its own is ticked when everything in it is (Documents holding only Lab)", () => {
    const v: ChoiceView = {
      groups: [],
      folders: [
        { role: "documents", parts: [], files: 0, bytes: 0 },
        { role: "documents", parts: ["Lab"], files: 426, bytes: 1 },
      ],
      apps: 0,
      leftBehind: [],
      onlineOnly: [],
      keepLocalHere: true,
    };
    expect(stateOf(v, v.folders[0], new Set([keyOf(v.folders[1])]))).toBe("on");
    expect(stateOf(v, v.folders[0], new Set())).toBe("off");
  });

  it("a folder with some of its folders ticked is partly ticked, and ticking it ticks all of it", () => {
    const v = found();
    const docs = v.folders.find((f) => f.role === "documents" && f.parts.length === 0);
    if (!docs) throw new Error("Documents is in the example");
    expect(stateOf(v, docs, new Set([keyOf({ role: "documents", parts: ["Work"] })]))).toBe("mixed");
    inApp(<ChooseWhat view={v} onContinue={vi.fn()} />);
    fireEvent.click(screen.getByRole("radio", { name: text("choose-only") }));
    fireEvent.click(screen.getByRole("button", { name: text("choose-open", { name: text("plan-role-documents") }) }));
    fireEvent.click(screen.getByRole("checkbox", { name: (n) => n.startsWith("Work") }));
    const box = screen.getByRole("checkbox", { name: (n) => n.startsWith(text("plan-role-documents")) });
    expect(box.getAttribute("aria-checked")).toBe("mixed");
    fireEvent.click(box);
    expect(box.getAttribute("aria-checked")).toBe("true");
  });

  it("Everything except starts with what PCTwin leaves behind anyway, caches first", () => {
    inApp(<ChooseWhat view={found()} onContinue={vi.fn()} />);
    fireEvent.click(screen.getByRole("radio", { name: text("choose-except") }));
    expect(screen.getByText(text("choose-left-caches"))).toBeTruthy();
    expect(screen.getByText(text("choose-left-rebuildable"))).toBeTruthy();
  });

  it("Only what I choose with nothing ticked cannot go on, and says why", () => {
    const onContinue = vi.fn();
    inApp(<ChooseWhat view={found()} onContinue={onContinue} />);
    fireEvent.click(screen.getByRole("radio", { name: text("choose-only") }));
    const go = screen.getByRole("button", { name: text("choose-continue") });
    expect((go as HTMLButtonElement).disabled).toBe(true);
    expect(screen.getByText(text("choose-nothing"))).toBeTruthy();
    fireEvent.click(screen.getByRole("checkbox", { name: (n) => n.startsWith(text("plan-role-pictures")) }));
    fireEvent.click(go);
    expect(onContinue).toHaveBeenCalledWith({ folders: [{ role: "pictures", parts: [] }], apps: false });
  });

  it("has no accessibility violations in any of the three choices", async () => {
    inApp(<ChooseWhat view={found()} onContinue={vi.fn()} />);
    expect(await violations()).toEqual([]);
    fireEvent.click(screen.getByRole("radio", { name: text("choose-except") }));
    fireEvent.click(screen.getByRole("button", { name: text("choose-open", { name: text("plan-role-documents") }) }));
    expect(await violations()).toEqual([]);
  });

  it("a choice page is only left by the person's answer", () => {
    const view = { kind: "choose", choice: found() } as const;
    expect(next({ kind: "wait", stage: "scanning" }, { kind: "choose", choice: found() })).toEqual(view);
    expect(next(view, { kind: "progress", progress: {} as never })).toBe(view);
  });
});

// The first real test (28 October 2026): direction marks Fluent put around names were stored and
// shown as "�". No sentence in any language carries them now.
describe("names in sentences", () => {
  it("carry no direction marks, in any language", () => {
    for (const code of ["en", "es", "fr", "pt", "hi", "zh-CN", "ar"] as const) {
      const words = localization(code).getString("accept-title", { from: "Pink Swordfish" });
      expect(words).toContain("Pink Swordfish");
      expect(words).not.toMatch(/[⁦-⁩]/);
    }
  });
});

describe("both laptops by their names", () => {
  it("the route strip names the old and the new laptop, as their people named them", () => {
    inApp(<RouteStrip laptops={{ old: { kind: "named", name: "Amun" }, new: { kind: "named", name: "Ada's new laptop" } }} />);
    expect(screen.getByText("Amun")).toBeTruthy();
    expect(screen.getByText("Ada's new laptop")).toBeTruthy();
  });
});

describe("where things landed", () => {
  it("the results page names each group's folder on the new laptop", () => {
    const outcome = parseMoveOutcome(outcomes[1]);
    if (!outcome?.landed) throw new Error("the engine's full example says where things landed");
    inApp(<Outcome outcome={outcome} onReport={vi.fn()} />);
    expect(screen.getByText(text("outcome-landed-title"))).toBeTruthy();
    expect(screen.getByText(outcome.landed[0].folder)).toBeTruthy();
    // A folder moved during the move: said in words, never its old path as if the files were there.
    const moved = outcome.landed.find((l) => l.moved);
    if (!moved) throw new Error("the engine's full example has a folder moved during the move");
    expect(screen.queryByText(moved.folder)).toBeNull();
    expect(screen.getByText(text("outcome-landed-moved"))).toBeTruthy();
  });
});

describe("undo from the history", () => {
  it("a past move that can be undone offers Undo, by its own undo book; one that cannot, does not", () => {
    const onUndo = vi.fn();
    const base = { kind: "move", when: "2026-10-28T20:22:00Z", from: "Amun", to: "Ada", state: "finished" } as const;
    inApp(
      <History
        entries={[
          { ...base, id: "1", undoId: "0123456789abcdef0123456789abcdef" },
          { ...base, id: "2", when: "2026-10-27T20:22:00Z" },
        ]}
        onOpen={vi.fn()}
        onContinue={vi.fn()}
        onUndo={onUndo}
        onExport={vi.fn()}
        onStart={vi.fn()}
        onBack={vi.fn()}
      />,
    );
    const undo = screen.getAllByRole("button", { name: (n) => n.includes(text("history-undo")) });
    expect(undo).toHaveLength(1);
    fireEvent.click(undo[0]);
    expect(onUndo).toHaveBeenCalledWith("0123456789abcdef0123456789abcdef");
  });
});

describe("updating an older app already here", () => {
  it("an older app offers Update it, off unless ticked, and the tick names its place in the plan", () => {
    const full = parsePlanView(plansFull[1])?.rebuild;
    if (!full) throw new Error("the engine's full plan example parses");
    const line = { ...full.changes[0], action: "leave-alone" as const, needs: null, update: 4 };
    const rebuild = { ...full, changes: [line] };
    const onUpdate = vi.fn();
    inApp(<RebuildPlanSection rebuild={rebuild} onUpdate={onUpdate} />);
    const name = line.title.kind === "name" ? line.title.value : text(line.title.value);
    const box: HTMLInputElement = screen.getByRole("checkbox", { name: (n) => n.startsWith(text("fullmove-update-it", { name })) });
    expect(box.checked).toBe(false);
    fireEvent.click(box);
    expect(onUpdate).toHaveBeenCalledWith(4, true);
  });
});

describe("accepting on the new laptop", () => {
  const plan = () => {
    const p = parsePlanView(plans[0]);
    if (!p) throw new Error("the engine's example parses");
    return p;
  };

  it("says who sends what, and Accept, See the details and Not now each do their one thing", () => {
    const onAccept = vi.fn();
    const onDetails = vi.fn();
    const onNotNow = vi.fn();
    inApp(<AcceptMove plan={plan()} from="Yellow Wolf" onAccept={onAccept} onDetails={onDetails} onNotNow={onNotNow} />);
    expect(screen.getByRole("heading", { level: 1 }).textContent).toContain("Yellow Wolf");
    fireEvent.click(screen.getByRole("button", { name: text("accept-details") }));
    fireEvent.click(screen.getByRole("button", { name: text("accept-not-now") }));
    expect(onDetails).toHaveBeenCalledOnce();
    expect(onNotNow).toHaveBeenCalledOnce();
    const accept: HTMLButtonElement = screen.getByRole("button", { name: text("accept-accept") });
    if (!accept.disabled) {
      fireEvent.click(accept);
      expect(onAccept).toHaveBeenCalledOnce();
    } else {
      expect(screen.getByText(text("accept-needs"))).toBeTruthy();
    }
  });

  it("lists only what changes this laptop: an app already here or a line about what stays is not coming", () => {
    const withRebuild = plans.map(parsePlanView).find((p) => p?.rebuild && p.rebuild.changes.length > 0);
    if (!withRebuild?.rebuild) throw new Error("the engine's examples include a plan with changes");
    const changes = withRebuild.rebuild.changes.map((c) => ({ ...c, action: "leave-alone" as const }));
    const p = { ...withRebuild, rebuild: { ...withRebuild.rebuild, changes } };
    inApp(<AcceptMove plan={p} from="Yellow Wolf" onAccept={vi.fn()} onDetails={vi.fn()} onNotNow={vi.fn()} />);
    expect(screen.queryByText(text("accept-settings"))).toBeNull();
  });

  it("never offers Accept while something needs the person", () => {
    const base = plan();
    if (!base.preview) throw new Error("the engine's example has its preview");
    const p = { ...base, preview: { ...base.preview, fits: false } };
    inApp(<AcceptMove plan={p} from="Yellow Wolf" onAccept={vi.fn()} onDetails={vi.fn()} onNotNow={vi.fn()} />);
    const accept: HTMLButtonElement = screen.getByRole("button", { name: text("accept-accept") });
    expect(accept.disabled).toBe(true);
    expect(screen.getByText(text("accept-needs"))).toBeTruthy();
  });
});
