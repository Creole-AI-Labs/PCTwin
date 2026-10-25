import { Localized, useLocalization } from "@fluent/react";
import { useEffect, useRef, useState } from "react";
import { Banner, Button, Checkbox } from "../components/kit";
import { ArrowForward, Info } from "../icons";
import { approvalHolds, leftOutByHand, movesFirst, type Approval, type FolderRole, type PlanPreview, type PlanView } from "../plan";
import { pageLocale } from "../move";
import FrameFooter from "./FrameFooter";
import { TopStrip } from "./FrameParts";
import { Heading } from "./PairParts";
import { FolderTitle, PathText, PlanBox, useFormat } from "./PlanParts";

type ListFormatter = new (locale: string, options: { type: "conjunction" }) => { format: (items: string[]) => string };

/** Names as a list in the page's language ("Desktop and Documents"); commas where the system has no list format. */
function listText(names: string[]): string {
  const ListFormat = (Intl as unknown as { ListFormat?: ListFormatter }).ListFormat;
  return ListFormat ? new ListFormat(pageLocale(), { type: "conjunction" }).format(names) : names.join(", ");
}

/**
 * What the new laptop found before anything moves, each said plainly: whether it fits, about how
 * long, what moves first, and every file that will not go as it is. Nothing here is a promise;
 * each line is measured or predicted by the very rules the copy uses. `first`: the groups the move
 * sends first ("we'll move these first"), so the new laptop is useful sooner.
 */
function PreviewNotes({ preview, first }: { preview: PlanPreview; first: FolderRole[] }) {
  const { l10n } = useLocalization();
  const fmt = useFormat();
  const lines: { key: string; text: string }[] = [];
  if (first.length > 0) {
    const names = first.map((role) => l10n.getString(`plan-role-${role}`));
    const list = listText(names);
    lines.push({ key: "first", text: l10n.getString("plan-preview-first", { places: list }) });
  }
  if (preview.fits === true) lines.push({ key: "fits", text: l10n.getString("plan-preview-fits") });
  else if (preview.fits === false) lines.push({ key: "fits", text: l10n.getString("plan-preview-short", { size: fmt.total(preview.shortBytes, false) }) });
  else lines.push({ key: "fits", text: l10n.getString("plan-preview-space-unknown") });
  if (preview.minutes) {
    const { low, high } = preview.minutes;
    lines.push({ key: "time", text: low === high ? l10n.getString("plan-preview-time-one", { minutes: low }) : l10n.getString("plan-preview-time", { low, high }) });
  }
  for (const [key, count] of [
    ["renamed", preview.renamed],
    ["too-big", preview.tooBig],
    ["refused", preview.refused],
    ["runs-at-sign-in", preview.runsAtSignIn],
  ] as const) {
    if (count > 0) lines.push({ key, text: l10n.getString(`plan-preview-${key}`, { count }) });
  }
  const attention = preview.fits === false || preview.tooBig + preview.refused + preview.runsAtSignIn > 0;
  // (The overview counts the same items, by planAttention; space that could not be read is said here in its own words.)
  return (
    <div className="plan-notice">
      <Banner tone={attention ? "attention" : "info"} title={l10n.getString("plan-preview-title")}>
        <ul className="plan-items">
          {lines.map((l) => (
            <li key={l.key} className="plan-item">
              <small className="plan-item-text">{l.text}</small>
            </li>
          ))}
        </ul>
      </Banner>
    </div>
  );
}

/** A folder the person chose for a group (M05), as the system shows it, with its free space when known. */
export type ChosenPlace = { path: string; freeBytes: number | null };

/** Why the place just chosen for a group was not taken. */
export type PlaceNote = { role: FolderRole; kind: "no-safe-name" | "cannot-open" };

