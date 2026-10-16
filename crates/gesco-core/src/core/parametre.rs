//! Paramètres d'établissement : rien n'est écrit en dur (§3.8), tout est une donnée.

use serde::{Deserialize, Serialize};

use super::etablissement::Etablissement;
use crate::commun::Id;

/// Valeur typée d'un paramètre.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(tag = "type", content = "valeur", rename_all = "snake_case")]
pub enum ValeurParametre {
    /// Texte libre.
    Texte(String),
    /// Entier signé.
    Entier(i64),
    /// Vrai ou faux.
    Booleen(bool),
}

/// Paramètre d'un établissement.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
pub struct Parametre {
    /// Établissement propriétaire.
    pub etablissement_id: Id<Etablissement>,
    /// Clé hiérarchique, par exemple `session.verrouillage_inactivite_s`.
    pub cle: String,
    /// Valeur courante.
    pub valeur: ValeurParametre,
    /// Faux pour les paramètres que seul le support modifie.
    pub modifiable_par_utilisateur: bool,
}

/// Vrai si la valeur sort des bornes d'un paramètre : la politique de mot de passe ne descend pas
/// sous le plancher du produit, le verrouillage reste entre 3 et 10 échecs, le matricule garde
/// une forme lisible.
pub fn hors_bornes(cle: &str, valeur: &ValeurParametre) -> bool {
    match (cle, valeur) {
        ("mot_de_passe.longueur_min", ValeurParametre::Entier(n)) => !(8..=64).contains(n),
        ("connexion.echecs_avant_verrouillage", ValeurParametre::Entier(n)) => {
            !(3..=10).contains(n)
        }
        ("inscription.alerte_apres_jours", ValeurParametre::Entier(n)) => !(0..=365).contains(n),
        // Q-26 : trois règles seulement pour un élève arrivé en cours d'année.
        ("facturation.arrivee_en_cours", ValeurParametre::Texte(t)) => {
            crate::finance::ArriveeEnCours::depuis_nom(t).is_none()
        }
        // SPEC_PHASE_3 §2 : un arrondi au franc, ou aux coupures en usage.
        (cle, ValeurParametre::Texte(t)) if cle.starts_with("document.format.") => {
            !matches!(t.as_str(), "a4" | "a5")
        }
        ("conge.jours_par_mois", ValeurParametre::Texte(t)) => {
            crate::rh::conge::lire_jours_par_mois(t).is_none()
        }
        // SPEC_PHASE_7 §2 : des mois de 1 à 12, une règle de prorata parmi trois.
        ("paie.mois_vacances", ValeurParametre::Texte(t)) => {
            crate::paie::reglages::lire_mois_vacances(t).is_none()
        }
        ("paie.prorata", ValeurParametre::Texte(t)) => {
            crate::paie::Prorata::depuis_nom(t).is_none()
        }
        // SPEC_PHASE_10 §3 : la moyenne de passage reste dans le barème de son cycle.
        (cle, ValeurParametre::Entier(n))
            if cle.starts_with(crate::scolarite::fin_annee::PREFIXE_PASSAGE) =>
        {
            crate::scolarite::fin_annee::passage_hors_bornes(cle, *n)
        }
        // SPEC_PHASE_8 §3 : le seuil d'accord des dépenses, en francs ; 0 : jamais d'accord.
        ("depense.seuil_accord", ValeurParametre::Entier(n)) => !(0..=1_000_000_000).contains(n),
        ("montant.multiple_arrondi", ValeurParametre::Entier(n)) => {
            ![1, 5, 10, 25, 50, 100].contains(n)
        }
        _ => crate::scolarite::matricule::hors_bornes(cle, valeur),
    }
}

/// Format du papier de chaque document, par son réglage (réponse du 14 octobre 2026) : la
/// plupart des écoles impriment le reçu et le bulletin en A5.
pub const FORMATS_PAR_DEFAUT: &[(&str, &str)] = &[
    ("document.format.recu", "a5"),
    ("document.format.bulletin", "a5"),
    ("document.format.fiche_inscription", "a4"),
    ("document.format.releve_compte", "a4"),
    ("document.format.etat_caisse", "a4"),
    ("document.format.certificat_transfert", "a4"),
    ("document.format.releve_moyennes", "a4"),
    ("document.format.engagement", "a4"),
    ("document.format.etat_heures", "a4"),
    ("document.format.bulletin_paie", "a4"),
    ("document.format.etat_retenues", "a4"),
];

/// Format par défaut d'un document qui a un réglage de format.
pub fn format_par_defaut(document: &str) -> Option<&'static str> {
    let cle = format!("document.format.{document}");
    FORMATS_PAR_DEFAUT
        .iter()
        .find(|(c, _)| *c == cle)
        .map(|(_, f)| *f)
}

