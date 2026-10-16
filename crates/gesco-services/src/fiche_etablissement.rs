//! Fiche de l'établissement et logo (SPEC_PHASE_1 §2) : ce que l'en-tête de chaque document
//! imprime. Le logo vit dans la base, à côté de la fiche, pour voyager avec les sauvegardes.

use gesco_core::core::{Etablissement, Operation, StatutEtablissement, TypeEvenement};
use gesco_db::depots::etablissement;
use gesco_db::unite::Durabilite;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::erreur::{ErreurService, Resultat};
use crate::image;
use crate::noyau::Noyau;
use crate::session::Session;

/// Taille maximale d'un logo : 1 Mo.
const LOGO_MAX: usize = 1024 * 1024;

fn regle(cle: &'static str) -> ErreurService {
    ErreurService::Regle { cle }
}

/// Fiche affichée par l'écran Établissement.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, specta::Type)]
pub struct FicheEtablissement {
    /// Fiche.
    pub etablissement: Etablissement,
    /// Logo en URL de données (`data:image/png;base64,…`), pour l'aperçu.
    pub logo: Option<String>,
}

/// Champs modifiables de la fiche.
#[derive(Debug, Clone, Deserialize, specta::Type)]
pub struct ChampsEtablissement {
    /// Nom imprimé sur les documents.
    pub nom: String,
    /// Statut.
    pub statut: StatutEtablissement,
    /// Code de l'académie d'enseignement.
    pub code_academie: String,
    /// Code du CAP.
    pub code_cap: String,
    /// Adresse.
    pub adresse: String,
    /// Téléphones.
    pub telephones: Vec<String>,
    /// Mentions légales imprimées en pied de page.
    pub mentions_legales: String,
}

/// Noms des champs dont la valeur a changé, pour la charge utile de l'événement.
fn champs_changes(avant: &Etablissement, apres: &Etablissement) -> Resultat<Vec<String>> {
    let (avant, apres) = (serde_json::to_value(avant)?, serde_json::to_value(apres)?);
    Ok(apres
        .as_object()
        .map(|champs| {
            champs
                .iter()
                .filter(|(nom, valeur)| avant.get(nom.as_str()) != Some(*valeur))
                .map(|(nom, _)| nom.clone())
                .collect()
        })
        .unwrap_or_default())
}

/// Service de la fiche d'établissement.
pub struct ServiceEtablissement<'n> {
    noyau: &'n Noyau,
}

impl<'n> ServiceEtablissement<'n> {
    /// Service adossé au noyau.
    pub const fn nouveau(noyau: &'n Noyau) -> Self {
        Self { noyau }
    }

    /// Fiche et logo de l'établissement de la session.
    pub async fn fiche(&self, session: &Session) -> Resultat<FicheEtablissement> {
        let contexte = self.noyau.contexte(session.etablissement_id, Some(session));
        let mut cnx = self
            .noyau
            .base()
            .pool()
            .acquire()
            .await
            .map_err(gesco_db::ErreurBase::from)?;
        let fiche = etablissement::lire(&mut cnx, session.etablissement_id).await?;
        let logo = etablissement::logo(&mut cnx, &contexte)
            .await?
            .map(|(octets, format)| image::url_de_donnees(&octets, &format));
        Ok(FicheEtablissement {
            etablissement: fiche,
            logo,
        })
    }

    /// Récrit la fiche : nom obligatoire, téléphones vides écartés.
    pub async fn modifier(
        &self,
        session: &Session,
        champs: ChampsEtablissement,
    ) -> Resultat<FicheEtablissement> {
        let nom = champs.nom.trim();
        if nom.is_empty() {
            return Err(regle("etablissement.nom_obligatoire"));
        }
        let contexte = self.noyau.contexte(session.etablissement_id, Some(session));
        let mut unite = self
            .noyau
            .commencer(contexte.clone(), Durabilite::Critique)
            .await?;
        let avant = etablissement::lire(unite.connexion(), session.etablissement_id).await?;
        let apres = Etablissement {
            nom: nom.to_owned(),
            statut: champs.statut,
            code_academie: champs.code_academie.trim().to_owned(),
            code_cap: champs.code_cap.trim().to_owned(),
            adresse: champs.adresse.trim().to_owned(),
            telephones: champs
                .telephones
                .iter()
                .map(|t| t.trim().to_owned())
                .filter(|t| !t.is_empty())
                .collect(),
            mentions_legales: champs.mentions_legales.trim().to_owned(),
            ..avant.clone()
        };
        etablissement::modifier(unite.connexion(), &apres, &contexte).await?;
        let id = apres.id.vers_chaine();
        unite.auditer(
            "etablissement",
            &id,
            Operation::Modification,
            Some(serde_json::to_value(&avant)?),
            Some(serde_json::to_value(&apres)?),
            "etablissement.modifier",
        );
        unite.emettre(
            TypeEvenement::FicheEtablissementRectifiee,
            "etablissement",
            &id,
            serde_json::json!({ "nom": apres.nom, "champs": champs_changes(&avant, &apres)? }),
        );
        self.noyau.terminer(unite).await?;
        self.fiche(session).await
    }

    /// Pose un logo PNG ou JPEG d'1 Mo au plus, transmis en base64, ou le retire (`None`).
    pub async fn definir_logo(
        &self,
        session: &Session,
        contenu_base64: Option<&str>,
    ) -> Resultat<FicheEtablissement> {
        let logo = contenu_base64
            .map(|texte| {
                image::decoder(
                    texte,
                    LOGO_MAX,
                    &image::Refus {
                        illisible: "etablissement.logo_illisible",
                        trop_lourde: "etablissement.logo_trop_lourd",
                        format: "etablissement.logo_format",
                    },
                )
            })
            .transpose()?;
        let contexte = self.noyau.contexte(session.etablissement_id, Some(session));
        let mut unite = self
            .noyau
            .commencer(contexte.clone(), Durabilite::Critique)
            .await?;
        let avant = etablissement::logo(unite.connexion(), &contexte)
            .await?
            .map(|(octets, _)| hex::encode(Sha256::digest(octets)));
        etablissement::definir_logo(
            unite.connexion(),
            logo.as_ref().map(|i| (i.octets.as_slice(), i.format)),
            &contexte,
        )
        .await?;
        let apres = logo.as_ref().map(|i| image::trace(&i.octets, i.format));
        let id = session.etablissement_id.vers_chaine();
        unite.auditer(
            "etablissement_logo",
            &id,
            Operation::Modification,
            Some(serde_json::json!({ "empreinte": avant })),
            Some(apres.clone().unwrap_or(serde_json::Value::Null)),
            "etablissement.logo",
        );
        unite.emettre(
            TypeEvenement::LogoEtablissementRemplace,
            "etablissement",
            &id,
            apres.unwrap_or(serde_json::json!({ "empreinte": null })),
        );
        self.noyau.terminer(unite).await?;
        self.fiche(session).await
    }
}
