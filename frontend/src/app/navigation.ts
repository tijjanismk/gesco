import type { Cle } from "../i18n";
import { ACCES, type NomCommande } from "../types/acces";

export type IdSection = "accueil" | "compte" | "utilisateurs" | "parametres" | "roles" | "annees" | "niveaux" | "pedagogie" | "enseignants" | "classes" | "eleves" | "inscriptions" | "notes" | "absences" | "bulletins" | "emploi-du-temps" | "discipline" | "fin-annee" | "frais" | "caisse" | "sessions-caisse" | "impayes" | "journal-encaissements" | "depenses" | "recettes" | "fournisseurs" | "livres" | "journal-tresorerie" | "cantine" | "tresorerie" | "messages" | "personnel" | "heures" | "conges" | "paie" | "sauvegarde" | "journal" | "licence";

interface Section {
  id: IdSection;
  libelle: Cle;
  groupe: Cle;
  /** Commande qui alimente l'écran : sa permission décide si la section s'affiche. */
  commande: NomCommande | null;
}

/** Barre latérale (§19.2). Une section n'apparaît que si la garde Rust autorise la commande qui
 *  la nourrit ; masquer n'est qu'un confort, la garde reste la seule décision (R-01). */
const SECTIONS: readonly Section[] = [
  { id: "accueil", libelle: "nav.accueil", groupe: "nav.groupe.general", commande: null },
  { id: "eleves", libelle: "nav.eleves", groupe: "nav.groupe.scolarite", commande: "eleves_rechercher" },
  { id: "inscriptions", libelle: "nav.inscriptions", groupe: "nav.groupe.scolarite", commande: "inscriptions_lister" },
  { id: "notes", libelle: "nav.notes", groupe: "nav.groupe.scolarite", commande: "evaluation_creer" },
  { id: "absences", libelle: "nav.absences", groupe: "nav.groupe.scolarite", commande: "absences_releve_lire" },
  { id: "bulletins", libelle: "nav.bulletins", groupe: "nav.groupe.scolarite", commande: "bulletins_lire" },
  { id: "discipline", libelle: "nav.discipline", groupe: "nav.groupe.scolarite", commande: "discipline_registre_lire" },
  { id: "emploi-du-temps", libelle: "nav.emploi_du_temps", groupe: "nav.groupe.scolarite", commande: "emploi_du_temps_lire" },
  { id: "fin-annee", libelle: "nav.fin_annee", groupe: "nav.groupe.scolarite", commande: "fin_annee_lire" },
  { id: "caisse", libelle: "nav.caisse", groupe: "nav.groupe.finance", commande: "caisse_ouvrir" },
  { id: "impayes", libelle: "nav.impayes", groupe: "nav.groupe.finance", commande: "impayes_lister" },
  { id: "sessions-caisse", libelle: "nav.sessions_caisse", groupe: "nav.groupe.finance", commande: "caisse_sessions_lister" },
  { id: "journal-encaissements", libelle: "nav.journal_encaissements", groupe: "nav.groupe.finance", commande: "encaissements_lister" },
  { id: "depenses", libelle: "nav.depenses", groupe: "nav.groupe.finance", commande: "depense_saisir" },
  { id: "recettes", libelle: "nav.recettes", groupe: "nav.groupe.finance", commande: "recettes_lire" },
  { id: "fournisseurs", libelle: "nav.fournisseurs", groupe: "nav.groupe.finance", commande: "facture_fournisseur_enregistrer" },
  { id: "livres", libelle: "nav.livres", groupe: "nav.groupe.finance", commande: "comptes_soldes_lire" },
  { id: "journal-tresorerie", libelle: "nav.journal_tresorerie", groupe: "nav.groupe.finance", commande: "journal_tresorerie_lire" },
  { id: "cantine", libelle: "nav.cantine", groupe: "nav.groupe.finance", commande: "cantine_lire" },
  { id: "tresorerie", libelle: "nav.tresorerie", groupe: "nav.groupe.finance", commande: "tresorerie_lire" },
  { id: "messages", libelle: "nav.messages", groupe: "nav.groupe.communication", commande: "messages_lire" },
  { id: "personnel", libelle: "nav.personnel", groupe: "nav.groupe.personnel", commande: "personnel_lister" },
  { id: "heures", libelle: "nav.heures", groupe: "nav.groupe.personnel", commande: "heures_lire" },
  { id: "conges", libelle: "nav.conges", groupe: "nav.groupe.personnel", commande: "absences_conges_lire" },
  { id: "paie", libelle: "nav.paie", groupe: "nav.groupe.personnel", commande: "elements_paie_lire" },
  // Mise en place de l'année et réglages : repliés, ils ne servent pas tous les jours.
  { id: "classes", libelle: "nav.classes", groupe: "nav.groupe.parametres", commande: "classes_lister" },
  { id: "frais", libelle: "nav.frais", groupe: "nav.groupe.parametres", commande: "grilles_lister" },
  { id: "annees", libelle: "nav.annees", groupe: "nav.groupe.parametres", commande: "annee_creer" },
  { id: "pedagogie", libelle: "nav.pedagogie", groupe: "nav.groupe.parametres", commande: "programme_enregistrer" },
  { id: "enseignants", libelle: "nav.enseignants", groupe: "nav.groupe.parametres", commande: "enseignant_enregistrer" },
  { id: "niveaux", libelle: "nav.niveaux", groupe: "nav.groupe.parametres", commande: "niveau_modifier" },
  { id: "parametres", libelle: "nav.parametres", groupe: "nav.groupe.parametres", commande: "parametres_lister" },
  { id: "utilisateurs", libelle: "nav.utilisateurs", groupe: "nav.groupe.parametres", commande: "utilisateurs_lister" },
  { id: "roles", libelle: "nav.roles", groupe: "nav.groupe.parametres", commande: "roles_lister" },
  { id: "compte", libelle: "nav.compte", groupe: "nav.groupe.outils", commande: null },
  { id: "sauvegarde", libelle: "nav.sauvegarde", groupe: "nav.groupe.outils", commande: "sauvegardes_lister" },
  { id: "journal", libelle: "nav.journal", groupe: "nav.groupe.outils", commande: "audit_lister" },
  { id: "licence", libelle: "nav.licence", groupe: "nav.groupe.outils", commande: "licence_etat" },
];

/** Groupes repliés à l'ouverture : on les déplie d'un clic, ou en y entrant. */
export const GROUPES_REPLIES: readonly Cle[] = ["nav.groupe.parametres", "nav.groupe.outils"];

/** Vrai si la session détient l'accès qu'exige la commande : un confort d'affichage, la garde Rust
 *  décide (R-21). */
export function commandeAutorisee(commande: NomCommande | null, permissions: readonly string[]): boolean {
  if (commande === null) return true;
  const acces: { type: string; permission?: string } = ACCES[commande];
  return acces.type !== "permission" || (acces.permission !== undefined && permissions.includes(acces.permission));
}

export function sectionsAutorisees(permissions: readonly string[]): Section[] {
  return SECTIONS.filter((s) => commandeAutorisee(s.commande, permissions));
}
