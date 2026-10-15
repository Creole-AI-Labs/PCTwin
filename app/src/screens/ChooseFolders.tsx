import { Localized, useLocalization } from "@fluent/react";
import { useId, useState } from "react";
import { Badge, Button } from "../components/kit";
import { ArrowForward, Info } from "../icons";
import { chosenTotals, type FolderChoice } from "../plan";
import { Heading } from "./PairParts";
import { AttentionMark, FolderIcon, RemoveIcon, FolderTitle, PathText, PlanBack, leaf, useFolderName, useFormat } from "./PlanParts";

export interface ChooseFoldersProps {
  folders: FolderChoice[];
  onToggle: (id: string, selected: boolean) => void;
  /** Open the system folder picker; the folder comes back as a new entry marked `custom`. */
  onAddFolder: () => void;
  /** Remove a folder the person added. */
  onRemove: (id: string) => void;
  /** Ask the system for permission to read a protected folder. */
  onAllowAccess: (id: string) => void;
  onContinue: () => void;
  onBack: () => void;
}

/**
 * M04 Keep what matters to you. A folder is ticked to move; there is no cap on size or count. A protected
 * folder cannot be ticked until the person allows access, and is never counted until then.
 */
export default function ChooseFolders({ folders, onToggle, onAddFolder, onRemove, onAllowAccess, onContinue, onBack }: ChooseFoldersProps) {
  const { l10n } = useLocalization();
  const fmt = useFormat();
  const nameOf = useFolderName();
  const [query, setQuery] = useState("");
  const searchId = useId();
  const totals = chosenTotals(folders);
  // Two folders with the same name (a work Documents and a personal one) are told apart by their full place.
  const counts = new Map<string, number>();
  for (const f of folders) counts.set(nameOf(f.role, f.path), (counts.get(nameOf(f.role, f.path)) ?? 0) + 1);
  const nameFor = (f: FolderChoice) => (counts.get(nameOf(f.role, f.path)) === 1 ? nameOf(f.role, f.path) : f.path);
  const shown = folders.filter((f) => {
    const q = query.trim().toLowerCase();
    return q === "" || nameOf(f.role, f.path).toLowerCase().includes(q) || leaf(f.path).toLowerCase().includes(q);
  });

  return (
    <div className="plan-screen">
      <PlanBack onBack={onBack} eyebrow="files-eyebrow" />
      <Heading id="files-title" />
      <Localized id="files-body">
        <p className="subhead" />
      </Localized>

      <div className="kit-field plan-search">
        <label htmlFor={searchId}>{l10n.getString("files-search-label")}</label>
        <Localized id="files-search-placeholder" attrs={{ placeholder: true }}>
          <input id={searchId} type="search" value={query} autoComplete="off" spellCheck={false} onChange={(e) => { setQuery(e.target.value); }} />
        </Localized>
      </div>

      <ul className="plan-folders">
        {shown.map((f) => (
          <FolderRow key={f.id} folder={f} name={nameFor(f)} onToggle={onToggle} onRemove={onRemove} onAllowAccess={onAllowAccess} />
        ))}
      </ul>
      {shown.length === 0 && (
        <Localized id="files-no-match">
          <p className="plan-quiet" />
        </Localized>
      )}

      <div className="plan-add">
        <Button onClick={onAddFolder} icon={<FolderIcon />}>
          {l10n.getString("files-add")}
        </Button>
      </div>

      <div className="note">
        <Info />
        <Localized id="files-no-limit">
          <p className="note-text" />
        </Localized>
      </div>

      <p className="plan-total" role="status" id="files-total">
        {totals.count > 0
          ? l10n.getString("files-total", { count: totals.count, size: fmt.total(totals.bytes, totals.atLeast) })
          : l10n.getString("files-none")}
      </p>
      <div className="plan-actions">
        <Button variant="primary" icon={<ArrowForward />} disabled={totals.count === 0} aria-describedby="files-total" onClick={onContinue}>
          {l10n.getString("files-continue")}
        </Button>
      </div>
    </div>
  );
}

function FolderRow({
  folder,
  name,
  onToggle,
  onRemove,
  onAllowAccess,
}: {
  folder: FolderChoice;
  /** The name a screen reader hears for this folder's controls. */
  name: string;
  onToggle: (id: string, selected: boolean) => void;
  onRemove: (id: string) => void;
  onAllowAccess: (id: string) => void;
}) {
  const { l10n } = useLocalization();
  const fmt = useFormat();
  const id = useId();
  const protectedFolder = folder.access === "needs-permission";
  const detail =
    folder.size.kind === "unreadable"
      ? null
      : l10n.getString("files-folder-detail", { size: fmt.size(folder.size), count: folder.size.files });
  return (
    <li className="plan-folder">
      <div className="plan-folder-main">
        <Localized id="files-select" vars={{ name }} attrs={{ "aria-label": true }}>
          <input
            type="checkbox"
            className="plan-check"
            checked={folder.selected && !protectedFolder}
            disabled={protectedFolder}
            aria-describedby={`${id}-detail`}
            onChange={(e) => {
              onToggle(folder.id, e.target.checked);
            }}
          />
        </Localized>
        <span className="plan-folder-text" id={`${id}-detail`}>
          <span className="plan-folder-name">
            <FolderTitle role={folder.role} path={folder.path} />
          </span>
          {detail && <small className="kit-hint">{detail}</small>}
          <small className="kit-hint plan-path-line">
            <PathText path={folder.path} />
          </small>
          {folder.onlineOnlyFiles ? <small className="kit-hint">{l10n.getString("files-online-only", { count: folder.onlineOnlyFiles })}</small> : null}
          {protectedFolder && <small className="kit-hint">{l10n.getString("files-protected-body")}</small>}
          {folder.managedBy === "organisation" && <small className="kit-hint">{l10n.getString("files-work-body")}</small>}
        </span>
      </div>
      <div className="plan-folder-side">
        {protectedFolder && (
          <>
            <Badge tone="warn" icon={<AttentionMark />}>
              {l10n.getString("files-status-protected")}
            </Badge>
            <Button
              variant="quiet"
              aria-describedby={`${id}-detail`}
              onClick={() => {
                onAllowAccess(folder.id);
              }}
            >
              {l10n.getString("files-allow")}
            </Button>
          </>
        )}
        {folder.managedBy === "organisation" && !protectedFolder && <Badge icon={<Info />}>{l10n.getString("files-status-work")}</Badge>}
        {folder.custom && (
          <>
            <Badge icon={<FolderIcon />}>{l10n.getString("files-status-added")}</Badge>
            <Localized id="files-remove" vars={{ name }} attrs={{ "aria-label": true }}>
              <button
                type="button"
                className="kit-button kit-button-quiet"
                onClick={() => {
                  onRemove(folder.id);
                }}
              >
                <RemoveIcon />
              </button>
            </Localized>
          </>
        )}
      </div>
    </li>
  );
}
