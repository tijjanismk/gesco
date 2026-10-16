//! Erreurs métier des types fondamentaux.
//!
//! Chaque erreur porte une clé de traduction (R-39) : le texte affiché à l'utilisateur vient du
//! catalogue i18n, jamais du message `Display`, réservé aux journaux techniques.

use thiserror::Error;

/// Erreur sur un identifiant technique.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum ErreurId {
    /// La chaîne n'est pas un ULID valide (26 caractères Crockford base 32).
    #[error("identifiant mal formé : « {0} »")]
    FormatInvalide(String),
}

/// Erreur sur un montant en francs CFA.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub enum ErreurMontant {
    /// Le résultat dépasse la capacité d'un entier signé 64 bits.
    #[error("débordement arithmétique sur un montant")]
    Debordement,
    /// Division par zéro.
    #[error("division d'un montant par zéro")]
    DivisionParZero,
    /// Le multiple d'arrondi doit être strictement positif.
    #[error("multiple d'arrondi invalide : {0}")]
    MultipleInvalide(i64),
    /// Montant trop grand pour être écrit en toutes lettres (au-delà de 999 999 999 999 F).
    #[error("montant trop grand pour être écrit en lettres")]
    TropGrandPourLettres,
}

/// Erreur sur une note ou une moyenne.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub enum ErreurNote {
    /// La valeur est négative ou dépasse le barème.
    #[error("note hors barème : {valeur_centiemes} pour un barème de {bareme_centiemes}")]
    HorsBareme {
        /// Valeur refusée, en centièmes.
        valeur_centiemes: i32,
        /// Barème de l'évaluation, en centièmes.
        bareme_centiemes: i32,
    },
    /// Un barème nul ou négatif n'a pas de sens.
    #[error("barème invalide : {0}")]
    BaremeInvalide(i32),
    /// Deux notes de barèmes différents ne se comparent pas sans normalisation explicite.
    #[error("barèmes différents : {0} et {1}")]
    BaremesDifferents(i32, i32),
    /// Une pondération nulle ou négative est refusée.
    #[error("pondération invalide : {0}")]
    PonderationInvalide(i64),
    /// Aucune note à moyenner.
    #[error("aucune note dans le calcul")]
    AucuneNote,
    /// Le calcul intermédiaire dépasse la capacité prévue.
    #[error("débordement arithmétique sur une note")]
    Debordement,
}

/// Erreur sur un horodatage.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub enum ErreurHorodatage {
    /// Le décalage demandé sort de la plage représentable.
    #[error("horodatage hors plage")]
    HorsPlage,
}

/// Erreurs portant une clé de traduction pour l'interface.
pub trait CleTraduction {
    /// Clé du catalogue i18n, de la forme `erreur.<domaine>.<cas>`.
    fn cle_traduction(&self) -> &'static str;
}

impl CleTraduction for ErreurId {
    fn cle_traduction(&self) -> &'static str {
        match self {
            Self::FormatInvalide(_) => "erreur.id.format_invalide",
        }
    }
}

impl CleTraduction for ErreurMontant {
    fn cle_traduction(&self) -> &'static str {
        match self {
            Self::Debordement => "erreur.montant.debordement",
            Self::DivisionParZero => "erreur.montant.division_par_zero",
            Self::MultipleInvalide(_) => "erreur.montant.multiple_invalide",
            Self::TropGrandPourLettres => "erreur.montant.trop_grand_pour_lettres",
        }
    }
}

impl CleTraduction for ErreurNote {
    fn cle_traduction(&self) -> &'static str {
        match self {
            Self::HorsBareme { .. } => "erreur.note.hors_bareme",
            Self::BaremeInvalide(_) => "erreur.note.bareme_invalide",
            Self::BaremesDifferents(..) => "erreur.note.baremes_differents",
            Self::PonderationInvalide(_) => "erreur.note.ponderation_invalide",
            Self::AucuneNote => "erreur.note.aucune_note",
            Self::Debordement => "erreur.note.debordement",
        }
    }
}

impl CleTraduction for ErreurHorodatage {
    fn cle_traduction(&self) -> &'static str {
        match self {
            Self::HorsPlage => "erreur.horodatage.hors_plage",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chaque_cle_suit_le_format_du_catalogue() {
        let cles = [
            ErreurId::FormatInvalide(String::new()).cle_traduction(),
            ErreurMontant::Debordement.cle_traduction(),
            ErreurMontant::MultipleInvalide(0).cle_traduction(),
            ErreurNote::AucuneNote.cle_traduction(),
            ErreurNote::HorsBareme {
                valeur_centiemes: 1,
                bareme_centiemes: 1,
            }
            .cle_traduction(),
            ErreurHorodatage::HorsPlage.cle_traduction(),
        ];
        for cle in cles {
            let segments: Vec<&str> = cle.split('.').collect();
            assert_eq!(segments.len(), 3, "clé {cle}");
            assert_eq!(segments.first(), Some(&"erreur"));
        }
    }

    #[test]
    fn le_message_technique_est_en_francais_et_porte_les_valeurs() {
        let message = ErreurNote::HorsBareme {
            valeur_centiemes: 2150,
            bareme_centiemes: 2000,
        }
        .to_string();
        assert!(message.contains("2150") && message.contains("2000"));
    }
}
