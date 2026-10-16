//! Paie du mois (SPEC_PHASE_7 §5) : une paie par mois et par établissement, calculée puis validée ;
//! un bulletin par membre payé ; les montants saisis par la direction (prime, régularisation,
//! retenue) ; les absences non justifiées qu'elle retient ou non, une à une (T-56).

use serde::{Deserialize, Serialize};

use crate::commun::{Horodatage, Id, Montant};
use crate::core::{Etablissement, MachineAEtats, Utilisateur};
use crate::paie::calcul::ResultatBulletin;
use crate::paie::element::{Calcul, ElementPaie, NatureElement};
use crate::rh::{AbsencePersonnel, MembrePersonnel, Mois};

/// Longueur maximale du libellé d'un montant saisi.
pub const LIBELLE_SAISIE_MAX: usize = 80;

/// État de la paie d'un mois : calculée, elle se refait ; validée, elle ne change plus (§10.6).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "snake_case")]
pub enum StatutPaie {
    /// Calculée : la direction la revoit, la refait autant qu'il faut.
    Calculee,
    /// Validée : figée, ses bulletins numérotés.
    Validee,
}

impl MachineAEtats for StatutPaie {
    const ETATS: &'static [Self] = &[Self::Calculee, Self::Validee];
    const TRANSITIONS: &'static [(Self, Self)] = &[(Self::Calculee, Self::Validee)];

    fn nom(&self) -> &'static str {
        match self {
            Self::Calculee => "calculee",
            Self::Validee => "validee",
        }
    }
}

impl StatutPaie {
    /// Lecture d'un nom persisté.
    pub fn depuis_nom(nom: &str) -> Option<Self> {
        Self::ETATS.iter().copied().find(|s| s.nom() == nom)
    }
}

/// Paie d'un mois.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
pub struct PaieMois {
    /// Identifiant technique.
    pub id: Id<PaieMois>,
    /// Établissement.
    pub etablissement_id: Id<Etablissement>,
    /// Mois payé.
    pub mois: Mois,
    /// État.
    pub statut: StatutPaie,
    /// Dernier calcul.
    pub calculee_le: Horodatage,
    /// Auteur du dernier calcul.
    pub calculee_par: Option<Id<Utilisateur>>,
    /// Validation.
    pub validee_le: Option<Horodatage>,
    /// Auteur de la validation.
    pub validee_par: Option<Id<Utilisateur>>,
}

/// Bulletin de paie d'un membre pour un mois.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
pub struct BulletinPaie {
    /// Identifiant technique.
    pub id: Id<BulletinPaie>,
    /// Établissement.
    pub etablissement_id: Id<Etablissement>,
    /// Paie du mois.
    pub paie_id: Id<PaieMois>,
    /// Membre payé.
    pub membre_id: Id<MembrePersonnel>,
    /// Numéro, donné à la validation.
    pub numero: Option<String>,
    /// Lignes et totaux calculés.
    pub resultat: ResultatBulletin,
    /// Le membre n'est plus payé ce mois-là au dernier calcul : le bulletin reste, hors de la
    /// paie (R-14).
    pub exclu: bool,
}

/// Montant saisi par la direction dans la paie du mois, pour un membre.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
pub struct SaisieMois {
    /// Identifiant technique.
    pub id: Id<SaisieMois>,
    /// Établissement.
    pub etablissement_id: Id<Etablissement>,
    /// Paie du mois.
    pub paie_id: Id<PaieMois>,
    /// Membre.
    pub membre_id: Id<MembrePersonnel>,
    /// Élément saisi : prime, régularisation, retenue.
    pub element_id: Id<ElementPaie>,
    /// Montant ; négatif pour une régularisation qui reprend un trop-perçu.
    pub montant: Montant,
    /// Libellé imprimé : le motif.
    pub libelle: String,
    /// Retirée avant la validation : elle ne compte plus, elle reste (R-14).
    pub retiree: bool,
}

