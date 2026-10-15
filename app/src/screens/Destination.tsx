import { Localized, useLocalization } from "@fluent/react";
import { useId, useRef } from "react";
import { Badge, Banner, Button } from "../components/kit";
import { ArrowForward, Check } from "../icons";
import { checkDestinations, type DestDrive, type DestGroup, type DestinationCheck, type GroupIssue } from "../plan";
import { Heading } from "./PairParts";
import { AttentionMark, FolderTitle, PathText, PlanBack, useFolderName, useFormat } from "./PlanParts";

export interface DestinationProps {
  groups: DestGroup[];
  /** The new laptop's drives, including one that was chosen and is no longer connected. */
  drives: DestDrive[];
  onSetDestination: (groupId: string, driveId: string) => void;
  /** One usual place for everything. */
  onSetAll: (driveId: string) => void;
  /** Open the folder picker on the new laptop for one group. */
  onChooseFolder: (groupId: string) => void;
  onContinue: () => void;
  onBack: () => void;
}

const CHOOSE = "__choose__";

type Status = "none" | "missing" | "readonly" | "format" | "space" | "ready";

/** What the row says: the first thing in the way, in the order the person would fix it. */
function statusOf(g: DestGroup, check: DestinationCheck): Status {
  const issues: GroupIssue[] = check.groupIssues[g.id] ?? [];
  if (issues.includes("no-destination")) return "none";
  if (issues.includes("drive-missing")) return "missing";
  if (issues.includes("drive-read-only")) return "readonly";
  if (issues.includes("file-too-big")) return "format";
  if (g.to && check.loads.some((l) => l.drive.id === g.to?.drive && l.shortBytes > 0)) return "space";
  return "ready";
}

/**
 * M05 Give every file a home. Each chosen folder goes to a place on the new laptop the person controls.
 * Nothing continues while a place is missing, not writable, too small or not chosen; the screen says which
 * and how much, and nothing is guessed.
 */
