//! Grilles de frais et plans de paiement (SPEC_PHASE_3 §3) : saisie en brouillon, mise en
//! vigueur, copie d'une année à l'autre. La facturation (étape 3) lit la grille en vigueur.

use gesco_core::commun::{DateCivile, Id, Montant};
use gesco_core::core::{AnneeScolaire, MachineAEtats, Operation, StatutAnnee, TypeEvenement};
use gesco_core::finance::tarif::{
    JOUR_ECHEANCE_DEFAUT, plans_livres, un_an_plus_tard, verifier_lignes, verifier_plans,
};
use gesco_core::finance::{
    ContenuGrille, EtatGrille, Exigibilite, Grille, LigneTarifaire, PlanPaiement, Tranche,
    TypeFrais,
};
use gesco_core::scolarite::academique::libelle_valide;
use gesco_core::scolarite::{Niveau, Serie};
use gesco_db::depots::{academique, annee_scolaire, parametre, tarifs};
use gesco_db::nouvel_id;
use gesco_db::unite::Durabilite;
use serde::{Deserialize, Serialize};

use crate::erreur::{ErreurService, Resultat};
use crate::noyau::Noyau;
use crate::session::Session;

/// Jour du mois des échéances des grilles neuves, au choix de l'école.
pub const PARAMETRE_JOUR_ECHEANCE: &str = "facturation.jour_echeance";

fn regle(cle: &'static str) -> ErreurService {
    ErreurService::Regle { cle }
}

/// Année dont on veut les grilles.
#[derive(Debug, Deserialize, specta::Type)]
pub struct DemandeGrillesAnnee {
    /// Année scolaire.
    pub annee_id: Id<AnneeScolaire>,
}

/// Grille désignée.
#[derive(Debug, Deserialize, specta::Type)]
pub struct DemandeGrille {
    /// Grille.
    pub id: Id<Grille>,
}

/// Nouvelle grille : vide avec les plans livrés, ou copie d'une grille existante.
#[derive(Debug, Deserialize, specta::Type)]
pub struct DemandeCreationGrille {
    /// Année de la grille.
    pub annee_id: Id<AnneeScolaire>,
    /// Grille à recopier, de la même année ou d'une autre ; les dates suivent l'écart d'années.
    pub copie_de: Option<Id<Grille>>,
}

/// Ligne de frais saisie.
#[derive(Debug, Clone, Deserialize, specta::Type)]
pub struct SaisieLigne {
    /// Niveau.
    pub niveau_id: Id<Niveau>,
    /// Série, au lycée ; absente : tout le niveau.
    pub serie_id: Option<Id<Serie>>,
    /// Nature du frais.
    pub type_frais: TypeFrais,
    /// Libellé.
    pub libelle: String,
    /// Montant annuel.
    pub montant: Montant,
    /// Faux pour un frais coché à l'inscription.
    pub obligatoire: bool,
    /// Quand le frais est dû.
    pub exigibilite: Exigibilite,
}

/// Plan de paiement saisi.
#[derive(Debug, Clone, Deserialize, specta::Type)]
pub struct SaisiePlan {
    /// Libellé.
    pub libelle: String,
    /// Plan appliqué quand le parent n'en choisit pas.
    pub par_defaut: bool,
    /// Tranches, dans l'ordre des échéances.
    pub tranches: Vec<Tranche>,
}

/// Contenu d'un brouillon, remplacé en entier.
#[derive(Debug, Deserialize, specta::Type)]
pub struct DemandeModificationGrille {
    /// Grille en brouillon.
    pub id: Id<Grille>,
    /// Libellé.
    pub libelle: String,
    /// Lignes de frais.
    pub lignes: Vec<SaisieLigne>,
    /// Plans de paiement.
    pub plans: Vec<SaisiePlan>,
}

/// Grille avec son contenu.
#[derive(Debug, Clone, Serialize, specta::Type)]
pub struct GrilleAffichee {
    /// Grille.
    pub grille: Grille,
    /// Lignes et plans.
    pub contenu: ContenuGrille,
}

/// Résultat d'une mise en vigueur.
#[derive(Debug, Clone, Serialize, specta::Type)]
pub struct MiseEnVigueur {
    /// Grille en vigueur.
    pub grille: GrilleAffichee,
    /// Niveaux qui ont des classes cette année et aucune ligne de scolarité : signalés, sans
    /// blocage (une école peut ne pas faire payer le jardin).
    pub niveaux_sans_scolarite: Vec<String>,
}

