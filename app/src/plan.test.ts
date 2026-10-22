import { describe, expect, it } from "vitest";
import plans from "./contract/plan-view.json";
import { parsePlanView } from "./bridge";
import { overviewOf } from "./plan";

describe("the overview (M03) of a received plan", () => {
  const plan = parsePlanView(plans[0]);
  if (!plan) throw new Error("the engine's plan example does not parse");

  it("counts what is chosen from the plan's own sums, and nothing it cannot know", () => {
    expect(overviewOf(plan)).toEqual({
      chosen: { folders: 1, files: 34, bytes: 1_200_000, atLeast: false },
      unreadable: 0,
      destinationProblems: 0,
      peopleWaiting: 0,
    });
  });

  it("a plan that does not fit is a destination problem; one held for people has people waiting", () => {
    const short = { ...plan, preview: plan.preview && { ...plan.preview, fits: false } };
    expect(overviewOf(short).destinationProblems).toBe(1);
    expect(overviewOf({ ...plan, blocked: "someone-unmapped" as const }).peopleWaiting).toBe(1);
    expect(overviewOf({ ...plan, blocked: "account-used-twice" as const }).peopleWaiting).toBe(1);
    expect(overviewOf({ ...plan, blocked: "invalid" as const }).peopleWaiting).toBe(0);
  });
});
