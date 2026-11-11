import type { Contrat } from "../types/gesco";
import { transportTauri } from "./tauri";

import { ErreurApi } from "./erreur";

export { ErreurApi };

/** Nom d'une commande du contrat généré depuis Rust (ADR-007). */
export type NomCommande = keyof Contrat;
export type Entree<N extends NomCommande> = Contrat[N]["entree"];
export type Sortie<N extends NomCommande> = Contrat[N]["sortie"];

// Une seule implémentation active (spécification §12.2) ; le transport HTTP est déclaré dans
// `http.ts` pour le mode réseau local de la phase 11.
const transport = transportTauri;
let jeton: string | null = null;

/** Jeton de la session ouverte sur ce poste, joint à chaque commande. */
export function definirJeton(valeur: string | null): void {
  jeton = valeur;
}

/** État de la liaison d'un poste relié, tel que la dernière commande le montre : le poste
 *  principal a répondu, il est injoignable, ou il refuse ce poste (révoqué, autre version, autre
 *  empreinte). */
export type EtatLiaison = "etablie" | "coupee" | "refusee";

const CLE_INJOIGNABLE = "reseau.poste_principal_injoignable";
const CLES_DE_REFUS: readonly string[] = ["reseau.poste_inconnu", "reseau.version_differente", "reseau.empreinte_differente"];

function etatDeLiaison(erreur: unknown): EtatLiaison {
  if (!(erreur instanceof ErreurApi)) return "etablie";
  if (erreur.cle === CLE_INJOIGNABLE) return "coupee";
  return CLES_DE_REFUS.includes(erreur.cle) ? "refusee" : "etablie";
}

let suiviLiaison: ((etat: EtatLiaison, commande: string) => void) | null = null;

/** Abonne l'interface à l'état de la liaison d'un poste relié, après chaque commande. */
export function surEtatDeLiaison(fonction: ((etat: EtatLiaison, commande: string) => void) | null): void {
  suiviLiaison = fonction;
}

/** Seul point d'accès du frontend aux données de l'école (R-38) : aucun composant n'appelle
 *  `invoke`. */
export async function appeler<N extends NomCommande>(nom: N, entree: Entree<N>): Promise<Sortie<N>> {
  try {
    // Le contrat est généré depuis Rust et la frontière valide l'entrée : la sortie a ce type.
    const sortie = (await transport.appeler(nom, jeton, entree)) as Sortie<N>;
    suiviLiaison?.("etablie", nom);
    return sortie;
  } catch (erreur) {
    // Un refus du noyau est une réponse : seule la commande qui n'arrive pas dit la coupure.
    suiviLiaison?.(etatDeLiaison(erreur), nom);
    throw erreur;
  }
}
