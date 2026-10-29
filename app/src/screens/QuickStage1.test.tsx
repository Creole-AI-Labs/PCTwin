import { LocalizationProvider } from "@fluent/react";
import type { ReactNode } from "react";
import { act, cleanup, fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import knownExamples from "../contract/known-connection.json";
import incomingExamples from "../contract/known-incoming.json";
import noteExamples from "../contract/qs-note.json";
import receipts from "../contract/qs-receipt.json";
import waitingExamples from "../contract/waiting-send.json";
import sessionExamples from "../contract/qs-session.json";
import sendExamples from "../contract/qs-send.json";
import sessionReceiptExamples from "../contract/qs-session-receipt.json";
import { violations } from "../testing/a11y";
import { localization } from "../i18n";
import App from "../App";
import KnownHost from "./QsKnownHost";
import KnownReceiveHost from "./QsKnownIncoming";
import { noteProblem } from "./QsNote";
import OpenSendActions from "./QsOpenSend";
import WaitingHost from "./QsWaiting";
import QuickHost from "./QuickHost";
import { INCOMING } from "../quicksend.fixtures";

/*
 * Quick Send stage 1 screens (Security Design T): notes and links, Remember this connection,
 * the remembered connections, a remembered laptop's incoming send, waiting sends, several
 * recipients in one approval, and files PCTwin was started with. Each drives the screen as the
 * person would and checks the command the engine is sent, and nothing more.
 */

const tauri = vi.hoisted(() => ({
  invoke: vi.fn(),
  handlers: new Map<string, ((event: { payload: unknown }) => void)[]>(),
}));

vi.mock("@tauri-apps/api/core", () => ({ invoke: tauri.invoke }));
vi.mock("@tauri-apps/api/event", () => ({
  listen: (name: string, handler: (event: { payload: unknown }) => void) => {
    tauri.handlers.set(name, [...(tauri.handlers.get(name) ?? []), handler]);
    return Promise.resolve(() => {
      tauri.handlers.set(
        name,
        (tauri.handlers.get(name) ?? []).filter((h) => h !== handler),
      );
    });
  },
}));

const l10n = localization("en");
const text = (id: string, vars?: Record<string, string | number>) => l10n.getString(id, vars);
const plain = (s: string) => s.replace(/[\u2066-\u2069]/g, "");

let warnings: string[] = [];
let replies: Partial<Record<string, (args: unknown) => unknown>> = {};

const DOWNLOADS = { name: "Downloads", path: "C:/Users/Ada/Downloads", freeBytes: 80_000_000_000 };
const [ADA, KITCHEN, DESKTOP] = knownExamples;

beforeEach(() => {
  warnings = [];
  replies = { pairing_heads_up: () => [], pairing_new_search: () => [] };
  tauri.handlers.clear();
  tauri.invoke.mockReset();
  tauri.invoke.mockImplementation((command: string, args?: unknown) => Promise.resolve().then(() => replies[command]?.(args)));
  vi.spyOn(console, "warn").mockImplementation((...args: unknown[]) => {
    warnings.push(args.map(String).join(" "));
  });
});

afterEach(() => {
  expect(warnings.filter((w) => /missing|not found/i.test(w))).toEqual([]);
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

function send(event: string, payload: unknown) {
  act(() => {
    for (const h of tauri.handlers.get(event) ?? []) h({ payload });
  });
}

/** The engine's refusal, as Tauri gives it: a plain object. */
function refuse(kind: string): () => never {
  return () => {
    // eslint-disable-next-line @typescript-eslint/only-throw-error
    throw { kind };
  };
}

const heading = () => plain(screen.getByRole("heading", { level: 1 }).textContent);
const called = (command: string) => tauri.invoke.mock.calls.some((c: unknown[]) => c[0] === command);
const page = () => plain(document.body.textContent);
const button = (name: string) => screen.getByRole("button", { name: (n) => plain(n) === plain(name) });

describe("a note or link", () => {
  it("is checked by the engine's rules before pairing, then sent the moment the laptops are paired", async () => {
    inApp(<QuickHost mode="note" onHome={vi.fn()} />);
    expect(heading()).toBe(text("qs-note-title"));
    expect((button(text("qs-choose-continue")) as HTMLButtonElement).disabled).toBe(true);
    const box = screen.getByRole("textbox", { name: text("qs-note-label") });
    fireEvent.change(box, { target: { value: "x".repeat(1001) } });
    fireEvent.click(button(text("qs-choose-continue")));
    expect(page()).toContain(text("qs-refusal-note-too-long"));
    expect(called("pairing_old_start")).toBe(false);
    fireEvent.change(box, { target: { value: "Wi-Fi: Home\nPassword: blue-river-42" } });
    fireEvent.click(button(text("qs-choose-continue")));
    await waitFor(() => {
      expect(tauri.invoke).toHaveBeenCalledWith("pairing_old_start", { network: null });
    });
    expect(called("quicksend_send_note")).toBe(false);
    send("pairing-old", { kind: "paired" });
    await waitFor(() => {
      expect(tauri.invoke).toHaveBeenCalledWith("quicksend_send_note", { text: "Wi-Fi: Home\nPassword: blue-river-42" });
    });
    expect(heading()).toBe(text("qs-note-sending-title"));
    send("quicksend-note-sent", { taken: true });
    expect(heading()).toBe(text("qs-note-sent-title"));
    expect(page()).toContain(text("qs-note-sent-body"));
    expect(await violations()).toEqual([]);
  });

  it("the same rules as the engine: empty, too long, too many lines", () => {
    expect(noteProblem("   \r\n ")).toEqual({ kind: "note-empty" });
    expect(noteProblem("a".repeat(1000))).toBeNull();
    expect(noteProblem("a".repeat(1001))).toEqual({ kind: "note-too-long" });
    expect(noteProblem(Array(20).fill("a").join("\r\n"))).toBeNull();
    expect(noteProblem(Array(21).fill("a").join("\n"))).toEqual({ kind: "note-too-many-lines" });
  });

  it("a note the engine refuses after pairing goes back to the box, said there", async () => {
    replies.quicksend_send_note = refuse("note-hidden");
    inApp(<QuickHost mode="note" onHome={vi.fn()} />);
    fireEvent.change(screen.getByRole("textbox", { name: text("qs-note-label") }), { target: { value: "hello" } });
    fireEvent.click(button(text("qs-choose-continue")));
    await waitFor(() => {
      expect(called("pairing_old_start")).toBe(true);
    });
    send("pairing-old", { kind: "paired" });
    await waitFor(() => {
      expect(heading()).toBe(text("qs-note-title"));
    });
    expect(page()).toContain(text("qs-refusal-note-hidden"));
  });

  it("a received link shows its site, never opens by itself, and Open names this note", async () => {
    const writeText = vi.fn(() => Promise.resolve());
    Object.defineProperty(navigator, "clipboard", { value: { writeText }, configurable: true });
    replies.quicksend_folder = () => DOWNLOADS;
    inApp(<QuickHost mode="receive" onHome={vi.fn()} />);
    await waitFor(() => {
      expect(called("pairing_new_search")).toBe(true);
    });
    send("pairing-new", { kind: "paired" });
    await waitFor(() => {
      expect(called("quicksend_receive")).toBe(true);
    });
    send("quicksend-note", { ...noteExamples[1], from: null });
    expect(heading()).toBe(text("qs-note-received-link-title"));
    expect(page()).toContain("https://example.org/menu");
    expect(page()).toContain(plain(text("qs-note-link-to", { host: "example.org" })));
    expect(page()).toContain(text("qs-note-from-code"));
    expect(called("quicksend_open_link")).toBe(false);
    fireEvent.click(button(text("qs-note-copy")));
    await waitFor(() => {
      expect(page()).toContain(text("qs-note-copied"));
    });
    expect(writeText).toHaveBeenCalledWith("https://example.org/menu");
    fireEvent.click(button(text("qs-note-open")));
    await waitFor(() => {
      expect(tauri.invoke).toHaveBeenCalledWith("quicksend_open_link", { id: noteExamples[1].id });
    });
    expect(await violations()).toEqual([]);
  });

  it("a plain note has Copy and no Open", async () => {
    replies.quicksend_folder = () => DOWNLOADS;
    inApp(<QuickHost mode="receive" onHome={vi.fn()} />);
    await waitFor(() => {
      expect(called("pairing_new_search")).toBe(true);
    });
    send("pairing-new", { kind: "paired" });
    await waitFor(() => {
      expect(called("quicksend_receive")).toBe(true);
    });
    send("quicksend-note", noteExamples[0]);
    expect(heading()).toBe(text("qs-note-received-title"));
    expect(screen.queryByRole("button", { name: text("qs-note-open") })).toBeNull();
    expect(button(text("qs-note-copy"))).toBeTruthy();
  });
});

describe("Remember this connection", () => {
  async function receivedWithRemember() {
    replies.quicksend_folder = () => DOWNLOADS;
    replies.known_remember = () => knownExamples;
    inApp(<QuickHost mode="receive" onHome={vi.fn()} />);
    await waitFor(() => {
      expect(called("pairing_new_search")).toBe(true);
    });
    send("pairing-new", { kind: "paired" });
    await waitFor(() => {
      expect(called("quicksend_receive")).toBe(true);
    });
    send("quicksend-incoming", { files: [], prompt: 1 });
    fireEvent.click(button(text("qs-where-save")));
    send("quicksend-receipt", receipts[0]);
    send("quicksend-remember", { fallbackStore: false, connections: [{ id: ADA.id, label: ADA.label, lastUsed: ADA.lastUsed }, { id: KITCHEN.id, label: KITCHEN.label, lastUsed: KITCHEN.lastUsed }] });
    return within(screen.getByRole("region", { name: text("qs-remember-ask") }));
  }

  it("is offered after the send, and a name that matches an earlier connection has it chosen already", async () => {
    const card = await receivedWithRemember();
    expect(card.getByRole("radio", { name: text("qs-remember-keep-both") })).toHaveProperty("checked", true);
    fireEvent.change(card.getByRole("textbox", { name: text("qs-remember-name") }), { target: { value: " kitchen LAPTOP " } });
    expect(card.getByRole("radio", { name: (n) => n.includes("Kitchen laptop") })).toHaveProperty("checked", true);
    expect(plain(card.getByRole("status").textContent)).toBe(plain(text("qs-remember-replaces-suggested", { label: "Kitchen laptop" })));
    fireEvent.click(card.getByRole("button", { name: text("qs-remember-replace") }));
    await waitFor(() => {
      expect(tauri.invoke).toHaveBeenCalledWith("known_remember", { label: "kitchen LAPTOP", replaces: KITCHEN.id });
    });
    await waitFor(() => {
      expect(page()).toContain(plain(text("qs-remember-done", { label: "kitchen LAPTOP" })));
    });
    expect(await violations()).toEqual([]);
  });

  it("the chosen replacement can be unticked: keep both", async () => {
    const card = await receivedWithRemember();
    fireEvent.change(card.getByRole("textbox", { name: text("qs-remember-name") }), { target: { value: "Kitchen laptop" } });
    fireEvent.click(card.getByRole("radio", { name: text("qs-remember-keep-both") }));
    // Typing again no longer moves the person's own choice.
    fireEvent.change(card.getByRole("textbox", { name: text("qs-remember-name") }), { target: { value: "Kitchen laptop " } });
    expect(card.getByRole("radio", { name: text("qs-remember-keep-both") })).toHaveProperty("checked", true);
    fireEvent.click(card.getByRole("button", { name: text("qs-remember-save") }));
    await waitFor(() => {
      expect(tauri.invoke).toHaveBeenCalledWith("known_remember", { label: "Kitchen laptop", replaces: null });
    });
  });

  it("a refusal is said on the card", async () => {
    const card = await receivedWithRemember();
    replies.known_remember = refuse("too-many-connections");
    fireEvent.change(card.getByRole("textbox", { name: text("qs-remember-name") }), { target: { value: "Ada" } });
    fireEvent.click(card.getByRole("button", { name: text("qs-remember-save") }));
    await waitFor(() => {
      expect(page()).toContain(text("qs-refusal-too-many-connections"));
    });
  });
});

describe("remembered connections", () => {
  it("lists each with last used, what PCTwin can say about it, and its folder; each control sends its own command", async () => {
    replies.known_list = () => knownExamples;
    replies.known_allow_folder = () => knownExamples;
    replies.known_renew_folder = () => knownExamples;
    replies.known_clear_next_folder = () => knownExamples;
    replies.known_rename = () => knownExamples;
    replies.known_forget = () => knownExamples.slice(1);
    inApp(<KnownHost onSend={vi.fn()} onBack={vi.fn()} />);
    expect(heading()).toBe(text("qs-known-title"));
    await waitFor(() => {
      expect(page()).toContain("Ada's laptop");
    });
    expect(page()).toContain(text("qs-known-same-device"));
    expect(page()).toContain(text("qs-known-restored"));
    expect(page()).toContain(text("qs-known-cannot-tell"));
    // Never "verified", never a person.
    expect(page()).not.toMatch(/verified|trusted/i);
    expect(page()).toContain(plain(text("qs-known-folder-on", { folder: "From Ada" })));
    expect(page()).toContain(plain(text("qs-known-next-on", { folder: "Photos" })));
    expect(await violations()).toEqual([]);

    const cards = screen.getAllByRole("listitem").filter((li) => li.classList.contains("qs-known"));
    const ada = within(cards[0]);
    const kitchen = within(cards[1]);
    expect(ada.getByRole("switch", { name: text("qs-known-allow-folder") }).getAttribute("aria-checked")).toBe("true");
    expect(kitchen.getByRole("switch", { name: text("qs-known-allow-folder") }).getAttribute("aria-checked")).toBe("false");
    fireEvent.click(kitchen.getByRole("switch", { name: text("qs-known-allow-folder") }));
    await waitFor(() => {
      expect(tauri.invoke).toHaveBeenCalledWith("known_allow_folder", { id: KITCHEN.id });
    });
    fireEvent.click(ada.getByRole("button", { name: text("qs-known-renew") }));
    await waitFor(() => {
      expect(tauri.invoke).toHaveBeenCalledWith("known_renew_folder", { id: ADA.id });
    });
    fireEvent.click(ada.getByRole("button", { name: text("qs-known-next-clear") }));
    await waitFor(() => {
      expect(tauri.invoke).toHaveBeenCalledWith("known_clear_next_folder", { id: ADA.id });
    });
    fireEvent.click(ada.getByRole("button", { name: text("qs-known-rename") }));
    fireEvent.change(ada.getByRole("textbox", { name: text("qs-known-rename-label") }), { target: { value: "Ada's new laptop" } });
    fireEvent.click(ada.getByRole("button", { name: text("qs-known-rename-save") }));
    await waitFor(() => {
      expect(tauri.invoke).toHaveBeenCalledWith("known_rename", { id: ADA.id, label: "Ada's new laptop" });
    });
    // Forget asks once more first.
    fireEvent.click(ada.getByRole("button", { name: text("qs-known-forget") }));
    expect(called("known_forget")).toBe(false);
    fireEvent.click(screen.getByRole("button", { name: text("qs-known-forget-confirm") }));
    await waitFor(() => {
      expect(tauri.invoke).toHaveBeenCalledWith("known_forget", { id: ADA.id });
    });
    await waitFor(() => {
      expect(page()).not.toContain("Ada's laptop");
    });
  });

  it("turning the folder permission off revokes it; a refused folder is said on that card", async () => {
    replies.known_list = () => knownExamples;
    replies.known_revoke_folder = () => knownExamples;
    replies.known_allow_folder = refuse("folder-runs-at-sign-in");
    inApp(<KnownHost onSend={vi.fn()} onBack={vi.fn()} />);
    await waitFor(() => {
      expect(page()).toContain("Kitchen laptop");
    });
    const cards = screen.getAllByRole("listitem").filter((li) => li.classList.contains("qs-known"));
    fireEvent.click(within(cards[0]).getByRole("switch", { name: text("qs-known-allow-folder") }));
    await waitFor(() => {
      expect(tauri.invoke).toHaveBeenCalledWith("known_revoke_folder", { id: ADA.id });
    });
    fireEvent.click(within(cards[2]).getByRole("switch", { name: text("qs-known-allow-folder") }));
    await waitFor(() => {
      expect(within(cards[2]).getByRole("alert").textContent).toContain(text("qs-refusal-folder-runs-at-sign-in"));
    });
    expect(within(cards[0]).queryByRole("alert")).toBeNull();
  });

  it("follows known-changed, and says when none are remembered", async () => {
    replies.known_list = () => [];
    inApp(<KnownHost onSend={vi.fn()} onBack={vi.fn()} />);
    await waitFor(() => {
      expect(page()).toContain(text("qs-known-empty"));
    });
    send("known-changed", [DESKTOP]);
    expect(page()).toContain("Old desktop");
    expect(page()).not.toContain(text("qs-known-empty"));
  });
});

describe("a remembered laptop sends files", () => {
  it("asks where to save (Downloads held when it has no folder for next time), and saves only on Save", async () => {
    replies.quicksend_folder = () => DOWNLOADS;
    const incoming = { ...incomingExamples[1] };
    const onHome = vi.fn();
    inApp(<KnownReceiveHost incoming={incoming} onHome={onHome} />);
    expect(heading()).toBe(text("qs-where-title"));
    expect(page()).toContain(plain(text("qs-known-wants-to-send", { label: "Ada's laptop" })));
    expect(page()).toContain("beach.jpg");
    await waitFor(() => {
      expect(page()).toContain(plain(text("qs-where-folder", { folder: "Downloads" })));
    });
    expect(called("known_save_incoming")).toBe(false);
    expect(await violations()).toEqual([]);
    fireEvent.click(screen.getByRole("checkbox", { name: (n) => plain(n) === plain(text("qs-known-in-next", { label: "Ada's laptop" })) }));
    fireEvent.click(button(text("qs-where-save")));
    await waitFor(() => {
      expect(tauri.invoke).toHaveBeenCalledWith("known_save_incoming", { nextTime: true });
    });
    send("quicksend-progress", { phase: "copying", filesDone: 0, filesTotal: 1, bytesDone: 0, bytesTotal: 2_400_000, current: null, failed: 0, speed: null, minutesLeft: null });
    expect(heading()).toBe(text("qs-receiving-title"));
    send("quicksend-receipt", receipts[0]);
    expect(heading()).toBe(text("qs-done-partial-title"));
  });

  it("Decline writes nothing and tells the engine", async () => {
    const onHome = vi.fn();
    inApp(<KnownReceiveHost incoming={INCOMING} onHome={onHome} />);
    expect(page()).toContain(plain(text("qs-where-folder", { folder: "Downloads" })));
    expect(called("quicksend_folder")).toBe(false);
    fireEvent.click(button(text("qs-known-in-decline")));
    await waitFor(() => {
      expect(called("known_decline_incoming")).toBe(true);
    });
    expect(onHome).toHaveBeenCalled();
    expect(called("known_save_incoming")).toBe(false);
  });
});

describe("choosing who gets the files", () => {
  async function atRecipients(onWaiting = vi.fn()) {
    replies.known_list = () => knownExamples;
    replies.quicksend_pick = () => [{ id: "p1", name: "letter.txt", kind: "file", included: true, bytes: 1200, files: 1 }];
    replies.quicksend_offer = () => [{ id: "f1", path: "letter.txt", size: 1200 }];
    inApp(<QuickHost mode="send" onHome={vi.fn()} onWaiting={onWaiting} />);
    await waitFor(() => {
      expect(called("known_list")).toBe(true);
    });
    fireEvent.click(screen.getByRole("button", { name: new RegExp(text("qs-add-files")) }));
    await waitFor(() => {
      expect(screen.getByText("letter.txt")).toBeTruthy();
    });
    fireEvent.click(button(text("qs-choose-continue")));
    await waitFor(() => {
      expect(heading()).toBe(text("qs-to-title"));
    });
    return onWaiting;
  }

  it("one remembered laptop, now: sent over its own connection, then the list of sends", async () => {
    await atRecipients();
    expect(await violations()).toEqual([]);
    fireEvent.click(screen.getByRole("checkbox", { name: (n) => n.includes("Ada's laptop") }));
    fireEvent.click(button(text("qs-to-send-now")));
    await waitFor(() => {
      expect(tauri.invoke).toHaveBeenCalledWith("known_send", { id: ADA.id });
    });
    await waitFor(() => {
      expect(heading()).toBe(text("qs-sends-title"));
    });
    expect(called("pairing_old_start")).toBe(false);
  });

  it("several laptops now: the same files offered again for each, each sent over its own connection", async () => {
    replies.known_send = (args) => {
      if ((args as { id: string }).id === DESKTOP.id) refuse("not-nearby")();
      return null;
    };
    await atRecipients();
    fireEvent.click(screen.getByRole("checkbox", { name: (n) => n.includes("Ada's laptop") }));
    fireEvent.click(screen.getByRole("checkbox", { name: (n) => n.includes("Old desktop") }));
    expect(page()).toContain(text("qs-to-now-several-hint"));
    fireEvent.click(button(text("qs-to-send-now")));
    await waitFor(() => {
      expect(heading()).toBe(text("qs-sends-title"));
    });
    const sends = tauri.invoke.mock.calls.filter((c: unknown[]) => c[0] === "known_send").map((c: unknown[]) => c[1]);
    expect(sends).toEqual([{ id: ADA.id }, { id: DESKTOP.id }]);
    // Offered once on the pick page, and again before the second laptop.
    expect(tauri.invoke.mock.calls.filter((c: unknown[]) => c[0] === "quicksend_offer").length).toBe(2);
    // The laptop that could not be reached is said by name, with why.
    expect(page()).toContain(plain(text("qs-sends-not-sent", { name: DESKTOP.label })));
    expect(page()).toContain(text("qs-refusal-not-nearby"));
    expect(await violations()).toEqual([]);
  });

  it("not nearby: said on the same page, nothing else happens", async () => {
    await atRecipients();
    replies.known_send = refuse("not-nearby");
    fireEvent.click(screen.getByRole("checkbox", { name: (n) => n.includes("Ada's laptop") }));
    fireEvent.click(button(text("qs-to-send-now")));
    await waitFor(() => {
      expect(page()).toContain(text("qs-refusal-not-nearby"));
    });
    expect(heading()).toBe(text("qs-to-title"));
  });

  it("several laptops in one approval: a waiting send for each, for the days chosen", async () => {
    replies.waiting_prepare = () => waitingExamples;
    const onWaiting = await atRecipients();
    fireEvent.click(screen.getByRole("checkbox", { name: (n) => n.includes("Ada's laptop") }));
    fireEvent.click(screen.getByRole("checkbox", { name: (n) => n.includes("Old desktop") }));
    // Several can go now too; When it is nearby makes one approval for all of them.
    fireEvent.click(screen.getByRole("radio", { name: text("qs-to-later") }));
    expect(screen.queryByRole("button", { name: text("qs-to-send-now") })).toBeNull();
    fireEvent.change(screen.getByRole("combobox", { name: text("qs-to-days") }), { target: { value: "14" } });
    fireEvent.click(button(text("qs-to-approve")));
    await waitFor(() => {
      expect(tauri.invoke).toHaveBeenCalledWith("waiting_prepare", { ids: [ADA.id, DESKTOP.id], days: 14 });
    });
    expect(onWaiting).toHaveBeenCalled();
    expect(called("known_send")).toBe(false);
  });

  it("Use a code instead pairs as before", async () => {
    await atRecipients();
    fireEvent.click(button(text("qs-to-code")));
    await waitFor(() => {
      expect(tauri.invoke).toHaveBeenCalledWith("pairing_old_start", { network: null });
    });
  });
});

describe("files PCTwin was started with", () => {
  it("join the pick page without the picker", async () => {
    replies.quicksend_launch_files = () => [{ id: "p9", name: "Handed over.pdf", kind: "file", included: true, bytes: 5000, files: null }];
    inApp(<QuickHost mode="send" onHome={vi.fn()} />);
    await waitFor(() => {
      expect(screen.getByText("Handed over.pdf")).toBeTruthy();
    });
    expect(called("quicksend_pick")).toBe(false);
    send("quicksend-launch", [
      { id: "p9", name: "Handed over.pdf", kind: "file", included: true, bytes: 5000, files: null },
      { id: "p10", name: "Second.txt", kind: "file", included: true, bytes: 10, files: null },
    ]);
    expect(screen.getByText("Second.txt")).toBeTruthy();
  });
});

describe("waiting sends", () => {
  it("shows each laptop's own result, a newer version, and Renew and Cancel send their commands", async () => {
    replies.waiting_list = () => waitingExamples;
    replies.waiting_renew = () => waitingExamples;
    replies.waiting_cancel = () => [];
    inApp(<WaitingHost onSend={vi.fn()} onBack={vi.fn()} />);
    expect(heading()).toBe(text("qs-waiting-title"));
    await waitFor(() => {
      expect(page()).toContain("Kitchen laptop");
    });
    expect(page()).toContain(text("qs-waiting-state-waiting"));
    expect(page()).toContain(plain(text("qs-waiting-state-delivered", { arrived: 3, total: 3 })));
    expect(page()).toContain(text("qs-waiting-to-unknown"));
    expect(page()).toContain(plain(text("qs-waiting-state-failed", { count: 1 })));
    expect(page()).toContain(text("qs-waiting-newer"));
    expect(page()).toContain("menu.txt");
    expect(await violations()).toEqual([]);
    fireEvent.change(screen.getByRole("combobox", { name: text("qs-waiting-renew-label") }), { target: { value: "14" } });
    fireEvent.click(button(text("qs-waiting-renew")));
    await waitFor(() => {
      expect(tauri.invoke).toHaveBeenCalledWith("waiting_renew", { id: waitingExamples[0].id, days: 14 });
    });
    fireEvent.click(button(text("qs-waiting-cancel")));
    expect(called("waiting_cancel")).toBe(false);
    const dialog = within(screen.getByRole("dialog"));
    fireEvent.click(dialog.getByRole("button", { name: text("qs-waiting-cancel") }));
    await waitFor(() => {
      expect(tauri.invoke).toHaveBeenCalledWith("waiting_cancel", { id: waitingExamples[0].id });
    });
    await waitFor(() => {
      expect(page()).toContain(text("qs-waiting-empty"));
    });
  });
});

describe("the open send's own buttons", () => {
  const [OPEN, ENDING] = sessionExamples;
  it("show nothing without an open send; Add files on the sender, Send to <name> on the receiver, and End", () => {
    const { container } = inApp(<OpenSendActions sessions={[]} peer="Receiving laptop" side="send" onAdd={vi.fn()} onEnd={vi.fn()} />);
    expect(container.querySelector("[data-slot=open-send]")).toBeNull();
    cleanup();
    const onAdd = vi.fn();
    const onEnd = vi.fn();
    inApp(<OpenSendActions sessions={[{ ...OPEN, name: null }]} peer="Receiving laptop" side="send" onAdd={onAdd} onEnd={onEnd} />);
    fireEvent.click(button(text("qs-session-add-files")));
    expect(onAdd).toHaveBeenCalledWith({ ...OPEN, name: null });
    // The six-digit send has no name from the engine: the page's own name for the other laptop.
    fireEvent.click(screen.getByRole("button", { name: (n) => plain(n) === plain(text("qs-sends-action-name", { what: text("qs-session-end"), name: "Receiving laptop" })) }));
    expect(onEnd).toHaveBeenCalled();
    cleanup();
    inApp(<OpenSendActions sessions={[OPEN]} peer="Sending laptop" side="receive" onAdd={onAdd} onEnd={onEnd} />);
    expect(button(text("qs-session-send-to", { name: "Ada's laptop" }))).toBeTruthy();
  });

  it("an ending send says so and offers nothing", () => {
    inApp(<OpenSendActions sessions={[ENDING]} peer="Receiving laptop" side="send" onAdd={vi.fn()} onEnd={vi.fn()} />);
    expect(page()).toContain(text("qs-session-ended"));
    expect(screen.queryByRole("button")).toBeNull();
  });
});

const PICKED = { id: "p1", name: "letter.txt", kind: "file", included: true, bytes: 1200, files: 1 };

describe("an open send, after the first files", () => {
  const SESSION = sessionExamples[0].id;
  async function sentWithTheCode() {
    replies.quicksend_offer = () => [{ id: "f1", path: "letter.txt", size: 1200 }];
    replies.quicksend_pick = () => [PICKED];
    inApp(<QuickHost mode="send" onHome={vi.fn()} />);
    fireEvent.click(screen.getByRole("button", { name: new RegExp(text("qs-add-files")) }));
    await waitFor(() => {
      expect(screen.getByText("letter.txt")).toBeTruthy();
    });
    fireEvent.click(await screen.findByRole("button", { name: (n) => plain(n) === plain(text("qs-choose-continue")) }));
    await waitFor(() => {
      expect(called("pairing_old_start")).toBe(true);
    });
    send("pairing-old", { kind: "paired" });
    await waitFor(() => {
      expect(called("quicksend_send")).toBe(true);
    });
    send("quicksend-receipt", receipts[0]);
    // The engine opens the send for more, with no name: the code's pairing.
    send("quicksend-sessions", [{ id: SESSION, name: null, ending: false }]);
  }

  it("the sender's Add files: a fresh pick page, the offer queued in that send, then the list with Pause and Cancel", async () => {
    await sentWithTheCode();
    replies.quicksend_session_add = () => [{ ...sendExamples[0], session: SESSION, name: null, state: "waiting" }];
    fireEvent.click(await screen.findByRole("button", { name: text("qs-session-add-files") }));
    await waitFor(() => {
      expect(called("quicksend_clear")).toBe(true);
    });
    fireEvent.click(screen.getByRole("button", { name: new RegExp(text("qs-add-files")) }));
    fireEvent.click(await screen.findByRole("button", { name: (n) => plain(n) === plain(text("qs-choose-continue")) }));
    await waitFor(() => {
      expect(tauri.invoke).toHaveBeenCalledWith("quicksend_session_add", { session: SESSION });
    });
    await waitFor(() => {
      expect(heading()).toBe(text("qs-sends-title"));
    });
    // Named as the page names the other laptop; no second pairing.
    expect(page()).toContain(text("qs-label-receiving"));
    expect(page()).toContain(text("qs-sends-waiting"));
    expect(tauri.invoke.mock.calls.filter((c: unknown[]) => c[0] === "pairing_old_start").length).toBe(1);
    replies.quicksend_send_pause = () => [{ ...sendExamples[0], session: SESSION, name: null, state: "waiting", paused: true }];
    fireEvent.click(screen.getByRole("button", { name: (n) => plain(n) === plain(text("qs-sends-action-name", { what: text("qs-send-pause"), name: text("qs-label-receiving") })) }));
    await waitFor(() => {
      expect(tauri.invoke).toHaveBeenCalledWith("quicksend_send_pause", { id: 1 });
    });
    await waitFor(() => {
      expect(page()).toContain(text("qs-sends-paused"));
    });
    replies.quicksend_send_cancel = () => [];
    fireEvent.click(screen.getByRole("button", { name: (n) => plain(n) === plain(text("qs-sends-action-name", { what: text("qs-send-cancel"), name: text("qs-label-receiving") })) }));
    await waitFor(() => {
      expect(tauri.invoke).toHaveBeenCalledWith("quicksend_send_cancel", { id: 1 });
    });
    // Its result comes by the receiving laptop's own count, never before.
    send("quicksend-sends", [{ ...sendExamples[0], session: SESSION, name: null }]);
    send("quicksend-session-receipt", { ...sessionReceiptExamples[0], session: SESSION, send: 1 });
    send("quicksend-sends", []);
    await waitFor(() => {
      expect(page()).toContain(text("qs-sends-ended-title"));
    });
    expect(page()).toContain(plain(text("qs-known-delivered-counts", { arrived: 2, total: 5 })));
    expect(await violations()).toEqual([]);
  });

  it("the other laptop's Send to: asked where to save like the first, then received, then its Done page", async () => {
    await sentWithTheCode();
    replies.quicksend_folder = () => DOWNLOADS;
    send("quicksend-incoming", { files: [{ id: "00112233445566778899aabbccddeef9", path: "back.pdf", size: 20 }], prompt: 2 });
    await waitFor(() => {
      expect(heading()).toBe(text("qs-where-title"));
    });
    await waitFor(() => {
      expect(page()).toContain(plain(text("qs-where-folder", { folder: "Downloads" })));
    });
    fireEvent.click(button(text("qs-where-save")));
    await waitFor(() => {
      expect(called("quicksend_save_here")).toBe(true);
    });
    send("quicksend-session-receipt", { session: SESSION, send: null, receipt: receipts.find((r) => r.side === "receiver") ?? receipts[0] });
    await waitFor(() => {
      expect(screen.getByRole("heading", { level: 1 }).textContent).not.toBe(text("qs-where-title"));
    });
    expect(screen.queryByRole("button", { name: text("qs-where-save") })).toBeNull();
  });

  it("declining more files keeps the send open and goes back", async () => {
    await sentWithTheCode();
    replies.quicksend_folder = () => DOWNLOADS;
    send("quicksend-incoming", { files: [{ id: "00112233445566778899aabbccddeef9", path: "back.pdf", size: 20 }], prompt: 2 });
    await waitFor(() => {
      expect(heading()).toBe(text("qs-where-title"));
    });
    fireEvent.click(button(text("qs-where-stop")));
    await waitFor(() => {
      expect(called("quicksend_stop")).toBe(true);
    });
    expect(called("quicksend_session_end")).toBe(false);
    expect(button(text("qs-session-add-files"))).toBeTruthy();
  });

  it("End closes the send", async () => {
    await sentWithTheCode();
    fireEvent.click(await screen.findByRole("button", { name: (n) => plain(n) === plain(text("qs-sends-action-name", { what: text("qs-session-end"), name: text("qs-label-receiving") })) }));
    await waitFor(() => {
      expect(tauri.invoke).toHaveBeenCalledWith("quicksend_session_end", { session: SESSION });
    });
  });
});

describe("a remembered laptop asking while the person is busy", () => {
  it("is said at the top, never dropped, and asked as soon as the busy page is left", async () => {
    replies.quicksend_folder = () => DOWNLOADS;
    render(<App initialLanguage="en" />);
    await act(async () => {});
    // PCTwin was handed files to send: the send's pick page opens.
    send("quicksend-launch", [PICKED]);
    await waitFor(() => {
      expect(heading()).toBe(text("qs-choose-title"));
    });
    send("known-incoming", INCOMING);
    // The send page stays; the question waits, said in words.
    expect(heading()).toBe(text("qs-choose-title"));
    expect(page()).toContain(plain(text("qs-asking-files", { label: INCOMING.label })));
    expect(page()).toContain(text("qs-asking-later"));
    expect(await violations()).toEqual([]);
    fireEvent.click(screen.getByRole("button", { name: "PCTwin home" }));
    await waitFor(() => {
      expect(heading()).toBe(text("qs-where-title"));
    });
    expect(page()).not.toContain(plain(text("qs-asking-later")));
  });
});

describe("from anywhere in the app", () => {
  it("a remembered laptop's send opens its question, and a note from one shows with its name", async () => {
    render(<App initialLanguage="en" />);
    await act(async () => {});
    send("known-incoming", INCOMING);
    await waitFor(() => {
      expect(heading()).toBe(text("qs-where-title"));
    });
    fireEvent.click(button(text("qs-known-in-decline")));
    await waitFor(() => {
      expect(called("known_decline_incoming")).toBe(true);
    });
    send("quicksend-note", noteExamples[1]);
    await waitFor(() => {
      expect(heading()).toBe(text("qs-note-received-link-title"));
    });
    expect(page()).toContain(plain(text("qs-note-from", { label: "Ada's laptop" })));
  });

  it("files that landed in an allowed folder are told on the first after a quiet spell", async () => {
    render(<App initialLanguage="en" />);
    await act(async () => {});
    send("known-delivered", { connection: ADA.id, label: "Ada's laptop", folder: "From Ada", lines: [{ path: "a.jpg", outcome: { kind: "arrived", landedAs: "a.jpg" } }], tell: false });
    expect(page()).not.toContain(plain(text("qs-known-delivered", { label: "Ada's laptop", folder: "From Ada" })));
    send("known-delivered", { connection: ADA.id, label: "Ada's laptop", folder: "From Ada", lines: [{ path: "a.jpg", outcome: { kind: "arrived", landedAs: "a.jpg" } }], tell: true });
    expect(page()).toContain(plain(text("qs-known-delivered", { label: "Ada's laptop", folder: "From Ada" })));
    expect(page()).toContain(plain(text("qs-known-delivered-counts", { arrived: 1, total: 1 })));
  });
});
