import { useLocalization } from "@fluent/react";
import { useState } from "react";
import { Banner, Button, ResultChip } from "../components/kit";
import { ITEM_STATUSES, type ItemStatus, type ReportItem } from "../move";
import { Heading, EvidenceLine, ReasonText, Topline, isMemory } from "./ProgressParts";

/** How many items show at once; "Show more" adds this many. A report has no cap, only pages. */
const PAGE = 100;

type Filter = "all" | ItemStatus;

/**
 * R02 Your move, item by item (760 x 620): every item with its result chip, its reason (what
 * happened and what to do), and how it was checked. A copied file was either read back from the
 * drive or checked from this laptop's memory, and the two are different lines, never merged.
 * Source and destination sit behind More details. Filter by result; long reports show a page at a time.
 */
export default function Report({ items, onBack }: { items: ReportItem[]; onBack: () => void }) {
  const { l10n } = useLocalization();
  const [filter, setFilter] = useState<Filter>("all");
  const [shown, setShown] = useState(PAGE);
  const counts = ITEM_STATUSES.reduce<Record<ItemStatus, number>>((acc, s) => ({ ...acc, [s]: items.filter((i) => i.status === s).length }), {} as Record<ItemStatus, number>);
  const memory = items.filter((i) => i.status === "copied" && isMemory(i.evidence)).length;
  const matching = filter === "all" ? items : items.filter((i) => i.status === filter);
  const visible = matching.slice(0, shown);
  const choose = (next: Filter) => {
    setFilter(next);
    setShown(PAGE);
  };
  return (
    <>
      <Topline onBack={onBack} eyebrow="outcome-eyebrow" />
      <Heading id="report-title" />
      <p className="subhead">{l10n.getString("report-body")}</p>
      {memory > 0 && (
        <Banner tone="info">
          <p>{l10n.getString("report-memory-summary", { count: memory })}</p>
        </Banner>
      )}
      <div className="mv-filter" role="group" aria-label={l10n.getString("report-filter-label")}>
        <button type="button" className="mv-filter-button" aria-pressed={filter === "all"} onClick={() => { choose("all"); }}>
          {l10n.getString("report-filter-all")} <span className="mv-filter-count">{items.length}</span>
        </button>
        {ITEM_STATUSES.map((s) => (
          <button key={s} type="button" className="mv-filter-button" aria-pressed={filter === s} onClick={() => { choose(s); }}>
            {l10n.getString(`status-${s}`)} <span className="mv-filter-count">{counts[s]}</span>
          </button>
        ))}
      </div>
      <p role="status" className="mv-count">
        {l10n.getString("report-count", { count: matching.length })}
      </p>
      {matching.length === 0 ? (
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
                    <bdi dir="auto">{item.name}</bdi>
                  </th>
                  <td>
                    <ResultChip result={item.status} />
                  </td>
                  <td>
                    {item.reason && <ReasonText reason={item.reason} />}
                    <EvidenceLine outcome={item} />
                    <details className="mv-details">
                      <summary>{l10n.getString("report-more-details")}</summary>
                      <p dir="auto">{l10n.getString("report-from", { path: item.source })}</p>
                      {item.destination && <p dir="auto">{l10n.getString("report-to", { path: item.destination })}</p>}
                      {item.status === "copied" && item.evidence.from === "memory" && <p>{l10n.getString("evidence-memory-explain")}</p>}
                    </details>
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      )}
      {matching.length > visible.length && (
        <>
          <p className="mv-count">{l10n.getString("report-showing", { shown: visible.length, total: matching.length })}</p>
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
