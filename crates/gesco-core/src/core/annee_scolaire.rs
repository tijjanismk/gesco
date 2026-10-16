//! Année scolaire : dimension de partition universelle (ADR-012).

use serde::{Deserialize, Serialize};

use super::etablissement::Etablissement;
use super::transition::MachineAEtats;
use crate::commun::{Horodatage, Id};

/// Cycle de vie d'une année (§10.8 du blueprint).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "snake_case")]
pub enum StatutAnnee {
    /// Paramétrage en cours, aucune inscription.
    Preparation,
    /// Année en cours.
    Active,
    /// Clôturée : lecture seule sauf déverrouillage tracé.
    Cloturee,
    /// Archivée après sauvegarde d'archive.
    Archivee,
}

impl MachineAEtats for StatutAnnee {
    const ETATS: &'static [Self] = &[
        Self::Preparation,
        Self::Active,
        Self::Cloturee,
        Self::Archivee,
    ];
    const TRANSITIONS: &'static [(Self, Self)] = &[
        (Self::Preparation, Self::Active),
        (Self::Active, Self::Cloturee),
        (Self::Cloturee, Self::Archivee),
        // Déverrouillage par `academic_year:reopen`, audité (ADR-012).
        (Self::Cloturee, Self::Active),
    ];

    fn nom(&self) -> &'static str {
        match self {
            Self::Preparation => "preparation",
            Self::Active => "active",
            Self::Cloturee => "cloturee",
            Self::Archivee => "archivee",
        }
    }
}

/// Année scolaire d'un établissement.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
pub struct AnneeScolaire {
    /// Identifiant technique.
    pub id: Id<AnneeScolaire>,
    /// Établissement propriétaire (ADR-006).
    pub etablissement_id: Id<Etablissement>,
    /// Libellé affiché, par exemple « 2026-2027 ».
    pub libelle: String,
    /// Premier jour.
    pub debut: Horodatage,
    /// Dernier jour.
    pub fin: Horodatage,
    /// État du cycle de vie.
    pub statut: StatutAnnee,
    /// Une seule année courante par établissement, garantie en base.
    pub est_courante: bool,
}

/// Durée maximale d'une année scolaire : quatorze mois, comptés en 427 jours.
pub const DUREE_MAX_MILLIS: i64 = 427 * 24 * 3_600_000;

/// Vrai pour un libellé `AAAA-AAAA` dont la seconde année suit la première.
fn libelle_valide(libelle: &str) -> bool {
    let Some((premiere, seconde)) = libelle.split_once('-') else {
        return false;
    };
    let lire = |texte: &str| {
        (texte.len() == 4 && texte.chars().all(|c| c.is_ascii_digit()))
            .then(|| texte.parse::<u16>().ok())
            .flatten()
    };
    matches!((lire(premiere), lire(seconde)), (Some(p), Some(s)) if s == p + 1)
}

impl AnneeScolaire {
    /// Vrai si l'année accepte des écritures métier.
    pub fn est_modifiable(&self) -> bool {
        matches!(self.statut, StatutAnnee::Preparation | StatutAnnee::Active)
    }

    /// Vérifie le libellé et les bornes (SPEC_PHASE_1 §3.1) ; l'erreur est la clé du message.
    pub fn verifier(&self) -> Result<(), &'static str> {
        if !libelle_valide(&self.libelle) {
            return Err("annee.libelle_invalide");
        }
        let duree = self.fin.millis_epoque() - self.debut.millis_epoque();
        if duree <= 0 || duree > DUREE_MAX_MILLIS {
            return Err("annee.bornes_invalides");
        }
        Ok(())
    }

    /// Vrai si les deux années ont au moins un jour en commun.
    pub fn chevauche(&self, autre: &Self) -> bool {
        self.debut.millis_epoque() < autre.fin.millis_epoque()
            && autre.debut.millis_epoque() < self.fin.millis_epoque()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::transition::{ErreurTransition, verifier_exhaustivement};

    #[test]
    fn table_des_transitions_exhaustive() {
        verifier_exhaustivement::<StatutAnnee>();
    }

    fn annee(libelle: &str, debut_jour: i64, fin_jour: i64) -> AnneeScolaire {
        let jour = 24 * 3_600_000;
        AnneeScolaire {
            id: Id::depuis_ulid(ulid::Ulid::nil()),
            etablissement_id: Id::depuis_ulid(ulid::Ulid::nil()),
            libelle: libelle.to_owned(),
            debut: Horodatage::depuis_millis_epoque(debut_jour * jour),
            fin: Horodatage::depuis_millis_epoque(fin_jour * jour),
            statut: StatutAnnee::Preparation,
            est_courante: false,
        }
    }

    #[test]
    fn libelle_et_bornes() {
        assert_eq!(annee("2026-2027", 0, 300).verifier(), Ok(()));
        for libelle in ["2026-2028", "2026/2027", "26-27", "2026-2027 ", "abcd-abce"] {
            assert_eq!(
                annee(libelle, 0, 300).verifier(),
                Err("annee.libelle_invalide"),
                "{libelle}"
            );
        }
        assert_eq!(
            annee("2026-2027", 300, 0).verifier(),
            Err("annee.bornes_invalides")
        );
        assert_eq!(
            annee("2026-2027", 0, 428).verifier(),
            Err("annee.bornes_invalides")
        );
    }

    #[test]
    fn chevauchement() {
        let a = annee("2026-2027", 0, 300);
        assert!(a.chevauche(&annee("2027-2028", 299, 600)));
        assert!(!a.chevauche(&annee("2027-2028", 300, 600)));
    }

    #[test]
    fn une_annee_archivee_ne_revient_jamais() {
        assert_eq!(
            StatutAnnee::Archivee.transiter(StatutAnnee::Active),
            Err(ErreurTransition::Interdite {
                depuis: "archivee",
                vers: "active"
            })
        );
        assert_eq!(
            StatutAnnee::Cloturee.transiter(StatutAnnee::Active),
            Ok(StatutAnnee::Active)
        );
        assert_eq!(StatutAnnee::Archivee.suivants(), Vec::<StatutAnnee>::new());
    }
}
