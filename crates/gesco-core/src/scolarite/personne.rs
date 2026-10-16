//! Personnes, élèves, responsables et leurs liens (SPEC_PHASE_2 §4 ; blueprint §3.5, §3.6, §8.3).
//! Aucune unicité sur nom, prénoms et date de naissance : l'homonymie est la règle (§3.5).

use serde::{Deserialize, Serialize};

use crate::commun::{DateCivile, Horodatage, Id};
use crate::core::Etablissement;

/// Sexe d'une personne.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "snake_case")]
pub enum Sexe {
    /// Garçon, homme.
    Masculin,
    /// Fille, femme.
    Feminin,
}

impl Sexe {
    /// Nom persisté.
    pub const fn nom(&self) -> &'static str {
        match self {
            Self::Masculin => "masculin",
            Self::Feminin => "feminin",
        }
    }

    /// Lecture d'un nom persisté.
    pub fn depuis_nom(nom: &str) -> Option<Self> {
        [Self::Masculin, Self::Feminin]
            .into_iter()
            .find(|s| s.nom() == nom)
    }
}

/// Socle commun à l'élève et au responsable.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
pub struct Personne {
    /// Identifiant technique.
    pub id: Id<Personne>,
    /// Établissement propriétaire.
    pub etablissement_id: Id<Etablissement>,
    /// Nom de famille.
    pub nom: String,
    /// Prénoms.
    pub prenoms: String,
    /// Sexe ; exigé pour un élève.
    pub sexe: Option<Sexe>,
    /// Date de naissance ; exigée pour un élève (Q-19).
    pub date_naissance: Option<DateCivile>,
    /// Vrai pour une date approximative ou issue d'un jugement supplétif (§24.4).
    pub date_naissance_estimee: bool,
    /// Lieu de naissance.
    pub lieu_naissance: String,
    /// Nationalité.
    pub nationalite: String,
    /// Adresse.
    pub adresse: String,
    /// Téléphones, sous forme canonique E.164.
    pub telephones: Vec<String>,
}

/// Nature de la pièce d'état civil d'un élève.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "snake_case")]
pub enum NatureActe {
    /// Acte de naissance.
    Acte,
    /// Jugement supplétif.
    JugementSuppletif,
}

impl NatureActe {
    /// Nom persisté.
    pub const fn nom(&self) -> &'static str {
        match self {
            Self::Acte => "acte",
            Self::JugementSuppletif => "jugement_suppletif",
        }
    }

    /// Lecture d'un nom persisté.
    pub fn depuis_nom(nom: &str) -> Option<Self> {
        [Self::Acte, Self::JugementSuppletif]
            .into_iter()
            .find(|n| n.nom() == nom)
    }
}

/// Élève de l'établissement.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
pub struct Eleve {
    /// Identifiant technique.
    pub id: Id<Eleve>,
    /// Établissement propriétaire.
    pub etablissement_id: Id<Etablissement>,
    /// Identité.
    pub personne_id: Id<Personne>,
    /// Matricule interne `AAAA-NNNNN`, attribué sans trou, qui fait autorité (T-01).
    pub matricule_interne: String,
    /// Matricule national, exigé au lycée (Q-18).
    pub matricule_national: Option<String>,
    /// Pièce d'état civil.
    pub acte_nature: Option<NatureActe>,
    /// Numéro de l'acte ou du jugement.
    pub acte_numero: String,
    /// Date de l'acte ou du jugement.
    pub acte_date: Option<DateCivile>,
    /// École fréquentée avant l'arrivée.
    pub ecole_origine: String,
    /// Fiche conservée qui a absorbé celle-ci par une fusion (SPEC_PHASE_2 §8).
    pub fusionne_dans: Option<Id<Eleve>>,
}

/// Responsable d'un ou plusieurs élèves, réutilisé d'une fratrie à l'autre (§25.5).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
pub struct Responsable {
    /// Identifiant technique.
    pub id: Id<Responsable>,
    /// Établissement propriétaire.
    pub etablissement_id: Id<Etablissement>,
    /// Identité.
    pub personne_id: Id<Personne>,
    /// Profession.
    pub profession: String,
    /// Employeur.
    pub employeur: String,
}

