/**
 * The full move on the move's own screens: the apps, settings and app data section of the one
 * approval screen (M23), the page shown while they are set up after the files, and the section of
 * the outcome (R01) with each one's result word and Undo.
 *
 * Every line says what happens or what the person does next, in plain words; the result is a
 * word with an icon, never colour alone. Names are shown exactly as they are.
 */
import { useLocalization } from "@fluent/react";
import { Badge, Banner, Button, Checkbox, Row, RowList, Section } from "../components/kit";
import type { ChangeView, DataProgress, RebuildPlan, RebuildStep, Rebuilt, RebuiltLine, Title } from "../fullmove";
import { shownName } from "../names";
import FrameFooter from "./FrameFooter";
import { Attention, Check } from "../icons";
import { pageLocale } from "../move";
import { TopStrip } from "./FrameParts";
import { PromiseBadge, UndoLegend } from "./ChangesPreview";
import { Name } from "./GuardParts";
import { PlanBox } from "./PlanParts";
import { Heading } from "./ProgressParts";

function TitleText({ title }: { title: Title }) {
  const { l10n } = useLocalization();
  return title.kind === "name" ? <Name>{title.value}</Name> : <>{l10n.getString(title.value)}</>;
}

/** What a change still needs from the person, or what it does, in words. */
function changeDetail(c: ChangeView, l10n: ReturnType<typeof useLocalization>["l10n"]): string {
  if (c.special) return l10n.getString(`fullmove-step-${c.special}`);
  if (c.kind === "settings") {
    return [l10n.getString("fullmove-settings-count", { count: c.count }), l10n.getString(c.action === "replace" ? "fullmove-settings-replace" : "fullmove-settings-merge")].join(" ");
  }
  if (c.needs) return l10n.getString(`reason-${c.needs}`);
  if (c.action === "install") return l10n.getString("fullmove-app-install");
  return l10n.getString("fullmove-app-here");
}

function Changes({ titleId, changes }: { titleId: string; changes: ChangeView[] }) {
  const { l10n } = useLocalization();
  if (changes.length === 0) return null;
  return (
    <PlanBox title={l10n.getString(titleId, { count: changes.length })}>
      <RowList>
        {changes.map((c, i) => (
          <Row key={i} title={<TitleText title={c.title} />} detail={changeDetail(c, l10n)} end={<PromiseBadge promise={c.undo} />} />
        ))}
      </RowList>
    </PlanBox>
  );
}

/** M23's section for the full move: every app, setting and special category, with what undo can do for each. */
export function RebuildPlanSection({ rebuild, picked = [], onPick }: { rebuild: RebuildPlan; picked?: readonly number[]; onPick?: (line: number, bring: boolean) => void }) {
  const { l10n } = useLocalization();
  const install = rebuild.changes.filter((c) => c.kind === "app" && c.action === "install");
  const settings = rebuild.changes.filter((c) => c.kind === "settings");
  const appData = rebuild.changes.filter((c) => c.kind === "special" && c.special === "copy" && c.secret === null);
  const secrets = rebuild.changes.filter((c) => c.secret !== null);
  const needs = rebuild.changes.filter((c) => c.needs !== null);
  const here = rebuild.changes.filter((c) => c.kind === "app" && c.action === "leave-alone" && c.needs === null);
  const left = rebuild.notMoved.filter((n) => n.count > 0);
  return (
    <section className="fm-plan" aria-labelledby="fm-plan-title">
      <h2 id="fm-plan-title" className="plan-box-title">
        {l10n.getString("fullmove-plan-title")}
      </h2>
      <p className="subhead">{l10n.getString("fullmove-plan-body")}</p>
      <UndoLegend />
      <Changes titleId="fullmove-group-install" changes={install} />
      <Changes titleId="fullmove-group-settings" changes={settings} />
      <Changes titleId="fullmove-group-appdata" changes={appData} />
      {secrets.length > 0 && (
        <PlanBox title={l10n.getString("fullmove-group-secrets", { count: secrets.length })}>
          <p className="plan-lead">{l10n.getString("fullmove-secrets-body")}</p>
          <ul className="plan-items">
            {secrets.map((c) => {
              const line = c.secret ?? -1;
              const name = c.title.kind === "name" ? c.title.value : l10n.getString(c.title.value);
              return (
                <li key={line} className="plan-item">
                  <Checkbox
                    checked={picked.includes(line)}
                    onChange={(on) => {
                      onPick?.(line, on);
                    }}
                    label={l10n.getString("fullmove-secret-bring", { name })}
                    hint={l10n.getString("fullmove-secret-hint")}
                  />
                </li>
              );
            })}
          </ul>
        </PlanBox>
      )}
      <Changes titleId="fullmove-group-needs" changes={needs} />
      <Changes titleId="fullmove-group-here" changes={here} />
      {rebuild.more > 0 && <p className="subhead">{l10n.getString("fullmove-more", { count: rebuild.more })}</p>}
      {(left.length > 0 || rebuild.specialNotMoved > 0) && (
        <PlanBox title={l10n.getString("fullmove-not-moved-title")}>
          <RowList>
            {left.map((n) => (
              <Row key={n.group} title={l10n.getString(`fullmove-not-moved-${n.group}`, { count: n.count })} />
            ))}
            {rebuild.specialNotMoved > 0 && <Row title={l10n.getString("fullmove-special-not-moved", { count: rebuild.specialNotMoved })} />}
          </RowList>
        </PlanBox>
      )}
      {rebuild.prompt && (
        <div className="mv-note">
          <Banner tone="info">{l10n.getString(rebuild.restorePoint ? "fullmove-prompt-restore" : "fullmove-prompt")}</Banner>
        </div>
      )}
    </section>
  );
}

