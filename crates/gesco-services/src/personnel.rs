//! Registre du personnel (SPEC_PHASE_6 §2) : la direction enregistre tout le personnel, dans des
//! catégories et des fonctions que l'école choisit (T-39), tient le dossier de chacun et ses
//! pièces. Un enseignant de la phase 4 est le membre qui partage sa personne (Q-63).

use std::path::Path;

use gesco_core::commun::{DateCivile, Id};
use gesco_core::core::{Etablissement, Operation, TypeEvenement, Utilisateur};
use gesco_core::pedagogie::{Enseignant, StatutEnseignant};
use gesco_core::rh::personnel::{
    CATEGORIES_LIVREES, FONCTIONS_LIVREES, TAILLE_MAX_PIECE, extension_piece,
    fonction_d_enseignant, matricule_suivant, verifier_membre,
};
use gesco_core::rh::{
    CategoriePersonnel, Diplome, FonctionPersonnel, MembrePersonnel, MoyenPaiement, NaturePiece,
    PieceDossier, SituationFamille,
};
use gesco_core::scolarite::telephone::{normaliser, normaliser_liste};
use gesco_core::scolarite::{Personne, Sexe};
use gesco_db::depots::{enseignants, personnel, personnes, utilisateur};
use gesco_db::unite::Durabilite;
use gesco_db::{UniteDeTravail, nouvel_id};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::effets::Effet;
use crate::erreur::{ErreurService, Resultat};
use crate::noyau::Noyau;
use crate::session::Session;

fn regle(cle: &'static str) -> ErreurService {
    ErreurService::Regle { cle }
}

/// Catégories et fonctions de l'école.
#[derive(Debug, Clone, Serialize, specta::Type)]
pub struct ReferentielPersonnel {
    /// Catégories, retirées comprises.
    pub categories: Vec<CategoriePersonnel>,
    /// Fonctions, retirées comprises.
    pub fonctions: Vec<FonctionPersonnel>,
}

/// Ligne du registre.
#[derive(Debug, Clone, Serialize, specta::Type)]
pub struct MembreAffiche {
    /// Membre.
    pub membre: MembrePersonnel,
    /// Nom.
    pub nom: String,
    /// Prénoms.
    pub prenoms: String,
    /// Téléphones, forme canonique.
    pub telephones: Vec<String>,
    /// Libellé de la catégorie.
    pub categorie: String,
    /// Vrai s'il est aussi enseignant actif.
    pub enseignant: bool,
}

/// Fiche complète d'un membre.
#[derive(Debug, Clone, Serialize, specta::Type)]
pub struct FicheMembre {
    /// Membre.
    pub membre: MembrePersonnel,
    /// Personne.
    pub personne: Personne,
    /// Libellé de la catégorie.
    pub categorie: String,
    /// Pièces du dossier, retirées comprises.
    pub pieces: Vec<PieceDossier>,
    /// L'enseignant qu'il est, s'il l'est.
    pub enseignant: Option<Enseignant>,
    /// Nom affiché du compte lié.
    pub compte: Option<String>,
}

/// Fiche du membre lié au compte connecté, avec ses engagements montants compris
/// (SPEC_PHASE_6 §7) : chacun lit la sienne sans aucun droit.
#[derive(Debug, Clone, Serialize, specta::Type)]
pub struct MaFiche {
    /// Fiche.
    pub fiche: FicheMembre,
    /// Engagements, du plus ancien au plus récent.
    pub engagements: Vec<crate::engagements::EngagementAffiche>,
}

/// Membre saisi ; sans `id`, une création.
#[derive(Debug, Clone, Deserialize, specta::Type)]
pub struct DemandeMembre {
    /// Membre modifié ; absent pour une création.
    pub id: Option<Id<MembrePersonnel>>,
    /// Nom.
    pub nom: String,
    /// Prénoms.
    pub prenoms: String,
    /// Sexe.
    pub sexe: Option<Sexe>,
    /// Date de naissance.
    pub date_naissance: Option<DateCivile>,
    /// Téléphones, tels que saisis.
    pub telephones: Vec<String>,
    /// Adresse.
    pub adresse: String,
    /// Matricule ; vide, le suivant est attribué (Q-64).
    pub matricule: String,
    /// Catégorie.
    pub categorie_id: Id<CategoriePersonnel>,
    /// Fonction, choisie ou écrite.
    pub fonction: String,
    /// Arrivée dans l'école.
    pub arrive_le: Option<DateCivile>,
    /// Fonctionnaire de l'État.
    pub fonctionnaire: bool,
    /// Numéro de fonctionnaire.
    pub numero_fonctionnaire: String,
    /// Numéro INPS.
    pub numero_inps: String,
    /// Personne à prévenir.
    pub prevenir_nom: String,
    /// Son téléphone, tel que saisi.
    pub prevenir_telephone: String,
    /// Moyen de paiement.
    pub paiement: MoyenPaiement,
    /// Diplômes et qualifications.
    pub diplomes: Vec<Diplome>,
    /// Compte lié.
    pub utilisateur_id: Option<Id<Utilisateur>>,
    /// Situation de famille, pour la réduction d'impôt pour charges de famille.
    pub situation_famille: Option<SituationFamille>,
    /// Enfants à charge.
    pub enfants_a_charge: Option<i64>,
}

