//! Journée de l'école et emploi du temps (SPEC_PHASE_8 §8, T-38, T-64) : la journée de chaque
//! cycle que l'école règle, l'horaire de canicule du 1er cycle ; les séances de chaque classe
//! placées à la main dans sa journée, les conflits signalés sans refus ; l'emploi du temps
//! imprimé par classe et par enseignant.

use std::collections::BTreeSet;

use gesco_core::commun::Id;
use gesco_core::core::{AnneeScolaire, Operation, Permission, TypeEvenement};
use gesco_core::pedagogie::emploi_du_temps::{
    Conflit, JOURS, JourneeCycle, Plage, Seance, conflits, verifier_journee, verifier_seance,
};
use gesco_core::pedagogie::{Enseignant, Matiere};
use gesco_core::scolarite::{Classe, Cycle};
use gesco_db::depots::{
    academique, annee_scolaire, emploi_du_temps, enseignants, pedagogie, personnes, role,
};
use gesco_db::unite::Durabilite;
use gesco_db::{Contexte, nouvel_id};
use serde::{Deserialize, Serialize};

use crate::erreur::{ErreurService, Resultat};
use crate::noyau::Noyau;
use crate::session::Session;

fn regle(cle: &'static str) -> ErreurService {
    ErreurService::Regle { cle }
}

/// Nom d'un jour de classe.
pub fn nom_du_jour(jour: u8) -> &'static str {
    match jour {
        1 => "Lundi",
        2 => "Mardi",
        3 => "Mercredi",
        4 => "Jeudi",
        5 => "Vendredi",
        _ => "Samedi",
    }
}

/// Horaire lisible : « 08:00 – 09:00 ».
pub fn horaire(p: Plage) -> String {
    format!(
        "{:02}:{:02} – {:02}:{:02}",
        p.debut / 60,
        p.debut % 60,
        p.fin / 60,
        p.fin % 60
    )
}

/// Matière au programme d'une classe.
#[derive(Debug, Clone, Serialize, specta::Type)]
pub struct MatiereDeClasse {
    /// Matière.
    pub matiere_id: Id<Matiere>,
    /// Libellé.
    pub libelle: String,
    /// Enseignant affecté, s'il y en a un.
    pub enseignant_id: Option<Id<Enseignant>>,
    /// Minutes par semaine prévues au programme.
    pub minutes_prevues: Option<i64>,
    /// Minutes placées.
    pub minutes_placees: i64,
}

/// Classe de l'emploi du temps.
#[derive(Debug, Clone, Serialize, specta::Type)]
pub struct ClasseEmploi {
    /// Classe.
    pub classe_id: Id<Classe>,
    /// Libellé.
    pub libelle: String,
    /// Cycle.
    pub cycle: Cycle,
    /// Matières au programme.
    pub matieres: Vec<MatiereDeClasse>,
}

/// Enseignant de l'emploi du temps.
#[derive(Debug, Clone, Serialize, specta::Type)]
pub struct EnseignantEmploi {
    /// Enseignant.
    pub enseignant_id: Id<Enseignant>,
    /// Prénoms et nom.
    pub nom: String,
}

/// Séance affichée.
#[derive(Debug, Clone, Serialize, specta::Type)]
pub struct SeanceAffichee {
    /// Séance.
    pub seance: Seance,
    /// Classe.
    pub classe: String,
    /// Matière.
    pub matiere: String,
    /// Enseignant.
    pub enseignant: String,
}

/// L'emploi du temps de l'année courante.
#[derive(Debug, Clone, Serialize, specta::Type)]
pub struct EmploiDuTemps {
    /// Année.
    pub annee_id: Id<AnneeScolaire>,
    /// Journée de chaque cycle de l'école.
    pub journees: Vec<JourneeCycle>,
    /// Classes.
    pub classes: Vec<ClasseEmploi>,
    /// Enseignants actifs.
    pub enseignants: Vec<EnseignantEmploi>,
    /// Séances actives.
    pub seances: Vec<SeanceAffichee>,
    /// Conflits signalés.
    pub conflits: Vec<Conflit>,
    /// Qui consulte place les séances et règle les journées.
    pub gere: bool,
}

