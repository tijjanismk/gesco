//! Absences, retards et congés du personnel (SPEC_PHASE_6 §5 et §6, T-43, T-44) : les absences et
//! les retards se notent, sans pointage ; un congé se demande, puis la direction l'accorde ou le
//! refuse. Le décompte des jours acquis est facultatif, désactivé par défaut.

use serde::{Deserialize, Serialize};

use crate::commun::{DateCivile, Horodatage, Id};
use crate::core::transition::MachineAEtats;
use crate::core::{Etablissement, Utilisateur};
use crate::rh::MembrePersonnel;

/// Nature d'une absence du personnel.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "snake_case")]
pub enum NatureAbsencePersonnel {
    /// Absent toute la journée.
    Journee,
    /// Absent une demi-journée.
    DemiJournee,
    /// Arrivé en retard.
    Retard,
}

impl NatureAbsencePersonnel {
    /// Les trois natures.
    pub const TOUTES: [Self; 3] = [Self::Journee, Self::DemiJournee, Self::Retard];

    /// Nom persisté.
    pub const fn nom(&self) -> &'static str {
        match self {
            Self::Journee => "journee",
            Self::DemiJournee => "demi_journee",
            Self::Retard => "retard",
        }
    }

    /// Lecture d'un nom persisté.
    pub fn depuis_nom(nom: &str) -> Option<Self> {
        Self::TOUTES.into_iter().find(|n| n.nom() == nom)
    }
}

/// Retard le plus long qu'on note, en minutes : au-delà, c'est une absence.
pub const RETARD_PERSONNEL_MAX: i64 = 240;

/// Absence ou retard d'un membre du personnel, un jour.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
pub struct AbsencePersonnel {
    /// Identifiant technique.
    pub id: Id<AbsencePersonnel>,
    /// Établissement.
    pub etablissement_id: Id<Etablissement>,
    /// Membre absent ou en retard.
    pub membre_id: Id<MembrePersonnel>,
    /// Jour.
    pub date: DateCivile,
    /// Journée, demi-journée ou retard.
    pub nature: NatureAbsencePersonnel,
    /// Durée du retard en minutes, si elle est connue.
    pub minutes: Option<i64>,
    /// Motif donné.
    pub motif: String,
    /// Justifiée.
    pub justifiee: bool,
    /// Annulée : notée par erreur (R-14).
    pub annulee: bool,
    /// Motif de l'annulation.
    pub motif_annulation: String,
}

/// Contrôle d'une absence avant écriture : ni dans l'avenir, ni hors de la présence du membre,
/// ni pendant un congé accordé (le congé suffit), ni deux fois le même jour.
pub fn verifier_absence(
    a: &AbsencePersonnel,
    aujourd_hui: DateCivile,
    membre: &MembrePersonnel,
    conges: &[Conge],
    absences: &[AbsencePersonnel],
) -> Result<(), &'static str> {
    if a.date > aujourd_hui {
        return Err("absence_personnel.date_future");
    }
    if membre.arrive_le.is_some_and(|d| a.date < d) || membre.parti_le.is_some_and(|d| d < a.date) {
        return Err("absence_personnel.hors_presence");
    }
    match (a.nature, a.minutes) {
        (NatureAbsencePersonnel::Retard, Some(m)) if !(1..=RETARD_PERSONNEL_MAX).contains(&m) => {
            return Err("absence_personnel.retard_invalide");
        }
        (NatureAbsencePersonnel::Journee | NatureAbsencePersonnel::DemiJournee, Some(_)) => {
            return Err("absence_personnel.retard_invalide");
        }
        _ => {}
    }
    if conges
        .iter()
        .any(|c| c.membre_id == a.membre_id && c.statut == StatutConge::Accorde && c.couvre(a.date))
    {
        return Err("absence_personnel.en_conge");
    }
    if absences
        .iter()
        .any(|x| x.id != a.id && x.membre_id == a.membre_id && x.date == a.date && !x.annulee)
    {
        return Err("absence_personnel.deja_notee");
    }
    Ok(())
}

/// Type de congé, propre à l'école.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
pub struct TypeConge {
    /// Identifiant technique.
    pub id: Id<TypeConge>,
    /// Établissement.
    pub etablissement_id: Id<Etablissement>,
    /// Code stable d'un type livré ; absent pour un type ajouté par l'école.
    pub code: Option<String>,
    /// Libellé.
    pub libelle: String,
    /// Rang dans la liste.
    pub ordre: i64,
    /// Proposé aux demandes ; un type retiré reste sur les congés qui le portent.
    pub actif: bool,
    /// Ses jours se déduisent du solde, quand le décompte est actif : le congé annuel.
    pub deduit_du_solde: bool,
}

