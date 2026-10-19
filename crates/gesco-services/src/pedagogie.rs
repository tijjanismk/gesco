//! Référentiel pédagogique (SPEC_PHASE_4 §2) : matières, programme de chaque niveau, périodes de
//! notes, règles d'évaluation. Les défauts se posent à l'installation et à la création d'une
//! année, et pour les années existantes à l'ouverture de la base ; l'école les corrige ensuite
//! (`academic_year:manage`).

use gesco_core::commun::{DateCivile, Id};
use gesco_core::core::{AnneeScolaire, Etablissement, Operation, StatutAnnee, TypeEvenement};
use gesco_core::pedagogie::matiere::{
    MATIERES_LIVREES, MatiereLivree, au_programme_par_defaut, programme_premier_cycle,
    verifier_matiere,
};
use gesco_core::pedagogie::periode::{
    calendrier_par_defaut, decoupage_par_defaut, verifier_calendrier, verifier_decoupage,
};
use gesco_core::pedagogie::programme::verifier_programme;
use gesco_core::pedagogie::regle::verifier_regle;
use gesco_core::pedagogie::{
    ContenuRegle, GroupeMatiere, LigneProgramme, Matiere, Periode, RegleEvaluation,
    SaisieProgramme, TypePeriode,
};
use gesco_core::scolarite::personne::millesime;
use gesco_core::scolarite::{Cycle, Niveau, Serie};
use gesco_db::depots::{academique, annee_scolaire, pedagogie};
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

/// Cycles notés, dans l'ordre.
const CYCLES_NOTES: [Cycle; 4] = [
    Cycle::PremierCycle,
    Cycle::SecondCycle,
    Cycle::Lycee,
    Cycle::Technique,
];

/// Pose les matières livrées qui manquent à l'établissement, reconnues par leur code. Rend le
/// nombre de matières ajoutées.
pub(crate) async fn poser_matieres(
    unite: &mut UniteDeTravail<Effet>,
    etablissement_id: Id<Etablissement>,
    origine: &str,
) -> Resultat<usize> {
    let contexte = unite.contexte().clone();
    let existantes = pedagogie::matieres(unite.connexion(), &contexte).await?;
    let mut ajoutees = 0;
    for (ordre, livree) in (1_i64..).zip(MATIERES_LIVREES) {
        if existantes.iter().any(|m| m.code == livree.code) {
            continue;
        }
        let m = Matiere {
            id: nouvel_id(),
            etablissement_id,
            code: livree.code.to_owned(),
            libelle: livree.libelle.to_owned(),
            libelle_court: livree.libelle_court.to_owned(),
            groupe: livree.groupe,
            ordre,
            active: true,
        };
        pedagogie::inserer_matiere(unite.connexion(), &m, &contexte).await?;
        unite.auditer(
            "matiere",
            &m.id.vers_chaine(),
            Operation::Creation,
            None,
            Some(serde_json::to_value(&m)?),
            origine,
        );
        ajoutees += 1;
    }
    Ok(ajoutees)
}

/// Première année civile d'une année scolaire : le millésime de son libellé, sinon l'année de
/// son début.
fn rentree(annee: &AnneeScolaire) -> i32 {
    millesime(&annee.libelle)
        .and_then(|m| m.parse().ok())
        .unwrap_or_else(|| i32::try_from(annee.debut.civil_utc().annee).unwrap_or(2026))
}

pub(crate) fn series_de(niveau: &Niveau, series: &[Serie]) -> Vec<Option<Serie>> {
    let du_niveau: Vec<Option<Serie>> = series
        .iter()
        .filter(|s| s.niveau_id == niveau.id)
        .cloned()
        .map(Some)
        .collect();
    if du_niveau.is_empty() {
        vec![None]
    } else {
        du_niveau
    }
}

