import { useLocalization } from "@fluent/react";
import { finishOverview } from "../bridge";
import { Badge, Banner, Button, Row, RowList, Section } from "../components/kit";
import type { FinishView, Tally } from "../finish";
import { Attention, Check, Info } from "../icons";
import { listText } from "../move";
import { FinishPage, Loaded, MoreLine, Name, useLoad, whenText } from "./FinishParts";

export type FinishStep = "setup" | "goodbye" | "catchup" | "handover" | "undo" | "cleanup" | "report" | "doctor";

const STEPS: readonly FinishStep[] = ["setup", "goodbye", "catchup", "handover", "undo", "cleanup", "report", "doctor"];

const TALLY_ROWS: readonly { key: keyof Tally; kebab: string; always: boolean }[] = [
  { key: "selected", kebab: "selected", always: true },
  { key: "done", kebab: "done", always: true },
  { key: "needsAction", kebab: "needs-action", always: false },
  { key: "failed", kebab: "failed", always: false },
  { key: "waiting", kebab: "waiting", always: false },
  { key: "notReachable", kebab: "not-reachable", always: false },
  { key: "noResultYet", kebab: "no-result-yet", always: false },
  { key: "skipped", kebab: "skipped", always: false },
  { key: "leftOut", kebab: "left-out", always: false },
  { key: "acceptedLoss", kebab: "accepted-loss", always: false },
];

/**
 * Finish your move (R01 side): one headline that is never a green tick over unfinished work, the
 * counts, the check made on this laptop now, what is not finished, and the next steps, each of
 * which opens its own page. The state is told in words and a shape, never by colour alone, and a
 * partly finished move gets no celebration.
 */
export default function FinishHub({ id, onBack, onOpen }: { id: string | null; onBack: () => void; onOpen: (step: FinishStep) => void }) {
  const { l10n } = useLocalization();
  const { state, reload } = useLoad(() => finishOverview(id));
  return (
    <FinishPage titleId="finish-title" onBack={onBack}>
      <Loaded state={state} onRetry={reload}>
        {(view) => (
          <>
            <Headline view={view} />
            <Section title={l10n.getString("finish-tally-title")}>
              <RowList>
                {TALLY_ROWS.filter((r) => r.always || view.tally[r.key] > 0).map((r) => (
                  <Row key={r.key} title={l10n.getString(`finish-tally-${r.kebab}`, { count: view.tally[r.key].toLocaleString() })} />
                ))}
                {view.others > 0 && <Row title={l10n.getString("finish-others", { count: view.others.toLocaleString() })} />}
              </RowList>
            </Section>
            {view.notes.length > 0 && (
              <div className="mv-note">
                {view.notes.map((n) => (
                  <Banner key={n} tone="info">
                    {l10n.getString(`finish-note-${n}`)}
                  </Banner>
                ))}
              </div>
            )}
            <After view={view} />
            <NotFinished view={view} />
            <Where view={view} />
            <Section title={l10n.getString("finish-steps-title")}>
              <RowList>
                {STEPS.filter((s) => s !== "undo" || view.notes.includes("undo-open")).map((s) => {
                  const title = l10n.getString(`finish-step-${s}`);
                  return (
                    <Row
                      key={s}
                      title={title}
                      detail={l10n.getString(`finish-step-${s}-detail`)}
                      end={
                        <Button
                          variant="quiet"
                          aria-label={l10n.getString("finish-open-name", { title })}
                          onClick={() => {
                            onOpen(s);
                          }}
                        >
                          {l10n.getString("finish-open")}
                        </Button>
                      }
                    />
                  );
                })}
              </RowList>
            </Section>
          </>
        )}
      </Loaded>
    </FinishPage>
  );
}

function Headline({ view }: { view: FinishView }) {
  const { l10n } = useLocalization();
  const ok = view.headline === "ready-for-selected";
  return (
    <div className="fin-headline">
      {ok ? <Check className="mv-icon" /> : view.headline === "nothing-moved" ? <Info className="mv-icon" /> : <Attention className="mv-icon" />}
      <div>
        <h2>{l10n.getString(`finish-headline-${view.headline}`)}</h2>
        <p>{l10n.getString(`finish-headline-${view.headline}-body`)}</p>
      </div>
    </div>
  );
}

function After({ view }: { view: FinishView }) {
  const { l10n } = useLocalization();
  const groups = view.afterMove.filter((g) => g.atScan > 0);
  if (groups.length === 0) return null;
  return (
    <Section title={l10n.getString("finish-after-title")}>
      <p className="subhead">{l10n.getString("finish-after-body")}</p>
      <RowList>
        {groups.map((g) => (
          <Row
            key={g.group}
            title={l10n.getString(`finish-group-${g.group}`)}
            detail={l10n.getString("finish-after-count", { onNew: g.onNewLaptop.toLocaleString(), atScan: g.atScan.toLocaleString() })}
            end={
              g.onNewLaptop >= g.atScan ? (
                <Badge tone="good" icon={<Check className="mv-icon" />}>
                  {l10n.getString("finish-after-all")}
                </Badge>
              ) : (
                <Badge tone="warn" icon={<Attention className="mv-icon" />}>
                  {l10n.getString("finish-after-short")}
                </Badge>
              )
            }
          />
        ))}
      </RowList>
    </Section>
  );
}

function NotFinished({ view }: { view: FinishView }) {
  const { l10n } = useLocalization();
  if (view.notFinished.length === 0) return null;
  return (
    <Section title={l10n.getString("finish-open-title")}>
      <p className="subhead">{l10n.getString("finish-open-body")}</p>
      <RowList>
        {view.notFinished.map((item, i) => (
          <Row
            key={`${item.id}-${String(i)}`}
            title={<Name>{item.label}</Name>}
            detail={
              [item.reason ? l10n.getString(`reason-${item.reason}`) : "", item.access === "usable" ? "" : l10n.getString(`finish-access-${item.access}`)]
                .filter(Boolean)
                .join(" ") || undefined
            }
            end={<Badge tone="warn">{l10n.getString(`finish-standing-${item.standing}`)}</Badge>}
          />
        ))}
      </RowList>
      <MoreLine count={view.more} />
    </Section>
  );
}

function Where({ view }: { view: FinishView }) {
  const { l10n } = useLocalization();
  const { search } = view;
  return (
    <Section title={l10n.getString("finish-search-title")}>
      {search.searched.length > 0 && <p>{l10n.getString("finish-search-searched", { places: listText(search.searched) })}</p>}
      {search.notSearched.length > 0 && <p>{l10n.getString("finish-search-not-searched", { places: listText(search.notSearched) })}</p>}
      {search.foundEverything && <p>{l10n.getString("finish-search-all")}</p>}
      <p>{view.oldLaptopCheckedAt === null ? l10n.getString("finish-old-never") : l10n.getString("finish-old-checked", { when: whenText(view.oldLaptopCheckedAt) })}</p>
    </Section>
  );
}
