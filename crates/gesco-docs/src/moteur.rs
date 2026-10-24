//! Composition d'un document : modèle + données + variante d'établissement → PDF.

use std::collections::{BTreeMap, HashMap};

use serde::Serialize;
use sha2::{Digest, Sha256};
use typst::diag::SourceDiagnostic;
use typst::foundations::{Bytes, Smart};
use typst::utils::LazyHash;
use typst::{Library, LibraryExt};
use typst_layout::PagedDocument;
use typst_pdf::PdfOptions;

use crate::catalogue::{Catalogue, ENTETE_COMMUN, VARIABLE_ETABLISSEMENT};
use crate::erreur::ErreurDocument;
use crate::monde::{CHEMIN_DONNEES, CHEMIN_ENTETE, CHEMIN_MODELE, Monde, fichier};
use crate::polices::Polices;

/// Format d'image du logo d'établissement.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FormatLogo {
    /// PNG.
    Png,
    /// JPEG.
    Jpeg,
}

/// Logo d'établissement.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Logo {
    /// Contenu du fichier image.
    pub octets: Vec<u8>,
    /// Format de l'image.
    pub format: FormatLogo,
}

/// Variante d'établissement (blueprint §13.2, niveau 2) : ce qui distingue l'en-tête d'une école
/// de celui d'une autre, sans toucher au modèle.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Variante {
    /// Nom de l'établissement.
    pub nom: String,
    /// Adresse.
    pub adresse: String,
    /// Téléphones.
    pub telephones: Vec<String>,
    /// Mentions légales imprimées en pied de page.
    pub mentions: String,
    /// Logo, facultatif.
    pub logo: Option<Logo>,
    /// Format du papier de ce document, au choix de l'école.
    pub papier: Papier,
    /// Largeur des colonnes de ce document que l'école a réglées, en millimètres, par nom de
    /// colonne ; une colonne absente garde la largeur du modèle.
    pub largeurs: BTreeMap<String, u32>,
}

/// Format du papier d'un document.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Papier {
    /// A4.
    #[default]
    A4,
    /// A5 : le reçu et le bulletin, dans la plupart des écoles.
    A5,
}

impl Papier {
    /// Nom donné aux modèles : `a4`, `a5`.
    pub const fn nom(self) -> &'static str {
        match self {
            Self::A4 => "a4",
            Self::A5 => "a5",
        }
    }

    /// Lecture d'un nom ; A4 pour tout autre.
    pub fn depuis_nom(nom: &str) -> Self {
        if nom == "a5" { Self::A5 } else { Self::A4 }
    }
}

#[derive(Serialize)]
struct EtablissementInjecte<'a> {
    nom: &'a str,
    adresse: &'a str,
    telephones: &'a [String],
    mentions: &'a str,
    logo: Option<&'static str>,
    papier: &'static str,
    largeurs: &'a BTreeMap<String, u32>,
}

impl Variante {
    fn chemin_logo(&self) -> Option<&'static str> {
        self.logo.as_ref().map(|l| match l.format {
            FormatLogo::Png => "/variante/logo.png",
            FormatLogo::Jpeg => "/variante/logo.jpg",
        })
    }
}

/// Fichier joint à une composition (photo d'un élève, QR code), lu par le modèle sous
/// `/pieces/<nom>`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Piece {
    /// Nom du fichier, extension comprise : `photo.jpg`, `qr.svg`.
    pub nom: String,
    /// Contenu.
    pub octets: Vec<u8>,
}

impl Piece {
    /// Chemin sous lequel le modèle lit la pièce.
    pub fn chemin(&self) -> String {
        format!("/pieces/{}", self.nom)
    }
}

/// Document composé, avant tout enregistrement.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DocumentCompose {
    /// Contenu PDF.
    pub pdf: Vec<u8>,
    /// Empreinte SHA-256 du PDF, en hexadécimal.
    pub empreinte_sha256: String,
    /// Identifiant du modèle.
    pub modele_id: String,
    /// Version du modèle (R-45).
    pub modele_version: u32,
}

/// Empreinte SHA-256 en hexadécimal.
pub(crate) fn empreinte(octets: &[u8]) -> String {
    hex::encode(Sha256::digest(octets))
}

/// Le moteur : catalogue, polices et bibliothèque Typst chargés une fois, partagés par toutes les
/// compositions, y compris celles d'un lot parallèle.
pub struct Moteur {
    catalogue: Catalogue,
    polices: Polices,
    bibliotheque: LazyHash<Library>,
}

impl std::fmt::Debug for Moteur {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Moteur")
            .field("catalogue", &self.catalogue)
            .finish_non_exhaustive()
    }
}

fn premier_diagnostic(diagnostics: &[SourceDiagnostic]) -> String {
    diagnostics.first().map_or_else(
        || "erreur sans diagnostic".to_owned(),
        |d| d.message.to_string(),
    )
}

