/**
 * The component kit, from the approved mockups (design batch 01): buttons, fields, the six-digit
 * code, toggles, checkboxes, selects, segmented choices, rows and sections, result chips, progress,
 * the mini progress panel, banners, dialogs and toasts. Text comes from the caller (translated);
 * the kit adds structure, states and accessibility. Status is never shown by colour alone.
 */
import { Localized, useLocalization } from "@fluent/react";
import { useEffect, useId, useRef, type ButtonHTMLAttributes, type InputHTMLAttributes, type KeyboardEvent, type ReactNode } from "react";
import "./kit.css";

/* ---------- Buttons ---------- */

type ButtonProps = ButtonHTMLAttributes<HTMLButtonElement> & { variant?: "primary" | "secondary" | "citron" | "quiet"; icon?: ReactNode };

export function Button({ variant = "secondary", icon, children, className, type = "button", ...rest }: ButtonProps) {
  return (
    <button type={type} className={`kit-button kit-button-${variant} ${className ?? ""}`} {...rest}>
      <span>{children}</span>
      {icon}
    </button>
  );
}

/* ---------- Text field ---------- */

type FieldProps = InputHTMLAttributes<HTMLInputElement> & { label: ReactNode; hint?: ReactNode; error?: ReactNode };

export function TextField({ label, hint, error, id, ...rest }: FieldProps) {
  const auto = useId();
  const inputId = id ?? auto;
  const hintId = `${inputId}-hint`;
  const errorId = `${inputId}-error`;
  const describedBy = [hint ? hintId : "", error ? errorId : ""].filter(Boolean).join(" ") || undefined;
  return (
    <div className="kit-field">
      <label htmlFor={inputId}>{label}</label>
      <input id={inputId} aria-describedby={describedBy} aria-invalid={error ? true : undefined} {...rest} />
      {hint && (
        <small id={hintId} className="kit-hint">
          {hint}
        </small>
      )}
      {error && (
        <small id={errorId} className="kit-error">
          <ErrorIcon />
          {error}
        </small>
      )}
    </div>
  );
}

/* ---------- Six-digit code ---------- */

/** The code on the sending laptop: six large tiles in a box, read out as one label, with a line under it (the expiry). */
export function CodeDisplay({ digits, meta, group }: { digits: string; meta?: ReactNode; group?: number }) {
  const { l10n } = useLocalization();
  // With a grouping (3 gives "123 456") the tiles sit in runs with a gap; it still reads as one code.
  const size = group && group > 0 ? group : Math.max(digits.length, 1);
  const runs: string[] = [];
  for (let i = 0; i < digits.length; i += size) runs.push(digits.slice(i, i + size));
  return (
    <div className="kit-code-box">
      <div className="kit-code-display" dir="ltr" role="group" aria-label={`${l10n.getString("code-label")}: ${digits.split("").join(" ")}`}>
        {runs.map((run, r) => (
          <span key={r} className="kit-code-run" aria-hidden="true">
            {run.split("").map((d, i) => (
              <span key={i} className="kit-code-tile">
                {d}
              </span>
            ))}
          </span>
        ))}
      </div>
      {meta && <p className="kit-code-meta">{meta}</p>}
    </div>
  );
}

/** Digits from any script ("٤٨٢", "４８２") become 0–9; anything else is dropped. */
export function onlyDigits(text: string): string {
  let out = "";
  for (const ch of text) {
    if (/\p{Nd}/u.test(ch)) {
      const cp = ch.codePointAt(0) ?? 0;
      let zero = cp;
      while (zero > 0 && cp - zero < 60 && /\p{Nd}/u.test(String.fromCodePoint(zero - 1))) zero--;
      out += String((cp - zero) % 10);
    }
  }
  return out;
}

/** The code on the receiving laptop: one field with one label, six digits, pasting allowed. */
export function CodeInput({ value, onChange, label, hint }: { value: string; onChange: (digits: string) => void; label: ReactNode; hint?: ReactNode }) {
  return (
    <TextField
      className="kit-code-input"
      label={label}
      hint={hint}
      value={value}
      inputMode="numeric"
      autoComplete="one-time-code"
      spellCheck={false}
      onChange={(e) => {
        onChange(onlyDigits(e.target.value).slice(0, 6));
      }}
    />
  );
}

