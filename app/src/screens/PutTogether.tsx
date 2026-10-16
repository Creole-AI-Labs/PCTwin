import { Localized } from "@fluent/react";
import { Button } from "../components/kit";
import { ArrowForward, Info } from "../icons";
import FrameFooter from "./FrameFooter";
import { Steps, TopStrip, type StepState } from "./FrameParts";
import { Heading } from "./PairParts";
import "./connection.css";

export type { StepState };

const DEFAULT_STEPS: readonly [StepState, StepState, StepState] = ["current", "todo", "todo"];

/**
 * C02 Put both laptops together: three steps (shapes plus hidden state words, see Steps). PCTwin
 * cannot see whether the other laptop has done its part, so a step is shown as done only when the
 * caller knows it is; `network` is the Wi-Fi this laptop is on, shown only when known.
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
      <TopStrip onBack={onBack} labelId="connect-eyebrow" />
      <Heading id="connect-prep-title" />
      <Localized id="connect-prep-body">
        <p className="subhead" />
      </Localized>
      <Steps
        steps={steps.map((state, i) => ({
          state,
          title: (
            <Localized id={`connect-prep-step-${(i + 1).toString()}-title`}>
              <span />
            </Localized>
          ),
          body: (
            <Localized id={`connect-prep-step-${(i + 1).toString()}-body`}>
              <span />
            </Localized>
          ),
          extra:
            i === 0 && network ? (
              <Localized id="connect-prep-step-1-network" vars={{ network }}>
                <small className="connect-step-network" />
              </Localized>
            ) : undefined,
        }))}
      />
      <div className="note">
        <Info />
        <Localized id="connect-prep-note">
          <p className="note-text" />
        </Localized>
      </div>
      <FrameFooter>
        <Button variant="primary" icon={<ArrowForward />} onClick={onContinue}>
          <Localized id="connect-prep-continue">
            <span />
          </Localized>
        </Button>
      </FrameFooter>
    </>
  );
}
