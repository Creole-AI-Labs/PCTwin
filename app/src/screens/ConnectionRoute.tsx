import { Localized } from "@fluent/react";
import { Badge, Button } from "../components/kit";
import { ArrowForward, Check, Info, Wifi } from "../icons";
import FrameFooter from "./FrameFooter";
import { TopStrip } from "./FrameParts";
import { Heading } from "./PairParts";
import "./connection.css";

/**
 * C01 How will you connect? Wi-Fi on the same network is the only way in this release, so it is the
 * one card and the one action. Later ways are one quiet line, never a greyed-out card.
 */
export default function ConnectionRoute({ onBack, onWifi }: { onBack: () => void; onWifi: () => void }) {
  return (
    <>
      <TopStrip onBack={onBack} labelId="connect-eyebrow" />
      <Heading id="connect-title" />
      <Localized id="connect-body">
        <p className="subhead" />
      </Localized>
      <section className="connect-card" aria-labelledby="connect-wifi-title">
        <span className="connect-card-icon">
          <Wifi />
        </span>
        <div className="connect-card-text">
          <Localized id="connect-wifi-title">
            <h2 id="connect-wifi-title" />
          </Localized>
          <Localized id="connect-wifi-body">
            <p />
          </Localized>
        </div>
        <Badge tone="good" icon={<Check />}>
          <Localized id="connect-wifi-status">
            <span />
          </Localized>
        </Badge>
      </section>
      <Localized id="connect-later">
        <p className="connect-quiet" />
      </Localized>
      <div className="note">
        <Info />
        <Localized id="connect-usb-note">
          <p className="note-text" />
        </Localized>
      </div>
      <FrameFooter>
        <Button variant="primary" icon={<ArrowForward />} onClick={onWifi}>
          <Localized id="connect-use-wifi">
            <span />
          </Localized>
        </Button>
      </FrameFooter>
    </>
  );
}
