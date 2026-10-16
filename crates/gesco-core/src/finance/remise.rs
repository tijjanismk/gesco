//! Remises (SPEC_PHASE_3 §5, blueprint §10.10) : proposées par le secrétariat, accordées ou
//! refusées par la direction, toujours motivées. Accordée, une remise réduit les échéances non
//! soldées du frais visé en commençant par la dernière (Q-52) ; jamais un reste ne devient
//! négatif.

use serde::{Deserialize, Serialize};

use crate::commun::{DateCivile, ErreurMontant, Id, ModeArrondi, Montant};
use crate::core::{Etablissement, MachineAEtats, Utilisateur};
use crate::finance::facture::Echeance;
use crate::finance::tarif::TypeFrais;
use crate::scolarite::Inscription;

/// Nature d'une remise (blueprint §8.8, T-17).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "snake_case")]
pub enum TypeRemise {
    /// Bourse.
    Bourse,
    /// Demi-bourse.
    DemiBourse,
    /// Réduction accordée à une fratrie.
    Fratrie,
    /// Enfant d'un membre du personnel.
    EnfantPersonnel,
    /// Arrangement avec la famille.
    Arrangement,
}

impl TypeRemise {
    /// Tous les types, dans l'ordre d'affichage.
    pub const TOUS: &'static [Self] = &[
        Self::Bourse,
        Self::DemiBourse,
        Self::Fratrie,
        Self::EnfantPersonnel,
        Self::Arrangement,
    ];

    /// Nom persisté.
    pub const fn nom(&self) -> &'static str {
        match self {
            Self::Bourse => "bourse",
            Self::DemiBourse => "demi_bourse",
            Self::Fratrie => "fratrie",
            Self::EnfantPersonnel => "enfant_personnel",
            Self::Arrangement => "arrangement",
        }
    }

    /// Lecture d'un nom persisté.
    pub fn depuis_nom(nom: &str) -> Option<Self> {
        Self::TOUS.iter().copied().find(|t| t.nom() == nom)
    }
}

/// Valeur d'une remise : un pourcentage entier du frais visé, ou un montant.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ValeurRemise {
    /// Pourcentage entier, de 1 à 100.
    Taux {
        /// Pourcentage.
        pour_cent: i64,
    },
    /// Montant en francs, strictement positif.
    Montant {
        /// Montant.
        montant: Montant,
    },
}

/// État d'une remise (§10.10 du blueprint).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "snake_case")]
pub enum StatutRemise {
    /// Proposée, en attente de la direction.
    Proposee,
    /// Accordée : elle réduit l'échéancier.
    Accordee,
    /// Refusée, avec un motif.
    Refusee,
    /// Annulée après avoir été accordée : contre-passée.
    Annulee,
}

impl MachineAEtats for StatutRemise {
    const ETATS: &'static [Self] = &[Self::Proposee, Self::Accordee, Self::Refusee, Self::Annulee];
    const TRANSITIONS: &'static [(Self, Self)] = &[
        (Self::Proposee, Self::Accordee),
        (Self::Proposee, Self::Refusee),
        (Self::Accordee, Self::Annulee),
    ];

    fn nom(&self) -> &'static str {
        match self {
            Self::Proposee => "proposee",
            Self::Accordee => "accordee",
            Self::Refusee => "refusee",
            Self::Annulee => "annulee",
        }
    }
}

/// Remise d'une inscription.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
pub struct Remise {
    /// Identifiant technique.
    pub id: Id<Remise>,
    /// Établissement.
    pub etablissement_id: Id<Etablissement>,
    /// Inscription.
    pub inscription_id: Id<Inscription>,
    /// Nature.
    pub type_remise: TypeRemise,
    /// Taux ou montant.
    pub valeur: ValeurRemise,
    /// Frais visé : la scolarité, le plus souvent.
    pub type_frais: TypeFrais,
    /// Motif, obligatoire.
    pub motif: String,
    /// État.
    pub statut: StatutRemise,
    /// Auteur de la proposition.
    pub proposee_par: Id<Utilisateur>,
    /// Jour de la proposition.
    pub proposee_le: DateCivile,
    /// Auteur de la décision : accord, refus ou annulation (`autorisee_par` du blueprint).
    pub decidee_par: Option<Id<Utilisateur>>,
    /// Motif du refus ou de l'annulation.
    pub motif_decision: String,
    /// Montant réparti sur l'échéancier quand elle est accordée et facturée.
    pub montant_applique: Montant,
}

/// Vérifie une proposition : motif présent, taux de 1 à 100, montant positif.
pub fn verifier_proposition(valeur: &ValeurRemise, motif: &str) -> Result<(), &'static str> {
    if motif.trim().is_empty() {
        return Err("remise.motif_obligatoire");
    }
    match valeur {
        ValeurRemise::Taux { pour_cent } if !(1..=100).contains(pour_cent) => {
            Err("remise.taux_invalide")
        }
        ValeurRemise::Montant { montant } if montant.francs() <= 0 => {
            Err("remise.montant_invalide")
        }
        _ => Ok(()),
    }
}

