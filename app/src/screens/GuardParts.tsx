/**
 * Pieces shared by the Phase 2 screens (Task List 2.8): the guard's context, the one button that
 * sends an intent, names shown as plain text, the state line, measured progress with bounded
 * announcements, and More details.
 *
 * The rules they keep:
 * - A button is drawn only when the engine allows its intent now, and a press sends one intent bound
 *   to the job, session and plan generation on screen. While it is on its way the button is busy and
 *   a second press sends nothing (a double-click is one intent). The engine checks it again anyway.
 * - Names from the laptops are text, isolated for direction (`<bdi>`), never markup or links.
 * - The state is always said in words with its own icon shape, never by colour alone.
 * - Progress comes only from measured work; with no total the bar is indeterminate and still, with
 *   what is known in words. A screen reader hears the quarter marks, not every byte.
 */
import { Localized, useLocalization } from "@fluent/react";
import { createContext, useContext, useEffect, useRef, useState, type ReactNode } from "react";
import { Button } from "../components/kit";
import { allows, shouldAnnounce, type DisplayState, type IntentKind, type IntentName, type Measured, type Projection, type Spoken } from "../guard";
import { Attention, Check, Clock, Info, Lock, Shield } from "../icons";
import { useFormat } from "./PlanParts";
import "./guard.css";

export type GuardApi = {
  view: Projection;
  /** Sends one intent bound to `view`; resolves when the engine answered. */
  send: (intent: IntentKind) => void;
  /** Whether this intent is on its way now. */
  busy: (intent: IntentKind) => boolean;
  /** The last refusal's word key, if the engine refused. */
  refusal: string | null;
};

const GuardContext = createContext<GuardApi | null>(null);

export function GuardProvider({ value, children }: { value: GuardApi; children: ReactNode }) {
  return <GuardContext.Provider value={value}>{children}</GuardContext.Provider>;
}

export function useGuard(): GuardApi {
  const api = useContext(GuardContext);
  if (!api) throw new Error("a Phase 2 screen is drawn outside the guard");
  return api;
}

/** A name from a laptop, as plain text, isolated so a right-to-left name cannot reorder the line. */
export function Name({ children }: { children: string }) {
  return <bdi className="g-name">{children}</bdi>;
}

/**
 * The one button that sends an intent. Not drawn when the engine does not allow the intent now.
 * `name` defaults to the intent's own kind.
 */
export function IntentButton({ intent, label, variant = "secondary", name }: { intent: IntentKind; label: string; variant?: "primary" | "secondary" | "quiet"; name?: IntentName }) {
  const { view, send, busy } = useGuard();
  if (!allows(view, name ?? intent.kind)) return null;
  const sending = busy(intent);
  return (
    <Button
      variant={variant}
      aria-disabled={sending || undefined}
      data-busy={sending || undefined}
      onClick={() => {
        if (!sending) send(intent);
      }}
    >
      {label}
    </Button>
  );
}

const STATE_ICON: Partial<Record<DisplayState, () => ReactNode>> = {
  complete: () => <Check />,
  failed: () => <Attention />,
  blocked: () => <Attention />,
  partial: () => <Attention />,
  "source-unavailable": () => <Attention />,
  "awaiting-person": () => <Info />,
  "awaiting-system": () => <Shield />,
  "awaiting-maker": () => <Clock />,
  "pause-requested": () => <Clock />,
  paused: () => <Clock />,
  reconciling: () => <Clock />,
  cancelled: () => <Lock />,
};

/** The job's state in words, with an icon of its own shape. A status region, read without moving focus. */
export function StateLine() {
  const { view } = useGuard();
  const icon = STATE_ICON[view.display];
  return (
    <p className="g-state" role="status" aria-atomic="true" data-state={view.display}>
      {icon ? icon() : <Info />}
      <Localized id={`guard-state-${view.display}`}>
        <span />
      </Localized>
    </p>
  );
}

/** What the engine refused, in words, announced when it appears. */
export function RefusalLine() {
  const { refusal } = useGuard();
  if (!refusal) return null;
  return (
    <p className="g-refusal" role="alert">
      <Attention />
      <Localized id={`guard-refused-${refusal}`}>
        <span />
      </Localized>
    </p>
  );
}

/**
 * Measured progress: a bar when the total is known, an indeterminate still bar and the measured
 * amount in words when it is not. The polite region beside it is in the page from the start and
 * speaks only at quarter marks, at most every 30 seconds.
 */
export function MeasuredLine({ progress }: { progress: Measured | null }) {
  const { l10n } = useLocalization();
  const { bytes, number } = useFormat();
  const [said, setSaid] = useState("");
  const last = useRef<Spoken>({ mark: 0, at: 0 });
  const amount = (n: number) => (progress?.unit === "items" ? number(n) : bytes(n));
  const known = progress !== null && progress.total !== null && progress.total > 0;
  const pct = known ? Math.min(100, Math.floor((progress.done / (progress.total ?? 1)) * 100)) : null;

  useEffect(() => {
    const next = shouldAnnounce(progress, last.current, Date.now());
    if (next.say) {
      last.current = { mark: next.mark, at: Date.now() };
      setSaid(l10n.getString("guard-progress-mark", { percent: next.mark }));
    }
  }, [progress, l10n]);

  return (
    <div className="g-progress">
      <div
        className={known ? "kit-progress" : "kit-progress g-indeterminate"}
        role="progressbar"
        aria-label={l10n.getString("guard-progress-label")}
        aria-valuemin={known ? 0 : undefined}
        aria-valuemax={known ? 100 : undefined}
        aria-valuenow={pct ?? undefined}
        aria-valuetext={known ? undefined : progress ? l10n.getString("guard-progress-unknown-total", { done: amount(progress.done) }) : l10n.getString("guard-progress-none")}
      >
        {pct !== null && <span className="kit-progress-fill" style={{ transform: `scaleX(${String(pct / 100)})` }} />}
      </div>
      <p className="kit-progress-meta">
        <span>
          {known
            ? l10n.getString("guard-progress-of", { done: amount(progress.done), total: amount(progress.total ?? 0) })
            : progress
              ? l10n.getString("guard-progress-unknown-total", { done: amount(progress.done) })
              : l10n.getString("guard-progress-none")}
        </span>
      </p>
      <p className="g-sr" role="status" aria-live="polite" aria-atomic="true">
        {said}
      </p>
    </div>
  );
}

/** Technical words, out of the main flow. */
export function MoreDetails({ children }: { children: ReactNode }) {
  return (
    <details className="mv-details">
      <Localized id="report-more-details">
        <summary />
      </Localized>
      <div className="g-details">{children}</div>
    </details>
  );
}

/** "and 12 more", when a list was cut. */
export function MoreLine({ more }: { more: number }) {
  if (more === 0) return null;
  return (
    <Localized id="guard-and-more" vars={{ count: more }}>
      <p className="g-more" />
    </Localized>
  );
}
