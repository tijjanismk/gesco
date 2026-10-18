//! Consultation par SMS entrant (SPEC_PHASE_5 §9, blueprint §25.4 et §25.5) : un parent écrit
//! « SOLDE code », « ABS code » ou « NOTE code » au numéro de l'école et reçoit la réponse. Le code
//! d'accès est une identité du responsable, remise en main propre au guichet ; la base n'en garde
//! qu'une empreinte. Un numéro inconnu ne reçoit rien ; un code faux ne reçoit qu'une réponse par
//! jour, et cinq codes faux bloquent le numéro jusqu'au lendemain ; au-delà du quota, plus rien.
//! Ni appréciation, ni sanction, ni décision du conseil de classe ne partent par SMS (§8.5bis).

use serde::{Deserialize, Serialize};

use super::gsm7;
use super::message::Message;
use crate::commun::{Horodatage, Id};
use crate::core::Etablissement;
use crate::scolarite::Responsable;

/// Caractères d'un code : lettres et chiffres sans ambiguïté, ni O, 0, I, 1. Trente-deux signes :
/// un octet tiré au hasard, réduit à cinq bits, les choisit tous avec la même chance.
pub const ALPHABET_CODE: &[u8; 32] = b"ABCDEFGHJKLMNPQRSTUVWXYZ23456789";

/// Longueur d'un code d'accès.
pub const LONGUEUR_CODE: usize = 6;

/// Codes faux dans la journée qui bloquent un numéro jusqu'au lendemain.
pub const CODES_FAUX_MAX: u32 = 5;

/// Consultations par numéro et par jour, par défaut.
pub const QUOTA_DEFAUT: i64 = 5;

/// Code tiré de six octets aléatoires.
pub fn code_depuis_octets(octets: &[u8; LONGUEUR_CODE]) -> String {
    octets
        .iter()
        .map(|o| {
            char::from(
                ALPHABET_CODE
                    .get(usize::from(o & 0x1F))
                    .copied()
                    .unwrap_or(b'A'),
            )
        })
        .collect()
}

/// Code saisi ou lu dans un SMS, sous sa forme comparable : majuscules, sans espace.
pub fn normaliser_code(saisie: &str) -> String {
    saisie
        .chars()
        .filter(|c| !c.is_whitespace())
        .flat_map(char::to_uppercase)
        .collect()
}

/// Vrai pour un code de six caractères de l'alphabet.
pub fn code_bien_forme(code: &str) -> bool {
    code.len() == LONGUEUR_CODE && code.bytes().all(|o| ALPHABET_CODE.contains(&o))
}

/// Code en groupes de trois, pour le billet : `K7P X3M`.
pub fn code_lisible(code: &str) -> String {
    match (code.get(..3), code.get(3..)) {
        (Some(debut), Some(fin)) if !fin.is_empty() => format!("{debut} {fin}"),
        _ => code.to_owned(),
    }
}

/// État d'un code d'accès.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "snake_case")]
pub enum EtatCodeAcces {
    /// Le code ouvre la consultation.
    Actif,
    /// Remplacé par un nouveau code, ou révoqué : il n'ouvre plus rien.
    Revoque,
}

impl EtatCodeAcces {
    /// Nom persisté.
    pub const fn nom(&self) -> &'static str {
        match self {
            Self::Actif => "actif",
            Self::Revoque => "revoque",
        }
    }

    /// Lecture d'un nom persisté.
    pub fn depuis_nom(nom: &str) -> Option<Self> {
        [Self::Actif, Self::Revoque]
            .into_iter()
            .find(|e| e.nom() == nom)
    }
}

