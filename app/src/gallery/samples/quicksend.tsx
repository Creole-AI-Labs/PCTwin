/** Samples: Quick Send and My PCTwin history (F08), each screen in each state, from made-up data. */
import { useState } from "react";
import { BLUE_FOX, GREEN_OWL, HISTORY, OFFER, PICKED, PROGRESS, RECEIPT_PARTIAL, RECEIPT_PROBLEMS, RECEIPT_WHOLE } from "../../quicksend.fixtures";
import type { PickedItem } from "../../quicksend";
import History from "../../screens/History";
import QuickDone from "../../screens/QuickDone";
import QuickHome from "../../screens/QuickHome";
import QuickPick from "../../screens/QuickPick";
import QuickProgress from "../../screens/QuickProgress";
import QuickStop from "../../screens/QuickStop";
import type { Sample } from "../samples";

const nothing = () => undefined;

function Pick({ items, dragging = false }: { items: PickedItem[]; dragging?: boolean }) {
  const [list, setList] = useState(items);
  return (
    <QuickPick
      items={list}
      dragging={dragging}
      onAddFiles={nothing}
      onAddFolders={nothing}
      onToggle={(id, on) => {
        setList((prev) => prev.map((i) => (i.id === id ? { ...i, included: on } : i)));
      }}
      onAllowAccess={nothing}
      onContinue={nothing}
      onBack={nothing}
    />
  );
}

const progress = (patch: Partial<typeof PROGRESS>) => (
  <QuickProgress from={BLUE_FOX} to={GREEN_OWL} progress={{ ...PROGRESS, ...patch }} onPause={nothing} onResume={nothing} onCompact={nothing} onStop={nothing} />
);

const done = (props: Partial<Parameters<typeof QuickDone>[0]>) => <QuickDone side="send" receipt={RECEIPT_WHOLE} files={OFFER} onDone={nothing} {...props} />;

export const samples: Sample[] = [
  { id: "Q01-default", title: "Quick Send home", size: "everyday", render: () => <QuickHome onSend={nothing} onReceive={nothing} onHistory={nothing} onBack={nothing} /> },
  { id: "Q01-receiving", title: "Quick Send home, ready to receive", size: "everyday", render: () => <QuickHome receiving={BLUE_FOX} onSend={nothing} onReceive={nothing} onStopReceiving={nothing} onHistory={nothing} onBack={nothing} /> },
  { id: "Q01-no-nearby", title: "Quick Send home, no laptop found nearby", size: "everyday", render: () => <QuickHome receiving={BLUE_FOX} noNearby onSend={nothing} onReceive={nothing} onStopReceiving={nothing} onHistory={nothing} onBack={nothing} /> },
  { id: "Q01-not-connected", title: "Quick Send home, engine not connected yet", size: "everyday", render: () => <QuickHome notice onSend={nothing} onReceive={nothing} onHistory={nothing} onBack={nothing} /> },

  { id: "Q02-empty", title: "Choose files, nothing chosen", size: "everyday", render: () => <Pick items={[]} /> },
  { id: "Q02-chosen", title: "Choose files, three chosen", size: "everyday", render: () => <Pick items={PICKED} /> },
  { id: "Q02-dragging", title: "Choose files, a drag is over the window", size: "everyday", render: () => <Pick items={PICKED} dragging /> },
  {
    id: "Q02-problems",
    title: "Choose files, online-only, protected and missing items",
    size: "everyday",
    render: () => (
      <Pick
        items={[
          PICKED[0],
          { id: "p4", name: "Holiday video.mov", kind: "file", included: true, bytes: null, files: null, problem: "online-only" },
          { id: "p5", name: "Tax papers", kind: "folder", included: true, bytes: null, files: null, problem: "unreadable" },
          { id: "p6", name: "Old notes.txt", kind: "file", included: true, bytes: null, files: null, problem: "missing" },
        ]}
      />
    ),
  },

  { id: "Q12-copying", title: "Send progress", size: "everyday", render: () => progress({}) },
  { id: "Q12-paused", title: "Send progress, paused", size: "everyday", render: () => progress({ phase: "paused" }) },
  { id: "Q12-checking", title: "Send progress, checking", size: "everyday", render: () => progress({ phase: "checking", filesDone: 4, bytesDone: 7_800_000, current: null }) },
  { id: "Q12-needs-action", title: "Send progress, something needs action", size: "everyday", render: () => progress({ phase: "needs-action", filesDone: 3, failed: 1, bytesDone: 5_000_000, current: null, speed: null }) },
  { id: "Q12-complete", title: "Send progress, completed", size: "everyday", render: () => progress({ phase: "complete", filesDone: 4, bytesDone: 7_800_000, current: null, speed: null }) },
  { id: "Q12-lost", title: "Send progress, connection dropped", size: "everyday", render: () => progress({ phase: "lost", speed: null }) },

  { id: "Q14-sent", title: "Done, sender, everything arrived", size: "everyday", render: () => done({}) },
  { id: "Q14-here", title: "Done, receiver, everything arrived", size: "everyday", render: () => done({ side: "receive", folder: "Downloads", onOpenFolder: nothing }) },
  { id: "Q14-partial-sender", title: "Done, sender, a file did not arrive", size: "everyday", render: () => done({ receipt: RECEIPT_PARTIAL, onSendMissing: nothing }) },
  { id: "Q14-problems-receiver", title: "Done, receiver, a file did not arrive and one needs a step", size: "everyday", render: () => done({ side: "receive", receipt: RECEIPT_PROBLEMS, folder: "Downloads", onOpenFolder: nothing }) },
  { id: "Q14-stopped", title: "Done, the send was stopped", size: "everyday", render: () => done({ stopped: true, receipt: RECEIPT_PARTIAL, onSendMissing: nothing }) },
  { id: "Q14-open-failed", title: "Done, receiver, the folder could not be opened", size: "everyday", render: () => done({ side: "receive", folder: "Downloads", onOpenFolder: nothing, openFailed: true }) },
  { id: "Q14-history", title: "Done, opened again from history", size: "everyday", render: () => <QuickDone side="receive" receipt={RECEIPT_PARTIAL} onDone={nothing} onBack={nothing} /> },

  { id: "Q20-stop", title: "Stop this send", size: "everyday", render: () => <QuickStop open onConfirm={nothing} onClose={nothing} /> },
  { id: "Q20-finished", title: "Stop this send, already finished", size: "everyday", render: () => <QuickStop open finished onConfirm={nothing} onClose={nothing} /> },

  { id: "F08-history", title: "My PCTwin history", size: "everyday", render: () => <History entries={HISTORY} onOpen={nothing} onContinue={nothing} onExport={nothing} onStart={nothing} onBack={nothing} /> },
  { id: "F08-exported", title: "My PCTwin history, exported", size: "everyday", render: () => <History entries={HISTORY} exported={{ kind: "saved", path: "Documents/PCTwin history.json" }} onOpen={nothing} onContinue={nothing} onExport={nothing} onStart={nothing} onBack={nothing} /> },
  { id: "F08-export-failed", title: "My PCTwin history, export failed", size: "everyday", render: () => <History entries={HISTORY} exported={{ kind: "failed" }} onOpen={nothing} onContinue={nothing} onExport={nothing} onStart={nothing} onBack={nothing} /> },
  { id: "F08-empty", title: "My PCTwin history, nothing yet", size: "everyday", render: () => <History entries={[]} onOpen={nothing} onContinue={nothing} onExport={nothing} onStart={nothing} onBack={nothing} /> },
];
