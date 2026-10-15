/** Pieces shared by the progress and result screens: measured bar, counts, notices, state words. */
import { Localized, useLocalization } from "@fluent/react";
import type { ReactNode } from "react";
import { Banner, Badge, ProgressBar, ProgressValue, ResultChip, Row, RowList } from "../components/kit";
import { ArrowBack, ArrowForward, Info } from "../icons";
import { formatBytes, formatSpeed, minutesRange, measuredPercent, pageLocale, type Evidence, type Outcome, type Phase, type MoveProgress, type Reason } from "../move";
import "./progress.css";

export { Eyebrow, Heading } from "./PairParts";

/** An attribute of a translated message, like the progress bars' `.aria-label`. */
export function useAttribute() {
  const { l10n } = useLocalization();
  return (id: string, name: string): string => {
    const bundle = l10n.getBundle(id);
    const pattern = bundle?.getMessage(id)?.attributes[name];
    return bundle && pattern ? bundle.formatPattern(pattern) : "";
  };
}

/** The measured time left as words ("About 15 to 20 minutes left"), or null when nothing is measured. */
export function useTimeLeft(progress: Pick<MoveProgress, "minutesLeft">): string | null {
  const { l10n } = useLocalization();
  if (!progress.minutesLeft) return null;
  const range = l10n.getString("progress-minutes", minutesRange(progress.minutesLeft));
  return l10n.getString("progress-time-left", { range });
}

/** The measured speed as a range ("58 to 66 MB/s, measured"), or null. */
export function useSpeed(progress: Pick<MoveProgress, "speed">): string | null {
  const { l10n } = useLocalization();
  if (!progress.speed) return null;
  const s = formatSpeed(progress.speed, pageLocale());
  return l10n.getString("progress-speed", { speed: l10n.getString("progress-speed-range", s) });
}

/** The big measured percent, or a sentence saying nothing is measured yet (never a made-up number). */
export function PercentValue({ pct }: { pct: number | null }) {
  if (pct === null) {
    return (
      <Localized id="progress-percent-unknown">
        <p className="mv-unknown" />
      </Localized>
    );
  }
  return <ProgressValue value={pct / 100} />;
}

/**
 * The bar and what is under it: bytes written and checked, and the time left when it is measured.
 * The number comes only from bytes written and checked; it never reaches 100 while anything is
 * unfinished; with no size the bar is empty and still.
 */
export function MeasuredBar({ progress }: { progress: MoveProgress }) {
  const { l10n } = useLocalization();
  const attr = useAttribute();
  const pct = measuredPercent(progress);
  const locale = pageLocale();
  const timeLeft = useTimeLeft(progress);
  const speed = useSpeed(progress);
  return (
    <div className="mv-measure">
      <ProgressBar
        value={pct === null ? null : pct / 100}
        label={attr("progress-percent", "aria-label")}
        left={pct === null ? undefined : l10n.getString("progress-amount", { done: formatBytes(progress.bytes.checked, locale), total: formatBytes(progress.bytes.total, locale) })}
        right={timeLeft ?? l10n.getString("progress-time-measuring")}
      />
      {speed && <p className="mv-speed">{speed}</p>}
    </div>
  );
}

/** Back on the left, the screen's eyebrow on the right (the mockups' top line). */
export function Topline({ onBack, eyebrow }: { onBack?: () => void; eyebrow: string }) {
  return (
    <div className="mv-topline">
      {onBack ? (
        <button type="button" className="back" onClick={onBack}>
          <ArrowBack />
          <Localized id="report-back">
            <span />
          </Localized>
        </button>
      ) : (
        <span />
      )}
      <Localized id={eyebrow}>
        <p className="eyebrow mv-eyebrow" />
      </Localized>
    </div>
  );
}

/** The paused move's panel: a title, the measured percent, the bar and a line under it. */
export function ProgressPanel({ title, progress, note }: { title: string; progress: MoveProgress; note?: string }) {
  const attr = useAttribute();
  const pct = measuredPercent(progress);
  return (
    <section className="mv-panel" aria-label={title}>
      <div className="mv-panel-title">
        <strong>{title}</strong>
        {pct !== null && <ProgressValue value={pct / 100} />}
      </div>
      <ProgressBar value={pct === null ? null : pct / 100} label={attr("progress-percent", "aria-label")} />
      {note && <p className="mv-panel-note">{note}</p>}
    </section>
  );
}

/** Each count on its own row, with its state in an icon and a word. All measured. Needs-action and failed appear only when there are some. */
export function CountRows({ counts }: { counts: MoveProgress["counts"] }) {
  const { l10n } = useLocalization();
  return (
    <RowList>
      <Row title={l10n.getString("progress-count-copied", { count: counts.done })} end={<ResultChip result="copied" />} />
      <Row title={l10n.getString("progress-count-checking", { count: counts.checking })} end={<PhaseBadge phase="checking" paused={false} />} />
      <Row title={l10n.getString("progress-count-waiting", { count: counts.waiting + counts.copying })} end={<PhaseBadge phase="waiting" paused={false} />} />
      {counts.needsAction > 0 && <Row title={l10n.getString("progress-count-needs-action", { count: counts.needsAction })} end={<ResultChip result="needs-action" />} />}
      {counts.failed > 0 && <Row title={l10n.getString("progress-count-failed", { count: counts.failed })} end={<ResultChip result="failed" />} />}
    </RowList>
  );
}

