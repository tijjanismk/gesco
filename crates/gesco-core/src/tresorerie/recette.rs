//! Autres recettes (SPEC_VERSION_1_1 §1, étape 1) : l'argent qui entre hors des versements des
//! parents — un don, une location de salle —, rangé dans une catégorie de recette et reçu sur un
//! compte de l'école. Une recette saisie par erreur s'annule avec un motif (R-14).

use serde::{Deserialize, Serialize};

use crate::commun::{DateCivile, Id, Montant};
use crate::core::{Etablissement, Utilisateur};
use crate::finance::{ModePaiement, SessionCaisse};
use crate::tresorerie::{CategorieTresorerie, CompteTresorerie};

/// Longueur maximale du libellé et du payeur.
pub const TEXTE_RECETTE_MAX: usize = 120;

/// Recette de l'école hors des versements des parents.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
pub struct Recette {
    /// Identifiant technique.
    pub id: Id<Recette>,
    /// Établissement.
    pub etablissement_id: Id<Etablissement>,
    /// Jour.
    pub date: DateCivile,
    /// Catégorie de recette.
    pub categorie_id: Id<CategorieTresorerie>,
    /// Ce qui est reçu.
    pub libelle: String,
    /// Qui paie : un donateur, un locataire, un organisme.
    pub payeur: String,
    /// Montant.
    pub montant: Montant,
    /// Compte où l'argent entre.
    pub compte_id: Id<CompteTresorerie>,
    /// Mode.
    pub mode: ModePaiement,
    /// Référence de la transaction, hors espèces.
    pub reference: String,
    /// Session de caisse où entrent les espèces.
    pub session_id: Option<Id<SessionCaisse>>,
    /// Qui l'a saisie.
    pub saisie_par: Id<Utilisateur>,
    /// Annulée, avec son motif.
    pub annulee: bool,
    /// Motif de l'annulation.
    pub motif_annulation: String,
}

/// Vérifie une recette : un libellé, un payeur, un montant positif.
pub fn verifier_recette(libelle: &str, payeur: &str, montant: Montant) -> Result<(), &'static str> {
    for texte in [libelle, payeur] {
        let t = texte.trim();
        if t.is_empty() || t.chars().count() > TEXTE_RECETTE_MAX {
            return Err("recette.texte_invalide");
        }
    }
    if montant.francs() <= 0 {
        return Err("recette.montant_invalide");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn une_recette_a_un_libelle_un_payeur_et_un_montant_positif() {
        let m = Montant::depuis_francs;
        assert_eq!(
            verifier_recette(
                "Don pour la bibliothèque",
                "Association des parents",
                m(100_000)
            ),
            Ok(())
        );
        assert_eq!(
            verifier_recette(" ", "Mairie", m(5_000)),
            Err("recette.texte_invalide")
        );
        assert_eq!(
            verifier_recette("Location de la salle", "", m(5_000)),
            Err("recette.texte_invalide")
        );
        assert_eq!(
            verifier_recette("Location de la salle", "Mairie", m(0)),
            Err("recette.montant_invalide")
        );
    }
}
