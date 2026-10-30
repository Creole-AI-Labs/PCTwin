/**
 * The full move on the move's own screens: the apps, settings and app data section of the one
 * approval screen (M23), the page shown while they are set up after the files, and the section of
 * the outcome (R01) with each one's result word and Undo.
 *
 * Every line says what happens or what the person does next, in plain words; the result is a
 * word with an icon, never colour alone. Names are shown exactly as they are.
 */
import { useLocalization } from "@fluent/react";
import { Badge, Banner, Button, Checkbox, Row, RowList, Section } from "../components/kit";
import type { AppsOpen, ChangeView, DataProgress, Downloads, LineNote, LiveWait as LiveWaitView, RebuildPlan, RebuildStep, Rebuilt, RebuiltLine, Title } from "../fullmove";
import { shownName } from "../names";
import FrameFooter from "./FrameFooter";
import { Attention, Check } from "../icons";
import { pageLocale } from "../move";
import { TopStrip } from "./FrameParts";
import { PromiseBadge, UndoLegend } from "./ChangesPreview";
import { Name } from "./GuardParts";
import { PlanBox, useFormat } from "./PlanParts";
import { CompatRow } from "./Compat";
import { Heading } from "./ProgressParts";

export function TitleText({ title }: { title: Title }) {
  const { l10n } = useLocalization();
  return title.kind === "name" ? <Name>{title.value}</Name> : <>{l10n.getString(title.value)}</>;
}

/** What the person is told because of a change's special category (2.5), each in its words. */
export function specialNotes(c: Pick<ChangeView, "notes">, l10n: ReturnType<typeof useLocalization>["l10n"]): string[] {
  return c.notes.map((n) => l10n.getString(`special-note-${n.note}`));
}

/** What a change still needs from the person, or what it does, in words; an app made for Intel Macs says it needs Rosetta after that. */
function changeDetail(c: ChangeView, l10n: ReturnType<typeof useLocalization>["l10n"]): string {
  const what = changeWhat(c, l10n);
  return c.note?.note === "intel-only" ? [what, l10n.getString("fullmove-app-rosetta")].join(" ") : what;
}

function changeWhat(c: ChangeView, l10n: ReturnType<typeof useLocalization>["l10n"]): string {
  if (c.note?.note === "sites") return l10n.getString("fullmove-step-sites");
  if (c.note?.note === "older-here") return l10n.getString("fullmove-here-older", { version: shownName(c.note.version) });
  if (c.special) return [l10n.getString(`fullmove-step-${c.special}`), ...specialNotes(c, l10n)].join(" ");
  if (c.kind === "settings") {
    return [l10n.getString("fullmove-settings-count", { count: c.count }), l10n.getString(c.action === "replace" ? "fullmove-settings-replace" : "fullmove-settings-merge")].join(" ");
  }
  if (c.needs) return l10n.getString(`reason-${c.needs}`);
  if (c.action === "install") return c.source ? l10n.getString("fullmove-app-from", { id: c.source }) : l10n.getString("fullmove-app-install");
  return l10n.getString("fullmove-app-here");
}

function Changes({ titleId, changes }: { titleId: string; changes: ChangeView[] }) {
  const { l10n } = useLocalization();
  if (changes.length === 0) return null;
  return (
    <PlanBox title={l10n.getString(titleId, { count: changes.length })}>
      <RowList>
        {changes.map((c, i) => (
          <Row key={i} title={<TitleText title={c.title} />} detail={changeDetail(c, l10n)} end={<PromiseBadge promise={c.undo} />} />
        ))}
      </RowList>
    </PlanBox>
  );
}

/** The plan's two install choices, as the person set them (both go with the approval). */
export type InstallChoices = { waitFreeWifi: boolean; reuseInstallers: boolean };

/** Where the choices start: waiting for free Wi-Fi is on when the connection now may charge for data. */
export function installDefaults(downloads: Downloads | undefined): InstallChoices {
  return { waitFreeWifi: downloads !== undefined && downloads.apps > 0 && downloads.connection === "may-charge", reuseInstallers: false };
}

/**
 * M23's downloads: how many apps come from the internet, the connection now, and the two choices
 * (Wait for free Wi-Fi when anything is downloaded; Use the installers found on the old laptop
 * when one came). Nothing shows when nothing is downloaded.
 */