/// Pose, pour une année, ce qui lui manque : périodes par défaut de chaque niveau noté,
/// programme par défaut de chaque niveau et série, règle livrée de chaque cycle. Rien
/// d'existant n'est modifié. Rend le nombre de lignes ajoutées.
pub(crate) async fn poser_annee(
    unite: &mut UniteDeTravail<Effet>,
    annee: &AnneeScolaire,
    origine: &str,
) -> Resultat<usize> {
    let contexte = unite.contexte().clone();
    let niveaux = academique::niveaux(unite.connexion(), &contexte).await?;
    let series = academique::series(unite.connexion(), &contexte).await?;
    let matieres = pedagogie::matieres(unite.connexion(), &contexte).await?;
    let periodes = pedagogie::periodes(unite.connexion(), annee.id, &contexte).await?;
    let programme = pedagogie::programme(unite.connexion(), annee.id, &contexte).await?;
    let regles = pedagogie::regles_en_vigueur(unite.connexion(), annee.id, &contexte).await?;
    let mut ajoutes = 0;
    for niveau in niveaux.iter().filter(|n| n.cycle != Cycle::Jardin) {
        let (nature, nombre) = decoupage_par_defaut(niveau.cycle);
        if !periodes.iter().any(|p| p.niveau_id == niveau.id)
            && let Some((nature, calendrier)) =
                calendrier_par_defaut(niveau.cycle, rentree(annee), nature, nombre)
        {
            {
                for (rang, (debut, fin)) in (1_i64..).zip(calendrier) {
                    let p = Periode {
                        id: nouvel_id(),
                        etablissement_id: annee.etablissement_id,
                        annee_id: annee.id,
                        niveau_id: niveau.id,
                        rang,
                        type_periode: nature,
                        debut,
                        fin,
                        ponderation: 1,
                        verrouillee: false,
                    };
                    pedagogie::inserer_periode(unite.connexion(), &p, &contexte).await?;
                    ajoutes += 1;
                }
            }
        }
        for serie in series_de(niveau, &series) {
            let serie_id = serie.as_ref().map(|s| s.id);
            if programme
                .iter()
                .any(|l| l.niveau_id == niveau.id && l.serie_id == serie_id)
            {
                continue;
            }
            let code_serie = serie.as_ref().map(|s| s.code.as_str());
            // Au 1er cycle, l'ordre de l'emploi du temps ; ensuite celui du référentiel.
            let livrees: Vec<&MatiereLivree> =
                match niveau.code.parse().ok().and_then(programme_premier_cycle) {
                    Some(codes) => codes
                        .iter()
                        .filter_map(|c| MATIERES_LIVREES.iter().find(|l| l.code == *c))
                        .collect(),
                    None => MATIERES_LIVREES
                        .iter()
                        .filter(|l| au_programme_par_defaut(l.code, &niveau.code, code_serie))
                        .collect(),
                };
            let mut ordre = 0;
            for livree in livrees {
                let Some(matiere) = matieres.iter().find(|m| m.code == livree.code) else {
                    continue;
                };
                ordre += 1;
                let l = LigneProgramme {
                    id: nouvel_id(),
                    etablissement_id: annee.etablissement_id,
                    annee_id: annee.id,
                    niveau_id: niveau.id,
                    serie_id,
                    matiere_id: matiere.id,
                    coefficient: None,
                    minutes_hebdo: if niveau.cycle == Cycle::PremierCycle {
                        livree.minutes_premier_cycle
                    } else {
                        None
                    },
                    ordre,
                };
                pedagogie::inserer_ligne_programme(unite.connexion(), &l, &contexte).await?;
                ajoutes += 1;
            }
        }
    }
    for cycle in CYCLES_NOTES {
        if regles.iter().any(|r| r.cycle == cycle) {
            continue;
        }
        let Some(contenu) = ContenuRegle::par_defaut(cycle) else {
            continue;
        };
        let r = RegleEvaluation {
            id: nouvel_id(),
            etablissement_id: annee.etablissement_id,
            annee_id: annee.id,
            cycle,
            version: 1,
            contenu,
            le: contexte.maintenant,
        };
        pedagogie::inserer_regle(unite.connexion(), &r, &contexte).await?;
        ajoutes += 1;
    }
    if ajoutes > 0 {
        unite.auditer(
            "annee_scolaire",
            &annee.id.vers_chaine(),
            Operation::Modification,
            None,
            Some(serde_json::json!({ "referentiel_pedagogique_pose": ajoutes })),
            origine,
        );
    }
    Ok(ajoutes)
}

