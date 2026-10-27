/**
 * The more move types' screens (Task List 2.6, shown by 2.8):
 * - Space (M31, M05): each drive's room at the busiest moment, what PCTwin proposes for what does
 *   not fit (left in its cloud, an approved offload drive, or left on the old laptop: never dropped),
 *   the offload drive approved on its own, and what is safe in a cloud.
 * - Local copy (M09): keep a copy of online-only files, with the download size and the old laptop's
 *   room, and its paused states.
 * - Owners (M07): who goes where across laptops.
 * - Partial report (X12): what did not finish, each with its own Try again, and what is safe in a
 *   cloud (never counted as copied).
 */
import { Localized, useLocalization } from "@fluent/react";
import { Badge, Banner, Row, RowList } from "../components/kit";
import type { LineWhat, SafeInLine, ScreenView } from "../guard";
import { Attention, Check, Globe } from "../icons";
import FrameFooter from "./FrameFooter";
import { TopStrip } from "./FrameParts";
import { DriveRow } from "./ChangesPreview";
import { IntentButton, MeasuredLine, MoreLine, Name, RefusalLine, StateLine, useGuard } from "./GuardParts";
import { DriveIcon, PeopleIcon, PlanBox, useFormat } from "./PlanParts";
import { Heading } from "./ProgressParts";

/** "Safe in OneDrive: 120 files. They come back when you sign in on the new laptop." */
function SafeInRows({ lines }: { lines: SafeInLine[] }) {
  const { l10n } = useLocalization();
  if (lines.length === 0) return null;
  return (
    <RowList>
      {lines.map((s, i) => (
        <Row
          key={i}
          icon={<Globe />}
          title={s.provider === null ? l10n.getString("guard-safe-in-cloud") : l10n.getString("guard-safe-in", { provider: s.provider })}
          detail={l10n.getString("guard-safe-in-detail", { count: s.files })}
          end={<Badge>{l10n.getString("status-needs-action")}</Badge>}
        />
      ))}
    </RowList>
  );
}

export function SpaceBudget({ screen }: { screen: Extract<ScreenView, { kind: "space" }> }) {
  const { l10n } = useLocalization();
  const { bytes, number } = useFormat();
  const short = screen.drives.some((d) => d.fits === false);
  return (
    <>
      <TopStrip labelId="guard-strip-plan" />
      <Heading id={short ? "guard-space-title-short" : "guard-space-title"} />
      <Localized id="guard-space-body">
        <p className="subhead" />
      </Localized>
      <StateLine />
      <RefusalLine />
      <PlanBox title={l10n.getString("guard-changes-space")}>
        <RowList>
          {screen.drives.map((d, i) => (
            <DriveRow key={i} drive={d} />
          ))}
        </RowList>
      </PlanBox>
      {screen.proposal.length > 0 && (
        <PlanBox title={l10n.getString("guard-space-proposal")}>
          <RowList>
            {screen.proposal.map((p, i) => (
              <Row key={i} title={l10n.getString(`guard-keep-off-${p.to}`)} detail={l10n.getString("guard-items-size", { items: number(p.items), size: bytes(p.bytes) })} />
            ))}
          </RowList>
        </PlanBox>
      )}
      {screen.offload && (
        <PlanBox title={l10n.getString("guard-offload-title")}>
          <RowList>
            <Row
              icon={<DriveIcon />}
              title={<Name>{screen.offload.drive}</Name>}
              detail={screen.offload.free === null ? l10n.getString("guard-free-unknown") : l10n.getString("guard-free", { free: bytes(screen.offload.free) })}
              end={
                screen.offload.connected ? (
                  <Badge tone="good" icon={<Check />}>
                    {l10n.getString("guard-offload-connected")}
                  </Badge>
                ) : (
                  <Badge tone="warn" icon={<Attention />}>
                    {l10n.getString("guard-offload-not-connected")}
                  </Badge>
                )
              }
            />
          </RowList>
          <p className="kit-hint">{l10n.getString(screen.offload.approved ? "guard-offload-approved" : "guard-offload-ask")}</p>
          {!screen.offload.approved && (
            <div className="g-actions">
              <IntentButton intent={{ kind: "approve-offload" }} label={l10n.getString("guard-offload-use")} />
            </div>
          )}
        </PlanBox>
      )}
      <SafeInRows lines={screen.safeIn} />
      <FrameFooter>
        <IntentButton intent={{ kind: "back" }} variant="quiet" label={l10n.getString("frame-back")} />
        <IntentButton intent={{ kind: "approve" }} variant="primary" label={l10n.getString("guard-space-approve")} />
      </FrameFooter>
    </>
  );
}

