/**
 * Pieces shared by the Quick Send screens, in the arrangement the 14 October mockups use: the Back
 * strip with "Quick Send" at the far side, the route between two laptops, the tally bar, rows in a
 * card with an icon tile, and the shared top strip and footer (FrameParts, FrameFooter).
 */
import { Localized, useLocalization } from "@fluent/react";
import type { ReactNode } from "react";
import { Badge, Row, RowList } from "../components/kit";
import { Check, Laptop } from "../icons";
import { labelText, type Label } from "../pairing";
import { fileName, useBytes } from "../quicksend";
import { TopStrip } from "./FrameParts";
import "./quicksend.css";

export { Heading } from "./PairParts";

/** The text of an attribute of a message (a message with only attributes has no value to fetch). */
export function useAttribute(): (id: string, attribute: string, vars?: Record<string, string | number>) => string {
  const { l10n } = useLocalization();
  return (id, attribute, vars) => {
    const bundle = l10n.getBundle(id);
    const pattern = bundle?.getMessage(id)?.attributes[attribute];
    return bundle && pattern ? bundle.formatPattern(pattern, vars ?? null) : id;
  };
}

/** A folder, drawn like the app's other line icons (decorative). */
export function Folder({ size = 20 }: { size?: number }) {
  return (
    <svg width={size} height={size} viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth={1.8} strokeLinecap="round" strokeLinejoin="round" aria-hidden="true" focusable="false">
      <path d="M3 7.5A1.5 1.5 0 0 1 4.5 6H9l2 2.5h8.5A1.5 1.5 0 0 1 21 10v7.5a1.5 1.5 0 0 1-1.5 1.5h-15A1.5 1.5 0 0 1 3 17.5z" />
    </svg>
  );
}

/** The icon tile at the start of a card or row, as the choice cards have it. */
export function Tile({ children }: { children: ReactNode }) {
  return <span className="qs-tile">{children}</span>;
}

/** Back on one side and "Quick Send" (or "Receiving on this device") on the other: the shared top strip. */
export function QsStrip({ onBack, receiving = false }: { onBack?: () => void; receiving?: boolean }) {
  return <TopStrip onBack={onBack} labelId={receiving ? "qs-eyebrow-receiving" : "qs-eyebrow"} />;
}

/** Who sends to whom: the two laptops, one under the other, each with its role. */
export function Route({ from, to }: { from: Label; to: Label }) {
  const { l10n } = useLocalization();
  const attribute = useAttribute();
  const a = labelText(l10n, from);
  const b = labelText(l10n, to);
  return (
    <div className="qs-route" role="group" aria-label={attribute("qs-route", "aria-label", { from: a, to: b })}>
      {[
        { who: a, role: "qs-route-sender" },
        { who: b, role: "qs-route-receiver" },
      ].map(({ who, role }) => (
        <div key={role} className="qs-route-row">
          <Tile>
            <Laptop className="qs-small-icon" />
          </Tile>
          <span className="qs-route-text">
            <strong>
              <bdi dir="auto">{who}</bdi>
            </strong>
            <Localized id={role}>
              <small />
            </Localized>
          </span>
        </div>
      ))}
    </div>
  );
}

/** "3 items, 24.8 MB" in a bar with "Local send" at the end, as the mockups' tally has it. */
export function TallyBar({ count, size, live = false }: { count: number; size: string; live?: boolean }) {
  const { l10n } = useLocalization();
  return (
    <div className="qs-tallybar" role={live ? "status" : undefined}>
      <strong>{l10n.getString("qs-tally", { count, size })}</strong>
      <Badge>
        <Localized id="qs-tally-badge">
          <span />
        </Localized>
      </Badge>
    </div>
  );
}

/** What is wrong with a file that is no longer as it was when the person looked. */
export const ISSUE_TEXT = { changed: "qs-check-changed", missing: "qs-check-missing", unreadable: "qs-check-unreadable" } as const;

/** A list of rows in a card that wraps its end items under the text when the window is narrow or the text is large. */
export function QsRowList({ children }: { children: ReactNode }) {
  return (
    <div className="qs-rows">
      <RowList>{children}</RowList>
    </div>
  );
}

/** The buttons of a screen that are not the main action (they sit above the foot bar), as the mockups' "inline actions". */
export function QsActions({ children }: { children: ReactNode }) {
  return <div className="qs-actions">{children}</div>;
}

/** Text that may be in any script or direction (file names, laptop names): isolated so it cannot scramble the words around it. */
export function Name({ children }: { children: string }) {
  return <bdi dir="auto">{children}</bdi>;
}

/** The folder part of a path in the send ("Holiday/beach.jpg" -> "Holiday"), or null at the top. */
export function folderOf(path: string): string | null {
  const at = path.lastIndexOf("/");
  return at > 0 ? path.slice(0, at) : null;
}

/** Offered files as rows: icon tile, name, folder and size, and optionally a word saying it is included. */
export function FileRows({ files, limit, included = false }: { files: readonly { path: string; size: number }[]; limit?: number; included?: boolean }) {
  const bytes = useBytes();
  const shown = limit === undefined ? files : files.slice(0, limit);
  return (
    <QsRowList>
      {shown.map((f) => {
        const folder = folderOf(f.path);
        return (
          <Row
            key={f.path}
            icon={
              <Tile>
                <Folder size={18} />
              </Tile>
            }
            title={<Name>{fileName(f.path)}</Name>}
            detail={
              <>
                <bdi>{bytes(f.size)}</bdi>
                {folder && (
                  <>
                    {" · "}
                    <Name>{folder}</Name>
                  </>
                )}
              </>
            }
            end={
              included ? (
                <Badge tone="good" icon={<Check />}>
                  <Localized id="qs-check-included">
                    <span />
                  </Localized>
                </Badge>
              ) : undefined
            }
          />
        );
      })}
    </QsRowList>
  );
}