/// Membre désigné.
#[derive(Debug, Clone, Deserialize, specta::Type)]
pub struct DemandeIdMembre {
    /// Membre.
    pub id: Id<MembrePersonnel>,
}

/// Départ d'un membre.
#[derive(Debug, Clone, Deserialize, specta::Type)]
pub struct DemandeDepartMembre {
    /// Membre.
    pub id: Id<MembrePersonnel>,
    /// Jour du départ.
    pub le: DateCivile,
    /// Motif.
    pub motif: String,
}

/// Un membre devient enseignant.
#[derive(Debug, Clone, Deserialize, specta::Type)]
pub struct DemandeMembreEnseignant {
    /// Membre.
    pub id: Id<MembrePersonnel>,
    /// Statut d'enseignant.
    pub statut: StatutEnseignant,
}

/// Catégorie saisie ; sans `id`, une création.
#[derive(Debug, Clone, Deserialize, specta::Type)]
pub struct DemandeCategoriePersonnel {
    /// Catégorie modifiée.
    pub id: Option<Id<CategoriePersonnel>>,
    /// Libellé.
    pub libelle: String,
    /// Faux pour la retirer.
    pub active: bool,
}

/// Fonction saisie ; sans `id`, une création.
#[derive(Debug, Clone, Deserialize, specta::Type)]
pub struct DemandeFonctionPersonnel {
    /// Fonction modifiée.
    pub id: Option<Id<FonctionPersonnel>>,
    /// Catégorie.
    pub categorie_id: Id<CategoriePersonnel>,
    /// Libellé.
    pub libelle: String,
    /// Faux pour la retirer de la liste.
    pub active: bool,
}

/// Pièce à ajouter : un fichier choisi par le dialogue du système.
#[derive(Debug, Clone, Deserialize, specta::Type)]
pub struct DemandePieceDossier {
    /// Membre.
    pub membre_id: Id<MembrePersonnel>,
    /// Intitulé.
    pub intitule: String,
    /// Nature.
    pub nature: NaturePiece,
    /// Chemin du fichier.
    pub chemin: String,
}

/// Pièce désignée.
#[derive(Debug, Clone, Deserialize, specta::Type)]
pub struct DemandeIdPiece {
    /// Pièce.
    pub id: Id<PieceDossier>,
}

/// Pièce à enregistrer hors de GESCO, au chemin choisi.
#[derive(Debug, Clone, Deserialize, specta::Type)]
pub struct DemandePieceSous {
    /// Pièce.
    pub id: Id<PieceDossier>,
    /// Chemin choisi par le dialogue du système.
    pub chemin: String,
}

/// Pose dans l'unité les catégories, fonctions et types de congé livrés qui manquent, reconnus
/// par leur code. Rend le nombre de lignes ajoutées.
pub(crate) async fn poser_personnel(
    unite: &mut UniteDeTravail<Effet>,
    etablissement_id: Id<Etablissement>,
    origine: &str,
) -> Resultat<usize> {
    let contexte = unite.contexte().clone();
    let mut categories = personnel::categories(unite.connexion(), &contexte).await?;
    let fonctions = personnel::fonctions(unite.connexion(), &contexte).await?;
    let mut ajoutes = 0;
    for (ordre, (code, libelle)) in (1_i64..).zip(CATEGORIES_LIVREES) {
        if categories.iter().any(|c| c.code.as_deref() == Some(code)) {
            continue;
        }
        let c = CategoriePersonnel {
            id: nouvel_id(),
            etablissement_id,
            code: Some(code.to_owned()),
            libelle: libelle.to_owned(),
            ordre,
            active: true,
        };
        personnel::inserer_categorie(unite.connexion(), &c, &contexte).await?;
        unite.auditer(
            "categorie_personnel",
            &c.id.vers_chaine(),
            Operation::Creation,
            None,
            Some(serde_json::to_value(&c)?),
            origine,
        );
        categories.push(c);
        ajoutes += 1;
    }
    for (ordre, (code, categorie, libelle)) in (1_i64..).zip(FONCTIONS_LIVREES) {
        if fonctions.iter().any(|f| f.code.as_deref() == Some(code)) {
            continue;
        }
        let Some(categorie) = categories
            .iter()
            .find(|c| c.code.as_deref() == Some(categorie))
        else {
            continue;
        };
        let f = FonctionPersonnel {
            id: nouvel_id(),
            etablissement_id,
            categorie_id: categorie.id,
            code: Some(code.to_owned()),
            libelle: libelle.to_owned(),
            ordre,
            active: true,
        };
        personnel::inserer_fonction(unite.connexion(), &f, &contexte).await?;
        unite.auditer(
            "fonction_personnel",
            &f.id.vers_chaine(),
            Operation::Creation,
            None,
            Some(serde_json::to_value(&f)?),
            origine,
        );
        ajoutes += 1;
    }
    ajoutes += crate::conges::poser_types_conge(unite, etablissement_id, origine).await?;
    Ok(ajoutes)
}