/** After the files: apps and settings being set up. Nothing to press; it ends on the outcome. */
export function Rebuilding({ step }: { step: RebuildStep | null }) {
  const { l10n } = useLocalization();
  const line =
    step === null
      ? null
      : step.step === "permission"
        ? l10n.getString("fullmove-rebuilding-permission")
        : l10n.getString(`fullmove-rebuilding-${step.step}`, { done: step.done.toLocaleString(pageLocale()), total: step.total.toLocaleString(pageLocale()) });
  return (
    <>
      <TopStrip labelId="outcome-eyebrow" />
      <Heading id="fullmove-rebuilding-title" />
      <p className="subhead">{l10n.getString("fullmove-rebuilding-body")}</p>
      {line && (
        <p className="subhead" role="status">
          {line}
        </p>
      )}
    </>
  );
}

function lineDetail(l: RebuiltLine, l10n: ReturnType<typeof useLocalization>["l10n"]): string {
  const parts: string[] = [];
  if (l.kind === "settings") parts.push(l10n.getString("fullmove-settings-count", { count: l.count }));
  if (l.reason) parts.push(l10n.getString(l.kind === "special" && l.reason === "finish-by-hand" ? "fullmove-special-finish" : `reason-${l.reason}`));
  return parts.join(" ");
}

/** R01's section for the full move: each app, group of settings and special category with its result, and Undo. */
export function RebuiltSection({ rebuilt, onUndo }: { rebuilt: Rebuilt; onUndo?: () => void }) {
  const { l10n } = useLocalization();
  if (rebuilt.lines.length === 0 && !rebuilt.stopped) return null;
  return (
    <Section title={l10n.getString("fullmove-outcome-title")}>
      {rebuilt.stopped && (
        <div className="mv-note">
          <Banner tone="attention">{l10n.getString("fullmove-outcome-stopped")}</Banner>
        </div>
      )}
      <RowList>
        {rebuilt.lines.map((l, i) => {
          const good = l.status === "copied" || l.status === "reinstalled" || l.status === "converted";
          return (
            <Row
              key={i}
              title={<TitleText title={l.title} />}
              detail={lineDetail(l, l10n)}
              end={
                <Badge tone={good ? "good" : "warn"} icon={good ? <Check className="mv-icon" /> : <Attention className="mv-icon" />}>
                  {l10n.getString(`status-${l.status}`)}
                </Badge>
              }
            />
          );
        })}
      </RowList>
      {rebuilt.more > 0 && <p className="subhead">{l10n.getString("fullmove-more", { count: rebuilt.more })}</p>}
      {rebuilt.restorePoint !== "not-asked" && <p className="subhead">{l10n.getString(`fullmove-restore-${rebuilt.restorePoint}`)}</p>}
      {rebuilt.canUndo && onUndo && (
        <div className="fm-undo">
          <Button variant="secondary" onClick={onUndo}>
            {l10n.getString("fullmove-outcome-undo")}
          </Button>
        </div>
      )}
    </Section>
  );
}

/** How many of the full move's lines need the person (added to R01's "needs action" figure). */
export function rebuiltNeedsAction(rebuilt: Rebuilt | undefined): number {
  return rebuilt ? rebuilt.lines.filter((l) => l.status === "needs-action").length : 0;
}

/** The new laptop, while the old laptop's app data comes: measured, and what the old laptop waits for. */
export function AppDataComing({ progress }: { progress: DataProgress }) {
  const { l10n } = useLocalization();
  return (
    <>
      <TopStrip labelId="outcome-eyebrow" />
      <Heading id="fullmove-appdata-title" />
      <p className="subhead" role="status">
        {l10n.getString("fullmove-appdata-progress", { done: progress.done.toLocaleString(pageLocale()), total: progress.total.toLocaleString(pageLocale()) })}
      </p>
      {progress.waitingFor.length > 0 && (
        <div className="mv-note">
          <Banner tone="attention">{l10n.getString("fullmove-appdata-waiting", { apps: progress.waitingFor.map(shownName).join(", ") })}</Banner>
        </div>
      )}
    </>
  );
}

/** The old laptop: apps to close before their data can be read, with Try again or Go on without them. */
export function CloseApps({ apps, onAnswer }: { apps: string[]; onAnswer: (retry: boolean) => void }) {
  const { l10n } = useLocalization();
  return (
    <>
      <TopStrip labelId="move-wait-eyebrow" />
      <Heading id="fullmove-close-title" />
      <p className="subhead">{l10n.getString("fullmove-close-body")}</p>
      <RowList>
        {apps.map((a) => (
          <Row key={a} title={<Name>{a}</Name>} />
        ))}
      </RowList>
      <FrameFooter>
        <Button
          variant="quiet"
          onClick={() => {
            onAnswer(false);
          }}
        >
          {l10n.getString("fullmove-close-without")}
        </Button>
        <Button
          variant="primary"
          onClick={() => {
            onAnswer(true);
          }}
        >
          {l10n.getString("fullmove-close-again")}
        </Button>
      </FrameFooter>
    </>
  );
}
