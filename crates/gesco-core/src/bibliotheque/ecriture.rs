//! Écrire dans la bibliothèque (§7 et §8 de la spécification) : états d'une version de l'école,
//! plan proposé à une leçon nouvelle, notes de l'auteur sous l'aperçu, comparaison ligne à ligne
//! de deux textes (écrite ici, sans dépendance). Règles pures, sans E/S.

use serde::{Deserialize, Serialize};

use super::lecon::{Bloc, Enligne, Lecon};

/// Plan des leçons livrées, proposé au départ d'une leçon nouvelle ; l'auteur le garde ou
/// l'efface.
pub const PLAN_LECON: &str = "## L'essentiel\n\n\n\n## Les méthodes\n\n\n\n## Exemples résolus\n\n\n\n\
## Les erreurs à ne pas faire\n\n\n\n## Exercices\n\n\n\n## Corrigés\n\n\n\n## Quiz\n\n";

/// Taille la plus grande d'un texte de leçon, en octets.
pub const TEXTE_MAX: usize = 2_000_000;

/// Début d'une note de la bibliothèque qui demande une image.
const IMAGE_A_AJOUTER: &str = "image à ajouter ici";

/// Au-delà de ce nombre de cases, la comparaison ne cherche plus les lignes communes au milieu
/// des deux textes : elle montre le milieu retiré puis ajouté (jamais de refus).
const CASES_MAX: usize = 16_000_000;

/// État d'une version de l'école.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "snake_case")]
pub enum EtatVersion {
    /// En cours d'écriture ; se réécrit sur place.
    Brouillon,
    /// Soumise à la direction ; ne change plus.
    Soumise,
    /// Renvoyée à son auteur avec un commentaire.
    Renvoyee,
    /// Publiée : c'est la version visible.
    Publiee,
    /// Publiée puis remplacée par une autre.
    Remplacee,
    /// Brouillon abandonné, gardé dans l'historique.
    Abandonnee,
}

impl EtatVersion {
    /// Nom persisté.
    pub const fn nom(self) -> &'static str {
        match self {
            Self::Brouillon => "brouillon",
            Self::Soumise => "soumise",
            Self::Renvoyee => "renvoyee",
            Self::Publiee => "publiee",
            Self::Remplacee => "remplacee",
            Self::Abandonnee => "abandonnee",
        }
    }

    /// État d'après son nom persisté.
    pub fn depuis_nom(nom: &str) -> Option<Self> {
        Some(match nom {
            "brouillon" => Self::Brouillon,
            "soumise" => Self::Soumise,
            "renvoyee" => Self::Renvoyee,
            "publiee" => Self::Publiee,
            "remplacee" => Self::Remplacee,
            "abandonnee" => Self::Abandonnee,
            _ => return None,
        })
    }
}

/// Une ligne numérotée d'un des deux textes comparés.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
pub struct LigneNumerotee {
    /// Numéro de la ligne dans son texte, à partir de 1.
    pub numero: u32,
    /// Texte de la ligne.
    pub texte: String,
}

