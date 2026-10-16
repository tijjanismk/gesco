//! Versements et imputations (SPEC_PHASE_3 §7, blueprint §8.8) : un versement se répartit sur
//! les échéances, la plus ancienne d'abord ou à la main ; la somme des imputations égale le
//! versement ; aucune imputation ne dépasse le reste de son échéance ; un versement supérieur au
//! reste dû est refusé (T-18).

use serde::{Deserialize, Serialize};

use crate::commun::{ErreurMontant, Horodatage, Id, Montant};
use crate::core::{Etablissement, MachineAEtats, Utilisateur};
use crate::finance::caisse::{ModePaiement, SessionCaisse};
use crate::finance::facture::{Echeance, Facture, StatutFacture};
use crate::scolarite::{Inscription, Responsable};

/// État d'un versement (§10.3 du blueprint).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "snake_case")]
pub enum StatutVersement {
    /// Encaissé, imputé.
    Valide,
    /// Annulé par contre-passation ; le reçu reste, marqué annulé.
    Annule,
}

impl MachineAEtats for StatutVersement {
    const ETATS: &'static [Self] = &[Self::Valide, Self::Annule];
    const TRANSITIONS: &'static [(Self, Self)] = &[(Self::Valide, Self::Annule)];

    fn nom(&self) -> &'static str {
        match self {
            Self::Valide => "valide",
            Self::Annule => "annule",
        }
    }
}

/// Versement d'un parent pour une inscription.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
pub struct Versement {
    /// Identifiant technique.
    pub id: Id<Versement>,
    /// Établissement.
    pub etablissement_id: Id<Etablissement>,
    /// Inscription.
    pub inscription_id: Id<Inscription>,
    /// Facture réglée.
    pub facture_id: Id<Facture>,
    /// Session de caisse où il est encaissé.
    pub session_id: Id<SessionCaisse>,
    /// Numéro du reçu, sans trou : `REC-2026-000001` (R-46).
    pub numero_recu: String,
    /// Instant de l'encaissement.
    pub encaisse_le: Horodatage,
    /// Montant.
    pub montant: Montant,
    /// Mode de paiement.
    pub mode: ModePaiement,
    /// Référence de la transaction : exigée hors espèces (T-19).
    pub reference: String,
    /// Responsable payeur, s'il est enregistré.
    pub payeur_id: Option<Id<Responsable>>,
    /// Nom du payeur imprimé sur le reçu : le responsable, ou un tiers (§3.6).
    pub payeur_nom: String,
    /// Caissier.
    pub encaisse_par: Id<Utilisateur>,
    /// Clé tirée par l'écran : rejouer la même clé ne double pas la recette (R-35).
    pub cle_idempotence: String,
    /// État.
    pub statut: StatutVersement,
    /// Motif d'annulation.
    pub motif_annulation: String,
}

/// Part d'un versement sur une échéance.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
pub struct Imputation {
    /// Échéance.
    pub echeance_id: Id<Echeance>,
    /// Montant imputé.
    pub montant: Montant,
}

/// Vérifie un versement avant imputation : montant positif, référence hors espèces, payeur nommé.
pub fn verifier_versement(
    montant: Montant,
    mode: ModePaiement,
    reference: &str,
    payeur_nom: &str,
) -> Result<(), &'static str> {
    if montant.francs() <= 0 {
        return Err("versement.montant_invalide");
    }
    if mode.exige_reference() && reference.trim().is_empty() {
        return Err("versement.reference_obligatoire");
    }
    if payeur_nom.trim().is_empty() {
        return Err("versement.payeur_obligatoire");
    }
    Ok(())
}

fn somme(valeurs: impl Iterator<Item = Montant>) -> Result<Montant, ErreurMontant> {
    valeurs
        .into_iter()
        .try_fold(Montant::ZERO, |s, m| s.ajouter(m))
}

/// Vérifie une annulation : un motif, et un versement encore valide (§10.3 du blueprint).
pub fn verifier_annulation(versement: &Versement, motif: &str) -> Result<(), &'static str> {
    if motif.trim().is_empty() {
        return Err("versement.motif_obligatoire");
    }
    if versement.statut != StatutVersement::Valide {
        return Err("versement.deja_annule");
    }
    Ok(())
}

