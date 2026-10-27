/**
 * Undo (Task List 2.3 and 2.7; blueprint R05 and R06).
 *
 * R05, the preview: what undo would do with each change, looked at now: put back, the app removed
 * (the person's own data kept), kept because it changed after the move, only its service or maker can
 * undo it, or left alone because it was there before. Nothing happens until the person chooses Undo.
 *
 * R06, the result: Restored, Removed, As before, Kept because you changed it, Kept (and why under
 * More details), Kept under another name, Done elsewhere, or Not done yet (undo tries again). Never
 * "rolled back" unless it was.
 */
import { Localized, useLocalization } from "@fluent/react";
import { Badge, Row, RowList } from "../components/kit";
import type { ScreenView, UndoPlan, UndoResult } from "../guard";
import { Attention, Check, Lock, Restore } from "../icons";
import FrameFooter from "./FrameFooter";
import { TopStrip } from "./FrameParts";
import { UndoLegend } from "./ChangesPreview";
import { IntentButton, MeasuredLine, MoreDetails, MoreLine, Name, RefusalLine, StateLine, useGuard } from "./GuardParts";
import { Heading } from "./ProgressParts";

function PlanBadge({ plan }: { plan: UndoPlan }) {
  const { l10n } = useLocalization();
  const icon = plan === "put-back" || plan === "remove-app" ? <Restore /> : plan === "elsewhere" ? <Attention /> : <Lock />;
  return (
    <Badge tone={plan === "elsewhere" ? "warn" : "neutral"} icon={icon}>
      {l10n.getString(`guard-undo-plan-${plan}`)}
    </Badge>
  );
}

function ResultBadge({ result }: { result: UndoResult }) {
  const { l10n } = useLocalization();
  const good = result.kind === "restored" || result.kind === "removed" || result.kind === "as-before";
  const words = result.kind === "kept-at" ? l10n.getString("guard-undo-result-kept-at", { name: result.at }) : l10n.getString(`guard-undo-result-${result.kind}`);
  return (
    <Badge tone={good ? "good" : "warn"} icon={good ? <Check /> : <Attention />}>
      {words}
    </Badge>
  );
}

export function UndoPreview({ screen }: { screen: Extract<ScreenView, { kind: "undo-preview" }> }) {
  const { l10n } = useLocalization();
  return (
    <>
      <TopStrip labelId="guard-strip-undo" />
      <Heading id="guard-undo-preview-title" />
      <Localized id="guard-undo-preview-body">
        <p className="subhead" />
      </Localized>
      <StateLine />
      <RefusalLine />
      <UndoLegend />
      <RowList>
        {screen.lines.map((l, i) => (
          <Row key={i} title={<Name>{l.name}</Name>} end={<PlanBadge plan={l.plan} />} />
        ))}
      </RowList>
      <MoreLine more={screen.more} />
      <FrameFooter>
        <IntentButton intent={{ kind: "back" }} variant="quiet" label={l10n.getString("guard-undo-keep")} />
        <IntentButton intent={{ kind: "undo" }} variant="primary" label={l10n.getString("guard-undo-run")} />
      </FrameFooter>
    </>
  );
}

export function UndoDone({ screen }: { screen: Extract<ScreenView, { kind: "undo-done" }> }) {
  const { l10n } = useLocalization();
  const { view } = useGuard();
  const details = screen.lines.filter((l) => l.detail !== null);
  const working = view.display === "working" || view.display === "applying";
  return (
    <>
      <TopStrip labelId="guard-strip-undo" />
      <Heading id={working ? "guard-undo-running-title" : "guard-undo-done-title"} />
      <StateLine />
      <RefusalLine />
      {working && <MeasuredLine progress={view.progress} />}
      <RowList>
        {screen.lines.map((l, i) => (
          <Row key={i} title={<Name>{l.name}</Name>} end={<ResultBadge result={l.result} />} />
        ))}
      </RowList>
      <MoreLine more={screen.more} />
      {details.length > 0 && (
        <MoreDetails>
          <ul>
            {details.map((l, i) => (
              <li key={i}>
                <Name>{l.name}</Name>: <span lang="en">{l.detail}</span>
              </li>
            ))}
          </ul>
        </MoreDetails>
      )}
      <FrameFooter>
        <IntentButton intent={{ kind: "back" }} variant="primary" label={l10n.getString("guard-undo-close")} />
      </FrameFooter>
    </>
  );
}
