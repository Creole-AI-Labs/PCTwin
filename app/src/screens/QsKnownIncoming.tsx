import { useLocalization } from "@fluent/react";
import { useEffect, useRef, useState } from "react";
import {
  knownDeclineIncoming,
  knownSaveIncoming,
  listenQuickSend,
  qsChooseFolder,
  qsFolder,
  qsKeepOriginalDates,
  qsOpenFolder,
  qsPause,
  qsResume,
  qsSessionEnd,
  qsStop,
  toQsRefusal,
  type KnownIncoming,
  type QsFolder,
  type QsProblem,
  type QsReceipt,
  type QsRefusal,
} from "../bridge";
import { Banner, Button, Checkbox } from "../components/kit";
import { fileName, useBytes, type SendProgress } from "../quicksend";
import FrameFooter from "./FrameFooter";
import { Folder, Heading, Name, QsStrip, Tile } from "./QsParts";
import QuickDone from "./QuickDone";
import OpenSendActions from "./QsOpenSend";
import { useOpenSends } from "../openSends";
import QuickProgress from "./QuickProgress";
import QuickStop from "./QuickStop";

/** How many names the question lists before "and N more". */
const NAMES_SHOWN = 5;

/**
 * A remembered laptop wants to send files (permission 2, asked every time): who, how many, how big,
 * their names, and the folder they would go into (its folder for next time, or Downloads), with
 * Choose another folder, "Use this folder for <name> next time", Decline and Save. Nothing is
 * written before Save.
 * Events: onChoose, onNextTime(on), onDecline, onSave.
 */
export function KnownIncomingAsk({
  incoming,
  folder,
  nextTime,
  problem = null,
  onChoose,
  onNextTime,
  onDecline,
  onSave,
}: {
  incoming: KnownIncoming;
  folder: QsFolder | null;
  nextTime: boolean;
  problem?: QsRefusal | null;
  onChoose: () => void;
  onNextTime: (on: boolean) => void;
  onDecline: () => void;
  onSave: () => void;
}) {
  const { l10n } = useLocalization();
  const bytes = useBytes();
  const shown = incoming.files.slice(0, NAMES_SHOWN);
  const more = incoming.files.length - shown.length;
  return (
    <>
      <QsStrip receiving />
      <Heading id="qs-where-title" />
      <p className="subhead">
        {l10n.getString("qs-known-wants-to-send", { label: incoming.label })} {l10n.getString("qs-where-body", { count: incoming.files.length, size: bytes(incoming.bytes) })}
      </p>
      <ul className="qs-name-list">
        {shown.map((f) => (
          <li key={f.id}>
            <Name>{fileName(f.path)}</Name> <span className="kit-hint">{bytes(f.size)}</span>
          </li>
        ))}
        {more > 0 && <li className="kit-hint">{l10n.getString("qs-known-in-more", { count: more })}</li>}
      </ul>
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
      <Checkbox
        checked={nextTime}
        onChange={onNextTime}
        label={l10n.getString("qs-known-in-next", { label: incoming.label })}
      />
      {problem && <Banner tone="attention">{l10n.getString(`qs-refusal-${problem.kind}`)}</Banner>}
      <FrameFooter note="send">
        <Button variant="quiet" onClick={onDecline}>
          {l10n.getString("qs-known-in-decline")}
        </Button>
        <Button onClick={onChoose}>{l10n.getString("qs-where-choose")}</Button>
        <Button variant="primary" disabled={folder === null} onClick={onSave}>
          {l10n.getString("qs-where-save")}
        </Button>
      </FrameFooter>
    </>
  );
}

const STARTING: SendProgress = { phase: "copying", filesDone: 0, filesTotal: 0, bytesDone: 0, bytesTotal: 0, current: null, failed: 0, speed: null, minutesLeft: null };

type View =
  | { kind: "ask" }
  | { kind: "progress"; progress: SendProgress }
  | { kind: "done"; receipt: QsReceipt }
  | { kind: "problem"; problem: QsProblem };

/**
 * Files from a remembered laptop, with the engine: the question above, then the measured progress
 * and the Done page, as for a send paired with the code. The folder shown is the one the engine
 * holds: the connection's folder for next time, else the one `quicksend_folder` holds (Downloads,
 * or the one chosen this session).
 */
