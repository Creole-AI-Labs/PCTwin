/** Samples: choosing what moves (M02 scan progress, M03 overview, M04 folders, M05 destination, M23 plan review). */
import { WINDOW_MODE } from "../../plan";
import * as data from "../../planSamples";
import ChooseFolders from "../../screens/ChooseFolders";
import Destination from "../../screens/Destination";
import Overview from "../../screens/Overview";
import PlanReview from "../../screens/PlanReview";
import ScanProgress from "../../screens/ScanProgress";
import type { Sample } from "../samples";

const nothing = () => undefined;

const scan = (view: Parameters<typeof ScanProgress>[0]["scan"]) => (
  <ScanProgress scan={view} onPause={nothing} onResume={nothing} onContinue={nothing} onAllowAccess={nothing} onBack={nothing} />
);
const folders = (list: typeof data.foldersDefault) => (
  <ChooseFolders folders={list} onToggle={nothing} onAddFolder={nothing} onRemove={nothing} onAllowAccess={nothing} onContinue={nothing} onBack={nothing} />
);
const where = (drives: typeof data.drivesMapped) => (
  <Destination groups={data.groupsMapped} drives={drives} onSetDestination={nothing} onSetAll={nothing} onChooseFolder={nothing} onContinue={nothing} onBack={nothing} />
);
const review = (plan: typeof data.plan, approval: Parameters<typeof PlanReview>[0]["approval"]) => (
  <PlanReview plan={plan} approval={approval} onApprovalChange={nothing} onStart={nothing} onEdit={nothing} onFixPeople={nothing} onBack={nothing} />
);

export const samples: Sample[] = [
  { id: "M02-scanning", title: "Finding your files: scanning", size: WINDOW_MODE.scan, render: () => scan(data.scanning) },
  { id: "M02-pausing", title: "Finding your files: pausing", size: WINDOW_MODE.scan, render: () => scan(data.pausing) },
  { id: "M02-paused", title: "Finding your files: paused", size: WINDOW_MODE.scan, render: () => scan(data.paused) },
  { id: "M02-resumed", title: "Finding your files: resumed", size: WINDOW_MODE.scan, render: () => scan(data.resumed) },
  { id: "M02-partial", title: "Finding your files: partial scan", size: WINDOW_MODE.scan, render: () => scan(data.partial) },
  { id: "M02-done", title: "Finding your files: finished", size: WINDOW_MODE.scan, render: () => scan(data.finished) },
  {
    id: "M03-default",
    title: "Everything you chose, in view",
    size: WINDOW_MODE.overview,
    render: () => <Overview view={data.overview} onOpen={nothing} onContinue={nothing} onBack={nothing} />,
  },
  {
    id: "M03-partial",
    title: "Everything you chose: partial scan",
    size: WINDOW_MODE.overview,
    render: () => <Overview view={data.overviewPartial} onOpen={nothing} onContinue={nothing} onAllowAccess={nothing} onBack={nothing} />,
  },
  { id: "M04-default", title: "Keep what matters to you", size: WINDOW_MODE.files, render: () => folders(data.foldersDefault) },
  { id: "M04-protected", title: "Keep what matters: protected folder", size: WINDOW_MODE.files, render: () => folders(data.foldersProtected) },
  { id: "M04-custom", title: "Keep what matters: folder added", size: WINDOW_MODE.files, render: () => folders(data.foldersCustom) },
  { id: "M05-mapped", title: "Give every file a home: mapped", size: WINDOW_MODE.destination, render: () => where(data.drivesMapped) },
  { id: "M05-missing-drive", title: "Give every file a home: drive missing", size: WINDOW_MODE.destination, render: () => where(data.drivesMissing) },
  { id: "M05-not-enough-space", title: "Give every file a home: not enough space", size: WINDOW_MODE.destination, render: () => where(data.drivesFull) },
  { id: "M23-review", title: "Approve the move: review", size: WINDOW_MODE.review, render: () => review(data.plan, null) },
  {
    id: "M23-approved",
    title: "Approve the move: approved",
    size: WINDOW_MODE.review,
    render: () => review(data.plan, { revision: data.plan.revision, fingerprint: data.plan.fingerprint }),
  },
  {
    id: "M23-changed",
    title: "Approve the move: plan changed",
    size: WINDOW_MODE.review,
    render: () => review(data.planNext, { revision: data.plan.revision, fingerprint: data.plan.fingerprint }),
  },
  { id: "M23-blocked", title: "Approve the move: cannot be approved yet", size: WINDOW_MODE.review, render: () => review({ ...data.plan, blocked: "someone-unmapped" }, null) },
];
