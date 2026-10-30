/**
 * The full move on the move's own screens: the approval screen's apps, settings and app data
 * section (from the engine's own example), the page while they are set up, the outcome's section
 * with each result word and Undo, and where the move's events take the page.
 */
import { LocalizationProvider } from "@fluent/react";
import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import outcomes from "../contract/move-outcome.json";
import plans from "../contract/plan-view.json";
import steps from "../contract/move-rebuild.json";
import { parseMoveOutcome, parsePlanView } from "../bridge";
import { parseRebuildStep } from "../fullmove";
import { localization } from "../i18n";
import { violations } from "../testing/a11y";
import { AppDataComing, CloseApps, LiveWait, RebuildPlanSection, Rebuilding, RebuiltSection, rebuiltNeedsAction } from "./FullMoveParts";
import appdata from "../contract/move-appdata.json";
import opens from "../contract/move-appdata-open.json";
import { parseAppsOpen, parseDataProgress } from "../fullmove";
import { next } from "./MoveHost";

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
  // Every word on these pages exists in English (a missing message warns).
  expect(warnings).toEqual([]);
  cleanup();
  vi.restoreAllMocks();
});

const inApp = (node: React.ReactNode) => render(<LocalizationProvider l10n={l10n}>{node}</LocalizationProvider>);

const fullPlan = () => {
  const plan = parsePlanView(plans[1]);
  if (!plan?.rebuild) throw new Error("the engine's full move example has its section");
  return plan.rebuild;
};

const fullOutcome = () => {
  const outcome = parseMoveOutcome(outcomes[1]);
  if (!outcome?.rebuilt) throw new Error("the engine's full move outcome has its section");
  return { outcome, rebuilt: outcome.rebuilt };
};

describe("the approval screen's apps, settings and app data", () => {
  it("lists what is installed, copied, needs the person and is already here, each with what undo can do", async () => {
    const { container } = inApp(<RebuildPlanSection rebuild={fullPlan()} />);
    const body = plain(container.textContent);
    expect(body).toContain(text("fullmove-plan-title"));
    expect(body).toContain(plain(text("fullmove-group-install", { count: 2 })));
    expect(body).toContain(plain(text("fullmove-group-settings", { count: 1 })));
    expect(body).toContain(plain(text("fullmove-group-needs", { count: 4 })));
    // The app installed: PCTwin does not remove apps yet, and says so before the approval.
    expect(body).toContain(text("guard-undo-remove-yourself"));
    expect(body).toContain(text("guard-undo-removes-if-package"));
    // Settings: how many, and that some replace what is here, with undo.
    expect(body).toContain(text("fullmove-settings-replace"));
    expect(body).toContain(text("guard-undo-can-reverse"));
    // A special category's step, and PCTwin's own "not moved yet" line, never claimed done.
    expect(body).toContain(text("fullmove-step-sign-in"));
    expect(body).toContain(text("fullmove-special-not-yet"));
    expect(body).toContain(text("fullmove-step-not-yet"));
    // What each special category adds is said beside its step (2.5).
    expect(body).toContain(text("special-note-search-rebuilds"));
    expect(body).toContain(text("special-note-tool-settings-review"));
    // What stays behind is counted and said.
    expect(body).toContain(plain(text("fullmove-not-moved-private", { count: 40 })));
    expect(body).toContain(plain(text("fullmove-special-not-moved", { count: 7 })));
    expect(await violations()).toEqual([]);
  });

  it("says an older version here is left as it is, that saved website sign-ins stay behind, and counts shortcuts not followed", () => {
    const { container } = inApp(<RebuildPlanSection rebuild={fullPlan()} />);
    const body = plain(container.textContent);
    // Gap 6: an older version already here, with its version, left as it is.
    expect(body).toContain(plain(text("fullmove-here-older", { version: "2.0" })));
    // An app made for Intel Macs going to Apple silicon says so before the approval.
    expect(body).toContain(text("fullmove-app-rosetta"));
    // R1: a browser's saved sign-ins to websites are tied to the old laptop.
    expect(body).toContain(text("fullmove-step-sites"));
    // Gap 8: links and junctions are counted and said, never silently left.
    expect(body).toContain(plain(text("fullmove-links", { count: 4 })));
  });

  it("a secret is brought only when the person ticks it, one by one, and none is ticked to begin with", () => {
    const onPick = vi.fn();
    inApp(<RebuildPlanSection rebuild={fullPlan()} picked={[]} onPick={onPick} />);
    const tick = screen.getByRole("checkbox", { name: (n) => plain(n).startsWith(plain(text("fullmove-secret-bring", { name: "Browser" }))) });
    expect((tick as HTMLInputElement).checked).toBe(false);
    fireEvent.click(tick);
    expect(onPick).toHaveBeenCalledWith(2, true);
    // App data that copies is listed with its undo promise.
    expect(plain(document.body.textContent)).toContain(plain(text("fullmove-group-appdata", { count: 5 })));
  });

  it("an older old laptop's plan has no section, and the files-only plan is unchanged", () => {
    const plan = parsePlanView(plans[0]);
    expect(plan).not.toBeNull();
    expect(plan?.rebuild).toBeUndefined();
  });

  it("refuses a section with an unknown key or value", () => {
    const bad = structuredClone(plans[1]) as { rebuild: { changes: { undo: string }[] } };
    bad.rebuild.changes[0].undo = "deletes-everything";
    expect(parsePlanView(bad)).toBeNull();
    const extra = structuredClone(plans[1]) as { rebuild: Record<string, unknown> };
    extra.rebuild.approveEverything = true;
    expect(parsePlanView(extra)).toBeNull();
  });
});

