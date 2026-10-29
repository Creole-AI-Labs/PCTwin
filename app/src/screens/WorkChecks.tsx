/**
 * "Can I actually work?" (Task List 2.9 item 2; Product Spec R17): the files, fonts and jobs the
 * person needs on this laptop, each with its latest result. PCTwin reads what it can (a project's
 * linked files, a font, an archive, a database) and never opens or runs anything. A job the person
 * does themselves is answered by the person.
 *
 * Every result always carries its label in words: checked by PCTwin, checked by the person, or
 * still needs a look. A person's yes is never shown as a PCTwin check. The list's `generation` goes
 * back exactly as received; a `stale` answer reloads the list.
 */
import { useLocalization } from "@fluent/react";
import { Fragment, useCallback, useEffect, useState } from "react";
import { toWorkProblem, workCheckAdd, workCheckAnswer, workCheckRemove, workCheckRun, workChecks } from "../bridge";
import { Badge, Banner, Button, Row, RowList, Section, TextField } from "../components/kit";
import type { WorkCheck, WorkChecks as WorkList, WorkLabel, WorkProblem } from "../deliver";
import { Attention, Check } from "../icons";
import { shownName } from "../names";
import { TopStrip } from "./FrameParts";
import { Loading, MoreLine } from "./FinishParts";
import { Heading } from "./ProgressParts";

/** Names from the laptop as text, one after the other. */
function Names({ names }: { names: string[] }) {
  return (
    <>
      {names.map((n, i) => (
        <Fragment key={i}>
          {i > 0 && ", "}
          <bdi dir="auto">{shownName(n)}</bdi>
        </Fragment>
      ))}
    </>
  );
}

export function LabelBadge({ label }: { label: WorkLabel }) {
  const { l10n } = useLocalization();
  return (
    <Badge tone={label === "still-needs-attention" ? "warn" : "good"} icon={label === "still-needs-attention" ? <Attention className="mv-icon" /> : <Check className="mv-icon" />}>
      {l10n.getString(`work-label-${label}`)}
    </Badge>
  );
}

function CheckRow({ check, busy, onAnswer, onRemove }: { check: WorkCheck; busy: boolean; onAnswer: (worked: boolean) => void; onRemove: () => void }) {
  const { l10n } = useLocalization();
  const name = shownName(check.name);
  const detail = (
    <>
      {l10n.getString(`work-what-${check.what}`)}. {l10n.getString(`work-state-${check.state}`)}.
      {check.of > 0 && <> {l10n.getString("work-looked-at", { checked: check.checked, of: check.of })}</>}
      {check.missing.length > 0 && (
        <>
          {" "}
          {l10n.getString("work-missing")} <Names names={check.missing} />
          {check.missingMore > 0 && <> {l10n.getString("work-and-more", { count: check.missingMore })}</>}
        </>
      )}
      {check.notChecked.length > 0 && (
        <>
          {" "}
          {l10n.getString("work-not-looked-at")} <Names names={check.notChecked} />
        </>
      )}
    </>
  );
  return (
    <Row
      title={<bdi dir="auto">{name}</bdi>}
      detail={detail}
      end={
        <>
          <LabelBadge label={check.label} />
          {check.what === "task" && (
            <>
              <Button variant="quiet" disabled={busy} onClick={() => { onAnswer(true); }} aria-label={l10n.getString("work-answer-worked-name", { name })}>
                {l10n.getString("work-answer-worked")}
              </Button>
              <Button variant="quiet" disabled={busy} onClick={() => { onAnswer(false); }} aria-label={l10n.getString("work-answer-not-name", { name })}>
                {l10n.getString("work-answer-not")}
              </Button>
            </>
          )}
          <Button variant="quiet" disabled={busy} onClick={onRemove} aria-label={l10n.getString("work-remove-name", { name })}>
            {l10n.getString("work-remove")}
          </Button>
        </>
      }
    />
  );
}

