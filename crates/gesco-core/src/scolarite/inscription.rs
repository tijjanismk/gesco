//! Inscription (SPEC_PHASE_2 §5 ; blueprint §8.3, §10.1) : la table pivot. Notes, absences,
//! factures et versements référencent l'inscription, jamais l'élève.

use serde::{Deserialize, Serialize};

use super::academique::{Classe, Cycle, DiplomeTerminal, Niveau};
use super::personne::{Eleve, Personne, Qualites};
use crate::commun::{DateCivile, Id};
use crate::core::transition::MachineAEtats;
use crate::core::{AnneeScolaire, Etablissement};

/// Cycle de vie d'une inscription (§10.1).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "snake_case")]
pub enum StatutInscription {
    /// Saisie en cours, sans effet.
    Brouillon,
    /// L'élève fait partie de la classe ; la facture la rejoindra en phase 3 (§0.3).
    Validee,
    /// Parti dans une autre école (phase 4).
    Transferee,
    /// Abandon, avec son motif.
    Abandonnee,
    /// Exclu définitivement (radiation disciplinaire) ; la direction peut revenir sur la
    /// décision et réintégrer l'élève (réponse du 13 octobre 2026).
    Radiee,
    /// Année terminée, à la clôture (phase 10).
    Achevee,
}

impl MachineAEtats for StatutInscription {
    const ETATS: &'static [Self] = &[
        Self::Brouillon,
        Self::Validee,
        Self::Transferee,
        Self::Abandonnee,
        Self::Radiee,
        Self::Achevee,
    ];
    const TRANSITIONS: &'static [(Self, Self)] = &[
        (Self::Brouillon, Self::Validee),
        // Q-47 : un brouillon s'abandonne avec un motif, il ne se supprime pas (R-14).
        (Self::Brouillon, Self::Abandonnee),
        (Self::Validee, Self::Abandonnee),
        (Self::Validee, Self::Transferee),
        (Self::Validee, Self::Radiee),
        // Exclusion révoquée par décision motivée : l'élève revient dans sa classe.
        (Self::Radiee, Self::Validee),
        (Self::Validee, Self::Achevee),
        // Réouverture d'une année clôturée (SPEC_PHASE_10 §4), décision gardée.
        (Self::Achevee, Self::Validee),
    ];

    fn nom(&self) -> &'static str {
        match self {
            Self::Brouillon => "brouillon",
            Self::Validee => "validee",
            Self::Transferee => "transferee",
            Self::Abandonnee => "abandonnee",
            Self::Radiee => "radiee",
            Self::Achevee => "achevee",
        }
    }
}

impl StatutInscription {
    /// Vrai pour une inscription qui occupe l'élève dans l'année : un seul brouillon ou une seule
    /// inscription validée à la fois.
    pub const fn est_vivante(&self) -> bool {
        matches!(self, Self::Brouillon | Self::Validee)
    }
}

/// Nature d'une inscription, déduite et non saisie (SPEC_PHASE_2 §5.1).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "snake_case")]
pub enum TypeInscription {
    /// Premier passage dans l'école.
    Nouvelle,
    /// L'élève a une inscription validée d'une année antérieure.
    Reinscription,
    /// L'élève vient d'une autre école, qui est indiquée.
    TransfertEntrant,
}

impl TypeInscription {
    /// Nom persisté.
    pub const fn nom(&self) -> &'static str {
        match self {
            Self::Nouvelle => "nouvelle",
            Self::Reinscription => "reinscription",
            Self::TransfertEntrant => "transfert_entrant",
        }
    }

    /// Lecture d'un nom persisté.
    pub fn depuis_nom(nom: &str) -> Option<Self> {
        [Self::Nouvelle, Self::Reinscription, Self::TransfertEntrant]
            .into_iter()
            .find(|t| t.nom() == nom)
    }

    /// Déduit la nature : réinscription d'abord, puis transfert si une école d'origine est
    /// indiquée, nouvelle sinon.
    pub fn deduire(deja_valide_avant: bool, etablissement_origine: &str) -> Self {
        if deja_valide_avant {
            Self::Reinscription
        } else if etablissement_origine.trim().is_empty() {
            Self::Nouvelle
        } else {
            Self::TransfertEntrant
        }
    }
}

/// Décision de fin d'année, posée à la clôture (phase 10).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "snake_case")]
pub enum DecisionFinAnnee {
    /// Pas encore décidée (Q-30).
    NonStatue,
    /// Passe au niveau supérieur.
    Admis,
    /// Redouble.
    Redouble,
    /// Exclu.
    Exclu,
    /// Passe un rattrapage.
    Rattrapage,
}

