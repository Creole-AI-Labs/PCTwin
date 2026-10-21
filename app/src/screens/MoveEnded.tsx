import { useLocalization } from "@fluent/react";
import { Badge, Banner, Button, Row, RowList } from "../components/kit";
import type { EndedEarly, MoveProblem } from "../bridge";
import FrameFooter from "./FrameFooter";
import { TopStrip } from "./FrameParts";
import { Heading } from "./ProgressParts";
import "./progress.css";

/** Why the move page is showing an end: the move stopped part way, or it could not start. */
export type MoveEnd = { kind: "ended"; why: EndedEarly } | { kind: "refused"; why: MoveProblem };

/** The words each end uses: a title, what happened and what to do, and whether anything may already be on the new laptop. */
function wordsFor(end: MoveEnd, old: boolean): { title: string; body: string; copiedMayStay: boolean } {
  if (end.kind === "refused") {
    return { title: "move-refused-title", body: `move-refused-${end.why.kind}`, copiedMayStay: false };
  }
  switch (end.why.kind) {
    case "link-lost":
      return { title: "move-lost-title", body: "move-lost-body", copiedMayStay: true };
    case "not-approved":
      return { title: "move-not-approved-title", body: old ? "move-not-approved-old" : "move-not-approved-new", copiedMayStay: false };
    case "failed":
      return { title: "move-failed-title", body: "move-failed-body", copiedMayStay: true };
  }
}

/**
 * The move's error pages, each in plain words: what happened, what is kept, and what to do next
 * (C09 when the connection is lost, C10's "what is kept", and the general error page). The old
 * laptop is never changed by a move, and the page says so. The engine's own words, when there are
 * any, are under "More details" only.
 */
export default function MoveEnded({ end, old, onPairAgain, onHome }: { end: MoveEnd; old: boolean; onPairAgain: () => void; onHome: () => void }) {
  const { l10n } = useLocalization();
  const words = wordsFor(end, old);
  const detail = end.kind === "ended" && "detail" in end.why ? end.why.detail : "";
  return (
    <>
      <TopStrip labelId="move-ended-eyebrow" />
      <Heading id={words.title} />
      <p className="subhead">{l10n.getString(words.body)}</p>
      <RowList>
        {words.copiedMayStay && (
          <Row
            title={l10n.getString("move-ended-kept-title")}
            detail={l10n.getString("move-ended-kept-body")}
            end={<Badge>{l10n.getString("move-status-kept")}</Badge>}
          />
        )}
        <Row title={l10n.getString("move-ended-old-title")} detail={l10n.getString("move-ended-old-body")} end={<Badge tone="good">{l10n.getString("move-ended-old-badge")}</Badge>} />
      </RowList>
      <div className="mv-note">
        <Banner tone="info">{l10n.getString("move-ended-next")}</Banner>
      </div>
      {detail && (
        <details className="mv-details">
          <summary>{l10n.getString("report-more-details")}</summary>
          <p dir="auto">{detail}</p>
        </details>
      )}
      <FrameFooter>
        <Button variant="quiet" onClick={onHome}>
          {l10n.getString("outcome-home")}
        </Button>
        <Button variant="primary" onClick={onPairAgain}>
          {l10n.getString("ended-pair-again")}
        </Button>
      </FrameFooter>
    </>
  );
}
