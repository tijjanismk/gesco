//! Alphabet GSM-7 (C-3, T-76) : un seul caractère hors de l'alphabet fait passer un SMS de 160 à
//! 70 caractères. Le texte est donc translittéré avant l'envoi, en français sans accents
//! (« eleve », « etait ») ; le nombre de SMS se compte comme sur un téléphone.

/// Caractères d'un SMS seul.
pub const CARACTERES_SMS_SEUL: usize = 160;

/// Caractères de chaque partie d'un SMS en plusieurs parties (l'en-tête en prend sept).
pub const CARACTERES_PAR_PARTIE: usize = 153;

/// Nombre de SMS au-delà duquel un message est refusé.
pub const SMS_MAX: usize = 3;

/// Caractères de la table d'extension : chacun compte pour deux.
const EXTENSION: &[char] = &['^', '{', '}', '\\', '[', ']', '~', '|', '€'];

/// Translittération d'un caractère vers l'alphabet GSM-7 sans accents.
fn translitterer_caractere(c: char, sortie: &mut String) {
    let remplacement = match c {
        'à' | 'â' | 'ä' | 'á' | 'ã' | 'å' => "a",
        'À' | 'Â' | 'Ä' | 'Á' | 'Ã' | 'Å' => "A",
        'é' | 'è' | 'ê' | 'ë' => "e",
        'É' | 'È' | 'Ê' | 'Ë' => "E",
        'î' | 'ï' | 'í' | 'ì' => "i",
        'Î' | 'Ï' | 'Í' | 'Ì' => "I",
        'ô' | 'ö' | 'ó' | 'ò' | 'õ' | 'ø' => "o",
        'Ô' | 'Ö' | 'Ó' | 'Ò' | 'Õ' | 'Ø' => "O",
        'ù' | 'û' | 'ü' | 'ú' => "u",
        'Ù' | 'Û' | 'Ü' | 'Ú' => "U",
        'ÿ' | 'ý' => "y",
        'Ÿ' | 'Ý' => "Y",
        'ç' => "c",
        'Ç' => "C",
        'ñ' => "n",
        'Ñ' => "N",
        'œ' => "oe",
        'Œ' => "OE",
        'æ' => "ae",
        'Æ' => "AE",
        'ß' => "ss",
        '’' | '‘' | '`' | '´' => "'",
        '“' | '”' | '«' | '»' => "\"",
        '–' | '—' | '‑' => "-",
        '…' => "...",
        '\u{a0}' | '\u{202f}' | '\t' => " ",
        '\r' => "",
        '°' => "o",
        '€' => "€",
        c if c == '\n' || (' '..='~').contains(&c) => {
            sortie.push(c);
            return;
        }
        _ => "?",
    };
    sortie.push_str(remplacement);
}

/// Texte translittéré en GSM-7 sans accents : les accents tombent, les guillemets et apostrophes
/// typographiques deviennent droits, un caractère sans équivalent devient « ? ».
pub fn translitterer(texte: &str) -> String {
    let mut sortie = String::with_capacity(texte.len());
    for c in texte.chars() {
        translitterer_caractere(c, &mut sortie);
    }
    sortie
}

/// Longueur en caractères GSM-7 d'un texte déjà translittéré : la table d'extension compte double.
pub fn longueur(texte: &str) -> usize {
    texte
        .chars()
        .map(|c| if EXTENSION.contains(&c) { 2 } else { 1 })
        .sum()
}

/// Nombre de SMS d'un texte déjà translittéré ; zéro pour un texte vide.
pub fn nombre_sms(texte: &str) -> usize {
    match longueur(texte) {
        0 => 0,
        n if n <= CARACTERES_SMS_SEUL => 1,
        n => n.div_ceil(CARACTERES_PAR_PARTIE),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn francais_sans_accents() {
        assert_eq!(
            translitterer("Votre enfant était absent, élève de 6ème — « Noël »"),
            "Votre enfant etait absent, eleve de 6eme - \" Noel \""
        );
        assert_eq!(translitterer("Cœur d’école à 7 h"), "Coeur d'ecole a 7 h");
        assert_eq!(translitterer("Bamako 🇲🇱"), "Bamako ??");
    }

    #[test]
    fn un_texte_translittere_ne_contient_que_de_l_ascii_ou_l_euro() {
        let t = translitterer("Ÿ Œ ß ø ñ ç ° … \u{a0}");
        assert!(t.chars().all(|c| c.is_ascii() || c == '€'), "{t}");
    }

    #[test]
    fn nombre_de_sms_comme_un_telephone() {
        assert_eq!(nombre_sms(""), 0);
        assert_eq!(nombre_sms(&"a".repeat(160)), 1);
        assert_eq!(nombre_sms(&"a".repeat(161)), 2);
        assert_eq!(nombre_sms(&"a".repeat(306)), 2);
        assert_eq!(nombre_sms(&"a".repeat(307)), 3);
        // La table d'extension compte double.
        assert_eq!(longueur("[1]"), 5);
        assert_eq!(nombre_sms(&format!("{}{{", "a".repeat(159))), 2);
    }
}
