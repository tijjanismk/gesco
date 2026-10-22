import { Component, type ErrorInfo, type ReactNode } from "react";

import { appeler } from "../api";
import { t } from "../i18n";
import { Bouton } from "./Bouton";

/** Note une panne de l'interface au journal technique du poste ; son propre échec est ignoré. */
export function noterPanne(message: string): void {
  void appeler("journal_technique_noter", { message: message.slice(0, 2000) }).catch(() => undefined);
}

/** Garde de l'interface : une erreur inattendue dans un écran affiche un message clair, au lieu
 *  d'une page blanche ou figée, et se note au journal technique pour l'assistance. */
export class GardeErreur extends Component<{ children: ReactNode }, { panne: boolean }> {
  override state = { panne: false };

  static getDerivedStateFromError(): { panne: boolean } {
    return { panne: true };
  }

  override componentDidCatch(erreur: Error, info: ErrorInfo): void {
    noterPanne(`${erreur.name}: ${erreur.message} ${info.componentStack ?? ""}`);
  }

  override render(): ReactNode {
    if (!this.state.panne) return this.props.children;
    return (
      <div role="alert" className="mx-auto mt-8 flex max-w-2xl flex-col gap-3 p-6 text-sm text-encre">
        <p>{t("erreur.interface")}</p>
        <div>
          <Bouton onClick={() => window.location.reload()}>{t("erreur.recharger")}</Bouton>
        </div>
      </div>
    );
  }
}