/// Fait de la personne d'un enseignant un membre du personnel, s'il ne l'est pas déjà : catégorie
/// *Enseignement*, fonction *maître* au 1er cycle, *enseignant* sinon (Q-63). Rend le membre créé.
pub(crate) async fn rattacher_enseignant(
    unite: &mut UniteDeTravail<Effet>,
    enseignant: &Enseignant,
    au_premier_cycle: bool,
    origine: &str,
) -> Resultat<Option<MembrePersonnel>> {
    let contexte = unite.contexte().clone();
    if personnel::membre_de_personne(unite.connexion(), enseignant.personne_id, &contexte)
        .await?
        .is_some()
    {
        return Ok(None);
    }
    poser_personnel(unite, enseignant.etablissement_id, origine).await?;
    let categories = personnel::categories(unite.connexion(), &contexte).await?;
    let fonctions = personnel::fonctions(unite.connexion(), &contexte).await?;
    let Some(categorie) = categories
        .iter()
        .find(|c| c.code.as_deref() == Some("enseignement"))
    else {
        return Ok(None);
    };
    let code = fonction_d_enseignant(au_premier_cycle);
    let fonction = fonctions
        .iter()
        .find(|f| f.code.as_deref() == Some(code))
        .map_or_else(|| "Enseignant".to_owned(), |f| f.libelle.clone());
    let membres = personnel::membres(unite.connexion(), &contexte).await?;
    let compte_libre = match enseignant.utilisateur_id {
        Some(u) => !membres.iter().any(|m| m.utilisateur_id == Some(u)),
        None => false,
    };
    let m = MembrePersonnel {
        id: nouvel_id(),
        etablissement_id: enseignant.etablissement_id,
        personne_id: enseignant.personne_id,
        matricule: matricule_suivant(membres.iter().map(|m| m.matricule.as_str())),
        categorie_id: categorie.id,
        fonction,
        arrive_le: None,
        parti_le: None,
        motif_depart: String::new(),
        fonctionnaire: false,
        numero_fonctionnaire: String::new(),
        numero_inps: String::new(),
        prevenir_nom: String::new(),
        prevenir_telephone: String::new(),
        paiement: MoyenPaiement::Especes,
        diplomes: Vec::new(),
        utilisateur_id: if compte_libre {
            enseignant.utilisateur_id
        } else {
            None
        },
        situation_famille: None,
        enfants_a_charge: None,
    };
    personnel::inserer_membre(unite.connexion(), &m, &contexte).await?;
    let id = m.id.vers_chaine();
    unite.auditer(
        "membre_personnel",
        &id,
        Operation::Creation,
        None,
        Some(serde_json::to_value(&m)?),
        origine,
    );
    unite.emettre(
        TypeEvenement::MembreEnregistre,
        "membre_personnel",
        &id,
        serde_json::json!({ "enseignant": enseignant.id }),
    );
    Ok(Some(m))
}

/// Service du personnel.
pub struct ServicePersonnel<'n> {
    noyau: &'n Noyau,
}

impl<'n> ServicePersonnel<'n> {
    /// Service sur le noyau ouvert.
    pub const fn nouveau(noyau: &'n Noyau) -> Self {
        Self { noyau }
    }

    async fn connexion(&self) -> Resultat<gesco_db::ConnexionEmpruntee> {
        Ok(self
            .noyau
            .base()
            .pool()
            .acquire()
            .await
            .map_err(gesco_db::ErreurBase::from)?)
    }

    /// Catégories et fonctions de l'école.
    pub async fn referentiel(&self, session: &Session) -> Resultat<ReferentielPersonnel> {
        let contexte = self.noyau.contexte(session.etablissement_id, Some(session));
        let mut cnx = self.connexion().await?;
        Ok(ReferentielPersonnel {
            categories: personnel::categories(&mut cnx, &contexte).await?,
            fonctions: personnel::fonctions(&mut cnx, &contexte).await?,
        })
    }

