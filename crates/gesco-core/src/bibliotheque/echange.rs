//! Fichier d'échange de leçons entre écoles (ADR « échange de leçons entre écoles ») : un fichier
//! `.gles` que l'école passe sur une clé ou par WhatsApp. Il porte la version visible des leçons
//! choisies, leurs images, leurs sons et les fichiers de leurs documents ; ni vidéo, ni
//! historique, ni nom d'auteur, ni nom d'école.
//!
//! ```text
//! "GLES" | format (u16, petit-boutiste) | empreinte SHA-256 de tout ce qui suit (32 octets)
//!        | longueur du manifeste (u32) | manifeste JSON | fichiers compressés, bout à bout
//! ```
//!
//! Aucune signature : n'importe quelle école fabrique un fichier. L'empreinte d'en-tête et celle
//! de chaque fichier font refuser entier un fichier dont un seul octet a changé ; ce qui protège
//! l'école qui reçoit, c'est le filtre du texte à la lecture (`analyser`), le nettoyage des SVG,
//! les bornes des fichiers et l'arrivée en brouillon. Lecture et fabrication travaillent en
//! mémoire, sans E/S (R-03).

use std::collections::{BTreeMap, BTreeSet};
use std::io::{Read, Write};

use serde::{Deserialize, Serialize};

use super::ecriture::TEXTE_MAX;
use super::fichiers::{
    SorteFichier, TAILLE_DOCUMENT_MAX, TAILLE_IMAGE_MAX, TAILLE_SON_MAX, est_empreinte,
    nettoyer_svg, reconnaitre,
};
use super::paquet::{ExamenSujet, FichierPaquet, NatureContenu, empreinte};

/// Marque de début d'un fichier d'échange.
const MARQUE: &[u8; 4] = b"GLES";
/// Version du format de fichier.
pub const FORMAT_ECHANGE: u16 = 1;
/// Taille d'un fichier d'échange au-delà de laquelle il est refusé sans être lu : 200 Mo.
pub const TAILLE_ECHANGE_MAX: u64 = 200 * 1024 * 1024;
/// Nombre de leçons le plus grand d'un fichier.
pub const LECONS_ECHANGE_MAX: usize = 5_000;
/// Taille du manifeste la plus grande.
const TAILLE_MANIFESTE_MAX: usize = 16 * 1024 * 1024;
/// Taille décompressée de l'ensemble des fichiers la plus grande : 1 Go.
const TAILLE_DECOMPRESSEE_MAX: u64 = 1024 * 1024 * 1024;
/// Longueur la plus grande d'une clé reçue : la clé d'une leçon rangée sous `ecole/recu/` tient
/// dans les 300 caractères de la base.
const CLE_MAX: usize = 280;

/// Pourquoi un fichier d'échange est refusé.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ErreurEchange {
    /// Fichier tronqué, modifié, ou qui n'est pas un fichier de leçons.
    #[error("Ce fichier de leçons est abîmé ou n'en est pas un.")]
    Abime,
    /// Format plus récent que celui que ce GESCO sait lire.
    #[error("Ce fichier de leçons demande une version plus récente de GESCO.")]
    FormatInconnu,
    /// Fabrication impossible.
    #[error("Fabrication du fichier de leçons impossible : {0}")]
    Fabrication(String),
}

/// Le chapitre d'une leçon, tel que l'école qui exporte le range.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChapitreEchange {
    /// Clé du chapitre (clé livrée, ou `ecole/…`).
    pub cle: String,
    /// Chapitre livré par le paquet.
    pub livre: bool,
    /// Code national du niveau.
    pub niveau: String,
    /// Code de la discipline de référence (sinon le code de la matière de l'école).
    pub discipline: String,
    /// Libellé de référence de la discipline (sinon celui de la matière de l'école).
    pub discipline_libelle: String,
    /// Séries ; vide pour toutes.
    pub series: Vec<String>,
    /// Partie ; vide si aucune.
    pub partie: String,
    /// Titre.
    pub titre: String,
}