/// Code d'accès d'un responsable : son empreinte, jamais le code lui-même.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, specta::Type)]
pub struct CodeAcces {
    /// Identifiant technique.
    pub id: Id<CodeAcces>,
    /// Établissement.
    pub etablissement_id: Id<Etablissement>,
    /// Responsable à qui il a été remis.
    pub responsable_id: Id<Responsable>,
    /// Empreinte du code ; ne quitte jamais le cœur.
    #[serde(skip)]
    pub empreinte: String,
    /// État.
    pub etat: EtatCodeAcces,
    /// Remise du code.
    pub cree_le: Horodatage,
    /// Révocation, ou remplacement par un nouveau code.
    pub revoque_le: Option<Horodatage>,
    /// Dernière consultation servie avec ce code.
    pub derniere_utilisation: Option<Horodatage>,
}

/// Mot d'une consultation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "snake_case")]
pub enum MotConsultation {
    /// Reste à payer et prochaine échéance.
    Solde,
    /// Absences de la période en cours.
    Abs,
    /// Moyenne générale de la dernière période publiée.
    Note,
    /// La liste des mots.
    Aide,
}

impl MotConsultation {
    /// Tous les mots, dans l'ordre où l'aide les cite.
    pub const TOUS: [Self; 4] = [Self::Solde, Self::Abs, Self::Note, Self::Aide];

    /// Nom persisté.
    pub const fn nom(&self) -> &'static str {
        match self {
            Self::Solde => "solde",
            Self::Abs => "abs",
            Self::Note => "note",
            Self::Aide => "aide",
        }
    }

    /// Lecture d'un nom persisté.
    pub fn depuis_nom(nom: &str) -> Option<Self> {
        Self::TOUS.into_iter().find(|m| m.nom() == nom)
    }

    /// Mot tel que le parent l'écrit.
    const fn ecrit(&self) -> &'static str {
        match self {
            Self::Solde => "SOLDE",
            Self::Abs => "ABS",
            Self::Note => "NOTE",
            Self::Aide => "AIDE",
        }
    }
}

/// Demande lue dans un SMS : le mot, et le code qui le suit (vide pour l'aide).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DemandeSms {
    /// Mot.
    pub mot: MotConsultation,
    /// Code saisi, normalisé.
    pub code: String,
}

/// Lecture d'un SMS « MOT CODE », sans souci des majuscules, des accents, des espaces ni de la
/// ponctuation : « Solde k7p x3m » vaut « SOLDE K7PX3M ». Rien si le SMS ne commence par aucun
/// mot connu. « NOTES » et « ABSENCES » se comprennent aussi.
pub fn lire_demande(texte: &str) -> Option<DemandeSms> {
    let compact: String = gsm7::translitterer(texte)
        .chars()
        .filter(char::is_ascii_alphanumeric)
        .map(|c| c.to_ascii_uppercase())
        .collect();
    let variantes = [
        ("SOLDE", MotConsultation::Solde),
        ("ABSENCES", MotConsultation::Abs),
        ("ABS", MotConsultation::Abs),
        ("NOTES", MotConsultation::Note),
        ("NOTE", MotConsultation::Note),
        ("AIDE", MotConsultation::Aide),
    ];
    variantes.into_iter().find_map(|(ecrit, mot)| {
        compact.strip_prefix(ecrit).map(|code| DemandeSms {
            mot,
            code: code.to_owned(),
        })
    })
}

/// Ce qu'il est advenu d'un SMS reçu.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "snake_case")]
pub enum IssueConsultation {
    /// Code juste : la réponse est partie.
    Servie,
    /// L'aide, demandée ou envoyée à un SMS incompris d'un numéro connu.
    Aide,
    /// Code faux, premier du jour : « code incorrect » est répondu.
    CodeIncorrect,
    /// Code faux, déjà répondu aujourd'hui : rien ne part.
    CodeIncorrectMuet,
    /// Cinq codes faux aujourd'hui : le numéro n'a plus de réponse jusqu'au lendemain.
    Bloquee,
    /// Quota du jour atteint : plus de réponse jusqu'au lendemain.
    QuotaDepasse,
    /// Numéro d'aucun responsable : aucune réponse.
    NumeroInconnu,
    /// La consultation par SMS n'est pas ouverte par l'école : aucune réponse.
    Inactive,
}