    /// Crée, renomme, retire ou remet une catégorie.
    pub async fn enregistrer_categorie(
        &self,
        session: &Session,
        demande: &DemandeCategoriePersonnel,
    ) -> Resultat<CategoriePersonnel> {
        let libelle = demande.libelle.trim().to_owned();
        if libelle.is_empty() {
            return Err(regle("personnel.libelle_obligatoire"));
        }
        let contexte = self.noyau.contexte(session.etablissement_id, Some(session));
        let mut unite = self
            .noyau
            .commencer(contexte.clone(), Durabilite::Critique)
            .await?;
        let existantes = personnel::categories(unite.connexion(), &contexte).await?;
        if existantes.iter().any(|c| {
            c.active
                && Some(c.id) != demande.id
                && c.libelle.to_lowercase() == libelle.to_lowercase()
        }) {
            return Err(regle("personnel.libelle_existe"));
        }
        let avant = match demande.id {
            Some(id) => Some(
                existantes
                    .iter()
                    .find(|c| c.id == id)
                    .cloned()
                    .ok_or(regle("personnel.categorie_inconnue"))?,
            ),
            None => None,
        };
        let apres = match &avant {
            Some(a) => CategoriePersonnel {
                libelle,
                active: demande.active,
                ..a.clone()
            },
            None => CategoriePersonnel {
                id: nouvel_id(),
                etablissement_id: session.etablissement_id,
                code: None,
                libelle,
                ordre: existantes.iter().map(|c| c.ordre).max().unwrap_or(0) + 1,
                active: true,
            },
        };
        match &avant {
            Some(_) => personnel::ecrire_categorie(unite.connexion(), &apres, &contexte).await?,
            None => personnel::inserer_categorie(unite.connexion(), &apres, &contexte).await?,
        }
        let id = apres.id.vers_chaine();
        unite.auditer(
            "categorie_personnel",
            &id,
            if avant.is_some() {
                Operation::Modification
            } else {
                Operation::Creation
            },
            avant.as_ref().map(serde_json::to_value).transpose()?,
            Some(serde_json::to_value(&apres)?),
            "personnel.categorie",
        );
        unite.emettre(
            TypeEvenement::ReferentielPersonnelModifie,
            "categorie_personnel",
            &id,
            serde_json::json!({ "active": apres.active }),
        );
        self.noyau.terminer(unite).await?;
        Ok(apres)
    }

    /// Crée, renomme, déplace, retire ou remet une fonction.
    pub async fn enregistrer_fonction(
        &self,
        session: &Session,
        demande: &DemandeFonctionPersonnel,
    ) -> Resultat<FonctionPersonnel> {
        let libelle = demande.libelle.trim().to_owned();
        if libelle.is_empty() {
            return Err(regle("personnel.libelle_obligatoire"));
        }
        let contexte = self.noyau.contexte(session.etablissement_id, Some(session));
        let mut unite = self
            .noyau
            .commencer(contexte.clone(), Durabilite::Critique)
            .await?;
        if !personnel::categories(unite.connexion(), &contexte)
            .await?
            .iter()
            .any(|c| c.id == demande.categorie_id)
        {
            return Err(regle("personnel.categorie_inconnue"));
        }
        let existantes = personnel::fonctions(unite.connexion(), &contexte).await?;
        if existantes.iter().any(|f| {
            f.active
                && Some(f.id) != demande.id
                && f.libelle.to_lowercase() == libelle.to_lowercase()
        }) {
            return Err(regle("personnel.libelle_existe"));
        }
        let avant = match demande.id {
            Some(id) => Some(
                existantes
                    .iter()
                    .find(|f| f.id == id)
                    .cloned()
                    .ok_or(regle("personnel.fonction_inconnue"))?,
            ),
            None => None,
        };
        let apres = match &avant {
            Some(a) => FonctionPersonnel {
                categorie_id: demande.categorie_id,
                libelle,
                active: demande.active,
                ..a.clone()
            },
            None => FonctionPersonnel {
                id: nouvel_id(),
                etablissement_id: session.etablissement_id,
                categorie_id: demande.categorie_id,
                code: None,
                libelle,
                ordre: existantes.iter().map(|f| f.ordre).max().unwrap_or(0) + 1,
                active: true,
            },
        };
        match &avant {
            Some(_) => personnel::ecrire_fonction(unite.connexion(), &apres, &contexte).await?,
            None => personnel::inserer_fonction(unite.connexion(), &apres, &contexte).await?,
        }
        let id = apres.id.vers_chaine();
        unite.auditer(
            "fonction_personnel",
            &id,
            if avant.is_some() {
                Operation::Modification
            } else {
                Operation::Creation
            },
            avant.as_ref().map(serde_json::to_value).transpose()?,
            Some(serde_json::to_value(&apres)?),
            "personnel.fonction",
        );
        unite.emettre(
            TypeEvenement::ReferentielPersonnelModifie,
            "fonction_personnel",
            &id,
            serde_json::json!({ "active": apres.active }),
        );
        self.noyau.terminer(unite).await?;
        Ok(apres)
    }

