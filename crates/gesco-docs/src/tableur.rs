//! Classeur Excel d'une liste affichée (R-41, SPEC_PHASE_2 §11). Le webview envoie les en-têtes
//! et les lignes telles qu'il les montre ; le classeur est composé ici, en Rust (R-44), jamais
//! dans le navigateur.

use rust_xlsxwriter::{DocProperties, Format, FormatBorder, Workbook};

use crate::erreur::ErreurDocument;

/// Lignes au plus dans un export : bien sous la limite d'Excel, assez pour toute liste d'école.
pub const LIGNES_MAX: usize = 100_000;
/// Colonnes au plus dans un export.
pub const COLONNES_MAX: usize = 100;
/// Longueur maximale du nom d'une feuille, imposée par Excel.
const NOM_FEUILLE_MAX: usize = 31;
/// Caractères qu'Excel refuse dans un nom de feuille.
const INTERDITS_FEUILLE: [char; 7] = ['[', ']', ':', '*', '?', '/', '\\'];

/// Liste à exporter : un titre, des en-têtes, des lignes de même largeur.
#[derive(Debug)]
pub struct Liste<'a> {
    /// Titre de la liste, repris en nom de feuille et en titre du classeur.
    pub titre: &'a str,
    /// En-têtes des colonnes, dans l'ordre affiché.
    pub entetes: &'a [String],
    /// Lignes, chacune aussi large que les en-têtes.
    pub lignes: &'a [Vec<String>],
}

/// Compose le classeur : une feuille, en-têtes en gras et figés, filtre automatique, largeurs
/// ajustées. Les cellules qui ne contiennent qu'un nombre sont écrites en nombre, pour que les
/// sommes d'Excel fonctionnent ; tout le reste reste du texte, matricules et téléphones compris.
pub fn classeur(liste: &Liste<'_>) -> Result<Vec<u8>, ErreurDocument> {
    classeur_feuilles(liste.titre, std::slice::from_ref(liste))
}