/// Service des frais.
pub struct ServiceFrais<'n> {
    noyau: &'n Noyau,
}

/// Rentrée d'une année : l'année civile de son premier jour.
fn annee_de_rentree(annee: &AnneeScolaire) -> Resultat<i32> {
    DateCivile::du_jour(annee.debut)
        .map(|d| d.annee())
        .ok_or(regle("frais.annee_invalide"))
}

/// Décale une date d'un nombre d'années, positif ou négatif ; le 29 février devient le 28.
fn decaler(date: DateCivile, annees: i32) -> Option<DateCivile> {
    if annees >= 0 {
        (0..annees).try_fold(date, |d, _| un_an_plus_tard(d))
    } else {
        let annee = date.annee().checked_add(annees)?;
        DateCivile::nouvelle(annee, date.mois(), date.jour())
            .or_else(|| DateCivile::nouvelle(annee, date.mois(), 28))
    }
}

impl<'n> ServiceFrais<'n> {
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

    async fn affichee(&self, session: &Session, grille: Grille) -> Resultat<GrilleAffichee> {
        let contexte = self.noyau.contexte(session.etablissement_id, Some(session));
        let mut cnx = self.connexion().await?;
        let contenu = tarifs::contenu(&mut cnx, grille.id, &contexte).await?;
        Ok(GrilleAffichee { grille, contenu })
    }

    /// Grilles d'une année, la plus récente d'abord, avec leur contenu.
    pub async fn lister(
        &self,
        session: &Session,
        annee_id: Id<AnneeScolaire>,
    ) -> Resultat<Vec<GrilleAffichee>> {
        let contexte = self.noyau.contexte(session.etablissement_id, Some(session));
        let grilles = {
            let mut cnx = self.connexion().await?;
            tarifs::grilles(&mut cnx, annee_id, &contexte).await?
        };
        let mut affichees = Vec::with_capacity(grilles.len());
        for grille in grilles {
            affichees.push(self.affichee(session, grille).await?);
        }
        Ok(affichees)
    }

    async fn annee_ouverte(
        &self,
        session: &Session,
        annee_id: Id<AnneeScolaire>,
    ) -> Resultat<AnneeScolaire> {
        let contexte = self.noyau.contexte(session.etablissement_id, Some(session));
        let mut cnx = self.connexion().await?;
        let annee = annee_scolaire::lire(&mut cnx, annee_id, &contexte).await?;
        if !matches!(annee.statut, StatutAnnee::Preparation | StatutAnnee::Active) {
            return Err(regle("frais.annee_figee"));
        }
        Ok(annee)
    }

    /// Crée une grille en brouillon : vide avec les plans livrés, ou copie d'une autre grille.
    pub async fn creer(
        &self,
        session: &Session,
        demande: &DemandeCreationGrille,
    ) -> Resultat<GrilleAffichee> {
        let contexte = self.noyau.contexte(session.etablissement_id, Some(session));
        let annee = self.annee_ouverte(session, demande.annee_id).await?;
        let rentree = annee_de_rentree(&annee)?;
        let (version, contenu) = {
            let mut cnx = self.connexion().await?;
            let version = tarifs::prochaine_version(&mut cnx, annee.id, &contexte).await?;
            let contenu = match demande.copie_de {
                None => {
                    let jour = parametre::entier(
                        &mut cnx,
                        PARAMETRE_JOUR_ECHEANCE,
                        i64::from(JOUR_ECHEANCE_DEFAUT),
                        &contexte,
                    )
                    .await?;
                    ContenuGrille {
                        lignes: Vec::new(),
                        plans: plans_livres(
                            rentree,
                            u8::try_from(jour).unwrap_or(JOUR_ECHEANCE_DEFAUT),
                            nouvel_id,
                        ),
                    }
                }
                Some(source_id) => {
                    let source = tarifs::grille(&mut cnx, source_id, &contexte).await?;
                    let annee_source =
                        annee_scolaire::lire(&mut cnx, source.annee_id, &contexte).await?;
                    let ecart = rentree - annee_de_rentree(&annee_source)?;
                    let copie = tarifs::contenu(&mut cnx, source_id, &contexte).await?;
                    recopier(copie, ecart)?
                }
            };
            (version, contenu)
        };
        let libelle = if version == 1 {
            format!("Frais {}", annee.libelle)
        } else {
            format!("Frais {} (version {version})", annee.libelle)
        };
        let grille = Grille {
            id: nouvel_id(),
            etablissement_id: session.etablissement_id,
            annee_id: annee.id,
            libelle,
            version,
            etat: EtatGrille::Brouillon,
        };
        let mut unite = self
            .noyau
            .commencer(contexte.clone(), Durabilite::Critique)
            .await?;
        tarifs::inserer_grille(unite.connexion(), &grille, &contexte).await?;
        tarifs::remplacer_contenu(unite.connexion(), grille.id, &contenu, &contexte).await?;
        unite.auditer(
            "grille_tarifaire",
            &grille.id.vers_chaine(),
            Operation::Creation,
            None,
            Some(serde_json::json!({ "grille": grille, "contenu": contenu, "copie_de": demande.copie_de })),
            "frais.creer",
        );
        self.noyau.terminer(unite).await?;
        Ok(GrilleAffichee { grille, contenu })
    }

