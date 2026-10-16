//! Niveaux, séries et classes (SPEC_PHASE_2 §2, §3) ; filières de l'enseignement technique, que
//! l'école crée (SPEC_PHASE_6 §9).

use gesco_core::commun::Id;
use gesco_core::core::{AnneeScolaire, Etablissement, Operation, StatutAnnee, TypeEvenement};
use gesco_core::scolarite::academique::{
    NIVEAUX_LIVRES, code_filiere, libelle_valide, verifier_rattachement,
};
use gesco_core::scolarite::{Classe, Cycle, Niveau, Serie, StatutInscription};
use gesco_db::depots::{academique, annee_scolaire, inscriptions};
use gesco_db::unite::Durabilite;
use gesco_db::{UniteDeTravail, nouvel_id};
use serde::{Deserialize, Serialize};

use crate::effets::Effet;
use crate::erreur::{ErreurService, Resultat};
use crate::noyau::Noyau;
use crate::session::Session;

fn regle(cle: &'static str) -> ErreurService {
    ErreurService::Regle { cle }
}

/// Niveaux et séries de l'établissement.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
pub struct Referentiel {
    /// Niveaux, dans l'ordre d'affichage.
    pub niveaux: Vec<Niveau>,
    /// Séries du lycée et filières de l'enseignement technique.
    pub series: Vec<Serie>,
}

/// Filière créée par l'école sur des années de l'enseignement technique : une série par année.
#[derive(Debug, Clone, Deserialize, specta::Type)]
pub struct DemandeFiliere {
    /// Code court : `ELEC`, `MECA-AUTO`.
    pub code: String,
    /// Libellé : « Électricité ».
    pub libelle: String,
    /// Années où elle s'enseigne.
    pub niveaux: Vec<Id<Niveau>>,
}

/// Classe affichée : libellés du niveau et de la série.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
pub struct FicheClasse {
    /// Classe.
    pub classe: Classe,
    /// Libellé du niveau.
    pub niveau: String,
    /// Code de la série, s'il y en a une.
    pub serie: Option<String>,
    /// Élèves inscrits.
    pub effectif: i64,
}

/// Pose dans l'unité de travail les niveaux et séries livrés qui manquent à l'établissement,
/// reconnus par leur code : un établissement neuf les reçoit tous, une base installée ce qui lui
/// manque. Un niveau posé est actif si son cycle est enseigné. Rien d'existant n'est modifié.
/// Rend le nombre de lignes ajoutées.
pub async fn poser_referentiel(
    unite: &mut UniteDeTravail<Effet>,
    etablissement_id: Id<Etablissement>,
    cycles: &[Cycle],
    origine: &str,
) -> Resultat<usize> {
    let contexte = unite.contexte().clone();
    let existants = academique::niveaux(unite.connexion(), &contexte).await?;
    let series_existantes = academique::series(unite.connexion(), &contexte).await?;
    let mut ajoutes = 0;
    for (rang, livre) in (1_i64..).zip(NIVEAUX_LIVRES) {
        let niveau = match existants.iter().find(|n| n.code == livre.code) {
            Some(n) => n.clone(),
            None => {
                let n = Niveau {
                    id: nouvel_id(),
                    etablissement_id,
                    code: livre.code.to_owned(),
                    libelle: livre.libelle.to_owned(),
                    cycle: livre.cycle,
                    ordre: rang,
                    mode_evaluation: livre.mode_evaluation,
                    diplome_terminal: livre.diplome_terminal,
                    actif: cycles.contains(&livre.cycle),
                };
                academique::inserer_niveau(unite.connexion(), &n, &contexte).await?;
                unite.auditer(
                    "niveau",
                    &n.id.vers_chaine(),
                    Operation::Creation,
                    None,
                    Some(serde_json::to_value(&n)?),
                    origine,
                );
                ajoutes += 1;
                n
            }
        };
        for (code, libelle) in livre.series {
            if series_existantes
                .iter()
                .any(|s| s.niveau_id == niveau.id && s.code == *code)
            {
                continue;
            }
            let s = Serie {
                id: nouvel_id(),
                etablissement_id,
                niveau_id: niveau.id,
                code: (*code).to_owned(),
                libelle: (*libelle).to_owned(),
                actif: true,
            };
            academique::inserer_serie(unite.connexion(), &s, &contexte).await?;
            unite.auditer(
                "serie",
                &s.id.vers_chaine(),
                Operation::Creation,
                None,
                Some(serde_json::to_value(&s)?),
                origine,
            );
            ajoutes += 1;
        }
    }
    // Les matières livrées suivent les niveaux (SPEC_PHASE_4 §2.1).
    ajoutes += crate::pedagogie::poser_matieres(unite, etablissement_id, origine).await?;
    // Les catégories et fonctions du personnel aussi (SPEC_PHASE_6 §2).
    ajoutes += crate::personnel::poser_personnel(unite, etablissement_id, origine).await?;
    // Et les éléments de paie livrés (SPEC_PHASE_7 §3).
    ajoutes += crate::paie::poser_elements(unite, etablissement_id, origine).await?;
    // Et les catégories et la caisse de la trésorerie (SPEC_PHASE_8 §2).
    ajoutes += crate::tresorerie::poser_tresorerie(unite, etablissement_id, origine).await?;
    Ok(ajoutes)
}

