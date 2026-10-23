import { LocalizationProvider } from "@fluent/react";
import type { ReactNode } from "react";
import { cleanup, fireEvent, render, screen, within } from "@testing-library/react";
import { violations } from "../testing/a11y";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { localization } from "../i18n";
import type { Account, BeforeItem, HelpCode, Person } from "../people";
import BeforeYouStart from "./BeforeYouStart";
import { FooterProvider } from "./FrameFooter";
import GetHelp from "./GetHelp";
import WhoIsMoving from "./WhoIsMoving";

// The accessibility check is slow on a busy machine (several suites run at once), so these tests get more time.
vi.setConfig({ testTimeout: 20000 });

const l10n = localization("en");
const text = (id: string, vars?: Record<string, string | number>) => l10n.getString(id, vars);
/** Fluent isolates variables for right-to-left safety; the marks are invisible, so tests compare without them. */
const plain = (s: string) => s.replace(/[⁦-⁩]/g, "");
const nameIs = (expected: string) => (accessible: string) => plain(accessible) === plain(expected);
const withText = (expected: string) => (content: string) => plain(content) === plain(expected);

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
  Reflect.deleteProperty(navigator, "clipboard");
});

function inApp(ui: ReactNode) {
  return render(
    <LocalizationProvider l10n={l10n}>
      <main>{ui}</main>
    </LocalizationProvider>,
  );
}

/** A message that is only attributes (an accessible name): its text with the variables filled in. */
function attr(id: string, vars?: Record<string, string | number>, name = "aria-label"): string {
  const bundle = l10n.getBundle(id);
  const pattern = bundle?.getMessage(id)?.attributes[name];
  if (!bundle || !pattern) throw new Error(`no ${name} on ${id}`);
  return bundle.formatPattern(pattern, vars);
}
const pairBack = () => screen.getByRole("button", { name: nameIs(text("pair-back")) });

const heading = () => screen.getByRole("heading", { level: 1 });
const button = (id: string, vars?: Record<string, string | number>) => screen.getByRole("button", { name: nameIs(text(id, vars)) });

function noMissingText() {
  expect(warnings.filter((w) => /missing|not found|did not match/i.test(w))).toEqual([]);
}

/** Words that would mean something paid from PCTwin is on the screen (other companies' paid apps are named on purpose in Before you start). */
const PAID = /price|pricing|upgrade|premium|subscri|trial|cloud twin|quota|[$£€]|\bpro\b/i;
function noPaidWords() {
  expect(document.body.textContent).not.toMatch(PAID);
}

/* ---------- M06 Who is moving? ---------- */

const accounts: Account[] = [
  { id: "a1", name: "Ada" },
  { id: "a2", name: "Studio" },
];

const ada: Person = { id: "p1", name: "Ada", isYou: true, state: "ready", destination: { kind: "undecided" }, suggestion: "a1" };
const maya: Person = { id: "p2", name: "Maya", isYou: false, state: "waiting-for-owner", destination: { kind: "undecided" } };

function renderPeople(over: Partial<Parameters<typeof WhoIsMoving>[0]> = {}) {
  const props = {
    people: [ada],
    accounts,
    shared: "shared" as const,
    onBack: vi.fn(),
    onChoose: vi.fn(),
    onChooseShared: vi.fn(),
    onContinue: vi.fn(),
    ...over,
  };
  inApp(<WhoIsMoving {...props} />);
  return props;
}

const personSelect = (name: string) => screen.getByRole<HTMLSelectElement>("combobox", { name: nameIs(attr("people-select", { name })) });
const continueButton = () => screen.getByRole("button", { name: nameIs(text("people-continue")) });

