//! Engagements du personnel (SPEC_PHASE_6 §3) : ce qui lie chaque membre à l'école, écrit ou non
//! (T-42) ; pour un fonctionnaire de l'État qui donne des heures, un engagement de vacation pour
//! l'année (T-40). Un changement de salaire ou de taux est un nouvel engagement qui succède au
//! précédent à une date (Q-65) : la paie d'un mois passé ne change jamais.

use serde::{Deserialize, Serialize};

use crate::commun::{DateCivile, Id, Montant};
use crate::core::Etablissement;
use crate::core::transition::MachineAEtats;
use crate::rh::MembrePersonnel;

/// Nature d'un engagement.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "snake_case")]
pub enum NatureEngagement {
    /// À durée indéterminée : salaire mensuel, sans fin.
    DureeIndeterminee,
    /// À durée déterminée : salaire mensuel, fin exigée.
    DureeDeterminee,
    /// Vacation : taux de l'heure et heures prévues par semaine, pour l'année.
    Vacation,
    /// Stage : indemnité mensuelle, éventuellement nulle ; fin exigée.
    Stage,
    /// Bénévole : sans rémunération.
    Benevole,
}

impl NatureEngagement {
    /// Les cinq natures.
    pub const TOUTES: [Self; 5] = [
        Self::DureeIndeterminee,
        Self::DureeDeterminee,
        Self::Vacation,
        Self::Stage,
        Self::Benevole,
    ];

    /// Nom persisté.
    pub const fn nom(&self) -> &'static str {
        match self {
            Self::DureeIndeterminee => "duree_indeterminee",
            Self::DureeDeterminee => "duree_determinee",
            Self::Vacation => "vacation",
            Self::Stage => "stage",
            Self::Benevole => "benevole",
        }
    }

    /// Lecture d'un nom persisté.
    pub fn depuis_nom(nom: &str) -> Option<Self> {
        Self::TOUTES.into_iter().find(|n| n.nom() == nom)
    }

    /// Vrai si la nature exige une date de fin.
    pub const fn fin_exigee(&self) -> bool {
        matches!(self, Self::DureeDeterminee | Self::Stage)
    }
}

/// Rémunération prévue par l'engagement ; la paie (phase 7) en part.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Remuneration {
    /// Salaire de base ou indemnité, par mois.
    Mensuelle {
        /// Montant mensuel.
        montant: Montant,
    },
    /// Vacation : taux de l'heure et heures prévues par semaine, en minutes (quarts d'heure).
    Horaire {
        /// Taux de l'heure.
        taux: Montant,
        /// Heures prévues par semaine, en minutes.
        minutes_par_semaine: i64,
    },
    /// Aucune.
    Aucune,
    /// Montants retirés par le service pour un compte sans `contracts:read` (SPEC_PHASE_6 §7,
    /// Q-67) ; jamais enregistrée : le contrôle d'un engagement la refuse.
    Masquee,
}

impl Remuneration {
    /// La même rémunération, montants retirés : un engagement sans rémunération reste lisible tel
    /// quel, il n'a aucun montant à cacher.
    pub const fn sans_montants(&self) -> Self {
        match self {
            Self::Aucune => Self::Aucune,
            _ => Self::Masquee,
        }
    }
}

/// État d'un engagement.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "snake_case")]
pub enum StatutEngagement {
    /// En cours.
    EnCours,
    /// Arrivé à sa fin, ou arrêté avant avec un motif, ou remplacé par un engagement nouveau.
    Termine,
    /// Rompu, avec un motif.
    Rompu,
}

impl MachineAEtats for StatutEngagement {
    const ETATS: &'static [Self] = &[Self::EnCours, Self::Termine, Self::Rompu];
    const TRANSITIONS: &'static [(Self, Self)] =
        &[(Self::EnCours, Self::Termine), (Self::EnCours, Self::Rompu)];