/// Types livrés : code, libellé, déduit du solde. L'école les renomme, en ajoute, en retire.
pub const TYPES_CONGE_LIVRES: [(&str, &str, bool); 5] = [
    ("annuel", "Congé annuel", true),
    ("permission", "Permission d'absence", false),
    ("maladie", "Congé de maladie", false),
    ("maternite", "Congé de maternité", false),
    ("autre", "Autre congé", false),
];

/// État d'un congé (§10.5 du blueprint).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "snake_case")]
pub enum StatutConge {
    /// Demandé : la direction décide.
    Demande,
    /// Accordé.
    Accorde,
    /// Refusé, avec un motif.
    Refuse,
    /// Annulé avant son début, avec un motif.
    Annule,
}

impl MachineAEtats for StatutConge {
    const ETATS: &'static [Self] = &[Self::Demande, Self::Accorde, Self::Refuse, Self::Annule];
    const TRANSITIONS: &'static [(Self, Self)] = &[
        (Self::Demande, Self::Accorde),
        (Self::Demande, Self::Refuse),
        (Self::Accorde, Self::Annule),
    ];

    fn nom(&self) -> &'static str {
        match self {
            Self::Demande => "demande",
            Self::Accorde => "accorde",
            Self::Refuse => "refuse",
            Self::Annule => "annule",
        }
    }
}

impl StatutConge {
    /// Lecture d'un nom persisté.
    pub fn depuis_nom(nom: &str) -> Option<Self> {
        Self::ETATS.iter().copied().find(|s| s.nom() == nom)
    }
}

/// Congé ou permission d'un membre du personnel.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
pub struct Conge {
    /// Identifiant technique.
    pub id: Id<Conge>,
    /// Établissement.
    pub etablissement_id: Id<Etablissement>,
    /// Membre.
    pub membre_id: Id<MembrePersonnel>,
    /// Type.
    pub type_id: Id<TypeConge>,
    /// Premier jour.
    pub debut: DateCivile,
    /// Dernier jour.
    pub fin: DateCivile,
    /// Jours comptés à la demande (Q-68).
    pub jours: i64,
    /// Motif de la demande.
    pub motif: String,
    /// État.
    pub statut: StatutConge,
    /// Instant de la décision.
    pub decide_le: Option<Horodatage>,
    /// Qui a décidé.
    pub decide_par: Option<Id<Utilisateur>>,
    /// Motif du refus.
    pub motif_refus: String,
    /// Motif de l'annulation.
    pub motif_annulation: String,
}

impl Conge {
    /// Vrai si le congé couvre ce jour.
    pub fn couvre(&self, jour: DateCivile) -> bool {
        self.debut <= jour && jour <= self.fin
    }

    /// Vrai tant qu'il compte : demandé ou accordé.
    pub const fn vivant(&self) -> bool {
        matches!(self.statut, StatutConge::Demande | StatutConge::Accorde)
    }
}

/// Un congé ne dépasse pas un an.
pub const DUREE_MAX_CONGE_JOURS: i64 = 366;

/// Jours d'un congé : du lundi au vendredi, le samedi aussi si l'école le compte ; pas de jours
/// fériés en phase 6 (Q-68).
pub fn jours_de_conge(debut: DateCivile, fin: DateCivile, samedi_compte: bool) -> i64 {
    let n = (0..=debut.jours_jusqua(fin))
        .filter_map(|i| debut.decaler(i))
        .filter(|d| match d.jour_de_semaine() {
            0..=4 => true,
            5 => samedi_compte,
            _ => false,
        })
        .count();
    i64::try_from(n).unwrap_or(i64::MAX)
}

