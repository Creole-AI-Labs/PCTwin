import { Localized } from "@fluent/react";
import { ArrowForward, MoveIllustration } from "../icons";

/** F01 Welcome: the free move first. Restore, rescue and sign-in appear when those flows exist. */
export default function Welcome({ onStart }: { onStart: () => void }) {
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
        <p className="lead">Bring your files, apps and familiar settings along for the move.</p>
      </Localized>
      <MoveIllustration className="illustration flip" />
      <button type="button" className="button button-primary" onClick={onStart}>
        <Localized id="welcome-start">
          <span>Start a move</span>
        </Localized>
        <ArrowForward />
      </button>
    </>
  );
}
