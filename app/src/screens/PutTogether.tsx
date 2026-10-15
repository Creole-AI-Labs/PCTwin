import { Localized } from "@fluent/react";
import { Badge, Button } from "../components/kit";
import { ArrowForward, Check, Info } from "../icons";
import { Heading, PairStrip } from "./PairParts";
import "./connection.css";

export type StepState = "done" | "current" | "todo";

const DEFAULT_STEPS: readonly [StepState, StepState, StepState] = ["current", "todo", "todo"];

/**
 * C02 Put both laptops together: three steps, each with its state in words (never colour alone).
 * PCTwin cannot see whether the other laptop has done its part, so a step is shown as done only
 * when the caller knows it is; `network` is the Wi-Fi this laptop is on, shown only when known.
 */
export default function PutTogether({
  steps = DEFAULT_STEPS,
  network,
  onBack,
  onContinue,
}: {
  steps?: readonly [StepState, StepState, StepState];
  network?: string;
  onBack: () => void;
  onContinue: () => void;
}) {
  return (
    <>
      <PairStrip onBack={onBack} />
      <Heading id="connect-prep-title" />
      <Localized id="connect-prep-body">
        <p className="subhead" />
      </Localized>
      <ol className="connect-steps">
        {steps.map((state, i) => (
          <li key={i} className={`connect-step connect-step-${state}`} aria-current={state === "current" ? "step" : undefined}>
            <span className="connect-step-marker" aria-hidden="true">
              {state === "done" ? <Check /> : state === "current" ? <span className="connect-step-dot" /> : null}
            </span>
            <span className="connect-step-body">
              <Localized id={`connect-prep-step-${(i + 1).toString()}-title`}>
                <strong />
              </Localized>
              <Localized id={`connect-prep-step-${(i + 1).toString()}-body`}>
                <small />
              </Localized>
              {i === 0 && network && (
                <Localized id="connect-prep-step-1-network" vars={{ network }}>
                  <small className="connect-step-network" />
                </Localized>
              )}
            </span>
            <span className="visually-hidden">
              <Localized id="connect-step-position" vars={{ number: i + 1, total: steps.length }}>
                <span />
              </Localized>
            </span>
            <StepWord state={state} />
          </li>
        ))}
      </ol>
      <div className="note">
        <Info />
        <Localized id="connect-prep-note">
          <p className="note-text" />
        </Localized>
      </div>
      <div className="connect-actions">
        <Button variant="primary" icon={<ArrowForward />} className="connect-block" onClick={onContinue}>
          <Localized id="connect-prep-continue">
            <span />
          </Localized>
        </Button>
      </div>
    </>
  );
}

/** Done, now or next: always a word, with a shape that also differs (tick, filled dot, empty ring). */
function StepWord({ state }: { state: StepState }) {
  if (state === "done") {
    return (
      <Badge tone="good" icon={<Check />}>
        <Localized id="connect-step-done">
          <span />
        </Localized>
      </Badge>
    );
  }
  return (
    <span className={`kit-chip connect-chip-${state}`}>
      <svg width="13" height="13" viewBox="0 0 24 24" aria-hidden="true" focusable="false" fill={state === "current" ? "currentColor" : "none"} stroke="currentColor" strokeWidth="2.5">
        <circle cx="12" cy="12" r="7" />
      </svg>
      <Localized id={`connect-step-${state}`}>
        <span />
      </Localized>
    </span>
  );
}
