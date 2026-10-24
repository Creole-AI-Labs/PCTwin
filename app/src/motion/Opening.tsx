import { useLocalization } from "@fluent/react";
import { useEffect, useLayoutEffect, useRef, type RefObject } from "react";
import lockup from "../assets/pctwin-lockup.png";
import { useMotion } from "./preference";
import { DURATION, EASING, at } from "./tokens";
import "./opening.css";

/** Where each letter of PCTwin starts: x, y (px) and turn (deg), as the pack has them. */
const STARTS: readonly (readonly [number, number, number])[] = [
  [-270, 0, -45],
  [-60, -240, 20],
  [90, 260, -25],
  [310, -40, 85],
  [40, -270, -70],
  [290, 170, 35],
];
const LETTERS = ["P", "C", "T", "w", "i", "n"];

/** The case mark is the header's mark drawn 3.2 times larger (27 px wide in the header). */
const MARK_SCALE = 3.2;
const MARK_WIDTH = 27;

export type OpeningEnd = "finished" | "skipped" | "interrupted";

/**
 * The Wordplay opening (MC01), ported from the pack's PCTwin_Wordplay_Opening.js onto the Web
 * Animations API: six letters settle into PCTwin around the citron block, the case mark arrives
 * and docks into the real header mark as Home appears, at half the pack's speed (4,800 ms). With
 * reduced motion it is the still identity for 560 ms instead.
 *
 * It is a layer over the app, which is built underneath from the first frame: the opening never
 * waits on the app and its end never means the app is ready. Skip intro and Escape end it at
 * once; so does anything that needs the person (`interrupt`: a screen other than Home came up),
 * the window being hidden or resized, or motion being turned down. Its finishing only removes
 * itself. Focus starts on Skip intro and goes to the app after.
 */
