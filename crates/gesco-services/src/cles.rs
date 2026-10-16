//! Gestion de la clé de données sur le poste (spécification §6.3, R-57).
//!
//! `cles.json`, hors de la base, porte deux enveloppes de la clé : l'enveloppe machine, ouverte
//! par une clé du trousseau du système, et l'enveloppe de secours, ouverte par la phrase du
//! certificat. Le trousseau est une interface : DPAPI sous Windows dans `gesco-app`, mémoire dans
//! les tests.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use gesco_core::core::phrase::OCTETS_ENTROPIE;
use gesco_core::core::{
    CleDonnees, CleEnveloppe, Enveloppe, ErreurCle, NatureEnveloppe, ParametresDerivation,
    PhraseSecours,
};
use gesco_db::nouvel_id;
use serde::{Deserialize, Serialize};

use crate::alea;
use crate::erreur::{ErreurService, Resultat};
use crate::espace::ecrire_atomiquement;

/// Coffre du système où vit la clé de l'enveloppe machine. Une entrée par clé de données
/// (`cle_id`) : deux espaces de données d'un même compte Windows ne s'écrasent plus l'un
/// l'autre (Q-48 bis, `ALERTES.md` du 9 octobre 2026).
pub trait Trousseau: Send + Sync {
    /// Clé enregistrée pour cette clé de données, s'il y en a une.
    fn lire(&self, cle_id: &str) -> Resultat<Option<CleEnveloppe>>;
    /// Enregistre la clé de cette clé de données, en remplaçant la précédente.
    fn ecrire(&self, cle_id: &str, cle: &CleEnveloppe) -> Resultat<()>;
    /// Entrée unique des versions antérieures au 9 octobre 2026, partagée par toutes les
    /// installations du compte. Relue une seule fois, pour migrer un poste déjà installé ; jamais
    /// écrite ni effacée, puisqu'une autre installation peut encore en dépendre.
    fn lire_ancienne(&self) -> Resultat<Option<CleEnveloppe>> {
        Ok(None)
    }
}

/// Trousseau en mémoire : tests, et poste sans trousseau.
#[derive(Debug, Default)]
pub struct TrousseauMemoire {
    entrees: Mutex<HashMap<String, CleEnveloppe>>,
    ancienne: Mutex<Option<CleEnveloppe>>,
}

impl TrousseauMemoire {
    /// Trousseau qui ne porte que l'entrée unique des versions antérieures : un poste installé
    /// avant la correction du 9 octobre 2026.
    pub fn avec_ancienne_entree(cle: CleEnveloppe) -> Self {
        Self {
            entrees: Mutex::default(),
            ancienne: Mutex::new(Some(cle)),
        }
    }
}

impl Trousseau for TrousseauMemoire {
    fn lire(&self, cle_id: &str) -> Resultat<Option<CleEnveloppe>> {
        Ok(self
            .entrees
            .lock()
            .ok()
            .and_then(|e| e.get(cle_id).cloned()))
    }

    fn ecrire(&self, cle_id: &str, cle: &CleEnveloppe) -> Resultat<()> {
        if let Ok(mut e) = self.entrees.lock() {
            e.insert(cle_id.to_owned(), cle.clone());
        }
        Ok(())
    }

    fn lire_ancienne(&self) -> Resultat<Option<CleEnveloppe>> {
        Ok(self.ancienne.lock().ok().and_then(|c| c.clone()))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct FichierCles {
    version: u16,
    cle_id: String,
    enveloppes: Vec<Enveloppe>,
}

impl FichierCles {
    fn enveloppe(&self, nature: NatureEnveloppe) -> Option<&Enveloppe> {
        self.enveloppes.iter().find(|e| e.nature == nature)
    }

    fn remplacer(&mut self, enveloppe: Enveloppe) {
        self.enveloppes.retain(|e| e.nature != enveloppe.nature);
        self.enveloppes.push(enveloppe);
    }
}

/// Résultat de la préparation des clés au démarrage.
#[derive(Debug)]
pub enum EtatCles {
    /// Clé de données disponible.
    Ouvertes(CleDonnees),
    /// L'enveloppe machine ne s'ouvre pas (trousseau perdu, autre poste) : la phrase de secours
    /// est demandée.
    PhraseRequise,
}

/// Enveloppes de la clé de données d'une installation.
pub struct EspaceCles {
    chemin: PathBuf,
    trousseau: Arc<dyn Trousseau>,
}

impl std::fmt::Debug for EspaceCles {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("EspaceCles")
            .field("chemin", &self.chemin)
            .finish_non_exhaustive()
    }
}

fn enveloppe_machine(
    cle_id: &str,
    cle: &CleDonnees,
    cle_trousseau: &CleEnveloppe,
) -> Resultat<Enveloppe> {
    Ok(Enveloppe::sceller(
        nouvel_id::<Enveloppe>().vers_chaine(),
        NatureEnveloppe::Machine,
        cle_id.to_owned(),
        None,
        alea::octets()?,
        cle_trousseau,
        cle,
    )?)
}

impl EspaceCles {
    /// Clés stockées dans `chemin`, enveloppe machine protégée par `trousseau`.
    pub fn nouveau(chemin: impl Into<PathBuf>, trousseau: Arc<dyn Trousseau>) -> Self {
        Self {
            chemin: chemin.into(),
            trousseau,
        }
    }

