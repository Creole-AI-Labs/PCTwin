/** Samples: Quick Send (Q01 to Q20) and My PCTwin history (F08), each screen in each state, from made-up data. */
import { useState, type ReactNode } from "react";
import { BLUE_FOX, CONFLICTS, GREEN_OWL, HISTORY, NEARBY, OFFER, PICKED, PROGRESS, RECEIPT_PARTIAL, RECEIPT_WHOLE } from "../../quicksend.fixtures";
import type { Conflict, PickedItem } from "../../quicksend";
import History from "../../screens/History";
import QuickApprove from "../../screens/QuickApprove";
import QuickHome from "../../screens/QuickHome";
import QuickIncoming from "../../screens/QuickIncoming";
import QuickKeep from "../../screens/QuickKeep";
import QuickMissing from "../../screens/QuickMissing";
import QuickPick from "../../screens/QuickPick";
import QuickProgress from "../../screens/QuickProgress";
import QuickReceipt from "../../screens/QuickReceipt";
import QuickReview from "../../screens/QuickReview";
import QuickStop from "../../screens/QuickStop";
import QuickWhere from "../../screens/QuickWhere";
import QuickWho from "../../screens/QuickWho";
import type { Sample } from "../samples";

const nothing = () => undefined;

/** A value the sample lets you change, so a tick or a choice can be tried on the screens page. */
function Try<T>({ initial, children }: { initial: T; children: (value: T, set: (next: T) => void) => ReactNode }) {
  const [value, set] = useState(initial);
  return <>{children(value, set)}</>;
}

function Keep({ conflicts, allowReplace = false }: { conflicts: Conflict[]; allowReplace?: boolean }) {
  const [list, setList] = useState(conflicts);
  const [approved, setApproved] = useState(false);
  const change = (path: string, patch: Partial<Conflict>) => {
    setList((prev) => prev.map((c) => (c.path === path ? { ...c, ...patch } : c)));
  };
  return (
    <QuickKeep
      files={OFFER}
      conflicts={list}
      allowReplace={allowReplace}
      approved={approved}
      onChoose={(path, choice) => {
        change(path, { choice, replaceConfirmed: false, changed: false });
      }}
      onConfirmReplace={(path, on) => {
        change(path, { replaceConfirmed: on });
      }}
      onApprovedChange={setApproved}
      onContinue={nothing}
      onBack={nothing}
    />
  );
}

function Where(props: Omit<Parameters<typeof QuickWhere>[0], "confirmed" | "onConfirmedChange" | "onChoose" | "onContinue" | "onBack">) {
  const [confirmed, setConfirmed] = useState(false);
  return <QuickWhere {...props} confirmed={confirmed} onConfirmedChange={setConfirmed} onChoose={nothing} onContinue={nothing} onBack={nothing} />;
}

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

const review = (checked: boolean, issues: Parameters<typeof QuickReview>[0]["issues"] = []) => (
  <Try initial={checked}>
    {(on, set) => <QuickReview files={OFFER} issues={issues} checked={on} onCheckedChange={set} onChange={nothing} onContinue={nothing} onBack={nothing} />}
  </Try>
);

const approve = (receiver: Parameters<typeof QuickApprove>[0]["receiver"], approved: boolean, issues: Parameters<typeof QuickApprove>[0]["issues"] = []) => (
  <Try initial={approved}>
    {(on, set) => <QuickApprove from={BLUE_FOX} to={GREEN_OWL} count={4} size={7_800_000} receiver={receiver} issues={issues} approved={on} onApprovedChange={set} onStart={nothing} onBack={nothing} />}
  </Try>
);

const progress = (patch: Partial<typeof PROGRESS>) => (
  <QuickProgress from={BLUE_FOX} to={GREEN_OWL} progress={{ ...PROGRESS, ...patch }} onPause={nothing} onResume={nothing} onCompact={nothing} onStop={nothing} onReceipt={nothing} />
);

