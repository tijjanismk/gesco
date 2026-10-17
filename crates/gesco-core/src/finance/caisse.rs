//! Sessions de caisse (SPEC_PHASE_3 §6, blueprint §10.9) : ouverte avec son fonds, clôturée avec
//! le total compté, visée ensuite. Une session dure autant que le caissier le décide (T-21).
//! Le tiroir ne contient que les espèces : les paiements mobiles, chèques et virements sont
//! totalisés à part.

use serde::{Deserialize, Serialize};

use crate::commun::{ErreurMontant, Horodatage, Id, Montant};
use crate::core::{Etablissement, MachineAEtats, Utilisateur};

/// État d'une session de caisse.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "snake_case")]
pub enum StatutSession {
    /// Ouverte : seuls les encaissements d'une session ouverte sont permis.
    Ouverte,
    /// Clôturée avec le total compté ; ne se rouvre jamais.
    Cloturee,
    /// Visée par la direction ou le comptable.
    Rapprochee,
}

impl MachineAEtats for StatutSession {
    const ETATS: &'static [Self] = &[Self::Ouverte, Self::Cloturee, Self::Rapprochee];
    const TRANSITIONS: &'static [(Self, Self)] = &[
        (Self::Ouverte, Self::Cloturee),
        (Self::Cloturee, Self::Rapprochee),
    ];

    fn nom(&self) -> &'static str {
        match self {
            Self::Ouverte => "ouverte",
            Self::Cloturee => "cloturee",
            Self::Rapprochee => "rapprochee",
        }
    }
}

/// Mode d'un paiement (blueprint §8.8, T-19).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "snake_case")]
pub enum ModePaiement {
    /// Espèces : le seul mode qui entre dans le tiroir.
    Especes,
    /// Orange Money.
    OrangeMoney,
    /// Wave.
    Wave,
    /// Moov Money.
    Moov,
    /// Chèque.
    Cheque,
    /// Virement.
    Virement,
}

impl ModePaiement {
    /// Tous les modes, dans l'ordre d'affichage.
    pub const TOUS: &'static [Self] = &[
        Self::Especes,
        Self::OrangeMoney,
        Self::Wave,
        Self::Moov,
        Self::Cheque,
        Self::Virement,
    ];

    /// Nom persisté.
    pub const fn nom(&self) -> &'static str {
        match self {
            Self::Especes => "especes",
            Self::OrangeMoney => "orange_money",
            Self::Wave => "wave",
            Self::Moov => "moov",
            Self::Cheque => "cheque",
            Self::Virement => "virement",
        }
    }

    /// Lecture d'un nom persisté.
    pub fn depuis_nom(nom: &str) -> Option<Self> {
        Self::TOUS.iter().copied().find(|m| m.nom() == nom)
    }

    /// Tout paiement qui n'est pas en espèces exige la référence de sa transaction (T-19).
    pub const fn exige_reference(&self) -> bool {
        !matches!(self, Self::Especes)
    }
}

/// Session de caisse d'un caissier.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
pub struct SessionCaisse {
    /// Identifiant technique.
    pub id: Id<SessionCaisse>,
    /// Établissement.
    pub etablissement_id: Id<Etablissement>,
    /// Caissier.
    pub caissier_id: Id<Utilisateur>,
    /// Ouverture.
    pub ouverte_le: Horodatage,
    /// Fonds de caisse initial, en espèces.
    pub fonds_initial: Montant,
    /// État.
    pub statut: StatutSession,
    /// Clôture.
    pub cloturee_le: Option<Horodatage>,
    /// Total théorique des espèces à la clôture : fonds, plus entrées, moins sorties.
    pub total_theorique: Option<Montant>,
    /// Total compté par le caissier.
    pub total_compte: Option<Montant>,
    /// Compté moins théorique : négatif s'il manque de l'argent.
    pub ecart: Option<Montant>,
    /// Explication d'un écart non nul.
    pub motif_ecart: String,
    /// Auteur du visa.
    pub rapprochee_par: Option<Id<Utilisateur>>,
    /// Visa.
    pub rapprochee_le: Option<Horodatage>,
    /// Commentaire du visa.
    pub commentaire_visa: String,
}

