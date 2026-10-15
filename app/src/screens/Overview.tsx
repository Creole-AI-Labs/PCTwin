import { Localized, useLocalization } from "@fluent/react";
import { useId, type ReactNode } from "react";
import { Badge, Banner, Button, Row, RowList } from "../components/kit";
import { ArrowForward } from "../icons";
import { attentionCount, type OverviewRow, type OverviewView } from "../plan";
import FrameFooter from "./FrameFooter";
import { TopStrip } from "./FrameParts";
import { Heading } from "./PairParts";
import { AttentionMark, DriveIcon, FolderIcon, PeopleIcon, useFormat } from "./PlanParts";

export interface OverviewProps {
  view: OverviewView;
  /** Open the screen behind a row (file selection, destinations, people). */
  onOpen: (row: OverviewRow) => void;
  onContinue: () => void;
  /** Ask the system for permission to read the places the search could not open. */
  onAllowAccess?: () => void;
  onBack?: () => void;
}

/**
 * M03 Everything you chose, in view. As the mockup: three totals in cards, the rows that open each part in one
 * box, the partial-scan note, then the action. The mockup's second column (browser, games, settings) is a later
 * phase and is not drawn. For the wide window: the host sets it with `windowModeFor("overview")` when it shows
 * this screen; the screen never changes the window itself.
 */
export default function Overview({ view, onOpen, onContinue, onAllowAccess, onBack }: OverviewProps) {
  const { l10n } = useLocalization();
  const fmt = useFormat();
  const attention = attentionCount(view);
  const rows: { row: OverviewRow; icon: ReactNode; title: string; body: string; needs: boolean }[] = [
    {
      row: "files",
      icon: <FolderIcon />,
      title: l10n.getString("overview-files-title"),
      body: l10n.getString("overview-files-body", { count: view.chosen.folders }),
      needs: view.unreadable > 0,
    },
    { row: "destinations", icon: <DriveIcon />, title: l10n.getString("overview-dest-title"), body: l10n.getString("overview-dest-body"), needs: view.destinationProblems > 0 },
    { row: "people", icon: <PeopleIcon />, title: l10n.getString("overview-people-title"), body: l10n.getString("overview-people-body"), needs: view.peopleWaiting > 0 },
  ];

  return (
    <div className="plan-screen plan-wide">
      <TopStrip onBack={onBack} labelId="overview-eyebrow" />
      <Heading id="overview-title" />
      <Localized id="overview-body">
        <p className="subhead" />
      </Localized>

      <dl className="plan-metrics">
        <Metric value={fmt.total(view.chosen.bytes, view.chosen.atLeast)} label={l10n.getString("overview-size-label")} />
        <Metric value={fmt.number(view.chosen.files)} label={l10n.getString("overview-items-label", { count: view.chosen.files })} />
        <Metric value={fmt.number(attention)} label={l10n.getString("overview-attention-label", { count: attention })} />
      </dl>

      <div className="plan-box plan-box-rows">
        <RowList>
          {rows.map((r) => (
            <OverviewItem key={r.row} {...r} onOpen={() => { onOpen(r.row); }} />
          ))}
        </RowList>
      </div>

      {view.unreadable > 0 && (
        <div className="plan-notice">
          <Banner tone="attention">
            <p>{l10n.getString("overview-partial", { count: view.unreadable })}</p>
            {onAllowAccess && (
              <Button variant="quiet" onClick={onAllowAccess}>
                {l10n.getString("overview-allow")}
              </Button>
            )}
          </Banner>
        </div>
      )}

      <FrameFooter>
          <Button variant="primary" icon={<ArrowForward />} onClick={onContinue}>
            {l10n.getString("overview-continue")}
          </Button>
      </FrameFooter>
    </div>
  );
}

function Metric({ value, label }: { value: string; label: string }) {
  return (
    <div className="plan-metric">
      <dt>{label}</dt>
      <dd>{value}</dd>
    </div>
  );
}

function OverviewItem({ icon, title, body, needs, onOpen }: { icon: ReactNode; title: string; body: string; needs: boolean; onOpen: () => void }) {
  const { l10n } = useLocalization();
  const id = useId();
  return (
    <Row
      icon={<span className="plan-tile">{icon}</span>}
      title={<strong id={`${id}-title`}>{title}</strong>}
      detail={body}
      end={
        <>
          {needs && (
            <Badge tone="warn" icon={<AttentionMark />}>
              {l10n.getString("overview-row-attention")}
            </Badge>
          )}
          <Button id={`${id}-open`} variant="quiet" aria-labelledby={`${id}-open ${id}-title`} icon={<ArrowForward />} onClick={onOpen}>
            {l10n.getString("overview-review")}
          </Button>
        </>
      }
    />
  );
}
