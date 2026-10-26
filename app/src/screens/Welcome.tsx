import { Localized } from "@fluent/react";
import type { ReactNode } from "react";
import { ArrowForward, Chevron, Clock, History, MoveBoth, MoveIllustration, Rescue, Restore, Send } from "../icons";
import type { Soon } from "./ComingSoon";

/** One choice card: icon, title, one plain line. The motion pack reads `data-bookmark` and `data-icon-job`. */
function Card({ job, title, body, icon, filled = false, onClick }: { job: string; title: string; body: string; icon: ReactNode; filled?: boolean; onClick: () => void }) {
  return (
    <button type="button" className={`home-card${filled ? " filled" : ""}`} data-bookmark="" data-icon-job={job} onClick={onClick}>
      <span className="home-card-icon">{icon}</span>
      <span className="home-card-text">
        <Localized id={title}>
          <strong />
        </Localized>
        <Localized id={body}>
          <small />
        </Localized>
      </span>
    </button>
  );
}

/**
 * F01 Home, as in the approved Home and Quick Send mockup: the headline with the small two-laptop
 * picture beside it, four cards (Quick Send, Start a full move, Restore setup, Rescue a laptop),
 * then My history. Nothing paid, no sign-in.
 *
 * States:
 * - Returning and First use: the page as above.
 * - Interrupted move (`unfinished` with `onResume`): "Resume your move" sits at the top and reopens
 *   the saved move; the headline is smaller and the picture is left out so the cards stay in view.
 * - Waiting files (`waiting` with `onWaiting`): a quiet line under My history says how many files
 *   wait for which laptop.
 * - A newer version (`update` with `onUpdate`): found when PCTwin opened; "A new version is
 *   available." sits at the top with Update now. While it installs it says PCTwin will open again by
 *   itself; if it could not, it says so and PCTwin carries on as it is. Never beside Resume your
 *   move: a move comes first.
 */
export type UpdateState = { version: string; state: "ready" | "installing" | "failed" | "busy" };

export default function Welcome({
  onStart,
  onSoon,
  onQuick,
  onHistory,
  unfinished,
  onResume,
  waiting,
  onWaiting,
  update,
  onUpdate,
}: {
  onStart: () => void;
  onSoon: (which: Soon) => void;
  onQuick: () => void;
  onHistory: () => void;
  unfinished?: boolean;
  onResume?: () => void;
  waiting?: { count: number; to: string };
  onWaiting?: () => void;
  update?: UpdateState;
  onUpdate?: () => void;
}) {
  const resuming = unfinished === true && onResume !== undefined;
  const updating = !resuming && update !== undefined && onUpdate !== undefined;
  const waits = waiting !== undefined && waiting.count > 0 && onWaiting !== undefined;
  return (
    <div className="home">
      {resuming && (
        <div className="home-resume">
          <Restore className="home-resume-icon" />
          <span className="home-resume-text">
            <Localized id="resume-open">
              <strong />
            </Localized>
            <Localized id="home-resume-body">
              <small />
            </Localized>
          </span>
          <Localized id="home-resume-action" attrs={{ "aria-label": true }}>
            <button type="button" className="home-resume-button" data-bookmark="" onClick={onResume} />
          </Localized>
        </div>
      )}
      {updating && (
        <div className="home-resume home-update" role="status">
          <ArrowForward className="home-resume-icon home-update-icon" />
          <span className="home-resume-text">
            <Localized id="home-update-title">
              <strong />
            </Localized>
            {update.state !== "ready" && (
              <Localized id={update.state === "installing" ? "home-updating" : update.state === "failed" ? "home-update-failed" : "home-update-busy"}>
                <small />
              </Localized>
            )}
          </span>
          {update.state === "ready" && (
            <Localized id="home-update-action">
              <button type="button" className="home-update-button" onClick={onUpdate} />
            </Localized>
          )}
        </div>
      )}
      <div className={`home-headline${resuming ? " compact" : ""}`}>
        <h1 className="home-title">
          <Localized id="welcome-title-line1">
            <span />
          </Localized>
          <Localized id="welcome-title-line2">
            <span />
          </Localized>
        </h1>
        {!resuming && (
          <div className="home-art" aria-hidden="true">
            <MoveIllustration className="flip" />
          </div>
        )}
      </div>
      <Localized id="welcome-body">
        <p className="home-subline" />
      </Localized>
      <div className="home-grid">
        <Card job="send" title="welcome-quick-send" body="home-quick-send-body" icon={<Send />} filled onClick={onQuick} />
        <Card job="move" title="welcome-start" body="home-move-body" icon={<MoveBoth />} onClick={onStart} />
        <Card job="restore" title="welcome-restore" body="home-restore-body" icon={<Restore />} onClick={() => { onSoon("restore"); }} />
        <Card job="rescue" title="welcome-rescue" body="home-rescue-body" icon={<Rescue />} onClick={() => { onSoon("rescue"); }} />
      </div>
      <button type="button" className="home-history" data-bookmark="" onClick={onHistory}>
        <History />
        <span className="home-history-text">
          <Localized id="home-history">
            <strong />
          </Localized>
          <Localized id="home-history-body">
            <small />
          </Localized>
        </span>
        <Chevron className="chevron" />
      </button>
      {waits && (
        <button type="button" className="home-waiting" onClick={onWaiting}>
          <Clock />
          <Localized id="home-waiting" vars={{ count: waiting.count, to: waiting.to }}>
            <span />
          </Localized>
        </button>
      )}
    </div>
  );
}
