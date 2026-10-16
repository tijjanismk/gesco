//! Journal d'audit chaîné (§12 du blueprint, R-26, R-27).
//!
//! Chaque ligne porte l'empreinte SHA-256 de la précédente et la sienne, calculée sur tous ses
//! champs. Modifier une ligne après coup rompt la chaîne, ce qu'une commande de vérification
//! détecte. Le calcul est ici, pur ; l'écriture est dans `gesco-db`.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use super::etablissement::Etablissement;
use super::utilisateur::{Poste, Utilisateur};
use crate::commun::{Horodatage, Id};

/// Empreinte SHA-256, 32 octets.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct EmpreinteAudit(#[serde(with = "hex_serde")] [u8; 32]);

// Sérialisée en hexadécimal.
impl specta::Type for EmpreinteAudit {
    fn definition(_types: &mut specta::Types) -> specta::datatype::DataType {
        specta::datatype::DataType::Primitive(specta::datatype::Primitive::str)
    }
}

mod hex_serde {
    use serde::{Deserialize, Deserializer, Serializer};

    pub fn serialize<S: Serializer>(octets: &[u8; 32], s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(&hex::encode(octets))
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<[u8; 32], D::Error> {
        let texte = String::deserialize(d)?;
        let octets = hex::decode(&texte).map_err(serde::de::Error::custom)?;
        <[u8; 32]>::try_from(octets)
            .map_err(|_| serde::de::Error::custom("empreinte de 32 octets attendue"))
    }
}

impl EmpreinteAudit {
    /// Empreinte d'amorçage d'un établissement : la première ligne chaîne dessus.
    pub fn amorce(etablissement_id: Id<Etablissement>) -> Self {
        let mut hachage = Sha256::new();
        hachage.update(b"gesco-audit-amorce:");
        hachage.update(etablissement_id.octets());
        Self(hachage.finalize().into())
    }

    /// Lecture depuis l'hexadécimal persisté.
    pub fn depuis_hex(texte: &str) -> Option<Self> {
        hex::decode(texte)
            .ok()
            .and_then(|o| <[u8; 32]>::try_from(o).ok())
            .map(Self)
    }

    /// Hexadécimal minuscule, 64 caractères.
    pub fn vers_hex(&self) -> String {
        hex::encode(self.0)
    }

    /// Octets bruts.
    pub const fn octets(&self) -> &[u8; 32] {
        &self.0
    }
}

/// Nature de la mutation auditée.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "snake_case")]
pub enum Operation {
    /// Création d'une entité.
    Creation,
    /// Modification de champs.
    Modification,
    /// Suppression logique.
    Suppression,
    /// Changement d'état d'une machine à états.
    Transition,
    /// Annulation par écriture inverse (ADR-010).
    Annulation,
    /// Connexion, déverrouillage, bascule d'utilisateur.
    Session,
    /// Accès à une fonction sensible : sauvegarde, restauration, export.
    Acces,
}

impl Operation {
    /// Nom persisté.
    pub const fn nom(&self) -> &'static str {
        match self {
            Self::Creation => "creation",
            Self::Modification => "modification",
            Self::Suppression => "suppression",
            Self::Transition => "transition",
            Self::Annulation => "annulation",
            Self::Session => "session",
            Self::Acces => "acces",
        }
    }

    /// Lecture d'un nom persisté.
    pub fn depuis_nom(nom: &str) -> Option<Self> {
        [
            Self::Creation,
            Self::Modification,
            Self::Suppression,
            Self::Transition,
            Self::Annulation,
            Self::Session,
            Self::Acces,
        ]
        .into_iter()
        .find(|o| o.nom() == nom)
    }
}

/// Une ligne du journal (§5.1 de la spécification).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
pub struct LigneAudit {
    /// Identifiant technique.
    pub id: Id<LigneAudit>,
    /// Instant de la mutation.
    pub horodatage: Horodatage,
    /// Utilisateur à l'origine ; absent pour les opérations du système.
    pub utilisateur_id: Option<Id<Utilisateur>>,
    /// Code du rôle actif au moment de l'opération.
    pub role_actif: Option<String>,
    /// Établissement.
    pub etablissement_id: Id<Etablissement>,
    /// Nom de l'entité touchée (`utilisateur`, `versement`…).
    pub entite: String,
    /// Identifiant de l'entité, en texte.
    pub entite_id: String,
    /// Nature de l'opération.
    pub operation: Operation,
    /// État avant, en JSON, sans aucun secret (R-25).
    pub avant: Option<serde_json::Value>,
    /// État après, même règle.
    pub apres: Option<serde_json::Value>,
    /// Module et écran d'origine.
    pub contexte: String,
    /// Poste émetteur.
    pub poste: Poste,
    /// Empreinte de la machine.
    pub empreinte_machine: String,
    /// Identifiant de corrélation partagé avec l'événement `outbox` de la même transaction.
    pub correlation_id: String,
    /// Empreinte de la ligne précédente.
    pub empreinte_precedente: EmpreinteAudit,
    /// Empreinte de cette ligne, calculée sur tous les champs ci-dessus.
    pub empreinte: EmpreinteAudit,
}

/// Rupture détectée dans une chaîne.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuptureChaine {
    /// Position de la première ligne incohérente.
    pub index: usize,
    /// Identifiant de cette ligne.
    pub ligne_id: Id<LigneAudit>,
}