/* ---------- Toggle, checkbox, select, segmented ---------- */

export function Toggle({ checked, onChange, label, hint }: { checked: boolean; onChange: (on: boolean) => void; label: ReactNode; hint?: ReactNode }) {
  const id = useId();
  return (
    <div className="kit-setting">
      <span className="kit-setting-text">
        <span id={`${id}-label`} className="kit-setting-title">
          {label}
        </span>
        {hint && (
          <small id={`${id}-hint`} className="kit-hint">
            {hint}
          </small>
        )}
      </span>
      <button
        type="button"
        role="switch"
        className="kit-toggle"
        aria-checked={checked}
        aria-labelledby={`${id}-label`}
        aria-describedby={hint ? `${id}-hint` : undefined}
        onClick={() => {
          onChange(!checked);
        }}
      >
        <span className="kit-toggle-track" />
      </button>
    </div>
  );
}

export function Checkbox({ checked, onChange, label, hint }: { checked: boolean; onChange: (on: boolean) => void; label: ReactNode; hint?: ReactNode }) {
  return (
    <label className="kit-check">
      <input
        type="checkbox"
        checked={checked}
        onChange={(e) => {
          onChange(e.target.checked);
        }}
      />
      <span>
        <strong>{label}</strong>
        {hint && <small className="kit-hint">{hint}</small>}
      </span>
    </label>
  );
}

export function Select<T extends string>({
  label,
  hint,
  value,
  options,
  onChange,
}: {
  label: ReactNode;
  hint?: ReactNode;
  value: T;
  options: readonly { value: T; label: string; lang?: string }[];
  onChange: (value: T) => void;
}) {
  const id = useId();
  return (
    <div className="kit-setting">
      <span className="kit-setting-text">
        <label htmlFor={id} className="kit-setting-title">
          {label}
        </label>
        {hint && <small className="kit-hint">{hint}</small>}
      </span>
      <select
        id={id}
        className="kit-select"
        value={value}
        onChange={(e) => {
          onChange(e.target.value as T);
        }}
      >
        {options.map((o) => (
          <option key={o.value} value={o.value} lang={o.lang}>
            {o.label}
          </option>
        ))}
      </select>
    </div>
  );
}

/** A small set of exclusive choices (Light / System). Arrow keys move the choice, as in a radio group. */
export function Segmented<T extends string>({
  label,
  hint,
  value,
  options,
  onChange,
}: {
  label: ReactNode;
  hint?: ReactNode;
  value: T;
  options: readonly { value: T; label: ReactNode }[];
  onChange: (value: T) => void;
}) {
  const id = useId();
  const refs = useRef<(HTMLButtonElement | null)[]>([]);
  const move = (e: KeyboardEvent, i: number) => {
    const rtl = document.documentElement.dir === "rtl";
    const next = { ArrowRight: rtl ? -1 : 1, ArrowDown: 1, ArrowLeft: rtl ? 1 : -1, ArrowUp: -1 }[e.key];
    if (next === undefined) return;
    e.preventDefault();
    const j = (i + next + options.length) % options.length;
    onChange(options[j].value);
    refs.current[j]?.focus();
  };
  return (
    <div className="kit-setting">
      <span className="kit-setting-text">
        <span id={`${id}-label`} className="kit-setting-title">
          {label}
        </span>
        {hint && <small className="kit-hint">{hint}</small>}
      </span>
      <div className="kit-segmented" role="radiogroup" aria-labelledby={`${id}-label`}>
        {options.map((o, i) => (
          <button
            key={o.value}
            ref={(el) => {
              refs.current[i] = el;
            }}
            type="button"
            role="radio"
            aria-checked={o.value === value}
            tabIndex={o.value === value ? 0 : -1}
            onClick={() => {
              onChange(o.value);
            }}
            onKeyDown={(e) => {
              move(e, i);
            }}
          >
            {o.label}
          </button>
        ))}
      </div>
    </div>
  );
}

