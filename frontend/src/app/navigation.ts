import type { Cle } from "../i18n";
import { ACCES, COMMANDES_DE_LA_SAISIE, COMMANDES_DU_MAITRE, COMMANDES_POSTE_PRINCIPAL, type NomCommande } from "../types/acces";
import type { ModuleEcole } from "../types/gesco";

export type IdSection = "accueil" | "guichet" | "compte" | "utilisateurs" | "parametres" | "roles" | "annees" | "niveaux" | "pedagogie" | "enseignants" | "classes" | "eleves" | "inscriptions" | "notes" | "absences" | "bulletins" | "emploi-du-temps" | "bibliotheque" | "discipline" | "conduite" | "remises" | "convocations" | "fin-annee" | "frais" | "caisse" | "sessions-caisse" | "impayes" | "paye-avant-gesco" | "prise-en-charge-etat" | "journal-encaissements" | "depenses" | "recettes" | "fournisseurs" | "livres" | "journal-tresorerie" | "cantine" | "tresorerie" | "messages" | "personnel" | "heures" | "conges" | "paie" | "sauvegarde" | "journal" | "licence" | "postes" | "statistiques" | "inventaire" | "rapports";

interface Section {
  id: IdSection;
  libelle: Cle;
  groupe: Cle;
  /** Commande qui alimente l'écran : sa permission décide si la section s'affiche. */
  commande: NomCommande | null;
  /** Module que l'école active : éteint, la section ne paraît pas. */
  module?: ModuleEcole;
  /** Pour une commande que son service garde : la section ne paraît qu'avec l'une de ces
   *  permissions. */
  unePermission?: readonly string[];
}

/** Barre latérale (§19.2). Une section n'apparaît que si la garde Rust autorise la commande qui
 *  la nourrit ; masquer n'est qu'un confort, la garde reste la seule décision (R-01). */
const SECTIONS: readonly Section[] = [
  { id: "accueil", libelle: "nav.accueil", groupe: "nav.groupe.general", commande: null },
  { id: "statistiques", libelle: "nav.statistiques", groupe: "nav.groupe.general", commande: "statistiques_lire" },
  { id: "inventaire", libelle: "nav.inventaire", groupe: "nav.groupe.general", commande: "inventaire_lire" },
  { id: "rapports", libelle: "nav.rapports", groupe: "nav.groupe.general", commande: "rapports_lister" },
  { id: "guichet", libelle: "nav.guichet", groupe: "nav.groupe.scolarite", commande: "guichet_lire" },
  { id: "eleves", libelle: "nav.eleves", groupe: "nav.groupe.scolarite", commande: "eleves_rechercher" },
  { id: "inscriptions", libelle: "nav.inscriptions", groupe: "nav.groupe.scolarite", commande: "inscriptions_lister" },
  { id: "notes", libelle: "nav.notes", groupe: "nav.groupe.scolarite", commande: "evaluation_creer" },
  { id: "absences", libelle: "nav.absences", groupe: "nav.groupe.scolarite", commande: "absences_releve_lire" },
  { id: "bulletins", libelle: "nav.bulletins", groupe: "nav.groupe.scolarite", commande: "bulletins_lire" },
  { id: "discipline", libelle: "nav.discipline", groupe: "nav.groupe.scolarite", commande: "discipline_registre_lire", module: "discipline" },
  { id: "conduite", libelle: "nav.conduite", groupe: "nav.groupe.scolarite", commande: "conduite_classes_lire", module: "conduite" },
  { id: "remises", libelle: "nav.remises", groupe: "nav.groupe.scolarite", commande: "bulletins_non_retires_lire" },
  { id: "convocations", libelle: "nav.convocations", groupe: "nav.groupe.scolarite", commande: "convocations_lire" },
  { id: "emploi-du-temps", libelle: "nav.emploi_du_temps", groupe: "nav.groupe.scolarite", commande: "emploi_du_temps_lire", module: "emploi_du_temps" },
  { id: "bibliotheque", libelle: "nav.bibliotheque", groupe: "nav.groupe.scolarite", commande: "bibliotheque_catalogue", module: "bibliotheque" },
  { id: "fin-annee", libelle: "nav.fin_annee", groupe: "nav.groupe.scolarite", commande: "fin_annee_lire" },
  { id: "caisse", libelle: "nav.caisse", groupe: "nav.groupe.finance", commande: "caisse_ouvrir" },
  { id: "impayes", libelle: "nav.impayes", groupe: "nav.groupe.finance", commande: "impayes_lister" },
  { id: "sessions-caisse", libelle: "nav.sessions_caisse", groupe: "nav.groupe.finance", commande: "caisse_sessions_lister" },
  { id: "paye-avant-gesco", libelle: "nav.paye_avant_gesco", groupe: "nav.groupe.finance", commande: "reprises_lister" },
  {
    id: "prise-en-charge-etat",
    libelle: "nav.prise_en_charge_etat",
    groupe: "nav.groupe.finance",
    commande: "prises_en_charge_lister",
    unePermission: ["invoices:issue", "invoices:discount", "treasury:read"],
  },
  { id: "journal-encaissements", libelle: "nav.journal_encaissements", groupe: "nav.groupe.finance", commande: "encaissements_lister" },
  { id: "depenses", libelle: "nav.depenses", groupe: "nav.groupe.finance", commande: "depense_saisir", module: "tresorerie" },
  { id: "recettes", libelle: "nav.recettes", groupe: "nav.groupe.finance", commande: "recettes_lire", module: "tresorerie" },
  { id: "fournisseurs", libelle: "nav.fournisseurs", groupe: "nav.groupe.finance", commande: "facture_fournisseur_enregistrer", module: "tresorerie" },
  { id: "livres", libelle: "nav.livres", groupe: "nav.groupe.finance", commande: "comptes_soldes_lire", module: "tresorerie" },
  { id: "journal-tresorerie", libelle: "nav.journal_tresorerie", groupe: "nav.groupe.finance", commande: "journal_tresorerie_lire", module: "tresorerie" },
  { id: "cantine", libelle: "nav.cantine", groupe: "nav.groupe.finance", commande: "cantine_lire", module: "cantine" },
  { id: "tresorerie", libelle: "nav.tresorerie", groupe: "nav.groupe.finance", commande: "tresorerie_lire", module: "tresorerie" },
  { id: "messages", libelle: "nav.messages", groupe: "nav.groupe.communication", commande: "messages_lire", module: "sms" },
  { id: "personnel", libelle: "nav.personnel", groupe: "nav.groupe.personnel", commande: "personnel_lister", module: "personnel" },
  { id: "heures", libelle: "nav.heures", groupe: "nav.groupe.personnel", commande: "heures_lire", module: "personnel" },
  { id: "conges", libelle: "nav.conges", groupe: "nav.groupe.personnel", commande: "absences_conges_lire", module: "personnel" },
  { id: "paie", libelle: "nav.paie", groupe: "nav.groupe.personnel", commande: "elements_paie_lire", module: "personnel" },
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
  { id: "postes", libelle: "nav.postes", groupe: "nav.groupe.parametres", commande: "reseau_etat_lire" },
  { id: "compte", libelle: "nav.compte", groupe: "nav.groupe.outils", commande: null },
  { id: "sauvegarde", libelle: "nav.sauvegarde", groupe: "nav.groupe.outils", commande: "sauvegardes_lister" },
  { id: "journal", libelle: "nav.journal", groupe: "nav.groupe.outils", commande: "audit_lister" },
  { id: "licence", libelle: "nav.licence", groupe: "nav.groupe.outils", commande: "licence_etat" },
];

