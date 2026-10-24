//! Production des documents (étape 10) : composition par `gesco-docs`, écriture du fichier,
//! enregistrement de son empreinte et de la version du modèle (R-45), audit et événement dans
//! une même transaction (R-17). La composition et l'écriture du fichier précèdent la
//! transaction, qui reste courte (R-19).

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use gesco_core::commun::{Horodatage, Id};
use gesco_core::core::{Etablissement, Operation, PhraseSecours, TypeEvenement};
use gesco_db::depots::document::{self, Document};
use gesco_db::depots::etablissement;
use gesco_db::unite::Durabilite;
use gesco_db::{Contexte, nouvel_id};
use gesco_docs::{
    Catalogue, DocumentCompose, FormatLogo, Logo, Moteur, Papier, Piece, Variante, qr,
};

use crate::effets::{Effet, RecepteurEffets};
use crate::erreur::{ErreurService, Resultat};
use crate::espace::{EspaceDonnees, ecrire_atomiquement};
use crate::noyau::Noyau;

const MOIS: [&str; 12] = [
    "janvier",
    "février",
    "mars",
    "avril",
    "mai",
    "juin",
    "juillet",
    "août",
    "septembre",
    "octobre",
    "novembre",
    "décembre",
];

/// Date en toutes lettres pour les documents : « 4 octobre 2026 ». Le Mali vit à l'heure UTC.
pub fn date_longue(instant: Horodatage) -> String {
    let civil = instant.civil_utc();
    let mois = MOIS
        .get(usize::from(civil.mois.saturating_sub(1)))
        .copied()
        .unwrap_or_default();
    if civil.jour == 1 {
        format!("1er {mois} {}", civil.annee)
    } else {
        format!("{} {mois} {}", civil.jour, civil.annee)
    }
}

/// Date civile en toutes lettres : « 2 mars 2014 ».
pub fn date_civile_longue(date: gesco_core::commun::DateCivile) -> String {
    let texte = date.to_string();
    let mut parties = texte.split('-');
    let (Some(annee), Some(mois), Some(jour)) = (parties.next(), parties.next(), parties.next())
    else {
        return texte;
    };
    let nom_mois = mois
        .parse::<usize>()
        .ok()
        .and_then(|m| MOIS.get(m.saturating_sub(1)))
        .copied()
        .unwrap_or_default();
    match jour.trim_start_matches('0') {
        "1" => format!("1er {nom_mois} {annee}"),
        j => format!("{j} {nom_mois} {annee}"),
    }
}

/// Le moteur et le dossier des documents d'une installation.
#[derive(Debug)]
pub struct AtelierDocuments {
    moteur: Arc<Moteur>,
    dossier: PathBuf,
}

impl AtelierDocuments {
    /// Modèles de base, remplacés par les modèles sur mesure de l'espace s'il y en a.
    pub fn nouveau(espace: &EspaceDonnees) -> Resultat<Self> {
        let mut catalogue = Catalogue::integre()?;
        catalogue.surcharger_depuis(&espace.modeles())?;
        Ok(Self {
            moteur: Arc::new(Moteur::nouveau(catalogue)),
            dossier: espace.documents(),
        })
    }

    /// Moteur documentaire.
    pub fn moteur(&self) -> &Moteur {
        &self.moteur
    }
}

/// Document produit, avec son contenu pour l'aperçu et l'impression.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DocumentProduit {
    /// Ligne enregistrée.
    pub document: Document,
    /// Contenu PDF.
    pub pdf: Vec<u8>,
}

/// Service de production des documents.
pub struct ServiceDocuments<'a> {
    noyau: &'a Noyau,
    atelier: &'a AtelierDocuments,
}

impl<'a> ServiceDocuments<'a> {
    /// Service adossé au noyau et à l'atelier.
    pub const fn nouveau(noyau: &'a Noyau, atelier: &'a AtelierDocuments) -> Self {
        Self { noyau, atelier }
    }

    /// Format du papier d'un document, d'après le réglage `document.format.<modèle>` ; A4 pour un
    /// document sans réglage.
    async fn papier(
        &self,
        etablissement_id: Id<Etablissement>,
        modele_id: &str,
    ) -> Resultat<Papier> {
        let Some(defaut) = gesco_core::core::parametre::format_par_defaut(modele_id) else {
            return Ok(Papier::A4);
        };
        let contexte = self.noyau.contexte(etablissement_id, None);
        let mut cnx = self
            .noyau
            .base()
            .pool()
            .acquire()
            .await
            .map_err(gesco_db::ErreurBase::from)?;
        let cle = format!("document.format.{modele_id}");
        Ok(Papier::depuis_nom(
            &gesco_db::depots::parametre::texte(&mut cnx, &cle, defaut, &contexte).await?,
        ))
    }

