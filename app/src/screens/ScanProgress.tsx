import { Localized, useLocalization } from "@fluent/react";
import { Badge, Banner, Button, ProgressValue, Row, RowList } from "../components/kit";
import { ArrowForward } from "../icons";
import { isPartial, type ScanStep, type ScanView } from "../plan";
import FrameFooter from "./FrameFooter";
import { Steps, TopStrip, type StepItem } from "./FrameParts";
import { Heading } from "./PairParts";
import { AttentionMark, PathText, useFormat } from "./PlanParts";

export interface ScanProgressProps {
  scan: ScanView;
  onPause: () => void;
  onResume: () => void;
  /** The search is finished: go and see what was found. */
  onContinue: () => void;
  /** Ask the system for permission to read one place. Without it there is no button, only the honest list. */
  onAllowAccess?: (path: string) => void;
  onBack?: () => void;
}

/**
 * M02 Finding your files. Laid out as the mockup: the notice on top, a progress card (title, the measured
 * share of places searched, the bar, what was found), then the steps with their markers, then the action.
 * Measured counts only (files found, places searched, both read from what the scan has really done); places
 * it could not read stay on the list and are never counted as searched.
 * scanning -> pausing -> paused -> (scanning again, resumed) -> finished, with or without places left unread.
 */
export default function ScanProgress({ scan, onPause, onResume, onContinue, onAllowAccess, onBack }: ScanProgressProps) {
  const { l10n } = useLocalization();
  const fmt = useFormat();
  const partial = isPartial(scan);
  const finished = scan.status === "finished";
  const share = scan.places.total > 0 ? Math.min(scan.places.done, scan.places.total) / scan.places.total : 0;
  const notice =
    scan.status === "pausing" ? "scan-pausing" : scan.status === "paused" ? "scan-paused" : scan.status === "scanning" && scan.resumed ? "scan-resumed" : null;

  return (
    <div className="plan-screen">
      <TopStrip onBack={onBack} labelId="scan-eyebrow" />
      {/* A new state is a new heading, and the heading takes focus; steady progress inside one state does not. */}
      <Heading key={`${scan.status}-${String(scan.resumed)}`} id="scan-title" />
      <Localized id="scan-body">
        <p className="subhead" />
      </Localized>

      <div className="plan-notice" role="status">
        {notice && (
          <Banner tone="info">
            <Localized id={notice}>
              <p />
            </Localized>
          </Banner>
        )}
      </div>
      {partial && (
        <div className="plan-notice">
          <Banner tone="attention">
            <p>{l10n.getString("scan-partial", { count: scan.unreadable.length })}</p>
          </Banner>
        </div>
      )}

      <div className="plan-card">
        <div className="plan-card-head">
          <h2 className="kit-progress-title">{l10n.getString("scan-card-title")}</h2>
          <ProgressValue value={share} />
        </div>
        <Localized id="scan-progress" attrs={{ "aria-label": true }}>
          <div className="kit-progress" role="progressbar" aria-valuemin={0} aria-valuemax={100} aria-valuenow={Math.round(share * 100)}>
            <span className="kit-progress-fill" style={{ inlineSize: `${String(Math.round(share * 100))}%` }} />
          </div>
        </Localized>
        <p className="plan-counts">
          <span>{l10n.getString("scan-found", { count: scan.found.files, size: fmt.bytes(scan.found.bytes) })}</span>
          <span>{l10n.getString("scan-places", { done: scan.places.done, total: scan.places.total })}</span>
        </p>
      </div>

      <Steps steps={scan.steps.map((step) => stepItem(step, scan.unreadable.length, (id, vars) => l10n.getString(id, vars)))} />

      {scan.unreadable.length > 0 && (
        <div className="plan-block">
          <Localized id="scan-unreadable-note">
            <p className="plan-quiet" />
          </Localized>
          <RowList>
            {scan.unreadable.map((place, i) => {
              const id = `scan-unreadable-${String(i)}`;
              return (
                <Row
                  key={place.path}
                  title={<PathText path={place.path} id={id} />}
                  end={
                    <>
                      <Badge tone="warn" icon={<AttentionMark />}>
                        {l10n.getString("scan-status-not-searched")}
                      </Badge>
                      {onAllowAccess && (
                        <Button
                          variant="quiet"
                          aria-describedby={id}
                          onClick={() => {
                            onAllowAccess(place.path);
                          }}
                        >
                          {l10n.getString("scan-allow")}
                        </Button>
                      )}
                    </>
                  }
                />
              );
            })}
          </RowList>
        </div>
      )}

      {(scan.status === "scanning" || scan.status === "paused" || finished) && (
        <FrameFooter>
          {scan.status === "scanning" && <Button onClick={onPause}>{l10n.getString("scan-pause")}</Button>}
          {scan.status === "paused" && (
            <Button variant="primary" onClick={onResume}>
              {l10n.getString("scan-resume")}
            </Button>
          )}
          {finished && (
            <Button variant="primary" icon={<ArrowForward />} onClick={onContinue}>
              {l10n.getString("scan-continue")}
            </Button>
          )}
        </FrameFooter>
      )}
    </div>
  );
}

/** A step for the shared list: its title and what it covers (the state comes from the marker, with hidden words for screen readers). */
function stepItem(step: ScanStep, unreadable: number, t: (id: string, vars?: Record<string, number>) => string): StepItem {
  return {
    state: step.state,
    title: t(`scan-step-${step.key}-title`),
    body: step.key === "protected" ? t("scan-step-protected-body", { count: unreadable }) : t(`scan-step-${step.key}-body`),
  };
}