/// Un fichier joint à un document.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct JointEchange {
    /// Empreinte SHA-256 du fichier.
    pub empreinte: String,
    /// Nom du fichier.
    pub nom: String,
}

/// Une leçon (ou un sujet, un corrigé, un document) du fichier.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LeconEchange {
    /// Clé du contenu : la clé livrée pour une version de l'école d'une leçon livrée, sinon la
    /// clé de l'école (`ecole/…`), unique partout.
    pub cle: String,
    /// Version de l'école d'une leçon livrée.
    pub livre: bool,
    /// Nature.
    pub nature: NatureContenu,
    /// Titre.
    pub titre: String,
    /// Phrase de présentation d'un document.
    pub presentation: String,
    /// Examen d'un sujet ou d'un corrigé.
    pub examen: Option<ExamenSujet>,
    /// Série d'un sujet du lycée.
    pub serie: Option<String>,
    /// Session d'un sujet.
    pub session: Option<u16>,
    /// Contenu lié (le corrigé d'un sujet, et l'inverse), par sa clé.
    pub lie: Option<String>,
    /// Chapitre.
    pub chapitre: ChapitreEchange,
    /// Nom du fichier du texte, dans le fichier d'échange.
    pub texte: String,
    /// Fichiers d'un document, dans l'ordre.
    pub joints: Vec<JointEchange>,
}

/// Manifeste d'un fichier d'échange.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ManifesteEchange {
    /// Date de fabrication (AAAA-MM-JJ).
    pub date: String,
    /// Version de GESCO qui a fabriqué le fichier.
    pub gesco: String,
    /// Leçons.
    pub lecons: Vec<LeconEchange>,
    /// Noms des images et des sons, par empreinte.
    pub noms: BTreeMap<String, String>,
    /// Fichiers ; rempli à la fabrication.
    pub fichiers: Vec<FichierPaquet>,
}

/// Un fichier d'échange lu et vérifié : son manifeste et ses fichiers décompressés.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EchangeLu {
    /// Manifeste.
    pub manifeste: ManifesteEchange,
    /// Fichiers par nom.
    pub fichiers: BTreeMap<String, Vec<u8>>,
}

impl EchangeLu {
    /// Texte d'une leçon (vérifié en UTF-8 à la lecture).
    pub fn texte(&self, lecon: &LeconEchange) -> &str {
        self.fichiers
            .get(&lecon.texte)
            .and_then(|o| std::str::from_utf8(o).ok())
            .unwrap_or_default()
    }

    /// Octets d'un fichier rangé par empreinte (image, son ou fichier d'un document).
    pub fn fichier(&self, empreinte: &str) -> Option<&[u8]> {
        self.fichiers
            .get(&nom_fichier_echange(empreinte))
            .map(Vec::as_slice)
    }
}

/// Nom, dans le fichier d'échange, d'un fichier rangé par empreinte.
pub fn nom_fichier_echange(empreinte: &str) -> String {
    format!("fichiers/{empreinte}")
}

/// Nom, dans le fichier d'échange, du texte de la leçon de rang `rang`.
pub fn nom_texte_echange(rang: usize) -> String {
    format!("textes/{rang}.md")
}

