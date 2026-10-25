import { Localized } from "@fluent/react";
import type { ReactNode } from "react";
import { Chevron, Clock, History, MoveBoth, MoveIllustration, Rescue, Restore, Send } from "../icons";
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
 */
export default function Welcome({
  onStart,
  onSoon,
  onQuick,
  onHistory,
  unfinished,
  onResume,
  waiting,
  onWaiting,
}: {
  onStart: () => void;
  onSoon: (which: Soon) => void;
  onQuick: () => void;
  onHistory: () => void;
  unfinished?: boolean;
  onResume?: () => void;
  waiting?: { count: number; to: string };
  onWaiting?: () => void;
}) {
  const resuming = unfinished === true && onResume !== undefined;
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
