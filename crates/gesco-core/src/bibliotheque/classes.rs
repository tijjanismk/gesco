//! Un ajout de l'école réservé à certaines classes d'un niveau (phase 12, lot 2, T-125). Règles
//! pures : un chapitre ou un contenu de l'école sans classe choisie pour l'année d'une classe est
//! pour tout le niveau ; avec des classes choisies, il n'est que pour elles. Une classe choisie
//! une année passée ne compte plus : l'année suivante, le contenu revient à tout le niveau tant
//! que personne ne choisit d'autres classes. Le contenu livré, et la version de l'école d'une
//! leçon livrée, sont toujours pour tout le niveau.

/// Une classe choisie pour un chapitre ou un contenu, avec l'année de la classe.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClasseChoisie {
    /// Identifiant de la classe.
    pub classe_id: String,
    /// Année scolaire de la classe.
    pub annee_id: String,
}

/// Les classes choisies qui comptent pour une année.
pub fn classes_de_l_annee<'a>(choix: &'a [ClasseChoisie], annee_id: &str) -> Vec<&'a str> {
    choix
        .iter()
        .filter(|c| c.annee_id == annee_id)
        .map(|c| c.classe_id.as_str())
        .collect()
}

/// Vrai si un choix de classes laisse voir à cette classe de cette année.
fn choix_permet(choix: &[ClasseChoisie], classe_id: &str, annee_id: &str) -> bool {
    let de_l_annee = classes_de_l_annee(choix, annee_id);
    de_l_annee.is_empty() || de_l_annee.contains(&classe_id)
}

/// « Visible pour une classe » : ce que le portail de l'élève (lot 4) appellera. Le contenu se
/// montre à une classe si ni son propre choix ni celui de son chapitre ne la laisse de côté pour
/// l'année de cette classe. Le reste de la visibilité (publication, retraits, niveau, série) se
/// décide ailleurs.
pub fn visible_pour_classe(
    choix_du_contenu: &[ClasseChoisie],
    choix_du_chapitre: &[ClasseChoisie],
    classe_id: &str,
    annee_id: &str,
) -> bool {
    choix_permet(choix_du_contenu, classe_id, annee_id)
        && choix_permet(choix_du_chapitre, classe_id, annee_id)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn choix(classe: &str, annee: &str) -> ClasseChoisie {
        ClasseChoisie {
            classe_id: classe.to_owned(),
            annee_id: annee.to_owned(),
        }
    }

    #[test]
    fn sans_choix_tout_le_niveau_voit() {
        assert!(visible_pour_classe(&[], &[], "9B", "2026"));
    }

    #[test]
    fn une_lecon_reservee_a_la_9e_a_n_est_que_pour_elle() {
        let lecon = [choix("9A", "2026")];
        assert!(visible_pour_classe(&lecon, &[], "9A", "2026"));
        assert!(!visible_pour_classe(&lecon, &[], "9B", "2026"));
    }

    #[test]
    fn le_choix_du_chapitre_vaut_pour_ses_contenus() {
        let chapitre = [choix("9A", "2026"), choix("9C", "2026")];
        assert!(visible_pour_classe(&[], &chapitre, "9C", "2026"));
        assert!(!visible_pour_classe(&[], &chapitre, "9B", "2026"));
        // Le contenu ne peut que restreindre encore.
        assert!(!visible_pour_classe(
            &[choix("9A", "2026")],
            &chapitre,
            "9C",
            "2026"
        ));
    }

    #[test]
    fn l_annee_suivante_le_contenu_revient_a_tout_le_niveau() {
        let lecon = [choix("9A-2026", "2026")];
        assert!(visible_pour_classe(&lecon, &[], "9B-2027", "2027"));
        assert_eq!(classes_de_l_annee(&lecon, "2027"), Vec::<&str>::new());
        let lecon = [choix("9A-2026", "2026"), choix("9A-2027", "2027")];
        assert!(!visible_pour_classe(&lecon, &[], "9B-2027", "2027"));
    }
}
