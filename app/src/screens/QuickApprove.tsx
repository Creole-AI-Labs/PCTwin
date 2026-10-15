import { Localized, useLocalization } from "@fluent/react";
import { Badge, Banner, Button, Checkbox, Row } from "../components/kit";
import { Attention, Check, Info, Laptop } from "../icons";
import { labelText, type Label } from "../pairing";
import { useBytes, type SourceIssue } from "../quicksend";
import FrameFooter from "./FrameFooter";
import { Folder, Heading, ISSUE_TEXT, QsRowList, QsStrip, Route, TallyBar, Tile } from "./QsParts";

/** Where the other laptop's approval stands: waiting, given, taken back, or cleared by a change to the files. */
export type ReceiverApproval = "waiting" | "accepted" | "withdrawn" | "cleared";

/**
 * Q11 Ready to send these files? (sender) The last look before anything is sent: the route, the tally,
 * readiness rows, and the person's own approval. Sending needs three things at once: the other laptop
 * has accepted and chosen its folder, the chosen files are still as they were, and the person ticks
 * their approval. Changing the files clears the other laptop's approval (it goes to "cleared" and must
 * accept again), and the person's tick is a controlled value so the parent clears it too.
 * Events: onApprovedChange, onStart, onBack.
 */
export default function QuickApprove({
  from,
  to,
  count,
  size,
  receiver,
  folder,
  issues = [],
  approved,
  onApprovedChange,
  onStart,
  onBack,
}: {
  from: Label;
  to: Label;
  count: number;
  size: number;
  receiver: ReceiverApproval;
  /** The folder the receiver chose, once they have. */
  folder?: string;
  issues?: SourceIssue[];
  approved: boolean;
  onApprovedChange: (on: boolean) => void;
  onStart: () => void;
  onBack: () => void;
}) {
  const { l10n } = useLocalization();
  const bytes = useBytes();
  const label = labelText(l10n, to);
  const accepted = receiver === "accepted";
  const blocked = !accepted || issues.length > 0 || !approved;
  return (
    <>
      <QsStrip onBack={onBack} />
      <Heading id="qs-approve-title" />
      <Localized id="qs-approve-body">
        <p className="subhead" />
      </Localized>
      <Route from={from} to={to} />
      <TallyBar count={count} size={bytes(size)} />

      <div role="status" className="qs-status">
        {receiver === "withdrawn" && (
          <Banner tone="attention">
            <Localized id="qs-approve-withdrawn" vars={{ label }}>
              <p />
            </Localized>
          </Banner>
        )}
        {receiver === "cleared" && (
          <Banner tone="info">
            <Localized id="qs-approve-cleared" vars={{ label }}>
              <p />
            </Localized>
          </Banner>
        )}
        {issues.length > 0 && (
          <Banner tone="attention">
            <ul className="qs-issues">
              {issues.map((i) => (
                <li key={i.path}>{l10n.getString(ISSUE_TEXT[i.kind], { name: i.path })}</li>
              ))}
            </ul>
          </Banner>
        )}
      </div>

      <QsRowList>
        <Row
          icon={
            <Tile>
              <Laptop className="qs-small-icon" />
            </Tile>
          }
          title={
            <Localized id="qs-approve-receiver-title">
              <span />
            </Localized>
          }
          detail={l10n.getString(accepted ? "qs-approve-receiver-ready" : "qs-approve-receiver-waiting", { label })}
          end={
            accepted ? (
              <Badge tone="good" icon={<Check />}>
                {l10n.getString("qs-status-accepted")}
              </Badge>
            ) : (
              <Badge tone={receiver === "withdrawn" ? "warn" : "neutral"} icon={receiver === "withdrawn" ? <Attention /> : <Info />}>
                {l10n.getString("qs-status-waiting")}
              </Badge>
            )
          }
        />
        <Row
          icon={
            <Tile>
              <Folder size={18} />
            </Tile>
          }
          title={
            <Localized id="qs-approve-folder-title">
              <span />
            </Localized>
          }
          detail={folder ? <bdi dir="auto">{folder}</bdi> : l10n.getString("qs-approve-receiver-waiting")}
          end={
            <Badge tone="neutral" icon={<Info />}>
              {l10n.getString("qs-status-recipient-choice")}
            </Badge>
          }
        />
        <Row
          icon={
            <Tile>
              <Folder size={18} />
            </Tile>
          }
          title={
            <Localized id="qs-approve-files-title">
              <span />
            </Localized>
          }
          detail={l10n.getString(issues.length > 0 ? "qs-approve-files-changed" : "qs-approve-files-same")}
          end={
            issues.length > 0 ? (
              <Badge tone="warn" icon={<Attention />}>
                {l10n.getString("qs-status-changed")}
              </Badge>
            ) : (
              <Badge tone="good" icon={<Check />}>
                {l10n.getString("qs-status-unchanged")}
              </Badge>
            )
          }
        />
        <Row
          icon={
            <Tile>
              <Folder size={18} />
            </Tile>
          }
          title={
            <Localized id="qs-approve-existing-title">
              <span />
            </Localized>
          }
          detail={l10n.getString("qs-approve-existing-body")}
          end={
            <Badge tone="neutral" icon={<Info />}>
              {l10n.getString("qs-status-unchanged")}
            </Badge>
          }
        />
      </QsRowList>

      <div className="qs-block qs-card">
        <Checkbox
          checked={approved}
          onChange={onApprovedChange}
          label={
            <Localized id="qs-approve-ack">
              <span />
            </Localized>
          }
          hint={
            <Localized id="qs-approve-ack-hint">
              <span />
            </Localized>
          }
        />
      </div>

      <FrameFooter note="send">
        <Button variant="primary" disabled={blocked} onClick={onStart}>
          <Localized id="qs-approve-start">
            <span />
          </Localized>
        </Button>
      </FrameFooter>
    </>
  );
}
