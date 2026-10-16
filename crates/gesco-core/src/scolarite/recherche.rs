//! Recherche tolérante de personnes (SPEC_PHASE_2 §6, blueprint §3.5) : Keïta et Keita, Coulibaly
//! et Kouloubaly, Mohamed et Mahamadou se retrouvent. La recherche propose ; elle ne fusionne
//! jamais rien d'elle-même.

use crate::commun::DateCivile;

/// Forme de comparaison d'un texte : majuscules sans accents, ponctuation ramenée à des espaces,
/// espaces simples.
pub fn normaliser(texte: &str) -> String {
    let mut sortie = String::with_capacity(texte.len());
    for c in texte.chars() {
        match c {
            'à' | 'â' | 'ä' | 'á' | 'ã' | 'å' | 'À' | 'Â' | 'Ä' | 'Á' | 'Ã' | 'Å' => {
                sortie.push('A')
            }
            'ç' | 'Ç' => sortie.push('C'),
            'é' | 'è' | 'ê' | 'ë' | 'É' | 'È' | 'Ê' | 'Ë' => sortie.push('E'),
            'î' | 'ï' | 'í' | 'ì' | 'Î' | 'Ï' | 'Í' | 'Ì' => sortie.push('I'),
            'ô' | 'ö' | 'ó' | 'ò' | 'õ' | 'Ô' | 'Ö' | 'Ó' | 'Ò' | 'Õ' => sortie.push('O'),
            'ù' | 'û' | 'ü' | 'ú' | 'Ù' | 'Û' | 'Ü' | 'Ú' => sortie.push('U'),
            'ÿ' | 'ý' | 'Ÿ' | 'Ý' => sortie.push('Y'),
            'ñ' | 'Ñ' => sortie.push('N'),
            'œ' | 'Œ' => sortie.push_str("OE"),
            'æ' | 'Æ' => sortie.push_str("AE"),
            c if c.is_alphanumeric() => sortie.extend(c.to_uppercase()),
            _ => sortie.push(' '),
        }
    }
    sortie.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Clé phonétique d'un mot normalisé : `C`, `K` et `Q` se confondent (`C` devant `E`, `I`, `Y`
/// sonne `S`), `OU` et `U`, `PH` et `F`, `DJ` et `J`, `DI` devant une voyelle et `J` (Diallo,
/// Jallo), `CH` et `SH` et `S`, `Y` et `I`, `W` et `U`, `Z` et `S` ; le `H` est muet, les
/// lettres doublées simplifiées, le `E` final retiré.
pub fn cle_phonetique(mot: &str) -> String {
    let remplace = mot
        .replace("DJ", "J")
        .replace("DIA", "JA")
        .replace("DIE", "JE")
        .replace("DIO", "JO")
        .replace("DIU", "JU")
        .replace("OU", "U")
        .replace("PH", "F")
        .replace("QU", "K")
        .replace("CK", "K")
        .replace("CH", "S")
        .replace("SH", "S");
    let lettres: Vec<char> = remplace.chars().collect();
    let mut cle = String::with_capacity(lettres.len());
    for (i, c) in lettres.iter().enumerate() {
        let suivante = lettres.get(i + 1).copied();
        let son = match c {
            'C' if matches!(suivante, Some('E' | 'I' | 'Y')) => Some('S'),
            'C' | 'Q' => Some('K'),
            'Y' => Some('I'),
            'W' => Some('U'),
            'Z' => Some('S'),
            'H' => None,
            autre => Some(*autre),
        };
        if let Some(s) = son
            && !cle.ends_with(s)
        {
            cle.push(s);
        }
    }
    if cle.chars().count() > 3 && cle.ends_with('E') {
        cle.pop();
    }
    cle
}

/// Distance d'édition (Levenshtein) entre deux mots.
pub fn distance(a: &str, b: &str) -> usize {
    let b: Vec<char> = b.chars().collect();
    let mut precedente: Vec<usize> = (0..=b.len()).collect();
    for (i, ca) in a.chars().enumerate() {
        let mut courante = Vec::with_capacity(b.len() + 1);
        courante.push(i + 1);
        for (j, cb) in b.iter().enumerate() {
            let substitution = precedente.get(j).copied().unwrap_or(0) + usize::from(ca != *cb);
            let suppression = precedente.get(j + 1).copied().unwrap_or(0) + 1;
            let insertion = courante.get(j).copied().unwrap_or(0) + 1;
            courante.push(substitution.min(suppression).min(insertion));
        }
        precedente = courante;
    }
    precedente.last().copied().unwrap_or(0)
}

/// Prénoms équivalents livrés avec le produit (T-07) : chaque ligne est une classe.
const EQUIVALENCES: &[&[&str]] = &[
    &[
        "MOHAMED",
        "MOHAMMED",
        "MOHAMAD",
        "MOHAMADOU",
        "MAHAMADOU",
        "MAHAMANE",
        "MAMADOU",
    ],
    &["AMADOU", "AHMADOU", "HAMADOU", "AHMED"],
    &["FATOUMATA", "FATIMATA", "FATOUMA", "FATIMA"],
    &["OUMAR", "OMAR", "OUMAROU"],
    &["OUSMANE", "OUSMAN", "USMAN"],
    &["ABDOULAYE", "ABDOULAHI", "ABDALLAH"],
    &["MOUSSA", "MOUSA", "MUSA"],
    &["BOUBACAR", "ABOUBACAR", "BOCAR", "BAKARY"],
    &["SEYDOU", "SEIDOU", "SAIDOU"],
    &["ALI", "ALY"],
    &["IBRAHIM", "IBRAHIMA", "BRAHIMA"],
    &["AMINATA", "AMINATOU", "AMINA"],
    &["KADIATOU", "KADIDIA", "KHADIJA"],
    &["MARIAM", "MARIAME", "MARIAMA", "MARYAM"],
    &["AISSATA", "AISSATOU", "AICHA"],
    &["HAWA", "AWA"],
    &["SALIF", "SALIFOU"],
    &["YOUSSOUF", "YOUSOUF", "YUSUF"],
];

/// Vrai si deux prénoms normalisés appartiennent à une même classe d'équivalence.
pub fn equivalents(a: &str, b: &str) -> bool {
    EQUIVALENCES
        .iter()
        .any(|classe| classe.contains(&a) && classe.contains(&b))
}

/// Écart toléré entre deux clés phonétiques, selon la plus courte : un nom court ne tolère
/// aucune faute, sans quoi Awa approcherait Moussa.
fn seuil(longueur: usize) -> usize {
    match longueur {
        0..=3 => 0,
        4..=5 => 1,
        _ => 2,
    }
}

/// Un mot et sa clé phonétique, calculée une fois.
#[derive(Debug, Clone)]
struct Mot {
    texte: String,
    cle: String,
}

impl Mot {
    fn nouveau(texte: &str) -> Self {
        Self {
            texte: texte.to_owned(),
            cle: cle_phonetique(texte),
        }
    }
}

/// Proximité d'un mot cherché et d'un mot du nom, du plus sûr au plus lâche : 0 identique,
/// 1 début du mot, 2 équivalence ou même son, 3 orthographe proche. `None` : sans rapport.
fn proximite(cherche: &Mot, mot: &Mot) -> Option<u8> {
    if cherche.texte == mot.texte {
        return Some(0);
    }
    if mot.texte.starts_with(&cherche.texte) {
        return Some(1);
    }
    if cherche.cle == mot.cle || equivalents(&cherche.texte, &mot.texte) {
        return Some(2);
    }
    let courte = cherche.cle.chars().count().min(mot.cle.chars().count());
    (distance(&cherche.cle, &mot.cle) <= seuil(courte)).then_some(3)
}

/// Une recherche préparée : ses mots normalisés et leurs clés phonétiques.
#[derive(Debug, Clone)]
pub struct Recherche {
    mots: Vec<Mot>,
}

impl Recherche {
    /// Prépare la recherche d'un texte saisi.
    pub fn nouvelle(texte: &str) -> Self {
        Self {
            mots: normaliser(texte)
                .split(' ')
                .filter(|m| !m.is_empty())
                .map(Mot::nouveau)
                .collect(),
        }
    }

    /// Vrai si rien n'est cherché.
    pub fn est_vide(&self) -> bool {
        self.mots.is_empty()
    }

    /// Rang d'un nom : chaque mot cherché doit approcher un mot du nom. Plus petit est
    /// meilleur : la pire proximité, puis leur somme. `None` si un mot cherché ne trouve rien.
    pub fn rang(&self, nom: &str) -> Option<(u8, u32)> {
        let mots: Vec<Mot> = normaliser(nom)
            .split(' ')
            .filter(|m| !m.is_empty())
            .map(Mot::nouveau)
            .collect();
        let mut pire = 0;
        let mut somme = 0;
        for cherche in &self.mots {
            let meilleure = mots.iter().filter_map(|m| proximite(cherche, m)).min()?;
            pire = pire.max(meilleure);
            somme += u32::from(meilleure);
        }
        Some((pire, somme))
    }

    /// Vrai si chaque mot cherché commence un mot du nom : « tra aw » trouve « Traoré Awa ».
    pub fn commence(&self, nom: &str) -> bool {
        self.rang(nom).is_some_and(|(pire, _)| pire <= 1)
    }
}

/// Identité comparée pour détecter un doublon.
#[derive(Debug, Clone, Copy)]
pub struct IdentiteComparee<'a> {
    /// Nom.
    pub nom: &'a str,
    /// Prénoms.
    pub prenoms: &'a str,
    /// Date de naissance.
    pub date_naissance: Option<DateCivile>,
}

