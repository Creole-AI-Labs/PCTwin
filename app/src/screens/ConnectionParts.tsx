/** Pieces shared by the connection screens: how measured sizes, speeds and times are said. */
import { useLocalization } from "@fluent/react";
import "./connection.css";

/** Sizes and speeds come out of the translation files, so their units and number style follow the language. */
export function useUnits() {
  const { l10n } = useLocalization();
  return {
    gb: (value: number) => l10n.getString("connect-unit-gb", { value }),
    mbps: (value: number) => l10n.getString("connect-unit-mbps", { value }),
  };
}

/** "45 seconds ago", "5 minutes ago", "2 hours ago", in the language in use. Never rounds a gap up to look shorter. */
export function useAgo() {
  const { l10n } = useLocalization();
  const locale = (() => {
    for (const bundle of l10n.bundles) return bundle.locales[0];
    return "en";
  })();
  return (seconds: number) => {
    const rtf = new Intl.RelativeTimeFormat(locale, { numeric: "always" });
    if (seconds < 120) return rtf.format(-Math.max(0, Math.floor(seconds)), "second");
    if (seconds < 7200) return rtf.format(-Math.floor(seconds / 60), "minute");
    if (seconds < 172800) return rtf.format(-Math.floor(seconds / 3600), "hour");
    return rtf.format(-Math.floor(seconds / 86400), "day");
  };
}