/// Une rangée de la comparaison côte à côte : la ligne d'avant à gauche, celle d'après à droite.
/// Une ligne retirée n'a rien à droite, une ligne ajoutée rien à gauche ; une ligne changée a
/// les deux, différentes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
pub struct RangeeComparaison {
    /// Ligne du premier texte.
    pub gauche: Option<LigneNumerotee>,
    /// Ligne du second texte.
    pub droite: Option<LigneNumerotee>,
    /// Vrai si la rangée n'est pas une ligne commune aux deux textes.
    pub change: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Pas {
    Pareille(usize, usize),
    Retiree(usize),
    Ajoutee(usize),
}

/// Lignes communes du milieu, par la plus longue sous-suite commune.
fn pas_du_milieu(a: &[&str], b: &[&str]) -> Vec<Pas> {
    let (n, m) = (a.len(), b.len());
    let mut pas = Vec::new();
    if n.saturating_mul(m) > CASES_MAX {
        pas.extend((0..n).map(Pas::Retiree));
        pas.extend((0..m).map(Pas::Ajoutee));
        return pas;
    }
    // longueurs[i * (m + 1) + j] : plus longue sous-suite commune de a[i..] et b[j..].
    let largeur = m + 1;
    let mut longueurs = vec![0u32; (n + 1) * largeur];
    let lire = |t: &[u32], i: usize, j: usize| t.get(i * largeur + j).copied().unwrap_or(0);
    for i in (0..n).rev() {
        for j in (0..m).rev() {
            let valeur = if a.get(i) == b.get(j) {
                lire(&longueurs, i + 1, j + 1) + 1
            } else {
                lire(&longueurs, i + 1, j).max(lire(&longueurs, i, j + 1))
            };
            if let Some(c) = longueurs.get_mut(i * largeur + j) {
                *c = valeur;
            }
        }
    }
    let (mut i, mut j) = (0, 0);
    while i < n && j < m {
        if a.get(i) == b.get(j) {
            pas.push(Pas::Pareille(i, j));
            i += 1;
            j += 1;
        } else if lire(&longueurs, i + 1, j) >= lire(&longueurs, i, j + 1) {
            pas.push(Pas::Retiree(i));
            i += 1;
        } else {
            pas.push(Pas::Ajoutee(j));
            j += 1;
        }
    }
    pas.extend((i..n).map(Pas::Retiree));
    pas.extend((j..m).map(Pas::Ajoutee));
    pas
}

fn ligne(lignes: &[&str], i: usize) -> LigneNumerotee {
    LigneNumerotee {
        numero: u32::try_from(i + 1).unwrap_or(u32::MAX),
        texte: lignes.get(i).copied().unwrap_or_default().to_owned(),
    }
}

/// Compare deux textes ligne à ligne et rend les rangées côte à côte : les lignes communes en
/// regard, puis chaque suite de lignes retirées en face de la suite de lignes ajoutées qui la
/// suit.
pub fn comparer_lignes(avant: &str, apres: &str) -> Vec<RangeeComparaison> {
    let a: Vec<&str> = avant.lines().collect();
    let b: Vec<&str> = apres.lines().collect();
    // Début et fin communs, sans calcul.
    let debut = a.iter().zip(&b).take_while(|(x, y)| x == y).count();
    let reste_a = a.len() - debut;
    let reste_b = b.len() - debut;
    let fin = a
        .iter()
        .rev()
        .zip(b.iter().rev())
        .take(reste_a.min(reste_b))
        .take_while(|(x, y)| x == y)
        .count();
    let milieu_a = a.get(debut..a.len() - fin).unwrap_or_default();
    let milieu_b = b.get(debut..b.len() - fin).unwrap_or_default();

    let mut pas: Vec<Pas> = (0..debut).map(|i| Pas::Pareille(i, i)).collect();
    pas.extend(
        pas_du_milieu(milieu_a, milieu_b)
            .into_iter()
            .map(|p| match p {
                Pas::Pareille(i, j) => Pas::Pareille(i + debut, j + debut),
                Pas::Retiree(i) => Pas::Retiree(i + debut),
                Pas::Ajoutee(j) => Pas::Ajoutee(j + debut),
            }),
    );
    pas.extend((0..fin).map(|k| Pas::Pareille(a.len() - fin + k, b.len() - fin + k)));

    let mut rangees = Vec::new();
    let mut retirees: Vec<usize> = Vec::new();
    let mut ajoutees: Vec<usize> = Vec::new();
    let vider = |retirees: &mut Vec<usize>,
                 ajoutees: &mut Vec<usize>,
                 rangees: &mut Vec<RangeeComparaison>| {
        let n = retirees.len().max(ajoutees.len());
        for k in 0..n {
            rangees.push(RangeeComparaison {
                gauche: retirees.get(k).map(|&i| ligne(&a, i)),
                droite: ajoutees.get(k).map(|&j| ligne(&b, j)),
                change: true,
            });
        }
        retirees.clear();
        ajoutees.clear();
    };
    for p in pas {
        match p {
            Pas::Pareille(i, j) => {
                vider(&mut retirees, &mut ajoutees, &mut rangees);
                rangees.push(RangeeComparaison {
                    gauche: Some(ligne(&a, i)),
                    droite: Some(ligne(&b, j)),
                    change: false,
                });
            }
            Pas::Retiree(i) => {
                // Une ligne retirée après des ajoutées commence une nouvelle suite.
                if !ajoutees.is_empty() {
                    vider(&mut retirees, &mut ajoutees, &mut rangees);
                }
                retirees.push(i);
            }
            Pas::Ajoutee(j) => ajoutees.push(j),
        }
    }
    vider(&mut retirees, &mut ajoutees, &mut rangees);
    rangees
}

fn texte_enlignes(contenu: &[Enligne], texte: &mut String) {
    for e in contenu {
        match e {
            Enligne::Texte { texte: t } | Enligne::Code { texte: t } => texte.push_str(t),
            Enligne::Gras { contenu } | Enligne::Italique { contenu } => {
                texte_enlignes(contenu, texte);
            }
            Enligne::Formule { latex, .. } => texte.push_str(latex),
            Enligne::Image { description, .. } => texte.push_str(description),
            Enligne::Video { titre, .. } => texte.push_str(titre),
            Enligne::Saut => texte.push(' '),
        }
    }
}

fn texte_blocs(blocs: &[Bloc], texte: &mut String) {
    for b in blocs {
        match b {
            Bloc::Titre { contenu, .. } | Bloc::Paragraphe { contenu } => {
                texte_enlignes(contenu, texte);
                texte.push(' ');
            }
            Bloc::Formule { latex } => texte.push_str(latex),
            Bloc::TexteBrut { texte: t } => texte.push_str(t),
            Bloc::Liste { elements, .. } => {
                for e in elements {
                    texte_blocs(e, texte);
                }
            }
            Bloc::Citation { blocs } | Bloc::NoteMaitre { blocs } => texte_blocs(blocs, texte),
            Bloc::Tableau { .. } | Bloc::Filet => {}
        }
    }
}

fn notes_image(blocs: &[Bloc], notes: &mut Vec<String>) {
    for b in blocs {
        match b {
            Bloc::NoteMaitre { blocs } => {
                let mut texte = String::new();
                texte_blocs(blocs, &mut texte);
                let texte = texte.split_whitespace().collect::<Vec<_>>().join(" ");
                let bas = texte.to_lowercase();
                if let Some(position) = bas.find(IMAGE_A_AJOUTER) {
                    // Ce qui suit « Image à ajouter ici », sans les deux-points.
                    let suite = texte
                        .get(position + IMAGE_A_AJOUTER.len()..)
                        .unwrap_or_default()
                        .trim_start_matches([' ', ':'])
                        .trim();
                    notes.push(suite.to_owned());
                }
            }
            Bloc::Citation { blocs } => notes_image(blocs, notes),
            Bloc::Liste { elements, .. } => {
                for e in elements {
                    notes_image(e, notes);
                }
            }
            _ => {}
        }
    }
}

/// Les images que les notes de la bibliothèque demandent d'ajouter (« Image à ajouter ici : …»),
/// dans l'ordre du texte : ce qui suit la mention, pour l'auteur seulement.
pub fn images_a_ajouter(lecon: &Lecon) -> Vec<String> {
    let mut notes = Vec::new();
    notes_image(&lecon.blocs, &mut notes);
    notes
}

#[cfg(test)]
#[allow(clippy::indexing_slicing)]
mod tests {
    use super::*;
    use crate::bibliotheque::analyser;

