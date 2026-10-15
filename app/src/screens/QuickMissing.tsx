import { Localized, useLocalization } from "@fluent/react";
import { Badge, Banner, Button, Checkbox, Row } from "../components/kit";
import { Attention, Check, Info } from "../icons";
import { fileName, missingLines, type Receipt, type SourceIssue } from "../quicksend";
import { Folder, Heading, ISSUE_TEXT, Name, QsFooter, QsRowList, QsStrip, Tile } from "./QsParts";

/**
 * Q19 Send the missing pieces (sender): uses the last receipt to send only what is missing. Files that
 * arrived (or were already there) are kept and not sent again. A file that changed since the first send
 * is a new file as far as the other laptop is concerned: it is listed, and it is checked and accepted
 * again on the next screens. Without a receipt the screen cannot tell what arrived, so it says so and
 * offers a fresh send instead.
 * Events: onApprovedChange, onContinue (check the missing files), onNewSend, onBack.
 */
export default function QuickMissing({
  receipt,
  issues = [],
  approved,
  onApprovedChange,
  onContinue,
  onNewSend,
  onBack,
}: {
  receipt: Receipt | null;
  issues?: SourceIssue[];
  approved: boolean;
  onApprovedChange: (on: boolean) => void;
  onContinue: () => void;
  onNewSend: () => void;
  onBack: () => void;
}) {
  const { l10n } = useLocalization();
  const missing = receipt ? missingLines(receipt) : [];
  const kept = receipt ? receipt.lines.filter((l) => l.outcome.kind === "arrived" || l.outcome.kind === "already-there") : [];
  const tile = (
    <Tile>
      <Folder size={18} />
    </Tile>
  );
  return (
    <>
      <QsStrip onBack={onBack} />
      <Heading id="qs-missing-title" />
      <Localized id="qs-missing-body">
        <p className="subhead" />
      </Localized>

      <div role="status" className="qs-status">
        {receipt === null && (
          <Banner tone="attention">
            <Localized id="qs-missing-unavailable">
              <p />
            </Localized>
          </Banner>
        )}
        {receipt !== null && missing.length === 0 && (
          <Banner tone="ready">
            <Localized id="qs-missing-complete">
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

      {receipt !== null && (
        <QsRowList>
          {kept.map((l) => (
            <Row
              key={l.path}
              icon={tile}
              title={<Name>{fileName(l.path)}</Name>}
              detail={l10n.getString("qs-missing-there-body")}
              end={
                <Badge tone="good" icon={<Check />}>
                  {l10n.getString("qs-missing-status-there")}
                </Badge>
              }
            />
          ))}
          {missing.map((l) => (
            <Row
              key={l.path}
              icon={tile}
              title={<Name>{fileName(l.path)}</Name>}
              detail={l10n.getString("qs-missing-missing-body")}
              end={
                <Badge tone="warn" icon={<Attention />}>
                  {l10n.getString("qs-missing-status-missing")}
                </Badge>
              }
            />
          ))}
        </QsRowList>
      )}

      {missing.length > 0 && (
        <>
          <div className="qs-block qs-card">
            <Checkbox
              checked={approved}
              onChange={onApprovedChange}
              label={
                <Localized id="qs-missing-ack">
                  <span />
                </Localized>
              }
              hint={
                <Localized id="qs-missing-ack-hint">
                  <span />
                </Localized>
              }
            />
          </div>
          <div className="note">
            <Info />
            <Localized id="qs-missing-note">
              <p className="note-text" />
            </Localized>
          </div>
          <QsFooter>
            <Button variant="primary" disabled={!approved} onClick={onContinue}>
              <Localized id="qs-missing-continue">
                <span />
              </Localized>
            </Button>
          </QsFooter>
        </>
      )}

      {receipt === null && (
        <QsFooter>
          <Button variant="primary" onClick={onNewSend}>
            <Localized id="qs-missing-new-send">
              <span />
            </Localized>
          </Button>
        </QsFooter>
      )}
    </>
  );
}