impl IssueConsultation {
    /// Toutes les issues.
    pub const TOUTES: [Self; 8] = [
        Self::Servie,
        Self::Aide,
        Self::CodeIncorrect,
        Self::CodeIncorrectMuet,
        Self::Bloquee,
        Self::QuotaDepasse,
        Self::NumeroInconnu,
        Self::Inactive,
    ];

    /// Nom persisté.
    pub const fn nom(&self) -> &'static str {
        match self {
            Self::Servie => "servie",
            Self::Aide => "aide",
            Self::CodeIncorrect => "code_incorrect",
            Self::CodeIncorrectMuet => "code_incorrect_muet",
            Self::Bloquee => "bloquee",
            Self::QuotaDepasse => "quota_depasse",
            Self::NumeroInconnu => "numero_inconnu",
            Self::Inactive => "inactive",
        }
    }

    /// Lecture d'un nom persisté.
    pub fn depuis_nom(nom: &str) -> Option<Self> {
        Self::TOUTES.into_iter().find(|i| i.nom() == nom)
    }

    /// Vrai si une réponse part.
    pub const fn repond(&self) -> bool {
        matches!(self, Self::Servie | Self::Aide | Self::CodeIncorrect)
    }

    /// Vrai si la demande compte dans le quota du jour du numéro.
    pub const fn compte_dans_le_quota(&self) -> bool {
        matches!(
            self,
            Self::Servie | Self::Aide | Self::CodeIncorrect | Self::CodeIncorrectMuet
        )
    }

    /// Vrai pour un code faux.
    pub const fn code_faux(&self) -> bool {
        matches!(self, Self::CodeIncorrect | Self::CodeIncorrectMuet)
    }
}

/// SMS reçu, tel que le journal le garde : le code qu'il portait est masqué.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, specta::Type)]
pub struct SmsRecu {
    /// Identifiant technique.
    pub id: Id<SmsRecu>,
    /// Établissement.
    pub etablissement_id: Id<Etablissement>,
    /// Numéro appelant, sous forme canonique s'il en a une.
    pub numero: String,
    /// Texte reçu, le code masqué.
    pub texte: String,
    /// Mot compris ; aucun si le SMS n'est pas une demande.
    pub mot: Option<MotConsultation>,
    /// Issue.
    pub issue: IssueConsultation,
    /// Responsable du numéro, s'il est connu.
    pub responsable_id: Option<Id<Responsable>>,
    /// Réponse mise en file.
    pub reponse_id: Option<Id<Message>>,
    /// Réception.
    pub recu_le: Horodatage,
}

/// Texte d'un SMS reçu tel que le journal le garde : une demande comprise s'écrit avec son mot et
/// son code masqué ; le reste est gardé tel quel, borné à mille caractères.
pub fn texte_a_garder(texte: &str, demande: Option<&DemandeSms>) -> String {
    match demande {
        Some(d) if d.code.is_empty() => d.mot.ecrit().to_owned(),
        Some(d) => format!(
            "{} {}",
            d.mot.ecrit(),
            "*".repeat(d.code.chars().count().min(12))
        ),
        None => texte.trim().chars().take(1_000).collect(),
    }
}

/// Ce que le numéro a déjà demandé aujourd'hui.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct CompteDuJour {
    /// Demandes qui comptent dans le quota.
    pub demandes: u32,
    /// Codes faux.
    pub codes_faux: u32,
}

/// Ce que le service sait d'un SMS reçu, pour en décider l'issue.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Faits {
    /// Consultation ouverte par l'école.
    pub active: bool,
    /// Le numéro appelant est celui d'au moins un responsable.
    pub numero_connu: bool,
    /// Le numéro a déjà demandé aujourd'hui.
    pub compte: CompteDuJour,
    /// Quota du jour.
    pub quota: u32,
    /// Le mot lu ; aucun si le SMS n'est pas compris.
    pub mot: Option<MotConsultation>,
    /// Le code lu appartient à un responsable de ce numéro, et il est actif.
    pub code_juste: bool,
}

