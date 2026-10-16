import { QueryClientProvider } from "@tanstack/react-query";
import { StrictMode } from "react";
import { createRoot } from "react-dom/client";

import { Application } from "./app/Application";
import { clientRequetes } from "./app/requetes";
import { FournisseurSession } from "./app/session";
import "./styles.css";

const racine = document.getElementById("racine");
if (racine !== null) {
  createRoot(racine).render(
    <StrictMode>
      <QueryClientProvider client={clientRequetes}>
        <FournisseurSession>
          <Application />
        </FournisseurSession>
      </QueryClientProvider>
    </StrictMode>,
  );
}