/// Lien de parenté ou de garde.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "snake_case")]
pub enum Lien {
    /// Père.
    Pere,
    /// Mère.
    Mere,
    /// Tuteur.
    Tuteur,
    /// Autre : oncle, grand frère, employeur…
    Autre,
}

impl Lien {
    /// Sexe qu'impose le lien : un père est un homme, une mère une femme. Le sexe ne se
    /// redemande pas (remarque du porteur du projet, 9 octobre 2026).
    pub const fn sexe(&self) -> Option<Sexe> {
        match self {
            Self::Pere => Some(Sexe::Masculin),
            Self::Mere => Some(Sexe::Feminin),
            Self::Tuteur | Self::Autre => None,
        }
    }

    /// Nom persisté.
    pub const fn nom(&self) -> &'static str {
        match self {
            Self::Pere => "pere",
            Self::Mere => "mere",
            Self::Tuteur => "tuteur",
            Self::Autre => "autre",
        }
    }

    /// Lecture d'un nom persisté.
    pub fn depuis_nom(nom: &str) -> Option<Self> {
        [Self::Pere, Self::Mere, Self::Tuteur, Self::Autre]
            .into_iter()
            .find(|l| l.nom() == nom)
    }
}

/// Qualités d'un responsable pour un élève donné : indépendantes et propres au lien (§3.6, T-03).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
pub struct Qualites {
    /// Lien de parenté ou de garde.
    pub lien: Lien,
    /// Détient l'autorité légale.
    pub est_responsable_legal: bool,
    /// Paie la scolarité.
    pub est_payeur: bool,
    /// Reçoit les messages de l'école.
    pub recoit_notifications: bool,
    /// Vient chercher l'enfant à la sortie (jardin, T-02).
    pub vient_chercher: bool,
    /// Rang d'appel : 1 pour le premier contact.
    pub ordre_priorite: i64,
}

/// Longueur maximale d'un nom, de prénoms ou d'un lieu.
pub const TEXTE_MAX: usize = 80;

/// Texte saisi, espaces superflus retirés, borné ; vide autorisé.
pub fn texte_propre(texte: &str, cle_erreur: &'static str) -> Result<String, &'static str> {
    let propre = texte.split_whitespace().collect::<Vec<_>>().join(" ");
    if propre.chars().count() > TEXTE_MAX {
        return Err(cle_erreur);
    }
    Ok(propre)
}

/// Texte saisi et exigé.
pub fn texte_exige(texte: &str, cle_erreur: &'static str) -> Result<String, &'static str> {
    let propre = texte_propre(texte, cle_erreur)?;
    if propre.is_empty() {
        return Err(cle_erreur);
    }
    Ok(propre)
}

impl Personne {
    /// Nom complet affiché, prénoms puis nom, tels que saisis (décision du porteur du projet,
    /// 9 octobre 2026). La recherche ne dépend pas de l'ordre des mots.
    pub fn nom_complet(&self) -> String {
        format!("{} {}", self.prenoms, self.nom)
    }

    /// Vérifie l'identité exigée à la création d'une fiche élève : nom, prénoms, sexe et date de
    /// naissance (Q-19), qui ne peut être dans le futur. Le lieu de naissance n'est exigé qu'à la
    /// validation de l'inscription (SPEC_PHASE_2 §5.3).
    pub fn verifier_eleve(&self, aujourd_hui: Horodatage) -> Result<(), &'static str> {
        if self.nom.is_empty() || self.prenoms.is_empty() {
            return Err("eleve.nom_exige");
        }
        if self.sexe.is_none() {
            return Err("eleve.sexe_exige");
        }
        let Some(naissance) = self.date_naissance else {
            return Err("eleve.date_naissance_exigee");
        };
        if DateCivile::du_jour(aujourd_hui).is_some_and(|jour| naissance > jour) {
            return Err("eleve.date_naissance_future");
        }
        Ok(())
    }

    /// Vérifie l'identité d'un responsable : nom, prénoms et au moins un téléphone (SPEC_PHASE_2
    /// §4.3).
    pub fn verifier_responsable(&self) -> Result<(), &'static str> {
        if self.nom.is_empty() || self.prenoms.is_empty() {
            return Err("responsable.nom_exige");
        }
        if self.telephones.is_empty() {
            return Err("responsable.telephone_exige");
        }
        Ok(())
    }
}