const receipt = (props: Partial<Parameters<typeof QuickReceipt>[0]>) => (
  <QuickReceipt side="send" receipt={RECEIPT_WHOLE} from={BLUE_FOX} to={GREEN_OWL} files={OFFER} onHistory={nothing} onDone={nothing} {...props} />
);

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

  { id: "Q03-default", title: "Check what you are sharing", size: "everyday", render: () => review(false) },
  { id: "Q03-checked", title: "Check what you are sharing, ticked", size: "everyday", render: () => review(true) },
  { id: "Q03-source-changed", title: "Check what you are sharing, a file changed", size: "everyday", render: () => review(true, [{ path: "Budget.xlsx", kind: "changed" }, { path: "Reference photos/beach.jpg", kind: "unreadable" }]) },

  { id: "Q04-nearby", title: "Who should receive it, laptops nearby", size: "everyday", render: () => <QuickWho count={4} size={7_800_000} nearby={NEARBY} onChooseNearby={nothing} onUseCode={nothing} onBack={nothing} /> },
  { id: "Q04-none", title: "Who should receive it, none found nearby", size: "everyday", render: () => <QuickWho count={4} size={7_800_000} nearby={[]} onChooseNearby={nothing} onUseCode={nothing} onBack={nothing} /> },
  {
    id: "Q04-busy-and-same-name",
    title: "Who should receive it, one busy and a repeated name",
    size: "everyday",
    render: () => <QuickWho count={4} size={7_800_000} nearby={[{ ...NEARBY[0], busy: true }, { ...NEARBY[1], sameLabelNearby: true }]} onChooseNearby={nothing} onUseCode={nothing} onBack={nothing} />,
  },

  { id: "Q08-default", title: "Incoming offer", size: "everyday", render: () => <QuickIncoming from={BLUE_FOX} to={GREEN_OWL} files={OFFER} onAccept={nothing} onDecline={nothing} onDone={nothing} /> },
  { id: "Q08-changed", title: "Incoming offer, the sender changed the files", size: "everyday", render: () => <QuickIncoming from={BLUE_FOX} to={GREEN_OWL} files={OFFER} changed onAccept={nothing} onDecline={nothing} onDone={nothing} /> },
  { id: "Q08-declined", title: "Incoming offer, declined", size: "everyday", render: () => <QuickIncoming from={BLUE_FOX} to={GREEN_OWL} files={OFFER} state="declined" onAccept={nothing} onDecline={nothing} onDone={nothing} /> },
  { id: "Q08-gone", title: "Incoming offer, the sender is no longer there", size: "everyday", render: () => <QuickIncoming from={BLUE_FOX} to={GREEN_OWL} files={OFFER} state="gone" onAccept={nothing} onDecline={nothing} onDone={nothing} /> },

  { id: "Q09-default", title: "Where the files land", size: "everyday", render: () => <Where folder="Documents/PCTwin received" free={86_000_000_000} needed={7_800_000} /> },
  { id: "Q09-no-folder", title: "Where the files land, no folder yet", size: "everyday", render: () => <Where folder={null} free={null} needed={7_800_000} /> },
  { id: "Q09-no-space", title: "Where the files land, not enough space", size: "everyday", render: () => <Where folder="Desktop/Received files" free={4_000_000} needed={7_800_000} /> },
  { id: "Q09-space-unknown", title: "Where the files land, space could not be measured", size: "everyday", render: () => <Where folder="Documents/PCTwin received" free={null} needed={7_800_000} /> },
  { id: "Q09-unavailable", title: "Where the files land, folder not available", size: "everyday", render: () => <Where folder="E:/Received" free={null} needed={7_800_000} problem="unavailable" /> },
  { id: "Q09-denied", title: "Where the files land, not allowed to save there", size: "everyday", render: () => <Where folder="C:/Windows" free={null} needed={7_800_000} problem="no-permission" /> },

  { id: "Q10-default", title: "Keep your existing work safe", size: "everyday", render: () => <Keep conflicts={CONFLICTS} /> },
  { id: "Q10-no-conflicts", title: "Keep your existing work safe, no clashes", size: "everyday", render: () => <Keep conflicts={[]} /> },
  { id: "Q10-target-changed", title: "Keep your existing work safe, a file changed in the folder", size: "everyday", render: () => <Keep conflicts={[{ ...CONFLICTS[0], choice: null, changed: true }, CONFLICTS[1]]} /> },
  { id: "Q10-replace-review", title: "Keep your existing work safe, replace needs its own confirmation", size: "everyday", render: () => <Keep allowReplace conflicts={[{ ...CONFLICTS[0], choice: "replace" }, { ...CONFLICTS[1], choice: "skip" }]} /> },

  { id: "Q11-waiting", title: "Approve this send, waiting for the receiver", size: "everyday", render: () => approve("waiting", false) },
  { id: "Q11-ready", title: "Approve this send, receiver accepted", size: "everyday", render: () => approve("accepted", true) },
  { id: "Q11-source-changed", title: "Approve this send, a file changed", size: "everyday", render: () => approve("accepted", false, [{ path: "Budget.xlsx", kind: "changed" }]) },
  { id: "Q11-cleared", title: "Approve this send, approval cleared by a change", size: "everyday", render: () => approve("cleared", false) },
  { id: "Q11-withdrawn", title: "Approve this send, the receiver no longer accepts", size: "everyday", render: () => approve("withdrawn", false) },

  { id: "Q12-copying", title: "Send progress", size: "everyday", render: () => progress({}) },
  { id: "Q12-paused", title: "Send progress, paused", size: "everyday", render: () => progress({ phase: "paused" }) },
  { id: "Q12-checking", title: "Send progress, checking", size: "everyday", render: () => progress({ phase: "checking", filesDone: 4, bytesDone: 7_800_000, current: null }) },
  { id: "Q12-needs-action", title: "Send progress, something needs action", size: "everyday", render: () => progress({ phase: "needs-action", filesDone: 3, failed: 1, bytesDone: 5_000_000, current: null, speed: null }) },
  { id: "Q12-complete", title: "Send progress, completed", size: "everyday", render: () => progress({ phase: "complete", filesDone: 4, bytesDone: 7_800_000, current: null, speed: null }) },
  { id: "Q12-lost", title: "Send progress, connection dropped", size: "everyday", render: () => progress({ phase: "lost", speed: null }) },

  { id: "Q14-whole-sender", title: "Handover receipt, sender, whole", size: "expanded", render: () => receipt({}) },
  { id: "Q14-whole-receiver", title: "Handover receipt, receiver, whole", size: "expanded", render: () => receipt({ side: "receive", onAcknowledge: nothing }) },
  { id: "Q14-partial-sender", title: "Handover receipt, sender, partial", size: "expanded", render: () => receipt({ receipt: RECEIPT_PARTIAL, onSendMissing: nothing }) },
  { id: "Q14-partial-receiver", title: "Handover receipt, receiver, partial", size: "expanded", render: () => receipt({ side: "receive", receipt: RECEIPT_PARTIAL, onAcknowledge: nothing }) },
  { id: "Q14-acknowledged", title: "Handover receipt, receiver, acknowledged", size: "expanded", render: () => receipt({ side: "receive", acknowledgedAt: "2026-10-15T14:10:00Z" }) },
  { id: "Q14-offline", title: "Handover receipt, the other laptop is offline", size: "expanded", render: () => receipt({ peerOffline: true }) },
  { id: "Q14-stopped", title: "Handover receipt, stopped", size: "expanded", render: () => receipt({ stopped: true, receipt: RECEIPT_PARTIAL, onSendMissing: nothing }) },

  { id: "Q19-default", title: "Send the missing pieces", size: "everyday", render: () => <Try initial={false}>{(on, set) => <QuickMissing receipt={RECEIPT_PARTIAL} approved={on} onApprovedChange={set} onContinue={nothing} onNewSend={nothing} onBack={nothing} />}</Try> },
  { id: "Q19-complete", title: "Send the missing pieces, nothing missing", size: "everyday", render: () => <QuickMissing receipt={RECEIPT_WHOLE} approved={false} onApprovedChange={nothing} onContinue={nothing} onNewSend={nothing} onBack={nothing} /> },
  { id: "Q19-source-changed", title: "Send the missing pieces, a file changed", size: "everyday", render: () => <QuickMissing receipt={RECEIPT_PARTIAL} issues={[{ path: "Reference photos/beach.jpg", kind: "changed" }]} approved={false} onApprovedChange={nothing} onContinue={nothing} onNewSend={nothing} onBack={nothing} /> },
  { id: "Q19-no-receipt", title: "Send the missing pieces, no receipt found", size: "everyday", render: () => <QuickMissing receipt={null} approved={false} onApprovedChange={nothing} onContinue={nothing} onNewSend={nothing} onBack={nothing} /> },

  { id: "Q20-stop", title: "Stop this send", size: "everyday", render: () => <QuickStop open kind="stop" onConfirm={nothing} onClose={nothing} /> },
  { id: "Q20-decline", title: "Decline these files", size: "everyday", render: () => <QuickStop open kind="decline" onConfirm={nothing} onClose={nothing} /> },
  { id: "Q20-finished", title: "Stop this send, already finished", size: "everyday", render: () => <QuickStop open kind="stop" finished onConfirm={nothing} onClose={nothing} /> },

  { id: "F08-history", title: "My PCTwin history", size: "everyday", render: () => <History entries={HISTORY} onOpen={nothing} onContinue={nothing} onExport={nothing} onStart={nothing} onBack={nothing} /> },
  { id: "F08-exported", title: "My PCTwin history, exported", size: "everyday", render: () => <History entries={HISTORY} exported={{ kind: "saved", path: "Documents/PCTwin history.json" }} onOpen={nothing} onContinue={nothing} onExport={nothing} onStart={nothing} onBack={nothing} /> },
  { id: "F08-export-failed", title: "My PCTwin history, export failed", size: "everyday", render: () => <History entries={HISTORY} exported={{ kind: "failed" }} onOpen={nothing} onContinue={nothing} onExport={nothing} onStart={nothing} onBack={nothing} /> },
  { id: "F08-empty", title: "My PCTwin history, nothing yet", size: "everyday", render: () => <History entries={[]} onOpen={nothing} onContinue={nothing} onExport={nothing} onStart={nothing} onBack={nothing} /> },
];
