//! Clé de données et enveloppes (blueprint §17.3, spécification §6.3, R-57).
//!
//! Une clé de données aléatoire de 256 bits chiffre la base et les sauvegardes. Elle n'est jamais
//! écrite en clair : elle est conservée sous deux enveloppes, l'une ouverte par une clé du
//! trousseau du poste, l'autre par une clé dérivée de la phrase de secours. Le noyau scelle et
//! ouvre les enveloppes ; l'aléa et le stockage lui sont fournis (R-03).

use chacha20poly1305::aead::{Aead, Payload};
use chacha20poly1305::{KeyInit, XChaCha20Poly1305, XNonce};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use zeroize::{Zeroize, ZeroizeOnDrop, Zeroizing};

/// Erreurs de la gestion des clés.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum ErreurCle {
    /// L'enveloppe ne s'ouvre pas avec cette clé : phrase fausse, trousseau d'un autre poste ou
    /// enveloppe altérée. Les trois cas sont délibérément indiscernables.
    #[error("enveloppe impossible à ouvrir")]
    EnveloppeFermee,
    /// Paramètres de dérivation refusés par Argon2.
    #[error("paramètres de dérivation invalides : {0}")]
    Derivation(String),
    /// Bloc de sauvegarde altéré, tronqué ou chiffré par une autre clé.
    #[error("bloc chiffré invalide")]
    BlocInvalide,
    /// Phrase de secours mal formée.
    #[error("phrase de secours : {0}")]
    Phrase(&'static str),
}

/// Clé de données. Son affichage est masqué et sa mémoire effacée à la libération (R-25).
#[derive(Clone, PartialEq, Eq, Zeroize, ZeroizeOnDrop)]
pub struct CleDonnees([u8; 32]);

impl std::fmt::Debug for CleDonnees {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("CleDonnees(****)")
    }
}

impl CleDonnees {
    /// Clé tirée d'un aléa de 256 bits fourni par l'appelant.
    pub const fn depuis_octets(octets: [u8; 32]) -> Self {
        Self(octets)
    }

    /// Octets de la clé.
    pub const fn octets(&self) -> &[u8; 32] {
        &self.0
    }

    /// Forme hexadécimale, pour `PRAGMA key` ; effacée à la libération.
    pub fn en_hexadecimal(&self) -> Zeroizing<String> {
        Zeroizing::new(hex::encode(self.0))
    }
}

/// Clé qui ouvre une enveloppe : clé du trousseau ou clé dérivée de la phrase de secours.
#[derive(Clone, PartialEq, Eq, Zeroize, ZeroizeOnDrop)]
pub struct CleEnveloppe([u8; 32]);

impl std::fmt::Debug for CleEnveloppe {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("CleEnveloppe(****)")
    }
}

impl CleEnveloppe {
    /// Clé d'enveloppe à partir de ses octets.
    pub const fn depuis_octets(octets: [u8; 32]) -> Self {
        Self(octets)
    }

    /// Octets de la clé.
    pub const fn octets(&self) -> &[u8; 32] {
        &self.0
    }
}

/// Ce qui protège une enveloppe.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NatureEnveloppe {
    /// Clé conservée dans le trousseau du système : déverrouillage transparent sur le poste.
    Machine,
    /// Clé dérivée de la phrase de secours : restauration sur un autre poste (UC-07).
    Secours,
}

impl NatureEnveloppe {
    const fn nom(self) -> &'static str {
        match self {
            Self::Machine => "machine",
            Self::Secours => "secours",
        }
    }
}

/// Paramètres Argon2id de la dérivation de la phrase de secours, conservés avec l'enveloppe pour
/// qu'une enveloppe ancienne reste ouvrable si la recommandation évolue.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ParametresDerivation {
    /// Sel aléatoire.
    #[serde(with = "hex::serde")]
    pub sel: [u8; 16],
    /// Mémoire en kibioctets.
    pub memoire_kio: u32,
    /// Nombre de passes.
    pub iterations: u32,
    /// Parallélisme.
    pub parallelisme: u32,
}

impl ParametresDerivation {
    /// Recommandation : 64 Mio, trois passes. La dérivation n'a lieu qu'à la restauration et à la
    /// régénération : elle peut être lente, elle protège une phrase imprimée.
    pub const fn recommandes(sel: [u8; 16]) -> Self {
        Self {
            sel,
            memoire_kio: 64 * 1024,
            iterations: 3,
            parallelisme: 1,
        }
    }