describe("Who is moving", () => {
  it("shows just you, with the heading focused and nothing assumed from a matching name", async () => {
    renderPeople();
    expect(heading().textContent).toBe(text("people-title"));
    expect(document.activeElement).toBe(heading());
    // A matching account is only a suggestion; the choice is still not made.
    expect(personSelect("Ada").value).toBe("undecided");
    expect(screen.getByText(withText(text("people-suggested")))).toBeTruthy();
    expect(screen.getByText(withText(text("people-status-choose")))).toBeTruthy();
    expect(screen.queryByText(withText(text("people-status-ready")))).toBeNull();
    expect(screen.getByText(withText(text("people-just-me")))).toBeTruthy();
    expect(continueButton().hasAttribute("disabled")).toBe(true);
    expect(screen.getByText(withText(text("people-todo", { count: 1 })))).toBeTruthy();
    expect(screen.getByText(withText(text("people-note")))).toBeTruthy();
    noMissingText();
    noPaidWords();
    expect(await violations()).toEqual([]);
  });

  it("Use the suggestion sends the choice; the screen does not choose by itself", () => {
    const props = renderPeople();
    expect(props.onChoose).not.toHaveBeenCalled();
    fireEvent.click(button("people-suggested-use", { account: "Ada" }));
    expect(props.onChoose).toHaveBeenCalledWith("p1", { kind: "existing", accountId: "a1" });
  });

  it("offers every existing account, a new account and leaving out, and sends the choice", () => {
    const props = renderPeople();
    const select = personSelect("Ada");
    expect([...select.options].map((o) => plain(o.textContent))).toEqual([
      plain(text("people-option-choose")),
      plain(text("people-option-existing", { account: "Ada" })),
      plain(text("people-option-existing", { account: "Studio" })),
      plain(text("people-option-new")),
      plain(text("people-option-leave-out")),
    ]);
    fireEvent.change(select, { target: { value: "existing:a2" } });
    expect(props.onChoose).toHaveBeenLastCalledWith("p1", { kind: "existing", accountId: "a2" });
    fireEvent.change(select, { target: { value: "new" } });
    expect(props.onChoose).toHaveBeenLastCalledWith("p1", { kind: "new" });
    fireEvent.change(select, { target: { value: "leave-out" } });
    expect(props.onChoose).toHaveBeenLastCalledWith("p1", { kind: "leave-out" });
  });

  it("a new account says the new laptop asks permission first", () => {
    renderPeople({ people: [{ ...ada, destination: { kind: "new" } }] });
    expect(screen.getByText(withText(text("people-new-account-hint")))).toBeTruthy();
    expect(screen.getByText(withText(text("people-status-ready")))).toBeTruthy();
  });

  it("an account that no longer exists is shown as not chosen, never as a stale choice", () => {
    renderPeople({ people: [{ ...ada, destination: { kind: "existing", accountId: "gone" } }] });
    expect(personSelect("Ada").value).toBe("undecided");
    expect(continueButton().hasAttribute("disabled")).toBe(true);
    expect(screen.queryByText(withText(text("people-status-ready")))).toBeNull();
  });

  it("continues only once every person who can be mapped has a choice", () => {
    const props = renderPeople({ people: [{ ...ada, destination: { kind: "new" } }] });
    expect(screen.getByText(withText(text("people-todo", { count: 0 })))).toBeTruthy();
    fireEvent.click(continueButton());
    expect(props.onContinue).toHaveBeenCalledTimes(1);
  });

  it("leaving out is a choice, so it does not block continuing, and it is said in words", () => {
    renderPeople({ people: [{ ...ada, destination: { kind: "leave-out" } }] });
    expect(screen.getByText(withText(text("people-status-left-out")))).toBeTruthy();
    expect(continueButton().hasAttribute("disabled")).toBe(false);
  });

  it("lists your own profile first, then others; one waiting for its owner cannot be mapped", async () => {
    const props = renderPeople({ people: [maya, { ...ada, destination: { kind: "new" } }] });
    const rows = screen.getAllByRole("row").filter((r) => r.classList.contains("people-person"));
    expect(rows).toHaveLength(2);
    expect(within(rows[0]).getByText(withText(text("people-you")))).toBeTruthy();
    // The waiting profile says so in words, has no way to choose, and does not hold up the rest.
    expect(within(rows[1]).getByText(withText(text("people-status-waiting")))).toBeTruthy();
    expect(within(rows[1]).getByText(withText(text("people-waiting-body", { name: "Maya" })))).toBeTruthy();
    expect(within(rows[1]).queryByRole("combobox")).toBeNull();
    expect(screen.queryByText(withText(text("people-just-me")))).toBeNull();
    expect(screen.getByText(withText(text("people-waiting-count", { count: 1 })))).toBeTruthy();
    expect(continueButton().hasAttribute("disabled")).toBe(false);
    fireEvent.click(continueButton());
    expect(props.onContinue).toHaveBeenCalledTimes(1);
    noMissingText();
    noPaidWords();
    expect(await violations()).toEqual([]);
  });

  it("a waiting profile is never shown as ready, even if the bridge sent a choice for it", () => {
    renderPeople({ people: [ada, { ...maya, destination: { kind: "new" } }] });
    expect(screen.queryByRole("combobox", { name: nameIs(attr("people-select", { name: "Maya" })) })).toBeNull();
    expect(screen.queryByText(withText(text("people-status-ready")))).toBeNull();
  });

  it("a mapping the person changed says so in words", () => {
    renderPeople({ people: [{ ...ada, destination: { kind: "existing", accountId: "a2" }, changedByYou: true }] });
    expect(screen.getByText(withText(text("people-status-changed")))).toBeTruthy();
    expect(screen.getByText(withText(text("people-changed-note")))).toBeTruthy();
    expect(screen.getByText(withText(text("people-status-ready")))).toBeTruthy();
    // A chosen person gets no suggestion line.
    expect(screen.queryByText(withText(text("people-suggested")))).toBeNull();
  });

  it("shared items have their own choice, and 'with the files of' names you", () => {
    const props = renderPeople();
    const select = screen.getByRole<HTMLSelectElement>("combobox", { name: nameIs(attr("people-shared-select")) });
    expect(select.value).toBe("shared");
    expect([...select.options].map((o) => [o.value, plain(o.textContent)])).toEqual([
      ["shared", plain(text("people-shared-option-shared"))],
      ["person", plain(text("people-shared-option-person", { name: "Ada" }))],
      ["leave-out", plain(text("people-shared-option-leave-out"))],
    ]);
    fireEvent.change(select, { target: { value: "leave-out" } });
    expect(props.onChooseShared).toHaveBeenCalledWith("leave-out");
  });

  it("Back goes back", () => {
    const props = renderPeople();
    fireEvent.click(pairBack());
    expect(props.onBack).toHaveBeenCalledTimes(1);
  });

  it("keeps names in their own direction", () => {
    renderPeople({ people: [{ ...ada, name: "مريم" }] });
    const name = screen.getAllByText("مريم")[0];
    expect(name.closest("bdi, [dir='auto']")).not.toBeNull();
  });
});