/// Compose un classeur d'une feuille par liste, chacune comme [`classeur`] la compose ; le
/// classeur porte le titre donné.
pub fn classeur_feuilles(titre: &str, listes: &[Liste<'_>]) -> Result<Vec<u8>, ErreurDocument> {
    let mut classeur = Workbook::new();
    classeur.set_properties(&DocProperties::new().set_title(titre));
    for liste in listes {
        ecrire_feuille(&mut classeur, liste)?;
    }
    Ok(classeur.save_to_buffer()?)
}

fn ecrire_feuille(classeur: &mut Workbook, liste: &Liste<'_>) -> Result<(), ErreurDocument> {
    verifier(liste)?;
    let feuille = classeur.add_worksheet();
    feuille.set_name(nom_feuille(liste.titre))?;
    let entete = Format::new()
        .set_bold()
        .set_border_bottom(FormatBorder::Thin);
    for (colonne, texte) in liste.entetes.iter().enumerate() {
        feuille.write_string_with_format(0, u16::try_from(colonne)?, texte, &entete)?;
    }
    for (index, ligne) in liste.lignes.iter().enumerate() {
        let rang = u32::try_from(index + 1)?;
        for (colonne, texte) in ligne.iter().enumerate() {
            let colonne = u16::try_from(colonne)?;
            match nombre(texte) {
                Some(valeur) => feuille.write_number(rang, colonne, valeur)?,
                None => feuille.write_string(rang, colonne, texte)?,
            };
        }
    }
    feuille.set_freeze_panes(1, 0)?;
    let derniere_ligne = u32::try_from(liste.lignes.len())?;
    let derniere_colonne = u16::try_from(liste.entetes.len().saturating_sub(1))?;
    feuille.autofilter(0, 0, derniere_ligne, derniere_colonne)?;
    feuille.autofit();
    Ok(())
}

/// Refuse une liste sans colonne, trop grande, ou dont une ligne n'a pas la largeur des
/// en-têtes : on n'exporte pas un tableau de travers.
fn verifier(liste: &Liste<'_>) -> Result<(), ErreurDocument> {
    if liste.entetes.is_empty() {
        return Err(ErreurDocument::ListeIrreguliere { ligne: 0 });
    }
    if liste.entetes.len() > COLONNES_MAX || liste.lignes.len() > LIGNES_MAX {
        return Err(ErreurDocument::ListeTropGrande);
    }
    if let Some(index) = liste
        .lignes
        .iter()
        .position(|ligne| ligne.len() != liste.entetes.len())
    {
        return Err(ErreurDocument::ListeIrreguliere { ligne: index + 1 });
    }
    Ok(())
}

/// Nom de feuille acceptable par Excel : sans ses caractères interdits ni apostrophe aux bouts,
/// 31 caractères au plus, jamais vide.
pub fn nom_feuille(titre: &str) -> String {
    let nettoye: String = titre
        .chars()
        .map(|c| {
            if INTERDITS_FEUILLE.contains(&c) {
                ' '
            } else {
                c
            }
        })
        .collect();
    let nom: String = nettoye
        .trim()
        .trim_matches('\'')
        .trim()
        .chars()
        .take(NOM_FEUILLE_MAX)
        .collect();
    let nom = nom.trim_end().to_owned();
    if nom.is_empty() {
        "Liste".to_owned()
    } else {
        nom
    }
}

/// Valeur numérique d'une cellule qui n'est qu'un nombre : chiffres, signe, séparateur décimal
/// point ou virgule. Un zéro de tête (« 00042 »), un tiret intérieur (« 2026-00001 ») ou un
/// espace (« +223 76 12 34 56 ») en font un texte, et il le reste.
pub fn nombre(texte: &str) -> Option<f64> {
    let texte = texte.trim();
    let sans_signe = texte.strip_prefix('-').unwrap_or(texte);
    let (entiere, decimale) = match sans_signe.split_once([',', '.']) {
        Some((e, d)) => (e, Some(d)),
        None => (sans_signe, None),
    };
    let chiffres = |s: &str| !s.is_empty() && s.bytes().all(|b| b.is_ascii_digit());
    if !chiffres(entiere) || (entiere.len() > 1 && entiere.starts_with('0')) {
        return None;
    }
    if decimale.is_some_and(|d| !chiffres(d)) {
        return None;
    }
    if sans_signe.len() > 15 {
        return None;
    }
    texte.replace(',', ".").parse().ok()
}

#[cfg(test)]
#[allow(clippy::indexing_slicing)]
mod tests {
    use calamine::{Data, Reader, Xlsx};

    use super::*;

    fn lignes(valeurs: &[&[&str]]) -> Vec<Vec<String>> {
        valeurs
            .iter()
            .map(|l| l.iter().map(|c| (*c).to_owned()).collect())
            .collect()
    }

    fn relire(octets: Vec<u8>) -> (String, Vec<Vec<Data>>) {
        let Ok(mut classeur) = Xlsx::new(std::io::Cursor::new(octets)) else {
            unreachable!("classeur illisible")
        };
        let nom = classeur.sheet_names().first().cloned().unwrap_or_default();
        let Ok(plage) = classeur.worksheet_range(&nom) else {
            unreachable!("feuille illisible")
        };
        (nom, plage.rows().map(<[Data]>::to_vec).collect())
    }

    #[test]
    fn le_classeur_se_relit_avec_ses_textes_et_ses_nombres() {
        let entetes = ["Matricule".to_owned(), "Nom".to_owned(), "Âge".to_owned()];
        let lignes = lignes(&[
            &["2026-00001", "Traoré Awa", "12"],
            &["00042", "Diallo Moussa", "11,5"],
        ]);
        let liste = Liste {
            titre: "Inscriptions 2026/2027",
            entetes: &entetes,
            lignes: &lignes,
        };
        let Ok(octets) = classeur(&liste) else {
            unreachable!("composition refusée")
        };
        let (nom, cellules) = relire(octets);
        assert_eq!(nom, "Inscriptions 2026 2027");
        assert_eq!(cellules.len(), 3);
        assert_eq!(cellules[0][2], Data::String("Âge".to_owned()));
        assert_eq!(cellules[1][0], Data::String("2026-00001".to_owned()));
        assert_eq!(cellules[1][2], Data::Float(12.0));
        assert_eq!(cellules[2][0], Data::String("00042".to_owned()));
        assert_eq!(cellules[2][2], Data::Float(11.5));
    }

    #[test]
    fn une_ligne_de_travers_est_refusee() {
        let entetes = ["A".to_owned(), "B".to_owned()];
        let lignes = lignes(&[&["1", "2"], &["seul"]]);
        let liste = Liste {
            titre: "Essai",
            entetes: &entetes,
            lignes: &lignes,
        };
        assert!(matches!(
            classeur(&liste),
            Err(ErreurDocument::ListeIrreguliere { ligne: 2 })
        ));
        let sans_colonne = Liste {
            titre: "Essai",
            entetes: &[],
            lignes: &[],
        };
        assert!(matches!(
            classeur(&sans_colonne),
            Err(ErreurDocument::ListeIrreguliere { ligne: 0 })
        ));
    }

    #[test]
    fn le_nom_de_feuille_respecte_excel() {
        assert_eq!(
            nom_feuille("Élèves : classe 7e [A]"),
            "Élèves   classe 7e  A"
        );
        assert_eq!(nom_feuille("'  '"), "Liste");
        assert_eq!(nom_feuille("[]"), "Liste");
        let long = nom_feuille("Liste des inscriptions validées de l'année 2026");
        assert_eq!(long.chars().count(), 31);
        assert_eq!(long, "Liste des inscriptions validées");
    }

    #[test]
    fn seuls_les_nombres_francs_deviennent_des_nombres() {
        assert_eq!(nombre("12"), Some(12.0));
        assert_eq!(nombre("-3,5"), Some(-3.5));
        assert_eq!(nombre(" 0 "), Some(0.0));
        assert_eq!(nombre("00042"), None);
        assert_eq!(nombre("2026-00001"), None);
        assert_eq!(nombre("+223 76 12 34 56"), None);
        assert_eq!(nombre("12e"), None);
        assert_eq!(nombre(""), None);
        assert_eq!(nombre("1234567890123456"), None);
    }
}
