/**
 * Undo (Task List 2.3 and 2.7; blueprint R05 and R06).
 *
 * R05, the preview: what undo would do with each change, looked at now: put back, the app removed
 * (PCTwin does not touch files the person made; it cannot promise what the app's own uninstaller
 * does), kept because it changed after the move, only its service or maker can undo it, or left
 * alone because it was there before. When an app is to be removed, it says the laptop will ask an
 * administrator to approve. Nothing happens until the person chooses Undo.
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
import { OWN_TITLES } from "../fullmove";

/** A line's name: one of the settings catalogue's groups by its key (translated), or a name as it is. */
function LineName({ name }: { name: string }) {
  const { l10n } = useLocalization();
  const own = name.startsWith("fullmove-settings-") && (OWN_TITLES as readonly string[]).includes(name);
  return own ? <>{l10n.getString(name)}</> : <Name>{name}</Name>;
}

function PlanBadge({ plan }: { plan: UndoPlan }) {
  const { l10n } = useLocalization();
  const icon = plan === "put-back" || plan === "remove-app" || plan === "already-gone" ? <Restore /> : plan === "elsewhere" ? <Attention /> : <Lock />;
  return (
    <Badge tone={plan === "elsewhere" ? "warn" : "neutral"} icon={icon}>
      {l10n.getString(`guard-undo-plan-${plan}`)}
    </Badge>
  );
}

function ResultBadge({ result }: { result: UndoResult }) {
  const { l10n } = useLocalization();
  const good = result.kind === "restored" || result.kind === "removed" || result.kind === "removed-restart" || result.kind === "as-before";
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
      {screen.when !== null ? (
        <p className="subhead">{l10n.getString("guard-undo-preview-which", { when: new Date(screen.when * 1000) })}</p>
      ) : (
        <Localized id="guard-undo-preview-which-earlier">
          <p className="subhead" />
        </Localized>
      )}
      {screen.lines.some((l) => l.plan === "remove-app") ? (
        <Localized id="guard-undo-preview-admin">
          <p className="subhead" />
        </Localized>
      ) : null}
      <StateLine />
      <RefusalLine />
      <UndoLegend />
      <RowList>
        {screen.lines.map((l, i) => (
          <Row key={i} title={<LineName name={l.name} />} end={<PlanBadge plan={l.plan} />} />
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
          <Row key={i} title={<LineName name={l.name} />} end={<ResultBadge result={l.result} />} />
        ))}
      </RowList>
      <MoreLine more={screen.more} />
      {details.length > 0 && (
        <MoreDetails>
          <ul>
            {details.map((l, i) => (
              <li key={i}>
                <LineName name={l.name} />: <span lang="en">{l.detail}</span>
              </li>
            ))}
          </ul>
        </MoreDetails>
      )}
      <div className="fm-undo">
        <IntentButton intent={{ kind: "back" }} variant="primary" label={l10n.getString("guard-undo-close")} />
      </div>
    </>
  );
}