/// Décision de la direction sur une absence non justifiée du mois.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
pub struct AbsenceConfirmee {
    /// Identifiant technique.
    pub id: Id<AbsenceConfirmee>,
    /// Établissement.
    pub etablissement_id: Id<Etablissement>,
    /// Paie du mois.
    pub paie_id: Id<PaieMois>,
    /// Absence.
    pub absence_id: Id<AbsencePersonnel>,
    /// Vrai : retenue sur le salaire ; faux : laissée.
    pub retenue: bool,
}

/// Vérifie un montant saisi : un élément « saisi », un libellé, un montant non nul ; seul un gain
/// peut être négatif (une régularisation qui reprend un trop-perçu).
pub fn verifier_saisie(
    element: &ElementPaie,
    calcul: Option<&Calcul>,
    montant: Montant,
    libelle: &str,
) -> Result<(), &'static str> {
    if !matches!(calcul, Some(Calcul::Saisi)) {
        return Err("paie.element_non_saisi");
    }
    if element.nature == NatureElement::CotisationPatronale {
        return Err("paie.element_non_saisi");
    }
    let libelle = libelle.trim();
    if libelle.is_empty() || libelle.chars().count() > LIBELLE_SAISIE_MAX {
        return Err("paie.libelle_saisie");
    }
    if montant.est_nul() {
        return Err("paie.montant_nul");
    }
    if montant.est_negatif() && element.nature != NatureElement::Gain {
        return Err("paie.montant_negatif");
    }
    Ok(())
}

/// Un mois se calcule s'il est commencé : pas de paie d'un mois à venir.
pub fn mois_calculable(mois: Mois, aujourd_hui: Mois) -> bool {
    mois <= aujourd_hui
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::paie::element::ELEMENTS_LIVRES;

    fn element(nature: NatureElement) -> ElementPaie {
        ElementPaie {
            id: Id::depuis_octets([1; 16]),
            etablissement_id: Id::depuis_octets([2; 16]),
            code: None,
            libelle: "Prime".to_owned(),
            nature,
            natures: Vec::new(),
            fonctionnaires: true,
            ordre: 1,
            actif: true,
        }
    }

    #[test]
    fn une_saisie_a_un_element_saisi_un_libelle_et_un_montant() {
        let gain = element(NatureElement::Gain);
        let retenue = element(NatureElement::Retenue);
        let saisi = Calcul::Saisi;
        let fixe = Calcul::Fixe {
            montant: Montant::depuis_francs(10_000),
        };
        assert_eq!(
            verifier_saisie(
                &gain,
                Some(&saisi),
                Montant::depuis_francs(5_000),
                "Prime d'examen"
            ),
            Ok(())
        );
        // Une régularisation reprend un trop-perçu ; une retenue n'est jamais négative.
        assert_eq!(
            verifier_saisie(
                &gain,
                Some(&saisi),
                Montant::depuis_francs(-5_000),
                "Trop-perçu"
            ),
            Ok(())
        );
        assert_eq!(
            verifier_saisie(
                &retenue,
                Some(&saisi),
                Montant::depuis_francs(-5_000),
                "Erreur"
            ),
            Err("paie.montant_negatif")
        );
        assert_eq!(
            verifier_saisie(&gain, Some(&fixe), Montant::depuis_francs(5_000), "Prime"),
            Err("paie.element_non_saisi")
        );
        assert_eq!(
            verifier_saisie(&gain, Some(&saisi), Montant::ZERO, "Prime"),
            Err("paie.montant_nul")
        );
        assert_eq!(
            verifier_saisie(&gain, Some(&saisi), Montant::depuis_francs(5_000), "  "),
            Err("paie.libelle_saisie")
        );
        assert!(ELEMENTS_LIVRES.iter().any(|e| e.code == "REGUL"));
    }

    #[test]
    fn la_paie_calculee_se_valide_une_fois() {
        assert!(StatutPaie::Calculee.transiter(StatutPaie::Validee).is_ok());
        assert!(StatutPaie::Validee.transiter(StatutPaie::Calculee).is_err());
        let octobre = Mois::nouveau(2026, 10).unwrap_or_else(|| unreachable!());
        let novembre = Mois::nouveau(2026, 11).unwrap_or_else(|| unreachable!());
        assert!(mois_calculable(octobre, octobre));
        assert!(!mois_calculable(novembre, octobre));
    }
}