    /// Dérive une clé d'enveloppe d'un secret (la phrase de secours sous sa forme canonique).
    pub fn deriver(&self, secret: &[u8]) -> Result<CleEnveloppe, ErreurCle> {
        let params = argon2::Params::new(
            self.memoire_kio,
            self.iterations,
            self.parallelisme,
            Some(32),
        )
        .map_err(|e| ErreurCle::Derivation(e.to_string()))?;
        let argon =
            argon2::Argon2::new(argon2::Algorithm::Argon2id, argon2::Version::V0x13, params);
        let mut sortie = [0u8; 32];
        argon
            .hash_password_into(secret, &self.sel, &mut sortie)
            .map_err(|e| ErreurCle::Derivation(e.to_string()))?;
        let cle = CleEnveloppe(sortie);
        sortie.zeroize();
        Ok(cle)
    }
}

/// La clé de données chiffrée sous une clé d'enveloppe. L'identifiant, la nature et la clé
/// enveloppée sont liés au chiffré : déplacer une enveloppe d'un fichier à l'autre la rend
/// inutilisable.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Enveloppe {
    /// Identifiant de l'enveloppe, imprimé sur le certificat de secours.
    pub id: String,
    /// Ce qui la protège.
    pub nature: NatureEnveloppe,
    /// Identifiant de la clé de données enveloppée.
    pub cle_id: String,
    /// Paramètres de dérivation, pour une enveloppe de secours.
    pub derivation: Option<ParametresDerivation>,
    /// Nonce XChaCha20-Poly1305.
    #[serde(with = "hex::serde")]
    pub nonce: [u8; 24],
    /// Clé de données chiffrée et son étiquette d'authentification.
    #[serde(with = "hex::serde")]
    pub chiffre: Vec<u8>,
}

impl Enveloppe {
    fn donnees_associees(id: &str, nature: NatureEnveloppe, cle_id: &str) -> Vec<u8> {
        format!("gesco-enveloppe-v1|{id}|{}|{cle_id}", nature.nom()).into_bytes()
    }

    /// Scelle la clé de données. Le nonce est un aléa de 192 bits fourni par l'appelant.
    pub fn sceller(
        id: String,
        nature: NatureEnveloppe,
        cle_id: String,
        derivation: Option<ParametresDerivation>,
        nonce: [u8; 24],
        cle_enveloppe: &CleEnveloppe,
        cle: &CleDonnees,
    ) -> Result<Self, ErreurCle> {
        let aad = Self::donnees_associees(&id, nature, &cle_id);
        let chiffreur = XChaCha20Poly1305::new(&cle_enveloppe.0.into());
        let chiffre = chiffreur
            .encrypt(
                &XNonce::from(nonce),
                Payload {
                    msg: cle.octets(),
                    aad: &aad,
                },
            )
            .map_err(|_| ErreurCle::EnveloppeFermee)?;
        Ok(Self {
            id,
            nature,
            cle_id,
            derivation,
            nonce,
            chiffre,
        })
    }

    /// Ouvre l'enveloppe.
    pub fn ouvrir(&self, cle_enveloppe: &CleEnveloppe) -> Result<CleDonnees, ErreurCle> {
        let aad = Self::donnees_associees(&self.id, self.nature, &self.cle_id);
        let chiffreur = XChaCha20Poly1305::new(&cle_enveloppe.0.into());
        let clair = Zeroizing::new(
            chiffreur
                .decrypt(
                    &XNonce::from(self.nonce),
                    Payload {
                        msg: &self.chiffre,
                        aad: &aad,
                    },
                )
                .map_err(|_| ErreurCle::EnveloppeFermee)?,
        );
        let octets: [u8; 32] = clair
            .as_slice()
            .try_into()
            .map_err(|_| ErreurCle::EnveloppeFermee)?;
        Ok(CleDonnees(octets))
    }
}

/// Chiffre un bloc de sauvegarde (construction STREAM) : le nonce porte le préfixe de l'archive
/// et le rang du bloc, les données associées l'empreinte du manifeste et le marqueur de dernier
/// bloc. Un bloc déplacé, retiré ou ajouté, un manifeste modifié ou une archive tronquée sont
/// détectés au déchiffrement.
pub fn chiffrer_bloc(
    cle: &CleDonnees,
    prefixe: &[u8; 16],
    rang: u64,
    dernier: bool,
    empreinte_manifeste: &[u8; 32],
    clair: &[u8],
) -> Result<Vec<u8>, ErreurCle> {
    let (nonce, aad) = nonce_et_aad(prefixe, rang, dernier, empreinte_manifeste);
    XChaCha20Poly1305::new(&cle.0.into())
        .encrypt(
            &nonce,
            Payload {
                msg: clair,
                aad: &aad,
            },
        )
        .map_err(|_| ErreurCle::BlocInvalide)
}

