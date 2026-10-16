//! Identifiant technique typé par entité (ADR-004, R-10).
//!
//! `Id<Eleve>` et `Id<Classe>` sont deux types distincts : inverser les deux dans un appel est une
//! erreur de compilation, pas une ligne fausse en production.
//!
//! ```compile_fail
//! use gesco_core::commun::{Id, Horodatage};
//! struct Eleve; struct Classe;
//! let a: Id<Eleve> = Id::nouveau(Horodatage::depuis_millis_epoque(0), 0);
//! let b: Id<Classe> = Id::nouveau(Horodatage::depuis_millis_epoque(0), 0);
//! let _ = a == b;
//! ```

use std::cmp::Ordering;
use std::fmt;
use std::hash::{Hash, Hasher};
use std::marker::PhantomData;
use std::str::FromStr;

use serde::{Deserialize, Deserializer, Serialize, Serializer};
use ulid::Ulid;

use super::erreur::ErreurId;
use super::horodatage::Horodatage;

/// Identifiant ULID typé. Généré côté client à partir d'un horodatage et d'aléa fournis par
/// l'appelant : le noyau ne lit ni l'horloge ni un générateur (R-03).
pub struct Id<T> {
    ulid: Ulid,
    // `fn() -> T` : covariant, sans exiger `T: Send + Sync` ni posséder un `T`.
    entite: PhantomData<fn() -> T>,
}

impl<T> Id<T> {
    /// Construit un identifiant à partir d'un horodatage et de 80 bits d'aléa.
    /// Les bits au-delà des 80 de poids faible sont ignorés, conformément au format ULID.
    pub fn nouveau(horodatage: Horodatage, aleatoire: u128) -> Self {
        let millis = u64::try_from(horodatage.millis_epoque()).unwrap_or(0);
        Self::depuis_ulid(Ulid::from_parts(millis, aleatoire))
    }

    /// Enveloppe un ULID déjà formé.
    pub const fn depuis_ulid(ulid: Ulid) -> Self {
        Self {
            ulid,
            entite: PhantomData,
        }
    }

    /// Analyse la représentation textuelle (26 caractères Crockford base 32).
    pub fn analyser(texte: &str) -> Result<Self, ErreurId> {
        Ulid::from_string(texte)
            .map(Self::depuis_ulid)
            .map_err(|_| ErreurId::FormatInvalide(texte.to_owned()))
    }

    /// Reconstruit depuis les 16 octets bruts.
    pub const fn depuis_octets(octets: [u8; 16]) -> Self {
        Self::depuis_ulid(Ulid::from_bytes(octets))
    }

    /// Les 16 octets bruts, pour le stockage binaire.
    pub const fn octets(&self) -> [u8; 16] {
        self.ulid.to_bytes()
    }

    /// Représentation textuelle canonique.
    pub fn vers_chaine(&self) -> String {
        self.ulid.to_string()
    }

    /// Le ULID sous-jacent.
    pub const fn ulid(&self) -> Ulid {
        self.ulid
    }

    /// Instant de génération encodé dans l'identifiant.
    pub fn horodatage(&self) -> Horodatage {
        Horodatage::depuis_millis_epoque(
            i64::try_from(self.ulid.timestamp_ms()).unwrap_or(i64::MAX),
        )
    }

    /// Change l'entité associée. Réservé aux couches de persistance qui relisent une colonne
    /// brute ; le code métier n'en a jamais besoin.
    pub const fn retyper<U>(self) -> Id<U> {
        Id {
            ulid: self.ulid,
            entite: PhantomData,
        }
    }
}

// Les dérivations automatiques ajouteraient la contrainte `T: Clone`, `T: Eq`, etc. à cause du
// PhantomData. Les implémentations manuelles portent sur le ULID seul.

impl<T> Clone for Id<T> {
    fn clone(&self) -> Self {
        *self
    }
}
impl<T> Copy for Id<T> {}

