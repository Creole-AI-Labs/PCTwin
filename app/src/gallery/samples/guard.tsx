/** Samples: the Phase 2 screens (2.8: M07, M09, M21/M24/M31, M25/H10, X06, X09, X12, T01 to T04, R05, R06), from made-up data. */
import type { Sample } from "../samples";
import type { Projection } from "../../guard";
import * as g from "../../guardSamples";
import { GuardProvider } from "../../screens/GuardParts";
import { GuardScreen } from "../../screens/GuardHost";

/** A screen as the host draws it, with buttons that do nothing (a review page sends no intents). */
function shown(view: Projection) {
  return (
    <GuardProvider value={{ view, send: () => undefined, busy: () => false, refusal: null }}>
      <GuardScreen view={view} />
    </GuardProvider>
  );
}

export const samples: Sample[] = [
  { id: "P29-changes", title: "Before we change anything", size: "expanded", render: () => shown(g.changes) },
  { id: "P29-changes-small", title: "Before we change anything, one change", size: "everyday", render: () => shown(g.changesNothingToDownload) },
  { id: "X06-changed", title: "Only the changed items, to approve again", size: "everyday", render: () => shown(g.changed) },
  { id: "M31-short", title: "Space: new laptop too small, with a proposal and an offload drive", size: "expanded", render: () => shown(g.spaceShort) },
  { id: "M31-offload-away", title: "Space: offload drive approved but not connected", size: "everyday", render: () => shown(g.spaceOffloadAway) },
  { id: "M09-choose", title: "Keep a copy of online-only files?", size: "everyday", render: () => shown(g.localChoosing) },
  { id: "M09-working", title: "Keeping copies, measured", size: "everyday", render: () => shown(g.localWorking) },
  { id: "M09-metered", title: "Keeping copies, paused on a metered connection", size: "everyday", render: () => shown(g.localMetered) },
  { id: "M09-no-space", title: "Keeping copies, paused: old laptop full", size: "everyday", render: () => shown(g.localNoSpace) },
  { id: "M07-owners", title: "Who goes where, across laptops", size: "expanded", render: () => shown(g.owners) },
  { id: "M25-explain", title: "One system approval: why", size: "everyday", render: () => shown(g.permissionExplain) },
  { id: "M25-waiting", title: "Waiting for the system's own prompt", size: "everyday", render: () => shown(g.permissionWaiting) },
  { id: "M25-closed", title: "Prompt closed without an answer", size: "everyday", render: () => shown(g.permissionClosed) },
  { id: "M25-denied", title: "Approval refused; restore point not chosen", size: "everyday", render: () => shown(g.permissionDenied) },
  { id: "M25-given", title: "Approved; restore point not made, protection off", size: "everyday", render: () => shown(g.permissionGiven) },
  { id: "H10-slept", title: "Approval ended by sleep; restore point made", size: "everyday", render: () => shown(g.permissionSlept) },
  { id: "H10-other-session", title: "Approval ended by another session; earlier restore point", size: "everyday", render: () => shown(g.permissionOtherSession) },
  { id: "T01-intro", title: "Rescue: what it can and cannot do", size: "everyday", render: () => shown(g.rescueIntro) },
  { id: "T02-pick", title: "Rescue: pick the drive (target refused)", size: "everyday", render: () => shown(g.rescuePick) },
  { id: "T03-bitlocker", title: "Rescue: BitLocker unlock guidance", size: "everyday", render: () => shown(g.rescueBitLocker) },
  { id: "T03-apple-chip", title: "Rescue: Mac drive tied to its chip", size: "everyday", render: () => shown(g.rescueAppleChip) },
  { id: "T04-salvage", title: "Rescue: what could be read, unreadable list", size: "everyday", render: () => shown(g.rescueSalvage) },
  { id: "X12-partial", title: "Partial report with Try again", size: "everyday", render: () => shown(g.report) },
  { id: "R05-preview", title: "Undo preview", size: "everyday", render: () => shown(g.undoPreview) },
  { id: "R06-done", title: "Undo result", size: "everyday", render: () => shown(g.undoDone) },
  { id: "R06-running", title: "Undo running, total unknown", size: "everyday", render: () => shown(g.undoRunning) },
  { id: "X09-checking", title: "Checking where things got to", size: "everyday", render: () => shown(g.reconciling) },
];
