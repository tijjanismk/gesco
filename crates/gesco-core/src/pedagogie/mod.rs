//! Pédagogie (SPEC_PHASE_4) : matières, programmes et coefficients, périodes de notes, règles
//! d'évaluation. Règles pures, sans E/S (R-03) ; notes et moyennes en centièmes entiers (RC-6).

pub mod assiduite;
pub mod bulletin;
pub mod calcul;
pub mod conduite;
pub mod emploi_du_temps;
pub mod enseignant;
pub mod evaluation;
pub mod matiere;
pub mod periode;
pub mod programme;
pub mod regle;
pub mod resultats;

pub use assiduite::{Absence, DemiJournee, NatureAbsence, PorteeAbsences, TotauxAbsences};
pub use bulletin::{
    Bulletin, ConduiteBulletin, ContenuBulletin, LigneBulletin, MoyenneReprise, StatutBulletin,
};
pub use enseignant::{AffectationEnseignement, Enseignant, PorteeNotes, StatutEnseignant};
pub use evaluation::{Evaluation, NoteEleve, SaisieNote, StatutEvaluation};
pub use matiere::{GroupeMatiere, Matiere};
pub use periode::{Periode, TypePeriode};
pub use programme::{LigneProgramme, SaisieProgramme};
pub use regle::{
    ArrondiMoyenne, ContenuRegle, FormuleMatiere, ImpressionMoyenneAnnuelle, Mention,
    RegleEvaluation, TraitementAbsence, TypeAutorise, TypeEvaluation,
};
