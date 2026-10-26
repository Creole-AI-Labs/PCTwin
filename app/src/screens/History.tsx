import { Localized, useLocalization } from "@fluent/react";
import { Badge, Banner, Button, Row } from "../components/kit";
import { Attention, Check, Info } from "../icons";
import { formatWhen, type HistoryEntry } from "../quicksend";
import FrameFooter from "./FrameFooter";
import { Folder, Heading, Name, QsActions, QsRowList, QsStrip, Tile } from "./QsParts";

export type ExportResult = { kind: "saved"; path: string } | { kind: "failed" };

function tone(state: HistoryEntry["state"]): "good" | "warn" | "neutral" {
  if (state === "finished") return "good";
  if (state === "paused") return "neutral";
  return "warn";
}

/**
 * F08 My PCTwin history: the moves and Quick Sends done on this laptop, newest first, each with its
 * report or receipt, and Start a full move as the main action. Kept on this laptop only: no account,
 * nothing sent anywhere. A state says "Finished" only when the bridge says the whole thing finished
 * (for a Quick Send that is a receipt with every file in place, see historyStateOf). Export saves a
 * plain copy of the list; the bridge says where it went (or that it could not).
 * Events: onOpen(id), onContinue(id) for a paused move, onExport, onStart, onBack.
 */
export default function History({
  entries,
  exported,
  onOpen,
  onContinue,
  onExport,
  onStart,
  onBack,
}: {
  entries: HistoryEntry[];
  exported?: ExportResult;
  onOpen: (id: string) => void;
  onContinue: (id: string) => void;
  onExport: () => void;
  onStart: () => void;
  onBack: () => void;
}) {
  const { l10n } = useLocalization();
  const language = typeof document === "undefined" ? "en" : document.documentElement.lang;
  const sorted = [...entries].sort((a, b) => Date.parse(b.when) - Date.parse(a.when));
  const title = (e: HistoryEntry) =>
    e.kind === "move"
      ? l10n.getString("history-row-move", { from: e.from, to: e.to })
      : l10n.getString(e.kind === "send" ? "history-row-sent" : "history-row-received", { label: e.peer });
  const status = (e: HistoryEntry) =>
    e.kind === "move" && e.state === "steps-left" ? l10n.getString("history-status-steps-left", { count: e.stepsLeft ?? 0 }) : l10n.getString(`history-status-${e.state}`);
  return (
    <>
      <QsStrip onBack={onBack} />
      <Heading id="history-title" />
      <Localized id="history-body">
        <p className="subhead" />
      </Localized>

      {sorted.length === 0 ? (
        <div className="note">
          <Info />
          <div>
            <Localized id="history-empty-title">
              <strong className="note-text" />
            </Localized>
            <Localized id="history-empty">
              <p className="note-text" />
            </Localized>
          </div>
        </div>
      ) : (
        <QsRowList>
          {sorted.map((e) => (
            <Row
              key={e.id}
              icon={
                <Tile>
                  <Folder size={18} />
                </Tile>
              }
              title={<Name>{title(e)}</Name>}
              detail={<bdi>{formatWhen(e.when, language)}</bdi>}
              end={
                <>
                  <Badge tone={tone(e.state)} icon={e.state === "finished" ? <Check /> : e.state === "paused" ? <Info /> : <Attention />}>
                    {status(e)}
                  </Badge>
                  {e.state === "paused" && (
                    <Button
                      variant="quiet"
                      onClick={() => {
                        onContinue(e.id);
                      }}
                    >
                      {l10n.getString("history-continue")}
                    </Button>
                  )}
                  <Button
                    variant="quiet"
                    aria-label={l10n.getString("history-open-name", { what: l10n.getString(e.kind === "move" ? "history-open-report" : "history-open-files"), title: title(e) })}
                    onClick={() => {
                      onOpen(e.id);
                    }}
                  >
                    {l10n.getString(e.kind === "move" ? "history-open-report" : "history-open-files")}
                  </Button>
                </>
              }
            />
          ))}
        </QsRowList>
      )}

      <div role="status" className="qs-status">
        {exported?.kind === "saved" && (
          <Banner tone="ready">
            <Localized id="history-exported" vars={{ path: exported.path }}>
              <p />
            </Localized>
          </Banner>
        )}
        {exported?.kind === "failed" && (
          <Banner tone="attention">
            <Localized id="history-export-failed">
              <p />
            </Localized>
          </Banner>
        )}
      </div>

      {sorted.length > 0 && (
        <QsActions>
          <Button onClick={onExport}>
            <Localized id="history-export">
              <span />
            </Localized>
          </Button>
        </QsActions>
      )}
      <div className="note">
        <Info />
        <Localized id="history-note">
          <p className="note-text" />
        </Localized>
      </div>

      <FrameFooter note="send">
        <Button variant="primary" onClick={onStart}>
          <Localized id="history-start">
            <span />
          </Localized>
        </Button>
      </FrameFooter>
    </>
  );
}
