/**
 * The few things the screens ask of the desktop shell's window and tray, through bridge.ts. Each
 * is a named request the shell checks; outside the app (tests, a browser preview) they do nothing.
 */
export { setTrayLabels, setWindowMode, windowDrag, type WindowMode } from "./bridge";
