import { LocalizationProvider } from "@fluent/react";
import { act, cleanup, fireEvent, render, screen } from "@testing-library/react";
import axe from "axe-core";
import { useState, type ReactNode } from "react";
import { afterEach, describe, expect, it, vi } from "vitest";
import Gallery from "../gallery/Gallery";
import { localization } from "../i18n";
import { CodeDisplay, CodeInput, Dialog, ProgressBar, ResultChip, Segmented, Toast, Toggle, onlyDigits, type Result } from "./kit";

afterEach(() => {
  cleanup();
  document.documentElement.dir = "";
  vi.restoreAllMocks();
});

function withL10n(ui: ReactNode) {
  return render(<LocalizationProvider l10n={localization("en")}>{ui}</LocalizationProvider>);
}

/** axe-core on the rendered page. Colour contrast needs real layout, so it is checked in the built app instead. */
async function violations(): Promise<string[]> {
  const result = await axe.run(document.body, { rules: { "color-contrast": { enabled: false } } });
  return result.violations.map((v) => `${v.id}: ${v.nodes.map((n) => n.target.join(" ")).join(", ")}`);
}

describe("onlyDigits", () => {
  it("keeps 0-9 and drops everything else", () => {
    expect(onlyDigits("48 27-16")).toBe("482716");
    expect(onlyDigits("abc")).toBe("");
  });

  it("reads digits typed in other scripts", () => {
    expect(onlyDigits("٤٨٢٧١٦")).toBe("482716"); // Arabic-Indic
    expect(onlyDigits("۴۸۲۷۱۶")).toBe("482716"); // Persian
    expect(onlyDigits("४८२७१६")).toBe("482716"); // Devanagari
    expect(onlyDigits("４８２７１６")).toBe("482716"); // fullwidth
  });
});