export default function WorkChecks({ onBack }: { onBack: () => void }) {
  const { l10n } = useLocalization();
  const [list, setList] = useState<WorkList | null>(null);
  const [problem, setProblem] = useState<WorkProblem | null>(null);
  const [busy, setBusy] = useState(false);
  const [font, setFont] = useState("");
  const [task, setTask] = useState("");
  const [failed, setFailed] = useState(false);

  const load = useCallback(() => {
    workChecks().then(
      (next) => {
        setFailed(false);
        setList(next);
      },
      () => {
        setFailed(true);
      },
    );
  }, []);
  useEffect(load, [load]);

  /** One change: the list it answers with replaces the one shown; a refusal says why, and a stale one looks again. */
  const apply = (call: Promise<WorkList>, done?: () => void) => {
    setBusy(true);
    call.then(
      (next) => {
        setList(next);
        setProblem(null);
        done?.();
        setBusy(false);
      },
      (error: unknown) => {
        const p = toWorkProblem(error);
        setProblem(p);
        setBusy(false);
        if (p === "stale") load();
      },
    );
  };

  return (
    <>
      <TopStrip onBack={onBack} labelId="work-strip" />
      <Heading id="work-title" />
      <p className="subhead">{l10n.getString("work-body")}</p>
      <div role="status">
        {problem && (
          <div className="mv-note">
            <Banner tone="attention">{l10n.getString(`work-problem-${problem}`)}</Banner>
          </div>
        )}
      </div>
      {failed && (
        <div className="mv-note">
          <Banner tone="attention">{l10n.getString("work-problem-failed")}</Banner>
          <div className="mv-actions">
            <Button variant="secondary" onClick={load}>
              {l10n.getString("finish-retry")}
            </Button>
          </div>
        </div>
      )}
      {list === null && !failed && <Loading />}
      {list && (
        <>
          {list.checks.length === 0 ? (
            <p role="status">{l10n.getString("work-empty")}</p>
          ) : (
            <RowList>
              {list.checks.map((c) => (
                <CheckRow
                  key={c.index}
                  check={c}
                  busy={busy}
                  onAnswer={(worked) => {
                    apply(workCheckAnswer(c.index, list.generation, worked));
                  }}
                  onRemove={() => {
                    apply(workCheckRemove(c.index, list.generation));
                  }}
                />
              ))}
            </RowList>
          )}
          <MoreLine count={list.more} />
          <div className="mv-actions">
            <Button
              variant="primary"
              disabled={busy || list.checks.length === 0}
              onClick={() => {
                apply(workCheckRun());
              }}
            >
              {l10n.getString("work-run")}
            </Button>
          </div>
          <Section title={l10n.getString("work-add-title")}>
            <div className="mv-actions">
              <Button
                variant="secondary"
                disabled={busy}
                onClick={() => {
                  apply(workCheckAdd({ kind: "file" }));
                }}
              >
                {l10n.getString("work-add-file")}
              </Button>
            </div>
            <p className="kit-hint">{l10n.getString("work-add-file-hint")}</p>
            <form
              onSubmit={(e) => {
                e.preventDefault();
                apply(workCheckAdd({ kind: "font", family: font.trim() }), () => {
                  setFont("");
                });
              }}
            >
              <TextField
                label={l10n.getString("work-font-label")}
                value={font}
                onChange={(e) => {
                  setFont(e.target.value);
                }}
                maxLength={200}
              />
              <div className="mv-actions">
                <Button type="submit" variant="secondary" disabled={busy || font.trim() === ""}>
                  {l10n.getString("work-add-font")}
                </Button>
              </div>
            </form>
            <form
              onSubmit={(e) => {
                e.preventDefault();
                apply(workCheckAdd({ kind: "task", task: task.trim() }), () => {
                  setTask("");
                });
              }}
            >
              <TextField
                label={l10n.getString("work-task-label")}
                hint={l10n.getString("work-task-hint")}
                value={task}
                onChange={(e) => {
                  setTask(e.target.value);
                }}
                maxLength={200}
              />
              <div className="mv-actions">
                <Button type="submit" variant="secondary" disabled={busy || task.trim() === ""}>
                  {l10n.getString("work-add-task")}
                </Button>
              </div>
            </form>
          </Section>
        </>
      )}
    </>
  );
}
