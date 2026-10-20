import { LocalizationProvider } from "@fluent/react";
import type { ReactNode } from "react";
import { act, cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import axe from "axe-core";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { localization } from "../i18n";
import PairNew from "./PairNew";
import PairOld from "./PairOld";

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
const nameIs = (expected: string) => (accessible: string) => plain(accessible) === plain(expected);
const withText = (expected: string) => (content: string) => plain(content) === plain(expected);

/** What a rejected command carries from the engine: a plain object, as the shell sends it. */
const problem = (key: string, networks: string[] = []) => ({ key, networks }) as unknown as Error;

const fox = { kind: "picked", colour: "blue", animal: "fox" } as const;
const foxText = () => text("label-picked", { colour: text("label-colour-blue"), animal: text("label-animal-fox") });

let warnings: string[] = [];
let replies: Partial<Record<string, (args: unknown) => unknown>> = {};

beforeEach(() => {
  warnings = [];
  replies = {};
  tauri.handlers.clear();
  tauri.invoke.mockReset();
  // An async reply turns a thrown problem into a rejection, as the shell does.
  tauri.invoke.mockImplementation((command: string, args?: unknown) => Promise.resolve().then(() => replies[command]?.(args)));
  vi.spyOn(console, "warn").mockImplementation((...args: unknown[]) => {
    warnings.push(args.map(String).join(" "));
  });
});

afterEach(() => {
  cleanup();
  vi.restoreAllMocks();
});

function inApp(ui: ReactNode) {
  return render(<LocalizationProvider l10n={l10n}>
      <main>{ui}</main>
    </LocalizationProvider>);
}

async function renderOld(onBack = vi.fn()) {
  inApp(<PairOld onBack={onBack} />);
  await waitFor(() => {
    expect(tauri.invoke).toHaveBeenCalledWith("pairing_old_start", { network: null });
  });
  return onBack;
}

async function renderNew(found: unknown[], onBack = vi.fn()) {
  replies.pairing_new_search = () => found;
  inApp(<PairNew onBack={onBack} />);
  await waitFor(() => {
    expect(tauri.invoke).toHaveBeenCalledWith("pairing_new_search");
  });
  return onBack;
}

async function emit(channel: "pairing-old" | "pairing-new", payload: unknown) {
  await act(async () => {
    tauri.handlers.get(channel)?.({ payload });
    await Promise.resolve();
  });
}

const button = (id: string, vars?: Record<string, string | number>) => screen.getByRole("button", { name: nameIs(text(id, vars)) });
const heading = () => screen.getByRole("heading", { level: 1 });

async function violations(): Promise<string[]> {
  const result = await axe.run(document.body, { rules: { "color-contrast": { enabled: false } } });
  return result.violations.map((v) => `${v.id}: ${v.nodes.map((n) => n.target.join(" ")).join(", ")}`);
}

function noMissingText() {
  expect(warnings.filter((w) => /missing|not found|did not match/i.test(w))).toEqual([]);
}

describe("old laptop", () => {
  it("listens before it starts, shows the label and the code as six even tiles with its countdown", async () => {
    inApp(<PairOld onBack={vi.fn()} />);
    await waitFor(() => {
      expect(tauri.invoke).toHaveBeenCalledWith("pairing_old_start", { network: null });
    });
    expect(tauri.handlers.has("pairing-old")).toBe(true);
    expect(heading().textContent).toBe(text("pair-old-title"));

    await emit("pairing-old", { kind: "label", label: fox });
    await emit("pairing-old", { kind: "code", code: "123456", secondsLeft: 42 });

    expect(screen.getByText(withText(foxText()))).toBeTruthy();
    const group = screen.getByRole("group", { name: /1 2 3 4 5 6$/ });
    const runs = [...group.querySelectorAll(".kit-code-run")].map((r) => r.textContent);
    expect(runs).toEqual(["123456"]);
    expect(group.querySelectorAll(".kit-code-tile")).toHaveLength(6);
    expect(screen.getByText(withText(text("pair-old-code-meta", { seconds: 42 })))).toBeTruthy();
    expect(document.querySelectorAll(".pair-steps li")).toHaveLength(3);
    // The heading takes focus on the new state, and the countdown is not a live region.
    expect(document.activeElement).toBe(heading());
    expect(screen.getByText(withText(text("pair-old-code-meta", { seconds: 42 }))).closest("[aria-live]")).toBeNull();
    noMissingText();
  });

  it("lets the person shuffle the name and type their own, with the visibility note", async () => {
    await renderOld();
    await emit("pairing-old", { kind: "label", label: fox });
    await emit("pairing-old", { kind: "code", code: "123456", secondsLeft: 60 });

    fireEvent.click(button("pair-old-shuffle"));
    expect(tauri.invoke).toHaveBeenCalledWith("pairing_old_shuffle");

    fireEvent.click(button("pair-old-rename"));
    expect(screen.getByText(text("pair-old-rename-note"))).toBeTruthy();
    fireEvent.change(screen.getByLabelText(text("pair-old-rename-label")), { target: { value: "Ada's laptop" } });
    fireEvent.click(button("pair-old-rename-save"));
    await waitFor(() => {
      expect(tauri.invoke).toHaveBeenCalledWith("pairing_old_rename", { name: "Ada's laptop" });
    });
    await waitFor(() => {
      expect(screen.queryByLabelText(text("pair-old-rename-label"))).toBeNull();
    });
    // Closing the field returns focus to the button that opened it (moved once the field has gone).
    await waitFor(() => {
      expect(document.activeElement).toBe(button("pair-old-rename"));
    });

    fireEvent.click(button("pair-old-rename"));
    fireEvent.click(button("pair-old-rename-cancel"));
    expect(screen.queryByLabelText(text("pair-old-rename-label"))).toBeNull();
    noMissingText();
  });

  it("offers the three numbers; picking one sends it, and the note is always there", async () => {
    await renderOld();
    await emit("pairing-old", { kind: "code", code: "123456", secondsLeft: 50 });
    await emit("pairing-old", { kind: "choose", round: 7, numbers: [42, 17, 85] });

    expect(heading().textContent).toBe(text("pair-choose-title"));
    expect(screen.getByText(text("pair-choose-note"))).toBeTruthy();
    expect(screen.getByText(text("pair-choose-safe"))).toBeTruthy();
    // The code keeps counting behind the choice; it must not pull the person out of it.
    await emit("pairing-old", { kind: "code", code: "123456", secondsLeft: 49 });
    expect(heading().textContent).toBe(text("pair-choose-title"));

    fireEvent.click(screen.getByRole("button", { name: nameIs(text("pair-choose-number", { number: 17 })) }));

    fireEvent.click(button("pair-choose-confirm"));
    expect(tauri.invoke).toHaveBeenCalledWith("pairing_old_pick", { round: 7, number: 17 });
    expect(await screen.findByText(text("pair-choose-checking"))).toBeTruthy();
    noMissingText();
  });

  it("None of these match tells the engine and shows the next code with a notice", async () => {
    await renderOld();
    await emit("pairing-old", { kind: "code", code: "123456", secondsLeft: 50 });
    await emit("pairing-old", { kind: "choose", round: 7, numbers: [1, 2, 3] });
    fireEvent.click(button("pair-choose-none"));
    await waitFor(() => {
      expect(tauri.invoke).toHaveBeenCalledWith("pairing_old_decline", { round: 7 });
    });
    expect(await screen.findByText(text("pair-old-declined"))).toBeTruthy();
    expect(heading().textContent).toBe(text("pair-old-title"));
    noMissingText();
  });

  it("stays on Paired whatever arrives afterwards", async () => {
    await renderOld();
    await emit("pairing-old", { kind: "code", code: "123456", secondsLeft: 50 });
    await emit("pairing-old", { kind: "choose", round: 1, numbers: [1, 2, 3] });
    await emit("pairing-old", { kind: "paired" });
    expect(heading().textContent).toBe(text("pair-done-title"));
    for (const late of [
      { kind: "code", code: "654321", secondsLeft: 59 },
      { kind: "choose", round: 2, numbers: [4, 5, 6] },
      { kind: "timed-out" },
      { kind: "wait", secondsLeft: 9 },
      { kind: "locked" },
    ]) {
      await emit("pairing-old", late);
      expect(heading().textContent).toBe(text("pair-done-title"));
    }
    noMissingText();
  });

  it("says when the new laptop left during the pick, and shows the next code", async () => {
    await renderOld();
    await emit("pairing-old", { kind: "code", code: "123456", secondsLeft: 50 });
    await emit("pairing-old", { kind: "choose", round: 1, numbers: [1, 2, 3] });
    await emit("pairing-old", { kind: "left" });
    expect(screen.getByText(text("pair-old-left"))).toBeTruthy();
    expect(heading().textContent).toBe(text("pair-old-title"));
    noMissingText();
  });

  it("a pick rejected after Paired arrived leaves the screen on Paired", async () => {
    let reject: (e: unknown) => void = () => undefined;
    replies.pairing_old_pick = () =>
      new Promise((_, r) => {
        reject = r;
      });
    await renderOld();
    await emit("pairing-old", { kind: "code", code: "123456", secondsLeft: 50 });
    await emit("pairing-old", { kind: "choose", round: 1, numbers: [1, 2, 3] });
    fireEvent.click(screen.getByRole("button", { name: nameIs(text("pair-choose-number", { number: 2 })) }));
    fireEvent.click(button("pair-choose-confirm"));
    await emit("pairing-old", { kind: "paired" });
    await act(async () => {
      reject(problem("pairing-failed"));
      await Promise.resolve();
      await Promise.resolve();
    });
    expect(heading().textContent).toBe(text("pair-done-title"));
  });

  it("a pick after newer numbers names the newer round", async () => {
    await renderOld();
    await emit("pairing-old", { kind: "code", code: "123456", secondsLeft: 50 });
    await emit("pairing-old", { kind: "choose", round: 7, numbers: [1, 2, 3] });
    await emit("pairing-old", { kind: "choose", round: 8, numbers: [4, 5, 6] });
    fireEvent.click(screen.getByRole("button", { name: nameIs(text("pair-choose-number", { number: 5 })) }));
    fireEvent.click(button("pair-choose-confirm"));
    expect(tauri.invoke).toHaveBeenCalledWith("pairing_old_pick", { round: 8, number: 5 });
  });

  it("says when the person did not pick in time, and keeps showing the next code", async () => {
    await renderOld();
    await emit("pairing-old", { kind: "code", code: "123456", secondsLeft: 50 });
    await emit("pairing-old", { kind: "choose", round: 7, numbers: [1, 2, 3] });
    await emit("pairing-old", { kind: "timed-out" });
    expect(screen.getByText(text("pair-old-timed-out"))).toBeTruthy();
    expect(screen.getByRole("group", { name: /1 2 3 4 5 6$/ })).toBeTruthy();
    noMissingText();
  });

  it("shows the pause after a failed try, with its seconds", async () => {
    await renderOld();
    await emit("pairing-old", { kind: "wait", secondsLeft: 9 });
    expect(heading().textContent).toBe(text("pair-old-wait-title"));
    expect(screen.getByText(withText(text("pair-old-wait-body", { seconds: 9 })))).toBeTruthy();
    noMissingText();
  });

  it("when locked, Start again restarts pairing", async () => {
    await renderOld();
    await emit("pairing-old", { kind: "locked" });
    expect(heading().textContent).toBe(text("pair-old-locked-title"));
    fireEvent.click(button("pair-old-start-again"));
    expect(tauri.invoke).toHaveBeenCalledWith("pairing_old_start_again");
    noMissingText();
  });

  it("Back stops pairing and leaves", async () => {
    const onBack = await renderOld();
    fireEvent.click(button("pair-back"));
    expect(tauri.invoke).toHaveBeenCalledWith("pairing_stop");
    expect(onBack).toHaveBeenCalled();
  });

  it("stops listening when the screen goes away", async () => {
    await renderOld();
    expect(tauri.handlers.has("pairing-old")).toBe(true);
    cleanup();
    expect(tauri.handlers.has("pairing-old")).toBe(false);
  });

  it("shows a rejected command's problem in words", async () => {
    replies.pairing_old_pick = () => {
      throw problem("pairing-wrong-number");
    };
    await renderOld();
    await emit("pairing-old", { kind: "code", code: "123456", secondsLeft: 50 });
    await emit("pairing-old", { kind: "choose", round: 7, numbers: [1, 2, 3] });
    fireEvent.click(screen.getByRole("button", { name: nameIs(text("pair-choose-number", { number: 2 })) }));
    fireEvent.click(button("pair-choose-confirm"));
    expect(await screen.findByText(text("pairing-wrong-number"))).toBeTruthy();
    noMissingText();
  });

  it("anything unexpected becomes the generic problem, with Try again", async () => {
    replies.pairing_old_start = () => {
      throw new Error("boom");
    };
    inApp(<PairOld onBack={vi.fn()} />);
    expect(await screen.findByText(text("pairing-failed"))).toBeTruthy();
    replies.pairing_old_start = () => undefined;
    fireEvent.click(button("pair-try-again"));
    await waitFor(() => {
      expect(tauri.invoke.mock.calls.filter((c) => c[0] === "pairing_old_start")).toHaveLength(2);
    });
    noMissingText();
  });

  it("a problem key this version has no words for shows the generic problem, never an empty notice", async () => {
    replies.pairing_old_start = () => {
      throw problem("pairing-something-new");
    };
    inApp(<PairOld onBack={vi.fn()} />);
    expect(await screen.findByText(text("pairing-failed"))).toBeTruthy();
    noMissingText();
  });

  it("the code comes before the laptop's name on the screen", async () => {
    await renderOld();
    await emit("pairing-old", { kind: "label", label: fox });
    await emit("pairing-old", { kind: "code", code: "123456", secondsLeft: 50 });
    const code = screen.getByRole("group", { name: /123456|1 2 3 4 5 6/ });
    const name = screen.getByText(withText(foxText()));
    expect(code.compareDocumentPosition(name) & Node.DOCUMENT_POSITION_FOLLOWING).toBeTruthy();
  });

  it("when the network is unclear, lists the networks verbatim and starts on the one chosen", async () => {
    let first = true;
    replies.pairing_old_start = () => {
      if (first) {
        first = false;
        throw problem("pairing-choose-network", ["Wi-Fi", "Ethernet 2"]);
      }
      return undefined;
    };
    inApp(<PairOld onBack={vi.fn()} />);
    expect(await screen.findByText(text("pair-network-title"))).toBeTruthy();
    expect(screen.getByText("Ethernet 2")).toBeTruthy();
    fireEvent.click(screen.getByRole("button", { name: /Wi-Fi/ }));
    await waitFor(() => {
      expect(tauri.invoke).toHaveBeenCalledWith("pairing_old_start", { network: "Wi-Fi" });
    });
    noMissingText();
  });

  it("paired shows the Paired screen", async () => {
    await renderOld();
    await emit("pairing-old", { kind: "paired" });
    expect(heading().textContent).toBe(text("pair-done-title"));
    noMissingText();
  });

  it("has no accessibility problems on the code, choose and paired states", async () => {
    await renderOld();
    await emit("pairing-old", { kind: "label", label: fox });
    await emit("pairing-old", { kind: "code", code: "123456", secondsLeft: 50 });
    expect(await violations()).toEqual([]);
    await emit("pairing-old", { kind: "choose", round: 7, numbers: [42, 17, 85] });
    expect(await violations()).toEqual([]);
    await emit("pairing-old", { kind: "paired" });
    expect(await violations()).toEqual([]);
  });
});

describe("new laptop", () => {
  it("with nothing found, says so and searches again", async () => {
    await renderNew([]);
    expect(await screen.findByText(text("pair-new-none-title"))).toBeTruthy();
    fireEvent.click(button("pair-new-search-again"));
    await waitFor(() => {
      expect(tauri.invoke.mock.calls.filter((c) => c[0] === "pairing_new_search")).toHaveLength(2);
    });
    noMissingText();
  });

  it("with exactly one found, skips the list; the code goes to the engine as typed; the number is shown", async () => {
    await renderNew([{ index: 0, label: fox, sameLabelNearby: false }]);
    expect(await screen.findByText(text("pair-new-code-title"))).toBeTruthy();
    expect(screen.queryByRole("button", { name: nameIs(text("pair-new-other-laptop")) })).toBeNull();
    expect(screen.getByText(withText(text("pair-new-code-body", { label: foxText() })))).toBeTruthy();

    const field = screen.getByLabelText<HTMLInputElement>(text("pair-new-code-label"));
    expect(field.getAttribute("inputmode")).toBe("numeric");
    expect(field.getAttribute("autocomplete")).toBe("one-time-code");
    fireEvent.change(field, { target: { value: "123 456" } });
    expect(field.value).toBe("123 456");

    replies.pairing_new_connect = () => 57;
    fireEvent.click(button("pair-new-connect"));
    await waitFor(() => {
      expect(tauri.invoke).toHaveBeenCalledWith("pairing_new_connect", { index: 0, code: "123 456" });
    });
    expect(await screen.findByText(text("pair-new-number-title"))).toBeTruthy();
    expect(screen.getByRole("group", { name: nameIs(text("pair-new-number-label", { number: 57 })) }).textContent).toBe(`${text("pair-new-number-tile")}57`);
    expect(screen.getByText(text("pair-new-number-waiting"))).toBeTruthy();

    await emit("pairing-new", { kind: "paired" });
    expect(heading().textContent).toBe(text("pair-done-title"));
    noMissingText();
  });

  it("flags two laptops with the same name, in words, and offers a way back to the list", async () => {
    await renderNew([
      { index: 0, label: fox, sameLabelNearby: true },
      { index: 1, label: fox, sameLabelNearby: true },
      { index: 2, label: { kind: "named", name: "Ada's laptop" }, sameLabelNearby: false },
    ]);
    expect(await screen.findByText(text("pair-new-list-title"))).toBeTruthy();
    const flags = screen.getAllByText(text("pair-new-same-label"));
    expect(flags).toHaveLength(2);
    // A flagged row's name is the laptop's name followed by the flag, so the visible words are all in it.
    const rows = screen.getAllByRole("button", { name: (n) => n.includes(foxText()) });
    expect(rows).toHaveLength(2);
    for (const row of rows) expect(row.textContent).toContain(text("pair-new-same-label"));
    expect(screen.getByRole("button", { name: "Ada's laptop" }).getAttribute("aria-labelledby")?.includes(" ")).toBe(false);

    fireEvent.click(rows[1]);
    expect(await screen.findByText(text("pair-new-code-title"))).toBeTruthy();
    fireEvent.click(button("pair-new-other-laptop"));
    expect(await screen.findByText(text("pair-new-list-title"))).toBeTruthy();
    noMissingText();
  });

  it("a problem after the number returns to an empty code field with the problem in words", async () => {
    await renderNew([{ index: 0, label: fox, sameLabelNearby: false }]);
    await screen.findByText(text("pair-new-code-title"));
    replies.pairing_new_connect = () => 57;
    fireEvent.change(screen.getByLabelText(text("pair-new-code-label")), { target: { value: "123456" } });
    fireEvent.click(button("pair-new-connect"));
    await screen.findByText(text("pair-new-number-title"));

    await emit("pairing-new", { kind: "problem", problem: { key: "pairing-timed-out", networks: [] } });
    expect(await screen.findByText(text("pairing-timed-out"))).toBeTruthy();
    await emit("pairing-new", { kind: "problem", problem: { key: "pairing-no-match", networks: [] } });
    expect(await screen.findByText(text("pairing-no-match"))).toBeTruthy();
    expect(screen.getByLabelText<HTMLInputElement>(text("pair-new-code-label")).value).toBe("");
    noMissingText();
  });

  it("a rejected connect shows its problem and keeps the person in the code field", async () => {
    await renderNew([{ index: 0, label: fox, sameLabelNearby: false }]);
    await screen.findByText(text("pair-new-code-title"));
    replies.pairing_new_connect = () => {
      throw problem("pairing-code-wrong");
    };
    fireEvent.change(screen.getByLabelText(text("pair-new-code-label")), { target: { value: "000000" } });
    fireEvent.click(button("pair-new-connect"));
    expect(await screen.findByText(text("pairing-code-wrong"))).toBeTruthy();
    await waitFor(() => {
      expect(document.activeElement).toBe(screen.getByLabelText(text("pair-new-code-label")));
    });
    noMissingText();
  });

  it("Back stops pairing and leaves", async () => {
    const onBack = await renderNew([]);
    await screen.findByText(text("pair-new-none-title"));
    fireEvent.click(button("pair-back"));
    expect(tauri.invoke).toHaveBeenCalledWith("pairing_stop");
    expect(onBack).toHaveBeenCalled();
  });

  it("has no accessibility problems on the list, code, number and paired states", async () => {
    await renderNew([
      { index: 0, label: fox, sameLabelNearby: true },
      { index: 1, label: fox, sameLabelNearby: true },
    ]);
    await screen.findByText(text("pair-new-list-title"));
    expect(await violations()).toEqual([]);
    fireEvent.click(screen.getAllByRole("button", { name: (n) => n.includes(foxText()) })[0]);
    await screen.findByText(text("pair-new-code-title"));
    expect(await violations()).toEqual([]);
    replies.pairing_new_connect = () => 57;
    fireEvent.change(screen.getByLabelText(text("pair-new-code-label")), { target: { value: "123 456" } });
    fireEvent.click(button("pair-new-connect"));
    await screen.findByText(text("pair-new-number-title"));
    expect(await violations()).toEqual([]);
    await emit("pairing-new", { kind: "paired" });
    expect(await violations()).toEqual([]);
  });
});

describe("the new laptop once paired", () => {
  it("stays on Paired whatever arrives afterwards", async () => {
    await renderNew([{ index: 0, label: fox, sameLabelNearby: false }]);
    await screen.findByText(text("pair-new-code-title"));
    replies.pairing_new_connect = () => 57;
    fireEvent.change(screen.getByLabelText(text("pair-new-code-label")), { target: { value: "123456" } });
    fireEvent.click(button("pair-new-connect"));
    await screen.findByText(text("pair-new-number-title"));
    await emit("pairing-new", { kind: "paired" });
    expect(heading().textContent).toBe(text("pair-done-title"));
    await emit("pairing-new", { kind: "problem", problem: { key: "pairing-timed-out", networks: [] } });
    expect(heading().textContent).toBe(text("pair-done-title"));
  });

  it("leaves Paired only when the paired connection is lost, and says so", async () => {
    await renderNew([{ index: 0, label: fox, sameLabelNearby: false }]);
    await screen.findByText(text("pair-new-code-title"));
    replies.pairing_new_connect = () => 57;
    fireEvent.change(screen.getByLabelText(text("pair-new-code-label")), { target: { value: "123456" } });
    fireEvent.click(button("pair-new-connect"));
    await screen.findByText(text("pair-new-number-title"));
    await emit("pairing-new", { kind: "paired" });
    const searches = () => tauri.invoke.mock.calls.filter((c) => c[0] === "pairing_new_search").length;
    const before = searches();
    await emit("pairing-new", { kind: "lost" });
    // The old laptop starts again elsewhere, so it is looked for again rather than retried.
    await waitFor(() => {
      expect(searches()).toBe(before + 1);
    });
    expect(await screen.findByText(text("pairing-connection-lost-new"))).toBeTruthy();
    expect(await screen.findByText(text("pair-new-code-title"))).toBeTruthy();
    noMissingText();
  });

  it("after a lost connection, says so while it looks for the old laptop again", async () => {
    await renderNew([{ index: 0, label: fox, sameLabelNearby: false }]);
    await screen.findByText(text("pair-new-code-title"));
    replies.pairing_new_connect = () => 57;
    fireEvent.change(screen.getByLabelText(text("pair-new-code-label")), { target: { value: "123456" } });
    fireEvent.click(button("pair-new-connect"));
    await screen.findByText(text("pair-new-number-title"));
    await emit("pairing-new", { kind: "paired" });
    // The search does not answer yet: the person sees the searching screen with the notice.
    replies.pairing_new_search = () => new Promise(() => undefined);
    await emit("pairing-new", { kind: "lost" });
    expect(await screen.findByText(text("pair-new-searching-title"))).toBeTruthy();
    expect(await screen.findByText(text("pairing-connection-lost-new"))).toBeTruthy();
    noMissingText();
  });
});

describe("the old laptop once paired", () => {
  it("leaves Paired only when the paired connection is lost, and offers Try again", async () => {
    await renderOld();
    await emit("pairing-old", { kind: "paired" });
    await emit("pairing-old", { kind: "lost" });
    expect(await screen.findByText(text("pairing-connection-lost"))).toBeTruthy();
    expect(button("pair-try-again")).toBeTruthy();
    noMissingText();
    // No longer paired: the next start's updates show again.
    await emit("pairing-old", { kind: "code", code: "123456", secondsLeft: 50 });
    expect(screen.getByRole("group", { name: /1 2 3 4 5 6$/ })).toBeTruthy();
  });
});

describe("the new laptop after an attempt ends", () => {
  it("looks for the old laptop again, keeping what happened on screen", async () => {
    await renderNew([{ index: 0, label: fox, sameLabelNearby: false }]);
    await screen.findByText(text("pair-new-code-title"));
    replies.pairing_new_connect = () => 57;
    fireEvent.change(screen.getByLabelText(text("pair-new-code-label")), { target: { value: "123456" } });
    fireEvent.click(button("pair-new-connect"));
    await screen.findByText(text("pair-new-number-title"));
    const searches = () => tauri.invoke.mock.calls.filter((c) => c[0] === "pairing_new_search").length;
    const before = searches();
    await emit("pairing-new", { kind: "problem", problem: { key: "pairing-connection-lost", networks: [] } });
    await waitFor(() => {
      expect(searches()).toBe(before + 1);
    });
    // Said in the new laptop's own words: it has no Try again button.
    expect(await screen.findByText(text("pairing-connection-lost-new"))).toBeTruthy();
    expect(screen.queryByText(text("pairing-connection-lost"))).toBeNull();
    noMissingText();
  });

  it("ignores a number that arrives after its attempt ended", async () => {
    await renderNew([{ index: 0, label: fox, sameLabelNearby: false }]);
    await screen.findByText(text("pair-new-code-title"));
    let answer: (n: number) => void = () => undefined;
    replies.pairing_new_connect = () =>
      new Promise((resolve) => {
        answer = resolve;
      });
    fireEvent.change(screen.getByLabelText(text("pair-new-code-label")), { target: { value: "123456" } });
    fireEvent.click(button("pair-new-connect"));
    // The attempt's end arrives before the reply.
    await emit("pairing-new", { kind: "problem", problem: { key: "pairing-connection-lost", networks: [] } });
    await act(async () => {
      answer(57);
      await Promise.resolve();
      await Promise.resolve();
    });
    expect(screen.queryByText(text("pair-new-number-title"))).toBeNull();
    expect(await screen.findByText(text("pair-new-code-title"))).toBeTruthy();
  });
});

describe("choosing the matching number", () => {
  it("sends nothing until the person confirms, and a different number can be chosen first", async () => {
    await renderOld();
    await emit("pairing-old", { kind: "code", code: "123456", secondsLeft: 50 });
    await emit("pairing-old", { kind: "choose", round: 3, numbers: [11, 42, 87] });
    const confirm = button("pair-choose-confirm") as HTMLButtonElement;
    expect(confirm.disabled).toBe(true);
    const tile = (n: number) => screen.getByRole("button", { name: nameIs(text("pair-choose-number", { number: n })) });
    fireEvent.click(tile(11));
    fireEvent.click(tile(42));
    expect(tile(42).getAttribute("aria-pressed")).toBe("true");
    expect(tile(11).getAttribute("aria-pressed")).toBe("false");
    expect(tauri.invoke.mock.calls.some((c) => c[0] === "pairing_old_pick")).toBe(false);
    fireEvent.click(confirm);
    expect(tauri.invoke).toHaveBeenCalledWith("pairing_old_pick", { round: 3, number: 42 });
    noMissingText();
  });

  it("new numbers clear an earlier choice", async () => {
    await renderOld();
    await emit("pairing-old", { kind: "code", code: "123456", secondsLeft: 50 });
    await emit("pairing-old", { kind: "choose", round: 1, numbers: [1, 2, 3] });
    fireEvent.click(screen.getByRole("button", { name: nameIs(text("pair-choose-number", { number: 2 })) }));
    await emit("pairing-old", { kind: "choose", round: 2, numbers: [4, 5, 6] });
    expect((button("pair-choose-confirm") as HTMLButtonElement).disabled).toBe(true);
  });
});

describe("late replies never move a screen off what it shows", () => {
  const tile = (n: number) => screen.getByRole("button", { name: nameIs(text("pair-choose-number", { number: n })) });

  it("a rejected choice from an earlier round leaves the newer numbers alone", async () => {
    let reject: (e: unknown) => void = () => undefined;
    replies.pairing_old_pick = () =>
      new Promise((_, r) => {
        reject = r;
      });
    await renderOld();
    await emit("pairing-old", { kind: "code", code: "123456", secondsLeft: 50 });
    await emit("pairing-old", { kind: "choose", round: 7, numbers: [1, 2, 3] });
    fireEvent.click(tile(2));
    fireEvent.click(button("pair-choose-confirm"));
    await emit("pairing-old", { kind: "choose", round: 8, numbers: [4, 5, 6] });
    await act(async () => {
      reject(problem("pairing-timed-out"));
      await Promise.resolve();
      await Promise.resolve();
    });
    expect(heading().textContent).toBe(text("pair-choose-title"));
    expect(tile(5)).toBeTruthy();
  });

  it("a decline answered after newer numbers leaves them alone", async () => {
    let answer: () => void = () => undefined;
    replies.pairing_old_decline = () =>
      new Promise((resolve) => {
        answer = () => { resolve(undefined); };
      });
    await renderOld();
    await emit("pairing-old", { kind: "code", code: "123456", secondsLeft: 50 });
    await emit("pairing-old", { kind: "choose", round: 7, numbers: [1, 2, 3] });
    fireEvent.click(button("pair-choose-none"));
    await emit("pairing-old", { kind: "choose", round: 8, numbers: [4, 5, 6] });
    await act(async () => {
      answer();
      await Promise.resolve();
      await Promise.resolve();
    });
    expect(heading().textContent).toBe(text("pair-choose-title"));
    expect(screen.queryByText(text("pair-old-declined"))).toBeNull();
  });

  it("a choice that came too late says so once", async () => {
    replies.pairing_old_pick = () => {
      throw problem("pairing-timed-out");
    };
    await renderOld();
    await emit("pairing-old", { kind: "code", code: "123456", secondsLeft: 50 });
    await emit("pairing-old", { kind: "choose", round: 1, numbers: [1, 2, 3] });
    fireEvent.click(tile(2));
    fireEvent.click(button("pair-choose-confirm"));
    await emit("pairing-old", { kind: "timed-out" });
    await waitFor(() => {
      expect(screen.getByText(text("pair-old-timed-out"))).toBeTruthy();
    });
    expect(screen.queryByText(text("pairing-timed-out"))).toBeNull();
  });

  it("Confirm keeps the person's place, and a double press sends one choice", async () => {
    replies.pairing_old_pick = () => new Promise(() => undefined);
    await renderOld();
    await emit("pairing-old", { kind: "code", code: "123456", secondsLeft: 50 });
    await emit("pairing-old", { kind: "choose", round: 1, numbers: [1, 2, 3] });
    fireEvent.click(tile(2));
    const confirm = button("pair-choose-confirm");
    confirm.focus();
    fireEvent.click(confirm);
    fireEvent.click(confirm);
    expect(tauri.invoke.mock.calls.filter((c) => c[0] === "pairing_old_pick")).toHaveLength(1);
    await waitFor(() => {
      expect(document.activeElement?.classList.contains("pair-checking")).toBe(true);
    });
  });

  it("new numbers are announced and take the focus", async () => {
    await renderOld();
    await emit("pairing-old", { kind: "code", code: "123456", secondsLeft: 50 });
    await emit("pairing-old", { kind: "choose", round: 1, numbers: [1, 2, 3] });
    await emit("pairing-old", { kind: "choose", round: 2, numbers: [4, 5, 6] });
    expect(await screen.findByText(text("pair-choose-renewed"))).toBeTruthy();
    await waitFor(() => {
      expect(document.activeElement).toBe(tile(4));
    });
    noMissingText();
  });

  it("the new laptop stays Paired when Paired arrives before its own reply", async () => {
    await renderNew([{ index: 0, label: fox, sameLabelNearby: false }]);
    await screen.findByText(text("pair-new-code-title"));
    let answer: (n: number) => void = () => undefined;
    replies.pairing_new_connect = () =>
      new Promise((resolve) => {
        answer = resolve;
      });
    fireEvent.change(screen.getByLabelText(text("pair-new-code-label")), { target: { value: "123456" } });
    fireEvent.click(button("pair-new-connect"));
    await emit("pairing-new", { kind: "paired" });
    await act(async () => {
      answer(57);
      await Promise.resolve();
      await Promise.resolve();
    });
    expect(heading().textContent).toBe(text("pair-done-title"));
  });

  it("when looking again fails, what happened is still what is said", async () => {
    await renderNew([{ index: 0, label: fox, sameLabelNearby: false }]);
    await screen.findByText(text("pair-new-code-title"));
    replies.pairing_new_connect = () => 57;
    fireEvent.change(screen.getByLabelText(text("pair-new-code-label")), { target: { value: "123456" } });
    fireEvent.click(button("pair-new-connect"));
    await screen.findByText(text("pair-new-number-title"));
    replies.pairing_new_search = () => {
      throw problem("pairing-failed");
    };
    await emit("pairing-new", { kind: "lost" });
    expect(await screen.findByText(text("pairing-connection-lost-new"))).toBeTruthy();
  });

  it("pairs again after a lost connection", async () => {
    await renderNew([{ index: 0, label: fox, sameLabelNearby: false }]);
    await screen.findByText(text("pair-new-code-title"));
    replies.pairing_new_connect = () => 57;
    const connect = async () => {
      fireEvent.change(screen.getByLabelText(text("pair-new-code-label")), { target: { value: "123456" } });
      fireEvent.click(button("pair-new-connect"));
      await screen.findByText(text("pair-new-number-title"));
    };
    await connect();
    await emit("pairing-new", { kind: "paired" });
    await emit("pairing-new", { kind: "lost" });
    await screen.findByText(text("pair-new-code-title"));
    await connect();
    await emit("pairing-new", { kind: "paired" });
    expect(heading().textContent).toBe(text("pair-done-title"));
  });
});

describe("new numbers and the checking state", () => {
  const tile = (n: number) => screen.getByRole("button", { name: nameIs(text("pair-choose-number", { number: n })) });

  it("the first numbers, or numbers after a code screen, are not announced as new", async () => {
    await renderOld();
    await emit("pairing-old", { kind: "code", code: "123456", secondsLeft: 50 });
    await emit("pairing-old", { kind: "choose", round: 1, numbers: [1, 2, 3] });
    expect(screen.queryByText(text("pair-choose-renewed"))).toBeNull();
    await emit("pairing-old", { kind: "timed-out" });
    await emit("pairing-old", { kind: "choose", round: 2, numbers: [4, 5, 6] });
    expect(screen.queryByText(text("pair-choose-renewed"))).toBeNull();
  });

  it("while checking, the numbers cannot be changed and only Checking is said", async () => {
    replies.pairing_old_pick = () => new Promise(() => undefined);
    await renderOld();
    await emit("pairing-old", { kind: "code", code: "123456", secondsLeft: 50 });
    await emit("pairing-old", { kind: "choose", round: 1, numbers: [1, 2, 3] });
    await emit("pairing-old", { kind: "choose", round: 2, numbers: [4, 5, 6] });
    fireEvent.click(tile(5));
    fireEvent.click(button("pair-choose-confirm"));
    expect(await screen.findByText(text("pair-choose-checking"))).toBeTruthy();
    expect(screen.queryByText(text("pair-choose-renewed"))).toBeNull();
    expect((tile(4) as HTMLButtonElement).disabled).toBe(true);
  });
});

describe("someone tried wrong codes, and the system's own questions", () => {
  beforeEach(() => {
    localStorage.clear();
  });

  it("locked says someone on this network tried wrong codes, suggests a phone hotspot, and only Start again restarts", async () => {
    await renderOld();
    await emit("pairing-old", { kind: "locked" });
    expect(heading().textContent).toBe(text("pair-old-locked-title"));
    expect(heading().textContent).toBe("Someone on this network tried wrong codes.");
    expect(screen.getByText(withText(text("pair-old-locked-body")))).toBeTruthy();
    expect(screen.getByText(withText(text("pair-old-locked-hotspot")))).toBeTruthy();
    // No code is on screen while locked, and nothing restarts by itself.
    expect(screen.queryByRole("group", { name: /\d \d \d/ })).toBeNull();
    expect(tauri.invoke).not.toHaveBeenCalledWith("pairing_old_start_again");
    expect(await violations()).toEqual([]);
    fireEvent.click(button("pair-old-start-again"));
    expect(tauri.invoke).toHaveBeenCalledWith("pairing_old_start_again");
    await emit("pairing-old", { kind: "code", code: "654321", secondsLeft: 60 });
    expect(heading().textContent).toBe(text("pair-old-title"));
    noMissingText();
  });

  it("on a Mac, the old laptop says what the Mac will ask before it starts, and starts on Continue", async () => {
    replies.pairing_heads_up = () => [{ kind: "local-network" }];
    inApp(<PairOld onBack={vi.fn()} />);
    await waitFor(() => {
      expect(heading().textContent).toBe(text("pair-heads-up-title"));
    });
    expect(tauri.invoke).toHaveBeenCalledWith("pairing_heads_up", { role: "old" });
    expect(screen.getByText(withText(text("pair-heads-up-local-network")))).toBeTruthy();
    expect(tauri.invoke).not.toHaveBeenCalledWith("pairing_old_start", expect.anything());
    expect(await violations()).toEqual([]);
    fireEvent.click(button("pair-heads-up-continue"));
    await waitFor(() => {
      expect(tauri.invoke).toHaveBeenCalledWith("pairing_old_start", { network: null });
    });
    noMissingText();
  });

  it("a note shown once is not shown again: the next start goes straight to the code", async () => {
    replies.pairing_heads_up = () => [{ kind: "firewall" }];
    inApp(<PairOld onBack={vi.fn()} />);
    await waitFor(() => {
      expect(screen.getByText(withText(text("pair-heads-up-firewall")))).toBeTruthy();
    });
    fireEvent.click(button("pair-heads-up-continue"));
    await waitFor(() => {
      expect(tauri.invoke).toHaveBeenCalledWith("pairing_old_start", { network: null });
    });
    cleanup();
    tauri.invoke.mockClear();
    await renderOld();
    expect(screen.queryByText(withText(text("pair-heads-up-firewall")))).toBeNull();
  });

  it("when the notes cannot be asked for, pairing starts as usual", async () => {
    replies.pairing_heads_up = () => {
      throw problem("pairing-failed");
    };
    await renderOld();
    expect(heading().textContent).toBe(text("pair-old-title"));
  });

  it("an old laptop whose Mac refuses the local network says where to switch it on, and Try again starts again", async () => {
    let refuse = true;
    replies.pairing_old_start = () => {
      if (refuse) throw problem("pairing-local-network-refused");
    };
    await renderOld();
    await waitFor(() => {
      expect(screen.getByText(withText(text("pairing-local-network-refused")))).toBeTruthy();
    });
    expect(text("pairing-local-network-refused")).toContain("Privacy & Security");
    refuse = false;
    tauri.invoke.mockClear();
    fireEvent.click(button("pair-try-again"));
    await waitFor(() => {
      expect(tauri.invoke).toHaveBeenCalledWith("pairing_old_start", { network: null });
    });
    noMissingText();
  });

  it("on a Mac, the new laptop says what the Mac will ask before it looks", async () => {
    replies.pairing_heads_up = () => [{ kind: "local-network" }];
    replies.pairing_new_search = () => [];
    inApp(<PairNew onBack={vi.fn()} />);
    await waitFor(() => {
      expect(heading().textContent).toBe(text("pair-heads-up-title"));
    });
    expect(tauri.invoke).toHaveBeenCalledWith("pairing_heads_up", { role: "new" });
    expect(tauri.invoke).not.toHaveBeenCalledWith("pairing_new_search");
    fireEvent.click(button("pair-heads-up-continue"));
    await waitFor(() => {
      expect(tauri.invoke).toHaveBeenCalledWith("pairing_new_search");
    });
    noMissingText();
  });

  it("a new laptop that found nothing because its Mac refuses the local network says so, with where to switch it on", async () => {
    replies.pairing_new_search = () => {
      throw problem("pairing-local-network-refused");
    };
    inApp(<PairNew onBack={vi.fn()} />);
    await waitFor(() => {
      expect(heading().textContent).toBe(text("pair-new-none-title"));
    });
    expect(screen.getByText(withText(text("pairing-local-network-refused")))).toBeTruthy();
    expect(button("pair-new-search-again")).toBeTruthy();
    expect(await violations()).toEqual([]);
    noMissingText();
  });
});
