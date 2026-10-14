import { Localized } from "@fluent/react";
import { ArrowBack } from "../icons";

export type Soon = "restore" | "rescue" | "quick-send" | "sign-in";

const ENGLISH: Record<Soon, { title: string; body: string }> = {
  restore: { title: "Restore setup", body: "Rebuild a laptop from a setup profile you saved after a move. This is coming in a later version." },
  rescue: { title: "Rescue a laptop", body: "Bring files back from a laptop that will not start, by connecting its drive to a working one. This is coming in a later version." },
  "quick-send": {
    title: "Quick Send",
    body: "Send chosen files or folders to a laptop nearby, at any time, not only during a move. It is free, with no account, and it is coming in the first release.",
  },
  "sign-in": {
    title: "Cloud Twin",
    body: "Signing in to see your saved setup comes with Cloud Twin, in a later version. Moving your own laptops will always be free, with no account.",
  },
};

/** A feature shown on Welcome that is not built yet: says what it will do, changes nothing. */
export default function ComingSoon({ which, onBack }: { which: Soon; onBack: () => void }) {
  return (
    <>
      <div className="pair-strip">
        <button type="button" className="back" onClick={onBack}>
          <ArrowBack />
          <Localized id="role-back">
            <span>Back</span>
          </Localized>
        </button>
      </div>
      <Localized id="soon-eyebrow">
        <p className="eyebrow">Coming soon</p>
      </Localized>
      <Localized id={`soon-${which}-title`}>
        <h1>{ENGLISH[which].title}</h1>
      </Localized>
      <Localized id={`soon-${which}-body`}>
        <p className="subhead">{ENGLISH[which].body}</p>
      </Localized>
    </>
  );
}
