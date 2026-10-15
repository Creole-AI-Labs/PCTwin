import { Localized, useLocalization } from "@fluent/react";
import { useCallback, useEffect, useRef, useState, type SyntheticEvent } from "react";
import { Banner, Button, CodeDisplay, TextField } from "../components/kit";
import { ArrowForward, Info, Laptop, Lock, Shield } from "../icons";
import {
  LOST,
  labelText,
  listenOld,
  oldDecline,
  oldPick,
  oldRename,
  oldShuffle,
  oldStart,
  oldStartAgain,
  stop,
  toProblem,
  type Label,
  type OldEvent,
  type Problem,
} from "../pairing";
import Paired from "./Paired";
import { Eyebrow, Heading, PairStrip, ProblemNotice } from "./PairParts";

type Phase = "starting" | "code" | "wait" | "locked" | "choose" | "network" | "error" | "paired";
type Notice = "timed-out" | "declined" | "left";

const NOTICE_TEXT: Record<Notice, string> = {
  "timed-out": "pair-old-timed-out",
  declined: "pair-old-declined",
  left: "pair-old-left",
};

/**
 * C03 and C05 on the old laptop. The engine drives it: a label, then a code that changes, then (when a
 * new laptop has the code) three numbers to choose from, then paired.
 * starting -> code | wait | locked -> choose -> (code again | paired); a start that cannot pick the
 * network shows the networks to choose from.
 */