/* ---------- Before you start ---------- */

const items: BeforeItem[] = [
  { id: "i1", kind: "licences", done: false },
  { id: "i2", kind: "passwords", done: true },
  { id: "i3", kind: "codes", done: false },
  { id: "i4", kind: "online", done: false },
];

function renderBefore(over: Partial<Parameters<typeof BeforeYouStart>[0]> = {}) {
  const props = { items, onBack: vi.fn(), onToggle: vi.fn(), onContinue: vi.fn(), ...over };
  inApp(<BeforeYouStart {...props} />);
  return props;
}

const tick = (kind: string) => screen.getByRole<HTMLInputElement>("checkbox", { name: nameIs(attr("before-done", { item: text(`before-${kind}-title`) })) });

describe("Before you start", () => {
  it("lists what to do on the old laptop, each as the person's own tick, and says PCTwin cannot see it", async () => {
    renderBefore();
    expect(heading().textContent).toBe(text("before-title"));
    expect(document.activeElement).toBe(heading());
    expect(screen.getAllByRole("checkbox")).toHaveLength(4);
    for (const kind of ["licences", "passwords", "codes", "online"]) {
      expect(screen.getByText(withText(text(`before-${kind}-title`)))).toBeTruthy();
      expect(screen.getByText(withText(text(`before-${kind}-body`)))).toBeTruthy();
    }
    expect(screen.getByText(withText(text("before-note")))).toBeTruthy();
    noMissingText();
    noPaidWords();
    expect(await violations()).toEqual([]);
  });

  it("reflects what is ticked and sends each tick", () => {
    const props = renderBefore();
    expect(tick("licences").checked).toBe(false);
    expect(tick("passwords").checked).toBe(true);
    fireEvent.click(tick("licences"));
    expect(props.onToggle).toHaveBeenCalledWith("i1", true);
    fireEvent.click(tick("passwords"));
    expect(props.onToggle).toHaveBeenCalledWith("i2", false);
  });

  it("is optional: Continue works with nothing ticked", () => {
    const props = renderBefore({ items: items.map((i) => ({ ...i, done: false })) });
    const go = button("before-continue");
    expect(go.hasAttribute("disabled")).toBe(false);
    fireEvent.click(go);
    expect(props.onContinue).toHaveBeenCalledTimes(1);
  });

  it("with nothing to do, says so and still continues", async () => {
    const props = renderBefore({ items: [] });
    expect(screen.getByText(withText(text("before-empty")))).toBeTruthy();
    expect(screen.queryAllByRole("checkbox")).toHaveLength(0);
    fireEvent.click(button("before-continue"));
    expect(props.onContinue).toHaveBeenCalledTimes(1);
    noMissingText();
    expect(await violations()).toEqual([]);
  });

  it("Back goes back", () => {
    const props = renderBefore();
    fireEvent.click(pairBack());
    expect(props.onBack).toHaveBeenCalledTimes(1);
  });
});

