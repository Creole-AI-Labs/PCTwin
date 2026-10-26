import { Localized, useLocalization } from "@fluent/react";
import { useCallback, useEffect, useRef, useState, type SyntheticEvent } from "react";
import { useSettle } from "../motion/screen";
import { Badge, Banner, Button, CodeDisplay, TextField } from "../components/kit";
import { ArrowForward, Check, Info, Laptop, Lock, Shield, Wifi } from "../icons";
import {
  LOST,
  labelText,
  listenOld,
  oldDecline,
  oldPick,
  oldRename,
  oldShuffle,
  oldStart,
  oldNearby,
  oldStartAgain,
  stop,
  toProblem,
  unseenHeadsUp,
  type HeadsUp,
  type Label,
  type OldEvent,
  type Problem,
} from "../pairing";
import Paired from "./Paired";
import { Eyebrow, HeadsUpStep, Heading, PairStrip, PairedNow, ProblemNotice, QuickPairing, pairWord } from "./PairParts";

type Phase = "starting" | "heads-up" | "code" | "wait" | "locked" | "choose" | "network" | "error" | "paired";
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
 * network shows the networks to choose from. Before the first start on a system that will ask a
 * question (a Mac's local network, Windows Firewall), a note says what it will ask (heads-up).
 * Locked: someone on this network tried wrong codes; only the person here can start again.
 * `quick`: Quick Send's sender. The pages say "this laptop" and "the other laptop" (never old or
 * new), say nothing about both laptops choosing the same side, and the send starts the moment the
 * laptops are paired (onPaired at once, with no Paired page in between).
 */