/// Montant d'une remise sur un frais facturé `base` : le pourcentage arrondi au plus proche
/// multiple de l'école, sans jamais dépasser la base ; ou le montant tel quel.
pub fn montant_de_la_remise(
    valeur: &ValeurRemise,
    base: Montant,
    multiple: i64,
) -> Result<Montant, ErreurMontant> {
    Ok(match valeur {
        ValeurRemise::Taux { pour_cent } => base
            .pourcentage(*pour_cent)?
            .quotient
            .arrondir_commercial(multiple, ModeArrondi::AuPlusProche)?
            .min(base),
        ValeurRemise::Montant { montant } => *montant,
    })
}

/// Répartit une remise sur des échéances, données dans l'ordre des dates avec leur reste dû :
/// la dernière d'abord, puis en remontant (Q-52). Refuse une remise supérieure au reste.
pub fn repartir_depuis_la_fin(
    echeances: &[(Id<Echeance>, Montant)],
    remise: Montant,
) -> Result<Vec<(Id<Echeance>, Montant)>, &'static str> {
    let reste_total = echeances
        .iter()
        .try_fold(Montant::ZERO, |s, (_, r)| s.ajouter(*r))
        .map_err(|_| "remise.calcul_impossible")?;
    if remise > reste_total {
        return Err("remise.superieure_au_reste");
    }
    let mut a_repartir = remise;
    let mut parts = Vec::new();
    for (id, reste) in echeances.iter().rev() {
        if a_repartir.est_nul() {
            break;
        }
        let part = (*reste).min(a_repartir);
        if !part.est_nul() {
            parts.push((*id, part));
            a_repartir = a_repartir
                .soustraire(part)
                .map_err(|_| "remise.calcul_impossible")?;
        }
    }
    Ok(parts)
}

#[cfg(test)]
#[allow(clippy::indexing_slicing)]
mod tests {
    use super::*;
    use crate::core::transition::verifier_exhaustivement;

    fn f(v: i64) -> Montant {
        Montant::depuis_francs(v)
    }

    fn id<T>(n: u128) -> Id<T> {
        Id::depuis_ulid(ulid::Ulid::from(n))
    }

    #[test]
    fn etats_de_la_remise() {
        verifier_exhaustivement::<StatutRemise>();
    }

    #[test]
    fn une_proposition_est_motivee_et_bornee() {
        let taux = |p| ValeurRemise::Taux { pour_cent: p };
        assert_eq!(verifier_proposition(&taux(50), "Deux frères"), Ok(()));
        assert_eq!(
            verifier_proposition(&taux(50), "  "),
            Err("remise.motif_obligatoire")
        );
        assert_eq!(
            verifier_proposition(&taux(0), "x"),
            Err("remise.taux_invalide")
        );
        assert_eq!(
            verifier_proposition(&taux(101), "x"),
            Err("remise.taux_invalide")
        );
        assert_eq!(
            verifier_proposition(&ValeurRemise::Montant { montant: f(0) }, "x"),
            Err("remise.montant_invalide")
        );
    }

    #[test]
    fn le_taux_s_arrondit_sans_depasser_le_frais() {
        let taux = |p| ValeurRemise::Taux { pour_cent: p };
        assert_eq!(
            montant_de_la_remise(&taux(50), f(150_000), 5),
            Ok(f(75_000))
        );
        // 15 % de 137 503 = 20 625,45 : 20 625.
        assert_eq!(
            montant_de_la_remise(&taux(15), f(137_503), 5),
            Ok(f(20_625))
        );
        // 100 % de 137 503 arrondi à 137 505 : borné au frais.
        assert_eq!(
            montant_de_la_remise(&taux(100), f(137_503), 5),
            Ok(f(137_503))
        );
        assert_eq!(
            montant_de_la_remise(&ValeurRemise::Montant { montant: f(12_345) }, f(1), 5),
            Ok(f(12_345))
        );
    }

    #[test]
    fn la_remise_part_de_la_derniere_echeance() {
        let echeances = [(id(1), f(60_000)), (id(2), f(45_000)), (id(3), f(45_000))];
        assert_eq!(
            repartir_depuis_la_fin(&echeances, f(30_000)),
            Ok(vec![(id(3), f(30_000))])
        );
        assert_eq!(
            repartir_depuis_la_fin(&echeances, f(75_000)),
            Ok(vec![(id(3), f(45_000)), (id(2), f(30_000))])
        );
        assert_eq!(
            repartir_depuis_la_fin(&echeances, f(150_000)).map(|v| v.len()),
            Ok(3)
        );
        assert_eq!(
            repartir_depuis_la_fin(&echeances, f(150_001)),
            Err("remise.superieure_au_reste")
        );
        // Une échéance déjà soldée par une autre remise est sautée.
        let partiel = [(id(1), f(60_000)), (id(2), f(0)), (id(3), f(10_000))];
        assert_eq!(
            repartir_depuis_la_fin(&partiel, f(20_000)),
            Ok(vec![(id(3), f(10_000)), (id(1), f(10_000))])
        );
    }
}
