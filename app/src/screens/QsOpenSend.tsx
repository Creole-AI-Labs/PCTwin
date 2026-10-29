import { useLocalization } from "@fluent/react";
import { Button } from "../components/kit";
import { QsActions } from "./QsParts";

/**
 * The place on an open send for its own two buttons (Security Design T, 29 October 2026): the
 * sender's Add files and the receiver's Send to <name>, each a new offer inside the same paired
 * session that the other side is asked about like the first. The engine part is being built: a
 * button shows only once its handler is passed, so until then this renders nothing.
 */
export default function OpenSendActions({ peer, onAddFiles, onSendTo }: { peer: string; onAddFiles?: () => void; onSendTo?: () => void }) {
  const { l10n } = useLocalization();
  if (!onAddFiles && !onSendTo) return null;
  return (
    <div className="qs-open-send" data-slot="open-send">
      <QsActions>
        {onAddFiles && <Button onClick={onAddFiles}>{l10n.getString("qs-add-files")}</Button>}
        {onSendTo && <Button onClick={onSendTo}>{l10n.getString("qs-send-to", { name: peer })}</Button>}
      </QsActions>
    </div>
  );
}