impl Moteur {
    /// Moteur sur un catalogue.
    pub fn nouveau(catalogue: Catalogue) -> Self {
        Self {
            catalogue,
            polices: Polices::embarquees(),
            bibliotheque: LazyHash::new(Library::default()),
        }
    }

    /// Catalogue des modèles.
    pub fn catalogue(&self) -> &Catalogue {
        &self.catalogue
    }

    /// Compose un document. Refuse si une variable requise manque.
    pub fn composer(
        &self,
        modele_id: &str,
        variante: &Variante,
        donnees: &serde_json::Value,
    ) -> Result<DocumentCompose, ErreurDocument> {
        self.composer_avec(modele_id, variante, donnees, &[])
    }

    /// Compose un document avec des pièces jointes, que les données désignent par leur chemin.
    pub fn composer_avec(
        &self,
        modele_id: &str,
        variante: &Variante,
        donnees: &serde_json::Value,
        pieces: &[Piece],
    ) -> Result<DocumentCompose, ErreurDocument> {
        let (document, modele_id, modele_version) =
            self.mettre_en_page(modele_id, variante, donnees, pieces)?;
        let options = PdfOptions {
            creator: Smart::Custom(Some("GESCO".into())),
            ..PdfOptions::default()
        };
        let pdf =
            typst_pdf::pdf(&document, &options).map_err(|diagnostics| ErreurDocument::Export {
                modele: modele_id.clone(),
                detail: premier_diagnostic(&diagnostics),
            })?;
        Ok(DocumentCompose {
            empreinte_sha256: empreinte(&pdf),
            pdf,
            modele_id,
            modele_version,
        })
    }

    /// Pages d'un document en SVG, pour la relecture à l'œil d'un modèle pendant le
    /// développement : le produit n'imprime que des PDF.
    #[cfg(feature = "apercu-svg")]
    pub fn composer_svg(
        &self,
        modele_id: &str,
        variante: &Variante,
        donnees: &serde_json::Value,
        pieces: &[Piece],
    ) -> Result<Vec<String>, ErreurDocument> {
        let (document, _, _) = self.mettre_en_page(modele_id, variante, donnees, pieces)?;
        Ok(document
            .pages()
            .iter()
            .map(|page| typst_svg::svg(page, &typst_svg::SvgOptions::default()))
            .collect())
    }

    /// Met un document en pages : (pages, identifiant et version du modèle).
    fn mettre_en_page(
        &self,
        modele_id: &str,
        variante: &Variante,
        donnees: &serde_json::Value,
        pieces: &[Piece],
    ) -> Result<(PagedDocument, String, u32), ErreurDocument> {
        let modele = self.catalogue.modele(modele_id)?;
        modele.verifier(donnees)?;
        let erreur_interne = |detail: &str| ErreurDocument::Composition {
            modele: modele.id.clone(),
            detail: detail.to_owned(),
        };

        let mut complet = donnees.clone();
        if let Some(objet) = complet.as_object_mut() {
            objet.insert(
                VARIABLE_ETABLISSEMENT.to_owned(),
                serde_json::to_value(EtablissementInjecte {
                    nom: &variante.nom,
                    adresse: &variante.adresse,
                    telephones: &variante.telephones,
                    mentions: &variante.mentions,
                    logo: variante.chemin_logo(),
                    papier: variante.papier.nom(),
                    largeurs: &variante.largeurs,
                })
                .map_err(|e| erreur_interne(&e.to_string()))?,
            );
        }
        let json = serde_json::to_vec(&complet).map_err(|e| erreur_interne(&e.to_string()))?;

        let chemin = |c: &str| fichier(c).ok_or_else(|| erreur_interne(c));
        let principal = chemin(CHEMIN_MODELE)?;
        let mut fichiers = HashMap::from([
            (principal, Bytes::from_string(modele.source.clone())),
            (chemin(CHEMIN_ENTETE)?, Bytes::new(ENTETE_COMMUN.as_bytes())),
            (chemin(CHEMIN_DONNEES)?, Bytes::new(json)),
        ]);
        if let (Some(logo), Some(chemin_logo)) = (&variante.logo, variante.chemin_logo()) {
            fichiers.insert(chemin(chemin_logo)?, Bytes::new(logo.octets.clone()));
        }
        for piece in pieces {
            fichiers.insert(chemin(&piece.chemin())?, Bytes::new(piece.octets.clone()));
        }

        let monde = Monde {
            bibliotheque: &self.bibliotheque,
            polices: &self.polices,
            principal,
            fichiers,
        };
        let document = typst::compile::<PagedDocument>(&monde)
            .output
            .map_err(|diagnostics| ErreurDocument::Composition {
                modele: modele.id.clone(),
                detail: premier_diagnostic(&diagnostics),
            })?;
        Ok((document, modele.id.clone(), modele.version))
    }
}
