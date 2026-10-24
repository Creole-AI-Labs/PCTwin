import { Localized, useLocalization } from "@fluent/react";
import { Badge, Banner, Button, ResultChip, Row } from "../components/kit";
import { ArrowForward, Attention, Check, Info } from "../icons";
import { labelText, type Label } from "../pairing";
import { fileName, formatWhen, missingLines, receiptState, tally, useBytes, type OfferedFile, type Outcome, type Receipt, type ReceiptLine } from "../quicksend";
import Seal from "../motion/Seal";
import FrameFooter from "./FrameFooter";
import { Folder, Heading, Name, QsActions, QsRowList, QsStrip, Tile } from "./QsParts";

type Side = "send" | "receive";

/**
 * The status word and icon for one line. A file that arrived is "Verified" and one that did not is
 * "Needs action" (as the mockup's receipt has it); once the send was stopped the lines that did not
 * arrive are "Unfinished". An identical file that was already there and a file the receiver did not
 * accept have their own words: neither is shown as an arrival.
 */
function LineEnd({ outcome, stopped }: { outcome: Outcome; stopped: boolean }) {
  const { l10n } = useLocalization();
  switch (outcome.kind) {
    case "arrived":
      return (
        <Badge tone="good" icon={<Check />}>
          {l10n.getString("qs-receipt-status-verified")}
        </Badge>
      );
    case "already-there":
      return (
        <Badge tone="neutral" icon={<Info />}>
          {l10n.getString("qs-receipt-chip-already")}
        </Badge>
      );
    case "declined":
      return (
        <Badge tone="warn" icon={<Attention />}>
          {l10n.getString(stopped ? "qs-receipt-status-unfinished" : "qs-receipt-chip-declined")}
        </Badge>
      );
    case "needs-action":
      return <ResultChip result="needs-action" />;
    case "failed":
      return stopped ? (
        <Badge tone="warn" icon={<Attention />}>
          {l10n.getString("qs-receipt-status-unfinished")}
        </Badge>
      ) : (
        <ResultChip result="needs-action" />
      );
  }
}

/** What happened to one file, in plain words. The reason a file failed stays behind More details. */
function LineDetail({ line, side, stopped }: { line: ReceiptLine; side: Side; stopped: boolean }) {
  const { l10n } = useLocalization();
  const { outcome } = line;
  switch (outcome.kind) {
    case "arrived":
      return (
        <>
          <Name>{l10n.getString(stopped ? "qs-receipt-stopped-row" : side === "receive" ? "qs-receipt-saved-as" : "qs-receipt-sent-as", { path: outcome.landedAs })}</Name>
          {!stopped && side === "receive" && outcome.landedAs !== line.path && ` ${l10n.getString("qs-receipt-renamed")}`}
        </>
      );
    case "already-there":
      return <>{l10n.getString("qs-receipt-already", { path: outcome.landedAs })}</>;
    case "declined":
      return <>{l10n.getString(stopped ? "qs-receipt-not-completed" : `qs-receipt-declined-${side}`)}</>;
    case "needs-action":
      return (
        <>
          {l10n.getString(outcome.word ? `qs-receipt-${outcome.word}` : "qs-receipt-needs-action", { path: outcome.landedAs })}
          <details className="qs-more">
            <Localized id="qs-receipt-more">
              <summary />
            </Localized>
            <p dir="auto">{outcome.why}</p>
          </details>
        </>
      );
    case "failed":
      return (
        <>
          {l10n.getString(stopped ? "qs-receipt-not-completed" : "qs-receipt-not-arrived")}
          <details className="qs-more">
            <Localized id="qs-receipt-more">
              <summary />
            </Localized>
            <p dir="auto">{outcome.why}</p>
          </details>
        </>
      );
  }
}

/**
 * Q14 Handover receipt, on either side: a card saying plainly whether everything arrived, the two
 * counts, each file with what happened to it, and what to do next. It never says the handover is whole
 * unless every line ended with the file in place; a file that was declined or did not arrive makes it
 * "not everything arrived" and says how many still need attention. An identical file that was already
 * there is its own line with its own word. Acknowledging is the receiver saying "I have seen this
 * receipt", and the screen says in words that it does not mean anything was opened. `files` (the
 * offer) gives the sizes; the receipt itself has none.
 * Events: onAcknowledge (receiver), onSendMissing (sender), onHistory, onDone, onBack.
 */
