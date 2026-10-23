//! Matricule interne d'un élève (T-01, révisé le 7 octobre 2026) : saisi par l'école ou généré
//! selon ses règles. Le matricule national (DEF, BAC) ne se génère jamais : il se recopie.

/// Paramètres d'établissement qui règlent le matricule interne.
pub mod parametres {
    /// Vrai : la secrétaire saisit le matricule ; faux : GESCO le génère.
    pub const SAISIE_MANUELLE: &str = "matricule.saisie_manuelle";
    /// Préfixe de l'école, lettres et chiffres, dix au plus ; vide pour aucun.
    pub const PREFIXE: &str = "matricule.prefixe";
    /// Chiffres de l'année d'entrée : 4 (`2026`), 2 (`26`) ou 0 (aucune année).
    pub const CHIFFRES_ANNEE: &str = "matricule.chiffres_annee";
    /// Chiffres du numéro d'ordre, de 3 à 8.
    pub const CHIFFRES: &str = "matricule.chiffres";
    /// Vrai : la numérotation repart à 1 chaque année ; faux : elle continue.
    pub const REMISE_ANNUELLE: &str = "matricule.remise_annuelle";
    /// Séparateur entre le préfixe, l'année et le numéro : `/`, `-`, `.` ou rien.
    pub const SEPARATEUR: &str = "matricule.separateur";
}

/// Séparateur livré : la barre oblique, `2026/00042` (porteur du projet, 23 octobre 2026).
pub const SEPARATEUR_DEFAUT: &str = "/";

/// Séparateur d'une école dont des élèves ont déjà leur matricule au tiret : la mise à niveau
/// le garde, pour que ses matricules restent d'une seule forme.
pub const SEPARATEUR_ANCIEN: &str = "-";

/// Séparateurs permis.
pub const SEPARATEURS: [&str; 4] = ["/", "-", ".", ""];

/// Longueur maximale d'un matricule saisi.
const SAISI_MAX: usize = 30;

/// Règles de génération du matricule interne.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReglesMatricule {
    /// Préfixe de l'école.
    pub prefixe: String,
    /// Chiffres de l'année : 4, 2 ou 0.
    pub chiffres_annee: i64,
    /// Chiffres du numéro d'ordre.
    pub chiffres: i64,
    /// Numérotation qui repart à 1 chaque année.
    pub remise_annuelle: bool,
    /// Séparateur entre les parties.
    pub separateur: String,
}

impl Default for ReglesMatricule {
    /// `2026/00042` : la forme de T-01, à la barre oblique.
    fn default() -> Self {
        Self {
            prefixe: String::new(),
            chiffres_annee: 4,
            chiffres: 5,
            remise_annuelle: true,
            separateur: SEPARATEUR_DEFAUT.to_owned(),
        }
    }
}

impl ReglesMatricule {
    /// Clé de la séquence : le millésime si la numérotation repart chaque année, une séquence
    /// unique sinon. Sans année dans le matricule, la remise à zéro produirait des doublons :
    /// la numérotation continue alors toujours.
    pub fn exercice(&self, millesime: &str) -> String {
        if self.remise_annuelle && self.chiffres_annee > 0 {
            millesime.to_owned()
        } else {
            "continu".to_owned()
        }
    }

    /// Compose le matricule : préfixe, année, numéro, joints par le séparateur de l'école.
    pub fn formater(&self, millesime: &str, compteur: u64) -> String {
        let mut parties: Vec<String> = Vec::with_capacity(3);
        if !self.prefixe.is_empty() {
            parties.push(self.prefixe.clone());
        }
        match self.chiffres_annee {
            4 => parties.push(millesime.to_owned()),
            2 => parties.push(millesime.chars().skip(2).collect()),
            _ => {}
        }
        let largeur = usize::try_from(self.chiffres).unwrap_or(5);
        parties.push(format!("{compteur:0largeur$}"));
        parties.join(&self.separateur)
    }
}

/// Vrai si la valeur sort des bornes d'un paramètre du matricule.
pub fn hors_bornes(cle: &str, valeur: &crate::core::ValeurParametre) -> bool {
    use crate::core::ValeurParametre::{Entier, Texte};
    match (cle, valeur) {
        (parametres::CHIFFRES_ANNEE, Entier(n)) => ![0, 2, 4].contains(n),
        (parametres::CHIFFRES, Entier(n)) => !(3..=8).contains(n),
        (parametres::SEPARATEUR, Texte(t)) => !SEPARATEURS.contains(&t.as_str()),
        (parametres::PREFIXE, Texte(t)) => {
            t.chars().count() > 10 || !t.chars().all(|c| c.is_ascii_alphanumeric())
        }
        _ => false,
    }
}

/// Matricule saisi par l'école : espaces de bord retirés, non vide, trente caractères au plus.
pub fn saisi(texte: &str) -> Result<String, &'static str> {
    let propre = texte.trim();
    if propre.is_empty()
        || propre.chars().count() > SAISI_MAX
        || propre.chars().any(char::is_control)
    {
        return Err("eleve.matricule_invalide");
    }
    Ok(propre.to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::ValeurParametre;

    #[test]
    fn formes_du_matricule() {
        assert_eq!(
            ReglesMatricule::default().formater("2026", 42),
            "2026/00042"
        );
        let ecole = ReglesMatricule {
            prefixe: "ASK".to_owned(),
            chiffres_annee: 2,
            chiffres: 3,
            remise_annuelle: true,
            separateur: "-".to_owned(),
        };
        assert_eq!(ecole.formater("2026", 42), "ASK-26-042");
        assert_eq!(ecole.formater("2026", 1234), "ASK-26-1234");
        let continu = ReglesMatricule {
            prefixe: "ASK".to_owned(),
            chiffres_annee: 0,
            chiffres: 6,
            remise_annuelle: true,
            separateur: String::new(),
        };
        assert_eq!(continu.formater("2026", 7), "ASK000007");
        assert!(hors_bornes(
            parametres::SEPARATEUR,
            &ValeurParametre::Texte("_".to_owned())
        ));
        assert!(!hors_bornes(
            parametres::SEPARATEUR,
            &ValeurParametre::Texte(".".to_owned())
        ));
    }

    #[test]
    fn sequence_annuelle_ou_continue() {
        assert_eq!(ReglesMatricule::default().exercice("2026"), "2026");
        let continu = ReglesMatricule {
            remise_annuelle: false,
            ..ReglesMatricule::default()
        };
        assert_eq!(continu.exercice("2026"), "continu");
        let sans_annee = ReglesMatricule {
            chiffres_annee: 0,
            ..ReglesMatricule::default()
        };
        assert_eq!(sans_annee.exercice("2026"), "continu");
    }

    #[test]
    fn bornes_et_saisie() {
        assert!(hors_bornes(
            parametres::CHIFFRES_ANNEE,
            &ValeurParametre::Entier(3)
        ));
        assert!(!hors_bornes(
            parametres::CHIFFRES_ANNEE,
            &ValeurParametre::Entier(2)
        ));
        assert!(hors_bornes(
            parametres::CHIFFRES,
            &ValeurParametre::Entier(9)
        ));
        assert!(hors_bornes(
            parametres::PREFIXE,
            &ValeurParametre::Texte("ASK-1".to_owned())
        ));
        assert!(!hors_bornes(
            parametres::PREFIXE,
            &ValeurParametre::Texte("ASK1".to_owned())
        ));
        assert_eq!(saisi("  L-0042 "), Ok("L-0042".to_owned()));
        assert_eq!(saisi("  "), Err("eleve.matricule_invalide"));
    }
}