/// Lit un fichier d'échange, vérifie son empreinte et chaque fichier, et rend son contenu. Un
/// seul octet changé fait refuser le fichier entier.
pub fn lire_echange(octets: &[u8]) -> Result<EchangeLu, ErreurEchange> {
    if octets.len() as u64 > TAILLE_ECHANGE_MAX {
        return Err(ErreurEchange::Abime);
    }
    if octets.get(..4) != Some(MARQUE.as_slice()) {
        return Err(ErreurEchange::Abime);
    }
    let format = octets
        .get(4..6)
        .and_then(|o| <[u8; 2]>::try_from(o).ok())
        .map(u16::from_le_bytes)
        .ok_or(ErreurEchange::Abime)?;
    if format != FORMAT_ECHANGE {
        return Err(ErreurEchange::FormatInconnu);
    }
    let annoncee = octets.get(6..38).ok_or(ErreurEchange::Abime)?;
    let reste = octets.get(38..).ok_or(ErreurEchange::Abime)?;
    if hex::encode(annoncee) != empreinte(reste) {
        return Err(ErreurEchange::Abime);
    }
    let longueur = reste
        .get(..4)
        .and_then(|o| <[u8; 4]>::try_from(o).ok())
        .map(u32::from_le_bytes)
        .ok_or(ErreurEchange::Abime)?;
    let longueur = usize::try_from(longueur).map_err(|_| ErreurEchange::Abime)?;
    if longueur > TAILLE_MANIFESTE_MAX {
        return Err(ErreurEchange::Abime);
    }
    let fin_manifeste = 4_usize.checked_add(longueur).ok_or(ErreurEchange::Abime)?;
    let manifeste_octets = reste.get(4..fin_manifeste).ok_or(ErreurEchange::Abime)?;
    let donnees = reste.get(fin_manifeste..).ok_or(ErreurEchange::Abime)?;
    let manifeste: ManifesteEchange =
        serde_json::from_slice(manifeste_octets).map_err(|_| ErreurEchange::Abime)?;

    let mut fichiers = BTreeMap::new();
    let mut fin_attendue = 0_u64;
    let mut total = 0_u64;
    for f in &manifeste.fichiers {
        // Les fichiers se suivent sans trou ni chevauchement ; aucun ne dépasse la plus grande
        // borne d'un fichier de la bibliothèque (un document de 20 Mo, un texte de 2 Mo).
        if f.debut != fin_attendue || f.taille > TAILLE_DOCUMENT_MAX {
            return Err(ErreurEchange::Abime);
        }
        total = total.saturating_add(f.taille);
        if total > TAILLE_DECOMPRESSEE_MAX {
            return Err(ErreurEchange::Abime);
        }
        fin_attendue = f
            .debut
            .checked_add(f.longueur)
            .ok_or(ErreurEchange::Abime)?;
        let debut = usize::try_from(f.debut).map_err(|_| ErreurEchange::Abime)?;
        let fin = usize::try_from(fin_attendue).map_err(|_| ErreurEchange::Abime)?;
        let compresse = donnees.get(debut..fin).ok_or(ErreurEchange::Abime)?;
        let mut decompresse = Vec::new();
        flate2::read::DeflateDecoder::new(compresse)
            .take(f.taille.saturating_add(1))
            .read_to_end(&mut decompresse)
            .map_err(|_| ErreurEchange::Abime)?;
        if decompresse.len() as u64 != f.taille || empreinte(&decompresse) != f.empreinte {
            return Err(ErreurEchange::Abime);
        }
        if fichiers.insert(f.nom.clone(), decompresse).is_some() {
            return Err(ErreurEchange::Abime);
        }
    }
    if fin_attendue != donnees.len() as u64 {
        return Err(ErreurEchange::Abime);
    }
    verifier(&manifeste, &fichiers)?;
    Ok(EchangeLu {
        manifeste,
        fichiers,
    })
}