/// Niveau d'une classe tel qu'il s'imprime sur ses documents : dans l'enseignement technique, avec
/// la filière, « CAP 1re année, Électricité » (SPEC_PHASE_6 §11) ; ailleurs, son libellé.
pub(crate) async fn niveau_imprime(
    cnx: &mut gesco_db::Connexion,
    niveau: &Niveau,
    classe: &Classe,
    contexte: &gesco_db::Contexte,
) -> Resultat<String> {
    match classe.serie_id {
        Some(id) if niveau.cycle.a_filieres() => {
            let filiere = academique::serie(cnx, id, contexte).await?;
            Ok(format!("{}, {}", niveau.libelle, filiere.libelle))
        }
        _ => Ok(niveau.libelle.clone()),
    }
}

/// Vérifie qu'au moins un cycle est enseigné : une école sans niveau n'inscrit personne.
pub fn verifier_cycles(cycles: &[Cycle]) -> Resultat<()> {
    if cycles.is_empty() {
        return Err(regle("referentiel.aucun_cycle"));
    }
    Ok(())
}

/// Service du référentiel et des classes.
pub struct ServiceReferentiel<'n> {
    noyau: &'n Noyau,
}

impl<'n> ServiceReferentiel<'n> {
    /// Service adossé au noyau.
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

    /// Niveaux et séries de l'établissement.
    pub async fn referentiel(&self, session: &Session) -> Resultat<Referentiel> {
        let contexte = self.noyau.contexte(session.etablissement_id, Some(session));
        let mut cnx = self.connexion().await?;
        Ok(Referentiel {
            niveaux: academique::niveaux(&mut cnx, &contexte).await?,
            series: academique::series(&mut cnx, &contexte).await?,
        })
    }

    /// Active les niveaux des cycles que l'école enseigne et désactive les autres : une école
    /// n'a parfois que le 1er cycle, parfois le fondamental, parfois aussi le lycée.
    pub async fn definir_cycles(
        &self,
        session: &Session,
        cycles: &[Cycle],
    ) -> Resultat<Referentiel> {
        verifier_cycles(cycles)?;
        let contexte = self.noyau.contexte(session.etablissement_id, Some(session));
        let niveaux = {
            let mut cnx = self.connexion().await?;
            academique::niveaux(&mut cnx, &contexte).await?
        };
        let mut unite = self
            .noyau
            .commencer(contexte.clone(), Durabilite::Critique)
            .await?;
        let mut changes = 0;
        for avant in niveaux {
            let actif = cycles.contains(&avant.cycle);
            if avant.actif == actif {
                continue;
            }
            let apres = Niveau {
                actif,
                ..avant.clone()
            };
            academique::modifier_niveau(unite.connexion(), &apres, &contexte).await?;
            unite.auditer(
                "niveau",
                &apres.id.vers_chaine(),
                Operation::Modification,
                Some(serde_json::to_value(&avant)?),
                Some(serde_json::to_value(&apres)?),
                "referentiel.definir_cycles",
            );
            changes += 1;
        }
        if changes == 0 {
            unite.annuler().await?;
        } else {
            self.noyau.terminer(unite).await?;
        }
        self.referentiel(session).await
    }

