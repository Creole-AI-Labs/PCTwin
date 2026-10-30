import { useLocalization } from "@fluent/react";
import { useEffect, useRef, useState } from "react";
import { useSettle } from "../motion/screen";
import {
  fullmoveUndo,
  moveAppdataAnswer,
  helpCode,
  listenFinishDelta,
  listenMove,
  moveApprove,
  moveBegin,
  moveChoosePlace,
  moveChooseWhat,
  movePause,
  movePeopleHere,
  movePlacePeople,
  movePrivacyAnswer,
  moveUsualPlace,
  openPrivacySettings,
  moveReport,
  moveResume,
  rescueBegin,
  moveRetry,
  moveStop,
  toMoveProblem,
  type ChoiceView,
  type ConnectionView,
  type HelpStep,
  type HelpTopic,
  type MoveEvent,
  type OthersWhy,
  type PeopleHere,
  type PeopleToPlace,
  type RescueDrive as RescueDriveView,
  type Peer,
  type WaitStage,
} from "../bridge";
import AcceptMove from "./AcceptMove";
import BeforeYouStart from "./BeforeYouStart";
import ChooseWhat from "./ChooseWhat";
import FullDiskAccess from "./FullDiskAccess";
import type { BeforeItem } from "../people";
import PlaceOthers from "./PlaceOthers";
import WhoHere from "./WhoHere";
import { RescueDrive } from "./rescueContext";
import type { HelpCode } from "../people";
import GetHelp from "./GetHelp";
import type { MoveOutcome, MoveProgress, ReportItem } from "../move";
import { setWindowMode, type WindowMode } from "../native";
import MiniPanel from "./MiniPanel";
import type { DeltaView } from "../finish";
import type { Approval, FolderRole, PlanView } from "../plan";
import DriveProblem from "./DriveProblem";
import MoveEnded, { type MoveEnd } from "./MoveEnded";
import MoveWaiting from "./MoveWaiting";
import Outcome from "./Outcome";
import PlanReview, { type ChosenPlace, type PlaceNote } from "./PlanReview";
import MeetLaptop from "./MeetLaptop";
import CheckConnection from "./CheckConnection";
import { connectionCheck, peerReadiness, type Greeted, type Naming, type WaitingState } from "../connection";
import ConnectionEnded from "./ConnectionEnded";
import WaitingForLaptop from "./WaitingForLaptop";
import { labelText } from "../pairing";
import ProgressCompact from "./ProgressCompact";
import ProgressDetail from "./ProgressDetail";
import Report from "./Report";
import { AppDataComing, CloseApps, LiveWait, Rebuilding, installDefaults, type InstallChoices } from "./FullMoveParts";
import UndoHost from "./UndoHost";
import FinishHost from "./FinishHost";
import type { AppsOpen, DataProgress, RebuildStep, LiveWait as LiveWaitView } from "../fullmove";
import { ownFolderName } from "../ownFolder";
import StopMove from "./StopMove";
import MoveChanged from "./MoveChanged";
import type { ChangesView } from "../deliver";
import FinishCatchup from "./FinishCatchup";
import RepairItems from "./RepairItems";

/** What the move page shows now. */
type View =
  /** The old laptop: who lives here, before the move starts (M06). */
  | { kind: "who"; here: PeopleHere }
  /** The old laptop: the person chooses what moves (decided 28 October 2026). */
  | { kind: "choose"; choice: ChoiceView }
  | { kind: "wait"; stage: WaitStage }
  | { kind: "plan"; plan: PlanView }
  /** The new laptop: where the other people's things go (M06). */
  | { kind: "people"; toPlace: PeopleToPlace }
  /** The old laptop, a Mac: some of the mover's places were kept private (Full Disk Access). */
  | { kind: "privacy"; places: number }
  | { kind: "progress"; progress: MoveProgress }
  | { kind: "outcome"; outcome: MoveOutcome }
  | { kind: "report"; outcome: MoveOutcome; items: ReportItem[] }
  | { kind: "end"; end: MoveEnd }
  /** The connection dropped once the plan was approved: waiting for the other laptop (C09). */
  | { kind: "reconnecting"; link: Waiting }
  /** The laptops could not find each other again: the move ended, what moved is kept (C10). */
  | { kind: "link-ended"; link: LinkEnded }
  /** What moved before the connection ended for good, from the C10 dialog. */
  | { kind: "moved"; link: LinkEnded; items: ReportItem[] }
  /** The new laptop, after the files: apps, settings and special categories being set up. */
  | { kind: "rebuilding"; step: RebuildStep | null }
  /** Undo of the full move's apps and settings, through the flow guard (R05, R06). */
  | { kind: "undo"; outcome: MoveOutcome }
  /** Finishing the move (2.7), from the outcome on the new laptop: the newest move. */
  | { kind: "finish"; outcome: MoveOutcome }
  /** The new laptop: fix what did not finish, from the outcome (the newest move). */
  | { kind: "repair"; outcome: MoveOutcome }
  /** The new laptop, paired to bring it up to date: what changed on the old laptop, to review. `null` could not be read. */
  | { kind: "catchup"; delta: DeltaView | null }
  /** The new laptop: the old laptop's app data coming, before the files. */
  | { kind: "appdata"; progress: DataProgress }
  /** The old laptop: apps to close before their data can be read. */
  | { kind: "close-apps"; open: AppsOpen }
  /** The old laptop: files an app nobody can name holds open, waited for by PCTwin itself. */
  | { kind: "live-wait"; wait: LiveWaitView }
  /** Something changed after the person said yes: only that is asked about again (2.9, X06). */
  | { kind: "changed"; changes: ChangesView };

