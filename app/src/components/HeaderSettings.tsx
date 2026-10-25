import { Localized } from "@fluent/react";
import { useEffect, useId, useRef, useState } from "react";
import { LANGUAGES, type LanguageCode } from "../i18n";
import { Globe } from "../icons";
import MotionChoice from "../motion/MotionChoice";

/** The short name on the header control, as in the mockup's "EN". */
const SHORT: Record<LanguageCode, string> = { en: "EN", fr: "FR", es: "ES", pt: "PT", ar: "AR", "zh-CN": "ZH", hi: "HI" };

/**
 * The header's language control (the mockup's globe and "EN"). It opens a small panel with the
 * language choice and the Motion choice, the app's settings for now. Escape, or a click outside,
 * closes it; Escape returns focus to the control.
 */
export default function HeaderSettings({ language, onLanguage }: { language: LanguageCode; onLanguage: (code: LanguageCode) => void }) {
  const [open, setOpen] = useState(false);
  const root = useRef<HTMLDivElement>(null);
  const button = useRef<HTMLButtonElement>(null);
  const panel = useId();

  useEffect(() => {
    if (!open) return;
    const outside = (e: PointerEvent) => {
      if (e.target instanceof Node && root.current?.contains(e.target)) return;
      setOpen(false);
    };
    document.addEventListener("pointerdown", outside);
    return () => {
      document.removeEventListener("pointerdown", outside);
    };
  }, [open]);

  return (
    <div
      className="header-settings"
      ref={root}
      onKeyDown={(e) => {
        if (e.key !== "Escape" || !open) return;
        e.stopPropagation();
        setOpen(false);
        button.current?.focus();
      }}
    >
      <Localized id="header-settings" attrs={{ "aria-label": true }}>
        <button
          type="button"
          ref={button}
          className="header-icon header-language"
          aria-label="Language and motion"
          aria-expanded={open}
          aria-controls={panel}
          onClick={() => {
            setOpen((o) => !o);
          }}
        >
          <Globe />
          <span lang="en">{SHORT[language]}</span>
        </button>
      </Localized>
      {open && (
        <div className="header-settings-panel" id={panel}>
          <label className="language">
            <Localized id="language-label">
              <span className="visually-hidden">Language</span>
            </Localized>
            <select
              value={language}
              onChange={(e) => {
                onLanguage(e.target.value as LanguageCode);
              }}
            >
              {LANGUAGES.map((l) => (
                <option key={l.code} value={l.code} lang={l.code}>
                  {l.name}
                </option>
              ))}
            </select>
          </label>
          <MotionChoice />
        </div>
      )}
    </div>
  );
}
