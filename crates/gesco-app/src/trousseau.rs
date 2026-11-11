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

    /// Entrée où un poste relié range sa liaison au poste principal : adresse, empreinte
    /// complète, identifiant et secret du poste (SPEC_PHASE_11_A §2).
    const COMPTE_LIAISON: &str = "liaison-poste-principal";

    /// Liaison d'un poste relié, dans le gestionnaire d'identifiants de Windows.
    pub(super) struct CoffreWindows;

    impl super::CoffreLiaison for CoffreWindows {
        fn lire(&self) -> Resultat<Option<String>> {
            match entree(COMPTE_LIAISON)?.get_password() {
                Ok(contenu) => Ok(Some(contenu)),
                Err(keyring_core::Error::NoEntry) => Ok(None),
                Err(e) => Err(erreur(e)),
            }
        }

        fn ecrire(&self, contenu: &str) -> Resultat<()> {
            entree(COMPTE_LIAISON)?
                .set_password(contenu)
                .map_err(erreur)
        }

        fn effacer(&self) -> Resultat<()> {
            match entree(COMPTE_LIAISON)?.delete_credential() {
                Ok(()) | Err(keyring_core::Error::NoEntry) => Ok(()),
                Err(e) => Err(erreur(e)),
            }
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

/// Où un poste relié range ce qu'il retient du poste principal : l'adresse, l'empreinte
/// complète, son identifiant et son secret. Jamais dans un fichier lisible du poste.
pub(crate) trait CoffreLiaison: Send + Sync {
    /// La liaison rangée, s'il y en a une.
    fn lire(&self) -> Resultat<Option<String>>;
    /// Range la liaison.
    fn ecrire(&self, contenu: &str) -> Resultat<()>;
    /// Efface la liaison : le poste quitte le poste principal.
    fn effacer(&self) -> Resultat<()>;
}

/// Coffre du poste, à ouvrir après [`du_systeme`], qui choisit le trousseau de Windows.
pub(crate) fn coffre_du_systeme() -> Arc<dyn CoffreLiaison> {
    #[cfg(windows)]
    {
        Arc::new(identifiants::CoffreWindows)
    }
    #[cfg(not(windows))]
    {
        Arc::new(CoffreMemoire::default())
    }
}

/// Coffre des systèmes sans trousseau retenu : la liaison ne survit pas à la fermeture.
#[cfg(not(windows))]
#[derive(Default)]
struct CoffreMemoire(std::sync::Mutex<Option<String>>);

#[cfg(not(windows))]
impl CoffreLiaison for CoffreMemoire {
    fn lire(&self) -> Resultat<Option<String>> {
        Ok(self.0.lock().ok().and_then(|garde| garde.clone()))
    }

    fn ecrire(&self, contenu: &str) -> Resultat<()> {
        if let Ok(mut garde) = self.0.lock() {
            *garde = Some(contenu.to_owned());
        }
        Ok(())
    }

    fn effacer(&self) -> Resultat<()> {
        if let Ok(mut garde) = self.0.lock() {
            *garde = None;
        }
        Ok(())
    }
}

/// Coffre du mode d'essai : la liaison est un fichier du dossier d'essai. Sans aucune
/// protection : jamais utilisé hors de `GESCO_DOSSIER_ESSAI`.
pub(crate) struct CoffreEssai {
    fichier: std::path::PathBuf,
}

impl CoffreEssai {
    pub(crate) fn nouveau(dossier: &std::path::Path) -> Self {
        Self {
            fichier: dossier.join("liaison.json"),
        }
    }
}

impl CoffreLiaison for CoffreEssai {
    fn lire(&self) -> Resultat<Option<String>> {
        match std::fs::read_to_string(&self.fichier) {
            Ok(contenu) => Ok(Some(contenu)),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    fn ecrire(&self, contenu: &str) -> Resultat<()> {
        if let Some(dossier) = self.fichier.parent() {
            std::fs::create_dir_all(dossier)?;
        }
        std::fs::write(&self.fichier, contenu)?;
        Ok(())
    }

    fn effacer(&self) -> Resultat<()> {
        match std::fs::remove_file(&self.fichier) {
            Ok(()) => Ok(()),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(e) => Err(e.into()),
        }
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
