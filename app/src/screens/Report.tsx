import { useLocalization } from "@fluent/react";
import { useState } from "react";
import { Button, ResultChip } from "../components/kit";
import { whyMemoryKey, type ReportItem } from "../move";
import { TopStrip } from "./FrameParts";
import { Heading, EvidenceLine, ReasonText } from "./ProgressParts";
import { shownName } from "../names";

/** How many items show at once; "Show more" adds this many. A report has no cap, only pages. */
const PAGE = 100;

/**
 * R02 Your move, item by item (760 x 620): every item with its result chip, its reason (what
 * happened and what to do), and how it was checked. A copied file was either read back from the
 * drive or checked from this laptop's memory, and the two are different lines, never merged.
 * Source and destination sit behind More details. Long reports show a page at a time.
 *
 * A line's note says what else happened: "changed since the scan" (copied as it is now, so the
 * copy is newer than the plan showed), or "no longer there" (gone from the old laptop before it
 * could be sent), which says why in place of the general reason.
 */
export default function Report({ items, onBack }: { items: ReportItem[]; onBack: () => void }) {
  const { l10n } = useLocalization();
  const [shown, setShown] = useState(PAGE);
  const visible = items.slice(0, shown);
  return (
    <>
      <TopStrip onBack={onBack} labelId="outcome-eyebrow" />
      <Heading id="report-title" />
      <p className="subhead">{l10n.getString("report-body")}</p>
      {items.length === 0 ? (
        <p className="mv-empty">{l10n.getString("report-none")}</p>
      ) : (
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
              {visible.map((item) => (
                <tr key={item.id}>
                  <th scope="row">
                    <bdi dir="auto">{shownName(item.name)}</bdi>
                  </th>
                  <td>
                    <ResultChip result={item.status} />
                  </td>
                  <td>
                    {item.note && <p className="mv-note" data-note={item.note}>{l10n.getString(`report-note-${item.note}`)}</p>}
                    {item.reason && item.note !== "no-longer-there" && <ReasonText reason={item.reason} />}
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
      )}
      {items.length > visible.length && (
        <>
          <p className="mv-count">{l10n.getString("report-showing", { shown: visible.length, total: items.length })}</p>
          <Button
            onClick={() => {
              setShown(shown + PAGE);
            }}
          >
            {l10n.getString("report-show-more")}
          </Button>
        </>
      )}
    </>
  );
}
