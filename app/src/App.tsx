import { LocalizationProvider, Localized } from "@fluent/react";
import { useCallback, useEffect, useLayoutEffect, useMemo, useRef, useState } from "react";
import lockup from "./assets/pctwin-lockup.png";
import { directionOf, localization, pickLanguage, saveLanguage, savedLanguage, type LanguageCode } from "./i18n";
import { laptopName, laptopNameShuffle, laptopNameType, laptopNameUse, listenKnown, qsLaunchFiles, recoveryCheck, recoveryRun, thingsReady, toMoveProblem, toProblem, updateCheck, updateInstall, type KnownDelivered, type KnownIncoming, type Label, type Problem, type QsNote, type ReadyNote, type RescueDrive } from "./bridge";
import { Banner, Button, Toast } from "./components/kit";
import KnownHost from "./screens/QsKnownHost";
import KnownReceiveHost from "./screens/QsKnownIncoming";
import { NoteInHost } from "./screens/QsNote";
import WaitingHost from "./screens/QsWaiting";
import ThingsReady from "./screens/ThingsReady";
import { setFileWords, setTrayLabels, type WindowMode } from "./native";
import { stop as stopPairing } from "./pairing";
import ComingSoon, { type Soon } from "./screens/ComingSoon";
import ConnectionRoute from "./screens/ConnectionRoute";
import HelpHome from "./screens/HelpHome";
import SafeStop from "./screens/SafeStop";
import MoveHost from "./screens/MoveHost";
import RescuePick from "./screens/RescuePick";
import PickUp from "./screens/PickUp";
import { FooterProvider } from "./screens/FrameFooter";
import PairNew from "./screens/PairNew";
import PairOld from "./screens/PairOld";
import PutTogether from "./screens/PutTogether";
import QuickHome from "./screens/QuickHome";
import QuickHost from "./screens/QuickHost";
import HistoryHost from "./screens/HistoryHost";
import NameLaptop from "./screens/NameLaptop";
import Welcome, { type UpdateState } from "./screens/Welcome";
import WhichLaptop, { type Role } from "./screens/WhichLaptop";
import { HeaderLanguage, HeaderMotion } from "./components/HeaderSettings";
import Opening from "./motion/Opening";
import { openingKind, type OpeningKind } from "./motion/openingRule";
import { applyToDocument } from "./motion/preference";
import { restoreReveal } from "./motion/play";
import { installScreenMotion, settleScreen } from "./motion/screen";
import "./styles.css";

/** Welcome, Which laptop is this?, How will you connect? (C01), Put both laptops together (C02), pairing (C03 to C05), then the move; and Quick Send's home. */
type Screen =
  | { id: "welcome" }
  | { id: "role" }
  | { id: "connect"; role: Role }
  | { id: "together"; role: Role }
  | { id: "pair"; role: Role }
  // The move, from paired to its outcome. Left only by its own buttons, so a running move always has a screen.
  | { id: "move"; role: Role; rescue?: RescueDrive }
  // Rescue a laptop (2.6), from Home: choose a drive connected here; the move follows as the new laptop.
  | { id: "rescue" }
  // Another person's things were handed over at their sign-in (M32): shown to them once, at start.
  | { id: "ready"; note: ReadyNote }
  // A move on this laptop stopped part way (X09): shown at start, before anything else.
  | { id: "pickup"; unfinished: number }
  // Quick Send, either way, or a note or link, from its home; `sends`: the list of sends going on.
  // `session`: Add files to this open send (Send to <name>, from a remembered laptop's page).
  | { id: "qs"; mode: "send" | "receive" | "note" | "sends"; session?: string }
  // Remembered connections and waiting sends (Quick Send stage 1), from Quick Send's home.
  | { id: "known" }
  | { id: "waiting" }
  // A remembered laptop wants to send files: where should they go? (Shown whenever it asks, but never over a move, a pairing or a send.)
  | { id: "known-in"; incoming: KnownIncoming }
  // A note or link from a remembered laptop.
  | { id: "note-in"; note: QsNote }
  // My PCTwin history (F08), from Home; Back returns there.
  | { id: "history" }
  // What an update is waiting for (safe stop), from the update offer on Home.
  | { id: "safe-stop" }
  // Help (S01), from the footer; Back returns to where it was opened.
  | { id: "help"; from: Screen }
  | { id: "soon"; which: Soon }
  // Quick Send home. `notice` is set when the person chose something the engine cannot do yet.
  | { id: "quick"; notice: boolean }
  // Name this laptop: the first time PCTwin opens, before Home, until a name is chosen.
  | { id: "name"; suggested: Label };

