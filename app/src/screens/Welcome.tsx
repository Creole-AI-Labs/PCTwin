import { Localized } from "@fluent/react";
import { ArrowForward, History, MoveIllustration, Wrench } from "../icons";
import type { Soon } from "./ComingSoon";

/**
 * F01 Welcome: the free move first; restore, rescue and Quick Send as secondary choices. Nothing paid, no sign-in.
 * With a move left unfinished on this laptop, the primary button reads "Resume your move" and reopens it (`unfinished`, `onResume`).
 */
export default function Welcome({
  onStart,
  onSoon,
  unfinished,
  onResume,
}: {
  onStart: () => void;
  onSoon: (which: Soon) => void;
  unfinished?: boolean;
  onResume?: () => void;
}) {
  const resuming = unfinished === true && onResume !== undefined;
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
      <button type="button" className="button button-primary" onClick={resuming ? onResume : onStart}>
        {resuming ? (
          <Localized id="resume-open">
            <span>Resume your move</span>
          </Localized>
        ) : (
          <Localized id="welcome-start">
            <span>Start a move</span>
          </Localized>
        )}
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
          onClick={() => {
            onSoon("quick-send");
          }}
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