export default function KnownReceiveHost({
  incoming,
  onHome,
  onSendTo,
}: {
  incoming: KnownIncoming;
  onHome: () => void;
  /** Send to <name>: files back to that laptop in the send it left open (the pick page, then the list of sends). */
  onSendTo?: (session: string) => void;
}) {
  const { l10n } = useLocalization();
  // The send with that laptop stays open after these files: the one that opened since this page did.
  const open = useOpenSends();
  const [before] = useState(() => new Set(open.sessions.map((s) => s.id)));
  const opened = open.sessions.filter((s) => !before.has(s.id) && s.name !== null);
  const [view, setView] = useState<View>({ kind: "ask" });
  const [folder, setFolder] = useState<QsFolder | null>(incoming.folder);
  const [nextTime, setNextTime] = useState(false);
  const [problem, setProblem] = useState<QsRefusal | null>(null);
  const [stopping, setStopping] = useState(false);
  const [openFailed, setOpenFailed] = useState(false);
  const [dates, setDates] = useState<{ kind: "kept"; count: number } | { kind: "failed" } | null>(null);
  const saved = useRef(false);

  // No folder for next time: the one held for any receive (Downloads, or the one chosen).
  useEffect(() => {
    if (incoming.folder) return;
    let live = true;
    qsFolder().then(
      (f) => {
        if (live) setFolder(f);
      },
      () => undefined,
    );
    return () => {
      live = false;
    };
  }, [incoming.folder]);

  useEffect(() => {
    let live = true;
    let unlisten: (() => void) | null = null;
    void listenQuickSend((event) => {
      if (!live || !saved.current) return;
      setView((v) => {
        if (v.kind !== "progress") return v;
        if (event.kind === "progress") return { kind: "progress", progress: event.progress };
        if (event.kind === "receipt") return { kind: "done", receipt: event.receipt };
        if (event.kind === "problem") return { kind: "problem", problem: event.problem };
        return v;
      });
    }).then((stop) => {
      if (live) unlisten = stop;
      else stop();
    });
    return () => {
      live = false;
      unlisten?.();
    };
  }, []);

  const me = { kind: "named" as const, name: l10n.getString("qs-label-yours") };
  const them = { kind: "named" as const, name: incoming.label };

  switch (view.kind) {
    case "ask":
      return (
        <KnownIncomingAsk
          incoming={incoming}
          folder={folder}
          nextTime={nextTime}
          problem={problem}
          onNextTime={setNextTime}
          onChoose={() => {
            setProblem(null);
            qsChooseFolder(incoming.prompt).then(
              (chosen) => {
                if (!("none" in chosen)) setFolder(chosen);
              },
              () => {
                setProblem({ kind: "failed" });
              },
            );
          }}
          onDecline={() => {
            void knownDeclineIncoming(incoming.prompt).catch(() => undefined);
            onHome();
          }}
          onSave={() => {
            setProblem(null);
            saved.current = true;
            setView({ kind: "progress", progress: STARTING });
            knownSaveIncoming(incoming.prompt, nextTime).catch((error: unknown) => {
              saved.current = false;
              setProblem(toQsRefusal(error));
              setView({ kind: "ask" });
            });
          }}
        />
      );
    case "progress":
      return (
        <>
          <QuickProgress
            from={them}
            to={me}
            progress={view.progress}
            side="receive"
            onPause={() => void qsPause().catch(() => undefined)}
            onResume={() => void qsResume().catch(() => undefined)}
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
    case "done": {
      const done = (
        <QuickDone
          side="receive"
          receipt={{ lines: view.receipt.lines }}
          files={incoming.files}
          stopped={view.receipt.stopped}
          folder={folder?.name ?? null}
          openFailed={openFailed}
          dates={dates ?? { kind: "new", count: view.receipt.newDates }}
          onKeepDates={() => {
            qsKeepOriginalDates().then(
              (count) => {
                setDates({ kind: "kept", count });
              },
              () => {
                setDates({ kind: "failed" });
              },
            );
          }}
          onOpenFolder={() => {
            setOpenFailed(false);
            qsOpenFolder().catch(() => {
              setOpenFailed(true);
            });
          }}
          onDone={onHome}
        />
      );
      return (
        <>
          {done}
          {onSendTo && (
            <OpenSendActions
              sessions={opened}
              peer={incoming.label}
              side="receive"
              onAdd={(s) => {
                onSendTo(s.id);
              }}
              onEnd={(s) => {
                void qsSessionEnd(s.id).catch(() => undefined);
              }}
            />
          )}
        </>
      );
    }
    case "problem":
      return (
        <>
          <QsStrip receiving />
          <Heading id={view.problem.kind === "link-lost" ? "qs-problem-lost-title" : "qs-problem-failed-title"} />
          <p className="subhead">{l10n.getString("qs-problem-body")}</p>
          <FrameFooter>
            <Button variant="primary" onClick={onHome}>
              {l10n.getString("outcome-home")}
            </Button>
          </FrameFooter>
        </>
      );
  }
}
