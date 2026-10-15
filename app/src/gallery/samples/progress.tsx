/** Samples: progress and results (X01, X02, F09, X05, X10, R01, R02), from made-up data. */
import type { Sample } from "../samples";
import * as data from "../../moveSamples";
import MiniPanel from "../../screens/MiniPanel";
import Outcome from "../../screens/Outcome";
import Paused from "../../screens/Paused";
import ProgressCompact from "../../screens/ProgressCompact";
import ProgressDetail from "../../screens/ProgressDetail";
import Report from "../../screens/Report";
import StopMove from "../../screens/StopMove";
import type { MoveProgress } from "../../move";

const nothing = () => undefined;
const handlers = { onPause: nothing, onResume: nothing, onStop: nothing, onSeeNeedsAction: nothing };

const compact = (progress: MoveProgress) => <ProgressCompact progress={progress} onExpand={nothing} onMinimise={nothing} {...handlers} />;
const detail = (progress: MoveProgress) => <ProgressDetail progress={progress} onCollapse={nothing} {...handlers} />;
const mini = (progress: MoveProgress, kind: "move" | "send" = "move") => <MiniPanel kind={kind} progress={progress} onPause={nothing} onResume={nothing} onOpen={nothing} />;

export const samples: Sample[] = [
  { id: "X01-moving", title: "Move progress", size: "everyday", render: () => compact(data.running) },
  { id: "X01-starting", title: "Move progress, nothing measured yet", size: "everyday", render: () => compact(data.starting) },
  { id: "X01-checking-last", title: "Move progress, last files being checked", size: "everyday", render: () => compact(data.almost) },
  { id: "X01-needs-action", title: "Move progress, some items need you", size: "everyday", render: () => compact(data.withFailures) },
  { id: "X01-connection-lost", title: "Move progress, connection dropped", size: "everyday", render: () => compact(data.linkLost) },
  { id: "X01-low-battery", title: "Move progress, low battery", size: "everyday", render: () => compact(data.lowBattery) },
  { id: "X02-moving", title: "Move progress, every item", size: "expanded", render: () => detail(data.running) },
  { id: "X02-paused", title: "Move progress, every item, paused", size: "expanded", render: () => detail(data.paused) },
  { id: "F09-moving", title: "Mini panel", size: "mini", render: () => mini(data.running) },
  { id: "F09-paused", title: "Mini panel, paused", size: "mini", render: () => mini(data.paused) },
  { id: "F09-send", title: "Mini panel, Quick Send", size: "mini", render: () => mini(data.running, "send") },
  { id: "X05-pausing", title: "Pausing, file still finishing", size: "everyday", render: () => <Paused progress={data.pausing} onResume={nothing} onStop={nothing} /> },
  { id: "X05-paused", title: "Paused at a safe point", size: "everyday", render: () => <Paused progress={data.paused} onResume={nothing} onStop={nothing} /> },
  { id: "X10-stop", title: "Stop the rest of the move?", size: "everyday", render: () => <StopMove open onStop={nothing} onKeep={nothing} /> },
  { id: "R01-clean", title: "Outcome, everything copied", size: "everyday", render: () => <Outcome outcome={data.finishedClean} onReport={nothing} onHome={nothing} /> },
  { id: "R01-mixed", title: "Outcome, some need you or failed", size: "everyday", render: () => <Outcome outcome={data.finishedMixed} onReport={nothing} onHome={nothing} /> },
  { id: "R01-memory", title: "Outcome, checked from memory", size: "everyday", render: () => <Outcome outcome={data.finishedMemory} onReport={nothing} onHome={nothing} /> },
  { id: "R01-stopped", title: "Outcome, stopped by the person", size: "everyday", render: () => <Outcome outcome={data.finishedStopped} onReport={nothing} onHome={nothing} /> },
  { id: "R02-report", title: "Report, item by item", size: "expanded", render: () => <Report items={data.report} onBack={nothing} /> },
];
