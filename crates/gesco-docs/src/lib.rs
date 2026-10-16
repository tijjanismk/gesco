//! Moteur documentaire de GESCO (ADR-008, R-43 à R-45).
//!
//! Un seul moteur produit tous les PDF, en Rust, hors du webview (R-44). Un modèle Typst déclare
//! ses variables ; le moteur refuse de composer si une variable requise manque (spécification
//! §10.3) et rend, avec le PDF, son empreinte SHA-256 et la version du modèle (R-45).
//! L'établissement (nom, adresse, logo, mentions) est une donnée injectée par le moteur, jamais
//! une modification du modèle (blueprint §13.2, niveau 2).

pub mod catalogue;
pub mod erreur;
pub mod lot;
mod monde;
pub mod moteur;
mod polices;
pub mod qr;
pub mod tableur;

pub use catalogue::{Catalogue, Modele, Variable};
pub use erreur::ErreurDocument;
pub use lot::{Annulation, BilanLot, DocumentLot, ElementLot, Progression};
pub use moteur::{DocumentCompose, FormatLogo, Logo, Moteur, Papier, Piece, Variante};
