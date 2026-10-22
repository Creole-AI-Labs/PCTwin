import { useLocalization } from "@fluent/react";
import { useEffect, useRef, useState } from "react";
import {
  listenQuickSend,
  qsAnswer,
  qsChooseFolder,
  qsAcceptDrops,
  qsClear,
  qsOffer,
  qsPause,
  qsPick,
  qsReceive,
  qsResume,
  qsSend,
  qsStop,
  type QsFolder,
  type QsProblem,
  type QsReceipt,
} from "../bridge";
import type { Label } from "../pairing";
import { totalBytes, type OfferedFile, type PickedItem, type SendProgress } from "../quicksend";
import { Heading } from "./ProgressParts";
import { TopStrip } from "./FrameParts";
import FrameFooter from "./FrameFooter";
import { Banner, Button } from "../components/kit";
import PairNew from "./PairNew";
import PairOld from "./PairOld";
import QuickApprove from "./QuickApprove";
import QuickIncoming from "./QuickIncoming";
import QuickPick from "./QuickPick";
import QuickProgress from "./QuickProgress";
import QuickReceipt from "./QuickReceipt";
import QuickReview from "./QuickReview";
import QuickStop from "./QuickStop";
import QuickWhere from "./QuickWhere";
import "./progress.css";

type View =
  | { kind: "pick" }
  | { kind: "review"; files: OfferedFile[] }
  | { kind: "pair" }
  | { kind: "approve" }
  | { kind: "waiting" }
  | { kind: "incoming"; files: OfferedFile[] }
  | { kind: "where"; files: OfferedFile[] }
  | { kind: "progress"; progress: SendProgress }
  | { kind: "receipt"; receipt: QsReceipt }
  | { kind: "problem"; problem: QsProblem };

const STARTING: SendProgress = { phase: "copying", filesDone: 0, filesTotal: 0, bytesDone: 0, bytesTotal: 0, current: null, failed: 0, speed: null };

/** Merges a fresh pick into the list, keeping each thing's tick as the person left it. */
function merged(old: PickedItem[], fresh: PickedItem[]): PickedItem[] {
  return fresh.map((f) => ({ ...f, included: old.find((o) => o.id === f.id)?.included ?? f.included }));
}

/**
 * Quick Send, either way (Q02 to Q14), over the move's pairing: the sender chooses files in the
 * system's picker, checks them, pairs (showing the code, as an old laptop does) and starts; the
 * receiver pairs (typing the code), sees what is offered, chooses a folder in the system's picker
 * and accepts. Both follow the measured progress and end on their own receipt. The screens never
 * see a path on this laptop, only names and sizes.
 */