impl DecisionFinAnnee {
    /// Nom persisté.
    pub const fn nom(&self) -> &'static str {
        match self {
            Self::NonStatue => "non_statue",
            Self::Admis => "admis",
            Self::Redouble => "redouble",
            Self::Exclu => "exclu",
            Self::Rattrapage => "rattrapage",
        }
    }

    /// Lecture d'un nom persisté.
    pub fn depuis_nom(nom: &str) -> Option<Self> {
        [
            Self::NonStatue,
            Self::Admis,
            Self::Redouble,
            Self::Exclu,
            Self::Rattrapage,
        ]
        .into_iter()
        .find(|d| d.nom() == nom)
    }

    /// Vrai pour une décision qui permet de clôturer l'année : ni « non statué », ni un
    /// rattrapage dont le résultat n'est pas encore posé.
    pub const fn est_definitive(&self) -> bool {
        !matches!(self, Self::NonStatue | Self::Rattrapage)
    }
}

/// Inscription d'un élève dans une classe, pour une année.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
pub struct Inscription {
    /// Identifiant technique.
    pub id: Id<Inscription>,
    /// Établissement propriétaire.
    pub etablissement_id: Id<Etablissement>,
    /// Année scolaire.
    pub annee_id: Id<AnneeScolaire>,
    /// Élève.
    pub eleve_id: Id<Eleve>,
    /// Classe.
    pub classe_id: Id<Classe>,
    /// Jour de l'inscription.
    pub date_inscription: DateCivile,
    /// Nature, déduite.
    pub type_inscription: TypeInscription,
    /// État.
    pub statut: StatutInscription,
    /// Redouble son niveau.
    pub redoublant: bool,
    /// Décision de fin d'année.
    pub decision_fin_annee: DecisionFinAnnee,
    /// Classe quittée lors du dernier changement de classe.
    pub classe_precedente_id: Option<Id<Classe>>,
    /// École d'où vient l'élève, pour un transfert entrant.
    pub etablissement_origine: String,
    /// Motif d'un abandon.
    pub motif_abandon: String,
    /// Départ, rempli par le transfert sortant (phase 4).
    pub date_depart: Option<DateCivile>,
    /// École de destination (phase 4).
    pub etablissement_destination: String,
    /// Motif du départ (phase 4).
    pub motif_depart: String,
}

/// Ce que la validation examine (SPEC_PHASE_2 §5.3).
#[derive(Debug, Clone, Copy)]
pub struct Dossier<'a> {
    /// Identité de l'élève.
    pub identite: &'a Personne,
    /// Élève.
    pub eleve: &'a Eleve,
    /// Une photo existe.
    pub a_photo: bool,
    /// Niveau de la classe.
    pub niveau: &'a Niveau,
    /// La série de la classe, si elle en a une, est active.
    pub serie_active: bool,
    /// Qualités des liens et présence d'un téléphone sur chaque responsable.
    pub responsables: &'a [(Qualites, bool)],
    /// Jours écoulés entre le début de l'année et l'inscription.
    pub jours_depuis_rentree: i64,
    /// Seuil d'alerte d'une inscription tardive (Q-31) ; 0 : jamais.
    pub alerte_apres_jours: i64,
}

/// Vérifie les conditions de validation, dans l'ordre du §5.3 ; rend les avertissements, qui
/// n'empêchent rien. L'erreur est la clé du message.
pub fn verifier_validation(dossier: &Dossier<'_>) -> Result<Vec<&'static str>, &'static str> {
    let identite = dossier.identite;
    if identite.nom.is_empty()
        || identite.prenoms.is_empty()
        || identite.sexe.is_none()
        || identite.date_naissance.is_none()
        || identite.lieu_naissance.is_empty()
    {
        return Err("inscription.identite_incomplete");
    }
    if !dossier
        .responsables
        .iter()
        .any(|(q, _)| q.est_responsable_legal)
    {
        return Err("inscription.responsable_legal_absent");
    }
    if !dossier.responsables.iter().any(|(_, telephone)| *telephone) {
        return Err("inscription.telephone_absent");
    }
    let niveau = dossier.niveau;
    // Q-18 révisée : le matricule national est exigé dès la 9e (DEF) et au lycée.
    let examen = niveau.cycle == Cycle::Lycee || niveau.diplome_terminal == DiplomeTerminal::Def;
    if examen && dossier.eleve.matricule_national.is_none() {
        return Err("inscription.matricule_national_absent");
    }
    let mut avertissements = Vec::new();
    if niveau.cycle == Cycle::Jardin {
        if !dossier.a_photo {
            return Err("inscription.photo_absente");
        }
        let autorisees = dossier
            .responsables
            .iter()
            .filter(|(q, _)| q.vient_chercher)
            .count();
        if autorisees == 0 {
            return Err("inscription.personne_autorisee_absente");
        }
        // Q-46 : deux à trois personnes attendues ; une seule avertit sans bloquer.
        if autorisees < 2 {
            avertissements.push("inscription.une_seule_personne_autorisee");
        }
    }
    if !niveau.actif || !dossier.serie_active {
        return Err("inscription.classe_inactive");
    }
    if dossier.alerte_apres_jours > 0 && dossier.jours_depuis_rentree > dossier.alerte_apres_jours {
        avertissements.push("inscription.tardive");
    }
    Ok(avertissements)
}

