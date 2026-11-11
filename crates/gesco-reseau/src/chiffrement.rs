//! Chiffrement du réseau local (ADR « sécurité du réseau local » du 10 novembre 2026) : le
//! poste principal fabrique son certificat ; un poste relié n'en retient que l'empreinte, vue à
//! l'appairage, et refuse tout poste principal qui en présente une autre. Aucune autorité ne
//! signe ce certificat : la confiance vient de l'empreinte retenue, pas d'une date ni d'un nom.

use std::fmt;
use std::sync::Arc;

use gesco_core::core::reseau::empreinte_certificat;
use rustls::client::danger::{HandshakeSignatureValid, ServerCertVerified, ServerCertVerifier};
use rustls::crypto::{CryptoProvider, ring, verify_tls12_signature, verify_tls13_signature};
use rustls::pki_types::{CertificateDer, PrivateKeyDer, PrivatePkcs8KeyDer, ServerName, UnixTime};
use rustls::{ClientConfig, DigitallySignedStruct, ServerConfig, SignatureScheme};

/// Nom porté par le certificat du poste principal, et demandé par les postes reliés. Il ne
/// sert qu'au protocole : c'est l'empreinte qui fait foi.
pub const NOM_CERTIFICAT: &str = "gesco-poste-principal";

/// Échec d'une opération de chiffrement : certificat illisible, clé refusée.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ErreurChiffrement(String);

impl fmt::Display for ErreurChiffrement {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "chiffrement du réseau local : {}", self.0)
    }
}

impl std::error::Error for ErreurChiffrement {}

fn erreur(source: impl fmt::Display) -> ErreurChiffrement {
    ErreurChiffrement(source.to_string())
}

/// Brique de chiffrement de rustls : `ring`, qui se compile avec le seul compilateur C.
fn fournisseur() -> Arc<CryptoProvider> {
    Arc::new(ring::default_provider())
}

/// Fabrique le certificat du poste principal et sa clé privée : le certificat en DER, la clé
/// en PKCS#8 (DER). Ils se gardent ensemble dans la base chiffrée.
pub fn fabriquer_certificat() -> Result<(Vec<u8>, Vec<u8>), ErreurChiffrement> {
    let cle = rcgen::KeyPair::generate().map_err(erreur)?;
    let parametres =
        rcgen::CertificateParams::new(vec![NOM_CERTIFICAT.to_owned()]).map_err(erreur)?;
    let certificat = parametres.self_signed(&cle).map_err(erreur)?;
    Ok((certificat.der().to_vec(), cle.serialize_der()))
}

/// Réglage du serveur : il présente ce certificat et ne demande aucun certificat au poste
/// relié, qui se fait reconnaître par son secret.
pub fn configuration_serveur(
    certificat: Vec<u8>,
    cle: Vec<u8>,
) -> Result<Arc<ServerConfig>, ErreurChiffrement> {
    let configuration = ServerConfig::builder_with_provider(fournisseur())
        .with_safe_default_protocol_versions()
        .map_err(erreur)?
        .with_no_client_auth()
        .with_single_cert(
            vec![CertificateDer::from(certificat)],
            PrivateKeyDer::Pkcs8(PrivatePkcs8KeyDer::from(cle)),
        )
        .map_err(erreur)?;
    Ok(Arc::new(configuration))
}

/// Vérification faite par un poste relié : le certificat présenté doit avoir l'empreinte
/// retenue à l'appairage. Ni sa date ni son nom ne comptent.
#[derive(Debug)]
struct EmpreinteRetenue {
    empreinte: String,
    fournisseur: Arc<CryptoProvider>,
}

impl ServerCertVerifier for EmpreinteRetenue {
    fn verify_server_cert(
        &self,
        end_entity: &CertificateDer<'_>,
        _intermediates: &[CertificateDer<'_>],
        _server_name: &ServerName<'_>,
        _ocsp_response: &[u8],
        _now: UnixTime,
    ) -> Result<ServerCertVerified, rustls::Error> {
        if empreinte_certificat(end_entity.as_ref()) == self.empreinte {
            Ok(ServerCertVerified::assertion())
        } else {
            Err(rustls::Error::General(
                "le poste principal présente une autre empreinte que celle retenue".to_owned(),
            ))
        }
    }

    fn verify_tls12_signature(
        &self,
        message: &[u8],
        cert: &CertificateDer<'_>,
        dss: &DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, rustls::Error> {
        verify_tls12_signature(
            message,
            cert,
            dss,
            &self.fournisseur.signature_verification_algorithms,
        )
    }

    fn verify_tls13_signature(
        &self,
        message: &[u8],
        cert: &CertificateDer<'_>,
        dss: &DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, rustls::Error> {
        verify_tls13_signature(
            message,
            cert,
            dss,
            &self.fournisseur.signature_verification_algorithms,
        )
    }

    fn supported_verify_schemes(&self) -> Vec<SignatureScheme> {
        self.fournisseur
            .signature_verification_algorithms
            .supported_schemes()
    }
}

/// Réglage d'un poste relié : il n'accepte que le poste principal dont le certificat a cette
/// empreinte (en hexadécimal, telle que [`empreinte_certificat`] la rend).
pub fn configuration_client(empreinte: &str) -> Result<Arc<ClientConfig>, ErreurChiffrement> {
    let fournisseur = fournisseur();
    let configuration = ClientConfig::builder_with_provider(Arc::clone(&fournisseur))
        .with_safe_default_protocol_versions()
        .map_err(erreur)?
        .dangerous()
        .with_custom_certificate_verifier(Arc::new(EmpreinteRetenue {
            empreinte: empreinte.to_owned(),
            fournisseur,
        }))
        .with_no_client_auth();
    Ok(Arc::new(configuration))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chaque_certificat_fabrique_a_sa_propre_empreinte() {
        let (premier, cle) = fabriquer_certificat().unwrap_or_default();
        let (second, _) = fabriquer_certificat().unwrap_or_default();
        assert!(!premier.is_empty() && !cle.is_empty());
        assert_ne!(
            empreinte_certificat(&premier),
            empreinte_certificat(&second)
        );
        assert!(configuration_serveur(premier.clone(), cle).is_ok());
        assert!(configuration_client(&empreinte_certificat(&premier)).is_ok());
        // Une clé qui n'en est pas une est refusée.
        assert!(configuration_serveur(premier, b"pas une cle".to_vec()).is_err());
    }
}
