import { useLayoutEffect, useRef, type ReactNode } from "react";
import { play } from "./play";
import { DURATION } from "./tokens";

/**
 * A small accent that follows a real event (MC14: the laptops are paired and the other one is
 * confirmed). The icon settles once when it first appears; the words beside it never move. It is
 * not a result seal: that belongs to verified results only.
 */
export default function Accent({ children }: { children: ReactNode }) {
  const ref = useRef<HTMLSpanElement>(null);
  useLayoutEffect(() => {
    play(ref.current, [{ opacity: 0.6, transform: "scale(.9)" }, { opacity: 1, transform: "none" }], { duration: DURATION.state });
  }, []);
  return (
    <span ref={ref} className="motion-accent">
      {children}
    </span>
  );
}
