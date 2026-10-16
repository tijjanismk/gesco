//! Licence signée (blueprint §3.13 et §26.5, spécification §13).
//!
//! Un fichier de licence porte une charge JSON et sa signature Ed25519. La vérification est faite
//! ici, hors ligne et en Rust (R-02), contre la clé publique de l'éditeur que lui passe l'appelant.
//!
//! Les règles du §26.5 protègent le client : une licence absente, à réémettre ou échue ne bloque
//! jamais l'édition Essentielle ni l'accès aux données. Seuls les modules connectés s'éteignent,
//! et seulement après une période de grâce.

use ed25519_dalek::{Signature, Signer, SigningKey, VerifyingKey};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use thiserror::Error;

use crate::commun::Horodatage;

/// Version du format de fichier produite et lue par cette version de l'application.
pub const FORMAT_LICENCE: u32 = 1;

/// Empreinte que porte une licence libre : elle s'installe sur n'importe quel poste, qui s'y
/// attache à l'installation. Hors ligne, rien n'empêche d'installer le même fichier ailleurs ;
/// chaque numéro est tracé dans le journal d'audit du poste qui l'installe.
pub const EMPREINTE_LIBRE: &str = "LIBRE";

/// Grâce accordée aux modules connectés après une échéance ou un changement de machine : 90 jours.
pub const GRACE_MILLIS: i64 = 90 * 24 * 3_600 * 1_000;

/// Édition achetée (§26.1). Un seul binaire : l'édition n'active que des modules.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "snake_case")]
pub enum Edition {
    /// Licence perpétuelle, produit complet sans service récurrent (§26.2).
    Essentielle,
    /// Abonnement annuel aux services qui coûtent à l'éditeur (§26.3).
    Connectee,
}

/// Contenu signé d'une licence.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(deny_unknown_fields)]
pub struct Licence {
    /// Identifiant attribué par l'éditeur, cité au support.
    pub licence_id: String,
    /// Nom de l'établissement titulaire.
    pub etablissement: String,
    /// Édition.
    pub edition: Edition,
    /// Modules optionnels activés (§26.4).
    pub modules: Vec<String>,
    /// Empreinte du poste pour lequel la licence est émise (voir [`empreinte_machine`]).
    pub empreinte_machine: String,
    /// Instant d'émission.
    pub emise_le: Horodatage,
    /// Fin de l'abonnement des modules connectés ; absente pour une licence perpétuelle.
    pub echeance: Option<Horodatage>,
}

impl Licence {
    /// Licence libre, attachée au poste qui l'installe.
    pub fn est_libre(&self) -> bool {
        self.empreinte_machine == EMPREINTE_LIBRE
    }

    /// Empreinte du poste auquel la licence est liée : celle qu'elle porte, ou, pour une licence
    /// libre, celle du poste qui l'a installée.
    pub fn empreinte_liee<'a>(&'a self, attachee: Option<&'a str>) -> Option<&'a str> {
        if self.est_libre() {
            attachee
        } else {
            Some(&self.empreinte_machine)
        }
    }
}

/// Fichier de licence : la charge est gardée telle qu'elle a été signée, octet pour octet.
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Fichier {
    format: u32,
    charge: String,
    signature: String,
}

/// Licence illisible ou non authentique.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum ErreurLicence {
    /// Le fichier n'a pas la forme d'une licence GESCO.
    #[error("fichier de licence illisible")]
    Illisible,
    /// Le fichier vient d'une version plus récente du format.
    #[error("format de licence inconnu : {0}")]
    FormatInconnu(u32),
    /// La signature ne correspond pas à la clé de l'éditeur.
    #[error("signature de licence invalide")]
    SignatureInvalide,
    /// La clé fournie n'est pas une clé Ed25519 valide.
    #[error("clé de licence invalide")]
    CleInvalide,
}

/// Situation de la licence sur ce poste.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "snake_case")]
pub enum StatutLicence {
    /// Aucune licence installée : l'application fonctionne, un avertissement le signale.
    Absente,
    /// Licence authentique, émise pour ce poste et en cours.
    Valide,
    /// Licence authentique émise pour une autre machine : l'éditeur doit la réémettre.
    AReemettre,
    /// Abonnement des modules connectés échu.
    Echue,
}

/// Évaluation d'une licence à un instant donné.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
pub struct EtatLicence {
    /// Situation.
    pub statut: StatutLicence,
    /// Les modules connectés (synchronisation, portail…) peuvent fonctionner.
    pub modules_connectes_actifs: bool,
    /// Fin de la grâce en cours, quand une grâce court.
    pub fin_grace: Option<Horodatage>,
}

impl EtatLicence {
    /// État d'un poste sans licence.
    pub const ABSENTE: Self = Self {
        statut: StatutLicence::Absente,
        modules_connectes_actifs: false,
        fin_grace: None,
    };
}

