//! Erreurs des services. Chaque erreur destinée à l'utilisateur porte une clé de traduction (R-39).

use gesco_core::core::{ErreurCle, ErreurLicence, ErreurTransition, Permission};
use gesco_db::ErreurBase;
use gesco_docs::ErreurDocument;
use thiserror::Error;

/// Raison d'un refus de connexion, sans révéler lequel des deux facteurs est faux.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MotifRefus {
    /// Identifiant inconnu ou mot de passe faux : même message, délibérément.
    IdentifiantsInvalides,
    /// Compte désactivé par un administrateur.
    CompteInactif,
    /// Trop d'échecs successifs ; `jusqua` est l'horodatage de fin de verrouillage en millisecondes.
    Verrouille {
        /// Fin du verrouillage, millisecondes depuis l'époque.
        jusqua: i64,
    },
    /// Code PIN absent ou faux.
    PinInvalide,
    /// Aucun code PIN défini pour cet utilisateur.
    PinNonDefini,
    /// Trop de PIN faux : la session est fermée, le mot de passe est exigé.
    PinEpuise,
    /// Session inconnue ou expirée.
    SessionInvalide,
    /// La session est verrouillée : un code PIN est attendu.
    SessionVerrouillee,
    /// Trop de phrases de secours fausses ; `jusqua` est la fin de l'attente en millisecondes.
    PhraseVerrouillee {
        /// Fin de l'attente, millisecondes depuis l'époque.
        jusqua: i64,
    },
}

/// Erreur d'un service.
#[derive(Debug, Error)]
pub enum ErreurService {
    /// Erreur de la couche de persistance.
    #[error(transparent)]
    Base(#[from] ErreurBase),
    /// L'utilisateur ne détient pas la permission requise.
    #[error("permission refusée : {permission}")]
    PermissionRefusee {
        /// Code de la permission manquante.
        permission: &'static str,
    },
    /// L'utilisateur détient la permission mais pas la portée sur la ressource visée (§11.3).
    #[error("portée refusée sur {ressource}")]
    PorteeRefusee {
        /// Ressource visée, sous forme lisible.
        ressource: String,
    },
    /// Connexion ou déverrouillage refusé.
    #[error("authentification refusée")]
    Authentification(MotifRefus),
    /// Transition d'état interdite.
    #[error(transparent)]
    Transition(#[from] ErreurTransition),
    /// Règle métier violée, avec sa clé de traduction.
    #[error("règle métier : {cle}")]
    Regle {
        /// Clé du catalogue i18n.
        cle: &'static str,
    },
    /// Impossible de calculer une empreinte de mot de passe ou de PIN.
    #[error("secret : {0}")]
    Secret(String),
    /// Sérialisation.
    #[error(transparent)]
    Json(#[from] serde_json::Error),
    /// Clé de données, enveloppe ou phrase de secours.
    #[error(transparent)]
    Cle(#[from] ErreurCle),
    /// Moteur documentaire.
    #[error(transparent)]
    Document(#[from] ErreurDocument),
    /// Lecture ou écriture de fichier.
    #[error("fichier : {0}")]
    Fichier(#[from] std::io::Error),
    /// Fichier de licence illisible ou non authentique.
    #[error(transparent)]
    Licence(#[from] ErreurLicence),
}

impl ErreurService {
    /// Construit un refus de permission à partir de la permission manquante.
    pub const fn permission(permission: Permission) -> Self {
        Self::PermissionRefusee {
            permission: permission.code(),
        }
    }

    /// Clé de traduction du message destiné à l'utilisateur (R-39).
    pub fn cle_traduction(&self) -> &'static str {
        match self {
            Self::Base(ErreurBase::Introuvable { .. }) => "erreur.introuvable",
            Self::Base(ErreurBase::Cloisonnement { .. }) => "erreur.cloisonnement",
            Self::Base(ErreurBase::ChaineRompue { .. }) => "erreur.audit.chaine_rompue",
            Self::Base(_) => "erreur.base",
            Self::PermissionRefusee { .. } => "erreur.permission_refusee",
            Self::PorteeRefusee { .. } => "erreur.portee_refusee",
            Self::Authentification(motif) => match motif {
                MotifRefus::IdentifiantsInvalides => "connexion.identifiants_invalides",
                MotifRefus::CompteInactif => "connexion.compte_inactif",
                MotifRefus::Verrouille { .. } => "connexion.verrouille",
                MotifRefus::PinInvalide => "session.pin_invalide",
                MotifRefus::PinNonDefini => "session.pin_non_defini",
                MotifRefus::PinEpuise => "session.pin_epuise",
                MotifRefus::SessionInvalide => "session.invalide",
                MotifRefus::SessionVerrouillee => "session.verrouillee",
                MotifRefus::PhraseVerrouillee { .. } => "cles.phrase_verrouillee",
            },
            Self::Transition(_) => "erreur.transition_interdite",
            Self::Regle { cle } => cle,
            Self::Secret(_) => "erreur.secret",
            Self::Json(_) => "erreur.serialisation",
            Self::Cle(ErreurCle::EnveloppeFermee) => "cles.phrase_invalide",
            Self::Cle(ErreurCle::Phrase(_)) => "cles.phrase_mal_formee",
            Self::Cle(ErreurCle::BlocInvalide) => "sauvegarde.archive_alteree",
            Self::Cle(ErreurCle::Derivation(_)) => "erreur.secret",
            Self::Document(e) => e.cle_traduction(),
            Self::Fichier(_) => "erreur.fichier",
            Self::Licence(ErreurLicence::Illisible) => "licence.illisible",
            Self::Licence(ErreurLicence::FormatInconnu(_)) => "licence.format_inconnu",
            Self::Licence(ErreurLicence::SignatureInvalide) => "licence.signature_invalide",
            Self::Licence(ErreurLicence::CleInvalide) => "licence.cle_invalide",
        }
    }
}

/// Résultat des services.
pub type Resultat<T> = Result<T, ErreurService>;