/// Matricule interne `AAAA-NNNNN` (T-01) : millésime d'entrée et numéro d'ordre.
pub fn millesime(libelle_annee: &str) -> Option<&str> {
    libelle_annee
        .split_once('-')
        .map(|(premiere, _)| premiere)
        .filter(|a| a.len() == 4 && a.chars().all(|c| c.is_ascii_digit()))
}

/// Matricule national saisi : espaces retirés, majuscules ; vide devient absent.
pub fn matricule_national(texte: &str) -> Result<Option<String>, &'static str> {
    let propre: String = texte
        .chars()
        .filter(|c| !c.is_whitespace())
        .collect::<String>()
        .to_uppercase();
    if propre.chars().count() > 30 {
        return Err("eleve.matricule_national_invalide");
    }
    Ok((!propre.is_empty()).then_some(propre))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn personne() -> Personne {
        Personne {
            id: Id::depuis_ulid(ulid::Ulid::nil()),
            etablissement_id: Id::depuis_ulid(ulid::Ulid::nil()),
            nom: "Traoré".to_owned(),
            prenoms: "Awa".to_owned(),
            sexe: Some(Sexe::Feminin),
            date_naissance: DateCivile::analyser("2014-03-02"),
            date_naissance_estimee: false,
            lieu_naissance: String::new(),
            nationalite: "Malienne".to_owned(),
            adresse: String::new(),
            telephones: vec![],
        }
    }

    const AUJOURD_HUI: Horodatage = Horodatage::depuis_millis_epoque(20_733 * 86_400_000);

    #[test]
    fn identite_d_un_eleve() {
        assert_eq!(personne().verifier_eleve(AUJOURD_HUI), Ok(()));
        let manque = |modifier: fn(&mut Personne)| {
            let mut p = personne();
            modifier(&mut p);
            p.verifier_eleve(AUJOURD_HUI)
        };
        assert_eq!(manque(|p| p.nom.clear()), Err("eleve.nom_exige"));
        assert_eq!(manque(|p| p.sexe = None), Err("eleve.sexe_exige"));
        assert_eq!(
            manque(|p| p.date_naissance = None),
            Err("eleve.date_naissance_exigee")
        );
        assert_eq!(
            manque(|p| p.date_naissance = DateCivile::analyser("2026-10-08")),
            Err("eleve.date_naissance_future")
        );
    }

    #[test]
    fn identite_d_un_responsable() {
        let mut p = personne();
        assert_eq!(p.verifier_responsable(), Err("responsable.telephone_exige"));
        p.telephones.push("+22376123456".to_owned());
        assert_eq!(p.verifier_responsable(), Ok(()));
    }

    #[test]
    fn textes_matricules_et_millesime() {
        assert_eq!(
            texte_exige("  Keïta   Fanta ", "x"),
            Ok("Keïta Fanta".to_owned())
        );
        assert_eq!(texte_exige("  ", "x"), Err("x"));
        assert_eq!(texte_propre(&"a".repeat(81), "x"), Err("x"));
        assert_eq!(millesime("2026-2027"), Some("2026"));
        assert_eq!(millesime("26-27"), None);
        assert_eq!(
            matricule_national(" 12 ab 345 "),
            Ok(Some("12AB345".to_owned()))
        );
        assert_eq!(matricule_national("   "), Ok(None));
    }

    #[test]
    fn noms_persistes_reversibles() {
        for s in [Sexe::Masculin, Sexe::Feminin] {
            assert_eq!(Sexe::depuis_nom(s.nom()), Some(s));
        }
        for n in [NatureActe::Acte, NatureActe::JugementSuppletif] {
            assert_eq!(NatureActe::depuis_nom(n.nom()), Some(n));
        }
        for l in [Lien::Pere, Lien::Mere, Lien::Tuteur, Lien::Autre] {
            assert_eq!(Lien::depuis_nom(l.nom()), Some(l));
        }
    }
}