/// Nature d'un mouvement de caisse hors encaissement.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "snake_case")]
pub enum NatureMouvement {
    /// Sortie qui régularise un versement annulé après la clôture de sa session (phase 3).
    SortieRegularisation,
    /// Salaire payé en espèces de la caisse (SPEC_PHASE_7 §7).
    SortieSalaire,
    /// Avance versée en espèces de la caisse.
    SortieAvance,
    /// Dépense payée en espèces de la caisse (SPEC_PHASE_8 §3).
    SortieDepense,
    /// Espèces déposées sur un autre compte de l'école (SPEC_PHASE_8 §5).
    SortieTransfert,
    /// Entrée qui remet dans la caisse un paiement du personnel annulé.
    EntreeRegularisation,
    /// Espèces retirées d'un autre compte de l'école.
    EntreeTransfert,
    /// Recette hors des versements des parents : un don, une location (SPEC_VERSION_1_1).
    EntreeRecette,
}

impl NatureMouvement {
    /// Toutes les natures.
    pub const TOUTES: [Self; 8] = [
        Self::SortieRegularisation,
        Self::SortieSalaire,
        Self::SortieAvance,
        Self::SortieDepense,
        Self::SortieTransfert,
        Self::EntreeRegularisation,
        Self::EntreeTransfert,
        Self::EntreeRecette,
    ];

    /// Nom persisté.
    pub const fn nom(&self) -> &'static str {
        match self {
            Self::SortieRegularisation => "sortie_regularisation",
            Self::SortieSalaire => "sortie_salaire",
            Self::SortieAvance => "sortie_avance",
            Self::SortieDepense => "sortie_depense",
            Self::SortieTransfert => "sortie_transfert",
            Self::EntreeRegularisation => "entree_regularisation",
            Self::EntreeTransfert => "entree_transfert",
            Self::EntreeRecette => "entree_recette",
        }
    }

    /// Lecture d'un nom persisté.
    pub fn depuis_nom(nom: &str) -> Option<Self> {
        Self::TOUTES.into_iter().find(|n| n.nom() == nom)
    }

    /// Vrai pour un mouvement qui fait entrer de l'argent dans le tiroir.
    pub const fn est_entree(&self) -> bool {
        matches!(
            self,
            Self::EntreeRegularisation | Self::EntreeTransfert | Self::EntreeRecette
        )
    }
}

/// Mouvement de caisse hors encaissement (SPEC_PHASE_3 §8, blueprint §8.8) : la sortie de
/// régularisation d'un versement annulé après la clôture de sa session ; depuis la phase 7, les
/// salaires et les avances payés en espèces, et l'entrée qui remet un paiement annulé.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
pub struct MouvementCaisse {
    /// Identifiant technique.
    pub id: Id<MouvementCaisse>,
    /// Établissement.
    pub etablissement_id: Id<Etablissement>,
    /// Session où la sortie est passée : celle, ouverte, de qui annule.
    pub session_id: Id<SessionCaisse>,
    /// Montant sorti.
    pub montant: crate::commun::Montant,
    /// Mode : seules les espèces sortent du tiroir.
    pub mode: ModePaiement,
    /// Motif.
    pub motif: String,
    /// Versement annulé que la sortie régularise.
    pub versement_id: Option<Id<crate::finance::versement::Versement>>,
    /// Instant.
    pub le: Horodatage,
    /// Auteur.
    pub par: Id<Utilisateur>,
    /// Nature.
    pub nature: NatureMouvement,
    /// Paiement du personnel que le mouvement porte, ou dont il remet l'annulation.
    pub paiement_id: Option<Id<crate::paie::PaiementPersonnel>>,
    /// Dépense que le mouvement porte, ou dont il remet l'annulation.
    pub depense_id: Option<Id<crate::tresorerie::Depense>>,
    /// Transfert que le mouvement porte.
    pub transfert_id: Option<Id<crate::tresorerie::livre::Transfert>>,
    /// Recette que le mouvement porte, ou dont il sort l'annulation.
    pub recette_id: Option<Id<crate::tresorerie::recette::Recette>>,
}

