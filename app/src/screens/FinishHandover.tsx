import { useLocalization } from "@fluent/react";
import { useState } from "react";
import { catchupArm, handoverConfirm, handoverPreview, handoverResult, toFinishProblem } from "../bridge";
import { Banner, Button, Checkbox, Row, RowList, Section, Select } from "../components/kit";
import { PERSON_SAYS, type FinishProblem, type HandoverAction, type HandoverRecorded, type HandoverStarted, type HandoverView, type PersonSays, type Route } from "../finish";
import { FinishPage, Loaded, Name, ProblemNotice, useHas, useLoad, whenText } from "./FinishParts";
import { listText } from "../move";

type Ready = Extract<HandoverView, { kind: "ready" }>;

type Page =
  | { kind: "choose" }
  | { kind: "preview"; action: HandoverAction }
  | { kind: "confirm"; action: HandoverAction; ready: Ready }
  | { kind: "started"; action: HandoverAction; started: HandoverStarted }
  | { kind: "recorded"; recorded: HandoverRecorded };

const SHORT: Record<HandoverAction, "remove" | "erase"> = { "remove-my-account": "remove", "erase-old-laptop": "erase" };
const ACTIONS: readonly HandoverAction[] = ["remove-my-account", "erase-old-laptop"];

/**
 * Hand the old laptop on (R08 to R10). Each request is asked separately; the preview shows exactly
 * what is removed and what stays and the system's own route; the irreversible confirmation is its
 * OWN page (an unticked box naming the exact target, then a separate button: no countdown, no typed
 * phrase); then the system's own tool does the work, and PCTwin records only what the person says
 * happened, never a certificate.
 */
export default function FinishHandover({ id, onBack, onList }: { id: string | null; onBack: () => void; onList: () => void }) {
  const [page, setPage] = useState<Page>({ kind: "choose" });
  switch (page.kind) {
    case "choose":
      return <Choose onBack={onBack} onPick={(action) => { setPage({ kind: "preview", action }); }} />;
    case "preview":
      return (
        <Preview
          id={id}
          action={page.action}
          onBack={() => { setPage({ kind: "choose" }); }}
          onList={onList}
          onContinue={(ready) => { setPage({ kind: "confirm", action: page.action, ready }); }}
        />
      );
    case "confirm":
      return (
        <Confirm
          id={id}
          ready={page.ready}
          onBack={() => { setPage({ kind: "preview", action: page.action }); }}
          onStarted={(started) => { setPage({ kind: "started", action: page.action, started }); }}
        />
      );
    case "started":
      return <Started id={id} action={page.action} started={page.started} onRecorded={(recorded) => { setPage({ kind: "recorded", recorded }); }} />;
    case "recorded":
      return <Recorded recorded={page.recorded} onDone={onBack} />;
  }
}

function Choose({ onBack, onPick }: { onBack: () => void; onPick: (a: HandoverAction) => void }) {
  const { l10n } = useLocalization();
  return (
    <FinishPage titleId="finish-handover-title" onBack={onBack}>
      <p className="subhead">{l10n.getString("finish-handover-body")}</p>
      <RowList>
        {ACTIONS.map((a) => {
          const title = l10n.getString(`finish-handover-${SHORT[a]}-title`);
          return (
            <Row
              key={a}
              title={title}
              detail={l10n.getString(`finish-handover-${SHORT[a]}-detail`)}
              end={
                <Button
                  variant="quiet"
                  aria-label={l10n.getString("finish-open-name", { title })}
                  onClick={() => {
                    onPick(a);
                  }}
                >
                  {l10n.getString("finish-open")}
                </Button>
              }
            />
          );
        })}
      </RowList>
    </FinishPage>
  );
}

/** The system's own route, as numbered steps in words. */
function RouteSteps({ route }: { route: Route }) {
  const { l10n } = useLocalization();
  const has = useHas();
  return (
    <>
      {route.system !== null && <p>{l10n.getString(`finish-route-system-${route.system}`)}</p>}
      <ol className="fin-list">
        {route.steps.map((s, i) => (
          <li key={`${s}-${String(i)}`}>{has(s) ? l10n.getString(s) : l10n.getString("finish-route-unknown-step")}</li>
        ))}
      </ol>
    </>
  );
}