/// Contrôle d'un congé demandé : des dates dans l'ordre, au moins un jour compté, un type
/// proposé, un membre présent, aucun autre congé demandé ou accordé sur les mêmes jours.
pub fn verifier_conge(
    c: &Conge,
    membre: &MembrePersonnel,
    type_conge: &TypeConge,
    autres: &[Conge],
) -> Result<(), &'static str> {
    if c.fin < c.debut {
        return Err("conge.fin_avant_debut");
    }
    if c.debut.jours_jusqua(c.fin) >= DUREE_MAX_CONGE_JOURS {
        return Err("conge.trop_long");
    }
    if c.jours <= 0 {
        return Err("conge.aucun_jour");
    }
    if !type_conge.actif {
        return Err("conge.type_retire");
    }
    if membre.parti_le.is_some_and(|p| p < c.debut) {
        return Err("conge.membre_parti");
    }
    if autres.iter().any(|x| {
        x.id != c.id
            && x.membre_id == c.membre_id
            && x.vivant()
            && x.debut <= c.fin
            && c.debut <= x.fin
    }) {
        return Err("conge.chevauchement");
    }
    Ok(())
}

/// Jours acquis par mois quand le décompte est actif (T-44) : 2,5 par défaut.
pub const JOURS_PAR_MOIS_DEFAUT: &str = "2,5";

/// Lit des jours par mois au demi-jour près (« 2 », « 2,5 », « 2.5 »), de 0 à 10 ; rend des
/// demi-journées.
pub fn lire_jours_par_mois(texte: &str) -> Option<i64> {
    let t = texte.trim().replace('.', ",");
    let (entier, demi) = match t.split_once(',') {
        Some((e, "5")) => (e, 1),
        Some((e, "0")) => (e, 0),
        Some(_) => return None,
        None => (t.as_str(), 0),
    };
    if entier.is_empty() || entier.len() > 2 || !entier.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    let demi_journees = entier.parse::<i64>().ok()? * 2 + demi;
    (0..=20).contains(&demi_journees).then_some(demi_journees)
}

/// Demi-journées écrites en jours : « 12 », « 12,5 », « -3,5 ».
pub fn ecrire_demi_journees(demi_journees: i64) -> String {
    let signe = if demi_journees < 0 { "-" } else { "" };
    let d = demi_journees.abs();
    if d % 2 == 0 {
        format!("{signe}{}", d / 2)
    } else {
        format!("{signe}{},5", d / 2)
    }
}

/// Mois entiers de présence avant un jour : du premier mois commencé après l'arrivée (celui de
/// l'arrivée si elle tombe le 1er) au mois qui précède ce jour.
fn mois_entiers(arrive_le: DateCivile, jour: DateCivile) -> i64 {
    let rang = |d: DateCivile| i64::from(d.annee()) * 12 + i64::from(d.mois()) - 1;
    let premier = rang(arrive_le) + i64::from(arrive_le.jour() != 1);
    (rang(jour) - premier).max(0)
}

