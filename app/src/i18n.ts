/**
 * Screen text: Fluent files, one per language (src/locales/<language>/main.ftl), all bundled with
 * the app, never fetched and never translated live. The chosen language comes first and English is
 * always last, so a missing string falls back to English instead of showing an id.
 */
import { FluentBundle, FluentResource } from "@fluent/bundle";
import { negotiateLanguages } from "@fluent/langneg";
import { ReactLocalization } from "@fluent/react";

/** The seven launch languages, each named in its own language for the picker. */
export const LANGUAGES = [
  { code: "en", name: "English" },
  { code: "fr", name: "Français" },
  { code: "es", name: "Español" },
  { code: "pt", name: "Português" },
  { code: "ar", name: "العربية" },
  { code: "zh-CN", name: "简体中文" },
  { code: "hi", name: "हिन्दी" },
] as const;

export type LanguageCode = (typeof LANGUAGES)[number]["code"];

const SOURCE = "en";
const RIGHT_TO_LEFT = new Set<string>(["ar"]);

const FILES = import.meta.glob<string>("./locales/*/main.ftl", { query: "?raw", import: "default", eager: true });

function fileFor(code: string): string {
  return FILES[`./locales/${code}/main.ftl`] ?? "";
}

/** The best launch language for what the system asks for, English if none fits. */
export function pickLanguage(requested: readonly string[]): LanguageCode {
  const [best] = negotiateLanguages(
    requested,
    LANGUAGES.map((l) => l.code),
    { defaultLocale: SOURCE, strategy: "lookup" },
  );
  // With a default locale the result is never empty.
  return best as LanguageCode;
}

/** Bundles for a language with English behind it. Every variable is isolated (bidi-safe). */
export function localization(code: LanguageCode): ReactLocalization {
  const order = code === SOURCE ? [SOURCE] : [code, SOURCE];
  const bundles = order.map((locale) => {
    const bundle = new FluentBundle(locale, { useIsolating: true });
    const errors = bundle.addResource(new FluentResource(fileFor(locale)));
    if (errors.length > 0) throw new Error(`Translation file for ${locale} has errors: ${errors.map(String).join("; ")}`);
    return bundle;
  });
  return new ReactLocalization(bundles);
}

/** Text direction for a language. */
export function directionOf(code: string): "ltr" | "rtl" {
  return RIGHT_TO_LEFT.has(code) ? "rtl" : "ltr";
}

const STORE_KEY = "pctwin.language";

/** The language the person picked before, kept on this laptop only. */
export function savedLanguage(): LanguageCode | null {
  try {
    const v = localStorage.getItem(STORE_KEY);
    return LANGUAGES.some((l) => l.code === v) ? (v as LanguageCode) : null;
  } catch {
    return null;
  }
}

export function saveLanguage(code: LanguageCode): void {
  try {
    localStorage.setItem(STORE_KEY, code);
  } catch {
    // Not saved: the system language is used next time.
  }
}
