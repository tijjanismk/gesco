//! Dépôts : une fonction par requête, le filtre d'établissement appliqué dans chacune (R-12).
//! Chaque dépôt reçoit une [`crate::Connexion`] — celle d'une [`crate::UniteDeTravail`] pour
//! écrire, celle du pool pour lire.

pub mod absences;
pub mod absences_seances;
pub mod academique;
pub mod annee_scolaire;
pub mod audit;
pub mod budget;
pub mod bulletins;
pub mod caisse;
pub mod cantine;
pub mod conges;
pub mod departs;
pub mod depenses;
pub mod discipline;
pub mod document;
pub mod emploi_du_temps;
pub mod enseignants;
pub mod etablissement;
pub mod evaluations;
pub mod factures;
pub mod file_sms;
pub mod fournisseurs;
pub mod inscriptions;
pub mod licence;
pub mod livres;
pub mod messages;
pub mod outbox;
pub mod paie;
pub mod parametre;
pub mod pedagogie;
pub mod personnel;
pub mod personnes;
pub mod recettes;
pub mod remises;
pub mod remises_documents;
pub mod role;
pub mod sauvegarde;
pub mod sequence;
pub mod tableau;
pub mod tarifs;
pub mod tresorerie;
pub mod utilisateur;
pub mod versements;

use gesco_core::commun::Horodatage;

pub(crate) fn booleen(valeur: i64) -> bool {
    valeur != 0
}

pub(crate) fn horodatage_optionnel(valeur: Option<i64>) -> Option<Horodatage> {
    valeur.map(Horodatage::depuis_millis_epoque)
}
