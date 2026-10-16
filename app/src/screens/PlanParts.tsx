/** Pieces shared by the plan screens (M02 to M23): Back, folder names, sizes and numbers in the person's language, and the step icons. */
import { Localized, useLocalization } from "@fluent/react";
import type { ReactNode } from "react";
import { sizeParts, type FolderRole, type Size } from "../plan";
import "./plan.css";

/** A card with a title, like the boxes in the mockups. */
export function PlanBox({ title, children }: { title: ReactNode; children: ReactNode }) {
  return (
    <section className="plan-box">
      <h2 className="plan-box-title">{title}</h2>
      {children}
    </section>
  );
}

export function DriveIcon() {
  return (
    <svg {...icon} strokeLinecap="round" strokeLinejoin="round">
      <rect x="3" y="14" width="18" height="6" rx="2" />
      <path d="M5 14l2.5-8h9L19 14M16.5 17h.01" />
    </svg>
  );
}

export function PeopleIcon() {
  return (
    <svg {...icon} strokeLinecap="round" strokeLinejoin="round">
      <circle cx="9" cy="8" r="3" />
      <path d="M3 20c0-3.3 2.7-6 6-6s6 2.7 6 6M16 5a3 3 0 0 1 0 6M18 14c1.8.7 3 2.4 3 4.5" />
    </svg>
  );
}

/** Sizes and numbers written the way the person's language writes them (the words live in the strings). */
export function useFormat() {
  const { l10n } = useLocalization();
  const bytes = (n: number): string => {
    const { unit, n: value } = sizeParts(n);
    return l10n.getString(`plan-size-${unit}`, { n: value });
  };
  /** A measured size; part-read folders say "at least", unreadable ones say so. */
  const size = (s: Size): string => {
    if (s.kind === "unreadable") return l10n.getString("plan-size-unknown");
    return s.kind === "at-least" ? l10n.getString("plan-size-at-least", { size: bytes(s.bytes) }) : bytes(s.bytes);
  };
  /** A byte total with an "at least" when any part of it was not fully read. */
  const total = (n: number, atLeast: boolean): string => (atLeast ? l10n.getString("plan-size-at-least", { size: bytes(n) }) : bytes(n));
  const number = (n: number): string => l10n.getString("plan-number", { n });
  return { bytes, size, total, number };
}

/** The last part of a path, whichever way the system writes its separators. */
export function leaf(path: string): string {
  const parts = path.split(/[\\/]+/).filter(Boolean);
  return parts.length > 0 ? parts[parts.length - 1] : path;
}

/** A folder's name for a screen reader or a label: its role in words, or (for other folders) its own name. */
export function useFolderName() {
  const { l10n } = useLocalization();
  return (role: FolderRole, path: string): string => (role === "other" ? leaf(path) : l10n.getString(`plan-role-${role}`));
}

/** The folder's name in the screen: a known role in words, an other folder by its own name, kept in its own direction. */
export function FolderTitle({ role, path }: { role: FolderRole; path: string }): ReactNode {
  if (role === "other") return <bdi dir="auto">{leaf(path)}</bdi>;
  return (
    <Localized id={`plan-role-${role}`}>
      <span />
    </Localized>
  );
}

/** A path from the person's laptop: kept in its own direction, may break anywhere. */
export function PathText({ path, id }: { path: string; id?: string }) {
  return (
    <bdi dir="auto" id={id} className="plan-path">
      {path}
    </bdi>
  );
}

const icon = { width: 16, height: 16, viewBox: "0 0 24 24", fill: "none", stroke: "currentColor", strokeWidth: 2, "aria-hidden": true, focusable: false };

/** A step being done now. */
export function NowIcon() {
  return (
    <svg {...icon}>
      <circle cx="12" cy="12" r="5" fill="currentColor" />
    </svg>
  );
}

/** A step still to come. */
export function NextIcon() {
  return (
    <svg {...icon}>
      <circle cx="12" cy="12" r="6" />
    </svg>
  );
}

export function AttentionMark() {
  return (
    <svg {...icon} strokeLinecap="round" strokeLinejoin="round">
      <path d="M12 3l9.5 17h-19z" />
      <path d="M12 10v4M12 17.5h.01" />
    </svg>
  );
}

export function FolderIcon() {
  return (
    <svg {...icon} strokeLinecap="round" strokeLinejoin="round">
      <path d="M3 7a2 2 0 0 1 2-2h4l2 2h8a2 2 0 0 1 2 2v8a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2z" />
    </svg>
  );
}

/** Take away: a cross, for the button that removes a folder the person added (its name is on the button). */
export function RemoveIcon() {
  return (
    <svg {...icon} strokeLinecap="round" strokeLinejoin="round">
      <path d="M6 6l12 12M18 6L6 18" />
    </svg>
  );
}
