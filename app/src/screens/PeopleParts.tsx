/** Pieces the people-and-help screens share: the Back strip. (The focus-taking heading and eyebrow come from PairParts.) */
import { Localized } from "@fluent/react";
import { ArrowBack } from "../icons";
import "./people.css";

/** Back, in the same place and look as on the pairing screens, without the connection dots. */
export function BackStrip({ id, onBack }: { id: string; onBack: () => void }) {
  return (
    <div className="pair-strip">
      <button type="button" className="back" onClick={onBack}>
        <ArrowBack />
        <Localized id={id}>
          <span />
        </Localized>
      </button>
    </div>
  );
}
