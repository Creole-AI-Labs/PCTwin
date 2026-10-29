/**
 * PCTwin's own update or uninstall waits for a safe point (Task List 2.9 item 6). This page says what
 * it is waiting for, in words (moving files, setting up apps, app data, undoing a move), and that it
 * is safe once none of them is going on. There is no percentage: nothing here is measured. The
 * engine's `safe-stop` event refreshes it.
 */
import { useLocalization } from "@fluent/react";
import { useCallback, useEffect, useState } from "react";
import { listenSafeStop, safeStop } from "../bridge";
import { Banner, Button, Row, RowList } from "../components/kit";
import type { SafeStopView } from "../deliver";
import { Attention, Check } from "../icons";
import { Loading } from "./FinishParts";
import { TopStrip } from "./FrameParts";
import { Heading } from "./ProgressParts";

export default function SafeStop({ onBack, shown }: { onBack: () => void; /** A fixed view, for the screens gallery: nothing is asked of the engine. */ shown?: SafeStopView }) {
  const { l10n } = useLocalization();
  const [view, setView] = useState<SafeStopView | null>(shown ?? null);
  const [failed, setFailed] = useState(false);
  const look = useCallback(() => {
    if (shown) return;
    safeStop().then(
      (next) => {
        setFailed(false);
        setView(next);
      },
      () => {
        setFailed(true);
      },
    );
  }, [shown]);
  useEffect(look, [look]);
  useEffect(() => {
    if (shown) return;
    let off: (() => void) | undefined;
    let gone = false;
    void listenSafeStop((next) => {
      // A view that cannot be read is looked at again, never guessed.
      if (next) setView(next);
      else look();
    }).then((un) => {
      if (gone) un();
      else off = un;
    });
    return () => {
      gone = true;
      off?.();
    };
  }, [look, shown]);
  return (
    <>
      <TopStrip onBack={onBack} labelId="safe-stop-strip" />
      <Heading id="safe-stop-title" />
      <p className="subhead">{l10n.getString("safe-stop-body")}</p>
      {failed && (
        <div className="mv-note">
          <Banner tone="attention">{l10n.getString("safe-stop-failed")}</Banner>
        </div>
      )}
      {view === null && !failed && <Loading />}
      <div role="status">
        {view?.safe && (
          <div className="mv-note">
            <Banner tone="ready">{l10n.getString("safe-stop-ready")}</Banner>
          </div>
        )}
      </div>
      {view && !view.safe && (
        <>
          <p>{l10n.getString("safe-stop-waiting-for")}</p>
          <RowList>
            {view.busy.map((b) => (
              <Row key={b} icon={<Attention className="mv-icon" />} title={l10n.getString(`safe-stop-busy-${b}`)} />
            ))}
          </RowList>
        </>
      )}
      {view?.safe && (
        <RowList>
          <Row icon={<Check className="mv-icon" />} title={l10n.getString("safe-stop-nothing-going-on")} />
        </RowList>
      )}
      <div className="mv-actions">
        <Button variant="secondary" onClick={look}>
          {l10n.getString("safe-stop-check")}
        </Button>
      </div>
    </>
  );
}
