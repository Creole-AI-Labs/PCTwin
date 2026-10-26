import { useLocalization } from "@fluent/react";
import { useEffect, useId, useRef, useState, type ReactNode } from "react";
import { LANGUAGES, type LanguageCode } from "../i18n";
import { Globe, MotionLines } from "../icons";
import { PREFERENCES, useMotion } from "../motion/preference";

/** The short name on the header control, as in the mockup's "EN". */
const SHORT: Record<LanguageCode, string> = { en: "EN", fr: "FR", es: "ES", pt: "PT", ar: "AR", "zh-CN": "ZH", hi: "HI" };

type Choice = { key: string; label: string; lang?: string; current: boolean; choose: () => void };

/**
 * One header control and the short list it opens: each choice is a button, the current one marked
 * as pressed; choosing applies at once and closes the list. Escape, or a click outside, closes it;
 * Escape and a choice return focus to the control.
 */
function HeaderMenu({ className, label, face, choices }: { className: string; label: string; face: ReactNode; choices: Choice[] }) {
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

  const close = () => {
    setOpen(false);
    button.current?.focus();
  };

  return (
    <div
      className="header-settings"
      ref={root}
      onKeyDown={(e) => {
        if (e.key !== "Escape" || !open) return;
        e.stopPropagation();
        close();
      }}
    >
      <button
        type="button"
        ref={button}
        className={`header-icon ${className}`}
        aria-label={label}
        aria-expanded={open}
        aria-controls={panel}
        onClick={() => {
          setOpen((o) => !o);
        }}
      >
        {face}
      </button>
      {open && (
        <ul className="header-settings-panel" id={panel} aria-label={label}>
          {choices.map((c) => (
            <li key={c.key}>
              <button
                type="button"
                className="header-choice"
                lang={c.lang}
                aria-pressed={c.current}
                onClick={() => {
                  c.choose();
                  close();
                }}
              >
                {c.label}
              </button>
            </li>
          ))}
        </ul>
      )}
    </div>
  );
}

/** The header's language control (the mockup's globe and "EN"): it opens the list of languages. */
export function HeaderLanguage({ language, onLanguage }: { language: LanguageCode; onLanguage: (code: LanguageCode) => void }) {
  const { l10n } = useLocalization();
  return (
    <HeaderMenu
      className="header-language"
      label={l10n.getString("language-label")}
      face={
        <>
          <Globe />
          <span lang="en">{SHORT[language]}</span>
        </>
      }
      choices={LANGUAGES.map((l) => ({
        key: l.code,
        label: l.name,
        lang: l.code,
        current: l.code === language,
        choose: () => {
          onLanguage(l.code);
        },
      }))}
    />
  );
}

/** The header's motion control: follow the system (the default), reduced, or off; applies at once. */
export function HeaderMotion() {
  const { l10n } = useLocalization();
  const { preference, setPreference } = useMotion();
  return (
    <HeaderMenu
      className="header-motion"
      label={l10n.getString("motion-label")}
      face={<MotionLines />}
      choices={PREFERENCES.map((p) => ({
        key: p,
        label: l10n.getString(`motion-${p}`),
        current: p === preference,
        choose: () => {
          setPreference(p);
        },
      }))}
    />
  );
}
