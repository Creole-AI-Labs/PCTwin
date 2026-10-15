import { useLocalization } from "@fluent/react";
import { Button, MiniProgress } from "../components/kit";
import { measuredPercent, type MoveProgress } from "../move";
import { useAttribute, useTimeLeft } from "./ProgressParts";

/**
 * F09 Mini panel (360 x 240): what is moving, the measured percent, Pause (or Resume) and Open.
 * Shows a move or, with `kind="send"`, a Quick Send (the same panel, the send's own words).
 * No paid states: nothing here is about cloud, storage or backup.
 */
export default function MiniPanel({
  kind = "move",
  progress,
  onPause,
  onResume,
  onOpen,
}: {
  kind?: "move" | "send";
  progress: MoveProgress;
  onPause: () => void;
  onResume: () => void;
  onOpen: () => void;
}) {
  const { l10n } = useLocalization();
  const attr = useAttribute();
  const timeLeft = useTimeLeft(progress);
  const pct = measuredPercent(progress);
  const send = kind === "send";
  const idle = progress.state !== "running";
  const title =
    progress.state === "pausing"
      ? l10n.getString("pause-title-finishing")
      : progress.state === "paused"
        ? l10n.getString(send ? "mini-title-send-paused" : "mini-title-paused")
        : l10n.getString(send ? "mini-title-send" : "mini-title");
  return (
    <MiniProgress
      kind={kind}
      title={title}
      value={pct === null ? null : pct / 100}
      label={attr(send ? "mini-progress-send" : "mini-progress", "aria-label")}
      left={send ? l10n.getString("mini-what-send") : l10n.getString("mini-what")}
      right={idle ? undefined : (timeLeft ?? undefined)}
      pause={
        idle ? (
          <Button variant="quiet" onClick={onResume}>
            {l10n.getString("mini-resume")}
          </Button>
        ) : (
          <Button variant="quiet" onClick={onPause}>
            {l10n.getString("mini-pause")}
          </Button>
        )
      }
      open={
        <Button variant="primary" onClick={onOpen}>
          {l10n.getString(send ? "mini-open-send" : "mini-open")}
        </Button>
      }
    />
  );
}
