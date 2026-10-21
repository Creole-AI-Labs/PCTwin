import { useLocalization } from "@fluent/react";
import { useEffect, useRef, useState } from "react";
import { helpCode, listenMove, moveApprove, moveBegin, movePause, moveReport, moveResume, moveStop, toMoveProblem, type HelpStep, type HelpTopic, type MoveEvent, type MoveStage } from "../bridge";
import type { HelpCode } from "../people";
import GetHelp from "./GetHelp";
import type { MoveOutcome, MoveProgress, ReportItem } from "../move";
import { setWindowMode } from "../native";
import type { Approval, FolderRole, PlanView } from "../plan";
import DriveProblem from "./DriveProblem";
import MoveEnded, { type MoveEnd } from "./MoveEnded";
import MoveWaiting from "./MoveWaiting";
import Outcome from "./Outcome";
import PlanReview from "./PlanReview";
import ProgressCompact from "./ProgressCompact";
import ProgressDetail from "./ProgressDetail";
import Report from "./Report";
import StopMove from "./StopMove";

/** What the move page shows now. */
type View =
  | { kind: "wait"; stage: MoveStage["kind"] }
  | { kind: "plan"; plan: PlanView }
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
      return "move-failed";
  }
}

/** How far the move got, for a help code, by what the page showed last. */
export function stepOf(view: View, old: boolean): HelpStep {
  switch (view.kind) {
    case "wait":
      if (view.stage === "scanning") return "finding";
      if (view.stage === "moving") return "moving";
      return old ? "approving" : "finding";
    case "plan":
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
      return view.kind === "wait" ? { kind: "wait", stage: event.stage.kind } : view;
    case "plan":
      return { kind: "plan", plan: event.plan };
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
export default function MoveHost({ old, onPairAgain, onHome }: { old: boolean; onPairAgain: () => void; onHome: () => void }) {
  const { l10n } = useLocalization();
  const [view, setView] = useState<View>({ kind: "wait", stage: old ? "scanning" : "waiting" });
  const [approval, setApproval] = useState<Approval | null>(null);
  const [planChanged, setPlanChanged] = useState(false);
  const [expanded, setExpanded] = useState(false);
  const [stopping, setStopping] = useState(false);
  const started = useRef(false);
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
      if (live) setView((v) => next(v, event));
    }).then((stop) => {
      if (!live) {
        stop();
        return;
      }
      unlisten = stop;
      // Started once, after the page listens, so no word from the engine is missed.
      if (started.current) return;
      started.current = true;
      const fallback = Object.fromEntries(MAY_LACK.map((role) => [role, `${l10n.getString("move-fallback-folder")}/${l10n.getString(`plan-role-${role}`)}`]));
      moveBegin(fallback).catch((error: unknown) => {
        if (live) setView({ kind: "end", end: { kind: "refused", why: toMoveProblem(error) } });
      });
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

  switch (view.kind) {
    case "wait":
      return <MoveWaiting stage={view.stage} old={old} onStop={stopNow} />;
    case "plan":
      return (
        <PlanReview
          plan={view.plan}
          approval={approval}
          changedNotice={planChanged}
          onApprovalChange={setApproval}
          onStart={(a) => {
            moveApprove(a, true).then(
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
      return (
        <>
          {expanded ? (
            <ProgressDetail progress={progress} onCollapse={() => { setExpanded(false); }} onSeeNeedsAction={handlers.onSeeNeedsAction} />
          ) : (
            <ProgressCompact progress={progress} {...handlers} onMinimise={() => void setWindowMode("mini").catch(() => undefined)} />
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
