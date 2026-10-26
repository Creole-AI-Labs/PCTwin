import { Localized, useLocalization } from "@fluent/react";
import { Banner, Button, ProgressBar, ProgressValue } from "../components/kit";
import type { Label } from "../pairing";
import { fileName, formatRate, fraction, useBytes, type SendProgress } from "../quicksend";
import { Steps, type StepItem, type StepState } from "./FrameParts";
import { Heading, QsActions, QsStrip, Route, useAttribute } from "./QsParts";
import { labelText } from "../pairing";

/** The two stages and which one a phase is in. "lost" stops short: the copying has not finished. */
function stages(p: SendProgress, say: (id: string, vars?: Record<string, number>) => string): StepItem[] {
  const at: Record<SendProgress["phase"], [StepState, StepState]> = {
    copying: ["current", "todo"],
    paused: ["current", "todo"],
    checking: ["done", "current"],
    "needs-action": ["done", "current"],
    lost: ["current", "todo"],
    complete: ["done", "done"],
  };
  const [copying, checking] = at[p.phase];
  return [
    { state: copying, title: say("qs-step-copying"), body: say("qs-step-copying-body", { done: p.filesDone, total: p.filesTotal }) },
    { state: checking, title: say("qs-step-checking"), body: say("qs-step-checking-body") },
  ];
}

/**
 * Q12 Your files are on their way (either side): the route, a card with the measured percentage and
 * bar, the two stages in words, and Pause / Keep it compact / Stop. When the send ends, the host
 * moves straight to the Done page.
 * Every number is counted work: the bar is bytes done over bytes to do, the speed is an observed range
 * or absent, nothing animates on its own. A change of phase is announced politely; the bar is not
 * announced on every tick. Keep it compact raises an event so the shell can show lane D's mini panel
 * (this screen does not build another).
 * Events: onPause, onResume, onCompact, onStop.
 */
export default function QuickProgress({
  from,
  to,
  progress: p,
  onPause,
  onResume,
  onCompact,
  onStop,
}: {
  from: Label;
  to: Label;
  progress: SendProgress;
  onPause: () => void;
  onResume: () => void;
  onCompact: () => void;
  onStop: () => void;
}) {
  const { l10n } = useLocalization();
  const attribute = useAttribute();
  const bytes = useBytes();
  const language = typeof document === "undefined" ? "en" : document.documentElement.lang;
  const label = labelText(l10n, to);
  const paused = p.phase === "paused";
  const lost = p.phase === "lost";
  const live = !paused && !lost;
  const value = fraction(p.bytesDone, p.bytesTotal);
  const detail = [
    live && p.current ? l10n.getString("qs-sending-current", { name: fileName(p.current) }) : null,
    live && p.speed ? l10n.getString("qs-sending-speed", { speed: formatRate(p.speed.low, p.speed.high, language) }) : null,
  ].filter(Boolean);
  return (
    <>
      <QsStrip onBack={undefined} />
      <Heading id="qs-sending-title" />
      <Localized id="qs-sending-body" vars={{ label }}>
        <p className="subhead" />
      </Localized>
      <Route from={from} to={to} />

      <div role="status" className="qs-status">
        {paused && (
          <Banner tone="info">
            <Localized id="qs-sending-paused">
              <p />
            </Localized>
          </Banner>
        )}
        {p.phase === "needs-action" && (
          <Banner tone="attention">
            <Localized id="qs-sending-needs-action" vars={{ count: p.failed }}>
              <p />
            </Localized>
          </Banner>
        )}
        {p.phase === "complete" && (
          <section className="qs-statuscard qs-statuscard-good">
            <Localized id="qs-sending-complete-title">
              <h2 />
            </Localized>
          </section>
        )}
        {lost && (
          <Banner tone="attention">
            <Localized id="qs-sending-disconnected" vars={{ label }}>
              <p />
            </Localized>
          </Banner>
        )}
      </div>

      <section className="qs-progress-card" aria-labelledby="qs-sending-card">
        <div className="qs-progress-head">
          <Localized id="qs-sending-card">
            <h2 id="qs-sending-card" />
          </Localized>
          <ProgressValue value={value} />
        </div>
        <ProgressBar
          value={value}
          label={attribute("qs-sending-progress", "aria-label")}
          left={l10n.getString("qs-tally", { count: p.filesDone, size: bytes(p.bytesDone) })}
          right={l10n.getString("qs-progress-of", { total: bytes(p.bytesTotal) })}
        />
        {detail.length > 0 && <p className="qs-small">{detail.join(" · ")}</p>}
      </section>

      <Steps steps={stages(p, (id, vars) => l10n.getString(id, vars))} />

      <QsActions>
        {p.phase === "complete" ? null : paused ? (
          <Button onClick={onResume}>
            <Localized id="qs-resume">
              <span />
            </Localized>
          </Button>
        ) : (
          !lost &&
          p.phase !== "needs-action" && (
            <Button onClick={onPause}>
              <Localized id="qs-pause">
                <span />
              </Localized>
            </Button>
          )
        )}
        {!lost && p.phase !== "complete" && (
          <Button onClick={onCompact}>
            <Localized id="qs-compact">
              <span />
            </Localized>
          </Button>
        )}
        {p.phase !== "complete" && (
          <Button variant="quiet" onClick={onStop}>
            <Localized id="qs-stop">
              <span />
            </Localized>
          </Button>
        )}
      </QsActions>
    </>
  );
}
