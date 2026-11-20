//! Tests d'intégration de `gesco-db`, réunis en un seul exécutable : un fichier par module.
//! Un seul fichier : `cargo test -p gesco-db --test db <module>`.

mod chiffrement;
mod migration_0068;
mod reconstruction_0063;
mod reconstruction_0067;
mod reprise_0065;
mod socle;
