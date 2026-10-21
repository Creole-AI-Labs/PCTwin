import { useLocalization } from "@fluent/react";
import { Badge, Banner, Button, Row, RowList } from "../components/kit";
import type { GroupName, MoveProgress } from "../move";
import FrameFooter from "./FrameFooter";
import { TopStrip } from "./FrameParts";
import { Heading } from "./ProgressParts";
import "./progress.css";

/** A group as the person reads it: its folder's name, or the folder's own name. */
function placeName(l10n: ReturnType<typeof useLocalization>["l10n"], name: GroupName): string {
  return name.kind === "role" ? l10n.getString(`plan-role-${name.role}`) : name.name;
}

/**
 * X07 A destination problem (480 x 620): the drive one place lands on is not keeping files
 * correctly (copies checked there did not match, several times), so the move is held. It says what
 * is kept and what waits, and gives the precise fix: check the drive, then carry on, or stop. No file
 * the person chose is dropped to get past it.
 */
export default function DriveProblem({
  progress,
  place,
  onCarryOn,
  onStop,
}: {
  progress: MoveProgress;
  place: GroupName;
  onCarryOn: () => void;
  onStop: () => void;
}) {
  const { l10n } = useLocalization();
  const where = placeName(l10n, place);
  return (
    <>
      <TopStrip labelId="progress-approved-job" />
      <Heading id="drive-problem-title" />
      <p className="subhead">{l10n.getString("drive-problem-body", { place: where })}</p>
      <RowList>
        <Row
          title={l10n.getString("drive-problem-kept-title")}
          detail={l10n.getString("pause-kept-body", { count: progress.counts.done })}
          end={<Badge>{l10n.getString("move-status-kept")}</Badge>}
        />
        <Row
          title={l10n.getString("drive-problem-waiting-title", { place: where })}
          detail={l10n.getString("pause-waiting-body", { count: progress.counts.waiting })}
          end={<Badge tone="warn">{l10n.getString("drive-problem-badge")}</Badge>}
        />
      </RowList>
      <div className="mv-note">
        <Banner tone="attention">{l10n.getString("drive-problem-fix")}</Banner>
      </div>
      <FrameFooter>
        <Button variant="quiet" onClick={onStop}>
          {l10n.getString("move-stop")}
        </Button>
        <Button variant="primary" onClick={onCarryOn}>
          {l10n.getString("drive-problem-carry-on")}
        </Button>
      </FrameFooter>
    </>
  );
}
