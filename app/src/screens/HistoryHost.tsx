import { useLocalization } from "@fluent/react";
import { useEffect, useState } from "react";
import { useSettle } from "../motion/screen";
import { fullmoveUndo, historyExport, historyList, historyOpen, type HistoryEntryRaw } from "../bridge";
import type { ReportItem } from "../move";
import type { HistoryEntry } from "../quicksend";
import { labelShownSafely } from "../pairing";
import History, { type ExportResult } from "./History";
import Report from "./Report";
import UndoHost from "./UndoHost";
import FinishHost from "./FinishHost";
import RecoveryKit from "./RecoveryKit";

type View = { kind: "list" } | { kind: "report"; items: ReportItem[] } | { kind: "undo" } | { kind: "finish"; id: string } | { kind: "kit"; id: string };

/**
 * F08 My PCTwin history, from what the app recorded on this laptop when each move ended (never from
 * the screens): the list, each move's report opened again, and an export to a folder the person
 * chooses in the system's picker. Quick Send keeps nothing here.
 */
export default function HistoryHost({ onStart, onBack }: { onStart: () => void; onBack: () => void }) {
  const { l10n } = useLocalization();
  const [entries, setEntries] = useState<HistoryEntryRaw[]>([]);
  const [view, setView] = useState<View>({ kind: "list" });
  useSettle(view.kind);
  const [exported, setExported] = useState<ExportResult | undefined>(undefined);

  useEffect(() => {
    let live = true;
    historyList().then(
      (list) => {
        if (live) setEntries(list);
      },
      () => undefined,
    );
    return () => {
      live = false;
    };
  }, []);

  // The screen's words for each entry: the old laptop by its pairing label, when known.
  const shown: HistoryEntry[] = entries.map((e) => {
    const other = e.other ? labelShownSafely(l10n, e.other) : null;
    const newName = e.newName ? labelShownSafely(l10n, e.newName) : null;
    return {
      id: e.id,
      kind: "move",
      when: e.when,
      from: other ?? l10n.getString("progress-route-old"),
      to: newName ?? l10n.getString("progress-route-new"),
      state: e.state,
      stepsLeft: e.stepsLeft,
      undoId: e.undoId ?? undefined,
    };
  });

  // Undo of a past move's apps and settings, through the flow guard (R05, R06), as from its result.
  if (view.kind === "undo") {
    return (
      <UndoHost
        onClosed={() => {
          setView({ kind: "list" });
        }}
      />
    );
  }
  if (view.kind === "finish") {
    return (
      <FinishHost
        id={view.id}
        onBack={() => {
          setView({ kind: "list" });
        }}
      />
    );
  }
  if (view.kind === "kit") {
    return (
      <RecoveryKit
        id={view.id}
        onBack={() => {
          setView({ kind: "list" });
        }}
      />
    );
  }
  if (view.kind === "report") {
    return (
      <Report
        items={view.items}
        onBack={() => {
          setView({ kind: "list" });
        }}
      />
    );
  }
  return (
    <History
      entries={shown}
      exported={exported}
      onOpen={(id) => {
        historyOpen(id).then(
          (kept) => {
            setView({ kind: "report", items: kept.items });
          },
          () => undefined,
        );
      }}
      onContinue={() => undefined}
      onFinish={(id) => {
        setView({ kind: "finish", id });
      }}
      onKit={(id) => {
        setView({ kind: "kit", id });
      }}
      onUndo={(undoId) => {
        fullmoveUndo(undoId).then(
          () => {
            setView({ kind: "undo" });
          },
          () => undefined,
        );
      }}
      onExport={() => {
        historyExport().then(
          (path) => {
            if (path !== null) setExported({ kind: "saved", path });
          },
          () => {
            setExported({ kind: "failed" });
          },
        );
      }}
      onStart={onStart}
      onBack={onBack}
    />
  );
}