export function LocalCopy({ screen }: { screen: Extract<ScreenView, { kind: "local-copy" }> }) {
  const { l10n } = useLocalization();
  const { bytes } = useFormat();
  const { view } = useGuard();
  const provider = screen.provider ?? l10n.getString("guard-your-cloud");
  return (
    <>
      <TopStrip labelId="guard-strip-plan" />
      <Heading id="guard-local-title" />
      <p className="subhead">{l10n.getString("guard-local-body", { count: screen.files, provider })}</p>
      <StateLine />
      <RefusalLine />
      <dl className="g-facts">
        <div className="g-fact">
          <dt>{l10n.getString("guard-local-download")}</dt>
          <dd>{bytes(screen.downloadBytes)}</dd>
        </div>
        <div className="g-fact">
          <dt>{l10n.getString("guard-local-old-space")}</dt>
          <dd>{screen.oldFreeBytes === null ? l10n.getString("guard-not-known-yet") : l10n.getString("guard-free", { free: bytes(screen.oldFreeBytes) })}</dd>
        </div>
      </dl>
      <Localized id="guard-local-how">
        <p className="kit-hint" />
      </Localized>
      {screen.state === "working" && <MeasuredLine progress={view.progress} />}
      {screen.state === "paused-metered" && <Banner tone="attention">{l10n.getString("guard-local-paused-metered")}</Banner>}
      {screen.state === "paused-no-space" && <Banner tone="attention">{l10n.getString("guard-local-paused-no-space", { provider })}</Banner>}
      {screen.state === "done" && <Banner tone="ready">{l10n.getString("guard-local-done")}</Banner>}
      <FrameFooter>
        <IntentButton intent={{ kind: "keep-local-copy", keep: false }} variant="quiet" label={l10n.getString("guard-local-leave", { provider })} />
        <IntentButton intent={{ kind: "allow-metered" }} label={l10n.getString("guard-local-allow-metered")} />
        <IntentButton intent={{ kind: "pause" }} label={l10n.getString("guard-pause")} />
        <IntentButton intent={{ kind: "resume" }} label={l10n.getString("guard-resume")} />
        <IntentButton intent={{ kind: "keep-local-copy", keep: true }} variant="primary" label={l10n.getString("guard-local-keep")} />
      </FrameFooter>
    </>
  );
}

export function OwnerTable({ screen }: { screen: Extract<ScreenView, { kind: "owners" }> }) {
  const { l10n } = useLocalization();
  return (
    <>
      <TopStrip labelId="guard-strip-plan" />
      <Heading id="guard-owners-title" />
      <Localized id="guard-owners-body">
        <p className="subhead" />
      </Localized>
      <StateLine />
      <RefusalLine />
      {[...new Set(screen.rows.map((r) => r.laptop))].map((laptop) => (
        <PlanBox key={laptop} title={l10n.getString("guard-owners-from", { laptop })}>
          <RowList>
            {screen.rows
              .filter((r) => r.laptop === laptop)
              .map((r, i) => (
                <Row
                  key={i}
                  icon={<PeopleIcon />}
                  title={<Name>{r.person}</Name>}
                  detail={r.account === null ? l10n.getString("guard-owners-new-account") : l10n.getString("guard-owners-to-account", { account: r.account })}
                  end={<Badge>{r.ownFolder ? l10n.getString("guard-owners-own-folder", { laptop: r.laptop }) : l10n.getString("guard-owners-usual-folders")}</Badge>}
                />
              ))}
          </RowList>
        </PlanBox>
      ))}
      <MoreLine more={screen.more} />
      <FrameFooter>
        <IntentButton intent={{ kind: "back" }} variant="quiet" label={l10n.getString("frame-back")} />
        <IntentButton intent={{ kind: "approve" }} variant="primary" label={l10n.getString("guard-owners-approve")} />
      </FrameFooter>
    </>
  );
}

function useWhat() {
  const { l10n } = useLocalization();
  return (what: LineWhat): string => (what.kind === "app" ? l10n.getString("guard-line-app", { name: what.name }) : l10n.getString(`guard-line-${what.kind}`, { count: what.count }));
}

export function PartialReport({ screen }: { screen: Extract<ScreenView, { kind: "report" }> }) {
  const { l10n } = useLocalization();
  const what = useWhat();
  return (
    <>
      <TopStrip labelId="guard-strip-report" />
      <Heading id={screen.lines.length + screen.more > 0 ? "guard-report-title-partial" : "guard-report-title-done"} />
      <Localized id={screen.lines.length + screen.more > 0 ? "guard-report-body-partial" : "guard-report-body-done"}>
        <p className="subhead" />
      </Localized>
      <StateLine />
      <RefusalLine />
      {screen.lines.length > 0 && (
        <ul className="kit-rows">
          {screen.lines.map((line) => (
            <li key={line.line} className="kit-row">
              <span className="kit-row-name">
                <span className="kit-row-icon">
                  <Attention />
                </span>
                <span>
                  <bdi className="g-name">{what(line.what)}</bdi>
                  <small className="kit-hint">
                    {l10n.getString(`guard-line-why-${line.why}`, { laptop: line.laptop })}
                  </small>
                </span>
              </span>
              <span className="kit-row-end">
                <IntentButton intent={{ kind: "try-again", line: line.line }} label={l10n.getString("guard-try-again")} />
              </span>
            </li>
          ))}
        </ul>
      )}
      <MoreLine more={screen.more} />
      <SafeInRows lines={screen.safeIn} />
    </>
  );
}
