import { useLocalization } from "@fluent/react";
import { useEffect, useState } from "react";
import { catchupApprove, catchupArm, listenFinishDelta, toFinishProblem } from "../bridge";
import { Banner, Button, Checkbox, Row, RowList, Select } from "../components/kit";
import { CHOICES_FOR, type CatchupChoice, type CatchupPurpose, type ChoiceIn, type DeltaView, type FinishProblem } from "../finish";
import { FinishPage, MoreLine, Name, ProblemNotice } from "./FinishParts";

type Stage = { kind: "choose" } | { kind: "armed"; purpose: CatchupPurpose } | { kind: "delta"; delta: DeltaView } | { kind: "unreadable" } | { kind: "sent" };

const PURPOSES: readonly CatchupPurpose[] = ["check", "catch-up"];

/**
 * R13 Bring me up to date: look at the old laptop again, see what changed since the move by item,
 * choose for each changed item what the engine allows, and send only the reviewed changes. Nothing
 * is chosen for the person; other people's changes are only counted.
 */
export default function FinishCatchup({ id, onBack, onPair, delta }: { id: string | null; onBack: () => void; onPair?: () => void; /** The list already arrived (the new laptop paired for it): shown at once. `null` is a list that could not be read. */ delta?: DeltaView | null }) {
  const { l10n } = useLocalization();
  const [stage, setStage] = useState<Stage>(() => (delta === undefined ? { kind: "choose" } : delta === null ? { kind: "unreadable" } : { kind: "delta", delta }));
  const [problem, setProblem] = useState<FinishProblem | null>(null);
  useEffect(() => {
    let live = true;
    let stop: (() => void) | null = null;
    void listenFinishDelta((delta) => {
      if (live) setStage(delta === null ? { kind: "unreadable" } : { kind: "delta", delta });
    }).then((u) => {
      if (live) stop = u;
      else u();
    });
    return () => {
      live = false;
      stop?.();
    };
  }, []);
  const arm = (purpose: CatchupPurpose) => {
    catchupArm(id, purpose).then(
      () => {
        setProblem(null);
        setStage({ kind: "armed", purpose });
      },
      (error: unknown) => {
        setProblem(toFinishProblem(error));
      },
    );
  };
  return (
    <FinishPage titleId="finish-catchup-title" onBack={onBack}>
      <p className="subhead">{l10n.getString("finish-catchup-body")}</p>
      {problem && <ProblemNotice problem={problem} />}
      {stage.kind === "choose" && (
        <RowList>
          {PURPOSES.map((p) => {
            const title = l10n.getString(`finish-catchup-purpose-${p}`);
            return (
              <Row
                key={p}
                title={title}
                detail={l10n.getString(`finish-catchup-purpose-${p}-detail`)}
                end={
                  <Button
                    variant="secondary"
                    aria-label={l10n.getString("finish-open-name", { title })}
                    onClick={() => {
                      arm(p);
                    }}
                  >
                    {l10n.getString("finish-open")}
                  </Button>
                }
              />
            );
          })}
        </RowList>
      )}
      <div role="status">
        {stage.kind === "armed" && (
          <>
            <Banner tone="info">{l10n.getString("finish-catchup-armed")}</Banner>
            <p>{l10n.getString("finish-catchup-waiting")}</p>
          </>
        )}
        {stage.kind === "sent" && <Banner tone="ready">{l10n.getString("finish-catchup-sent")}</Banner>}
      </div>
      {stage.kind === "armed" && onPair && (
        <div className="fin-actions">
          <Button variant="primary" onClick={onPair}>
            {l10n.getString("finish-catchup-connect")}
          </Button>
        </div>
      )}
      {stage.kind === "unreadable" && <Banner tone="attention">{l10n.getString("finish-catchup-unreadable")}</Banner>}
      {stage.kind === "delta" && (
        <Changes
          delta={stage.delta}
          onApprove={(choices) => {
            catchupApprove(id, stage.delta.digest, choices).then(
              () => {
                setProblem(null);
                setStage({ kind: "sent" });
              },
              (error: unknown) => {
                const p = toFinishProblem(error);
                setProblem(p);
                // A changed list is looked at again: the person starts over from the choice.
                if (p.kind === "changed") setStage({ kind: "choose" });
              },
            );
          }}
        />
      )}
    </FinishPage>
  );
}

function Changes({ delta, onApprove }: { delta: DeltaView; onApprove: (choices: ChoiceIn[]) => void }) {
  const { l10n } = useLocalization();
  const [picked, setPicked] = useState<Partial<Record<number, CatchupChoice>>>({});
  const chosen: ChoiceIn[] = delta.lines.flatMap((l, i) => {
    const choice = picked[i];
    return choice ? [{ id: l.id, choice }] : [];
  });
  const check = delta.purpose === "check";
  const put = (i: number, choice: CatchupChoice | null) => {
    setPicked((p) => {
      const next = { ...p, [i]: choice ?? undefined };
      return next;
    });
  };
  return (
    <>
      <h2 className="fin-block">{l10n.getString("finish-catchup-changes-title")}</h2>
      {check && <p>{l10n.getString("finish-catchup-check-only")}</p>}
      {delta.lines.length === 0 && <p>{l10n.getString("finish-catchup-none")}</p>}
      <RowList>
        {delta.lines.map((l, i) => {
          const allowed = CHOICES_FOR[l.change];
          const name = <Name>{l.label}</Name>;
          const detail = l10n.getString(`finish-change-${l.change}`);
          if (check) return <Row key={i} title={name} detail={detail} />;
          if (allowed.length === 0) return <Row key={i} title={name} detail={`${detail}. ${l10n.getString("finish-catchup-nothing-to-send")}`} />;
          if (allowed.length === 1 && !l.needsChoice) {
            return (
              <Row
                key={i}
                title={
                  <Checkbox
                    checked={picked[i] === "send"}
                    onChange={(on) => {
                      put(i, on ? "send" : null);
                    }}
                    label={l10n.getString("finish-catchup-send-label", { title: l.label })}
                    hint={detail}
                  />
                }
              />
            );
          }
          return (
            <Row
              key={i}
              title={
                <Select<CatchupChoice | "none">
                  label={l10n.getString("finish-catchup-choose-label", { title: l.label })}
                  hint={detail}
                  value={picked[i] ?? "none"}
                  options={[{ value: "none", label: l10n.getString("finish-choice-none") }, ...allowed.map((c) => ({ value: c, label: l10n.getString(`finish-choice-${c}`) }))]}
                  onChange={(v) => {
                    put(i, v === "none" ? null : v);
                  }}
                />
              }
            />
          );
        })}
      </RowList>
      <MoreLine count={delta.more} />
      {delta.others > 0 && <p className="kit-hint">{l10n.getString("finish-catchup-others", { count: delta.others.toLocaleString() })}</p>}
      {!check && (
        <div className="fin-actions">
          <Button
            variant="primary"
            disabled={chosen.length === 0}
            onClick={() => {
              onApprove(chosen);
            }}
          >
            {l10n.getString("finish-catchup-approve", { count: chosen.length.toLocaleString() })}
          </Button>
        </div>
      )}
    </>
  );
}
