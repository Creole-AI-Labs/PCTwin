import { Localized, useLocalization } from "@fluent/react";
import { useCallback, useEffect, useId, useRef, useState, type SyntheticEvent } from "react";
import { Badge, Button, TextField, onlyDigits } from "../components/kit";
import { Attention, Laptop, Shield } from "../icons";
import { FAILED_NEW, LOST_NEW, labelText, listenNew, newConnect, newSearch, stop, toProblem, unseenHeadsUp, type HeadsUp, type NewEvent, type Nearby, type Problem } from "../pairing";
import Paired from "./Paired";
import { Eyebrow, HeadsUpStep, Heading, PairStrip, ProblemNotice } from "./PairParts";

type Phase = "heads-up" | "searching" | "none" | "list" | "code" | "number" | "error" | "paired";

/** Keep what the person typed, up to a sensible length, with digits from any script as 0-9. The engine checks the code. */
function cleanCode(text: string): string {
  let out = "";
  for (const ch of text) {
    if (/\p{Nd}/u.test(ch)) out += onlyDigits(ch);
    else if (ch === "-" || /\s/u.test(ch)) out += ch;
  }
  return out.slice(0, 12);
}

/**
 * C04 and C05 on the new laptop.
 * searching -> none | list | code (exactly one found skips the list) ; list -> code -> number -> paired.
 * A problem after the number returns to the code field with the problem in words. Before the first
 * search on a Mac, a note says what the Mac will ask (heads-up); a search that found nothing because
 * the Mac refuses local discovery says so, with where to switch it on.
 */