    fn nom(&self) -> &'static str {
        match self {
            Self::EnCours => "en_cours",
            Self::Termine => "termine",
            Self::Rompu => "rompu",
        }
    }
}

impl StatutEngagement {
    /// Lecture d'un nom persisté.
    pub fn depuis_nom(nom: &str) -> Option<Self> {
        Self::ETATS.iter().copied().find(|s| s.nom() == nom)
    }
}

/// Engagement d'un membre du personnel.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
pub struct Engagement {
    /// Identifiant technique.
    pub id: Id<Engagement>,
    /// Établissement.
    pub etablissement_id: Id<Etablissement>,
    /// Membre engagé.
    pub membre_id: Id<MembrePersonnel>,
    /// Nature.
    pub nature: NatureEngagement,
    /// Premier jour.
    pub debut: DateCivile,
    /// Dernier jour prévu ; absent pour une durée indéterminée.
    pub fin: Option<DateCivile>,
    /// Fonction pour cet engagement.
    pub fonction: String,
    /// Rémunération.
    pub remuneration: Remuneration,
    /// Payé douze mois ; faux : seulement hors des mois de vacances de l'école (SPEC_PHASE_7
    /// §2, Q-73). Sans effet sur une vacation, payée à l'heure.
    pub paye_douze_mois: bool,
    /// État.
    pub statut: StatutEngagement,
    /// Dernier jour effectif d'un engagement terminé ou rompu.
    pub arrete_le: Option<DateCivile>,
    /// Motif de la fin.
    pub motif_fin: String,
    /// Engagement qu'il renouvelle ou remplace.
    pub precedent_id: Option<Id<Engagement>>,
    /// Note libre : conditions particulières, matières.
    pub note: String,
}

impl Engagement {
    /// Le même engagement, sa rémunération sans montants.
    pub fn sans_montants(mut self) -> Self {
        self.remuneration = self.remuneration.sans_montants();
        self
    }

    /// Dernier jour : celui où il a été arrêté, sinon sa fin prévue ; aucun s'il est sans fin.
    pub fn dernier_jour(&self) -> Option<DateCivile> {
        self.arrete_le.or(self.fin)
    }

    /// Vrai si l'engagement couvre ce jour.
    pub fn couvre(&self, jour: DateCivile) -> bool {
        self.debut <= jour && self.dernier_jour().is_none_or(|f| jour <= f)
    }
}

/// Un engagement à durée déterminée ou un stage qui finit dans moins de soixante jours est
/// signalé (T-42).
pub const PREAVIS_FIN_JOURS: i64 = 60;

/// Plus grand nombre d'heures de vacation par semaine : 60 heures.
pub const MINUTES_PAR_SEMAINE_MAX: i64 = 60 * 60;

/// Contrôle d'un engagement avant écriture ; l'erreur est la clé du message.
pub fn verifier_engagement(e: &Engagement) -> Result<(), &'static str> {
    if e.fonction.trim().is_empty() {
        return Err("engagement.fonction_obligatoire");
    }
    if e.nature.fin_exigee() && e.fin.is_none() {
        return Err("engagement.fin_obligatoire");
    }
    if e.fin.is_some_and(|f| f < e.debut) || e.arrete_le.is_some_and(|a| a < e.debut) {
        return Err("engagement.fin_avant_debut");
    }
    match (e.nature, &e.remuneration) {
        (
            NatureEngagement::DureeIndeterminee | NatureEngagement::DureeDeterminee,
            Remuneration::Mensuelle { montant },
        ) if montant.francs() > 0 => Ok(()),
        (NatureEngagement::DureeIndeterminee | NatureEngagement::DureeDeterminee, _) => {
            Err("engagement.salaire_obligatoire")
        }
        (
            NatureEngagement::Vacation,
            Remuneration::Horaire {
                taux,
                minutes_par_semaine,
            },
        ) => {
            if taux.francs() <= 0 {
                Err("engagement.taux_obligatoire")
            } else if *minutes_par_semaine <= 0
                || *minutes_par_semaine > MINUTES_PAR_SEMAINE_MAX
                || minutes_par_semaine % 15 != 0
            {
                Err("engagement.heures_invalides")
            } else {
                Ok(())
            }
        }
        (NatureEngagement::Vacation, _) => Err("engagement.taux_obligatoire"),
        (NatureEngagement::Stage, Remuneration::Mensuelle { montant })
            if !montant.est_negatif() =>
        {
            Ok(())
        }
        (NatureEngagement::Stage, Remuneration::Aucune)
        | (NatureEngagement::Benevole, Remuneration::Aucune) => Ok(()),
        (NatureEngagement::Stage, _) => Err("engagement.indemnite_invalide"),
        (NatureEngagement::Benevole, _) => Err("engagement.benevole_sans_remuneration"),
    }
}