/// Impute un versement sur les échéances, données dans l'ordre des dates avec leur reste : la
/// plus ancienne d'abord (§8.8 du blueprint). Refuse un versement supérieur au reste total.
pub fn imputer_plus_ancienne(
    restes: &[(Id<Echeance>, Montant)],
    montant: Montant,
) -> Result<Vec<Imputation>, &'static str> {
    let total = somme(restes.iter().map(|(_, r)| *r)).map_err(|_| "versement.calcul_impossible")?;
    if montant > total {
        return Err("versement.superieur_au_reste");
    }
    let mut a_imputer = montant;
    let mut imputations = Vec::new();
    for (echeance_id, reste) in restes {
        if a_imputer.est_nul() {
            break;
        }
        let part = (*reste).min(a_imputer);
        if !part.est_nul() {
            imputations.push(Imputation {
                echeance_id: *echeance_id,
                montant: part,
            });
            a_imputer = a_imputer
                .soustraire(part)
                .map_err(|_| "versement.calcul_impossible")?;
        }
    }
    Ok(imputations)
}

/// Vérifie une répartition saisie à la main : des échéances connues, une seule fois chacune, des
/// parts positives qui ne dépassent pas leur reste, et dont la somme égale le versement.
pub fn verifier_repartition(
    restes: &[(Id<Echeance>, Montant)],
    repartition: &[Imputation],
    montant: Montant,
) -> Result<(), &'static str> {
    for (i, imputation) in repartition.iter().enumerate() {
        let reste = restes
            .iter()
            .find(|(id, _)| *id == imputation.echeance_id)
            .map(|(_, r)| *r)
            .ok_or("versement.echeance_inconnue")?;
        if imputation.montant.francs() <= 0 || imputation.montant > reste {
            return Err("versement.part_invalide");
        }
        if repartition
            .iter()
            .skip(i + 1)
            .any(|autre| autre.echeance_id == imputation.echeance_id)
        {
            return Err("versement.part_invalide");
        }
    }
    let total =
        somme(repartition.iter().map(|i| i.montant)).map_err(|_| "versement.calcul_impossible")?;
    if total != montant {
        return Err("versement.repartition_differente");
    }
    Ok(())
}

/// État d'une facture d'après ce qui reste à payer : émise si rien n'est payé, soldée si plus
/// rien n'est dû, partiellement réglée entre les deux (R-20 : toujours recalculable).
pub fn statut_d_apres(paye: Montant, reste: Montant) -> StatutFacture {
    if reste.francs() <= 0 {
        StatutFacture::Soldee
    } else if paye.est_nul() {
        StatutFacture::Emise
    } else {
        StatutFacture::PartiellementReglee
    }
}

#[cfg(test)]
#[allow(clippy::indexing_slicing)]
mod tests {
    use super::*;
    use crate::core::transition::verifier_exhaustivement;
    use proptest::prelude::*;

    fn f(v: i64) -> Montant {
        Montant::depuis_francs(v)
    }

    fn id<T>(n: u128) -> Id<T> {
        Id::depuis_ulid(ulid::Ulid::from(n))
    }

    fn restes() -> Vec<(Id<Echeance>, Montant)> {
        vec![(id(1), f(10_000)), (id(2), f(60_000)), (id(3), f(45_000))]
    }

    #[test]
    fn etats_du_versement() {
        verifier_exhaustivement::<StatutVersement>();
    }

    #[test]
    fn une_annulation_exige_un_motif_et_un_versement_valide() {
        let mut v = Versement {
            id: id(10),
            etablissement_id: id(11),
            inscription_id: id(12),
            facture_id: id(13),
            session_id: id(14),
            numero_recu: "REC-2026-000001".to_owned(),
            encaisse_le: crate::commun::Horodatage::depuis_millis_epoque(0),
            montant: f(40_000),
            mode: ModePaiement::Especes,
            reference: String::new(),
            payeur_id: None,
            payeur_nom: "Mariam Coulibaly".to_owned(),
            encaisse_par: id(15),
            cle_idempotence: "cle-0001".to_owned(),
            statut: StatutVersement::Valide,
            motif_annulation: String::new(),
        };
        assert_eq!(verifier_annulation(&v, "Erreur de saisie"), Ok(()));
        assert_eq!(
            verifier_annulation(&v, " \t"),
            Err("versement.motif_obligatoire")
        );
        v.statut = StatutVersement::Annule;
        assert_eq!(
            verifier_annulation(&v, "Erreur de saisie"),
            Err("versement.deja_annule")
        );
    }

