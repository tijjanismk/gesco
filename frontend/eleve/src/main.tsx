import "./styles.css";

import { StrictMode } from "react";
import { createRoot } from "react-dom/client";

import { App } from "./App";

const racine = document.getElementById("racine");
if (racine !== null) {
  createRoot(racine).render(
    <StrictMode>
      <App />
    </StrictMode>,
  );
}
