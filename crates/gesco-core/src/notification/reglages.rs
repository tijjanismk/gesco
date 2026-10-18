//! Réglages des SMS (SPEC_PHASE_5 §3 et §5) : débit, heures d'envoi, plafond, messages qui partent
//! d'eux-mêmes et leurs délais. Les valeurs par défaut sont posées avec les autres paramètres.

use crate::core::parametre::ValeurParametre;

/// Préfixe des réglages des SMS.
pub const PREFIXE: &str = "sms.";

/// Heures d'envoi par défaut : de 7 h à 20 h, rien la nuit (C-4).
pub const FENETRE_DEFAUT: &str = "7-20";

/// Canaux d'envoi : aucun (rien ne part), simulé (les SMS s'écrivent dans un journal du poste,
/// rien ne part réellement), la clé modem GSM branchée sur le poste.
pub const CANAUX: [&str; 3] = ["aucun", "simule", "modem"];

/// Vrai pour un code USSD lisible : chiffres, `*` et `#`, vingt caractères au plus.
fn code_ussd_valide(code: &str) -> bool {
    code.len() <= 20
        && code
            .bytes()
            .all(|b| b.is_ascii_digit() || b == b'*' || b == b'#')
}

/// Longueur maximale de la signature, le nom de l'école en tête de chaque SMS.
pub const SIGNATURE_MAX: usize = 20;

/// Heures d'envoi « 7-20 » : de 7 h à 20 h. Le début précède la fin, dans la journée.
pub fn lire_fenetre(texte: &str) -> Option<(u8, u8)> {
    let (debut, fin) = texte.split_once('-')?;
    let heure = |t: &str| {
        let t = t.trim().trim_end_matches(['h', 'H']).trim();
        (!t.is_empty() && t.len() <= 2 && t.bytes().all(|b| b.is_ascii_digit()))
            .then(|| t.parse::<u8>().ok())
            .flatten()
    };
    let (debut, fin) = (heure(debut)?, heure(fin)?);
    (debut < fin && fin <= 24).then_some((debut, fin))
}

/// Vrai si l'heure (0 à 23) est dans les heures d'envoi.
pub const fn dans_la_fenetre(fenetre: (u8, u8), heure: u8) -> bool {
    fenetre.0 <= heure && heure < fenetre.1
}

/// `Some(vrai)` si la valeur d'un réglage des SMS sort de ses bornes ; `None` pour une autre clé.
pub fn hors_bornes(cle: &str, valeur: &ValeurParametre) -> Option<bool> {
    let entre = |n: &i64, min: i64, max: i64| !(min..=max).contains(n);
    let hors = match (cle, valeur) {
        ("sms.signature", ValeurParametre::Texte(t)) => t.trim().chars().count() > SIGNATURE_MAX,
        ("sms.fenetre_envoi", ValeurParametre::Texte(t)) => lire_fenetre(t).is_none(),
        ("sms.debit_par_minute", ValeurParametre::Entier(n)) => entre(n, 1, 20),
        ("sms.plafond_mensuel", ValeurParametre::Entier(n)) => entre(n, 0, 1_000_000),
        ("sms.impaye_delai_jours", ValeurParametre::Entier(n)) => entre(n, 0, 90),
        ("sms.impaye_intervalle_jours", ValeurParametre::Entier(n)) => entre(n, 1, 90),
        ("sms.impaye_nombre_max", ValeurParametre::Entier(n)) => entre(n, 1, 10),
        ("sms.impaye_montant_min", ValeurParametre::Entier(n)) => entre(n, 0, 1_000_000_000),
        ("sms.absence_delai_grace_min", ValeurParametre::Entier(n)) => entre(n, 0, 240),
        ("sms.canal", ValeurParametre::Texte(t)) => !CANAUX.contains(&t.as_str()),
        ("sms.port", ValeurParametre::Texte(t)) => t.chars().count() > 40,
        ("sms.ussd_solde", ValeurParametre::Texte(t)) => !code_ussd_valide(t.trim()),
        ("sms.seuil_credit", ValeurParametre::Entier(n)) => entre(n, 0, 1_000_000),
        ("sms.numero_alerte", ValeurParametre::Texte(t)) => {
            !t.trim().is_empty() && crate::scolarite::telephone::normaliser(t).is_err()
        }
        (cle, ValeurParametre::Texte(t)) if super::modele::nature_du_reglage(cle).is_some() => {
            super::modele::nature_du_reglage(cle)
                .is_none_or(|n| super::modele::verifier(n, t).is_err())
        }
        _ if cle.starts_with(PREFIXE) => false,
        _ => return None,
    };
    Some(hors)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn heures_d_envoi() {
        assert_eq!(lire_fenetre(FENETRE_DEFAUT), Some((7, 20)));
        assert_eq!(lire_fenetre(" 8 h - 18 h "), Some((8, 18)));
        assert_eq!(lire_fenetre("0-24"), Some((0, 24)));
        assert_eq!(lire_fenetre("20-7"), None);
        assert_eq!(lire_fenetre("7-25"), None);
        assert_eq!(lire_fenetre("7"), None);
        assert_eq!(lire_fenetre("sept-vingt"), None);
        assert!(dans_la_fenetre((7, 20), 7));
        assert!(dans_la_fenetre((7, 20), 19));
        assert!(!dans_la_fenetre((7, 20), 20));
        assert!(!dans_la_fenetre((7, 20), 3));
    }

    #[test]
    fn bornes() {
        let e = ValeurParametre::Entier;
        assert_eq!(hors_bornes("sms.debit_par_minute", &e(6)), Some(false));
        assert_eq!(hors_bornes("sms.debit_par_minute", &e(0)), Some(true));
        assert_eq!(hors_bornes("sms.impaye_nombre_max", &e(11)), Some(true));
        assert_eq!(
            hors_bornes(
                "sms.signature",
                &ValeurParametre::Texte("Groupe scolaire Les Etoiles de Bamako".to_owned())
            ),
            Some(true)
        );
        assert_eq!(
            hors_bornes(
                "sms.fenetre_envoi",
                &ValeurParametre::Texte("22-6".to_owned())
            ),
            Some(true)
        );
        assert_eq!(
            hors_bornes("sms.auto_bulletin", &ValeurParametre::Booleen(false)),
            Some(false)
        );
        assert_eq!(
            hors_bornes("sms.canal", &ValeurParametre::Texte("simule".to_owned())),
            Some(false)
        );
        assert_eq!(
            hors_bornes("sms.canal", &ValeurParametre::Texte("pigeon".to_owned())),
            Some(true)
        );
        assert_eq!(
            hors_bornes(
                "sms.modele_bulletin",
                &ValeurParametre::Texte("{montant}".to_owned())
            ),
            Some(true)
        );
        let texte = |t: &str| ValeurParametre::Texte(t.to_owned());
        assert_eq!(hors_bornes("sms.canal", &texte("modem")), Some(false));
        assert_eq!(hors_bornes("sms.ussd_solde", &texte("#111#")), Some(false));
        assert_eq!(hors_bornes("sms.ussd_solde", &texte("solde")), Some(true));
        assert_eq!(hors_bornes("sms.numero_alerte", &texte("")), Some(false));
        assert_eq!(
            hors_bornes("sms.numero_alerte", &texte("76 12 34 56")),
            Some(false)
        );
        assert_eq!(hors_bornes("sms.numero_alerte", &texte("12")), Some(true));
        assert_eq!(hors_bornes("depense.seuil_accord", &e(0)), None);
    }
}