    /// Remplace le contenu d'un brouillon, après vérification des lignes et des plans.
    pub async fn modifier(
        &self,
        session: &Session,
        demande: &DemandeModificationGrille,
    ) -> Resultat<GrilleAffichee> {
        let contexte = self.noyau.contexte(session.etablissement_id, Some(session));
        let libelle = libelle_valide(&demande.libelle, "frais.libelle_invalide").map_err(regle)?;
        let contenu = ContenuGrille {
            lignes: demande
                .lignes
                .iter()
                .map(|l| LigneTarifaire {
                    id: nouvel_id(),
                    niveau_id: l.niveau_id,
                    serie_id: l.serie_id,
                    type_frais: l.type_frais,
                    libelle: l.libelle.trim().to_owned(),
                    montant: l.montant,
                    obligatoire: l.obligatoire,
                    exigibilite: l.exigibilite,
                })
                .collect(),
            plans: demande
                .plans
                .iter()
                .map(|p| PlanPaiement {
                    id: nouvel_id(),
                    libelle: p.libelle.trim().to_owned(),
                    par_defaut: p.par_defaut,
                    tranches: p
                        .tranches
                        .iter()
                        .map(|t| Tranche {
                            libelle: t.libelle.trim().to_owned(),
                            ..t.clone()
                        })
                        .collect(),
                })
                .collect(),
        };
        verifier_lignes(&contenu.lignes).map_err(regle)?;
        verifier_plans(&contenu.plans).map_err(regle)?;
        let (avant, contenu_avant) = {
            let mut cnx = self.connexion().await?;
            let grille = tarifs::grille(&mut cnx, demande.id, &contexte).await?;
            if grille.etat != EtatGrille::Brouillon {
                return Err(regle("frais.grille_figee"));
            }
            let niveaux = academique::niveaux(&mut cnx, &contexte).await?;
            let series = academique::series(&mut cnx, &contexte).await?;
            for ligne in &contenu.lignes {
                if !niveaux.iter().any(|n| n.id == ligne.niveau_id) {
                    return Err(regle("frais.niveau_inconnu"));
                }
                if let Some(serie_id) = ligne.serie_id
                    && !series
                        .iter()
                        .any(|s| s.id == serie_id && s.niveau_id == ligne.niveau_id)
                {
                    return Err(regle("frais.serie_hors_niveau"));
                }
            }
            let contenu_avant = tarifs::contenu(&mut cnx, grille.id, &contexte).await?;
            (grille, contenu_avant)
        };
        self.annee_ouverte(session, avant.annee_id).await?;
        let grille = Grille {
            libelle,
            ..avant.clone()
        };
        let mut unite = self
            .noyau
            .commencer(contexte.clone(), Durabilite::Critique)
            .await?;
        tarifs::ecrire_grille(unite.connexion(), &grille, &contexte).await?;
        tarifs::remplacer_contenu(unite.connexion(), grille.id, &contenu, &contexte).await?;
        unite.auditer(
            "grille_tarifaire",
            &grille.id.vers_chaine(),
            Operation::Modification,
            Some(serde_json::json!({ "grille": avant, "contenu": contenu_avant })),
            Some(serde_json::json!({ "grille": grille, "contenu": contenu })),
            "frais.modifier",
        );
        self.noyau.terminer(unite).await?;
        Ok(GrilleAffichee { grille, contenu })
    }

