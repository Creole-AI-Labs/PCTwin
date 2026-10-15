import { useLocalization } from "@fluent/react";
import { Button } from "../components/kit";
import { ArrowForward } from "../icons";
import { measuredPercent, type MoveProgress } from "../move";
import Paused from "./Paused";
import { Eyebrow, Heading, MeasuredBar, Notices, NowLine, PercentValue, GroupRows, RouteStrip } from "./ProgressParts";

export type ProgressHandlers = {
  onPause: () => void;
  onResume: () => void;
  onStop: () => void;
  /** Open the list of items that need the person (the expanded view or the report). */
  onSeeNeedsAction: () => void;
};

/**
 * X01 Move progress, compact (480 x 620). Everything on it is measured: the percent from bytes
 * written and checked, each count on its own line. When the move is pausing or paused, the same
 * window shows X05 instead (the person is in the same place, the words change).
 */
export default function ProgressCompact({
  progress,
  onPause,
  onResume,
  onExpand,
  onMinimise,
  onStop,
  onSeeNeedsAction,
}: { progress: MoveProgress; onExpand: () => void; onMinimise: () => void } & ProgressHandlers) {
  const { l10n } = useLocalization();
  if (progress.state !== "running") return <Paused progress={progress} onResume={onResume} onStop={onStop} />;
  return (
    <>
      <RouteStrip laptops={progress.laptops} />
      <div className="mv-head">
        <div>
          <Eyebrow id="progress-eyebrow" />
          <Heading id="progress-title" />
        </div>
        <PercentValue pct={measuredPercent(progress)} />
      </div>
      <MeasuredBar progress={progress} />
      <GroupRows groups={progress.groups} />
      <NowLine current={progress.current} />
      <Notices progress={progress} onSeeNeedsAction={onSeeNeedsAction} />
      <p className="mv-safe">{l10n.getString("progress-old-safe")}</p>
      <div className="mv-actions">
        <Button variant="primary" onClick={onPause}>
          {l10n.getString("progress-pause")}
        </Button>
        <Button onClick={onExpand} icon={<ArrowForward />}>
          {l10n.getString("progress-expand")}
        </Button>
      </div>
      <div className="mv-actions mv-actions-quiet">
        <Button variant="quiet" onClick={onMinimise}>
          {l10n.getString("progress-compact")}
        </Button>
        <Button variant="quiet" className="mv-danger" onClick={onStop}>
          {l10n.getString("progress-stop")}
        </Button>
      </div>
    </>
  );
}
