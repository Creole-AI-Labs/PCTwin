import { useLocalization } from "@fluent/react";
import { undoPreview } from "../bridge";
import { Banner, Button, Row, RowList } from "../components/kit";
import { listText } from "../move";
import { FinishPage, Loaded, useLoad } from "./FinishParts";
import FrameFooter from "./FrameFooter";

/**
 * R05 Review before undo: what undo removes, what it puts back, and what it keeps (apps that are
 * now the person's own, anything changed after the move), with the limit of each. Starting it hands
 * over to the existing undo (R06), which asks the engine again before it does anything.
 */
export default function FinishUndo({ id, onBack, onStart }: { id: string | null; onBack: () => void; onStart: (undoId: string) => void }) {
  const { l10n } = useLocalization();
  const { state, reload } = useLoad(() => undoPreview(id));
  return (
    <FinishPage titleId="finish-undo-title" onBack={onBack}>
      <p className="subhead">{l10n.getString("finish-undo-body")}</p>
      <Loaded state={state} onRetry={reload}>
        {(view) => {
          const nothing = view.undoId === null;
          return (
            <>
              <div role="status">
                {!view.open && <Banner tone="attention">{l10n.getString("finish-undo-closed")}</Banner>}
                {view.open && nothing && <Banner tone="info">{l10n.getString("finish-undo-nothing")}</Banner>}
              </div>
              <RowList>
                {view.appsRemoved.length > 0 && <Row title={l10n.getString("finish-undo-row-removes")} detail={`${listText(view.appsRemoved)}. ${l10n.getString("finish-undo-limit-apps")}`} />}
                {view.appsYours.length > 0 && <Row title={l10n.getString("finish-undo-row-yours")} detail={listText(view.appsYours)} />}
                {view.settings > 0 && <Row title={l10n.getString("finish-undo-row-settings", { count: view.settings.toLocaleString() })} />}
                {view.appData.length > 0 && <Row title={l10n.getString("finish-undo-row-appdata")} detail={listText(view.appData)} />}
                {view.files > 0 && <Row title={l10n.getString("finish-undo-row-files", { count: view.files.toLocaleString() })} detail={l10n.getString("finish-undo-limit-files")} />}
              </RowList>
              <p>{l10n.getString("finish-undo-keeps")}</p>
              <FrameFooter>
                <Button
                  variant="primary"
                  disabled={!view.open || view.undoId === null}
                  onClick={() => {
                    if (view.undoId !== null) onStart(view.undoId);
                  }}
                >
                  {l10n.getString("finish-undo-start")}
                </Button>
              </FrameFooter>
            </>
          );
        }}
      </Loaded>
    </FinishPage>
  );
}
