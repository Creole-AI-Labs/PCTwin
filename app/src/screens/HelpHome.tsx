import { useLocalization } from "@fluent/react";
import { useState } from "react";
import { Button } from "../components/kit";
import { HELP_URL, openExternal } from "../external";
import Feedback from "./Feedback";
import FrameFooter from "./FrameFooter";
import { TopStrip } from "./FrameParts";
import { Heading } from "./ProgressParts";
import SafeStop from "./SafeStop";
import "./feedback.css";
import "./progress.css";

/** The guides, in the order a move meets them. Each has a title and a body (help-guide-<key>-title, -body). */
const GUIDES = ["find", "mac-network", "firewall", "stopped", "others", "code"] as const;

/**
 * S01 Help for the task in front of you: short guides that work without the internet, one per
 * common problem, each opened in place. More help is on getpctwin.com. Nothing here sends anything.
 */
export default function HelpHome({ onBack }: { onBack: () => void }) {
  const { l10n } = useLocalization();
  // Feedback and the update-or-remove page open in place; Back returns to the guides.
  const [page, setPage] = useState<"guides" | "feedback" | "safe-stop">("guides");
  const guides = () => {
    setPage("guides");
  };
  if (page === "feedback") return <Feedback onBack={guides} />;
  if (page === "safe-stop") return <SafeStop onBack={guides} />;
  return (
    <>
      <TopStrip onBack={onBack} labelId="help-home-eyebrow" />
      <Heading id="help-home-title" />
      <p className="subhead">{l10n.getString("help-home-body")}</p>
      <div className="mv-guides">
        {GUIDES.map((key) => (
          <details key={key} className="mv-details mv-guide">
            <summary>{l10n.getString(`help-guide-${key}-title`)}</summary>
            <p>{l10n.getString(`help-guide-${key}-body`)}</p>
          </details>
        ))}
      </div>
      <div className="mv-actions fb-actions">
        <Button
          variant="secondary"
          onClick={() => {
            setPage("feedback");
          }}
        >
          {l10n.getString("help-home-feedback")}
        </Button>
        <Button
          variant="secondary"
          onClick={() => {
            setPage("safe-stop");
          }}
        >
          {l10n.getString("help-home-update")}
        </Button>
      </div>
      <FrameFooter note="none">
        <Button variant="quiet" onClick={() => void openExternal(HELP_URL)}>
          {l10n.getString("help-home-more")}
        </Button>
      </FrameFooter>
    </>
  );
}
