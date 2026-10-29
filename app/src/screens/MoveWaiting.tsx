import { useLocalization } from "@fluent/react";
import { Banner, Button } from "../components/kit";
import type { WaitStage } from "../bridge";
import { formatBytes, pageLocale } from "../move";
import FrameFooter from "./FrameFooter";
import { TopStrip } from "./FrameParts";
import { Heading } from "./ProgressParts";
import "./progress.css";
import { useRescueDrive } from "./rescueContext";

/** The words for each wait, by which laptop this is. */
const WORDS: Record<"old" | "new", Record<WaitStage, string>> = {
  old: { scanning: "move-wait-scanning", waiting: "move-wait-approval", moving: "move-wait-starting", administrator: "move-wait-admin", "app-data": "move-wait-appdata" },
  new: { scanning: "move-wait-list", waiting: "move-wait-list", moving: "move-wait-starting", administrator: "move-wait-admin", "app-data": "move-wait-list" },
};

/**
 * Between pairing and progress (M02 on the old laptop; the new laptop waiting for the old one's
 * list): what is happening now, that the old laptop is only read, and a way to stop. Nothing is
 * counted here that the engine has not measured, so there is no bar.
 */
export default function MoveWaiting({
  stage,
  old,
  note,
  found = null,
  onBefore,
  onStop,
}: {
  stage: WaitStage;
  old: boolean;
  note?: string;
  /** While scanning: bytes of the mover's own things found so far (only ever grows). */
  found?: number | null;
  /** The old laptop, while it waits: open "Before you leave this laptop" (M10/M13). */
  onBefore?: () => void;
  onStop: () => void;
}) {
  const { l10n } = useLocalization();
  const drive = useRescueDrive();
  // A rescue reads a drive: no laptop on the other side makes a list.
  const key = drive !== null && !old && WORDS.new[stage] === "move-wait-list" ? "rescue-wait" : WORDS[old ? "old" : "new"][stage];
  const vars = drive !== null ? { drive } : undefined;
  return (
    <>
      <TopStrip labelId="move-wait-eyebrow" />
      <Heading key={key} id={`${key}-title`} vars={vars} />
      <p className="subhead" role="status">
        {l10n.getString(`${key}-body`, vars)}
      </p>
      {found !== null && found > 0 && (
        <p className="mv-found" role="status">
          {l10n.getString("move-found", { size: formatBytes(found, pageLocale()) })}
        </p>
      )}
      <div className="mv-note">
        {note && <Banner tone="attention">{note}</Banner>}
        <Banner tone="info">{l10n.getString(drive !== null ? "rescue-safe" : "progress-old-safe")}</Banner>
      </div>
      {onBefore && (
        <div className="mv-actions">
          <Button variant="quiet" onClick={onBefore}>
            {l10n.getString("move-before-open")}
          </Button>
        </div>
      )}
      <FrameFooter>
        <Button variant="quiet" onClick={onStop}>
          {l10n.getString("move-stop")}
        </Button>
      </FrameFooter>
    </>
  );
}
