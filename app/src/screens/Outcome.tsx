import { useLocalization } from "@fluent/react";
import { Badge, Banner, Button, Metrics, Row, RowList } from "../components/kit";
import { Attention, Check } from "../icons";
import { formatBytes, pageLocale, type MoveOutcome } from "../move";
import { Eyebrow, Heading } from "./ProgressParts";

const DONE = ["copied", "reinstalled", "converted"] as const;

/**
 * R01 Welcome to your next laptop (480 x 620): two measured numbers, one "Selected files" row with
 * its honest chip, and a warning when anything is unresolved. The chip says "Verified" only when
 * every approved item is copied and checked; anything else, including a stopped move, says
 * "Partial". Item-by-item detail is in the report (R02).
 */
export default function Outcome({ outcome, onReport, onHome }: { outcome: MoveOutcome; onReport: () => void; onHome: () => void }) {
  const { l10n } = useLocalization();
  const { byStatus, stopped, notCopied } = outcome;
  const needsAction = byStatus["needs-action"];
  const done = DONE.reduce((n, s) => n + byStatus[s], 0);
  // Everything the person approved: what ended some way (less what they left out themselves), plus what never started.
  const approved = done + needsAction + byStatus.failed + notCopied;
  const complete = done === approved;
  const body = [
    l10n.getString("outcome-files-summary", { done, total: approved }),
    outcome.checkedFromMemory > 0 ? l10n.getString("outcome-files-memory", { count: outcome.checkedFromMemory }) : "",
  ]
    .filter(Boolean)
    .join(" ");
  return (
    <>
      <Eyebrow id="outcome-eyebrow" />
      <Heading id={stopped ? "outcome-stopped-title" : "outcome-title"} />
      <p className="subhead">{l10n.getString(stopped ? "outcome-stopped-body" : "outcome-body")}</p>
      <Metrics
        items={[
          { value: formatBytes(outcome.bytesChecked, pageLocale()), label: l10n.getString("outcome-metric-size") },
          { value: needsAction.toLocaleString(pageLocale()), label: l10n.getString("outcome-metric-action") },
        ]}
      />
      <RowList>
        <Row
          title={l10n.getString("outcome-files-title")}
          detail={body}
          end={
            complete ? (
              <Badge tone="good" icon={<Check className="mv-icon" />}>
                {l10n.getString("progress-state-verified")}
              </Badge>
            ) : (
              <Badge tone="warn" icon={<Attention className="mv-icon" />}>
                {l10n.getString("outcome-status-partial")}
              </Badge>
            )
          }
        />
      </RowList>
      {!complete && (
        <div className="mv-note">
          <Banner tone="attention">{l10n.getString("outcome-old-safe")}</Banner>
        </div>
      )}
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
