//! Fichiers que l'école joint à la bibliothèque (§10 de la spécification ; ADR « stockage des
//! contenus ») : images (SVG, PNG, JPEG ; 5 Mo au plus) et documents (PDF ; 20 Mo au plus) dans
//! la base, vidéos (MP4) dans un dossier du poste principal. Règles pures : reconnaître un
//! fichier par ses premiers octets, borner sa taille, nettoyer un SVG.
//!
//! Le nettoyage d'un SVG est écrit ici, sans dépendance : il retire les scripts, les objets
//! étrangers, les attributs d'événement et tout appel vers l'extérieur (lien, feuille de style,
//! `url(…)` hors du fichier). Ce qu'il ne sait pas lire, il le refuse : une image refusée vaut
//! mieux qu'une image qui appelle autre chose qu'elle-même.

/// Taille la plus grande d'une image : 5 Mo.
pub const TAILLE_IMAGE_MAX: u64 = 5 * 1024 * 1024;

/// Taille la plus grande d'un document : 20 Mo (la limite du réseau local).
pub const TAILLE_DOCUMENT_MAX: u64 = 20 * 1024 * 1024;

/// Taille d'un morceau de vidéo qui voyage entre les postes.
pub const MORCEAU_VIDEO: u64 = 4 * 1024 * 1024;

/// Sorte d'un fichier reconnue par ses premiers octets.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SorteFichier {
    /// Image SVG.
    Svg,
    /// Image PNG.
    Png,
    /// Image JPEG.
    Jpeg,
    /// Document PDF.
    Pdf,
    /// Vidéo MP4.
    Mp4,
}

impl SorteFichier {
    /// Type MIME.
    pub const fn type_mime(self) -> &'static str {
        match self {
            Self::Svg => "image/svg+xml",
            Self::Png => "image/png",
            Self::Jpeg => "image/jpeg",
            Self::Pdf => "application/pdf",
            Self::Mp4 => "video/mp4",
        }
    }

    /// Extension habituelle.
    pub const fn extension(self) -> &'static str {
        match self {
            Self::Svg => "svg",
            Self::Png => "png",
            Self::Jpeg => "jpg",
            Self::Pdf => "pdf",
            Self::Mp4 => "mp4",
        }
    }

    /// Vrai pour une image.
    pub const fn est_image(self) -> bool {
        matches!(self, Self::Svg | Self::Png | Self::Jpeg)
    }
}

/// Reconnaît un fichier par ses premiers octets ; `None` pour une sorte que la bibliothèque ne
/// prend pas.
pub fn reconnaitre(octets: &[u8]) -> Option<SorteFichier> {
    if octets.starts_with(b"\x89PNG\r\n\x1a\n") {
        return Some(SorteFichier::Png);
    }
    if octets.starts_with(&[0xFF, 0xD8, 0xFF]) {
        return Some(SorteFichier::Jpeg);
    }
    if octets.starts_with(b"%PDF-") {
        return Some(SorteFichier::Pdf);
    }
    if octets.get(4..8) == Some(b"ftyp".as_slice()) {
        return Some(SorteFichier::Mp4);
    }
    let debut = octets.get(..octets.len().min(4096))?;
    let texte = String::from_utf8_lossy(debut).to_ascii_lowercase();
    texte.contains("<svg").then_some(SorteFichier::Svg)
}

/// Pourquoi un SVG ne se nettoie pas.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum SvgRefuse {
    /// Le texte n'est pas de l'UTF-8.
    #[error("le SVG n'est pas un texte UTF-8")]
    PasDuTexte,
    /// Une balise mal formée.
    #[error("le SVG est mal formé")]
    MalForme,
    /// Une déclaration d'entités ou un type de document avec définitions.
    #[error("le SVG déclare des entités")]
    Entites,
    /// Une feuille de style qui appelle l'extérieur.
    #[error("le SVG appelle un fichier extérieur")]
    AppelExterieur,
    /// Aucun élément `svg`.
    #[error("le fichier n'a pas d'élément svg")]
    SansSvg,
}

/// Éléments retirés avec tout ce qu'ils contiennent.
const ELEMENTS_RETIRES: [&str; 8] = [
    "script",
    "foreignobject",
    "iframe",
    "embed",
    "object",
    "handler",
    "listener",
    "audio",
];

/// Nom local d'un élément ou d'un attribut, en minuscules (`xlink:href` → `href`).
fn nom_local(nom: &str) -> String {
    nom.rsplit(':').next().unwrap_or(nom).to_ascii_lowercase()
}