    /// Renomme un niveau ou change son état ; code, cycle et ordre restent nationaux.
    pub async fn modifier_niveau(
        &self,
        session: &Session,
        id: Id<Niveau>,
        libelle: &str,
        actif: bool,
    ) -> Resultat<Niveau> {
        let contexte = self.noyau.contexte(session.etablissement_id, Some(session));
        let avant = {
            let mut cnx = self.connexion().await?;
            academique::niveau(&mut cnx, id, &contexte).await?
        };
        let apres = Niveau {
            libelle: libelle_valide(libelle, "niveau.libelle_invalide").map_err(regle)?,
            actif,
            ..avant.clone()
        };
        let mut unite = self
            .noyau
            .commencer(contexte.clone(), Durabilite::Critique)
            .await?;
        academique::modifier_niveau(unite.connexion(), &apres, &contexte).await?;
        unite.auditer(
            "niveau",
            &id.vers_chaine(),
            Operation::Modification,
            Some(serde_json::to_value(&avant)?),
            Some(serde_json::to_value(&apres)?),
            "referentiel.modifier_niveau",
        );
        self.noyau.terminer(unite).await?;
        Ok(apres)
    }

    /// Renomme une série ou change son état.
    pub async fn modifier_serie(
        &self,
        session: &Session,
        id: Id<Serie>,
        libelle: &str,
        actif: bool,
    ) -> Resultat<Serie> {
        let contexte = self.noyau.contexte(session.etablissement_id, Some(session));
        let avant = {
            let mut cnx = self.connexion().await?;
            academique::serie(&mut cnx, id, &contexte).await?
        };
        let apres = Serie {
            libelle: libelle_valide(libelle, "serie.libelle_invalide").map_err(regle)?,
            actif,
            ..avant.clone()
        };
        let mut unite = self
            .noyau
            .commencer(contexte.clone(), Durabilite::Critique)
            .await?;
        academique::modifier_serie(unite.connexion(), &apres, &contexte).await?;
        unite.auditer(
            "serie",
            &id.vers_chaine(),
            Operation::Modification,
            Some(serde_json::to_value(&avant)?),
            Some(serde_json::to_value(&apres)?),
            "referentiel.modifier_serie",
        );
        self.noyau.terminer(unite).await?;
        Ok(apres)
    }

