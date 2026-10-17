//! Journée de l'école et emploi du temps (SPEC_PHASE_8 §8, T-38, T-64) : la journée de chaque
//! cycle (matin, après-midi certains jours, récréations), l'horaire de canicule du 1er cycle sur
//! une plage de dates, sans après-midi ; les séances placées à la main dans la journée de leur
//! classe. Un enseignant à deux endroits à la fois, une salle prise deux fois, une séance sur une
//! récréation ou plus d'heures que le programme ne prévoit sont signalés, jamais refusés.

use serde::{Deserialize, Serialize};

use crate::commun::{DateCivile, Id};
use crate::core::{AnneeScolaire, Etablissement};
use crate::pedagogie::{Enseignant, Matiere};
use crate::scolarite::{Classe, Cycle};

/// Jours de classe possibles : 1 (lundi) à 6 (samedi).
pub const JOURS: [u8; 6] = [1, 2, 3, 4, 5, 6];

/// Longueur maximale du nom d'une salle.
pub const SALLE_MAX: usize = 40;

/// Plage horaire, en minutes depuis minuit.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
pub struct Plage {
    /// Début.
    pub debut: u16,
    /// Fin, après le début.
    pub fin: u16,
}

impl Plage {
    /// Vrai si la plage est bien formée et tient dans la journée.
    pub const fn valide(&self) -> bool {
        self.debut < self.fin && self.fin <= 24 * 60
    }

    /// Vrai si les deux plages se recouvrent.
    pub const fn recouvre(&self, autre: &Plage) -> bool {
        self.debut < autre.fin && autre.debut < self.fin
    }

    /// Vrai si l'autre plage tient dans celle-ci.
    pub const fn contient(&self, autre: &Plage) -> bool {
        self.debut <= autre.debut && autre.fin <= self.fin
    }

    /// Durée, en minutes.
    pub const fn minutes(&self) -> u16 {
        self.fin.saturating_sub(self.debut)
    }
}

/// Horaire de canicule (T-38) : du … au …, la matinée seule.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
pub struct Canicule {
    /// Premier jour.
    pub du: DateCivile,
    /// Dernier jour.
    pub au: DateCivile,
    /// Matinée.
    pub matin: Plage,
}

/// Journée d'un cycle.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
pub struct JourneeCycle {
    /// Cycle.
    pub cycle: Cycle,
    /// Jours de classe, 1 (lundi) à 6 (samedi).
    pub jours: Vec<u8>,
    /// Matinée.
    pub matin: Plage,
    /// Après-midi, s'il y en a.
    pub apres_midi: Option<Plage>,
    /// Jours sans après-midi.
    pub jours_sans_apres_midi: Vec<u8>,
    /// Récréations.
    pub recreations: Vec<Plage>,
    /// Minutes entre deux séances.
    pub inter_classe: u16,
    /// Horaire de canicule, au 1er cycle seulement.
    pub canicule: Option<Canicule>,
}

const fn h(heures: u16, minutes: u16) -> u16 {
    heures * 60 + minutes
}

impl JourneeCycle {
    /// Journée proposée, que l'école règle : lundi à vendredi, 8 h à 12 h et 15 h à 17 h sauf le
    /// mercredi après-midi, récréation de 10 h à 10 h 15 ; le jardin, le matin seul.
    pub fn par_defaut(cycle: Cycle) -> Self {
        let jardin = cycle == Cycle::Jardin;
        Self {
            cycle,
            jours: vec![1, 2, 3, 4, 5],
            matin: Plage {
                debut: h(8, 0),
                fin: h(12, 0),
            },
            apres_midi: if jardin {
                None
            } else {
                Some(Plage {
                    debut: h(15, 0),
                    fin: h(17, 0),
                })
            },
            jours_sans_apres_midi: if jardin { Vec::new() } else { vec![3] },
            recreations: vec![Plage {
                debut: h(10, 0),
                fin: if jardin { h(10, 30) } else { h(10, 15) },
            }],
            inter_classe: 0,
            canicule: None,
        }
    }

