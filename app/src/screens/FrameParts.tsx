/**
 * Pieces every extended mockup frame shares.
 *
 * `TopStrip`: "Back" on the start side and the section label in small capitals on the end side
 * (LOCAL MOVE, REVIEWED MOVE, APPROVED JOB, MOVE RESULTS, QUICK SEND ...).
 * - `onBack`: when given, Back shows and calls it; without it the start side is empty.
 * - `labelId`: the Fluent key of the label, or pass `label` for text you already have.
 *
 * `Steps`: the vertical list of steps (done, current, next) the mockups use. A tick in a circle is
 * done, a filled blue dot is current, an empty ring is next, joined by a line. The shapes differ, so
 * state is never colour alone, and each step also carries a visually hidden state word for screen
 * readers; the current step has aria-current="step".
 * - `steps`: `{ state: "done" | "current" | "todo"; title: ReactNode; body?: ReactNode; extra?: ReactNode }[]`.
 */
import { Localized } from "@fluent/react";
import type { ReactNode } from "react";
import { ArrowBack, Check } from "../icons";
import "./frame.css";

export function TopStrip({ onBack, labelId, label }: { onBack?: () => void; labelId?: string; label?: ReactNode }) {
  return (
    <div className="frame-strip">
      {onBack ? (
        <button type="button" className="back" onClick={onBack}>
          <ArrowBack />
          <Localized id="frame-back">
            <span />
          </Localized>
        </button>
      ) : (
        <span />
      )}
      {labelId ? (
        <Localized id={labelId}>
          <p className="eyebrow frame-label" />
        </Localized>
      ) : (
        <p className="eyebrow frame-label">{label}</p>
      )}
    </div>
  );
}

export type StepState = "done" | "current" | "todo";

export interface StepItem {
  state: StepState;
  title: ReactNode;
  body?: ReactNode;
  /** Anything extra under the body, such as the network name. */
  extra?: ReactNode;
}

export function Steps({ steps }: { steps: readonly StepItem[] }) {
  return (
    <ol className="frame-steps">
      {steps.map((step, i) => (
        <li key={i} className={`frame-step frame-step-${step.state}`} aria-current={step.state === "current" ? "step" : undefined}>
          <span className="frame-step-marker" aria-hidden="true">
            {step.state === "done" ? <Check /> : step.state === "current" ? <span className="frame-step-dot" /> : null}
          </span>
          <span className="frame-step-body">
            <strong>{step.title}</strong>
            {step.body && <small>{step.body}</small>}
            {step.extra}
          </span>
          <span className="visually-hidden">
            <Localized id={`frame-step-${step.state}`}>
              <span />
            </Localized>{" "}
            <Localized id="frame-step-position" vars={{ number: i + 1, total: steps.length }}>
              <span />
            </Localized>
          </span>
        </li>
      ))}
    </ol>
  );
}
