/**
 * Pieces every finishing page shares (Task List 2.7): the page frame (Back, the heading that takes
 * focus), loading a view from the engine with the engine's own problem words, and names shown as
 * text. A name from a laptop is always text, isolated for direction, never markup.
 */
import { useLocalization } from "@fluent/react";
import { useCallback, useEffect, useState, type ReactNode } from "react";
import { toFinishProblem } from "../bridge";
import type { FinishProblem } from "../finish";
import { Banner, Button } from "../components/kit";
import { pageLocale } from "../move";
import { shownName } from "../names";
import { TopStrip } from "./FrameParts";
import { Heading } from "./ProgressParts";
import "./finish.css";

export type Load<T> = { kind: "loading" } | { kind: "ready"; value: T } | { kind: "problem"; problem: FinishProblem };

/** Loads a view when the page opens, and again on `reload`. `set` puts in a newer view the engine sent back. */
export function useLoad<T>(fetch: () => Promise<T>): { state: Load<T>; set: (value: T) => void; reload: () => void } {
  const [state, setState] = useState<Load<T>>({ kind: "loading" });
  const [round, setRound] = useState(0);
  useEffect(() => {
    let live = true;
    fetch().then(
      (value) => {
        if (live) setState({ kind: "ready", value });
      },
      (error: unknown) => {
        if (live) setState({ kind: "problem", problem: toFinishProblem(error) });
      },
    );
    return () => {
      live = false;
    };
    // `fetch` is the page's own call and is made once per round.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [round]);
  const set = useCallback((value: T) => {
    setState({ kind: "ready", value });
  }, []);
  const reload = useCallback(() => {
    setState({ kind: "loading" });
    setRound((r) => r + 1);
  }, []);
  return { state, set, reload };
}

/** One finishing page: Back, the section label, a heading that takes focus, then the page. */
export function FinishPage({ titleId, onBack, children }: { titleId: string; onBack?: () => void; children: ReactNode }) {
  return (
    <>
      <TopStrip onBack={onBack} labelId="finish-strip" />
      <Heading id={titleId} />
      {children}
    </>
  );
}

export function Loading() {
  const { l10n } = useLocalization();
  return (
    <p role="status" className="subhead">
      {l10n.getString("finish-loading")}
    </p>
  );
}

/** What went wrong and what to do next, in words, announced when it appears. */
export function ProblemNotice({ problem, onRetry }: { problem: FinishProblem; onRetry?: () => void }) {
  const { l10n } = useLocalization();
  return (
    <div className="mv-note">
      <Banner tone="attention">{l10n.getString(`finish-problem-${problem.kind}`)}</Banner>
      {onRetry && (
        <div className="mv-actions">
          <Button variant="secondary" onClick={onRetry}>
            {l10n.getString("finish-retry")}
          </Button>
        </div>
      )}
    </div>
  );
}

/** A page's body once its view is loaded: the loading line, the problem, or the page. */
export function Loaded<T>({ state, onRetry, children }: { state: Load<T>; onRetry: () => void; children: (value: T) => ReactNode }) {
  if (state.kind === "loading") return <Loading />;
  if (state.kind === "problem") return <ProblemNotice problem={state.problem} onRetry={onRetry} />;
  return <>{children(state.value)}</>;
}

/** A name from a laptop, as text. */
export function Name({ children }: { children: string }) {
  return <bdi dir="auto">{shownName(children)}</bdi>;
}

/** A moment (seconds since 1970) as a date and time in the page's language. */
export function whenText(seconds: number): string {
  return new Intl.DateTimeFormat(pageLocale(), { dateStyle: "medium", timeStyle: "short" }).format(new Date(seconds * 1000));
}

export function dateText(seconds: number): string {
  return new Intl.DateTimeFormat(pageLocale(), { dateStyle: "medium" }).format(new Date(seconds * 1000));
}

/** "More not shown: N", when a list was cut. */
export function MoreLine({ count }: { count: number }) {
  const { l10n } = useLocalization();
  return count === 0 ? null : <p className="kit-hint">{l10n.getString("finish-more-not-shown", { count })}</p>;
}

/** Whether a message exists, so an unknown key from a newer engine says something useful instead of nothing. */
export function useHas() {
  const { l10n } = useLocalization();
  return (id: string) => l10n.getBundle(id) !== null;
}
