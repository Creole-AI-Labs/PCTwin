import { Localized } from "@fluent/react";
import type { ReactNode } from "react";
import { Dialog } from "../components/kit";
import { ArrowForward, Check, History, Info, Wifi } from "../icons";
import "./connection.css";

/**
 * C10 The connection has ended: a dialog, shown when the other laptop leaves. It says what is kept
 * (finished, checked work and the saved move) and what is not (the old code), and gives three ways
 * on. Closing it, or Escape, returns home, which changes nothing. `movedSomething` false hides
 * "See what has moved", because there would be nothing to see.
 */
export default function ConnectionEnded({
  movedSomething = true,
  onPairAgain,
  onSeeMoved,
  onHome,
}: {
  movedSomething?: boolean;
  onPairAgain: () => void;
  onSeeMoved: () => void;
  onHome: () => void;
}) {
  return (
    <Dialog
      open
      title={
        <Localized id="ended-title">
          <span />
        </Localized>
      }
      onClose={onHome}
      closeLabel={
        <Localized id="ended-home">
          <span />
        </Localized>
      }
      actions={
        <>
          <ActionCard icon={<Wifi />} title="ended-pair-again" body="ended-pair-again-body" onClick={onPairAgain} />
          {movedSomething && <ActionCard icon={<History />} title="ended-see-moved" body="ended-see-moved-body" onClick={onSeeMoved} />}
        </>
      }
    >
      <Localized id="ended-body">
        <p />
      </Localized>
      <ul className="connect-keep">
        <li>
          <span className="connect-keep-icon" aria-hidden="true">
            <Check />
          </span>
          <span>
            <Localized id="ended-kept-title">
              <strong />
            </Localized>
            <Localized id="ended-kept-body">
              <span />
            </Localized>
          </span>
        </li>
        <li>
          <span className="connect-keep-icon" aria-hidden="true">
            <Info />
          </span>
          <span>
            <Localized id="ended-not-kept-title">
              <strong />
            </Localized>
            <Localized id="ended-not-kept-body">
              <span />
            </Localized>
          </span>
        </li>
      </ul>
    </Dialog>
  );
}

function ActionCard({ icon, title, body, onClick }: { icon: ReactNode; title: string; body: string; onClick: () => void }) {
  return (
    <button type="button" className="connect-action-card" onClick={onClick}>
      <span className="connect-action-icon" aria-hidden="true">
        {icon}
      </span>
      <span className="connect-action-text">
        <Localized id={title}>
          <strong />
        </Localized>
        <Localized id={body}>
          <small />
        </Localized>
      </span>
      <ArrowForward />
    </button>
  );
}