    fn lire(&self) -> Resultat<Option<FichierCles>> {
        if !self.chemin.is_file() {
            return Ok(None);
        }
        Ok(Some(serde_json::from_slice(&std::fs::read(&self.chemin)?)?))
    }

    fn ecrire(&self, fichier: &FichierCles) -> Resultat<()> {
        Ok(ecrire_atomiquement(
            &self.chemin,
            &serde_json::to_vec_pretty(fichier)?,
        )?)
    }

    /// Ouvre la clé de données par l'enveloppe machine ; au tout premier lancement, tire une clé
    /// de données et son enveloppe machine. L'enveloppe de secours naît au premier démarrage
    /// (création de l'établissement), avec l'affichage unique de la phrase.
    pub fn preparer(&self) -> Resultat<EtatCles> {
        let Some(fichier) = self.lire()? else {
            let cle = CleDonnees::depuis_octets(alea::octets()?);
            let cle_trousseau = CleEnveloppe::depuis_octets(alea::octets()?);
            let cle_id = nouvel_id::<CleDonnees>().vers_chaine();
            let machine = enveloppe_machine(&cle_id, &cle, &cle_trousseau)?;
            self.trousseau.ecrire(&cle_id, &cle_trousseau)?;
            self.ecrire(&FichierCles {
                version: 1,
                cle_id,
                enveloppes: vec![machine],
            })?;
            return Ok(EtatCles::Ouvertes(cle));
        };
        let Some(machine) = fichier.enveloppe(NatureEnveloppe::Machine) else {
            return Ok(EtatCles::PhraseRequise);
        };
        if let Some(cle) = self
            .trousseau
            .lire(&fichier.cle_id)?
            .and_then(|c| machine.ouvrir(&c).ok())
        {
            return Ok(EtatCles::Ouvertes(cle));
        }
        // Poste installé avant la correction : l'entrée unique d'autrefois ouvre peut-être encore
        // cette clé. Si oui, elle est recopiée sous le nom propre à cette clé de données.
        if let Some(ancienne) = self.trousseau.lire_ancienne()?
            && let Ok(cle) = machine.ouvrir(&ancienne)
        {
            self.trousseau.ecrire(&fichier.cle_id, &ancienne)?;
            return Ok(EtatCles::Ouvertes(cle));
        }
        Ok(EtatCles::PhraseRequise)
    }

    /// Identifiant de la clé de données de l'installation.
    pub fn cle_id(&self) -> Resultat<Option<String>> {
        Ok(self.lire()?.map(|f| f.cle_id))
    }

    /// Enveloppe de secours en vigueur.
    pub fn enveloppe_secours(&self) -> Resultat<Option<Enveloppe>> {
        Ok(self
            .lire()?
            .and_then(|f| f.enveloppe(NatureEnveloppe::Secours).cloned()))
    }

    /// Ouvre la clé de données par la phrase de secours, puis scelle une nouvelle enveloppe
    /// machine pour ce poste : les démarrages suivants ne demandent plus rien.
    pub fn ouvrir_par_phrase(&self, phrase: &PhraseSecours) -> Resultat<CleDonnees> {
        let mut fichier = self.lire()?.ok_or(ErreurService::Regle {
            cle: "cles.absentes",
        })?;
        let secours = fichier
            .enveloppe(NatureEnveloppe::Secours)
            .ok_or(ErreurService::Regle {
                cle: "cles.secours_absente",
            })?;
        let derivation = secours.derivation.ok_or(ErreurCle::EnveloppeFermee)?;
        let cle = secours.ouvrir(&phrase.deriver(&derivation)?)?;
        let cle_trousseau = CleEnveloppe::depuis_octets(alea::octets()?);
        let machine = enveloppe_machine(&fichier.cle_id, &cle, &cle_trousseau)?;
        self.trousseau.ecrire(&fichier.cle_id, &cle_trousseau)?;
        fichier.remplacer(machine);
        self.ecrire(&fichier)?;
        Ok(cle)
    }