/// Chaque leçon a une clé unique et bornée, un titre, un texte UTF-8 de taille permise ; chaque
/// fichier rangé par empreinte porte un nom qui est son empreinte.
fn verifier(
    manifeste: &ManifesteEchange,
    fichiers: &BTreeMap<String, Vec<u8>>,
) -> Result<(), ErreurEchange> {
    if manifeste.lecons.len() > LECONS_ECHANGE_MAX {
        return Err(ErreurEchange::Abime);
    }
    let mut cles = BTreeSet::new();
    for l in &manifeste.lecons {
        let titre = |t: &str| !t.trim().is_empty() && t.trim().chars().count() <= 200;
        let code = |c: &str| !c.trim().is_empty() && c.trim().chars().count() <= 20;
        let texte = fichiers
            .get(&l.texte)
            .and_then(|o| std::str::from_utf8(o).ok());
        if l.cle.trim().is_empty()
            || l.cle.chars().count() > CLE_MAX
            || !cles.insert(l.cle.as_str())
            || !titre(&l.titre)
            || l.presentation.chars().count() > 500
            || !titre(&l.chapitre.titre)
            || l.chapitre.partie.chars().count() > 100
            || l.chapitre.cle.chars().count() > CLE_MAX
            || !code(&l.chapitre.niveau)
            || !code(&l.chapitre.discipline)
            || l.chapitre.discipline_libelle.chars().count() > 200
            || l.session.is_some_and(|s| !(1950..=2200).contains(&s))
            || texte.is_none_or(|t| t.len() > TEXTE_MAX)
            || l.joints
                .iter()
                .any(|j| !fichiers.contains_key(&nom_fichier_echange(&j.empreinte)))
        {
            return Err(ErreurEchange::Abime);
        }
    }
    for nom in fichiers.keys() {
        if let Some(e) = nom.strip_prefix("fichiers/")
            && !est_empreinte(e)
        {
            return Err(ErreurEchange::Abime);
        }
    }
    Ok(())
}

/// Fabrique un fichier d'échange. Les fichiers sont rangés dans l'ordre donné ; le champ
/// `fichiers` du manifeste est rempli ici.
pub fn fabriquer_echange(
    manifeste: &ManifesteEchange,
    fichiers: &[(String, Vec<u8>)],
) -> Result<Vec<u8>, ErreurEchange> {
    let erreur = |e: &dyn std::fmt::Display| ErreurEchange::Fabrication(e.to_string());
    let mut manifeste = manifeste.clone();
    manifeste.fichiers.clear();
    let mut donnees = Vec::new();
    for (nom, octets) in fichiers {
        let mut compresseur =
            flate2::write::DeflateEncoder::new(Vec::new(), flate2::Compression::default());
        compresseur.write_all(octets).map_err(|e| erreur(&e))?;
        let compresse = compresseur.finish().map_err(|e| erreur(&e))?;
        manifeste.fichiers.push(FichierPaquet {
            nom: nom.clone(),
            empreinte: empreinte(octets),
            taille: octets.len() as u64,
            debut: donnees.len() as u64,
            longueur: compresse.len() as u64,
        });
        donnees.extend_from_slice(&compresse);
    }
    let presents: BTreeMap<String, Vec<u8>> = fichiers.iter().cloned().collect();
    if presents.len() != fichiers.len() {
        return Err(ErreurEchange::Fabrication(
            "deux fichiers portent le même nom".into(),
        ));
    }
    verifier(&manifeste, &presents)
        .map_err(|_| ErreurEchange::Fabrication("leçon incomplète".into()))?;
    let texte = serde_json::to_vec(&manifeste).map_err(|e| erreur(&e))?;
    let longueur = u32::try_from(texte.len()).map_err(|e| erreur(&e))?;
    let mut reste = Vec::with_capacity(4 + texte.len() + donnees.len());
    reste.extend_from_slice(&longueur.to_le_bytes());
    reste.extend_from_slice(&texte);
    reste.extend_from_slice(&donnees);
    let mut fichier = Vec::with_capacity(38 + reste.len());
    fichier.extend_from_slice(MARQUE);
    fichier.extend_from_slice(&FORMAT_ECHANGE.to_le_bytes());
    fichier.extend_from_slice(&hex::decode(empreinte(&reste)).map_err(|e| erreur(&e))?);
    fichier.extend_from_slice(&reste);
    Ok(fichier)
}

/// Un fichier reçu, prêt à ranger : sa sorte et ses octets (nettoyés pour un SVG).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FichierRecu {
    /// Sorte reconnue par les premiers octets.
    pub sorte: SorteFichier,
    /// Octets à ranger.
    pub octets: Vec<u8>,
}