/** A remembered laptop's question that came while the person was busy: its files, or a note. */
type Asking = { kind: "incoming"; incoming: KnownIncoming } | { kind: "note"; note: QsNote };

/** The page for a question that waited. */
const askingScreen = (a: Asking): Screen => (a.kind === "incoming" ? { id: "known-in", incoming: a.incoming } : { id: "note-in", note: a.note });

/** The app frame: header with the language choice, one screen at a time, and the free-move footer. */
export default function App({ initialLanguage }: { initialLanguage?: LanguageCode }) {
  const [language, setLanguage] = useState<LanguageCode>(() => initialLanguage ?? savedLanguage() ?? pickLanguage(navigator.languages));
  const [screen, setScreen] = useState<Screen>({ id: "welcome" });
  // The screen now, for the engine's Quick Send events (the listener is set once).
  const screenRef = useRef<Screen>(screen);
  useEffect(() => {
    screenRef.current = screen;
  }, [screen]);
  // Files that landed under "allow future sends into this folder", told on the first after a quiet spell.
  const [delivered, setDelivered] = useState<KnownDelivered | null>(null);
  // A remembered laptop asked to send files (or sent a note) while a move, a pairing or a send was
  // on screen: said in a line at the top, and asked as soon as that page is left. Never dropped.
  const [asking, setAsking] = useState<Asking | null>(null);
  // Quick Send's page can be left without loss (its Done page, the list of sends): the question may be shown now.
  const [qsIdleShown, setQsIdleShown] = useState(false);
  const onQsIdle = useCallback((idle: boolean) => {
    setQsIdleShown(idle);
  }, []);
  // A move that stopped part way on this laptop: Home shows "Resume your move" until it is tidied up.
  const [unfinished, setUnfinished] = useState(0);
  // What went wrong on the naming page (a name that cannot be used), or null.
  const [nameProblem, setNameProblem] = useState<Problem | null>(null);
  // A newer build of PCTwin, found when the app opened: Home offers it.
  const [update, setUpdate] = useState<UpdateState | undefined>(undefined);
  // The mini panel (360 x 240) is the window: no header above it.
  const [mini, setMini] = useState(false);
  const onWindow = useCallback((mode: WindowMode) => {
    setMini(mode === "mini");
  }, []);
  const l10n = useMemo(() => localization(language), [language]);
  const main = useRef<HTMLElement>(null);
  const frame = useRef<HTMLDivElement>(null);
  // The Wordplay opening on the first launch and after an update; a short restoration otherwise.
  // It is drawn over the app, which is built underneath from the first frame.
  const [opening, setOpening] = useState<OpeningKind>(() => openingKind());

  // At start: a move that stopped part way on this laptop puts "Resume your move" on Home, which
  // opens the pick-up (X09). Asked once; a laptop with nothing to pick up (or outside the app) shows Home as usual.
  useEffect(() => {
    let live = true;
    recoveryCheck().then(
      ({ unfinished }) => {
        if (live && unfinished > 0) setUnfinished(unfinished);
      },
      () => undefined,
    );
    // A laptop with no name chosen yet opens on Name this laptop, before Home.
    laptopName().then(
      (names) => {
        if (live && names.kept === null) setScreen((s) => (s.id === "welcome" ? { id: "name", suggested: names.suggested } : s));
      },
      () => undefined,
    );
    // Asked once, when PCTwin opens: a newer build is offered on Home; no answer (no network,
    // say) is simply no offer.
    updateCheck().then(
      (found) => {
        if (live && found) setUpdate({ version: found.version, state: "ready" });
      },
      () => undefined,
    );
    thingsReady().then(
      (note) => {
        if (live && !("none" in note)) setScreen((s) => (s.id === "welcome" ? { id: "ready", note } : s));
      },
      () => undefined,
    );
    return () => {
      live = false;
    };
  }, []);

  // Quick Send from outside its pages: a remembered laptop asking to send, a note from one, and
  // files the system handed PCTwin (Send to, a file manager entry). None of them takes the person
  // away from a move, a pairing, a send or another question: the engine then declines or waits.
  useEffect(() => {
    let live = true;
    let unlisten: (() => void) | null = null;
    const busy = () => ["move", "pair", "qs", "known-in", "name", "pickup"].includes(screenRef.current.id);
    void listenKnown((event) => {
      if (!live) return;
      // Asked now when the person is free; otherwise kept and said at the top until they are.
      if (event.kind === "incoming") {
        const a: Asking = { kind: "incoming", incoming: event.incoming };
        if (busy()) setAsking(a);
        else setScreen(askingScreen(a));
      } else if (event.kind === "note" && event.note.from !== null) {
        const a: Asking = { kind: "note", note: event.note };
        if (busy()) setAsking((old) => old ?? a);
        else setScreen(askingScreen(a));
      } else if (event.kind === "launch" && !busy()) setScreen({ id: "qs", mode: "send" });
      else if (event.kind === "delivered") {
        const d = event.delivered;
        const whole = d.lines.every((l) => l.outcome.kind === "arrived" || l.outcome.kind === "already-there");
        // Said on the first after a quiet spell, and whenever something did not arrive.
        if (d.tell || !whole) setDelivered(d);
      }
    }).then((stop) => {
      if (live) unlisten = stop;
      else stop();
    });
    // PCTwin was started to send something (Send to): the pick page opens with it.
    qsLaunchFiles().then(
      (items) => {
        if (live && items.length > 0) setScreen((s) => (s.id === "welcome" ? { id: "qs", mode: "send" } : s));
      },
      () => undefined,
    );
    return () => {
      live = false;
      unlisten?.();
    };
  }, []);

  // A question that waited is asked as soon as the person leaves the busy page.
  useEffect(() => {
    if (!asking) return;
    if (["move", "pair", "qs", "known-in", "name", "pickup"].includes(screen.id)) return;
    // The question is the engine's, waiting outside React: shown the moment the busy page is left.
    // eslint-disable-next-line react-hooks/set-state-in-effect
    setScreen(askingScreen(asking));
    setAsking(null);
  }, [asking, screen.id]);

  // The motion setting and its timings, on the page before anything moves.
  useEffect(() => {
    applyToDocument();
    installScreenMotion();
  }, []);

  // Screen to screen: the new screen settles in (Back reverses the direction, right to left mirrors
  // it). Pairing arrives without travel: codes stay still. The first screen is the opening's.
  const arrived = useRef(false);
  useLayoutEffect(() => {
    if (!arrived.current) {
      arrived.current = true;
      return;
    }
    settleScreen(main.current, { calm: screen.id === "pair" });
  }, [screen]);

  useEffect(() => {
    if (opening === "restore") restoreReveal(frame.current);
    // Once, at start.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  useEffect(() => {
    document.documentElement.lang = language;
    document.documentElement.dir = directionOf(language);
    void setTrayLabels(l10n.getString("tray-show"), l10n.getString("tray-quit")).catch(() => undefined);
    void setFileWords({
      bookmarksFile: l10n.getString("browser-file-bookmarks"),
      listsFile: l10n.getString("browser-file-lists"),
      bookmarksTitle: l10n.getString("browser-file-bookmarks-title"),
      addOns: l10n.getString("browser-file-add-ons"),
      openPages: l10n.getString("browser-file-open-pages"),
    }).catch(() => undefined);
  }, [language, l10n]);

  // A new screen starts at the top and moves keyboard focus to its heading area.
  useEffect(() => {
    if (!main.current) return;
    main.current.scrollTop = 0;
    // A screen that already moved focus to its own heading keeps it; while the opening is up, focus is on Skip intro.
    if (main.current.closest("[inert]")) return;
    if (!main.current.contains(document.activeElement)) main.current.focus();
  }, [screen]);

  // Leaving pairing, a move or a send for the help page would end them, so there Help opens the website.
  const onHelp =
    screen.id === "pair" || screen.id === "move" || screen.id === "qs"
      ? undefined
      : () => {
          setScreen((s) => (s.id === "help" ? s : { id: "help", from: s }));
        };
  const startMove = () => {
    // The full move: files, people, apps, settings and app data (switched on for everyone, 28 October 2026).
    setScreen({ id: "role" });
  };

  const choose = (code: LanguageCode) => {
    setLanguage(code);
    saveLanguage(code);
  };

  return (
    <LocalizationProvider l10n={l10n}>
      <div className="app" ref={frame}>
        <FooterProvider onHelp={onHelp}>
          <header className="app-header" hidden={mini && screen.id === "move"}>
            <Localized id="brand-home" attrs={{ "aria-label": true }}>
              <button
                type="button"
                className="brand"
                aria-label="PCTwin home"
                data-nav="back"
                // During a move, home is reached by the move's own buttons only.
                aria-disabled={screen.id === "move" ? true : undefined}
                onClick={() => {
                  if (screen.id === "move") return;
                  if (screen.id === "pair" || screen.id === "qs") void stopPairing().catch(() => undefined);
                  setScreen({ id: "welcome" });
                }}
              >
                <span className="brand-mark">
                  <img src={lockup} alt="" />
                </span>
                <Localized id="app-name">
                  <span>PCTwin</span>
                </Localized>
              </button>
            </Localized>
            <div className="app-settings">
              <Localized id="header-this-laptop">
                <span className="header-badge">This laptop</span>
              </Localized>
              <HeaderLanguage language={language} onLanguage={choose} />
              {/* Help is in the footer of every page, so the header keeps only the settings. */}
              <HeaderMotion />
            </div>
          </header>
          {asking && (
            <div className="app-asking" role="status">
              <Banner tone="info">
                {asking.kind === "incoming"
                  ? l10n.getString("qs-asking-files", { label: asking.incoming.label, count: asking.incoming.files.length })
                  : l10n.getString("qs-asking-note", { label: asking.note.from ?? "" })}{" "}
                {l10n.getString("qs-asking-later")}
              </Banner>
              {/* Quick Send's Done page and list of sends lose nothing when left: the question can be answered now. */}
              {screen.id === "qs" && qsIdleShown && (
                <Button
                  onClick={() => {
                    setScreen(askingScreen(asking));
                    setAsking(null);
                  }}
                >
                  {l10n.getString(asking.kind === "incoming" ? "qs-asking-show" : "qs-asking-show-note")}
                </Button>
              )}
            </div>
          )}
          <main className="app-main" ref={main} tabIndex={-1} key={screen.id}>
            {screen.id === "welcome" && (
              <Welcome
                onStart={startMove}
                onSoon={(which) => { setScreen(which === "rescue" ? { id: "rescue" } : { id: "soon", which }); }}
                onQuick={() => { setScreen({ id: "quick", notice: false }); }}
                onHistory={() => { setScreen({ id: "history" }); }}
                unfinished={unfinished > 0}
                onResume={() => { setScreen({ id: "pickup", unfinished }); }}
                update={update}
                onSafeStop={() => { setScreen({ id: "safe-stop" }); }}
                onUpdate={() => {
                  if (!update) return;
                  setUpdate({ ...update, state: "installing" });
                  // On success PCTwin closes and opens again as the new version.
                  updateInstall().catch((error: unknown) => {
                    setUpdate({ ...update, state: toMoveProblem(error).kind === "busy" ? "busy" : "failed" });
                  });
                }}
                // Waiting sends (Q18) are not built in the engine yet, so Home has none to show.
              />
            )}
            {screen.id === "quick" && (
              <QuickHome
                notice={screen.notice}
                onSend={() => { setScreen({ id: "qs", mode: "send" }); }}
                onReceive={() => { setScreen({ id: "qs", mode: "receive" }); }}
                onNote={() => { setScreen({ id: "qs", mode: "note" }); }}
                onKnown={() => { setScreen({ id: "known" }); }}
                onWaiting={() => { setScreen({ id: "waiting" }); }}
                onBack={() => { setScreen({ id: "welcome" }); }}
              />
            )}
            {screen.id === "known" && <KnownHost onSend={() => { setScreen({ id: "qs", mode: "send" }); }} onBack={() => { setScreen({ id: "quick", notice: false }); }} />}
            {screen.id === "waiting" && <WaitingHost onSend={() => { setScreen({ id: "qs", mode: "send" }); }} onBack={() => { setScreen({ id: "quick", notice: false }); }} />}
            {screen.id === "known-in" && (
              <KnownReceiveHost
                incoming={screen.incoming}
                onHome={() => { setScreen({ id: "quick", notice: false }); }}
                onSendTo={(session) => { setScreen({ id: "qs", mode: "send", session }); }}
              />
            )}
            {screen.id === "note-in" && <NoteInHost note={screen.note} onDone={() => { setScreen({ id: "quick", notice: false }); }} />}
            {screen.id === "name" && (
              <NameLaptop
                suggested={screen.suggested}
                problem={nameProblem}
                onUse={() => {
                  setNameProblem(null);
                  laptopNameUse().then(
                    () => { setScreen({ id: "welcome" }); },
                    (error: unknown) => { setNameProblem(toProblem(error)); },
                  );
                }}
                onShuffle={() => {
                  setNameProblem(null);
                  laptopNameShuffle().then(
                    (suggested) => { setScreen({ id: "name", suggested }); },
                    (error: unknown) => { setNameProblem(toProblem(error)); },
                  );
                }}
                onType={(name) => {
                  setNameProblem(null);
                  return laptopNameType(name).then(
                    () => { setScreen({ id: "welcome" }); return true; },
                    (error: unknown) => { setNameProblem(toProblem(error)); return false; },
                  );
                }}
              />
            )}
            {screen.id === "safe-stop" && <SafeStop onBack={() => { setScreen({ id: "welcome" }); }} />}
            {screen.id === "history" && <HistoryHost onStart={startMove} onPair={() => { setScreen({ id: "pair", role: "new" }); }} onBack={() => { setScreen({ id: "welcome" }); }} />}
            {screen.id === "qs" && (
              <QuickHost
                key={`${screen.mode}-${screen.session ?? ""}`}
                mode={screen.mode}
                session={screen.session}
                onWindow={onWindow}
                onIdle={onQsIdle}
                onWaiting={() => { setScreen({ id: "waiting" }); }}
                onHome={() => { setScreen({ id: "quick", notice: false }); }}
              />
            )}
            {screen.id === "rescue" && <RescuePick onBack={() => { setScreen({ id: "welcome" }); }} onChoose={(drive) => { setScreen({ id: "move", role: "new", rescue: drive }); }} />}
            {screen.id === "soon" && <ComingSoon which={screen.which} onBack={() => { setScreen({ id: "welcome" }); }} />}
            {screen.id === "role" && <WhichLaptop onBack={() => { setScreen({ id: "welcome" }); }} onChoose={(role) => { setScreen({ id: "connect", role }); }} />}
            {screen.id === "connect" && <ConnectionRoute onBack={() => { setScreen({ id: "role" }); }} onWifi={() => { setScreen({ id: "together", role: screen.role }); }} />}
            {screen.id === "together" && <PutTogether onBack={() => { setScreen({ id: "connect", role: screen.role }); }} onContinue={() => { setScreen({ id: "pair", role: screen.role }); }} />}
            {screen.id === "pair" && screen.role === "old" && <PairOld onBack={() => { setScreen({ id: "together", role: screen.role }); }} onPaired={() => { setScreen({ id: "move", role: "old" }); }} />}
            {screen.id === "pair" && screen.role === "new" && <PairNew onBack={() => { setScreen({ id: "together", role: screen.role }); }} onPaired={() => { setScreen({ id: "move", role: "new" }); }} />}
            {screen.id === "help" && <HelpHome onBack={() => { setScreen(screen.from); }} />}
            {screen.id === "ready" && <ThingsReady note={screen.note} onDone={() => { setScreen({ id: "welcome" }); }} />}
            {screen.id === "pickup" && (
              <PickUp
                unfinished={screen.unfinished}
                onTidy={() =>
                  recoveryRun().then((tidied) => {
                    // Tidied up: nothing is left to resume.
                    setUnfinished(0);
                    return tidied;
                  })
                }
                onPairAgain={() => { setScreen({ id: "pair", role: "new" }); }}
                onLater={() => { setScreen({ id: "welcome" }); }}
              />
            )}
            {screen.id === "move" && (
              <MoveHost
                old={screen.role === "old"}
                rescue={screen.rescue}
                onWindow={onWindow}
                onPairAgain={() => { setScreen({ id: "pair", role: screen.role }); }}
                onHome={() => { setScreen({ id: "welcome" }); }}
              />
            )}
          </main>
          {delivered && (
            <Toast onDismiss={() => { setDelivered(null); }}>
              {l10n.getString("qs-known-delivered", { label: delivered.label, folder: delivered.folder })}{" "}
              {l10n.getString("qs-known-delivered-counts", {
                arrived: delivered.lines.filter((l) => l.outcome.kind === "arrived" || l.outcome.kind === "already-there").length,
                total: delivered.lines.length,
              })}
            </Toast>
          )}
        </FooterProvider>
      </div>
      {(opening === "wordplay" || opening === "still") && (
        <Opening
          kind={opening}
          app={frame}
          // Something that needs the person (a move to resume, things handed over) ends it at once.
          interrupt={screen.id !== "welcome" || unfinished > 0}
          onDone={() => {
            setOpening("none");
            main.current?.focus({ preventScroll: true });
          }}
        />
      )}
    </LocalizationProvider>
  );
}
