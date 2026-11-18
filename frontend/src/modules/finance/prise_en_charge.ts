// Calculs de l'écran « Prise en charge par l'État » : résumé de l'aperçu d'un cochage ou d'un
// décochage, pied d'une répartition de versement, « même montant pour chaque élève coché »,
// part au-delà de ce que l'État doit. Le noyau Rust décide ; ces calculs ne servent qu'à montrer
// et à préparer la demande.
import type { ApercuEleve, ApercuRepartitionEtat, EleveARepartir, PartSaisie } from "../../types/gesco";
import { lireSaisieReprise } from "./reprise";

/** Vrai si la session lit la liste des élèves pris en charge : la facturation ou la remise. */
export function litLaListe(permissions: readonly string[]): boolean {
  return permissions.includes("invoices:issue") || permissions.includes("invoices:discount");
}

/** Clés de requête relues après un geste de la prise en charge ou un versement de l'État. */
export const APRES_PRISE_EN_CHARGE: readonly string[] = ["prises-en-charge", "situation", "impayes", "versements-etat", "repartition-etat", "complements-eleves", "montants-etat", "preparation", "du-etat"];

/** Ce que l'aperçu d'un geste dit au parent et à l'école, pour un élève ou une sélection. */
export interface ResumeApercu {
  /** Élèves pour qui le geste change quelque chose. */
  nombre: number;
  /** Élèves pour qui le geste serait refusé. */
  refuses: number;
  /** Ce que les parents ne doivent plus (cochage). */
  ne_doit_plus: number;
  /** Ce que les parents devront en plus (décochage). */
  devra_en_plus: number;
  /** Dont l'argent déjà rendu au parent, qu'il devra de nouveau. */
  dont_deja_rembourse: number;
  /** Payé par le parent sur ce qui ne le regarde plus. */
  trop_paye: number;
  /** Trop-payé reporté sur ses autres frais (cantine, tenue…). */
  reporte: number;
  /** À rembourser au parent, après le geste. */
  a_rembourser: number;
  /** Ce que l'État doit encore, avant et après. */
  etat_avant: number;
  etat_apres: number;
}

/** Résume l'aperçu rendu par le noyau, élève par élève : seuls comptent les élèves dont le geste
 *  n'est pas refusé. « Ne doit plus » et « devra en plus » viennent de la variation du reste du
 *  parent ; le trop-payé, le report et l'à rembourser sont ceux d'après le geste. */
export function resumerApercu(eleves: readonly ApercuEleve[]): ResumeApercu {
  const resume: ResumeApercu = {
    nombre: 0,
    refuses: 0,
    ne_doit_plus: 0,
    devra_en_plus: 0,
    dont_deja_rembourse: 0,
    trop_paye: 0,
    reporte: 0,
    a_rembourser: 0,
    etat_avant: 0,
    etat_apres: 0,
  };
  for (const e of eleves) {
    if (e.refus !== null) {
      resume.refuses += 1;
      continue;
    }
    if (e.change) resume.nombre += 1;
    resume.ne_doit_plus += Math.max(0, e.reste_parent_avant - e.reste_parent_apres);
    resume.devra_en_plus += Math.max(0, e.reste_parent_apres - e.reste_parent_avant);
    resume.dont_deja_rembourse += e.rendu_apres;
    resume.trop_paye += e.trop_paye_apres;
    resume.reporte += e.trop_paye_reporte_apres;
    resume.a_rembourser += e.a_rembourser_apres;
    resume.etat_avant += e.doit_encore_etat_avant;
    resume.etat_apres += e.doit_encore_etat_apres;
  }
  return resume;
}

/** Montant tapé dans une case de l'écran : « 120000 », « 120 000 », « 120 000 F », « 120.000 » ;
 *  vide ou illisible : rien. */
export function lireFrancs(texte: string): number | null {
  const lu = lireSaisieReprise(texte);
  return lu.sorte === "montant" ? lu.montant : null;
}

