import { LocalizationProvider } from "@fluent/react";
import { cleanup, render } from "@testing-library/react";
import { violations } from "../testing/a11y";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { localization } from "../i18n";
import { GROUPS, SAMPLES } from "./samples";

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

describe("screen samples", () => {
  it("every group file is in the shared list, each with samples (a group left out would skip its checks without a word)", () => {
    // Named here, not found the same way the list finds them: a file the list missed is caught.
    // A new group file is added here too.
    const groups = ["connection.tsx", "people.tsx", "plan.tsx", "progress.tsx", "quicksend.tsx", "start.tsx"];
    expect(GROUPS.map((g) => g.file)).toEqual(groups);
    expect(GROUPS.filter((g) => g.count === 0)).toEqual([]);
  });

  it("every sample has its own id", () => {
    const ids = SAMPLES.map((s) => s.id);
    expect(new Set(ids).size).toBe(ids.length);
    expect(ids.length).toBeGreaterThan(0);
  });

  it.each(SAMPLES.map((s) => [s.id, s] as const))("%s has every string and no accessibility violations", async (_id, sample) => {
    render(
      <LocalizationProvider l10n={localization("en")}>
        <main>{sample.render()}</main>
      </LocalizationProvider>,
    );
    expect(warnings.filter((w) => /missing|not found|unknown/i.test(w))).toEqual([]);
    expect(await violations()).toEqual([]);
  });
});