    /// Plages de cours d'un jour de la semaine : la matinée, et l'après-midi s'il y en a.
    pub fn plages(&self, jour: u8) -> Vec<Plage> {
        if !self.jours.contains(&jour) {
            return Vec::new();
        }
        let mut plages = vec![self.matin];
        if let Some(a) = self.apres_midi
            && !self.jours_sans_apres_midi.contains(&jour)
        {
            plages.push(a);
        }
        plages
    }

    /// Horaire d'une date : sous la canicule, la matinée de canicule seule.
    pub fn plages_du(&self, date: DateCivile) -> Vec<Plage> {
        let jour = date.jour_de_semaine() + 1;
        match self.canicule {
            Some(c) if c.du <= date && date <= c.au => {
                if self.jours.contains(&jour) {
                    vec![c.matin]
                } else {
                    Vec::new()
                }
            }
            _ => self.plages(jour),
        }
    }
}

/// Vérifie la journée d'un cycle : des jours de classe, des plages bien formées, l'après-midi
/// après la matinée, les récréations dans la journée, la canicule au 1er cycle seulement.
pub fn verifier_journee(j: &JourneeCycle) -> Result<(), &'static str> {
    if j.jours.is_empty() || j.jours.iter().any(|d| !JOURS.contains(d)) {
        return Err("journee.jours_invalides");
    }
    if !j.matin.valide()
        || j.apres_midi
            .is_some_and(|a| !a.valide() || a.debut < j.matin.fin)
    {
        return Err("journee.horaire_invalide");
    }
    let dans_la_journee = |p: &Plage| {
        p.valide() && (j.matin.contient(p) || j.apres_midi.is_some_and(|a| a.contient(p)))
    };
    if !j.recreations.iter().all(dans_la_journee) {
        return Err("journee.recreation_hors_journee");
    }
    if j.inter_classe > 30 {
        return Err("journee.inter_classe_invalide");
    }
    if let Some(c) = j.canicule {
        if j.cycle != Cycle::PremierCycle {
            return Err("journee.canicule_premier_cycle");
        }
        if c.au < c.du || !c.matin.valide() {
            return Err("journee.canicule_invalide");
        }
    }
    Ok(())
}

/// Séance de l'emploi du temps d'une classe.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
pub struct Seance {
    /// Identifiant technique.
    pub id: Id<Seance>,
    /// Établissement.
    pub etablissement_id: Id<Etablissement>,
    /// Année.
    pub annee_id: Id<AnneeScolaire>,
    /// Classe.
    pub classe_id: Id<Classe>,
    /// Jour, 1 (lundi) à 6 (samedi).
    pub jour: u8,
    /// Heure de début et de fin.
    pub plage: Plage,
    /// Matière.
    pub matiere_id: Id<Matiere>,
    /// Enseignant, s'il est connu.
    pub enseignant_id: Option<Id<Enseignant>>,
    /// Salle, facultative.
    pub salle: String,
    /// Faux pour une séance retirée : elle ne compte plus.
    pub active: bool,
}

/// Vérifie qu'une séance tient dans la journée de sa classe.
pub fn verifier_seance(s: &Seance, journee: &JourneeCycle) -> Result<(), &'static str> {
    if s.salle.trim().chars().count() > SALLE_MAX {
        return Err("seance.salle_invalide");
    }
    if !s.plage.valide() {
        return Err("seance.horaire_invalide");
    }
    if !journee.plages(s.jour).iter().any(|p| p.contient(&s.plage)) {
        return Err("seance.hors_journee");
    }
    Ok(())
}

/// Ce qu'un conflit signale.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "snake_case")]
pub enum NatureConflit {
    /// L'enseignant a une autre séance en même temps.
    Enseignant,
    /// La salle est prise par une autre séance en même temps.
    Salle,
    /// La séance empiète sur une récréation.
    Recreation,
    /// La matière a plus d'heures que le programme ne prévoit.
    Volume,
}

