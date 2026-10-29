import { useLocalization } from "@fluent/react";
import { useEffect, useId, useState } from "react";
import { listenKnown, toQsRefusal, waitingCancel, waitingRenew, waitingSends, type QsRefusal, type WaitingDelivery, type WaitingSend } from "../bridge";
import { Badge, Banner, Button, Dialog, Select } from "../components/kit";
import { Laptop } from "../icons";
import { useBytes } from "../quicksend";
import { dateText } from "./FinishParts";
import FrameFooter from "./FrameFooter";
import { Heading, Name, QsActions, QsStrip, Tile } from "./QsParts";
import { WAIT_DAYS, type WaitDays } from "./QsRecipients";

/** One recipient's own result, in words; a failure's reason stays behind More details. */
function Delivery({ d, files }: { d: WaitingDelivery; files: number }) {
  const { l10n } = useLocalization();
  switch (d.kind) {
    case "waiting":
      return <Badge>{l10n.getString("qs-waiting-state-waiting")}</Badge>;
    case "delivered":
      return (
        <span>
          <Badge tone={d.needsAction > 0 ? "warn" : "good"}>{l10n.getString("qs-waiting-state-delivered", { arrived: d.arrived, total: files })}</Badge>
          {d.needsAction > 0 && <small className="kit-hint"> {l10n.getString("qs-waiting-state-needs", { count: d.needsAction })}</small>}
          <small className="kit-hint"> {dateText(d.at)}</small>
        </span>
      );
    case "declined":
      return <Badge tone="warn">{l10n.getString("qs-waiting-state-declined", { date: dateText(d.at) })}</Badge>;
    case "failed":
      return (
        <span>
          <Badge tone="warn">{l10n.getString("qs-waiting-state-failed", { count: d.failed })}</Badge>
          {d.why && (
            <details className="qs-more">
              <summary>{l10n.getString("report-more-details")}</summary>
              <p dir="auto">{d.why}</p>
            </details>
          )}
        </span>
      );
  }
}

/** One waiting send: what, until when, each recipient's own result, a newer version, Renew and Cancel. */
function WaitingCard({ w, now, problem, onRenew, onCancel }: { w: WaitingSend; now: number; problem: QsRefusal | null; onRenew: (days: number) => void; onCancel: () => void }) {
  const { l10n } = useLocalization();
  const bytes = useBytes();
  const titleId = useId();
  const [days, setDays] = useState<WaitDays>("7");
  const [cancelling, setCancelling] = useState(false);
  const ended = w.deadline <= now;
  return (
    <li className="qs-card qs-waiting">
      <strong id={titleId} className="qs-card-title">
        {l10n.getString("qs-waiting-files", { count: w.files })} · {bytes(w.bytes)}
      </strong>
      <p className="qs-small">
        {l10n.getString("qs-waiting-approved", { date: dateText(w.madeAt) })} · {l10n.getString(ended ? "qs-waiting-ended" : "qs-waiting-until", { date: dateText(w.deadline) })}
      </p>
      <ul className="qs-recipient-results" aria-labelledby={titleId}>
        {w.recipients.map((r) => (
          <li key={r.connection}>
            <Tile>
              <Laptop className="qs-small-icon" />
            </Tile>
            <span className="qs-known-text">
              <strong>{r.label === null ? l10n.getString("qs-waiting-to-unknown") : <Name>{r.label}</Name>}</strong>
              <Delivery d={r.delivery} files={w.files} />
            </span>
          </li>
        ))}
      </ul>
      {w.newer.length > 0 && (
        <Banner tone="attention">
          {l10n.getString("qs-waiting-newer")}{" "}
          {w.newer.map((n, i) => (
            <span key={n}>
              {i > 0 && ", "}
              <Name>{n}</Name>
            </span>
          ))}
        </Banner>
      )}
      {problem && <Banner tone="attention">{l10n.getString(`qs-refusal-${problem.kind}`)}</Banner>}
      <div className="qs-waiting-renew">
        <Select
          label={l10n.getString("qs-waiting-renew-label")}
          value={days}
          options={WAIT_DAYS.map((d) => ({ value: d, label: l10n.getString("qs-waiting-days", { count: Number(d) }) }))}
          onChange={setDays}
        />
      </div>
      <QsActions>
        <Button
          onClick={() => {
            onRenew(Number(days));
          }}
        >
          {l10n.getString("qs-waiting-renew")}
        </Button>
        <Button
          variant="quiet"
          onClick={() => {
            setCancelling(true);
          }}
        >
          {l10n.getString("qs-waiting-cancel")}
        </Button>
      </QsActions>
      <Dialog
        open={cancelling}
        title={l10n.getString("qs-waiting-cancel-title")}
        initialFocus="close"
        closeLabel={l10n.getString("qs-waiting-keep")}
        onClose={() => {
          setCancelling(false);
        }}
        actions={
          <Button
            variant="primary"
            onClick={() => {
              setCancelling(false);
              onCancel();
            }}
          >
            {l10n.getString("qs-waiting-cancel")}
          </Button>
        }
      >
        <p>{l10n.getString("qs-waiting-cancel-body")}</p>
      </Dialog>
    </li>
  );
}