/// Issue d'un SMS reçu, dans l'ordre des contrôles : école, numéro, blocage, quota, demande, code.
pub const fn decider(f: &Faits) -> IssueConsultation {
    if !f.active {
        return IssueConsultation::Inactive;
    }
    if !f.numero_connu {
        return IssueConsultation::NumeroInconnu;
    }
    if f.compte.codes_faux >= CODES_FAUX_MAX {
        return IssueConsultation::Bloquee;
    }
    if f.compte.demandes >= f.quota {
        return IssueConsultation::QuotaDepasse;
    }
    match f.mot {
        None | Some(MotConsultation::Aide) => IssueConsultation::Aide,
        Some(_) if f.code_juste => IssueConsultation::Servie,
        Some(_) if f.compte.codes_faux == 0 => IssueConsultation::CodeIncorrect,
        Some(_) => IssueConsultation::CodeIncorrectMuet,
    }
}

// --- Réponses -------------------------------------------------------------------------------

/// Note en centièmes écrite à la française : `12,50`.
fn note(centiemes: i32) -> String {
    format!("{},{:02}", centiemes / 100, (centiemes % 100).abs())
}

/// Réponse de l'aide.
pub fn reponse_aide(ecole: &str) -> String {
    let mots: Vec<&str> = MotConsultation::TOUS
        .iter()
        .filter(|m| **m != MotConsultation::Aide)
        .map(MotConsultation::ecrit)
        .collect();
    format!(
        "{ecole} : envoyez {} suivi de votre code. Exemple : SOLDE K7PX3M.",
        mots.join(", ")
    )
}

/// Réponse à un code faux.
pub fn reponse_code_incorrect(ecole: &str) -> String {
    format!("{ecole} : code incorrect. Votre code est sur le billet remis par l'ecole.")
}

/// Ce que doit un enfant.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SoldeEnfant {
    /// Prénoms.
    pub prenom: String,
    /// Reste dû sur l'année, en francs, déjà écrit (`25 000 F`).
    pub reste: String,
    /// Part échue du reste, déjà écrite ; aucune si rien n'est échu.
    pub echu: Option<String>,
}

/// Réponse à SOLDE : le reste de chaque enfant, la part échue, la prochaine échéance.
pub fn reponse_solde(
    ecole: &str,
    enfants: &[SoldeEnfant],
    prochaine: Option<(&str, &str)>,
) -> String {
    if enfants.is_empty() {
        return format!("{ecole} : aucun enfant inscrit cette annee.");
    }
    let lignes: Vec<String> = enfants
        .iter()
        .map(|e| match &e.echu {
            Some(echu) => format!("{} {} dont {echu} echus", e.prenom, e.reste),
            None => format!("{} {}", e.prenom, e.reste),
        })
        .collect();
    let mut texte = format!("{ecole} : reste a payer {}.", lignes.join(", "));
    if let Some((date, montant)) = prochaine {
        texte.push_str(&format!(" Prochaine echeance le {date} : {montant}."));
    }
    texte
}

/// Absences d'un enfant sur la période.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AbsencesEnfant {
    /// Prénoms.
    pub prenom: String,
    /// Demi-journées d'absence.
    pub demi_journees: u32,
    /// Dont non justifiées.
    pub non_justifiees: u32,
}

/// Réponse à ABS : les absences de chaque enfant sur la période en cours.
pub fn reponse_absences(ecole: &str, periode: &str, enfants: &[AbsencesEnfant]) -> String {
    if enfants.is_empty() {
        return format!("{ecole} : aucun enfant inscrit cette annee.");
    }
    let lignes: Vec<String> = enfants
        .iter()
        .map(|e| match (e.demi_journees, e.non_justifiees) {
            (0, _) => format!("{} aucune", e.prenom),
            (n, 0) => format!("{} {n} demi-journee(s), justifiees", e.prenom),
            (n, nj) => format!(
                "{} {n} demi-journee(s) dont {nj} non justifiee(s)",
                e.prenom
            ),
        })
        .collect();
    format!("{ecole} : absences {periode} : {}.", lignes.join(" ; "))
}

