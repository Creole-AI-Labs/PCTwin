/**
 * A stand-in for the Web Animations API (jsdom has none): records every `element.animate` call so a
 * test can see what would move, finish or cancel it, and check nothing waits on it.
 */
export class FakeAnimation {
  readonly finished: Promise<FakeAnimation>;
  cancelled = false;
  done = false;
  private settle!: (a: FakeAnimation) => void;
  private abort!: (e: Error) => void;
  constructor(
    readonly target: Element,
    readonly keyframes: Keyframe[],
    readonly options: KeyframeAnimationOptions,
  ) {
    this.finished = new Promise<FakeAnimation>((resolve, reject) => {
      this.settle = resolve;
      this.abort = reject;
    });
    // Like the real thing, an unobserved cancellation is not an error.
    this.finished.catch(() => undefined);
  }
  finish(): void {
    if (this.cancelled || this.done) return;
    this.done = true;
    this.settle(this);
  }
  cancel(): void {
    if (this.cancelled || this.done) return;
    this.cancelled = true;
    this.abort(new DOMException("cancelled", "AbortError"));
  }
  pause(): void {}
  play(): void {}
  get duration(): number {
    return Number(this.options.duration ?? 0);
  }
  get delay(): number {
    return this.options.delay ?? 0;
  }
  /** Every animated property in the keyframes. */
  get properties(): string[] {
    return [...new Set(this.keyframes.flatMap((k) => Object.keys(k).filter((p) => !["offset", "easing", "composite"].includes(p))))];
  }
}

export function fakeAnimations() {
  const made: FakeAnimation[] = [];
  const had = Object.prototype.hasOwnProperty.call(Element.prototype, "animate");
  const original = (Element.prototype as { animate?: unknown }).animate;
  Object.defineProperty(Element.prototype, "animate", {
    configurable: true,
    writable: true,
    value: function animate(this: Element, keyframes: Keyframe[], options: KeyframeAnimationOptions) {
      const a = new FakeAnimation(this, keyframes, options);
      made.push(a);
      return a;
    },
  });
  return {
    made,
    /** Animations on elements matching a selector (or the element itself). */
    on(target: string | Element): FakeAnimation[] {
      return made.filter((a) => (typeof target === "string" ? a.target.matches(target) : a.target === target));
    },
    running(): FakeAnimation[] {
      return made.filter((a) => !a.cancelled && !a.done);
    },
    restore(): void {
      if (had) Object.defineProperty(Element.prototype, "animate", { configurable: true, writable: true, value: original });
      else delete (Element.prototype as { animate?: unknown }).animate;
    },
  };
}