export function InstallChoicesBox({ downloads, choices, onChange }: { downloads: Downloads; choices: InstallChoices; onChange: (choices: InstallChoices) => void }) {
  const { l10n } = useLocalization();
  if (downloads.apps === 0) return null;
  return (
    <PlanBox title={l10n.getString("plan-downloads-title")}>
      <p className="plan-lead">{l10n.getString("plan-downloads", { count: downloads.apps })}</p>
      <p className="plan-quiet" data-connection={downloads.connection}>
        {l10n.getString(`plan-connection-${downloads.connection}`)}
      </p>
      <ul className="plan-items">
        <li className="plan-item">
          <Checkbox
            checked={choices.waitFreeWifi}
            onChange={(on) => {
              onChange({ ...choices, waitFreeWifi: on });
            }}
            label={l10n.getString("plan-wait-free-wifi")}
            hint={l10n.getString("plan-wait-free-wifi-help")}
          />
        </li>
        {downloads.withOldInstaller > 0 && (
          <li className="plan-item">
            <Checkbox
              checked={choices.reuseInstallers}
              onChange={(on) => {
                onChange({ ...choices, reuseInstallers: on });
              }}
              label={l10n.getString("plan-reuse-installers", { count: downloads.withOldInstaller })}
              hint={l10n.getString("plan-reuse-installers-help")}
            />
          </li>
        )}
      </ul>
    </PlanBox>
  );
}

/** M23's section for the full move: every app, setting and special category, with what undo can do for each. */
export function RebuildPlanSection({
  rebuild,
  picked = [],
  onPick,
  updates = [],
  onUpdate,
  onCompat,
  installs,
  onInstalls,
}: {
  rebuild: RebuildPlan;
  /** The two install choices; without `onInstalls` they are not shown. */
  installs?: InstallChoices;
  onInstalls?: (choices: InstallChoices) => void;
  picked?: readonly number[];
  onPick?: (line: number, bring: boolean) => void;
  /** The older apps here ticked "Update it", by their place in the plan. */
  updates?: readonly number[];
  onUpdate?: (app: number, update: boolean) => void;
  /** Opens one app's compatibility record on its own page. */
  onCompat?: (change: ChangeView) => void;
}) {
  const { l10n } = useLocalization();
  const { bytes } = useFormat();
  const tested = rebuild.changes.filter((c) => c.kind === "app" && c.tested !== null);
  const { downloads, signIns } = rebuild.internet;
  const room = rebuild.room;
  const install = rebuild.changes.filter((c) => c.kind === "app" && c.action === "install");
  const settings = rebuild.changes.filter((c) => c.kind === "settings");
  const appData = rebuild.changes.filter((c) => c.kind === "special" && c.special === "copy" && c.secret === null);
  const secrets = rebuild.changes.filter((c) => c.secret !== null);
  const needs = rebuild.changes.filter((c) => c.needs !== null);
  const here = rebuild.changes.filter((c) => c.kind === "app" && c.action === "leave-alone" && c.needs === null);
  const left = rebuild.notMoved.filter((n) => n.count > 0);
  const older = rebuild.changes.filter((c) => c.update !== null);
  return (
    <section className="fm-plan" aria-labelledby="fm-plan-title">
      <h2 id="fm-plan-title" className="plan-box-title">
        {l10n.getString("fullmove-plan-title")}
      </h2>
      <p className="subhead">{l10n.getString("fullmove-plan-body")}</p>
      <UndoLegend />
      <Changes titleId="fullmove-group-install" changes={install} />
      {installs && onInstalls && <InstallChoicesBox downloads={rebuild.downloads} choices={installs} onChange={onInstalls} />}
      <Changes titleId="fullmove-group-settings" changes={settings} />
      <Changes titleId="fullmove-group-appdata" changes={appData} />
      {secrets.length > 0 && (
        <PlanBox title={l10n.getString("fullmove-group-secrets", { count: secrets.length })}>
          <p className="plan-lead">{l10n.getString("fullmove-secrets-body")}</p>
          <ul className="plan-items">
            {secrets.map((c) => {
              const line = c.secret ?? -1;
              const name = c.title.kind === "name" ? c.title.value : l10n.getString(c.title.value);
              return (
                <li key={line} className="plan-item">
                  <Checkbox
                    checked={picked.includes(line)}
                    onChange={(on) => {
                      onPick?.(line, on);
                    }}
                    label={l10n.getString("fullmove-secret-bring", { name })}
                    hint={l10n.getString("fullmove-secret-hint")}
                  />
                </li>
              );
            })}
          </ul>
        </PlanBox>
      )}
      <Changes titleId="fullmove-group-needs" changes={needs} />
      <Changes titleId="fullmove-group-here" changes={here} />
      {older.length > 0 && onUpdate && (
        // An older version is here: left as it is unless the person ticks Update it.
        <PlanBox title={l10n.getString("fullmove-group-update", { count: older.length })}>
          <ul className="plan-items">
            {older.map((c) => {
              const at = c.update ?? -1;
              const name = c.title.kind === "name" ? c.title.value : l10n.getString(c.title.value);
              return (
                <li key={at} className="plan-item">
                  <Checkbox
                    checked={updates.includes(at)}
                    onChange={(on) => {
                      onUpdate(at, on);
                    }}
                    label={l10n.getString("fullmove-update-it", { name })}
                    hint={[l10n.getString("fullmove-update-hint"), c.undoIfUpdated === "keeps-newer" ? l10n.getString("guard-undo-keeps-newer") : ""].filter(Boolean).join(" ")}
                  />
                </li>
              );
            })}
          </ul>
        </PlanBox>
      )}
      {tested.length > 0 && onCompat && (
        <PlanBox title={l10n.getString("compat-list-title", { count: tested.length })}>
          <RowList>
            {tested.map((c, i) =>
              c.tested ? (
                <CompatRow
                  key={i}
                  change={c}
                  tested={c.tested}
                  onOpen={() => {
                    onCompat(c);
                  }}
                />
              ) : null,
            )}
          </RowList>
        </PlanBox>
      )}
      {rebuild.more > 0 && <p className="subhead">{l10n.getString("fullmove-more", { count: rebuild.more })}</p>}
      {(left.length > 0 || rebuild.specialNotMoved > 0) && (
        <PlanBox title={l10n.getString("fullmove-not-moved-title")}>
          <RowList>
            {left.map((n) => (
              <Row key={n.group} title={l10n.getString(`fullmove-not-moved-${n.group}`, { count: n.count })} />
            ))}
            {rebuild.specialNotMoved > 0 && <Row title={l10n.getString("fullmove-special-not-moved", { count: rebuild.specialNotMoved })} />}
          </RowList>
        </PlanBox>
      )}
      <PlanBox title={l10n.getString("plan-net-title")}>
        {downloads === 0 && signIns === 0 ? (
          <p className="plan-lead">{l10n.getString("plan-net-none")}</p>
        ) : (
          <>
            <RowList>
              {downloads > 0 && <Row title={l10n.getString("plan-net-downloads", { count: downloads })} detail={l10n.getString("plan-net-sizes")} />}
              {signIns > 0 && <Row title={l10n.getString("plan-net-signins", { count: signIns })} />}
            </RowList>
          </>
        )}
      </PlanBox>
      {(room.installs > 0 || room.appData > 0) && (
        <PlanBox title={l10n.getString("plan-room-title")}>
          <p className="plan-lead">{l10n.getString("plan-room-body", { installs: bytes(room.installs), appData: bytes(room.appData) })}</p>
        </PlanBox>
      )}
      {rebuild.links > 0 && <p className="subhead">{l10n.getString("fullmove-links", { count: rebuild.links })}</p>}
      {rebuild.prompt && (
        <div className="mv-note">
          <Banner tone="info">{l10n.getString(rebuild.restorePoint ? "fullmove-prompt-restore" : "fullmove-prompt")}</Banner>
        </div>
      )}
    </section>
  );
}

