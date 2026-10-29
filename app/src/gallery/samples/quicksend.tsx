/** Samples: Quick Send and My PCTwin history (F08), each screen in each state, from made-up data. */
import { useState } from "react";
import { BLUE_FOX, GREEN_OWL, HISTORY, INCOMING, KNOWN, NOTE_LINK, NOTE_TEXT, OFFER, PICKED, PROGRESS, RECEIPT_PARTIAL, RECEIPT_PROBLEMS, RECEIPT_WHOLE, REMEMBER_CHOICES, WAITING } from "../../quicksend.fixtures";
import KnownList from "../../screens/QsKnown";
import { KnownIncomingAsk } from "../../screens/QsKnownIncoming";
import { NoteCompose, NoteReceived, NoteSent } from "../../screens/QsNote";
import QsRecipients from "../../screens/QsRecipients";
import { RememberOffer } from "../../screens/QsRemember";
import { WaitingList } from "../../screens/QsWaiting";
import type { PickedItem } from "../../quicksend";
import History from "../../screens/History";
import QuickDone from "../../screens/QuickDone";
import QuickHome from "../../screens/QuickHome";
import QuickPick from "../../screens/QuickPick";
import QuickProgress from "../../screens/QuickProgress";
import QuickStop from "../../screens/QuickStop";
import QsSends from "../../screens/QsSends";
import OpenSendActions from "../../screens/QsOpenSend";
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

const known = { onRename: nothing, onForget: nothing, onAllowFolder: nothing, onRevokeFolder: nothing, onRenewFolder: nothing, onNextFolder: nothing, onClearNextFolder: nothing, onSend: nothing, onBack: nothing };
const to = { onToggle: nothing, onWhen: nothing, onDays: nothing, onCode: nothing, onSendNow: nothing, onApprove: nothing, onBack: nothing };
const ask = { onChoose: nothing, onNextTime: nothing, onDecline: nothing, onSave: nothing };
const DOWNLOADS = { name: "Downloads", path: "C:\\Users\\Ada\\Downloads", freeBytes: 80_000_000_000 };

const progress = (patch: Partial<typeof PROGRESS>) => (
  <QuickProgress from={BLUE_FOX} to={GREEN_OWL} progress={{ ...PROGRESS, ...patch }} onPause={nothing} onResume={nothing} onCompact={nothing} onStop={nothing} />
);

const done = (props: Partial<Parameters<typeof QuickDone>[0]>) => <QuickDone side="send" receipt={RECEIPT_WHOLE} files={OFFER} onDone={nothing} {...props} />;