/// Empreinte d'un poste, dérivée de l'identifiant stable de la machine (le `MachineGuid` de
/// Windows) : SHA-256 tronqué à 128 bits, en huit groupes de quatre caractères pour être dictée
/// au téléphone. L'identifiant brut ne quitte jamais le poste.
pub fn empreinte_machine(identifiant: &str) -> String {
    let condensat = Sha256::digest(format!("gesco-poste:{}", identifiant.trim().to_lowercase()));
    let hexa = hex::encode_upper(condensat.get(..16).unwrap_or_default());
    hexa.as_bytes()
        .chunks(4)
        .map(|groupe| String::from_utf8_lossy(groupe).into_owned())
        .collect::<Vec<_>>()
        .join("-")
}

/// Clé publique correspondant à une graine de clé privée.
pub fn cle_publique(graine: &[u8; 32]) -> [u8; 32] {
    SigningKey::from_bytes(graine).verifying_key().to_bytes()
}

/// Produit le fichier de licence signé avec la clé privée de l'éditeur (outil `gesco-licence`).
pub fn signer(licence: &Licence, graine: &[u8; 32]) -> Result<String, ErreurLicence> {
    let charge = serde_json::to_string(licence).map_err(|_| ErreurLicence::Illisible)?;
    let signature = SigningKey::from_bytes(graine).sign(charge.as_bytes());
    serde_json::to_string_pretty(&Fichier {
        format: FORMAT_LICENCE,
        charge,
        signature: hex::encode(signature.to_bytes()),
    })
    .map_err(|_| ErreurLicence::Illisible)
}

/// Vérifie un fichier de licence contre la clé publique de l'éditeur et rend son contenu.
pub fn verifier(texte: &str, cle_editeur: &[u8; 32]) -> Result<Licence, ErreurLicence> {
    let fichier: Fichier = serde_json::from_str(texte).map_err(|_| ErreurLicence::Illisible)?;
    if fichier.format != FORMAT_LICENCE {
        return Err(ErreurLicence::FormatInconnu(fichier.format));
    }
    let octets: [u8; 64] = hex::decode(&fichier.signature)
        .ok()
        .and_then(|o| o.try_into().ok())
        .ok_or(ErreurLicence::Illisible)?;
    let cle = VerifyingKey::from_bytes(cle_editeur).map_err(|_| ErreurLicence::CleInvalide)?;
    cle.verify_strict(fichier.charge.as_bytes(), &Signature::from_bytes(&octets))
        .map_err(|_| ErreurLicence::SignatureInvalide)?;
    serde_json::from_str(&fichier.charge).map_err(|_| ErreurLicence::Illisible)
}