describe("saved Wi-Fi networks (2.4)", () => {
  it("the plan names each network with what happens to it, and never a password", async () => {
    const plan = fullPlan();
    expect(plan.wifi.map((w) => w.step)).toEqual(["add", "add-by-profile", "skipped", "organisation", "not-this-kind", "no-password"]);
    const { container } = inApp(<RebuildPlanSection rebuild={plan} />);
    const body = plain(container.textContent);
    expect(body).toContain(plain(text("fullmove-group-wifi", { count: 6 })));
    for (const w of plan.wifi) {
      expect(body).toContain(w.name);
      expect(body).toContain(plain(text(`fullmove-wifi-step-${w.step}`)));
    }
    expect(await violations()).toEqual([]);
    // No networks: no box.
    const none = inApp(<RebuildPlanSection rebuild={{ ...plan, wifi: [] }} />);
    expect(plain(none.container.textContent)).not.toContain(plain(text("fullmove-group-wifi", { count: 0 })));
  });

  it("refuses a plan line with an unknown step or a field it does not know", () => {
    for (const bad of [{ name: "Home", step: "copy" }, { name: "", step: "add" }, { name: "Home", step: "add", password: "x" }]) {
      const p = structuredClone(plans[1]) as { rebuild: { wifi: unknown[] } };
      p.rebuild.wifi = [bad];
      expect(parsePlanView(p)).toBeNull();
    }
  });

  it("each network's result says why it needs the person, in words", () => {
    const { rebuilt } = fullOutcome();
    const wifi = rebuilt.lines.filter((l) => l.kind === "wifi");
    expect(wifi.map((l) => l.status)).toContain("copied");
    expect(wifi.map((l) => l.status)).toContain("skipped-by-user");
    const { container } = inApp(<RebuiltSection rebuilt={rebuilt} />);
    const body = plain(container.textContent);
    for (const why of ["organisation", "not-this-kind", "no-password", "did-not-arrive", "already-here", "not-added", "install-profile"]) {
      expect(body).toContain(plain(text(`fullmove-wifi-why-${why}`)));
    }
    // The finish-by-hand reason is said by the network's own words, never as app data's.
    expect(wifi.filter((l) => l.status === "needs-action").every((l) => l.notes.length === 1 && l.notes[0].note === "wifi")).toBe(true);
  });

  it("refuses a Wi-Fi note on another line, or an unknown why", () => {
    const at = (outcomes[1] as { rebuilt: { lines: { kind: string; notes: unknown[] }[] } }).rebuilt.lines.findIndex((l) => l.kind === "wifi" && l.notes.length > 0);
    expect(at).toBeGreaterThanOrEqual(0);
    const onApp = structuredClone(outcomes[1]) as { rebuilt: { lines: { kind: string; notes: unknown[] }[] } };
    onApp.rebuilt.lines[at].kind = "app";
    expect(parseMoveOutcome(onApp)).toBeNull();
    const unknown = structuredClone(outcomes[1]) as { rebuilt: { lines: { notes: { why?: string }[] }[] } };
    unknown.rebuilt.lines[at].notes[0].why = "all-good";
    expect(parseMoveOutcome(unknown)).toBeNull();
    const other = structuredClone(outcomes[1]) as { rebuilt: { lines: { notes: unknown[] }[] } };
    other.rebuilt.lines[at].notes = [{ note: "sign-in" }];
    expect(parseMoveOutcome(other)).toBeNull();
  });

  it("says when it adds the networks, and on a Mac how long it waits for the profile", () => {
    const parsed = steps.map(parseRebuildStep);
    const adding = parsed.find((s) => s?.step === "wifi") ?? null;
    const waiting = parsed.find((s) => s?.step === "wifi-profile") ?? null;
    const first = inApp(<Rebuilding step={adding} waitsForFreeWifi />);
    expect(plain(screen.getByRole("status").textContent)).toBe(plain(text("fullmove-rebuilding-wifi")));
    // Waiting for free Wi-Fi is about apps: not said while the networks are added.
    expect(plain(first.container.textContent)).not.toContain(plain(text("fullmove-rebuilding-wait-wifi")));
    first.unmount();
    inApp(<Rebuilding step={waiting} />);
    expect(plain(screen.getByRole("status").textContent)).toBe(plain(text("fullmove-rebuilding-wifi-profile", { minutes: 6 })));
    expect(parseRebuildStep({ step: "wifi-profile" })).toBeNull();
    expect(parseRebuildStep({ step: "wifi", minutesLeft: 1 })).toBeNull();
  });
});