impl<T> PartialEq for Id<T> {
    fn eq(&self, autre: &Self) -> bool {
        self.ulid == autre.ulid
    }
}
impl<T> Eq for Id<T> {}

impl<T> PartialOrd for Id<T> {
    fn partial_cmp(&self, autre: &Self) -> Option<Ordering> {
        Some(self.cmp(autre))
    }
}
impl<T> Ord for Id<T> {
    fn cmp(&self, autre: &Self) -> Ordering {
        self.ulid.cmp(&autre.ulid)
    }
}

impl<T> Hash for Id<T> {
    fn hash<H: Hasher>(&self, etat: &mut H) {
        self.ulid.hash(etat);
    }
}

impl<T> fmt::Debug for Id<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Id<{}>({})",
            std::any::type_name::<T>()
                .rsplit("::")
                .next()
                .unwrap_or("?"),
            self.ulid
        )
    }
}

impl<T> fmt::Display for Id<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.ulid, f)
    }
}

impl<T> FromStr for Id<T> {
    type Err = ErreurId;
    fn from_str(texte: &str) -> Result<Self, Self::Err> {
        Self::analyser(texte)
    }
}

impl<T> Serialize for Id<T> {
    fn serialize<S: Serializer>(&self, serialiseur: S) -> Result<S::Ok, S::Error> {
        serialiseur.serialize_str(&self.vers_chaine())
    }
}

impl<'de, T> Deserialize<'de> for Id<T> {
    fn deserialize<D: Deserializer<'de>>(deserialiseur: D) -> Result<Self, D::Error> {
        let texte = String::deserialize(deserialiseur)?;
        Self::analyser(&texte).map_err(serde::de::Error::custom)
    }
}

// Un identifiant voyage en texte (ULID) : c'est ce que voit le frontend (ADR-007).
impl<T> specta::Type for Id<T> {
    fn definition(_types: &mut specta::Types) -> specta::datatype::DataType {
        specta::datatype::DataType::Primitive(specta::datatype::Primitive::str)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Eleve;
    struct Classe;

    fn id_test<T>(millis: i64, aleatoire: u128) -> Id<T> {
        Id::nouveau(Horodatage::depuis_millis_epoque(millis), aleatoire)
    }

    #[test]
    fn aller_retour_texte_et_octets() {
        let id: Id<Eleve> = id_test(1_700_000_000_000, 42);
        assert_eq!(Id::<Eleve>::analyser(&id.vers_chaine()), Ok(id));
        assert_eq!(Id::<Eleve>::depuis_octets(id.octets()), id);
    }

    #[test]
    fn texte_invalide_refuse() {
        assert_eq!(
            Id::<Eleve>::analyser("pas-un-ulid"),
            Err(ErreurId::FormatInvalide("pas-un-ulid".to_owned()))
        );
    }

    #[test]
    fn ordre_chronologique_conserve() {
        let ancien: Id<Eleve> = id_test(1_000, u128::MAX);
        let recent: Id<Eleve> = id_test(2_000, 0);
        assert!(ancien < recent);
        assert_eq!(recent.horodatage().millis_epoque(), 2_000);
    }

    #[test]
    fn copie_egalite_et_hachage_sans_contrainte_sur_l_entite() {
        // `Eleve` n'implémente ni Clone ni Hash : l'identifiant doit quand même les offrir.
        let id: Id<Eleve> = id_test(5, 5);
        let copie = id;
        let mut ensemble = std::collections::HashSet::new();
        ensemble.insert(id);
        assert!(ensemble.contains(&copie));
        assert_eq!(
            format!("{copie:?}"),
            format!("Id<Eleve>({})", copie.vers_chaine())
        );
    }

    #[test]
    fn serde_en_chaine() {
        let id: Id<Classe> = id_test(7, 7);
        let json = serde_json::to_string(&id).unwrap_or_default();
        assert_eq!(json, format!("\"{}\"", id.vers_chaine()));
        let relu: Id<Classe> = serde_json::from_str(&json).unwrap_or_else(|_| id_test(0, 0));
        assert_eq!(relu, id);
    }
}