    /// Largeur des colonnes d'un document que l'école a réglées (`colonnes.<modèle>.<colonne>`),
    /// en millimètres ; une colonne à 0 garde la largeur du modèle.
    async fn largeurs(
        &self,
        etablissement_id: Id<Etablissement>,
        modele_id: &str,
    ) -> Resultat<BTreeMap<String, u32>> {
        let mut largeurs = BTreeMap::new();
        let Some((_, colonnes)) = gesco_core::core::parametre::COLONNES_DOCUMENTS
            .iter()
            .find(|(document, _)| *document == modele_id)
        else {
            return Ok(largeurs);
        };
        let contexte = self.noyau.contexte(etablissement_id, None);
        let mut cnx = self
            .noyau
            .base()
            .pool()
            .acquire()
            .await
            .map_err(gesco_db::ErreurBase::from)?;
        for colonne in *colonnes {
            let cle = gesco_core::core::parametre::cle_colonne(modele_id, colonne);
            let mm = gesco_db::depots::parametre::entier(&mut cnx, &cle, 0, &contexte).await?;
            if let Ok(mm) = u32::try_from(mm)
                && mm > 0
            {
                largeurs.insert((*colonne).to_owned(), mm);
            }
        }
        Ok(largeurs)
    }

    /// Variante d'établissement : en-tête et mentions tirés de la fiche de l'établissement.
    pub async fn variante(&self, etablissement_id: Id<Etablissement>) -> Resultat<Variante> {
        let mut cnx = self
            .noyau
            .base()
            .pool()
            .acquire()
            .await
            .map_err(gesco_db::ErreurBase::from)?;
        let fiche = etablissement::lire(&mut cnx, etablissement_id).await?;
        let contexte = self.noyau.contexte(etablissement_id, None);
        let logo = etablissement::logo(&mut cnx, &contexte)
            .await?
            .map(|(octets, format)| Logo {
                octets,
                format: if format == "png" {
                    FormatLogo::Png
                } else {
                    FormatLogo::Jpeg
                },
            });
        Ok(Variante {
            nom: fiche.nom,
            adresse: fiche.adresse,
            telephones: fiche.telephones,
            mentions: fiche.mentions_legales,
            logo,
            papier: Papier::A4,
            largeurs: BTreeMap::new(),
        })
    }

    async fn composer(
        &self,
        etablissement_id: Id<Etablissement>,
        modele_id: &str,
        donnees: serde_json::Value,
        pieces: Vec<Piece>,
    ) -> Resultat<DocumentCompose> {
        let mut variante = self.variante(etablissement_id).await?;
        variante.papier = self.papier(etablissement_id, modele_id).await?;
        variante.largeurs = self.largeurs(etablissement_id, modele_id).await?;
        let moteur = Arc::clone(&self.atelier.moteur);
        let modele = modele_id.to_owned();
        tokio::task::spawn_blocking(move || {
            moteur.composer_avec(&modele, &variante, &donnees, &pieces)
        })
        .await
        .map_err(|e| ErreurService::Secret(e.to_string()))?
        .map_err(ErreurService::from)
    }

    /// Compose, écrit et enregistre un document. Le type de document désigne le modèle.
    pub async fn produire(
        &self,
        contexte: Contexte,
        type_document: &str,
        entite: &str,
        entite_id: &str,
        donnees: serde_json::Value,
    ) -> Resultat<DocumentProduit> {
        self.produire_avec(
            contexte,
            type_document,
            entite,
            entite_id,
            donnees,
            Vec::new(),
        )
        .await
    }