/// Période affichée avec son libellé.
#[derive(Debug, Clone, Serialize, specta::Type)]
pub struct PeriodeAffichee {
    /// Période.
    pub periode: Periode,
    /// « Octobre », « 1er trimestre ».
    pub libelle: String,
}

/// Référentiel pédagogique d'une année.
#[derive(Debug, Clone, Serialize, specta::Type)]
pub struct Pedagogie {
    /// Matières de l'établissement.
    pub matieres: Vec<Matiere>,
    /// Niveaux de l'établissement.
    pub niveaux: Vec<Niveau>,
    /// Séries du lycée.
    pub series: Vec<Serie>,
    /// Programme de l'année.
    pub programme: Vec<LigneProgramme>,
    /// Périodes de l'année.
    pub periodes: Vec<PeriodeAffichee>,
    /// Règle en vigueur de chaque cycle.
    pub regles: Vec<RegleEvaluation>,
}

/// Matière saisie par l'école ; sans `id`, une création.
#[derive(Debug, Clone, Deserialize, specta::Type)]
pub struct DemandeMatiere {
    /// Matière modifiée ; absente pour une création.
    pub id: Option<Id<Matiere>>,
    /// Code.
    pub code: String,
    /// Libellé.
    pub libelle: String,
    /// Libellé court.
    pub libelle_court: String,
    /// Groupe.
    pub groupe: GroupeMatiere,
    /// Faux pour la désactiver.
    pub active: bool,
}

/// Programme d'un niveau (et d'une série) saisi par l'école.
#[derive(Debug, Clone, Deserialize, specta::Type)]
pub struct DemandeProgramme {
    /// Année.
    pub annee_id: Id<AnneeScolaire>,
    /// Niveau.
    pub niveau_id: Id<Niveau>,
    /// Série, au lycée.
    pub serie_id: Option<Id<Serie>>,
    /// Matières au programme, dans l'ordre du bulletin.
    pub lignes: Vec<SaisieProgramme>,
}

/// Découpage de l'année d'un niveau du 2e cycle, du lycée (T-26) ou de l'enseignement technique
/// (Q-71).
#[derive(Debug, Clone, Deserialize, specta::Type)]
pub struct DemandeNombrePeriodes {
    /// Année.
    pub annee_id: Id<AnneeScolaire>,
    /// Niveau.
    pub niveau_id: Id<Niveau>,
    /// Trimestres, ou semestres dans l'enseignement technique.
    pub nature: TypePeriode,
    /// Trois ou deux trimestres ; deux semestres.
    pub nombre: u8,
}

/// Période corrigée par l'école.
#[derive(Debug, Clone, Deserialize, specta::Type)]
pub struct SaisiePeriode {
    /// Période.
    pub id: Id<Periode>,
    /// Premier jour.
    pub debut: DateCivile,
    /// Dernier jour.
    pub fin: DateCivile,
    /// Poids dans la moyenne annuelle.
    pub ponderation: i64,
}

/// Calendrier d'un niveau corrigé par l'école.
#[derive(Debug, Clone, Deserialize, specta::Type)]
pub struct DemandeCalendrier {
    /// Année.
    pub annee_id: Id<AnneeScolaire>,
    /// Niveau.
    pub niveau_id: Id<Niveau>,
    /// Périodes, dans l'ordre.
    pub periodes: Vec<SaisiePeriode>,
}

/// Règle d'un cycle saisie par l'école.
#[derive(Debug, Clone, Deserialize, specta::Type)]
pub struct DemandeRegle {
    /// Année.
    pub annee_id: Id<AnneeScolaire>,
    /// Cycle.
    pub cycle: Cycle,
    /// Contenu.
    pub contenu: ContenuRegle,
}

/// Service du référentiel pédagogique.
pub struct ServicePedagogie<'n> {
    noyau: &'n Noyau,
}

impl<'n> ServicePedagogie<'n> {
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

