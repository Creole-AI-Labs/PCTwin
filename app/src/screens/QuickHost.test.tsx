import { LocalizationProvider } from "@fluent/react";
import type { ReactNode } from "react";
import { act, cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import offeredExamples from "../contract/qs-offered.json";
import receipts from "../contract/qs-receipt.json";
import progress from "../contract/qs-progress.json";
import { formatBytes } from "../move";
import { violations } from "../testing/a11y";
import { localization } from "../i18n";
import QuickHost from "./QuickHost";

/*
 * Quick Send, as decided on 26 October 2026: the sender picks, pairs, and the send starts the moment
 * the laptops are paired; the receiver sees where the files will be saved (Downloads unless it
 * chose another folder), pairs, and receives everything offered with no second question. Both end
 * on the Done page.
 */

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
const plain = (s: string) => s.replace(/[\u2066-\u2069]/g, "");

let warnings: string[] = [];
let replies: Partial<Record<string, (args: unknown) => unknown>> = {};

const DOWNLOADS = { name: "Downloads", path: "C:/Users/Ada/Downloads", freeBytes: 80_000_000_000 };

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
    tauri.handlers.get(event)?.({ payload });
  });
}

const heading = () => plain(screen.getByRole("heading", { level: 1 }).textContent);
const called = (command: string) => tauri.invoke.mock.calls.some((c: unknown[]) => c[0] === command);
const page = () => plain(document.body.textContent);

/** The sender, from Quick Send's home to its pairing page with one file chosen. */
async function senderAtPairing() {
  replies.quicksend_pick = () => [{ id: "p1", name: "letter.txt", kind: "file", included: true, bytes: 1200, files: 1 }];
  replies.quicksend_offer = () => offeredExamples;
  inApp(<QuickHost mode="send" onHome={vi.fn()} />);
  fireEvent.click(screen.getByRole("button", { name: new RegExp(text("qs-add-files")) }));
  await waitFor(() => {
    expect(screen.getByText("letter.txt")).toBeTruthy();
  });
  fireEvent.click(screen.getByRole("button", { name: new RegExp(text("qs-choose-continue")) }));
  await waitFor(() => {
    expect(tauri.invoke).toHaveBeenCalledWith("pairing_old_start", { network: null });
  });
}

describe("sending", () => {
  it("picks, offers only the ticked things, and goes straight to pairing (no check or approve page)", async () => {
    replies.quicksend_pick = () => [
      { id: "p1", name: "letter.txt", kind: "file", included: true, bytes: 1200, files: 1 },
      { id: "p2", name: "Holiday", kind: "folder", included: true, bytes: null, files: null },
    ];
    replies.quicksend_offer = () => offeredExamples;
    inApp(<QuickHost mode="send" onHome={vi.fn()} />);
    expect(heading()).toBe(text("qs-choose-title"));
    fireEvent.click(screen.getByRole("button", { name: new RegExp(text("qs-add-files")) }));
    await waitFor(() => {
      expect(screen.getByText("letter.txt")).toBeTruthy();
    });
    expect(tauri.invoke).toHaveBeenCalledWith("quicksend_pick", { folders: false });
    fireEvent.click(screen.getByRole("checkbox", { name: (n) => n.includes("Holiday") }));
    fireEvent.click(screen.getByRole("button", { name: new RegExp(text("qs-choose-continue")) }));
    await waitFor(() => {
      expect(tauri.invoke).toHaveBeenCalledWith("quicksend_offer", { ids: ["p1"] });
    });
    await waitFor(() => {
      expect(tauri.invoke).toHaveBeenCalledWith("pairing_old_start", { network: null });
    });
    send("pairing-old", { kind: "code", code: "123456", secondsLeft: 50 });
    expect(page()).toContain(plain(text("qs-pair-old-body")));
    expect(page()).not.toMatch(/old laptop|new laptop|receipt/i);
    expect(called("quicksend_send")).toBe(false);
  });

  it("the moment the laptops are paired, the send starts by itself: no Paired page, no click", async () => {
    await senderAtPairing();
    send("pairing-old", { kind: "code", code: "123456", secondsLeft: 50 });
    send("pairing-old", { kind: "paired" });
    await waitFor(() => {
      expect(called("quicksend_send")).toBe(true);
    });
    expect(heading()).toBe(text("qs-sending-title"));
    expect(screen.queryByText(text("pair-done-title"))).toBeNull();
    expect(tauri.invoke.mock.calls.filter((c: unknown[]) => c[0] === "quicksend_send")).toHaveLength(1);
  });

  it("ends on the Done page: what arrived in one line, and the files that did not", async () => {
    await senderAtPairing();
    send("pairing-old", { kind: "paired" });
    await waitFor(() => {
      expect(called("quicksend_send")).toBe(true);
    });
    send("quicksend-progress", { phase: "copying", filesDone: 1, filesTotal: 2, bytesDone: 1, bytesTotal: 2, current: null, failed: 0, speed: null, minutesLeft: null });
    send("quicksend-receipt", { ...receipts[0], side: "sender" });
    expect(heading()).toBe(text("qs-done-partial-title"));
    expect(page()).not.toMatch(/receipt/i);
  });

  it("Back on the pairing page returns to the chosen files", async () => {
    await senderAtPairing();
    fireEvent.click(screen.getByRole("button", { name: text("pair-back") }));
    await waitFor(() => {
      expect(heading()).toBe(text("qs-choose-title"));
    });
    expect(screen.getByText("letter.txt")).toBeTruthy();
  });
});

