/**
 * Rescue a laptop (2.6; blueprint T01 to T03): the drives connected to this laptop, one to choose,
 * an empty state with Check again, and help for a locked drive (the system asks for the key in
 * its own window; PCTwin never does).
 */
import { LocalizationProvider } from "@fluent/react";
import { act, cleanup, fireEvent, render, screen, within } from "@testing-library/react";
import type { ReactNode } from "react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import drives from "../contract/rescue-drives.json";
import { parseRescueDrive, type RescueDrive } from "../bridge";
import { localization } from "../i18n";
import { violations } from "../testing/a11y";
import { FooterProvider } from "./FrameFooter";
import RescuePick from "./RescuePick";

const l10n = localization("en");
const text = (id: string, vars?: Record<string, string | number>) => l10n.getString(id, vars);
const plain = (s: string) => s.replace(/[\u2066-\u2069]/g, "");
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

const sample: RescueDrive[] = drives[0].map((d) => {
  const p = parseRescueDrive(d);
  if (!p) throw new Error("the engine's example parses");
  return p;
});

async function show(load: () => Promise<RescueDrive[]>, onChoose = vi.fn(), onBack = vi.fn()) {
  const ui: ReactNode = <RescuePick load={load} onChoose={onChoose} onBack={onBack} />;
  render(
    <LocalizationProvider l10n={l10n}>
      <FooterProvider>
        <main>{ui}</main>
      </FooterProvider>
    </LocalizationProvider>,
  );
  await act(async () => {
    await Promise.resolve();
  });
  return { onChoose, onBack };
}

describe("Rescue a laptop", () => {
  it("says what it does in one line, lists each drive with its size and file system, and marks a laptop's drive", async () => {
    const { onChoose } = await show(() => Promise.resolve(sample));
    const heading = screen.getByRole("heading", { level: 1 });
    expect(heading.textContent).toBe(text("rescue-title"));
    expect(document.activeElement).toBe(heading);
    expect(screen.getByText(text("rescue-lead"))).toBeTruthy();
    const rows = screen.getAllByRole("listitem");
    expect(rows).toHaveLength(2);
    expect(within(rows[0]).getByText("Old laptop drive (E:\\)")).toBeTruthy();
    expect(plain(rows[0].textContent)).toContain("NTFS");
    expect(within(rows[0]).getByText(text("rescue-looks-like-laptop"))).toBeTruthy();
    expect(within(rows[1]).queryByText(text("rescue-looks-like-laptop"))).toBeNull();
    // Each button names its drive, so a screen reader hears which one.
    fireEvent.click(within(rows[1]).getByRole("button", { name: text("rescue-choose", { name: "Photos stick (F:\\)" }) }));
    expect(onChoose).toHaveBeenCalledWith(sample[1]);
    expect(await violations()).toEqual([]);
  });

  it("with no drive: says how to connect one, and checks again", async () => {
    const load = vi.fn(() => Promise.resolve<RescueDrive[]>([]));
    await show(load);
    expect(screen.getByText(text("rescue-empty"))).toBeTruthy();
    expect(screen.queryAllByRole("listitem")).toHaveLength(0);
    load.mockResolvedValueOnce(sample);
    await act(async () => {
      fireEvent.click(screen.getByRole("button", { name: text("rescue-check-again") }));
      await Promise.resolve();
    });
    expect(load).toHaveBeenCalledTimes(2);
    expect(screen.getAllByRole("listitem")).toHaveLength(2);
  });

  it("when the drives cannot be listed, says so and offers Check again; never an empty list as if none were there", async () => {
    await show(() => Promise.reject(new Error("no")));
    expect(screen.getByText(text("rescue-failed"))).toBeTruthy();
    expect(screen.queryByText(text("rescue-empty"))).toBeNull();
    expect(screen.getByRole("button", { name: text("rescue-check-again") })).toBeTruthy();
  });

  it("explains a locked drive: the system asks for the key in its own window, where the key can be found, and that PCTwin never asks", async () => {
    await show(() => Promise.resolve(sample));
    const help = screen.getByText(text("rescue-locked-title")).closest("details");
    if (!help) throw new Error("locked-drive help");
    expect(plain(help.textContent)).toContain(plain(text("rescue-locked-windows")));
    expect(plain(help.textContent)).toContain(plain(text("guard-rescue-never-key")));
  });

  it("goes back", async () => {
    const { onBack } = await show(() => Promise.resolve(sample));
    fireEvent.click(screen.getByRole("button", { name: text("frame-back") }));
    expect(onBack).toHaveBeenCalledOnce();
  });

  it("is accessible right to left, also when empty", async () => {
    document.documentElement.dir = "rtl";
    await show(() => Promise.resolve(sample));
    expect(await violations()).toEqual([]);
    cleanup();
    await show(() => Promise.resolve([]));
    expect(await violations()).toEqual([]);
  });
});
