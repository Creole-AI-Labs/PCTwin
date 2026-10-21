/**
 * Links leave the app for the person's own browser; the app window never shows remote content.
 * Only our own site is allowed (see the capability's opener scope).
 */
import { openInBrowser } from "./bridge";

export const HELP_URL = "https://getpctwin.com/";

export async function openExternal(url: string): Promise<void> {
  if (!url.startsWith("https://getpctwin.com/")) return;
  await openInBrowser(url);
}
