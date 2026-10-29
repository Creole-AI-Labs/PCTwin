import { useLocalization } from "@fluent/react";
import { useState, type ReactNode } from "react";
import { Button, ResultChip, Row, RowList } from "../components/kit";
import type { ChangeKind, HereLine, Rebuilt, RebuiltLine } from "../fullmove";
import { FILE_KINDS, formatBytes, pageLocale, whyMemoryKey, type FileKind, type PersonOutcome, type ReportItem } from "../move";
import { TopStrip } from "./FrameParts";
import { lineDetail, TitleText } from "./FullMoveParts";
import { Name } from "./GuardParts";
import { Heading, EvidenceLine, ReasonText } from "./ProgressParts";
import { shownName } from "../names";

/** What a laptop holds beyond files (decided 29 October 2026), in the order the report lists them. */
const BEYOND = ["apps", "settings", "special", "people"] as const;
type Beyond = (typeof BEYOND)[number];
const KIND_OF: Record<Exclude<Beyond, "people">, ChangeKind> = { apps: "app", settings: "settings", special: "special" };

/** A line of the full move that needs the person: it failed, or it waits for them. */
const needsYou = (l: RebuiltLine) => l.status === "failed" || l.status === "needs-action";
const personNeedsYou = (p: PersonOutcome) => p.failed + p.needsAction > 0;

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
 * R02 What moved (decided 29 October 2026, after the person saw a list of every file and
 * said nobody would understand it): like a phone's storage page. Anything that needs the person
 * comes first, in its own list, never folded away. Then one line per kind of file (photos, videos,
 * music, documents, other files) with how many, how much, and how it went in words, under a bar
 * that shows their sizes. Selecting a group opens its own page (Back returns to the overview): its
 * files, by folder, a page at a time; each file keeps
 * its result chip, its reason, and how it was checked, with where it came from and went behind
 * More details. A copied file was either read back from the drive or checked from this laptop's
 * memory, and the two are different lines, never merged.
 *
 * A laptop holds more than a phone (decided 29 October 2026): after the files come the apps (with
 * the apps already on this laptop, left as they are, as their own plain line), the settings, the
 * app data and other things (Wi-Fi, fonts, browser data and the like), and the other people
 * moved, each one line that opens its own page. Whatever of them needs the person is also in
 * "Needs you" at the top. When the engine sent fewer lines than there are, the report says how
 * many more, never a shorter list as if it were all.
 *
 * A line's note says what else happened: "changed since the scan" (copied as it is now, so the
 * copy is newer than the plan showed), or "no longer there" (gone from the old laptop before it
 * could be sent), which says why in place of the general reason.
 */
