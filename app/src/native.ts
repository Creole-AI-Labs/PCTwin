/**
 * The few things the screens ask of the desktop shell. Each is a named request the shell checks;
 * outside the app (tests, a browser preview) they do nothing.
 */
const inApp = () => "__TAURI_INTERNALS__" in window;

async function invoke(command: string, args: Record<string, unknown>): Promise<void> {
  if (!inApp()) return;
  const core = await import("@tauri-apps/api/core");
  await core.invoke(command, args);
}

/** The three window sizes from the design system. */
export type WindowMode = "everyday" | "mini" | "expanded";

export function setWindowMode(mode: WindowMode): Promise<void> {
  return invoke("set_window_mode", { mode });
}

/** The tray menu speaks the person's language too. */
export function setTrayLabels(show: string, quit: string): Promise<void> {
  return invoke("set_tray_labels", { show, quit });
}
