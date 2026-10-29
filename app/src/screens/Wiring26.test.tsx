/**
 * Task List 2.6 on the screens: the close page lists held files apart from the apps; the report
 * names the app that had a file open; the choice offers files kept only online cloud by cloud; the
 * outcome says what bringing them did; the plan offers the own folder and the two install choices,
 * and the page while apps are set up says when PCTwin may wait for free Wi-Fi.
 */
import { LocalizationProvider } from "@fluent/react";
import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import chooses from "../contract/move-choose.json";
import outcomes from "../contract/move-outcome.json";
import plans from "../contract/plan-view.json";
import reportItems from "../contract/report-item.json";
import { parseChoiceView, parseMoveOutcome, parsePlanView, parseReportItem, type Choice, type ChoiceView } from "../bridge";
import { localization } from "../i18n";
import { folderSafe, ownFolderName } from "../ownFolder";
import { violations } from "../testing/a11y";
import type { PlanView } from "../plan";
import AcceptMove from "./AcceptMove";
import ChooseWhat, { choiceOf } from "./ChooseWhat";
import { CloseApps, InstallChoicesBox, Rebuilding, installDefaults } from "./FullMoveParts";
import Outcome, { KeptOnlineSection } from "./Outcome";
import PlanReview from "./PlanReview";
import Report from "./Report";

const l10n = localization("en");
const text = (id: string, vars?: Record<string, string | number>) => l10n.getString(id, vars);
const plain = (s: string | null) => (s ?? "").replace(/[\u2066-\u2069]/g, "");
const body = () => plain(document.body.textContent);
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

const inApp = (node: React.ReactNode) =>
  render(
    <LocalizationProvider l10n={l10n}>
      <main>{node}</main>
    </LocalizationProvider>,
  );

const choice = (): ChoiceView => {
  const v = parseChoiceView(chooses[0]);
  if (!v) throw new Error("the engine's example parses");
  return v;
};

const fullPlan = (): PlanView => {
  const p = parsePlanView(plans[1]);
  if (!p?.rebuild || !p.ownFolderOffered) throw new Error("the engine's example offers the own folder and rebuilds");
  return p;
};

describe("the close page", () => {
  it("lists the apps and, apart from them, the files an app holds open, with what each means", async () => {
    const onAnswer = vi.fn();
    inApp(<CloseApps open={{ apps: ["Mail"], files: ["Budget.xlsx", "Notes.docx"] }} onAnswer={onAnswer} />);
    expect(screen.getByRole("heading", { level: 1 }).textContent).toBe(text("fullmove-close-title"));
    expect(screen.getByRole("heading", { level: 2 }).textContent).toBe(text("fullmove-close-files-title"));
    expect(body()).toContain(text("fullmove-close-body"));
    expect(body()).toContain(text("fullmove-close-files"));
    const lists = screen.getAllByRole("list");
    expect(lists).toHaveLength(2);
    expect(plain(lists[0].textContent)).toBe("Mail");
    expect(plain(lists[1].textContent)).toBe("Budget.xlsxNotes.docx");
    expect(await violations()).toEqual([]);
  });

  it("with only held files, the page is about them and says nothing of app data", async () => {
    inApp(<CloseApps open={{ apps: [], files: ["Budget.xlsx"] }} onAnswer={vi.fn()} />);
    expect(screen.getByRole("heading", { level: 1 }).textContent).toBe(text("fullmove-close-files-title"));
    expect(body()).not.toContain(text("fullmove-close-body"));
    expect(body()).toContain("Budget.xlsx");
    expect(await violations()).toEqual([]);
  });
});

describe("the report", () => {
  it("names the app that had the file open, under its reason", async () => {
    const items = reportItems.map(parseReportItem);
    const held = items.find((i) => i?.openIn);
    if (!held) throw new Error("the engine's example has a held file");
    inApp(<Report items={[held]} onBack={vi.fn()} />);
    fireEvent.click(screen.getByRole("button", { name: (n) => n.startsWith(text(`report-group-${held.kind ?? "other"}`)) }));
    expect(body()).toContain(text("reason-open-in-app"));
    expect(body()).toContain(plain(text("report-open-in", { app: "Outlook" })));
    expect(await violations()).toEqual([]);
  });
});