    fn rangee(g: Option<(u32, &str)>, d: Option<(u32, &str)>, change: bool) -> RangeeComparaison {
        let l = |(numero, texte): (u32, &str)| LigneNumerotee {
            numero,
            texte: texte.to_owned(),
        };
        RangeeComparaison {
            gauche: g.map(l),
            droite: d.map(l),
            change,
        }
    }

    #[test]
    fn deux_textes_pareils_n_ont_aucun_changement() {
        let r = comparer_lignes("a\nb\nc", "a\nb\nc");
        assert_eq!(r.len(), 3);
        assert!(r.iter().all(|x| !x.change));
    }

    #[test]
    fn une_ligne_changee_se_montre_en_face() {
        let r = comparer_lignes("a\nb\nc", "a\nB\nc");
        assert_eq!(
            r,
            vec![
                rangee(Some((1, "a")), Some((1, "a")), false),
                rangee(Some((2, "b")), Some((2, "B")), true),
                rangee(Some((3, "c")), Some((3, "c")), false),
            ]
        );
    }

    #[test]
    fn lignes_ajoutees_et_retirees() {
        let r = comparer_lignes("a\nb\nc\nd", "a\nc\nd\ne");
        assert_eq!(
            r,
            vec![
                rangee(Some((1, "a")), Some((1, "a")), false),
                rangee(Some((2, "b")), None, true),
                rangee(Some((3, "c")), Some((2, "c")), false),
                rangee(Some((4, "d")), Some((3, "d")), false),
                rangee(None, Some((4, "e")), true),
            ]
        );
    }