    /// Crée une filière sur des années du cycle technique : une série par année, active. Les
    /// années ouvertes reçoivent son programme par défaut, à compléter des matières de pratique.
    pub async fn creer_filiere(
        &self,
        session: &Session,
        demande: &DemandeFiliere,
    ) -> Resultat<Referentiel> {
        let code = code_filiere(&demande.code).map_err(regle)?;
        let libelle = libelle_valide(&demande.libelle, "serie.libelle_invalide").map_err(regle)?;
        if demande.niveaux.is_empty() {
            return Err(regle("filiere.aucun_niveau"));
        }
        let contexte = self.noyau.contexte(session.etablissement_id, Some(session));
        let mut unite = self
            .noyau
            .commencer(contexte.clone(), Durabilite::Critique)
            .await?;
        let niveaux = academique::niveaux(unite.connexion(), &contexte).await?;
        let series = academique::series(unite.connexion(), &contexte).await?;
        for (i, niveau_id) in demande.niveaux.iter().enumerate() {
            if demande.niveaux.iter().take(i).any(|n| n == niveau_id) {
                continue;
            }
            let niveau = niveaux
                .iter()
                .find(|n| n.id == *niveau_id)
                .ok_or(regle("filiere.niveau_inconnu"))?;
            if !niveau.cycle.a_filieres() {
                return Err(regle("filiere.hors_technique"));
            }
            if series
                .iter()
                .any(|s| s.niveau_id == niveau.id && s.code == code)
            {
                return Err(regle("filiere.code_pris"));
            }
            let s = Serie {
                id: nouvel_id(),
                etablissement_id: session.etablissement_id,
                niveau_id: niveau.id,
                code: code.clone(),
                libelle: libelle.clone(),
                actif: true,
            };
            academique::inserer_serie(unite.connexion(), &s, &contexte).await?;
            unite.auditer(
                "serie",
                &s.id.vers_chaine(),
                Operation::Creation,
                None,
                Some(serde_json::to_value(&s)?),
                "referentiel.creer_filiere",
            );
        }
        for annee in annee_scolaire::lister(unite.connexion(), &contexte)
            .await?
            .iter()
            .filter(|a| matches!(a.statut, StatutAnnee::Preparation | StatutAnnee::Active))
        {
            crate::pedagogie::poser_annee(&mut unite, annee, "referentiel.creer_filiere").await?;
        }
        self.noyau.terminer(unite).await?;
        self.referentiel(session).await
    }

    /// Classes d'une année, avec niveau, série et effectif.
    pub async fn classes(
        &self,
        session: &Session,
        annee_id: Id<AnneeScolaire>,
    ) -> Resultat<Vec<FicheClasse>> {
        let contexte = self.noyau.contexte(session.etablissement_id, Some(session));
        let mut cnx = self.connexion().await?;
        let niveaux = academique::niveaux(&mut cnx, &contexte).await?;
        let series = academique::series(&mut cnx, &contexte).await?;
        let validees: Vec<_> = inscriptions::de_l_annee(&mut cnx, annee_id, &contexte)
            .await?
            .into_iter()
            .filter(|i| i.statut == StatutInscription::Validee)
            .collect();
        let mut fiches: Vec<FicheClasse> = academique::classes(&mut cnx, annee_id, &contexte)
            .await?
            .into_iter()
            .map(|classe| FicheClasse {
                niveau: niveaux
                    .iter()
                    .find(|n| n.id == classe.niveau_id)
                    .map(|n| n.libelle.clone())
                    .unwrap_or_default(),
                serie: classe
                    .serie_id
                    .and_then(|s| series.iter().find(|x| x.id == s))
                    .map(|s| s.code.clone()),
                effectif: i64::try_from(
                    validees.iter().filter(|i| i.classe_id == classe.id).count(),
                )
                .unwrap_or(i64::MAX),
                classe,
            })
            .collect();
        let ordre = |f: &FicheClasse| {
            niveaux
                .iter()
                .find(|n| n.id == f.classe.niveau_id)
                .map_or(i64::MAX, |n| n.ordre)
        };
        fiches.sort_by(|a, b| {
            ordre(a)
                .cmp(&ordre(b))
                .then_with(|| a.classe.libelle.cmp(&b.classe.libelle))
        });
        Ok(fiches)
    }

