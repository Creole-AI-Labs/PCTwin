import { useLocalization } from "@fluent/react";
import { useEffect, useRef, useState } from "react";
import {
  helpCode,
  listenMove,
  moveApprove,
  moveBegin,
  moveChoosePlace,
  movePause,
  movePeopleHere,
  movePlacePeople,
  movePrivacyAnswer,
  moveUsualPlace,
  openPrivacySettings,
  moveReport,
  moveResume,
  moveStop,
  toMoveProblem,
  type HelpStep,
  type HelpTopic,
  type MoveEvent,
  type OthersWhy,
  type PeopleHere,
  type PeopleToPlace,
  type Peer,
  type WaitStage,
} from "../bridge";
import BeforeYouStart from "./BeforeYouStart";
import FullDiskAccess from "./FullDiskAccess";
import type { BeforeItem } from "../people";
import PlaceOthers from "./PlaceOthers";
import WhoHere from "./WhoHere";
import type { HelpCode } from "../people";
import GetHelp from "./GetHelp";
import type { MoveOutcome, MoveProgress, ReportItem } from "../move";
import { setWindowMode, type WindowMode } from "../native";
import MiniPanel from "./MiniPanel";
import type { Approval, FolderRole, PlanView } from "../plan";
import DriveProblem from "./DriveProblem";
import MoveEnded, { type MoveEnd } from "./MoveEnded";
import MoveWaiting from "./MoveWaiting";
import Outcome from "./Outcome";
import PlanReview, { type ChosenPlace, type PlaceNote } from "./PlanReview";
import MeetLaptop from "./MeetLaptop";
import CheckConnection from "./CheckConnection";
import { connectionCheck, peerReadiness, type Greeted, type Naming } from "../connection";
import { labelText } from "../pairing";
import ProgressCompact from "./ProgressCompact";
import ProgressDetail from "./ProgressDetail";
import Report from "./Report";
import StopMove from "./StopMove";

/** What the move page shows now. */
type View =
  /** The old laptop: who lives here, before the move starts (M06). */
  | { kind: "who"; here: PeopleHere }
  | { kind: "wait"; stage: WaitStage }
  | { kind: "plan"; plan: PlanView }
  /** The new laptop: where the other people's things go (M06). */
  | { kind: "people"; toPlace: PeopleToPlace }
  /** The old laptop, a Mac: some of the mover's places were kept private (Full Disk Access). */
  | { kind: "privacy"; places: number }
  | { kind: "progress"; progress: MoveProgress }
  | { kind: "outcome"; outcome: MoveOutcome }
  | { kind: "report"; outcome: MoveOutcome; items: ReportItem[] }
  | { kind: "end"; end: MoveEnd };

/** The folders a new laptop may lack: each gets a folder in the home, named in the person's language. */
const MAY_LACK: readonly FolderRole[] = ["desktop", "documents", "downloads", "pictures", "music", "videos"];

/** The help topic for an end, and for a drive problem. */
function topicOf(end: MoveEnd): HelpTopic {
  if (end.kind === "refused") return "move-refused";
  switch (end.why.kind) {
    case "link-lost":
      return "connection-lost";
    case "not-approved":
      return "not-approved";
    case "failed":
    case "update-here":
    case "update-there":
      return "move-failed";
  }
}

/** How far the move got, for a help code, by what the page showed last. */
export function stepOf(view: View, old: boolean): HelpStep {
  switch (view.kind) {
    case "who":
    case "privacy":
      return "finding";
    case "wait":
      if (view.stage === "scanning" || view.stage === "administrator") return "finding";
      if (view.stage === "moving") return "moving";
      return old ? "approving" : "finding";
    case "plan":
    case "people":
      return "approving";
    case "progress":
      return "moving";
    case "outcome":
    case "report":
      return "finished";
    case "end":
      return "finding";
  }
}

/** Where a moved event takes the page. Nothing moves the page off an end, and a plan or progress never goes back to waiting. */
export function next(view: View, event: MoveEvent): View {
  if (view.kind === "end" || view.kind === "outcome" || view.kind === "report") return view;
  switch (event.kind) {
    case "stage":
      // Others not moved is a note, not a page of its own.
      if (event.stage.kind === "others-not-moved") return view;
      return view.kind === "wait" ? { kind: "wait", stage: event.stage.kind } : view;
    case "plan":
      return { kind: "plan", plan: event.plan };
    case "people":
      return { kind: "people", toPlace: event.toPlace };
    case "privacy":
      return { kind: "privacy", places: event.privacy.places };
    case "found":
    case "peer":
      // A count, and the two laptops, for the waiting page: not pages of their own.
      return view;
    case "progress":
      return { kind: "progress", progress: event.progress };
    case "finished":
      return { kind: "outcome", outcome: event.outcome };
    case "problem":
      return { kind: "end", end: { kind: "ended", why: event.problem } };
  }
}