export default function Destination({ groups, drives, onSetDestination, onSetAll, onChooseFolder, onContinue, onBack }: DestinationProps) {
  const { l10n } = useLocalization();
  const fmt = useFormat();
  const nameOf = useFolderName();
  const check = checkDestinations(groups, drives);
  const selects = useRef(new Map<string, HTMLSelectElement>());
  const defaultId = useId();
  const connected = drives.filter((d) => d.connected);
  const drivesAre = new Set(groups.map((g) => g.to?.drive ?? ""));
  const common = drivesAre.size === 1 ? [...drivesAre][0] : "";
  const driveName = (id: string) => drives.find((d) => d.id === id)?.name ?? id;

  // The messages, one per cause rather than one per row, each with the row to change.
  const messages: { key: string; text: string; groupId: string }[] = [];
  const seen = new Set<string>();
  for (const g of groups) {
    const issues = check.groupIssues[g.id] ?? [];
    const name = nameOf(g.role, g.path);
    const once = (key: string, text: string) => {
      if (seen.has(key)) return;
      seen.add(key);
      messages.push({ key, text, groupId: g.id });
    };
    for (const issue of issues) {
      const drive = g.to ? driveName(g.to.drive) : "";
      if (issue === "no-destination") once(`none-${g.id}`, l10n.getString("dest-none", { name }));
      if (issue === "drive-missing") once(`missing-${g.to?.drive ?? ""}`, l10n.getString("dest-missing", { drive }));
      if (issue === "drive-read-only") once(`readonly-${g.to?.drive ?? ""}`, l10n.getString("dest-readonly", { place: drive }));
      if (issue === "file-too-big") once(`format-${g.to?.drive ?? ""}`, l10n.getString("dest-format", { place: drive }));
    }
  }
  for (const load of check.loads) {
    if (load.shortBytes === 0) continue;
    const first = groups.find((g) => g.to?.drive === load.drive.id);
    messages.push({
      key: `space-${load.drive.id}`,
      text: l10n.getString("dest-space", { place: load.drive.name, free: fmt.bytes(load.drive.freeBytes), needed: fmt.total(load.needBytes, load.needAtLeast) }),
      groupId: first?.id ?? "",
    });
  }

  return (
    <div className="plan-screen">
      <PlanBack onBack={onBack} eyebrow="dest-eyebrow" />
      <Heading id="dest-title" />
      <Localized id="dest-body">
        <p className="subhead" />
      </Localized>

      {/* Explicit roles keep the table a table when a narrow window stacks its cells. */}
      <div className="plan-table" role="table" aria-label={l10n.getString("dest-title")}>
        <div className="plan-table-head" role="row">
          <span role="columnheader">{l10n.getString("dest-col-chosen")}</span>
          <span role="columnheader">{l10n.getString("dest-col-place")}</span>
          <span role="columnheader">{l10n.getString("dest-col-placement")}</span>
          <span role="columnheader">{l10n.getString("dest-col-status")}</span>
        </div>
        {groups.map((g) => {
          const name = nameOf(g.role, g.path);
          const status = statusOf(g, check);
          const current = g.to ? drives.find((d) => d.id === g.to?.drive) : undefined;
          return (
            <div className="plan-table-row" role="row" key={g.id}>
              <span role="cell" className="plan-cell-name">
                <span className="plan-folder-name">
                  <FolderTitle role={g.role} path={g.path} />
                </span>
                <small className="kit-hint">{fmt.size(g.size)}</small>
                <small className="kit-hint plan-path-line">
                  <PathText path={g.path} />
                </small>
              </span>
              <span role="cell" className="plan-cell-place">
                <Localized id="dest-select" vars={{ name }} attrs={{ "aria-label": true }}>
                  <select
                    className="kit-select"
                    value={g.to?.drive ?? ""}
                    ref={(el) => {
                      if (el) selects.current.set(g.id, el);
                      else selects.current.delete(g.id);
                    }}
                    onChange={(e) => {
                      if (e.target.value === CHOOSE) onChooseFolder(g.id);
                      else onSetDestination(g.id, e.target.value);
                    }}
                  >
                    {!g.to && (
                      <option value="" disabled>
                        —
                      </option>
                    )}
                    {drives
                      .filter((d) => d.connected || d.id === g.to?.drive)
                      .map((d) => (
                        <option key={d.id} value={d.id}>
                          {!d.connected
                            ? d.name
                            : d.isSystem
                              ? l10n.getString("dest-option-folder", { folder: g.to?.drive === d.id && g.to.folder ? g.to.folder : name })
                              : l10n.getString("dest-option-drive", { drive: d.name })}
                        </option>
                      ))}
                    <option value={CHOOSE}>{l10n.getString("dest-option-choose")}</option>
                  </select>
                </Localized>
                {g.to && current && g.to.folder && (
                  <small className="kit-hint">
                    <bdi dir="auto">{l10n.getString("plan-place", { folder: g.to.folder, drive: current.name })}</bdi>
                  </small>
                )}
              </span>
              <span role="cell" className="plan-cell-placement">
                {current?.connected ? l10n.getString(current.removable ? "dest-placement-external" : "dest-placement-local") : "—"}
              </span>
              <span role="cell" className="plan-cell-status">
                <StatusBadge status={status} />
              </span>
            </div>
          );
        })}
      </div>

      <div className="plan-default">
        <label htmlFor={defaultId}>{l10n.getString("dest-default-label")}</label>
        <select
          id={defaultId}
          className="kit-select"
          value={common}
          onChange={(e) => {
            onSetAll(e.target.value);
          }}
        >
          {common === "" && (
            <option value="" disabled>
              —
            </option>
          )}
          {connected.map((d) => (
            <option key={d.id} value={d.id}>
              {d.name}
            </option>
          ))}
        </select>
      </div>

      <div className="plan-notice" role="status">
        {check.ok ? (
          <Banner tone="ready">
            <p>{l10n.getString("dest-all-ready")}</p>
          </Banner>
        ) : (
          <Banner tone="attention">
            {messages.map((m) => (
              <div className="plan-message" key={m.key}>
                <p id={`msg-${m.key}`}>{m.text}</p>
                <Button
                  variant="quiet"
                  aria-describedby={`msg-${m.key}`}
                  onClick={() => {
                    selects.current.get(m.groupId)?.focus();
                  }}
                >
                  {l10n.getString("dest-change")}
                </Button>
              </div>
            ))}
          </Banner>
        )}
      </div>

      <div className="plan-actions">
        <Button variant="primary" icon={<ArrowForward />} disabled={!check.ok} onClick={onContinue}>
          {l10n.getString("dest-continue")}
        </Button>
      </div>
    </div>
  );
}

function StatusBadge({ status }: { status: Status }) {
  const { l10n } = useLocalization();
  if (status === "ready")
    return (
      <Badge tone="good" icon={<Check />}>
        {l10n.getString("dest-status-ready")}
      </Badge>
    );
  const key = { none: "dest-status-none", missing: "dest-status-missing", readonly: "dest-status-readonly", format: "dest-status-format", space: "dest-status-space" }[status];
  return (
    <Badge tone="warn" icon={<AttentionMark />}>
      {l10n.getString(key)}
    </Badge>
  );
}