    /// Tire une nouvelle phrase et remplace l'enveloppe de secours. L'ancienne enveloppe est
    /// détruite : l'ancienne phrase n'ouvre plus la clé du poste ni les sauvegardes suivantes.
    pub fn regenerer_secours(&self, cle: &CleDonnees) -> Resultat<(PhraseSecours, Enveloppe)> {
        let mut fichier = self.lire()?.ok_or(ErreurService::Regle {
            cle: "cles.absentes",
        })?;
        let phrase = PhraseSecours::depuis_entropie(&alea::octets::<OCTETS_ENTROPIE>()?);
        let derivation = ParametresDerivation::recommandes(alea::octets()?);
        let secours = Enveloppe::sceller(
            nouvel_id::<Enveloppe>().vers_chaine(),
            NatureEnveloppe::Secours,
            fichier.cle_id.clone(),
            Some(derivation),
            alea::octets()?,
            &phrase.deriver(&derivation)?,
            cle,
        )?;
        fichier.remplacer(secours.clone());
        self.ecrire(&fichier)?;
        Ok((phrase, secours))
    }

    /// Installe une clé restaurée d'une sauvegarde (UC-07) : nouvelle enveloppe machine pour ce
    /// poste, enveloppe de secours reprise de l'archive.
    pub fn installer(&self, cle: &CleDonnees, secours: Enveloppe) -> Resultat<()> {
        let cle_trousseau = CleEnveloppe::depuis_octets(alea::octets()?);
        let machine = enveloppe_machine(&secours.cle_id, cle, &cle_trousseau)?;
        self.trousseau.ecrire(&secours.cle_id, &cle_trousseau)?;
        self.ecrire(&FichierCles {
            version: 1,
            cle_id: secours.cle_id.clone(),
            enveloppes: vec![machine, secours],
        })
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::panic)]
mod tests {
    use super::*;

    fn ouverte(etat: EtatCles) -> CleDonnees {
        match etat {
            EtatCles::Ouvertes(cle) => cle,
            EtatCles::PhraseRequise => panic!("phrase demandée"),
        }
    }

    #[test]
    fn deux_espaces_d_un_meme_compte_gardent_chacun_leur_cle() {
        let dossier = tempfile::tempdir().unwrap();
        let trousseau: Arc<dyn Trousseau> = Arc::new(TrousseauMemoire::default());
        let premier = EspaceCles::nouveau(dossier.path().join("a.json"), Arc::clone(&trousseau));
        let second = EspaceCles::nouveau(dossier.path().join("b.json"), Arc::clone(&trousseau));
        let cle_a = ouverte(premier.preparer().unwrap());
        // Le second espace naît après : avant la correction, il écrasait l'entrée du premier.
        let cle_b = ouverte(second.preparer().unwrap());
        assert_eq!(
            ouverte(premier.preparer().unwrap()).octets(),
            cle_a.octets()
        );
        assert_eq!(ouverte(second.preparer().unwrap()).octets(), cle_b.octets());
        assert_ne!(cle_a.octets(), cle_b.octets());
    }

    #[test]
    fn un_poste_installe_avant_la_correction_migre_sans_phrase() {
        let dossier = tempfile::tempdir().unwrap();
        let chemin = dossier.path().join("cles.json");
        // Installation d'autrefois : la clé du trousseau sous l'entrée unique.
        let source = Arc::new(TrousseauMemoire::default());
        let espace = EspaceCles::nouveau(&chemin, Arc::clone(&source) as Arc<dyn Trousseau>);
        let cle = ouverte(espace.preparer().unwrap());
        let cle_id = espace.cle_id().unwrap().unwrap();
        let cle_trousseau = source.lire(&cle_id).unwrap().unwrap();

        let migre = Arc::new(TrousseauMemoire::avec_ancienne_entree(
            cle_trousseau.clone(),
        ));
        let poste = EspaceCles::nouveau(&chemin, Arc::clone(&migre) as Arc<dyn Trousseau>);
        assert_eq!(ouverte(poste.preparer().unwrap()).octets(), cle.octets());
        // Recopiée sous son propre nom : l'entrée unique ne sert plus.
        assert_eq!(
            migre.lire(&cle_id).unwrap().unwrap().octets(),
            cle_trousseau.octets()
        );
    }

    #[test]
    fn une_ancienne_entree_etrangere_n_ouvre_rien() {
        let dossier = tempfile::tempdir().unwrap();
        let chemin = dossier.path().join("cles.json");
        let espace = EspaceCles::nouveau(
            &chemin,
            Arc::new(TrousseauMemoire::default()) as Arc<dyn Trousseau>,
        );
        ouverte(espace.preparer().unwrap());
        // L'entrée unique appartient à une autre installation : la phrase est demandée.
        let autre = CleEnveloppe::depuis_octets([7; 32]);
        let poste = EspaceCles::nouveau(
            &chemin,
            Arc::new(TrousseauMemoire::avec_ancienne_entree(autre)) as Arc<dyn Trousseau>,
        );
        assert!(matches!(poste.preparer().unwrap(), EtatCles::PhraseRequise));
    }
}