/** Groupes repliés à l'ouverture : on les déplie d'un clic, ou en y entrant. */
export const GROUPES_REPLIES: readonly Cle[] = ["nav.groupe.parametres", "nav.groupe.outils"];

/** Vrai si la session détient l'accès qu'exige la commande : un confort d'affichage, la garde Rust
 *  décide (R-21). Un compte de maître n'a que les commandes de sa liste, et celles de la saisie
 *  seulement quand l'école lui a ouvert la saisie des notes. */
export function commandeAutorisee(commande: NomCommande | null, permissions: readonly string[], maitre = false, saisie = true): boolean {
  if (commande === null) return true;
  if (maitre && !COMMANDES_DU_MAITRE.includes(commande)) return false;
  if (maitre && !saisie && COMMANDES_DE_LA_SAISIE.includes(commande)) return false;
  const acces: { type: string; permission?: string } = ACCES[commande];
  return acces.type !== "permission" || (acces.permission !== undefined && permissions.includes(acces.permission));
}

/** Sections affichées. Sur un poste relié, celles de ce qui ne se fait que sur le poste principal
 *  (sauvegardes, postes de l'école) ne paraissent pas ; `cachees` retire celles que l'école n'a
 *  pas à voir (la prise en charge par l'État sans lycée ni technique). */
export function sectionsAutorisees(
  permissions: readonly string[],
  modules: readonly ModuleEcole[],
  relie = false,
  maitre = false,
  saisie = true,
  cachees: readonly IdSection[] = [],
): Section[] {
  return SECTIONS.filter(
    (s) =>
      !cachees.includes(s.id) &&
      commandeAutorisee(s.commande, permissions, maitre, saisie) &&
      (s.unePermission === undefined || s.unePermission.some((p) => permissions.includes(p))) &&
      (s.module === undefined || modules.includes(s.module)) &&
      !(relie && s.commande !== null && COMMANDES_POSTE_PRINCIPAL.includes(s.commande)),
  );
}
