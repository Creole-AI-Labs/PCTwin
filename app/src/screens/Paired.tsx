import { Localized } from "@fluent/react";
import { Badge, Button } from "../components/kit";
import { ArrowForward, Check } from "../icons";
import Accent from "../motion/Accent";
import FrameFooter from "./FrameFooter";
import { Heading, PairStrip } from "./PairParts";

/** Both laptops are paired. Continue starts the move: the old laptop finds the files, the new laptop shows the plan to approve. */
export default function Paired({ onContinue }: { onContinue?: () => void }) {
  return (
    <>
      <PairStrip step={2} />
      <div className="pair-paired-badge">
        <Badge
          tone="good"
          icon={
            <Accent>
              <Check />
            </Accent>
          }
        >
          <Localized id="pair-done-eyebrow">
            <span />
          </Localized>
        </Badge>
      </div>
      <Heading id="pair-done-title" />
      <Localized id="pair-done-body">
        <p className="subhead" />
      </Localized>
      {onContinue && (
        <FrameFooter>
          <Localized id="pair-done-continue">
            <Button variant="primary" icon={<ArrowForward />} onClick={onContinue}>
              Continue
            </Button>
          </Localized>
        </FrameFooter>
      )}
    </>
  );
}