export default function QuickReceipt({
  side,
  receipt,
  from,
  to,
  files,
  stopped = false,
  peerOffline = false,
  acknowledgedAt,
  onAcknowledge,
  onSendMissing,
  onHistory,
  onDone,
  onBack,
}: {
  side: Side;
  receipt: Receipt;
  from: Label;
  to: Label;
  files?: OfferedFile[];
  stopped?: boolean;
  /** The other laptop could not be reached, so it has not seen its own receipt yet (the sender's). */
  peerOffline?: boolean;
  /** When the receiver acknowledged (ISO 8601), once they have. */
  acknowledgedAt?: string;
  onAcknowledge?: () => void;
  onSendMissing?: () => void;
  onHistory: () => void;
  onDone: () => void;
  onBack?: () => void;
}) {
  const { l10n } = useLocalization();
  const bytes = useBytes();
  const language = typeof document === "undefined" ? "en" : document.documentElement.lang;
  const state = receiptState(receipt);
  const t = tally(receipt);
  const missing = missingLines(receipt).length;
  const sizeOf = new Map((files ?? []).map((f) => [f.path, f.size]));
  const arrivedBytes = receipt.lines.reduce((sum, l) => (l.outcome.kind === "arrived" ? sum + (sizeOf.get(l.path) ?? 0) : sum), 0);
  const peer = side === "send" ? to : from;
  const whole = state === "complete";
  return (
    <>
      <QsStrip onBack={onBack} receiving={side === "receive"} />
      <Heading id="qs-receipt-title" />
      <Localized id="qs-receipt-body">
        <p className="subhead" />
      </Localized>

      <section className={`qs-statuscard ${whole ? "qs-statuscard-good" : "qs-statuscard-warn"}`} role="status">
        <h2>
          {whole ? (
            <Seal verified={!stopped}>
              <Check />
            </Seal>
          ) : (
            <Attention />
          )}
          <Localized id={whole ? "qs-receipt-box-title" : "qs-receipt-partial-title"}>
            <span />
          </Localized>
        </h2>
        {whole && (
          <Localized id="qs-receipt-box-body">
            <p />
          </Localized>
        )}
        {state === "partial" && (
          <Localized id="qs-receipt-partial" vars={{ count: missing }}>
            <p />
          </Localized>
        )}
        {state === "empty" && (
          <Localized id="qs-receipt-empty">
            <p />
          </Localized>
        )}
        {stopped && (
          <Localized id="qs-receipt-stopped">
            <p />
          </Localized>
        )}
      </section>

      <div role="status" className="qs-status">
        {peerOffline && (
          <Banner tone="info">
            <Localized id="qs-receipt-offline" vars={{ label: labelText(l10n, peer) }}>
              <p />
            </Localized>
          </Banner>
        )}
        {acknowledgedAt && (
          <Banner tone="info">
            <Localized id="qs-receipt-acked" vars={{ when: formatWhen(acknowledgedAt, language) }}>
              <p />
            </Localized>
          </Banner>
        )}
      </div>

      <div className="qs-metrics">
        <div className="qs-metric">
          <strong>{t.arrived}</strong>
          <span>{l10n.getString("qs-receipt-metric-items", { count: t.arrived })}</span>
        </div>
        <div className="qs-metric">
          <strong>{bytes(arrivedBytes)}</strong>
          <Localized id="qs-receipt-metric-size">
            <span />
          </Localized>
        </div>
      </div>
      {t.alreadyThere > 0 && (
        <ul className="qs-tally">
          <li>{l10n.getString("qs-receipt-already-count", { count: t.alreadyThere })}</li>
        </ul>
      )}

      <QsRowList>
        {receipt.lines.map((line) => (
          <Row
            key={line.path}
            icon={
              <Tile>
                <Folder size={18} />
              </Tile>
            }
            title={<Name>{fileName(line.path)}</Name>}
            detail={<LineDetail line={line} side={side} stopped={stopped} />}
            end={<LineEnd outcome={line.outcome} stopped={stopped} />}
          />
        ))}
      </QsRowList>

      {side === "send" && missing > 0 && onSendMissing && (
        <ul className="choices">
          <li>
            <button type="button" className="choice" onClick={onSendMissing}>
              <span className="choice-icon">
                <Folder />
              </span>
              <span className="choice-text">
                <Localized id="qs-receipt-missing">
                  <span className="choice-title" />
                </Localized>
                <Localized id="qs-receipt-missing-body">
                  <span className="choice-body" />
                </Localized>
              </span>
              <ArrowForward />
            </button>
          </li>
        </ul>
      )}

      <QsActions>
        {side === "receive" && onAcknowledge && !acknowledgedAt && (
          <Button onClick={onAcknowledge}>
            <Localized id="qs-receipt-ack">
              <span />
            </Localized>
          </Button>
        )}
        <Button onClick={onHistory}>
          <Localized id="qs-receipt-history">
            <span />
          </Localized>
        </Button>
      </QsActions>

      <div className="note">
        <Info />
        <div>
          <Localized id="qs-receipt-not-opened">
            <p className="note-text" />
          </Localized>
          {side === "receive" && (
            <Localized id="qs-receipt-ack-note">
              <p className="note-text" />
            </Localized>
          )}
        </div>
      </div>

      <FrameFooter note="send">
        <Button variant="primary" onClick={onDone}>
          <Localized id="qs-receipt-done">
            <span />
          </Localized>
        </Button>
      </FrameFooter>
    </>
  );
}