describe("files kept only online, on the old laptop", () => {
  it("none come unless ticked; each cloud has its own tick, and the one for a connection that may charge only once it is ticked", async () => {
    const onContinue = vi.fn<(c: Choice) => void>();
    inApp(<ChooseWhat view={choice()} onContinue={onContinue} />);
    expect(screen.getByRole("heading", { level: 2 }).textContent).toBe(text("choose-online-only-title"));
    const oneDrive = screen.getByRole("checkbox", { name: (n) => plain(n).includes("OneDrive") });
    // A cloud PCTwin cannot name is said plainly.
    expect(screen.getByRole("checkbox", { name: (n) => plain(n).includes(text("choose-online-only-unnamed")) })).toBeTruthy();
    expect(screen.queryByRole("checkbox", { name: text("choose-online-only-metered") })).toBeNull();
    fireEvent.click(oneDrive);
    const metered = screen.getByRole("checkbox", { name: text("choose-online-only-metered") });
    expect((metered as HTMLInputElement).checked).toBe(false);
    expect(await violations()).toEqual([]);
    fireEvent.click(screen.getByRole("button", { name: text("choose-continue") }));
    expect(onContinue.mock.calls[0][0].keepLocal).toEqual([{ provider: "one-drive", onMetered: false }]);
    fireEvent.click(metered);
    fireEvent.click(screen.getByRole("button", { name: text("choose-continue") }));
    expect(onContinue.mock.calls[1][0].keepLocal).toEqual([{ provider: "one-drive", onMetered: true }]);
    // Unticking the cloud takes its second tick with it.
    fireEvent.click(oneDrive);
    fireEvent.click(screen.getByRole("button", { name: text("choose-continue") }));
    expect(onContinue.mock.calls[2][0].keepLocal).toBeUndefined();
  });

  it("a laptop that cannot keep local copies says so, and sends none", () => {
    const v: ChoiceView = { ...choice(), onlineOnly: [], keepLocalHere: false };
    inApp(<ChooseWhat view={v} onContinue={vi.fn()} />);
    expect(body()).toContain(text("choose-online-only-not-here"));
    expect(choiceOf({ ...choice(), keepLocalHere: false }, "everything", new Set(), true, { "one-drive": { bring: true, onMetered: true } }).keepLocal).toBeUndefined();
  });

  it("the outcome says what was copied, what stays online, the space still taken and why it stopped", async () => {
    const o = outcomes.map(parseMoveOutcome).find((x) => x?.keptOnline);
    if (!o?.keptOnline) throw new Error("the engine's example has keptOnline");
    inApp(<Outcome old outcome={o} onReport={vi.fn()} />);
    expect(screen.getByRole("heading", { name: text("kept-online-title") })).toBeTruthy();
    expect(body()).toContain(text("kept-online-copied", { count: 40 }));
    expect(body()).toContain(text("kept-online-safe", { count: 3 }));
    expect(body()).toContain(text("kept-online-paused-space"));
    expect(body()).toContain("Free up space");
    expect(await violations()).toEqual([]);
  });

  it("leaves out the lines with nothing to say", () => {
    inApp(<KeptOnlineSection kept={{ copied: 5, safeInCloud: 0, downloadedBytes: 0, stillOnOldLaptopFiles: 0, stillOnOldLaptopBytes: 0, paused: null }} />);
    expect(screen.getAllByRole("listitem")).toHaveLength(1);
    expect(screen.queryByRole("alert")).toBeNull();
    expect(body()).not.toContain(text("kept-online-paused-metered"));
  });
});

describe("several old laptops into one", () => {
  const get = (id: string, args?: Record<string, string>) => l10n.getString(id, args);

  it("the folder's name carries no invisible direction marks", () => {
    expect(ownFolderName(get, "Bea's laptop")).toBe("From Bea's laptop");
    // As Fluent writes it with isolation on: each name wrapped in invisible marks.
    const isolating = (id: string, args?: Record<string, string>) => get(id, args && Object.fromEntries(Object.entries(args).map(([k, v]) => [k, `\u2068${v}\u2069`])));
    expect(ownFolderName(isolating, "Bea's laptop")).toBe("From Bea's laptop");
  });

  it("a laptop name a folder cannot hold, or none, gives the plain form", () => {
    const unnamed = plain(text("plan-own-folder-name-unnamed"));
    for (const bad of [null, "", "  ", "A/B", "C:\\", "Bea?", "end.", "tab\there", "x".repeat(101)]) expect(ownFolderName(get, bad)).toBe(unnamed);
    expect(folderSafe("Bea's laptop")).toBe(true);
    expect(folderSafe("\u2068Bea\u2069")).toBe(true);
  });

  it("every language's name is a folder name, for an ordinary laptop name and for none", () => {
    for (const lang of ["en", "fr", "es", "pt", "ar", "hi", "zh-CN"] as const) {
      const l = localization(lang);
      const g = (id: string, args?: Record<string, string>) => l.getString(id, args);
      for (const laptop of ["Bea", null]) {
        const n = ownFolderName(g, laptop);
        expect(folderSafe(n)).toBe(true);
        expect(/[\u2066-\u2069]/.test(n)).toBe(false);
      }
    }
  });

  it("the plan offers the own folder, on, with its name; turning it off is the person's choice", async () => {
    const onChange = vi.fn();
    inApp(<PlanReview plan={fullPlan()} approval={null} onApprovalChange={vi.fn()} onStart={vi.fn()} ownFolder={{ on: true, laptop: "Bea's laptop", folder: "From Bea's laptop", onChange }} />);
    const tick = screen.getByRole("checkbox", { name: (n) => plain(n).startsWith(plain(text("plan-own-folder", { laptop: "Bea's laptop" }))) });
    expect((tick as HTMLInputElement).checked).toBe(true);
    expect(body()).toContain(plain(text("plan-own-folder-called", { folder: "From Bea's laptop" })));
    fireEvent.click(tick);
    expect(onChange).toHaveBeenCalledWith(false);
    expect(await violations()).toEqual([]);
  });

  it("a plan that does not offer it shows no tick", () => {
    const p = { ...fullPlan(), ownFolderOffered: undefined };
    inApp(<PlanReview plan={p} approval={null} onApprovalChange={vi.fn()} onStart={vi.fn()} ownFolder={{ on: true, laptop: "Bea", folder: "From Bea", onChange: vi.fn() }} />);
    expect(screen.queryByRole("checkbox", { name: (n) => plain(n).startsWith(plain(text("plan-own-folder", { laptop: "Bea" }))) })).toBeNull();
  });

  it("Accept says where the things go, when they get their own folder", () => {
    inApp(<AcceptMove plan={fullPlan()} from="Bea" ownFolder="From Bea" onAccept={vi.fn()} onDetails={vi.fn()} onNotNow={vi.fn()} />);
    expect(body()).toContain(plain(text("accept-own-folder", { folder: "From Bea" })));
    cleanup();
    inApp(<AcceptMove plan={fullPlan()} from="Bea" onAccept={vi.fn()} onDetails={vi.fn()} onNotNow={vi.fn()} />);
    expect(body()).not.toContain(plain(text("accept-own-folder", { folder: "From Bea" })));
  });
});