    /// Référentiel pédagogique d'une année.
    pub async fn lire(
        &self,
        session: &Session,
        annee_id: Id<AnneeScolaire>,
    ) -> Resultat<Pedagogie> {
        let contexte = self.noyau.contexte(session.etablissement_id, Some(session));
        let mut cnx = self.connexion().await?;
        annee_scolaire::lire(&mut cnx, annee_id, &contexte).await?;
        Ok(Pedagogie {
            matieres: pedagogie::matieres(&mut cnx, &contexte).await?,
            niveaux: academique::niveaux(&mut cnx, &contexte).await?,
            series: academique::series(&mut cnx, &contexte).await?,
            programme: pedagogie::programme(&mut cnx, annee_id, &contexte).await?,
            periodes: pedagogie::periodes(&mut cnx, annee_id, &contexte)
                .await?
                .into_iter()
                .map(|p| PeriodeAffichee {
                    libelle: p.libelle(),
                    periode: p,
                })
                .collect(),
            regles: pedagogie::regles_en_vigueur(&mut cnx, annee_id, &contexte).await?,
        })
    }

    /// Année modifiable : ni clôturée ni archivée.
    async fn annee_ouverte(
        &self,
        session: &Session,
        annee_id: Id<AnneeScolaire>,
    ) -> Resultat<AnneeScolaire> {
        let contexte = self.noyau.contexte(session.etablissement_id, Some(session));
        let mut cnx = self.connexion().await?;
        let annee = annee_scolaire::lire(&mut cnx, annee_id, &contexte).await?;
        if matches!(annee.statut, StatutAnnee::Cloturee | StatutAnnee::Archivee) {
            return Err(regle("pedagogie.annee_close"));
        }
        Ok(annee)
    }

    /// Crée ou modifie une matière.
    pub async fn enregistrer_matiere(
        &self,
        session: &Session,
        demande: &DemandeMatiere,
    ) -> Resultat<Matiere> {
        verifier_matiere(&demande.code, &demande.libelle, &demande.libelle_court).map_err(regle)?;
        let contexte = self.noyau.contexte(session.etablissement_id, Some(session));
        let existantes = {
            let mut cnx = self.connexion().await?;
            pedagogie::matieres(&mut cnx, &contexte).await?
        };
        let code = demande.code.trim().to_owned();
        if existantes
            .iter()
            .any(|m| m.code == code && Some(m.id) != demande.id)
        {
            return Err(regle("matiere.code_pris"));
        }
        let avant = match demande.id {
            Some(id) => Some(
                existantes
                    .iter()
                    .find(|m| m.id == id)
                    .cloned()
                    .ok_or(regle("matiere.inconnue"))?,
            ),
            None => None,
        };
        let apres = Matiere {
            id: avant.as_ref().map_or_else(nouvel_id, |a| a.id),
            etablissement_id: session.etablissement_id,
            code,
            libelle: demande.libelle.trim().to_owned(),
            libelle_court: demande.libelle_court.trim().to_owned(),
            groupe: demande.groupe,
            ordre: avant.as_ref().map_or_else(
                || existantes.iter().map(|m| m.ordre).max().unwrap_or(0) + 1,
                |a| a.ordre,
            ),
            active: demande.active,
        };
        let mut unite = self
            .noyau
            .commencer(contexte.clone(), Durabilite::Critique)
            .await?;
        match &avant {
            Some(_) => pedagogie::ecrire_matiere(unite.connexion(), &apres, &contexte).await?,
            None => pedagogie::inserer_matiere(unite.connexion(), &apres, &contexte).await?,
        }
        let id = apres.id.vers_chaine();
        unite.auditer(
            "matiere",
            &id,
            if avant.is_some() {
                Operation::Modification
            } else {
                Operation::Creation
            },
            avant.as_ref().map(serde_json::to_value).transpose()?,
            Some(serde_json::to_value(&apres)?),
            "pedagogie.matiere",
        );
        unite.emettre(
            TypeEvenement::MatiereEnregistree,
            "matiere",
            &id,
            serde_json::json!({ "code": apres.code, "active": apres.active }),
        );
        self.noyau.terminer(unite).await?;
        Ok(apres)
    }

