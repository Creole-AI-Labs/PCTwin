import { Localized } from "@fluent/react";
import { ArrowForward, History, MoveIllustration, Wrench } from "../icons";
import type { Soon } from "./ComingSoon";

/** F01 Welcome: the free move first; restore, rescue and Quick Send as secondary choices. Nothing paid, no sign-in. */
export default function Welcome({ onStart, onSoon, onQuick }: { onStart: () => void; onSoon: (which: Soon) => void; onQuick: () => void }) {
  return (
    <>
      <Localized id="welcome-eyebrow">
        <p className="eyebrow">Your next chapter, ready.</p>
      </Localized>
      <h1 className="welcome">
        <Localized id="welcome-title-line1">
          <span>Your setup.</span>
        </Localized>
        <Localized id="welcome-title-line2">
          <span>Your next laptop.</span>
        </Localized>
      </h1>
      <Localized id="welcome-body">
        <p className="subhead welcome-subhead">Bring your files, apps and familiar settings along for the move.</p>
      </Localized>
      <MoveIllustration className="illustration flip" />
      <button type="button" className="button button-primary" onClick={onStart}>
        <Localized id="welcome-start">
          <span>Start a move</span>
        </Localized>
        <ArrowForward />
      </button>
      <div className="secondary-actions">
        <button
          type="button"
          className="button"
          onClick={() => {
            onSoon("restore");
          }}
        >
          <Localized id="welcome-restore">
            <span>Restore setup</span>
          </Localized>
          <History />
        </button>
        <button
          type="button"
          className="button"
          onClick={() => {
            onSoon("rescue");
          }}
        >
          <Localized id="welcome-rescue">
            <span>Rescue a laptop</span>
          </Localized>
          <Wrench />
        </button>
        <button
          type="button"
          className="button"
          onClick={onQuick}
        >
          <Localized id="welcome-quick-send">
            <span>Quick Send</span>
          </Localized>
          <ArrowForward />
        </button>
      </div>
    </>
  );
}
