//! Téléphones sous forme canonique E.164 (SPEC_PHASE_2 §4.4, blueprint §25.6) : la même personne
//! ne doit pas exister sous trois formes et recevoir trois SMS, ou aucun.

/// Indicatif du Mali.
const INDICATIF_MALI: &str = "223";

/// Forme canonique d'un numéro saisi. Espaces, points, tirets et parenthèses sont ignorés ; huit
/// chiffres sont un numéro malien ; `00` vaut `+`. L'erreur est la clé du message.
pub fn normaliser(saisie: &str) -> Result<String, &'static str> {
    let compact: String = saisie
        .chars()
        .filter(|c| !matches!(c, ' ' | '.' | '-' | '(' | ')' | '\u{a0}'))
        .collect();
    let international = compact
        .strip_prefix('+')
        .or_else(|| compact.strip_prefix("00"));
    let chiffres = international.unwrap_or(&compact);
    if chiffres.is_empty() || !chiffres.chars().all(|c| c.is_ascii_digit()) {
        return Err("telephone.invalide");
    }
    match international {
        None if chiffres.len() == 8 => Ok(format!("+{INDICATIF_MALI}{chiffres}")),
        None => Err("telephone.invalide"),
        Some(numero) if numero.starts_with(INDICATIF_MALI) => {
            if numero.len() == INDICATIF_MALI.len() + 8 {
                Ok(format!("+{numero}"))
            } else {
                Err("telephone.invalide")
            }
        }
        Some(numero) if (8..=15).contains(&numero.len()) => Ok(format!("+{numero}")),
        Some(_) => Err("telephone.invalide"),
    }
}

/// Forme lisible d'un numéro canonique, pour l'impression : `+223 76 12 34 56`. Un numéro
/// étranger reste tel quel.
pub fn lisible(numero: &str) -> String {
    match numero.strip_prefix("+223") {
        Some(chiffres) if chiffres.len() == 8 && chiffres.chars().all(|c| c.is_ascii_digit()) => {
            let paires: Vec<&str> = (0..4)
                .filter_map(|i| chiffres.get(i * 2..i * 2 + 2))
                .collect();
            format!("+223 {}", paires.join(" "))
        }
        _ => numero.to_owned(),
    }
}

/// Liste saisie, normalisée, sans doublon ni ligne vide, dans l'ordre de saisie.
pub fn normaliser_liste(saisies: &[String]) -> Result<Vec<String>, &'static str> {
    let mut numeros: Vec<String> = Vec::new();
    for saisie in saisies.iter().filter(|s| !s.trim().is_empty()) {
        let numero = normaliser(saisie)?;
        if !numeros.contains(&numero) {
            numeros.push(numero);
        }
    }
    Ok(numeros)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn trois_formes_d_un_meme_numero_malien() {
        for saisie in [
            "76 12 34 56",
            "+223 76.12.34.56",
            "0022376123456",
            "(+223) 76-12-34-56",
        ] {
            assert_eq!(
                normaliser(saisie),
                Ok("+22376123456".to_owned()),
                "{saisie}"
            );
        }
    }

    #[test]
    fn numeros_etrangers_et_refus() {
        assert_eq!(
            normaliser("+33 6 12 34 56 78"),
            Ok("+33612345678".to_owned())
        );
        assert_eq!(
            normaliser("+225 07 07 07 07 07"),
            Ok("+2250707070707".to_owned())
        );
        for saisie in [
            "7612345",
            "761234567",
            "+223 76 12 34",
            "76 12 AB 56",
            "",
            "+",
        ] {
            assert_eq!(normaliser(saisie), Err("telephone.invalide"), "{saisie}");
        }
    }

    #[test]
    fn forme_lisible() {
        assert_eq!(lisible("+22376123456"), "+223 76 12 34 56");
        assert_eq!(lisible("+33612345678"), "+33612345678");
    }

    #[test]
    fn liste_sans_doublon() {
        let saisies = vec![
            "76 12 34 56".to_owned(),
            " ".to_owned(),
            "+22376123456".to_owned(),
            "66 00 11 22".to_owned(),
        ];
        assert_eq!(
            normaliser_liste(&saisies),
            Ok(vec!["+22376123456".to_owned(), "+22366001122".to_owned()])
        );
    }
}
