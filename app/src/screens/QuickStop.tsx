import { Localized, useLocalization } from "@fluent/react";
import { useState, type ReactNode } from "react";
import { Badge, Banner, Button, Checkbox, Dialog, Row } from "../components/kit";
import { QsRowList } from "./QsParts";

export type StopKind = "stop" | "decline";

/**
 * Q20 Stop or decline: a dialog that says what stays and what does not before anything stops. Closing
 * it (Keep sending, Escape) keeps everything going. Stopping a send needs the person to tick that files
 * which already arrived are kept, and focus starts on the first sentence, never on the stop button.
 * Declining an offer is plainer: nothing was saved, nothing is sent. `finished` is true when the send
 * ended while the dialog was open; then there is nothing left to stop.
 * Events: onConfirm, onClose.
 */
export default function QuickStop(props: StopProps) {
  // Each time the dialog opens the person starts from an untouched tick: opening is a fresh component.
  return <StopInner key={props.open ? "open" : "closed"} {...props} />;
}

type StopProps = {
  open: boolean;
  kind: StopKind;
  finished?: boolean;
  onConfirm: () => void;
  onClose: () => void;
};

function StopInner({
  open,
  kind,
  finished = false,
  onConfirm,
  onClose,
}: StopProps) {
  const { l10n } = useLocalization();
  const [understood, setUnderstood] = useState(false);

  if (kind === "decline") {
    return (
      <StopDialog open={open} title="qs-decline-title" onClose={onClose} confirm="qs-decline-confirm" onConfirm={onConfirm}>
        <Localized id="qs-decline-body">
          <p data-autofocus tabIndex={-1} />
        </Localized>
      </StopDialog>
    );
  }
  if (finished) {
    return (
      <StopDialog open={open} title="qs-stop-title" onClose={onClose}>
        <Banner tone="info">
          <Localized id="qs-already-finished">
            <p data-autofocus tabIndex={-1} />
          </Localized>
        </Banner>
      </StopDialog>
    );
  }
  return (
    <StopDialog open={open} title="qs-stop-title" onClose={onClose} confirm="qs-stop-confirm" onConfirm={onConfirm} disabled={!understood} keep>
      <Localized id="qs-stop-body">
        <p data-autofocus tabIndex={-1} />
      </Localized>
      <QsRowList>
        {(["arrived", "rest", "partial"] as const).map((part) => (
          <Row
            key={part}
            title={l10n.getString(`qs-stop-${part}-title`)}
            detail={l10n.getString(`qs-stop-${part}-body`)}
            end={<Badge tone="neutral">{l10n.getString(`qs-stop-status-${part === "arrived" ? "keep" : part === "rest" ? "stop" : "review"}`)}</Badge>}
          />
        ))}
      </QsRowList>
      <Checkbox
        checked={understood}
        onChange={setUnderstood}
        label={
          <Localized id="qs-stop-ack">
            <span />
          </Localized>
        }
        hint={
          <Localized id="qs-stop-ack-hint">
            <span />
          </Localized>
        }
      />
    </StopDialog>
  );
}

/** The kit dialog with this screen's words: a title key, an optional confirm button, and "Keep sending" as the safe way out when stopping. */
function StopDialog({
  open,
  title,
  onClose,
  confirm,
  onConfirm,
  disabled = false,
  keep = false,
  children,
}: {
  open: boolean;
  title: string;
  onClose: () => void;
  confirm?: string;
  onConfirm?: () => void;
  disabled?: boolean;
  keep?: boolean;
  children: ReactNode;
}) {
  return (
    <Dialog
      open={open}
      onClose={onClose}
      title={
        <Localized id={title}>
          <span />
        </Localized>
      }
      closeLabel={
        keep ? (
          <Localized id="qs-stop-keep">
            <span />
          </Localized>
        ) : undefined
      }
      actions={
        confirm && onConfirm ? (
          <Button variant="primary" disabled={disabled} onClick={onConfirm}>
            <Localized id={confirm}>
              <span />
            </Localized>
          </Button>
        ) : undefined
      }
    >
      {children}
    </Dialog>
  );
}
