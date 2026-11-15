//! Publier (§9 de la spécification) : l'auteur soumet son brouillon ; qui valide
//! (`library:publish`) publie ou renvoie avec un commentaire ; publier rend la version visible
//! aussitôt et passe la précédente à « remplacée ». La direction donne nommément à un maître le
//! droit de publier sans soumettre, dans les matières et niveaux de ses affectations.
//!
//! Aucune marque de relecture : GESCO ne dit nulle part qu'un cours a été relu ou non. Qui écrit
//! garde, à côté de la leçon, la liste des faits à vérifier.

use std::collections::HashMap;

use gesco_core::bibliotheque::EtatVersion;
use gesco_core::bibliotheque::paquet::empreinte;
use gesco_core::core::{Operation, TypeEvenement};
use gesco_db::ErreurBase;
use gesco_db::depots::academique;
use gesco_db::depots::bibliotheque::{self as depot, Publication};
use gesco_db::unite::Durabilite;
use serde::{Deserialize, Serialize};

use super::ecrire::{DemandeBrouillonVersion, DemandeContenuBibliotheque, EcritureOuverte};
use super::{ServiceBibliotheque, regle, reindexer_contenu};
use crate::erreur::{ErreurService, Resultat};
use crate::session::Session;

/// Renvoyer une version soumise.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
pub struct DemandeRenvoiVersion {
    /// Identifiant du contenu.
    pub contenu_id: String,
    /// Identifiant de la version soumise.
    pub version_id: String,
    /// Ce que l'auteur doit reprendre.
    pub commentaire: String,
}

/// Une version en attente de validation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
pub struct VersionEnAttente {
    /// Identifiant du contenu.
    pub contenu_id: String,
    /// Identifiant de la version.
    pub version_id: String,
    /// Titre du contenu.
    pub titre: String,
    /// Niveau (libellé de l'école).
    pub niveau: String,
    /// Chapitre.
    pub chapitre: String,
    /// Nom de l'auteur.
    pub auteur: String,
    /// Soumise le, millisecondes depuis l'époque.
    pub soumise_le: i64,
    /// « Revenir à la version livrée ».
    pub retour_livre: bool,
}

/// Un maître qui peut publier lui-même.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
pub struct DroitPublicationAffiche {
    /// Compte du maître.
    pub utilisateur_id: String,
    /// Nom.
    pub nom: String,
    /// Donné le, millisecondes depuis l'époque.
    pub donne_le: i64,
    /// Nom de qui l'a donné.
    pub donne_par: String,
}

/// Un maître à qui donner le droit de publier.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
pub struct MaitreChoisissable {
    /// Compte du maître.
    pub utilisateur_id: String,
    /// Nom.
    pub nom: String,
}

/// « Peuvent publier eux-mêmes ».
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
pub struct DroitsPublication {
    /// Maîtres qui ont le droit, par nom.
    pub droits: Vec<DroitPublicationAffiche>,
    /// Maîtres avec un compte qui ne l'ont pas.
    pub maitres: Vec<MaitreChoisissable>,
}

/// Donner ou retirer le droit de publier.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
pub struct DemandeDroitPublication {
    /// Compte du maître.
    pub utilisateur_id: String,
}

fn introuvable(entite: &'static str, id: &str) -> ErreurService {
    ErreurBase::Introuvable {
        entite,
        id: id.to_owned(),
    }
    .into()
}

