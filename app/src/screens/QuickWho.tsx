import { Localized, useLocalization } from "@fluent/react";
import { Banner, Button } from "../components/kit";
import { ArrowForward, Info, Laptop } from "../icons";
import { labelText } from "../pairing";
import { useBytes, type NearbyLaptop } from "../quicksend";
import FrameFooter from "./FrameFooter";
import { Heading, QsStrip, TallyBar } from "./QsParts";

/**
 * Q04 Who should receive it? (sender) Laptops nearby that can receive, and "use a code" as the other
 * card, with the main action showing the send code. The code and the number match that follow are the
 * existing pairing screens (PairOld shows the code and asks for the number), so choosing here only
 * starts them. A laptop that is busy with another send says so and cannot be chosen; two laptops with
 * the same name say so, so the person checks the other screen.
 * Events: onChooseNearby(index), onUseCode, onBack.
 */
export default function QuickWho({
  count,
  size,
  nearby,
  onChooseNearby,
  onUseCode,
  onBack,
}: {
  count: number;
  size: number;
  nearby: NearbyLaptop[];
  onChooseNearby: (index: number) => void;
  onUseCode: () => void;
  onBack: () => void;
}) {
  const { l10n } = useLocalization();
  const bytes = useBytes();
  return (
    <>
      <QsStrip onBack={onBack} />
      <Heading id="qs-who-title" />
      <Localized id="qs-who-body">
        <p className="subhead" />
      </Localized>
      <TallyBar count={count} size={bytes(size)} />

      <ul className="choices">
        {nearby.map((n) => {
          const name = labelText(l10n, n.label);
          return (
            <li key={n.index}>
              <button
                type="button"
                className="choice"
                disabled={n.busy}
                onClick={() => {
                  onChooseNearby(n.index);
                }}
              >
                <span className="choice-icon">
                  <Laptop />
                </span>
                <span className="choice-text">
                  <span className="choice-title">
                    <bdi dir="auto">{name}</bdi>
                  </span>
                  <span className="choice-body">
                    {n.busy ? l10n.getString("qs-who-busy", { label: name }) : n.sameLabelNearby ? l10n.getString("qs-who-same-label") : l10n.getString("qs-who-nearby")}
                  </span>
                </span>
                <ArrowForward />
              </button>
            </li>
          );
        })}
        <li>
          <button type="button" className="choice" onClick={onUseCode}>
            <span className="choice-icon">
              <Laptop />
            </span>
            <span className="choice-text">
              <Localized id="qs-who-code">
                <span className="choice-title" />
              </Localized>
              <Localized id="qs-who-code-body">
                <span className="choice-body" />
              </Localized>
            </span>
            <ArrowForward />
          </button>
        </li>
      </ul>

      {nearby.length === 0 && (
        <div role="status" className="qs-status">
          <Banner tone="info">
            <Localized id="qs-no-nearby">
              <p />
            </Localized>
          </Banner>
        </div>
      )}

      <div className="note">
        <Info />
        <Localized id="qs-who-note">
          <p className="note-text" />
        </Localized>
      </div>

      <FrameFooter note="send">
        <Button variant="primary" onClick={onUseCode}>
          <Localized id="qs-who-action">
            <span />
          </Localized>
        </Button>
      </FrameFooter>
    </>
  );
}