export default function Opening({
  kind,
  app,
  interrupt,
  onDone,
}: {
  kind: "wordplay" | "still";
  app: RefObject<HTMLElement | null>;
  interrupt: boolean;
  onDone: (how: OpeningEnd) => void;
}) {
  const { l10n } = useLocalization();
  const layer = useRef<HTMLElement>(null);
  const skip = useRef<HTMLButtonElement>(null);
  const ended = useRef(false);
  const done = useRef(onDone);
  const stop = useRef<() => void>(() => undefined);
  useLayoutEffect(() => {
    done.current = onDone;
  });

  const end = (how: OpeningEnd) => {
    if (ended.current) return;
    ended.current = true;
    stop.current();
    done.current(how);
  };
  const endRef = useRef(end);
  useLayoutEffect(() => {
    endRef.current = end;
  });

  useLayoutEffect(() => {
    const root = layer.current;
    const under = app.current;
    if (!root) return;
    const tracks: Animation[] = [];
    const track = (el: Element | null, frames: Keyframe[], duration: number, delay = 0, easing: string = EASING.settle, fill: FillMode = "both") => {
      if (!el) return null;
      const a = el.animate(frames, { duration, delay, easing, fill });
      a.finished.catch(() => undefined);
      tracks.push(a);
      return a;
    };
    const total = kind === "wordplay" ? DURATION.wordplay : DURATION.reducedOpening;
    const whole = (el: Element | null, frames: Keyframe[]) => track(el, frames, total, 0, "linear");

    under?.setAttribute("inert", "");
    under?.classList.add("opening-under");

    if (kind === "wordplay") {
      whole(root.querySelector(".opening-block"), [
        { opacity: 0, transform: "translateX(-320px) rotate(-25deg) scale(.6)", offset: 0, easing: "cubic-bezier(.2,.65,.15,1)" },
        { opacity: 1, transform: "translateX(0) rotate(-7deg) scale(1)", offset: 0.25 },
        { opacity: 1, transform: "translateX(0) rotate(4deg) scale(1.05)", offset: 0.64, easing: EASING.dock },
        { opacity: 0, transform: "translateX(330px) rotate(28deg) scale(.7)", offset: 0.91 },
        { opacity: 0, offset: 1 },
      ]);
      root.querySelectorAll(".opening-letter").forEach((letter, i) => {
        const [x, y, r] = STARTS[i] ?? [0, 0, 0];
        const away = `translate(${String((i - 2.5) * 16)}px,-135px) rotate(${String((i - 2.5) * 5)}deg) scale(.24)`;
        whole(letter, [
          { opacity: 0, transform: `translate(${String(x)}px,${String(y)}px) rotate(${String(r)}deg) scale(1.4)`, offset: 0 },
          { opacity: 0, transform: `translate(${String(x)}px,${String(y)}px) rotate(${String(r)}deg) scale(1.4)`, offset: 0.025 * i, easing: "cubic-bezier(.16,.75,.2,1)" },
          { opacity: 1, transform: "translate(0,0) rotate(0deg) scale(1.025)", offset: 0.29 + 0.018 * i, easing: EASING.settle },
          { opacity: 1, transform: "translate(0,0) rotate(0deg) scale(1)", offset: 0.39 + 0.018 * i },
          { opacity: 1, transform: "translate(0,0) rotate(0deg) scale(1)", offset: 0.67, easing: "cubic-bezier(.55,0,.35,1)" },
          { opacity: 0, transform: away, offset: 0.91 },
          { opacity: 0, offset: 1 },
        ]);
      });
      const mark = root.querySelector(".opening-case");
      whole(mark, [
        { opacity: 0, transform: "translateY(-50px) scale(.7)", offset: 0 },
        { opacity: 0, transform: "translateY(-50px) scale(.7)", offset: 0.3, easing: "cubic-bezier(.16,.75,.2,1)" },
        { opacity: 1, transform: "translateY(0) scale(1)", offset: 0.51 },
        { opacity: 1, transform: "translateY(0) scale(1)", offset: 1 },
      ]);
      whole(root.querySelector(".opening-tagline"), [
        { opacity: 0, transform: "translateY(6px)", offset: 0 },
        { opacity: 0, transform: "translateY(6px)", offset: 0.4, easing: EASING.settle },
        { opacity: 1, transform: "translateY(0)", offset: 0.55 },
        { opacity: 1, transform: "translateY(0)", offset: 0.7 },
        { opacity: 0, transform: "translateY(-4px)", offset: 0.87 },
        { opacity: 0, offset: 1 },
      ]);
      // The name finds its place: the case mark docks onto the header's own mark.
      const markInner = mark?.querySelector(".brand-mark");
      const target = under?.querySelector(".app-header .brand-mark")?.getBoundingClientRect();
      const from = markInner?.getBoundingClientRect();
      if (mark && from && target && target.width > 0 && from.width > 0) {
        const dx = target.left + target.width / 2 - (from.left + from.width / 2);
        const dy = target.top + target.height / 2 - (from.top + from.height / 2);
        const scale = target.width / (MARK_WIDTH * MARK_SCALE);
        track(mark, [{ transform: "translate(0,0) scale(1)" }, { transform: `translate(${String(dx)}px,${String(dy)}px) scale(${String(scale)})` }], at(2310 - 1740), at(1740), EASING.dock, "forwards");
      }
      track(root.querySelector(".opening-backdrop"), [{ opacity: 1 }, { opacity: 0 }], at(470), at(1870), "linear");
      track(under ?? null, [{ opacity: 0 }, { opacity: 1 }], at(470), at(1870), "cubic-bezier(0,0,0,1)");
      track(skip.current, [{ opacity: 1 }, { opacity: 0 }], at(160), total - at(220), "linear");
    } else {
      track(under ?? null, [{ opacity: 0 }, { opacity: 0 }], total, 0, "linear");
    }

    const clock = track(root, [{ opacity: 1 }, { opacity: 1 }], total, 0, "linear");
    clock?.finished.then(
      () => {
        endRef.current("finished");
      },
      () => undefined,
    );

    stop.current = () => {
      tracks.forEach((a) => {
        try {
          a.cancel();
        } catch {
          // Already gone.
        }
      });
      tracks.length = 0;
      under?.removeAttribute("inert");
      under?.classList.remove("opening-under");
    };
    skip.current?.focus({ preventScroll: true });
    return () => {
      stop.current();
    };
  }, [kind, app]);

  // Escape skips; Tab stays on Skip intro while the opening is up. Hidden, resized or motion
  // turned down: end at once.
  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") {
        e.preventDefault();
        e.stopPropagation();
        endRef.current("skipped");
      } else if (e.key === "Tab") {
        e.preventDefault();
        skip.current?.focus();
      }
    };
    const onHidden = () => {
      if (document.hidden) endRef.current("interrupted");
    };
    const onResize = () => {
      endRef.current("interrupted");
    };
    document.addEventListener("keydown", onKey, true);
    document.addEventListener("visibilitychange", onHidden);
    window.addEventListener("resize", onResize);
    return () => {
      document.removeEventListener("keydown", onKey, true);
      document.removeEventListener("visibilitychange", onHidden);
      window.removeEventListener("resize", onResize);
    };
  }, []);

  // Something that needs the person, or motion turned down, ends it.
  const { mode } = useMotion();
  useEffect(() => {
    if (interrupt || mode === "off" || (kind === "wordplay" && mode !== "full")) endRef.current("interrupted");
  }, [interrupt, mode, kind]);

  return (
    <section ref={layer} className={`opening opening-${kind}`} role="dialog" aria-modal="true" aria-label={l10n.getString("opening-label")} tabIndex={-1}>
      <div className="opening-backdrop" />
      {kind === "wordplay" && <div className="opening-block" />}
      {kind === "wordplay" && (
        <div className="opening-word" aria-hidden="true" dir="ltr">
          {LETTERS.map((ch, i) => (
            <span key={i} className="opening-letter">
              {ch}
            </span>
          ))}
        </div>
      )}
      <div className="opening-case" aria-hidden="true">
        <span className="brand-mark">
          <img src={lockup} alt="" />
        </span>
      </div>
      {kind === "still" && (
        <p className="opening-name" aria-hidden="true" dir="ltr">
          PCTwin
        </p>
      )}
      <p className="opening-tagline">{l10n.getString("opening-tagline")}</p>
      <button
        ref={skip}
        type="button"
        className="opening-skip"
        onClick={() => {
          end("skipped");
        }}
      >
        {l10n.getString("opening-skip")}
      </button>
      <p className="visually-hidden" role="status">
        {l10n.getString("opening-status")}
      </p>
    </section>
  );
}