impl ServiceBibliotheque<'_> {
    /// Rouvre le contenu pour l'écriture, après un geste.
    async fn rouvrir(&self, session: &Session, contenu_id: &str) -> Resultat<EcritureOuverte> {
        self.ecriture(
            session,
            &DemandeContenuBibliotheque {
                contenu_id: contenu_id.to_owned(),
            },
        )
        .await
    }

    /// Soumet le brouillon à la direction. Seul son auteur le soumet ; une version soumise ne
    /// change plus.
    pub async fn soumettre(
        &self,
        session: &Session,
        demande: &DemandeBrouillonVersion,
    ) -> Resultat<EcritureOuverte> {
        let contexte = self.noyau.contexte(session.etablissement_id, Some(session));
        let portee = self.portee(session).await?;
        let mut unite = self
            .noyau
            .commencer(contexte.clone(), Durabilite::Critique)
            .await?;
        Self::contenu_permis(unite.connexion(), &portee, &demande.contenu_id, &contexte).await?;
        let avant = Self::brouillon_attendu(
            unite.connexion(),
            &demande.contenu_id,
            &demande.version_id,
            &contexte,
        )
        .await?;
        if avant.auteur_id != session.utilisateur_id.vers_chaine() {
            return Err(regle("bibliotheque.brouillon_d_un_autre"));
        }
        if !depot::soumettre_brouillon(unite.connexion(), &demande.version_id, &contexte).await? {
            return Err(regle("bibliotheque.modifiee_ailleurs"));
        }
        unite.auditer(
            "version_contenu",
            &demande.version_id,
            Operation::Modification,
            Some(serde_json::json!({ "etat": EtatVersion::Brouillon.nom() })),
            Some(serde_json::json!({ "etat": EtatVersion::Soumise.nom() })),
            "bibliotheque.soumettre",
        );
        unite.emettre(
            TypeEvenement::VersionContenuModifiee,
            "version_contenu",
            &demande.version_id,
            serde_json::json!({
                "contenu_id": demande.contenu_id,
                "numero": avant.numero,
                "etat": EtatVersion::Soumise.nom(),
            }),
        );
        self.noyau.terminer(unite).await?;
        self.rouvrir(session, &demande.contenu_id).await
    }

    /// Publie une version : la version soumise, pour qui valide ; son propre brouillon, pour qui
    /// valide ou pour un maître qui a reçu le droit de publier, dans ses matières. La version
    /// devient visible aussitôt ; la précédente passe à « remplacée » ; l'index de recherche suit.
    pub async fn publier(
        &self,
        session: &Session,
        demande: &DemandeBrouillonVersion,
    ) -> Resultat<EcritureOuverte> {
        let (_, peut_valider) = self.permissions(session).await?;
        let contexte = self.noyau.contexte(session.etablissement_id, Some(session));
        let portee = self.portee(session).await?;
        let moi = session.utilisateur_id.vers_chaine();
        let droit_nomme = if peut_valider {
            false
        } else {
            let mut cnx = self.connexion().await?;
            depot::a_le_droit_de_publier(&mut cnx, &moi, &contexte).await?
        };
        let paquets = self.paquets.tous();
        let mut unite = self
            .noyau
            .commencer(contexte.clone(), Durabilite::Critique)
            .await?;
        let (contenu, permis) =
            Self::contenu_et_portee(unite.connexion(), &portee, &demande.contenu_id, &contexte)
                .await?;
        let versions = depot::versions(unite.connexion(), &demande.contenu_id, &contexte).await?;
        let version = versions
            .iter()
            .find(|v| v.id == demande.version_id && (v.etat == "brouillon" || v.etat == "soumise"))
            .ok_or(regle("bibliotheque.modifiee_ailleurs"))?;
        if version.etat == "brouillon" {
            if version.auteur_id != moi {
                return Err(regle("bibliotheque.brouillon_d_un_autre"));
            }
            if !peut_valider {
                if !permis {
                    return Err(ErreurService::PorteeRefusee {
                        ressource: format!("contenu {}", demande.contenu_id),
                    });
                }
                if !droit_nomme {
                    return Err(regle("bibliotheque.sans_droit_de_publier"));
                }
            }
        } else if !peut_valider {
            return Err(regle("bibliotheque.sans_droit_de_publier"));
        }
        let etat_avant = version.etat.clone();
        let numero = version.numero;
        let remplacee = match depot::publier_version(
            unite.connexion(),
            &demande.contenu_id,
            &demande.version_id,
            &contexte,
        )
        .await?
        {
            Publication::Faite { remplacee } => remplacee,
            Publication::Impossible => return Err(regle("bibliotheque.modifiee_ailleurs")),
        };
        if let Some(ancienne) = &remplacee {
            unite.auditer(
                "version_contenu",
                ancienne,
                Operation::Modification,
                Some(serde_json::json!({ "etat": EtatVersion::Publiee.nom() })),
                Some(serde_json::json!({ "etat": EtatVersion::Remplacee.nom() })),
                "bibliotheque.publier",
            );
        }
        unite.auditer(
            "version_contenu",
            &demande.version_id,
            Operation::Modification,
            Some(serde_json::json!({ "etat": etat_avant })),
            Some(serde_json::json!({
                "etat": EtatVersion::Publiee.nom(),
                "remplace": remplacee,
            })),
            "bibliotheque.publier",
        );
        unite.emettre(
            TypeEvenement::VersionContenuModifiee,
            "version_contenu",
            &demande.version_id,
            serde_json::json!({
                "contenu_id": demande.contenu_id,
                "numero": numero,
                "etat": EtatVersion::Publiee.nom(),
            }),
        );
        reindexer_contenu(unite.connexion(), &contexte, &contenu, &paquets).await?;
        self.noyau.terminer(unite).await?;
        self.rouvrir(session, &demande.contenu_id).await
    }

    /// Renvoie une version soumise avec un commentaire : elle passe à « renvoyée », et son auteur
    /// retrouve son texte dans un brouillon nouveau, avec le commentaire.
    pub async fn renvoyer(
        &self,
        session: &Session,
        demande: &DemandeRenvoiVersion,
    ) -> Resultat<EcritureOuverte> {
        let contexte = self.noyau.contexte(session.etablissement_id, Some(session));
        let commentaire = demande.commentaire.trim();
        let mut unite = self
            .noyau
            .commencer(contexte.clone(), Durabilite::Critique)
            .await?;
        let versions = depot::versions(unite.connexion(), &demande.contenu_id, &contexte).await?;
        let version = versions
            .iter()
            .find(|v| v.id == demande.version_id && v.etat == "soumise")
            .ok_or(regle("bibliotheque.modifiee_ailleurs"))?
            .clone();
        if !depot::renvoyer_version(
            unite.connexion(),
            &demande.version_id,
            commentaire,
            &contexte,
        )
        .await?
        {
            return Err(regle("bibliotheque.modifiee_ailleurs"));
        }
        unite.auditer(
            "version_contenu",
            &demande.version_id,
            Operation::Modification,
            Some(serde_json::json!({ "etat": EtatVersion::Soumise.nom() })),
            Some(serde_json::json!({
                "etat": EtatVersion::Renvoyee.nom(),
                "commentaire": commentaire,
            })),
            "bibliotheque.renvoyer",
        );
        unite.emettre(
            TypeEvenement::VersionContenuModifiee,
            "version_contenu",
            &demande.version_id,
            serde_json::json!({
                "contenu_id": demande.contenu_id,
                "numero": version.numero,
                "etat": EtatVersion::Renvoyee.nom(),
            }),
        );
        // Le texte revient à son auteur, dans un brouillon nouveau qui part de la version
        // renvoyée (un brouillon commencé entre-temps est gardé tel quel).
        if !versions.iter().any(|v| v.etat == "brouillon") {
            let (id, numero) = depot::inserer_brouillon(
                unite.connexion(),
                &depot::NouvelleVersion {
                    contenu_id: &demande.contenu_id,
                    texte: &version.texte,
                    empreinte: &empreinte(version.texte.as_bytes()),
                    retour_livre: version.retour_livre != 0,
                    depart_version_id: Some(&version.id),
                    depart_livre_sha256: version.depart_livre_sha256.as_deref(),
                    auteur_id: &version.auteur_id,
                },
                &contexte,
            )
            .await?;
            unite.auditer(
                "version_contenu",
                &id,
                Operation::Creation,
                None,
                Some(serde_json::json!({
                    "contenu_id": demande.contenu_id,
                    "numero": numero,
                    "etat": EtatVersion::Brouillon.nom(),
                    "depart_version_id": version.id,
                    "auteur_id": version.auteur_id,
                })),
                "bibliotheque.renvoyer",
            );
            unite.emettre(
                TypeEvenement::VersionContenuModifiee,
                "version_contenu",
                &id,
                serde_json::json!({
                    "contenu_id": demande.contenu_id,
                    "numero": numero,
                    "etat": EtatVersion::Brouillon.nom(),
                }),
            );
        }
        self.noyau.terminer(unite).await?;
        self.rouvrir(session, &demande.contenu_id).await
    }

    /// Les versions en attente de validation, les plus anciennes d'abord.
    pub async fn en_attente(&self, session: &Session) -> Resultat<Vec<VersionEnAttente>> {
        let contexte = self.noyau.contexte(session.etablissement_id, Some(session));
        let mut cnx = self.connexion().await?;
        let soumises = depot::versions_soumises(&mut cnx, &contexte).await?;
        if soumises.is_empty() {
            return Ok(Vec::new());
        }
        let contenus: HashMap<String, _> = depot::contenus(&mut cnx, &contexte)
            .await?
            .into_iter()
            .map(|c| (c.id.vers_chaine(), c))
            .collect();
        let chapitres: HashMap<String, _> = depot::chapitres(&mut cnx, &contexte)
            .await?
            .into_iter()
            .map(|c| (c.id.vers_chaine(), c))
            .collect();
        let niveaux: HashMap<String, String> = academique::niveaux(&mut cnx, &contexte)
            .await?
            .into_iter()
            .map(|n| (n.code, n.libelle))
            .collect();
        Ok(soumises
            .into_iter()
            .filter_map(|v| {
                let contenu = contenus.get(&v.contenu_id)?;
                let chapitre = chapitres.get(&contenu.chapitre_id.vers_chaine());
                Some(VersionEnAttente {
                    contenu_id: v.contenu_id.clone(),
                    version_id: v.id,
                    titre: contenu.titre.clone(),
                    niveau: chapitre
                        .map(|c| {
                            niveaux
                                .get(&c.niveau_code)
                                .cloned()
                                .unwrap_or_else(|| c.niveau_code.clone())
                        })
                        .unwrap_or_default(),
                    chapitre: chapitre.map(|c| c.titre.clone()).unwrap_or_default(),
                    auteur: v.auteur,
                    soumise_le: v.soumise_le.unwrap_or(v.mis_a_jour_le),
                    retour_livre: v.retour_livre != 0,
                })
            })
            .collect())
    }

    /// « Peuvent publier eux-mêmes » : les maîtres qui ont le droit, et ceux à qui le donner.
    pub async fn droits_publication(&self, session: &Session) -> Resultat<DroitsPublication> {
        let contexte = self.noyau.contexte(session.etablissement_id, Some(session));
        let mut cnx = self.connexion().await?;
        let droits: Vec<DroitPublicationAffiche> = depot::droits_publication(&mut cnx, &contexte)
            .await?
            .into_iter()
            .map(|d| DroitPublicationAffiche {
                utilisateur_id: d.utilisateur_id,
                nom: d.nom,
                donne_le: d.donne_le,
                donne_par: d.donne_par,
            })
            .collect();
        let maitres = depot::maitres_avec_compte(&mut cnx, &contexte)
            .await?
            .into_iter()
            .filter(|(id, _)| !droits.iter().any(|d| &d.utilisateur_id == id))
            .map(|(utilisateur_id, nom)| MaitreChoisissable {
                utilisateur_id,
                nom,
            })
            .collect();
        Ok(DroitsPublication { droits, maitres })
    }

    /// Donne le droit de publier à un maître (un compte lié à un enseignant actif). Le donner
    /// à qui l'a déjà ne change rien.
    pub async fn donner_droit_publication(
        &self,
        session: &Session,
        demande: &DemandeDroitPublication,
    ) -> Resultat<DroitsPublication> {
        let contexte = self.noyau.contexte(session.etablissement_id, Some(session));
        let mut unite = self
            .noyau
            .commencer(contexte.clone(), Durabilite::Critique)
            .await?;
        if !depot::maitres_avec_compte(unite.connexion(), &contexte)
            .await?
            .iter()
            .any(|(id, _)| id == &demande.utilisateur_id)
        {
            return Err(introuvable("enseignant", &demande.utilisateur_id));
        }
        if !depot::a_le_droit_de_publier(unite.connexion(), &demande.utilisateur_id, &contexte)
            .await?
        {
            let id = depot::donner_droit_publication(
                unite.connexion(),
                &demande.utilisateur_id,
                &contexte,
            )
            .await?;
            unite.auditer(
                "droit_publication",
                &id,
                Operation::Creation,
                None,
                Some(serde_json::json!({ "utilisateur_id": demande.utilisateur_id })),
                "bibliotheque.droit_publication_donner",
            );
            unite.emettre(
                TypeEvenement::DroitPublicationModifie,
                "droit_publication",
                &id,
                serde_json::json!({ "utilisateur_id": demande.utilisateur_id, "donne": true }),
            );
            self.noyau.terminer(unite).await?;
        }
        self.droits_publication(session).await
    }

    /// Retire le droit de publier d'un maître ; vaut aussitôt. Le retirer à qui ne l'a pas ne
    /// change rien.
    pub async fn retirer_droit_publication(
        &self,
        session: &Session,
        demande: &DemandeDroitPublication,
    ) -> Resultat<DroitsPublication> {
        let contexte = self.noyau.contexte(session.etablissement_id, Some(session));
        let mut unite = self
            .noyau
            .commencer(contexte.clone(), Durabilite::Critique)
            .await?;
        if let Some(id) =
            depot::retirer_droit_publication(unite.connexion(), &demande.utilisateur_id, &contexte)
                .await?
        {
            unite.auditer(
                "droit_publication",
                &id,
                Operation::Modification,
                Some(serde_json::json!({ "utilisateur_id": demande.utilisateur_id })),
                Some(serde_json::json!({
                    "utilisateur_id": demande.utilisateur_id,
                    "retire": true,
                })),
                "bibliotheque.droit_publication_retirer",
            );
            unite.emettre(
                TypeEvenement::DroitPublicationModifie,
                "droit_publication",
                &id,
                serde_json::json!({ "utilisateur_id": demande.utilisateur_id, "donne": false }),
            );
            self.noyau.terminer(unite).await?;
        }
        self.droits_publication(session).await
    }
}