export default function PairOld({ onBack, onPaired, quick = false }: { onBack: () => void; onPaired?: () => void; quick?: boolean }) {
  const { l10n } = useLocalization();
  const w = (id: string) => pairWord(id, quick);
  const [phase, setPhase] = useState<Phase>("starting");
  // Pairing steps arrive without travel; a step showing a code or numbers does not move at all.
  useSettle(phase, { calm: true });
  const phaseNow = useRef<Phase>("starting");
  useEffect(() => {
    phaseNow.current = phase;
  }, [phase]);
  const [label, setLabel] = useState<Label | null>(null);
  const [code, setCode] = useState<{ digits: string; secondsLeft: number } | null>(null);
  const [waitLeft, setWaitLeft] = useState(0);
  const [numbers, setNumbers] = useState<number[]>([]);
  // The number chosen on this round, sent only once the person confirms it (a mis-tap can be changed).
  const [selected, setSelected] = useState<number | null>(null);
  // The round of numbers on screen now: a reply to an earlier round's choice changes nothing.
  const roundNow = useRef(0);
  // Said (and focused) when a fresh round of numbers replaces one the person was looking at.
  const [renewed, setRenewed] = useState(false);
  const checkingNote = useRef<HTMLDivElement>(null);
  const firstNumber = useRef<HTMLButtonElement>(null);
  const [round, setRound] = useState(0);
  const [notice, setNotice] = useState<Notice | null>(null);
  const [problem, setProblem] = useState<Problem | null>(null);
  const [checking, setChecking] = useState(false);
  // A "None of these match" on its way: nothing else on the round may be pressed meanwhile.
  const [declining, setDeclining] = useState(false);
  const [renaming, setRenaming] = useState(false);
  const [name, setName] = useState("");
  const [networks, setNetworks] = useState<string[]>([]);
  const [notes, setNotes] = useState<HeadsUp[]>([]);
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
        setRenewed(roundNow.current !== 0 && phaseNow.current === "choose");
        roundNow.current = event.round;
        setNumbers(event.numbers);
        setSelected(null);
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
    const gone = () => life.cancelled;
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
      const ahead = await unseenHeadsUp("old");
      // The screen may have gone while asking (a check the compiler cannot see across the wait).
      if (gone()) return;
      if (ahead.length > 0) {
        setNotes(ahead);
        setPhase("heads-up");
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

  // Whether a reply to something sent for round `sent` still applies: only while that round is
  // on screen and the laptops are not paired (a newer round, or Paired, has moved the screen on).
  const stillOn = (sent: number) => roundNow.current === sent && !paired.current;

  const pick = async (number: number) => {
    const sent = roundNow.current;
    setChecking(true);
    setProblem(null);
    try {
      // The right number ends in the paired event.
      await oldPick(sent, number);
    } catch (error) {
      if (!stillOn(sent)) return;
      setChecking(false);
      const p = toProblem(error);
      // The connection dropped while checking (or the paired link could not be kept): this
      // pairing cannot go on, so the page with a Try again that starts afresh is shown.
      if (p.key === "pairing-connection-lost") {
        setProblem(LOST);
        setPhase("error");
        return;
      }
      // A choice that came too late: the timed-out notice already says so, once.
      if (p.key !== "pairing-timed-out") setProblem(p);
      setPhase(code ? "code" : "starting");
    }
  };

  const decline = async () => {
    const sent = roundNow.current;
    setProblem(null);
    setDeclining(true);
    try {
      await oldDecline(sent);
    } catch (error) {
      if (stillOn(sent)) setProblem(toProblem(error));
      return;
    } finally {
      setDeclining(false);
    }
    // The round may have ended some other way while the decline was on its way (the time ran
    // out): what that said stands, and "did not match" is not said over it.
    if (!stillOn(sent) || phaseNow.current !== "choose") return;
    setNotice("declined");
    setPhase(code ? "code" : "starting");
  };

  // Both laptops chose Old laptop: neither looks for the other. While the code shows, listen now
  // and then for another old laptop nearby, and say so.
  const [sameRole, setSameRole] = useState(false);
  useEffect(() => {
    if (phase !== "code") return;
    let live = true;
    const look = () => {
      oldNearby().then(
        (found) => {
          if (live) setSameRole(found);
        },
        () => undefined,
      );
    };
    look();
    const every = window.setInterval(look, 15000);
    return () => {
      live = false;
      window.clearInterval(every);
    };
  }, [phase]);

  // Confirm disables itself while checking: keep the person's place on the checking note.
  useEffect(() => {
    if (checking) checkingNote.current?.focus();
  }, [checking]);

  // A fresh round replaced the numbers being looked at: its first number takes the focus.
  useEffect(() => {
    if (renewed) firstNumber.current?.focus();
  }, [renewed, round]);

  const saveName = async (e: SyntheticEvent) => {
    e.preventDefault();
    if (await attempt(() => oldRename(name.trim()))) {
      setRenaming(false);
      setName("");
    }
  };

  if (phase === "paired") return quick ? <PairedNow onPaired={onPaired} /> : <Paired onContinue={onPaired} />;

  return (
    <QuickPairing.Provider value={quick}>
      <PairStrip onBack={back} />

      {phase === "starting" && (
        <>
          <Eyebrow id={w("pair-old-eyebrow")} />
          <Heading key="starting" id={w("pair-old-title")} />
          <Localized id={w("pair-old-starting")}>
            <p className="subhead" />
          </Localized>
        </>
      )}

      {phase === "heads-up" && (
        <HeadsUpStep
          eyebrow="pair-old-eyebrow"
          notes={notes}
          onContinue={() => {
            void start(null);
          }}
        />
      )}

      {phase === "network" && (
        <>
          <Heading key="network" id={w("pair-network-title")} />
          <Localized id={w("pair-network-hint")}>
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
          <Eyebrow id={w("pair-old-eyebrow")} />
          <Heading key="error" id={w("pair-old-title")} />
          {problem && <ProblemNotice problem={problem} />}
          <div className="pair-section">
            <Button
              variant="primary"
              className="pair-block"
              onClick={() => {
                // Whatever was left of the last attempt is let go first, so the start is fresh.
                void stop()
                  .catch(() => undefined)
                  .then(() => start(null));
              }}
            >
              <Localized id={w("pair-try-again")}>
                <span />
              </Localized>
            </Button>
          </div>
        </>
      )}

      {phase === "wait" && (
        <>
          <Heading key="wait" id={w("pair-old-wait-title")} />
          <Localized id={w("pair-old-wait-body")} vars={{ seconds: waitLeft }}>
            <p className="subhead" />
          </Localized>
        </>
      )}

      {phase === "locked" && (
        <>
          <Eyebrow id={w("pair-old-eyebrow")} />
          <Heading key="locked" id={w("pair-old-locked-title")} />
          <Localized id={w("pair-old-locked-body")}>
            <p className="subhead" />
          </Localized>
          <div className="note">
            <Wifi />
            <Localized id={w("pair-old-locked-hotspot")}>
              <p className="note-text" />
            </Localized>
          </div>
          {problem && <ProblemNotice problem={problem} />}
          <div className="pair-section">
            <Button
              variant="primary"
              className="pair-block"
              onClick={() => {
                void attempt(oldStartAgain);
              }}
            >
              <Localized id={w("pair-old-start-again")}>
                <span />
              </Localized>
            </Button>
          </div>
        </>
      )}

      {phase === "code" && (
        <>
          <Eyebrow id={w("pair-old-eyebrow")} />
          <Heading key="code" id={w("pair-old-title")} />
          <Localized id={w("pair-old-body")}>
            <p className="subhead" />
          </Localized>

          <div role="status" className="pair-notice">
            {notice && (
              <Banner tone="info">
                <Localized id={w(NOTICE_TEXT[notice])}>
                  <p />
                </Localized>
              </Banner>
            )}
            {sameRole && !quick && (
              <Banner tone="attention">
                <Localized id={w("pair-old-same-role")}>
                  <p />
                </Localized>
                <Button onClick={back}>
                  <Localized id={w("pair-old-switch")}>
                    <span />
                  </Localized>
                </Button>
              </Banner>
            )}
          </div>
          {problem && <ProblemNotice problem={problem} />}

          {/* The code is the hero: first, and largest. The name card supports it. */}
          {code && (
            <CodeDisplay
              digits={code.digits}
              meta={
                <>
                  <Lock />
                  <Localized id={w("pair-old-code-meta")} vars={{ seconds: code.secondsLeft }}>
                    <span />
                  </Localized>
                </>
              }
            />
          )}

          <div className="pair-network">
            <span className="pair-network-text">
              <Wifi />
              <Localized id={w("pair-old-network")}>
                <span />
              </Localized>
            </span>
            <Badge>
              <Localized id={w("pair-old-waiting")}>
                <span />
              </Localized>
            </Badge>
          </div>

          <div className="note pair-next">
            <Shield />
            <Localized id={w("pair-old-next")}>
              <p className="note-text" />
            </Localized>
          </div>

          {label && (
            <section className="pair-card" aria-labelledby="pair-label-title">
              <Localized id={w("pair-old-label-title")}>
                <h2 id="pair-label-title" className="pair-card-title" />
              </Localized>
              <p className="pair-label-name" dir="auto">
                <Laptop />
                <span>{labelText(l10n, label)}</span>
              </p>
              <Localized id={w("pair-old-label-hint")}>
                <p className="pair-card-hint" />
              </Localized>
              {renaming ? (
                <form className="pair-rename" onSubmit={(e) => void saveName(e)}>
                  <TextField
                    label={
                      <Localized id={w("pair-old-rename-label")}>
                        <span />
                      </Localized>
                    }
                    hint={
                      <Localized id={w("pair-old-rename-note")}>
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
                      <Localized id={w("pair-old-rename-save")}>
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
                      <Localized id={w("pair-old-rename-cancel")}>
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
                    <Localized id={w("pair-old-shuffle")}>
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
                    <Localized id={w("pair-old-rename")}>
                      <span />
                    </Localized>
                  </button>
                </div>
              )}
            </section>
          )}

          <section className="pair-section" aria-labelledby="pair-steps-title">
            <Localized id={w("pair-old-steps-title")}>
              <h2 id="pair-steps-title" className="pair-card-title" />
            </Localized>
            <ol className="pair-steps">
              {[1, 2, 3].map((n) => (
                <Localized key={n} id={w(`pair-old-step-${n.toString()}`)}>
                  <li />
                </Localized>
              ))}
            </ol>
          </section>
        </>
      )}

      {phase === "choose" && (
        <>
          <Eyebrow id={w("pair-choose-eyebrow")} />
          <Heading key="choose" id={w("pair-choose-title")} />
          <Localized id={w("pair-choose-body")}>
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
                  ref={i === 0 ? firstNumber : undefined}
                  aria-pressed={selected === n}
                  disabled={checking || declining}
                  onClick={() => {
                    setSelected(n);
                  }}
                >
                  {/* The name ("Number 47") comes from real text, not a label that differs from what is seen. */}
                  <span className="visually-hidden">{l10n.getString("pair-choose-number", { number: n })}</span>
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
          <Button
            variant="primary"
            className="pair-block"
            icon={<Check />}
            disabled={selected === null || checking || declining}
            onClick={() => {
              if (selected !== null) void pick(selected);
            }}
          >
            <Localized id={w("pair-choose-confirm")}>
              <span />
            </Localized>
          </Button>
          <div role="status" className="pair-checking" ref={checkingNote} tabIndex={-1}>
            {checking && (
              <Localized id={w("pair-choose-checking")}>
                <p />
              </Localized>
            )}
            {!checking && renewed && (
              <Localized id={w("pair-choose-renewed")}>
                <p />
              </Localized>
            )}
          </div>
          <Button
            variant="quiet"
            className="pair-block pair-text-btn"
            disabled={checking || declining}
            onClick={() => {
              void decline();
            }}
          >
            <Localized id={w("pair-choose-none")}>
              <span />
            </Localized>
          </Button>
          <p className="pair-safe">
            <Shield />
            <Localized id={w("pair-choose-safe")}>
              <span />
            </Localized>
          </p>
          <div className="note">
            <Info />
            <Localized id={w("pair-choose-note")}>
              <p className="note-text" />
            </Localized>
          </div>
        </>
      )}
    </QuickPairing.Provider>
  );
}
