import { Localized, useLocalization } from "@fluent/react";
import { Banner, Button } from "../components/kit";
import { Info } from "../icons";
import { labelText, type Label } from "../pairing";
import { totalBytes, useBytes, type OfferedFile } from "../quicksend";
import FrameFooter from "./FrameFooter";
import { FileRows, Heading, QsActions, QsStrip, Route, TallyBar } from "./QsParts";

/**
 * Q08 Someone wants to send you files (receiver): who is sending to whom, the tally, and names and
 * sizes only, nothing from inside the files. The person accepts the whole offer (the receiver's own
 * "skip this file" choices come at Keep your existing work safe) or declines it. The sender never sees
 * a choice of place. `changed` is true when the offer was replaced while it was open. `state` is what
 * replaces the offer when it is over: "declined" after Decline, "gone" when the sender went away; in
 * both nothing was saved.
 * Events: onAccept, onDecline, and onDone from the two ended states.
 */
export default function QuickIncoming({
  from,
  to,
  files,
  changed = false,
  state,
  onAccept,
  onDecline,
  onDone,
}: {
  from: Label;
  to: Label;
  files: OfferedFile[];
  changed?: boolean;
  state?: "declined" | "gone";
  onAccept: () => void;
  onDecline: () => void;
  onDone: () => void;
}) {
  const { l10n } = useLocalization();
  const bytes = useBytes();
  const who = labelText(l10n, from);

  if (state) {
    return (
      <>
        <QsStrip receiving />
        <Heading key={state} id="qs-offer-title" />
        <div role="status" className="qs-status">
          <Banner tone={state === "gone" ? "attention" : "info"}>
            <Localized id={state === "gone" ? "qs-offer-gone" : "qs-offer-declined"} vars={{ label: who }}>
              <p />
            </Localized>
          </Banner>
        </div>
        <FrameFooter note="send">
          <Button variant="primary" onClick={onDone}>
            <Localized id="qs-receipt-done">
              <span />
            </Localized>
          </Button>
        </FrameFooter>
      </>
    );
  }

  return (
    <>
      <QsStrip receiving />
      <Heading id="qs-offer-title" />
      <Localized id="qs-offer-body" vars={{ label: who }}>
        <p className="subhead" />
      </Localized>
      <Route from={from} to={to} />

      <div role="status" className="qs-status">
        {changed && (
          <Banner tone="attention">
            <Localized id="qs-offer-changed">
              <p />
            </Localized>
          </Banner>
        )}
      </div>

      <TallyBar count={files.length} size={bytes(totalBytes(files))} />
      <FileRows files={files} included types />

      <QsActions>
        <button type="button" className="link" onClick={onDecline}>
          <Localized id="qs-offer-decline">
            <span />
          </Localized>
        </button>
      </QsActions>

      <div className="note">
        <Info />
        <Localized id="qs-offer-note">
          <p className="note-text" />
        </Localized>
      </div>

      <FrameFooter note="send">
        <Button variant="primary" disabled={files.length === 0} onClick={onAccept}>
          <Localized id="qs-offer-accept">
            <span />
          </Localized>
        </Button>
      </FrameFooter>
    </>
  );
}