/**
 * After the files: apps and settings being set up. Nothing to press; it ends on the outcome.
 * `waitsForFreeWifi`: the person chose Wait for free Wi-Fi and apps are downloaded, so the
 * engine may hold the downloads up to 20 minutes on a connection that may charge; said while it
 * sets up apps, in words that stay true whether it is waiting or not.
 */
export function Rebuilding({ step, waitsForFreeWifi = false }: { step: RebuildStep | null; waitsForFreeWifi?: boolean }) {
  const { l10n } = useLocalization();
  const line =
    step === null
      ? null
      : step.step === "permission"
        ? l10n.getString("fullmove-rebuilding-permission")
        : step.step === "waiting-free-wifi"
          ? l10n.getString("fullmove-rebuilding-waiting-wifi", { minutes: step.minutesLeft })
          : l10n.getString(`fullmove-rebuilding-${step.step}`, { done: step.done.toLocaleString(pageLocale()), total: step.total.toLocaleString(pageLocale()) });
  return (
    <>
      <TopStrip labelId="outcome-eyebrow" />
      <Heading id="fullmove-rebuilding-title" />
      <p className="subhead">{l10n.getString("fullmove-rebuilding-body")}</p>
      {line && (
        <p className="subhead" role="status">
          {line}
        </p>
      )}
      {waitsForFreeWifi && step?.step !== "settings" && step?.step !== "waiting-free-wifi" && (
        <div className="mv-note">
          <Banner tone="info">{l10n.getString("fullmove-rebuilding-wait-wifi")}</Banner>
        </div>
      )}
    </>
  );
}

