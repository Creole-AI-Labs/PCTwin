import { Localized } from "@fluent/react";
import type { ReactNode } from "react";
import { Badge, Banner, Button, RowList, Row } from "../components/kit";
import type { ConnectionCheck } from "../connection";
import { ArrowForward, Attention, Check, Info, Laptop, Wifi } from "../icons";
import { useUnits } from "./ConnectionParts";
import { Eyebrow, Heading, PairStrip } from "./PairParts";
import "./connection.css";

type Tone = "good" | "warn" | "neutral";

/**
 * C07 Let's check the connection: measured speed and free space, power on both laptops and the
 * internet. Every judgement is a word with an icon. Something that could not be read says so; it is
 * never shown as fine. Time comes later as a range, so no time is promised here. `check` is null
 * while the engine is still measuring.
 */
export default function CheckConnection({
  check,
  onContinue,
  onRecheck,
  onBack,
}: {
  check: ConnectionCheck | null;
  onContinue: () => void;
  onRecheck: () => void;
  onBack: () => void;
}) {
  return (
    <>
      <PairStrip step={2} onBack={onBack} />
      <Eyebrow id="check-eyebrow" />
      <Heading id="check-title" />
      <Localized id="check-body">
        <p className="subhead" />
      </Localized>
      {check === null ? (
        <Localized id="check-measuring">
          <p role="status" className="connect-measuring" />
        </Localized>
      ) : (
        <Results check={check} />
      )}
      <div className="connect-actions">
        {check !== null && (
          <Button variant="primary" icon={<ArrowForward />} className="connect-block" onClick={onContinue}>
            <Localized id="check-continue">
              <span />
            </Localized>
          </Button>
        )}
        <Button className="connect-block" onClick={onRecheck}>
          <Localized id="check-again">
            <span />
          </Localized>
        </Button>
      </div>
    </>
  );
}

function Results({ check }: { check: ConnectionCheck }) {
  const units = useUnits();
  const speed = check.speedMbPerSecond;
  const space = check.freeSpaceGb;

  // Power: name each laptop that is on battery or not known; both plugged in gets one line.
  const power: { id: string; label: string }[] = [];
  const batteryNames = [check.powerOld === "on-battery" ? check.oldLabel : null, check.powerNew === "on-battery" ? check.newLabel : null].filter((n): n is string => n !== null);
  const unknownNames = [check.powerOld === "unknown" ? check.oldLabel : null, check.powerNew === "unknown" ? check.newLabel : null].filter((n): n is string => n !== null);
  for (const label of batteryNames) power.push({ id: "check-power-battery", label });
  for (const label of unknownNames) power.push({ id: "check-power-unknown", label });
  const powerStatus: [string, Tone] = batteryNames.length > 0 ? ["check-status-plug-in", "warn"] : unknownNames.length > 0 ? ["check-status-unknown", "neutral"] : ["check-status-ready", "good"];

  return (
    <>
      <div className="connect-metrics">
        <Metric label="check-speed-label">{speed === null ? <Word id="check-status-unknown" /> : units.mbps(speed)}</Metric>
        <Metric label="check-space-label">{space === null ? <Word id="check-status-unknown" /> : units.gb(space)}</Metric>
      </div>
      {(check.link === "weak" || speed === null) && (
        <div className="connect-notice">
          <Banner tone="attention">
            <Word id={speed === null ? "check-speed-unknown" : "check-link-weak"} />
          </Banner>
        </div>
      )}
      <div className="connect-panel">
      <RowList>
        <Row
          icon={<Laptop />}
          title={<Word id="check-power-title" />}
          detail={
            power.length === 0 ? (
              <Word id="check-power-both" />
            ) : (
              power.map((p) => (
                <span key={p.id + p.label} className="connect-line">
                  <Word id={p.id} vars={{ label: p.label }} />
                </span>
              ))
            )
          }
          end={<Status id={powerStatus[0]} tone={powerStatus[1]} />}
        />
        <Row
          icon={<Laptop />}
          title={<Word id="check-space-title" />}
          detail={space === null ? <Word id="check-space-unknown" /> : <Word id="check-space-body" vars={{ free: units.gb(space) }} />}
          end={space === null ? <Status id="check-status-unknown" tone="neutral" /> : undefined}
        />
        <Row
          icon={<Wifi />}
          title={<Word id="check-internet-title" />}
          detail={<Word id={{ online: "check-internet-on", offline: "check-internet-off", unknown: "check-internet-unknown" }[check.internet]} />}
          end={
            check.internet === "online" ? (
              <Status id="check-status-connected" tone="good" />
            ) : check.internet === "offline" ? (
              <Status id="check-status-not-needed" tone="neutral" />
            ) : (
              <Status id="check-status-unknown" tone="neutral" />
            )
          }
        />
      </RowList>
      </div>
      <div className="note">
        <Info />
        <Localized id="check-estimate-note">
          <p className="note-text" />
        </Localized>
      </div>
    </>
  );
}

function Metric({ label, children }: { label: string; children: ReactNode }) {
  return (
    <div className="connect-metric">
      <Localized id={label}>
        <span />
      </Localized>
      <strong>{children}</strong>
    </div>
  );
}

/** A Fluent string as an inline span, with optional variables. */
function Word({ id, vars }: { id: string; vars?: Record<string, string | number> }) {
  return (
    <Localized id={id} vars={vars}>
      <span />
    </Localized>
  );
}

/** The judgement as a word with an icon: ticked, needs attention, or neutral. */
function Status({ id, tone }: { id: string; tone: Tone }) {
  return (
    <Badge tone={tone} icon={tone === "good" ? <Check /> : tone === "warn" ? <Attention /> : <Info />}>
      <Word id={id} />
    </Badge>
  );
}