/// Vrai si deux engagements couvrent au moins un même jour.
pub fn se_chevauchent(a: &Engagement, b: &Engagement) -> bool {
    let apres = |x: &Engagement, y: &Engagement| x.dernier_jour().is_some_and(|f| f < y.debut);
    !apres(a, b) && !apres(b, a)
}

/// Vrai si le nouvel engagement chevauche un engagement en cours du même membre, hors celui qu'il
/// remplace.
pub fn chevauche_un_engagement_en_cours(
    existants: &[Engagement],
    nouveau: &Engagement,
    remplace: Option<Id<Engagement>>,
) -> bool {
    existants.iter().any(|e| {
        e.membre_id == nouveau.membre_id
            && e.id != nouveau.id
            && Some(e.id) != remplace
            && e.statut == StatutEngagement::EnCours
            && se_chevauchent(e, nouveau)
    })
}

/// Vrai si l'engagement en cours finit dans moins de soixante jours, ou est déjà échu sans avoir
/// été clos.
pub fn fin_proche(e: &Engagement, aujourd_hui: DateCivile) -> bool {
    e.statut == StatutEngagement::EnCours
        && e.nature != NatureEngagement::DureeIndeterminee
        && e.fin
            .is_some_and(|f| aujourd_hui.jours_jusqua(f) <= PREAVIS_FIN_JOURS)
}

/// Dates proposées au renouvellement : l'année scolaire suivante pour une vacation quand elle est
/// connue ; sinon la même durée, à partir du lendemain de la fin. Rien pour un engagement sans
/// fin.
pub fn dates_de_renouvellement(
    e: &Engagement,
    annee_suivante: Option<(DateCivile, DateCivile)>,
) -> Option<(DateCivile, DateCivile)> {
    let fin = e.fin?;
    if e.nature == NatureEngagement::Vacation
        && let Some((debut, fin_annee)) = annee_suivante
        && debut > fin
    {
        return Some((debut, fin_annee));
    }
    let debut = fin.decaler(1)?;
    // Un engagement de mois entiers (du 1er janvier au 30 juin) se renouvelle d'autant de mois ;
    // un autre, d'autant de jours.
    if debut.jour() == e.debut.jour() {
        let mois = (debut.annee() - e.debut.annee()) * 12 + i32::from(debut.mois())
            - i32::from(e.debut.mois());
        return Some((debut, ajouter_mois(debut, mois)?.decaler(-1)?));
    }
    Some((debut, debut.decaler(e.debut.jours_jusqua(fin))?))
}

/// La date `mois` mois plus tard, au même jour du mois ; `None` si ce jour n'existe pas.
fn ajouter_mois(date: DateCivile, mois: i32) -> Option<DateCivile> {
    let rang = date.annee() * 12 + i32::from(date.mois()) - 1 + mois;
    DateCivile::nouvelle(
        rang.div_euclid(12),
        u8::try_from(rang.rem_euclid(12) + 1).ok()?,
        date.jour(),
    )
}

