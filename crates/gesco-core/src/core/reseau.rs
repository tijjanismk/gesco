//! Réseau local de l'école (SPEC_PHASE_11_A §4, ADR du 10 novembre 2026) : règles pures de
//! l'appairage d'un poste relié — code à recopier, secret du poste, empreinte courte du
//! certificat — et nombre de postes que la licence couvre.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::commun::{Horodatage, Id};
use crate::core::{Etablissement, Utilisateur};
use crate::notification::consultation::ALPHABET_CODE;

/// Longueur du code d'appairage affiché sur le poste principal.
pub const LONGUEUR_CODE_APPAIRAGE: usize = 8;

/// Durée de validité d'un code d'appairage : dix minutes.
pub const VALIDITE_CODE_MILLIS: i64 = 10 * 60 * 1_000;

/// Essais faux qui annulent un code d'appairage.
pub const ESSAIS_CODE_MAX: u32 = 5;

/// Longueur du secret d'un poste relié, en octets.
pub const LONGUEUR_SECRET: usize = 32;

/// Longueur maximale du nom donné à un poste.
pub const LONGUEUR_NOM_POSTE_MAX: usize = 40;

/// Postes reliés admis quand la licence ne dit rien : un seul, pour essayer.
pub const POSTES_A_L_ESSAI: u32 = 1;

/// Préfixe de la mention des postes parmi les modules d'une licence.
pub const MENTION_POSTES: &str = "postes:";

/// Code d'appairage tiré de huit octets aléatoires, dans l'alphabet sans O, 0, I ni 1.
pub fn code_appairage(octets: &[u8; LONGUEUR_CODE_APPAIRAGE]) -> String {
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

/// Code d'appairage tel qu'on le lit à l'écran : deux groupes de quatre.
pub fn code_appairage_lisible(code: &str) -> String {
    match (code.get(..4), code.get(4..)) {
        (Some(debut), Some(fin)) if !fin.is_empty() => format!("{debut} {fin}"),
        _ => code.to_owned(),
    }
}

/// Code tel qu'il a été saisi, ramené à sa forme de comparaison : majuscules, sans espace ni tiret.
pub fn normaliser_code(saisie: &str) -> String {
    saisie
        .chars()
        .filter(|c| !c.is_whitespace() && *c != '-')
        .flat_map(char::to_uppercase)
        .collect()
}

/// Code d'appairage en attente : il sert une fois, expire, et s'annule après trop d'essais faux.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CodeAppairage {
    /// Le code, sous sa forme de comparaison.
    pub code: String,
    /// Fin de validité.
    pub expire_le: Horodatage,
    /// Essais faux déjà comptés.
    pub essais_faux: u32,
}

/// Issue de la présentation d'un code.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IssueCode {
    /// Le code est le bon : il est consommé.
    Accepte,
    /// Le code est faux ; il reste des essais.
    Faux,
    /// Le code est faux une fois de trop : il est annulé.
    Annule,
    /// Le code a expiré.
    Expire,
}

impl CodeAppairage {
    /// Code neuf, valable dix minutes à partir de `maintenant`.
    pub fn nouveau(code: String, maintenant: Horodatage) -> Self {
        Self {
            code,
            expire_le: maintenant
                .plus_millis(VALIDITE_CODE_MILLIS)
                .unwrap_or(maintenant),
            essais_faux: 0,
        }
    }

    /// Présente un code saisi. Un code accepté, expiré ou annulé ne doit plus servir : l'appelant
    /// l'oublie.
    pub fn presenter(&mut self, saisie: &str, maintenant: Horodatage) -> IssueCode {
        if !maintenant.avant(&self.expire_le) {
            return IssueCode::Expire;
        }
        if egal_en_temps_constant(normaliser_code(saisie).as_bytes(), self.code.as_bytes()) {
            return IssueCode::Accepte;
        }
        self.essais_faux = self.essais_faux.saturating_add(1);
        if self.essais_faux >= ESSAIS_CODE_MAX {
            IssueCode::Annule
        } else {
            IssueCode::Faux
        }
    }
}

/// Compare deux suites d'octets sans s'arrêter à la première différence.
fn egal_en_temps_constant(a: &[u8], b: &[u8]) -> bool {
    a.len() == b.len() && a.iter().zip(b).fold(0u8, |acc, (x, y)| acc | (x ^ y)) == 0
}

/// Secret d'un poste, en hexadécimal : c'est lui que le poste relié garde dans son trousseau.
pub fn secret_en_hexadecimal(octets: &[u8; LONGUEUR_SECRET]) -> String {
    hex::encode(octets)
}

