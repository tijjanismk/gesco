//! Convocations (SPEC_PHASE_5 §7, blueprint §8.5bis) : le canal effectif entre l'école et le
//! parent est l'enfant, sans accusé de réception. Une convocation sait donc par quel canal elle
//! est partie et si elle a été reçue : confiée à l'élève, elle est « transmise, non confirmée »,
//! ce qui permet de décider de la doubler par SMS. Elle a une suite : honorée, non honorée,
//! reportée.

use serde::{Deserialize, Serialize};

use crate::commun::{DateCivile, Horodatage, Id};
use crate::core::{Etablissement, Utilisateur};
use crate::scolarite::discipline::Sanction;
use crate::scolarite::{Inscription, Responsable};

/// Longueur maximale d'un motif.
pub const MOTIF_MAX: usize = 200;

/// Canal par lequel la convocation est partie.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "snake_case")]
pub enum CanalConvocation {
    /// Confiée à l'élève.
    Eleve,
    /// Remise en main propre au responsable.
    MainPropre,
    /// Par SMS, après relecture.
    Sms,
    /// Par WhatsApp, noté à la main.
    Whatsapp,
}

/// Ce que l'école sait de la réception.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "snake_case")]
pub enum AccuseConvocation {
    /// Le responsable a confirmé l'avoir reçue.
    Confirme,
    /// Partie, sans confirmation.
    TransmisNonConfirme,
    /// Relancé sans réponse.
    SansReponse,
}

/// Suite donnée à la convocation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "snake_case")]
pub enum SuiteConvocation {
    /// Le rendez-vous n'a pas encore eu lieu.
    EnAttente,
    /// Le responsable est venu.
    Honoree,
    /// Le responsable n'est pas venu.
    NonHonoree,
    /// Le rendez-vous est déplacé.
    Reportee,
}

macro_rules! noms {
    ($type:ty, [$($variante:ident => $nom:literal),+ $(,)?]) => {
        impl $type {
            /// Toutes les valeurs.
            pub const TOUS: &'static [Self] = &[$(Self::$variante),+];

            /// Nom persisté.
            pub const fn nom(&self) -> &'static str {
                match self {
                    $(Self::$variante => $nom),+
                }
            }

            /// Lecture d'un nom persisté.
            pub fn depuis_nom(nom: &str) -> Option<Self> {
                Self::TOUS.iter().copied().find(|v| v.nom() == nom)
            }
        }
    };
}

noms!(CanalConvocation, [Eleve => "eleve", MainPropre => "main_propre", Sms => "sms", Whatsapp => "whatsapp"]);
noms!(AccuseConvocation, [Confirme => "confirme", TransmisNonConfirme => "transmis_non_confirme", SansReponse => "sans_reponse"]);
noms!(SuiteConvocation, [EnAttente => "en_attente", Honoree => "honoree", NonHonoree => "non_honoree", Reportee => "reportee"]);

impl CanalConvocation {
    /// Accusé au départ : seule une remise en main propre est confirmée d'emblée.
    pub const fn accuse_initial(&self) -> AccuseConvocation {
        match self {
            Self::MainPropre => AccuseConvocation::Confirme,
            Self::Eleve | Self::Sms | Self::Whatsapp => AccuseConvocation::TransmisNonConfirme,
        }
    }
}

/// Convocation d'un responsable au sujet d'un élève.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
pub struct Convocation {
    /// Identifiant technique.
    pub id: Id<Convocation>,
    /// Établissement.
    pub etablissement_id: Id<Etablissement>,
    /// Inscription de l'élève.
    pub inscription_id: Id<Inscription>,
    /// Responsable convoqué ; aucun : les responsables légaux.
    pub responsable_id: Option<Id<Responsable>>,
    /// Sanction « convocation des parents » dont elle naît.
    pub sanction_id: Option<Id<Sanction>>,
    /// Motif.
    pub motif: String,
    /// Jour du rendez-vous.
    pub date: DateCivile,
    /// Heure du rendez-vous, `HH:MM`.
    pub heure: String,
    /// Canal.
    pub canal: CanalConvocation,
    /// Réception.
    pub accuse: AccuseConvocation,
    /// Suite.
    pub suite: SuiteConvocation,
    /// Émission.
    pub emise_le: Horodatage,
    /// Qui l'a émise.
    pub emise_par: Id<Utilisateur>,
    /// Annulée, avec un motif.
    pub annulee: bool,
    /// Motif de l'annulation.
    pub motif_annulation: String,
}