    /// Registre : tout le personnel, partis compris, par nom.
    pub async fn lister(&self, session: &Session) -> Resultat<Vec<MembreAffiche>> {
        let contexte = self.noyau.contexte(session.etablissement_id, Some(session));
        let mut cnx = self.connexion().await?;
        let categories = personnel::categories(&mut cnx, &contexte).await?;
        let enseignants = enseignants::enseignants(&mut cnx, &contexte).await?;
        let mut liste = Vec::new();
        for m in personnel::membres(&mut cnx, &contexte).await? {
            let p = personnes::personne(&mut cnx, m.personne_id, &contexte).await?;
            liste.push(MembreAffiche {
                categorie: categories
                    .iter()
                    .find(|c| c.id == m.categorie_id)
                    .map(|c| c.libelle.clone())
                    .unwrap_or_default(),
                enseignant: enseignants
                    .iter()
                    .any(|e| e.personne_id == m.personne_id && e.actif),
                nom: p.nom,
                prenoms: p.prenoms,
                telephones: p.telephones,
                membre: m,
            });
        }
        liste.sort_by(|a, b| a.nom.cmp(&b.nom).then_with(|| a.prenoms.cmp(&b.prenoms)));
        Ok(liste)
    }

    /// Fiche d'un membre.
    pub async fn lire(&self, session: &Session, id: Id<MembrePersonnel>) -> Resultat<FicheMembre> {
        let contexte = self.noyau.contexte(session.etablissement_id, Some(session));
        let mut cnx = self.connexion().await?;
        let membre = personnel::membre(&mut cnx, id, &contexte).await?;
        let personne = personnes::personne(&mut cnx, membre.personne_id, &contexte).await?;
        let categorie = personnel::categories(&mut cnx, &contexte)
            .await?
            .into_iter()
            .find(|c| c.id == membre.categorie_id)
            .map(|c| c.libelle)
            .unwrap_or_default();
        let enseignant = enseignants::enseignants(&mut cnx, &contexte)
            .await?
            .into_iter()
            .find(|e| e.personne_id == membre.personne_id);
        let compte = match membre.utilisateur_id {
            Some(u) => utilisateur::lire(&mut cnx, u, &contexte)
                .await
                .ok()
                .map(|u| u.nom_affiche),
            None => None,
        };
        Ok(FicheMembre {
            pieces: personnel::pieces(&mut cnx, id, &contexte).await?,
            membre,
            personne,
            categorie,
            enseignant,
            compte,
        })
    }

    /// Fiche du compte connecté, s'il est lié à un membre du personnel.
    pub async fn ma_fiche(&self, session: &Session) -> Resultat<Option<MaFiche>> {
        let contexte = self.noyau.contexte(session.etablissement_id, Some(session));
        let mut cnx = self.connexion().await?;
        let membre = personnel::membres(&mut cnx, &contexte)
            .await?
            .into_iter()
            .find(|m| m.utilisateur_id == Some(session.utilisateur_id));
        drop(cnx);
        let Some(membre) = membre else {
            return Ok(None);
        };
        let fiche = self.lire(session, membre.id).await?;
        let engagements = crate::engagements::ServiceEngagements::nouveau(self.noyau)
            .lister(session, membre.id)
            .await?;
        Ok(Some(MaFiche { fiche, engagements }))
    }