    /// Récrit le programme d'un niveau (et d'une série) pour une année.
    pub async fn enregistrer_programme(
        &self,
        session: &Session,
        demande: &DemandeProgramme,
    ) -> Resultat<Vec<LigneProgramme>> {
        let annee = self.annee_ouverte(session, demande.annee_id).await?;
        let contexte = self.noyau.contexte(session.etablissement_id, Some(session));
        let (niveau, series, matieres, avant) = {
            let mut cnx = self.connexion().await?;
            let niveau = academique::niveau(&mut cnx, demande.niveau_id, &contexte).await?;
            let series = academique::series(&mut cnx, &contexte).await?;
            let matieres = pedagogie::matieres(&mut cnx, &contexte).await?;
            let avant: Vec<LigneProgramme> = pedagogie::programme(&mut cnx, annee.id, &contexte)
                .await?
                .into_iter()
                .filter(|l| l.niveau_id == niveau.id && l.serie_id == demande.serie_id)
                .collect();
            (niveau, series, matieres, avant)
        };
        let attendues: Vec<Option<Id<Serie>>> = series_de(&niveau, &series)
            .iter()
            .map(|s| s.as_ref().map(|s| s.id))
            .collect();
        if !attendues.contains(&demande.serie_id) {
            return Err(regle("programme.serie_invalide"));
        }
        if demande
            .lignes
            .iter()
            .any(|l| !matieres.iter().any(|m| m.id == l.matiere_id))
        {
            return Err(regle("programme.matiere_inconnue"));
        }
        verifier_programme(niveau.cycle, &demande.lignes).map_err(regle)?;
        let apres: Vec<LigneProgramme> = (1_i64..)
            .zip(&demande.lignes)
            .map(|(ordre, l)| LigneProgramme {
                id: nouvel_id(),
                etablissement_id: session.etablissement_id,
                annee_id: annee.id,
                niveau_id: niveau.id,
                serie_id: demande.serie_id,
                matiere_id: l.matiere_id,
                coefficient: l.coefficient,
                minutes_hebdo: l.minutes_hebdo,
                ordre,
            })
            .collect();
        let mut unite = self
            .noyau
            .commencer(contexte.clone(), Durabilite::Critique)
            .await?;
        pedagogie::retirer_programme(
            unite.connexion(),
            annee.id,
            niveau.id,
            demande.serie_id,
            &contexte,
        )
        .await?;
        for l in &apres {
            pedagogie::inserer_ligne_programme(unite.connexion(), l, &contexte).await?;
        }
        let id = niveau.id.vers_chaine();
        unite.auditer(
            "programme_matiere",
            &id,
            Operation::Modification,
            Some(serde_json::to_value(&avant)?),
            Some(serde_json::to_value(&apres)?),
            "pedagogie.programme",
        );
        unite.emettre(
            TypeEvenement::ProgrammeEnregistre,
            "niveau",
            &id,
            serde_json::json!({ "annee_id": annee.id, "serie_id": demande.serie_id, "matieres": apres.len() }),
        );
        self.noyau.terminer(unite).await?;
        Ok(apres)
    }