    #[test]
    fn un_versement_se_controle() {
        assert_eq!(
            verifier_versement(f(5_000), ModePaiement::Especes, "", "Mariam Coulibaly"),
            Ok(())
        );
        assert_eq!(
            verifier_versement(f(0), ModePaiement::Especes, "", "x"),
            Err("versement.montant_invalide")
        );
        assert_eq!(
            verifier_versement(f(5_000), ModePaiement::OrangeMoney, " ", "x"),
            Err("versement.reference_obligatoire")
        );
        assert_eq!(
            verifier_versement(f(5_000), ModePaiement::Wave, "W-123", ""),
            Err("versement.payeur_obligatoire")
        );
    }

    #[test]
    fn la_plus_ancienne_echeance_d_abord() {
        assert_eq!(
            imputer_plus_ancienne(&restes(), f(40_000)),
            Ok(vec![
                Imputation {
                    echeance_id: id(1),
                    montant: f(10_000)
                },
                Imputation {
                    echeance_id: id(2),
                    montant: f(30_000)
                },
            ])
        );
        assert_eq!(
            imputer_plus_ancienne(&restes(), f(115_000)).map(|v| v.len()),
            Ok(3)
        );
        assert_eq!(
            imputer_plus_ancienne(&restes(), f(115_001)),
            Err("versement.superieur_au_reste")
        );
        // Une échéance déjà soldée est sautée.
        let partiel = [(id(1), f(0)), (id(2), f(60_000))];
        assert_eq!(
            imputer_plus_ancienne(&partiel, f(5_000)),
            Ok(vec![Imputation {
                echeance_id: id(2),
                montant: f(5_000)
            }])
        );
    }

    #[test]
    fn une_repartition_manuelle_se_verifie() {
        let r = |n, m| Imputation {
            echeance_id: id(n),
            montant: f(m),
        };
        assert_eq!(
            verifier_repartition(&restes(), &[r(3, 45_000), r(1, 5_000)], f(50_000)),
            Ok(())
        );
        assert_eq!(
            verifier_repartition(&restes(), &[r(3, 45_000)], f(50_000)),
            Err("versement.repartition_differente")
        );
        assert_eq!(
            verifier_repartition(&restes(), &[r(1, 10_001)], f(10_001)),
            Err("versement.part_invalide")
        );
        assert_eq!(
            verifier_repartition(&restes(), &[r(9, 1)], f(1)),
            Err("versement.echeance_inconnue")
        );
        assert_eq!(
            verifier_repartition(&restes(), &[r(1, 1), r(1, 1)], f(2)),
            Err("versement.part_invalide")
        );
    }

    #[test]
    fn l_etat_de_la_facture_se_deduit() {
        assert_eq!(statut_d_apres(f(0), f(160_000)), StatutFacture::Emise);
        assert_eq!(
            statut_d_apres(f(40_000), f(120_000)),
            StatutFacture::PartiellementReglee
        );
        assert_eq!(statut_d_apres(f(160_000), f(0)), StatutFacture::Soldee);
        assert_eq!(
            statut_d_apres(f(0), f(0)),
            StatutFacture::Soldee,
            "une facture à zéro est soldée"
        );
    }

    proptest! {
        #[test]
        fn les_imputations_font_exactement_le_versement(
            restes in prop::collection::vec(0i64..200_000, 1..12),
            fraction in 0u32..=100,
        ) {
            let echeances: Vec<(Id<Echeance>, Montant)> = restes
                .iter()
                .enumerate()
                .map(|(i, r)| (id(u128::try_from(i).unwrap_or(0) + 1), f(*r)))
                .collect();
            let total: i64 = restes.iter().sum();
            let montant = total * i64::from(fraction) / 100;
            let imputations = imputer_plus_ancienne(&echeances, f(montant));
            prop_assert!(imputations.is_ok());
            if let Ok(v) = imputations {
                prop_assert_eq!(v.iter().map(|i| i.montant.francs()).sum::<i64>(), montant);
                for i in &v {
                    let reste = echeances.iter().find(|(e, _)| *e == i.echeance_id).map(|(_, r)| *r);
                    prop_assert!(Some(i.montant) <= reste);
                    prop_assert!(i.montant.francs() > 0);
                }
            }
        }
    }
}