/** The item being worked on, named by what is happening to it. */
export function NowLine({ current }: { current: MoveProgress["current"] }) {
  if (!current) return null;
  return (
    <Localized id={current.step === "copying" ? "progress-current-copying" : "progress-current-checking"} vars={{ name: current.name }}>
      <p className="mv-now" />
    </Localized>
  );
}

/** What needs the person's eye: a dropped connection, a low battery, items that need action. */
export function Notices({ progress, onSeeNeedsAction }: { progress: MoveProgress; onSeeNeedsAction: () => void }) {
  const { l10n } = useLocalization();
  const needs = progress.counts.needsAction;
  return (
    <div className="mv-notices">
      {progress.notice?.kind === "link-lost" && (
        <Banner tone="attention">
          <p>{l10n.getString("progress-interrupted")}</p>
        </Banner>
      )}
      {progress.notice?.kind === "low-battery" && (
        <Banner tone="attention">
          <p>{l10n.getString("progress-low-battery", { label: progress.notice.label })}</p>
        </Banner>
      )}
      {needs > 0 && (
        <Banner tone="info">
          <p>{l10n.getString("progress-needs-action", { count: needs })}</p>
          <button type="button" className="mv-link" onClick={onSeeNeedsAction}>
            {l10n.getString("progress-see-needs-action")}
          </button>
        </Banner>
      )}
    </div>
  );
}

/* ---------- Item state: always an icon and a word ---------- */

const svg = { width: 16, height: 16, viewBox: "0 0 24 24", fill: "none", stroke: "currentColor", strokeWidth: 2, strokeLinecap: "round" as const, strokeLinejoin: "round" as const, "aria-hidden": true, focusable: false };

function WaitingIcon() {
  return (
    <svg {...svg}>
      <circle cx="12" cy="12" r="8" />
    </svg>
  );
}

function CheckingIcon() {
  return (
    <svg {...svg}>
      <circle cx="11" cy="11" r="6" />
      <path d="M16 16l4 4" />
    </svg>
  );
}

function PauseIcon() {
  return (
    <svg {...svg}>
      <path d="M9 6v12M15 6v12" />
    </svg>
  );
}

/** A file still moving: its phase as an icon and a word. Nothing here is a check mark; that is for finished work. */
export function PhaseBadge({ phase, paused }: { phase: Phase; paused: boolean }) {
  const { l10n } = useLocalization();
  if (phase === "waiting") {
    return (
      <Badge icon={paused ? <PauseIcon /> : <WaitingIcon />}>{l10n.getString(paused ? "progress-state-paused" : "progress-state-waiting")}</Badge>
    );
  }
  if (phase === "copying") return <Badge tone="neutral" icon={<ArrowForward className="mv-icon" />}>{l10n.getString("progress-state-copying")}</Badge>;
  if (phase === "checking") return <Badge icon={<CheckingIcon />}>{l10n.getString("progress-state-checking")}</Badge>;
  if (phase === "putting-in-place") return <Badge icon={<ArrowForward className="mv-icon" />}>{l10n.getString("progress-state-placing")}</Badge>;
  return <Badge icon={<Info className="mv-icon" />}>{l10n.getString("progress-state-existing")}</Badge>;
}

/** The reason an item did not finish: what happened, then what to do. */
export function ReasonText({ reason }: { reason: Reason }) {
  return (
    <Localized id={`reason-${reason}`}>
      <p className="mv-reason" />
    </Localized>
  );
}

/** How a copied file was checked: read back from the drive, or from this laptop's memory. Two different lines. */
export function EvidenceLine({ outcome }: { outcome: Outcome }) {
  if (outcome.status === "failed") {
    return (
      <Localized id="evidence-not-checked">
        <p className="mv-evidence" />
      </Localized>
    );
  }
  if (outcome.status !== "copied") return null;
  const memory = outcome.evidence.from === "memory";
  return (
    <Localized id={memory ? "evidence-memory" : "evidence-read-back"}>
      <p className={memory ? "mv-evidence mv-evidence-memory" : "mv-evidence"} data-evidence={outcome.evidence.from} />
    </Localized>
  );
}

export function isMemory(e: Evidence | undefined): boolean {
  return e?.from === "memory";
}

/** A result chip with the reason, when it has one. */
export function ResultCell({ outcome }: { outcome: Outcome }): ReactNode {
  return (
    <>
      <ResultChip result={outcome.status} />
      {"reason" in outcome && outcome.reason && <ReasonText reason={outcome.reason} />}
    </>
  );
}
