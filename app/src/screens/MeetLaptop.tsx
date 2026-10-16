import { Localized } from "@fluent/react";
import { useId } from "react";
import type { ReactNode } from "react";
import { Badge, Banner, Button, Row, RowList } from "../components/kit";
import type { Laptop, PeerReadiness } from "../connection";
import { ArrowForward, Attention, Check, Laptop as LaptopIcon, Shield } from "../icons";
import FrameFooter from "./FrameFooter";
import { TopStrip } from "./FrameParts";
import { Heading } from "./PairParts";
import "./connection.css";

/**
 * C06 Meet your other laptop, laid out as in the mockup: the sending and receiving laptops side by
 * side (name and system, then the PCTwin version), a note, and one action. Only the ready state can
 * go on: in the other two the action stays visible but off, and the note says why. Nothing has moved.
 */
export default function MeetLaptop({ readiness, onContinue, onBack }: { readiness: PeerReadiness; onContinue: () => void; onBack: () => void }) {
  const { state, sending, receiving } = readiness;
  return (
    <>
      <TopStrip onBack={onBack} labelId="peer-eyebrow" />
      <Heading key={state} id="peer-title" />
      <Localized id="peer-body">
        <p className="subhead" />
      </Localized>
      {readiness.state === "versions-differ" ? <VersionRows readiness={readiness} /> : (
        <div className="connect-pair">
          <LaptopCard heading="peer-sending" role="peer-old" laptop={sending} />
          <LaptopCard heading="peer-receiving" role="peer-new" laptop={receiving} />
        </div>
      )}
      {state === "ready" && (
        <div className="note connect-note-good">
          <Check />
          <Localized id="peer-ready-note">
            <p className="note-text" />
          </Localized>
        </div>
      )}
      {state === "versions-differ" && (
        <div className="connect-notice">
          <Banner tone="attention">
            <Localized id="peer-version-differs">
              <p />
            </Localized>
          </Banner>
        </div>
      )}
      {state === "direction-unsupported" && (
        <div className="connect-notice">
          <Banner tone="attention">
            <Localized id="peer-direction-unsupported" vars={{ from: sending.system, to: receiving.system }}>
              <p />
            </Localized>
          </Banner>
        </div>
      )}
      <FrameFooter>
        <Button variant="primary" icon={<ArrowForward />} disabled={state !== "ready"} onClick={onContinue}>
          <Localized id="peer-continue">
            <span />
          </Localized>
        </Button>
      </FrameFooter>
    </>
  );
}

/** Version mismatch, as in the mockup: one row for each laptop (blocked or waiting) and the trusted update to use. */
function VersionRows({ readiness }: { readiness: Extract<PeerReadiness, { state: "versions-differ" }> }) {
  const { sending, receiving, updateOn } = readiness;
  const sides = [
    { side: "sending" as const, role: "peer-old", laptop: sending },
    { side: "receiving" as const, role: "peer-new", laptop: receiving },
  ];
  return (
    <div className="connect-panel connect-pair-rows">
      <RowList>
        {sides.map(({ side, role, laptop }) => (
          <Row
            key={side}
            icon={<LaptopIcon />}
            title={<Word id={role} />}
            detail={
              <>
                <span className="connect-line">
                  <bdi>{laptop.label}</bdi>
                </span>
                <span className="connect-line">{side === updateOn ? <Word id="peer-version-old-body" vars={{ version: laptop.version }} /> : <Word id="peer-version-new-body" />}</span>
              </>
            }
            end={
              <Badge tone="warn" icon={<Attention />}>
                <Word id={side === updateOn ? "peer-status-update" : "peer-status-unsupported"} />
              </Badge>
            }
          />
        ))}
        <Row icon={<Shield />} title={<Word id="peer-update-title" />} detail={<Word id="peer-update-body" />} />
      </RowList>
    </div>
  );
}

function Word({ id, vars }: { id: string; vars?: Record<string, string | number> }) {
  return (
    <Localized id={id} vars={vars}>
      <span />
    </Localized>
  );
}

/** One laptop: which side it is, then its name and system, then the PCTwin version. Names are read in their own direction. */
function LaptopCard({ heading, role, laptop }: { heading: string; role: string; laptop: Laptop }) {
  const id = useId();
  return (
    <section className="connect-laptop" aria-labelledby={id}>
      <Localized id={heading}>
        <h2 id={id} className="connect-laptop-side" />
      </Localized>
      <dl className="connect-facts">
        <Fact label={<Word id={role} />}>
          <bdi>{laptop.label}</bdi> · <bdi>{laptop.system}</bdi>
        </Fact>
        <Fact label={<Word id="peer-version-label" />}>
          <Localized id="peer-version" vars={{ version: laptop.version }}>
            <span />
          </Localized>
        </Fact>
      </dl>
    </section>
  );
}

function Fact({ label, children }: { label: ReactNode; children: ReactNode }) {
  return (
    <div>
      <dt>{label}</dt>
      <dd>{children}</dd>
    </div>
  );
}
