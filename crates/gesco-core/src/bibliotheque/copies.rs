//! Photos des copies rendues par le portail (SPEC_PHASE_12_LOT_5 §4 ; ADR « copies
//! photographiées »). La page du téléphone réduit chaque photo à 1 600 pixels au plus avant
//! l'envoi ; le noyau la reconnaît par ses premiers octets, borne sa taille et retire toute
//! métadonnée. Règles pures, sans décodage de l'image : du JPEG, seuls les segments qui font
//! l'image sont gardés (les segments d'application — EXIF avec la position GPS, XMP, vignettes —
//! et les commentaires partent, comme tout ce qui suit la fin de l'image) ; du PNG, seuls les
//! morceaux qui font l'image (les textes, l'EXIF, la date partent).

/// Photos d'un rendu, au plus.
pub const PHOTOS_MAX: usize = 5;

/// Taille d'une photo, au plus, une fois réduite par le téléphone : 2 Mo.
pub const TAILLE_PHOTO_MAX: usize = 2 * 1024 * 1024;

/// Longueur du texte d'un rendu, au plus (caractères).
pub const TEXTE_RENDU_MAX: usize = 20_000;

/// Plus grand côté d'une photo réduite par la page, en pixels.
pub const COTE_PHOTO_MAX: u32 = 1_600;

/// Sorte d'une photo de copie.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SortePhoto {
    /// JPEG (ce que fait la page).
    Jpeg,
    /// PNG (une capture d'écran).
    Png,
}

impl SortePhoto {
    /// Nom gardé en base.
    pub const fn nom(self) -> &'static str {
        match self {
            Self::Jpeg => "jpeg",
            Self::Png => "png",
        }
    }

    /// Depuis le nom gardé en base.
    pub fn depuis_nom(nom: &str) -> Self {
        if nom == "png" { Self::Png } else { Self::Jpeg }
    }

    /// Type MIME.
    pub const fn type_mime(self) -> &'static str {
        match self {
            Self::Jpeg => "image/jpeg",
            Self::Png => "image/png",
        }
    }
}

/// Pourquoi une photo est refusée.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PhotoRefusee {
    /// Ni JPEG ni PNG.
    PasUnePhoto,
    /// Plus de 2 Mo.
    TropGrosse,
    /// Reconnue, mais abîmée (coupée en route, mal formée).
    Illisible,
}

const SIGNATURE_PNG: [u8; 8] = [0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A];

/// Reconnaît une photo par ses premiers octets.
pub fn reconnaitre_photo(octets: &[u8]) -> Option<SortePhoto> {
    if octets.starts_with(&[0xFF, 0xD8, 0xFF]) {
        Some(SortePhoto::Jpeg)
    } else if octets.starts_with(&SIGNATURE_PNG) {
        Some(SortePhoto::Png)
    } else {
        None
    }
}

/// Vérifie une photo et la rend sans métadonnée.
pub fn nettoyer_photo(octets: &[u8]) -> Result<(SortePhoto, Vec<u8>), PhotoRefusee> {
    let sorte = reconnaitre_photo(octets).ok_or(PhotoRefusee::PasUnePhoto)?;
    if octets.len() > TAILLE_PHOTO_MAX {
        return Err(PhotoRefusee::TropGrosse);
    }
    let propre = match sorte {
        SortePhoto::Jpeg => nettoyer_jpeg(octets),
        SortePhoto::Png => nettoyer_png(octets),
    }
    .ok_or(PhotoRefusee::Illisible)?;
    Ok((sorte, propre))
}

/// Segments d'application (APP0 à APP15) et commentaire : ce qui porte les métadonnées.
const fn segment_de_metadonnee(marqueur: u8) -> bool {
    matches!(marqueur, 0xE0..=0xEF | 0xFE)
}