/* ---------- Rows and sections ---------- */

export function Section({ title, children }: { title: ReactNode; children: ReactNode }) {
  const id = useId();
  return (
    <section className="kit-section" aria-labelledby={id}>
      <h2 id={id} className="kit-section-title">
        {title}
      </h2>
      {children}
    </section>
  );
}

export function RowList({ children }: { children: ReactNode }) {
  return <ul className="kit-rows">{children}</ul>;
}

/** One row: an icon and a name on one side, its state on the other (the move's task list). */
export function Row({ icon, title, detail, end }: { icon?: ReactNode; title: ReactNode; detail?: ReactNode; end?: ReactNode }) {
  return (
    <li className="kit-row">
      <span className="kit-row-name">
        {icon && <span className="kit-row-icon">{icon}</span>}
        <span>
          {title}
          {detail && <small className="kit-hint">{detail}</small>}
        </span>
      </span>
      {end && <span className="kit-row-end">{end}</span>}
    </li>
  );
}

/* ---------- Result chips and badges ---------- */

/** Every item ends with one of these (the honest-results rule). */
export type Result = "copied" | "reinstalled" | "converted" | "needs-action" | "skipped" | "failed";

const TONE: Record<Result, "good" | "warn" | "neutral" | "bad"> = {
  copied: "good",
  reinstalled: "good",
  converted: "good",
  "needs-action": "warn",
  skipped: "neutral",
  failed: "bad",
};

export function ResultChip({ result }: { result: Result }) {
  const tone = TONE[result];
  return (
    <span className={`kit-chip kit-chip-${tone}`}>
      {tone === "good" && <CheckIcon />}
      {tone === "warn" && <AttentionIcon />}
      {tone === "bad" && <ErrorIcon />}
      {tone === "neutral" && <SkipIcon />}
      <Localized id={`status-${result}`}>
        <span>{result}</span>
      </Localized>
    </span>
  );
}

export function Badge({ tone = "neutral", icon, children }: { tone?: "neutral" | "good" | "warn"; icon?: ReactNode; children: ReactNode }) {
  return (
    <span className={`kit-chip kit-chip-${tone}`}>
      {icon}
      <span>{children}</span>
    </span>
  );
}

/* ---------- Progress ---------- */

/** Measured progress only: the value comes from work actually done, never an animation. */
export function ProgressBar({ value, label, left, right }: { value: number; label: string; left?: ReactNode; right?: ReactNode }) {
  const pct = Math.max(0, Math.min(100, Math.round(value * 100)));
  return (
    <>
      <div className="kit-progress" role="progressbar" aria-label={label} aria-valuemin={0} aria-valuemax={100} aria-valuenow={pct}>
        <span className="kit-progress-fill" style={{ inlineSize: `${pct.toString()}%` }} />
      </div>
      {(left ?? right) && (
        <p className="kit-progress-meta">
          <span>{left}</span>
          <span>{right}</span>
        </p>
      )}
    </>
  );
}

export function ProgressValue({ value }: { value: number }) {
  const pct = Math.max(0, Math.min(100, Math.round(value * 100)));
  return (
    <span className="kit-progress-value" aria-hidden="true">
      {pct}
      <small>%</small>
    </span>
  );
}

/** The 360 × 240 mini panel (F09): what is moving, measured progress, pause and reopen. */
export function MiniProgress({
  title,
  value,
  label,
  left,
  right,
  pause,
  open,
}: {
  title: ReactNode;
  value: number;
  label: string;
  left?: ReactNode;
  right?: ReactNode;
  pause: ReactNode;
  open: ReactNode;
}) {
  return (
    <div className="kit-mini">
      <div className="kit-mini-heading">
        <h3>{title}</h3>
        <strong aria-hidden="true">{Math.round(value * 100)}%</strong>
      </div>
      <ProgressBar value={value} label={label} left={left} right={right} />
      <div className="kit-mini-actions">
        {pause}
        {open}
      </div>
    </div>
  );
}

/* ---------- Banners, dialogs, toasts ---------- */