/// Empreinte d'un secret de poste : la seule chose que le poste principal en garde.
pub fn empreinte_secret(secret: &str) -> String {
    hex::encode(Sha256::digest(format!("gesco-poste-relie:{secret}")))
}

/// Vrai si le secret présenté est celui dont on garde l'empreinte.
pub fn secret_correspond(secret: &str, empreinte: &str) -> bool {
    egal_en_temps_constant(empreinte_secret(secret).as_bytes(), empreinte.as_bytes())
}

/// Empreinte d'un certificat (son contenu en DER), en hexadécimal.
pub fn empreinte_certificat(der: &[u8]) -> String {
    hex::encode(Sha256::digest(der))
}

/// Empreinte courte, à comparer à l'œil entre deux écrans : quatre groupes de quatre caractères.
pub fn empreinte_courte(empreinte: &str) -> String {
    let debut: String = empreinte
        .chars()
        .filter(char::is_ascii_hexdigit)
        .take(16)
        .flat_map(char::to_uppercase)
        .collect();
    debut
        .as_bytes()
        .chunks(4)
        .map(|groupe| String::from_utf8_lossy(groupe).into_owned())
        .collect::<Vec<_>>()
        .join("-")
}

/// Nom donné à un poste, nettoyé ; refusé s'il est vide, trop long ou porte un caractère de
/// commande.
pub fn nom_de_poste(saisie: &str) -> Result<String, &'static str> {
    let nom = saisie.trim();
    if nom.is_empty() {
        return Err("reseau.nom_poste_vide");
    }
    if nom.chars().count() > LONGUEUR_NOM_POSTE_MAX || nom.chars().any(char::is_control) {
        return Err("reseau.nom_poste_invalide");
    }
    Ok(nom.to_owned())
}

/// Postes reliés que la licence couvre, en plus du poste principal (ADR « licence et postes »).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(tag = "type", content = "nombre", rename_all = "snake_case")]
pub enum PostesCouverts {
    /// Formule « postes compris » : autant qu'on veut.
    Illimites,
    /// Formule par poste, ou poste à l'essai quand la licence ne dit rien.
    Nombre(u32),
}

impl PostesCouverts {
    /// Lit la mention des postes parmi les modules d'une licence. Sans mention, ou sans licence
    /// (`modules` vide), un poste à l'essai. Une mention illisible ne donne rien de plus.
    pub fn depuis_modules(modules: &[String]) -> Self {
        modules
            .iter()
            .filter_map(|module| Self::depuis_mention(module))
            .next()
            .unwrap_or(Self::Nombre(POSTES_A_L_ESSAI))
    }

    /// Lit une mention `postes:illimites` ou `postes:N` ; `None` pour un autre module ou une
    /// mention mal écrite.
    pub fn depuis_mention(module: &str) -> Option<Self> {
        match module.strip_prefix(MENTION_POSTES)? {
            "illimites" => Some(Self::Illimites),
            nombre => nombre.parse().ok().map(Self::Nombre),
        }
    }

    /// Vrai si un poste de plus peut être relié quand `en_service` le sont déjà.
    pub fn admet_un_de_plus(&self, en_service: u32) -> bool {
        match self {
            Self::Illimites => true,
            Self::Nombre(n) => en_service < *n,
        }
    }
}

/// Poste relié au poste principal. Rien ne se supprime : un poste se révoque.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
pub struct PosteRelie {
    /// Identifiant technique ; le poste relié le présente à chaque appel.
    pub id: Id<PosteRelie>,
    /// Établissement.
    pub etablissement_id: Id<Etablissement>,
    /// Nom donné au poste : c'est lui qui signe ses écritures au journal.
    pub nom: String,
    /// Empreinte du secret du poste ; jamais rendue à l'écran.
    #[serde(skip_serializing, default)]
    #[specta(skip)]
    pub empreinte_secret: String,
    /// Instant de l'appairage.
    pub relie_le: Horodatage,
    /// Qui a ouvert l'appairage sur le poste principal.
    pub relie_par: Option<Id<Utilisateur>>,
    /// Dernier appel reçu de ce poste.
    pub derniere_connexion: Option<Horodatage>,
    /// Instant de la révocation ; `None` tant que le poste est en service.
    pub revoque_le: Option<Horodatage>,
    /// Qui l'a révoqué.
    pub revoque_par: Option<Id<Utilisateur>>,
}