describe("after the files", () => {
  it("says what it is setting up, measured", () => {
    const parsed = steps.map(parseRebuildStep);
    expect(parsed.every((s) => s !== null)).toBe(true);
    inApp(<Rebuilding step={parsed[2]} />);
    expect(screen.getByRole("heading", { level: 1 }).textContent).toBe(text("fullmove-rebuilding-title"));
    expect(plain(screen.getByRole("status").textContent)).toBe(plain(text("fullmove-rebuilding-settings", { done: "40", total: "120" })));
  });

  it("the move's events take the page from the progress to setting up, then to the outcome", () => {
    const progress = { kind: "progress", progress: {} } as Parameters<typeof next>[0];
    const building = next(progress, { kind: "stage", stage: { kind: "rebuilding" } });
    expect(building).toEqual({ kind: "rebuilding", step: null });
    const step = parseRebuildStep(steps[1]);
    if (!step) throw new Error("parses");
    const counted = next(building, { kind: "rebuild", step });
    expect(counted).toEqual({ kind: "rebuilding", step });
    // A late progress reading never takes the page back to the files.
    expect(next(counted, { kind: "progress", progress: {} as never })).toBe(counted);
    const done = next(counted, { kind: "finished", outcome: fullOutcome().outcome });
    expect(done.kind).toBe("outcome");
  });
});

