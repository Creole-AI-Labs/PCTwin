/**
 * Undo of the full move's apps and settings, on the move's own page: the flow guard's screens
 * (the preview R05, then the result R06), every press checked again by the engine. When the person
 * goes back, the engine closes it and the outcome (R01) shows again.
 */
import { useEffect } from "react";
import { listenUndoClosed } from "../bridge";
import GuardHost from "./GuardHost";

export default function UndoHost({ onClosed }: { onClosed: () => void }) {
  useEffect(() => {
    let live = true;
    let stop: (() => void) | null = null;
    void listenUndoClosed(() => {
      if (live) onClosed();
    }).then((u) => {
      if (live) stop = u;
      else u();
    });
    return () => {
      live = false;
      stop?.();
    };
  }, [onClosed]);
  return <GuardHost />;
}