/** The old laptop was not checked in the last 15 minutes: check it again, then look at the preview again. */
function CheckOldAgain({ id, onLookAgain }: { id: string | null; onLookAgain: () => void }) {
  const { l10n } = useLocalization();
  const [armed, setArmed] = useState(false);
  const [failed, setFailed] = useState(false);
  return (
    <div className="fin-actions">
      {armed ? (
        <>
          <p role="status">{l10n.getString("finish-handover-check-armed")}</p>
          <Button variant="primary" onClick={onLookAgain}>
            {l10n.getString("finish-handover-look-again")}
          </Button>
        </>
      ) : (
        <>
          {failed && <p role="status">{l10n.getString("finish-problem-failed")}</p>}
          <Button
            variant="primary"
            onClick={() => {
              catchupArm(id, "check").then(
                () => {
                  setArmed(true);
                },
                () => {
                  setFailed(true);
                },
              );
            }}
          >
            {l10n.getString("finish-handover-check-old")}
          </Button>
        </>
      )}
    </div>
  );
}

function Preview({ id, action, onBack, onList, onContinue }: { id: string | null; action: HandoverAction; onBack: () => void; onList: () => void; onContinue: (r: Ready) => void }) {
  const { l10n } = useLocalization();
  const { state, reload } = useLoad(() => handoverPreview(id, action));
  return (
    <FinishPage titleId={`finish-handover-preview-${SHORT[action]}`} onBack={onBack}>
      <Loaded state={state} onRetry={reload}>
        {(view) =>
          view.kind === "refused" ? (
            <>
              <div className="mv-note" role="status">
                <Banner tone="attention" title={l10n.getString("finish-handover-not-offered")}>
                  {l10n.getString(`finish-refused-${view.why}`)} {l10n.getString("finish-handover-refused-nothing-done")}
                </Banner>
              </div>
              {view.open > 0 && <p>{l10n.getString("finish-handover-refused-open", { count: view.open.toLocaleString() })}</p>}
              {view.why === "old-laptop-not-checked" && <CheckOldAgain id={id} onLookAgain={reload} />}
              <div className="fin-actions">
                <Button variant="secondary" onClick={onList}>
                  {l10n.getString("finish-handover-to-list")}
                </Button>
              </div>
            </>
          ) : (
            <>
              <Section title={l10n.getString("finish-handover-removes")}>
                <ul className="fin-list">
                  {view.removes.map((n, i) => (
                    <li key={`${n}-${String(i)}`}>
                      <Name>{n}</Name>
                    </li>
                  ))}
                </ul>
              </Section>
              <Section title={l10n.getString("finish-handover-keeps")}>
                {view.keeps.length === 0 ? (
                  <p>{l10n.getString("finish-handover-keeps-none")}</p>
                ) : (
                  <ul className="fin-list">
                    {view.keeps.map((n, i) => (
                      <li key={`${n}-${String(i)}`}>
                        <Name>{n}</Name>
                      </li>
                    ))}
                  </ul>
                )}
              </Section>
              <Section title={l10n.getString("finish-handover-route")}>
                <p>{l10n.getString("finish-handover-route-body")}</p>
                <RouteSteps route={view.route} />
              </Section>
              <Notes notes={view.notes} />
              <div className="fin-actions">
                <Button
                  variant="primary"
                  onClick={() => {
                    onContinue(view);
                  }}
                >
                  {l10n.getString("finish-handover-continue")}
                </Button>
              </div>
            </>
          )
        }
      </Loaded>
    </FinishPage>
  );
}

function Notes({ notes }: { notes: readonly string[] }) {
  const { l10n } = useLocalization();
  if (notes.length === 0) return null;
  return (
    <div className="mv-note">
      {notes.map((n) => (
        <Banner key={n} tone={n === "cannot-be-undone" || n === "closes-undo" ? "attention" : "info"}>
          {l10n.getString(`finish-hnote-${n}`)}
        </Banner>
      ))}
    </div>
  );
}

