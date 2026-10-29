import { Localized, useLocalization } from "@fluent/react";
import { Banner, Button } from "../components/kit";
import { ArrowForward } from "../icons";
import { labelText, type Label } from "../pairing";
import FrameFooter from "./FrameFooter";
import { Folder, Heading, QsStrip } from "./QsParts";

/**
 * Q01 Quick Send home: Send files and Receive files as the two big choice cards (Quick Send keeps no
 * record of what was sent or received, so there is no history row; the mockup's Request and Pending
 * rows are out of scope), with Choose files as the main action in the window footer. While this laptop is ready to receive, `receiving`
 * carries the name nearby laptops see for it, and `noNearby` says no other laptop was found. `notice`
 * says the engine is not connected yet (the screens exist, nothing is sent or received).
 * Under the cards, when their handlers are given: Send a note or link (Q24) as a third card, and
 * the rows Remembered connections and Waiting sends.
 * Events: onSend, onReceive, onNote, onKnown, onWaiting, onStopReceiving, onBack.
 */
export default function QuickHome({
  receiving,
  noNearby = false,
  notice = false,
  onSend,
  onReceive,
  onNote,
  onKnown,
  onWaiting,
  onStopReceiving,
  onBack,
}: {
  receiving?: Label;
  noNearby?: boolean;
  notice?: boolean;
  onSend: () => void;
  onReceive: () => void;
  onNote?: () => void;
  onKnown?: () => void;
  onWaiting?: () => void;
  onStopReceiving?: () => void;
  onBack: () => void;
}) {
  const { l10n } = useLocalization();
  const cards = [
    { id: "send", go: onSend },
    { id: "receive", go: onReceive },
    ...(onNote ? [{ id: "note-card", go: onNote }] : []),
  ];
  const rows = [
    ...(onKnown ? [{ id: "qs-home-known", go: onKnown }] : []),
    ...(onWaiting ? [{ id: "qs-home-waiting", go: onWaiting }] : []),
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

      {rows.length > 0 && (
        <ul className="qs-home-rows">
          {rows.map(({ id, go }) => (
            <li key={id}>
              <button type="button" className="qs-home-row" onClick={go}>
                <span>{l10n.getString(id)}</span>
                <ArrowForward />
              </button>
            </li>
          ))}
        </ul>
      )}

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
