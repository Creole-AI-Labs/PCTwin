import { LocalizationProvider } from "@fluent/react";
import React from "react";
import ReactDOM from "react-dom/client";
import { localization } from "../i18n";
import "../styles.css";
import Gallery from "./Gallery";

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>
    <LocalizationProvider l10n={localization("en")}>
      <Gallery />
    </LocalizationProvider>
  </React.StrictMode>,
);