describe("too many files for one send", () => {
  it("is said plainly on the choosing page, before anything is sent", async () => {
    replies.quicksend_pick = () => [{ id: "p1", name: "Everything", kind: "folder", included: true, bytes: null, files: null }];
    replies.quicksend_offer = () => {
      // The engine's refusal arrives as Tauri gives it: a plain object, not an Error.
      // eslint-disable-next-line @typescript-eslint/only-throw-error
      throw { kind: "too-many-files" };
    };
    inApp(<QuickHost mode="send" onHome={vi.fn()} />);
    fireEvent.click(screen.getByRole("button", { name: new RegExp(text("qs-add-files")) }));
    await waitFor(() => {
      expect(screen.getByText("Everything")).toBeTruthy();
    });
    fireEvent.click(screen.getByRole("button", { name: new RegExp(text("qs-choose-continue")) }));
    await waitFor(() => {
      expect(screen.getAllByRole("alert").some((a) => a.textContent.includes(text("qs-pick-too-many")))).toBe(true);
    });
    expect(heading()).toBe(text("qs-choose-title"));
    expect(called("quicksend_send")).toBe(false);
  });
});

describe("sending the missing files", () => {
  it("from a Done page with files that did not arrive, offers only those again and goes straight to pairing", async () => {
    replies.quicksend_offer_missing = () => offeredExamples.slice(0, 1);
    await senderAtPairing();
    send("pairing-old", { kind: "paired" });
    await waitFor(() => {
      expect(called("quicksend_send")).toBe(true);
    });
    send("quicksend-receipt", { ...receipts[0], side: "sender" });
    // A late word from the engine never takes the person off this page.
    send("quicksend-progress", { phase: "copying", filesDone: 1, filesTotal: 2, bytesDone: 1, bytesTotal: 2, current: null, failed: 0, speed: null, minutesLeft: null });
    expect(heading()).toBe(text("qs-done-partial-title"));
    tauri.invoke.mockClear();
    fireEvent.click(screen.getByRole("button", { name: (n) => n.includes(text("qs-done-send-missing")) }));
    await waitFor(() => {
      expect(called("quicksend_offer_missing")).toBe(true);
    });
    await waitFor(() => {
      expect(tauri.invoke).toHaveBeenCalledWith("pairing_old_start", { network: null });
    });
    send("pairing-old", { kind: "paired" });
    await waitFor(() => {
      expect(called("quicksend_send")).toBe(true);
    });
  });
});

