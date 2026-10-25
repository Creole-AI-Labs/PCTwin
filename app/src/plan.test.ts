import { describe, expect, it } from "vitest";
import plans from "./contract/plan-view.json";
import { parsePlanView } from "./bridge";
import { attentionCount, overviewOf } from "./plan";

describe("the overview (M03) of a received plan", () => {
  const plan = parsePlanView(plans[0]);
  if (!plan) throw new Error("the engine's plan example does not parse");

  it("counts what is chosen from the plan's own sums, and nothing it cannot know", () => {
    expect(overviewOf(plan)).toEqual({
      chosen: { folders: 1, files: 34, bytes: 1_200_000, atLeast: false },
      unreadable: 0,
      destinationProblems: 0,
      peopleWaiting: 0,
      // The example plan has one file that would run at sign-in.
      filesNeedingLook: 1,
    });
  });

  it("needs attention exactly when the plan page warns: files that won't go as they are, and space short or unknown", () => {
    const preview = plan.preview;
    if (!preview) throw new Error("the example plan has a preview");
    const clean = { ...plan, preview: { ...preview, tooBig: 0, refused: 0, runsAtSignIn: 0, fits: true } };
    expect(attentionCount(overviewOf(clean))).toBe(0);
    const files = { ...clean, preview: { ...clean.preview, tooBig: 3, refused: 2, runsAtSignIn: 1 } };
    expect(attentionCount(overviewOf(files))).toBe(6);
    expect(overviewOf({ ...clean, preview: { ...clean.preview, fits: null } }).destinationProblems).toBe(1);
  });

  it("a plan that does not fit is a destination problem; one held for people has people waiting", () => {
    const short = { ...plan, preview: plan.preview && { ...plan.preview, fits: false } };
    expect(overviewOf(short).destinationProblems).toBe(1);
    expect(overviewOf({ ...plan, blocked: "someone-unmapped" as const }).peopleWaiting).toBe(1);
    expect(overviewOf({ ...plan, blocked: "account-used-twice" as const }).peopleWaiting).toBe(1);
    expect(overviewOf({ ...plan, blocked: "invalid" as const }).peopleWaiting).toBe(0);
  });
});