/// Paramètres livrés avec leur valeur par défaut. Posés au premier démarrage ; ceux qui manquent
/// à une base plus ancienne sont ajoutés à son ouverture.
pub fn parametres_par_defaut(etablissement_id: Id<Etablissement>) -> Vec<Parametre> {
    let p = |cle: &str, valeur: ValeurParametre, modifiable: bool| Parametre {
        etablissement_id,
        cle: cle.to_owned(),
        valeur,
        modifiable_par_utilisateur: modifiable,
    };
    let mut parametres = vec![
        p(
            "session.verrouillage_inactivite_s",
            ValeurParametre::Entier(120),
            true,
        ),
        p(
            "session.expiration_s",
            ValeurParametre::Entier(8 * 3_600),
            true,
        ),
        p(
            "mot_de_passe.longueur_min",
            ValeurParametre::Entier(8),
            true,
        ),
        p(
            "mot_de_passe.refuser_identifiant",
            ValeurParametre::Booleen(true),
            true,
        ),
        p(
            "mot_de_passe.refuser_courants",
            ValeurParametre::Booleen(true),
            true,
        ),
        p(
            "connexion.echecs_avant_verrouillage",
            ValeurParametre::Entier(3),
            true,
        ),
        p(
            "sauvegarde.intervalle_min",
            ValeurParametre::Entier(60),
            true,
        ),
        p("sauvegarde.quotidiennes", ValeurParametre::Entier(7), true),
        p("sauvegarde.hebdomadaires", ValeurParametre::Entier(4), true),
        p("sauvegarde.mensuelles", ValeurParametre::Entier(12), true),
        p(
            "sauvegarde.certificat_confirme",
            ValeurParametre::Booleen(false),
            false,
        ),
        p(
            "numerotation.reinitialisation_par_exercice",
            ValeurParametre::Booleen(true),
            true,
        ),
        p("montant.multiple_arrondi", ValeurParametre::Entier(5), true),
        // Q-31 : une inscription tardive avertit sans bloquer ; 0, jamais.
        p(
            "inscription.alerte_apres_jours",
            ValeurParametre::Entier(0),
            true,
        ),
        // T-08 : chaque école choisit si les responsables figurent sur la fiche imprimée.
        p(
            "fiche_inscription.afficher_responsables",
            ValeurParametre::Booleen(true),
            true,
        ),
        // Q-33 : un transfert avertit d'un impayé sans le bloquer, sauf si l'école l'exige.
        p(
            "transfert.solde_nul_exige",
            ValeurParametre::Booleen(false),
            true,
        ),
        // Q-26 : frais d'inscription entiers et tranches non échues, par défaut.
        p(
            "facturation.arrivee_en_cours",
            ValeurParametre::Texte("echeances_restantes".to_owned()),
            true,
        ),
        // Matricule interne (T-01 révisé) : `2026-00042` par défaut.
        p(
            "matricule.saisie_manuelle",
            ValeurParametre::Booleen(false),
            true,
        ),
        p(
            "matricule.prefixe",
            ValeurParametre::Texte(String::new()),
            true,
        ),
        p("matricule.chiffres_annee", ValeurParametre::Entier(4), true),
        p("matricule.chiffres", ValeurParametre::Entier(5), true),
        // Q-68 : un congé se compte du lundi au vendredi ; T-44 : décompte désactivé.
        p("conge.samedi_compte", ValeurParametre::Booleen(false), true),
        p(
            "conge.decompte_actif",
            ValeurParametre::Booleen(false),
            true,
        ),
        p(
            "conge.jours_par_mois",
            ValeurParametre::Texte(crate::rh::conge::JOURS_PAR_MOIS_DEFAUT.to_owned()),
            true,
        ),
        // T-52, Q-73 : douze mois payés par défaut, juillet à septembre en vacances ; Q-74 : au
        // trentième.
        p("paie.douze_mois", ValeurParametre::Booleen(true), true),
        // T-61 : une dépense au-dessus de 50 000 F attend l'accord de la direction ; T-47 : la
        // cantine est un module que l'école active.
        p(
            "depense.seuil_accord",
            ValeurParametre::Entier(50_000),
            true,
        ),
        p("cantine.active", ValeurParametre::Booleen(false), true),
        p(
            "cantine.repas_comptes",
            ValeurParametre::Booleen(false),
            true,
        ),
        p(
            "paie.mois_vacances",
            ValeurParametre::Texte(crate::paie::reglages::MOIS_VACANCES_DEFAUT.to_owned()),
            true,
        ),
        p(
            "paie.prorata",
            ValeurParametre::Texte(crate::paie::Prorata::Trentieme.nom().to_owned()),
            true,
        ),
        p(
            "matricule.remise_annuelle",
            ValeurParametre::Booleen(true),
            true,
        ),
    ];
    for (cle, defaut) in FORMATS_PAR_DEFAUT {
        parametres.push(p(cle, ValeurParametre::Texte((*defaut).to_owned()), true));
    }
    // T-65 : moyenne de passage de chaque cycle noté, reprise du CAP ou de l'académie.
    for cycle in crate::scolarite::Cycle::TOUS {
        if let (Some(cle), Some(defaut)) = (
            crate::scolarite::fin_annee::cle_passage(cycle),
            crate::scolarite::fin_annee::passage_par_defaut(cycle),
        ) {
            parametres.push(p(&cle, ValeurParametre::Entier(i64::from(defaut)), true));
        }
    }
    // Bulletin imprimé : chaque colonne s'affiche ou non, cycle par cycle.
    for cycle in crate::pedagogie::bulletin::CYCLES_A_BULLETIN {
        parametres.extend(
            crate::pedagogie::bulletin::reglages_impression(cycle)
                .iter()
                .map(|(cle, defaut)| p(cle, ValeurParametre::Booleen(*defaut), true)),
        );
    }
    parametres
}
