import { useLocalization } from "@fluent/react";
import { useState } from "react";
import { goodbyeAnswer, goodbyeKeepUntil, goodbyeList, toFinishProblem } from "../bridge";
import { Badge, Banner, Button, Row, RowList, Section } from "../components/kit";
import type { ExceptionLine, FinishProblem, GoodbyeAnswer, GoodbyeView } from "../finish";
import { Attention, Check } from "../icons";
import { listText } from "../move";
import { dateText, FinishPage, Loaded, MoreLine, Name, ProblemNotice, useLoad, whenText } from "./FinishParts";

const DAY = 86_400;

function isoDay(seconds: number): string {
  return new Date(seconds * 1000).toISOString().slice(0, 10);
}

function titleOf(l10n: ReturnType<typeof useLocalization>["l10n"], e: ExceptionLine): string {
  const special = e.key.startsWith("special-");
  return l10n.getString(`finish-goodbye-${e.key}`, special ? { names: listText(e.names) } : { count: e.count.toLocaleString() });
}

/**
 * R08 Before you leave the old laptop: what is still only on the old laptop, each answered "I did it"
 * or "I accept it is lost". Nothing is pre-answered, and the handover stays blocked, in words, until
 * every thing is answered and the old laptop has been checked since the move.
 */
export default function FinishGoodbye({ id, onBack, onHandover, onCheckAgain }: { id: string | null; onBack: () => void; onHandover: () => void; onCheckAgain: () => void }) {
  const { l10n } = useLocalization();
  const { state, set, reload } = useLoad(() => goodbyeList(id));
  const [problem, setProblem] = useState<FinishProblem | null>(null);
  const apply = (call: Promise<GoodbyeView>) => {
    call.then(
      (next) => {
        setProblem(null);
        set(next);
      },
      (error: unknown) => {
        const p = toFinishProblem(error);
        setProblem(p);
        // A changed list is looked at again, and the person answers again.
        if (p.kind === "changed") reload();
      },
    );
  };
  return (
    <FinishPage titleId="finish-goodbye-title" onBack={onBack}>
      <p className="subhead">{l10n.getString("finish-goodbye-body")}</p>
      {problem && <ProblemNotice problem={problem} />}
      <Loaded state={state} onRetry={reload}>
        {(view) => {
          const open = view.exceptions.filter((e) => e.resolution.kind === "open").length;
          const unchecked = view.oldLaptop.kind === "not-checked";
          return (
            <>
              {view.saved && <p>{l10n.getString("finish-goodbye-saved")}</p>}
              {view.exceptions.length > 0 && (
                <RowList>
                  {view.exceptions.map((e) => (
                    <Exception
                      key={e.id}
                      e={e}
                      onAnswer={(answer) => {
                        apply(goodbyeAnswer(id, e.id, e.shown, answer));
                      }}
                    />
                  ))}
                </RowList>
              )}
              <Section title={l10n.getString("finish-goodbye-keep-title")}>
                <Keep view={view} onSet={(at) => { apply(goodbyeKeepUntil(id, at)); }} />
              </Section>
              <div className="fin-block" role="status">
                {view.clear ? (
                  <Banner tone="ready">{l10n.getString("finish-goodbye-clear")}</Banner>
                ) : (
                  <Banner tone="info">
                    {unchecked ? l10n.getString("finish-goodbye-blocked-unchecked") : l10n.getString("finish-goodbye-blocked", { count: open })}
                  </Banner>
                )}
                {view.oldLaptop.kind === "checked" && <p className="kit-hint">{l10n.getString("finish-old-checked", { when: whenText(view.oldLaptop.at) })}</p>}
              </div>
              <div className="fin-actions">
                <Button variant="secondary" onClick={onCheckAgain}>
                  {l10n.getString("finish-goodbye-check-again")}
                </Button>
                <Button variant="primary" disabled={!view.clear} onClick={onHandover}>
                  {l10n.getString("finish-goodbye-handover")}
                </Button>
              </div>
            </>
          );
        }}
      </Loaded>
    </FinishPage>
  );
}

function Exception({ e, onAnswer }: { e: ExceptionLine; onAnswer: (a: GoodbyeAnswer) => void }) {
  const { l10n } = useLocalization();
  const title = titleOf(l10n, e);
  const answered = e.resolution.kind !== "open";
  return (
    <Row
      icon={answered ? <Check className="mv-icon" /> : <Attention className="mv-icon" />}
      title={title}
      detail={
        <>
          {e.items.length > 0 && (
            <ul className="fin-list">
              {e.items.map((n) => (
                <li key={n.id}>
                  <Name>{n.label}</Name>
                </li>
              ))}
            </ul>
          )}
          <MoreLine count={e.more} />
          {e.places.length > 0 && <p>{l10n.getString("finish-goodbye-places", { places: listText(e.places) })}</p>}
          <p>
            <Badge tone={answered ? "good" : "warn"}>{l10n.getString(`finish-goodbye-res-${e.resolution.kind}`)}</Badge>
          </p>
          {(e.canMarkDone || e.canAccept) && (
            <div className="fin-actions" role="group" aria-label={l10n.getString("finish-goodbye-answer-label", { title })}>
              {e.canMarkDone && (
                <Button variant="secondary" aria-pressed={e.resolution.kind === "person-did-it"} onClick={() => { onAnswer("did-it"); }}>
                  {l10n.getString("finish-goodbye-did-it")}
                </Button>
              )}
              {e.canAccept && (
                <Button variant="secondary" aria-pressed={e.resolution.kind === "accepted-loss"} onClick={() => { onAnswer("accept-loss"); }}>
                  {l10n.getString("finish-goodbye-accept")}
                </Button>
              )}
            </div>
          )}
          {e.canAccept && <p className="kit-hint">{l10n.getString("finish-goodbye-accept-warning")}</p>}
          {!e.canMarkDone && !e.canAccept && <p className="kit-hint">{l10n.getString("finish-goodbye-cannot-answer")}</p>}
        </>
      }
    />
  );
}

function Keep({ view, onSet }: { view: GoodbyeView; onSet: (at: number) => void }) {
  const { l10n } = useLocalization();
  const [value, setValue] = useState(isoDay(view.keepUntil ?? view.suggestedKeepUntil));
  const at = Date.parse(`${value}T00:00:00Z`) / 1000;
  return (
    <>
      <p>{l10n.getString("finish-goodbye-keep-body", { date: dateText(view.suggestedKeepUntil) })}</p>
      <p role="status">{view.keepUntil === null ? l10n.getString("finish-goodbye-keep-none") : l10n.getString("finish-goodbye-keep-set", { date: dateText(view.keepUntil) })}</p>
      <div className="fin-field">
        <label htmlFor="fin-keep">{l10n.getString("finish-goodbye-keep-label")}</label>
        <input
          id="fin-keep"
          type="date"
          value={value}
          min={isoDay(view.madeAt - DAY)}
          onChange={(e) => {
            setValue(e.target.value);
          }}
        />
      </div>
      <div className="fin-actions">
        <Button
          variant="secondary"
          onClick={() => {
            setValue(isoDay(view.suggestedKeepUntil));
            onSet(view.suggestedKeepUntil);
          }}
        >
          {l10n.getString("finish-goodbye-keep-use")}
        </Button>
        <Button variant="secondary" disabled={!Number.isFinite(at)} onClick={() => { onSet(at); }}>
          {l10n.getString("finish-goodbye-keep-save")}
        </Button>
      </div>
    </>
  );
}