    /// Comme [`Self::produire`], avec des pièces jointes (photo). Un modèle qui déclare la
    /// variable `qr` reçoit aussi la référence du document et son QR code de vérification
    /// (§25.9), sous `reference` et `qr`.
    pub async fn produire_avec(
        &self,
        contexte: Contexte,
        type_document: &str,
        entite: &str,
        entite_id: &str,
        mut donnees: serde_json::Value,
        mut pieces: Vec<Piece>,
    ) -> Resultat<DocumentProduit> {
        let id: Id<Document> = nouvel_id();
        let veut_qr = self
            .atelier
            .moteur
            .catalogue()
            .modele(type_document)
            .is_ok_and(|m| m.variables.iter().any(|v| v.nom == "qr"));
        if veut_qr {
            let qr = qr::piece(&qr::reference(&id.vers_chaine()))?;
            if let Some(objet) = donnees.as_object_mut() {
                objet.insert("reference".to_owned(), id.vers_chaine().into());
                objet.insert("qr".to_owned(), qr.chemin().into());
            }
            pieces.push(qr);
        }
        let compose = self
            .composer(contexte.etablissement_id, type_document, donnees, pieces)
            .await?;
        let dossier = self.atelier.dossier.join(type_document);
        std::fs::create_dir_all(&dossier)?;
        let chemin = dossier.join(format!("{}.pdf", id.vers_chaine()));
        ecrire_atomiquement(&chemin, &compose.pdf)?;
        let document = Document {
            id,
            etablissement_id: contexte.etablissement_id,
            type_document: type_document.to_owned(),
            entite: entite.to_owned(),
            entite_id: entite_id.to_owned(),
            chemin: chemin.to_string_lossy().into_owned(),
            empreinte_sha256: compose.empreinte_sha256.clone(),
            taille: i64::try_from(compose.pdf.len()).unwrap_or(i64::MAX),
            modele_id: compose.modele_id.clone(),
            modele_version: i64::from(compose.modele_version),
            produit_le: contexte.maintenant,
            produit_par: contexte.utilisateur_id,
        };
        let mut unite = self
            .noyau
            .commencer(contexte.clone(), Durabilite::Ordinaire)
            .await?;
        document::inserer(unite.connexion(), &document, &contexte).await?;
        let id_texte = id.vers_chaine();
        unite.auditer(
            "document",
            &id_texte,
            Operation::Creation,
            None,
            Some(serde_json::to_value(&document)?),
            "documents.produire",
        );
        unite.emettre(
            TypeEvenement::DocumentCompose,
            "document",
            &id_texte,
            serde_json::json!({
                "type_document": document.type_document,
                "entite": document.entite,
                "entite_id": document.entite_id,
                "modele_id": document.modele_id,
                "modele_version": document.modele_version,
                "empreinte_sha256": document.empreinte_sha256,
            }),
        );
        self.noyau.terminer(unite).await?;
        Ok(DocumentProduit {
            document,
            pdf: compose.pdf,
        })
    }

    /// Compose le billet d'un code d'accès de responsable, en mémoire seulement : il porte le
    /// code, que la base ne garde pas (SPEC_PHASE_5 §9). L'audit de la remise en garde la trace.
    pub async fn composer_billet_code_acces(
        &self,
        etablissement_id: Id<Etablissement>,
        donnees: serde_json::Value,
    ) -> Resultat<DocumentCompose> {
        self.composer(etablissement_id, "code_acces", donnees, Vec::new())
            .await
    }

    /// Compose le certificat de clé de secours, en mémoire seulement : il porte la phrase, il
    /// n'est jamais écrit sur le disque du poste (R-25). L'audit de l'opération appelante en
    /// garde la trace (enveloppe, version du modèle).
    pub async fn composer_certificat(
        &self,
        etablissement_id: Id<Etablissement>,
        enveloppe_id: &str,
        phrase: &PhraseSecours,
        emis_par: &str,
        instant: Horodatage,
    ) -> Resultat<DocumentCompose> {
        self.composer(
            etablissement_id,
            "certificat_cle_secours",
            serde_json::json!({
                "date": date_longue(instant),
                "enveloppe_id": enveloppe_id,
                "mots": phrase.mots(),
                "emis_par": emis_par,
            }),
            Vec::new(),
        )
        .await
    }
}

/// Effet qui n'a pas pu être exécuté, affiché à la direction jusqu'à la fermeture.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EffetEchoue {
    /// Clé de traduction du motif.
    pub cle: String,
    /// Entité concernée.
    pub entite_id: String,
}

/// Effets échoués de la session de l'application.
pub type EffetsEchoues = Arc<Mutex<Vec<EffetEchoue>>>;

/// Tâche de fond qui exécute les effets après commit (R-18) : composition des documents
/// demandés par les services. Un échec n'annule rien, la transaction est déjà validée : il est
/// conservé pour l'écran d'avertissements.
pub async fn traiter_effets(
    noyau: Arc<Noyau>,
    atelier: Arc<AtelierDocuments>,
    echecs: EffetsEchoues,
    mut recepteur: RecepteurEffets,
) {
    while let Some((contexte, effet)) = recepteur.recv().await {
        if let Effet::ComposerDocument {
            type_document,
            entite,
            entite_id,
            donnees,
        } = effet
        {
            let service = ServiceDocuments::nouveau(&noyau, &atelier);
            if let Err(erreur) = service
                .produire(contexte, &type_document, &entite, &entite_id, donnees)
                .await
                && let Ok(mut liste) = echecs.lock()
            {
                liste.push(EffetEchoue {
                    cle: erreur.cle_traduction().to_owned(),
                    entite_id,
                });
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dates_en_toutes_lettres() {
        // 2026-10-04 00:00 UTC et 2026-11-01 12:00 UTC.
        assert_eq!(
            date_longue(Horodatage::depuis_millis_epoque(1_791_072_000_000)),
            "4 octobre 2026"
        );
        assert_eq!(
            date_longue(Horodatage::depuis_millis_epoque(1_793_534_400_000)),
            "1er novembre 2026"
        );
    }
}
