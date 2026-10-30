/**
 * The compatibility record for one app (Task List 2.9 item 8): how well PCTwin tested it, each part
 * with its own result, the steps the person will do, and when and how it was checked. It opens on
 * its own page (Back returns to the plan). The result is a word with an icon, never colour alone, and
 * an app nobody tested says so plainly.
 */
import { useLocalization } from "@fluent/react";
import { Badge, Button, Row, RowList, Section } from "../components/kit";
import type { AppDetail, ChangeView, Tested, TestedOutcome } from "../fullmove";
import { Attention, Check, Info } from "../icons";
import { MoreDetails } from "./GuardParts";
import { TopStrip } from "./FrameParts";
import { Heading } from "./ProgressParts";
import { Name } from "./FinishParts";
import { pageLocale } from "../move";
import { shownName } from "../names";

/** The tested outcome as a badge: a word, and a shape that differs for good, partly and unknown. */
export function OutcomeBadge({ outcome }: { outcome: TestedOutcome }) {
  const { l10n } = useLocalization();
  const good = outcome === "works";
  const icon = good ? <Check className="mv-icon" /> : outcome === "not-tested" ? <Info className="mv-icon" /> : <Attention className="mv-icon" />;
  return (
    <Badge tone={good ? "good" : outcome === "not-tested" ? "neutral" : "warn"} icon={icon}>
      {l10n.getString(`compat-outcome-${outcome}`)}
    </Badge>
  );
}

/** One line in the plan: the app, how well it was tested, and the way into its own page. */
export function CompatRow({ change, tested, onOpen }: { change: ChangeView; tested: Tested; onOpen: () => void }) {
  const { l10n } = useLocalization();
  const name = change.title.kind === "name" ? change.title.value : l10n.getString(change.title.value);
  return (
    <Row
      title={<Name>{name}</Name>}
      end={
        <>
          <OutcomeBadge outcome={tested.outcome} />
          <Button variant="quiet" onClick={onOpen} aria-label={l10n.getString("compat-open-name", { name: shownName(name) })}>
            {l10n.getString("compat-open")}
          </Button>
        </>
      }
    />
  );
}

/**
 * One app's detail (M11): every place PCTwin can get it from, in the order it tries them, and what
 * the old laptop's copy says about who made it. When the person installs it from a store, the store's
 * own page for it: PCTwin opens it, the person installs, and the plan still says it is theirs to do.
 */
export function AppOrigin({ detail, onOpenStore }: { detail: AppDetail; onOpenStore?: (page: string) => void }) {
  const { l10n } = useLocalization();
  const maker =
    detail.checked === "unsigned"
      ? l10n.getString("app-maker-unsigned")
      : detail.checked === "not-read"
        ? l10n.getString("app-maker-not-read")
        : detail.maker
          ? l10n.getString("app-maker-signed", { maker: shownName(detail.maker) })
          : l10n.getString("app-maker-signed-unnamed");
  const store = detail.store;
  return (
    <>
      <Section title={l10n.getString("app-from-title")}>
        {detail.from.length === 0 ? (
          <p>{l10n.getString("app-from-none")}</p>
        ) : (
          <>
            <p>{l10n.getString("app-from-body")}</p>
            <RowList>
              {detail.from.map((k) => (
                <Row key={k} title={l10n.getString(`app-from-${k}`)} />
              ))}
            </RowList>
          </>
        )}
      </Section>
      <Section title={l10n.getString("app-maker-title")}>
        <p>{maker}</p>
      </Section>
      {store && onOpenStore && (
        <Section title={l10n.getString("app-store-title")}>
          <p>{l10n.getString("app-store-body")}</p>
          <Button
            variant="secondary"
            onClick={() => {
              onOpenStore(store.page);
            }}
          >
            {l10n.getString(`app-store-${store.kind}`)}
          </Button>
        </Section>
      )}
    </>
  );
}

export default function Compat({ change, tested, onBack, onOpenStore }: { change: ChangeView; tested: Tested; onBack: () => void; onOpenStore?: (page: string) => void }) {
  const { l10n } = useLocalization();
  const name = change.title.kind === "name" ? change.title.value : l10n.getString(change.title.value);
  const checked = tested.checkedOn !== null;
  return (
    <div className="plan-screen">
      <TopStrip onBack={onBack} labelId="compat-strip" />
      <Heading id="compat-title" vars={{ name: shownName(name) }} />
      <p className="subhead">
        <OutcomeBadge outcome={tested.outcome} />
      </p>
      {!checked && <p>{l10n.getString("compat-never-checked")}</p>}
      <Section title={l10n.getString("compat-aspects-title")}>
        <RowList>
          {tested.aspects.map((a) => (
            <Row
              key={a.aspect}
              title={l10n.getString(`compat-aspect-${a.aspect}`)}
              end={
                <Badge
                  tone={a.result === "works" ? "good" : a.result === "guided" ? "warn" : "neutral"}
                  icon={a.result === "works" ? <Check className="mv-icon" /> : a.result === "not-tested" || a.result === "not-applicable" ? <Info className="mv-icon" /> : <Attention className="mv-icon" />}
                >
                  {l10n.getString(`compat-result-${a.result}`)}
                </Badge>
              }
            />
          ))}
        </RowList>
      </Section>
      <Section title={l10n.getString("compat-steps-title")}>
        {tested.steps.length === 0 ? (
          <p>{l10n.getString("compat-steps-none")}</p>
        ) : (
          <RowList>
            {tested.steps.map((s) => (
              <Row key={s} title={l10n.getString(`compat-step-${s}`)} />
            ))}
          </RowList>
        )}
      </Section>
      {change.detail && <AppOrigin detail={change.detail} onOpenStore={onOpenStore} />}
      {checked && (
        <MoreDetails>
          <p>{l10n.getString("compat-checked-on", { date: new Intl.DateTimeFormat(pageLocale(), { dateStyle: "medium", timeZone: "UTC" }).format(new Date(`${tested.checkedOn ?? ""}T00:00:00Z`)) })}</p>
          {tested.how && <p>{l10n.getString(`compat-how-${tested.how}`)}</p>}
          {tested.version && <p>{l10n.getString("compat-version", { version: shownName(tested.version) })}</p>}
        </MoreDetails>
      )}
    </div>
  );
}
