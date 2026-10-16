//! Dépenses (SPEC_PHASE_8 §3, T-61) : chaque sortie d'argent de l'école, rangée dans une catégorie
//! et payée d'un compte. Au-dessus du seuil réglé par l'école, la dépense attend l'accord de la
//! direction ; en dessous, elle naît approuvée. Une dépense payée par erreur s'annule avec un motif
//! (R-14) ; une dépense refusée ne sort jamais.

use serde::{Deserialize, Serialize};

use crate::commun::{DateCivile, Horodatage, Id, Montant};
use crate::core::{Etablissement, MachineAEtats, Utilisateur};
use crate::finance::{ModePaiement, SessionCaisse};
use crate::paie::PaiementPersonnel;
use crate::tresorerie::fournisseur::FactureFournisseur;
use crate::tresorerie::{CategorieTresorerie, CompteTresorerie, NatureCompte};

/// Longueur maximale du libellé, du bénéficiaire et d'un motif.
pub const TEXTE_MAX: usize = 120;

/// État d'une dépense.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "snake_case")]
pub enum StatutDepense {
    /// Au-dessus du seuil : attend l'accord de la direction.
    AApprouver,
    /// Accordée, ou sous le seuil : à payer.
    Approuvee,
    /// Refusée par la direction, avec un motif : elle ne sort pas.
    Refusee,
    /// Payée d'un compte.
    Payee,
    /// Annulée avec un motif : approuvée sans être payée, ou payée par erreur.
    Annulee,
}

impl MachineAEtats for StatutDepense {
    const ETATS: &'static [Self] = &[
        Self::AApprouver,
        Self::Approuvee,
        Self::Refusee,
        Self::Payee,
        Self::Annulee,
    ];
    const TRANSITIONS: &'static [(Self, Self)] = &[
        (Self::AApprouver, Self::Approuvee),
        (Self::AApprouver, Self::Refusee),
        (Self::Approuvee, Self::Payee),
        (Self::Approuvee, Self::Annulee),
        (Self::Payee, Self::Annulee),
    ];

    fn nom(&self) -> &'static str {
        match self {
            Self::AApprouver => "a_approuver",
            Self::Approuvee => "approuvee",
            Self::Refusee => "refusee",
            Self::Payee => "payee",
            Self::Annulee => "annulee",
        }
    }
}

impl StatutDepense {
    /// Lecture d'un nom persisté.
    pub fn depuis_nom(nom: &str) -> Option<Self> {
        Self::ETATS.iter().copied().find(|s| s.nom() == nom)
    }

    /// État d'une dépense qui vient d'être saisie : au-dessus du seuil, elle attend l'accord ;
    /// un seuil nul supprime l'accord.
    pub fn initial(montant: Montant, seuil: Montant) -> Self {
        if seuil.francs() > 0 && montant.francs() > seuil.francs() {
            Self::AApprouver
        } else {
            Self::Approuvee
        }
    }
}

/// Dépense de l'école.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
pub struct Depense {
    /// Identifiant technique.
    pub id: Id<Depense>,
    /// Établissement.
    pub etablissement_id: Id<Etablissement>,
    /// Jour de la dépense.
    pub date: DateCivile,
    /// Catégorie de dépense.
    pub categorie_id: Id<CategorieTresorerie>,
    /// Ce qui est acheté ou payé.
    pub libelle: String,
    /// À qui : un fournisseur, une personne, un organisme.
    pub beneficiaire: String,
    /// Montant.
    pub montant: Montant,
    /// État.
    pub statut: StatutDepense,
    /// Qui l'a saisie.
    pub saisie_par: Id<Utilisateur>,
    /// Qui l'a accordée ou refusée, au-dessus du seuil.
    pub decidee_par: Option<Id<Utilisateur>>,
    /// Compte d'où elle est payée.
    pub compte_id: Option<Id<CompteTresorerie>>,
    /// Mode du paiement.
    pub mode: Option<ModePaiement>,
    /// Référence de la transaction, hors espèces.
    pub reference: String,
    /// Session de caisse d'où sortent les espèces.
    pub session_id: Option<Id<SessionCaisse>>,
    /// Instant du paiement.
    pub payee_le: Option<Horodatage>,
    /// Motif d'un refus ou d'une annulation.
    pub motif: String,
    /// Facture de fournisseur qu'elle paie.
    pub facture_id: Option<Id<FactureFournisseur>>,
    /// Salaire ou avance payé dont elle est née (SPEC_PHASE_8 §6).
    pub paiement_id: Option<Id<PaiementPersonnel>>,
}