describe("dropping", () => {
  it("takes drops only while the pick page is open, shows what was dropped, and says when something is over the window", async () => {
    inApp(<QuickHost mode="send" onHome={vi.fn()} />);
    await waitFor(() => {
      expect(tauri.invoke).toHaveBeenCalledWith("quicksend_accept_drops", { on: true });
    });
    send("quicksend-dragging", true);
    expect(screen.getByText(text("qs-drop-active"))).toBeTruthy();
    send("quicksend-picked", [{ id: "p7", name: "photo.jpg", kind: "file", included: true, bytes: 900, files: 1 }]);
    expect(screen.getByText("photo.jpg")).toBeTruthy();
  });
});

describe("receiving", () => {
  it("the pairing page says the files will be saved in Downloads, with Change", async () => {
    replies.quicksend_folder = () => DOWNLOADS;
    replies.quicksend_choose_folder = () => ({ name: "Work", path: "D:/Work", freeBytes: null });
    inApp(<QuickHost mode="receive" onHome={vi.fn()} />);
    await waitFor(() => {
      expect(tauri.invoke).toHaveBeenCalledWith("pairing_new_search");
    });
    expect(called("quicksend_folder")).toBe(true);
    await waitFor(() => {
      expect(page()).toContain(plain(text("qs-folder-line", { folder: "Downloads" })));
    });
    fireEvent.click(screen.getByRole("button", { name: text("qs-folder-change") }));
    await waitFor(() => {
      expect(page()).toContain(plain(text("qs-folder-line", { folder: "Work" })));
    });
    expect(called("quicksend_choose_folder")).toBe(true);
    expect(page()).not.toMatch(/old laptop|new laptop|receipt/i);
  });

  it("the moment the laptops are paired, receives with no accept step, and ends on Done with Open folder", async () => {
    replies.quicksend_folder = () => DOWNLOADS;
    inApp(<QuickHost mode="receive" onHome={vi.fn()} />);
    await waitFor(() => {
      expect(tauri.invoke).toHaveBeenCalledWith("pairing_new_search");
    });
    send("pairing-new", { kind: "paired" });
    await waitFor(() => {
      expect(called("quicksend_receive")).toBe(true);
    });
    // What is coming is announced; nothing is saved until the person says where (29 October 2026).
    send("quicksend-incoming", { files: offeredExamples });
    expect(heading()).toBe(text("qs-where-title"));
    expect(called("quicksend_save_here")).toBe(false);
    fireEvent.click(screen.getByRole("button", { name: text("qs-where-save") }));
    await waitFor(() => {
      expect(called("quicksend_save_here")).toBe(true);
    });
    expect(heading()).toBe(text("qs-receiving-title"));
    send("quicksend-receipt", receipts[0]);
    expect(heading()).toBe(text("qs-done-partial-title"));
    // A file there but not finished is never shown as arrived.
    expect(page()).toContain(text("qs-done-needs-action"));
    expect(page()).toContain(plain(text("qs-done-folder", { folder: "Downloads" })));
    fireEvent.click(screen.getByRole("button", { name: text("qs-done-open-folder") }));
    await waitFor(() => {
      expect(called("quicksend_open_folder")).toBe(true);
    });
    expect(page()).not.toMatch(/receipt/i);
  });

  it("asks where the files go when they arrive: how many, how much, the folder, another folder, or Stop", async () => {
    replies.quicksend_folder = () => DOWNLOADS;
    replies.quicksend_choose_folder = () => ({ name: "Work", path: "D:/Work", freeBytes: null });
    const onHome = vi.fn();
    inApp(<QuickHost mode="receive" onHome={onHome} />);
    await waitFor(() => {
      expect(tauri.invoke).toHaveBeenCalledWith("pairing_new_search");
    });
    send("pairing-new", { kind: "paired" });
    await waitFor(() => {
      expect(called("quicksend_receive")).toBe(true);
    });
    send("quicksend-incoming", { files: offeredExamples });
    const total = offeredExamples.reduce((n, f) => n + f.size, 0);
    expect(page()).toContain(plain(text("qs-where-body", { count: offeredExamples.length, size: formatBytes(total, "en") })));
    expect(page()).toContain(plain(text("qs-where-folder", { folder: "Downloads" })));
    // A progress word before saving does not take the page.
    send("quicksend-progress", progress[0]);
    expect(heading()).toBe(text("qs-where-title"));
    expect(await violations()).toEqual([]);
    fireEvent.click(screen.getByRole("button", { name: text("qs-where-choose") }));
    await waitFor(() => {
      expect(page()).toContain(plain(text("qs-where-folder", { folder: "Work" })));
    });
    document.documentElement.dir = "rtl";
    expect(await violations()).toEqual([]);
    document.documentElement.dir = "ltr";
    fireEvent.click(screen.getByRole("button", { name: text("qs-where-stop") }));
    await waitFor(() => {
      expect(called("quicksend_stop")).toBe(true);
    });
    expect(called("quicksend_save_here")).toBe(false);
    expect(onHome).toHaveBeenCalled();
  });

  it("with no Downloads folder, asks for a folder before pairing", async () => {
    replies.quicksend_folder = () => {
      // eslint-disable-next-line @typescript-eslint/only-throw-error
      throw { kind: "failed" };
    };
    replies.quicksend_choose_folder = () => ({ name: "Work", path: "D:/Work", freeBytes: null });
    inApp(<QuickHost mode="receive" onHome={vi.fn()} />);
    await waitFor(() => {
      expect(heading()).toBe(text("qs-folder-title"));
    });
    expect(called("pairing_new_search")).toBe(false);
    fireEvent.click(screen.getByRole("button", { name: text("qs-folder-choose") }));
    await waitFor(() => {
      expect(tauri.invoke).toHaveBeenCalledWith("pairing_new_search");
    });
    await waitFor(() => {
      expect(page()).toContain(plain(text("qs-folder-line", { folder: "Work" })));
    });
  });

  it("no room in the folder: says so with the folder and the sizes, and that nothing was saved", async () => {
    replies.quicksend_folder = () => DOWNLOADS;
    inApp(<QuickHost mode="receive" onHome={vi.fn()} />);
    await waitFor(() => {
      expect(tauri.invoke).toHaveBeenCalledWith("pairing_new_search");
    });
    send("pairing-new", { kind: "paired" });
    await waitFor(() => {
      expect(called("quicksend_receive")).toBe(true);
    });
    send("quicksend-problem", { kind: "no-room", needed: 238_000_000, room: 120_000_000 });
    expect(heading()).toBe(plain(text("qs-problem-room-title", { folder: "Downloads" })));
    expect(page()).toContain(plain(text("qs-problem-room-body", { needed: "238 MB", room: "120 MB" })));
  });

  it("a lost connection says so", async () => {
    replies.quicksend_folder = () => DOWNLOADS;
    inApp(<QuickHost mode="receive" onHome={vi.fn()} />);
    await waitFor(() => {
      expect(tauri.invoke).toHaveBeenCalledWith("pairing_new_search");
    });
    send("pairing-new", { kind: "paired" });
    await waitFor(() => {
      expect(called("quicksend_receive")).toBe(true);
    });
    send("quicksend-problem", { kind: "link-lost", detail: "the connection dropped" });
    expect(heading()).toBe(text("qs-problem-lost-title"));
  });

  it("a word from the engine that does not parse ends on the problem page", async () => {
    replies.quicksend_folder = () => DOWNLOADS;
    inApp(<QuickHost mode="receive" onHome={vi.fn()} />);
    await waitFor(() => {
      expect(tauri.invoke).toHaveBeenCalledWith("pairing_new_search");
    });
    send("pairing-new", { kind: "paired" });
    await waitFor(() => {
      expect(called("quicksend_receive")).toBe(true);
    });
    send("quicksend-progress", { phase: 1 });
    expect(heading()).toBe(text("qs-problem-failed-title"));
  });
});