    /// Crée ou modifie un membre ; le départ passe par [`Self::partir`].
    pub async fn enregistrer(
        &self,
        session: &Session,
        demande: &DemandeMembre,
    ) -> Resultat<FicheMembre> {
        let nom = demande.nom.trim().to_owned();
        let prenoms = demande.prenoms.trim().to_owned();
        if nom.is_empty() || prenoms.is_empty() {
            return Err(regle("personnel.nom_obligatoire"));
        }
        let telephones = normaliser_liste(&demande.telephones).map_err(regle)?;
        let prevenir_telephone = match demande.prevenir_telephone.trim() {
            "" => String::new(),
            t => normaliser(t).map_err(regle)?,
        };
        let paiement = match &demande.paiement {
            MoyenPaiement::MobileMoney { numero } if !numero.trim().is_empty() => {
                MoyenPaiement::MobileMoney {
                    numero: normaliser(numero).map_err(regle)?,
                }
            }
            MoyenPaiement::Banque { banque, compte } => MoyenPaiement::Banque {
                banque: banque.trim().to_owned(),
                compte: compte.trim().to_owned(),
            },
            autre => autre.clone(),
        };
        let diplomes: Vec<Diplome> = demande
            .diplomes
            .iter()
            .map(|d| Diplome {
                intitule: d.intitule.trim().to_owned(),
                delivre_par: d.delivre_par.trim().to_owned(),
                annee: d.annee,
            })
            .collect();
        let contexte = self.noyau.contexte(session.etablissement_id, Some(session));
        let mut unite = self
            .noyau
            .commencer(contexte.clone(), Durabilite::Critique)
            .await?;
        let membres = personnel::membres(unite.connexion(), &contexte).await?;
        let avant = match demande.id {
            Some(id) => Some(
                membres
                    .iter()
                    .find(|m| m.id == id)
                    .cloned()
                    .ok_or(regle("personnel.membre_inconnu"))?,
            ),
            None => None,
        };
        let categories = personnel::categories(unite.connexion(), &contexte).await?;
        let categorie = categories
            .iter()
            .find(|c| c.id == demande.categorie_id)
            .ok_or(regle("personnel.categorie_inconnue"))?;
        // Une catégorie retirée reste sur les fiches qui la portent, sans en recevoir d'autres.
        if !categorie.active && avant.as_ref().map(|a| a.categorie_id) != Some(categorie.id) {
            return Err(regle("personnel.categorie_retiree"));
        }
        let matricule = match demande.matricule.trim() {
            "" => avant.as_ref().map_or_else(
                || matricule_suivant(membres.iter().map(|m| m.matricule.as_str())),
                |a| a.matricule.clone(),
            ),
            m => m.to_owned(),
        };
        if membres
            .iter()
            .any(|m| m.matricule == matricule && Some(m.id) != demande.id)
        {
            return Err(regle("personnel.matricule_pris"));
        }
        if let Some(u) = demande.utilisateur_id {
            if membres
                .iter()
                .any(|m| m.utilisateur_id == Some(u) && Some(m.id) != demande.id)
            {
                return Err(regle("personnel.compte_deja_lie"));
            }
            utilisateur::lire(unite.connexion(), u, &contexte).await?;
        }
        let personne = match &avant {
            Some(a) => {
                let ancienne =
                    personnes::personne(unite.connexion(), a.personne_id, &contexte).await?;
                let p = Personne {
                    nom: nom.clone(),
                    prenoms: prenoms.clone(),
                    sexe: demande.sexe,
                    date_naissance: demande.date_naissance,
                    adresse: demande.adresse.trim().to_owned(),
                    telephones,
                    ..ancienne
                };
                personnes::modifier_personne(unite.connexion(), &p, &contexte).await?;
                p
            }
            None => {
                let p = Personne {
                    id: nouvel_id(),
                    etablissement_id: session.etablissement_id,
                    nom: nom.clone(),
                    prenoms: prenoms.clone(),
                    sexe: demande.sexe,
                    date_naissance: demande.date_naissance,
                    date_naissance_estimee: false,
                    lieu_naissance: String::new(),
                    nationalite: String::new(),
                    adresse: demande.adresse.trim().to_owned(),
                    telephones,
                };
                personnes::inserer_personne(unite.connexion(), &p, &contexte).await?;
                p
            }
        };
        let apres = MembrePersonnel {
            id: avant.as_ref().map_or_else(nouvel_id, |a| a.id),
            etablissement_id: session.etablissement_id,
            personne_id: personne.id,
            matricule,
            categorie_id: categorie.id,
            fonction: demande.fonction.trim().to_owned(),
            arrive_le: demande.arrive_le,
            parti_le: avant.as_ref().and_then(|a| a.parti_le),
            motif_depart: avant
                .as_ref()
                .map(|a| a.motif_depart.clone())
                .unwrap_or_default(),
            fonctionnaire: demande.fonctionnaire,
            numero_fonctionnaire: if demande.fonctionnaire {
                demande.numero_fonctionnaire.trim().to_owned()
            } else {
                String::new()
            },
            numero_inps: demande.numero_inps.trim().to_owned(),
            prevenir_nom: demande.prevenir_nom.trim().to_owned(),
            prevenir_telephone,
            paiement,
            diplomes,
            utilisateur_id: demande.utilisateur_id,
            situation_famille: demande.situation_famille,
            enfants_a_charge: demande.enfants_a_charge,
        };
        verifier_membre(&apres).map_err(regle)?;
        match &avant {
            Some(_) => personnel::ecrire_membre(unite.connexion(), &apres, &contexte).await?,
            None => personnel::inserer_membre(unite.connexion(), &apres, &contexte).await?,
        }
        let id = apres.id.vers_chaine();
        unite.auditer(
            "membre_personnel",
            &id,
            if avant.is_some() {
                Operation::Modification
            } else {
                Operation::Creation
            },
            avant.as_ref().map(serde_json::to_value).transpose()?,
            Some(serde_json::json!({ "membre": apres, "nom": nom, "prenoms": prenoms })),
            "personnel.enregistrer",
        );
        unite.emettre(
            TypeEvenement::MembreEnregistre,
            "membre_personnel",
            &id,
            serde_json::json!({ "creation": avant.is_none() }),
        );
        self.noyau.terminer(unite).await?;
        self.lire(session, apres.id).await
    }