impl Convocation {
    /// Vrai tant que le rendez-vous n'a pas eu de suite définitive.
    pub const fn en_cours(&self) -> bool {
        !self.annulee
            && matches!(
                self.suite,
                SuiteConvocation::EnAttente | SuiteConvocation::Reportee
            )
    }
}

/// Heure `HH:MM` lisible.
pub fn heure_valide(heure: &str) -> bool {
    let Some((h, m)) = heure.split_once(':') else {
        return false;
    };
    h.len() == 2
        && m.len() == 2
        && h.parse::<u8>().is_ok_and(|h| h < 24)
        && m.parse::<u8>().is_ok_and(|m| m < 60)
}

/// Heure écrite pour un SMS : `10 h 30`, `8 h`.
pub fn heure_lisible(heure: &str) -> String {
    match heure.split_once(':') {
        Some((h, "00")) => format!("{} h", h.trim_start_matches('0').max("0")),
        Some((h, m)) => format!("{} h {m}", h.trim_start_matches('0').max("0")),
        None => heure.to_owned(),
    }
}

/// Vérifie une convocation : un motif, une heure lisible, un rendez-vous qui n'est pas passé.
pub fn verifier_convocation(
    motif: &str,
    heure: &str,
    date: DateCivile,
    aujourd_hui: DateCivile,
) -> Result<(), &'static str> {
    let motif = motif.trim();
    if motif.is_empty() {
        return Err("convocation.motif_obligatoire");
    }
    if motif.chars().count() > MOTIF_MAX {
        return Err("convocation.motif_trop_long");
    }
    if !heure_valide(heure) {
        return Err("convocation.heure_invalide");
    }
    if date < aujourd_hui {
        return Err("convocation.date_passee");
    }
    Ok(())
}

/// Vérifie un changement de suite : seule une convocation en cours en reçoit une ; reportée, elle
/// garde un rendez-vous.
pub fn verifier_suite(
    convocation: &Convocation,
    suite: SuiteConvocation,
) -> Result<(), &'static str> {
    if !convocation.en_cours() {
        return Err("convocation.close");
    }
    if suite == SuiteConvocation::EnAttente {
        return Err("convocation.suite_invalide");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn jour(j: u8) -> DateCivile {
        DateCivile::nouvelle(2026, 10, j).unwrap_or_else(|| unreachable!())
    }

    #[test]
    fn confiee_a_l_eleve_elle_est_transmise_non_confirmee() {
        assert_eq!(
            CanalConvocation::Eleve.accuse_initial(),
            AccuseConvocation::TransmisNonConfirme
        );
        assert_eq!(
            CanalConvocation::MainPropre.accuse_initial(),
            AccuseConvocation::Confirme
        );
        for c in CanalConvocation::TOUS {
            assert_eq!(CanalConvocation::depuis_nom(c.nom()), Some(*c));
        }
        for s in SuiteConvocation::TOUS {
            assert_eq!(SuiteConvocation::depuis_nom(s.nom()), Some(*s));
        }
    }

    #[test]
    fn heures() {
        assert!(heure_valide("10:30"));
        assert!(heure_valide("08:00"));
        assert!(!heure_valide("8:00"));
        assert!(!heure_valide("24:00"));
        assert!(!heure_valide("10h30"));
        assert_eq!(heure_lisible("10:30"), "10 h 30");
        assert_eq!(heure_lisible("08:00"), "8 h");
        assert_eq!(heure_lisible("00:15"), "0 h 15");
    }

    #[test]
    fn convocations_verifiees() {
        assert_eq!(
            verifier_convocation("Retards", "10:30", jour(20), jour(14)),
            Ok(())
        );
        assert_eq!(
            verifier_convocation(" ", "10:30", jour(20), jour(14)),
            Err("convocation.motif_obligatoire")
        );
        assert_eq!(
            verifier_convocation("Retards", "10:30", jour(13), jour(14)),
            Err("convocation.date_passee")
        );
    }
}
