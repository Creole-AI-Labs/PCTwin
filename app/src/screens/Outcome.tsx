import { useLocalization } from "@fluent/react";
import { Badge, Banner, Button, Metrics, Row, RowList, Section } from "../components/kit";
import { Attention, Check } from "../icons";
import { KINDS, formatBytes, pageLocale, type MoveOutcome } from "../move";
import Seal from "../motion/Seal";
import FrameFooter from "./FrameFooter";
import { RebuiltSection, rebuiltNeedsAction } from "./FullMoveParts";
import { TopStrip } from "./FrameParts";
import { GroupLabel, Heading } from "./ProgressParts";
import { useRescueDrive } from "./rescueContext";
import { shownName } from "../names";

const DONE = ["copied", "reinstalled", "converted"] as const;

/**
 * R01 Welcome to your next laptop (480 x 620): two measured numbers, one "Selected files" row with
 * its honest chip, and a warning when anything is unresolved. The chip says "Verified" only when
 * every approved item is copied and checked; anything else, including a stopped move, says
 * "Partial". Item-by-item detail is in the report (R02).
 *
 * Under them (and the warning, which stays first), the after-move check: each kind of file the person cares about (photos, videos, music,
 * documents) that the plan held, with how many arrived copied and checked, counted the way the
 * scan counted them. A kind the plan did not hold is left out; one short of its plan says Partial.
 */
export default function Outcome({ outcome, onReport, onHome, onUndo, onFinish, old = false }: { outcome: MoveOutcome; onReport: () => void; onHome?: () => void; onUndo?: () => void; /** Finish your move (2.7): on the new laptop only. */ onFinish?: () => void; /** This is the old laptop: it sent, it is not where the person arrives. */ old?: boolean }) {
  const { l10n } = useLocalization();
  const { byStatus, stopped, notCopied } = outcome;
  const drive = useRescueDrive();
  // A move that ended early on its own (something went wrong between the laptops) never says the person stopped it.
  const [titleId, bodyId] = outcome.brokeOff
    ? ["outcome-broke-title", "outcome-broke-body"]
    : stopped
      ? ["outcome-stopped-title", "outcome-stopped-body"]
      : old
        ? ["outcome-title-old", "outcome-body-old"]
        : ["outcome-title", "outcome-body"];
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
      <TopStrip labelId="outcome-eyebrow" />
      {drive !== null && titleId === "outcome-title" ? <Heading id="rescue-outcome-title" vars={{ drive }} /> : <Heading id={titleId} />}
      <p className="subhead">{l10n.getString(bodyId)}</p>
      <Metrics
        items={[
          { value: formatBytes(outcome.bytesChecked, pageLocale()), label: l10n.getString("outcome-metric-size") },
          { value: (needsAction + rebuiltNeedsAction(outcome.rebuilt)).toLocaleString(pageLocale()), label: l10n.getString("outcome-metric-action") },
        ]}
      />
      <RowList>
        <Row
          title={l10n.getString("outcome-files-title")}
          detail={body}
          end={
            complete ? (
              <Badge
                tone="good"
                icon={
                  <Seal verified={!stopped}>
                    <Check className="mv-icon" />
                  </Seal>
                }
              >
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
      {(outcome.people ?? []).length > 0 && (
        <RowList>
          {(outcome.people ?? []).map((p) => (
            <Row
              key={p.name}
              title={<bdi dir="auto">{shownName(p.name)}</bdi>}
              detail={[
                l10n.getString("outcome-person-copied", { count: p.copied }),
                p.failed + p.needsAction > 0 ? l10n.getString("outcome-person-not-done", { count: p.failed + p.needsAction }) : "",
                p.atSignIn ? l10n.getString("outcome-person-at-sign-in", { name: p.name }) : "",
              ]
                .filter(Boolean)
                .join(" ")}
            />
          ))}
        </RowList>
      )}
      {!complete && (
        <div className="mv-note">
          <Banner tone="attention">{l10n.getString("outcome-old-safe")}</Banner>
        </div>
      )}
      {outcome.landed && outcome.landed.length > 0 && (
        // Where the person's things are now, by group (first real test, 28 October 2026).
        <Section title={l10n.getString("outcome-landed-title")}>
          <RowList>
            {outcome.landed.map((l) => (
              <Row
                key={l.folder}
                title={<GroupLabel name={l.name} />}
                detail={l.moved ? l10n.getString("outcome-landed-moved") : <bdi dir="auto">{l.folder}</bdi>}
              />
            ))}
          </RowList>
        </Section>
      )}
      <AfterMoveCheck outcome={outcome} />
      {outcome.rebuilt && <RebuiltSection rebuilt={outcome.rebuilt} onUndo={onUndo} />}
      <FrameFooter>
        {onHome && (
          <Button variant="quiet" onClick={onHome}>
            {l10n.getString("outcome-home")}
          </Button>
        )}
        {onFinish && (
          <Button variant="secondary" onClick={onFinish}>
            {l10n.getString("finish-title")}
          </Button>
        )}
        <Button variant="primary" onClick={onReport}>
          {l10n.getString("outcome-report")}
        </Button>
      </FrameFooter>
    </>
  );
}

/** The after-move check: one row per kind the plan held, with its counts and an honest chip. */
function AfterMoveCheck({ outcome }: { outcome: MoveOutcome }) {
  const { l10n } = useLocalization();
  const kinds = KINDS.filter((k) => (outcome.afterMove?.[k].planned ?? 0) > 0);
  if (!outcome.afterMove || kinds.length === 0) return null;
  const after = outcome.afterMove;
  return (
    <Section title={l10n.getString("outcome-kinds-title")}>
      <RowList>
        {kinds.map((k) => {
          const { planned, copied } = after[k];
          return (
            <Row
              key={k}
              title={l10n.getString(`outcome-kind-${k}`)}
              detail={l10n.getString("outcome-kind-count", { copied, planned })}
              end={
                copied === planned ? (
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
          );
        })}
      </RowList>
    </Section>
  );
}