/// Vérifie un changement de classe en cours d'année (Q-35) : même niveau et même série, sauf
/// décision de la direction, motivée.
pub fn verifier_changement(
    ancienne: &Classe,
    nouvelle: &Classe,
    decision_direction: bool,
    motif: &str,
) -> Result<(), &'static str> {
    if ancienne.id == nouvelle.id {
        return Err("inscription.meme_classe");
    }
    if ancienne.annee_id != nouvelle.annee_id {
        return Err("inscription.autre_annee");
    }
    let meme_filiere =
        ancienne.niveau_id == nouvelle.niveau_id && ancienne.serie_id == nouvelle.serie_id;
    if meme_filiere {
        return Ok(());
    }
    if !decision_direction {
        return Err("inscription.changement_hors_filiere");
    }
    if motif.trim().is_empty() {
        return Err("inscription.motif_obligatoire");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::transition::verifier_exhaustivement;
    use crate::scolarite::academique::ModeEvaluation;
    use crate::scolarite::personne::{Lien, Sexe};

    fn id<T>(n: u128) -> Id<T> {
        Id::depuis_ulid(ulid::Ulid::from(n))
    }

    #[test]
    fn table_des_transitions_exhaustive() {
        verifier_exhaustivement::<StatutInscription>();
        assert!(StatutInscription::Brouillon.est_vivante());
        assert!(!StatutInscription::Abandonnee.est_vivante());
        assert!(
            StatutInscription::Abandonnee
                .transiter(StatutInscription::Validee)
                .is_err()
        );
    }

    #[test]
    fn nature_deduite() {
        assert_eq!(
            TypeInscription::deduire(true, "Lycée de Kati"),
            TypeInscription::Reinscription
        );
        assert_eq!(
            TypeInscription::deduire(false, " "),
            TypeInscription::Nouvelle
        );
        assert_eq!(
            TypeInscription::deduire(false, "Lycée de Kati"),
            TypeInscription::TransfertEntrant
        );
        for d in [
            DecisionFinAnnee::NonStatue,
            DecisionFinAnnee::Admis,
            DecisionFinAnnee::Redouble,
            DecisionFinAnnee::Exclu,
            DecisionFinAnnee::Rattrapage,
        ] {
            assert_eq!(DecisionFinAnnee::depuis_nom(d.nom()), Some(d));
        }
    }

    fn personne() -> Personne {
        Personne {
            id: id(1),
            etablissement_id: id(0),
            nom: "Traoré".to_owned(),
            prenoms: "Awa".to_owned(),
            sexe: Some(Sexe::Feminin),
            date_naissance: DateCivile::analyser("2014-03-02"),
            date_naissance_estimee: false,
            lieu_naissance: "Bamako".to_owned(),
            nationalite: String::new(),
            adresse: String::new(),
            telephones: vec![],
        }
    }

    fn eleve(national: Option<&str>) -> Eleve {
        Eleve {
            id: id(2),
            etablissement_id: id(0),
            personne_id: id(1),
            matricule_interne: "2026-00001".to_owned(),
            matricule_national: national.map(str::to_owned),
            acte_nature: None,
            acte_numero: String::new(),
            acte_date: None,
            ecole_origine: String::new(),
            fusionne_dans: None,
        }
    }

    fn niveau(cycle: Cycle, diplome: DiplomeTerminal) -> Niveau {
        Niveau {
            id: id(3),
            etablissement_id: id(0),
            code: "x".to_owned(),
            libelle: "x".to_owned(),
            cycle,
            ordre: 1,
            mode_evaluation: ModeEvaluation::Notes,
            diplome_terminal: diplome,
            actif: true,
        }
    }

    fn qualites(legal: bool, vient: bool) -> Qualites {
        Qualites {
            lien: Lien::Mere,
            est_responsable_legal: legal,
            est_payeur: true,
            recoit_notifications: true,
            vient_chercher: vient,
            ordre_priorite: 1,
        }
    }

    fn dossier<'a>(
        identite: &'a Personne,
        eleve: &'a Eleve,
        niveau: &'a Niveau,
        a_photo: bool,
        responsables: &'a [(Qualites, bool)],
    ) -> Dossier<'a> {
        Dossier {
            identite,
            eleve,
            a_photo,
            niveau,
            serie_active: true,
            responsables,
            jours_depuis_rentree: 3,
            alerte_apres_jours: 0,
        }
    }

    #[test]
    fn chaque_condition_refuse_ce_qu_elle_vise() {
        let p = personne();
        let fondamental = niveau(Cycle::PremierCycle, DiplomeTerminal::Aucun);
        let neuvieme = niveau(Cycle::SecondCycle, DiplomeTerminal::Def);
        let lycee = niveau(Cycle::Lycee, DiplomeTerminal::Aucun);
        let jardin = niveau(Cycle::Jardin, DiplomeTerminal::Aucun);
        let sans_national = eleve(None);
        let avec_national = eleve(Some("ML123"));
        let mere = [(qualites(true, true), true)];
        let verifier = |d: Dossier<'_>| verifier_validation(&d);
        assert_eq!(
            verifier(dossier(&p, &sans_national, &fondamental, false, &mere)),
            Ok(vec![])
        );

        let mut sans_lieu = personne();
        sans_lieu.lieu_naissance.clear();
        assert_eq!(
            verifier(dossier(
                &sans_lieu,
                &sans_national,
                &fondamental,
                false,
                &mere
            )),
            Err("inscription.identite_incomplete")
        );
        assert_eq!(
            verifier(dossier(
                &p,
                &sans_national,
                &fondamental,
                false,
                &[(qualites(false, false), true)]
            )),
            Err("inscription.responsable_legal_absent")
        );
        assert_eq!(
            verifier(dossier(
                &p,
                &sans_national,
                &fondamental,
                false,
                &[(qualites(true, false), false)]
            )),
            Err("inscription.telephone_absent")
        );
        for n in [&neuvieme, &lycee] {
            assert_eq!(
                verifier(dossier(&p, &sans_national, n, false, &mere)),
                Err("inscription.matricule_national_absent")
            );
            assert_eq!(
                verifier(dossier(&p, &avec_national, n, false, &mere)),
                Ok(vec![])
            );
        }
        assert_eq!(
            verifier(dossier(&p, &sans_national, &jardin, false, &mere)),
            Err("inscription.photo_absente")
        );
        assert_eq!(
            verifier(dossier(
                &p,
                &sans_national,
                &jardin,
                true,
                &[(qualites(true, false), true)]
            )),
            Err("inscription.personne_autorisee_absente")
        );
        assert_eq!(
            verifier(dossier(&p, &sans_national, &jardin, true, &mere)),
            Ok(vec!["inscription.une_seule_personne_autorisee"])
        );
        let deux = [(qualites(true, true), true), (qualites(false, true), true)];
        assert_eq!(
            verifier(dossier(&p, &sans_national, &jardin, true, &deux)),
            Ok(vec![])
        );
        let mut inactif = fondamental.clone();
        inactif.actif = false;
        assert_eq!(
            verifier(dossier(&p, &sans_national, &inactif, false, &mere)),
            Err("inscription.classe_inactive")
        );
        let tardive = Dossier {
            jours_depuis_rentree: 61,
            alerte_apres_jours: 60,
            ..dossier(&p, &sans_national, &fondamental, false, &mere)
        };
        assert_eq!(verifier(tardive), Ok(vec!["inscription.tardive"]));
    }

    fn classe(n: u128, niveau: u128, serie: Option<u128>) -> Classe {
        Classe {
            id: id(n),
            etablissement_id: id(0),
            annee_id: id(9),
            niveau_id: id(niveau),
            serie_id: serie.map(id),
            libelle: String::new(),
        }
    }

    #[test]
    fn changement_de_classe() {
        let septa = classe(1, 7, None);
        assert_eq!(
            verifier_changement(&septa, &classe(2, 7, None), false, ""),
            Ok(())
        );
        assert_eq!(
            verifier_changement(&septa, &septa, false, ""),
            Err("inscription.meme_classe")
        );
        let onze_s = classe(3, 11, Some(1));
        let onze_ll = classe(4, 11, Some(2));
        assert_eq!(
            verifier_changement(&onze_s, &onze_ll, false, "x"),
            Err("inscription.changement_hors_filiere")
        );
        assert_eq!(
            verifier_changement(&onze_s, &onze_ll, true, " "),
            Err("inscription.motif_obligatoire")
        );
        assert_eq!(
            verifier_changement(&onze_s, &onze_ll, true, "Orientation décidée en conseil"),
            Ok(())
        );
    }
}
