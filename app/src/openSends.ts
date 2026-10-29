/**
 * Open sends, as the engine lists them (src-tauri/src/sessions.rs): the sessions still open with
 * another laptop, the sends waiting their turn or being sent, and the result of each of this
 * laptop's sends that ended. One store for the whole app, so the list stays the same whichever
 * page shows it, and a result that came while another page was open is still there.
 *
 * It listens while any page uses it; when the last one leaves it forgets everything (the engine's
 * own list is asked again next time). Results are only this laptop's sends (`send` set); what the
 * other laptop sent here is the receiving page's to show.
 */
import { useSyncExternalStore } from "react";
import { listenKnown, qsSends, qsSessions, type QsSend, type QsSession, type QsSessionReceipt } from "./bridge";

export type OpenSends = {
  sessions: QsSession[];
  sends: QsSend[];
  /** This laptop's sends that ended, oldest first, each with the other laptop's name when known. */
  results: (QsSessionReceipt & { send: number; name: string | null })[];
};

const EMPTY: OpenSends = { sessions: [], sends: [], results: [] };

let state: OpenSends = EMPTY;
const subscribers = new Set<() => void>();
let stop: (() => void) | null = null;
/** Bumped when the store stops, so a late reply from an earlier start is dropped. */
let round = 0;

function set(next: OpenSends) {
  state = next;
  for (const s of subscribers) s();
}

function start() {
  const mine = ++round;
  void listenKnown((event) => {
    if (mine !== round) return;
    if (event.kind === "sessions") set({ ...state, sessions: event.sessions });
    else if (event.kind === "sends") set({ ...state, sends: event.sends });
    else if (event.kind === "session-receipt" && event.result.send !== null) {
      const { send, session } = event.result;
      // The name is the one the list had for it, before the send left the list.
      const name = state.sends.find((s) => s.id === send)?.name ?? state.sessions.find((s) => s.id === session)?.name ?? null;
      if (state.results.some((r) => r.send === send)) return;
      set({ ...state, results: [...state.results, { ...event.result, send, name }] });
    }
  }).then((un) => {
    if (mine === round) stop = un;
    else un();
  });
  qsSessions().then(
    (sessions) => {
      if (mine === round) set({ ...state, sessions });
    },
    () => undefined,
  );
  qsSends().then(
    (sends) => {
      if (mine === round) set({ ...state, sends });
    },
    () => undefined,
  );
}

function subscribe(onChange: () => void): () => void {
  subscribers.add(onChange);
  if (subscribers.size === 1) start();
  return () => {
    subscribers.delete(onChange);
    if (subscribers.size > 0) return;
    round++;
    stop?.();
    stop = null;
    state = EMPTY;
  };
}

const snapshot = () => state;

/** The open sends, kept current while the page is shown. */
export function useOpenSends(): OpenSends {
  return useSyncExternalStore(subscribe, snapshot, snapshot);
}

/** A list the engine sent back from a command (Add files, Pause, Resume, Cancel): shown at once. */
export function showSends(sends: QsSend[]) {
  set({ ...state, sends });
}

/** Done on the list of sends: the results shown there are cleared (the sends themselves go on). */
export function clearResults() {
  set({ ...state, results: [] });
}
