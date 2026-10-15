import { LocalizationProvider, Localized } from "@fluent/react";
import { useEffect, useMemo, useRef, useState } from "react";
import lockup from "./assets/pctwin-lockup.png";
import { HELP_URL, openExternal } from "./external";
import { LANGUAGES, directionOf, localization, pickLanguage, saveLanguage, savedLanguage, type LanguageCode } from "./i18n";
import { Lock } from "./icons";
import { setTrayLabels } from "./native";
import { stop as stopPairing } from "./pairing";
import ComingSoon, { type Soon } from "./screens/ComingSoon";
import PairNew from "./screens/PairNew";
import PairOld from "./screens/PairOld";
import QuickHome from "./screens/QuickHome";
import Welcome from "./screens/Welcome";
import WhichLaptop, { type Role } from "./screens/WhichLaptop";
import "./styles.css";

type Screen =
  | { id: "welcome" }
  | { id: "role" }
  | { id: "pair"; role: Role }
  | { id: "soon"; which: Soon }
  // Quick Send home. `notice` is set when the person chose something the engine cannot do yet.
  | { id: "quick"; notice: boolean };

/** The app frame: header with the language choice, one screen at a time, and the free-move footer. */
export default function App({ initialLanguage }: { initialLanguage?: LanguageCode }) {
  const [language, setLanguage] = useState<LanguageCode>(() => initialLanguage ?? savedLanguage() ?? pickLanguage(navigator.languages));
  const [screen, setScreen] = useState<Screen>({ id: "welcome" });
  const l10n = useMemo(() => localization(language), [language]);
  const main = useRef<HTMLElement>(null);

  useEffect(() => {
    document.documentElement.lang = language;
    document.documentElement.dir = directionOf(language);
    void setTrayLabels(l10n.getString("tray-show"), l10n.getString("tray-quit")).catch(() => undefined);
  }, [language, l10n]);

  // A new screen starts at the top and moves keyboard focus to its heading area.
  useEffect(() => {
    if (!main.current) return;
    main.current.scrollTop = 0;
    // A screen that already moved focus to its own heading keeps it.
    if (!main.current.contains(document.activeElement)) main.current.focus();
  }, [screen]);

  const choose = (code: LanguageCode) => {
    setLanguage(code);
    saveLanguage(code);
  };

  return (
    <LocalizationProvider l10n={l10n}>
      <div className="app">
        <header className="app-header">
          <Localized id="brand-home" attrs={{ "aria-label": true }}>
            <button
              type="button"
              className="brand"
              aria-label="PCTwin home"
              onClick={() => {
                if (screen.id === "pair") void stopPairing().catch(() => undefined);
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
          <label className="language">
            <Localized id="language-label">
              <span className="visually-hidden">Language</span>
            </Localized>
            <select value={language} onChange={(e) => { choose(e.target.value as LanguageCode); }}>
              {LANGUAGES.map((l) => (
                <option key={l.code} value={l.code} lang={l.code}>
                  {l.name}
                </option>
              ))}
            </select>
          </label>
        </header>
        <main className="app-main" ref={main} tabIndex={-1} key={screen.id}>
          {screen.id === "welcome" && <Welcome
              onStart={() => { setScreen({ id: "role" }); }}
              onSoon={(which) => { setScreen({ id: "soon", which }); }}
              onQuick={() => { setScreen({ id: "quick", notice: false }); }}
            />}
          {screen.id === "quick" && (
            // Until the bridge exists nothing can be sent or received: each action says so, plainly, and changes nothing.
            <QuickHome
              notice={screen.notice}
              onSend={() => { setScreen({ ...screen, notice: true }); }}
              onReceive={() => { setScreen({ ...screen, notice: true }); }}
              onHistory={() => { setScreen({ ...screen, notice: true }); }}
              onBack={() => { setScreen({ id: "welcome" }); }}
            />
          )}
          {screen.id === "soon" && <ComingSoon which={screen.which} onBack={() => { setScreen({ id: "welcome" }); }} />}
          {screen.id === "role" && <WhichLaptop onBack={() => { setScreen({ id: "welcome" }); }} onChoose={(role) => { setScreen({ id: "pair", role }); }} />}
          {screen.id === "pair" && screen.role === "old" && <PairOld onBack={() => { setScreen({ id: "role" }); }} />}
          {screen.id === "pair" && screen.role === "new" && <PairNew onBack={() => { setScreen({ id: "role" }); }} />}
        </main>
        <footer className="app-footer">
          <span className="free">
            <Lock />
            <Localized id="footer-free">
              <span>Free personal moves. No account needed.</span>
            </Localized>
          </span>
          <button type="button" className="link" onClick={() => void openExternal(HELP_URL)}>
            <Localized id="footer-help">
              <span>Help</span>
            </Localized>
          </button>
        </footer>
      </div>
    </LocalizationProvider>
  );
}