/// Moyenne d'un enfant sur la dernière période publiée.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NoteEnfant {
    /// Prénoms.
    pub prenom: String,
    /// Période publiée, « 1er trimestre » ; aucune si rien n'est publié.
    pub periode: Option<String>,
    /// Moyenne générale en centièmes.
    pub moyenne: Option<i32>,
    /// Barème en centièmes (2 000 pour 20).
    pub bareme: i32,
    /// Rang et effectif, si l'école les donne par SMS.
    pub rang: Option<(u32, u32)>,
}

/// Réponse à NOTE : la moyenne générale de la dernière période publiée de chaque enfant ; le rang
/// seulement si l'école l'a voulu.
pub fn reponse_notes(ecole: &str, enfants: &[NoteEnfant]) -> String {
    if enfants.is_empty() {
        return format!("{ecole} : aucun enfant inscrit cette annee.");
    }
    let lignes: Vec<String> = enfants
        .iter()
        .map(|e| match (&e.periode, e.moyenne) {
            (Some(periode), Some(moyenne)) => {
                let mut ligne = format!(
                    "{} {periode} {}/{}",
                    e.prenom,
                    note(moyenne),
                    e.bareme / 100
                );
                if let Some((rang, effectif)) = e.rang {
                    let suffixe = if rang == 1 { "er" } else { "e" };
                    ligne.push_str(&format!(", rang {rang}{suffixe}/{effectif}"));
                }
                ligne
            }
            (Some(periode), None) => format!("{} {periode} sans moyenne", e.prenom),
            (None, _) => format!("{} pas encore de bulletin", e.prenom),
        })
        .collect();
    format!("{ecole} : moyennes {}.", lignes.join(" ; "))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn codes_tires_dans_l_alphabet_sans_ambiguite() {
        let code = code_depuis_octets(&[0, 31, 32, 255, 14, 8]);
        assert_eq!(code, "A9A9QJ");
        assert!(code_bien_forme(&code));
        for interdit in ['O', '0', 'I', '1'] {
            assert!(!ALPHABET_CODE.contains(&(interdit as u8)));
        }
        assert!(!code_bien_forme("A99PQ"));
        assert!(!code_bien_forme("A99PQO"));
        assert_eq!(normaliser_code(" k7p x3m "), "K7PX3M");
        assert_eq!(code_lisible("K7PX3M"), "K7P X3M");
    }

    #[test]
    fn demandes_lues_sans_souci_de_la_forme() {
        let d = |t: &str| lire_demande(t).map(|d| (d.mot, d.code));
        assert_eq!(
            d("SOLDE K7PX3M"),
            Some((MotConsultation::Solde, "K7PX3M".into()))
        );
        assert_eq!(
            d("solde k7p x3m"),
            Some((MotConsultation::Solde, "K7PX3M".into()))
        );
        assert_eq!(
            d("Abs: k7px3m"),
            Some((MotConsultation::Abs, "K7PX3M".into()))
        );
        assert_eq!(
            d("absences K7PX3M"),
            Some((MotConsultation::Abs, "K7PX3M".into()))
        );
        assert_eq!(
            d("Notes K7PX3M"),
            Some((MotConsultation::Note, "K7PX3M".into()))
        );
        assert_eq!(d("aide"), Some((MotConsultation::Aide, String::new())));
        assert_eq!(d("Bonjour, merci"), None);
        let demande = lire_demande("solde k7px3m");
        assert_eq!(
            texte_a_garder("solde k7px3m", demande.as_ref()),
            "SOLDE ******"
        );
        assert_eq!(texte_a_garder(" Merci ", None), "Merci");
        assert_eq!(d(""), None);
    }

    fn faits() -> Faits {
        Faits {
            active: true,
            numero_connu: true,
            compte: CompteDuJour::default(),
            quota: 5,
            mot: Some(MotConsultation::Solde),
            code_juste: true,
        }
    }

    #[test]
    fn controles_dans_l_ordre() {
        use IssueConsultation as I;
        assert_eq!(decider(&faits()), I::Servie);
        assert_eq!(
            decider(&Faits {
                active: false,
                ..faits()
            }),
            I::Inactive
        );
        assert_eq!(
            decider(&Faits {
                numero_connu: false,
                ..faits()
            }),
            I::NumeroInconnu
        );
        let faux = Faits {
            code_juste: false,
            ..faits()
        };
        assert_eq!(decider(&faux), I::CodeIncorrect);
        let deja_faux = Faits {
            compte: CompteDuJour {
                demandes: 1,
                codes_faux: 1,
            },
            ..faux
        };
        assert_eq!(decider(&deja_faux), I::CodeIncorrectMuet);
        // Cinq codes faux : même le bon code ne reçoit plus rien jusqu'au lendemain.
        let bloque = Faits {
            compte: CompteDuJour {
                demandes: 5,
                codes_faux: 5,
            },
            quota: 10,
            ..faits()
        };
        assert_eq!(decider(&bloque), I::Bloquee);
        let quota = Faits {
            compte: CompteDuJour {
                demandes: 5,
                codes_faux: 0,
            },
            ..faits()
        };
        assert_eq!(decider(&quota), I::QuotaDepasse);
        assert_eq!(decider(&Faits { mot: None, ..faux }), I::Aide);
        assert!(I::Servie.repond() && I::CodeIncorrect.repond());
        assert!(!I::CodeIncorrectMuet.repond() && !I::NumeroInconnu.repond());
    }

    #[test]
    fn reponses_d_un_seul_sms() {
        let ecole = "GS Les Etoiles";
        let solde = reponse_solde(
            ecole,
            &[
                SoldeEnfant {
                    prenom: "Awa".into(),
                    reste: "25 000 F".into(),
                    echu: Some("10 000 F".into()),
                },
                SoldeEnfant {
                    prenom: "Moussa".into(),
                    reste: "0 F".into(),
                    echu: None,
                },
            ],
            Some(("05/11/2026", "15 000 F")),
        );
        assert_eq!(
            solde,
            "GS Les Etoiles : reste a payer Awa 25 000 F dont 10 000 F echus, Moussa 0 F. Prochaine echeance le 05/11/2026 : 15 000 F."
        );
        assert_eq!(gsm7::nombre_sms(&solde), 1);
        let abs = reponse_absences(
            ecole,
            "du 1er trimestre",
            &[
                AbsencesEnfant {
                    prenom: "Awa".into(),
                    demi_journees: 3,
                    non_justifiees: 1,
                },
                AbsencesEnfant {
                    prenom: "Moussa".into(),
                    demi_journees: 0,
                    non_justifiees: 0,
                },
            ],
        );
        assert_eq!(
            abs,
            "GS Les Etoiles : absences du 1er trimestre : Awa 3 demi-journee(s) dont 1 non justifiee(s) ; Moussa aucune."
        );
        let notes = reponse_notes(
            ecole,
            &[
                NoteEnfant {
                    prenom: "Awa".into(),
                    periode: Some("1er trimestre".into()),
                    moyenne: Some(1_250),
                    bareme: 2_000,
                    rang: Some((1, 42)),
                },
                NoteEnfant {
                    prenom: "Moussa".into(),
                    periode: None,
                    moyenne: None,
                    bareme: 2_000,
                    rang: None,
                },
            ],
        );
        assert_eq!(
            notes,
            "GS Les Etoiles : moyennes Awa 1er trimestre 12,50/20, rang 1er/42 ; Moussa pas encore de bulletin."
        );
        assert_eq!(
            reponse_aide(ecole),
            "GS Les Etoiles : envoyez SOLDE, ABS, NOTE suivi de votre code. Exemple : SOLDE K7PX3M."
        );
        assert!(gsm7::nombre_sms(&reponse_code_incorrect(ecole)) == 1);
    }
}