export default function PairOld({ onBack }: { onBack: () => void }) {
  const { l10n } = useLocalization();
  const [phase, setPhase] = useState<Phase>("starting");
  const [label, setLabel] = useState<Label | null>(null);
  const [code, setCode] = useState<{ digits: string; secondsLeft: number } | null>(null);
  const [waitLeft, setWaitLeft] = useState(0);
  const [numbers, setNumbers] = useState<number[]>([]);
  const [round, setRound] = useState(0);
  const [notice, setNotice] = useState<Notice | null>(null);
  const [problem, setProblem] = useState<Problem | null>(null);
  const [checking, setChecking] = useState(false);
  const [renaming, setRenaming] = useState(false);
  const [name, setName] = useState("");
  const [networks, setNetworks] = useState<string[]>([]);
  const renameButton = useRef<HTMLButtonElement>(null);
  const wasRenaming = useRef(false);
  // Paired is the end of pairing: nothing that arrives afterwards moves the screen off it, except
  // the paired connection dying before the move began.
  const paired = useRef(false);

  const handle = useCallback((event: OldEvent) => {
    if (paired.current && event.kind !== "lost") return;
    switch (event.kind) {
      case "label":
        setLabel(event.label);
        break;
      case "code":
        setCode({ digits: event.code, secondsLeft: event.secondsLeft });
        // The code keeps counting down behind the number choice; it must not pull the person out of it.
        setPhase((p) => (p === "choose" ? p : "code"));
        break;
      case "wait":
        setWaitLeft(event.secondsLeft);
        setPhase("wait");
        break;
      case "locked":
        setPhase("locked");
        break;
      case "choose":
        setNumbers(event.numbers);
        setRound(event.round);
        setChecking(false);
        setNotice(null);
        setProblem(null);
        setPhase("choose");
        break;
      case "paired":
        paired.current = true;
        setPhase("paired");
        break;
      case "timed-out":
        setChecking(false);
        setNotice("timed-out");
        setPhase("code");
        break;
      case "left":
        setChecking(false);
        setNotice("left");
        setPhase("code");
        break;
      case "lost":
        paired.current = false;
        setChecking(false);
        setNotice(null);
        setProblem(LOST);
        setPhase("error");
        break;
    }
  }, []);

  const start = useCallback(async (network: string | null) => {
    setProblem(null);
    setNotice(null);
    setPhase("starting");
    try {
      await oldStart(network);
    } catch (error) {
      const p = toProblem(error);
      if (p.key === "pairing-choose-network") {
        setNetworks(p.networks);
        setPhase("network");
      } else {
        setProblem(p);
        setPhase("error");
      }
    }
  }, []);

  // Listen first, then start, so no early update is missed. Leaving the screen stops listening.
  useEffect(() => {
    const life = { cancelled: false };
    let unlisten: (() => void) | undefined;
    void (async () => {
      try {
        const un = await listenOld(handle);
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
      await start(null);
    })();
    return () => {
      life.cancelled = true;
      unlisten?.();
    };
  }, [handle, start]);

  // Closing the name field returns focus to the button that opened it.
  useEffect(() => {
    if (wasRenaming.current && !renaming) renameButton.current?.focus();
    wasRenaming.current = renaming;
  }, [renaming]);

  const back = () => {
    void stop().catch(() => undefined);
    onBack();
  };

  const attempt = async (action: () => Promise<unknown>): Promise<boolean> => {
    setProblem(null);
    try {
      await action();
      return true;
    } catch (error) {
      setProblem(toProblem(error));
      return false;
    }
  };

  const pick = async (number: number) => {
    setChecking(true);
    // The right number ends in the paired event. A rejection means this try is over.
    // Once paired, a late reply to this pick changes nothing.
    if (!(await attempt(() => oldPick(round, number))) && !paired.current) {
      setChecking(false);
      setPhase(code ? "code" : "starting");
    }
  };

  const decline = async () => {
    if (await attempt(() => oldDecline(round))) {
      setNotice("declined");
      setPhase(code ? "code" : "starting");
    }
  };

  const saveName = async (e: SyntheticEvent) => {
    e.preventDefault();
    if (await attempt(() => oldRename(name.trim()))) {
      setRenaming(false);
      setName("");
    }
  };

  if (phase === "paired") return <Paired />;

  return (
    <>
      <PairStrip onBack={back} />

      {phase === "starting" && (
        <>
          <Eyebrow id="pair-old-eyebrow" />
          <Heading key="starting" id="pair-old-title" />
          <Localized id="pair-old-starting">
            <p className="subhead" />
          </Localized>
        </>
      )}

      {phase === "network" && (
        <>
          <Heading key="network" id="pair-network-title" />
          <Localized id="pair-network-hint">
            <p className="subhead" />
          </Localized>
          <ul className="pair-list">
            {networks.map((n) => (
              <li key={n}>
                <button
                  type="button"
                  className="pair-pick"
                  onClick={() => {
                    void start(n);
                  }}
                >
                  <span className="pair-pick-name" dir="auto">
                    {n}
                  </span>
                  <ArrowForward />
                </button>
              </li>
            ))}
          </ul>
        </>
      )}

      {phase === "error" && (
        <>
          <Eyebrow id="pair-old-eyebrow" />
          <Heading key="error" id="pair-old-title" />
          {problem && <ProblemNotice problem={problem} />}
          <div className="pair-section">
            <Button
              variant="primary"
              className="pair-block"
              onClick={() => {
                void start(null);
              }}
            >
              <Localized id="pair-try-again">
                <span />
              </Localized>
            </Button>
          </div>
        </>
      )}

      {phase === "wait" && (
        <>
          <Heading key="wait" id="pair-old-wait-title" />
          <Localized id="pair-old-wait-body" vars={{ seconds: waitLeft }}>
            <p className="subhead" />
          </Localized>
        </>
      )}

      {phase === "locked" && (
        <>
          <Heading key="locked" id="pair-old-locked-title" />
          <Localized id="pair-old-locked-body">
            <p className="subhead" />
          </Localized>
          {problem && <ProblemNotice problem={problem} />}
          <div className="pair-section">
            <Button
              variant="primary"
              className="pair-block"
              onClick={() => {
                void attempt(oldStartAgain);
              }}
            >
              <Localized id="pair-old-start-again">
                <span />
              </Localized>
            </Button>
          </div>
        </>
      )}

      {phase === "code" && (
        <>
          <Eyebrow id="pair-old-eyebrow" />
          <Heading key="code" id="pair-old-title" />
          <Localized id="pair-old-body">
            <p className="subhead" />
          </Localized>

          <div role="status" className="pair-notice">
            {notice && (
              <Banner tone="info">
                <Localized id={NOTICE_TEXT[notice]}>
                  <p />
                </Localized>
              </Banner>
            )}
          </div>
          {problem && <ProblemNotice problem={problem} />}

          {/* The code is the hero: first, and largest. The name card supports it. */}
          {code && (
            <CodeDisplay
              digits={code.digits}
              group={3}
              meta={
                <>
                  <Lock />
                  <Localized id="pair-old-code-meta" vars={{ seconds: code.secondsLeft }}>
                    <span />
                  </Localized>
                </>
              }
            />
          )}

          <div className="note pair-next">
            <Shield />
            <Localized id="pair-old-next">
              <p className="note-text" />
            </Localized>
          </div>

          {label && (
            <section className="pair-card" aria-labelledby="pair-label-title">
              <Localized id="pair-old-label-title">
                <h2 id="pair-label-title" className="pair-card-title" />
              </Localized>
              <p className="pair-label-name" dir="auto">
                <Laptop />
                <span>{labelText(l10n, label)}</span>
              </p>
              <Localized id="pair-old-label-hint">
                <p className="pair-card-hint" />
              </Localized>
              {renaming ? (
                <form className="pair-rename" onSubmit={(e) => void saveName(e)}>
                  <TextField
                    label={
                      <Localized id="pair-old-rename-label">
                        <span />
                      </Localized>
                    }
                    hint={
                      <Localized id="pair-old-rename-note">
                        <span />
                      </Localized>
                    }
                    value={name}
                    onChange={(e) => {
                      setName(e.target.value);
                    }}
                    autoComplete="off"
                    spellCheck={false}
                    autoFocus
                  />
                  <div className="pair-actions">
                    <Button type="submit" variant="primary">
                      <Localized id="pair-old-rename-save">
                        <span />
                      </Localized>
                    </Button>
                    <Button
                      onClick={() => {
                        setRenaming(false);
                        setName("");
                        setProblem(null);
                      }}
                    >
                      <Localized id="pair-old-rename-cancel">
                        <span />
                      </Localized>
                    </Button>
                  </div>
                </form>
              ) : (
                <div className="pair-actions">
                  <Button
                    onClick={() => {
                      void attempt(oldShuffle);
                    }}
                  >
                    <Localized id="pair-old-shuffle">
                      <span />
                    </Localized>
                  </Button>
                  <button
                    ref={renameButton}
                    type="button"
                    className="kit-button kit-button-quiet"
                    onClick={() => {
                      setProblem(null);
                      setRenaming(true);
                    }}
                  >
                    <Localized id="pair-old-rename">
                      <span />
                    </Localized>
                  </button>
                </div>
              )}
            </section>
          )}

          <section className="pair-section" aria-labelledby="pair-steps-title">
            <Localized id="pair-old-steps-title">
              <h2 id="pair-steps-title" className="pair-card-title" />
            </Localized>
            <ol className="pair-steps">
              {[1, 2, 3].map((n) => (
                <Localized key={n} id={`pair-old-step-${n.toString()}`}>
                  <li />
                </Localized>
              ))}
            </ol>
          </section>
        </>
      )}

      {phase === "choose" && (
        <>
          <Eyebrow id="pair-choose-eyebrow" />
          <Heading key="choose" id="pair-choose-title" />
          <Localized id="pair-choose-body">
            <p className="subhead" />
          </Localized>
          {problem && <ProblemNotice problem={problem} />}
          <ul className="pair-numbers">
            {numbers.map((n, i) => (
              <li key={n}>
                <button
                  type="button"
                  className="pair-tile pair-number"
                  dir="ltr"
                  disabled={checking}
                  aria-label={l10n.getString("pair-choose-number", { number: n })}
                  onClick={() => {
                    void pick(n);
                  }}
                >
                  <span className="pair-tile-index" aria-hidden="true">
                    {String(i + 1).padStart(2, "0")}
                  </span>
                  <span className="pair-tile-value" aria-hidden="true">
                    {n}
                  </span>
                </button>
              </li>
            ))}
          </ul>
          <div role="status" className="pair-checking">
            {checking && (
              <Localized id="pair-choose-checking">
                <p />
              </Localized>
            )}
          </div>
          <Button
            variant="quiet"
            className="pair-block pair-text-btn"
            disabled={checking}
            onClick={() => {
              void decline();
            }}
          >
            <Localized id="pair-choose-none">
              <span />
            </Localized>
          </Button>
          <p className="pair-safe">
            <Shield />
            <Localized id="pair-choose-safe">
              <span />
            </Localized>
          </p>
          <div className="note">
            <Info />
            <Localized id="pair-choose-note">
              <p className="note-text" />
            </Localized>
          </div>
        </>
      )}
    </>
  );
}
