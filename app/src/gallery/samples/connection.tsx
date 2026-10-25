/** Samples: the connection screens (F01 Resume your move, C01, C02, C06, C07, C09, C10), from made-up data. */
import type { ConnectionCheck, PeerReadiness, WaitingState } from "../../connection";
import ConnectionEnded from "../../screens/ConnectionEnded";
import ConnectionRoute from "../../screens/ConnectionRoute";
import CheckConnection from "../../screens/CheckConnection";
import MeetLaptop from "../../screens/MeetLaptop";
import PutTogether from "../../screens/PutTogether";
import WaitingForLaptop from "../../screens/WaitingForLaptop";
import Welcome from "../../screens/Welcome";
import type { Sample } from "../samples";

const nothing = () => undefined;

const old = { label: "Blue Fox", system: "Windows 11 · x64", version: "1.4.2" };
const fresh = { label: "Green Owl", system: "macOS 15 · ARM", version: "1.4.2" };
const ready: PeerReadiness = { state: "ready", sending: old, receiving: fresh };
const differ: PeerReadiness = { state: "versions-differ", sending: old, receiving: { ...fresh, version: "1.5.0" }, updateOn: "sending" };
const unsupported: PeerReadiness = { state: "direction-unsupported", sending: old, receiving: fresh };

const good: ConnectionCheck = { oldLabel: "Blue Fox", newLabel: "Green Owl", speedMbPerSecond: 62, link: "good", freeSpaceGb: 410, powerOld: "plugged-in", powerNew: "plugged-in", internet: "online" };
const weak: ConnectionCheck = { ...good, speedMbPerSecond: 4, link: "weak", powerOld: "on-battery" };
const offline: ConnectionCheck = { ...good, internet: "offline" };

const waiting: WaitingState = { kind: "reconnecting", otherLaptop: "Blue Fox", savedGb: 81.6, totalGb: 120, lastSeenSeconds: 45 };

const peer = (id: string, title: string, readiness: PeerReadiness): Sample => ({
  id,
  title,
  size: "expanded",
  render: () => <MeetLaptop readiness={readiness} onContinue={nothing} onBack={nothing} />,
});

const check = (id: string, title: string, value: ConnectionCheck | null): Sample => ({
  id,
  title,
  size: "expanded",
  render: () => <CheckConnection check={value} onContinue={nothing} onRecheck={nothing} onBack={nothing} />,
});

const wait = (id: string, title: string, state: WaitingState): Sample => ({
  id,
  title,
  size: "everyday",
  render: () => <WaitingForLaptop waiting={state} onRetry={nothing} onPairAgain={nothing} />,
});

export const samples: Sample[] = [
  {
    id: "F01-unfinished",
    title: "Home with a move to resume",
    size: "everyday",
    render: () => <Welcome onStart={nothing} onSoon={nothing} onQuick={nothing} onHistory={nothing} unfinished onResume={nothing} />,
  },
  { id: "C01-default", title: "How will you connect?", size: "everyday", render: () => <ConnectionRoute onBack={nothing} onWifi={nothing} /> },
  { id: "C02-default", title: "Put both laptops together", size: "everyday", render: () => <PutTogether onBack={nothing} onContinue={nothing} /> },
  {
    id: "C02-progress",
    title: "Put both laptops together, first step done",
    size: "everyday",
    render: () => <PutTogether steps={["done", "current", "todo"]} network="Home Wi-Fi" onBack={nothing} onContinue={nothing} />,
  },
  peer("C06-ready", "Meet your other laptop", ready),
  peer("C06-versions-differ", "Meet your other laptop: versions differ", differ),
  peer("C06-unsupported", "Meet your other laptop: direction not supported", unsupported),
  check("C07-good", "Let’s check the connection", good),
  check("C07-weak", "Let’s check the connection: weak link, on battery", weak),
  check("C07-offline", "Let’s check the connection: offline", offline),
  check("C07-measuring", "Let’s check the connection: measuring", null),
  wait("C09-reconnecting", "Waiting for your other laptop", waiting),
  wait("C09-expired", "Waiting: pairing expired", { ...waiting, kind: "pairing-expired", lastSeenSeconds: 1800 }),
  wait("C09-unreachable", "Waiting: not reachable", { ...waiting, kind: "unreachable", lastSeenSeconds: 300 }),
  {
    id: "C10-default",
    title: "The connection has ended",
    size: "everyday",
    render: () => <ConnectionEnded onPairAgain={nothing} onSeeMoved={nothing} onHome={nothing} />,
  },
];
