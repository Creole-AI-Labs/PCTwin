import { useLocalization } from "@fluent/react";
import type { ReactNode } from "react";
import type { QsRefusal, QsSend } from "../bridge";
import { Badge, Banner, Button, Row } from "../components/kit";
import { Attention, Check, Clock } from "../icons";
import type { OpenSends } from "../openSends";
import { receiptState, tally, useBytes } from "../quicksend";
import FrameFooter from "./FrameFooter";
import { Heading, Name, QsRowList, QsStrip } from "./QsParts";

/** A laptop that could not be sent to, and why. */
export type NotSent = { name: string; refusal: QsRefusal };

/**
 * Your sends: every send going on now (several laptops at once). The engine sends one after another
 * to the same laptop and to up to three laptops at the same time; the others wait their turn. Each
 * says which laptop it is for, how many files and how big, and whether it is waiting, being sent or
 * paused, with Pause or Resume and Cancel. A send that ended shows its result by the receiving
 * laptop's own count ("5 of 5 copied and checked"), and More details opens it. Nothing here says a
 * send arrived unless its result says so.
 *
 * `peer`: the name for a send made with the code (the engine gives none). `openSend`: the open
 * sends' own buttons (Add files, End). Events: onPause, onResume, onCancel (by send id), onOpen (a
 * result, by send id), onDone.
 */
export default function QsSends({
  open,
  peer,
  notSent = [],
  failed = false,
  openSend,
  onPause,
  onResume,
  onCancel,
  onOpen,
  onDone,
}: {
  open: OpenSends;
  peer: string;
  notSent?: NotSent[];
  /** A Pause, Resume or Cancel the engine did not take. */
  failed?: boolean;
  openSend?: ReactNode;
  onPause: (id: number) => void;
  onResume: (id: number) => void;
  onCancel: (id: number) => void;
  onOpen: (send: number) => void;
  onDone: () => void;
}) {
  const { l10n } = useLocalization();
  const bytes = useBytes();
  const nameOf = (s: { name: string | null }) => s.name ?? peer;
  const state = (s: QsSend) => (s.paused ? "paused" : s.state);
  const action = (what: string, name: string) => l10n.getString("qs-sends-action-name", { what, name });
  const empty = open.sends.length === 0 && open.results.length === 0;
  return (
    <>
      <QsStrip />
      <Heading id="qs-sends-title" />
      <p className="subhead">{l10n.getString("qs-sends-intro")}</p>

      <div role="status">
        {notSent.map((n) => (
          <Banner key={n.name} tone="attention">
            {l10n.getString("qs-sends-not-sent", { name: n.name })} {l10n.getString(`qs-refusal-${n.refusal.kind}`)}
          </Banner>
        ))}
        {failed && <Banner tone="attention">{l10n.getString("qs-sends-failed")}</Banner>}
      </div>

      {empty && <p>{l10n.getString("qs-sends-empty")}</p>}

      {open.sends.length > 0 && (
        <QsRowList>
          {open.sends.map((s) => {
            const name = nameOf(s);
            const now = state(s);
            return (
              <Row
                key={s.id}
                icon={now === "sending" ? undefined : <Clock />}
                title={<Name>{name}</Name>}
                detail={`${l10n.getString("qs-tally-files", { count: s.files, size: bytes(s.bytes) })} · ${l10n.getString(`qs-sends-${now}`)}`}
                end={
                  <>
                    {s.paused ? (
                      <Button
                        variant="quiet"
                        aria-label={action(l10n.getString("qs-send-resume"), name)}
                        onClick={() => {
                          onResume(s.id);
                        }}
                      >
                        {l10n.getString("qs-send-resume")}
                      </Button>
                    ) : (
                      <Button
                        variant="quiet"
                        aria-label={action(l10n.getString("qs-send-pause"), name)}
                        onClick={() => {
                          onPause(s.id);
                        }}
                      >
                        {l10n.getString("qs-send-pause")}
                      </Button>
                    )}
                    <Button
                      variant="quiet"
                      aria-label={action(l10n.getString("qs-send-cancel"), name)}
                      onClick={() => {
                        onCancel(s.id);
                      }}
                    >
                      {l10n.getString("qs-send-cancel")}
                    </Button>
                  </>
                }
              />
            );
          })}
        </QsRowList>
      )}

      {open.results.length > 0 && (
        <>
          <h2 className="qs-section-title">{l10n.getString("qs-sends-ended-title")}</h2>
          <QsRowList>
            {open.results.map((r) => {
              const name = nameOf(r);
              const t = tally({ lines: r.receipt.lines });
              // In place and checked: arrived, or the same file was there already.
              const arrived = t.arrived + t.alreadyThere;
              const whole = receiptState({ lines: r.receipt.lines }) === "complete" && !r.receipt.stopped;
              return (
                <Row
                  key={r.send}
                  title={<Name>{name}</Name>}
                  detail={
                    r.receipt.stopped
                      ? `${l10n.getString("qs-known-delivered-counts", { arrived, total: t.total })} ${l10n.getString("qs-sends-stopped")}`
                      : l10n.getString("qs-known-delivered-counts", { arrived, total: t.total })
                  }
                  end={
                    <>
                      <Badge tone={whole ? "good" : "warn"} icon={whole ? <Check /> : <Attention />}>
                        {l10n.getString(whole ? "qs-sends-result-whole" : "qs-sends-result-partial")}
                      </Badge>
                      <Button
                        variant="quiet"
                        aria-label={action(l10n.getString("report-more-details"), name)}
                        onClick={() => {
                          onOpen(r.send);
                        }}
                      >
                        {l10n.getString("report-more-details")}
                      </Button>
                    </>
                  }
                />
              );
            })}
          </QsRowList>
        </>
      )}

      {openSend}

      <FrameFooter note="send">
        <Button variant="primary" onClick={onDone}>
          {l10n.getString("qs-done-done")}
        </Button>
      </FrameFooter>
    </>
  );
}