/// Séance placée ou déplacée.
#[derive(Debug, Deserialize, specta::Type)]
pub struct DemandeSeance {
    /// Séance déplacée ; vide pour une nouvelle.
    pub id: Option<Id<Seance>>,
    /// Classe.
    pub classe_id: Id<Classe>,
    /// Jour, 1 (lundi) à 6 (samedi).
    pub jour: u8,
    /// Début, en minutes depuis minuit.
    pub debut: u16,
    /// Fin.
    pub fin: u16,
    /// Matière.
    pub matiere_id: Id<Matiere>,
    /// Enseignant.
    pub enseignant_id: Option<Id<Enseignant>>,
    /// Salle.
    pub salle: String,
}

/// Séance à retirer.
#[derive(Debug, Deserialize, specta::Type)]
pub struct DemandeRetraitSeance {
    /// Séance.
    pub id: Id<Seance>,
}

/// Emploi du temps d'une classe à imprimer.
#[derive(Debug, Deserialize, specta::Type)]
pub struct DemandeEmploiClasse {
    /// Classe.
    pub classe_id: Id<Classe>,
}

/// Emploi du temps d'un enseignant à imprimer.
#[derive(Debug, Deserialize, specta::Type)]
pub struct DemandeEmploiEnseignant {
    /// Enseignant.
    pub enseignant_id: Id<Enseignant>,
}

/// Service de l'emploi du temps.
pub struct ServiceEmploiDuTemps<'n> {
    noyau: &'n Noyau,
}

/// Ce que l'emploi du temps lit d'une année.
struct Contenu {
    annee: AnneeScolaire,
    journees: Vec<JourneeCycle>,
    classes: Vec<(Classe, Cycle)>,
    programme: Vec<gesco_core::pedagogie::LigneProgramme>,
    matieres: Vec<Matiere>,
    enseignants: Vec<EnseignantEmploi>,
    affectations: Vec<gesco_core::pedagogie::AffectationEnseignement>,
    seances: Vec<Seance>,
}

impl Contenu {
    fn journee_de(&self, classe: Id<Classe>) -> Option<JourneeCycle> {
        let cycle = self.classes.iter().find(|(c, _)| c.id == classe)?.1;
        self.journees.iter().find(|j| j.cycle == cycle).cloned()
    }

    fn programme_de(&self, classe: &Classe) -> Vec<&gesco_core::pedagogie::LigneProgramme> {
        self.programme
            .iter()
            .filter(|l| l.niveau_id == classe.niveau_id && l.serie_id == classe.serie_id)
            .collect()
    }

    fn matiere(&self, id: Id<Matiere>) -> String {
        self.matieres
            .iter()
            .find(|m| m.id == id)
            .map(|m| m.libelle.clone())
            .unwrap_or_default()
    }

    fn enseignant(&self, id: Option<Id<Enseignant>>) -> String {
        id.and_then(|id| self.enseignants.iter().find(|e| e.enseignant_id == id))
            .map(|e| e.nom.clone())
            .unwrap_or_default()
    }

    fn classe(&self, id: Id<Classe>) -> String {
        self.classes
            .iter()
            .find(|(c, _)| c.id == id)
            .map(|(c, _)| c.libelle.clone())
            .unwrap_or_default()
    }

    fn volumes(&self) -> Vec<(Id<Classe>, Id<Matiere>, i64)> {
        self.classes
            .iter()
            .flat_map(|(c, _)| {
                self.programme_de(c)
                    .into_iter()
                    .filter_map(|l| l.minutes_hebdo.map(|m| (c.id, l.matiere_id, m)))
            })
            .collect()
    }

    fn conflits(&self) -> Vec<Conflit> {
        conflits(&self.seances, |c| self.journee_de(c), &self.volumes())
    }
}

