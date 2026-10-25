import React from "react";
import ReactDOM from "react-dom/client";
import App from "./App";
import { applyToDocument } from "./motion/preference";

// The motion setting is read before the first frame is drawn.
applyToDocument();

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>
    <App />
  </React.StrictMode>,
);
