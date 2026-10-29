import { useLocalization } from "@fluent/react";
import { Banner, Button, Row, RowList } from "../components/kit";
import { formatBytes, pageLocale } from "../move";
import { overviewOf, planAttention, type PlanView } from "../plan";
import FrameFooter from "./FrameFooter";
import { TopStrip } from "./FrameParts";
import { Heading } from "./ProgressParts";
import "./progress.css";

/**
 * The new laptop: what the old laptop's person chose to send, for one tap to accept (decided 28
 * October 2026). The choosing happens on the old laptop; this laptop, the one written to, only
 * accepts or not. When something needs the person (the space, a file), Accept waits and "See the
 * details" opens the full plan, where a place can be chosen.
 */
export default function AcceptMove({
  plan,
  from,
  onAccept,
  onDetails,
  onNotNow,
}: {
  plan: PlanView;
  /** The old laptop's name, as pairing showed it. */
  from: string;
  onAccept: () => void;
  onDetails: () => void;
  onNotNow: () => void;
}) {
  const { l10n } = useLocalization();
  const { chosen } = overviewOf(plan);
  const attention = planAttention(plan);
  const needs = attention.space || attention.files > 0 || plan.blocked !== undefined;
  // Only what changes this laptop is listed: an app already here, or a line about what stays
  // behind, is not coming.
  const changes = (plan.rebuild?.changes ?? []).filter((c) => c.action !== "leave-alone");
  const apps = changes.filter((c) => c.kind === "app").length;
  const settings = changes.some((c) => c.kind !== "app");
  return (
    <>
      <TopStrip labelId="move-wait-eyebrow" />
      <Heading id="accept-title" vars={{ from }} />
      <p className="subhead">{l10n.getString("accept-body")}</p>
      <RowList>
        <Row title={l10n.getString("accept-files", { count: chosen.files })} detail={formatBytes(chosen.bytes, pageLocale())} />
        {apps > 0 && <Row title={l10n.getString("accept-apps", { count: apps })} />}
        {settings && <Row title={l10n.getString("accept-settings")} />}
      </RowList>
      <div className="mv-note">
        <Banner tone={needs ? "attention" : "ready"}>{l10n.getString(needs ? "accept-needs" : "accept-fits")}</Banner>
      </div>
      <FrameFooter>
        <Button variant="quiet" onClick={onNotNow}>
          {l10n.getString("accept-not-now")}
        </Button>
        <Button variant="quiet" onClick={onDetails}>
          {l10n.getString("accept-details")}
        </Button>
        <Button variant="primary" disabled={needs} onClick={onAccept}>
          {l10n.getString("accept-accept")}
        </Button>
      </FrameFooter>
    </>
  );
}
