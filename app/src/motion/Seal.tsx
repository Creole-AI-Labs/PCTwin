import { useLayoutEffect, useRef, type ReactNode } from "react";
import { play } from "./play";
import { readingSign } from "./screen";
import { DURATION, at } from "./tokens";

/**
 * The receipt seal (MC29): one check trace and a small settle on the icon of a result that is
 * verified as a whole. Only `verified` results get it; a partial, stopped or unchecked result shows
 * the same icon still. It plays once, when the result first appears, never on a refresh. The icon
 * is fully there from the first frame; the trace is a cover that slides off it (transform only).
 */
export default function Seal({ verified, children }: { verified: boolean; children: ReactNode }) {
  const ref = useRef<HTMLSpanElement>(null);
  useLayoutEffect(() => {
    const el = ref.current;
    if (!verified || !el) return;
    play(el, [{ transform: "scale(.86) rotate(-8deg)" }, { transform: "scale(1.025) rotate(1deg)", offset: 0.7 }, { transform: "none" }], { duration: DURATION.signature });
    const cover = el.querySelector(".motion-seal-trace");
    if (cover) {
      (cover as HTMLElement).style.transformOrigin = readingSign(el) === 1 ? "right center" : "left center";
      play(cover, [{ transform: "scaleX(1)" }, { transform: "scaleX(0)" }], { duration: at(250) });
    }
    // Once, when the result first appears.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);
  return (
    <span ref={ref} className="motion-seal" data-verified={verified ? "" : undefined}>
      {children}
      {verified && <span className="motion-seal-trace" aria-hidden="true" />}
    </span>
  );
}