/** Une ligne de la répartition telle que l'écran la tient : cochée ou non, et le montant tapé. */
export interface SaisieRepartition {
  coche: boolean;
  texte: string;
}

/** Parts à envoyer : les lignes cochées dont le montant est lisible et positif. */
export function partsDeLaSaisie(saisies: Readonly<Record<string, SaisieRepartition>>): PartSaisie[] {
  return Object.entries(saisies).flatMap(([inscription_id, s]) => {
    if (!s.coche) return [];
    const montant = lireFrancs(s.texte);
    return montant === null || montant <= 0 ? [] : [{ inscription_id, montant }];
  });
}

/** Pied de la répartition : reçu, réparti, non réparti ; au-delà du reçu, le dépassement et le
 *  refus (l'enregistrement serait refusé). Sans montant reçu lisible, rien n'est refusé encore. */
export interface PiedRepartition {
  recu: number;
  reparti: number;
  non_reparti: number;
  depassement: number;
  nombre: number;
  refuse: boolean;
}

export function piedRepartition(recu: number | null, parts: readonly Pick<PartSaisie, "montant">[]): PiedRepartition {
  const comptees = parts.filter((p) => p.montant > 0);
  const reparti = comptees.reduce((s, p) => s + p.montant, 0);
  const montant = recu ?? 0;
  const depassement = recu === null ? 0 : Math.max(0, reparti - montant);
  return {
    recu: montant,
    reparti,
    non_reparti: Math.max(0, montant - reparti),
    depassement,
    nombre: comptees.length,
    refuse: depassement > 0,
  };
}

/** « Même montant pour chaque élève coché » : le montant remplace celui de chaque ligne cochée,
 *  les autres lignes ne changent pas. */
export function memeMontantPourChaque(
  saisies: Readonly<Record<string, SaisieRepartition>>,
  montant: number,
): Record<string, SaisieRepartition> {
  return Object.fromEntries(
    Object.entries(saisies).map(([id, s]) => [id, s.coche ? { coche: true, texte: String(montant) } : s]),
  );
}

/** Saisie initiale d'une répartition : un élève qui a déjà une part du versement est coché avec
 *  elle ; les autres, non cochés, avec le reste dû par l'État prérempli. */
export function saisieInitiale(eleves: readonly Pick<EleveARepartir, "inscription_id" | "part" | "propose">[]): Record<string, SaisieRepartition> {
  return Object.fromEntries(eleves.map((e) => [e.inscription_id, { coche: e.part > 0, texte: e.propose === 0 ? "" : String(e.propose) }]));
}

/** Part au-delà de ce que l'État doit encore pour l'élève : acceptée, la différence est « reçue
 *  en plus » ; l'attendu porté est celui qui égale la somme reçue. Rien si la part ne dépasse pas. */
export function depassementAttendu(
  eleve: Pick<EleveARepartir, "attendu" | "reste_du_etat" | "pris_en_charge">,
  montant: number,
): { en_plus: number; attendu_porte: number | null } | null {
  const en_plus = montant - eleve.reste_du_etat;
  if (en_plus <= 0) return null;
  return { en_plus, attendu_porte: eleve.pris_en_charge ? eleve.attendu + en_plus : null };
}

/** Ce que l'import d'un fichier ferait de la répartition d'un versement : lignes écrites, et si
 *  l'écriture est possible. Le fichier donne la répartition entière : un fichier sans ligne à
 *  écrire peut encore retirer les parts actuelles ; un total au-delà du reçu refuse tout. */
export function importRepartition(
  apercu: Pick<ApercuRepartitionEtat, "valides" | "avertissements" | "retirees" | "refus">,
): { aEcrire: number; possible: boolean } {
  const aEcrire = apercu.valides + apercu.avertissements;
  return { aEcrire, possible: apercu.refus === null && (aEcrire > 0 || apercu.retirees > 0) };
}