    /// Met un brouillon en vigueur ; la grille en vigueur de l'année, s'il y en a une, est
    /// remplacée. Les niveaux qui ont des classes et aucune scolarité sont signalés.
    pub async fn mettre_en_vigueur(
        &self,
        session: &Session,
        id: Id<Grille>,
    ) -> Resultat<MiseEnVigueur> {
        let contexte = self.noyau.contexte(session.etablissement_id, Some(session));
        let (grille, contenu, precedente, niveaux_sans_scolarite) = {
            let mut cnx = self.connexion().await?;
            let grille = tarifs::grille(&mut cnx, id, &contexte).await?;
            let etat = grille.etat.transiter(EtatGrille::EnVigueur)?;
            let contenu = tarifs::contenu(&mut cnx, id, &contexte).await?;
            if contenu.lignes.is_empty() {
                return Err(regle("frais.grille_vide"));
            }
            verifier_lignes(&contenu.lignes).map_err(regle)?;
            verifier_plans(&contenu.plans).map_err(regle)?;
            let precedente =
                tarifs::grille_en_vigueur(&mut cnx, grille.annee_id, &contexte).await?;
            let niveaux = academique::niveaux(&mut cnx, &contexte).await?;
            let classes = academique::classes(&mut cnx, grille.annee_id, &contexte).await?;
            let sans: Vec<String> = niveaux
                .iter()
                .filter(|n| n.actif && classes.iter().any(|c| c.niveau_id == n.id))
                .filter(|n| {
                    !contenu
                        .lignes
                        .iter()
                        .any(|l| l.niveau_id == n.id && l.type_frais == TypeFrais::Scolarite)
                })
                .map(|n| n.libelle.clone())
                .collect();
            (Grille { etat, ..grille }, contenu, precedente, sans)
        };
        self.annee_ouverte(session, grille.annee_id).await?;
        let mut unite = self
            .noyau
            .commencer(contexte.clone(), Durabilite::Critique)
            .await?;
        if let Some(ancienne) = &precedente {
            let remplacee = Grille {
                etat: ancienne.etat.transiter(EtatGrille::Remplacee)?,
                ..ancienne.clone()
            };
            tarifs::ecrire_grille(unite.connexion(), &remplacee, &contexte).await?;
            unite.auditer(
                "grille_tarifaire",
                &remplacee.id.vers_chaine(),
                Operation::Transition,
                Some(serde_json::to_value(ancienne)?),
                Some(serde_json::to_value(&remplacee)?),
                "frais.mettre_en_vigueur",
            );
        }
        tarifs::ecrire_grille(unite.connexion(), &grille, &contexte).await?;
        let id_texte = grille.id.vers_chaine();
        unite.auditer(
            "grille_tarifaire",
            &id_texte,
            Operation::Transition,
            Some(serde_json::json!({ "etat": EtatGrille::Brouillon })),
            Some(serde_json::to_value(&grille)?),
            "frais.mettre_en_vigueur",
        );
        unite.emettre(
            TypeEvenement::GrilleActivee,
            "grille_tarifaire",
            &id_texte,
            serde_json::json!({
                "annee_id": grille.annee_id,
                "version": grille.version,
                "remplace": precedente.map(|g| g.id),
            }),
        );
        self.noyau.terminer(unite).await?;
        Ok(MiseEnVigueur {
            grille: GrilleAffichee { grille, contenu },
            niveaux_sans_scolarite,
        })
    }
}

/// Copie d'un contenu : nouveaux identifiants, échéances décalées de l'écart d'années.
fn recopier(contenu: ContenuGrille, ecart: i32) -> Resultat<ContenuGrille> {
    let lignes = contenu
        .lignes
        .into_iter()
        .map(|l| LigneTarifaire {
            id: nouvel_id(),
            ..l
        })
        .collect();
    let plans = contenu
        .plans
        .into_iter()
        .map(|p| {
            let tranches = p
                .tranches
                .into_iter()
                .map(|t| {
                    Ok(Tranche {
                        echeance: decaler(t.echeance, ecart)
                            .ok_or(regle("frais.annee_invalide"))?,
                        ..t
                    })
                })
                .collect::<Resultat<Vec<_>>>()?;
            Ok(PlanPaiement {
                id: nouvel_id(),
                tranches,
                ..p
            })
        })
        .collect::<Resultat<Vec<_>>>()?;
    Ok(ContenuGrille { lignes, plans })
}