/// Vrai si une valeur appelle autre chose que le fichier lui-même : `url(…)` hors d'un
/// fragment `#…`, `javascript:`, `@import`.
fn appelle_l_exterieur(valeur: &str) -> bool {
    let bas = valeur.to_ascii_lowercase();
    if bas.contains("javascript:") || bas.contains("@import") || bas.contains("expression(") {
        return true;
    }
    let mut reste = bas.as_str();
    while let Some(i) = reste.find("url(") {
        let apres = reste.get(i + 4..).unwrap_or_default();
        let cible = apres.trim_start().trim_start_matches(['"', '\'']);
        if !cible.starts_with('#') {
            return true;
        }
        reste = apres;
    }
    false
}

/// Vrai si un lien d'image ou de renvoi reste dans le fichier : fragment, ou image PNG ou JPEG
/// écrite dans le lien même.
fn lien_interne(valeur: &str) -> bool {
    let v = valeur.trim().to_ascii_lowercase();
    v.starts_with('#') || v.starts_with("data:image/png;") || v.starts_with("data:image/jpeg;")
}

/// Une balise lue : nom, attributs, fermante, vide.
struct Balise {
    nom: String,
    attributs: Vec<(String, String)>,
    fermante: bool,
    vide: bool,
}

/// Lit une balise à partir de `<` ; rend la balise et l'indice qui suit `>`.
fn lire_balise(texte: &str, debut: usize) -> Result<(Balise, usize), SvgRefuse> {
    let octets = texte.as_bytes();
    let mut i = debut + 1;
    let fermante = octets.get(i) == Some(&b'/');
    if fermante {
        i += 1;
    }
    let nom_debut = i;
    while octets
        .get(i)
        .is_some_and(|o| !o.is_ascii_whitespace() && *o != b'>' && *o != b'/')
    {
        i += 1;
    }
    let nom = texte
        .get(nom_debut..i)
        .ok_or(SvgRefuse::MalForme)?
        .to_owned();
    if nom.is_empty()
        || !nom
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, ':' | '-' | '_' | '.'))
    {
        return Err(SvgRefuse::MalForme);
    }
    let mut attributs = Vec::new();
    loop {
        while octets.get(i).is_some_and(u8::is_ascii_whitespace) {
            i += 1;
        }
        match octets.get(i) {
            None => return Err(SvgRefuse::MalForme),
            Some(b'>') => {
                return Ok((
                    Balise {
                        nom,
                        attributs,
                        fermante,
                        vide: false,
                    },
                    i + 1,
                ));
            }
            Some(b'/') => {
                if octets.get(i + 1) != Some(&b'>') {
                    return Err(SvgRefuse::MalForme);
                }
                return Ok((
                    Balise {
                        nom,
                        attributs,
                        fermante,
                        vide: true,
                    },
                    i + 2,
                ));
            }
            Some(_) if fermante => return Err(SvgRefuse::MalForme),
            Some(_) => {}
        }
        let a_debut = i;
        while octets
            .get(i)
            .is_some_and(|o| !o.is_ascii_whitespace() && *o != b'=' && *o != b'>' && *o != b'/')
        {
            i += 1;
        }
        let a_nom = texte.get(a_debut..i).ok_or(SvgRefuse::MalForme)?.to_owned();
        if a_nom.is_empty()
            || !a_nom
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || matches!(c, ':' | '-' | '_' | '.'))
        {
            return Err(SvgRefuse::MalForme);
        }
        while octets.get(i).is_some_and(u8::is_ascii_whitespace) {
            i += 1;
        }
        if octets.get(i) != Some(&b'=') {
            return Err(SvgRefuse::MalForme);
        }
        i += 1;
        while octets.get(i).is_some_and(u8::is_ascii_whitespace) {
            i += 1;
        }
        let guillemet = match octets.get(i) {
            Some(g @ (b'"' | b'\'')) => *g,
            _ => return Err(SvgRefuse::MalForme),
        };
        i += 1;
        let v_debut = i;
        while octets.get(i).is_some_and(|o| *o != guillemet) {
            i += 1;
        }
        if octets.get(i).is_none() {
            return Err(SvgRefuse::MalForme);
        }
        let valeur = texte.get(v_debut..i).ok_or(SvgRefuse::MalForme)?.to_owned();
        if valeur.contains('<') {
            return Err(SvgRefuse::MalForme);
        }
        i += 1;
        attributs.push((a_nom, valeur));
    }
}