export function Banner({ tone = "info", title, children }: { tone?: "info" | "ready" | "attention"; title?: ReactNode; children: ReactNode }) {
  return (
    <div className={`kit-banner kit-banner-${tone}`} role={tone === "attention" ? "alert" : undefined}>
      {tone === "ready" ? <CheckIcon /> : tone === "attention" ? <AttentionIcon /> : <InfoIcon />}
      <div>
        {title && <strong className="kit-banner-title">{title}</strong>}
        <div>{children}</div>
      </div>
    </div>
  );
}

/**
 * A modal dialog: the native element keeps focus inside it and closes on Escape; focus returns to
 * where it was. The title names it for screen readers.
 */
export function Dialog({
  open,
  title,
  onClose,
  children,
  actions,
  closeLabel,
}: {
  open: boolean;
  title: ReactNode;
  onClose: () => void;
  children: ReactNode;
  actions?: ReactNode;
  /** What the button that closes the dialog says, when "Close" is not the right word (for example "Keep sending"). */
  closeLabel?: ReactNode;
}) {
  const ref = useRef<HTMLDialogElement>(null);
  const id = useId();
  const { l10n } = useLocalization();
  useEffect(() => {
    const el = ref.current;
    if (!el) return;
    const before = document.activeElement as HTMLElement | null;
    if (open && !el.open) {
      if (typeof el.showModal === "function") el.showModal();
      else el.setAttribute("open", "");
      el.querySelector<HTMLElement>("[data-autofocus], button")?.focus();
    }
    if (!open && el.open) {
      if (typeof el.close === "function") el.close();
      else el.removeAttribute("open");
    }
    return () => {
      before?.focus();
    };
  }, [open]);
  return (
    <dialog
      ref={ref}
      className="kit-dialog"
      aria-labelledby={id}
      onCancel={(e) => {
        e.preventDefault();
        onClose();
      }}
      onKeyDown={(e) => {
        if (e.key === "Escape") {
          e.preventDefault();
          onClose();
        }
      }}
    >
      {open && (
        <>
          <h2 id={id} className="kit-dialog-title">
            {title}
          </h2>
          <div className="kit-dialog-body">{children}</div>
          <div className="kit-dialog-actions">
            {actions}
            <Button onClick={onClose}>{closeLabel ?? l10n.getString("dialog-close")}</Button>
          </div>
        </>
      )}
    </dialog>
  );
}

/** A short confirmation. It stays until dismissed (no timer), and is announced politely. */
export function Toast({ children, onDismiss }: { children: ReactNode; onDismiss: () => void }) {
  const { l10n } = useLocalization();
  return (
    <div
      className="kit-toast"
      role="status"
      aria-live="polite"
      onKeyDown={(e) => {
        if (e.key === "Escape") onDismiss();
      }}
    >
      <span>{children}</span>
      <button type="button" className="kit-toast-dismiss" onClick={onDismiss}>
        {l10n.getString("toast-dismiss")}
      </button>
    </div>
  );
}

/* ---------- Icons (decorative; the words carry the meaning) ---------- */

const icon = { width: 16, height: 16, viewBox: "0 0 24 24", fill: "none", stroke: "currentColor", strokeWidth: 2, strokeLinecap: "round" as const, strokeLinejoin: "round" as const, "aria-hidden": true, focusable: false };

function CheckIcon() {
  return (
    <svg {...icon}>
      <path d="M5 12.5l4.5 4.5L19 7.5" />
    </svg>
  );
}

function AttentionIcon() {
  return (
    <svg {...icon}>
      <path d="M12 3l9.5 17h-19z" />
      <path d="M12 10v4M12 17.5h.01" />
    </svg>
  );
}

function ErrorIcon() {
  return (
    <svg {...icon}>
      <circle cx="12" cy="12" r="9" />
      <path d="M9 9l6 6M15 9l-6 6" />
    </svg>
  );
}

function SkipIcon() {
  return (
    <svg {...icon}>
      <path d="M5 12h14" />
    </svg>
  );
}

function InfoIcon() {
  return (
    <svg {...icon}>
      <circle cx="12" cy="12" r="9" />
      <path d="M12 11v5M12 8h.01" />
    </svg>
  );
}