export const samples: Sample[] = [
  { id: "Q01-default", title: "Quick Send home", size: "everyday", render: () => <QuickHome onSend={nothing} onReceive={nothing} onBack={nothing} /> },
  { id: "Q01-receiving", title: "Quick Send home, ready to receive", size: "everyday", render: () => <QuickHome receiving={BLUE_FOX} onSend={nothing} onReceive={nothing} onStopReceiving={nothing} onBack={nothing} /> },
  { id: "Q01-no-nearby", title: "Quick Send home, no laptop found nearby", size: "everyday", render: () => <QuickHome receiving={BLUE_FOX} noNearby onSend={nothing} onReceive={nothing} onStopReceiving={nothing} onBack={nothing} /> },
  { id: "Q01-not-connected", title: "Quick Send home, engine not connected yet", size: "everyday", render: () => <QuickHome notice onSend={nothing} onReceive={nothing} onBack={nothing} /> },

  { id: "Q01-stage1", title: "Quick Send home, with notes, remembered connections and waiting sends", size: "everyday", render: () => <QuickHome onSend={nothing} onReceive={nothing} onNote={nothing} onKnown={nothing} onWaiting={nothing} onBack={nothing} /> },

  { id: "Q24-compose", title: "Send a note or link", size: "everyday", render: () => <NoteCompose text="Wi-Fi: Home" problem={null} onChange={nothing} onContinue={nothing} onBack={nothing} /> },
  { id: "Q24-too-long", title: "Send a note or link, too long", size: "everyday", render: () => <NoteCompose text="..." problem={{ kind: "note-too-long" }} onChange={nothing} onContinue={nothing} onBack={nothing} /> },
  { id: "Q24-sending", title: "Note on its way", size: "everyday", render: () => <NoteSent sent={false} onDone={nothing} /> },
  { id: "Q24-sent", title: "Note arrived", size: "everyday", render: () => <NoteSent sent onDone={nothing} /> },
  { id: "Q24-received-note", title: "A note arrived", size: "everyday", render: () => <NoteReceived note={NOTE_TEXT} onCopy={nothing} onOpen={nothing} onDone={nothing} /> },
  { id: "Q24-received-link", title: "A link arrived from a remembered laptop", size: "everyday", render: () => <NoteReceived note={NOTE_LINK} copy="copied" onCopy={nothing} onOpen={nothing} onDone={nothing} /> },

  { id: "Q21-remember", title: "Remember this connection", size: "everyday", render: () => <RememberOffer fallbackStore={false} connections={[]} onRemember={nothing} /> },
  { id: "Q21-replace", title: "Remember this connection, may replace an earlier one", size: "everyday", render: () => <RememberOffer fallbackStore connections={REMEMBER_CHOICES} suggestedName="Kitchen laptop" onRemember={nothing} /> },
  { id: "Q21-remembered", title: "Remember this connection, remembered", size: "everyday", render: () => <RememberOffer fallbackStore={false} connections={[]} remembered="Ada's laptop" onRemember={nothing} /> },

  { id: "Q27-known", title: "Remembered connections", size: "everyday", render: () => <KnownList connections={KNOWN} {...known} /> },
  { id: "Q27-known-problem", title: "Remembered connections, a folder refused", size: "everyday", render: () => <KnownList connections={KNOWN} problem={{ id: KNOWN[1].id, refusal: { kind: "folder-holds-keys" } }} {...known} /> },
  { id: "Q27-known-empty", title: "Remembered connections, none yet", size: "everyday", render: () => <KnownList connections={[]} {...known} /> },

  { id: "Q08-known", title: "A remembered laptop wants to send files", size: "everyday", render: () => <KnownIncomingAsk incoming={INCOMING} folder={DOWNLOADS} nextTime={false} {...ask} /> },

  { id: "Q04-one", title: "Who should get these, one laptop now", size: "everyday", render: () => <QsRecipients connections={KNOWN} chosen={[KNOWN[0].id]} when="now" days="7" {...to} /> },
  { id: "Q04-several-now", title: "Who should get these, several laptops now", size: "everyday", render: () => <QsRecipients connections={KNOWN} chosen={[KNOWN[0].id, KNOWN[2].id]} when="now" days="7" {...to} /> },
  { id: "Q04-several", title: "Who should get these, several laptops waiting", size: "everyday", render: () => <QsRecipients connections={KNOWN} chosen={[KNOWN[0].id, KNOWN[2].id]} when="later" days="7" {...to} /> },

  { id: "Q18-waiting", title: "Waiting sends, results for each laptop", size: "everyday", render: () => <WaitingList sends={WAITING} now={1_792_400_000} onRenew={nothing} onCancel={nothing} onSend={nothing} onBack={nothing} /> },
  { id: "Q18-ended", title: "Waiting sends, one ended", size: "everyday", render: () => <WaitingList sends={WAITING} now={1_800_000_000} onRenew={nothing} onCancel={nothing} onSend={nothing} onBack={nothing} /> },
  { id: "Q18-empty", title: "Waiting sends, nothing waiting", size: "everyday", render: () => <WaitingList sends={[]} now={1_792_400_000} onRenew={nothing} onCancel={nothing} onSend={nothing} onBack={nothing} /> },

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
  { id: "QS-sends", title: "Your sends: several laptops at once, one finished, one not reached", size: "everyday", render: () => <Sends /> },
  { id: "QS-sends-empty", title: "Your sends: nothing being sent", size: "everyday", render: () => <QsSends open={{ sessions: [], sends: [], results: [] }} peer="Receiving laptop" onPause={nothing} onResume={nothing} onCancel={nothing} onOpen={nothing} onDone={nothing} /> },
];

/** The list of sends: three laptops at once (one paused), one waiting its turn, one result, one laptop not reached. */
function Sends() {
  const sessions = [
    { id: "s1", name: "Ada’s laptop", ending: false },
    { id: "s2", name: null, ending: false },
    { id: "s3", name: "Kitchen desktop", ending: true },
  ];
  return (
    <QsSends
      open={{
        sessions,
        sends: [
          { id: 1, session: "s1", name: "Ada’s laptop", files: 12, bytes: 48_000_000, state: "sending", paused: false },
          { id: 2, session: "s2", name: null, files: 3, bytes: 2_400_000, state: "sending", paused: true },
          { id: 3, session: "s1", name: "Ada’s laptop", files: 1, bytes: 900_000, state: "waiting", paused: false },
        ],
        results: [{ session: "s3", send: 4, name: "Kitchen desktop", receipt: { side: "sender", lines: RECEIPT_PARTIAL.lines, stopped: false, newDates: 0 } }],
      }}
      peer="Receiving laptop"
      notSent={[{ name: "Old desktop", refusal: { kind: "not-nearby" } }]}
      openSend={<OpenSendActions sessions={sessions} peer="Receiving laptop" side="send" onAdd={nothing} onEnd={nothing} />}
      onPause={nothing}
      onResume={nothing}
      onCancel={nothing}
      onOpen={nothing}
      onDone={nothing}
    />
  );
}