/// Écrit une balise gardée, ses attributs entre guillemets droits.
fn ecrire_balise(balise: &Balise, sortie: &mut String) {
    sortie.push('<');
    if balise.fermante {
        sortie.push('/');
    }
    sortie.push_str(&balise.nom);
    for (nom, valeur) in &balise.attributs {
        sortie.push(' ');
        sortie.push_str(nom);
        sortie.push_str("=\"");
        sortie.push_str(&valeur.replace('"', "&quot;"));
        sortie.push('"');
    }
    if balise.vide {
        sortie.push('/');
    }
    sortie.push('>');
}

/// Nettoie un SVG : retire scripts, objets étrangers, commentaires, instructions autres que la
/// déclaration XML, attributs d'événement et liens vers l'extérieur. Refuse un SVG mal formé,
/// qui déclare des entités, dont une feuille de style appelle l'extérieur, ou sans élément
/// `svg`.
pub fn nettoyer_svg(octets: &[u8]) -> Result<String, SvgRefuse> {
    let texte = std::str::from_utf8(octets).map_err(|_| SvgRefuse::PasDuTexte)?;
    let texte = texte.trim_start_matches('\u{feff}');
    let mut sortie = String::with_capacity(texte.len());
    let mut i = 0;
    // Profondeur dans un élément retiré : (nom, profondeur des éléments de même nom).
    let mut retire: Option<(String, usize)> = None;
    // Vrai dans un élément `style`.
    let mut dans_style = false;
    let mut a_un_svg = false;
    while i < texte.len() {
        let reste = texte.get(i..).ok_or(SvgRefuse::MalForme)?;
        if !reste.starts_with('<') {
            let fin = reste.find('<').map_or(texte.len(), |p| i + p);
            let morceau = texte.get(i..fin).ok_or(SvgRefuse::MalForme)?;
            if retire.is_none() {
                if dans_style && appelle_l_exterieur(morceau) {
                    return Err(SvgRefuse::AppelExterieur);
                }
                sortie.push_str(morceau);
            }
            i = fin;
            continue;
        }
        if reste.starts_with("<!--") {
            let fin = reste.find("-->").ok_or(SvgRefuse::MalForme)?;
            i += fin + 3;
            continue;
        }
        if reste.starts_with("<![CDATA[") {
            let fin = reste.find("]]>").ok_or(SvgRefuse::MalForme)?;
            let morceau = reste.get(..fin + 3).ok_or(SvgRefuse::MalForme)?;
            if retire.is_none() {
                if appelle_l_exterieur(morceau) {
                    return Err(SvgRefuse::AppelExterieur);
                }
                sortie.push_str(morceau);
            }
            i += fin + 3;
            continue;
        }
        if reste.starts_with("<!") {
            let fin = reste.find('>').ok_or(SvgRefuse::MalForme)?;
            let declaration = reste.get(..fin).ok_or(SvgRefuse::MalForme)?;
            if declaration.contains('[') || declaration.to_ascii_uppercase().contains("ENTITY") {
                return Err(SvgRefuse::Entites);
            }
            i += fin + 1;
            continue;
        }
        if reste.starts_with("<?") {
            let fin = reste.find("?>").ok_or(SvgRefuse::MalForme)?;
            let instruction = reste.get(..fin + 2).ok_or(SvgRefuse::MalForme)?;
            if instruction.starts_with("<?xml ") && sortie.is_empty() {
                sortie.push_str(instruction);
            }
            i += fin + 2;
            continue;
        }
        let (mut balise, suite) = lire_balise(texte, i)?;
        i = suite;
        let local = nom_local(&balise.nom);
        if let Some((nom, profondeur)) = &mut retire {
            if local == *nom {
                if balise.fermante {
                    if *profondeur == 0 {
                        retire = None;
                    } else {
                        *profondeur -= 1;
                    }
                } else if !balise.vide {
                    *profondeur += 1;
                }
            }
            continue;
        }
        if ELEMENTS_RETIRES.contains(&local.as_str()) {
            if !balise.fermante && !balise.vide {
                retire = Some((local, 0));
            }
            continue;
        }
        if local == "svg" {
            a_un_svg = true;
        }
        if local == "style" {
            dans_style = !balise.fermante && !balise.vide;
        }
        balise.attributs.retain(|(nom, valeur)| {
            let a = nom_local(nom);
            if a.starts_with("on") {
                return false;
            }
            if a == "href" || a == "src" {
                return lien_interne(valeur);
            }
            !appelle_l_exterieur(valeur)
        });
        ecrire_balise(&balise, &mut sortie);
    }
    if retire.is_some() {
        return Err(SvgRefuse::MalForme);
    }
    if !a_un_svg {
        return Err(SvgRefuse::SansSvg);
    }
    Ok(sortie)
}

