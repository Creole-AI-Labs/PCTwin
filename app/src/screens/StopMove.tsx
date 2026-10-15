import { useLocalization } from "@fluent/react";
import { useId, useState } from "react";
import { Button, Checkbox, Dialog, Row, RowList } from "../components/kit";
import { ArrowForward, Attention, Check, Laptop } from "../icons";

/**
 * X10 Stop the rest of the move? A danger dialog: what stays, what is not copied, that the old
 * laptop is untouched, and that stopping is not undo. The stop button stays off until the person
 * ticks that they understand; the dialog opens on the safe choice, and the tick is forgotten each
 * time it opens.
 */
export default function StopMove({ open, onStop, onKeep }: { open: boolean; onStop: () => void; onKeep: () => void }) {
  const { l10n } = useLocalization();
  const [understood, setUnderstood] = useState(false);
  const hint = useId();
  // The tick is forgotten whenever the dialog closes (adjusted while rendering, not in an effect).
  const [wasOpen, setWasOpen] = useState(open);
  if (wasOpen !== open) {
    setWasOpen(open);
    if (!open) setUnderstood(false);
  }
  return (
    <Dialog
      open={open}
      title={l10n.getString("cancel-title")}
      onClose={onKeep}
      closeLabel={l10n.getString("cancel-keep")}
      initialFocus="close"
      actions={
        <Button variant="primary" className="mv-danger-fill" disabled={!understood} aria-describedby={hint} onClick={onStop}>
          {l10n.getString("cancel-stop")}
        </Button>
      }
    >
      <p>{l10n.getString("cancel-body")}</p>
      <RowList>
        <Row icon={<Check className="mv-icon" />} title={l10n.getString("cancel-copied-title")} detail={l10n.getString("cancel-copied-body")} />
        <Row icon={<ArrowForward className="mv-icon" />} title={l10n.getString("cancel-rest-title")} detail={l10n.getString("cancel-rest-body")} />
        <Row icon={<Attention className="mv-icon" />} title={l10n.getString("cancel-partial-title")} detail={l10n.getString("cancel-partial-body")} />
        <Row icon={<Laptop className="mv-icon" />} title={l10n.getString("cancel-old-title")} detail={l10n.getString("cancel-old-body")} />
      </RowList>
      <div className="mv-confirm">
        <Checkbox checked={understood} onChange={setUnderstood} label={l10n.getString("cancel-ack")} />
      </div>
      <p id={hint} className="kit-hint mv-hint">
        {l10n.getString("cancel-ack-hint")}
      </p>
    </Dialog>
  );
}
