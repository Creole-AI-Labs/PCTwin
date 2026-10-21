import { useLocalization } from "@fluent/react";
import { Banner, Button } from "../components/kit";
import type { MoveStage } from "../bridge";
import FrameFooter from "./FrameFooter";
import { TopStrip } from "./FrameParts";
import { Heading } from "./ProgressParts";
import "./progress.css";

/** The words for each wait, by which laptop this is. */
const WORDS: Record<"old" | "new", Record<MoveStage["kind"], string>> = {
  old: { scanning: "move-wait-scanning", waiting: "move-wait-approval", moving: "move-wait-starting" },
  new: { scanning: "move-wait-list", waiting: "move-wait-list", moving: "move-wait-starting" },
};

/**
 * Between pairing and progress (M02 on the old laptop; the new laptop waiting for the old one's
 * list): what is happening now, that the old laptop is only read, and a way to stop. Nothing is
 * counted here that the engine has not measured, so there is no bar.
 */
export default function MoveWaiting({ stage, old, onStop }: { stage: MoveStage["kind"]; old: boolean; onStop: () => void }) {
  const { l10n } = useLocalization();
  const key = WORDS[old ? "old" : "new"][stage];
  return (
    <>
      <TopStrip labelId="move-wait-eyebrow" />
      <Heading key={key} id={`${key}-title`} />
      <p className="subhead" role="status">
        {l10n.getString(`${key}-body`)}
      </p>
      <div className="mv-note">
        <Banner tone="info">{l10n.getString("progress-old-safe")}</Banner>
      </div>
      <FrameFooter>
        <Button variant="quiet" onClick={onStop}>
          {l10n.getString("move-stop")}
        </Button>
      </FrameFooter>
    </>
  );
}
