//! Le « monde » Typst d'une composition : un système de fichiers virtuel en mémoire, sans accès
//! au disque ni à l'horloge. Le modèle lit ses données dans `/donnees.json`.

use std::collections::HashMap;

use typst::diag::{FileError, FileResult};
use typst::foundations::{Bytes, Datetime, Duration};
use typst::syntax::{FileId, RootedPath, Source, VirtualPath, VirtualRoot};
use typst::text::{Font, FontBook};
use typst::utils::LazyHash;
use typst::{Library, World};

use crate::polices::Polices;

pub(crate) const CHEMIN_MODELE: &str = "/modele.typ";
pub(crate) const CHEMIN_ENTETE: &str = "/commun/entete.typ";
pub(crate) const CHEMIN_DONNEES: &str = "/donnees.json";

/// Identifiant de fichier virtuel. Les chemins sont des constantes ou construits par le moteur :
/// un chemin refusé par Typst est une erreur de programmation, signalée par `None`.
pub(crate) fn fichier(chemin: &str) -> Option<FileId> {
    VirtualPath::new(chemin)
        .ok()
        .map(|v| FileId::new(RootedPath::new(VirtualRoot::Project, v)))
}

pub(crate) struct Monde<'a> {
    pub(crate) bibliotheque: &'a LazyHash<Library>,
    pub(crate) polices: &'a Polices,
    pub(crate) principal: FileId,
    pub(crate) fichiers: HashMap<FileId, Bytes>,
}

impl World for Monde<'_> {
    fn library(&self) -> &LazyHash<Library> {
        self.bibliotheque
    }

    fn book(&self) -> &LazyHash<FontBook> {
        &self.polices.livre
    }

    fn main(&self) -> FileId {
        self.principal
    }

    fn source(&self, id: FileId) -> FileResult<Source> {
        let octets = self.file(id)?;
        let texte = std::str::from_utf8(octets.as_slice()).map_err(|_| FileError::InvalidUtf8)?;
        Ok(Source::new(id, texte.to_owned()))
    }

    fn file(&self, id: FileId) -> FileResult<Bytes> {
        self.fichiers
            .get(&id)
            .cloned()
            .ok_or_else(|| FileError::NotFound(id.get().vpath().get_with_slash().to_owned().into()))
    }

    fn font(&self, index: usize) -> Option<Font> {
        self.polices.polices.get(index).cloned()
    }

    // La date vient des données : un document ne dépend jamais de l'horloge du poste.
    fn today(&self, _offset: Option<Duration>) -> Option<Datetime> {
        None
    }
}
