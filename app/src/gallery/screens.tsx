/**
 * The screens page (screens.html): one screen sample at a time, in its window size, for review and
 * for layout checks in a real browser. A review tool, not part of the shipped app.
 *
 * The address picks what to show, so a check can open any state directly:
 * `?sample=C02-default&lang=ar&width=320` (width narrows the frame to test small windows).
 */
import { LocalizationProvider } from "@fluent/react";
import React, { useEffect, useMemo, useState } from "react";
import ReactDOM from "react-dom/client";
import { LANGUAGES, directionOf, localization, type LanguageCode } from "../i18n";
import "../styles.css";
import "./gallery.css";
import { FooterProvider } from "../screens/FrameFooter";
import { FRAME, SAMPLES, type Sample } from "./samples";

function param(name: string): string | null {
  return new URLSearchParams(window.location.search).get(name);
}

/** The first sample, if any group has one. */
const FIRST: Sample | undefined = SAMPLES[0];

function Screens() {
  const [id, setId] = useState(() => param("sample") ?? FIRST?.id ?? "");
  const [language, setLanguage] = useState<LanguageCode>(() => (LANGUAGES.find((l) => l.code === param("lang"))?.code ?? "en"));
  const width = Number(param("width")) || null;
  const l10n = useMemo(() => localization(language), [language]);
  const sample: Sample | undefined = SAMPLES.find((s) => s.id === id) ?? FIRST;
  const frame = sample ? FRAME[sample.size] : FRAME.everyday;

  useEffect(() => {
    document.documentElement.lang = language;
    document.documentElement.dir = directionOf(language);
    const url = new URL(window.location.href);
    url.searchParams.set("sample", id);
    url.searchParams.set("lang", language);
    window.history.replaceState(null, "", url);
  }, [id, language]);

  return (
    <div className="screens-page">
      <div className="screens-controls" dir="ltr" lang="en">
        <label>
          Screen{" "}
          <select value={id} onChange={(e) => { setId(e.target.value); }}>
            {SAMPLES.map((s) => (
              <option key={s.id} value={s.id}>
                {s.id}: {s.title}
              </option>
            ))}
          </select>
        </label>
        <label>
          Language{" "}
          <select value={language} onChange={(e) => { setLanguage(e.target.value as LanguageCode); }}>
            {LANGUAGES.map((l) => (
              <option key={l.code} value={l.code}>
                {l.name}
              </option>
            ))}
          </select>
        </label>
      </div>
      <LocalizationProvider l10n={l10n}>
        <div className="app screens-frame" data-sample={sample?.id} style={{ inlineSize: `${String(width ?? frame.width)}px`, blockSize: `${String(frame.height)}px` }}>
          <FooterProvider key={`${sample?.id ?? ""}-${language}`}>
            <main className="app-main" tabIndex={-1}>
              {sample?.render()}
            </main>
          </FooterProvider>
        </div>
      </LocalizationProvider>
    </div>
  );
}

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>
    <Screens />
  </React.StrictMode>,
);
