import { Localized } from "@fluent/react";
import { Badge } from "../components/kit";
import { Check } from "../icons";
import { Heading, PairStrip } from "./PairParts";

/** Both laptops are paired. Choosing what to move comes next and is not built yet, so nothing else to do here. */
export default function Paired() {
  return (
    <>
      <PairStrip step={2} />
      <div className="pair-paired-badge">
        <Badge tone="good" icon={<Check />}>
          <Localized id="pair-done-eyebrow">
            <span />
          </Localized>
        </Badge>
      </div>
      <Heading id="pair-done-title" />
      <Localized id="pair-done-body">
        <p className="subhead" />
      </Localized>
    </>
  );
}