describe("the outcome's apps, settings and app data", () => {
  it("shows each one's result word with its reason, counts what needs the person, and offers Undo", async () => {
    const { rebuilt } = fullOutcome();
    const onUndo = vi.fn();
    const { container } = inApp(<RebuiltSection rebuilt={rebuilt} onUndo={onUndo} />);
    const body = plain(container.textContent);
    expect(body).toContain(text("status-needs-action"));
    expect(body).toContain(text("status-converted"));
    expect(body).toContain(text("reason-manual-install"));
    expect(body).toContain(text("fullmove-special-finish"));
    expect(rebuiltNeedsAction(rebuilt)).toBe(rebuilt.lines.filter((l) => l.status === "needs-action").length);
    expect(rebuiltNeedsAction(rebuilt)).toBe(21);
    fireEvent.click(screen.getByRole("button", { name: text("fullmove-outcome-undo") }));
    expect(onUndo).toHaveBeenCalledOnce();
    expect(await violations()).toEqual([]);
  });

  it("an installed app with a step left says the step, and app data says why it needs the person", () => {
    const { rebuilt } = fullOutcome();
    const { container } = inApp(<RebuiltSection rebuilt={rebuilt} />);
    const body = plain(container.textContent);
    // Gap 1: a sign-in or licence is said with what to do (and the old laptop's licence details).
    expect(body).toContain(text("reason-sign-in"));
    expect(body).toContain(text("fullmove-note-licence-key"));
    expect(body).toContain(plain(text("fullmove-note-registration", { who: "Ada" })));
    expect(body).toContain(plain(text("fullmove-note-product-id", { id: "12345-OEM" })));
    expect(body).toContain(plain(text("fullmove-note-extras", { apps: "Writer Helper" })));
    // An app made for Intel Macs says it needs Rosetta.
    expect(body).toContain(text("fullmove-note-rosetta"));
    // Where its installer came from: only the yes or no, never an address.
    expect(body).toContain(plain(text("fullmove-note-came-from-elsewhere")));
    expect(body).not.toContain("http");
    expect(body).toContain(text("fullmove-outcome-fresh"));
    // Gap 2: each reason in words, with the folder for app data landed beside.
    for (const why of ["lost", "did-not-arrive", "app-open", "changing", "unreadable", "not-read", "not-whole", "not-landed"]) {
      expect(body).toContain(text(`fullmove-data-${why}`));
    }
    expect(body).toContain(plain(text("fullmove-data-beside", { folder: "Mail (from old laptop)" })));
    expect(body).toContain(plain(text("fullmove-data-placed", { folder: "Mail" })));
    // R1: website sign-ins are said on the browser's line.
    expect(body).toContain(text("fullmove-step-sites"));
    // 2.5: a special line's category notes stay on the outcome, as the plan said them.
    expect(body).toContain(text("special-note-own-sync"));
    expect(body).toContain(text("special-note-passwords-guided"));
    // Gap 7: apps already here are listed, left as they are, never counted as reinstalled.
    expect(body).toContain(plain(text("fullmove-outcome-here-title", { count: 2 })));
    expect(body).toContain(text("fullmove-outcome-here"));
    expect(body).toContain(plain(text("fullmove-outcome-here-older", { version: "2.0" })));
    expect(body).not.toContain(text("status-reinstalled"));
  });

  it("a line with no step to take says nothing more", () => {
    const { rebuilt } = fullOutcome();
    const bare = { ...rebuilt, lines: rebuilt.lines.map((l) => ({ ...l, notes: [], specialNotes: [] })), here: [], fresh: false };
    const { container } = inApp(<RebuiltSection rebuilt={bare} />);
    const body = plain(container.textContent);
    expect(body).not.toContain(text("special-note-own-sync"));
    expect(body).not.toContain(text("fullmove-outcome-fresh"));
    expect(body).not.toContain(plain(text("fullmove-outcome-here-title", { count: 2 })));
  });

  it("refuses an outcome line with an unknown note", () => {
    const bad = structuredClone(outcomes[1]) as { rebuilt: { lines: { notes: unknown[] }[] } };
    bad.rebuilt.lines[0].notes.push({ note: "all-done" });
    expect(parseMoveOutcome(bad)).toBeNull();
  });

  it("refuses an unknown special note, or one on a line that is not a special category", () => {
    const at = (outcomes[1] as { rebuilt: { lines: { kind: string; specialNotes: unknown[] }[] } }).rebuilt.lines.findIndex((l) => l.specialNotes.length > 0);
    expect(at).toBeGreaterThanOrEqual(0);
    const unknown = structuredClone(outcomes[1]) as { rebuilt: { lines: { kind: string; specialNotes: unknown[] }[] } };
    unknown.rebuilt.lines[at].specialNotes.push({ note: "all-done" });
    expect(parseMoveOutcome(unknown)).toBeNull();
    const onApp = structuredClone(outcomes[1]) as { rebuilt: { lines: { kind: string; specialNotes: unknown[] }[] } };
    onApp.rebuilt.lines[at].kind = "app";
    expect(parseMoveOutcome(onApp)).toBeNull();
    const missing = structuredClone(outcomes[1]) as { rebuilt: { lines: Record<string, unknown>[] } };
    delete missing.rebuilt.lines[at].specialNotes;
    expect(parseMoveOutcome(missing)).toBeNull();
  });

  it("offers no Undo when nothing can be put back", () => {
    const { rebuilt } = fullOutcome();
    inApp(<RebuiltSection rebuilt={{ ...rebuilt, canUndo: false }} onUndo={() => undefined} />);
    expect(screen.queryByRole("button", { name: text("fullmove-outcome-undo") })).toBeNull();
  });
});

