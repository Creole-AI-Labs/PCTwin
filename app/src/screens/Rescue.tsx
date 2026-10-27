/**
 * Dead-laptop rescue (Task List 2.6; blueprint T01 to T04), read only.
 *
 * - T01: what a rescue can read and what it cannot, and that the drive is only read.
 * - T02: choose the drive. A drive that something of the move is put on cannot also be rescued: it
 *   is shown, marked, and cannot be chosen; the engine refuses it too.
 * - T03: a locked drive. How to unlock it in the system itself; PCTwin never asks for the key.
 * - T04: what was read, how sure PCTwin is that the drive did not change, and what could not be read.
 */
import { Localized, useLocalization } from "@fluent/react";
import { Badge, Banner, Row, RowList } from "../components/kit";
import type { LockKind, ScreenView } from "../guard";
import { Attention, Lock, Shield } from "../icons";
import FrameFooter from "./FrameFooter";
import { TopStrip } from "./FrameParts";
import { IntentButton, MoreLine, Name, RefusalLine, StateLine, useGuard } from "./GuardParts";
import { DriveIcon, PlanBox, useFormat } from "./PlanParts";
import { Heading } from "./ProgressParts";

const STEPS: Record<LockKind, number> = { "bit-locker": 3, "file-vault": 2, "apple-chip": 2 };

function ReadOnlyLine({ id }: { id: string }) {
  const { l10n } = useLocalization();
  return (
    <RowList>
      <Row icon={<Shield />} title={l10n.getString("guard-rescue-read-only-title")} detail={l10n.getString(id)} />
    </RowList>
  );
}

export function Rescue({ screen }: { screen: Extract<ScreenView, { kind: "rescue" }> }) {
  const { l10n } = useLocalization();
  const { bytes, number } = useFormat();
  const { view, send, busy } = useGuard();
  const stage = screen.stage;
  return (
    <>
      <TopStrip labelId="guard-strip-rescue" />
      <Heading id={`guard-rescue-title-${stage.kind}`} />
      <Localized id={`guard-rescue-body-${stage.kind}`}>
        <p className="subhead" />
      </Localized>
      <StateLine />
      <RefusalLine />
      {stage.kind === "intro" && (
        <>
          <PlanBox title={l10n.getString("guard-rescue-can")}>
            <p className="kit-hint">{l10n.getString("guard-rescue-can-body")}</p>
          </PlanBox>
          <PlanBox title={l10n.getString("guard-rescue-cannot")}>
            <p className="kit-hint">{l10n.getString("guard-rescue-cannot-body")}</p>
          </PlanBox>
          <ReadOnlyLine id="guard-rescue-read-only-promise" />
        </>
      )}
      {stage.kind === "pick-drive" && (
        <>
          {stage.refused !== null && (
            <p className="g-refusal" role="alert">
              <Attention />
              <span>{l10n.getString("guard-rescue-refused-target")}</span>
            </p>
          )}
          <ul className="kit-rows">
            {stage.drives.map((d, i) => {
              const intent = { kind: "choose-drive" as const, drive: i };
              const allowed = view.allowed.includes("choose-drive") && !d.isTarget;
              return (
                <li key={i} className="kit-row">
                  <span className="kit-row-name">
                    <span className="kit-row-icon">
                      <DriveIcon />
                    </span>
                    <span>
                      <Name>{d.drive}</Name>
                      <small className="kit-hint">{l10n.getString("guard-rescue-drive-detail", { size: bytes(d.size), fs: d.fileSystem })}</small>
                      {d.isTarget && <small className="kit-hint">{l10n.getString("guard-rescue-drive-is-target")}</small>}
                    </span>
                  </span>
                  <span className="kit-row-end">
                    {d.isTarget ? (
                      <Badge icon={<Lock />}>{l10n.getString("guard-rescue-drive-cannot")}</Badge>
                    ) : (
                      allowed && (
                        <button
                          type="button"
                          className="kit-button kit-button-secondary g-drive"
                          aria-disabled={busy(intent) || undefined}
                          onClick={() => {
                            if (!busy(intent)) send(intent);
                          }}
                        >
                          <span>{l10n.getString("guard-rescue-choose")}</span>
                        </button>
                      )
                    )}
                  </span>
                </li>
              );
            })}
          </ul>
          <ReadOnlyLine id="guard-rescue-read-only-promise" />
        </>
      )}
      {stage.kind === "locked" && (
        <>
          <ol className="g-steps">
            {Array.from({ length: STEPS[stage.lock] }, (_, i) => (
              <li key={i}>{l10n.getString(`guard-rescue-${stage.lock}-step-${String(i + 1)}`)}</li>
            ))}
          </ol>
          <Banner tone="info">{l10n.getString("guard-rescue-never-key")}</Banner>
        </>
      )}
      {stage.kind === "salvage" && (
        <>
          <p className="subhead">{l10n.getString("guard-rescue-readable", { count: stage.readable, n: number(stage.readable) })}</p>
          <ReadOnlyLine id={`guard-rescue-${stage.readOnly}`} />
          {stage.unreadable.length > 0 && (
            <PlanBox title={l10n.getString("guard-rescue-unreadable", { count: stage.unreadable.length + stage.more })}>
              <RowList>
                {stage.unreadable.map((u, i) => (
                  <Row key={i} icon={<Attention />} title={<Name>{u}</Name>} />
                ))}
              </RowList>
              <MoreLine more={stage.more} />
            </PlanBox>
          )}
        </>
      )}
      <FrameFooter>
        <IntentButton intent={{ kind: "back" }} variant="quiet" label={l10n.getString("frame-back")} />
        {stage.kind !== "salvage" && stage.kind !== "pick-drive" && <IntentButton intent={{ kind: "check-again" }} variant="primary" label={l10n.getString(stage.kind === "intro" ? "guard-rescue-look" : "guard-rescue-check-again")} />}
        {stage.kind === "salvage" && <IntentButton intent={{ kind: "approve" }} variant="primary" label={l10n.getString("guard-rescue-use")} />}
      </FrameFooter>
    </>
  );
}
