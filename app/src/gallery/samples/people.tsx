/** Samples: Who is moving (M06), Before you start, and Get help, from made-up people and codes. */
import type { Sample } from "../samples";
import type { Account, BeforeItem, Person } from "../../people";
import BeforeYouStart from "../../screens/BeforeYouStart";
import GetHelp from "../../screens/GetHelp";
import WhoIsMoving from "../../screens/WhoIsMoving";

const nothing = () => undefined;

const accounts: Account[] = [
  { id: "a1", name: "Ada" },
  { id: "a2", name: "Studio" },
];

const you: Person = { id: "p1", name: "Ada", isYou: true, state: "ready", destination: { kind: "undecided" }, suggestion: "a1" };
const maya: Person = { id: "p2", name: "Maya", isYou: false, state: "waiting-for-owner", destination: { kind: "undecided" } };
const guest: Person = { id: "p3", name: "Guest", isYou: false, state: "ready", destination: { kind: "leave-out" } };

function people(list: Person[], shared: "shared" | "person" | "leave-out" = "shared") {
  return (
    <WhoIsMoving people={list} accounts={accounts} shared={shared} onBack={nothing} onChoose={nothing} onChooseShared={nothing} onContinue={nothing} />
  );
}

const tasks: BeforeItem[] = [
  { id: "t1", kind: "online", done: false },
  { id: "t2", kind: "passwords", done: true },
  { id: "t3", kind: "licences", done: false },
  { id: "t4", kind: "codes", done: false },
];

const help = (h: Parameters<typeof GetHelp>[0]["help"], canShare = true) => (
  <GetHelp help={h} canShare={canShare} onBack={nothing} onShare={nothing} onRetry={nothing} />
);

export const samples: Sample[] = [
  { id: "M06-just-me", title: "Who is moving: just me", size: "expanded", render: () => people([you]) },
  {
    id: "M06-several",
    title: "Who is moving: one waiting for its owner",
    size: "expanded",
    render: () => people([maya, { ...you, destination: { kind: "existing", accountId: "a1" } }, guest]),
  },
  {
    id: "M06-changed",
    title: "Who is moving: a mapping changed by the person",
    size: "expanded",
    render: () => people([{ ...you, destination: { kind: "new" }, changedByYou: true }, maya], "person"),
  },
  { id: "B01-default", title: "Before you start", size: "everyday", render: () => <BeforeYouStart items={tasks} onBack={nothing} onToggle={nothing} onContinue={nothing} /> },
  { id: "B01-empty", title: "Before you start: nothing to do", size: "everyday", render: () => <BeforeYouStart items={[]} onBack={nothing} onToggle={nothing} onContinue={nothing} /> },
  { id: "H01-making", title: "Get help: making the code", size: "everyday", render: () => help({ status: "making" }) },
  { id: "H01-ready", title: "Get help: code ready", size: "everyday", render: () => help({ status: "ready", code: "PCT-7F3K-21", included: ["problem", "step", "versions"] }) },
  { id: "H01-no-share", title: "Get help: no share sheet", size: "everyday", render: () => help({ status: "ready", code: "PCT-7F3K-21", included: ["problem", "step", "versions"] }, false) },
  { id: "H01-failed", title: "Get help: could not make a code", size: "everyday", render: () => help({ status: "failed" }) },
];
