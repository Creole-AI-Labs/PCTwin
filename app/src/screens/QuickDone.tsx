import { Localized, useLocalization } from "@fluent/react";
import { Banner, Button, ResultChip, Row } from "../components/kit";
import { ArrowForward, Attention, Check, Info } from "../icons";
import { fileName, missingLines, receiptState, tally, useBytes, type OfferedFile, type Outcome, type Receipt, type ReceiptLine } from "../quicksend";
import Seal from "../motion/Seal";
import FrameFooter from "./FrameFooter";
import { Folder, Heading, Name, QsRowList, QsStrip, Tile } from "./QsParts";

type Side = "send" | "receive";

/** Whether a line ended with the file in place and checked (it arrived, or an identical one was there). */
const inPlace = (o: Outcome) => o.kind === "arrived" || o.kind === "already-there";

/** What happened to a file that needs attention, in plain words; the engine's reason stays behind More details. */
function Trouble({ line, stopped }: { line: ReceiptLine; stopped: boolean }) {
  const { l10n } = useLocalization();
  const { outcome } = line;
  const words =
    outcome.kind === "needs-action"
      ? outcome.word
        ? `qs-done-${outcome.word}`
        : "qs-done-needs-action"
      : stopped
        ? "qs-done-not-completed"
        : "qs-done-not-arrived";
  const why = outcome.kind === "needs-action" || outcome.kind === "failed" ? outcome.why : null;
  return (
    <>
      {l10n.getString(words)}
      {why && (
        <details className="qs-more">
          <summary>{l10n.getString("report-more-details")}</summary>
          <p dir="auto">{why}</p>
        </details>
      )}
    </>
  );
}

/**
 * The end of a Quick Send, on either side, and a past one opened from history. It says plainly
 * whether everything arrived: one line with the count and size that arrived and matched the
 * originals, the receiver's folder with Open folder, and only when something did not arrive or
 * needs a step, those files with what to do (the engine's reason behind More details). It never
 * says the files were sent, or are here, unless every file ended in place and checked. The sender
 * can send only the missing files again. `files` (the offer) gives the sizes; without it (history)
 * only the count is said.
 * Events: onOpenFolder (receiver), onSendMissing (sender), onDone, onBack.
 */
export default function QuickDone({
  side,
  receipt,
  files,
  stopped = false,
  folder,
  openFailed = false,
  onOpenFolder,
  onSendMissing,
  onDone,
  onBack,
}: {
  side: Side;
  receipt: Receipt;
  files?: OfferedFile[];
  stopped?: boolean;
  /** The receiver's folder, by its name. */
  folder?: string | null;
  /** Open folder was pressed and the folder could not be opened. */
  openFailed?: boolean;
  onOpenFolder?: () => void;
  onSendMissing?: () => void;
  onDone: () => void;
  onBack?: () => void;
}) {
  const { l10n } = useLocalization();
  const bytes = useBytes();
  const state = receiptState(receipt);
  const whole = state === "complete";
  const t = tally(receipt);
  const missing = missingLines(receipt).length;
  const trouble = receipt.lines.filter((l) => !inPlace(l.outcome));
  const there = receipt.lines.filter((l) => inPlace(l.outcome));
  const sizeOf = files ? new Map(files.map((f) => [f.path, f.size])) : null;
  const arrivedBytes = sizeOf ? receipt.lines.reduce((sum, l) => (l.outcome.kind === "arrived" ? sum + (sizeOf.get(l.path) ?? 0) : sum), 0) : null;
  const title = stopped
    ? "qs-done-stopped-title"
    : whole
      ? side === "send"
        ? "qs-done-sent-title"
        : "qs-done-here-title"
      : state === "empty"
        ? "qs-done-empty-title"
        : "qs-done-partial-title";
  return (
    <>
      <QsStrip onBack={onBack} receiving={side === "receive"} />
      <div className="qs-done-mark">
        {whole && !stopped ? (
          <Seal verified>
            <Check />
          </Seal>
        ) : (
          <Attention />
        )}
      </div>
      <Heading id={title} />

      <div role="status" className="qs-status">
        {t.arrived > 0 && (
          <p className="subhead">
            {arrivedBytes === null
              ? l10n.getString("qs-done-arrived-count", { count: t.arrived })
              : l10n.getString("qs-done-arrived", { count: t.arrived, size: bytes(arrivedBytes) })}
          </p>
        )}
        {t.alreadyThere > 0 && <p className="qs-small">{l10n.getString("qs-done-already", { count: t.alreadyThere })}</p>}
        {stopped && (
          <Localized id="qs-done-stopped-body">
            <p className="qs-small" />
          </Localized>
        )}
        {openFailed && (
          <Banner tone="attention">
            <Localized id="qs-done-open-failed">
              <p />
            </Localized>
          </Banner>
        )}
      </div>

      {side === "receive" && folder && (
        <div className="qs-folder-line">
          <Tile>
            <Folder size={18} />
          </Tile>
          <span dir="auto">{l10n.getString("qs-done-folder", { folder })}</span>
          {onOpenFolder && (
            <Button onClick={onOpenFolder}>
              <Localized id="qs-done-open-folder">
                <span />
              </Localized>
            </Button>
          )}
        </div>
      )}

      {trouble.length > 0 && (
        <section aria-labelledby="qs-done-trouble">
          <Localized id="qs-done-problems-title">
            <h2 id="qs-done-trouble" className="qs-section-title" />
          </Localized>
          <QsRowList>
            {trouble.map((line) => (
              <Row
                key={line.path}
                title={<Name>{fileName(line.path)}</Name>}
                detail={<Trouble line={line} stopped={stopped} />}
                end={<ResultChip result={line.outcome.kind === "needs-action" ? "needs-action" : "failed"} />}
              />
            ))}
          </QsRowList>
        </section>
      )}

      {there.length > 0 && (
        <details className="qs-more qs-done-files">
          <Localized id="qs-done-see-files">
            <summary />
          </Localized>
          <ul>
            {there.map((line) => (
              <li key={line.path}>
                <Name>{fileName(line.path)}</Name>
                {line.outcome.kind === "already-there" && <span className="qs-small"> · {l10n.getString("qs-done-identical")}</span>}
                {line.outcome.kind === "arrived" && side === "receive" && fileName(line.outcome.landedAs) !== fileName(line.path) && (
                  <span className="qs-small"> · {l10n.getString("qs-done-renamed", { name: fileName(line.outcome.landedAs) })}</span>
                )}
              </li>
            ))}
          </ul>
        </details>
      )}

      {side === "send" && missing > 0 && onSendMissing && (
        <ul className="choices">
          <li>
            <button type="button" className="choice" onClick={onSendMissing}>
              <span className="choice-icon">
                <Folder />
              </span>
              <span className="choice-text">
                <Localized id="qs-done-send-missing">
                  <span className="choice-title" />
                </Localized>
                <Localized id="qs-done-send-missing-body">
                  <span className="choice-body" />
                </Localized>
              </span>
              <ArrowForward />
            </button>
          </li>
        </ul>
      )}

      {side === "receive" && (
        <div className="note">
          <Info />
          <Localized id="qs-done-not-opened">
            <p className="note-text" />
          </Localized>
        </div>
      )}

      <FrameFooter note="send">
        <Button variant="primary" onClick={onDone}>
          <Localized id="qs-done-done">
            <span />
          </Localized>
        </Button>
      </FrameFooter>
    </>
  );
}
