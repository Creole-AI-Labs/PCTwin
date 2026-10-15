/**
 * The window's footer bar, shared by every screen that follows the extended mockups: a short free
 * note on the start side and the screen's main action on the end side.
 *
 * Use it in a screen:
 *
 *   <FrameFooter note="move">
 *     <Button variant="primary" onClick={next}>Continue</Button>
 *   </FrameFooter>
 *
 * - `note`: "move" shows "Free personal moves", "send" shows "Free nearby sending", "none" shows no
 *   note and lets the children fill the bar (for example Pause move at the start and Keep compact at
 *   the end). Defaults to "move".
 * - `children`: the action button or buttons. Keep them to one or two.
 *
 * Where it goes: the app (and the screens page) wraps the window in `FooterProvider`, which owns the
 * footer element. While a screen renders a `FrameFooter`, the footer shows the note and the children;
 * when none does, it shows the usual lock, "Free personal moves. No account needed." and Help.
 * Without a `FooterProvider` (a screen rendered alone, as in unit tests) the children render inline
 * at the end of the content, so nothing is lost. The footer comes after the content in the page, so
 * the action is reached by Tab after everything else on the screen.
 */
import { Localized } from "@fluent/react";
import { createContext, useCallback, useContext, useLayoutEffect, useMemo, useState, type ReactNode } from "react";
import { createPortal } from "react-dom";
import { HELP_URL, openExternal } from "../external";
import { Lock } from "../icons";
import "./frame.css";

export type FooterNote = "move" | "send" | "none";

interface Slot {
  /** The element the footer's actions are drawn into. */
  slot: HTMLElement;
  /** A screen says what note its footer shows, or null when it leaves. */
  claim: (note: FooterNote | null) => void;
}

const FooterContext = createContext<Slot | null>(null);

/** The usual footer: the free promise and Help. */
function DefaultFooter() {
  return (
    <>
      <span className="free">
        <Lock />
        <Localized id="footer-free">
          <span>Free personal moves. No account needed.</span>
        </Localized>
      </span>
      <button type="button" className="link" onClick={() => void openExternal(HELP_URL)}>
        <Localized id="footer-help">
          <span>Help</span>
        </Localized>
      </button>
    </>
  );
}

/** Renders `children` (the window's header and content) and then the footer element, after them. */
export function FooterProvider({ children }: { children: ReactNode }) {
  const [slot] = useState(() => {
    const el = document.createElement("div");
    el.className = "frame-actions";
    return el;
  });
  const [note, setNote] = useState<FooterNote | null>(null);
  const value = useMemo<Slot>(() => ({ slot, claim: setNote }), [slot]);
  const host = useCallback(
    (el: HTMLElement | null) => {
      if (el && slot.parentNode !== el) el.appendChild(slot);
    },
    [slot],
  );
  return (
    <FooterContext.Provider value={value}>
      {children}
      <footer className="app-footer">
        {note === null ? (
          <DefaultFooter />
        ) : (
          <>
            {note !== "none" && (
              <Localized id={note === "send" ? "footer-note-send" : "footer-note-move"}>
                <span className="frame-note" />
              </Localized>
            )}
            <div className="frame-actions-host" ref={host} />
          </>
        )}
      </footer>
    </FooterContext.Provider>
  );
}

export default function FrameFooter({ note = "move", children }: { note?: FooterNote; children: ReactNode }) {
  const ctx = useContext(FooterContext);
  useLayoutEffect(() => {
    ctx?.claim(note);
    return () => {
      ctx?.claim(null);
    };
  }, [ctx, note]);
  if (!ctx) return <div className="frame-footer-inline">{children}</div>;
  return createPortal(children, ctx.slot);
}
