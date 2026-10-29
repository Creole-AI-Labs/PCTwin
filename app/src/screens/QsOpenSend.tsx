import { useLocalization } from "@fluent/react";
import type { QsSession } from "../bridge";
import { Button } from "../components/kit";
import { QsActions } from "./QsParts";

/**
 * An open send's own buttons (Security Design T, 29 October 2026): after the first files went, the
 * send with that laptop stays open. The sender's Add files and the receiver's Send to <name> each
 * make a new offer in it, which the other laptop is asked about like the first; End closes it, and
 * nothing more goes or arrives. An ending send says so and offers nothing.
 *
 * `sessions`: the open sends this page belongs to. A send made with the code has no name from the
 * engine: `peer` names it as the page does ("Receiving laptop", or the name pairing showed).
 * `side`: "send" shows Add files, "receive" shows Send to <name>.
 */
export default function OpenSendActions({
  sessions,
  peer,
  side,
  onAdd,
  onEnd,
}: {
  sessions: QsSession[];
  peer: string;
  side: "send" | "receive";
  onAdd: (session: QsSession) => void;
  onEnd: (session: QsSession) => void;
}) {
  const { l10n } = useLocalization();
  if (sessions.length === 0) return null;
  const several = sessions.length > 1;
  return (
    <div className="qs-open-send" data-slot="open-send">
      {sessions.map((s) => {
        const name = s.name ?? peer;
        const add = side === "send" ? l10n.getString("qs-session-add-files") : l10n.getString("qs-session-send-to", { name });
        const end = l10n.getString("qs-session-end");
        if (s.ending)
          return (
            <p key={s.id} className="qs-small" role="status">
              {several ? l10n.getString("qs-session-ended-name", { name }) : l10n.getString("qs-session-ended")}
            </p>
          );
        return (
          <QsActions key={s.id}>
            {several && (
              <span className="qs-open-send-name" dir="auto">
                {name}
              </span>
            )}
            <Button
              aria-label={several && side === "send" ? l10n.getString("qs-sends-action-name", { what: add, name }) : undefined}
              onClick={() => {
                onAdd(s);
              }}
            >
              {add}
            </Button>
            <Button
              variant="quiet"
              aria-label={l10n.getString("qs-sends-action-name", { what: end, name })}
              onClick={() => {
                onEnd(s);
              }}
            >
              {end}
            </Button>
          </QsActions>
        );
      })}
    </div>
  );
}