    /// Reprend le programme de l'année précédente : chaque niveau et série qu'elle a décrits
    /// remplace celui de l'année.
    pub async fn reprendre_programme(
        &self,
        session: &Session,
        annee_id: Id<AnneeScolaire>,
    ) -> Resultat<usize> {
        let annee = self.annee_ouverte(session, annee_id).await?;
        let contexte = self.noyau.contexte(session.etablissement_id, Some(session));
        let source = {
            let mut cnx = self.connexion().await?;
            let precedente = annee_scolaire::lister(&mut cnx, &contexte)
                .await?
                .into_iter()
                .filter(|a| a.debut.avant(&annee.debut))
                .max_by_key(|a| a.debut.millis_epoque())
                .ok_or(regle("programme.rien_a_reprendre"))?;
            pedagogie::programme(&mut cnx, precedente.id, &contexte).await?
        };
        if source.is_empty() {
            return Err(regle("programme.rien_a_reprendre"));
        }
        let mut cles: Vec<(Id<Niveau>, Option<Id<Serie>>)> = Vec::new();
        for l in &source {
            if !cles.contains(&(l.niveau_id, l.serie_id)) {
                cles.push((l.niveau_id, l.serie_id));
            }
        }
        let mut unite = self
            .noyau
            .commencer(contexte.clone(), Durabilite::Critique)
            .await?;
        for (niveau_id, serie_id) in &cles {
            pedagogie::retirer_programme(
                unite.connexion(),
                annee.id,
                *niveau_id,
                *serie_id,
                &contexte,
            )
            .await?;
        }
        for l in &source {
            let copie = LigneProgramme {
                id: nouvel_id(),
                annee_id: annee.id,
                ..l.clone()
            };
            pedagogie::inserer_ligne_programme(unite.connexion(), &copie, &contexte).await?;
        }
        let id = annee.id.vers_chaine();
        unite.auditer(
            "programme_matiere",
            &id,
            Operation::Modification,
            None,
            Some(serde_json::json!({ "repris": source.len() })),
            "pedagogie.programme_reprendre",
        );
        unite.emettre(
            TypeEvenement::ProgrammeEnregistre,
            "annee_scolaire",
            &id,
            serde_json::json!({ "repris": source.len() }),
        );
        self.noyau.terminer(unite).await?;
        Ok(source.len())
    }

    async fn niveau_et_periodes(
        &self,
        session: &Session,
        annee_id: Id<AnneeScolaire>,
        niveau_id: Id<Niveau>,
    ) -> Resultat<(Niveau, Vec<Periode>)> {
        let contexte = self.noyau.contexte(session.etablissement_id, Some(session));
        let mut cnx = self.connexion().await?;
        let niveau = academique::niveau(&mut cnx, niveau_id, &contexte).await?;
        let periodes = pedagogie::periodes(&mut cnx, annee_id, &contexte)
            .await?
            .into_iter()
            .filter(|p| p.niveau_id == niveau_id)
            .collect();
        Ok((niveau, periodes))
    }

    /// Change le découpage de l'année d'un niveau, aux dates par défaut : trois ou deux trimestres
    /// au 2e cycle et au lycée (T-26), jamais de semestre ; deux semestres, trois ou deux
    /// trimestres dans l'enseignement technique (Q-71).
    pub async fn definir_periodes(
        &self,
        session: &Session,
        demande: &DemandeNombrePeriodes,
    ) -> Resultat<Vec<Periode>> {
        let annee = self.annee_ouverte(session, demande.annee_id).await?;
        let (niveau, avant) = self
            .niveau_et_periodes(session, annee.id, demande.niveau_id)
            .await?;
        verifier_decoupage(niveau.cycle, demande.nature, demande.nombre).map_err(regle)?;
        let (nature, calendrier) = calendrier_par_defaut(
            niveau.cycle,
            rentree(&annee),
            demande.nature,
            demande.nombre,
        )
        .ok_or(regle("periode.nombre_fixe"))?;
        let apres: Vec<Periode> = (1_i64..)
            .zip(calendrier)
            .map(|(rang, (debut, fin))| Periode {
                id: nouvel_id(),
                etablissement_id: session.etablissement_id,
                annee_id: annee.id,
                niveau_id: niveau.id,
                rang,
                type_periode: nature,
                debut,
                fin,
                ponderation: 1,
                verrouillee: false,
            })
            .collect();
        let contexte = self.noyau.contexte(session.etablissement_id, Some(session));
        let mut unite = self
            .noyau
            .commencer(contexte.clone(), Durabilite::Critique)
            .await?;
        pedagogie::retirer_periodes(unite.connexion(), annee.id, niveau.id, &contexte).await?;
        for p in &apres {
            pedagogie::inserer_periode(unite.connexion(), p, &contexte).await?;
        }
        self.tracer_calendrier(
            &mut unite,
            &niveau,
            &avant,
            &apres,
            "pedagogie.periodes_definir",
        );
        self.noyau.terminer(unite).await?;
        Ok(apres)
    }

