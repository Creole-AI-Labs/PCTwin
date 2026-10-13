import { Localized } from "@fluent/react";
import { ArrowBack } from "../icons";

export type Soon = "restore" | "rescue" | "sign-in";

const ENGLISH: Record<Soon, { title: string; body: string }> = {
  restore: { title: "Restore setup", body: "Rebuild a laptop from a setup profile you saved after a move. This is coming in a later version." },
  rescue: { title: "Rescue a laptop", body: "Bring files back from a laptop that will not start, by connecting its drive to a working one. This is coming in a later version." },
  "sign-in": {
    title: "Cloud Twin",
    body: "Signing in to see your saved setup comes with Cloud Twin, in a later version. Moving your own laptops will always be free, with no account.",
  },
};

/** A feature shown on Welcome that is not built yet: says what it will do, changes nothing. */
export default function ComingSoon({ which, onBack }: { which: Soon; onBack: () => void }) {
  return (
    <>
      <button type="button" className="link back" onClick={onBack}>
        <ArrowBack />
        <Localized id="role-back">
          <span>Back</span>
        </Localized>
      </button>
      <Localized id="soon-eyebrow">
        <p className="eyebrow">Coming soon</p>
      </Localized>
      <Localized id={`soon-${which}-title`}>
        <h1>{ENGLISH[which].title}</h1>
      </Localized>
      <Localized id={`soon-${which}-body`}>
        <p className="lead">{ENGLISH[which].body}</p>
      </Localized>
    </>
  );
}