/// Le JPEG avec les seuls segments de l'image ; `None` s'il est mal formé ou sans image.
pub fn nettoyer_jpeg(o: &[u8]) -> Option<Vec<u8>> {
    if !o.starts_with(&[0xFF, 0xD8]) {
        return None;
    }
    let mut sortie = Vec::with_capacity(o.len());
    sortie.extend_from_slice(&[0xFF, 0xD8]);
    let mut i = 2usize;
    let mut cadre = false;
    let mut balayage = false;
    loop {
        if *o.get(i)? != 0xFF {
            return None;
        }
        // Octets de remplissage avant un marqueur.
        while *o.get(i)? == 0xFF {
            i = i.checked_add(1)?;
        }
        let marqueur = *o.get(i)?;
        i = i.checked_add(1)?;
        match marqueur {
            // Fin de l'image : ce qui suit (bandes ajoutées par certains téléphones) part aussi.
            0xD9 => {
                if !(cadre && balayage) {
                    return None;
                }
                sortie.extend_from_slice(&[0xFF, 0xD9]);
                return Some(sortie);
            }
            0x01 | 0xD0..=0xD7 => {
                sortie.extend_from_slice(&[0xFF, marqueur]);
                continue;
            }
            0x00 | 0xD8 => return None,
            _ => {}
        }
        let longueur = usize::from(u16::from_be_bytes([*o.get(i)?, *o.get(i.checked_add(1)?)?]));
        if longueur < 2 {
            return None;
        }
        let fin = i.checked_add(longueur)?;
        let segment = o.get(i..fin)?;
        if matches!(marqueur, 0xC0..=0xCF) && !matches!(marqueur, 0xC4 | 0xC8 | 0xCC) {
            cadre = true;
        }
        if !segment_de_metadonnee(marqueur) {
            sortie.extend_from_slice(&[0xFF, marqueur]);
            sortie.extend_from_slice(segment);
        }
        i = fin;
        if marqueur == 0xDA {
            balayage = true;
            // Données de l'image jusqu'au marqueur suivant (ni octet bourré, ni reprise).
            let debut = i;
            loop {
                if *o.get(i)? == 0xFF {
                    let suivant = *o.get(i.checked_add(1)?)?;
                    if suivant == 0x00 || (0xD0..=0xD7).contains(&suivant) {
                        i = i.checked_add(2)?;
                        continue;
                    }
                    if suivant == 0xFF {
                        i = i.checked_add(1)?;
                        continue;
                    }
                    break;
                }
                i = i.checked_add(1)?;
            }
            sortie.extend_from_slice(o.get(debut..i)?);
        }
    }
}

/// Morceaux PNG gardés : ceux qui font l'image et ses couleurs ; textes, EXIF et date partent.
const MORCEAUX_PNG_GARDES: &[&[u8; 4]] = &[
    b"IHDR", b"PLTE", b"IDAT", b"IEND", b"tRNS", b"gAMA", b"cHRM", b"sRGB", b"iCCP", b"sBIT",
    b"bKGD", b"pHYs",
];

/// Le PNG avec les seuls morceaux de l'image ; `None` s'il est mal formé.
pub fn nettoyer_png(o: &[u8]) -> Option<Vec<u8>> {
    if !o.starts_with(&SIGNATURE_PNG) {
        return None;
    }
    let mut sortie = Vec::with_capacity(o.len());
    sortie.extend_from_slice(&SIGNATURE_PNG);
    let mut i = SIGNATURE_PNG.len();
    let mut premier = true;
    loop {
        let longueur = u32::from_be_bytes(o.get(i..i.checked_add(4)?)?.try_into().ok()?);
        let longueur = usize::try_from(longueur).ok()?;
        let nom: [u8; 4] = o
            .get(i.checked_add(4)?..i.checked_add(8)?)?
            .try_into()
            .ok()?;
        // Longueur, nom, données, contrôle.
        let fin = i.checked_add(12)?.checked_add(longueur)?;
        let morceau = o.get(i..fin)?;
        if premier && &nom != b"IHDR" {
            return None;
        }
        premier = false;
        if MORCEAUX_PNG_GARDES.iter().any(|g| **g == nom) {
            sortie.extend_from_slice(morceau);
        }
        i = fin;
        if &nom == b"IEND" {
            return Some(sortie);
        }
    }
}

#[cfg(test)]
#[allow(clippy::indexing_slicing)]
mod tests {
    use super::*;

    fn segment(marqueur: u8, donnees: &[u8]) -> Vec<u8> {
        let mut s = vec![0xFF, marqueur];
        let longueur = u16::try_from(donnees.len() + 2).unwrap_or(u16::MAX);
        s.extend_from_slice(&longueur.to_be_bytes());
        s.extend_from_slice(donnees);
        s
    }

    /// Un JPEG de forme juste (non décodable, mais chaque segment est à sa place), avec un EXIF
    /// qui porte une position GPS, un XMP, un commentaire et une bande après la fin.
    fn jpeg_avec_gps() -> Vec<u8> {
        let mut o = vec![0xFF, 0xD8];
        o.extend(segment(0xE0, b"JFIF\0\x01\x01\0\0\x01\0\x01\0\0"));
        o.extend(segment(
            0xE1,
            b"Exif\0\0MM\0*GPSLatitude 12.6392 N GPSLongitude 8.0029 W",
        ));
        o.extend(segment(0xE1, b"http://ns.adobe.com/xap/1.0/\0<x:xmpmeta/>"));
        o.extend(segment(0xFE, b"Pris au quartier"));
        o.extend(segment(0xDB, &[0u8; 65]));
        o.extend(segment(0xC0, &[8, 0, 16, 0, 16, 1, 1, 0x11, 0]));
        o.extend(segment(0xC4, &[0u8; 20]));
        o.extend(segment(0xDA, &[1, 1, 0, 0, 63, 0]));
        // Données de l'image, avec un octet bourré et une reprise.
        o.extend_from_slice(&[0x12, 0x34, 0xFF, 0x00, 0x56, 0xFF, 0xD0, 0x78]);
        o.extend_from_slice(&[0xFF, 0xD9]);
        o.extend_from_slice(b"SEFT GPS trailer");
        o
    }

