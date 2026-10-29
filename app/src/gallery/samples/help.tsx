/** Samples: Help, then Feedback (R12), and the safe-stop page reached from Help or an update offer. */
import type { Sample } from "../samples";
import Feedback from "../../screens/Feedback";
import SafeStop from "../../screens/SafeStop";

const nothing = () => undefined;

export const samples: Sample[] = [
  { id: "R12-default", title: "Feedback", size: "everyday", render: () => <Feedback onBack={nothing} /> },
  { id: "S06-waiting", title: "Update or remove PCTwin: waiting for a safe moment", size: "everyday", render: () => <SafeStop onBack={nothing} shown={{ safe: false, busy: ["moving", "rebuilding", "app-data", "undoing"] }} /> },
  { id: "S06-safe", title: "Update or remove PCTwin: nothing is going on", size: "everyday", render: () => <SafeStop onBack={nothing} shown={{ safe: true, busy: [] }} /> },
];