/// Conflit signalé sur une séance.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
pub struct Conflit {
    /// Séance.
    pub seance_id: Id<Seance>,
    /// Nature.
    pub nature: NatureConflit,
    /// Séance avec laquelle elle se heurte, pour un enseignant ou une salle.
    pub avec: Option<Id<Seance>>,
}

/// Conflits des séances actives de l'école : un enseignant ou une salle à deux séances qui se
/// recouvrent le même jour, une séance sur une récréation de sa journée, une matière d'une classe
/// au-delà de son volume hebdomadaire (`volumes` : classe, matière, minutes prévues).
pub fn conflits(
    seances: &[Seance],
    journee_de: impl Fn(Id<Classe>) -> Option<JourneeCycle>,
    volumes: &[(Id<Classe>, Id<Matiere>, i64)],
) -> Vec<Conflit> {
    let actives: Vec<&Seance> = seances.iter().filter(|s| s.active).collect();
    let mut trouves = Vec::new();
    for (i, a) in actives.iter().enumerate() {
        for b in actives.iter().skip(i + 1) {
            if a.jour != b.jour || !a.plage.recouvre(&b.plage) {
                continue;
            }
            if a.enseignant_id.is_some() && a.enseignant_id == b.enseignant_id {
                for (x, y) in [(a, b), (b, a)] {
                    trouves.push(Conflit {
                        seance_id: x.id,
                        nature: NatureConflit::Enseignant,
                        avec: Some(y.id),
                    });
                }
            }
            let salle = a.salle.trim();
            if !salle.is_empty() && salle.eq_ignore_ascii_case(b.salle.trim()) {
                for (x, y) in [(a, b), (b, a)] {
                    trouves.push(Conflit {
                        seance_id: x.id,
                        nature: NatureConflit::Salle,
                        avec: Some(y.id),
                    });
                }
            }
        }
        if let Some(j) = journee_de(a.classe_id)
            && j.recreations.iter().any(|r| r.recouvre(&a.plage))
        {
            trouves.push(Conflit {
                seance_id: a.id,
                nature: NatureConflit::Recreation,
                avec: None,
            });
        }
    }
    for (classe, matiere, prevues) in volumes {
        let siennes: Vec<&&Seance> = actives
            .iter()
            .filter(|s| s.classe_id == *classe && s.matiere_id == *matiere)
            .collect();
        let placees: i64 = siennes.iter().map(|s| i64::from(s.plage.minutes())).sum();
        if placees > *prevues {
            trouves.extend(siennes.iter().map(|s| Conflit {
                seance_id: s.id,
                nature: NatureConflit::Volume,
                avec: None,
            }));
        }
    }
    trouves
}

/// Minutes de cours que des séances donnent sur des jours (SPEC_VERSION_1_1 §1, T-41) : chaque
/// jour, les séances de son jour de la semaine qui tiennent dans l'horaire de ce jour pour leur
/// classe ; sous la canicule, celles de l'après-midi tombent.
pub fn minutes_des_jours(
    seances: &[&Seance],
    journee_de: impl Fn(Id<Classe>) -> Option<JourneeCycle>,
    jours: impl IntoIterator<Item = DateCivile>,
) -> i64 {
    let mut total = 0i64;
    for jour in jours {
        let numero = jour.jour_de_semaine() + 1;
        for s in seances.iter().filter(|s| s.active && s.jour == numero) {
            let Some(journee) = journee_de(s.classe_id) else {
                continue;
            };
            if journee.plages_du(jour).iter().any(|p| p.contient(&s.plage)) {
                total = total.saturating_add(i64::from(s.plage.minutes()));
            }
        }
    }
    total
}

