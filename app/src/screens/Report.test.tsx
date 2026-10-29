/**
 * R02 "Your move, by kind" beyond files (decided 29 October 2026: a laptop holds more than a
 * phone): apps, settings, app data and other things, and the other people moved, each one line
 * that opens into its lines, with anything that needs the person also listed first.
 */
import { LocalizationProvider } from "@fluent/react";
import { cleanup, fireEvent, render, screen, within } from "@testing-library/react";
import type { ReactNode } from "react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import outcomes from "../contract/move-outcome.json";
import { parseMoveOutcome } from "../bridge";
import { localization } from "../i18n";
import { violations } from "../testing/a11y";
import type { MoveOutcome } from "../move";
import Report from "./Report";

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

const inApp = (node: ReactNode) =>
  render(
    <LocalizationProvider l10n={l10n}>
      <main>{node}</main>
    </LocalizationProvider>,
  );
const noop = () => undefined;

/** The engine's own full move outcome: apps, settings, app data and one other person. */
function full(): MoveOutcome {
  const o = parseMoveOutcome(outcomes[1]);
  if (!o?.rebuilt) throw new Error("the engine's example has its full move section");
  return o;
}

const head = (id: string) => screen.getByRole("button", { name: (n) => plain(n).startsWith(text(id)) });

describe("R02 beyond files", () => {
  it("shows apps, settings, app data and other people as one line each, in words", () => {
    const o = full();
    inApp(<Report items={[]} rebuilt={o.rebuilt} people={o.people} onBack={noop} />);
    // Two apps moved, plus two already here: the count names the moved ones.
    expect(plain(head("report-group-apps").textContent)).toContain(plain(text("report-count-apps", { count: 2 })));
    expect(plain(head("report-group-apps").textContent)).toContain(plain(text("report-beyond-some", { done: 0, total: 2 })));
    expect(plain(head("report-group-settings").textContent)).toContain(plain(text("report-beyond-done")));
    expect(plain(head("report-group-special").textContent)).toContain(plain(text("report-count-special", { count: 12 })));
    expect(plain(head("report-group-people").textContent)).toContain(plain(text("report-count-people", { count: 1 })));
    // Nothing is listed under a group until it is opened.
    expect(document.querySelector(".rp-body")).toBeNull();
    // No report about nothing: an empty file list does not say "no items" when apps moved.
    expect(screen.queryByText(text("report-none"))).toBeNull();
  });

  it("opens apps on their own page: each app with its result, and the apps already here as their own plain line", async () => {
    const o = full();
    const onBack = vi.fn();
    inApp(<Report items={[]} rebuilt={o.rebuilt} people={o.people} onBack={onBack} />);
    fireEvent.click(head("report-group-apps"));
    const page = screen.getByRole("heading", { level: 1 });
    expect(page.textContent).toBe(text("report-group-apps"));
    expect(document.activeElement).toBe(page);
    const body = document.querySelector<HTMLElement>(".rp-body");
    if (!body) throw new Error("the apps group opened");
    expect(within(body).getByText("Sketch Tool")).toBeTruthy();
    expect(within(body).getAllByText(text("status-needs-action")).length).toBe(2);
    expect(plain(body.textContent)).toContain(plain(text("report-here", { count: 2 })));
    expect(within(body).getByText("Paint")).toBeTruthy();
    expect(await violations()).toEqual([]);
    // Back returns to the overview, not out of the report.
    fireEvent.click(screen.getByRole("button", { name: text("frame-back") }));
    expect(onBack).not.toHaveBeenCalled();
    expect(screen.getByRole("heading", { level: 1 }).textContent).toBe(text("report-title"));
  });

  it("lists what needs the person first: apps, app data and people who did not finish", () => {
    const o = full();
    inApp(<Report items={[]} rebuilt={o.rebuilt} people={o.people} onBack={noop} />);
    const needs = o.rebuilt?.lines.filter((l) => l.status === "needs-action" || l.status === "failed").length ?? 0;
    const title = screen.getByRole("heading", { level: 2, name: (n) => plain(n) === plain(text("report-needs-title", { count: needs + 1 })) });
    const section = title.closest("section");
    if (!section) throw new Error("needs section");
    expect(within(section).getByText("Sketch Tool")).toBeTruthy();
    expect(within(section).getByText("Bea")).toBeTruthy();
    // Settings that converted fine are not in it.
    expect(within(section).queryByText("Writer settings")).toBeNull();
  });

  it("says honestly when more lines exist than were sent", () => {
    const o = full();
    const rebuilt = o.rebuilt ? { ...o.rebuilt, more: 40, hereMore: 3 } : undefined;
    inApp(<Report items={[]} rebuilt={rebuilt} onBack={noop} />);
    expect(plain(document.body.textContent)).toContain(plain(text("report-beyond-more", { count: 40 })));
    fireEvent.click(head("report-group-apps"));
    const body = document.querySelector<HTMLElement>(".rp-body");
    expect(plain(body?.textContent ?? "")).toContain(plain(text("report-here", { count: 5 })));
    expect(plain(body?.textContent ?? "")).toContain(plain(text("fullmove-more", { count: 3 })));
  });

  it("opens other people into one line each", () => {
    const o = full();
    inApp(<Report items={[]} rebuilt={o.rebuilt} people={o.people} onBack={noop} />);
    fireEvent.click(head("report-group-people"));
    const body = document.querySelector<HTMLElement>(".rp-body");
    expect(plain(body?.textContent ?? "")).toContain(plain(text("outcome-person-copied", { count: 120 })));
  });

  it("is accessible right to left, with a group open", async () => {
    document.documentElement.dir = "rtl";
    const o = full();
    inApp(<Report items={[]} rebuilt={o.rebuilt} people={o.people} onBack={noop} />);
    fireEvent.click(head("report-group-special"));
    expect(await violations()).toEqual([]);
  });
});
