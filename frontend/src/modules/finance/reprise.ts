// Calculs de l'écran « Payé avant GESCO » et de la section « Frais » de la fiche : lecture de la
// case, chiffres affichés après une saisie, préremplissage du remboursement d'un excédent.
// Le noyau Rust décide ; ces calculs ne servent qu'à montrer et à préparer la demande.
import type { DemandeDepense, LigneRepriseAvantGesco } from "../../types/gesco";

/** Lecture d'une case : vide, un montant entier (zéro compris), ou illisible. */
export type SaisieReprise = { sorte: "vide" } | { sorte: "montant"; montant: number } | { sorte: "illisible" };

/** Lit la case comme le noyau : « 75000 », « 75 000 », « 75 000 F », « 75.000 » ; pas de
 *  décimales ni de signe. */
export function lireSaisieReprise(texte: string): SaisieReprise {
  let sansFranc = texte.trim().toLowerCase();
  for (const suffixe of ["fcfa", "cfa", "f"]) {
    if (sansFranc.endsWith(suffixe)) {
      sansFranc = sansFranc.slice(0, -suffixe.length).trimEnd();
      break;
    }
  }
  const chiffres = sansFranc.replace(/[ \u00a0\u202f.]/g, "");
  if (chiffres === "") return { sorte: "vide" };
  if (!/^\d+$/.test(chiffres)) return { sorte: "illisible" };
  const montant = Number.parseInt(chiffres, 10);
  return Number.isSafeInteger(montant) ? { sorte: "montant", montant } : { sorte: "illisible" };
}

/** Ce que la saisie demande : rien (inchangé), enregistrer un montant, ou annuler la reprise
 *  (vide ou zéro sur une ligne qui en a une, ce qui ouvre la demande du motif). */
export type GesteReprise = { geste: "rien" } | { geste: "enregistrer"; montant: number } | { geste: "annuler" } | { geste: "illisible" };

export function gesteDeLaSaisie(texte: string, actuelle: number): GesteReprise {
  const lue = lireSaisieReprise(texte);
  if (lue.sorte === "illisible") return { geste: "illisible" };
  const montant = lue.sorte === "vide" ? 0 : lue.montant;
  if (montant === actuelle) return { geste: "rien" };
  if (montant === 0) return actuelle === 0 ? { geste: "rien" } : { geste: "annuler" };
  return { geste: "enregistrer", montant };
}

/** Chiffres d'une ligne si la reprise prenait ce montant, avant la réponse du noyau : la reprise
 *  remplit d'abord ce qui reste dû, le surplus est à rembourser (moins ce qui est déjà rendu). */
export function apercuLigne(ligne: Pick<LigneRepriseAvantGesco, "du" | "paye_gesco" | "rembourse">, montant: number): { reste: number; a_rembourser: number } {
  const utilisable = Math.max(0, montant - ligne.rembourse);
  const doit = Math.max(0, ligne.du - ligne.paye_gesco);
  const impute = Math.min(utilisable, doit);
  return { reste: doit - impute, a_rembourser: utilisable - impute };
}

/** Demande de dépense préremplie pour rembourser l'excédent payé avant GESCO : catégorie
 *  « Remboursements aux parents », bénéficiaire le payeur, montant l'excédent, libellé nommant
 *  l'élève, inscription liée. Le montant se baisse à l'écran, jamais au-delà de l'excédent. */
export function remboursementPrerempli({
  inscriptionId,
  eleve,
  payeur,
  aRembourser,
  categorieId,
  libelle,
}: {
  inscriptionId: string;
  eleve: string;
  payeur: string;
  aRembourser: number;
  categorieId: string;
  libelle: (eleve: string) => string;
}): DemandeDepense {
  return {
    date: "",
    categorie_id: categorieId,
    libelle: libelle(eleve),
    beneficiaire: payeur,
    montant: aRembourser,
    paiement: null,
    inscription_id: inscriptionId,
  };
}

/** Code stable de la catégorie livrée « Remboursements aux parents ». */
export const CATEGORIE_REMBOURSEMENT = "remboursement_parent";

/** Jour qui suit une date `AAAA-MM-JJ` : la date d'arrêt est la veille de la première caisse,
 *  l'écran dit « avant le » jour de la première caisse. */
export function lendemain(iso: string): string {
  const [a, m, j] = iso.split("-").map(Number);
  const jour = new Date(Date.UTC(a ?? 1970, (m ?? 1) - 1, (j ?? 1) + 1));
  return jour.toISOString().slice(0, 10);
}

function simplifier(texte: string): string {
  return texte.normalize("NFD").replace(/[\u0300-\u036f]/g, "").toLowerCase().trim();
}

/** Lignes dont le nom ou le matricule contient la recherche, sans tenir compte des accents. */
export function filtrerLignes<L extends { eleve: string; matricule: string }>(lignes: readonly L[], recherche: string): L[] {
  const cherche = simplifier(recherche);
  if (cherche === "") return [...lignes];
  return lignes.filter((l) => simplifier(l.eleve).includes(cherche) || simplifier(l.matricule).includes(cherche));
}