export default function PairNew({ onBack, onPaired }: { onBack: () => void; onPaired?: () => void }) {
  const { l10n } = useLocalization();
  const [phase, setPhase] = useState<Phase>("searching");
  const [nearby, setNearby] = useState<Nearby[]>([]);
  const [chosen, setChosen] = useState<Nearby | null>(null);
  const [hadList, setHadList] = useState(false);
  const [code, setCode] = useState("");
  const [busy, setBusy] = useState(false);
  const [number, setNumber] = useState<number | null>(null);
  const [problem, setProblem] = useState<Problem | null>(null);
  // What happened before a search that then failed: still said, first.
  const [earlier, setEarlier] = useState<Problem | null>(null);
  const [notes, setNotes] = useState<HeadsUp[]>([]);
  const fieldId = useId();
  const wasBusy = useRef(false);
  // Paired is the end of pairing: nothing that arrives afterwards moves the screen off it.
  const paired = useRef(false);
  // Counts attempts, so a reply that arrives after its attempt ended changes nothing.
  const attempt = useRef(0);


  // `notice` is shown while looking again (after a lost connection).
  const search = useCallback(async (notice: Problem | null = null) => {
    setProblem(notice);
    setEarlier(null);
    setPhase("searching");
    try {
      const found = await newSearch();
      setNearby(found);
      if (found.length === 0) {
        setPhase("none");
      } else if (found.length === 1 && !found[0].needsUpdate) {
        setChosen(found[0]);
        setHadList(false);
        setCode("");
        setPhase("code");
      } else {
        setHadList(true);
        setPhase("list");
      }
    } catch (error) {
      const p = toProblem(error);
      // Nothing was found, and the Mac says why: no old laptop found, with what to do.
      if (p.key === "pairing-local-network-refused") {
        setNearby([]);
        setProblem(p);
        setPhase("none");
        return;
      }
      // The search itself failed: said as that, after what happened before looking again.
      setEarlier(notice);
      setProblem(p);
      setPhase("error");
    }
  }, []);

  const handle = useCallback((event: NewEvent) => {
    // Once paired, only the paired connection dying before the move moves the screen.
    if (paired.current && event.kind !== "lost") return;
    if (event.kind === "paired") {
      paired.current = true;
      setPhase("paired");
    } else {
      // The attempt is over. The old laptop shows a fresh code, and may have started again
      // somewhere else (it left, or its app closed), so look for it again rather than retry an
      // address it may have left. What happened stays on screen while looking.
      paired.current = false;
      attempt.current += 1;
      setBusy(false);
      setNumber(null);
      setCode("");
      // A dropped connection, or a problem with no words of its own, is said in this laptop's
      // words: it looks again by itself, so there is no Try again to point to.
      const dropped = event.kind === "lost" || event.problem.key === "pairing-connection-lost";
      const generic = event.kind === "problem" && event.problem.key === "pairing-failed";
      void search(dropped ? LOST_NEW : generic ? FAILED_NEW : event.problem);
    }
  }, [search]);

  // Listen first, then search, so the answer to a code is never missed. Leaving the screen stops listening.
  useEffect(() => {
    const life = { cancelled: false };
    const gone = () => life.cancelled;
    let unlisten: (() => void) | undefined;
    void (async () => {
      try {
        const un = await listenNew(handle);
        if (life.cancelled) {
          un();
          return;
        }
        unlisten = un;
      } catch {
        setProblem({ key: "pairing-failed", networks: [] });
        setPhase("error");
        return;
      }
      const ahead = await unseenHeadsUp("new");
      // The screen may have gone while asking (a check the compiler cannot see across the wait).
      if (gone()) return;
      if (ahead.length > 0) {
        setNotes(ahead);
        setPhase("heads-up");
        return;
      }
      await search();
    })();
    return () => {
      life.cancelled = true;
      unlisten?.();
    };
  }, [handle, search]);

  // After a try that did not work, the person is back in the code field.
  useEffect(() => {
    if (wasBusy.current && !busy && phase === "code") document.getElementById(fieldId)?.focus();
    wasBusy.current = busy;
  }, [busy, phase, fieldId]);

  const back = () => {
    void stop().catch(() => undefined);
    onBack();
  };

  const connect = async (e: SyntheticEvent) => {
    e.preventDefault();
    if (!chosen || busy || code.trim() === "") return;
    const mine = ++attempt.current;
    setBusy(true);
    setProblem(null);
    try {
      const shown = await newConnect(chosen.index, code.trim());
      // The attempt may already have ended, or even paired (either can arrive before this
      // reply): then the screen has moved on, and this number is not shown.
      if (attempt.current !== mine || paired.current) return;
      setNumber(shown);
      setBusy(false);
      setPhase("number");
    } catch (error) {
      if (attempt.current !== mine || paired.current) return;
      setBusy(false);
      setProblem(toProblem(error));
    }
  };

  if (phase === "paired") return <Paired onContinue={onPaired} />;

  return (
    <>
      <PairStrip onBack={back} />

      {phase === "heads-up" && (
        <HeadsUpStep
          eyebrow="pair-new-eyebrow"
          notes={notes}
          onContinue={() => {
            void search();
          }}
        />
      )}

      {phase === "searching" && (
        <>
          <Eyebrow id="pair-new-eyebrow" />
          <Heading key="searching" id="pair-new-searching-title" />
          <Localized id="pair-new-searching-body">
            <p className="subhead" />
          </Localized>
          {problem && <ProblemNotice problem={problem} />}
        </>
      )}

      {phase === "error" && (
        <>
          <Eyebrow id="pair-new-eyebrow" />
          <Heading key="error" id="pair-new-search-failed-title" />
          {earlier && <ProblemNotice problem={earlier} />}
          {problem && <ProblemNotice problem={problem} />}
          <div className="pair-section">
            <Button
              variant="primary"
              className="pair-block"
              onClick={() => {
                void search();
              }}
            >
              <Localized id="pair-try-again">
                <span />
              </Localized>
            </Button>
          </div>
        </>
      )}

      {phase === "none" && (
        <>
          <Eyebrow id="pair-new-eyebrow" />
          <Heading key="none" id="pair-new-none-title" />
          <Localized id="pair-new-none-body">
            <p className="subhead" />
          </Localized>
          {/* Both laptops chose New laptop: nobody shows a code, so nothing can be found. */}
          <Localized id="pair-new-none-same-role">
            <p />
          </Localized>
          {problem && <ProblemNotice problem={problem} />}
          <div className="pair-section">
            <Button
              variant="primary"
              className="pair-block"
              onClick={() => {
                void search();
              }}
            >
              <Localized id="pair-new-search-again">
                <span />
              </Localized>
            </Button>
            <Button className="pair-block" onClick={back}>
              <Localized id="pair-new-switch">
                <span />
              </Localized>
            </Button>
          </div>
        </>
      )}

      {phase === "list" && (
        <>
          <Eyebrow id="pair-new-eyebrow" />
          <Heading key="list" id="pair-new-list-title" />
          <Localized id="pair-new-list-body">
            <p className="subhead" />
          </Localized>
          {problem && <ProblemNotice problem={problem} />}
          <ul className="pair-list">
            {nearby.map((n) => (
              <NearbyRow
                key={n.index}
                nearby={n}
                onChoose={() => {
                  setChosen(n);
                  setCode("");
                  setProblem(null);
                  setPhase("code");
                }}
              />
            ))}
          </ul>
          <div className="pair-section">
            <Button
              variant="quiet"
              className="pair-text-btn"
              onClick={() => {
                void search();
              }}
            >
              <Localized id="pair-new-search-again">
                <span />
              </Localized>
            </Button>
          </div>
        </>
      )}

      {phase === "code" && chosen && (
        <>
          <Eyebrow id="pair-new-eyebrow" />
          <Heading key="code" id="pair-new-code-title" />
          <Localized id="pair-new-code-body" vars={{ label: labelText(l10n, chosen.label) }}>
            <p className="subhead" />
          </Localized>
          {problem && <ProblemNotice problem={problem} />}
          <form className="pair-section pair-code-form" onSubmit={(e) => void connect(e)} aria-busy={busy}>
            <TextField
              id={fieldId}
              className="pair-code-input"
              label={
                <Localized id="pair-new-code-label">
                  <span />
                </Localized>
              }
              hint={
                <Localized id="pair-new-code-hint">
                  <span />
                </Localized>
              }
              value={code}
              onChange={(e) => {
                setCode(cleanCode(e.target.value));
              }}
              readOnly={busy}
              dir="ltr"
              inputMode="numeric"
              autoComplete="one-time-code"
              spellCheck={false}
            />
            <Button type="submit" variant="primary" className="pair-block" disabled={busy || code.trim() === ""}>
              <Localized id={busy ? "pair-new-connecting" : "pair-new-connect"}>
                <span />
              </Localized>
            </Button>
          </form>
          {hadList && (
            <div className="pair-section">
              <Button
                variant="quiet"
                className="pair-text-btn"
                disabled={busy}
                onClick={() => {
                  setProblem(null);
                  setPhase("list");
                }}
              >
                <Localized id="pair-new-other-laptop">
                  <span />
                </Localized>
              </Button>
            </div>
          )}
        </>
      )}

      {phase === "number" && number !== null && (
        <>
          <Eyebrow id="pair-new-number-eyebrow" />
          <Heading key="number" id="pair-new-number-title" />
          <Localized id="pair-new-number-body">
            <p className="subhead" />
          </Localized>
          <div className="pair-tile pair-number-tile" dir="ltr" role="group" aria-label={l10n.getString("pair-new-number-label", { number })}>
            <Localized id="pair-new-number-tile">
              <span className="pair-tile-index" aria-hidden="true" />
            </Localized>
            <span className="pair-tile-value" aria-hidden="true">
              {number}
            </span>
          </div>
          <div role="status" className="pair-waiting">
            <Localized id="pair-new-number-waiting">
              <p />
            </Localized>
          </div>
          <p className="pair-safe">
            <Shield />
            <Localized id="pair-choose-safe">
              <span />
            </Localized>
          </p>
        </>
      )}
    </>
  );
}