impl PosteRelie {
    /// Vrai tant que le poste n'est pas révoqué.
    pub fn en_service(&self) -> bool {
        self.revoque_le.is_none()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn a(millis: i64) -> Horodatage {
        Horodatage::depuis_millis_epoque(millis)
    }

    #[test]
    fn le_code_n_a_ni_o_ni_zero_ni_i_ni_un() {
        let code = code_appairage(&[0, 1, 7, 8, 31, 32, 200, 255]);
        assert_eq!(code.len(), LONGUEUR_CODE_APPAIRAGE);
        assert!(code.chars().all(|c| !"O0I1".contains(c)));
        assert_eq!(code_appairage_lisible("ABCDEFGH"), "ABCD EFGH");
        assert_eq!(normaliser_code(" abcd-efgh "), "ABCDEFGH");
    }

    #[test]
    fn un_code_s_accepte_une_fois_dans_son_delai() {
        let mut code = CodeAppairage::nouveau("ABCDEFGH".to_owned(), a(0));
        assert_eq!(code.presenter("abcd efgh", a(1_000)), IssueCode::Accepte);
        assert_eq!(
            code.presenter("ABCDEFGH", a(VALIDITE_CODE_MILLIS)),
            IssueCode::Expire
        );
    }

    #[test]
    fn cinq_essais_faux_annulent_le_code() {
        let mut code = CodeAppairage::nouveau("ABCDEFGH".to_owned(), a(0));
        for _ in 1..ESSAIS_CODE_MAX {
            assert_eq!(code.presenter("ZZZZZZZZ", a(1)), IssueCode::Faux);
        }
        assert_eq!(code.presenter("ZZZZZZZZ", a(1)), IssueCode::Annule);
    }

    #[test]
    fn le_secret_ne_se_garde_que_par_son_empreinte() {
        let secret = secret_en_hexadecimal(&[7; LONGUEUR_SECRET]);
        assert_eq!(secret.len(), 64);
        let empreinte = empreinte_secret(&secret);
        assert_ne!(empreinte, secret);
        assert!(secret_correspond(&secret, &empreinte));
        assert!(!secret_correspond(
            &secret_en_hexadecimal(&[8; LONGUEUR_SECRET]),
            &empreinte
        ));
        assert!(!secret_correspond("", &empreinte));
    }

    #[test]
    fn l_empreinte_courte_se_lit_en_quatre_groupes() {
        let empreinte = empreinte_certificat(b"certificat");
        let courte = empreinte_courte(&empreinte);
        assert_eq!(courte.len(), 19);
        assert_eq!(courte.split('-').count(), 4);
        assert!(courte.chars().all(|c| c == '-' || c.is_ascii_hexdigit()));
        assert_eq!(
            empreinte_courte("a1b2c3d4e5f60718ffff"),
            "A1B2-C3D4-E5F6-0718"
        );
    }

    #[test]
    fn le_nom_d_un_poste_est_nettoye_et_borne() {
        assert_eq!(nom_de_poste("  Caisse  "), Ok("Caisse".to_owned()));
        assert_eq!(nom_de_poste("   "), Err("reseau.nom_poste_vide"));
        assert_eq!(
            nom_de_poste(&"x".repeat(LONGUEUR_NOM_POSTE_MAX + 1)),
            Err("reseau.nom_poste_invalide")
        );
        assert_eq!(nom_de_poste("a\nb"), Err("reseau.nom_poste_invalide"));
    }

    #[test]
    fn la_licence_dit_combien_de_postes_elle_couvre() {
        let modules =
            |liste: &[&str]| -> Vec<String> { liste.iter().map(|m| (*m).to_owned()).collect() };
        // Sans mention : un poste à l'essai.
        assert_eq!(
            PostesCouverts::depuis_modules(&[]),
            PostesCouverts::Nombre(1)
        );
        assert_eq!(
            PostesCouverts::depuis_modules(&modules(&["multi_etablissement"])),
            PostesCouverts::Nombre(1)
        );
        assert_eq!(
            PostesCouverts::depuis_modules(&modules(&["postes:5"])),
            PostesCouverts::Nombre(5)
        );
        assert_eq!(
            PostesCouverts::depuis_modules(&modules(&["autre", "postes:illimites"])),
            PostesCouverts::Illimites
        );
        // Une mention mal écrite ne donne rien de plus que l'essai.
        assert_eq!(
            PostesCouverts::depuis_modules(&modules(&["postes:beaucoup"])),
            PostesCouverts::Nombre(1)
        );
        assert_eq!(PostesCouverts::depuis_mention("postes:-3"), None);
        assert!(PostesCouverts::Nombre(1).admet_un_de_plus(0));
        assert!(!PostesCouverts::Nombre(1).admet_un_de_plus(1));
        assert!(PostesCouverts::Illimites.admet_un_de_plus(1_000));
    }
}