/// Vrai pour une empreinte SHA-256 en hexadécimal minuscule : le nom d'un fichier de vidéo.
pub fn est_empreinte(texte: &str) -> bool {
    texte.len() == 64
        && texte
            .bytes()
            .all(|o| o.is_ascii_digit() || (b'a'..=b'f').contains(&o))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chaque_sorte_se_reconnait_par_ses_premiers_octets() {
        assert_eq!(reconnaitre(b"\x89PNG\r\n\x1a\nxx"), Some(SorteFichier::Png));
        assert_eq!(
            reconnaitre(&[0xFF, 0xD8, 0xFF, 0xE0]),
            Some(SorteFichier::Jpeg)
        );
        assert_eq!(reconnaitre(b"%PDF-1.7"), Some(SorteFichier::Pdf));
        assert_eq!(reconnaitre(b"\0\0\0\x18ftypmp42"), Some(SorteFichier::Mp4));
        assert_eq!(
            reconnaitre(b"<?xml version=\"1.0\"?>\n<svg xmlns=\"http://www.w3.org/2000/svg\"/>"),
            Some(SorteFichier::Svg)
        );
        assert_eq!(reconnaitre(b"MZ\x90\0"), None);
        assert_eq!(reconnaitre(b""), None);
    }

    #[test]
    fn un_svg_propre_reste_le_meme() {
        let svg = "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"10\" height=\"10\"><circle cx=\"5\" cy=\"5\" r=\"4\" fill=\"url(#d)\"/><text x=\"1\" y=\"9\">a &lt; b</text></svg>";
        assert_eq!(nettoyer_svg(svg.as_bytes()).as_deref(), Ok(svg));
    }

    #[test]
    fn le_nettoyage_retire_scripts_evenements_et_liens_exterieurs() {
        let svg = "<?xml version=\"1.0\"?><!-- note --><?xml-stylesheet href=\"http://x/a.css\"?><svg xmlns=\"http://www.w3.org/2000/svg\" onload=\"alert(1)\"><script type=\"text/javascript\">alert(2)</script><foreignObject><div>html</div></foreignObject><a xlink:href=\"https://exemple.org\"><rect width=\"2\" height=\"2\" style=\"fill:url(http://x/y)\"/></a><use href=\"#r\"/><image href=\"data:image/png;base64,AAAA\"/></svg>";
        let propre = nettoyer_svg(svg.as_bytes()).unwrap_or_default();
        assert!(propre.starts_with("<?xml version=\"1.0\"?><svg"));
        for interdit in [
            "onload",
            "script",
            "alert",
            "foreignObject",
            "html",
            "exemple.org",
            "http://x",
            "stylesheet",
            "note",
        ] {
            assert!(!propre.contains(interdit), "{interdit} reste dans {propre}");
        }
        assert!(propre.contains("<use href=\"#r\"/>"));
        assert!(propre.contains("data:image/png;base64,AAAA"));
        assert!(propre.contains("<rect width=\"2\" height=\"2\"/>"));
    }

    #[test]
    fn un_svg_qui_ne_se_nettoie_pas_est_refuse() {
        assert_eq!(
            nettoyer_svg(b"<!DOCTYPE svg [<!ENTITY a \"b\">]><svg/>"),
            Err(SvgRefuse::Entites)
        );
        assert_eq!(
            nettoyer_svg(b"<svg><style>@import url(http://x/a.css);</style></svg>"),
            Err(SvgRefuse::AppelExterieur)
        );
        assert_eq!(
            nettoyer_svg(b"<svg><rect width=2/></svg>"),
            Err(SvgRefuse::MalForme)
        );
        assert_eq!(nettoyer_svg(b"<svg><script>"), Err(SvgRefuse::MalForme));
        assert_eq!(nettoyer_svg(b"<html></html>"), Err(SvgRefuse::SansSvg));
        assert_eq!(
            nettoyer_svg(&[0xFF, 0xFE, 0x00]),
            Err(SvgRefuse::PasDuTexte)
        );
    }

    #[test]
    fn un_script_imbrique_est_retire_en_entier() {
        let propre = nettoyer_svg(b"<svg><script><script>x</script>y</script><g/></svg>")
            .unwrap_or_default();
        assert_eq!(propre, "<svg><g/></svg>");
    }

    #[test]
    fn une_empreinte_est_en_hexadecimal_minuscule() {
        assert!(est_empreinte(&"a".repeat(64)));
        assert!(!est_empreinte(&"A".repeat(64)));
        assert!(!est_empreinte("../videos"));
    }
}
