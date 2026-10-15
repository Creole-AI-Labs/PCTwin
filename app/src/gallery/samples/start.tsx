/** Samples: Welcome (F01) and Old or new laptop (F06). */
import type { Sample } from "../samples";
import Welcome from "../../screens/Welcome";
import WhichLaptop from "../../screens/WhichLaptop";

const nothing = () => undefined;

export const samples: Sample[] = [
  { id: "F01-default", title: "Welcome", size: "everyday", render: () => <Welcome onStart={nothing} onSoon={nothing} /> },
  { id: "F06-default", title: "Which laptop is this?", size: "everyday", render: () => <WhichLaptop onBack={nothing} onChoose={nothing} /> },
];
