/** Finish your move is reached from the new laptop's outcome and from each finished move in My PCTwin history. */
import { LocalizationProvider } from "@fluent/react";
import { cleanup, render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, describe, expect, it, vi } from "vitest";
import { localization } from "../i18n";
import { finishedMixed } from "../moveSamples";
import { violations } from "../testing/a11y";
import History from "./History";
import Outcome from "./Outcome";
import { FooterProvider } from "./FrameFooter";

const l10n = localization("en");
const text = (id: string, vars?: Record<string, string>) => l10n.getString(id, vars);

afterEach(cleanup);

const inApp = (ui: React.ReactNode) =>
  render(
    <LocalizationProvider l10n={l10n}>
      <FooterProvider>
        <main>{ui}</main>
      </FooterProvider>
    </LocalizationProvider>,
  );

describe("Finish your move, from the outcome", () => {
  it("is offered on the new laptop and not on the old one", async () => {
    const onFinish = vi.fn();
    const user = userEvent.setup();
    inApp(<Outcome outcome={finishedMixed} onReport={vi.fn()} onFinish={onFinish} />);
    await user.click(screen.getByRole("button", { name: text("finish-title") }));
    expect(onFinish).toHaveBeenCalledTimes(1);
    expect(await violations({ region: { enabled: false } })).toEqual([]);
    cleanup();
    inApp(<Outcome outcome={finishedMixed} onReport={vi.fn()} old />);
    expect(screen.queryByRole("button", { name: text("finish-title") })).toBeNull();
  });
});

describe("Finish your move, from My PCTwin history", () => {
  it("is on each finished move, by its own history ID, and not on a paused one", async () => {
    const onFinish = vi.fn();
    const user = userEvent.setup();
    const base = { kind: "move", when: "2026-10-28T20:22:00Z", from: "Amun", to: "Ada" } as const;
    inApp(
      <History
        entries={[
          { ...base, id: "1", state: "finished" },
          { ...base, id: "2", when: "2026-10-27T20:22:00Z", state: "paused" },
        ]}
        onOpen={vi.fn()}
        onContinue={vi.fn()}
        onFinish={onFinish}
        onExport={vi.fn()}
        onStart={vi.fn()}
        onBack={vi.fn()}
      />,
    );
    const buttons = screen.getAllByRole("button", { name: new RegExp(`^${text("history-open-name", { what: text("finish-title"), title: "" }).trim()}`) });
    expect(buttons).toHaveLength(1);
    await user.click(buttons[0]);
    expect(onFinish).toHaveBeenCalledWith("1");
  });
});
