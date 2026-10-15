import { useLocalization } from "@fluent/react";
import { Badge, Banner, Button, ResultChip, Row, RowList } from "../components/kit";
import FrameFooter from "./FrameFooter";
import { TopStrip } from "./FrameParts";
import type { MoveProgress } from "../move";
import { Heading, ProgressPanel } from "./ProgressParts";

/**
 * X05 Paused at a safe point (480 x 620). While the file being written finishes, it says so and the
 * move is not yet paused; once paused, it says what is kept and what waits. Resume works in both:
 * during "pausing" it withdraws the pause.
 */
export default function Paused({ progress, onResume }: { progress: MoveProgress; onResume: () => void }) {
  const { l10n } = useLocalization();
  const pausing = progress.state === "pausing";
  const { counts } = progress;
  return (
    <>
      <TopStrip labelId="progress-approved-job" />
      <Heading key={pausing ? "pausing" : "paused"} id={pausing ? "pause-title-finishing" : "pause-title"} />
      <p className="subhead">{l10n.getString("pause-body")}</p>
      <ProgressPanel
        title={l10n.getString(pausing ? "progress-panel-pausing" : "progress-title-paused")}
        progress={progress}
        note={pausing && progress.current ? l10n.getString("pause-finishing", { name: progress.current.name }) : undefined}
      />
      <RowList>
        <Row
          title={l10n.getString("pause-kept-title")}
          detail={l10n.getString("pause-kept-body", { count: counts.done })}
          end={<Badge>{l10n.getString("pause-status-kept")}</Badge>}
        />
        <Row
          title={l10n.getString("pause-waiting-title")}
          detail={l10n.getString("pause-waiting-body", { count: counts.waiting })}
          end={<Badge>{l10n.getString("pause-status-waiting")}</Badge>}
        />
        {counts.needsAction > 0 && <Row title={l10n.getString("progress-count-needs-action", { count: counts.needsAction })} end={<ResultChip result="needs-action" />} />}
      </RowList>
      <div className="mv-note">
        <Banner tone="info">{l10n.getString("pause-note")}</Banner>
      </div>
      <FrameFooter>
        <Button variant="primary" onClick={onResume}>
          {l10n.getString("pause-resume")}
        </Button>
      </FrameFooter>
    </>
  );
}