    /// Départ d'un membre : il garde sa fiche ; l'enseignant qu'il est cesse d'être actif.
    pub async fn partir(
        &self,
        session: &Session,
        demande: &DemandeDepartMembre,
    ) -> Resultat<FicheMembre> {
        let contexte = self.noyau.contexte(session.etablissement_id, Some(session));
        let mut unite = self
            .noyau
            .commencer(contexte.clone(), Durabilite::Critique)
            .await?;
        let avant = personnel::membre(unite.connexion(), demande.id, &contexte).await?;
        if !avant.present() {
            return Err(regle("personnel.deja_parti"));
        }
        let apres = MembrePersonnel {
            parti_le: Some(demande.le),
            motif_depart: demande.motif.trim().to_owned(),
            ..avant.clone()
        };
        verifier_membre(&apres).map_err(regle)?;
        personnel::ecrire_membre(unite.connexion(), &apres, &contexte).await?;
        if let Some(e) = enseignants::enseignants(unite.connexion(), &contexte)
            .await?
            .into_iter()
            .find(|e| e.personne_id == apres.personne_id && e.actif)
        {
            let parti = Enseignant {
                actif: false,
                ..e.clone()
            };
            enseignants::ecrire_enseignant(unite.connexion(), &parti, &contexte).await?;
            unite.auditer(
                "enseignant",
                &e.id.vers_chaine(),
                Operation::Modification,
                Some(serde_json::to_value(&e)?),
                Some(serde_json::to_value(&parti)?),
                "personnel.partir",
            );
        }
        let id = apres.id.vers_chaine();
        unite.auditer(
            "membre_personnel",
            &id,
            Operation::Modification,
            Some(serde_json::to_value(&avant)?),
            Some(serde_json::to_value(&apres)?),
            "personnel.partir",
        );
        unite.emettre(
            TypeEvenement::MembreParti,
            "membre_personnel",
            &id,
            serde_json::json!({ "le": demande.le, "retour": false }),
        );
        self.noyau.terminer(unite).await?;
        self.lire(session, demande.id).await
    }

    /// Retour d'un membre parti ; l'enseignant qu'il était se réactive dans « Enseignants ».
    pub async fn revenir(
        &self,
        session: &Session,
        id: Id<MembrePersonnel>,
    ) -> Resultat<FicheMembre> {
        let contexte = self.noyau.contexte(session.etablissement_id, Some(session));
        let mut unite = self
            .noyau
            .commencer(contexte.clone(), Durabilite::Critique)
            .await?;
        let avant = personnel::membre(unite.connexion(), id, &contexte).await?;
        if avant.present() {
            return Err(regle("personnel.pas_parti"));
        }
        let apres = MembrePersonnel {
            parti_le: None,
            motif_depart: String::new(),
            ..avant.clone()
        };
        personnel::ecrire_membre(unite.connexion(), &apres, &contexte).await?;
        let texte = id.vers_chaine();
        unite.auditer(
            "membre_personnel",
            &texte,
            Operation::Modification,
            Some(serde_json::to_value(&avant)?),
            Some(serde_json::to_value(&apres)?),
            "personnel.revenir",
        );
        unite.emettre(
            TypeEvenement::MembreParti,
            "membre_personnel",
            &texte,
            serde_json::json!({ "retour": true }),
        );
        self.noyau.terminer(unite).await?;
        self.lire(session, id).await
    }

    /// Un membre devient enseignant, sans ressaisie : même personne, son compte s'il en a un ;
    /// un enseignant inactif de cette personne se réactive.
    pub async fn devenir_enseignant(
        &self,
        session: &Session,
        demande: &DemandeMembreEnseignant,
    ) -> Resultat<FicheMembre> {
        let contexte = self.noyau.contexte(session.etablissement_id, Some(session));
        let mut unite = self
            .noyau
            .commencer(contexte.clone(), Durabilite::Critique)
            .await?;
        let membre = personnel::membre(unite.connexion(), demande.id, &contexte).await?;
        if !membre.present() {
            return Err(regle("personnel.deja_parti"));
        }
        let existants = enseignants::enseignants(unite.connexion(), &contexte).await?;
        let avant = existants
            .iter()
            .find(|e| e.personne_id == membre.personne_id)
            .cloned();
        if avant.as_ref().is_some_and(|e| e.actif) {
            return Err(regle("personnel.deja_enseignant"));
        }
        let compte = membre
            .utilisateur_id
            .filter(|u| !existants.iter().any(|e| e.utilisateur_id == Some(*u)));
        let apres = match &avant {
            Some(e) => Enseignant {
                statut: demande.statut,
                actif: true,
                utilisateur_id: e.utilisateur_id.or(compte),
                ..e.clone()
            },
            None => Enseignant {
                id: nouvel_id(),
                etablissement_id: session.etablissement_id,
                personne_id: membre.personne_id,
                statut: demande.statut,
                utilisateur_id: compte,
                actif: true,
            },
        };
        match &avant {
            Some(_) => enseignants::ecrire_enseignant(unite.connexion(), &apres, &contexte).await?,
            None => enseignants::inserer_enseignant(unite.connexion(), &apres, &contexte).await?,
        }
        let id = apres.id.vers_chaine();
        unite.auditer(
            "enseignant",
            &id,
            if avant.is_some() {
                Operation::Modification
            } else {
                Operation::Creation
            },
            avant.as_ref().map(serde_json::to_value).transpose()?,
            Some(serde_json::to_value(&apres)?),
            "personnel.devenir_enseignant",
        );
        unite.emettre(
            TypeEvenement::EnseignantEnregistre,
            "enseignant",
            &id,
            serde_json::json!({ "statut": apres.statut, "compte": apres.utilisateur_id.is_some() }),
        );
        self.noyau.terminer(unite).await?;
        self.lire(session, demande.id).await
    }

