/** Pieces shared by the pairing screens: the Back and progress strip, the focus-taking heading, eyebrow, and problem notice. */
import { Localized, useLocalization } from "@fluent/react";
import { createContext, useContext, useEffect, useRef } from "react";
import { Banner, Button } from "../components/kit";
import { ArrowBack, ArrowForward, Info } from "../icons";
import { HEADS_UP_TEXT, markHeadsUpSeen, type HeadsUp, type Problem } from "../pairing";
import "./pairing.css";

/**
 * The pairing words that name the laptops as old and new. Quick Send pairs with the same pages, but
 * its laptops are this one and the other one: in Quick Send each of these is said with its
 * `qs-` twin ("qs-pair-old-body"), and the full move keeps its own words.
 */
export const QUICK_WORDS: ReadonlySet<string> = new Set([
  "pair-old-eyebrow",
  "pair-old-body",
  "pair-old-waiting",
  "pair-old-label-hint",
  "pair-old-steps-title",
  "pair-old-step-1",
  "pair-old-left",
  "pair-network-hint",
  "pair-choose-body",
  "pair-choose-renewed",
  "pair-choose-note",
  "pair-heads-up-firewall",
  "pair-new-eyebrow",
  "pair-new-searching-title",
  "pair-new-searching-body",
  "pair-new-search-failed-title",
  "pair-new-list-title",
  "pair-new-list-body",
  "pair-new-same-label",
  "pair-new-none-title",
  "pair-new-none-body",
  "pair-new-code-hint",
  "pair-new-number-title",
  "pair-new-number-body",
  "pair-new-number-waiting",
  "pairing-code-invalid",
  "pairing-code-wrong",
  "pairing-code-expired",
  "pairing-busy",
  "pairing-wait",
  "pairing-locked",
  "pairing-unreachable",
  "pairing-wrong-number",
  "pairing-no-match",
  "pairing-timed-out",
  "pairing-connection-lost-new",
  "pairing-choose-network",
  "pairing-failed-new",
]);

/** The words for `id` on a Quick Send pairing page (`quick`) or a full move's. */
export function pairWord(id: string, quick: boolean): string {
  return quick && QUICK_WORDS.has(id) ? `qs-${id}` : id;
}

/** Whether the pairing page is Quick Send's: its parts say "the other laptop", never old or new. */
export const QuickPairing = createContext(false);

/**
 * Quick Send, the moment the laptops are paired: no Paired page and no Continue. The send (or the
 * receive) starts at once, so `onPaired` is called once, and the page stays calm meanwhile.
 */
export function PairedNow({ onPaired }: { onPaired?: () => void }) {
  const told = useRef(false);
  useEffect(() => {
    if (told.current) return;
    told.current = true;
    onPaired?.();
  }, [onPaired]);
  return <PairStrip step={2} />;
}

/** Back (when there is somewhere to go) and the three connection dots; the active one shows the stage. */
export function PairStrip({ onBack, step = 1 }: { onBack?: () => void; step?: 0 | 1 | 2 }) {
  return (
    <div className="pair-strip">
      {onBack ? (
        <button type="button" className="back" onClick={onBack}>
          <ArrowBack />
          <Localized id="pair-back">
            <span />
          </Localized>
        </button>
      ) : (
        <span />
      )}
      <span className="connection-dots" aria-hidden="true">
        {[0, 1, 2].map((i) => (
          <span key={i} className={i === step ? "active" : undefined} />
        ))}
      </span>
    </div>
  );
}

/**
 * The state's heading. It takes keyboard focus when it appears, so a screen reader follows the
 * person from one state to the next. Render it with a key per state so each one is a fresh heading.
 */
export function Heading({ id, vars }: { id: string; vars?: Record<string, string | number> }) {
  const ref = useRef<HTMLHeadingElement>(null);
  useEffect(() => {
    ref.current?.focus();
  }, []);
  return (
    <Localized id={id} vars={vars}>
      <h1 ref={ref} tabIndex={-1} className="pair-heading" />
    </Localized>
  );
}

export function Eyebrow({ id }: { id: string }) {
  return (
    <Localized id={id}>
      <p className="eyebrow" />
    </Localized>
  );
}

/** What went wrong and what to do next, in words, announced when it appears. */
export function ProblemNotice({ problem }: { problem: Problem }) {
  const { l10n } = useLocalization();
  const quick = useContext(QuickPairing);
  // A key this version has no words for still says something useful: the generic problem.
  const key = l10n.getBundle(problem.key) ? problem.key : "pairing-failed";
  // Other possible causes, when a problem has them, wait behind More details.
  const details = `${key}-details`;
  return (
    <div className="pair-notice">
      <Banner tone="attention">
        <Localized id={pairWord(key, quick)}>
          <p />
        </Localized>
        {l10n.getBundle(details) && (
          <details className="pair-more">
            <summary>{l10n.getString("pair-more-details")}</summary>
            <Localized id={details}>
              <p />
            </Localized>
          </details>
        )}
      </Banner>
    </div>
  );
}

/**
 * Before pairing starts: what the system is about to ask, and the answer that lets the laptops
 * find each other, so its question is never a surprise. Continue starts pairing (and the question).
 */
export function HeadsUpStep({ eyebrow, notes, onContinue }: { eyebrow: string; notes: HeadsUp[]; onContinue: () => void }) {
  const quick = useContext(QuickPairing);
  return (
    <>
      <Eyebrow id={pairWord(eyebrow, quick)} />
      <Heading key="heads-up" id="pair-heads-up-title" />
      {notes.map((n) => (
        <div className="note" key={n.kind}>
          <Info />
          <Localized id={pairWord(HEADS_UP_TEXT[n.kind], quick)}>
            <p className="note-text" />
          </Localized>
        </div>
      ))}
      <div className="pair-section">
        <Button
          variant="primary"
          className="pair-block"
          icon={<ArrowForward />}
          onClick={() => {
            markHeadsUpSeen(notes);
            onContinue();
          }}
        >
          <Localized id="pair-heads-up-continue">
            <span />
          </Localized>
        </Button>
      </div>
    </>
  );
}