export interface PlanReviewProps {
  plan: PlanView;
  /** The approval the host is holding. It only counts while it is for exactly this revision and fingerprint. */
  approval: Approval | null;
  /** The person ticked (an approval for the plan on screen) or unticked (null). Also null when a stale approval is dropped. */
  onApprovalChange: (approval: Approval | null) => void;
  /** Start the move. Only ever called with an approval that holds for the plan on screen. */
  onStart: (approval: Approval) => void;
  /** Go back and change the plan. Without it (the plan cannot be changed here yet), there is no Edit button. */
  onEdit?: () => void;
  /** The host was told the plan changed under an approval (the engine refused it): say so. */
  changedNotice?: boolean;
  /** Folders the person unticked: they stay on the old laptop. With `onToggleGroup`, each group has a tick. */
  leftOut?: readonly FolderRole[];
  onToggleGroup?: (role: FolderRole, move: boolean) => void;
  /** The places chosen for some groups. With `onChoosePlace`, each group can be sent to another folder. */
  chosen?: Partial<Record<FolderRole, ChosenPlace>>;
  placeNote?: PlaceNote | null;
  onChoosePlace?: (role: FolderRole) => void;
  onUsualPlace?: (role: FolderRole) => void;
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
export default function PlanReview({ plan, approval, onApprovalChange, onStart, onEdit, changedNotice = false, leftOut = [], onToggleGroup, chosen = {}, placeNote = null, onChoosePlace, onUsualPlace, onFixPeople, onBack }: PlanReviewProps) {
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
  const showChanged = (!holds && (changed || stale)) || changedNotice;
  // The latest callback, so a host that makes a new one each render does not re-run the check below.
  const tell = useRef(onApprovalChange);
  useEffect(() => {
    tell.current = onApprovalChange;
  });
  useEffect(() => {
    if (stale) tell.current(null);
  }, [stale]);

  const kept = plan.groups.filter((g) => !leftOut.includes(g.role));
  const moving = kept.reduce((sum, g) => sum + (g.size.kind === "unreadable" ? 0 : g.size.bytes), 0);
  const partly = plan.groups.some((g) => g.size.kind !== "known");
  const places = new Set(plan.groups.map((g) => `${g.toDrive}\u0000${g.toFolder}`));
  const placeText = (drive: string, folder: string) => (folder && drive ? l10n.getString("plan-place", { folder, drive }) : folder || drive);
  const byHand = leftOutByHand(plan);
  // Each chosen folder must hold every group sent to it.
  const needs = new Map<string, number>();
  for (const g of kept) {
    const place = chosen[g.role];
    if (place && g.size.kind !== "unreadable") needs.set(place.path, (needs.get(place.path) ?? 0) + g.size.bytes);
  }
  const noRoom = (role: FolderRole) => {
    const place = chosen[role];
    return !!place && !leftOut.includes(role) && place.freeBytes !== null && (needs.get(place.path) ?? 0) > place.freeBytes;
  };
  const roomless = plan.groups.some((g) => noRoom(g.role));
  const blocked = plan.blocked;
  const canStart = holds && !blocked && !roomless;
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

      {plan.preview && <PreviewNotes preview={plan.preview} first={movesFirst(plan, leftOut)} />}

      <div className="plan-columns">
        <PlanBox title={l10n.getString("plan-what-title")}>
          <p className="plan-lead">{l10n.getString("plan-what-body", { count: kept.length, size: fmt.total(moving, partly) })}</p>
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
                  <bdi dir="auto">{l10n.getString("plan-group-to", { place: chosen[g.role]?.path ?? placeText(g.toDrive, g.toFolder) })}</bdi>
                </small>
                {chosen[g.role]?.freeBytes != null && (
                  <small className="kit-hint">{l10n.getString("plan-place-free", { size: fmt.total(chosen[g.role]?.freeBytes ?? 0, false) })}</small>
                )}
                {noRoom(g.role) && (
                  <small className="kit-hint plan-attention" role="alert">
                    {l10n.getString("plan-place-no-room")}
                  </small>
                )}
                {placeNote?.role === g.role && (
                  <small className="kit-hint plan-attention" role="alert">
                    {l10n.getString(`plan-place-${placeNote.kind}`)}
                  </small>
                )}
                {onChoosePlace && !leftOut.includes(g.role) && (
                  <span className="plan-place-actions">
                    <Button
                      variant="quiet"
                      onClick={() => {
                        onChoosePlace(g.role);
                      }}
                    >
                      {l10n.getString("plan-place-change")}
                    </Button>
                    {chosen[g.role] && onUsualPlace && (
                      <Button
                        variant="quiet"
                        onClick={() => {
                          onUsualPlace(g.role);
                        }}
                      >
                        {l10n.getString("plan-place-usual")}
                      </Button>
                    )}
                  </span>
                )}
                {onToggleGroup && (
                  <Checkbox
                    checked={!leftOut.includes(g.role)}
                    onChange={(on) => {
                      onToggleGroup(g.role, on);
                    }}
                    label={l10n.getString("plan-group-move", { folder: l10n.getString(`plan-role-${g.role}`) })}
                  />
                )}
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

      {onEdit && (
        <div className="plan-actions">
          <Button onClick={onEdit}>{l10n.getString("plan-edit")}</Button>
        </div>
      )}
      <FrameFooter>
          <Button
            variant="primary"
            icon={<ArrowForward />}
            disabled={!canStart}
            aria-describedby={holds ? undefined : "plan-hint"}
            onClick={() => {
              // Checked again at the moment of the click: the approval must still be for this exact plan.
              if (approval && approvalHolds(plan, approval) && !blocked && !roomless) onStart(approval);
            }}
          >
            {l10n.getString("plan-start")}
          </Button>
      </FrameFooter>
    </div>
  );
}