    fn contient(foin: &[u8], aiguille: &[u8]) -> bool {
        foin.windows(aiguille.len()).any(|w| w == aiguille)
    }

    #[test]
    fn une_photo_avec_position_gps_ressort_sans_elle() {
        let photo = jpeg_avec_gps();
        assert!(contient(&photo, b"GPSLatitude"));
        let (sorte, propre) = nettoyer_photo(&photo).unwrap_or((SortePhoto::Png, Vec::new()));
        assert_eq!(sorte, SortePhoto::Jpeg);
        for absent in [
            &b"GPS"[..],
            b"Exif",
            b"xmpmeta",
            b"quartier",
            b"JFIF",
            b"trailer",
        ] {
            assert!(
                !contient(&propre, absent),
                "{}",
                String::from_utf8_lossy(absent)
            );
        }
        // L'image est entière : tables, cadre, balayage et données, fin.
        assert!(propre.starts_with(&[0xFF, 0xD8, 0xFF, 0xDB]));
        assert!(contient(
            &propre,
            &[0x12, 0x34, 0xFF, 0x00, 0x56, 0xFF, 0xD0, 0x78, 0xFF, 0xD9]
        ));
        assert!(propre.ends_with(&[0xFF, 0xD9]));
        // Nettoyer deux fois ne change plus rien.
        assert_eq!(nettoyer_jpeg(&propre).as_deref(), Some(propre.as_slice()));
    }

    #[test]
    fn un_fichier_qui_n_est_pas_une_photo_est_refuse() {
        assert_eq!(
            nettoyer_photo(b"%PDF-1.7 ...").err(),
            Some(PhotoRefusee::PasUnePhoto)
        );
        assert_eq!(
            nettoyer_photo(b"GIF89a....").err(),
            Some(PhotoRefusee::PasUnePhoto)
        );
        assert_eq!(nettoyer_photo(&[]).err(), Some(PhotoRefusee::PasUnePhoto));
        // Coupé en route : sans fin d'image.
        let photo = jpeg_avec_gps();
        let coupe = &photo[..photo.len() - 30];
        assert_eq!(nettoyer_photo(coupe).err(), Some(PhotoRefusee::Illisible));
        // Trop gros.
        let mut gros = jpeg_avec_gps();
        gros.resize(TAILLE_PHOTO_MAX + 1, 0);
        assert_eq!(nettoyer_photo(&gros).err(), Some(PhotoRefusee::TropGrosse));
    }

    fn morceau_png(nom: &[u8; 4], donnees: &[u8]) -> Vec<u8> {
        let mut m = u32::try_from(donnees.len())
            .unwrap_or(0)
            .to_be_bytes()
            .to_vec();
        m.extend_from_slice(nom);
        m.extend_from_slice(donnees);
        m.extend_from_slice(&[0, 0, 0, 0]);
        m
    }

    #[test]
    fn un_png_perd_ses_textes_et_garde_son_image() {
        let mut o = SIGNATURE_PNG.to_vec();
        o.extend(morceau_png(
            b"IHDR",
            &[0, 0, 0, 1, 0, 0, 0, 1, 8, 2, 0, 0, 0],
        ));
        o.extend(morceau_png(b"tEXt", b"Comment\0Pris au quartier"));
        o.extend(morceau_png(b"eXIf", b"MM\0*GPS"));
        o.extend(morceau_png(b"IDAT", &[1, 2, 3]));
        o.extend(morceau_png(b"IEND", &[]));
        let (sorte, propre) = nettoyer_photo(&o).unwrap_or((SortePhoto::Jpeg, Vec::new()));
        assert_eq!(sorte, SortePhoto::Png);
        assert!(!contient(&propre, b"quartier") && !contient(&propre, b"GPS"));
        assert!(
            contient(&propre, b"IDAT") && propre.ends_with(&[b'I', b'E', b'N', b'D', 0, 0, 0, 0])
        );
        // Sans IHDR en tête : refusé.
        let mut faux = SIGNATURE_PNG.to_vec();
        faux.extend(morceau_png(b"IDAT", &[1]));
        faux.extend(morceau_png(b"IEND", &[]));
        assert_eq!(nettoyer_photo(&faux).err(), Some(PhotoRefusee::Illisible));
    }
}