    #[test]
    fn un_texte_vide_d_un_cote() {
        let r = comparer_lignes("", "x\ny");
        assert_eq!(
            r,
            vec![
                rangee(None, Some((1, "x")), true),
                rangee(None, Some((2, "y")), true),
            ]
        );
        let r = comparer_lignes("x", "");
        assert_eq!(r, vec![rangee(Some((1, "x")), None, true)]);
    }

    #[test]
    fn les_lignes_communes_au_milieu_sont_retrouvees() {
        let avant = "t\n1\n2\nmilieu\n3\nfin";
        let apres = "t\nA\nmilieu\nB\nC\nfin";
        let r = comparer_lignes(avant, apres);
        let communes: Vec<&str> = r
            .iter()
            .filter(|x| !x.change)
            .filter_map(|x| x.gauche.as_ref().map(|l| l.texte.as_str()))
            .collect();
        assert_eq!(communes, vec!["t", "milieu", "fin"]);
        // Chaque ligne de chaque texte paraît une fois, dans l'ordre.
        let gauche: Vec<u32> = r
            .iter()
            .filter_map(|x| x.gauche.as_ref().map(|l| l.numero))
            .collect();
        let droite: Vec<u32> = r
            .iter()
            .filter_map(|x| x.droite.as_ref().map(|l| l.numero))
            .collect();
        assert_eq!(gauche, vec![1, 2, 3, 4, 5, 6]);
        assert_eq!(droite, vec![1, 2, 3, 4, 5, 6]);
    }

    #[test]
    fn chaque_etat_se_relit() {
        for e in [
            EtatVersion::Brouillon,
            EtatVersion::Soumise,
            EtatVersion::Renvoyee,
            EtatVersion::Publiee,
            EtatVersion::Remplacee,
            EtatVersion::Abandonnee,
        ] {
            assert_eq!(EtatVersion::depuis_nom(e.nom()), Some(e));
        }
        assert_eq!(EtatVersion::depuis_nom("autre"), None);
    }

    #[test]
    fn le_plan_d_une_lecon_a_ses_sept_parties() {
        let lecon = analyser(PLAN_LECON);
        let titres = lecon
            .blocs
            .iter()
            .filter(|b| matches!(b, Bloc::Titre { niveau: 2, .. }))
            .count();
        assert_eq!(titres, 7);
    }

    #[test]
    fn les_images_a_ajouter_viennent_des_notes() {
        let lecon = analyser(
            "Texte.\n\n> **Note pour le maître** : Image à ajouter ici : une pirogue sur le fleuve.\n\n\
             > Une citation.\n\n> **Note pour le maître** : penser au tableau.\n",
        );
        assert_eq!(images_a_ajouter(&lecon), vec!["une pirogue sur le fleuve."]);
    }
}
