import { LocalizationProvider } from "@fluent/react";
import type { ReactNode } from "react";
import { act, cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import axe from "axe-core";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import ended from "../contract/ended-early.json";
import outcomes from "../contract/move-outcome.json";
import placeAnswers from "../contract/place-answer.json";
import plans from "../contract/plan-view.json";
import progresses from "../contract/move-progress.json";
import reportItems from "../contract/report-item.json";
import { localization } from "../i18n";
import MoveHost, { next } from "./MoveHost";
import Paired from "./Paired";

const tauri = vi.hoisted(() => ({
  invoke: vi.fn(),
  handlers: new Map<string, (event: { payload: unknown }) => void>(),
}));

vi.mock("@tauri-apps/api/core", () => ({ invoke: tauri.invoke }));
vi.mock("@tauri-apps/api/event", () => ({
  listen: (name: string, handler: (event: { payload: unknown }) => void) => {
    tauri.handlers.set(name, handler);
    return Promise.resolve(() => {
      tauri.handlers.delete(name);
    });
  },
}));

const l10n = localization("en");
const text = (id: string, vars?: Record<string, string | number>) => l10n.getString(id, vars);
/** Fluent isolates variables for right-to-left safety; the marks are invisible, so tests compare without them. */
const plain = (s: string) => s.replace(/[⁦-⁩]/g, "");
const withText = (expected: string) => (content: string) => plain(content) === plain(expected);

let warnings: string[] = [];
let replies: Partial<Record<string, (args: unknown) => unknown>> = {};

beforeEach(() => {
  warnings = [];
  replies = {};
  tauri.handlers.clear();
  tauri.invoke.mockReset();
  tauri.invoke.mockImplementation((command: string, args?: unknown) => Promise.resolve().then(() => replies[command]?.(args)));
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

function inApp(ui: ReactNode) {
  return render(
    <LocalizationProvider l10n={l10n}>
      <main>{ui}</main>
    </LocalizationProvider>,
  );
}

/** What a rejected move command carries from the engine: a plain object, as the shell sends it. */
const refusal = (kind: string) => ({ kind }) as unknown as Error;

/** The people the move was started with, once it was. */
const beganWith = () => (tauri.invoke.mock.calls.find(([c]) => c === "move_begin")?.[1] as { others: string[] } | undefined)?.others;

/** The engine sends one event. */
function send(event: string, payload: unknown) {
  act(() => {
    tauri.handlers.get(event)?.({ payload });
  });
}

async function host(old: boolean) {
  const onPairAgain = vi.fn();
  const onHome = vi.fn();
  inApp(<MoveHost old={old} onPairAgain={onPairAgain} onHome={onHome} />);
  await waitFor(() => {
    expect(tauri.invoke).toHaveBeenCalledWith("move_begin", expect.anything());
  });
  return { onPairAgain, onHome };
}

const [running, pausing] = progresses;
/** The running example without its drive problem. */
const runningQuiet = Object.fromEntries(Object.entries(running).filter(([key]) => key !== "notice"));
const [linkLost, notApproved, failed] = ended;
const plan = plans[0];

describe("the move, once paired", () => {
  it("starts once, after listening, and names the folders a new laptop may lack in the person's words", async () => {
    await host(false);
    expect(tauri.handlers.size).toBe(8);
    const begins = tauri.invoke.mock.calls.filter(([c]) => c === "move_begin");
    expect(begins).toHaveLength(1);
    const { fallback } = begins[0][1] as { fallback: Record<string, string> };
    expect(fallback.music).toBe(`${text("move-fallback-folder")}/${text("plan-role-music")}`);
    expect(Object.keys(fallback).sort()).toEqual(["desktop", "documents", "downloads", "music", "pictures", "videos"]);
  });

  it("old laptop: says it is finding the files, then waiting for the approval; Stop stops", async () => {
    await host(true);
    expect(screen.getByRole("heading", { level: 1 }).textContent).toBe(text("move-wait-scanning-title"));
    // What the scan has found of the mover's own things so far, as it grows.
    send("move-found", { bytes: 2_500_000 });
    expect(screen.getByText((t) => plain(t) === plain(text("move-found", { size: "2.5 MB" })))).toBeTruthy();
    send("move-stage", { kind: "waiting" });
    expect(screen.getByRole("heading", { level: 1 }).textContent).toBe(text("move-wait-approval-title"));
    // While it waits, the old laptop offers the checklist; ticks are the person's own and stay.
    fireEvent.click(screen.getByRole("button", { name: text("move-before-open") }));
    expect(screen.getByRole("heading", { level: 1 }).textContent).toBe(text("before-title"));
    const tick = screen.getAllByRole("checkbox")[0] as HTMLInputElement;
    fireEvent.click(tick);
    fireEvent.click(screen.getByRole("button", { name: text("before-continue") }));
    expect(screen.getByRole("heading", { level: 1 }).textContent).toBe(text("move-wait-approval-title"));
    fireEvent.click(screen.getByRole("button", { name: text("move-before-open") }));
    expect((screen.getAllByRole("checkbox")[0] as HTMLInputElement).checked).toBe(true);
    fireEvent.click(screen.getByRole("button", { name: text("before-continue") }));
    fireEvent.click(screen.getByRole("button", { name: text("move-stop") }));
    expect(tauri.invoke).toHaveBeenCalledWith("move_stop");
  });

  it("new laptop: shows the plan with what was found before the move, and approves exactly that revision", async () => {
    await host(false);
    expect(screen.getByRole("heading", { level: 1 }).textContent).toBe(text("move-wait-list-title"));
    send("move-plan", plan);
    expect(screen.getByText(text("plan-preview-fits"))).toBeTruthy();
    expect(screen.getByText(withText(text("plan-preview-time", { low: 4, high: 8 })))).toBeTruthy();
    expect(screen.getByText(withText(text("plan-preview-renamed", { count: 1 })))).toBeTruthy();
    expect(screen.getByText(withText(text("plan-preview-runs-at-sign-in", { count: 1 })))).toBeTruthy();
    // There is no plan editing in this flow yet, so no Edit button that goes nowhere.
    expect(screen.queryByRole("button", { name: text("plan-edit") })).toBeNull();
    const start = screen.getByRole("button", { name: text("plan-start") });
    expect((start as HTMLButtonElement).disabled).toBe(true);
    fireEvent.click(screen.getByRole("checkbox", { name: text("plan-approve") }));
    fireEvent.click(start);
    await waitFor(() => {
      expect(tauri.invoke).toHaveBeenCalledWith("move_approve", { plan: { revision: plan.revision, fingerprint: plan.fingerprint }, approve: true, leftOut: [] });
    });
    await waitFor(() => {
      expect(screen.getByRole("heading", { level: 1 }).textContent).toBe(text("move-wait-starting-title"));
    });
  });

  it("a folder the person unticks stays on the old laptop: the approval says so, and the total leaves it out", async () => {
    await host(false);
    send("move-plan", plan);
    fireEvent.click(screen.getByRole("checkbox", { name: text("plan-group-move", { folder: text("plan-role-documents") }) }));
    fireEvent.click(screen.getByRole("checkbox", { name: text("plan-approve") }));
    fireEvent.click(screen.getByRole("button", { name: text("plan-start") }));
    await waitFor(() => {
      expect(tauri.invoke).toHaveBeenCalledWith("move_approve", { plan: { revision: plan.revision, fingerprint: plan.fingerprint }, approve: true, leftOut: ["documents"] });
    });
  });

  it("a folder sent to another place shows where and its free space, holds the start when it will not fit, and goes back to the usual place", async () => {
    await host(false);
    send("move-plan", plan);
    const [roomy] = placeAnswers;
    // A place too small for the group.
    replies.move_choose_place = () => ({ ...roomy, freeBytes: 1000 });
    fireEvent.click(screen.getByRole("button", { name: text("plan-place-change") }));
    await waitFor(() => {
      expect(tauri.invoke).toHaveBeenCalledWith("move_choose_place", { role: "documents" });
    });
    const where = (roomy as { path: string }).path;
    await waitFor(() => {
      expect(screen.getByText((t) => t === text("plan-group-to", { place: where }))).toBeTruthy();
    });
    expect(screen.getAllByRole("alert").map((a) => a.textContent)).toContain(text("plan-place-no-room"));
    fireEvent.click(screen.getByRole("checkbox", { name: text("plan-approve") }));
    expect(screen.getByRole<HTMLButtonElement>("button", { name: text("plan-start") }).disabled).toBe(true);
    // A drive with no safe way to name files is said so, and the place stays as it was.
    replies.move_choose_place = () => placeAnswers[3];
    fireEvent.click(screen.getByRole("button", { name: text("plan-place-change") }));
    await waitFor(() => {
      expect(screen.getAllByRole("alert").map((a) => a.textContent)).toContain(text("plan-place-no-safe-name"));
    });
    fireEvent.click(screen.getByRole("button", { name: text("plan-place-usual") }));
    await waitFor(() => {
      expect(tauri.invoke).toHaveBeenCalledWith("move_usual_place", { role: "documents" });
    });
    await waitFor(() => {
      expect(screen.queryByText(text("plan-place-no-room"))).toBeNull();
    });
    expect(screen.getByRole<HTMLButtonElement>("button", { name: text("plan-start") }).disabled).toBe(false);
  });

  it("a plan that changed under the approval says so and does not start", async () => {
    replies.move_approve = () => {
      throw refusal("plan-changed");
    };
    await host(false);
    send("move-plan", plan);
    fireEvent.click(screen.getByRole("checkbox", { name: text("plan-approve") }));
    fireEvent.click(screen.getByRole("button", { name: text("plan-start") }));
    await waitFor(() => {
      expect(screen.getByText(text("plan-changed"))).toBeTruthy();
    });
    expect(screen.getByRole("heading", { level: 1 }).textContent).toBe(text("plan-title"));
  });

  it("going back from the plan declines it", async () => {
    await host(false);
    send("move-plan", plan);
    fireEvent.click(screen.getByRole("button", { name: text("frame-back") }));
    expect(tauri.invoke).toHaveBeenCalledWith("move_approve", { plan: { revision: plan.revision, fingerprint: plan.fingerprint }, approve: false, leftOut: [] });
  });

  it("shows the measured progress, then pausing and paused with a way to stop that asks first", async () => {
    await host(true);
    send("move-progress", runningQuiet);
    expect(screen.getByRole("heading", { level: 1 }).textContent).toBe(text("progress-title"));
    fireEvent.click(screen.getByRole("button", { name: text("progress-pause") }));
    expect(tauri.invoke).toHaveBeenCalledWith("move_pause");
    send("move-progress", pausing);
    // The engine's example pause came from the battery's safe limit, and the page says so.
    expect(screen.getByText(text("progress-battery-paused"))).toBeTruthy();
    fireEvent.click(screen.getByRole("button", { name: text("move-stop") }));
    // The stop dialog (X10) asks first; nothing is stopped yet.
    expect(tauri.invoke).not.toHaveBeenCalledWith("move_stop");
    expect(screen.getByRole("dialog")).toBeTruthy();
  });

  it("a drive that is not keeping files holds the move (X07): carry on or stop, nothing dropped", async () => {
    await host(false);
    send("move-progress", running);
    expect(screen.getByRole("heading", { level: 1 }).textContent).toBe(text("drive-problem-title"));
    expect(screen.getByText(withText(text("drive-problem-body", { place: text("plan-role-pictures") })))).toBeTruthy();
    expect(screen.getByText(text("drive-problem-fix"))).toBeTruthy();
    fireEvent.click(screen.getByRole("button", { name: text("drive-problem-carry-on") }));
    expect(tauri.invoke).toHaveBeenCalledWith("move_resume");
  });

  it("ends with the outcome, then the report the engine gives", async () => {
    replies.move_report = () => reportItems;
    const { onHome } = await host(false);
    send("move-progress", runningQuiet);
    send("move-finished", outcomes[0]);
    expect(screen.getByRole("heading", { level: 1 }).textContent).toBe(text("outcome-title"));
    // A late progress word does not take the page back.
    send("move-progress", running);
    expect(screen.getByRole("heading", { level: 1 }).textContent).toBe(text("outcome-title"));
    fireEvent.click(screen.getByRole("button", { name: text("outcome-report") }));
    await waitFor(() => {
      expect(screen.getByText("1.txt")).toBeTruthy();
    });
    expect(screen.getByText(text("evidence-already-there"))).toBeTruthy();
    expect(screen.getByText(text("evidence-new-laptop"))).toBeTruthy();
    fireEvent.click(screen.getByRole("button", { name: text("frame-back") }));
    fireEvent.click(screen.getByRole("button", { name: text("outcome-home") }));
    expect(onHome).toHaveBeenCalled();
  });

  it("a report that cannot be read is never shown as an empty one", async () => {
    replies.move_report = () => [{ id: "1", name: "a", source: "a", status: "copied" }];
    await host(false);
    send("move-finished", outcomes[0]);
    fireEvent.click(screen.getByRole("button", { name: text("outcome-report") }));
    await waitFor(() => {
      expect(tauri.invoke).toHaveBeenCalledWith("move_report");
    });
    expect(screen.getByRole("heading", { level: 1 }).textContent).toBe(text("outcome-title"));
  });
});

describe("who's moving, on the old laptop", () => {
  const here = (canMoveOthers: boolean) => ({
    people: [
      { id: "S-1", name: "Ada", isYou: true },
      { id: "S-2", name: "Bea", isYou: false },
    ],
    canMoveOthers,
  });

  it("lists the people here; ticking someone starts the move with them, then says while the administrator is asked, and why they were not moved if not", async () => {
    replies.move_people_here = () => here(true);
    inApp(<MoveHost old={true} onPairAgain={vi.fn()} onHome={vi.fn()} />);
    expect(await screen.findByText(text("who-here-title"))).toBeTruthy();
    expect(tauri.invoke).not.toHaveBeenCalledWith("move_begin", expect.anything());
    expect(screen.getByText(text("who-here-approval"))).toBeTruthy();
    fireEvent.click(screen.getByRole("checkbox", { name: "Bea" }));
    fireEvent.click(screen.getByRole("button", { name: text("pair-done-continue") }));
    await waitFor(() => {
      expect(beganWith()).toEqual(["S-2"]);
    });
    send("move-stage", { kind: "administrator" });
    expect(screen.getByRole("heading", { level: 1 }).textContent).toBe(text("move-wait-admin-title"));
    send("move-stage", { kind: "others-not-moved", why: "not-approved" });
    send("move-stage", { kind: "waiting" });
    expect(screen.getByRole("heading", { level: 1 }).textContent).toBe(text("move-wait-approval-title"));
    expect(screen.getByText(text("move-others-not-approved"))).toBeTruthy();
  });

  it("where other people cannot be moved yet, they are listed without a tick and the page says so", async () => {
    replies.move_people_here = () => here(false);
    inApp(<MoveHost old={true} onPairAgain={vi.fn()} onHome={vi.fn()} />);
    expect(await screen.findByText(text("who-here-not-here"))).toBeTruthy();
    expect(screen.queryByRole("checkbox")).toBeNull();
    expect(screen.getByText("Bea")).toBeTruthy();
    fireEvent.click(screen.getByRole("button", { name: text("pair-done-continue") }));
    await waitFor(() => {
      expect(beganWith()).toEqual([]);
    });
  });

  it("the mover alone goes straight on", async () => {
    replies.move_people_here = () => ({ people: [{ id: "S-1", name: "Ada", isYou: true }], canMoveOthers: true });
    await host(true);
    expect(screen.queryByText(text("who-here-title"))).toBeNull();
  });
});

describe("the Full Disk Access guide", () => {
  it("says how many places the Mac kept private, opens the settings, and looks again or goes on without them", async () => {
    await host(true);
    send("move-privacy", { places: 2 });
    expect(screen.getByRole("heading", { level: 1 }).textContent).toBe(text("fda-title"));
    expect(screen.getByText((t) => plain(t) === plain(text("fda-body", { count: 2 })))).toBeTruthy();
    fireEvent.click(screen.getByRole("button", { name: text("fda-open") }));
    expect(tauri.invoke).toHaveBeenCalledWith("open_privacy_settings");
    fireEvent.click(screen.getByRole("button", { name: text("fda-again") }));
    expect(tauri.invoke).toHaveBeenCalledWith("move_privacy_answer", { again: true });
    expect(screen.getByRole("heading", { level: 1 }).textContent).toBe(text("move-wait-scanning-title"));
    send("move-privacy", { places: 1 });
    fireEvent.click(screen.getByRole("button", { name: text("fda-without") }));
    expect(tauri.invoke).toHaveBeenCalledWith("move_privacy_answer", { again: false });
  });
});

describe("other people, on the new laptop", () => {
  const toPlace = {
    people: [
      { id: "S-2", name: "Bea", files: 120, bytes: 3_400_000 },
      { id: "S-3", name: "Carl", files: 4, bytes: 1_000 },
    ],
    accounts: [{ id: "N-1", name: "bea" }],
    canPlace: true,
  };

  it("asks where each person's things go, choosing nothing for them, and never gives one account to two people", async () => {
    await host(false);
    send("move-people", toPlace);
    expect(screen.getByRole("heading", { level: 1 }).textContent).toBe(text("place-title"));
    const go = screen.getByRole("button", { name: text("pair-done-continue") });
    expect((go as HTMLButtonElement).disabled).toBe(true);
    const bea = screen.getByRole("combobox", { name: (n) => n.includes("Bea") });
    const carl = screen.getByRole("combobox", { name: (n) => n.includes("Carl") });
    fireEvent.change(bea, { target: { value: "existing:N-1" } });
    // Carl cannot be given Bea's account too.
    const carlsBea = Array.from((carl as HTMLSelectElement).options).find((o) => o.value === "existing:N-1");
    expect(carlsBea?.disabled).toBe(true);
    fireEvent.change(carl, { target: { value: "new" } });
    expect((go as HTMLButtonElement).disabled).toBe(false);
    fireEvent.click(go);
    await waitFor(() => {
      expect(tauri.invoke).toHaveBeenCalledWith("move_place_people", expect.objectContaining({ choices: [{ id: "S-2", to: { kind: "existing", accountId: "N-1" } }, { id: "S-3", to: { kind: "new" } }] }));
    });
    const call = tauri.invoke.mock.calls.find(([c]) => c === "move_place_people")?.[1] as { words: { body: string } };
    // The places in the password window's words reach the engine as they are, to be filled there.
    for (const place of ["{name}", "{account}", "{password}"]) expect(call.words.body).toContain(place);
  });

  it("the outcome gives each other person one line of counts, and when their things will be in place", async () => {
    await host(false);
    send("move-finished", outcomes[0]);
    expect(screen.getByText("Bea")).toBeTruthy();
    expect(screen.getByText((t) => plain(t).includes(plain(text("outcome-person-at-sign-in", { name: "Bea" }))))).toBeTruthy();
  });
});

describe("the move's error pages", () => {
  it("connection lost (C09): what is kept, the old laptop unchanged, pair again, and the engine's words only under More details", async () => {
    const { onPairAgain } = await host(false);
    send("move-problem", linkLost);
    expect(screen.getByRole("heading", { level: 1 }).textContent).toBe(text("move-lost-title"));
    expect(screen.getByText(text("move-ended-kept-body"))).toBeTruthy();
    expect(screen.getByText(text("move-ended-old-body"))).toBeTruthy();
    const details = screen.getByText(text("report-more-details")).closest("details");
    expect(details?.textContent).toContain((linkLost as { detail: string }).detail);
    fireEvent.click(screen.getByRole("button", { name: text("ended-pair-again") }));
    expect(onPairAgain).toHaveBeenCalled();
  });

  it("not approved: each laptop says it in its own words, and nothing is claimed as copied", async () => {
    await host(true);
    send("move-problem", notApproved);
    expect(screen.getByText(text("move-not-approved-old"))).toBeTruthy();
    expect(screen.queryByText(text("move-ended-kept-body"))).toBeNull();
    cleanup();
    await host(false);
    send("move-problem", notApproved);
    expect(screen.getByText(text("move-not-approved-new"))).toBeTruthy();
  });

  it("something else went wrong: the general page", async () => {
    await host(false);
    send("move-problem", failed);
    expect(screen.getByRole("heading", { level: 1 }).textContent).toBe(text("move-failed-title"));
  });

  it("a word from the engine that does not parse ends on the general page, never shown as sent", async () => {
    await host(false);
    send("move-progress", { state: "running", laptops: { old: null }, groups: [], counts: { total: -1 }, bytes: { checked: 0, total: 0 } });
    expect(screen.getByRole("heading", { level: 1 }).textContent).toBe(text("move-failed-title"));
    expect(screen.queryByText(text("report-more-details"))).toBeNull();
  });

  it.each(["not-paired", "busy", "failed"] as const)("a move that could not start (%s) says why", async (kind) => {
    replies.move_begin = () => {
      throw refusal(kind);
    };
    await host(false);
    await waitFor(() => {
      expect(screen.getByRole("heading", { level: 1 }).textContent).toBe(text("move-refused-title"));
    });
    expect(screen.getByText(text(`move-refused-${kind}`))).toBeTruthy();
  });

  it("has no accessibility violations", async () => {
    await host(false);
    send("move-problem", linkLost);
    const results = await axe.run(document.body, { rules: { region: { enabled: false } } });
    expect(results.violations.map((v) => v.id)).toEqual([]);
  });
});

describe("get help", () => {
  it("an error page opens a problem code for that problem and how far the move got, to copy (no share sheet on a laptop)", async () => {
    replies.help_code = () => "K3QD-8XWA-2M7P";
    await host(false);
    send("move-plan", plan);
    send("move-problem", linkLost);
    fireEvent.click(screen.getByRole("button", { name: text("move-get-help") }));
    await waitFor(() => {
      expect(screen.getByText("K3QD-8XWA-2M7P")).toBeTruthy();
    });
    expect(tauri.invoke).toHaveBeenCalledWith("help_code", { topic: "connection-lost", step: "approving" });
    expect(screen.queryByRole("button", { name: text("help-share") })).toBeNull();
    expect(screen.getByText(text("help-never-names"))).toBeTruthy();
    fireEvent.click(screen.getByRole("button", { name: text("frame-back") }));
    expect(screen.getByRole("heading", { level: 1 }).textContent).toBe(text("move-lost-title"));
  });

  it("a code that cannot be made says so and can be tried again; a reply that is not a code is not shown", async () => {
    replies.help_code = () => "<b>not a code</b>";
    await host(true);
    send("move-progress", running);
    fireEvent.click(screen.getByRole("button", { name: text("move-get-help") }));
    await waitFor(() => {
      expect(screen.getByText(text("help-failed"))).toBeTruthy();
    });
    expect(tauri.invoke).toHaveBeenCalledWith("help_code", { topic: "drive-problem", step: "moving" });
    replies.help_code = () => "K3QD-8XWA-2M7P";
    fireEvent.click(screen.getByRole("button", { name: text("help-try-again") }));
    await waitFor(() => {
      expect(screen.getByText("K3QD-8XWA-2M7P")).toBeTruthy();
    });
  });
});

describe("the window", () => {
  it("each page asks for its size, and Keep compact turns the progress into the mini panel (F09) until Open", async () => {
    const onWindow = vi.fn();
    inApp(<MoveHost old={false} onPairAgain={vi.fn()} onHome={vi.fn()} onWindow={onWindow} />);
    await waitFor(() => {
      expect(tauri.invoke).toHaveBeenCalledWith("move_begin", expect.anything());
    });
    expect(onWindow).toHaveBeenLastCalledWith("everyday");
    send("move-plan", plan);
    expect(onWindow).toHaveBeenLastCalledWith("expanded");
    send("move-progress", runningQuiet);
    expect(onWindow).toHaveBeenLastCalledWith("everyday");
    fireEvent.click(screen.getByRole("button", { name: text("progress-compact") }));
    expect(onWindow).toHaveBeenLastCalledWith("mini");
    expect(screen.getByText(text("mini-title"))).toBeTruthy();
    fireEvent.click(screen.getByRole("button", { name: text("mini-pause") }));
    expect(tauri.invoke).toHaveBeenCalledWith("move_pause");
    fireEvent.click(screen.getByRole("button", { name: text("mini-open") }));
    expect(onWindow).toHaveBeenLastCalledWith("everyday");
    expect(screen.getByRole("heading", { level: 1 }).textContent).toBe(text("progress-title"));
  });
});

describe("where events take the page", () => {
  const wait = { kind: "wait", stage: "waiting" } as const;
  it("an end stays an end, and a plan never goes back to waiting", () => {
    const end = next(wait, { kind: "problem", problem: { kind: "not-approved" } });
    expect(end.kind).toBe("end");
    expect(next(end, { kind: "stage", stage: { kind: "moving" } })).toBe(end);
    const planned = next(wait, { kind: "plan", plan: { ...plan, groups: [], leftOut: [] } });
    expect(next(planned, { kind: "stage", stage: { kind: "waiting" } })).toBe(planned);
  });
});

describe("paired", () => {
  it("offers to continue into the move", () => {
    const onContinue = vi.fn();
    inApp(<Paired onContinue={onContinue} />);
    fireEvent.click(screen.getByRole("button", { name: text("pair-done-continue") }));
    expect(onContinue).toHaveBeenCalled();
  });
});
