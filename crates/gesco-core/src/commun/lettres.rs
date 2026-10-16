//! Nombres en toutes lettres, en français (SPEC_PHASE_3 §2) : la mention qui fait foi sur un
//! reçu. Orthographe traditionnelle, celle des reçus et des chèques au Mali : traits d'union sous
//! cent seulement, « et un » de vingt à soixante et onze, « quatre-vingts » et « cents » au
//! pluriel quand rien ne les suit et devant « millions » ou « milliards », invariables devant
//! « mille » ; « mille » invariable, « million » et « milliard » variables.

/// Plus grand nombre écrit en lettres : 999 999 999 999 (neuf cent quatre-vingt-dix-neuf
/// milliards…). Au-delà, aucune école n'a de montant.
pub const MAXIMUM: u64 = 999_999_999_999;

const UNITES: [&str; 17] = [
    "zéro", "un", "deux", "trois", "quatre", "cinq", "six", "sept", "huit", "neuf", "dix", "onze",
    "douze", "treize", "quatorze", "quinze", "seize",
];

const DIZAINES: [&str; 7] = [
    "",
    "dix",
    "vingt",
    "trente",
    "quarante",
    "cinquante",
    "soixante",
];

fn unite(n: u64) -> &'static str {
    usize::try_from(n)
        .ok()
        .and_then(|i| UNITES.get(i))
        .copied()
        .unwrap_or("")
}

/// De 0 à 99. `final_` : rien ne suit dans le nombre, « quatre-vingts » prend son s.
fn moins_de_cent(n: u64, final_: bool) -> String {
    match n {
        0..=16 => unite(n).to_owned(),
        17..=19 => format!("dix-{}", unite(n - 10)),
        20..=69 => {
            let dizaine = usize::try_from(n / 10)
                .ok()
                .and_then(|i| DIZAINES.get(i))
                .copied()
                .unwrap_or("");
            match n % 10 {
                0 => dizaine.to_owned(),
                1 => format!("{dizaine} et un"),
                u => format!("{dizaine}-{}", unite(u)),
            }
        }
        70..=79 => match n - 60 {
            11 => "soixante et onze".to_owned(),
            r => format!("soixante-{}", moins_de_cent(r, false)),
        },
        80 => {
            if final_ {
                "quatre-vingts".to_owned()
            } else {
                "quatre-vingt".to_owned()
            }
        }
        _ => format!("quatre-vingt-{}", moins_de_cent(n - 80, false)),
    }
}

/// De 1 à 999. `final_` : rien ne suit, ou « millions » ou « milliards » suit (cents et
/// quatre-vingts prennent leur s) ; faux devant « mille ».
fn moins_de_mille(n: u64, final_: bool) -> String {
    let centaines = n / 100;
    let reste = n % 100;
    let tete = match centaines {
        0 => String::new(),
        1 => "cent".to_owned(),
        c => {
            if reste == 0 && final_ {
                format!("{} cents", unite(c))
            } else {
                format!("{} cent", unite(c))
            }
        }
    };
    match (tete.is_empty(), reste) {
        (true, _) => moins_de_cent(reste, final_),
        (false, 0) => tete,
        (false, r) => format!("{tete} {}", moins_de_cent(r, final_)),
    }
}

/// Écrit `n` en lettres ; `None` au-delà de [`MAXIMUM`].
pub fn en_lettres(n: u64) -> Option<String> {
    if n > MAXIMUM {
        return None;
    }
    if n == 0 {
        return Some("zéro".to_owned());
    }
    let milliards = n / 1_000_000_000;
    let millions = (n / 1_000_000) % 1_000;
    let milliers = (n / 1_000) % 1_000;
    let reste = n % 1_000;
    let mut mots: Vec<String> = Vec::new();
    for (valeur, singulier, pluriel) in [
        (milliards, "milliard", "milliards"),
        (millions, "million", "millions"),
    ] {
        match valeur {
            0 => {}
            1 => mots.push(format!("un {singulier}")),
            v => mots.push(format!("{} {pluriel}", moins_de_mille(v, true))),
        }
    }
    match milliers {
        0 => {}
        1 => mots.push("mille".to_owned()),
        v => mots.push(format!("{} mille", moins_de_mille(v, false))),
    }
    if reste > 0 {
        mots.push(moins_de_mille(reste, true));
    }
    Some(mots.join(" "))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn l(n: u64) -> String {
        en_lettres(n).unwrap_or_default()
    }

    #[test]
    fn unites_et_dizaines() {
        assert_eq!(l(0), "zéro");
        assert_eq!(l(1), "un");
        assert_eq!(l(16), "seize");
        assert_eq!(l(17), "dix-sept");
        assert_eq!(l(21), "vingt et un");
        assert_eq!(l(22), "vingt-deux");
        assert_eq!(l(61), "soixante et un");
        assert_eq!(l(70), "soixante-dix");
        assert_eq!(l(71), "soixante et onze");
        assert_eq!(l(77), "soixante-dix-sept");
        assert_eq!(l(80), "quatre-vingts");
        assert_eq!(l(81), "quatre-vingt-un");
        assert_eq!(l(90), "quatre-vingt-dix");
        assert_eq!(l(91), "quatre-vingt-onze");
        assert_eq!(l(99), "quatre-vingt-dix-neuf");
    }

    #[test]
    fn centaines_et_accords() {
        assert_eq!(l(100), "cent");
        assert_eq!(l(101), "cent un");
        assert_eq!(l(200), "deux cents");
        assert_eq!(l(201), "deux cent un");
        assert_eq!(l(280), "deux cent quatre-vingts");
        assert_eq!(l(999), "neuf cent quatre-vingt-dix-neuf");
    }

    #[test]
    fn milliers_millions_milliards() {
        assert_eq!(l(1_000), "mille");
        assert_eq!(l(1_001), "mille un");
        assert_eq!(l(2_000), "deux mille");
        assert_eq!(l(80_000), "quatre-vingt mille");
        assert_eq!(l(200_000), "deux cent mille");
        assert_eq!(l(125_000), "cent vingt-cinq mille");
        assert_eq!(l(350_500), "trois cent cinquante mille cinq cents");
        assert_eq!(l(1_000_000), "un million");
        assert_eq!(l(2_000_000), "deux millions");
        assert_eq!(l(200_000_000), "deux cents millions");
        assert_eq!(l(80_000_000), "quatre-vingts millions");
        assert_eq!(l(1_250_000), "un million deux cent cinquante mille");
        assert_eq!(
            l(999_999_999),
            "neuf cent quatre-vingt-dix-neuf millions neuf cent quatre-vingt-dix-neuf mille neuf cent quatre-vingt-dix-neuf"
        );
        assert_eq!(l(1_000_000_000), "un milliard");
        assert_eq!(l(3_000_000_005), "trois milliards cinq");
        assert_eq!(en_lettres(MAXIMUM + 1), None);
    }
}
