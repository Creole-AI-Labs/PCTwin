/**
 * "Fix these N items" (Task List 2.9 item 4; mockups R15, X08): only the items that did not finish
 * are tried again, and only the ones the person chooses. What already worked is kept and said so.
 * Each item says what it waits for (the old laptop, the internet, the administrator prompt, ...) or
 * that it can be tried now; the engine checks that again before it runs. The list's `generation`
 * goes back exactly as received.
 */
import { useLocalization } from "@fluent/react";
import { useCallback, useEffect, useState } from "react";
import { repairOffer, repairRun, toRepairProblem } from "../bridge";
import { Badge, Banner, Button, Checkbox, Row, RowList } from "../components/kit";
import type { RepairOffer, RepairProblem } from "../deliver";
import { Attention } from "../icons";
import { TopStrip } from "./FrameParts";
import FrameFooter from "./FrameFooter";
import { Loading, MoreLine } from "./FinishParts";
import { TitleText } from "./FullMoveParts";
import { Heading } from "./ProgressParts";

/** The lines that can be tried now: ticked to begin with. */
const readyLines = (offer: RepairOffer): number[] => offer.lines.filter((l) => l.waitsFor.length === 0).map((l) => l.line);

export default function RepairItems({ onBack }: { onBack: () => void }) {
  const { l10n } = useLocalization();
  const [offer, setOffer] = useState<RepairOffer | null>(null);
  const [chosen, setChosen] = useState<number[]>([]);
  const [problem, setProblem] = useState<RepairProblem | null>(null);
  const [fixed, setFixed] = useState<number | null>(null);
  const [busy, setBusy] = useState(false);
  const [failed, setFailed] = useState(false);

  const show = useCallback((next: RepairOffer) => {
    setOffer(next);
    setChosen(readyLines(next));
  }, []);
  const load = useCallback(() => {
    repairOffer().then(
      (next) => {
        setFailed(false);
        show(next);
      },
      () => {
        setFailed(true);
      },
    );
  }, [show]);
  useEffect(load, [load]);

  const run = (current: RepairOffer) => {
    setBusy(true);
    repairRun(current.generation, chosen).then(
      (done) => {
        setProblem(null);
        setFixed(done.fixed);
        show(done);
        setBusy(false);
      },
      (error: unknown) => {
        const p = toRepairProblem(error);
        setProblem(p);
        setBusy(false);
        // A list that changed is looked at again, and nothing the person had not seen is tried.
        if (p === "stale") load();
      },
    );
  };

  const total = offer ? offer.lines.length + offer.more : 0;
  return (
    <>
      <TopStrip onBack={onBack} labelId="repair-strip" />
      {offer ? <Heading id="repair-title" vars={{ count: total }} /> : <Heading id="repair-title-loading" />}
      {failed && (
        <div className="mv-note">
          <Banner tone="attention">{l10n.getString("repair-problem-failed")}</Banner>
          <div className="mv-actions">
            <Button variant="secondary" onClick={load}>
              {l10n.getString("finish-retry")}
            </Button>
          </div>
        </div>
      )}
      {offer === null && !failed && <Loading />}
      <div role="status">
        {problem && (
          <div className="mv-note">
            <Banner tone="attention">{l10n.getString(`repair-problem-${problem}`)}</Banner>
          </div>
        )}
        {fixed !== null && (
          <div className="mv-note">
            <Banner tone={fixed > 0 ? "ready" : "info"}>{l10n.getString("repair-fixed", { count: fixed })}</Banner>
          </div>
        )}
      </div>
      {offer && (
        <>
          <p className="subhead">{l10n.getString("repair-body", { kept: offer.kept })}</p>
          {offer.lines.length === 0 ? (
            <p>{l10n.getString("repair-nothing")}</p>
          ) : (
            <RowList>
              {offer.lines.map((l) => {
                const detail = [
                  l10n.getString(`status-${l.status}`),
                  l.reason ? l10n.getString(`reason-${l.reason}`) : "",
                  l.waitsFor.length === 0 ? l10n.getString("repair-can-now") : l.waitsFor.map((w) => l10n.getString(`repair-waits-${w}`)).join(". "),
                ]
                  .filter(Boolean)
                  .join(". ");
                return (
                  <Row
                    key={l.line}
                    title={
                      <Checkbox
                        checked={chosen.includes(l.line)}
                        onChange={(on) => {
                          setChosen((c) => (on ? [...c, l.line] : c.filter((x) => x !== l.line)));
                        }}
                        label={<TitleText title={l.title} />}
                        hint={detail}
                      />
                    }
                    end={l.waitsFor.length > 0 ? <Badge tone="warn" icon={<Attention className="mv-icon" />}>{l10n.getString("repair-waiting")}</Badge> : undefined}
                  />
                );
              })}
            </RowList>
          )}
          <MoreLine count={offer.more} />
          {offer.lines.length > 0 && (
            <FrameFooter>
              <Button
                variant="primary"
                disabled={busy || chosen.length === 0}
                onClick={() => {
                  run(offer);
                }}
              >
                {l10n.getString("repair-run", { count: chosen.length })}
              </Button>
            </FrameFooter>
          )}
        </>
      )}
    </>
  );
}
