import { useLocalization } from "@fluent/react";
import { Button } from "../components/kit";
import { measuredPercent, type MoveProgress } from "../move";
import FrameFooter from "./FrameFooter";
import MoveTips, { type TipContext } from "./MoveTips";
import Paused from "./Paused";
import { Eyebrow, GroupRows, Heading, MeasuredBar, Notices, NowLine, PercentValue, RouteStrip } from "./ProgressParts";

export type ProgressHandlers = {
  onPause: () => void;
  onResume: () => void;
  /** Open the stop dialog (X10), from the running and the paused screen. */
  onStop?: () => void;
  /** Open the list of items that need the person (the expanded view or the report). */
  onSeeNeedsAction: () => void;
};

/**
 * X01 Move progress, compact (480 x 620): the route between the two laptops, the measured percent,
 * the bar, and a row for each group. Footer, as in the mockup: Pause move at the start and Keep
 * compact at the end. When the move is pausing or paused, the same window shows X05 instead.
 * `tips`: what this laptop knows about itself, for the one tip under the bar (T14); none without it.
 */
export default function ProgressCompact({
  progress,
  onPause,
  onResume,
  onStop,
  onMinimise,
  onSeeNeedsAction,
  tips,
}: { progress: MoveProgress; onMinimise: () => void; tips?: Omit<TipContext, "batteryWarning"> } & ProgressHandlers) {
  const { l10n } = useLocalization();
  if (progress.state !== "running") return <Paused progress={progress} onResume={onResume} onStop={onStop} />;
  return (
    <>
      <RouteStrip laptops={progress.laptops} />
      <div className="mv-head">
        <div>
          <Eyebrow id="progress-eyebrow" />
          {/* The old laptop sends; the new laptop is being set up: each says its own part. */}
          <Heading id={tips?.old ? "progress-title-old" : "progress-title"} />
        </div>
        <PercentValue pct={measuredPercent(progress)} />
      </div>
      <MeasuredBar progress={progress} />
      {tips && <MoveTips context={{ ...tips, batteryWarning: progress.notice?.kind === "low-battery" }} />}
      <GroupRows groups={progress.groups} />
      <NowLine current={progress.current} />
      <Notices progress={progress} onSeeNeedsAction={onSeeNeedsAction} />
      <p className="mv-safe">{l10n.getString("progress-old-safe")}</p>
      <FrameFooter note="none">
        <Button variant="quiet" onClick={onPause}>
          {l10n.getString("progress-pause")}
        </Button>
        {/* Stop is never hidden behind Pause (found by the first real test, 28 October 2026). */}
        {onStop && (
          <Button variant="quiet" onClick={onStop}>
            {l10n.getString("move-stop")}
          </Button>
        )}
        <Button variant="quiet" onClick={onMinimise}>
          {l10n.getString("progress-compact")}
        </Button>
      </FrameFooter>
    </>
  );
}