/// Deux fiches sont des doublons probables (SPEC_PHASE_2 §6.3) si leurs noms et leurs prénoms
/// sont proches et si leurs dates de naissance sont connues et distantes d'un an au plus.
pub fn doublon_probable(a: IdentiteComparee<'_>, b: IdentiteComparee<'_>) -> bool {
    let (Some(da), Some(db)) = (a.date_naissance, b.date_naissance) else {
        return false;
    };
    if (da.jours_depuis_epoque() - db.jours_depuis_epoque()).abs() > 365 {
        return false;
    }
    let proches = |x: &str, y: &str| {
        let (rx, ry) = (Recherche::nouvelle(x), Recherche::nouvelle(y));
        !rx.est_vide() && !ry.est_vide() && (rx.rang(y).is_some() || ry.rang(x).is_some())
    };
    proches(a.nom, b.nom) && proches(a.prenoms, b.prenoms)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accents_casse_et_ponctuation() {
        assert_eq!(normaliser("Keïta"), normaliser("KEITA"));
        assert_eq!(normaliser("Traoré"), "TRAORE");
        assert_eq!(normaliser("  N'Diaye-Sow  Œuvre "), "N DIAYE SOW OEUVRE");
    }

    #[test]
    fn mots_en_debut() {
        let nom = "Traoré Awa Fatoumata";
        assert!(Recherche::nouvelle("tra aw").commence(nom));
        assert!(Recherche::nouvelle("FATOU").commence(nom));
        assert!(!Recherche::nouvelle("raore").commence(nom));
        assert!(Recherche::nouvelle("").commence(nom));
    }

    #[test]
    fn variantes_du_terrain() {
        let trouve = |nom: &str, cherche: &str| Recherche::nouvelle(cherche).rang(nom);
        assert_eq!(trouve("Traoré Awa", "Traore"), Some((0, 0)));
        assert_eq!(trouve("Keïta Fanta", "Keita"), Some((0, 0)));
        assert!(trouve("Coulibaly Mariam", "Kouloubaly").is_some());
        assert!(trouve("Diallo Oumar", "Jallo").is_some());
        assert_eq!(trouve("Sangaré Mohamed", "Mahamadou"), Some((2, 2)));
        assert!(trouve("Cissé Ali", "Sisse Aly").is_some());
        assert!(trouve("Traoré Awa", "Diarra").is_none());
        assert!(trouve("Dembélé Ali", "Diop").is_none());
    }

    #[test]
    fn classement_du_plus_sur_au_plus_lache() {
        let cherche = Recherche::nouvelle("Coulibaly");
        let exact = cherche.rang("Coulibaly Moussa");
        let proche = cherche.rang("Kouloubaly Moussa");
        assert!(exact < proche, "{exact:?} {proche:?}");
    }

    #[test]
    fn phonetique_et_distance() {
        assert_eq!(cle_phonetique("COULIBALY"), "KULIBALI");
        assert_eq!(cle_phonetique("KOULOUBALY"), "KULUBALI");
        assert_eq!(cle_phonetique("CISSE"), "SIS");
        assert_eq!(cle_phonetique("MARIAME"), cle_phonetique("MARIAM"));
        assert_eq!(distance("KULIBALI", "KULUBALI"), 1);
        assert_eq!(distance("", "ABC"), 3);
        assert_eq!(cle_phonetique("DIALLO"), cle_phonetique("JALLO"));
        assert!(equivalents("MOHAMED", "MAMADOU"));
        assert!(!equivalents("MOHAMED", "OUMAR"));
    }

    #[test]
    fn doublons_probables() {
        let date = |t: &str| DateCivile::analyser(t);
        let fiche = |nom, prenoms, naissance| IdentiteComparee {
            nom,
            prenoms,
            date_naissance: date(naissance),
        };
        let awa = fiche("Traoré", "Awa", "2014-03-02");
        assert!(doublon_probable(awa, fiche("TRAORE", "Hawa", "2014-07-15")));
        assert!(doublon_probable(
            awa,
            fiche("Traoré", "Awa Fatoumata", "2014-03-02")
        ));
        assert!(!doublon_probable(awa, fiche("Traoré", "Awa", "2016-03-02")));
        assert!(!doublon_probable(
            awa,
            fiche("Traoré", "Moussa", "2014-03-02")
        ));
        assert!(!doublon_probable(awa, fiche("Diarra", "Awa", "2014-03-02")));
    }

    #[test]
    fn deux_mille_noms_en_quelques_millisecondes() {
        let noms: Vec<String> = (0..2_000)
            .map(|i| format!("Coulibaly{} Fatoumata Awa {i}", i % 37))
            .collect();
        let cherche = Recherche::nouvelle("Kouloubaly fatimata");
        let debut = std::time::Instant::now();
        let trouves = noms.iter().filter(|n| cherche.rang(n).is_some()).count();
        assert!(trouves > 0);
        // Cible de 200 ms en compilation optimisée, celle du poste de l'école ; la compilation
        // de développement, sans optimisation, dispose de dix fois plus.
        let budget = if cfg!(debug_assertions) { 2_000 } else { 200 };
        let duree = debut.elapsed();
        assert!(
            duree < std::time::Duration::from_millis(budget),
            "{duree:?}"
        );
    }
}
