/**
 * "Before we change anything" (Task List 2.9; blueprint M21, M24, M31): every change the move will
 * make on this laptop, sorted into installed, added beside what is there, replaced and left alone,
 * each with what undo can do for it (PCTwin can undo it, PCTwin keeps it, or it needs the person),
 * the download, each drive's room at the busiest moment, and the system approval it needs. Clear
 * for someone moving onto a laptop they already use: nothing already there is hidden.
 *
 * And the re-approval (X06, H10): when something changed after approval, only those items are
 * asked about again; everything else stays approved.
 */
import { Localized, useLocalization } from "@fluent/react";
import { Badge, Row, RowList } from "../components/kit";
import type { ChangeAction, ChangeLine, DriveBudget, ScreenView, UndoPromise } from "../guard";
import { Attention, Check, Lock, Restore, Shield } from "../icons";
import FrameFooter from "./FrameFooter";
import { TopStrip } from "./FrameParts";
import { IntentButton, MoreLine, Name, RefusalLine, StateLine } from "./GuardParts";
import { PlanBox, useFormat } from "./PlanParts";
import { Heading } from "./ProgressParts";

const ACTIONS: ChangeAction[] = ["install", "merge", "replace", "leave-alone"];

/** The three groups of the undo explanation, each with its own icon shape. */
export function UndoLegend() {
  return (
    <ul className="g-legend">
      <li>
        <Restore />
        <Localized id="guard-undo-legend-can">
          <span />
        </Localized>
      </li>
      <li>
        <Lock />
        <Localized id="guard-undo-legend-keeps">
          <span />
        </Localized>
      </li>
      <li>
        <Attention />
        <Localized id="guard-undo-legend-you">
          <span />
        </Localized>
      </li>
    </ul>
  );
}

/** What undo can do for one change, as a badge with words and an icon. */
export function PromiseBadge({ promise }: { promise: UndoPromise }) {
  const { l10n } = useLocalization();
  const icon = promise === "can-reverse" || promise === "can-remove-app" ? <Restore /> : promise === "must-keep" || promise === "keeps-newer" ? <Lock /> : <Attention />;
  // Undo that depends on something the person does or on how the app came is told with a warning shape.
  const warn = promise === "needs-you-service" || promise === "needs-you-maker" || promise === "removes-if-package";
  return (
    <Badge tone={warn ? "warn" : "neutral"} icon={icon}>
      {l10n.getString(`guard-undo-${promise}`)}
    </Badge>
  );
}

/** One drive's room at the busiest moment, in words. */
export function DriveRow({ drive }: { drive: DriveBudget }) {
  const { l10n } = useLocalization();
  const { bytes } = useFormat();
  const detail =
    drive.free === null
      ? l10n.getString("guard-drive-needs-free-unknown", { needed: bytes(drive.needed) })
      : l10n.getString("guard-drive-needs", { needed: bytes(drive.needed), free: bytes(drive.free) });
  const end =
    drive.fits === true ? (
      <Badge tone="good" icon={<Check />}>
        {l10n.getString("guard-drive-fits")}
      </Badge>
    ) : drive.fits === false ? (
      <Badge tone="warn" icon={<Attention />}>
        {l10n.getString("guard-drive-short", { short: bytes(drive.shortBy) })}
      </Badge>
    ) : (
      <Badge>{l10n.getString("guard-drive-unknown")}</Badge>
    );
  return <Row title={<Name>{drive.drive}</Name>} detail={detail} end={end} />;
}

function ChangeRows({ lines }: { lines: ChangeLine[] }) {
  return (
    <RowList>
      {lines.map((c, i) => (
        <Row key={i} title={<Name>{c.name}</Name>} end={<PromiseBadge promise={c.undo} />} />
      ))}
    </RowList>
  );
}

export function ChangesPreview({ screen }: { screen: Extract<ScreenView, { kind: "changes" }> }) {
  const { l10n } = useLocalization();
  const { bytes } = useFormat();
  return (
    <>
      <TopStrip labelId="guard-strip-plan" />
      <Heading id="guard-changes-title" />
      <Localized id="guard-changes-body">
        <p className="subhead" />
      </Localized>
      <StateLine />
      <RefusalLine />
      <dl className="g-facts">
        <div className="g-fact">
          <dt>{l10n.getString("guard-changes-download")}</dt>
          <dd>{screen.downloadBytes === null ? l10n.getString("guard-not-known-yet") : screen.downloadBytes === 0 ? l10n.getString("guard-changes-download-none") : bytes(screen.downloadBytes)}</dd>
        </div>
        <div className="g-fact">
          <dt>{l10n.getString("guard-changes-permissions")}</dt>
          <dd>{l10n.getString(screen.permissions.length > 0 ? "guard-changes-one-prompt" : "guard-changes-no-prompt")}</dd>
        </div>
        <div className="g-fact">
          <dt>{l10n.getString("guard-changes-restore-point")}</dt>
          <dd>{l10n.getString(screen.restorePoint ? "guard-changes-restore-point-first" : "guard-changes-restore-point-none")}</dd>
        </div>
      </dl>
      <UndoLegend />
      {ACTIONS.map((action) => {
        const lines = screen.changes.filter((c) => c.action === action);
        if (lines.length === 0) return null;
        return (
          <PlanBox key={action} title={l10n.getString(`guard-changes-${action}`, { count: lines.length })}>
            <ChangeRows lines={lines} />
          </PlanBox>
        );
      })}
      <MoreLine more={screen.more} />
      {screen.drives.length > 0 && (
        <PlanBox title={l10n.getString("guard-changes-space")}>
          <RowList>
            {screen.drives.map((d, i) => (
              <DriveRow key={i} drive={d} />
            ))}
          </RowList>
        </PlanBox>
      )}
      {screen.permissions.length > 0 && (
        <PlanBox title={l10n.getString("guard-changes-why-prompt")}>
          <RowList>
            {screen.permissions.map((p) => (
              <Row key={p} icon={<Shield />} title={l10n.getString(`guard-permission-${p}`)} />
            ))}
          </RowList>
        </PlanBox>
      )}
      <FrameFooter>
        <IntentButton intent={{ kind: "back" }} variant="quiet" label={l10n.getString("frame-back")} />
        <IntentButton intent={{ kind: "approve" }} variant="primary" label={l10n.getString("guard-changes-approve")} />
      </FrameFooter>
    </>
  );
}

export function ChangedItems({ screen }: { screen: Extract<ScreenView, { kind: "changed" }> }) {
  const { l10n } = useLocalization();
  return (
    <>
      <TopStrip labelId="guard-strip-plan" />
      <Heading id="guard-changed-title" vars={{ count: screen.lines.length + screen.more }} />
      <Localized id="guard-changed-body">
        <p className="subhead" />
      </Localized>
      <StateLine />
      <RefusalLine />
      <RowList>
        {screen.lines.map((c, i) => (
          <Row key={i} title={<Name>{c.name}</Name>} detail={l10n.getString(`guard-changed-${c.why}`)} />
        ))}
      </RowList>
      <MoreLine more={screen.more} />
      <FrameFooter>
        <IntentButton intent={{ kind: "leave-out-changed" }} variant="quiet" label={l10n.getString("guard-changed-leave-out")} />
        <IntentButton intent={{ kind: "approve" }} variant="primary" label={l10n.getString("guard-changed-approve")} />
      </FrameFooter>
    </>
  );
}
