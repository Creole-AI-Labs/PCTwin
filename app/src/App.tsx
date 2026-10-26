import { LocalizationProvider, Localized } from "@fluent/react";
import { useCallback, useEffect, useLayoutEffect, useMemo, useRef, useState } from "react";
import lockup from "./assets/pctwin-lockup.png";
import { directionOf, localization, pickLanguage, saveLanguage, savedLanguage, type LanguageCode } from "./i18n";
import { recoveryCheck, recoveryRun, thingsReady, toMoveProblem, updateCheck, updateInstall, type ReadyNote } from "./bridge";
import ThingsReady from "./screens/ThingsReady";
import { setTrayLabels, type WindowMode } from "./native";
import { stop as stopPairing } from "./pairing";
import ComingSoon, { type Soon } from "./screens/ComingSoon";
import ConnectionRoute from "./screens/ConnectionRoute";
import HelpHome from "./screens/HelpHome";
import MoveHost from "./screens/MoveHost";
import PickUp from "./screens/PickUp";
import { FooterProvider } from "./screens/FrameFooter";
import PairNew from "./screens/PairNew";
import PairOld from "./screens/PairOld";
import PutTogether from "./screens/PutTogether";
import QuickHome from "./screens/QuickHome";
import QuickHost from "./screens/QuickHost";
import HistoryHost from "./screens/HistoryHost";
import Welcome, { type UpdateState } from "./screens/Welcome";
import WhichLaptop, { type Role } from "./screens/WhichLaptop";
import HeaderSettings from "./components/HeaderSettings";
import { Question } from "./icons";
import Opening from "./motion/Opening";
import { openingKind, type OpeningKind } from "./motion/openingRule";
import { applyToDocument } from "./motion/preference";
import { restoreReveal } from "./motion/play";
import { installScreenMotion, settleScreen } from "./motion/screen";
import { HELP_URL, openExternal } from "./external";
import "./styles.css";

/** Welcome, Which laptop is this?, How will you connect? (C01), Put both laptops together (C02), pairing (C03 to C05), then the move; and Quick Send's home. */
type Screen =
  | { id: "welcome" }
  | { id: "role" }
  | { id: "connect"; role: Role }
  | { id: "together"; role: Role }
  | { id: "pair"; role: Role }
  // The move, from paired to its outcome. Left only by its own buttons, so a running move always has a screen.
  | { id: "move"; role: Role }
  // Another person's things were handed over at their sign-in (M32): shown to them once, at start.
  | { id: "ready"; note: ReadyNote }
  // A move on this laptop stopped part way (X09): shown at start, before anything else.
  | { id: "pickup"; unfinished: number }
  // Quick Send, either way, from its home.
  | { id: "qs"; mode: "send" | "receive" }
  // My PCTwin history (F08), from Home or Quick Send's home; Back returns there.
  | { id: "history"; from: "welcome" | "quick" }
  // Help (S01), from the footer; Back returns to where it was opened.
  | { id: "help"; from: Screen }
  | { id: "soon"; which: Soon }
  // Quick Send home. `notice` is set when the person chose something the engine cannot do yet.
  | { id: "quick"; notice: boolean };

/** The app frame: header with the language choice, one screen at a time, and the free-move footer. */
export default function App({ initialLanguage }: { initialLanguage?: LanguageCode }) {
  const [language, setLanguage] = useState<LanguageCode>(() => initialLanguage ?? savedLanguage() ?? pickLanguage(navigator.languages));
  const [screen, setScreen] = useState<Screen>({ id: "welcome" });
  // A move that stopped part way on this laptop: Home shows "Resume your move" until it is tidied up.
  const [unfinished, setUnfinished] = useState(0);
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
    // The full move opens only in builds made with it switched on (VITE_PCTWIN_FULL_MOVE=1);
    // until it is ready for people to use, it is a coming-soon page like restore and rescue.
    setScreen(import.meta.env.VITE_PCTWIN_FULL_MOVE === "1" ? { id: "role" } : { id: "soon", which: "move" });
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
              <HeaderSettings language={language} onLanguage={choose} />
              <Localized id="header-help" attrs={{ "aria-label": true }}>
                <button
                  type="button"
                  className="header-icon"
                  aria-label="Help"
                  onClick={() => {
                    if (onHelp) onHelp();
                    else void openExternal(HELP_URL);
                  }}
                >
                  <Question />
                </button>
              </Localized>
            </div>
          </header>
          <main className="app-main" ref={main} tabIndex={-1} key={screen.id}>
            {screen.id === "welcome" && (
              <Welcome
                onStart={startMove}
                onSoon={(which) => { setScreen({ id: "soon", which }); }}
                onQuick={() => { setScreen({ id: "quick", notice: false }); }}
                onHistory={() => { setScreen({ id: "history", from: "welcome" }); }}
                unfinished={unfinished > 0}
                onResume={() => { setScreen({ id: "pickup", unfinished }); }}
                update={update}
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
                onHistory={() => { setScreen({ id: "history", from: "quick" }); }}
                onBack={() => { setScreen({ id: "welcome" }); }}
              />
            )}
            {screen.id === "history" && <HistoryHost onStart={startMove} onBack={() => { setScreen(screen.from === "welcome" ? { id: "welcome" } : { id: "quick", notice: false }); }} />}
            {screen.id === "qs" && <QuickHost mode={screen.mode} onHome={() => { setScreen({ id: "quick", notice: false }); }} />}
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
                onWindow={onWindow}
                onPairAgain={() => { setScreen({ id: "pair", role: screen.role }); }}
                onHome={() => { setScreen({ id: "welcome" }); }}
              />
            )}
          </main>
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
