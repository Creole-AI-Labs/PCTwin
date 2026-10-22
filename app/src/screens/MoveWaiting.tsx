import { useLocalization } from "@fluent/react";
import { Banner, Button, Row, RowList } from "../components/kit";
import type { About, Peer, WaitStage } from "../bridge";
import { formatBytes, pageLocale } from "../move";
import FrameFooter from "./FrameFooter";
import { TopStrip } from "./FrameParts";
import { Heading } from "./ProgressParts";
import "./progress.css";

/** The words for each wait, by which laptop this is. */
const WORDS: Record<"old" | "new", Record<WaitStage, string>> = {
  old: { scanning: "move-wait-scanning", waiting: "move-wait-approval", moving: "move-wait-starting", administrator: "move-wait-admin" },
  new: { scanning: "move-wait-list", waiting: "move-wait-list", moving: "move-wait-starting", administrator: "move-wait-admin" },
};

/** One laptop in plain words: its system, chip and PCTwin version, then how it is powered. */
function AboutRow({ title, about }: { title: string; about: About }) {
  const { l10n } = useLocalization();
  const power =
    about.power.kind === "battery"
      ? l10n.getString("move-peer-battery", { percent: about.power.percent })
      : l10n.getString(`move-peer-${about.power.kind}`);
  return (
    <Row
      title={title}
      detail={
        <bdi dir="ltr">
          {l10n.getString("move-peer-line", { system: l10n.getString(`move-peer-system-${about.system}`), chip: about.chip, app: about.app })}
        </bdi>
      }
      end={<small>{power}</small>}
    />
  );
}

/**
 * C06 Meet your other laptop: both laptops as each said when the move started, and (C07's power
 * check) a word to plug in when either runs on battery.
 */
function TwoLaptops({ peer, old }: { peer: Peer; old: boolean }) {
  const { l10n } = useLocalization();
  const [oldOne, newOne] = old ? [peer.mine, peer.theirs] : [peer.theirs, peer.mine];
  const onBattery = peer.mine.power.kind === "battery" || peer.theirs.power.kind === "battery";
  return (
    <section className="mv-peer" aria-label={l10n.getString("move-peer-title")}>
      <h2 className="mv-peer-title">{l10n.getString("move-peer-title")}</h2>
      <RowList>
        <AboutRow title={l10n.getString("move-peer-old")} about={oldOne} />
        <AboutRow title={l10n.getString("move-peer-new")} about={newOne} />
      </RowList>
      {onBattery && <Banner tone="attention">{l10n.getString("move-peer-plug-in")}</Banner>}
    </section>
  );
}

/**
 * Between pairing and progress (M02 on the old laptop; the new laptop waiting for the old one's
 * list): what is happening now, that the old laptop is only read, and a way to stop. Nothing is
 * counted here that the engine has not measured, so there is no bar.
 */
export default function MoveWaiting({
  stage,
  old,
  note,
  found = null,
  peer = null,
  onBefore,
  onStop,
}: {
  stage: WaitStage;
  old: boolean;
  note?: string;
  /** While scanning: bytes of the mover's own things found so far (only ever grows). */
  found?: number | null;
  /** The two laptops, once they have greeted (C06). */
  peer?: Peer | null;
  /** The old laptop, while it waits: open "Before you leave this laptop" (M10/M13). */
  onBefore?: () => void;
  onStop: () => void;
}) {
  const { l10n } = useLocalization();
  const key = WORDS[old ? "old" : "new"][stage];
  return (
    <>
      <TopStrip labelId="move-wait-eyebrow" />
      <Heading key={key} id={`${key}-title`} />
      <p className="subhead" role="status">
        {l10n.getString(`${key}-body`)}
      </p>
      {found !== null && found > 0 && (
        <p className="mv-found" role="status">
          {l10n.getString("move-found", { size: formatBytes(found, pageLocale()) })}
        </p>
      )}
      {peer && <TwoLaptops peer={peer} old={old} />}
      <div className="mv-note">
        {note && <Banner tone="attention">{note}</Banner>}
        <Banner tone="info">{l10n.getString("progress-old-safe")}</Banner>
      </div>
      {onBefore && (
        <div className="mv-actions">
          <Button variant="quiet" onClick={onBefore}>
            {l10n.getString("move-before-open")}
          </Button>
        </div>
      )}
      <FrameFooter>
        <Button variant="quiet" onClick={onStop}>
          {l10n.getString("move-stop")}
        </Button>
      </FrameFooter>
    </>
  );
}
