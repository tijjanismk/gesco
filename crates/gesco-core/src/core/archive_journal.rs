//! Archive du journal d'audit (blueprint §12.4, SPEC_VERSION_1_1 §1, étape 5, T-73) : à la
//! clôture de l'année, le journal de l'année s'écrit dans un fichier signé. La signature est un
//! HMAC-SHA256 du contenu, sous une clé tirée de la clé de données de l'école : seul un GESCO qui
//! tient cette clé la produit ou la vérifie ; un octet changé dans le fichier la rompt.

use sha2::{Digest, Sha256};

/// Format de l'archive, écrit dans le fichier.
pub const FORMAT_ARCHIVE_JOURNAL: &str = "gesco-journal-1";

/// Extension du fichier d'archive.
pub const EXTENSION_ARCHIVE_JOURNAL: &str = "gja";

const BLOC: usize = 64;

/// Clé de signature tirée de la clé de données, séparée de tout autre usage.
fn cle_de_signature(cle_donnees: &[u8; 32]) -> [u8; 32] {
    let mut h = Sha256::new();
    h.update(b"gesco-archive-journal-v1");
    h.update(cle_donnees);
    h.finalize().into()
}

/// HMAC-SHA256 (RFC 2104) du contenu sous la clé.
fn hmac(cle: &[u8; 32], contenu: &[u8]) -> [u8; 32] {
    let mut ipad = [0x36u8; BLOC];
    let mut opad = [0x5cu8; BLOC];
    for (i, octet) in cle.iter().enumerate() {
        if let (Some(a), Some(b)) = (ipad.get_mut(i), opad.get_mut(i)) {
            *a ^= octet;
            *b ^= octet;
        }
    }
    let interieur: [u8; 32] = Sha256::new()
        .chain_update(ipad)
        .chain_update(contenu)
        .finalize()
        .into();
    Sha256::new()
        .chain_update(opad)
        .chain_update(interieur)
        .finalize()
        .into()
}

/// Signature hexadécimale d'un contenu d'archive.
pub fn signer(cle_donnees: &[u8; 32], contenu: &[u8]) -> String {
    hex::encode(hmac(&cle_de_signature(cle_donnees), contenu))
}

/// Vrai si la signature est celle du contenu sous cette clé ; la comparaison ne s'arrête pas au
/// premier octet différent.
pub fn verifier(cle_donnees: &[u8; 32], contenu: &[u8], signature: &str) -> bool {
    let attendue = signer(cle_donnees, contenu);
    attendue.len() == signature.len()
        && attendue
            .bytes()
            .zip(signature.bytes())
            .fold(0u8, |acc, (a, b)| acc | (a ^ b))
            == 0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn le_hmac_suit_la_rfc_4231() {
        // RFC 4231, cas 2 : clé « Jefe », message « what do ya want for nothing? » ; la clé est
        // ici complétée de zéros jusqu'à 32 octets, ce que HMAC fait de lui-même.
        let mut cle = [0u8; 32];
        for (octet, lettre) in cle.iter_mut().zip(b"Jefe") {
            *octet = *lettre;
        }
        assert_eq!(
            hex::encode(hmac(&cle, b"what do ya want for nothing?")),
            "5bdcc146bf60754e6a042426089575c75a003f089d2739839dec58b964ec3843"
        );
    }

    #[test]
    fn un_octet_change_rompt_la_signature() {
        let cle = [7u8; 32];
        let signature = signer(&cle, b"journal de 2026-2027");
        assert!(verifier(&cle, b"journal de 2026-2027", &signature));
        assert!(!verifier(&cle, b"journal de 2026-2028", &signature));
        assert!(!verifier(&[8u8; 32], b"journal de 2026-2027", &signature));
    }
}