describe("kit components", () => {
  it("the gallery shows every component with no accessibility problems", async () => {
    withL10n(<Gallery />);
    expect(screen.getByRole("heading", { level: 1 }).textContent).toBe("PCTwin component kit");
    expect(await violations()).toEqual([]);
  });

  it("the gallery stays accessible right to left", async () => {
    document.documentElement.dir = "rtl";
    withL10n(<Gallery />);
    expect(await violations()).toEqual([]);
  });

  it("every result chip shows an icon and a word, never colour alone", () => {
    const results: Result[] = ["copied", "reinstalled", "converted", "needs-action", "skipped-by-user", "failed"];
    const words = ["Copied", "Reinstalled", "Converted", "Needs action", "Skipped by you", "Failed"];
    withL10n(
      <>
        {results.map((r) => (
          <ResultChip key={r} result={r} />
        ))}
      </>,
    );
    const chips = document.querySelectorAll(".kit-chip");
    expect(chips).toHaveLength(results.length);
    chips.forEach((chip, i) => {
      expect(chip.querySelector("svg")).not.toBeNull();
      expect(chip.textContent).toBe(words[i]);
    });
  });

  it("the pairing code is read out as one labelled group", () => {
    withL10n(<CodeDisplay digits="482716" />);
    const group = screen.getByRole("group");
    expect(group.getAttribute("aria-label")).toMatch(/4 8 2 7 1 6$/);
  });

  it("the code field takes pasted digits from any script and stops at six", () => {
    function Harness() {
      const [v, setV] = useState("");
      return <CodeInput value={v} onChange={setV} label="Code" />;
    }
    withL10n(<Harness />);
    const field = screen.getByLabelText<HTMLInputElement>("Code");
    fireEvent.change(field, { target: { value: "٤٨٢ ٧١٦ 99" } });
    expect(field.value).toBe("482716");
  });

  it("a toggle is a switch that flips on click and says its state", () => {
    function Harness() {
      const [on, setOn] = useState(false);
      return <Toggle label="Keep awake" checked={on} onChange={setOn} />;
    }
    withL10n(<Harness />);
    const toggle = screen.getByRole("switch", { name: "Keep awake" });
    expect(toggle.getAttribute("aria-checked")).toBe("false");
    fireEvent.click(toggle);
    expect(toggle.getAttribute("aria-checked")).toBe("true");
  });

  it("segmented choices move with arrow keys, mirrored right to left", () => {
    function Harness() {
      const [v, setV] = useState<"a" | "b" | "c">("a");
      return (
        <Segmented
          label="Theme"
          value={v}
          onChange={setV}
          options={[
            { value: "a", label: "A" },
            { value: "b", label: "B" },
            { value: "c", label: "C" },
          ]}
        />
      );
    }
    withL10n(<Harness />);
    const checked = () => screen.getAllByRole("radio").find((r) => r.getAttribute("aria-checked") === "true")?.textContent;
    fireEvent.keyDown(screen.getByRole("radio", { name: "A" }), { key: "ArrowRight" });
    expect(checked()).toBe("B");
    expect(document.activeElement?.textContent).toBe("B");
    fireEvent.keyDown(screen.getByRole("radio", { name: "B" }), { key: "ArrowLeft" });
    expect(checked()).toBe("A");
    fireEvent.keyDown(screen.getByRole("radio", { name: "A" }), { key: "ArrowLeft" });
    expect(checked()).toBe("C"); // wraps around
    document.documentElement.dir = "rtl";
    fireEvent.keyDown(screen.getByRole("radio", { name: "C" }), { key: "ArrowLeft" });
    expect(checked()).toBe("A"); // left means forward in a right-to-left language
    expect(screen.getAllByRole("radio").filter((r) => r.tabIndex === 0)).toHaveLength(1);
  });

  it("progress reports measured percent, clamped to 0-100", () => {
    withL10n(
      <>
        <ProgressBar value={0.68} label="Move" />
        <ProgressBar value={1.7} label="Over" />
        <ProgressBar value={-1} label="Under" />
      </>,
    );
    expect(screen.getByRole("progressbar", { name: "Move" }).getAttribute("aria-valuenow")).toBe("68");
    expect(screen.getByRole("progressbar", { name: "Over" }).getAttribute("aria-valuenow")).toBe("100");
    expect(screen.getByRole("progressbar", { name: "Under" }).getAttribute("aria-valuenow")).toBe("0");
  });

  it("a dialog is named by its title, closes on Escape and returns focus", () => {
    function Harness() {
      const [open, setOpen] = useState(false);
      return (
        <>
          <button
            type="button"
            onClick={() => {
              setOpen(true);
            }}
          >
            Open
          </button>
          <Dialog
            open={open}
            title="Stop this move?"
            onClose={() => {
              setOpen(false);
            }}
          >
            Body
          </Dialog>
        </>
      );
    }
    withL10n(<Harness />);
    const opener = screen.getByRole("button", { name: "Open" });
    opener.focus();
    fireEvent.click(opener);
    const dialog = document.querySelector("dialog");
    expect(dialog?.hasAttribute("open")).toBe(true);
    expect(dialog?.getAttribute("aria-labelledby")).toBe(screen.getByRole("heading", { name: "Stop this move?" }).id);
    expect(dialog?.contains(document.activeElement)).toBe(true);
    act(() => {
      fireEvent.keyDown(dialog as HTMLDialogElement, { key: "Escape" });
    });
    expect(dialog?.hasAttribute("open")).toBe(false);
    expect(document.activeElement).toBe(opener);
  });

  it("a toast is announced politely, has no timer, and dismisses by button or Escape", () => {
    vi.useFakeTimers();
    const dismiss = vi.fn();
    withL10n(<Toast onDismiss={dismiss}>Move paused.</Toast>);
    const toast = screen.getByRole("status");
    expect(toast.getAttribute("aria-live")).toBe("polite");
    act(() => {
      vi.advanceTimersByTime(60_000);
    });
    expect(screen.getByRole("status")).toBe(toast);
    expect(dismiss).not.toHaveBeenCalled();
    fireEvent.click(screen.getByRole("button", { name: "Dismiss" }));
    fireEvent.keyDown(toast, { key: "Escape" });
    expect(dismiss).toHaveBeenCalledTimes(2);
    vi.useRealTimers();
  });
});
