import { Localized, useLocalization } from "@fluent/react";
import { Button } from "../components/kit";
import { ArrowForward, Info } from "../icons";
import type { BeforeItem } from "../people";
import FrameFooter from "./FrameFooter";
import { TopStrip } from "./FrameParts";
import { Heading } from "./PairParts";
import "./people.css";

type Props = {
  items: readonly BeforeItem[];
  onBack: () => void;
  /** The person ticked or unticked one. Their word only: PCTwin cannot see whether it was done. */
  onToggle: (id: string, done: boolean) => void;
  onContinue: () => void;
};

/**
 * Before you start (M10/M13 early), laid out as the mockup's browser step (M13): a card of checkbox
 * rows, each with a title and a line under it, then a note. What to do on the old laptop first for
 * things that cannot move by copying. Each tick is the person's own check, kept apart from anything
 * PCTwin measured, and none of it is required: Continue always works.
 */
export default function BeforeYouStart({ items, onBack, onToggle, onContinue }: Props) {
  const { l10n } = useLocalization();

  return (
    <div className="before">
      <TopStrip onBack={onBack} labelId="before-eyebrow" />
      <Heading id="before-title" />
      <Localized id="before-body">
        <p className="subhead" />
      </Localized>

      {items.length === 0 ? (
        <Localized id="before-empty">
          <p className="people-note before-empty" />
        </Localized>
      ) : (
        <ul className="before-list">
          {items.map((item) => (
            <li key={item.id}>
              <label className="kit-check">
                <Localized id="before-done" attrs={{ "aria-label": true }} vars={{ item: l10n.getString(`before-${item.kind}-title`) }}>
                  <input
                    type="checkbox"
                    checked={item.done}
                    onChange={(e) => {
                      onToggle(item.id, e.target.checked);
                    }}
                  />
                </Localized>
                <span>
                  <Localized id={`before-${item.kind}-title`}>
                    <strong />
                  </Localized>
                  <Localized id={`before-${item.kind}-body`}>
                    <small className="kit-hint" />
                  </Localized>
                </span>
              </label>
            </li>
          ))}
        </ul>
      )}

      <div className="note">
        <Info />
        <Localized id="before-note">
          <p className="note-text" />
        </Localized>
      </div>

      <FrameFooter>
        <Button variant="primary" icon={<ArrowForward className="flip" />} onClick={onContinue}>
          {l10n.getString("before-continue")}
        </Button>
      </FrameFooter>
    </div>
  );
}