/**
 * The move, once both laptops are paired: it starts this laptop's side at once (the old laptop
 * finds the person's files; the new laptop waits for them), then follows the engine. The new
 * laptop's person approves the plan (M23); both laptops show the measured progress (X01, X02,
 * X05), a drive problem (X07), the stop dialog (X10), the outcome (R01) and the report (R02); a move
 * that ends early shows its error page. Leaving is only by the page's own buttons, so a running
 * move is never left without a screen.
 */
/** The window size each page is drawn for (the design system's three). */
export function windowFor(view: View, expanded: boolean, mini: boolean): WindowMode {
  if (view.kind === "progress") return mini ? "mini" : expanded ? "expanded" : "everyday";
  if (view.kind === "plan" || view.kind === "report") return "expanded";
  return "everyday";
}

export default function MoveHost({
  old,
  onPairAgain,
  onHome,
  onWindow,
}: {
  old: boolean;
  onPairAgain: () => void;
  onHome: () => void;
  /** The window size the page asks for, so the frame can fit it (the mini panel has no header). */
  onWindow?: (mode: WindowMode) => void;
}) {
  const { l10n } = useLocalization();
  const [view, setView] = useState<View>({ kind: "wait", stage: old ? "scanning" : "waiting" });
  const [approval, setApproval] = useState<Approval | null>(null);
  const [planChanged, setPlanChanged] = useState(false);
  // The folders the person unticked on the plan: they stay on the old laptop.
  const [leftOut, setLeftOut] = useState<FolderRole[]>([]);
  // The places the person chose for some folders (M05), and why the last one was not taken.
  const [chosen, setChosen] = useState<Partial<Record<FolderRole, ChosenPlace>>>({});
  const [placeNote, setPlaceNote] = useState<PlaceNote | null>(null);
  // Why the other people ticked were not moved, when they were not: said until the move ends.
  const [othersWhy, setOthersWhy] = useState<OthersWhy | null>(null);
  // How much of the mover's own things the scan has found so far (M02).
  const [found, setFound] = useState<number | null>(null);
  // The two laptops, as each said when the move started (C06), and the connection (C07): each
  // shown once, as its own page, before the waiting page.
  const [peer, setPeer] = useState<Peer | null>(null);
  const [intro, setIntro] = useState<"meet" | "check" | "done">("meet");
  // "Before you leave this laptop": open or not, and the person's own ticks (their word only).
  const [before, setBefore] = useState(false);
  const [beforeItems, setBeforeItems] = useState<BeforeItem[]>(() =>
    (["licences", "passwords", "codes", "online"] as const).map((kind) => ({ id: kind, kind, done: false })),
  );
  const [expanded, setExpanded] = useState(false);
  const [mini, setMini] = useState(false);
  const mode = windowFor(view, expanded, mini);
  useEffect(() => {
    void setWindowMode(mode).catch(() => undefined);
    onWindow?.(mode);
  }, [mode, onWindow]);
  const [stopping, setStopping] = useState(false);
  const started = useRef(false);
  // Starts this laptop's side, with the other people the mover ticked (the old laptop only).
  const begin = useRef((others: string[]) => {
    const fallback = Object.fromEntries(MAY_LACK.map((role) => [role, `${l10n.getString("move-fallback-folder")}/${l10n.getString(`plan-role-${role}`)}`]));
    moveBegin(fallback, others).catch((error: unknown) => {
      setView({ kind: "end", end: { kind: "refused", why: toMoveProblem(error) } });
    });
  });
  // How far the move got before it ended, for a help code.
  const step = useRef<HelpStep>("finding");
  useEffect(() => {
    if (view.kind !== "end") step.current = stepOf(view, old);
  }, [view, old]);
  // Get help, when open: for which problem, and the code as far as it has come.
  const [help, setHelp] = useState<{ topic: HelpTopic; code: HelpCode } | null>(null);
  const openHelp = (topic: HelpTopic) => {
    setHelp({ topic, code: { status: "making" } });
    helpCode(topic, step.current).then(
      (code) => {
        setHelp({ topic, code: { status: "ready", code, included: ["problem", "step", "versions"] } });
      },
      () => {
        setHelp({ topic, code: { status: "failed" } });
      },
    );
  };

  useEffect(() => {
    let live = true;
    let unlisten: (() => void) | null = null;
    void listenMove((event) => {
      if (!live) return;
      if (event.kind === "stage" && event.stage.kind === "others-not-moved") setOthersWhy(event.stage.why);
      if (event.kind === "found") setFound(event.bytes);
      if (event.kind === "peer") setPeer(event.peer);
      setView((v) => next(v, event));
    }).then((stop) => {
      if (!live) {
        stop();
        return;
      }
      unlisten = stop;
      // Started once, after the page listens, so no word from the engine is missed.
      if (started.current) return;
      started.current = true;
      if (!old) {
        begin.current([]);
        return;
      }
      // The old laptop asks who is moving first, unless the mover is the only person here.
      movePeopleHere().then(
        (here) => {
          if (!live) return;
          if (here.people.some((p) => !p.isYou)) setView({ kind: "who", here });
          else begin.current([]);
        },
        () => {
          if (live) begin.current([]);
        },
      );
    });
    return () => {
      live = false;
      unlisten?.();
    };
    // The page starts one move for its whole life; the language does not restart it.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  const stopNow = () => {
    void moveStop().catch(() => undefined);
  };

  // C06 and C07 come first. A move that ended over PCTwin versions stays on C06, which names the
  // laptop to update; any other end goes straight to its own page.
  const versionsDiffer = peer !== null && peer.versions !== "same";
  if (peer && !help && intro !== "done" && (view.kind !== "end" || versionsDiffer)) {
    const naming: Naming = {
      system: (s) => l10n.getString(`move-peer-system-${s}`),
      thisLaptop: l10n.getString("qs-label-this"),
      otherLaptop: l10n.getString("qs-label-other"),
    };
    const greeted: Greeted = { ...peer, otherLabel: peer.other ? labelText(l10n, peer.other) : null };
    if (intro === "meet" || versionsDiffer) {
      return (
        <MeetLaptop
          readiness={peerReadiness(greeted, old, naming)}
          onContinue={() => {
            setIntro("check");
          }}
          onBack={view.kind === "end" ? onHome : stopNow}
        />
      );
    }
    return (
      <CheckConnection
        check={connectionCheck(greeted, old, naming)}
        onContinue={() => {
          setIntro("done");
        }}
        onBack={() => {
          setIntro("meet");
        }}
      />
    );
  }

  if (help) {
    return (
      <GetHelp
        help={help.code}
        canShare={false}
        onBack={() => {
          setHelp(null);
        }}
        onShare={() => undefined}
        onRetry={() => {
          openHelp(help.topic);
        }}
      />
    );
  }

  const othersNote = othersWhy ? l10n.getString(`move-others-${othersWhy}`) : undefined;

  switch (view.kind) {
    case "who":
      return (
        <WhoHere
          here={view.here}
          onContinue={(others) => {
            setView({ kind: "wait", stage: "scanning" });
            begin.current(others);
          }}
        />
      );
    case "wait":
      if (before && old) {
        return (
          <BeforeYouStart
            items={beforeItems}
            onBack={() => {
              setBefore(false);
            }}
            onToggle={(id, done) => {
              setBeforeItems((items) => items.map((i) => (i.id === id ? { ...i, done } : i)));
            }}
            onContinue={() => {
              setBefore(false);
            }}
          />
        );
      }
      return (
        <MoveWaiting
          stage={view.stage}
          old={old}
          note={othersNote}
          found={view.stage === "scanning" ? found : null}
          onBefore={
            old && view.stage === "waiting"
              ? () => {
                  setBefore(true);
                }
              : undefined
          }
          onStop={stopNow}
        />
      );
    case "privacy":
      return (
        <FullDiskAccess
          places={view.places}
          onOpenSettings={() => void openPrivacySettings().catch(() => undefined)}
          onCheckAgain={() => {
            setView({ kind: "wait", stage: "scanning" });
            void movePrivacyAnswer(true).catch(() => undefined);
          }}
          onWithout={() => {
            setView({ kind: "wait", stage: "scanning" });
            void movePrivacyAnswer(false).catch(() => undefined);
          }}
        />
      );
    case "people":
      return (
        <PlaceOthers
          toPlace={view.toPlace}
          onDone={(choices) => {
            // The places to fill in pass through as they are: the engine fills them in the system's window.
            const words = {
              title: l10n.getString("password-title"),
              body: l10n.getString("password-body", { name: "{name}", account: "{account}", password: "{password}" }),
            };
            movePlacePeople(choices, words).then(
              () => {
                setView({ kind: "wait", stage: "waiting" });
              },
              (error: unknown) => {
                setView({ kind: "end", end: { kind: "refused", why: toMoveProblem(error) } });
              },
            );
          }}
        />
      );
    case "plan":
      return (
        <PlanReview
          plan={view.plan}
          approval={approval}
          changedNotice={planChanged}
          leftOut={leftOut}
          onToggleGroup={(role, move) => {
            setLeftOut((out) => (move ? out.filter((r) => r !== role) : out.includes(role) ? out : [...out, role]));
          }}
          chosen={chosen}
          placeNote={placeNote}
          onChoosePlace={(role) => {
            setPlaceNote(null);
            moveChoosePlace(role).then(
              (answer) => {
                if (answer.kind === "chosen") setChosen((c) => ({ ...c, [role]: { path: answer.path, freeBytes: answer.freeBytes } }));
                else if (answer.kind !== "not-chosen") setPlaceNote({ role, kind: answer.kind });
              },
              () => {
                setPlaceNote({ role, kind: "cannot-open" });
              },
            );
          }}
          onUsualPlace={(role) => {
            moveUsualPlace(role).then(
              () => {
                setChosen((c) => Object.fromEntries(Object.entries(c).filter(([r]) => r !== role)));
              },
              () => undefined,
            );
          }}
          onApprovalChange={setApproval}
          onStart={(a) => {
            moveApprove(a, true, leftOut).then(
              () => {
                setView({ kind: "wait", stage: "moving" });
              },
              (error: unknown) => {
                if (toMoveProblem(error).kind === "plan-changed") setPlanChanged(true);
                else setView({ kind: "end", end: { kind: "refused", why: toMoveProblem(error) } });
              },
            );
          }}
          onBack={() => {
            void moveApprove({ revision: view.plan.revision, fingerprint: view.plan.fingerprint }, false).catch(() => undefined);
          }}
        />
      );
    case "progress": {
      const { progress } = view;
      const stop = <StopMove open={stopping} onKeep={() => { setStopping(false); }} onStop={() => { setStopping(false); stopNow(); }} />;
      if (progress.notice?.kind === "drive-problem") {
        return (
          <>
            <DriveProblem
              progress={progress}
              place={progress.notice.place}
              onCarryOn={() => void moveResume().catch(() => undefined)}
              onStop={() => { setStopping(true); }}
              onHelp={() => { openHelp("drive-problem"); }}
            />
            {stop}
          </>
        );
      }
      const handlers = {
        onPause: () => void movePause().catch(() => undefined),
        onResume: () => void moveResume().catch(() => undefined),
        onStop: () => { setStopping(true); },
        onSeeNeedsAction: () => { setExpanded(true); },
      };
      if (mini) {
        return (
          <MiniPanel
            progress={progress}
            onPause={handlers.onPause}
            onResume={handlers.onResume}
            onOpen={() => {
              setMini(false);
            }}
          />
        );
      }
      return (
        <>
          {expanded ? (
            <ProgressDetail progress={progress} onCollapse={() => { setExpanded(false); }} onSeeNeedsAction={handlers.onSeeNeedsAction} />
          ) : (
            <ProgressCompact
              progress={progress}
              {...handlers}
              onMinimise={() => {
                setMini(true);
              }}
            />
          )}
          {stop}
        </>
      );
    }
    case "outcome":
      return (
        <Outcome
          outcome={view.outcome}
          onReport={() => {
            const outcome = view.outcome;
            // A report that cannot be read is not shown as an empty one: the outcome stays.
            moveReport().then(
              (items) => { setView({ kind: "report", outcome, items }); },
              () => undefined,
            );
          }}
          onHome={onHome}
        />
      );
    case "report":
      return <Report items={view.items} onBack={() => { setView({ kind: "outcome", outcome: view.outcome }); }} />;
    case "end":
      return <MoveEnded end={view.end} old={old} onPairAgain={onPairAgain} onHome={onHome} onHelp={() => { openHelp(topicOf(view.end)); }} />;
  }
}