describe("downloads and the two install choices", () => {
  const downloads = () => {
    const d = fullPlan().rebuild?.downloads;
    if (!d) throw new Error("the engine's example has downloads");
    return d;
  };

  it("start with Wait for free Wi-Fi on only on a connection that may charge, and installers from the old laptop off", () => {
    expect(installDefaults({ apps: 2, withOldInstaller: 1, connection: "may-charge" })).toEqual({ waitFreeWifi: true, reuseInstallers: false });
    expect(installDefaults({ apps: 2, withOldInstaller: 1, connection: "free" })).toEqual({ waitFreeWifi: false, reuseInstallers: false });
    expect(installDefaults({ apps: 0, withOldInstaller: 0, connection: "may-charge" }).waitFreeWifi).toBe(false);
    expect(installDefaults(undefined).waitFreeWifi).toBe(false);
  });

  it("say how many apps come from the internet and the connection now; each tick sends its own answer", async () => {
    const onChange = vi.fn();
    inApp(<InstallChoicesBox downloads={{ ...downloads(), withOldInstaller: 1 }} choices={{ waitFreeWifi: true, reuseInstallers: false }} onChange={onChange} />);
    expect(body()).toContain(text("plan-downloads", { count: 2 }));
    expect(body()).toContain(text("plan-connection-may-charge"));
    fireEvent.click(screen.getByRole("checkbox", { name: (n) => n.startsWith(text("plan-wait-free-wifi")) }));
    expect(onChange).toHaveBeenLastCalledWith({ waitFreeWifi: false, reuseInstallers: false });
    fireEvent.click(screen.getByRole("checkbox", { name: (n) => plain(n).startsWith(plain(text("plan-reuse-installers", { count: 1 }))) }));
    expect(onChange).toHaveBeenLastCalledWith({ waitFreeWifi: true, reuseInstallers: true });
    expect(await violations()).toEqual([]);
  });

  it("offer installers from the old laptop only when one came, and nothing when nothing is downloaded", () => {
    inApp(<InstallChoicesBox downloads={{ apps: 1, withOldInstaller: 0, connection: "not-known" }} choices={{ waitFreeWifi: false, reuseInstallers: false }} onChange={vi.fn()} />);
    expect(screen.getAllByRole("checkbox")).toHaveLength(1);
    expect(body()).toContain(text("plan-connection-not-known"));
    cleanup();
    const { container } = inApp(<InstallChoicesBox downloads={{ apps: 0, withOldInstaller: 0, connection: "free" }} choices={{ waitFreeWifi: false, reuseInstallers: false }} onChange={vi.fn()} />);
    expect(container.querySelector("main")?.childElementCount).toBe(0);
  });

  it("are on the plan page with the full move", () => {
    inApp(<PlanReview plan={fullPlan()} approval={null} onApprovalChange={vi.fn()} onStart={vi.fn()} installs={{ waitFreeWifi: true, reuseInstallers: false }} onInstalls={vi.fn()} />);
    expect(screen.getByRole("checkbox", { name: (n) => n.startsWith(text("plan-wait-free-wifi")) })).toBeTruthy();
  });

  it("while apps are set up, the page says PCTwin may wait for free Wi-Fi, only when the person chose it", async () => {
    inApp(<Rebuilding step={{ step: "apps", done: 0, total: 2 }} waitsForFreeWifi />);
    expect(body()).toContain(text("fullmove-rebuilding-wait-wifi"));
    expect(await violations()).toEqual([]);
    cleanup();
    inApp(<Rebuilding step={{ step: "apps", done: 0, total: 2 }} />);
    expect(body()).not.toContain(text("fullmove-rebuilding-wait-wifi"));
    cleanup();
    inApp(<Rebuilding step={{ step: "settings", done: 0, total: 2 }} waitsForFreeWifi />);
    expect(body()).not.toContain(text("fullmove-rebuilding-wait-wifi"));
  });
});
