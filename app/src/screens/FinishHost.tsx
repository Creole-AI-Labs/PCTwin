/**
 * Finishing a move (Task List 2.7): the hub and the pages it opens, each on its own page (Back
 * returns to the list). `id` is the move's My PCTwin history entry ID; none means the newest move.
 * Undo is the existing undo (R05 preview here, then the flow guard's screens for R06).
 */
import { useState } from "react";
import { fullmoveUndo } from "../bridge";
import FinishCatchup from "./FinishCatchup";
import FinishCleanup from "./FinishCleanup";
import FinishDoctor from "./FinishDoctor";
import FinishGoodbye from "./FinishGoodbye";
import FinishHandover from "./FinishHandover";
import FinishHub, { type FinishStep } from "./FinishHub";
import FinishReport from "./FinishReport";
import FinishSetup, { FinishSave } from "./FinishSetup";
import FinishUndo from "./FinishUndo";
import UndoHost from "./UndoHost";

type Page = { kind: "hub" } | { kind: FinishStep } | { kind: "save" } | { kind: "undoing" };

export default function FinishHost({ id, onBack, onPair }: { id: string | null; onBack: () => void; onPair?: () => void }) {
  const [page, setPage] = useState<Page>({ kind: "hub" });
  const hub = () => {
    setPage({ kind: "hub" });
  };
  switch (page.kind) {
    case "hub":
      return <FinishHub id={id} onBack={onBack} onOpen={(kind) => { setPage({ kind }); }} />;
    case "setup":
      return <FinishSetup id={id} onBack={hub} onSave={() => { setPage({ kind: "save" }); }} />;
    case "save":
      return <FinishSave id={id} onBack={() => { setPage({ kind: "setup" }); }} />;
    case "goodbye":
      return <FinishGoodbye id={id} onBack={hub} onHandover={() => { setPage({ kind: "handover" }); }} onCheckAgain={() => { setPage({ kind: "catchup" }); }} />;
    case "catchup":
      return <FinishCatchup id={id} onBack={hub} onPair={onPair} />;
    case "handover":
      return <FinishHandover id={id} onBack={hub} onList={() => { setPage({ kind: "goodbye" }); }} />;
    case "undo":
      return (
        <FinishUndo
          id={id}
          onBack={hub}
          onStart={(undoId) => {
            fullmoveUndo(undoId).then(
              () => {
                setPage({ kind: "undoing" });
              },
              () => undefined,
            );
          }}
        />
      );
    case "undoing":
      return <UndoHost onClosed={hub} />;
    case "cleanup":
      return <FinishCleanup onBack={hub} />;
    case "report":
      return <FinishReport id={id} onBack={hub} />;
    case "doctor":
      return <FinishDoctor onBack={hub} />;
  }
}