/// Déchiffre un bloc chiffré par [`chiffrer_bloc`].
pub fn dechiffrer_bloc(
    cle: &CleDonnees,
    prefixe: &[u8; 16],
    rang: u64,
    dernier: bool,
    empreinte_manifeste: &[u8; 32],
    chiffre: &[u8],
) -> Result<Vec<u8>, ErreurCle> {
    let (nonce, aad) = nonce_et_aad(prefixe, rang, dernier, empreinte_manifeste);
    XChaCha20Poly1305::new(&cle.0.into())
        .decrypt(
            &nonce,
            Payload {
                msg: chiffre,
                aad: &aad,
            },
        )
        .map_err(|_| ErreurCle::BlocInvalide)
}

fn nonce_et_aad(
    prefixe: &[u8; 16],
    rang: u64,
    dernier: bool,
    empreinte_manifeste: &[u8; 32],
) -> (XNonce, [u8; 33]) {
    let mut nonce = [0u8; 24];
    nonce[..16].copy_from_slice(prefixe);
    nonce[16..].copy_from_slice(&rang.to_be_bytes());
    let mut aad = [0u8; 33];
    aad[..32].copy_from_slice(empreinte_manifeste);
    aad[32] = u8::from(dernier);
    (XNonce::from(nonce), aad)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cle(octet: u8) -> CleDonnees {
        CleDonnees::depuis_octets([octet; 32])
    }

    fn enveloppe(cle_env: &CleEnveloppe) -> Result<Enveloppe, ErreurCle> {
        Enveloppe::sceller(
            "ENV1".to_owned(),
            NatureEnveloppe::Machine,
            "CLE1".to_owned(),
            None,
            [7; 24],
            cle_env,
            &cle(3),
        )
    }

    #[test]
    fn une_enveloppe_s_ouvre_avec_sa_cle_seulement() -> Result<(), ErreurCle> {
        let bonne = CleEnveloppe::depuis_octets([1; 32]);
        let e = enveloppe(&bonne)?;
        assert_eq!(e.ouvrir(&bonne)?, cle(3));
        assert_eq!(
            e.ouvrir(&CleEnveloppe::depuis_octets([2; 32])),
            Err(ErreurCle::EnveloppeFermee)
        );
        Ok(())
    }

    #[test]
    fn une_enveloppe_deplacee_ou_alteree_ne_s_ouvre_plus() -> Result<(), ErreurCle> {
        let bonne = CleEnveloppe::depuis_octets([1; 32]);
        let mut renommee = enveloppe(&bonne)?;
        renommee.cle_id = "CLE2".to_owned();
        assert_eq!(renommee.ouvrir(&bonne), Err(ErreurCle::EnveloppeFermee));
        let mut alteree = enveloppe(&bonne)?;
        if let Some(octet) = alteree.chiffre.first_mut() {
            *octet ^= 1;
        }
        assert_eq!(alteree.ouvrir(&bonne), Err(ErreurCle::EnveloppeFermee));
        Ok(())
    }

    #[test]
    fn les_cles_ne_s_affichent_pas() {
        assert_eq!(format!("{:?}", cle(9)), "CleDonnees(****)");
        assert_eq!(
            format!("{:?}", CleEnveloppe::depuis_octets([9; 32])),
            "CleEnveloppe(****)"
        );
    }

    #[test]
    fn un_bloc_n_est_lisible_qu_a_son_rang_et_avec_son_manifeste() -> Result<(), ErreurCle> {
        let c = cle(5);
        let prefixe = [4; 16];
        let manifeste = [8; 32];
        let chiffre = chiffrer_bloc(&c, &prefixe, 2, false, &manifeste, b"pages de la base")?;
        assert_eq!(
            dechiffrer_bloc(&c, &prefixe, 2, false, &manifeste, &chiffre)?,
            b"pages de la base"
        );
        assert!(dechiffrer_bloc(&c, &prefixe, 3, false, &manifeste, &chiffre).is_err());
        assert!(dechiffrer_bloc(&c, &prefixe, 2, true, &manifeste, &chiffre).is_err());
        assert!(dechiffrer_bloc(&c, &prefixe, 2, false, &[9; 32], &chiffre).is_err());
        assert!(dechiffrer_bloc(&cle(6), &prefixe, 2, false, &manifeste, &chiffre).is_err());
        Ok(())
    }

    #[test]
    fn la_derivation_depend_du_secret_et_du_sel() -> Result<(), ErreurCle> {
        let legers = |sel| ParametresDerivation {
            sel,
            memoire_kio: 64,
            iterations: 1,
            parallelisme: 1,
        };
        let a = legers([1; 16]).deriver(b"abaisser abandon")?;
        assert_eq!(a, legers([1; 16]).deriver(b"abaisser abandon")?);
        assert_ne!(a, legers([2; 16]).deriver(b"abaisser abandon")?);
        assert_ne!(a, legers([1; 16]).deriver(b"abaisser abeille")?);
        Ok(())
    }
}
