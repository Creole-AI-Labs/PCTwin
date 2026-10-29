import { useLocalization } from "@fluent/react";
import { useState } from "react";
import { Button, ResultChip } from "../components/kit";
import { FILE_KINDS, formatBytes, pageLocale, whyMemoryKey, type FileKind, type ReportItem } from "../move";
import { TopStrip } from "./FrameParts";
import { Heading, EvidenceLine, ReasonText } from "./ProgressParts";
import { shownName } from "../names";

/** How many items an open group shows at once; "Show more" adds this many. A report has no cap, only pages. */
const PAGE = 100;

/** Results that ended as asked: copied, reinstalled or converted. */
const DONE = new Set(["copied", "reinstalled", "converted"]);

/** The folder a line came from, inside its group (the source's parts between the group and the name). */
function folderOf(item: ReportItem): string {
  const parts = item.source.split(/[\\/]+/).filter(Boolean);
  return parts.slice(1, -1).join("/");
}

/**
 * R02 Your move, by kind (decided 29 October 2026, after the person saw a list of every file and
 * said nobody would understand it): like a phone's storage page. Anything that needs the person
 * comes first, in its own list, never folded away. Then one line per kind of file (photos, videos,
 * music, documents, other files) with how many, how much, and how it went in words, under a bar
 * that shows their sizes. A kind opens into its files, by folder, a page at a time; each file keeps
 * its result chip, its reason, and how it was checked, with where it came from and went behind
 * More details. A copied file was either read back from the drive or checked from this laptop's
 * memory, and the two are different lines, never merged.
 *
 * A line's note says what else happened: "changed since the scan" (copied as it is now, so the
 * copy is newer than the plan showed), or "no longer there" (gone from the old laptop before it
 * could be sent), which says why in place of the general reason.
 */
export default function Report({ items, onBack }: { items: ReportItem[]; onBack: () => void }) {
  const { l10n } = useLocalization();
  const [open, setOpen] = useState<FileKind | null>(null);
  const [shown, setShown] = useState(PAGE);
  const needs = items.filter((i) => i.status === "failed" || i.status === "needs-action");
  const groups = FILE_KINDS.map((kind) => {
    const lines = items.filter((i) => (i.kind ?? "other") === kind);
    const asked = lines.filter((i) => i.status !== "skipped-by-user");
    return {
      kind,
      lines: [...lines].sort((a, b) => folderOf(a).localeCompare(folderOf(b)) || a.name.localeCompare(b.name)),
      bytes: lines.reduce((n, i) => n + (i.bytes ?? 0), 0),
      done: asked.filter((i) => DONE.has(i.status)).length,
      asked: asked.length,
      left: lines.length - asked.length,
    };
  }).filter((g) => g.lines.length > 0);
  const total = groups.reduce((n, g) => n + g.bytes, 0);
  const name = (kind: FileKind) => l10n.getString(`report-group-${kind}`);

  const table = (lines: ReportItem[]) => (
    <div className="mv-table-wrap">
      <table className="mv-table mv-report">
        <thead>
          <tr>
            <th scope="col">{l10n.getString("report-col-item")}</th>
            <th scope="col">{l10n.getString("report-col-result")}</th>
            <th scope="col">{l10n.getString("report-col-details")}</th>
          </tr>
        </thead>
        <tbody>
          {lines.map((item) => (
            <tr key={item.id}>
              <th scope="row">
                <bdi dir="auto">{shownName(item.name)}</bdi>
                {folderOf(item) !== "" && (
                  <span className="rp-folder">
                    <bdi dir="auto">{shownName(folderOf(item))}</bdi>
                  </span>
                )}
              </th>
              <td>
                <ResultChip result={item.status} />
              </td>
              <td>
                {item.note && <p className="mv-note" data-note={item.note}>{l10n.getString(`report-note-${item.note}`)}</p>}
                {item.reason && item.note !== "no-longer-there" && <ReasonText reason={item.reason} />}
                {item.openIn && (
                  // The app that held the file open on the old laptop, by its own name.
                  <p className="mv-note" data-open-in="">
                    {l10n.getString("report-open-in", { app: shownName(item.openIn) })}
                  </p>
                )}
                <EvidenceLine outcome={item} />
                <details className="mv-details">
                  <summary>{l10n.getString("report-more-details")}</summary>
                  <p dir="auto">{l10n.getString("report-from", { path: item.source })}</p>
                  {item.destination && <p dir="auto">{l10n.getString("report-to", { path: item.destination })}</p>}
                  {item.status === "copied" && item.evidence.from === "memory" && (
                    <>
                      <p>{l10n.getString("evidence-memory-explain")}</p>
                      <p data-why={item.evidence.why}>{l10n.getString(whyMemoryKey(item.evidence.why))}</p>
                    </>
                  )}
                </details>
              </td>
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  );

  return (
    <>
      <TopStrip onBack={onBack} labelId="outcome-eyebrow" />
      <Heading id="report-title" />
      <p className="subhead">{l10n.getString("report-body")}</p>
      {items.length === 0 ? (
        <p className="mv-empty">{l10n.getString("report-none")}</p>
      ) : (
        <>
          {needs.length > 0 && (
            <section className="rp-needs" aria-labelledby="rp-needs-title">
              <h2 id="rp-needs-title" className="rp-title">
                {l10n.getString("report-needs-title", { count: needs.length })}
              </h2>
              {table(needs)}
            </section>
          )}
          {total > 0 && (
            // The sizes at a glance; each part is named in words in the list under it.
            <div className="rp-bar" aria-hidden="true">
              {groups.map((g) => (
                <span key={g.kind} className={`rp-part rp-${g.kind}`} style={{ width: `${String((100 * g.bytes) / total)}%` }} />
              ))}
            </div>
          )}
          <ul className="rp-groups">
            {groups.map((g) => {
              const isOpen = open === g.kind;
              return (
                <li key={g.kind} className="rp-group">
                  <button
                    type="button"
                    className="rp-head"
                    aria-expanded={isOpen}
                    onClick={() => {
                      setOpen(isOpen ? null : g.kind);
                      setShown(PAGE);
                    }}
                  >
                    <span className={`rp-swatch rp-${g.kind}`} aria-hidden="true" />
                    <span className="rp-name">{name(g.kind)}</span>
                    <span className="rp-size">{l10n.getString("report-group-size", { files: g.lines.length, size: formatBytes(g.bytes, pageLocale()) })}</span>
                    <span className="rp-state">
                      {g.asked > 0 && (g.done === g.asked ? l10n.getString("report-group-done") : l10n.getString("report-group-some", { done: g.done, total: g.asked }))}
                      {g.left > 0 && <span className="rp-left">{l10n.getString("report-group-left", { count: g.left })}</span>}
                    </span>
                  </button>
                  {isOpen && (
                    <div className="rp-body">
                      {table(g.lines.slice(0, shown))}
                      {g.lines.length > shown && (
                        <>
                          <p className="mv-count">{l10n.getString("report-showing", { shown, total: g.lines.length })}</p>
                          <Button
                            onClick={() => {
                              setShown(shown + PAGE);
                            }}
                          >
                            {l10n.getString("report-show-more")}
                          </Button>
                        </>
                      )}
                    </div>
                  )}
                </li>
              );
            })}
          </ul>
        </>
      )}
    </>
  );
}