    /// Crée une classe dans une année en préparation ou active.
    pub async fn creer_classe(
        &self,
        session: &Session,
        annee_id: Id<AnneeScolaire>,
        niveau_id: Id<Niveau>,
        serie_id: Option<Id<Serie>>,
        libelle: &str,
    ) -> Resultat<Classe> {
        let contexte = self.noyau.contexte(session.etablissement_id, Some(session));
        let libelle = libelle_valide(libelle, "classe.libelle_invalide").map_err(regle)?;
        {
            let mut cnx = self.connexion().await?;
            let annee = annee_scolaire::lire(&mut cnx, annee_id, &contexte).await?;
            if !matches!(annee.statut, StatutAnnee::Preparation | StatutAnnee::Active) {
                return Err(regle("classe.annee_figee"));
            }
            let niveau = academique::niveau(&mut cnx, niveau_id, &contexte).await?;
            let serie = match serie_id {
                Some(s) => Some(academique::serie(&mut cnx, s, &contexte).await?),
                None => None,
            };
            let series_du_niveau = academique::series(&mut cnx, &contexte)
                .await?
                .iter()
                .filter(|s| s.niveau_id == niveau_id)
                .count();
            verifier_rattachement(&niveau, serie.as_ref(), series_du_niveau).map_err(regle)?;
            if academique::libelle_pris(&mut cnx, annee_id, &libelle, None, &contexte).await? {
                return Err(regle("classe.libelle_pris"));
            }
        }
        let classe = Classe {
            id: nouvel_id(),
            etablissement_id: session.etablissement_id,
            annee_id,
            niveau_id,
            serie_id,
            libelle,
        };
        let mut unite = self
            .noyau
            .commencer(contexte.clone(), Durabilite::Critique)
            .await?;
        academique::inserer_classe(unite.connexion(), &classe, &contexte).await?;
        let id = classe.id.vers_chaine();
        unite.auditer(
            "classe",
            &id,
            Operation::Creation,
            None,
            Some(serde_json::to_value(&classe)?),
            "referentiel.creer_classe",
        );
        unite.emettre(
            TypeEvenement::ClasseCreee,
            "classe",
            &id,
            serde_json::json!({
                "annee_id": classe.annee_id,
                "niveau_id": classe.niveau_id,
                "serie_id": classe.serie_id,
                "libelle": classe.libelle,
            }),
        );
        self.noyau.terminer(unite).await?;
        Ok(classe)
    }

    /// Renomme une classe ; le libellé reste unique dans l'année.
    pub async fn renommer_classe(
        &self,
        session: &Session,
        id: Id<Classe>,
        libelle: &str,
    ) -> Resultat<Classe> {
        let contexte = self.noyau.contexte(session.etablissement_id, Some(session));
        let libelle = libelle_valide(libelle, "classe.libelle_invalide").map_err(regle)?;
        let avant = {
            let mut cnx = self.connexion().await?;
            let avant = academique::classe(&mut cnx, id, &contexte).await?;
            if academique::libelle_pris(&mut cnx, avant.annee_id, &libelle, Some(id), &contexte)
                .await?
            {
                return Err(regle("classe.libelle_pris"));
            }
            avant
        };
        let apres = Classe {
            libelle,
            ..avant.clone()
        };
        let mut unite = self
            .noyau
            .commencer(contexte.clone(), Durabilite::Critique)
            .await?;
        academique::renommer_classe(unite.connexion(), id, &apres.libelle, &contexte).await?;
        unite.auditer(
            "classe",
            &id.vers_chaine(),
            Operation::Modification,
            Some(serde_json::to_value(&avant)?),
            Some(serde_json::to_value(&apres)?),
            "referentiel.renommer_classe",
        );
        self.noyau.terminer(unite).await?;
        Ok(apres)
    }

    /// Supprime logiquement une classe qui n'a jamais reçu d'inscription.
    pub async fn supprimer_classe(&self, session: &Session, id: Id<Classe>) -> Resultat<()> {
        let contexte = self.noyau.contexte(session.etablissement_id, Some(session));
        let avant = {
            let mut cnx = self.connexion().await?;
            if inscriptions::compter_dans_classe(&mut cnx, id, &contexte).await? > 0 {
                return Err(regle("classe.non_vide"));
            }
            academique::classe(&mut cnx, id, &contexte).await?
        };
        let mut unite = self
            .noyau
            .commencer(contexte.clone(), Durabilite::Critique)
            .await?;
        academique::supprimer_classe(unite.connexion(), id, &contexte).await?;
        unite.auditer(
            "classe",
            &id.vers_chaine(),
            Operation::Suppression,
            Some(serde_json::to_value(&avant)?),
            None,
            "referentiel.supprimer_classe",
        );
        self.noyau.terminer(unite).await?;
        Ok(())
    }
}
