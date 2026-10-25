import { useLocalization } from "@fluent/react";
import { useState } from "react";
import { Badge, Banner, Button, Metrics, Row, RowList } from "../components/kit";
import type { Tidied } from "../bridge";
import { pageLocale } from "../move";
import FrameFooter from "./FrameFooter";
import { TopStrip } from "./FrameParts";
import { Heading } from "./ProgressParts";
import "./progress.css";

/**
 * X09 Pick up safely (480 x 620): a move on this laptop stopped part way (a crash, a restart, the
 * power). It says how many writes were left unfinished, then tidies them up when the person asks:
 * a finished copy is put in place, a part-copied file is kept to carry on, and nothing of the
 * person's is touched. Then pairing again carries on the move. Counts only: no file is named.
 */
export default function PickUp({
  unfinished,
  onTidy,
  onPairAgain,
  onLater,
}: {
  unfinished: number;
  /** Tidy up; resolves with what was done, or rejects when it could not. */
  onTidy: () => Promise<Tidied>;
  onPairAgain: () => void;
  onLater: () => void;
}) {
  const { l10n } = useLocalization();
  const [state, setState] = useState<{ kind: "ready" } | { kind: "tidying" } | { kind: "done"; tidied: Tidied } | { kind: "failed" }>({ kind: "ready" });
  const n = (v: number) => v.toLocaleString(pageLocale());

  const tidy = () => {
    setState({ kind: "tidying" });
    onTidy().then(
      (tidied) => {
        setState({ kind: "done", tidied });
      },
      () => {
        setState({ kind: "failed" });
      },
    );
  };

  return (
    <>
      <TopStrip labelId="pickup-eyebrow" />
      <Heading id="pickup-title" />
      <p className="subhead">{l10n.getString("pickup-body")}</p>
      {state.kind === "done" ? (
        <>
          <Metrics
            items={[
              { value: n(state.tidied.putInPlace), label: l10n.getString("pickup-put-in-place") },
              { value: n(state.tidied.keptToContinue), label: l10n.getString("pickup-kept") },
            ]}
          />
          {(state.tidied.notFinished > 0 || state.tidied.leftForLater > 0) && (
            <div className="mv-note">
              <Banner tone="attention">
                {state.tidied.notFinished > 0 && <p>{l10n.getString("pickup-not-finished", { count: state.tidied.notFinished })}</p>}
                {state.tidied.leftForLater > 0 && <p>{l10n.getString("pickup-left-for-later", { count: state.tidied.leftForLater })}</p>}
              </Banner>
            </div>
          )}
        </>
      ) : (
        <RowList>
          <Row
            title={l10n.getString("pickup-unfinished-title")}
            detail={l10n.getString("pickup-unfinished-body", { count: unfinished })}
            end={<Badge tone="warn">{l10n.getString("pickup-unfinished-badge")}</Badge>}
          />
          <Row title={l10n.getString("move-ended-old-title")} detail={l10n.getString("move-ended-old-body")} end={<Badge tone="good">{l10n.getString("move-ended-old-badge")}</Badge>} />
        </RowList>
      )}
      <div className="mv-note" role="status">
        {state.kind === "tidying" && <Banner tone="info">{l10n.getString("pickup-tidying")}</Banner>}
        {state.kind === "failed" && <Banner tone="attention">{l10n.getString("pickup-failed")}</Banner>}
        {state.kind === "done" && <Banner tone="info">{l10n.getString("pickup-done")}</Banner>}
      </div>
      <FrameFooter>
        <Button variant="quiet" onClick={onLater}>
          {l10n.getString("pickup-later")}
        </Button>
        {state.kind === "done" ? (
          <Button variant="primary" onClick={onPairAgain}>
            {l10n.getString("ended-pair-again")}
          </Button>
        ) : (
          <Button variant="primary" disabled={state.kind === "tidying"} onClick={tidy}>
            {l10n.getString(state.kind === "failed" ? "help-try-again" : "pickup-tidy")}
          </Button>
        )}
      </FrameFooter>
    </>
  );
}
