/** Pieces shared by the pairing screens: the Back and progress strip, the focus-taking heading, eyebrow, and problem notice. */
import { Localized, useLocalization } from "@fluent/react";
import { useEffect, useRef } from "react";
import { Banner } from "../components/kit";
import { ArrowBack } from "../icons";
import type { Problem } from "../pairing";
import "./pairing.css";

/** Back (when there is somewhere to go) and the three connection dots; the active one shows the stage. */
export function PairStrip({ onBack, step = 1 }: { onBack?: () => void; step?: 0 | 1 | 2 }) {
  return (
    <div className="pair-strip">
      {onBack ? (
        <button type="button" className="back" onClick={onBack}>
          <ArrowBack />
          <Localized id="pair-back">
            <span />
          </Localized>
        </button>
      ) : (
        <span />
      )}
      <span className="connection-dots" aria-hidden="true">
        {[0, 1, 2].map((i) => (
          <span key={i} className={i === step ? "active" : undefined} />
        ))}
      </span>
    </div>
  );
}

/**
 * The state's heading. It takes keyboard focus when it appears, so a screen reader follows the
 * person from one state to the next. Render it with a key per state so each one is a fresh heading.
 */
export function Heading({ id, vars }: { id: string; vars?: Record<string, string | number> }) {
  const ref = useRef<HTMLHeadingElement>(null);
  useEffect(() => {
    ref.current?.focus();
  }, []);
  return (
    <Localized id={id} vars={vars}>
      <h1 ref={ref} tabIndex={-1} className="pair-heading" />
    </Localized>
  );
}

export function Eyebrow({ id }: { id: string }) {
  return (
    <Localized id={id}>
      <p className="eyebrow" />
    </Localized>
  );
}

/** What went wrong and what to do next, in words, announced when it appears. */
export function ProblemNotice({ problem }: { problem: Problem }) {
  const { l10n } = useLocalization();
  // A key this version has no words for still says something useful: the generic problem.
  const key = l10n.getBundle(problem.key) ? problem.key : "pairing-failed";
  return (
    <div className="pair-notice">
      <Banner tone="attention">
        <Localized id={key}>
          <p />
        </Localized>
      </Banner>
    </div>
  );
}
