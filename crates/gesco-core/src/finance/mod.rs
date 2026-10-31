//! Finance (SPEC_PHASE_3, blueprint §8.8) : frais, factures, échéanciers, remises, caisse.
//! Règles pures, sans E/S (R-03) ; montants en francs entiers (R-11).

pub mod caisse;
pub mod facture;
pub mod recouvrement;
pub mod remise;
pub mod tarif;
pub mod versement;

pub use caisse::{ModePaiement, MouvementCaisse, NatureMouvement, SessionCaisse, StatutSession};
pub use facture::{
    ArriveeEnCours, ContexteTarifaire, Echeance, Facture, LigneFacture, StatutEcheance,
    StatutFacture,
};
pub use recouvrement::{Impaye, PARAMETRE_TRANCHES, TrancheAnciennete, Tranches};
pub use remise::{Remise, StatutRemise, TypeRemise, ValeurRemise};
pub use tarif::{
    ContenuGrille, EcheanceCalculee, EtatGrille, Exigibilite, Grille, LigneTarifaire, PlanPaiement,
    Tranche, TypeFrais,
};
pub use versement::{Imputation, StatutVersement, Versement};