/// Solde de congé d'un membre au premier jour d'un congé, en demi-journées : les jours acquis
/// depuis son arrivée, moins ceux des congés accordés qui se déduisent du solde, hors ce congé.
/// Aucun sans date d'arrivée.
pub fn solde_au(
    membre: &MembrePersonnel,
    conges: &[Conge],
    types: &[TypeConge],
    demi_journees_par_mois: i64,
    jour: DateCivile,
    hors: Option<Id<Conge>>,
) -> Option<i64> {
    let acquis = mois_entiers(membre.arrive_le?, jour) * demi_journees_par_mois;
    let pris: i64 = conges
        .iter()
        .filter(|c| {
            c.membre_id == membre.id
                && c.statut == StatutConge::Accorde
                && Some(c.id) != hors
                && types.iter().any(|t| t.id == c.type_id && t.deduit_du_solde)
        })
        .map(|c| c.jours * 2)
        .sum();
    Some(acquis - pris)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rh::MoyenPaiement;

    fn jour(a: i32, m: u8, j: u8) -> DateCivile {
        DateCivile::nouvelle(a, m, j).unwrap_or_else(|| unreachable!())
    }

    fn membre(arrive_le: Option<DateCivile>) -> MembrePersonnel {
        MembrePersonnel {
            id: Id::depuis_octets([3; 16]),
            etablissement_id: Id::depuis_octets([2; 16]),
            personne_id: Id::depuis_octets([4; 16]),
            matricule: "P-0001".to_owned(),
            categorie_id: Id::depuis_octets([5; 16]),
            fonction: "Secrétaire".to_owned(),
            arrive_le,
            parti_le: None,
            motif_depart: String::new(),
            fonctionnaire: false,
            numero_fonctionnaire: String::new(),
            numero_inps: String::new(),
            prevenir_nom: String::new(),
            prevenir_telephone: String::new(),
            paiement: MoyenPaiement::Especes,
            diplomes: Vec::new(),
            utilisateur_id: None,
            situation_famille: None,
            enfants_a_charge: None,
        }
    }

    fn type_conge(deduit: bool) -> TypeConge {
        TypeConge {
            id: Id::depuis_octets(if deduit { [6; 16] } else { [7; 16] }),
            etablissement_id: Id::depuis_octets([2; 16]),
            code: None,
            libelle: "Congé annuel".to_owned(),
            ordre: 1,
            actif: true,
            deduit_du_solde: deduit,
        }
    }

    fn conge(octet: u8, debut: DateCivile, fin: DateCivile, statut: StatutConge) -> Conge {
        Conge {
            id: Id::depuis_octets([octet; 16]),
            etablissement_id: Id::depuis_octets([2; 16]),
            membre_id: Id::depuis_octets([3; 16]),
            type_id: Id::depuis_octets([6; 16]),
            debut,
            fin,
            jours: jours_de_conge(debut, fin, false),
            motif: String::new(),
            statut,
            decide_le: None,
            decide_par: None,
            motif_refus: String::new(),
            motif_annulation: String::new(),
        }
    }

    fn absence(date: DateCivile, nature: NatureAbsencePersonnel) -> AbsencePersonnel {
        AbsencePersonnel {
            id: Id::depuis_octets([8; 16]),
            etablissement_id: Id::depuis_octets([2; 16]),
            membre_id: Id::depuis_octets([3; 16]),
            date,
            nature,
            minutes: None,
            motif: String::new(),
            justifiee: false,
            annulee: false,
            motif_annulation: String::new(),
        }
    }

    #[test]
    fn la_table_des_etats_est_exhaustive() {
        use StatutConge::*;
        for depuis in StatutConge::ETATS {
            for vers in StatutConge::ETATS {
                let permise = matches!(
                    (depuis, vers),
                    (Demande, Accorde) | (Demande, Refuse) | (Accorde, Annule)
                );
                assert_eq!(
                    depuis.transiter(*vers).is_ok(),
                    permise,
                    "{depuis:?} → {vers:?}"
                );
            }
        }
    }

    #[test]
    fn les_jours_se_comptent_du_lundi_au_vendredi() {
        // Du lundi 19 au dimanche 25 octobre 2026.
        assert_eq!(
            jours_de_conge(jour(2026, 10, 19), jour(2026, 10, 25), false),
            5
        );
        assert_eq!(
            jours_de_conge(jour(2026, 10, 19), jour(2026, 10, 25), true),
            6
        );
        // Un samedi seul : rien, sauf si l'école compte le samedi.
        assert_eq!(
            jours_de_conge(jour(2026, 10, 24), jour(2026, 10, 24), false),
            0
        );
        assert_eq!(
            jours_de_conge(jour(2026, 10, 24), jour(2026, 10, 24), true),
            1
        );
        // Deux semaines entières.
        assert_eq!(
            jours_de_conge(jour(2026, 10, 19), jour(2026, 11, 1), false),
            10
        );
    }

    #[test]
    fn un_conge_ne_chevauche_pas_un_autre() {
        let m = membre(None);
        let t = type_conge(true);
        let accorde = conge(
            1,
            jour(2026, 10, 19),
            jour(2026, 10, 23),
            StatutConge::Accorde,
        );
        let nouveau = conge(
            2,
            jour(2026, 10, 22),
            jour(2026, 10, 27),
            StatutConge::Demande,
        );
        assert_eq!(
            verifier_conge(&nouveau, &m, &t, std::slice::from_ref(&accorde)),
            Err("conge.chevauchement")
        );
        // Un congé refusé ne compte plus.
        let refuse = Conge {
            statut: StatutConge::Refuse,
            ..accorde.clone()
        };
        assert_eq!(verifier_conge(&nouveau, &m, &t, &[refuse]), Ok(()));
        let samedi = conge(
            3,
            jour(2026, 10, 24),
            jour(2026, 10, 25),
            StatutConge::Demande,
        );
        assert_eq!(
            verifier_conge(&samedi, &m, &t, &[]),
            Err("conge.aucun_jour")
        );
        let mut retire = t.clone();
        retire.actif = false;
        assert_eq!(
            verifier_conge(&nouveau, &m, &retire, &[]),
            Err("conge.type_retire")
        );
    }

    #[test]
    fn une_absence_pendant_un_conge_accorde_est_refusee() {
        let m = membre(Some(jour(2025, 10, 1)));
        let aujourd_hui = jour(2026, 10, 30);
        let accorde = conge(
            1,
            jour(2026, 10, 19),
            jour(2026, 10, 23),
            StatutConge::Accorde,
        );
        let a = absence(jour(2026, 10, 21), NatureAbsencePersonnel::Journee);
        assert_eq!(
            verifier_absence(&a, aujourd_hui, &m, std::slice::from_ref(&accorde), &[]),
            Err("absence_personnel.en_conge")
        );
        // Un congé seulement demandé ne couvre rien.
        let demande = Conge {
            statut: StatutConge::Demande,
            ..accorde
        };
        assert_eq!(
            verifier_absence(&a, aujourd_hui, &m, &[demande], &[]),
            Ok(())
        );
        let demain = absence(jour(2026, 10, 31), NatureAbsencePersonnel::Journee);
        assert_eq!(
            verifier_absence(&demain, aujourd_hui, &m, &[], &[]),
            Err("absence_personnel.date_future")
        );
        let avant = absence(jour(2025, 9, 30), NatureAbsencePersonnel::Journee);
        assert_eq!(
            verifier_absence(&avant, aujourd_hui, &m, &[], &[]),
            Err("absence_personnel.hors_presence")
        );
        let mut retard = absence(jour(2026, 10, 21), NatureAbsencePersonnel::Retard);
        retard.minutes = Some(300);
        assert_eq!(
            verifier_absence(&retard, aujourd_hui, &m, &[], &[]),
            Err("absence_personnel.retard_invalide")
        );
        retard.minutes = Some(20);
        let mut deja = a.clone();
        deja.id = Id::depuis_octets([9; 16]);
        assert_eq!(
            verifier_absence(&retard, aujourd_hui, &m, &[], std::slice::from_ref(&deja)),
            Err("absence_personnel.deja_notee")
        );
        deja.annulee = true;
        assert_eq!(
            verifier_absence(&retard, aujourd_hui, &m, &[], &[deja]),
            Ok(())
        );
    }

    #[test]
    fn les_jours_par_mois_se_lisent_au_demi_jour() {
        assert_eq!(lire_jours_par_mois("2,5"), Some(5));
        assert_eq!(lire_jours_par_mois("2.5"), Some(5));
        assert_eq!(lire_jours_par_mois(" 3 "), Some(6));
        assert_eq!(lire_jours_par_mois("0"), Some(0));
        assert_eq!(lire_jours_par_mois("2,25"), None);
        assert_eq!(lire_jours_par_mois("11"), None);
        assert_eq!(lire_jours_par_mois("deux"), None);
        assert_eq!(ecrire_demi_journees(25), "12,5");
        assert_eq!(ecrire_demi_journees(-7), "-3,5");
        assert_eq!(ecrire_demi_journees(4), "2");
    }

    #[test]
    fn le_solde_compte_les_mois_entiers_depuis_l_arrivee() {
        let types = [type_conge(true), type_conge(false)];
        // Arrivé le 15 janvier : février est le premier mois entier.
        let m = membre(Some(jour(2026, 1, 15)));
        let au = jour(2026, 8, 3);
        // De février à juillet : six mois, 15 jours.
        assert_eq!(solde_au(&m, &[], &types, 5, au, None), Some(30));
        let accorde = conge(1, jour(2026, 4, 6), jour(2026, 4, 10), StatutConge::Accorde);
        let mut maladie = conge(2, jour(2026, 5, 4), jour(2026, 5, 8), StatutConge::Accorde);
        maladie.type_id = types[1].id;
        let conges = [accorde.clone(), maladie];
        // Cinq jours de congé annuel déduits, pas la maladie.
        assert_eq!(solde_au(&m, &conges, &types, 5, au, None), Some(20));
        assert_eq!(
            solde_au(&m, &conges, &types, 5, au, Some(accorde.id)),
            Some(30)
        );
        assert_eq!(solde_au(&membre(None), &conges, &types, 5, au, None), None);
        // Arrivé un 1er : son mois compte.
        let m = membre(Some(jour(2026, 7, 1)));
        assert_eq!(solde_au(&m, &[], &types, 5, au, None), Some(5));
    }
}
