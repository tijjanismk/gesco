//! Ressources humaines (SPEC_PHASE_6) : registre et dossier du personnel, engagements, heures des
//! vacataires, absences et congés.

pub mod conge;
pub mod engagement;
pub mod heures;
pub mod personnel;

pub use conge::{AbsencePersonnel, Conge, NatureAbsencePersonnel, StatutConge, TypeConge};
pub use engagement::{Engagement, NatureEngagement, Remuneration, StatutEngagement};
pub use heures::{HeuresVacation, Mois, StatutHeures};
pub use personnel::{
    CategoriePersonnel, Diplome, FonctionPersonnel, MembrePersonnel, MoyenPaiement, NaturePiece,
    PieceDossier, SituationFamille,
};