/// Total théorique des espèces : fonds initial, plus entrées, moins sorties (R-35).
pub fn total_theorique(
    fonds: Montant,
    entrees: Montant,
    sorties: Montant,
) -> Result<Montant, ErreurMontant> {
    fonds.ajouter(entrees)?.soustraire(sorties)
}

/// Vérifie l'ouverture : fonds positif ou nul.
pub fn verifier_ouverture(fonds: Montant) -> Result<(), &'static str> {
    if fonds.est_negatif() {
        return Err("caisse.fonds_negatif");
    }
    Ok(())
}

/// Vérifie la clôture et rend l'écart : total compté positif ou nul ; un écart non nul exige
/// un motif (§10.9 du blueprint).
pub fn verifier_cloture(
    theorique: Montant,
    compte: Montant,
    motif: &str,
) -> Result<Montant, &'static str> {
    if compte.est_negatif() {
        return Err("caisse.compte_negatif");
    }
    let ecart = compte
        .soustraire(theorique)
        .map_err(|_| "caisse.calcul_impossible")?;
    if !ecart.est_nul() && motif.trim().is_empty() {
        return Err("caisse.motif_ecart_obligatoire");
    }
    Ok(ecart)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::transition::verifier_exhaustivement;
    use proptest::prelude::*;

    fn f(v: i64) -> Montant {
        Montant::depuis_francs(v)
    }

    #[test]
    fn etats_de_la_session() {
        verifier_exhaustivement::<StatutSession>();
    }

    #[test]
    fn la_cloture_exige_un_motif_pour_tout_ecart() {
        assert_eq!(verifier_cloture(f(50_000), f(50_000), ""), Ok(f(0)));
        assert_eq!(
            verifier_cloture(f(50_000), f(48_000), " "),
            Err("caisse.motif_ecart_obligatoire")
        );
        assert_eq!(
            verifier_cloture(f(50_000), f(48_000), "Billet déchiré rendu au parent"),
            Ok(f(-2_000))
        );
        assert_eq!(
            verifier_cloture(f(50_000), f(51_000), "Pièce trouvée"),
            Ok(f(1_000))
        );
        assert_eq!(
            verifier_cloture(f(0), f(-1), "x"),
            Err("caisse.compte_negatif")
        );
        assert_eq!(verifier_ouverture(f(-1)), Err("caisse.fonds_negatif"));
        assert_eq!(verifier_ouverture(f(0)), Ok(()));
    }

    #[test]
    fn seules_les_especes_se_passent_de_reference() {
        assert!(!ModePaiement::Especes.exige_reference());
        assert!(ModePaiement::OrangeMoney.exige_reference());
        assert!(ModePaiement::Cheque.exige_reference());
        for mode in ModePaiement::TOUS {
            assert_eq!(ModePaiement::depuis_nom(mode.nom()), Some(*mode));
        }
    }

    proptest! {
        #[test]
        fn le_theorique_est_fonds_plus_entrees_moins_sorties(
            fonds in 0i64..10_000_000,
            entrees in prop::collection::vec(0i64..1_000_000, 0..50),
            sorties in prop::collection::vec(0i64..1_000_000, 0..10),
        ) {
            let somme = |v: &[i64]| v.iter().fold(Montant::ZERO, |s, x| s.ajouter(f(*x)).unwrap_or(s));
            let t = total_theorique(f(fonds), somme(&entrees), somme(&sorties));
            prop_assert_eq!(
                t.map(|m| m.francs()),
                Ok(fonds + entrees.iter().sum::<i64>() - sorties.iter().sum::<i64>())
            );
        }
    }
}
