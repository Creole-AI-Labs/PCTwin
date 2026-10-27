/**
 * The host of the Phase 2 screens (Task List 2.8): it asks the engine what is true before drawing
 * anything, keeps only updates for the job, session and version shown, picks the screen from the
 * engine's view (never from where the screens navigated), and sends intents one at a time.
 *
 * - Opening (or after a restart): "Checking where things got to" until the engine answers; nothing
 *   saved from an earlier run is shown as current.
 * - The engine has no job, or does not answer: "Status unavailable", with Check again (which only
 *   asks the engine again). Never shown as idle or done.
 * - A refused intent shows why, in words; when the plan or session changed, the view is fetched
 *   again so the person reviews what is true now.
 * - Focus moves to the heading only when a new decision appears (a new screen or stage), never on a
 *   progress update.
 */
import { Localized, useLocalization } from "@fluent/react";
import { useCallback, useEffect, useRef, useState } from "react";
import { Button } from "../components/kit";
import { guardIntent as sendIntent, guardView as fetchView, listenGuard } from "../bridge";
import { arrive, bind, fromSnapshot, intentKey, type IntentKind, type Projection, type ScreenView, type Shown } from "../guard";
import { ChangedItems, ChangesPreview } from "./ChangesPreview";
import FrameFooter from "./FrameFooter";
import { TopStrip } from "./FrameParts";
import { GuardProvider, MeasuredLine, StateLine, type GuardApi } from "./GuardParts";
import { LocalCopy, OwnerTable, PartialReport, SpaceBudget } from "./MoveTypes";
import { Heading } from "./ProgressParts";
import { Rescue } from "./Rescue";
import { SystemPermission } from "./SystemPermission";
import { UndoDone, UndoPreview } from "./UndoScreens";

export type GuardBridge = {
  view: typeof fetchView;
  intent: typeof sendIntent;
  listen: typeof listenGuard;
};

const REAL: GuardBridge = { view: fetchView, intent: sendIntent, listen: listenGuard };

/** A key that changes only when a new decision appears: the screen, and the rescue's or permission's stage. */
export function decisionKey(screen: ScreenView): string {
  if (screen.kind === "rescue") return `rescue:${screen.stage.kind}`;
  if (screen.kind === "permission") return `permission:${screen.stage}`;
  if (screen.kind === "local-copy") return `local-copy:${screen.state}`;
  return screen.kind;
}

/** The screen for one view. */
export function GuardScreen({ view }: { view: Projection }) {
  const s = view.screen;
  switch (s.kind) {
    case "reconciling":
      return <Reconciling progress={view} />;
    case "changes":
      return <ChangesPreview screen={s} />;
    case "changed":
      return <ChangedItems screen={s} />;
    case "space":
      return <SpaceBudget screen={s} />;
    case "local-copy":
      return <LocalCopy screen={s} />;
    case "owners":
      return <OwnerTable screen={s} />;
    case "permission":
      return <SystemPermission screen={s} />;
    case "rescue":
      return <Rescue screen={s} />;
    case "report":
      return <PartialReport screen={s} />;
    case "undo-preview":
      return <UndoPreview screen={s} />;
    case "undo-done":
      return <UndoDone screen={s} />;
  }
}

/** X09: the engine is checking where things got to; no action is offered until it has looked. */
function Reconciling({ progress }: { progress: Projection | null }) {
  return (
    <>
      <TopStrip labelId="guard-strip-checking" />
      <Heading id="guard-checking-title" />
      <Localized id="guard-checking-body">
        <p className="subhead" />
      </Localized>
      {progress && <StateLine />}
      <MeasuredLine progress={progress?.progress ?? null} />
    </>
  );
}

function Unavailable({ onCheck }: { onCheck: () => void }) {
  const { l10n } = useLocalization();
  return (
    <>
      <TopStrip labelId="guard-strip-checking" />
      <Heading id="guard-unavailable-title" />
      <Localized id="guard-unavailable-body">
        <p className="subhead" role="status" />
      </Localized>
      <FrameFooter>
        <Button variant="primary" onClick={onCheck}>
          {l10n.getString("guard-check-again")}
        </Button>
      </FrameFooter>
    </>
  );
}

export default function GuardHost({ bridge = REAL }: { bridge?: GuardBridge }) {
  const [shown, setShown] = useState<Shown>({ kind: "checking" });
  const [refusal, setRefusal] = useState<string | null>(null);
  const pending = useRef(new Set<string>());
  const alive = useRef(true);

  const reload = useCallback(() => {
    void bridge.view().then((snap) => {
      if (alive.current) setShown(fromSnapshot(snap));
    });
  }, [bridge]);

  useEffect(() => {
    alive.current = true;
    let unlisten: (() => void) | null = null;
    // Listen first, then ask: an update between the two is not lost, and the answer settles it.
    void bridge
      .listen((event) => {
        if (!alive.current) return;
        if (event === null) {
          reload();
          return;
        }
        setShown((now) => {
          const next = arrive(now, event);
          if (next.askAgain) queueMicrotask(reload);
          return next.shown;
        });
      })
      .then((u) => {
        if (alive.current) unlisten = u;
        else u();
      });
    reload();
    return () => {
      alive.current = false;
      unlisten?.();
    };
  }, [bridge, reload]);

  const view = shown.kind === "ready" ? shown.view : null;
  // What is on its way: the ref stops a second press before the screen redraws (a double-click);
  // the state draws the busy buttons.
  const [busyKeys, setBusyKeys] = useState<ReadonlySet<string>>(new Set());

  const send = useCallback(
    (intent: IntentKind) => {
      if (!view) return;
      const key = intentKey(view, intent);
      if (pending.current.has(key)) return;
      pending.current.add(key);
      setBusyKeys(new Set(pending.current));
      setRefusal(null);
      void bridge.intent(bind(view, intent)).then((reply) => {
        pending.current.delete(key);
        if (!alive.current) return;
        setBusyKeys(new Set(pending.current));
        if (!reply.ok) {
          setRefusal(reply.refusal);
          if (reply.refusal !== "not-now" && reply.refusal !== "not-shown") reload();
        }
      });
    },
    [bridge, reload, view],
  );

  if (shown.kind === "checking") return <Reconciling progress={null} />;
  if (shown.kind === "unavailable" || !view) return <Unavailable onCheck={reload} />;

  const api: GuardApi = {
    view,
    send,
    busy: (intent) => busyKeys.has(intentKey(view, intent)),
    refusal,
  };
  return (
    <GuardProvider value={api}>
      <div className="g-screen" key={`${view.job}:${String(view.epoch)}:${decisionKey(view.screen)}`}>
        <GuardScreen view={view} />
      </div>
    </GuardProvider>
  );
}
