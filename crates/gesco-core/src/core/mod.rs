//! Domaine CORE : établissement, année scolaire, utilisateurs, permissions, paramètres, audit,
//! événements, numérotation, clés, sauvegardes et licence. Entités et invariants seulement ; la persistance est dans `gesco-db`.

pub mod annee_scolaire;
pub mod audit;
pub mod cle;
pub mod etablissement;
pub mod evenement;
pub mod licence;
pub mod mot_de_passe;
pub mod numerotation;
pub mod parametre;
pub mod permission;
pub mod phrase;
pub mod sauvegarde;
pub mod transition;
pub mod utilisateur;

pub use annee_scolaire::{AnneeScolaire, StatutAnnee};
pub use audit::{EmpreinteAudit, LigneAudit, Operation};
pub use cle::{
    CleDonnees, CleEnveloppe, Enveloppe, ErreurCle, NatureEnveloppe, ParametresDerivation,
};
pub use etablissement::{Etablissement, Organisation, StatutEtablissement};
pub use evenement::{Evenement, TypeEvenement};
pub use licence::{Edition, ErreurLicence, EtatLicence, Licence, StatutLicence};
pub use mot_de_passe::PolitiqueMotDePasse;
pub use numerotation::{FormatNumero, TypeDocumentNumerote};
pub use parametre::{Parametre, ValeurParametre};
pub use permission::{Permission, Role, RoleSysteme};
pub use phrase::PhraseSecours;
pub use sauvegarde::{Manifeste, MotifSauvegarde, PolitiqueRotation};
pub use transition::{ErreurTransition, MachineAEtats};
pub use utilisateur::{Poste, Utilisateur};
