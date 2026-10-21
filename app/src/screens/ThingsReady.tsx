import { useLocalization } from "@fluent/react";
import { Badge, Banner, Button, Metrics, Row, RowList } from "../components/kit";
import type { ReadyNote } from "../bridge";
import { Check } from "../icons";
import { pageLocale } from "../move";
import FrameFooter from "./FrameFooter";
import { TopStrip } from "./FrameParts";
import { Heading } from "./ProgressParts";
import "./progress.css";

/**
 * M32 Your things are ready: shown once to a person whose things came from the old laptop in
 * someone else's move, after the hand-over at their own sign-in. Counts only: how many are in
 * their own folders, how many were kept aside (a file of that name was already there), and how
 * many could not be handed over. Nothing of theirs is shown to anyone else.
 */
export default function ThingsReady({ note, onDone }: { note: ReadyNote; onDone: () => void }) {
  const { l10n } = useLocalization();
  const n = (v: number) => v.toLocaleString(pageLocale());
  const whole = note.kept === 0 && note.failed === 0;
  return (
    <>
      <TopStrip labelId="ready-eyebrow" />
      <Heading id="ready-title" />
      <p className="subhead">{l10n.getString("ready-body")}</p>
      <Metrics items={[{ value: n(note.landed), label: l10n.getString("ready-landed") }]} />
      <RowList>
        <Row
          title={l10n.getString("ready-where-title")}
          detail={l10n.getString("ready-where-body")}
          end={
            whole ? (
              <Badge tone="good" icon={<Check className="mv-icon" />}>
                {l10n.getString("progress-state-verified")}
              </Badge>
            ) : undefined
          }
        />
      </RowList>
      {!whole && (
        <div className="mv-note">
          <Banner tone="attention">
            {note.kept > 0 && <p>{l10n.getString("ready-kept", { count: note.kept })}</p>}
            {note.failed > 0 && <p>{l10n.getString("ready-failed", { count: note.failed })}</p>}
          </Banner>
        </div>
      )}
      <FrameFooter>
        <Button variant="primary" onClick={onDone}>
          {l10n.getString("ready-done")}
        </Button>
      </FrameFooter>
    </>
  );
}
