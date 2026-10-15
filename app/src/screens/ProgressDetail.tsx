import { useLocalization } from "@fluent/react";
import { Banner, Button, Metrics } from "../components/kit";
import { formatBytes, measuredPercent, minutesRange, pageLocale, type MoveProgress, type QueueItem } from "../move";
import type { ProgressHandlers } from "./ProgressCompact";
import { Heading, Notices, PhaseBadge, ResultCell, Topline } from "./ProgressParts";

/**
 * X02 Move progress, expanded (760 x 620): the same measured numbers as X01, then every item in
 * flight and the latest to end, each with its own state in words. An item says "copied" only once
 * it has ended as copied; nothing here counts a file as done while it is still being checked.
 * The whole list lives in the report (R02); this shows what is moving now.
 */
export default function ProgressDetail({
  progress,
  queue,
  onPause,
  onResume,
  onCollapse,
  onStop,
  onSeeNeedsAction,
}: { progress: MoveProgress; queue: QueueItem[]; onCollapse: () => void } & ProgressHandlers) {
  const { l10n } = useLocalization();
  const paused = progress.state !== "running";
  const pct = measuredPercent(progress);
  const timeLeft = progress.minutesLeft ? l10n.getString("progress-minutes", minutesRange(progress.minutesLeft)) : "—";
  return (
    <>
      <Topline onBack={onCollapse} eyebrow="progress-eyebrow" />
      <Heading id="progress-all-title" />
      <p className="subhead">{l10n.getString("progress-all-body")}</p>
      <Metrics
        items={[
          { value: pct === null ? "—" : `${pct.toString()}%`, label: l10n.getString("progress-metric-overall") },
          { value: formatBytes(progress.bytes.checked, pageLocale()), label: l10n.getString("progress-metric-committed") },
          { value: timeLeft, label: l10n.getString("progress-metric-estimate") },
        ]}
      />
      <Notices progress={progress} onSeeNeedsAction={onSeeNeedsAction} />
      {queue.length === 0 ? (
        <p className="mv-empty">{l10n.getString("progress-queue-empty")}</p>
      ) : (
        <div className="mv-table-wrap">
          <table className="mv-table">
            <thead>
              <tr>
                <th scope="col">{l10n.getString("progress-col-item")}</th>
                <th scope="col">{l10n.getString("progress-col-from")}</th>
                <th scope="col">{l10n.getString("progress-col-status")}</th>
              </tr>
            </thead>
            <tbody>
              {queue.map((item) => (
                <tr key={item.id}>
                  <th scope="row">
                    <bdi dir="auto">{item.name}</bdi>
                  </th>
                  <td>
                    <bdi dir="auto" className="mv-path">
                      {item.source}
                    </bdi>
                  </td>
                  <td>{item.phase === "ended" ? <ResultCell outcome={item.outcome} /> : <PhaseBadge phase={item.phase} paused={paused} />}</td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      )}
      <div className="mv-note">
        <Banner tone="info">{l10n.getString("progress-checked-note")}</Banner>
      </div>
      <div className="mv-actions">
        {paused ? (
          <Button variant="primary" onClick={onResume}>
            {l10n.getString("progress-resume")}
          </Button>
        ) : (
          <Button variant="primary" onClick={onPause}>
            {l10n.getString("progress-pause")}
          </Button>
        )}
        <Button variant="quiet" className="mv-danger" onClick={onStop}>
          {l10n.getString("progress-stop")}
        </Button>
      </div>
    </>
  );
}