type L10n = ReturnType<typeof useLocalization>["l10n"];

/** One note in words; a note the line's own reason already says is left out. */
function noteText(n: LineNote, l: RebuiltLine, l10n: L10n): string {
  switch (n.note) {
    case "sign-in":
      return l.reason === "sign-in" ? "" : l10n.getString("fullmove-note-sign-in");
    case "licence-key":
      return l.reason === "activation" ? "" : l10n.getString("fullmove-note-licence-key");
    case "registration": {
      const who = [n.owner, n.company].filter((w): w is string => w !== null).map(shownName).join(", ");
      return [who ? l10n.getString("fullmove-note-registration", { who }) : "", n.productId ? l10n.getString("fullmove-note-product-id", { id: shownName(n.productId) }) : ""].filter(Boolean).join(" ");
    }
    case "extras":
      return l10n.getString("fullmove-note-extras", { apps: n.apps.map(shownName).join(", ") });
    case "sites":
      return l10n.getString("fullmove-step-sites");
    case "rosetta":
      return l10n.getString("fullmove-note-rosetta");
    case "data":
      return l10n.getString(`fullmove-data-${n.why}`, n.folder === undefined ? {} : { folder: shownName(n.folder) });
    case "came-from":
      return l10n.getString(n.makersSite ? "fullmove-note-came-from-maker" : "fullmove-note-came-from-elsewhere");
  }
}

export function lineDetail(l: RebuiltLine, l10n: L10n): string {
  const parts: string[] = [];
  if (l.kind === "settings") parts.push(l10n.getString("fullmove-settings-count", { count: l.count }));
  // App data that says why it needs the person says that instead of the general reason.
  const why = l.notes.some((n) => n.note === "data");
  if (l.reason && !why) parts.push(l10n.getString(l.kind === "special" && l.reason === "finish-by-hand" ? "fullmove-special-finish" : `reason-${l.reason}`));
  for (const n of l.notes) parts.push(noteText(n, l, l10n));
  // What its special category tells the person (2.5), said as the plan said it.
  parts.push(...specialNotes({ notes: l.specialNotes }, l10n));
  return parts.filter(Boolean).join(" ");
}

/** R01's section for the full move: each app, group of settings and special category with its result, and Undo. */
export function RebuiltSection({ rebuilt, onUndo }: { rebuilt: Rebuilt; onUndo?: () => void }) {
  const { l10n } = useLocalization();
  if (rebuilt.lines.length === 0 && rebuilt.here.length === 0 && !rebuilt.stopped) return null;
  return (
    <Section title={l10n.getString("fullmove-outcome-title")}>
      {rebuilt.stopped && (
        <div className="mv-note">
          <Banner tone="attention">{l10n.getString("fullmove-outcome-stopped")}</Banner>
        </div>
      )}
      <RowList>
        {rebuilt.lines.map((l, i) => {
          const good = l.status === "copied" || l.status === "reinstalled" || l.status === "converted";
          return (
            <Row
              key={i}
              title={<TitleText title={l.title} />}
              detail={lineDetail(l, l10n)}
              end={
                <Badge tone={good ? "good" : "warn"} icon={good ? <Check className="mv-icon" /> : <Attention className="mv-icon" />}>
                  {l10n.getString(`status-${l.status}`)}
                </Badge>
              }
            />
          );
        })}
      </RowList>
      {rebuilt.more > 0 && <p className="subhead">{l10n.getString("fullmove-more", { count: rebuilt.more })}</p>}
      {rebuilt.fresh && <p className="subhead">{l10n.getString("fullmove-outcome-fresh")}</p>}
      {rebuilt.here.length > 0 && (
        <>
          <h3 className="plan-box-title">{l10n.getString("fullmove-outcome-here-title", { count: rebuilt.here.length + rebuilt.hereMore })}</h3>
          <RowList>
            {rebuilt.here.map((h, i) => (
              <Row
                key={i}
                title={<Name>{h.name}</Name>}
                detail={h.older === null ? l10n.getString("fullmove-outcome-here") : l10n.getString("fullmove-outcome-here-older", { version: shownName(h.older) })}
                end={<Badge>{l10n.getString("fullmove-outcome-here-badge")}</Badge>}
              />
            ))}
          </RowList>
          {rebuilt.hereMore > 0 && <p className="subhead">{l10n.getString("fullmove-more", { count: rebuilt.hereMore })}</p>}
        </>
      )}
      {rebuilt.restorePoint !== "not-asked" && <p className="subhead">{l10n.getString(`fullmove-restore-${rebuilt.restorePoint}`)}</p>}
      {rebuilt.canUndo && onUndo && (
        <div className="fm-undo">
          <Button variant="secondary" onClick={onUndo}>
            {l10n.getString("fullmove-outcome-undo")}
          </Button>
        </div>
      )}
    </Section>
  );
}

