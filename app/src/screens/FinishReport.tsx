import { useLocalization, type ReactLocalization } from "@fluent/react";
import { useState } from "react";
import { reportPreview, reportSave, toFinishProblem } from "../bridge";
import { Banner, Button, Checkbox, Section } from "../components/kit";
import type { Disclosure, FinishProblem, ReportView } from "../finish";
import { FinishPage, Loaded, ProblemNotice, useLoad } from "./FinishParts";

const TALLY: readonly [keyof ReportView["tally"], string][] = [
  ["selected", "selected"],
  ["done", "done"],
  ["needsAction", "needs-action"],
  ["failed", "failed"],
  ["waiting", "waiting"],
  ["notReachable", "not-reachable"],
  ["noResultYet", "no-result-yet"],
  ["skipped", "skipped"],
  ["leftOut", "left-out"],
  ["acceptedLoss", "accepted-loss"],
];

/**
 * The text the person reads and saves, made from exactly what the engine's report holds and in the
 * person's language. It holds nothing the report kept out: a name only where the engine sent one.
 */
export function reportText(l10n: ReactLocalization, view: ReportView): string {
  const lines: string[] = [l10n.getString("finish-report-text-title"), "", l10n.getString(`finish-headline-${view.headline}`), ""];
  for (const [key, kebab] of TALLY) {
    if (key === "selected" || key === "done" || view.tally[key] > 0) lines.push(l10n.getString(`finish-tally-${kebab}`, { count: view.tally[key].toLocaleString() }));
  }
  if (view.accountId !== null) lines.push("", l10n.getString("finish-report-text-account", { id: view.accountId }));
  if (view.lines.length > 0) {
    lines.push("", l10n.getString("finish-report-text-lines"));
    for (const l of view.lines) {
      const name = l.name ?? l10n.getString(l.isApp ? "finish-report-text-unnamed-app" : "finish-report-text-unnamed-file");
      const why = l.reason ? ` ${l10n.getString(`reason-${l.reason}`)}` : "";
      lines.push(`- ${name}: ${l10n.getString(`finish-standing-${l.standing}`)}.${why}`);
    }
  }
  if (view.othersNotFinished > 0) lines.push("", l10n.getString("finish-report-text-others", { count: view.othersNotFinished.toLocaleString() }));
  if (view.goodbye !== null) lines.push("", l10n.getString("finish-report-text-goodbye", { total: view.goodbye[0].toLocaleString(), open: view.goodbye[1].toLocaleString() }));
  return lines.join("\n");
}

const NONE: Disclosure = { fileNames: false, appNames: false, accountId: false };

/**
 * R11 Share the report: choices that are all off until the person turns them on, the exact text
 * that would be saved, what was kept out, and a save of the text they read. Nothing is sent.
 */
export default function FinishReport({ id, onBack }: { id: string | null; onBack: () => void }) {
  const { l10n } = useLocalization();
  const [disc, setDisc] = useState<Disclosure>(NONE);
  const toggle = (key: keyof Disclosure) => (on: boolean) => {
    setDisc((d) => ({ ...d, [key]: on }));
  };
  return (
    <FinishPage titleId="finish-report-title" onBack={onBack}>
      <p className="subhead">{l10n.getString("finish-report-body")}</p>
      <Section title={l10n.getString("finish-report-choices")}>
        <Checkbox checked={disc.fileNames} onChange={toggle("fileNames")} label={l10n.getString("finish-report-file-names")} />
        <Checkbox checked={disc.appNames} onChange={toggle("appNames")} label={l10n.getString("finish-report-app-names")} />
        <Checkbox checked={disc.accountId} onChange={toggle("accountId")} label={l10n.getString("finish-report-account-id")} />
      </Section>
      <Preview key={`${String(disc.fileNames)}-${String(disc.appNames)}-${String(disc.accountId)}`} id={id} disc={disc} />
      <p className="kit-hint">{l10n.getString("finish-report-feedback")}</p>
    </FinishPage>
  );
}

function Preview({ id, disc }: { id: string | null; disc: Disclosure }) {
  const { l10n } = useLocalization();
  const { state, reload } = useLoad(() => reportPreview(id, disc));
  const [saved, setSaved] = useState<{ path: string | null } | null>(null);
  const [problem, setProblem] = useState<FinishProblem | null>(null);
  return (
    <Loaded state={state} onRetry={reload}>
      {(view) => {
        const text = reportText(l10n, view);
        return (
          <Section title={l10n.getString("finish-report-preview")}>
            <pre className="fin-text" tabIndex={0} dir="auto" aria-label={l10n.getString("finish-report-preview")}>
              {text}
            </pre>
            <p className="kit-hint">
              {l10n.getString("finish-report-withheld", { names: view.withheld.names.toLocaleString(), secrets: view.withheld.secrets.toLocaleString(), ids: view.withheld.accountIds.toLocaleString() })}
            </p>
            {problem && <ProblemNotice problem={problem} onRetry={problem.kind === "changed" ? () => { setProblem(null); reload(); } : undefined} />}
            <div role="status">
              {saved && (saved.path === null ? <p>{l10n.getString("finish-report-not-saved")}</p> : <Banner tone="ready">{l10n.getString("finish-report-saved", { path: saved.path })}</Banner>)}
            </div>
            <div className="fin-actions">
              <Button
                variant="primary"
                onClick={() => {
                  reportSave(id, disc, view.digest, text).then(
                    (path) => {
                      setProblem(null);
                      setSaved({ path });
                    },
                    (error: unknown) => {
                      setProblem(toFinishProblem(error));
                    },
                  );
                }}
              >
                {l10n.getString("finish-report-save")}
              </Button>
            </div>
          </Section>
        );
      }}
    </Loaded>
  );
}
