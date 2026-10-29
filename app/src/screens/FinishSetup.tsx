import { useLocalization } from "@fluent/react";
import { useState } from "react";
import { setupChecklist, setupMark, setupSave, toFinishProblem } from "../bridge";
import { Badge, Banner, Button, Checkbox, Row, RowList, Section } from "../components/kit";
import type { FinishProblem, SavedSetup } from "../finish";
import { Check } from "../icons";
import { FinishPage, Loaded, Name, ProblemNotice, useLoad } from "./FinishParts";
import FrameFooter from "./FrameFooter";

/**
 * R03 Finish setting up: the checklist of steps only the person can do. The person's tick ("You
 * said it is done") is kept apart from PCTwin's own check ("PCTwin checked"): a tick never turns into a check.
 */
export default function FinishSetup({ id, onBack, onSave }: { id: string | null; onBack: () => void; onSave: () => void }) {
  const { l10n } = useLocalization();
  const { state, set, reload } = useLoad(() => setupChecklist(id));
  const [problem, setProblem] = useState<FinishProblem | null>(null);
  const mark = (task: string, done: boolean) => {
    setupMark(id, task, done).then(
      (next) => {
        setProblem(null);
        set(next);
      },
      (error: unknown) => {
        setProblem(toFinishProblem(error));
      },
    );
  };
  return (
    <FinishPage titleId="finish-setup-title" onBack={onBack}>
      <p className="subhead">{l10n.getString("finish-setup-body")}</p>
      <Loaded state={state} onRetry={reload}>
        {(view) => (
          <>
            {problem && <ProblemNotice problem={problem} />}
            {view.tasks.length === 0 ? (
              <p role="status">{l10n.getString("finish-setup-empty")}</p>
            ) : (
              <RowList>
                {view.tasks.map((t) => {
                  const text = l10n.getString(`finish-setup-${t.what}`, { name: t.name });
                  return t.mark === "machine-checked" ? (
                    <Row
                      key={t.id}
                      icon={<Check className="mv-icon" />}
                      title={<Name>{text}</Name>}
                      end={<Badge tone="good">{l10n.getString("finish-setup-mark-machine-checked")}</Badge>}
                    />
                  ) : (
                    <Row
                      key={t.id}
                      title={
                        <Checkbox
                          checked={t.mark === "you-did-it"}
                          onChange={(done) => {
                            mark(t.id, done);
                          }}
                          label={<Name>{text}</Name>}
                          hint={l10n.getString(`finish-setup-mark-${t.mark}`)}
                        />
                      }
                    />
                  );
                })}
              </RowList>
            )}
            <Section title={l10n.getString("finish-setup-save-row")}>
              <RowList>
                <Row
                  title={l10n.getString("finish-setup-save-row")}
                  detail={l10n.getString("finish-setup-save-row-detail")}
                  end={
                    <Button variant="quiet" onClick={onSave} aria-label={l10n.getString("finish-open-name", { title: l10n.getString("finish-setup-save-row") })}>
                      {l10n.getString("finish-open")}
                    </Button>
                  }
                />
              </RowList>
            </Section>
          </>
        )}
      </Loaded>
    </FinishPage>
  );
}

/** R04 Save your app list and setup: local only, kept in My PCTwin history; a list of apps and settings, not a copy of files. */
export function FinishSave({ id, onBack }: { id: string | null; onBack: () => void }) {
  const { l10n } = useLocalization();
  const { state, reload } = useLoad(() => setupChecklist(id));
  const [saved, setSaved] = useState<SavedSetup | null>(null);
  const [problem, setProblem] = useState<FinishProblem | null>(null);
  return (
    <FinishPage titleId="finish-save-title" onBack={onBack}>
      <p className="subhead">{l10n.getString("finish-save-body")}</p>
      {problem && <ProblemNotice problem={problem} />}
      <div role="status">
        {saved && (
          <Banner tone="ready">{l10n.getString("finish-save-saved", { apps: saved.apps.toLocaleString(), settings: saved.settings.toLocaleString() })}</Banner>
        )}
      </div>
      <Loaded state={state} onRetry={reload}>
        {(view) =>
          view.saved.length > 0 ? (
            <Section title={l10n.getString("finish-save-earlier-title")}>
              <RowList>
                {view.saved.map((s) => (
                  <Row key={s.historyId} title={l10n.getString("finish-save-earlier", { when: new Date(s.when).toLocaleString(document.documentElement.lang || "en") })} />
                ))}
              </RowList>
            </Section>
          ) : null
        }
      </Loaded>
      <FrameFooter>
        <Button
          variant="primary"
          onClick={() => {
            setupSave(id).then(
              (r) => {
                setProblem(null);
                setSaved(r);
              },
              (error: unknown) => {
                setProblem(toFinishProblem(error));
              },
            );
          }}
        >
          {l10n.getString("finish-save-button")}
        </Button>
      </FrameFooter>
    </FinishPage>
  );
}