/// Vérifie une dépense saisie : un libellé, un bénéficiaire, un montant positif.
pub fn verifier_depense(
    libelle: &str,
    beneficiaire: &str,
    montant: Montant,
) -> Result<(), &'static str> {
    for texte in [libelle, beneficiaire] {
        let t = texte.trim();
        if t.is_empty() || t.chars().count() > TEXTE_MAX {
            return Err("depense.texte_invalide");
        }
    }
    if montant.francs() <= 0 {
        return Err("depense.montant_invalide");
    }
    Ok(())
}

/// Vérifie le paiement d'une dépense : le mode suit la nature du compte (espèces pour la caisse,
/// virement ou chèque pour une banque, mobile money pour un compte mobile money) ; la référence
/// est exigée hors espèces.
pub fn verifier_paiement_depense(
    compte: &CompteTresorerie,
    mode: ModePaiement,
    reference: &str,
) -> Result<(), &'static str> {
    if !compte.actif {
        return Err("depense.compte_ferme");
    }
    let accorde = match compte.nature {
        NatureCompte::Caisse => mode == ModePaiement::Especes,
        NatureCompte::Banque => matches!(mode, ModePaiement::Virement | ModePaiement::Cheque),
        NatureCompte::MobileMoney => matches!(
            mode,
            ModePaiement::OrangeMoney | ModePaiement::Wave | ModePaiement::Moov
        ),
    };
    if !accorde {
        return Err("depense.mode_du_compte");
    }
    if mode.exige_reference() && reference.trim().is_empty() {
        return Err("depense.reference_obligatoire");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn f(n: i64) -> Montant {
        Montant::depuis_francs(n)
    }

    #[test]
    fn au_dessus_du_seuil_la_depense_attend_l_accord() {
        assert_eq!(
            StatutDepense::initial(f(50_000), f(50_000)),
            StatutDepense::Approuvee
        );
        assert_eq!(
            StatutDepense::initial(f(50_001), f(50_000)),
            StatutDepense::AApprouver
        );
        assert_eq!(
            StatutDepense::initial(f(5_000_000), f(0)),
            StatutDepense::Approuvee
        );
        assert!(
            StatutDepense::Refusee
                .transiter(StatutDepense::Payee)
                .is_err()
        );
        assert!(
            StatutDepense::AApprouver
                .transiter(StatutDepense::Payee)
                .is_err()
        );
        assert!(
            StatutDepense::Payee
                .transiter(StatutDepense::Annulee)
                .is_ok()
        );
    }

    #[test]
    fn le_mode_suit_le_compte() {
        let compte = |nature| CompteTresorerie {
            id: Id::depuis_octets([1; 16]),
            etablissement_id: Id::depuis_octets([2; 16]),
            nature,
            libelle: "Compte".to_owned(),
            organisme: "BDM".to_owned(),
            numero: String::new(),
            solde_ouverture: Montant::ZERO,
            ouvert_le: DateCivile::nouvelle(2026, 10, 1).unwrap_or_else(|| unreachable!()),
            actif: true,
        };
        let caisse = compte(NatureCompte::Caisse);
        let banque = compte(NatureCompte::Banque);
        let om = compte(NatureCompte::MobileMoney);
        assert_eq!(
            verifier_paiement_depense(&caisse, ModePaiement::Especes, ""),
            Ok(())
        );
        assert_eq!(
            verifier_paiement_depense(&caisse, ModePaiement::Virement, "V1"),
            Err("depense.mode_du_compte")
        );
        assert_eq!(
            verifier_paiement_depense(&banque, ModePaiement::Cheque, " "),
            Err("depense.reference_obligatoire")
        );
        assert_eq!(
            verifier_paiement_depense(&om, ModePaiement::OrangeMoney, "OM-1"),
            Ok(())
        );
        assert_eq!(
            verifier_depense("Craies", " ", f(5_000)),
            Err("depense.texte_invalide")
        );
        assert_eq!(
            verifier_depense("Craies", "Librairie", f(0)),
            Err("depense.montant_invalide")
        );
    }
}
