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
  it("listens before it starts, shows the label and the code in two groups with its countdown", async () => {
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
    expect(runs).toEqual(["123", "456"]);
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
    // Closing the field returns focus to the button that opened it.
    expect(document.activeElement).toBe(button("pair-old-rename"));

    fireEvent.click(button("pair-old-rename"));
    fireEvent.click(button("pair-old-rename-cancel"));
    expect(screen.queryByLabelText(text("pair-old-rename-label"))).toBeNull();
    noMissingText();
  });

  it("offers the three numbers; picking one sends it, and the note is always there", async () => {
    await renderOld();
    await emit("pairing-old", { kind: "code", code: "123456", secondsLeft: 50 });
    await emit("pairing-old", { kind: "choose", round: 7, numbers: [142, 317, 859] });

    expect(heading().textContent).toBe(text("pair-choose-title"));
    expect(screen.getByText(text("pair-choose-note"))).toBeTruthy();
    expect(screen.getByText(text("pair-choose-safe"))).toBeTruthy();
    // The code keeps counting behind the choice; it must not pull the person out of it.
    await emit("pairing-old", { kind: "code", code: "123456", secondsLeft: 49 });
    expect(heading().textContent).toBe(text("pair-choose-title"));

    fireEvent.click(screen.getByRole("button", { name: nameIs(text("pair-choose-number", { number: 317 })) }));
    expect(tauri.invoke).toHaveBeenCalledWith("pairing_old_pick", { round: 7, number: 317 });
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
    await emit("pairing-old", { kind: "choose", round: 7, numbers: [142, 317, 859] });
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
    expect(screen.getByRole("group", { name: nameIs(text("pair-new-number-label", { number: 57 })) }).textContent).toBe("57");
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
    expect(await screen.findByText(text("pairing-connection-lost"))).toBeTruthy();
    expect(await screen.findByText(text("pair-new-code-title"))).toBeTruthy();
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