    fn tracer_calendrier(
        &self,
        unite: &mut UniteDeTravail<Effet>,
        niveau: &Niveau,
        avant: &[Periode],
        apres: &[Periode],
        origine: &str,
    ) {
        let id = niveau.id.vers_chaine();
        unite.auditer(
            "periode_academique",
            &id,
            Operation::Modification,
            serde_json::to_value(avant).ok(),
            serde_json::to_value(apres).ok(),
            origine,
        );
        unite.emettre(
            TypeEvenement::CalendrierDefini,
            "niveau",
            &id,
            serde_json::json!({ "periodes": apres.len() }),
        );
    }

    /// Corrige les dates et les poids des périodes d'un niveau.
    pub async fn modifier_calendrier(
        &self,
        session: &Session,
        demande: &DemandeCalendrier,
    ) -> Resultat<Vec<Periode>> {
        let annee = self.annee_ouverte(session, demande.annee_id).await?;
        let (niveau, avant) = self
            .niveau_et_periodes(session, annee.id, demande.niveau_id)
            .await?;
        if demande.periodes.len() != avant.len()
            || demande
                .periodes
                .iter()
                .zip(&avant)
                .any(|(s, p)| s.id != p.id)
        {
            return Err(regle("periode.liste_differente"));
        }
        let bornes: Vec<(DateCivile, DateCivile, i64)> = demande
            .periodes
            .iter()
            .map(|s| (s.debut, s.fin, s.ponderation))
            .collect();
        verifier_calendrier(&bornes).map_err(regle)?;
        let apres: Vec<Periode> = avant
            .iter()
            .zip(&demande.periodes)
            .map(|(p, s)| Periode {
                debut: s.debut,
                fin: s.fin,
                ponderation: s.ponderation,
                ..p.clone()
            })
            .collect();
        let contexte = self.noyau.contexte(session.etablissement_id, Some(session));
        let mut unite = self
            .noyau
            .commencer(contexte.clone(), Durabilite::Critique)
            .await?;
        for p in &apres {
            pedagogie::ecrire_periode(unite.connexion(), p, &contexte).await?;
        }
        self.tracer_calendrier(&mut unite, &niveau, &avant, &apres, "pedagogie.calendrier");
        self.noyau.terminer(unite).await?;
        Ok(apres)
    }

    /// Enregistre une nouvelle version de la règle d'un cycle ; les anciennes restent (RC-4).
    pub async fn enregistrer_regle(
        &self,
        session: &Session,
        demande: &DemandeRegle,
    ) -> Resultat<RegleEvaluation> {
        verifier_regle(demande.cycle, &demande.contenu).map_err(regle)?;
        let annee = self.annee_ouverte(session, demande.annee_id).await?;
        let contexte = self.noyau.contexte(session.etablissement_id, Some(session));
        let avant = {
            let mut cnx = self.connexion().await?;
            pedagogie::regles_en_vigueur(&mut cnx, annee.id, &contexte)
                .await?
                .into_iter()
                .find(|r| r.cycle == demande.cycle)
        };
        let apres = RegleEvaluation {
            id: nouvel_id(),
            etablissement_id: session.etablissement_id,
            annee_id: annee.id,
            cycle: demande.cycle,
            version: avant.as_ref().map_or(1, |a| a.version + 1),
            contenu: demande.contenu.clone(),
            le: contexte.maintenant,
        };
        let mut unite = self
            .noyau
            .commencer(contexte.clone(), Durabilite::Critique)
            .await?;
        pedagogie::inserer_regle(unite.connexion(), &apres, &contexte).await?;
        let id = apres.id.vers_chaine();
        unite.auditer(
            "regle_evaluation",
            &id,
            Operation::Creation,
            avant.as_ref().map(serde_json::to_value).transpose()?,
            Some(serde_json::to_value(&apres)?),
            "pedagogie.regle",
        );
        unite.emettre(
            TypeEvenement::RegleEvaluationEnregistree,
            "regle_evaluation",
            &id,
            serde_json::json!({ "cycle": apres.cycle, "version": apres.version }),
        );
        self.noyau.terminer(unite).await?;
        Ok(apres)
    }
}
