import { useLocalization } from "@fluent/react";
import { useEffect, useState } from "react";
import { useSettle } from "../motion/screen";
import { historyExport, historyList, historyOpen, type HistoryEntryRaw, type QsReceipt } from "../bridge";
import type { ReportItem } from "../move";
import type { HistoryEntry } from "../quicksend";
import { labelShownSafely } from "../pairing";
import History, { type ExportResult } from "./History";
import QuickDone from "./QuickDone";
import Report from "./Report";

type View = { kind: "list" } | { kind: "report"; items: ReportItem[] } | { kind: "send"; receipt: QsReceipt };

/**
 * F08 My PCTwin history, from what the app recorded on this laptop when each move or Quick Send
 * ended (never from the screens): the list, each entry's report or what a send brought opened again, and an
 * export to a folder the person chooses in the system's picker.
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
    if (e.kind === "move") {
      return {
        id: e.id,
        kind: "move",
        when: e.when,
        from: other ?? l10n.getString("progress-route-old"),
        to: l10n.getString("progress-route-new"),
        state: e.state as Extract<HistoryEntry, { kind: "move" }>["state"],
        stepsLeft: e.stepsLeft,
      };
    }
    return {
      id: e.id,
      kind: e.kind,
      when: e.when,
      peer: other ?? l10n.getString(e.kind === "send" ? "qs-peer-receiving" : "qs-peer-sending"),
      state: e.state as Extract<HistoryEntry, { kind: "send" }>["state"],
    };
  });

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
  if (view.kind === "send") {
    return (
      <QuickDone
        side={view.receipt.side === "sender" ? "send" : "receive"}
        receipt={{ lines: view.receipt.lines }}
        stopped={view.receipt.stopped}
        onDone={() => {
          setView({ kind: "list" });
        }}
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
            setView(kept.kind === "report" ? { kind: "report", items: kept.items } : { kind: "send", receipt: kept.receipt });
          },
          () => undefined,
        );
      }}
      onContinue={() => undefined}
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
