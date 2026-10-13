/**
 * Links leave the app for the person's own browser; the app window never shows remote content.
 * Only our own site is allowed (see the capability's opener scope).
 */
export const HELP_URL = "https://getpctwin.com/";

export async function openExternal(url: string): Promise<void> {
  if (!url.startsWith("https://getpctwin.com/")) return;
  if ("__TAURI_INTERNALS__" in window) {
    const { openUrl } = await import("@tauri-apps/plugin-opener");
    await openUrl(url);
  } else {
    window.open(url, "_blank", "noopener,noreferrer");
  }
}