type Waiting = Extract<ConnectionView, { kind: "waiting" }>;
type LinkEnded = Extract<ConnectionView, { kind: "ended" }>;

/** C09's figures from the engine's: measured bytes as gigabytes, and why it waits. */
export function waitingOf(link: Waiting | LinkEnded, otherLaptop: string): WaitingState {
  const gb = (bytes: number) => bytes / 1_000_000_000;
  return {
    kind: link.kind === "ended" ? "pairing-expired" : link.nothingNew ? "unreachable" : "reconnecting",
    otherLaptop,
    savedGb: gb(link.savedBytes),
    totalGb: gb(link.totalBytes),
    lastSeenSeconds: link.kind === "waiting" ? link.lastSeenSeconds : null,
  };
}

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
    case "place-changed":
      return "move-failed";
  }
}

/** How far the move got, for a help code, by what the page showed last. */
export function stepOf(view: View, old: boolean): HelpStep {
  switch (view.kind) {
    case "who":
    case "privacy":
      return "finding";
    case "choose":
      return "approving";
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
    case "reconnecting":
    case "link-ended":
    case "moved":
    case "rebuilding":
    case "appdata":
    case "close-apps":
    case "live-wait":
    case "changed":
      return "moving";
    case "undo":
    case "finish":
    case "repair":
      return "finished";
    case "catchup":
      return "approving";
  }
}

