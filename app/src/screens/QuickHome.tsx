import { Localized, useLocalization } from "@fluent/react";
import { Banner, Button } from "../components/kit";
import { ArrowForward } from "../icons";
import { labelText, type Label } from "../pairing";
import FrameFooter from "./FrameFooter";
import { Folder, Heading, QsStrip, Tile } from "./QsParts";

/**
 * Q01 Quick Send home: Send files and Receive files as the two big choice cards, then the local
 * handover receipts in one smaller row card (the mockup's Request and Pending rows are out of scope),
 * with Choose files as the main action in the window footer. While this laptop is ready to receive, `receiving`
 * carries the name nearby laptops see for it, and `noNearby` says no other laptop was found. `notice`
 * says the engine is not connected yet (the screens exist, nothing is sent or received).
 * Events: onSend, onReceive, onStopReceiving, onHistory, onBack.
 */
export default function QuickHome({
  receiving,
  noNearby = false,
  notice = false,
  onSend,
  onReceive,
  onStopReceiving,
  onHistory,
  onBack,
}: {
  receiving?: Label;
  noNearby?: boolean;
  notice?: boolean;
  onSend: () => void;
  onReceive: () => void;
  onStopReceiving?: () => void;
  onHistory: () => void;
  onBack: () => void;
}) {
  const { l10n } = useLocalization();
  const cards = [
    { id: "send", go: onSend },
    { id: "receive", go: onReceive },
  ];
  return (
    <>
      <QsStrip onBack={onBack} />
      <Heading id="qs-home-title" />
      <Localized id="qs-home-body">
        <p className="subhead" />
      </Localized>

      <div role="status" className="qs-status">
        {notice && (
          <Banner tone="info">
            <Localized id="qs-soon-note">
              <p />
            </Localized>
          </Banner>
        )}
        {receiving && (
          <Banner tone="ready">
            <Localized id="qs-receiving" vars={{ label: labelText(l10n, receiving) }}>
              <p />
            </Localized>
          </Banner>
        )}
        {receiving && noNearby && (
          <Banner tone="info">
            <Localized id="qs-no-nearby">
              <p />
            </Localized>
          </Banner>
        )}
      </div>
      {receiving && onStopReceiving && (
        <div className="qs-block">
          <Button onClick={onStopReceiving}>
            <Localized id="qs-receive-stop">
              <span />
            </Localized>
          </Button>
        </div>
      )}

      <ul className="choices">
        {cards.map(({ id, go }) => (
          <li key={id}>
            <button type="button" className="choice" data-bookmark="" data-icon-job="send" onClick={go}>
              <span className="choice-icon">
                <Folder />
              </span>
              <span className="choice-text">
                <Localized id={`qs-${id}-title`}>
                  <span className="choice-title" />
                </Localized>
                <Localized id={`qs-${id}-body`}>
                  <span className="choice-body" />
                </Localized>
              </span>
              <ArrowForward />
            </button>
          </li>
        ))}
      </ul>

      <div className="qs-rows">
        <button type="button" className="qs-rowlink" onClick={onHistory}>
          <Tile>
            <Folder size={18} />
          </Tile>
          <span>
            <Localized id="qs-history-title">
              <strong />
            </Localized>
            <Localized id="qs-history-body">
              <small />
            </Localized>
          </span>
          <ArrowForward />
        </button>
      </div>

      <FrameFooter note="send">
        <Button variant="primary" onClick={onSend}>
          <Localized id="qs-home-action">
            <span />
          </Localized>
        </Button>
      </FrameFooter>
    </>
  );
}