export default function Report({
  items,
  rebuilt,
  people = [],
  onBack,
}: {
  items: ReportItem[];
  /** The full move's apps, settings and app data, when the move had them. */
  rebuilt?: Rebuilt;
  /** The other people moved. */
  people?: PersonOutcome[];
  onBack: () => void;
}) {
  const { l10n } = useLocalization();
  const [open, setOpen] = useState<FileKind | Beyond | null>(null);
  const [shown, setShown] = useState(PAGE);
  const needs = items.filter((i) => i.status === "failed" || i.status === "needs-action");
  const lines = rebuilt?.lines ?? [];
  const needsLines = lines.filter(needsYou);
  const needsPeople = people.filter(personNeedsYou);
  const needsCount = needs.length + needsLines.length + needsPeople.length;
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

  /** One line of the full move, as the outcome shows it, with its result in words. */
  const rebuiltRows = (shown: RebuiltLine[]) => (
    <RowList>
      {shown.map((l, i) => (
        <Row key={i} title={<TitleText title={l.title} />} detail={lineDetail(l, l10n) || undefined} end={<ResultChip result={l.status} />} />
      ))}
    </RowList>
  );
  const personRows = (shown: PersonOutcome[]) => (
    <RowList>
      {shown.map((p) => (
        <Row
          key={p.name}
          title={<Name>{p.name}</Name>}
          detail={[
            l10n.getString("outcome-person-copied", { count: p.copied }),
            personNeedsYou(p) ? l10n.getString("outcome-person-not-done", { count: p.failed + p.needsAction }) : "",
            p.atSignIn ? l10n.getString("outcome-person-at-sign-in", { name: p.name }) : "",
          ]
            .filter(Boolean)
            .join(" ")}
        />
      ))}
    </RowList>
  );
  const hereRows = (here: HereLine[], more: number) => (
    <>
      <p className="rp-here">{l10n.getString("report-here", { count: here.length + more })}</p>
      <RowList>
        {here.map((h, i) => (
          <Row
            key={i}
            title={<Name>{h.name}</Name>}
            detail={h.older === null ? l10n.getString("fullmove-outcome-here") : l10n.getString("fullmove-outcome-here-older", { version: shownName(h.older) })}
          />
        ))}
      </RowList>
      {more > 0 && <p className="mv-count">{l10n.getString("fullmove-more", { count: more })}</p>}
    </>
  );

  /** The groups beyond files: one line each, with how many and how it went in words. */
  const beyond = BEYOND.map((key) => {
    if (key === "people") {
      if (people.length === 0) return null;
      const done = people.filter((p) => !personNeedsYou(p)).length;
      return { key, count: l10n.getString("report-count-people", { count: people.length }), state: stateWords(done, people.length, 0), body: personRows(people) };
    }
    const mine = lines.filter((l) => l.kind === KIND_OF[key]);
    const here = key === "apps" ? (rebuilt?.here ?? []) : [];
    const hereMore = key === "apps" ? (rebuilt?.hereMore ?? 0) : 0;
    if (mine.length === 0 && here.length === 0 && hereMore === 0) return null;
    const asked = mine.filter((l) => l.status !== "skipped-by-user");
    const done = asked.filter((l) => DONE.has(l.status)).length;
    return {
      key,
      count: l10n.getString(`report-count-${key}`, { count: mine.length }),
      state: stateWords(done, asked.length, mine.length - asked.length),
      body: (
        <>
          {mine.length > 0 && rebuiltRows(mine)}
          {(here.length > 0 || hereMore > 0) && hereRows(here, hereMore)}
        </>
      ),
    };
  }).filter((g) => g !== null);

  function stateWords(done: number, asked: number, left: number): ReactNode {
    return (
      <>
        {asked > 0 && (done === asked ? l10n.getString("report-beyond-done") : l10n.getString("report-beyond-some", { done, total: asked }))}
        {left > 0 && <span className="rp-left">{l10n.getString("report-beyond-left", { count: left })}</span>}
      </>
    );
  }

  /** A group's line on the overview: a button that opens the group's own page. */
  const groupHead = (key: FileKind | Beyond, label: string, size: string, state: ReactNode, swatch: boolean) => (
    <button
      type="button"
      className="rp-head"
      onClick={() => {
        setOpen(key);
        setShown(PAGE);
      }}
    >
      <span className={swatch ? `rp-swatch rp-${key}` : "rp-swatch rp-beyond"} aria-hidden="true" />
      <span className="rp-name">{label}</span>
      <span className="rp-size">{size}</span>
      <span className="rp-state">{state}</span>
    </button>
  );

  const fileState = (g: (typeof groups)[number]) => (
    <>
      {g.asked > 0 && (g.done === g.asked ? l10n.getString("report-group-done") : l10n.getString("report-group-some", { done: g.done, total: g.asked }))}
      {g.left > 0 && <span className="rp-left">{l10n.getString("report-group-left", { count: g.left })}</span>}
    </>
  );
  const fileSize = (g: (typeof groups)[number]) => l10n.getString("report-group-size", { files: g.lines.length, size: formatBytes(g.bytes, pageLocale()) });

  /** A group's own page: Back to the overview, its name as the heading, then its lines. */
  const groupPage = (key: FileKind | Beyond, size: string, state: ReactNode, body: ReactNode) => (
    <>
      <TopStrip
        onBack={() => {
          setOpen(null);
        }}
        labelId="outcome-eyebrow"
      />
      <Heading key={key} id={`report-group-${key}`} />
      <p className="subhead rp-page-line">
        <span>{size}</span> <span>{state}</span>
      </p>
      <div className="rp-body">{body}</div>
    </>
  );

  if (open !== null) {
    const file = groups.find((g) => g.kind === open);
    if (file) {
      return groupPage(
        file.kind,
        fileSize(file),
        fileState(file),
        <>
          {table(file.lines.slice(0, shown))}
          {file.lines.length > shown && (
            <>
              <p className="mv-count">{l10n.getString("report-showing", { shown, total: file.lines.length })}</p>
              <Button
                onClick={() => {
                  setShown(shown + PAGE);
                }}
              >
                {l10n.getString("report-show-more")}
              </Button>
            </>
          )}
        </>,
      );
    }
    const other = beyond.find((g) => g.key === open);
    if (other) return groupPage(other.key, other.count, other.state, other.body);
  }

  const nothing = items.length === 0 && beyond.length === 0;
  return (
    <>
      <TopStrip onBack={onBack} labelId="outcome-eyebrow" />
      <Heading key="overview" id="report-title" />
      <p className="subhead">{l10n.getString("report-body")}</p>
      {nothing ? (
        <p className="mv-empty">{l10n.getString("report-none")}</p>
      ) : (
        <>
          {needsCount > 0 && (
            <section className="rp-needs" aria-labelledby="rp-needs-title">
              <h2 id="rp-needs-title" className="rp-title">
                {l10n.getString("report-needs-title", { count: needsCount })}
              </h2>
              {needs.length > 0 && table(needs)}
              {needsLines.length > 0 && rebuiltRows(needsLines)}
              {needsPeople.length > 0 && personRows(needsPeople)}
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
            {groups.map((g) => (
              <li key={g.kind} className="rp-group">
                {groupHead(g.kind, name(g.kind), fileSize(g), fileState(g), true)}
              </li>
            ))}
            {beyond.map((g) => (
              <li key={g.key} className="rp-group">
                {groupHead(g.key, l10n.getString(`report-group-${g.key}`), g.count, g.state, false)}
              </li>
            ))}
          </ul>
          {(rebuilt?.more ?? 0) > 0 && <p className="mv-count">{l10n.getString("report-beyond-more", { count: rebuilt?.more ?? 0 })}</p>}
        </>
      )}
    </>
  );
}
