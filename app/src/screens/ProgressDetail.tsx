import { useLocalization } from "@fluent/react";
import { Banner, Button, Metrics } from "../components/kit";
import { formatBytes, groupState, measuredPercent, minutesRange, pageLocale, type MoveProgress } from "../move";
import { labelText } from "../pairing";
import type { ProgressHandlers } from "./ProgressCompact";
import { GroupBadge, GroupLabel, Heading, Notices, Topline } from "./ProgressParts";

/**
 * X02 Move progress, expanded (760 x 620): three measured numbers, then one table row for each
 * group with its source and state. The state words are X01's, derived from the group's counts, so
 * a group still being checked is never shown as verified. The item-by-item list is the report (R02).
 */
export default function ProgressDetail({
  progress,
  onPause,
  onResume,
  onCollapse,
  onStop,
  onSeeNeedsAction,
}: { progress: MoveProgress; onCollapse: () => void } & ProgressHandlers) {
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
      {progress.groups.length === 0 ? (
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
              {progress.groups.map((g) => (
                <tr key={g.id}>
                  <th scope="row">
                    <GroupLabel name={g.name} />
                  </th>
                  <td>
                    <bdi dir="auto">{labelText(l10n, progress.laptops.old)}</bdi>
                  </td>
                  <td>
                    <GroupBadge state={groupState(g.counts)} paused={paused} />
                    {g.counts.needsAction > 0 && <p className="mv-reason">{l10n.getString("progress-count-needs-action", { count: g.counts.needsAction })}</p>}
                    {g.counts.failed > 0 && <p className="mv-reason">{l10n.getString("progress-count-failed", { count: g.counts.failed })}</p>}
                  </td>
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
