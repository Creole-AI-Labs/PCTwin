import { useLocalization } from "@fluent/react";
import { Banner, Button } from "../components/kit";
import FrameFooter from "./FrameFooter";
import { TopStrip } from "./FrameParts";
import { Heading } from "./ProgressParts";
import "./progress.css";

const STEPS = ["fda-step-1", "fda-step-2", "fda-step-3"] as const;

/**
 * The Full Disk Access guide (macOS): the Mac kept some of this person's own places private, so
 * PCTwin could not look in them. It says how many, how to let PCTwin in (with a button that opens
 * the very settings page), and gives two ways on: look again, or go on without them. Going on
 * without them leaves them out of the move entirely; nothing is claimed about them.
 */
export default function FullDiskAccess({
  places,
  onOpenSettings,
  onCheckAgain,
  onWithout,
}: {
  places: number;
  onOpenSettings: () => void;
  onCheckAgain: () => void;
  onWithout: () => void;
}) {
  const { l10n } = useLocalization();
  return (
    <>
      <TopStrip labelId="move-wait-eyebrow" />
      <Heading id="fda-title" />
      <p className="subhead">{l10n.getString("fda-body", { count: places })}</p>
      <ol className="mv-steps">
        {STEPS.map((id) => (
          <li key={id}>{l10n.getString(id)}</li>
        ))}
      </ol>
      <div className="mv-actions">
        <Button variant="quiet" onClick={onOpenSettings}>
          {l10n.getString("fda-open")}
        </Button>
      </div>
      <div className="mv-note">
        <Banner tone="info">{l10n.getString("fda-note")}</Banner>
      </div>
      <FrameFooter>
        <Button variant="quiet" onClick={onWithout}>
          {l10n.getString("fda-without")}
        </Button>
        <Button variant="primary" onClick={onCheckAgain}>
          {l10n.getString("fda-again")}
        </Button>
      </FrameFooter>
    </>
  );
}
