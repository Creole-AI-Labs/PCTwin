import { LocalizationProvider } from "@fluent/react";
import { cleanup, render } from "@testing-library/react";
import axe from "axe-core";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { localization } from "../i18n";
import { SAMPLES } from "./samples";

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

/** axe-core on the rendered page. Colour contrast needs real layout, so it is checked in a real browser instead. */
async function violations(): Promise<string[]> {
  const result = await axe.run(document.body, { rules: { "color-contrast": { enabled: false } } });
  return result.violations.map((v) => `${v.id}: ${v.nodes.map((n) => n.target.join(" ")).join(", ")}`);
}

describe("screen samples", () => {
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