    /// Ajoute au dossier une pièce lue sur le disque : PDF ou image, 2 Mo au plus.
    pub async fn ajouter_piece(
        &self,
        session: &Session,
        demande: &DemandePieceDossier,
    ) -> Resultat<PieceDossier> {
        let intitule = demande.intitule.trim().to_owned();
        if intitule.is_empty() {
            return Err(regle("personnel.piece_sans_intitule"));
        }
        let chemin = Path::new(&demande.chemin);
        let nom_fichier = chemin
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or_default()
            .to_owned();
        if extension_piece(&nom_fichier).is_none() {
            return Err(regle("personnel.piece_format"));
        }
        let taille = tokio::fs::metadata(chemin).await?.len();
        if taille == 0 || usize::try_from(taille).map_or(true, |t| t > TAILLE_MAX_PIECE) {
            return Err(regle("personnel.piece_trop_lourde"));
        }
        let contenu = tokio::fs::read(chemin).await?;
        let empreinte: String = Sha256::digest(&contenu)
            .iter()
            .map(|o| format!("{o:02x}"))
            .collect();
        let contexte = self.noyau.contexte(session.etablissement_id, Some(session));
        let mut unite = self
            .noyau
            .commencer(contexte.clone(), Durabilite::Critique)
            .await?;
        personnel::membre(unite.connexion(), demande.membre_id, &contexte).await?;
        let piece = PieceDossier {
            id: nouvel_id(),
            membre_id: demande.membre_id,
            intitule,
            nature: demande.nature,
            nom_fichier,
            taille: i64::try_from(contenu.len()).unwrap_or(i64::MAX),
            active: true,
        };
        personnel::inserer_piece(unite.connexion(), &piece, &contenu, &empreinte, &contexte)
            .await?;
        let id = piece.id.vers_chaine();
        unite.auditer(
            "piece_dossier",
            &id,
            Operation::Creation,
            None,
            Some(serde_json::json!({ "piece": piece, "empreinte_sha256": empreinte })),
            "personnel.piece_ajouter",
        );
        unite.emettre(
            TypeEvenement::PieceDossierAjoutee,
            "piece_dossier",
            &id,
            serde_json::json!({ "membre": piece.membre_id, "nature": piece.nature }),
        );
        self.noyau.terminer(unite).await?;
        Ok(piece)
    }

    /// Écrit une pièce du dossier au chemin choisi, pour l'ouvrir ou l'imprimer hors de GESCO.
    pub async fn enregistrer_piece_sous(
        &self,
        session: &Session,
        demande: &DemandePieceSous,
    ) -> Resultat<PieceDossier> {
        let contexte = self.noyau.contexte(session.etablissement_id, Some(session));
        let (piece, contenu) = {
            let mut cnx = self.connexion().await?;
            personnel::piece(&mut cnx, demande.id, &contexte).await?
        };
        tokio::fs::write(&demande.chemin, contenu).await?;
        Ok(piece)
    }

    /// Retire une pièce du dossier ; elle reste dans la base (R-14).
    pub async fn retirer_piece(
        &self,
        session: &Session,
        id: Id<PieceDossier>,
    ) -> Resultat<PieceDossier> {
        let contexte = self.noyau.contexte(session.etablissement_id, Some(session));
        let mut unite = self
            .noyau
            .commencer(contexte.clone(), Durabilite::Critique)
            .await?;
        let (avant, _) = personnel::piece(unite.connexion(), id, &contexte).await?;
        if !avant.active {
            return Err(regle("personnel.piece_deja_retiree"));
        }
        personnel::ecrire_piece_active(unite.connexion(), id, false, &contexte).await?;
        let apres = PieceDossier {
            active: false,
            ..avant.clone()
        };
        let texte = id.vers_chaine();
        unite.auditer(
            "piece_dossier",
            &texte,
            Operation::Modification,
            Some(serde_json::to_value(&avant)?),
            Some(serde_json::to_value(&apres)?),
            "personnel.piece_retirer",
        );
        unite.emettre(
            TypeEvenement::PieceDossierRetiree,
            "piece_dossier",
            &texte,
            serde_json::json!({ "membre": apres.membre_id }),
        );
        self.noyau.terminer(unite).await?;
        Ok(apres)
    }
}
