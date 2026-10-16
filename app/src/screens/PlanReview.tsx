import { Localized, useLocalization } from "@fluent/react";
import { useEffect, useRef, useState } from "react";
import { Banner, Button, Checkbox } from "../components/kit";
import { ArrowForward, Info } from "../icons";
import { approvalHolds, leftOutByHand, type Approval, type PlanView } from "../plan";
import FrameFooter from "./FrameFooter";
import { TopStrip } from "./FrameParts";
import { Heading } from "./PairParts";
import { FolderTitle, PathText, PlanBox, useFormat } from "./PlanParts";

export interface PlanReviewProps {
  plan: PlanView;
  /** The approval the host is holding. It only counts while it is for exactly this revision and fingerprint. */
  approval: Approval | null;
  /** The person ticked (an approval for the plan on screen) or unticked (null). Also null when a stale approval is dropped. */
  onApprovalChange: (approval: Approval | null) => void;
  /** Start the move. Only ever called with an approval that holds for the plan on screen. */
  onStart: (approval: Approval) => void;
  /** Go back and change the plan. */
  onEdit: () => void;
  /** The plan cannot be approved until people are mapped to accounts: go and do that. */
  onFixPeople?: () => void;
  onBack?: () => void;
}

/**
 * M23 Approve the move you want. As the mockup: the notice on top, two boxes side by side (what will move,
 * what you do by hand), the approval row, the note, then the action. For the wide window: the host sets it
 * with `windowModeFor("review")`.
 *
 * The approval is tied to the plan revision the person saw: it holds only for the same revision number and
 * the same fingerprint. If the plan changes under an approval, the approval is dropped, the person is told so
 * out loud, and Start the move stays closed until they approve the new version.
 */
export default function PlanReview({ plan, approval, onApprovalChange, onStart, onEdit, onFixPeople, onBack }: PlanReviewProps) {
  const { l10n } = useLocalization();
  const fmt = useFormat();
  const holds = approvalHolds(plan, approval);
  const key = `${String(plan.revision)}:${plan.fingerprint}`;
  // What the plan was the last time we looked, to hear when it moves on under an approval the person had given.
  const [last, setLast] = useState({ key, holds });
  const [changed, setChanged] = useState(false);
  if (last.key !== key || last.holds !== holds) {
    setLast({ key, holds });
    if (holds) setChanged(false);
    else if (last.holds) setChanged(true);
  }
  // An approval the host still holds for something other than this plan does not stand.
  const stale = approval !== null && !holds;
  const showChanged = !holds && (changed || stale);
  // The latest callback, so a host that makes a new one each render does not re-run the check below.
  const tell = useRef(onApprovalChange);
  useEffect(() => {
    tell.current = onApprovalChange;
  });
  useEffect(() => {
    if (stale) tell.current(null);
  }, [stale]);

  const moving = plan.groups.reduce((sum, g) => sum + (g.size.kind === "unreadable" ? 0 : g.size.bytes), 0);
  const partly = plan.groups.some((g) => g.size.kind !== "known");
  const places = new Set(plan.groups.map((g) => `${g.toDrive}\u0000${g.toFolder}`));
  const placeText = (drive: string, folder: string) => (folder ? l10n.getString("plan-place", { folder, drive }) : drive);
  const byHand = leftOutByHand(plan);
  const blocked = plan.blocked;
  const canStart = holds && !blocked;
  const reference = plan.fingerprint.slice(0, 8).toUpperCase();

  return (
    <div className="plan-screen plan-wide">
      <TopStrip onBack={onBack} labelId="plan-eyebrow" />
      <Heading id="plan-title" />
      <Localized id="plan-body">
        <p className="subhead" />
      </Localized>
      <p className="plan-version">{l10n.getString("plan-version", { revision: plan.revision })}</p>

      {showChanged && (
        <div className="plan-notice">
          <Banner tone="attention">
            <p>{l10n.getString("plan-changed")}</p>
          </Banner>
        </div>
      )}
      {blocked && (
        <div className="plan-notice">
          <Banner tone="attention">
            <p>{l10n.getString(`plan-blocked-${blocked}`)}</p>
            {blocked !== "invalid" && onFixPeople && (
              <Button variant="quiet" onClick={onFixPeople}>
                {l10n.getString("plan-blocked-fix")}
              </Button>
            )}
          </Banner>
        </div>
      )}

      <div className="plan-columns">
        <PlanBox title={l10n.getString("plan-what-title")}>
          <p className="plan-lead">{l10n.getString("plan-what-body", { count: plan.groups.length, size: fmt.total(moving, partly) })}</p>
          <ul className="plan-items">
            {plan.groups.map((g) => (
              <li key={g.id} className="plan-item">
                <span className="plan-item-head">
                  <strong>
                    <FolderTitle role={g.role} path={g.path} />
                  </strong>
                  <span className="plan-size">{fmt.size(g.size)}</span>
                </span>
                <small className="kit-hint">
                  <PathText path={g.path} />
                </small>
                <small className="kit-hint">
                  <bdi dir="auto">{l10n.getString("plan-group-to", { place: placeText(g.toDrive, g.toFolder) })}</bdi>
                </small>
              </li>
            ))}
          </ul>
          <p className="plan-quiet">{l10n.getString("plan-where-body", { count: places.size })}</p>
        </PlanBox>

        <PlanBox title={l10n.getString("plan-hand-title")}>
          {byHand.length === 0 ? (
            <Localized id="plan-hand-none">
              <p className="plan-lead" />
            </Localized>
          ) : (
            <>
              <p className="plan-lead">{l10n.getString("plan-hand-body", { count: byHand.length })}</p>
              <ul className="plan-items">
                {byHand.map((h) => (
                  <li key={h.reason} className="plan-item">
                    <small className="plan-item-text">{l10n.getString(`plan-hand-${h.reason}`, { count: h.count })}</small>
                  </li>
                ))}
              </ul>
            </>
          )}
          <ul className="plan-items">
            <li className="plan-item">
              <small className="plan-item-text">{l10n.getString("plan-old-safe")}</small>
            </li>
          </ul>
        </PlanBox>
      </div>

      {!blocked && (
        <div className="plan-approve">
          <Checkbox
            checked={holds}
            onChange={(on) => {
              onApprovalChange(on ? { revision: plan.revision, fingerprint: plan.fingerprint } : null);
            }}
            label={l10n.getString("plan-approve")}
          />
        </div>
      )}
      {!holds && (
        <p className="plan-quiet" id="plan-hint">
          {l10n.getString("plan-not-approved")}
        </p>
      )}

      <div className="note">
        <Info />
        <Localized id="plan-edit-note">
          <p className="note-text" />
        </Localized>
      </div>

      <details className="plan-more">
        <summary>{l10n.getString("plan-details")}</summary>
        <p>{l10n.getString("plan-reference", { reference })}</p>
      </details>

      <div className="plan-actions">
        <Button onClick={onEdit}>{l10n.getString("plan-edit")}</Button>
      </div>
      <FrameFooter>
          <Button
            variant="primary"
            icon={<ArrowForward />}
            disabled={!canStart}
            aria-describedby={holds ? undefined : "plan-hint"}
            onClick={() => {
              // Checked again at the moment of the click: the approval must still be for this exact plan.
              if (approval && approvalHolds(plan, approval) && !blocked) onStart(approval);
            }}
          >
            {l10n.getString("plan-start")}
          </Button>
      </FrameFooter>
    </div>
  );
}