/// Heures et minutes lisibles : « 6 h », « 6 h 30 ».
pub fn heures_lisibles(minutes: i64) -> String {
    let (h, m) = (minutes / 60, minutes % 60);
    if m == 0 {
        format!("{h} h")
    } else {
        format!("{h} h {m:02}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn jour(a: i32, m: u8, j: u8) -> DateCivile {
        DateCivile::nouvelle(a, m, j).unwrap_or_else(|| unreachable!())
    }

    fn engagement(
        nature: NatureEngagement,
        debut: DateCivile,
        fin: Option<DateCivile>,
    ) -> Engagement {
        Engagement {
            id: Id::depuis_octets([1; 16]),
            etablissement_id: Id::depuis_octets([2; 16]),
            membre_id: Id::depuis_octets([3; 16]),
            nature,
            debut,
            fin,
            fonction: "Professeur de mathématiques".to_owned(),
            remuneration: match nature {
                NatureEngagement::Vacation => Remuneration::Horaire {
                    taux: Montant::depuis_francs(2_500),
                    minutes_par_semaine: 6 * 60,
                },
                NatureEngagement::Benevole => Remuneration::Aucune,
                _ => Remuneration::Mensuelle {
                    montant: Montant::depuis_francs(150_000),
                },
            },
            paye_douze_mois: true,
            statut: StatutEngagement::EnCours,
            arrete_le: None,
            motif_fin: String::new(),
            precedent_id: None,
            note: String::new(),
        }
    }

    #[test]
    fn la_table_des_etats_est_exhaustive() {
        use StatutEngagement::*;
        for depuis in StatutEngagement::ETATS {
            for vers in StatutEngagement::ETATS {
                let permise = matches!((depuis, vers), (EnCours, Termine) | (EnCours, Rompu));
                assert_eq!(
                    depuis.transiter(*vers).is_ok(),
                    permise,
                    "{depuis:?} → {vers:?}"
                );
            }
        }
    }

    #[test]
    fn chaque_nature_a_sa_remuneration() {
        let rentree = jour(2026, 10, 1);
        let fin = Some(jour(2027, 6, 30));
        for nature in NatureEngagement::TOUTES {
            let e = engagement(nature, rentree, fin);
            assert_eq!(verifier_engagement(&e), Ok(()), "{nature:?}");
        }
        let mut e = engagement(NatureEngagement::DureeDeterminee, rentree, None);
        assert_eq!(verifier_engagement(&e), Err("engagement.fin_obligatoire"));
        e.fin = Some(jour(2026, 9, 30));
        assert_eq!(verifier_engagement(&e), Err("engagement.fin_avant_debut"));
        let mut e = engagement(NatureEngagement::Vacation, rentree, fin);
        e.remuneration = Remuneration::Horaire {
            taux: Montant::depuis_francs(2_500),
            minutes_par_semaine: 6 * 60 + 10,
        };
        assert_eq!(verifier_engagement(&e), Err("engagement.heures_invalides"));
        e.remuneration = Remuneration::Mensuelle {
            montant: Montant::depuis_francs(50_000),
        };
        assert_eq!(verifier_engagement(&e), Err("engagement.taux_obligatoire"));
        let mut e = engagement(NatureEngagement::DureeIndeterminee, rentree, None);
        e.remuneration = Remuneration::Aucune;
        assert_eq!(
            verifier_engagement(&e),
            Err("engagement.salaire_obligatoire")
        );
        let mut e = engagement(NatureEngagement::Benevole, rentree, None);
        e.remuneration = Remuneration::Mensuelle {
            montant: Montant::depuis_francs(1),
        };
        assert_eq!(
            verifier_engagement(&e),
            Err("engagement.benevole_sans_remuneration")
        );
        let mut e = engagement(NatureEngagement::Stage, rentree, fin);
        e.remuneration = Remuneration::Mensuelle {
            montant: Montant::ZERO,
        };
        assert_eq!(verifier_engagement(&e), Ok(()));
    }

    #[test]
    fn une_remuneration_masquee_ne_s_enregistre_jamais() {
        let rentree = jour(2026, 10, 1);
        let fin = Some(jour(2027, 6, 30));
        for nature in NatureEngagement::TOUTES {
            let e = engagement(nature, rentree, fin).sans_montants();
            if nature == NatureEngagement::Benevole {
                assert_eq!(e.remuneration, Remuneration::Aucune);
            } else {
                assert_eq!(e.remuneration, Remuneration::Masquee, "{nature:?}");
                assert!(verifier_engagement(&e).is_err(), "{nature:?}");
            }
        }
    }

    #[test]
    fn deux_engagements_en_cours_ne_se_chevauchent_pas() {
        let cdi = engagement(NatureEngagement::DureeIndeterminee, jour(2024, 10, 1), None);
        let mut augmente = engagement(NatureEngagement::DureeIndeterminee, jour(2026, 11, 1), None);
        augmente.id = Id::depuis_octets([9; 16]);
        assert!(se_chevauchent(&cdi, &augmente));
        assert!(chevauche_un_engagement_en_cours(
            std::slice::from_ref(&cdi),
            &augmente,
            None
        ));
        // Le nouveau salaire remplace l'ancien à une date : pas de chevauchement.
        assert!(!chevauche_un_engagement_en_cours(
            std::slice::from_ref(&cdi),
            &augmente,
            Some(cdi.id)
        ));
        // Un engagement terminé la veille ne chevauche plus.
        let mut termine = cdi.clone();
        termine.statut = StatutEngagement::Termine;
        termine.arrete_le = Some(jour(2026, 10, 31));
        assert!(!se_chevauchent(&termine, &augmente));
    }

    #[test]
    fn la_fin_proche_se_signale_soixante_jours_avant() {
        let cdd = engagement(
            NatureEngagement::DureeDeterminee,
            jour(2026, 1, 1),
            Some(jour(2026, 12, 31)),
        );
        assert!(!fin_proche(&cdd, jour(2026, 10, 31)));
        assert!(fin_proche(&cdd, jour(2026, 11, 1)));
        assert!(fin_proche(&cdd, jour(2027, 1, 15)));
        let cdi = engagement(NatureEngagement::DureeIndeterminee, jour(2026, 1, 1), None);
        assert!(!fin_proche(&cdi, jour(2026, 12, 31)));
    }

    #[test]
    fn un_renouvellement_reprend_l_annee_suivante_ou_la_meme_duree() {
        let vacation = engagement(
            NatureEngagement::Vacation,
            jour(2026, 10, 1),
            Some(jour(2027, 6, 30)),
        );
        assert_eq!(
            dates_de_renouvellement(&vacation, Some((jour(2027, 10, 1), jour(2028, 6, 30)))),
            Some((jour(2027, 10, 1), jour(2028, 6, 30)))
        );
        let cdd = engagement(
            NatureEngagement::DureeDeterminee,
            jour(2026, 1, 1),
            Some(jour(2026, 6, 30)),
        );
        assert_eq!(
            dates_de_renouvellement(&cdd, None),
            Some((jour(2026, 7, 1), jour(2026, 12, 31)))
        );
        let cdd_jours = engagement(
            NatureEngagement::Stage,
            jour(2026, 1, 10),
            Some(jour(2026, 3, 20)),
        );
        assert_eq!(
            dates_de_renouvellement(&cdd_jours, None),
            Some((jour(2026, 3, 21), jour(2026, 5, 29)))
        );
        let cdi = engagement(NatureEngagement::DureeIndeterminee, jour(2026, 1, 1), None);
        assert_eq!(dates_de_renouvellement(&cdi, None), None);
    }

    #[test]
    fn les_heures_se_lisent() {
        assert_eq!(heures_lisibles(360), "6 h");
        assert_eq!(heures_lisibles(390), "6 h 30");
        assert_eq!(heures_lisibles(15), "0 h 15");
    }
}