/* ---------- Get help ---------- */

const ready: HelpCode = { status: "ready", code: "PCT-7F3K-21", included: ["problem", "step", "versions"] };

function renderHelp(help: HelpCode = ready, over: Partial<Parameters<typeof GetHelp>[0]> = {}) {
  const props = { help, canShare: true, onBack: vi.fn(), onShare: vi.fn(), onRetry: vi.fn(), ...over };
  const view = inApp(<GetHelp {...props} />);
  return { props, view };
}

function rerenderHelp(view: ReturnType<typeof render>, help: HelpCode) {
  view.rerender(
    <LocalizationProvider l10n={l10n}>
      <main>
        <GetHelp help={help} canShare onBack={vi.fn()} onShare={vi.fn()} onRetry={vi.fn()} />
      </main>
    </LocalizationProvider>,
  );
}

function stubClipboard(write: (s: string) => Promise<void>) {
  const writeText = vi.fn(write);
  Object.defineProperty(navigator, "clipboard", { value: { writeText }, configurable: true });
  return writeText;
}

describe("Get help", () => {
  it("shows the code, what it contains in plain words, and what it never contains", async () => {
    renderHelp();
    expect(heading().textContent).toBe(text("help-title"));
    expect(document.activeElement).toBe(heading());
    const code = screen.getByRole("group", { name: nameIs(attr("help-code", { code: "PCT-7F3K-21" })) });
    expect(code.textContent).toBe("PCT-7F3K-21");
    expect(code.getAttribute("dir")).toBe("ltr");
    expect(code.getAttribute("translate")).toBe("no");
    for (const kind of ["problem", "step", "versions"]) expect(screen.getByText(withText(text(`help-contains-${kind}`)))).toBeTruthy();
    for (const never of ["names", "files", "network"]) expect(screen.getByText(withText(text(`help-never-${never}`)))).toBeTruthy();
    expect(screen.getByText(withText(text("help-note")))).toBeTruthy();
    noMissingText();
    noPaidWords();
    expect(await violations()).toEqual([]);
  });

  it("holds only what the engine said it includes (nothing extra is claimed)", () => {
    renderHelp({ status: "ready", code: "PCT-7F3K-21", included: ["problem"] });
    expect(screen.getByText(withText(text("help-contains-problem")))).toBeTruthy();
    expect(screen.queryByText(withText(text("help-contains-step")))).toBeNull();
    expect(screen.queryByText(withText(text("help-contains-versions")))).toBeNull();
  });

  it("while the code is being made, says so and offers no Copy or Share", async () => {
    renderHelp({ status: "making" });
    expect(screen.getByText(withText(text("help-making")))).toBeTruthy();
    expect(screen.queryByRole("button", { name: nameIs(text("help-copy")) })).toBeNull();
    expect(screen.queryByRole("button", { name: nameIs(text("help-share")) })).toBeNull();
    noMissingText();
    expect(await violations()).toEqual([]);
  });

  it("Copy puts the code on the clipboard, then says it was copied without moving focus", async () => {
    const writeText = stubClipboard(() => Promise.resolve());
    renderHelp();
    const copy = button("help-copy");
    copy.focus();
    fireEvent.click(copy);
    expect(writeText).toHaveBeenCalledWith("PCT-7F3K-21");
    const said = await screen.findByText(withText(text("help-copied")));
    expect(said.closest("[role='status']")).not.toBeNull();
    expect(document.activeElement).toBe(copy);
    expect(await violations()).toEqual([]);
  });

  it("does not say copied when the system refuses, and says how to copy by hand", async () => {
    stubClipboard(() => Promise.reject(new Error("denied")));
    renderHelp();
    fireEvent.click(button("help-copy"));
    expect(await screen.findByText(withText(text("help-copy-failed")))).toBeTruthy();
    expect(screen.queryByText(withText(text("help-copied")))).toBeNull();
  });

  it("does not break, or claim a copy, when there is no clipboard at all", async () => {
    // No clipboard defined at all.
    renderHelp();
    fireEvent.click(button("help-copy"));
    expect(await screen.findByText(withText(text("help-copy-failed")))).toBeTruthy();
    expect(screen.queryByText(withText(text("help-copied")))).toBeNull();
  });

  it("a new code starts uncopied", async () => {
    stubClipboard(() => Promise.resolve());
    const { view } = renderHelp();
    fireEvent.click(button("help-copy"));
    expect(await screen.findByText(withText(text("help-copied")))).toBeTruthy();
    rerenderHelp(view, { status: "ready", code: "PCT-9X2M-04", included: ["step"] });
    expect(screen.queryByText(withText(text("help-copied")))).toBeNull();
  });

  it("Share hands the code and its line to the bridge, and claims nothing about what the person then does", () => {
    const { props } = renderHelp();
    fireEvent.click(button("help-share"));
    expect(props.onShare).toHaveBeenCalledWith("PCT-7F3K-21", text("help-share-text", { code: "PCT-7F3K-21" }));
    expect(screen.queryByText(/\b(sent|shared with)\b/i)).toBeNull();
    expect(screen.getByText(withText(text("help-share-hint")))).toBeTruthy();
  });

  it("without a share sheet, Share is not offered and Copy still is", () => {
    renderHelp(ready, { canShare: false });
    expect(screen.queryByRole("button", { name: nameIs(text("help-share")) })).toBeNull();
    expect(screen.queryByText(withText(text("help-share-hint")))).toBeNull();
    expect(button("help-copy")).toBeTruthy();
  });

  it("could not make a code: says what happened and what to do, with a retry, and shows no code", async () => {
    const { props } = renderHelp({ status: "failed" });
    expect(screen.getByRole("alert").textContent).toContain(plain(text("help-failed")));
    expect(screen.queryByRole("button", { name: nameIs(text("help-copy")) })).toBeNull();
    expect(screen.queryByRole("button", { name: nameIs(text("help-share")) })).toBeNull();
    expect(screen.queryByText(/PCT-/)).toBeNull();
    fireEvent.click(button("help-try-again"));
    expect(props.onRetry).toHaveBeenCalledTimes(1);
    noMissingText();
    expect(await violations()).toEqual([]);
  });

  it("a change of state moves focus to the new heading", () => {
    const { view } = renderHelp({ status: "making" });
    expect(document.activeElement).toBe(heading());
    heading().blur();
    rerenderHelp(view, ready);
    expect(document.activeElement).toBe(heading());
    heading().blur();
    rerenderHelp(view, { status: "failed" });
    expect(document.activeElement).toBe(heading());
  });

  it("Back goes back", () => {
    const { props } = renderHelp();
    fireEvent.click(pairBack());
    expect(props.onBack).toHaveBeenCalledTimes(1);
  });
});