export default function QuickHost({ mode, onHome }: { mode: "send" | "receive"; onHome: () => void }) {
  const { l10n } = useLocalization();
  const sender = mode === "send";
  const [view, setView] = useState<View>({ kind: sender ? "pick" : "pair" });
  const [items, setItems] = useState<PickedItem[]>([]);
  const [offered, setOffered] = useState<OfferedFile[]>([]);
  const [checked, setChecked] = useState(false);
  const [approved, setApproved] = useState(false);
  const [folder, setFolder] = useState<QsFolder | null>(null);
  const [confirmed, setConfirmed] = useState(false);
  const [stopping, setStopping] = useState(false);
  const [failed, setFailed] = useState(false);
  const [dragging, setDragging] = useState(false);
  const live = useRef(true);
  const me: Label = { kind: "named", name: l10n.getString("qs-label-this") };
  const other: Label = { kind: "named", name: l10n.getString("qs-label-other") };
  const [from, to] = sender ? [me, other] : [other, me];

  useEffect(() => {
    live.current = true;
    let unlisten: (() => void) | null = null;
    void listenQuickSend((event) => {
      if (!live.current) return;
      // Drops are lists for the pick page, not pages of their own.
      if (event.kind === "picked") {
        setItems((old) => merged(old, event.items));
        return;
      }
      if (event.kind === "dragging") {
        setDragging(event.over);
        return;
      }
      setView((v) => {
        if (v.kind === "receipt" || v.kind === "problem") return v;
        switch (event.kind) {
          case "progress":
            return { kind: "progress", progress: event.progress };
          case "incoming":
            setOffered(event.files);
            return { kind: "incoming", files: event.files };
          case "receipt":
            return { kind: "receipt", receipt: event.receipt };
          case "problem":
            return { kind: "problem", problem: event.problem };
        }
      });
    }).then((stop) => {
      if (live.current) unlisten = stop;
      else stop();
    });
    return () => {
      live.current = false;
      unlisten?.();
      // A new send starts from nothing chosen.
      void qsClear().catch(() => undefined);
    };
  }, []);

  // Things dropped on the window are taken only while the pick page is open.
  const picking = view.kind === "pick";
  useEffect(() => {
    void qsAcceptDrops(picking).catch(() => undefined);
    return () => {
      void qsAcceptDrops(false).catch(() => undefined);
    };
  }, [picking]);

  const pick = (folders: boolean) => {
    qsPick(folders).then(
      (fresh) => {
        setItems((old) => merged(old, fresh));
      },
      () => {
        setFailed(true);
      },
    );
  };

  const fail = () => {
    setView({ kind: "problem", problem: { kind: "failed", detail: "" } });
  };

  switch (view.kind) {
    case "pick":
      return (
        <>
          <QuickPick
            items={items}
            dragging={dragging}
            onAddFiles={() => {
              pick(false);
            }}
            onAddFolders={() => {
              pick(true);
            }}
            onToggle={(id, on) => {
              setItems((list) => list.map((i) => (i.id === id ? { ...i, included: on } : i)));
            }}
            onAllowAccess={() => undefined}
            onContinue={() => {
              const ids = items.filter((i) => i.included && !i.problem).map((i) => i.id);
              qsOffer(ids).then((files) => {
                setOffered(files);
                setChecked(false);
                setView({ kind: "review", files });
              }, fail);
            }}
            onBack={onHome}
          />
          {failed && <Banner tone="attention">{l10n.getString("qs-pick-failed")}</Banner>}
        </>
      );
    case "review":
      return (
        <QuickReview
          files={view.files}
          checked={checked}
          onCheckedChange={setChecked}
          onChange={() => {
            setView({ kind: "pick" });
          }}
          onContinue={() => {
            setView({ kind: "pair" });
          }}
          onBack={() => {
            setView({ kind: "pick" });
          }}
        />
      );
    case "pair":
      return sender ? (
        <PairOld
          onBack={() => {
            setView({ kind: "review", files: offered });
          }}
          onPaired={() => {
            setApproved(false);
            setView({ kind: "approve" });
          }}
        />
      ) : (
        <PairNew
          onBack={onHome}
          onPaired={() => {
            setView({ kind: "waiting" });
            qsReceive().catch(fail);
          }}
        />
      );
    case "approve":
      return (
        <QuickApprove
          from={from}
          to={to}
          count={offered.length}
          size={totalBytes(offered)}
          receiver="waiting"
          approved={approved}
          onApprovedChange={setApproved}
          onStart={() => {
            setView({ kind: "progress", progress: STARTING });
            qsSend().catch(fail);
          }}
          onBack={onHome}
        />
      );
    case "waiting":
      return (
        <>
          <TopStrip labelId="qs-eyebrow-receiving" />
          <Heading id="qs-wait-title" />
          <p className="subhead" role="status">
            {l10n.getString("qs-wait-body")}
          </p>
        </>
      );
    case "incoming":
      return (
        <QuickIncoming
          from={from}
          to={to}
          files={view.files}
          onAccept={() => {
            setConfirmed(false);
            setView({ kind: "where", files: view.files });
          }}
          onDecline={() => {
            void qsAnswer({ kind: "decline" }).catch(() => undefined);
          }}
          onDone={onHome}
        />
      );
    case "where":
      return (
        <QuickWhere
          folder={folder?.path ?? null}
          free={folder?.freeBytes ?? null}
          needed={totalBytes(view.files)}
          confirmed={confirmed}
          onConfirmedChange={setConfirmed}
          onChoose={() => {
            qsChooseFolder().then((chosen) => {
              if (!("none" in chosen)) setFolder(chosen);
            }, fail);
          }}
          onContinue={() => {
            setView({ kind: "progress", progress: STARTING });
            qsAnswer({ kind: "accept", ids: null }).catch(fail);
          }}
          onBack={() => {
            setView({ kind: "incoming", files: view.files });
          }}
        />
      );
    case "progress":
      return (
        <>
          <QuickProgress
            from={from}
            to={to}
            progress={view.progress}
            onPause={() => void qsPause().catch(() => undefined)}
            onResume={() => void qsResume().catch(() => undefined)}
            onCompact={() => undefined}
            onStop={() => {
              setStopping(true);
            }}
            onReceipt={() => undefined}
          />
          <QuickStop
            open={stopping}
            kind="stop"
            onConfirm={() => {
              setStopping(false);
              void qsStop().catch(() => undefined);
            }}
            onClose={() => {
              setStopping(false);
            }}
          />
        </>
      );
    case "receipt":
      return (
        <QuickReceipt
          side={view.receipt.side === "sender" ? "send" : "receive"}
          receipt={{ lines: view.receipt.lines }}
          from={from}
          to={to}
          files={offered}
          stopped={view.receipt.stopped}
          onHistory={onHome}
          onDone={onHome}
        />
      );
    case "problem":
      return (
        <>
          <TopStrip labelId={sender ? "qs-eyebrow" : "qs-eyebrow-receiving"} />
          <Heading id={view.problem.kind === "link-lost" ? "qs-problem-lost-title" : "qs-problem-failed-title"} />
          <p className="subhead">{l10n.getString("qs-problem-body")}</p>
          {view.problem.detail && (
            <details className="mv-details">
              <summary>{l10n.getString("report-more-details")}</summary>
              <p dir="auto">{view.problem.detail}</p>
            </details>
          )}
          <FrameFooter>
            <Button variant="primary" onClick={onHome}>
              {l10n.getString("outcome-home")}
            </Button>
          </FrameFooter>
        </>
      );
  }
}
