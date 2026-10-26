import { useLocalization } from "@fluent/react";
import { useEffect, useRef, useState } from "react";
import { useSettle } from "../motion/screen";
import {
  listenQuickSend,
  qsChooseFolder,
  qsAcceptDrops,
  qsClear,
  qsFolder,
  qsOffer,
  qsOfferMissing,
  qsOpenFolder,
  qsPause,
  qsPick,
  qsReceive,
  qsResume,
  qsSend,
  qsStop,
  toMoveProblem,
  type QsFolder,
  type QsProblem,
  type QsReceipt,
} from "../bridge";
import { labelShownSafely, type Label } from "../pairing";
import type { OfferedFile, PickedItem, SendProgress } from "../quicksend";
import { Heading } from "./ProgressParts";
import { TopStrip } from "./FrameParts";
import FrameFooter from "./FrameFooter";
import { Banner, Button } from "../components/kit";
import PairNew from "./PairNew";
import PairOld from "./PairOld";
import QuickDone from "./QuickDone";
import QuickPick from "./QuickPick";
import QuickProgress from "./QuickProgress";
import QuickStop from "./QuickStop";
import { Folder, Tile } from "./QsParts";
import "./progress.css";

type View =
  | { kind: "pick" }
  /** The receiver, before pairing: finding the folder the files will go to. */
  | { kind: "folder" }
  /** The receiver has no folder (no Downloads folder could be found or opened): choose one first. */
  | { kind: "no-folder" }
  | { kind: "pair" }
  | { kind: "progress"; progress: SendProgress }
  | { kind: "done"; receipt: QsReceipt }
  | { kind: "problem"; problem: QsProblem };

const STARTING: SendProgress = { phase: "copying", filesDone: 0, filesTotal: 0, bytesDone: 0, bytesTotal: 0, current: null, failed: 0, speed: null };

/** Merges a fresh pick into the list, keeping each thing's tick as the person left it. */
function merged(old: PickedItem[], fresh: PickedItem[]): PickedItem[] {
  return fresh.map((f) => ({ ...f, included: old.find((o) => o.id === f.id)?.included ?? f.included }));
}

/**
 * Quick Send, either way, over the move's pairing with Quick Send's words. The sender chooses files
 * in the system's picker, then pairs (showing the code), and the send starts the moment the laptops
 * are paired. The receiver sees where the files will be saved (its Downloads folder, or the folder
 * it chose this session) with Change, pairs (typing the code), and receives everything offered into
 * that folder the moment the laptops are paired: choosing Receive, typing the code and matching the
 * number is the acceptance (Security Design T). Both follow the measured progress and end on the
 * Done page. The screens never see a path on this laptop, only names and sizes.
 */