describe("the app data before the files", () => {
  it("the new laptop says how far the old laptop's app data is, and what it waits for there", () => {
    const p = parseDataProgress(appdata[1]);
    if (!p) throw new Error("parses");
    inApp(<AppDataComing progress={p} />);
    expect(screen.getByRole("heading", { level: 1 }).textContent).toBe(text("fullmove-appdata-title"));
    expect(plain(screen.getByRole("status").textContent)).toBe(plain(text("fullmove-appdata-progress", { done: "2", total: "3" })));
    expect(plain(document.body.textContent)).toContain(plain(text("fullmove-appdata-waiting", { apps: "Chromey" })));
  });

  it("the old laptop names the apps to close, with Try again and Go on without them", () => {
    const o = parseAppsOpen(opens[0]);
    if (!o) throw new Error("parses");
    const onAnswer = vi.fn();
    inApp(<CloseApps open={o} onAnswer={onAnswer} />);
    expect(document.body.textContent).toContain("Chromey");
    fireEvent.click(screen.getByRole("button", { name: text("fullmove-close-again") }));
    expect(onAnswer).toHaveBeenCalledWith(true);
    fireEvent.click(screen.getByRole("button", { name: text("fullmove-close-without") }));
    expect(onAnswer).toHaveBeenCalledWith(false);
  });

  it("the old laptop's progress takes the page before the files, and the files' progress takes it on", () => {
    const p = parseDataProgress(appdata[0]);
    if (!p) throw new Error("parses");
    const wait = { kind: "wait", stage: "moving" } as const;
    const coming = next(wait, { kind: "appdata", progress: p });
    expect(coming.kind).toBe("appdata");
    expect(next(coming, { kind: "stage", stage: { kind: "moving" } })).toBe(coming);
    expect(next(coming, { kind: "progress", progress: {} as never }).kind).toBe("progress");
    expect(next(wait, { kind: "appdata-open", open: { apps: ["Mail"], files: [] } })).toEqual({ kind: "close-apps", open: { apps: ["Mail"], files: [] } });
    // Files an app holds open (2.5) travel to the page apart from the apps.
    expect(next(wait, { kind: "appdata-open", open: { apps: ["Outlook"], files: ["Accounts.db"] } })).toEqual({ kind: "close-apps", open: { apps: ["Outlook"], files: ["Accounts.db"] } });
  });

  // Found by the first real test (28 October 2026): progress before approval took both laptops
  // off the plan and onto "Settling you in".
  it("a page waiting on the person is never left for a progress word", () => {
    const progress = { kind: "progress", progress: {} as never } as const;
    const waiting = [
      { kind: "plan", plan: {} as never },
      { kind: "who", here: {} as never },
      { kind: "people", toPlace: {} as never },
      { kind: "privacy", places: 1 },
    ] as const;
    for (const view of waiting) expect(next(view, progress)).toBe(view);
    expect(next({ kind: "wait", stage: "moving" }, progress).kind).toBe("progress");
  });
});

describe("files an app nobody can name holds open", () => {
  it("never asks the person to find the app: says PCTwin waits, for how long at most, lists the files, and offers only Stop", () => {
    const onStop = vi.fn();
    inApp(<LiveWait wait={{ files: ["Accounts.db"], minutesLeft: 14 }} onStop={onStop} />);
    expect(screen.getByRole("heading", { name: text("fullmove-live-wait-title") })).toBeTruthy();
    expect(screen.getByRole("status").textContent).toBe(text("fullmove-live-wait-body", { minutes: 14 }));
    expect(screen.getByText("Accounts.db")).toBeTruthy();
    expect(screen.queryByText(text("fullmove-close-again"))).toBeNull();
    fireEvent.click(screen.getByRole("button", { name: text("fullmove-live-wait-stop") }));
    expect(onStop).toHaveBeenCalledOnce();
  });
});