/// Évalue une licence authentique pour ce poste.
///
/// `attachee` est l'empreinte du poste qui a installé une licence libre. `ecart_depuis` est
/// l'instant où le poste a constaté pour la première fois que l'empreinte ne correspondait pas :
/// la grâce des modules connectés court à partir de lui.
pub fn evaluer(
    licence: &Licence,
    attachee: Option<&str>,
    empreinte_poste: &str,
    ecart_depuis: Option<Horodatage>,
    maintenant: Horodatage,
) -> EtatLicence {
    let fin = |debut: Horodatage| debut.plus_millis(GRACE_MILLIS).unwrap_or(debut);
    let ecart = licence
        .empreinte_liee(attachee)
        .is_some_and(|liee| liee != empreinte_poste);
    let grace_ecart = ecart.then(|| fin(ecart_depuis.unwrap_or(maintenant)));
    let grace_echeance = licence.echeance.filter(|e| !maintenant.avant(e)).map(fin);
    let statut = if ecart {
        StatutLicence::AReemettre
    } else if grace_echeance.is_some() {
        StatutLicence::Echue
    } else {
        StatutLicence::Valide
    };
    let fin_grace = match (grace_ecart, grace_echeance) {
        (Some(a), Some(b)) => Some(a.min(b)),
        (a, b) => a.or(b),
    };
    let modules_connectes_actifs =
        licence.edition == Edition::Connectee && fin_grace.is_none_or(|f| maintenant.avant(&f));
    EtatLicence {
        statut,
        modules_connectes_actifs,
        fin_grace,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const GRAINE: [u8; 32] = [7; 32];
    const JOUR: i64 = 24 * 3_600 * 1_000;

    fn licence(edition: Edition, echeance: Option<i64>) -> Licence {
        Licence {
            licence_id: "LIC-2026-0001".to_owned(),
            etablissement: "École de test".to_owned(),
            edition,
            modules: vec![],
            empreinte_machine: empreinte_machine("poste-a"),
            emise_le: Horodatage::depuis_millis_epoque(0),
            echeance: echeance.map(Horodatage::depuis_millis_epoque),
        }
    }

    fn a(jours: i64) -> Horodatage {
        Horodatage::depuis_millis_epoque(jours * JOUR)
    }

    #[test]
    fn une_licence_signee_se_verifie_et_se_relit_a_l_identique() -> Result<(), ErreurLicence> {
        let l = licence(Edition::Connectee, Some(365 * JOUR));
        let fichier = signer(&l, &GRAINE)?;
        assert_eq!(verifier(&fichier, &cle_publique(&GRAINE)), Ok(l));
        Ok(())
    }

    #[test]
    fn une_charge_modifiee_ou_une_autre_cle_est_refusee() -> Result<(), ErreurLicence> {
        let fichier = signer(&licence(Edition::Essentielle, None), &GRAINE)?;
        let falsifie = fichier.replace("essentielle", "connectee");
        assert_ne!(falsifie, fichier);
        assert_eq!(
            verifier(&falsifie, &cle_publique(&GRAINE)),
            Err(ErreurLicence::SignatureInvalide)
        );
        assert_eq!(
            verifier(&fichier, &cle_publique(&[8; 32])),
            Err(ErreurLicence::SignatureInvalide)
        );
        assert_eq!(
            verifier("pas une licence", &cle_publique(&GRAINE)),
            Err(ErreurLicence::Illisible)
        );
        let futur = fichier.replace("\"format\": 1", "\"format\": 2");
        assert_eq!(
            verifier(&futur, &cle_publique(&GRAINE)),
            Err(ErreurLicence::FormatInconnu(2))
        );
        Ok(())
    }

    #[test]
    fn l_empreinte_est_stable_lisible_et_ne_revele_pas_l_identifiant() {
        let e = empreinte_machine("4C4C4544-0042-3510-8051-B4C04F395A32");
        assert_eq!(
            e,
            empreinte_machine(" 4c4c4544-0042-3510-8051-b4c04f395a32 ")
        );
        assert_eq!(e.len(), 39);
        assert_eq!(e.split('-').count(), 8);
        assert!(!e.contains("4C4C4544"));
        assert_ne!(e, empreinte_machine("autre-poste"));
    }

    #[test]
    fn l_edition_essentielle_n_est_jamais_bloquee() {
        let l = licence(Edition::Essentielle, None);
        let ici = evaluer(&l, None, &empreinte_machine("poste-a"), None, a(1_000));
        assert_eq!(ici.statut, StatutLicence::Valide);
        let ailleurs = evaluer(
            &l,
            None,
            &empreinte_machine("poste-b"),
            Some(a(0)),
            a(1_000),
        );
        assert_eq!(ailleurs.statut, StatutLicence::AReemettre);
        assert!(!ailleurs.modules_connectes_actifs);
    }

    #[test]
    fn une_licence_libre_s_attache_au_poste_qui_l_installe() -> Result<(), ErreurLicence> {
        let l = Licence {
            licence_id: "GESCO-0001".to_owned(),
            etablissement: String::new(),
            empreinte_machine: EMPREINTE_LIBRE.to_owned(),
            ..licence(Edition::Essentielle, None)
        };
        assert_eq!(
            verifier(&signer(&l, &GRAINE)?, &cle_publique(&GRAINE)),
            Ok(l.clone())
        );
        assert!(l.est_libre());
        let poste_a = empreinte_machine("poste-a");
        let poste_b = empreinte_machine("poste-b");
        let ici = evaluer(&l, Some(&poste_a), &poste_a, None, a(10));
        assert_eq!(ici.statut, StatutLicence::Valide);
        // La base restaurée sur un autre poste : la licence est à réinstaller.
        let ailleurs = evaluer(&l, Some(&poste_a), &poste_b, Some(a(10)), a(10));
        assert_eq!(ailleurs.statut, StatutLicence::AReemettre);
        // Une licence liée ignore l'attache.
        let liee = licence(Edition::Essentielle, None);
        assert_eq!(liee.empreinte_liee(Some(&poste_b)), Some(poste_a.as_str()));
        Ok(())
    }

    #[test]
    fn les_modules_connectes_survivent_quatre_vingt_dix_jours_a_un_changement_de_machine() {
        let l = licence(Edition::Connectee, None);
        let poste_b = empreinte_machine("poste-b");
        let debut = evaluer(&l, None, &poste_b, Some(a(10)), a(10));
        assert_eq!(debut.statut, StatutLicence::AReemettre);
        assert!(debut.modules_connectes_actifs);
        assert_eq!(debut.fin_grace, Some(a(100)));
        assert!(evaluer(&l, None, &poste_b, Some(a(10)), a(99)).modules_connectes_actifs);
        assert!(!evaluer(&l, None, &poste_b, Some(a(10)), a(100)).modules_connectes_actifs);
    }

    #[test]
    fn un_abonnement_echu_garde_ses_modules_pendant_la_grace() {
        let l = licence(Edition::Connectee, Some(365 * JOUR));
        let poste_a = empreinte_machine("poste-a");
        let avant = evaluer(&l, None, &poste_a, None, a(364));
        assert_eq!(avant.statut, StatutLicence::Valide);
        assert_eq!(avant.fin_grace, None);
        let echue = evaluer(&l, None, &poste_a, None, a(400));
        assert_eq!(echue.statut, StatutLicence::Echue);
        assert!(echue.modules_connectes_actifs);
        assert!(!evaluer(&l, None, &poste_a, None, a(455)).modules_connectes_actifs);
    }
}