describe("the primary action sits in the window footer", () => {
  function inFooter(ui: ReactNode) {
    return render(
      <LocalizationProvider l10n={l10n}>
        <FooterProvider>
          <main>{ui}</main>
        </FooterProvider>
      </LocalizationProvider>,
    );
  }
  const noop = () => undefined;

  it("Who is moving: Continue", () => {
    inFooter(<WhoIsMoving people={[{ ...ada, destination: { kind: "new" } }]} accounts={accounts} shared="shared" onBack={noop} onChoose={noop} onChooseShared={noop} onContinue={noop} />);
    const footer = document.querySelector("footer") as HTMLElement;
    expect(within(footer).getByRole("button", { name: nameIs(text("people-continue")) })).toBeTruthy();
    expect(within(screen.getByRole("main")).queryByRole("button", { name: nameIs(text("people-continue")) })).toBeNull();
  });

  it("Before you start: Continue", () => {
    inFooter(<BeforeYouStart items={items} onBack={noop} onToggle={noop} onContinue={noop} />);
    const footer = document.querySelector("footer") as HTMLElement;
    expect(within(footer).getByRole("button", { name: nameIs(text("before-continue")) })).toBeTruthy();
    expect(within(screen.getByRole("main")).queryByRole("button", { name: nameIs(text("before-continue")) })).toBeNull();
  });

  it("Get help: Copy code (and Try again when no code could be made); Share stays in the content", () => {
    const view = inFooter(<GetHelp help={ready} onBack={noop} onShare={noop} onRetry={noop} />);
    const footer = document.querySelector("footer") as HTMLElement;
    expect(within(footer).getByRole("button", { name: nameIs(text("help-copy")) })).toBeTruthy();
    expect(within(screen.getByRole("main")).getByRole("button", { name: nameIs(text("help-share")) })).toBeTruthy();
    view.unmount();
    inFooter(<GetHelp help={{ status: "failed" }} onBack={noop} onShare={noop} onRetry={noop} />);
    expect(within(document.querySelector("footer") as HTMLElement).getByRole("button", { name: nameIs(text("help-try-again")) })).toBeTruthy();
  });
});