/** Where a moved event takes the page. Nothing moves the page off an end, and a plan or progress never goes back to waiting. */
export function next(view: View, event: MoveEvent): View {
  if (view.kind === "end" || view.kind === "outcome" || view.kind === "report" || view.kind === "link-ended" || view.kind === "moved" || view.kind === "undo" || view.kind === "finish" || view.kind === "repair") return view;
  // The catch-up review stays until the approved changes start moving, or the move ends.
  if (view.kind === "catchup") {
    if (event.kind === "progress") return { kind: "progress", progress: event.progress };
    if (event.kind === "finished") return { kind: "outcome", outcome: event.outcome };
    if (event.kind === "problem") return { kind: "end", end: { kind: "ended", why: event.problem } };
    return view;
  }
  // What changed after approval takes the page until the person answers, wherever the move is.
  if (event.kind === "changed") return { kind: "changed", changes: event.changes };
  // After the files, the full move sets up apps and settings: only its own steps, the outcome or a problem move the page on.
  if (view.kind === "rebuilding") {
    if (event.kind === "rebuild") return { kind: "rebuilding", step: event.step };
    if (event.kind === "finished") return { kind: "outcome", outcome: event.outcome };
    if (event.kind === "problem") return { kind: "end", end: { kind: "ended", why: event.problem } };
    return view;
  }
  // Before the files, the old laptop's app data: its progress, and on the old laptop the apps to
  // close. The first measured progress of the files takes the page on.
  if (event.kind === "appdata") return view.kind === "progress" ? view : { kind: "appdata", progress: event.progress };
  // The apps to close, and the person's files an app holds open, each listed on its own.
  if (event.kind === "appdata-open") return { kind: "close-apps", open: event.open };
  if (event.kind === "live-wait") return { kind: "live-wait", wait: event.wait };
  if (view.kind === "appdata" && event.kind === "stage") return view;
  switch (event.kind) {
    case "link":
      // Waiting for the other laptop takes the page until connected again (then the progress
      // follows), or until the move ends.
      if (event.link.kind === "waiting") return { kind: "reconnecting", link: event.link };
      if (event.link.kind === "ended") return { kind: "link-ended", link: event.link };
      return view.kind === "reconnecting" ? { kind: "wait", stage: "moving" } : view;
    case "stage":
      // Others not moved is a note, not a page of its own.
      if (event.stage.kind === "others-not-moved") return view;
      if (event.stage.kind === "rebuilding") return { kind: "rebuilding", step: null };
      return view.kind === "wait" ? { kind: "wait", stage: event.stage.kind } : view;
    case "plan":
      return { kind: "plan", plan: event.plan };
    case "choose":
      return { kind: "choose", choice: event.choice };
    case "people":
      return { kind: "people", toPlace: event.toPlace };
    case "privacy":
      return { kind: "privacy", places: event.privacy.places };
    case "found":
    case "peer":
      // A count, and the two laptops, for the waiting page: not pages of their own.
      return view;
    case "progress":
      // A page waiting on the person (the plan, who is moving, where others go, privacy) is only
      // left by their answer, never by a progress word.
      if (view.kind === "plan" || view.kind === "who" || view.kind === "people" || view.kind === "privacy" || view.kind === "choose" || view.kind === "changed") return view;
      return view.kind === "reconnecting" ? view : { kind: "progress", progress: event.progress };
    case "finished":
      return { kind: "outcome", outcome: event.outcome };
    case "problem":
      return { kind: "end", end: { kind: "ended", why: event.problem } };
    case "rebuild":
      return view.kind === "changed" ? view : { kind: "rebuilding", step: event.step };
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
  if (view.kind === "plan" || view.kind === "report" || view.kind === "moved") return "expanded";
  return "everyday";
}

type HostProps = {
  old: boolean;
  onPairAgain: () => void;
  onHome: () => void;
  /** The window size the page asks for, so the frame can fit it (the mini panel has no header). */
  onWindow?: (mode: WindowMode) => void;
  /** Rescue a laptop (2.6): the drive chosen, read as the old laptop. This laptop is the new one. */
  rescue?: RescueDriveView;
};

/** The move's pages, with the rescued drive's name for the pages whose words must fit a drive. */
export default function MoveHost(props: HostProps) {
  return (
    <RescueDrive.Provider value={props.rescue?.name ?? null}>
      <MoveHostPages {...props} />
    </RescueDrive.Provider>
  );
}

function MoveHostPages({ old, onPairAgain, onHome, onWindow, rescue }: HostProps) {
  const { l10n } = useLocalization();
  const [view, setView] = useState<View>({ kind: "wait", stage: old ? "scanning" : "waiting" });
  // Each step of the move settles in like a new screen; one that ends or loses the link arrives without travel.
  useSettle(view.kind, { calm: view.kind === "end" || view.kind === "link-ended" || view.kind === "reconnecting" });
  const [approval, setApproval] = useState<Approval | null>(null);
  const [planChanged, setPlanChanged] = useState(false);
  // The folders the person unticked on the plan: they stay on the old laptop.
  const [leftOut, setLeftOut] = useState<FolderRole[]>([]);
  // The secret lines of app data the person ticked, one by one: none unless ticked.
  const [picked, setPicked] = useState<number[]>([]);
  // The older apps here the person ticked "Update it" for: none unless ticked.
  const [updates, setUpdates] = useState<number[]>([]);
  // The smaller-laptop proposal (2.6): leave the files used longest ago on the old laptop. Not unless ticked.
  const [leaveBehind, setLeaveBehind] = useState(false);
  // The places the person chose for some folders (M05), and why the last one was not taken.
  const [chosen, setChosen] = useState<Partial<Record<FolderRole, ChosenPlace>>>({});
  const [placeNote, setPlaceNote] = useState<PlaceNote | null>(null);
  // The overview (M03) comes once, before the first plan; a later revision goes straight to it.
  const [overviewSeen, setOverviewSeen] = useState(false);
  // Several old laptops into one: this old laptop's things in their own folder, on unless turned off.
  const [ownFolderOn, setOwnFolderOn] = useState(true);
  // The plan's two install choices, once the person changed them; until then the plan's defaults.
  const [installs, setInstalls] = useState<InstallChoices | null>(null);
  // What the approval asked for, for the page shown while apps are set up.
  const [waitsForFreeWifi, setWaitsForFreeWifi] = useState(false);
  // Why the other people ticked were not moved, when they were not: said until the move ends.
  const [othersWhy, setOthersWhy] = useState<OthersWhy | null>(null);
  // How much of the mover's own things the scan has found so far (M02).
  const [found, setFound] = useState<number | null>(null);
  // The two laptops, as each said when the move started (C06), and the connection (C07): each
  // shown once, as its own page, before the waiting page.
  const [peer, setPeer] = useState<Peer | null>(null);
  // A rescue has no other laptop to meet or check the connection with (C06, C07).
  const [intro, setIntro] = useState<"meet" | "check" | "done">(rescue ? "done" : "meet");
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
    // The place for the names passes through as it is: the engine fills it in a Mac's administrator window.
    const approve = l10n.getString("approve-prompt", { names: "{names}" });
    (rescue ? rescueBegin(rescue.id, fallback) : moveBegin(fallback, others, approve)).catch((error: unknown) => {
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

  // Bring me up to date: the new laptop gets the old laptop's changes to review instead of a plan.
  useEffect(() => {
    if (old) return;
    let live = true;
    let unlisten: (() => void) | null = null;
    void listenFinishDelta((delta) => {
      if (live) setView((v) => (v.kind === "wait" || v.kind === "catchup" || v.kind === "plan" ? { kind: "catchup", delta } : v));
    }).then((stop) => {
      if (live) unlisten = stop;
      else stop();
    });
    return () => {
      live = false;
      unlisten?.();
    };
  }, [old]);

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
        old={old}
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
  // The other laptop, as C09 names it: its label where pairing gave one (the old laptop's, so only
  // on the new laptop), else "the other laptop".
  // The other laptop by its own name: what it said in its greeting, else what pairing showed.
  const otherName = peer?.theirsName ?? (!old ? peer?.other : null) ?? null;
  const otherLaptop = otherName ? labelText(l10n, otherName) : l10n.getString("qs-label-other");

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
    case "choose":
      return (
        <ChooseWhat
          view={view.choice}
          onContinue={(choice) => {
            setView({ kind: "wait", stage: "waiting" });
            moveChooseWhat(choice).catch((error: unknown) => {
              setView({ kind: "end", end: { kind: "refused", why: toMoveProblem(error) } });
            });
          }}
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
              approve: l10n.getString("approve-prompt", { names: "{names}" }),
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
    case "plan": {
      // The answers that go with the approval, from either page: the own folder's name when the
      // plan offers it and the person kept it on, and the two install choices.
      const plan = view.plan;
      // The folder is named after the old laptop as this page names it.
      const folder = ownFolderName((id, args) => l10n.getString(id, args), otherName ? labelText(l10n, otherName) : null);
      const ownFolder = plan.ownFolderOffered && ownFolderOn ? folder : null;
      const chosenInstalls = installs ?? installDefaults(plan.rebuild?.downloads);
      const approveWith = (a: { revision: number; fingerprint: string }, out: FolderRole[], secrets: number[], ups: number[]) => {
        setWaitsForFreeWifi(chosenInstalls.waitFreeWifi && (plan.rebuild?.downloads.apps ?? 0) > 0);
        moveApprove(a, true, out, secrets, ups, leaveBehind, ownFolder, chosenInstalls.waitFreeWifi, chosenInstalls.reuseInstallers).then(
          () => {
            setView({ kind: "wait", stage: "moving" });
          },
          (error: unknown) => {
            if (toMoveProblem(error).kind === "plan-changed") setPlanChanged(true);
            else setView({ kind: "end", end: { kind: "refused", why: toMoveProblem(error) } });
          },
        );
      };
      // The choosing was done on the old laptop: here one tap accepts what comes (decided 28
      // October 2026); the full plan is there for the details, and for a place when it does not fit.
      if (!overviewSeen) {
        const key = { revision: view.plan.revision, fingerprint: view.plan.fingerprint };
        return (
          <AcceptMove
            plan={view.plan}
            from={otherLaptop}
            ownFolder={ownFolder}
            onAccept={() => {
              approveWith(key, [], [], []);
            }}
            onDetails={() => {
              setOverviewSeen(true);
            }}
            leaveBehind={leaveBehind}
            onLeaveBehind={setLeaveBehind}
            onNotNow={() => {
              void moveApprove(key, false).catch(() => undefined);
            }}
          />
        );
      }
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
          leaveBehind={leaveBehind}
          onLeaveBehind={setLeaveBehind}
          picked={picked}
          onPick={(line, bring) => {
            setPicked((p) => (bring ? (p.includes(line) ? p : [...p, line]) : p.filter((l) => l !== line)));
          }}
          updates={updates}
          onUpdate={(app, update) => {
            setUpdates((u) => (update ? (u.includes(app) ? u : [...u, app]) : u.filter((a) => a !== app)));
          }}
          ownFolder={{ on: ownFolderOn, laptop: otherLaptop, folder, onChange: setOwnFolderOn }}
          installs={chosenInstalls}
          onInstalls={setInstalls}
          onStart={(a) => {
            approveWith(a, leftOut, picked, updates);
          }}
          onBack={() => {
            // Back from the details is back to Accept, never a "no": declining is Not now
            // (found in the two-copy test, 29 October 2026).
            setOverviewSeen(false);
          }}
        />
      );
    }
    case "progress": {
      // Both laptops by their names on the route strip, from the greeting.
      const named = peer ? { old: old ? (peer.mineName ?? view.progress.laptops.old) : (peer.theirsName ?? view.progress.laptops.old), new: old ? peer.theirsName : peer.mineName } : view.progress.laptops;
      const progress = { ...view.progress, laptops: named };
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
              tips={{ old, onBattery: peer?.mine.power.kind === "battery", slow: peer?.link?.slow ?? false }}
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
          old={old}
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
          onRepair={
            old
              ? undefined
              : () => {
                  setView({ kind: "repair", outcome: view.outcome });
                }
          }
          onFinish={
            old
              ? undefined
              : () => {
                  setView({ kind: "finish", outcome: view.outcome });
                }
          }
          onUndo={
            view.outcome.rebuilt?.canUndo
              ? () => {
                  const outcome = view.outcome;
                  fullmoveUndo(outcome.rebuilt?.undoId ?? null).then(
                    () => {
                      setView({ kind: "undo", outcome });
                    },
                    () => undefined,
                  );
                }
              : undefined
          }
        />
      );
    case "changed":
      return (
        <MoveChanged
          changes={view.changes}
          onDone={() => {
            setView({ kind: "wait", stage: "moving" });
          }}
        />
      );
    case "rebuilding":
      return <Rebuilding step={view.step} waitsForFreeWifi={waitsForFreeWifi} />;
    case "appdata":
      return <AppDataComing progress={view.progress} />;
    case "live-wait":
      return <LiveWait wait={view.wait} onStop={stopNow} />;
    case "close-apps":
      return (
        <CloseApps
          open={view.open}
          onAnswer={(retry) => {
            setView({ kind: "wait", stage: "moving" });
            void moveAppdataAnswer(retry).catch(() => undefined);
          }}
        />
      );
    case "undo":
      return (
        <UndoHost
          onClosed={() => {
            setView({ kind: "outcome", outcome: view.outcome });
          }}
        />
      );
    case "finish":
      return (
        <FinishHost
          id={null}
          onBack={() => {
            setView({ kind: "outcome", outcome: view.outcome });
          }}
          onPair={onPairAgain}
        />
      );
    case "repair":
      return (
        <RepairItems
          onBack={() => {
            setView({ kind: "outcome", outcome: view.outcome });
          }}
        />
      );
    case "catchup":
      return <FinishCatchup id={null} delta={view.delta} onBack={onHome} />;
    case "report":
      return <Report items={view.items} rebuilt={view.outcome.rebuilt} people={view.outcome.people} onBack={() => { setView({ kind: "outcome", outcome: view.outcome }); }} />;
    case "reconnecting":
      return (
        <>
          <WaitingForLaptop
            waiting={waitingOf(view.link, otherLaptop)}
            // The new laptop does the trying; the old one waits for it to come back.
            onRetry={old ? undefined : () => void moveRetry().catch(() => undefined)}
            onPairAgain={onPairAgain}
            onStop={() => {
              setStopping(true);
            }}
          />
          <StopMove open={stopping} onKeep={() => { setStopping(false); }} onStop={() => { setStopping(false); stopNow(); }} />
        </>
      );
    case "link-ended": {
      const link = view.link;
      return (
        <>
          <WaitingForLaptop waiting={waitingOf(link, otherLaptop)} onPairAgain={onPairAgain} />
          <ConnectionEnded
            movedSomething={link.movedSomething}
            onPairAgain={onPairAgain}
            onHome={onHome}
            onSeeMoved={() => {
              // A report that cannot be read is not shown as an empty one: the dialog stays.
              moveReport().then(
                (items) => { setView({ kind: "moved", link, items }); },
                () => undefined,
              );
            }}
          />
        </>
      );
    }
    case "moved":
      return <Report items={view.items} onBack={() => { setView({ kind: "link-ended", link: view.link }); }} />;
    case "end":
      return <MoveEnded end={view.end} old={old} onPairAgain={onPairAgain} onHome={onHome} onHelp={() => { openHelp(topicOf(view.end)); }} />;
  }
}