/// Reconnaît un fichier reçu par ses premiers octets, nettoie un SVG et applique les bornes du
/// lot 1 : image 5 Mo, son 10 Mo, PDF 20 Mo. `None` pour un fichier que la bibliothèque ne
/// prend pas (une vidéo, un SVG qui ne se nettoie pas, un fichier trop lourd) : la leçon arrive
/// sans lui.
pub fn preparer_fichier_recu(octets: &[u8]) -> Option<FichierRecu> {
    let sorte = reconnaitre(octets)?;
    let borne = match sorte {
        SorteFichier::Svg | SorteFichier::Png | SorteFichier::Jpeg => TAILLE_IMAGE_MAX,
        SorteFichier::Pdf => TAILLE_DOCUMENT_MAX,
        SorteFichier::Mp3 | SorteFichier::M4a | SorteFichier::Ogg | SorteFichier::Wav => {
            TAILLE_SON_MAX
        }
        SorteFichier::Mp4 => return None,
    };
    let octets = if sorte == SorteFichier::Svg {
        nettoyer_svg(octets).ok()?.into_bytes()
    } else {
        octets.to_vec()
    };
    (octets.len() as u64 <= borne).then_some(FichierRecu { sorte, octets })
}

/// Remplace dans un texte les appels d'images et de sons d'une empreinte par une autre (un SVG
/// nettoyé à l'arrivée change d'empreinte).
pub fn remplacer_empreintes(texte: &str, remplacements: &BTreeMap<String, String>) -> String {
    let mut sortie = texte.to_owned();
    for (avant, apres) in remplacements {
        if avant != apres {
            for prefixe in ["image:", "son:"] {
                sortie = sortie.replace(&format!("{prefixe}{avant}"), &format!("{prefixe}{apres}"));
            }
        }
    }
    sortie
}

