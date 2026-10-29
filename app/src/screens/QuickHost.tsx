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
  qsKeepOriginalDates,
  qsOpenFolder,
  qsPause,
  qsPick,
  qsReceive,
  qsSaveHere,
  qsResume,
  qsSend,
  qsStop,
  toMoveProblem,
  type QsFolder,
  type QsProblem,
  type QsReceipt,
} from "../bridge";
import { labelShownSafely, type Label } from "../pairing";
import { useBytes, type OfferedFile, type PickedItem, type SendProgress } from "../quicksend";
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
import MiniPanel from "./MiniPanel";
import { setWindowMode, type WindowMode } from "../native";

type View =
  | { kind: "pick" }
  /** The receiver, before pairing: finding the folder the files will go to. */
  | { kind: "folder" }
  /** The receiver has no folder (no Downloads folder could be found or opened): choose one first. */
  | { kind: "no-folder" }
  | { kind: "pair" }
  /** The receiver, once paired and told what is coming: where should these go? Nothing is saved before Save here (29 October 2026). */
  | { kind: "where" }
  | { kind: "progress"; progress: SendProgress }
  | { kind: "done"; receipt: QsReceipt }
  | { kind: "problem"; problem: QsProblem };

const STARTING: SendProgress = { phase: "copying", filesDone: 0, filesTotal: 0, bytesDone: 0, bytesTotal: 0, current: null, failed: 0, speed: null, minutesLeft: null };

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
export default function QuickHost({
  mode,
  onHome,
  onWindow,
}: {
  mode: "send" | "receive";
  onHome: () => void;
  /** The window size the page asks for, so the frame can fit it (the mini box has no header). */
  onWindow?: (mode: WindowMode) => void;
}) {
  const { l10n } = useLocalization();
  const bytes = useBytes();
  const sender = mode === "send";
  // For the engine's events (the listener is set once): which side this is, and whether Save here was pressed.
  const senderRef = useRef(sender);
  const savedRef = useRef(false);
  const [view, setView] = useState<View>({ kind: sender ? "pick" : "folder" });
  // Each step settles in like a new screen; a problem arrives without travel.
  useSettle(view.kind, { calm: view.kind === "problem" });
  const [items, setItems] = useState<PickedItem[]>([]);
  const [offered, setOffered] = useState<OfferedFile[]>([]);
  const [folder, setFolder] = useState<QsFolder | null>(null);
  const [folderFailed, setFolderFailed] = useState(false);
  const [openFailed, setOpenFailed] = useState(false);
  // After Keep original dates: how many got theirs back, or that it could not be done.
  const [datesKept, setDatesKept] = useState<{ kind: "kept"; count: number } | { kind: "failed" } | null>(null);
  const [stopping, setStopping] = useState(false);
  // Keep it compact: the small box that floats in the corner, until Open.
  const [mini, setMini] = useState(false);
  const windowMode: WindowMode = mini && view.kind === "progress" ? "mini" : "everyday";
  useEffect(() => {
    void setWindowMode(windowMode).catch(() => undefined);
    onWindow?.(windowMode);
  }, [windowMode, onWindow]);
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
        // The receiver: what is coming is announced, then PCTwin waits for where it goes.
        if (!senderRef.current) setView((v) => (v.kind === "progress" && !savedRef.current ? { kind: "where" } : v));
        return;
      }
      setView((v) => {
        // Waiting for where the files go: only a problem (the sender stopped, the link was lost) moves the page.
        if (v.kind === "where") return event.kind === "problem" ? { kind: "problem", problem: event.problem } : v;
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
    case "where":
      return (
        <>
          <TopStrip labelId="qs-eyebrow-receiving" />
          <Heading id="qs-where-title" />
          <p className="subhead">{l10n.getString("qs-where-body", { count: offered.length, size: bytes(offered.reduce((n, f) => n + f.size, 0)) })}</p>
          <div className="qs-folder-line">
            <Tile>
              <Folder size={18} />
            </Tile>
            <span dir="auto" role="status">
              {folder ? l10n.getString("qs-where-folder", { folder: folder.name }) : ""}
            </span>
          </div>
          {folder && (
            <details className="mv-details">
              <summary>{l10n.getString("qs-where-details")}</summary>
              <p dir="auto">{l10n.getString("qs-where-path", { path: folder.path })}</p>
            </details>
          )}
          {folderFailed && <Banner tone="attention">{l10n.getString("qs-folder-failed")}</Banner>}
          <FrameFooter>
            <Button
              variant="quiet"
              onClick={() => {
                // Nothing is saved yet, so stopping loses nothing.
                void qsStop().catch(() => undefined);
                onHome();
              }}
            >
              {l10n.getString("qs-where-stop")}
            </Button>
            <Button onClick={chooseFolder}>{l10n.getString("qs-where-choose")}</Button>
            <Button
              variant="primary"
              onClick={() => {
                savedRef.current = true;
                setView({ kind: "progress", progress: STARTING });
                qsSaveHere().catch(fail);
              }}
            >
              {l10n.getString("qs-where-save")}
            </Button>
          </FrameFooter>
        </>
      );
    case "progress":
      if (mini) {
        const p = view.progress;
        const running = p.phase !== "paused" && p.phase !== "lost";
        return (
          <MiniPanel
            kind="send"
            progress={{
              state: running ? "running" : "paused",
              bytes: { checked: p.bytesDone, total: p.bytesTotal },
              // Only the percent is shown here: a send still under way is never shown as 100%.
              counts: { total: p.filesTotal, waiting: Math.max(0, p.filesTotal - p.filesDone - p.failed), copying: 0, checking: 0, done: p.filesDone, needsAction: 0, failed: p.failed, skippedByUser: 0 },
              minutesLeft: p.minutesLeft ?? undefined,
            }}
            onPause={() => void qsPause().catch(() => undefined)}
            onResume={() => void qsResume().catch(() => undefined)}
            onOpen={() => {
              setMini(false);
            }}
          />
        );
      }
      return (
        <>
          <QuickProgress
            from={from}
            to={to}
            progress={view.progress}
            side={sender ? "send" : "receive"}
            onPause={() => void qsPause().catch(() => undefined)}
            onResume={() => void qsResume().catch(() => undefined)}
            onCompact={() => {
              setMini(true);
            }}
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
          dates={datesKept ?? { kind: "new", count: view.receipt.newDates }}
          onKeepDates={() => {
            qsKeepOriginalDates().then(
              (count) => {
                setDatesKept({ kind: "kept", count });
              },
              () => {
                setDatesKept({ kind: "failed" });
              },
            );
          }}
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
      if (view.problem.kind === "no-room")
        return (
          <>
            <TopStrip labelId="qs-eyebrow-receiving" />
            <Heading id="qs-problem-room-title" vars={{ folder: folder?.name ?? "" }} />
            <p className="subhead">{l10n.getString("qs-problem-room-body", { needed: bytes(view.problem.needed), room: bytes(view.problem.room) })}</p>
            <FrameFooter>
              <Button variant="primary" onClick={onHome}>
                {l10n.getString("outcome-home")}
              </Button>
            </FrameFooter>
          </>
        );
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
