/**
 * The one system approval for a move (Task List 2.2; blueprint M25, H10), and the restore point's
 * honest result.
 *
 * PCTwin says exactly why it needs the system's own administrator prompt, then the system asks:
 * PCTwin never draws a password box and never sees the password. A prompt closed without an answer
 * is not a yes. When the approval ends (the laptop slept, or another person's session took over),
 * the screen says so and asks again; nothing is reused. The restore point says made, an earlier one
 * from a time, not made because the person chose so, or not made and why.
 */
import { Localized, useLocalization } from "@fluent/react";
import { Banner, Row, RowList } from "../components/kit";
import type { RestorePoint, ScreenView } from "../guard";
import { Restore, Shield } from "../icons";
import FrameFooter from "./FrameFooter";
import { TopStrip } from "./FrameParts";
import { IntentButton, RefusalLine, StateLine } from "./GuardParts";
import { PlanBox } from "./PlanParts";
import { Heading } from "./ProgressParts";

export function RestorePointLine({ point }: { point: RestorePoint }) {
  const { l10n } = useLocalization();
  let words: string;
  switch (point.kind) {
    case "made":
    case "earlier-exists":
      words = l10n.getString(`guard-restore-${point.kind}`, { time: point.at });
      break;
    case "not-chosen":
      words = l10n.getString("guard-restore-not-chosen");
      break;
    case "not-made":
      words = l10n.getString(`guard-restore-not-made-${point.why}`);
      break;
  }
  return (
    <RowList>
      <Row icon={<Restore />} title={l10n.getString("guard-restore-title")} detail={words} />
    </RowList>
  );
}

export function SystemPermission({ screen }: { screen: Extract<ScreenView, { kind: "permission" }> }) {
  const { l10n } = useLocalization();
  const ended = screen.stage === "ended-by-sleep" || screen.stage === "ended-other-session";
  const again = ended || screen.stage === "closed-without-answer";
  return (
    <>
      <TopStrip labelId="guard-strip-permission" />
      <Heading id={`guard-permission-title-${screen.stage}`} />
      <Localized id={`guard-permission-body-${screen.stage}`}>
        <p className="subhead" />
      </Localized>
      <StateLine />
      <RefusalLine />
      {screen.reasons.length > 0 && (
        <PlanBox title={l10n.getString("guard-changes-why-prompt")}>
          <RowList>
            {screen.reasons.map((r) => (
              <Row key={r} icon={<Shield />} title={l10n.getString(`guard-permission-${r}`)} />
            ))}
          </RowList>
        </PlanBox>
      )}
      <Banner tone="info">{l10n.getString("guard-permission-never-password")}</Banner>
      {screen.restorePoint && <RestorePointLine point={screen.restorePoint} />}
      <FrameFooter>
        <IntentButton intent={{ kind: "not-now" }} variant="quiet" label={l10n.getString("guard-permission-not-now")} />
        <IntentButton intent={{ kind: "ask-system" }} variant="primary" label={l10n.getString(again ? "guard-permission-ask-again" : "guard-permission-ask")} />
      </FrameFooter>
    </>
  );
}
