//! Trousseau du système qui protège l'enveloppe machine de la clé de données (spécification
//! §6.3). Sous Windows, le gestionnaire d'identifiants, chiffré par DPAPI pour la session de
//! l'utilisateur. Ailleurs, aucun trousseau n'est retenu en phase 0.5 (ADR-016 : Windows seul) :
//! la phrase de secours est demandée à chaque démarrage.

use std::sync::Arc;

use gesco_services::Resultat;
use gesco_services::cles::Trousseau;

#[cfg(windows)]
mod identifiants {
    use gesco_core::core::CleEnveloppe;
    use gesco_services::cles::Trousseau;
    use gesco_services::{ErreurService, Resultat};
    use zeroize::Zeroizing;

    const SERVICE: &str = "GESCO";
    /// Entrée unique des versions antérieures au 9 octobre 2026 : relue pour migrer, jamais
    /// écrite.
    const COMPTE_ANCIEN: &str = "cle-enveloppe-machine";
    /// Préfixe des entrées propres à chaque clé de données (Q-48 bis).
    const PREFIXE_COMPTE: &str = "cle-enveloppe-machine-";

    fn erreur(e: keyring_core::Error) -> ErreurService {
        ErreurService::Secret(e.to_string())
    }

    pub(super) struct TrousseauWindows;

    impl TrousseauWindows {
        pub(super) fn ouvrir() -> Resultat<Self> {
            keyring_core::set_default_store(
                windows_native_keyring_store::Store::new().map_err(erreur)?,
            );
            Ok(Self)
        }
    }

    fn entree(compte: &str) -> Resultat<keyring_core::Entry> {
        keyring_core::Entry::new(SERVICE, compte).map_err(erreur)
    }

    fn lire_entree(compte: &str) -> Resultat<Option<CleEnveloppe>> {
        match entree(compte)?.get_secret() {
            Ok(octets) => {
                let octets = Zeroizing::new(octets);
                let cle: [u8; 32] = octets.as_slice().try_into().map_err(|_| {
                    ErreurService::Secret("clé du trousseau de longueur inattendue".to_owned())
                })?;
                Ok(Some(CleEnveloppe::depuis_octets(cle)))
            }
            Err(keyring_core::Error::NoEntry) => Ok(None),
            Err(e) => Err(erreur(e)),
        }
    }

    impl Trousseau for TrousseauWindows {
        fn lire(&self, cle_id: &str) -> Resultat<Option<CleEnveloppe>> {
            lire_entree(&format!("{PREFIXE_COMPTE}{cle_id}"))
        }

        fn ecrire(&self, cle_id: &str, cle: &CleEnveloppe) -> Resultat<()> {
            entree(&format!("{PREFIXE_COMPTE}{cle_id}"))?
                .set_secret(cle.octets())
                .map_err(erreur)
        }

        fn lire_ancienne(&self) -> Resultat<Option<CleEnveloppe>> {
            lire_entree(COMPTE_ANCIEN)
        }
    }
}

/// Trousseau du poste.
pub(crate) fn du_systeme() -> Resultat<Arc<dyn Trousseau>> {
    #[cfg(windows)]
    {
        Ok(Arc::new(identifiants::TrousseauWindows::ouvrir()?))
    }
    #[cfg(not(windows))]
    {
        Ok(Arc::new(gesco_services::cles::TrousseauMemoire::default()))
    }
}

/// Trousseau du mode d'essai, des seules compilations de développement : la clé d'enveloppe est
/// un fichier du dossier d'essai, pour qu'une école d'essai se rouvre d'un lancement à l'autre.
/// Sans aucune protection : jamais utilisé hors de `GESCO_DOSSIER_ESSAI`.
pub(crate) struct TrousseauEssai {
    dossier: std::path::PathBuf,
}

impl TrousseauEssai {
    pub(crate) const fn nouveau(dossier: std::path::PathBuf) -> Self {
        Self { dossier }
    }

    fn chemin(&self, cle_id: &str) -> std::path::PathBuf {
        let nom: String = cle_id
            .chars()
            .map(|c| {
                if c.is_ascii_alphanumeric() || c == '-' {
                    c
                } else {
                    '_'
                }
            })
            .collect();
        self.dossier.join(format!("{nom}.cle"))
    }
}

impl Trousseau for TrousseauEssai {
    fn lire(&self, cle_id: &str) -> Resultat<Option<gesco_core::core::CleEnveloppe>> {
        let chemin = self.chemin(cle_id);
        if !chemin.exists() {
            return Ok(None);
        }
        let octets = std::fs::read(chemin)?;
        let cle: [u8; 32] = octets.as_slice().try_into().map_err(|_| {
            gesco_services::ErreurService::Secret("clé d'essai de longueur inattendue".to_owned())
        })?;
        Ok(Some(gesco_core::core::CleEnveloppe::depuis_octets(cle)))
    }

    fn ecrire(&self, cle_id: &str, cle: &gesco_core::core::CleEnveloppe) -> Resultat<()> {
        std::fs::create_dir_all(&self.dossier)?;
        std::fs::write(self.chemin(cle_id), cle.octets())?;
        Ok(())
    }
}
