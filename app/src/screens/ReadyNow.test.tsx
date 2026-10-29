/**
 * X03 Use your new laptop: while the move goes on, the new laptop says which groups are already
 * copied and checked, so the person can use them now. It is a milestone, never the end of the
 * move: shown only while something is still coming, and only on the new laptop.
 */
import { LocalizationProvider } from "@fluent/react";
import { cleanup, render, screen } from "@testing-library/react";
import type { ReactNode } from "react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { localization } from "../i18n";
import * as sample from "../moveSamples";
import { counts } from "../moveSamples";
import type { MoveProgress } from "../move";
import { violations } from "../testing/a11y";
import { FooterProvider } from "./FrameFooter";
import ProgressCompact from "./ProgressCompact";

const l10n = localization("en");
const text = (id: string, vars?: Record<string, string | number>) => l10n.getString(id, vars);
const plain = (s: string) => s.replace(/[⁦-⁩]/g, "");
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
  document.documentElement.dir = "ltr";
});

const noop = () => undefined;
const show = (progress: MoveProgress, old = false): void => {
  const ui: ReactNode = (
    <ProgressCompact
      progress={progress}
      onPause={noop}
      onResume={noop}
      onSeeNeedsAction={noop}
      onMinimise={noop}
      tips={{ old, onBattery: false, slow: false }}
    />
  );
  render(
    <LocalizationProvider l10n={l10n}>
      <FooterProvider>
        <main>{ui}</main>
      </FooterProvider>
    </LocalizationProvider>,
  );
};

const ready = () => screen.queryByRole("heading", { level: 2, name: text("progress-ready-title") });

describe("X03 Use your new laptop", () => {
  it("names the groups already copied and checked, and says the rest is still coming", async () => {
    show(sample.running);
    const heading = ready();
    expect(heading).not.toBeNull();
    const box = heading?.closest("section");
    const body = plain(box?.textContent ?? "");
    expect(body).toContain(text("plan-role-documents"));
    // Pictures are still copying: not ready.
    expect(body).not.toContain(text("plan-role-pictures"));
    expect(await violations()).toEqual([]);
  });

  it("is not shown on the old laptop, which only sends", () => {
    show(sample.running, true);
    expect(ready()).toBeNull();
  });

  it("is not shown before anything is checked", () => {
    show(sample.starting);
    expect(ready()).toBeNull();
  });

  it("is not the end of the move: not shown once nothing is still coming", () => {
    const done: MoveProgress = {
      ...sample.running,
      groups: sample.running.groups.map((g) => ({ ...g, counts: counts({ done: 10 }) })),
    };
    show(done);
    expect(ready()).toBeNull();
  });

  it("a group with something that needs the person is not called ready", () => {
    const mixed: MoveProgress = {
      ...sample.running,
      groups: [
        { id: "a", name: { kind: "role", role: "documents" }, counts: counts({ done: 5, needsAction: 1 }) },
        { id: "b", name: { kind: "folder", name: "Tax 2023" }, counts: counts({ done: 3 }) },
        { id: "c", name: { kind: "role", role: "music" }, counts: counts({ waiting: 3 }) },
      ],
    };
    show(mixed);
    const body = plain(ready()?.closest("section")?.textContent ?? "");
    expect(body).toContain("Tax 2023");
    expect(body).not.toContain(text("plan-role-documents"));
  });

  it("is accessible right to left", async () => {
    document.documentElement.dir = "rtl";
    show(sample.running);
    expect(await violations()).toEqual([]);
  });
});
