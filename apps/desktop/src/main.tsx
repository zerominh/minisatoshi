import React from "react";
import ReactDOM from "react-dom/client";
import { HashRouter } from "react-router-dom";
import App from "./App";
import { LocaleProvider } from "./i18n/LocaleContext";
import { getLocale } from "./lib/settings";
import "./styles.css";

document.documentElement.lang = getLocale() === "vi" ? "vi" : "en";

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>
    <HashRouter>
      <LocaleProvider>
        <App />
      </LocaleProvider>
    </HashRouter>
  </React.StrictMode>,
);
