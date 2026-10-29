import { useLocalization } from "@fluent/react";
import { useEffect, useRef, useState } from "react";
import { useSettle } from "../motion/screen";
import {
  knownConnections,
  knownRemember,
  knownSend,
  listenKnown,
  listenQuickSend,
  qsLaunchFiles,
  qsSendNote,
  toQsRefusal,
  waitingPrepare,
  type KnownConnection,
  type QsNote,
  type QsRefusal,
  type RememberChoice,
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
  qsSendCancel,
  qsSendPause,
  qsSendResume,
  qsSessionAdd,
  qsSessionEnd,
  toMoveProblem,
  type QsSend,
  type QsSession,
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
import { NoteCompose, NoteInHost, NoteSent, noteProblem } from "./QsNote";
import OpenSendActions from "./QsOpenSend";
import QsSends, { type NotSent } from "./QsSends";
import { clearResults, showSends, useOpenSends } from "../openSends";
import QsRecipients, { type WaitDays, type When } from "./QsRecipients";
import { RememberOffer } from "./QsRemember";
import "./progress.css";
import MiniPanel from "./MiniPanel";
import { setWindowMode, type WindowMode } from "../native";

type View =
  | { kind: "pick" }
  /** The sender: who gets the files, when there are remembered laptops (Q04, Q17). */
  | { kind: "to" }
  /** A note or link: typed (Q24), then paired, then on its way and taken. */
  | { kind: "compose" }
  | { kind: "note-sent"; sent: boolean }
  /** The receiver: a note or link arrived over the code. */
  | { kind: "note"; note: QsNote }
  /** The receiver, before pairing: finding the folder the files will go to. */
  | { kind: "folder" }
  /** The receiver has no folder (no Downloads folder could be found or opened): choose one first. */
  | { kind: "no-folder" }
  | { kind: "pair" }
  /** The receiver, once paired and told what is coming: where should these go? Nothing is saved before Save here (29 October 2026). */
  | { kind: "where" }
  | { kind: "progress"; progress: SendProgress }
  /** `listed`: a result opened from the list of sends, where Done returns. */
  | { kind: "done"; receipt: QsReceipt; listed?: boolean }
  /** Your sends: every send going on now, each with Pause, Resume and Cancel, and the results. */
  | { kind: "sends" }
  | { kind: "problem"; problem: QsProblem };

/** "Remember this connection", offered after an exchange over the code; `remembered`: the name it took. */
type Remember = { fallbackStore: boolean; connections: RememberChoice[]; remembered: string | null; problem: QsRefusal | null };

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
  session,
  onHome,
  onWaiting,
  onWindow,
  onIdle,
}: {
  /** `note`: send a note or link (Q24) over the code; `sends`: the list of sends going on now. */
  mode: "send" | "receive" | "note" | "sends";
  /** Add files to this open send (Send to <name> from a remembered laptop's page): the pick page first, then the list of sends. */
  session?: string;
  onHome: () => void;
  /** A waiting send was approved: show the waiting sends. */
  onWaiting?: () => void;
  /** The window size the page asks for, so the frame can fit it (the mini box has no header). */
  onWindow?: (mode: WindowMode) => void;
  /** Whether leaving now loses nothing (the Done page, the list of sends): another question may then be shown. */
  onIdle?: (idle: boolean) => void;
}) {
  const { l10n } = useLocalization();
  const bytes = useBytes();
  const sender = mode !== "receive";
  // Open sends (Add files, Send to <name>, several laptops at once), from the engine's list.
  const open = useOpenSends();
  // The open sends that were there before this page: not this page's (unless it opened on the list).
  const [before] = useState(() => (mode === "sends" || session ? new Set<string>() : new Set(open.sessions.map((s) => s.id))));
  const mine = open.sessions.filter((s) => !before.has(s.id));
  const mineRef = useRef<QsSession[]>(mine);
  useEffect(() => {
    mineRef.current = mine;
  });
  // Add files to an open send: which one, and the page to go back to.
  const [addTo, setAddTo] = useState<{ session: string; back: View } | null>(session ? { session, back: { kind: "sends" } } : null);
  // Laptops a send now could not reach (several at once), and a Pause, Resume or Cancel not taken.
  const [notSent, setNotSent] = useState<NotSent[]>([]);
  const [sendsFailed, setSendsFailed] = useState(false);
  // The other laptop offered more in an open send: the page asks where they go, then receives them.
  const sessionReceive = useRef<{ back: View } | null>(null);
  // ... and it came while the person was choosing files: said on that page, never taking them off it.
  const [offerWaiting, setOfferWaiting] = useState(false);
  // For the engine's events (the listener is set once): which side this is, and whether Save here was pressed.
  const senderRef = useRef(sender);
  const savedRef = useRef(false);
  // The question the Save on the "where" page answers (a late click never answers the next).
  const promptRef = useRef(0);
  const [view, setView] = useState<View>({ kind: mode === "sends" ? "sends" : mode === "note" ? "compose" : sender ? "pick" : "folder" });
  const viewRef = useRef<View>(view);
  useEffect(() => {
    viewRef.current = view;
    onIdle?.(view.kind === "done" || view.kind === "sends");
  }, [view, onIdle]);
  // The note being typed, and why the last try was refused.
  const [noteText, setNoteText] = useState("");
  const [noteRefused, setNoteRefused] = useState<QsRefusal | null>(null);
  // Remembered laptops the sender may choose, and the choice (Q04, Q17).
  const [known, setKnown] = useState<KnownConnection[]>([]);
  const [chosenTo, setChosenTo] = useState<string[]>([]);
  const [when, setWhen] = useState<When>("now");
  const [days, setDays] = useState<WaitDays>("7");
  const [toRefused, setToRefused] = useState<QsRefusal | null>(null);
  const [busy, setBusy] = useState(false);
  const [remember, setRemember] = useState<Remember | null>(null);
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
        promptRef.current = event.prompt;
        // More from the other laptop in an open send: asked where they go, exactly as the first.
        const v = viewRef.current;
        if (mineRef.current.some((s) => !s.ending) && v.kind !== "where" && !(v.kind === "progress" && !savedRef.current && !senderRef.current)) {
          savedRef.current = false;
          sessionReceive.current = { back: v.kind === "done" || v.kind === "sends" ? v : { kind: "sends" } };
          setDatesKept(null);
          if (v.kind === "pick" || v.kind === "to") setOfferWaiting(true);
          else setView({ kind: "where" });
          return;
        }
        // The receiver: what is coming is announced, then PCTwin waits for where it goes.
        if (!senderRef.current) setView((v) => (v.kind === "progress" && !savedRef.current ? { kind: "where" } : v));
        return;
      }
      setView((v) => {
        // Waiting for where the files go, or for the note to be taken: only a problem (the sender
        // stopped, the link was lost) moves the page.
        if (v.kind === "where" || (v.kind === "note-sent" && !v.sent)) return event.kind === "problem" ? { kind: "problem", problem: event.problem } : v;
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

  // Notes, "Remember this connection" and files the system handed PCTwin, for this page.
  useEffect(() => {
    let on = true;
    let unlisten: (() => void) | null = null;
    void listenKnown((event) => {
      if (!on) return;
      if (event.kind === "remember") {
        setRemember({ fallbackStore: event.fallbackStore, connections: event.connections, remembered: null, problem: null });
      } else if (event.kind === "note-sent") {
        setView((v) => (v.kind === "note-sent" ? { kind: "note-sent", sent: true } : v));
      } else if (event.kind === "note" && !senderRef.current) {
        // Over the code, a note comes instead of files; one from a remembered laptop is the app's to show.
        const note = event.note;
        if (note.from === null) setView((v) => (v.kind === "progress" || v.kind === "note" ? { kind: "note", note } : v));
      } else if (event.kind === "launch") {
        setItems((old) => merged(old, event.items));
      } else if (event.kind === "session-receipt" && event.result.send === null && sessionReceive.current) {
        // What the other laptop offered in the open send, received here: its Done page.
        sessionReceive.current = null;
        setOfferWaiting(false);
        const receipt = event.result.receipt;
        setView({ kind: "done", receipt });
      }
    }).then((stop) => {
      if (on) unlisten = stop;
      else stop();
    });
    return () => {
      on = false;
      unlisten?.();
    };
  }, []);

  // The sender: what the system handed PCTwin to send joins the pick page, and the remembered
  // laptops it may go to are asked once.
  useEffect(() => {
    if (mode !== "send") return;
    let on = true;
    qsLaunchFiles().then(
      (fresh) => {
        if (on && fresh.length > 0) setItems((old) => merged(old, fresh));
      },
      () => undefined,
    );
    knownConnections().then(
      (list) => {
        if (on) setKnown(list);
      },
      () => undefined,
    );
    return () => {
      on = false;
    };
  }, [mode]);

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

  // The sender, on the other laptop's way back in an open send: where those files go is asked
  // here too, so the folder is found when it is first needed.
  useEffect(() => {
    if (view.kind !== "where" || folder !== null) return;
    let on = true;
    qsFolder().then(
      (f) => {
        if (on) setFolder(f);
      },
      () => {
        if (on) setFolderFailed(true);
      },
    );
    return () => {
      on = false;
    };
  }, [view.kind, folder]);

  // Add files (or Send to <name>) in an open send: a fresh pick page, then the list of sends.
  const addFiles = (s: QsSession) => {
    setNotSent([]);
    setToRefused(null);
    setTooMany(false);
    void qsClear().catch(() => undefined);
    setItems([]);
    setAddTo({ session: s.id, back: view });
    setView({ kind: "pick" });
  };
  const endSession = (s: QsSession) => {
    setSendsFailed(false);
    qsSessionEnd(s.id).catch(() => {
      setSendsFailed(true);
    });
  };
  const onList = (command: (id: number) => Promise<QsSend[]>) => (id: number) => {
    setSendsFailed(false);
    command(id).then(showSends, () => {
      setSendsFailed(true);
    });
  };

  // Send now to the remembered laptops ticked: each gets the same files, up to three at the same
  // time (the engine queues the rest). The pick page's offer goes with the first; each further
  // laptop gets the same choice offered again. A laptop that could not be reached is said, by name.
  const sendNow = async () => {
    const ids = items.filter((i) => i.included && !i.problem).map((i) => i.id);
    const failedTo: NotSent[] = [];
    let first: QsRefusal | null = null;
    let sent = 0;
    for (const [i, id] of chosenTo.entries()) {
      const to = known.find((c) => c.id === id);
      try {
        if (i > 0) await qsOffer(ids);
        await knownSend(id);
        sent++;
      } catch (error: unknown) {
        const refusal = toQsRefusal(error);
        first ??= refusal;
        failedTo.push({ name: to?.label ?? "", refusal });
      }
    }
    setBusy(false);
    if (sent === 0) {
      setToRefused(first);
      return;
    }
    setNotSent(failedTo);
    setView({ kind: "sends" });
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

  const rememberOffer = remember && (
    <RememberOffer
      fallbackStore={remember.fallbackStore}
      connections={remember.connections}
      suggestedName={peer ? labelShownSafely(l10n, peer) : ""}
      remembered={remember.remembered}
      problem={remember.problem}
      onRemember={(label, replaces) => {
        knownRemember(label, replaces).then(
          () => {
            setRemember((r) => r && { ...r, remembered: label, problem: null });
          },
          (error: unknown) => {
            setRemember((r) => r && { ...r, problem: toQsRefusal(error) });
          },
        );
      }}
    />
  );

  const peerName = peer ? labelShownSafely(l10n, peer) : l10n.getString(sender ? "qs-label-receiving" : "qs-label-sending");
  const openSend = <OpenSendActions sessions={mine} peer={peerName} side={sender ? "send" : "receive"} onAdd={addFiles} onEnd={endSession} />;

  switch (view.kind) {
    case "compose":
      return (
        <NoteCompose
          text={noteText}
          problem={noteRefused}
          onChange={(t) => {
            setNoteText(t);
            setNoteRefused(null);
          }}
          onContinue={() => {
            // Said before pairing, by the engine's own rules; the engine checks again.
            const refused = noteProblem(noteText);
            setNoteRefused(refused);
            if (!refused) setView({ kind: "pair" });
          }}
          onBack={onHome}
        />
      );
    case "note-sent":
      return (
        <>
          <NoteSent sent={view.sent} onDone={onHome} />
          {view.sent && rememberOffer}
        </>
      );
    case "note":
      return (
        <>
          <NoteInHost key={view.note.id} note={view.note} onDone={onHome} />
          {rememberOffer}
        </>
      );
    case "to":
      return (
        <QsRecipients
          connections={known}
          chosen={chosenTo}
          when={when}
          days={days}
          problem={toRefused}
          busy={busy}
          onToggle={(id, on) => {
            setToRefused(null);
            setChosenTo((list) => (on ? [...list.filter((x) => x !== id), id] : list.filter((x) => x !== id)));
          }}
          onWhen={setWhen}
          onDays={setDays}
          onCode={() => {
            setView({ kind: "pair" });
          }}
          onSendNow={() => {
            setToRefused(null);
            setBusy(true);
            void sendNow();
          }}
          onApprove={() => {
            setToRefused(null);
            setBusy(true);
            waitingPrepare(chosenTo, Number(days)).then(
              () => {
                setBusy(false);
                (onWaiting ?? onHome)();
              },
              (error: unknown) => {
                setBusy(false);
                setToRefused(toQsRefusal(error));
              },
            );
          }}
          onBack={() => {
            setView({ kind: "pick" });
          }}
        />
      );
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
              setToRefused(null);
              qsOffer(ids).then(
                (files) => {
                  setOffered(files);
                  // Add files in an open send: it waits its turn there, shown in the list of sends.
                  if (addTo) {
                    qsSessionAdd(addTo.session).then(
                      (list) => {
                        showSends(list);
                        setAddTo(null);
                        setView({ kind: "sends" });
                      },
                      (error: unknown) => {
                        setToRefused(toQsRefusal(error));
                      },
                    );
                    return;
                  }
                  // Remembered laptops: choose who gets it; otherwise pair with the code.
                  setView({ kind: known.length > 0 ? "to" : "pair" });
                },
                (error: unknown) => {
                  // Said before anything is sent, on the page where the choice can change.
                  if (toMoveProblem(error).kind === "too-many-files") setTooMany(true);
                  else fail();
                },
              );
            }}
            onBack={
              addTo
                ? () => {
                    const back = addTo.back;
                    setAddTo(null);
                    if (session && back.kind === "sends") onHome();
                    else setView(back);
                  }
                : onHome
            }
          />
          {failed && <Banner tone="attention">{l10n.getString("qs-pick-failed")}</Banner>}
          {addTo && toRefused && <Banner tone="attention">{l10n.getString(`qs-refusal-${toRefused.kind}`)}</Banner>}
          {offerWaiting && (
            <div role="status" className="qs-offer-waiting">
              <Banner tone="info">{l10n.getString("qs-session-offer-waiting", { name: peerName })}</Banner>
              <Button
                onClick={() => {
                  setOfferWaiting(false);
                  setView({ kind: "where" });
                }}
              >
                {l10n.getString("qs-session-offer-show")}
              </Button>
            </div>
          )}
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
            setView({ kind: mode === "note" ? "compose" : known.length > 0 ? "to" : "pick" });
          }}
          onPaired={() => {
            if (mode === "note") {
              setView({ kind: "note-sent", sent: false });
              qsSendNote(noteText).catch((error: unknown) => {
                const refused = toQsRefusal(error);
                // A note the engine would not take goes back to the box, said there.
                if (refused.kind.startsWith("note-")) {
                  setNoteRefused(refused);
                  setView({ kind: "compose" });
                } else fail();
              });
              return;
            }
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
                // In an open send this declines only these files; the send stays open.
                const inSession = sessionReceive.current;
                sessionReceive.current = null;
                if (inSession) setView(inSession.back);
                else onHome();
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
                qsSaveHere(promptRef.current).catch(fail);
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
            // Add files (sender) and Send to <name> (receiver), while the send is open.
            openSend={openSend}
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
    case "sends":
      return (
        <QsSends
          open={open}
          peer={peerName}
          notSent={notSent}
          failed={sendsFailed}
          openSend={openSend}
          onPause={onList(qsSendPause)}
          onResume={onList(qsSendResume)}
          onCancel={onList(qsSendCancel)}
          onOpen={(send) => {
            const r = open.results.find((x) => x.send === send);
            if (r) setView({ kind: "done", receipt: r.receipt, listed: true });
          }}
          onDone={() => {
            clearResults();
            onHome();
          }}
        />
      );
    case "done":
      if (view.listed)
        return (
          <QuickDone
            side="send"
            receipt={{ lines: view.receipt.lines }}
            stopped={view.receipt.stopped}
            folder={null}
            onDone={() => {
              setView({ kind: "sends" });
            }}
            onBack={() => {
              setView({ kind: "sends" });
            }}
          />
        );
      return (
        <>
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
          {openSend}
          {rememberOffer}
        </>
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