export default function QuickHost({ mode, onHome }: { mode: "send" | "receive"; onHome: () => void }) {
  const { l10n } = useLocalization();
  const sender = mode === "send";
  const [view, setView] = useState<View>({ kind: sender ? "pick" : "folder" });
  // Each step settles in like a new screen; a problem arrives without travel.
  useSettle(view.kind, { calm: view.kind === "problem" });
  const [items, setItems] = useState<PickedItem[]>([]);
  const [offered, setOffered] = useState<OfferedFile[]>([]);
  const [folder, setFolder] = useState<QsFolder | null>(null);
  const [folderFailed, setFolderFailed] = useState(false);
  const [openFailed, setOpenFailed] = useState(false);
  const [stopping, setStopping] = useState(false);
  const [failed, setFailed] = useState(false);
  const [tooMany, setTooMany] = useState(false);
  const [dragging, setDragging] = useState(false);
  const live = useRef(true);
  // The receiver learns the sending laptop's name when it picks it to pair with; the sender is
  // never told the receiver's, so it says its role. A name from the network is shown safely.
  const [peer, setPeer] = useState<Label | null>(null);
  const me: Label = { kind: "named", name: l10n.getString("qs-label-yours") };
  const other: Label = {
    kind: "named",
    name: peer ? labelShownSafely(l10n, peer) : l10n.getString(sender ? "qs-label-receiving" : "qs-label-sending"),
  };
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
      // What is being received: kept for the sizes on the Done page, never a page of its own.
      if (event.kind === "incoming") {
        setOffered(event.files);
        return;
      }
      setView((v) => {
        // Only a send that is going on moves the page: a late word never takes the person off
        // the page they are on.
        if (v.kind !== "progress" && v.kind !== "pair") return v;
        switch (event.kind) {
          case "progress":
            return v.kind === "progress" ? { kind: "progress", progress: event.progress } : v;
          case "receipt":
            return v.kind === "progress" ? { kind: "done", receipt: event.receipt } : v;
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

  // The receiver: the folder the files will go to, held from now (the one chosen this session, or
  // Downloads). Without one, the person chooses a folder before pairing.
  useEffect(() => {
    if (sender) return;
    let on = true;
    qsFolder().then(
      (f) => {
        if (!on) return;
        setFolder(f);
        setView((v) => (v.kind === "folder" ? { kind: "pair" } : v));
      },
      () => {
        if (on) setView((v) => (v.kind === "folder" ? { kind: "no-folder" } : v));
      },
    );
    return () => {
      on = false;
    };
  }, [sender]);

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

  const chooseFolder = () => {
    setFolderFailed(false);
    qsChooseFolder().then(
      (chosen) => {
        if ("none" in chosen) return;
        setFolder(chosen);
        setView((v) => (v.kind === "no-folder" ? { kind: "pair" } : v));
      },
      () => {
        setFolderFailed(true);
      },
    );
  };

  const folderLine = (
    <div className="qs-folder-line">
      <Tile>
        <Folder size={18} />
      </Tile>
      <span dir="auto" role="status">{folder ? l10n.getString("qs-folder-line", { folder: folder.name }) : ""}</span>
      <Button onClick={chooseFolder}>{l10n.getString("qs-folder-change")}</Button>
      {folderFailed && <Banner tone="attention">{l10n.getString("qs-folder-failed")}</Banner>}
    </div>
  );

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
              setTooMany(false);
              qsOffer(ids).then(
                (files) => {
                  setOffered(files);
                  setView({ kind: "pair" });
                },
                (error: unknown) => {
                  // Said before anything is sent, on the page where the choice can change.
                  if (toMoveProblem(error).kind === "too-many-files") setTooMany(true);
                  else fail();
                },
              );
            }}
            onBack={onHome}
          />
          {failed && <Banner tone="attention">{l10n.getString("qs-pick-failed")}</Banner>}
          {tooMany && (
            <div role="alert">
              <Banner tone="attention">{l10n.getString("qs-pick-too-many")}</Banner>
            </div>
          )}
        </>
      );
    case "folder":
      return <TopStrip onBack={onHome} labelId="qs-eyebrow-receiving" />;
    case "no-folder":
      return (
        <>
          <TopStrip onBack={onHome} labelId="qs-eyebrow-receiving" />
          <Heading id="qs-folder-title" />
          <p className="subhead">{l10n.getString("qs-folder-none")}</p>
          {folderFailed && <Banner tone="attention">{l10n.getString("qs-folder-failed")}</Banner>}
          <FrameFooter>
            <Button variant="primary" onClick={chooseFolder}>
              {l10n.getString("qs-folder-choose")}
            </Button>
          </FrameFooter>
        </>
      );
    case "pair":
      // The pairing is the acceptance: the moment the laptops are paired the send (or the
      // receive) starts, with no page in between.
      return sender ? (
        <PairOld
          quick
          onBack={() => {
            setView({ kind: "pick" });
          }}
          onPaired={() => {
            setView({ kind: "progress", progress: STARTING });
            qsSend().catch(fail);
          }}
        />
      ) : (
        <PairNew
          quick
          aside={folderLine}
          onBack={onHome}
          onPaired={(label) => {
            setPeer(label ?? null);
            setView({ kind: "progress", progress: STARTING });
            qsReceive().catch(fail);
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
            side={sender ? "send" : "receive"}
            onPause={() => void qsPause().catch(() => undefined)}
            onResume={() => void qsResume().catch(() => undefined)}
            onCompact={() => undefined}
            onStop={() => {
              setStopping(true);
            }}
          />
          <QuickStop
            open={stopping}
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
    case "done":
      return (
        <QuickDone
          side={view.receipt.side === "sender" ? "send" : "receive"}
          receipt={{ lines: view.receipt.lines }}
          files={offered}
          stopped={view.receipt.stopped}
          folder={sender ? null : (folder?.name ?? null)}
          openFailed={openFailed}
          onOpenFolder={() => {
            setOpenFailed(false);
            qsOpenFolder().catch(() => {
              setOpenFailed(true);
            });
          }}
          onSendMissing={() => {
            // The same chosen things offered again, only what did not arrive; then paired again,
            // and sent the moment the laptops are paired, as any send.
            qsOfferMissing().then((files) => {
              if (files.length === 0) return;
              setOffered(files);
              setView({ kind: "pair" });
            }, fail);
          }}
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
