import type { Contrat } from "../types/gesco";
import { transportTauri } from "./tauri";

export { ErreurApi } from "./erreur";

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

/** Seul point d'accès du frontend au noyau (R-38) : aucun composant n'appelle `invoke`. */
export async function appeler<N extends NomCommande>(nom: N, entree: Entree<N>): Promise<Sortie<N>> {
  // Le contrat est généré depuis Rust et la frontière valide l'entrée : la sortie a ce type.
  return (await transport.appeler(nom, jeton, entree)) as Sortie<N>;
}
