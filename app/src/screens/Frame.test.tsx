import { LocalizationProvider } from "@fluent/react";
import { useState, type ReactNode } from "react";
import { cleanup, fireEvent, render, screen, within } from "@testing-library/react";
import { violations } from "../testing/a11y";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { localization } from "../i18n";
import FrameFooter, { FooterProvider } from "./FrameFooter";
import { Steps, TopStrip } from "./FrameParts";

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
  cleanup();
  vi.restoreAllMocks();
});

const wrap = (ui: ReactNode, framed: boolean) => (
  <LocalizationProvider l10n={l10n}>{framed ? <FooterProvider><main>{ui}</main></FooterProvider> : <main>{ui}</main>}</LocalizationProvider>
);
const alone = (ui: ReactNode) => render(wrap(ui, false));
const framed = (ui: ReactNode) => render(wrap(ui, true));

describe("FrameFooter", () => {
  it("alone, renders its actions inline at the end of the content, so the buttons are still there", () => {
    alone(<FrameFooter><button type="button">Go</button></FrameFooter>);
    expect(within(screen.getByRole("main")).getByRole("button", { name: "Go" })).toBeTruthy();
    expect(screen.queryByRole("contentinfo")).toBeNull();
  });

  it("in a window, draws the actions in the footer with the short note, not in the content", () => {
    framed(<FrameFooter><button type="button">Go</button></FrameFooter>);
    const footer = screen.getByRole("contentinfo");
    expect(within(footer).getByText(text("footer-note-move"))).toBeTruthy();
    expect(within(footer).getByRole("button", { name: "Go" })).toBeTruthy();
    expect(within(screen.getByRole("main")).queryByRole("button", { name: "Go" })).toBeNull();
    expect(within(footer).queryByText(text("footer-free"))).toBeNull();
    expect(within(footer).queryByRole("button", { name: text("footer-help") })).toBeNull();
    expect(warnings.filter((w) => /missing|not found/i.test(w))).toEqual([]);
  });

  it("says nearby sending for a send, and shows no note at all for none", () => {
    const { rerender } = framed(<FrameFooter note="send"><button type="button">Go</button></FrameFooter>);
    expect(within(screen.getByRole("contentinfo")).getByText(text("footer-note-send"))).toBeTruthy();
    rerender(wrap(<FrameFooter note="none"><button type="button">Pause</button><button type="button">Compact</button></FrameFooter>, true));
    const footer = screen.getByRole("contentinfo");
    expect(within(footer).queryByText(text("footer-note-send"))).toBeNull();
    expect(within(footer).queryByText(text("footer-note-move"))).toBeNull();
    expect(within(footer).getAllByRole("button")).toHaveLength(2);
  });

  it("without a FrameFooter the footer is the usual one: the free line and Help", () => {
    framed(<p>content</p>);
    const footer = screen.getByRole("contentinfo");
    expect(within(footer).getByText(text("footer-free"))).toBeTruthy();
    expect(within(footer).getByRole("button", { name: text("footer-help") })).toBeTruthy();
  });

  it("goes back to the usual footer when the screen leaves", () => {
    function Host() {
      const [on, setOn] = useState(true);
      return (
        <>
          <button type="button" onClick={() => { setOn(false); }}>leave</button>
          {on && <FrameFooter><button type="button">Go</button></FrameFooter>}
        </>
      );
    }
    framed(<Host />);
    expect(within(screen.getByRole("contentinfo")).queryByText(text("footer-free"))).toBeNull();
    fireEvent.click(screen.getByRole("button", { name: "leave" }));
    expect(within(screen.getByRole("contentinfo")).getByText(text("footer-free"))).toBeTruthy();
    expect(screen.queryByRole("button", { name: "Go" })).toBeNull();
  });

  it("the action comes after the content in tab order, and the page has no accessibility violations", async () => {
    framed(
      <>
        <button type="button">first</button>
        <FrameFooter><button type="button">Go</button></FrameFooter>
      </>,
    );
    const first = screen.getByRole("button", { name: "first" });
    const go = screen.getByRole("button", { name: "Go" });
    expect(first.compareDocumentPosition(go) & Node.DOCUMENT_POSITION_FOLLOWING).toBeTruthy();
    expect(await violations()).toEqual([]);
  });
});

describe("TopStrip", () => {
  it("shows Back on the start side and the label on the end side; Back calls out", () => {
    const onBack = vi.fn();
    alone(<TopStrip onBack={onBack} labelId="connect-eyebrow" />);
    fireEvent.click(screen.getByRole("button", { name: text("frame-back") }));
    expect(onBack).toHaveBeenCalledTimes(1);
    expect(screen.getByText(text("connect-eyebrow"))).toBeTruthy();
  });

  it("without onBack there is no Back, and a ready label can be passed as a node", () => {
    alone(<TopStrip label="Reviewed move" />);
    expect(screen.queryByRole("button")).toBeNull();
    expect(screen.getByText("Reviewed move")).toBeTruthy();
  });
});

describe("Steps", () => {
  const steps = [
    { state: "done", title: "One", body: "first" },
    { state: "current", title: "Two", body: "second" },
    { state: "todo", title: "Three" },
  ] as const;

  it("marks only the current step, and gives every step a hidden state word and its position", () => {
    alone(<Steps steps={steps} />);
    const items = screen.getAllByRole("listitem");
    expect(items.map((i) => i.getAttribute("aria-current"))).toEqual([null, "step", null]);
    expect(items[0].textContent).toContain(text("frame-step-done"));
    expect(items[1].textContent).toContain(text("frame-step-current"));
    expect(items[2].textContent).toContain(text("frame-step-todo"));
    expect(plain(items[1].textContent)).toContain(plain(text("frame-step-position", { number: 2, total: 3 })));
    expect(warnings.filter((w) => /missing|not found/i.test(w))).toEqual([]);
  });

  it("shows no visible state words: they are only for screen readers", () => {
    alone(<Steps steps={steps} />);
    for (const word of ["frame-step-done", "frame-step-current", "frame-step-todo"]) {
      expect(screen.getByText(text(word)).closest(".visually-hidden")).toBeTruthy();
    }
  });

  it("has no accessibility violations", async () => {
    alone(<Steps steps={steps} />);
    expect(await violations()).toEqual([]);
  });
});
