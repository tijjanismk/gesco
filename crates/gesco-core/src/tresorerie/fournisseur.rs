//! Fournisseurs et leurs factures (SPEC_PHASE_8 §4) : une facture se paie en une ou plusieurs
//! dépenses ; ce que l'école doit à chaque fournisseur se lit. Une facture saisie par erreur
//! s'annule avec un motif, tant qu'aucun paiement ne la touche.

use serde::{Deserialize, Serialize};

use crate::commun::{DateCivile, Id, Montant};
use crate::core::Etablissement;
use crate::tresorerie::CategorieTresorerie;

/// Longueur maximale d'un nom, d'un numéro ou d'un libellé.
pub const TEXTE_FOURNISSEUR_MAX: usize = 120;

/// Fournisseur de l'école.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
pub struct Fournisseur {
    /// Identifiant technique.
    pub id: Id<Fournisseur>,
    /// Établissement.
    pub etablissement_id: Id<Etablissement>,
    /// Nom.
    pub nom: String,
    /// Téléphone.
    pub telephone: String,
    /// Ce qu'il fournit.
    pub fourniture: String,
    /// Faux pour un fournisseur retiré : ses factures restent.
    pub actif: bool,
}

/// Facture d'un fournisseur.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
pub struct FactureFournisseur {
    /// Identifiant technique.
    pub id: Id<FactureFournisseur>,
    /// Établissement.
    pub etablissement_id: Id<Etablissement>,
    /// Fournisseur.
    pub fournisseur_id: Id<Fournisseur>,
    /// Numéro porté par le fournisseur.
    pub numero: String,
    /// Jour de la facture.
    pub date: DateCivile,
    /// Échéance, si le fournisseur en donne une.
    pub echeance: Option<DateCivile>,
    /// Ce qui est facturé.
    pub libelle: String,
    /// Catégorie de dépense des paiements.
    pub categorie_id: Id<CategorieTresorerie>,
    /// Montant.
    pub montant: Montant,
    /// Annulée, avec son motif.
    pub annulee: bool,
    /// Motif de l'annulation.
    pub motif_annulation: String,
}

fn texte(t: &str, obligatoire: bool) -> bool {
    let t = t.trim();
    (!obligatoire || !t.is_empty()) && t.chars().count() <= TEXTE_FOURNISSEUR_MAX
}

/// Vérifie un fournisseur : un nom.
pub fn verifier_fournisseur(f: &Fournisseur) -> Result<(), &'static str> {
    if !texte(&f.nom, true) || !texte(&f.telephone, false) || !texte(&f.fourniture, false) {
        return Err("fournisseur.texte_invalide");
    }
    Ok(())
}

/// Vérifie une facture : un libellé, un montant positif, une échéance qui ne précède pas la date.
pub fn verifier_facture(f: &FactureFournisseur) -> Result<(), &'static str> {
    if !texte(&f.libelle, true) || !texte(&f.numero, false) {
        return Err("fournisseur.texte_invalide");
    }
    if f.montant.francs() <= 0 {
        return Err("fournisseur.montant_invalide");
    }
    if f.echeance.is_some_and(|e| e < f.date) {
        return Err("fournisseur.echeance_avant_date");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn une_facture_a_un_libelle_un_montant_et_une_echeance_apres_sa_date() {
        let jour = |j| DateCivile::nouvelle(2026, 10, j).unwrap_or_else(|| unreachable!());
        let mut f = FactureFournisseur {
            id: Id::depuis_octets([1; 16]),
            etablissement_id: Id::depuis_octets([2; 16]),
            fournisseur_id: Id::depuis_octets([3; 16]),
            numero: "F-2026-118".to_owned(),
            date: jour(5),
            echeance: Some(jour(30)),
            libelle: "Riz et huile".to_owned(),
            categorie_id: Id::depuis_octets([4; 16]),
            montant: Montant::depuis_francs(180_000),
            annulee: false,
            motif_annulation: String::new(),
        };
        assert_eq!(verifier_facture(&f), Ok(()));
        f.echeance = Some(jour(1));
        assert_eq!(verifier_facture(&f), Err("fournisseur.echeance_avant_date"));
        f.echeance = None;
        f.montant = Montant::ZERO;
        assert_eq!(verifier_facture(&f), Err("fournisseur.montant_invalide"));
        let fournisseur = Fournisseur {
            id: Id::depuis_octets([3; 16]),
            etablissement_id: Id::depuis_octets([2; 16]),
            nom: " ".to_owned(),
            telephone: String::new(),
            fourniture: String::new(),
            actif: true,
        };
        assert_eq!(
            verifier_fournisseur(&fournisseur),
            Err("fournisseur.texte_invalide")
        );
    }
}
