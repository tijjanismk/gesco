//! Génération d'identifiants : l'effet (horloge, aléa) vit ici, hors du noyau (R-03).

use gesco_core::commun::Id;

/// Nouvel identifiant ULID pour une entité, généré sur le poste (ADR-004).
pub fn nouvel_id<T>() -> Id<T> {
    Id::depuis_ulid(ulid::Ulid::generate())
}

/// Lit un identifiant persisté en texte.
pub(crate) fn lire_id<T>(
    table: &'static str,
    colonne: &'static str,
    texte: &str,
) -> crate::erreur::Resultat<Id<T>> {
    Id::analyser(texte).map_err(|e| crate::ErreurBase::illisible(table, colonne, e))
}