impl<'n> ServiceEmploiDuTemps<'n> {
    /// Service sur le noyau.
    pub fn nouveau(noyau: &'n Noyau) -> Self {
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

    async fn contenu(cnx: &mut gesco_db::Connexion, contexte: &Contexte) -> Resultat<Contenu> {
        let annee = annee_scolaire::courante(cnx, contexte)
            .await?
            .ok_or(regle("emploi.sans_annee"))?;
        let niveaux = academique::niveaux(cnx, contexte).await?;
        let classes: Vec<(Classe, Cycle)> = academique::classes(cnx, annee.id, contexte)
            .await?
            .into_iter()
            .filter_map(|c| {
                let cycle = niveaux.iter().find(|n| n.id == c.niveau_id)?.cycle;
                Some((c, cycle))
            })
            .collect();
        let reglees = emploi_du_temps::journees(cnx, contexte).await?;
        let cycles: BTreeSet<&str> = classes.iter().map(|(_, cycle)| cycle.nom()).collect();
        let journees = Cycle::TOUS
            .into_iter()
            .filter(|c| cycles.contains(c.nom()))
            .map(|c| {
                reglees
                    .iter()
                    .find(|j| j.cycle == c)
                    .cloned()
                    .unwrap_or_else(|| JourneeCycle::par_defaut(c))
            })
            .collect();
        let mut liste = Vec::new();
        for e in enseignants::enseignants(cnx, contexte).await? {
            if !e.actif {
                continue;
            }
            let p = personnes::personne(cnx, e.personne_id, contexte).await?;
            liste.push(EnseignantEmploi {
                enseignant_id: e.id,
                nom: p.nom_complet(),
            });
        }
        liste.sort_by(|a, b| a.nom.cmp(&b.nom));
        Ok(Contenu {
            programme: pedagogie::programme(cnx, annee.id, contexte).await?,
            matieres: pedagogie::matieres(cnx, contexte).await?,
            affectations: enseignants::affectations(cnx, annee.id, contexte).await?,
            seances: emploi_du_temps::seances(cnx, annee.id, contexte)
                .await?
                .into_iter()
                .filter(|s| s.active)
                .collect(),
            enseignants: liste,
            annee,
            journees,
            classes,
        })
    }

    /// L'emploi du temps de l'année courante.
    pub async fn lire(&self, session: &Session) -> Resultat<EmploiDuTemps> {
        let contexte = self.noyau.contexte(session.etablissement_id, Some(session));
        let mut cnx = self.connexion().await?;
        let gere = role::permissions_de(&mut cnx, session.utilisateur_id, &contexte)
            .await?
            .contains(&Permission::TIMETABLE_MANAGE);
        let c = Self::contenu(&mut cnx, &contexte).await?;
        let classes = c
            .classes
            .iter()
            .map(|(classe, cycle)| ClasseEmploi {
                classe_id: classe.id,
                libelle: classe.libelle.clone(),
                cycle: *cycle,
                matieres: c
                    .programme_de(classe)
                    .into_iter()
                    .map(|l| MatiereDeClasse {
                        matiere_id: l.matiere_id,
                        libelle: c.matiere(l.matiere_id),
                        enseignant_id: c
                            .affectations
                            .iter()
                            .find(|a| a.classe_id == classe.id && a.matiere_id == l.matiere_id)
                            .map(|a| a.enseignant_id),
                        minutes_prevues: l.minutes_hebdo,
                        minutes_placees: c
                            .seances
                            .iter()
                            .filter(|s| s.classe_id == classe.id && s.matiere_id == l.matiere_id)
                            .map(|s| i64::from(s.plage.minutes()))
                            .sum(),
                    })
                    .collect(),
            })
            .collect();
        let seances = c
            .seances
            .iter()
            .map(|s| SeanceAffichee {
                classe: c.classe(s.classe_id),
                matiere: c.matiere(s.matiere_id),
                enseignant: c.enseignant(s.enseignant_id),
                seance: s.clone(),
            })
            .collect();
        Ok(EmploiDuTemps {
            annee_id: c.annee.id,
            conflits: c.conflits(),
            journees: c.journees.clone(),
            enseignants: c.enseignants.clone(),
            classes,
            seances,
            gere,
        })
    }

    /// Règle la journée d'un cycle.
    pub async fn regler_journee(
        &self,
        session: &Session,
        journee: &JourneeCycle,
    ) -> Resultat<EmploiDuTemps> {
        let mut journee = journee.clone();
        journee.jours.sort_unstable();
        journee.jours.dedup();
        journee.recreations.sort_by_key(|r| r.debut);
        verifier_journee(&journee).map_err(regle)?;
        let contexte = self.noyau.contexte(session.etablissement_id, Some(session));
        let mut unite = self
            .noyau
            .commencer(contexte.clone(), Durabilite::Ordinaire)
            .await?;
        emploi_du_temps::ecrire_journee(unite.connexion(), &journee, &contexte).await?;
        unite.auditer(
            "journee_cycle",
            journee.cycle.nom(),
            Operation::Modification,
            None,
            Some(serde_json::to_value(&journee)?),
            "emploi_du_temps.journee",
        );
        unite.emettre(
            TypeEvenement::JourneeCycleReglee,
            "journee_cycle",
            journee.cycle.nom(),
            serde_json::json!({ "cycle": journee.cycle }),
        );
        self.noyau.terminer(unite).await?;
        self.lire(session).await
    }

    /// Place ou déplace une séance dans la journée de sa classe ; ses conflits sont signalés.
    pub async fn placer(
        &self,
        session: &Session,
        demande: &DemandeSeance,
    ) -> Resultat<EmploiDuTemps> {
        let contexte = self.noyau.contexte(session.etablissement_id, Some(session));
        let mut unite = self
            .noyau
            .commencer(contexte.clone(), Durabilite::Ordinaire)
            .await?;
        let c = Self::contenu(unite.connexion(), &contexte).await?;
        let (classe, _) = c
            .classes
            .iter()
            .find(|(cl, _)| cl.id == demande.classe_id)
            .ok_or(regle("seance.classe_inconnue"))?;
        if !c
            .programme_de(classe)
            .iter()
            .any(|l| l.matiere_id == demande.matiere_id)
        {
            return Err(regle("seance.matiere_hors_programme"));
        }
        if demande
            .enseignant_id
            .is_some_and(|e| !c.enseignants.iter().any(|x| x.enseignant_id == e))
        {
            return Err(regle("seance.enseignant_inconnu"));
        }
        let avant = match demande.id {
            Some(id) => Some(
                emploi_du_temps::seances(unite.connexion(), c.annee.id, &contexte)
                    .await?
                    .into_iter()
                    .find(|s| s.id == id && s.active)
                    .ok_or(regle("seance.inconnue"))?,
            ),
            None => None,
        };
        let seance = Seance {
            id: demande.id.unwrap_or_else(nouvel_id),
            etablissement_id: session.etablissement_id,
            annee_id: c.annee.id,
            classe_id: classe.id,
            jour: demande.jour,
            plage: Plage {
                debut: demande.debut,
                fin: demande.fin,
            },
            matiere_id: demande.matiere_id,
            enseignant_id: demande.enseignant_id,
            salle: demande.salle.trim().to_owned(),
            active: true,
        };
        let journee = c
            .journee_de(classe.id)
            .ok_or(regle("seance.classe_inconnue"))?;
        if !JOURS.contains(&seance.jour) {
            return Err(regle("seance.hors_journee"));
        }
        verifier_seance(&seance, &journee).map_err(regle)?;
        if avant.is_some() {
            emploi_du_temps::ecrire_seance(unite.connexion(), &seance, &contexte).await?;
        } else {
            emploi_du_temps::inserer_seance(unite.connexion(), &seance, &contexte).await?;
        }
        let id = seance.id.vers_chaine();
        unite.auditer(
            "seance",
            &id,
            if avant.is_some() {
                Operation::Modification
            } else {
                Operation::Creation
            },
            avant.map(|a| serde_json::to_value(&a)).transpose()?,
            Some(serde_json::to_value(&seance)?),
            "emploi_du_temps.placer",
        );
        unite.emettre(
            TypeEvenement::SeancePlacee,
            "seance",
            &id,
            serde_json::json!({ "classe": classe.libelle, "jour": seance.jour,
                "debut": seance.plage.debut, "fin": seance.plage.fin }),
        );
        self.noyau.terminer(unite).await?;
        self.lire(session).await
    }

    /// Retire une séance.
    pub async fn retirer(
        &self,
        session: &Session,
        demande: &DemandeRetraitSeance,
    ) -> Resultat<EmploiDuTemps> {
        let contexte = self.noyau.contexte(session.etablissement_id, Some(session));
        let mut unite = self
            .noyau
            .commencer(contexte.clone(), Durabilite::Ordinaire)
            .await?;
        let annee = annee_scolaire::courante(unite.connexion(), &contexte)
            .await?
            .ok_or(regle("emploi.sans_annee"))?;
        let avant = emploi_du_temps::seances(unite.connexion(), annee.id, &contexte)
            .await?
            .into_iter()
            .find(|s| s.id == demande.id && s.active)
            .ok_or(regle("seance.inconnue"))?;
        let apres = Seance {
            active: false,
            ..avant.clone()
        };
        emploi_du_temps::ecrire_seance(unite.connexion(), &apres, &contexte).await?;
        let id = apres.id.vers_chaine();
        unite.auditer(
            "seance",
            &id,
            Operation::Modification,
            Some(serde_json::json!({ "active": true })),
            Some(serde_json::json!({ "active": false })),
            "emploi_du_temps.retirer",
        );
        unite.emettre(
            TypeEvenement::SeancePlacee,
            "seance",
            &id,
            serde_json::json!({ "retiree": true }),
        );
        self.noyau.terminer(unite).await?;
        self.lire(session).await
    }

    /// Données de l'emploi du temps imprimé d'une classe ou d'un enseignant : une colonne par
    /// jour de classe, une ligne par horaire.
    fn grille(
        c: &Contenu,
        seances: &[&Seance],
        detail: impl Fn(&Seance) -> String,
    ) -> (Vec<&'static str>, Vec<serde_json::Value>) {
        let mut jours: Vec<u8> = c
            .journees
            .iter()
            .flat_map(|j| j.jours.iter().copied())
            .chain(seances.iter().map(|s| s.jour))
            .collect();
        jours.sort_unstable();
        jours.dedup();
        let mut plages: Vec<Plage> = seances.iter().map(|s| s.plage).collect();
        plages.sort_by_key(|p| (p.debut, p.fin));
        plages.dedup();
        let lignes = plages
            .iter()
            .map(|p| {
                serde_json::json!({
                    "horaire": horaire(*p),
                    "cases": jours.iter().map(|j| {
                        seances.iter().find(|s| s.jour == *j && s.plage == *p).map(|s| {
                            serde_json::json!({ "matiere": c.matiere(s.matiere_id),
                                "detail": detail(s), "salle": s.salle })
                        })
                    }).collect::<Vec<_>>(),
                })
            })
            .collect();
        (jours.into_iter().map(nom_du_jour).collect(), lignes)
    }

    /// Données de l'emploi du temps imprimé d'une classe.
    pub async fn donnees_classe(
        &self,
        session: &Session,
        demande: &DemandeEmploiClasse,
    ) -> Resultat<serde_json::Value> {
        let contexte = self.noyau.contexte(session.etablissement_id, Some(session));
        let mut cnx = self.connexion().await?;
        let c = Self::contenu(&mut cnx, &contexte).await?;
        let (classe, cycle) = c
            .classes
            .iter()
            .find(|(cl, _)| cl.id == demande.classe_id)
            .ok_or(regle("seance.classe_inconnue"))?;
        let siennes: Vec<&Seance> = c
            .seances
            .iter()
            .filter(|s| s.classe_id == classe.id)
            .collect();
        let (jours, lignes) = Self::grille(&c, &siennes, |s| c.enseignant(s.enseignant_id));
        Ok(serde_json::json!({
            "titre": format!("Emploi du temps — {}", classe.libelle),
            "sous_titre": format!("Année {} · {}", c.annee.libelle, libelle_cycle(*cycle)),
            "jours": jours,
            "lignes": lignes,
            "date": crate::documents::date_longue(self.noyau.horloge().maintenant()),
        }))
    }

    /// Données de l'emploi du temps imprimé d'un enseignant.
    pub async fn donnees_enseignant(
        &self,
        session: &Session,
        demande: &DemandeEmploiEnseignant,
    ) -> Resultat<serde_json::Value> {
        let contexte = self.noyau.contexte(session.etablissement_id, Some(session));
        let mut cnx = self.connexion().await?;
        let c = Self::contenu(&mut cnx, &contexte).await?;
        let nom = c.enseignant(Some(demande.enseignant_id));
        if nom.is_empty() {
            return Err(regle("seance.enseignant_inconnu"));
        }
        let siens: Vec<&Seance> = c
            .seances
            .iter()
            .filter(|s| s.enseignant_id == Some(demande.enseignant_id))
            .collect();
        let minutes: i64 = siens.iter().map(|s| i64::from(s.plage.minutes())).sum();
        let (jours, lignes) = Self::grille(&c, &siens, |s| c.classe(s.classe_id));
        Ok(serde_json::json!({
            "titre": format!("Emploi du temps — {nom}"),
            "sous_titre": format!("Année {} · {} h {:02} par semaine", c.annee.libelle,
                minutes / 60, minutes % 60),
            "jours": jours,
            "lignes": lignes,
            "date": crate::documents::date_longue(self.noyau.horloge().maintenant()),
        }))
    }
}

/// Libellé imprimé d'un cycle.
fn libelle_cycle(cycle: Cycle) -> &'static str {
    match cycle {
        Cycle::Jardin => "Jardin d'enfants",
        Cycle::PremierCycle => "1er cycle",
        Cycle::SecondCycle => "2nd cycle",
        Cycle::Lycee => "Lycée",
        Cycle::Technique => "Enseignement technique",
    }
}
