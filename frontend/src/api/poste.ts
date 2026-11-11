import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";

import { versErreurApi } from "./erreur";

/** Ce que l'écran sait du poste avant toute session : il a sa base, ou il est relié au poste
 *  principal de l'école. Ces commandes sont celles du poste lui-même, hors du contrat de l'école :
 *  un poste relié y répond même quand le poste principal est éteint. */
export interface InfoPoste {
  relie: boolean;
  /** Nom du poste : celui de l'appairage pour un poste relié, celui de l'ordinateur sinon. */
  nom: string;
  nom_ordinateur: string;
  /** Adresse du poste principal, pour un poste relié. */
  adresse: string | null;
  version: string;
}

/** Ce que le poste principal montre de lui avant qu'on s'y relie. */
export interface ApercuPrincipal {
  adresse: string;
  version: string;
  /** À comparer avec l'écran « Postes de l'école » du poste principal. */
  empreinte_courte: string;
  meme_version: boolean;
}

async function demander<T>(commande: string, entree?: Record<string, unknown>): Promise<T> {
  try {
    return await invoke<T>(commande, entree);
  } catch (erreur) {
    throw versErreurApi(erreur);
  }
}

export const lirePoste = (): Promise<InfoPoste> => demander("poste_lire");

export const apercevoirPrincipal = (adresse: string): Promise<ApercuPrincipal> =>
  demander("poste_principal_apercevoir", { adresse });

export const rejoindrePrincipal = (code: string, nom: string): Promise<InfoPoste> =>
  demander("poste_rejoindre", { code, nom });

export const quitterPrincipal = (): Promise<InfoPoste> => demander("poste_quitter");

/** Vérifie que le poste principal répond ; rejette avec l'erreur de la liaison sinon. */
export const verifierLiaison = (): Promise<null> => demander("liaison_verifier");

/** Ferme GESCO sur le poste principal malgré les postes connectés. */
export const fermerQuandMeme = (): Promise<null> => demander("fermer_quand_meme");

/** S'abonne à la demande de fermeture du poste principal quand des postes reliés ont une session
 *  ouverte ; la fonction reçoit leur nombre. Rend de quoi se désabonner. */
export function surFermetureDemandee(fonction: (postes: number) => void): () => void {
  const abonnement = listen<number>("fermeture-demandee", (evenement) => fonction(evenement.payload));
  return () => void abonnement.then((arreter) => arreter()).catch(() => undefined);
}
