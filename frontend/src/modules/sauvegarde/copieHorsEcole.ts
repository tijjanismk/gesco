import type { Cle } from "../../i18n";
import type { Destination, DossierSynchronise, EtatEnvoi, RythmeHorsEcole, SorteDestination } from "../../types/gesco";

/** Libellé de l'état d'envoi d'une copie hors de l'école. */
export const CLES_ETAT: Record<EtatEnvoi, Cle> = {
  deposee: "sauvegarde.envoi.deposee",
  en_attente: "sauvegarde.envoi.en_attente",
  envoyee: "sauvegarde.envoi.envoyee",
  echec_envoi: "sauvegarde.envoi.echec_envoi",
  non_deposee: "sauvegarde.envoi.non_deposee",
  absente: "sauvegarde.envoi.absente",
};

/** Libellé de la sorte d'une destination. */
export const CLES_SORTE: Record<SorteDestination, Cle> = {
  dossier: "sauvegarde.sorte.dossier",
  amovible: "sauvegarde.sorte.amovible",
  synchronise: "sauvegarde.sorte.synchronise",
};

/** Libellé de chaque rythme, dans l'ordre de l'écran. */
export const RYTHMES: readonly { rythme: RythmeHorsEcole; cle: Cle }[] = [
  { rythme: "demande", cle: "sauvegarde.hors_ecole.rythme.demande" },
  { rythme: "semaine", cle: "sauvegarde.hors_ecole.rythme.semaine" },
  { rythme: "mois", cle: "sauvegarde.hors_ecole.rythme.mois" },
];

/** Copies par mois au rythme choisi : 52 semaines font 12 mois. */
export function copiesParMois(rythme: RythmeHorsEcole): number {
  if (rythme === "semaine") return 52 / 12;
  return rythme === "mois" ? 1 : 0;
}

/** Octets qui partiront par Internet pour une copie : une par dossier choisi. */
export function volumeCopie(tailleCopie: number | null, dossiers: number): number | null {
  if (tailleCopie === null || tailleCopie <= 0 || dossiers <= 0) return null;
  return tailleCopie * dossiers;
}

/** Ordre de grandeur par mois au rythme choisi ; rien à la demande. */
export function volumeMensuel(tailleCopie: number | null, dossiers: number, rythme: RythmeHorsEcole): number | null {
  const copie = volumeCopie(tailleCopie, dossiers);
  const parMois = copiesParMois(rythme);
  return copie === null || parMois === 0 ? null : copie * parMois;
}

/** Forme d'un chemin pour reconnaître deux fois le même dossier (Windows ignore la casse). */
function forme(chemin: string): string {
  return chemin.trim().replace(/[\\/]+$/, "").replaceAll("/", "\\").toLowerCase();
}

/** Les dossiers trouvés que l'école n'a pas encore choisis. */
export function aProposer(trouves: readonly DossierSynchronise[], choisis: readonly string[]): DossierSynchronise[] {
  const deja = new Set(choisis.map(forme));
  return trouves.filter((d) => !deja.has(forme(d.propose)));
}

/**
 * Destinations après le choix d'un dossier synchronisé. Un dossier déjà réglé sous une autre
 * sorte devient synchronisé, puisque l'école vient de le choisir comme tel ; il n'est pas doublé.
 */
export function avecDossier(destinations: readonly Destination[], chemin: string): Destination[] {
  const choisi: Destination = { chemin, sorte: "synchronise", amovible: false };
  const rang = destinations.findIndex((d) => forme(d.chemin) === forme(chemin));
  if (rang < 0) return [...destinations, choisi];
  return destinations.map((d, i) => (i === rang ? choisi : d));
}

/** Destinations après le retrait d'un dossier synchronisé ; les autres ne changent pas. */
export function sansDossier(destinations: readonly Destination[], chemin: string): Destination[] {
  return destinations.filter((d) => !(d.sorte === "synchronise" && forme(d.chemin) === forme(chemin)));
}