/** One old laptop in the list. A name shared by two laptops nearby carries a flag in words and an icon. */
function NearbyRow({ nearby, onChoose }: { nearby: Nearby; onChoose: () => void }) {
  const { l10n } = useLocalization();
  const id = useId();
  const nameId = `${id}-name`;
  const flagId = `${id}-flag`;
  return (
    <li>
      <button
        type="button"
        className="pair-pick"
        aria-labelledby={nearby.sameLabelNearby || nearby.needsUpdate ? `${nameId} ${flagId}` : nameId}
        onClick={onChoose}
        disabled={nearby.needsUpdate}
      >
        <span className="pair-pick-icon">
          <Laptop />
        </span>
        <span className="pair-pick-text">
          <span id={nameId} className="pair-pick-name" dir="auto">
            {labelText(l10n, nearby.label)}
          </span>
          {nearby.needsUpdate && (
            <span id={flagId} className="pair-flag">
              <Badge tone="warn" icon={<Attention />}>
                <Localized id="pair-new-needs-update">
                  <span />
                </Localized>
              </Badge>
            </span>
          )}
          {nearby.sameLabelNearby && !nearby.needsUpdate && (
            <span id={flagId} className="pair-flag">
              <Badge tone="warn" icon={<Attention />}>
                <Localized id="pair-new-same-label">
                  <span />
                </Localized>
              </Badge>
            </span>
          )}
        </span>
      </button>
    </li>
  );
}