/** How many of the full move's lines need the person (added to R01's "needs action" figure). */
export function rebuiltNeedsAction(rebuilt: Rebuilt | undefined): number {
  return rebuilt ? rebuilt.lines.filter((l) => l.status === "needs-action").length : 0;
}

/** The new laptop, while the old laptop's app data comes: measured, and what the old laptop waits for. */
export function AppDataComing({ progress }: { progress: DataProgress }) {
  const { l10n } = useLocalization();
  return (
    <>
      <TopStrip labelId="outcome-eyebrow" />
      <Heading id="fullmove-appdata-title" />
      <p className="subhead" role="status">
        {l10n.getString("fullmove-appdata-progress", { done: progress.done.toLocaleString(pageLocale()), total: progress.total.toLocaleString(pageLocale()) })}
      </p>
      {progress.waitingFor.length > 0 && (
        <div className="mv-note">
          <Banner tone="attention">{l10n.getString("fullmove-appdata-waiting", { apps: progress.waitingFor.map(shownName).join(", ") })}</Banner>
        </div>
      )}
    </>
  );
}

/**
 * The old laptop: apps to close before their data can be read, and files in the person's own
 * folders an app holds open (2.5), each list on its own with what it means; Try again or Go on
 * without them.
 */
/**
 * Files an app holds open that the system cannot name: nobody is asked to find the app. PCTwin
 * looks again every 30 seconds for up to 15 minutes, then goes on without them (the report
 * lists them). Only Stop the move is offered (the founder, 30 October 2026).
 */
export function LiveWait({ wait, onStop }: { wait: LiveWaitView; onStop: () => void }) {
  const { l10n } = useLocalization();
  return (
    <>
      <TopStrip labelId="move-wait-eyebrow" />
      <Heading id="fullmove-live-wait-title" />
      <p className="subhead" role="status">
        {l10n.getString("fullmove-live-wait-body", { minutes: wait.minutesLeft })}
      </p>
      <RowList>
        {wait.files.map((f) => (
          <Row key={f} title={<Name>{f}</Name>} />
        ))}
      </RowList>
      <FrameFooter>
        <Button variant="quiet" onClick={onStop}>
          {l10n.getString("fullmove-live-wait-stop")}
        </Button>
      </FrameFooter>
    </>
  );
}

export function CloseApps({ open, onAnswer }: { open: AppsOpen; onAnswer: (retry: boolean) => void }) {
  const { l10n } = useLocalization();
  const { apps, files } = open;
  return (
    <>
      <TopStrip labelId="move-wait-eyebrow" />
      <Heading id={apps.length > 0 ? "fullmove-close-title" : "fullmove-close-files-title"} />
      {apps.length > 0 && (
        <>
          <p className="subhead">{l10n.getString("fullmove-close-body")}</p>
          <RowList>
            {apps.map((a) => (
              <Row key={a} title={<Name>{a}</Name>} />
            ))}
          </RowList>
        </>
      )}
      {files.length > 0 && (
        <section aria-labelledby="fm-close-files">
          {apps.length > 0 && (
            <h2 id="fm-close-files" className="plan-box-title">
              {l10n.getString("fullmove-close-files-title")}
            </h2>
          )}
          <p className="subhead" id={apps.length > 0 ? undefined : "fm-close-files"}>
            {l10n.getString("fullmove-close-files")}
          </p>
          <RowList>
            {files.map((f) => (
              <Row key={f} title={<Name>{f}</Name>} />
            ))}
          </RowList>
        </section>
      )}
      <FrameFooter>
        <Button
          variant="quiet"
          onClick={() => {
            onAnswer(false);
          }}
        >
          {l10n.getString("fullmove-close-without")}
        </Button>
        <Button
          variant="primary"
          onClick={() => {
            onAnswer(true);
          }}
        >
          {l10n.getString("fullmove-close-again")}
        </Button>
      </FrameFooter>
    </>
  );
}
