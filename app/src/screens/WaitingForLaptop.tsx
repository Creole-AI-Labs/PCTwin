import { Localized } from "@fluent/react";
import { Badge, Banner, Button, ProgressBar, ProgressValue, Row, RowList } from "../components/kit";
import type { WaitingState } from "../connection";
import { Attention, Info, Laptop, Shield } from "../icons";
import { useAgo, useUnits } from "./ConnectionParts";
import FrameFooter from "./FrameFooter";
import { TopStrip } from "./FrameParts";
import { Heading } from "./PairParts";
import "./connection.css";
import { shownName } from "../names";

/**
 * C09 Waiting for your other laptop. Three states: reconnecting by itself, the pairing has expired
 * (pair again), and the other laptop cannot be reached (try now): then nothing new has arrived for
 * a while, which is said, with Stop. What is saved is measured, and nothing here says the move is
 * finished. Laid out as in the mockup: the saved share, the other laptop, the resume check, a note,
 * and the actions. `onRetry` is left out where this laptop does not do the trying (the old laptop
 * waits for the new one to come back); `onStop` stops the move, and is left out once it has ended.
 */
export default function WaitingForLaptop({
  waiting,
  onRetry,
  onPairAgain,
  onStop,
  onBack,
}: {
  waiting: WaitingState;
  onRetry?: () => void;
  onPairAgain: () => void;
  onStop?: () => void;
  onBack?: () => void;
}) {
  const units = useUnits();
  const ago = useAgo();
  const { kind } = waiting;
  const fraction = waiting.totalGb > 0 ? waiting.savedGb / waiting.totalGb : 0;
  return (
    <>
      <TopStrip onBack={onBack} labelId="connect-eyebrow" />
      <Heading key={kind} id="reconnect-title" />
      <Localized id="reconnect-body">
        <p className="subhead" />
      </Localized>
      <div className="connect-saved connect-panel">
        <div className="connect-saved-head">
          <Localized id="reconnect-card-title">
            <h2 />
          </Localized>
          <ProgressValue value={fraction} />
        </div>
        <Localized id="reconnect-progress" attrs={{ "aria-label": true }}>
          <SavedBar value={fraction} saved={units.gb(waiting.savedGb)} total={units.gb(waiting.totalGb)} />
        </Localized>
      </div>
      <div className="connect-panel">
      <RowList>
        <Row
          icon={<Laptop />}
          title={<bdi>{shownName(waiting.otherLaptop)}</bdi>}
          detail={
            waiting.lastSeenSeconds === null ? undefined : (
              <Localized id="reconnect-last-seen" vars={{ ago: ago(waiting.lastSeenSeconds) }}>
                <span />
              </Localized>
            )
          }
          end={
            kind === "reconnecting" ? (
              <Badge tone="warn" icon={<Attention />}>
                <Localized id="reconnect-status-trying">
                  <span />
                </Localized>
              </Badge>
            ) : (
              <Badge tone="neutral" icon={<Info />}>
                <Localized id="reconnect-status-offline">
                  <span />
                </Localized>
              </Badge>
            )
          }
        />
        <Row
          icon={<Shield />}
          title={
            <Localized id="reconnect-details">
              <span />
            </Localized>
          }
          detail={
            <Localized id="reconnect-details-body">
              <span />
            </Localized>
          }
          end={
            kind === "pairing-expired" ? undefined : (
              <Badge tone="warn" icon={<Attention />}>
                <Localized id="reconnect-status-pending">
                  <span />
                </Localized>
              </Badge>
            )
          }
        />
      </RowList>
      </div>
      <div className="connect-notice">
        {kind === "reconnecting" && (
          <div role="status">
            <Banner tone="info">
              <Localized id="reconnect-trying">
                <p />
              </Localized>
            </Banner>
          </div>
        )}
        {kind === "pairing-expired" && (
          <Banner tone="attention">
            <Localized id="reconnect-expired">
              <p />
            </Localized>
          </Banner>
        )}
        {kind === "unreachable" && (
          <Banner tone="attention">
            <Localized id="reconnect-unavailable" vars={{ label: waiting.otherLaptop }}>
              <p />
            </Localized>
            <Localized id="reconnect-nothing-new">
              <p />
            </Localized>
          </Banner>
        )}
      </div>
      <FrameFooter>
        {kind !== "pairing-expired" && onStop && (
          <Button variant="quiet" onClick={onStop}>
            <Localized id="move-stop">
              <span />
            </Localized>
          </Button>
        )}
        {kind === "pairing-expired" ? (
          <Button variant="primary" onClick={onPairAgain}>
            <Localized id="reconnect-pair-again">
              <span />
            </Localized>
          </Button>
        ) : (
          onRetry && (
            <Button variant="primary" onClick={onRetry}>
              <Localized id="reconnect-try-now">
                <span />
              </Localized>
            </Button>
          )
        )}
      </FrameFooter>
    </>
  );
}

/** The saved share of the move (measured), named by the translated aria-label that the Localized wrapper hands in. */
function SavedBar({ value, saved, total, "aria-label": label }: { value: number; saved: string; total: string; "aria-label"?: string }) {
  return (
    <ProgressBar
      value={value}
      label={label ?? ""}
      left={
        <Localized id="reconnect-saved" vars={{ done: saved, total }}>
          <span />
        </Localized>
      }
    />
  );
}
