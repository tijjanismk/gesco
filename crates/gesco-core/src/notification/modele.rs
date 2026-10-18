//! Formulation des messages (§25.8) : le fait porte des données, le modèle de l'école en fait une
//! phrase. Un modèle a ses variables entre accolades, `{eleve}` ; il se reformule dans les
//! réglages ; avec des valeurs longues, il tient en trois SMS au plus.

use super::gsm7;
use super::message::NatureMessage;

/// Variables qu'un modèle peut employer, par nature, avec une valeur d'exemple longue.
pub fn variables(nature: NatureMessage) -> &'static [(&'static str, &'static str)] {
    const ECOLE: (&str, &str) = ("ecole", "GS Les Etoiles");
    const ELEVE: (&str, &str) = ("eleve", "Aminata Kadiatou Coulibaly");
    const CLASSE: (&str, &str) = ("classe", "12e TSExp B");
    match nature {
        NatureMessage::Bulletin => &[ECOLE, ELEVE, CLASSE, ("periode", "2e trimestre")],
        NatureMessage::Impaye => &[
            ECOLE,
            ELEVE,
            CLASSE,
            ("montant", "125 000 F"),
            ("date", "05/11/2026"),
        ],
        NatureMessage::Absence => &[
            ECOLE,
            ELEVE,
            CLASSE,
            ("date", "12/03/2027"),
            ("demi_journee", "matinee et apres-midi"),
        ],
        NatureMessage::Convocation => &[
            ECOLE,
            ELEVE,
            ("date", "12/03/2027"),
            ("heure", "10 h 30"),
            ("motif", "travail insuffisant et retards repetes"),
        ],
        NatureMessage::Sanction => &[
            ECOLE,
            ELEVE,
            CLASSE,
            ("sanction", "Avertissement"),
            ("date", "12/03/2027"),
        ],
        NatureMessage::Essai => &[ECOLE],
    }
}

/// Natures dont l'école reformule le modèle.
pub const NATURES_A_MODELE: [NatureMessage; 5] = [
    NatureMessage::Bulletin,
    NatureMessage::Impaye,
    NatureMessage::Absence,
    NatureMessage::Convocation,
    NatureMessage::Sanction,
];

/// Modèle livré d'une nature.
pub const fn modele_par_defaut(nature: NatureMessage) -> &'static str {
    match nature {
        NatureMessage::Bulletin => {
            "{ecole} : le bulletin de {periode} de {eleve} ({classe}) est disponible. Passez le retirer et le signer a l'ecole."
        }
        NatureMessage::Impaye => {
            "{ecole} : il reste {montant} a payer pour {eleve} ({classe}), echu depuis le {date}. Merci de passer a l'ecole."
        }
        NatureMessage::Absence => {
            "{ecole} : {eleve} ({classe}) etait absent(e) le {date}, {demi_journee}, sans justification."
        }
        NatureMessage::Convocation => {
            "{ecole} : vous etes convoque(e) le {date} a {heure} au sujet de {eleve}. Motif : {motif}."
        }
        NatureMessage::Sanction => {
            "{ecole} : {eleve} ({classe}) a recu le {date} : {sanction}. Merci de passer a l'ecole."
        }
        NatureMessage::Essai => {
            "{ecole} : message d'essai de GESCO. Les SMS de l'ecole partent bien."
        }
    }
}

/// Clé du réglage qui garde le modèle d'une nature.
pub fn cle_reglage(nature: NatureMessage) -> String {
    format!("sms.modele_{}", nature.nom())
}

/// Nature dont une clé de réglage garde le modèle.
pub fn nature_du_reglage(cle: &str) -> Option<NatureMessage> {
    cle.strip_prefix("sms.modele_")
        .and_then(NatureMessage::depuis_nom)
}

/// Texte formulé : chaque variable remplacée par sa valeur, puis translittéré en GSM-7. Une
/// variable sans valeur reste vide.
pub fn formuler(modele: &str, valeurs: &[(&str, &str)]) -> String {
    let mut texte = String::with_capacity(modele.len() + 64);
    let mut reste = modele;
    while let Some(debut) = reste.find('{') {
        let (avant, apres) = reste.split_at(debut);
        texte.push_str(avant);
        match apres.find('}') {
            Some(fin) => {
                let nom = apres.get(1..fin).unwrap_or_default();
                if let Some((_, v)) = valeurs.iter().find(|(n, _)| *n == nom) {
                    texte.push_str(v);
                }
                reste = apres.get(fin + 1..).unwrap_or_default();
            }
            None => {
                texte.push_str(apres);
                reste = "";
            }
        }
    }
    texte.push_str(reste);
    gsm7::translitterer(texte.trim())
}

/// Vérifie un modèle d'une nature : non vide, variables connues, trois SMS au plus avec les valeurs
/// d'exemple. L'erreur est la clé du message.
pub fn verifier(nature: NatureMessage, modele: &str) -> Result<(), &'static str> {
    if modele.trim().is_empty() {
        return Err("sms.modele_vide");
    }
    let connues = variables(nature);
    let mut reste = modele;
    while let Some(debut) = reste.find('{') {
        let apres = reste.get(debut..).unwrap_or_default();
        let fin = apres.find('}').ok_or("sms.modele_accolade")?;
        let nom = apres.get(1..fin).unwrap_or_default();
        if !connues.iter().any(|(n, _)| *n == nom) {
            return Err("sms.modele_variable_inconnue");
        }
        reste = apres.get(fin + 1..).unwrap_or_default();
    }
    if gsm7::nombre_sms(&formuler(modele, connues)) > gsm7::SMS_MAX {
        return Err("sms.modele_trop_long");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn les_modeles_livres_tiennent_en_un_sms() {
        for nature in NatureMessage::TOUTES {
            let modele = modele_par_defaut(nature);
            assert_eq!(verifier(nature, modele), Ok(()), "{nature:?}");
            assert_eq!(
                gsm7::nombre_sms(&formuler(modele, variables(nature))),
                1,
                "{nature:?}"
            );
        }
    }

    #[test]
    fn formulation_et_translitteration() {
        let texte = formuler(
            "{ecole} : {eleve} était absente le {date}.",
            &[
                ("ecole", "École Askia"),
                ("eleve", "Awa Traoré"),
                ("date", "14/10"),
            ],
        );
        assert_eq!(texte, "Ecole Askia : Awa Traore etait absente le 14/10.");
        assert_eq!(formuler("{inconnue}x {", &[]), "x {");
    }

    #[test]
    fn modeles_refuses() {
        let n = NatureMessage::Bulletin;
        assert_eq!(verifier(n, "  "), Err("sms.modele_vide"));
        assert_eq!(verifier(n, "{eleve"), Err("sms.modele_accolade"));
        assert_eq!(
            verifier(n, "{montant}"),
            Err("sms.modele_variable_inconnue")
        );
        assert_eq!(
            verifier(n, &"Bulletin disponible. ".repeat(25)),
            Err("sms.modele_trop_long")
        );
    }

    #[test]
    fn cle_du_reglage() {
        assert_eq!(cle_reglage(NatureMessage::Impaye), "sms.modele_impaye");
        assert_eq!(
            nature_du_reglage("sms.modele_absence"),
            Some(NatureMessage::Absence)
        );
        assert_eq!(nature_du_reglage("sms.debit_par_minute"), None);
    }
}
