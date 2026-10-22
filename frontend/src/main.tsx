import { QueryClientProvider } from "@tanstack/react-query";
import { StrictMode } from "react";
import { createRoot } from "react-dom/client";

import { Application } from "./app/Application";
import { GardeErreur, noterPanne } from "./composants/GardeErreur";
import { clientRequetes } from "./app/requetes";
import { FournisseurSession } from "./app/session";
import "./styles.css";

// Les erreurs que React ne voit pas (promesses, gestionnaires) se notent aussi au journal technique.
window.addEventListener("error", (e) => noterPanne(`${e.message} ${e.filename}:${String(e.lineno)}`));
window.addEventListener("unhandledrejection", (e) => noterPanne(`promesse rejetée : ${String(e.reason)}`));

const racine = document.getElementById("racine");
if (racine !== null) {
  createRoot(racine).render(
    <StrictMode>
      <QueryClientProvider client={clientRequetes}>
        <FournisseurSession>
          <GardeErreur>
            <Application />
          </GardeErreur>
        </FournisseurSession>
      </QueryClientProvider>
    </StrictMode>,
  );
}