/**
 * Q18 and Q26: waiting sends, each with its files and size, when it was approved, until when it
 * waits (or that it ended), and each recipient's own result (waiting, copied and checked, declined,
 * or did not copy). A source changed since the approval says "a newer version exists", never swaps
 * it. Renew sets a new deadline; Cancel deletes PCTwin's kept copy (asked once more). Sends only
 * while this laptop is on. `now` is seconds since 1970.
 * Events: onRenew(id, days), onCancel(id), onSend, onBack.
 */
export function WaitingList({
  sends,
  now,
  failed = false,
  problem = null,
  onRenew,
  onCancel,
  onSend,
  onBack,
}: {
  sends: WaitingSend[];
  now: number;
  failed?: boolean;
  problem?: { id: string; refusal: QsRefusal } | null;
  onRenew: (id: string, days: number) => void;
  onCancel: (id: string) => void;
  onSend: () => void;
  onBack: () => void;
}) {
  const { l10n } = useLocalization();
  return (
    <>
      <QsStrip onBack={onBack} />
      <Heading id="qs-waiting-title" />
      <p className="subhead">
        {l10n.getString("qs-waiting-intro")} {l10n.getString("qs-waiting-on")}
      </p>
      {failed && <Banner tone="attention">{l10n.getString("qs-refusal-failed")}</Banner>}
      {!failed && sends.length === 0 && (
        <p role="status" className="qs-count">
          {l10n.getString("qs-waiting-empty")}
        </p>
      )}
      {sends.length > 0 && (
        <ul className="qs-known-list">
          {sends.map((w) => (
            <WaitingCard
              key={w.id}
              w={w}
              now={now}
              problem={problem?.id === w.id ? problem.refusal : null}
              onRenew={(days) => {
                onRenew(w.id, days);
              }}
              onCancel={() => {
                onCancel(w.id);
              }}
            />
          ))}
        </ul>
      )}
      <FrameFooter note="send">
        <Button variant="primary" onClick={onSend}>
          {l10n.getString("qs-home-action")}
        </Button>
      </FrameFooter>
    </>
  );
}

/** The waiting sends page with the engine: asked once, kept current by `waiting-changed`. */
export default function WaitingHost({ onSend, onBack }: { onSend: () => void; onBack: () => void }) {
  const [sends, setSends] = useState<WaitingSend[]>([]);
  const [failed, setFailed] = useState(false);
  const [problem, setProblem] = useState<{ id: string; refusal: QsRefusal } | null>(null);
  const [now] = useState(() => Math.floor(Date.now() / 1000));

  useEffect(() => {
    let live = true;
    let unlisten: (() => void) | null = null;
    waitingSends().then(
      (list) => {
        if (live) setSends(list);
      },
      () => {
        if (live) setFailed(true);
      },
    );
    void listenKnown((event) => {
      if (live && event.kind === "waiting") setSends(event.sends);
    }).then((stop) => {
      if (live) unlisten = stop;
      else stop();
    });
    return () => {
      live = false;
      unlisten?.();
    };
  }, []);

  const act = (id: string, command: () => Promise<WaitingSend[]>) => {
    setProblem(null);
    command().then(setSends, (error: unknown) => {
      setProblem({ id, refusal: toQsRefusal(error) });
    });
  };

  return (
    <WaitingList
      sends={sends}
      now={now}
      failed={failed}
      problem={problem}
      onSend={onSend}
      onBack={onBack}
      onRenew={(id, days) => {
        act(id, () => waitingRenew(id, days));
      }}
      onCancel={(id) => {
        act(id, () => waitingCancel(id));
      }}
    />
  );
}