/// Les champs hachés, dans un ordre fixe. La structure sérialisée en JSON compact est l'entrée
/// de SHA-256 ; `serde_json` trie les clés des objets `Value`, ce qui rend le hachage canonique.
#[derive(Serialize)]
struct ContenuHache<'a> {
    id: &'a Id<LigneAudit>,
    horodatage: &'a Horodatage,
    utilisateur_id: &'a Option<Id<Utilisateur>>,
    role_actif: &'a Option<String>,
    etablissement_id: &'a Id<Etablissement>,
    entite: &'a str,
    entite_id: &'a str,
    operation: &'a Operation,
    avant: &'a Option<serde_json::Value>,
    apres: &'a Option<serde_json::Value>,
    contexte: &'a str,
    poste: &'a Poste,
    empreinte_machine: &'a str,
    correlation_id: &'a str,
    empreinte_precedente: &'a EmpreinteAudit,
}

impl LigneAudit {
    /// Calcule l'empreinte attendue de la ligne à partir de son contenu et de la précédente.
    pub fn calculer_empreinte(&self) -> EmpreinteAudit {
        let contenu = ContenuHache {
            id: &self.id,
            horodatage: &self.horodatage,
            utilisateur_id: &self.utilisateur_id,
            role_actif: &self.role_actif,
            etablissement_id: &self.etablissement_id,
            entite: &self.entite,
            entite_id: &self.entite_id,
            operation: &self.operation,
            avant: &self.avant,
            apres: &self.apres,
            contexte: &self.contexte,
            poste: &self.poste,
            empreinte_machine: &self.empreinte_machine,
            correlation_id: &self.correlation_id,
            empreinte_precedente: &self.empreinte_precedente,
        };
        let json = serde_json::to_vec(&contenu).unwrap_or_default();
        let mut hachage = Sha256::new();
        hachage.update(b"gesco-audit-ligne:");
        hachage.update(json);
        EmpreinteAudit(hachage.finalize().into())
    }

    /// Fixe `empreinte_precedente` et recalcule `empreinte`. C'est l'unique façon de sceller une
    /// ligne avant écriture.
    pub fn sceller(mut self, precedente: EmpreinteAudit) -> Self {
        self.empreinte_precedente = precedente;
        self.empreinte = self.calculer_empreinte();
        self
    }
}

/// Vérifie une chaîne complète à partir de son amorce. Les lignes sont dans l'ordre d'écriture.
pub fn verifier_chaine(lignes: &[LigneAudit], amorce: EmpreinteAudit) -> Result<(), RuptureChaine> {
    let mut attendue = amorce;
    for (index, ligne) in lignes.iter().enumerate() {
        if ligne.empreinte_precedente != attendue || ligne.calculer_empreinte() != ligne.empreinte {
            return Err(RuptureChaine {
                index,
                ligne_id: ligne.id,
            });
        }
        attendue = ligne.empreinte;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn etab() -> Id<Etablissement> {
        Id::nouveau(Horodatage::depuis_millis_epoque(1), 1)
    }

    fn ligne(n: i64, apres: serde_json::Value) -> LigneAudit {
        LigneAudit {
            id: Id::nouveau(Horodatage::depuis_millis_epoque(n), 7),
            horodatage: Horodatage::depuis_millis_epoque(n),
            utilisateur_id: None,
            role_actif: Some("administrateur".to_owned()),
            etablissement_id: etab(),
            entite: "utilisateur".to_owned(),
            entite_id: "x".to_owned(),
            operation: Operation::Creation,
            avant: None,
            apres: Some(apres),
            contexte: "core.utilisateurs".to_owned(),
            poste: Poste("poste-1".to_owned()),
            empreinte_machine: "m".to_owned(),
            correlation_id: "c".to_owned(),
            empreinte_precedente: EmpreinteAudit([0; 32]),
            empreinte: EmpreinteAudit([0; 32]),
        }
    }

    fn chaine() -> Vec<LigneAudit> {
        let amorce = EmpreinteAudit::amorce(etab());
        let l1 = ligne(1, serde_json::json!({"b": 1, "a": 2})).sceller(amorce);
        let l2 = ligne(2, serde_json::json!({"nom": "x"})).sceller(l1.empreinte);
        let l3 = ligne(3, serde_json::json!(null)).sceller(l2.empreinte);
        vec![l1, l2, l3]
    }

    #[test]
    fn chaine_intacte_verifiee() {
        assert_eq!(
            verifier_chaine(&chaine(), EmpreinteAudit::amorce(etab())),
            Ok(())
        );
        assert_eq!(verifier_chaine(&[], EmpreinteAudit::amorce(etab())), Ok(()));
    }

    #[test]
    fn modification_a_posteriori_detectee() {
        let mut lignes = chaine();
        if let Some(l) = lignes.get_mut(1) {
            l.apres = Some(serde_json::json!({"nom": "y"}));
        }
        let id1 = lignes.get(1).map(|l| l.id);
        assert_eq!(
            verifier_chaine(&lignes, EmpreinteAudit::amorce(etab()))
                .map_err(|r| (r.index, Some(r.ligne_id))),
            Err((1, id1))
        );
    }

    #[test]
    fn suppression_d_une_ligne_detectee() {
        let mut lignes = chaine();
        lignes.remove(1);
        assert!(verifier_chaine(&lignes, EmpreinteAudit::amorce(etab())).is_err());
    }

    #[test]
    fn hachage_canonique_independant_de_l_ordre_des_cles() {
        let a = ligne(1, serde_json::json!({"b": 1, "a": 2})).sceller(EmpreinteAudit([9; 32]));
        let b = ligne(1, serde_json::json!({"a": 2, "b": 1})).sceller(EmpreinteAudit([9; 32]));
        assert_eq!(a.empreinte, b.empreinte);
        assert_eq!(a.empreinte.vers_hex().len(), 64);
        assert_eq!(
            EmpreinteAudit::depuis_hex(&a.empreinte.vers_hex()),
            Some(a.empreinte)
        );
    }
}