/** The irreversible step, on its own page: the exact target, what it means, an unticked box, then a separate button. */
function Confirm({ id, ready, onBack, onStarted }: { id: string | null; ready: Ready; onBack: () => void; onStarted: (s: HandoverStarted) => void }) {
  const { l10n } = useLocalization();
  const [ticked, setTicked] = useState(false);
  const [problem, setProblem] = useState<FinishProblem | null>(null);
  const [busy, setBusy] = useState(false);
  const target = listText(ready.removes);
  return (
    <FinishPage titleId={`finish-confirm-title-${ready.action}`} onBack={onBack}>
      <p className="subhead">{l10n.getString("finish-confirm-target", { target })}</p>
      <Section title={l10n.getString("finish-confirm-consequences")}>
        <Notes notes={ready.notes.filter((n) => n === "cannot-be-undone" || n === "closes-undo" || n === "ssd-not-secure-wipe" || n === "does-not-cancel-subscriptions")} />
      </Section>
      {problem && <ProblemNotice problem={problem} />}
      <div className="mv-confirm">
        <Checkbox
          checked={ticked}
          onChange={setTicked}
          label={l10n.getString(`finish-confirm-tick-${ready.action}`, { target })}
        />
      </div>
      {!ticked && <p className="kit-hint">{l10n.getString("finish-confirm-needs-tick")}</p>}
      <div className="fin-actions">
        <Button variant="secondary" onClick={onBack}>
          {l10n.getString("finish-confirm-cancel")}
        </Button>
        <Button
          variant="primary"
          className="mv-danger-fill"
          disabled={!ticked || busy}
          onClick={() => {
            setBusy(true);
            handoverConfirm(id, ready.action, ready.digest, ready.ticket).then(
              onStarted,
              (error: unknown) => {
                setBusy(false);
                setTicked(false);
                const p = toFinishProblem(error);
                setProblem(p);
                // A changed preview, or a confirmation the engine no longer takes (too soon, too late,
                // or already used), is looked at again: a fresh preview, and the person confirms again.
                if (p.kind === "changed" || p.kind === "not-allowed") onBack();
              },
            );
          }}
        >
          {l10n.getString("finish-confirm-go")}
        </Button>
      </div>
    </FinishPage>
  );
}

/** After the confirmation: the system's own steps (PCTwin draws no copy of a system prompt), then what the person saw. */
function Started({ id, action, started, onRecorded }: { id: string | null; action: HandoverAction; started: HandoverStarted; onRecorded: (r: HandoverRecorded) => void }) {
  const { l10n } = useLocalization();
  const [says, setSays] = useState<PersonSays | "">("");
  const [problem, setProblem] = useState<FinishProblem | null>(null);
  return (
    <FinishPage titleId="finish-started-title">
      <p className="subhead">{l10n.getString("finish-started-body")}</p>
      <div role="status">
        <p>{started.opened ? l10n.getString("finish-started-opened") : l10n.getString("finish-started-not-opened")}</p>
        {started.undoClosed && <p>{l10n.getString("finish-started-undo-closed")}</p>}
      </div>
      <RouteSteps route={started.route} />
      <h2 className="fin-block">{l10n.getString("finish-says-title")}</h2>
      {problem && <ProblemNotice problem={problem} />}
      <Select<PersonSays | "">
        label={l10n.getString("finish-says-label")}
        value={says}
        options={[{ value: "", label: l10n.getString("finish-says-choose") }, ...PERSON_SAYS.map((s) => ({ value: s, label: l10n.getString(`finish-says-${s}`) }))]}
        onChange={setSays}
      />
      <div className="fin-actions">
        <Button
          variant="primary"
          disabled={says === ""}
          onClick={() => {
            if (says === "") return;
            handoverResult(id, action, says).then(onRecorded, (error: unknown) => {
              setProblem(toFinishProblem(error));
            });
          }}
        >
          {l10n.getString("finish-says-record")}
        </Button>
      </div>
    </FinishPage>
  );
}

function Recorded({ recorded, onDone }: { recorded: HandoverRecorded; onDone: () => void }) {
  const { l10n } = useLocalization();
  return (
    <FinishPage titleId="finish-says-title" onBack={onDone}>
      <div role="status">
        <Banner tone="info">{l10n.getString(`finish-recorded-${recorded.outcome}`)}</Banner>
        <p className="kit-hint">{l10n.getString("finish-recorded-time", { when: whenText(recorded.at) })}</p>
      </div>
      <p>{l10n.getString("finish-hnote-not-a-certificate")}</p>
    </FinishPage>
  );
}
