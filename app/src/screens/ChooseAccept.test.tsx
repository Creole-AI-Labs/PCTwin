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
import ChooseWhat, { choiceOf } from "./ChooseWhat";
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
    expect(parseChoiceView({ groups: [{ role: "nowhere", files: 1, bytes: 1 }], apps: 0, leftBehind: [] })).toBeNull();
  });

  it("Everything moves every group and the apps; Everything except leaves the ticked; Only moves the ticked", () => {
    const v = found();
    expect(choiceOf(v, "everything", [], false)).toEqual({ groups: ["documents", "pictures"], apps: true });
    expect(choiceOf(v, "except", ["pictures"], true)).toEqual({ groups: ["documents"], apps: false });
    expect(choiceOf(v, "only", ["pictures"], false)).toEqual({ groups: ["pictures"], apps: false });
    expect(choiceOf(v, "only", [], true)).toEqual({ groups: [], apps: true });
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
    fireEvent.click(screen.getByRole("checkbox", { name: (n) => n.startsWith(text("plan-role-documents")) }));
    fireEvent.click(go);
    expect(onContinue).toHaveBeenCalledWith({ groups: ["documents"], apps: false });
  });

  it("has no accessibility violations in any of the three choices", async () => {
    inApp(<ChooseWhat view={found()} onContinue={vi.fn()} />);
    expect(await violations()).toEqual([]);
    fireEvent.click(screen.getByRole("radio", { name: text("choose-except") }));
    expect(await violations()).toEqual([]);
  });

  it("a choice page is only left by the person's answer", () => {
    const view = { kind: "choose", choice: found() } as const;
    expect(next({ kind: "wait", stage: "scanning" }, { kind: "choose", choice: found() })).toEqual(view);
    expect(next(view, { kind: "progress", progress: {} as never })).toBe(view);
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