/// Demi-journées d'absence que donnent les absences d'un élève aux séances d'un jour (T-71) :
/// absent à toutes les séances de la matinée (ou de l'après-midi), et à au moins une, il compte
/// la demi-journée. Rend (matin, après-midi).
pub fn demi_journees_absentes(
    journee: &JourneeCycle,
    jour: DateCivile,
    seances_du_jour: &[&Seance],
    absent_a: &[Id<Seance>],
) -> (bool, bool) {
    let plages = journee.plages_du(jour);
    let absent = |plage: Option<&Plage>| {
        let Some(plage) = plage else {
            return false;
        };
        let dedans: Vec<&&Seance> = seances_du_jour
            .iter()
            .filter(|s| plage.contient(&s.plage))
            .collect();
        !dedans.is_empty() && dedans.iter().all(|s| absent_a.contains(&s.id))
    };
    (absent(plages.first()), absent(plages.get(1)))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn seance(octet: u8, classe: u8, jour: u8, debut: u16, fin: u16, prof: Option<u8>) -> Seance {
        Seance {
            id: Id::depuis_octets([octet; 16]),
            etablissement_id: Id::depuis_octets([90; 16]),
            annee_id: Id::depuis_octets([91; 16]),
            classe_id: Id::depuis_octets([classe; 16]),
            jour,
            plage: Plage { debut, fin },
            matiere_id: Id::depuis_octets([92; 16]),
            enseignant_id: prof.map(|p| Id::depuis_octets([p; 16])),
            salle: String::new(),
            active: true,
        }
    }

    #[test]
    fn la_journee_proposee_a_cours_le_matin_et_l_apres_midi_sauf_le_mercredi() {
        let j = JourneeCycle::par_defaut(Cycle::PremierCycle);
        assert_eq!(verifier_journee(&j), Ok(()));
        assert_eq!(j.plages(1).len(), 2);
        assert_eq!(j.plages(3).len(), 1);
        assert!(j.plages(6).is_empty());
        let jardin = JourneeCycle::par_defaut(Cycle::Jardin);
        assert_eq!(jardin.plages(1).len(), 1);
    }

    #[test]
    fn sous_la_canicule_le_premier_cycle_n_a_que_la_matinee() {
        let jour = |j| DateCivile::nouvelle(2027, 4, j).unwrap_or_else(|| unreachable!());
        let mut j = JourneeCycle::par_defaut(Cycle::PremierCycle);
        j.canicule = Some(Canicule {
            du: jour(1),
            au: jour(30),
            matin: Plage {
                debut: h(7, 30),
                fin: h(12, 0),
            },
        });
        assert_eq!(verifier_journee(&j), Ok(()));
        // Le 5 avril 2027 est un lundi.
        assert_eq!(
            j.plages_du(jour(5)),
            vec![Plage {
                debut: h(7, 30),
                fin: h(12, 0)
            }]
        );
        let mut second = JourneeCycle::par_defaut(Cycle::SecondCycle);
        second.canicule = j.canicule;
        assert_eq!(
            verifier_journee(&second),
            Err("journee.canicule_premier_cycle")
        );
    }

    #[test]
    fn une_seance_tient_dans_la_journee_de_sa_classe() {
        let j = JourneeCycle::par_defaut(Cycle::PremierCycle);
        assert_eq!(
            verifier_seance(&seance(1, 1, 1, h(8, 0), h(9, 0), None), &j),
            Ok(())
        );
        assert_eq!(
            verifier_seance(&seance(1, 1, 3, h(15, 0), h(16, 0), None), &j),
            Err("seance.hors_journee")
        );
        assert_eq!(
            verifier_seance(&seance(1, 1, 1, h(11, 30), h(12, 30), None), &j),
            Err("seance.hors_journee")
        );
    }

    #[test]
    fn un_enseignant_a_deux_endroits_et_une_recreation_sont_signales() {
        let j = JourneeCycle::par_defaut(Cycle::PremierCycle);
        let seances = [
            seance(1, 1, 1, h(8, 0), h(9, 0), Some(7)),
            seance(2, 2, 1, h(8, 30), h(9, 30), Some(7)),
            seance(3, 2, 1, h(9, 45), h(10, 30), Some(8)),
        ];
        let c = conflits(&seances, |_| Some(j.clone()), &[]);
        assert_eq!(
            c.iter()
                .filter(|c| c.nature == NatureConflit::Enseignant)
                .count(),
            2
        );
        assert!(
            c.iter().any(|c| c.nature == NatureConflit::Recreation
                && c.seance_id == Id::depuis_octets([3; 16]))
        );
        // Plus d'heures que le programme : 60 minutes prévues, 105 placées en classe 2.
        let volumes = [(Id::depuis_octets([2; 16]), Id::depuis_octets([92; 16]), 60)];
        let c = conflits(&seances, |_| Some(j.clone()), &volumes);
        assert_eq!(
            c.iter()
                .filter(|c| c.nature == NatureConflit::Volume)
                .count(),
            2
        );
    }

    #[test]
    fn les_heures_d_un_mois_se_comptent_d_apres_les_seances_et_la_canicule() {
        let j = JourneeCycle::par_defaut(Cycle::PremierCycle);
        // Lundi 8 h–10 h, mercredi 15 h–16 h (pas d'après-midi le mercredi : ne compte pas).
        let lundi = seance(1, 1, 1, h(8, 0), h(10, 0), Some(7));
        let mercredi = seance(2, 1, 3, h(15, 0), h(16, 0), Some(7));
        let jeudi = seance(3, 1, 4, h(15, 0), h(17, 0), Some(7));
        let seances = [&lundi, &mercredi, &jeudi];
        // Avril 2027 : quatre lundis (5, 12, 19, 26), cinq jeudis (1, 8, 15, 22, 29).
        let avril: Vec<DateCivile> = (1..=30)
            .map(|d| DateCivile::nouvelle(2027, 4, d).unwrap_or_else(|| unreachable!()))
            .collect();
        assert_eq!(
            minutes_des_jours(&seances, |_| Some(j.clone()), avril.clone()),
            4 * 120 + 5 * 120
        );
        // Sous la canicule, l'après-midi du jeudi tombe.
        let mut canicule = j.clone();
        canicule.canicule = Some(Canicule {
            du: DateCivile::nouvelle(2027, 4, 1).unwrap_or_else(|| unreachable!()),
            au: DateCivile::nouvelle(2027, 4, 30).unwrap_or_else(|| unreachable!()),
            matin: Plage {
                debut: h(7, 30),
                fin: h(12, 0),
            },
        });
        assert_eq!(
            minutes_des_jours(&seances, |_| Some(canicule.clone()), avril),
            4 * 120
        );
    }

    #[test]
    fn absent_a_toutes_les_seances_du_matin_il_compte_la_demi_journee() {
        let j = JourneeCycle::par_defaut(Cycle::PremierCycle);
        // Lundi 5 avril 2027 : deux séances le matin, une l'après-midi.
        let lundi = DateCivile::nouvelle(2027, 4, 5).unwrap_or_else(|| unreachable!());
        let a = seance(1, 1, 1, h(8, 0), h(9, 0), None);
        let b = seance(2, 1, 1, h(9, 0), h(10, 0), None);
        let c = seance(3, 1, 1, h(15, 0), h(16, 0), None);
        let du_jour = [&a, &b, &c];
        assert_eq!(
            demi_journees_absentes(&j, lundi, &du_jour, &[a.id, b.id]),
            (true, false)
        );
        assert_eq!(
            demi_journees_absentes(&j, lundi, &du_jour, &[a.id, c.id]),
            (false, true)
        );
        assert_eq!(
            demi_journees_absentes(&j, lundi, &du_jour, &[]),
            (false, false)
        );
    }
}
