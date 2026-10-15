import { useLocalization } from "@fluent/react";
import { Badge, Banner, Button, Metrics, ResultChip, Row, RowList } from "../components/kit";
import { Info } from "../icons";
import { ITEM_STATUSES, formatBytes, pageLocale, type MoveOutcome } from "../move";
import { Eyebrow, Heading } from "./ProgressParts";

const DONE = ["copied", "reinstalled", "converted"] as const;

/**
 * R01 Welcome to your next laptop (480 x 620): what happened to everything the person approved,
 * counted per status. It says "every item" only when that is true; anything unfinished, failed or
 * needing the person stays on the page; files checked from this laptop's memory are called out.
 */
export default function Outcome({ outcome, onReport, onHome }: { outcome: MoveOutcome; onReport: () => void; onHome: () => void }) {
  const { l10n } = useLocalization();
  const { byStatus, stopped, notCopied } = outcome;
  const needsAction = byStatus["needs-action"];
  const failed = byStatus.failed;
  const left = needsAction + failed;
  const clean = !stopped && left === 0 && notCopied === 0;
  const rows = ITEM_STATUSES.map((status) => ({ status, count: byStatus[status] })).filter((r) => r.count > 0);
  return (
    <>
      <Eyebrow id="outcome-eyebrow" />
      <Heading id={stopped ? "outcome-stopped-title" : "outcome-title"} />
      <p className="subhead">{l10n.getString(stopped ? "outcome-stopped-body" : "outcome-body")}</p>
      <Metrics
        items={[
          { value: formatBytes(outcome.bytesChecked, pageLocale()), label: l10n.getString("outcome-metric-size") },
          { value: DONE.reduce((n, s) => n + byStatus[s], 0).toLocaleString(pageLocale()), label: l10n.getString("outcome-metric-done") },
          { value: needsAction.toLocaleString(pageLocale()), label: l10n.getString("outcome-metric-action") },
        ]}
      />
      <RowList>
        {rows.map((r) => (
          <Row key={r.status} title={l10n.getString(`outcome-${r.status}`, { count: r.count })} end={<ResultChip result={r.status} />} />
        ))}
        {notCopied > 0 && (
          <Row
            title={l10n.getString("outcome-not-copied", { count: notCopied })}
            end={<Badge icon={<Info className="mv-icon" />}>{l10n.getString("outcome-not-copied-chip")}</Badge>}
          />
        )}
      </RowList>
      <div className="mv-note">
        {clean && <Banner tone="ready">{l10n.getString("outcome-all-done")}</Banner>}
        {left > 0 && <Banner tone="attention">{l10n.getString("outcome-unfinished", { count: left })}</Banner>}
      </div>
      <h2 className="mv-subtitle">{l10n.getString("outcome-next-title")}</h2>
      <ul className="mv-next">
        {needsAction > 0 && <li>{l10n.getString("outcome-next-needs-action")}</li>}
        {failed > 0 && <li>{l10n.getString("outcome-next-failed")}</li>}
        {outcome.checkedFromMemory > 0 && <li>{l10n.getString("outcome-checked-memory", { count: outcome.checkedFromMemory })}</li>}
        <li>{l10n.getString("outcome-next-check")}</li>
      </ul>
      <p className="mv-safe">{l10n.getString("outcome-old-safe")}</p>
      <div className="mv-actions">
        <Button variant="primary" onClick={onReport}>
          {l10n.getString("outcome-report")}
        </Button>
        <Button variant="quiet" onClick={onHome}>
          {l10n.getString("outcome-home")}
        </Button>
      </div>
    </>
  );
}
