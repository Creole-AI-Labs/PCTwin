import { useLocalization } from "@fluent/react";
import { Button } from "../components/kit";
import { HELP_URL, openExternal } from "../external";
import FrameFooter from "./FrameFooter";
import { TopStrip } from "./FrameParts";
import { Heading } from "./ProgressParts";
import "./progress.css";

/** The guides, in the order a move meets them. Each has a title and a body (help-guide-<key>-title, -body). */
const GUIDES = ["find", "mac-network", "firewall", "stopped", "others", "code"] as const;

/**
 * S01 Help for the task in front of you: short guides that work without the internet, one per
 * common problem, each opened in place. More help is on getpctwin.com. Nothing here sends anything.
 */
export default function HelpHome({ onBack }: { onBack: () => void }) {
  const { l10n } = useLocalization();
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
      <FrameFooter note="none">
        <Button variant="quiet" onClick={() => void openExternal(HELP_URL)}>
          {l10n.getString("help-home-more")}
        </Button>
      </FrameFooter>
    </>
  );
}