/// Clé sous laquelle une leçon reçue se range quand l'école ne la connaît pas : la clé de
/// l'école telle quelle, toute autre clé (une leçon livrée d'un paquet que l'école n'a pas, une
/// clé fabriquée) sous `ecole/recu/`, pour qu'aucune mise à jour du paquet ne la prenne pour une
/// leçon livrée. Un second import la retrouve.
pub fn cle_recue(cle: &str) -> String {
    if cle.starts_with("ecole/") {
        cle.to_owned()
    } else {
        format!("ecole/recu/{cle}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn manifeste() -> ManifesteEchange {
        ManifesteEchange {
            date: "2026-11-16".to_owned(),
            gesco: "2.0.0".to_owned(),
            lecons: vec![LeconEchange {
                cle: "ecole/01JLECON0000000000000000AB".to_owned(),
                livre: false,
                nature: NatureContenu::Lecon,
                titre: "Les fractions".to_owned(),
                presentation: String::new(),
                examen: None,
                serie: None,
                session: None,
                lie: None,
                chapitre: ChapitreEchange {
                    cle: "ecole/01JCHAP00000000000000000AB".to_owned(),
                    livre: false,
                    niveau: "6".to_owned(),
                    discipline: "MATH".to_owned(),
                    discipline_libelle: "Mathématiques".to_owned(),
                    series: Vec::new(),
                    partie: String::new(),
                    titre: "Nombres".to_owned(),
                },
                texte: nom_texte_echange(0),
                joints: Vec::new(),
            }],
            noms: BTreeMap::new(),
            fichiers: Vec::new(),
        }
    }

    fn fichiers() -> Vec<(String, Vec<u8>)> {
        let svg = b"<svg xmlns=\"http://www.w3.org/2000/svg\"><circle r=\"2\"/></svg>".to_vec();
        vec![
            (
                nom_texte_echange(0),
                "## L'essentiel\n\nUne fraction.\n".repeat(30).into_bytes(),
            ),
            (nom_fichier_echange(&empreinte(&svg)), svg),
        ]
    }

    #[test]
    fn un_fichier_fabrique_se_relit() {
        let octets = fabriquer_echange(&manifeste(), &fichiers()).unwrap_or_default();
        let lu = match lire_echange(&octets) {
            Ok(lu) => lu,
            Err(e) => unreachable!("fichier refusé : {e}"),
        };
        assert_eq!(lu.manifeste.lecons.len(), 1);
        assert!(
            lu.manifeste
                .lecons
                .first()
                .is_some_and(|l| lu.texte(l).contains("fraction"))
        );
        assert_eq!(lu.manifeste.fichiers.len(), 2);
    }

    #[test]
    fn un_octet_change_n_importe_ou_fait_refuser_le_fichier() {
        let octets = fabriquer_echange(&manifeste(), &fichiers()).unwrap_or_default();
        for position in 0..octets.len() {
            let mut abime = octets.clone();
            if let Some(o) = abime.get_mut(position) {
                *o ^= 0x01;
            }
            assert!(
                lire_echange(&abime).is_err(),
                "octet {position} changé, fichier accepté"
            );
        }
        let mut allonge = octets.clone();
        allonge.push(0);
        assert_eq!(lire_echange(&allonge), Err(ErreurEchange::Abime));
        let tronque = octets.get(..octets.len() - 1).unwrap_or_default();
        assert_eq!(lire_echange(tronque), Err(ErreurEchange::Abime));
        assert_eq!(lire_echange(b"GBIB"), Err(ErreurEchange::Abime));
    }

    #[test]
    fn un_texte_sans_fichier_empeche_la_fabrication() {
        let sans_texte: Vec<_> = fichiers().into_iter().skip(1).collect();
        assert!(matches!(
            fabriquer_echange(&manifeste(), &sans_texte),
            Err(ErreurEchange::Fabrication(_))
        ));
    }

    #[test]
    fn deux_lecons_de_meme_cle_font_refuser_le_fichier() {
        let mut m = manifeste();
        if let Some(l) = m.lecons.first().cloned() {
            m.lecons.push(l);
        }
        assert!(fabriquer_echange(&m, &fichiers()).is_err());
    }

    #[test]
    fn un_svg_recu_est_nettoye_et_une_video_refusee() {
        let svg = b"<svg xmlns=\"http://www.w3.org/2000/svg\"><script>alert(1)</script>\
                    <circle r=\"2\" onclick=\"x()\"/></svg>";
        let recu = preparer_fichier_recu(svg);
        assert!(recu.as_ref().is_some_and(|r| r.sorte == SorteFichier::Svg));
        let texte = recu
            .map(|r| String::from_utf8_lossy(&r.octets).into_owned())
            .unwrap_or_default();
        assert!(!texte.contains("script") && !texte.contains("onclick"));
        assert!(texte.contains("circle"));
        let mut mp4 = vec![0, 0, 0, 0x18];
        mp4.extend_from_slice(b"ftypisom");
        assert_eq!(preparer_fichier_recu(&mp4), None);
        assert_eq!(preparer_fichier_recu(b"ni image ni son"), None);
        let mut png = b"\x89PNG\r\n\x1a\n".to_vec();
        png.resize(usize::try_from(TAILLE_IMAGE_MAX).unwrap_or(0) + 1, 0);
        assert_eq!(preparer_fichier_recu(&png), None);
    }

    #[test]
    fn les_appels_suivent_l_empreinte_nouvelle() {
        let a = "a".repeat(64);
        let b = "b".repeat(64);
        let texte = format!("![Figure](image:{a}) et ![Voix](son:{a}) et {a}");
        let remplace = remplacer_empreintes(&texte, &BTreeMap::from([(a.clone(), b.clone())]));
        assert_eq!(
            remplace,
            format!("![Figure](image:{b}) et ![Voix](son:{b}) et {a}")
        );
    }

    #[test]
    fn une_cle_inconnue_se_range_sous_l_ecole() {
        assert_eq!(cle_recue("ecole/01ABC"), "ecole/01ABC");
        assert_eq!(cle_recue("9-math-racine"), "ecole/recu/9-math-racine");
    }
}
