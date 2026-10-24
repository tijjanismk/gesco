//! Résultats, appréciations et bulletins (SPEC_PHASE_4 §7, UC-04) : la direction calcule les
//! bulletins d'une classe pour une période à partir des notes, les enseignants et la direction
//! y portent leurs appréciations, la direction valide puis publie ; le PDF de la classe sort d'un
//! seul tenant, chaque bulletin avec son QR de vérification. Publié, un bulletin ne change plus :
//! une correction en crée une nouvelle version. Les impayés ne bloquent rien (T-36).

use gesco_core::commun::{DateCivile, Horodatage, Id};
use gesco_core::core::{MachineAEtats, Operation, Permission, TypeEvenement};
use gesco_core::pedagogie::assiduite::{TotauxAbsences, totaux};
use gesco_core::pedagogie::bulletin::{
    REGLAGES_IMPRESSION, arrive_pendant, non_suivie, reglages_impression, texte_rang,
    verifier_reprise,
};
use gesco_core::pedagogie::calcul::{NoteCalcul, Rang, ecrire};
use gesco_core::pedagogie::evaluation::{SaisieNote, lire_saisie};
use gesco_core::pedagogie::regle::bareme_du_cycle;
use gesco_core::pedagogie::resultats::{NotesEleve, calculer_classe, moyennes_annuelles};
use gesco_core::pedagogie::{
    Bulletin, ConduiteBulletin, ContenuBulletin, ImpressionMoyenneAnnuelle, LigneBulletin, Matiere,
    MoyenneReprise, Periode, RegleEvaluation, StatutBulletin, StatutEvaluation,
};
use gesco_core::scolarite::{Classe, Cycle, DiplomeTerminal, Inscription, StatutInscription};
use gesco_db::depots::bulletins::APPRECIATION_GENERALE;
use gesco_db::depots::{
    absences, academique, annee_scolaire, bulletins, enseignants, evaluations, parametre,
    pedagogie, role,
};
use gesco_db::nouvel_id;
use gesco_db::unite::Durabilite;
use gesco_docs::{Piece, qr};
use serde::{Deserialize, Serialize};

use crate::annees;
use crate::enseignants::ServiceEnseignants;
use crate::erreur::{ErreurService, Resultat};
use crate::inscriptions::ServiceInscriptions;
use crate::noyau::Noyau;
use crate::session::Session;

fn regle(cle: &'static str) -> ErreurService {
    ErreurService::Regle { cle }
}

/// Plus longue appréciation imprimée.
const APPRECIATION_MAXIMUM: usize = 300;

/// Bulletins d'une classe pour une période.
#[derive(Debug, Clone, Deserialize, specta::Type)]
pub struct DemandeBulletins {
    /// Classe.
    pub classe_id: Id<Classe>,
    /// Période.
    pub periode_id: Id<Periode>,
}

/// Version antérieure d'un bulletin, consultable.
#[derive(Debug, Clone, Serialize, specta::Type)]
pub struct VersionBulletin {
    /// Bulletin.
    pub id: Id<Bulletin>,
    /// Version.
    pub version: i64,
    /// État.
    pub statut: StatutBulletin,
    /// Publication.
    pub publie_le: Option<Horodatage>,
}

/// Bulletin en vigueur d'un élève, avec ses versions antérieures.
#[derive(Debug, Clone, Serialize, specta::Type)]
pub struct BulletinAffiche {
    /// Dernière version.
    pub bulletin: Bulletin,
    /// Versions antérieures, de la plus ancienne à la plus récente.
    pub anterieures: Vec<VersionBulletin>,
}

/// Bulletins d'une classe pour une période.
#[derive(Debug, Clone, Serialize, specta::Type)]
pub struct BulletinsClasse {
    /// Classe.
    pub classe: String,
    /// Période.
    pub periode: String,
    /// Cycle.
    pub cycle: Cycle,
    /// Barème du niveau, en centièmes.
    pub bareme: i32,
    /// Bulletins, dans l'ordre de la liste de classe.
    pub bulletins: Vec<BulletinAffiche>,
    /// Évaluations de la période encore en saisie : la validation attend qu'elles soient validées.
    pub evaluations_en_saisie: u32,
    /// Matières dont l'utilisateur écrit l'appréciation ; toutes pour la direction.
    pub matieres_appreciables: Vec<Id<Matiere>>,
    /// Vrai si l'utilisateur écrit l'appréciation générale.
    pub appreciation_generale: bool,
}

/// Appréciation d'un élève, dans une matière ou générale.
#[derive(Debug, Clone, Deserialize, specta::Type)]
pub struct DemandeAppreciation {
    /// Élève.
    pub inscription_id: Id<Inscription>,
    /// Période.
    pub periode_id: Id<Periode>,
    /// Matière ; rien pour l'appréciation générale de la direction.
    pub matiere_id: Option<Id<Matiere>>,
    /// Texte libre ; vide pour l'effacer.
    pub texte: String,
}

/// Bulletin désigné.
#[derive(Debug, Clone, Deserialize, specta::Type)]
pub struct DemandeIdBulletin {
    /// Bulletin.
    pub id: Id<Bulletin>,
}

/// Correction d'un bulletin publié.
#[derive(Debug, Clone, Deserialize, specta::Type)]
pub struct DemandeCorrection {
    /// Bulletin publié.
    pub id: Id<Bulletin>,
    /// Motif, obligatoire.
    pub motif: String,
}

/// Moyenne reprise saisie depuis le relevé d'une autre école.
#[derive(Debug, Clone, Deserialize, specta::Type)]
pub struct DemandeReprise {
    /// Classe de l'élève.
    pub classe_id: Id<Classe>,
    /// Élève.
    pub inscription_id: Id<Inscription>,
    /// Période qu'il a suivie ailleurs.
    pub periode_id: Id<Periode>,
    /// Moyenne telle qu'écrite sur le relevé (`12,50`) ; vide pour la retirer.
    pub moyenne: String,
    /// École qui a délivré le relevé.
    pub etablissement_origine: String,
    /// Référence du relevé.
    pub piece: String,
}

/// Élève qui n'a pas suivi la période, et sa moyenne reprise.
#[derive(Debug, Clone, Serialize, specta::Type)]
pub struct LigneReprise {
    /// Inscription.
    pub inscription_id: Id<Inscription>,
    /// Prénoms et nom.
    pub eleve: String,
    /// Matricule.
    pub matricule: String,
    /// Arrivée dans l'école.
    pub inscrit_le: DateCivile,
    /// Moyenne reprise, si elle est saisie.
    pub reprise: Option<MoyenneReprise>,
}

/// Élèves d'une classe arrivés après une période.
#[derive(Debug, Clone, Serialize, specta::Type)]
pub struct ReprisesClasse {
    /// Barème du niveau, en centièmes.
    pub bareme: i32,
    /// Élèves.
    pub lignes: Vec<LigneReprise>,
}

/// Ce que le calcul d'une classe rassemble.
struct Classement {
    classe: Classe,
    periode: Periode,
    regle: RegleEvaluation,
    /// Contenu calculé de chaque élève de la classe, dans l'ordre de la liste.
    contenus: Vec<(Id<Inscription>, ContenuBulletin)>,
    evaluations_en_saisie: u32,
}

/// Service des bulletins.
pub struct ServiceBulletins<'n> {
    noyau: &'n Noyau,
}

impl<'n> ServiceBulletins<'n> {
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

    async fn permissions(&self, session: &Session) -> Resultat<Vec<Permission>> {
        let contexte = self.noyau.contexte(session.etablissement_id, Some(session));
        let mut cnx = self.connexion().await?;
        Ok(role::permissions_de(&mut cnx, session.utilisateur_id, &contexte).await?)
    }

    /// Calcule le contenu des bulletins d'une classe, sans rien écrire.
    async fn classer(&self, session: &Session, demande: &DemandeBulletins) -> Resultat<Classement> {
        let contexte = self.noyau.contexte(session.etablissement_id, Some(session));
        let mut cnx = self.connexion().await?;
        let classe = academique::classe(&mut cnx, demande.classe_id, &contexte).await?;
        let niveau = academique::niveau(&mut cnx, classe.niveau_id, &contexte).await?;
        let niveau_imprime =
            crate::referentiel::niveau_imprime(&mut cnx, &niveau, &classe, &contexte).await?;
        let bareme = bareme_du_cycle(niveau.cycle).ok_or(regle("bulletin.cycle_sans_notes"))?;
        let annee = annee_scolaire::lire(&mut cnx, classe.annee_id, &contexte).await?;
        let mut periodes_du_niveau: Vec<Periode> =
            pedagogie::periodes(&mut cnx, classe.annee_id, &contexte)
                .await?
                .into_iter()
                .filter(|p| p.niveau_id == classe.niveau_id)
                .collect();
        periodes_du_niveau.sort_by_key(|p| p.rang);
        let periode = periodes_du_niveau
            .iter()
            .find(|p| p.id == demande.periode_id)
            .cloned()
            .ok_or(regle("evaluation.periode_invalide"))?;
        let regle_cycle = pedagogie::regles_en_vigueur(&mut cnx, classe.annee_id, &contexte)
            .await?
            .into_iter()
            .find(|r| r.cycle == niveau.cycle)
            .ok_or(regle("evaluation.sans_regle"))?;
        let mut programme: Vec<_> = pedagogie::programme(&mut cnx, classe.annee_id, &contexte)
            .await?
            .into_iter()
            .filter(|l| l.niveau_id == classe.niveau_id && l.serie_id == classe.serie_id)
            .collect();
        programme.sort_by_key(|l| l.ordre);
        let matieres = pedagogie::matieres(&mut cnx, &contexte).await?;
        let affectations = enseignants::affectations(&mut cnx, classe.annee_id, &contexte)
            .await?
            .into_iter()
            .filter(|a| a.classe_id == classe.id)
            .collect::<Vec<_>>();
        let liste = evaluations::de_la_classe(&mut cnx, classe.id, periode.id, &contexte).await?;
        let mut notes_evaluations = Vec::with_capacity(liste.len());
        for e in &liste {
            notes_evaluations.push(evaluations::notes(&mut cnx, e.id, &contexte).await?);
        }
        let absences_periode =
            absences::de_la_classe(&mut cnx, classe.id, periode.debut, periode.fin, &contexte)
                .await?;
        let appreciations = bulletins::appreciations(&mut cnx, periode.id, &contexte).await?;
        let sanctions =
            crate::discipline::du_conseil_imprimees(&mut cnx, periode.id, &contexte).await?;
        drop(cnx);
        let fiches = ServiceEnseignants::nouveau(self.noyau)
            .lister(session)
            .await?;
        // Un élève arrivé après la fin de la période ne l'a pas suivie (Q-27) : pas de bulletin.
        let eleves: Vec<(Id<Inscription>, String, String, DateCivile)> =
            ServiceInscriptions::nouveau(self.noyau)
                .lister(session, classe.annee_id)
                .await?
                .into_iter()
                .filter(|l| {
                    l.inscription.classe_id == classe.id
                        && l.inscription.statut == StatutInscription::Validee
                        && !non_suivie(l.inscription.date_inscription, &periode)
                })
                .map(|l| {
                    (
                        l.inscription.id,
                        l.eleve,
                        l.matricule,
                        l.inscription.date_inscription,
                    )
                })
                .collect();

        // Note de conduite, si l'école l'a activée pour ce cycle.
        let conduite = {
            let ids: Vec<Id<Inscription>> = eleves.iter().map(|(i, _, _, _)| *i).collect();
            let mut cnx = self.connexion().await?;
            crate::conduite::pour_bulletins(
                &mut cnx,
                &classe,
                &periode,
                niveau.cycle,
                &ids,
                &contexte,
            )
            .await?
        };
        let note_conduite = |i: Id<Inscription>| {
            conduite
                .as_ref()
                .and_then(|(_, _, notes)| notes.iter().find(|(x, _)| *x == i))
                .map(|(_, n)| *n)
        };
        let coefficients: Vec<(Id<Matiere>, i64)> = programme
            .iter()
            .map(|l| (l.matiere_id, l.coefficient.unwrap_or(1)))
            .collect();
        let entrees: Vec<NotesEleve> = eleves
            .iter()
            .map(|(inscription_id, _, _, inscrit_le)| NotesEleve {
                inscription_id: *inscription_id,
                classable: regle_cycle.contenu.classer_arrivee_en_cours
                    || !arrive_pendant(*inscrit_le, &periode),
                conduite: note_conduite(*inscription_id),
                notes: liste
                    .iter()
                    .zip(&notes_evaluations)
                    .filter_map(|(e, notes)| {
                        notes
                            .iter()
                            .find(|n| n.inscription_id == *inscription_id)
                            .map(|n| {
                                (
                                    e.matiere_id,
                                    NoteCalcul {
                                        type_evaluation: e.type_evaluation,
                                        bareme: e.bareme,
                                        ponderation: e.ponderation,
                                        valeur: n.valeur,
                                        absent: n.absent,
                                        justifie: n.justifie,
                                        dispense: n.dispense,
                                    },
                                )
                            })
                    })
                    .collect(),
            })
            .collect();
        let coefficient_conduite = conduite
            .as_ref()
            .and_then(|(coefficient, comptee, _)| comptee.then_some(*coefficient));
        let resultats = calculer_classe(
            &regle_cycle.contenu,
            bareme,
            &coefficients,
            &entrees,
            coefficient_conduite,
        )
        .map_err(|_| regle("bulletin.calcul_impossible"))?;
        // Moyenne annuelle à la dernière période de l'année, pour toutes les classes (T-33) :
        // les moyennes des périodes déjà calculées, la moyenne reprise d'une période suivie
        // ailleurs (Q-28), rien pour une période non suivie (Q-27).
        let derniere = periodes_du_niveau
            .last()
            .is_some_and(|p| p.id == periode.id);
        let annuelles = if derniere {
            let mut cnx = self.connexion().await?;
            let ids: Vec<Id<Inscription>> = eleves.iter().map(|(i, _, _, _)| *i).collect();
            let reprises = bulletins::moyennes_reprises(&mut cnx, &ids, &contexte).await?;
            let mut par_eleve = Vec::with_capacity(eleves.len());
            for ((inscription_id, _, _, inscrit_le), r) in eleves.iter().zip(&resultats.eleves) {
                let siens =
                    bulletins::de_l_inscription(&mut cnx, *inscription_id, &contexte).await?;
                // Une période suivie dont le bulletin n'est pas encore calculé : pas de moyenne
                // annuelle tant qu'elle manque (une période calculée sans moyenne, elle, sort du
                // calcul comme une note absente).
                let mut manque = false;
                let moyennes: Vec<(Option<i32>, i64)> = periodes_du_niveau
                    .iter()
                    .map(|p| {
                        let moyenne = if p.id == periode.id {
                            r.moyenne
                        } else if non_suivie(*inscrit_le, p) {
                            reprises
                                .iter()
                                .find(|x| {
                                    x.inscription_id == *inscription_id && x.periode_id == p.id
                                })
                                .map(|x| x.moyenne)
                        } else {
                            match siens
                                .iter()
                                .filter(|b| b.periode_id == p.id)
                                .max_by_key(|b| b.version)
                            {
                                Some(b) => b.contenu.moyenne,
                                None => {
                                    manque = true;
                                    None
                                }
                            }
                        };
                        (moyenne, p.ponderation)
                    })
                    .collect();
                par_eleve.push(if manque { Vec::new() } else { moyennes });
            }
            moyennes_annuelles(&regle_cycle.contenu, &par_eleve)
                .map_err(|_| regle("bulletin.calcul_impossible"))?
        } else {
            Vec::new()
        };
        let annuelle_imprimee = regle_cycle.contenu.moyenne_annuelle
            == ImpressionMoyenneAnnuelle::Toutes
            || niveau.diplome_terminal != DiplomeTerminal::Aucun;
        let libelle_matiere = |id: Id<Matiere>| {
            matieres
                .iter()
                .find(|m| m.id == id)
                .map(|m| m.libelle.clone())
                .unwrap_or_default()
        };
        let enseignant_de = |id: Id<Matiere>| {
            affectations
                .iter()
                .find(|a| a.matiere_id == id)
                .and_then(|a| fiches.iter().find(|f| f.enseignant.id == a.enseignant_id))
                .map(|f| format!("{} {}", f.prenoms, f.nom))
                .unwrap_or_default()
        };
        let appreciation = |i: Id<Inscription>, cle: &str| {
            appreciations
                .iter()
                .find(|(x, m, _)| *x == i && m == cle)
                .map(|(_, _, t)| t.clone())
                .unwrap_or_default()
        };
        let effectif = u32::try_from(eleves.len()).unwrap_or(u32::MAX);
        let contenus = eleves
            .iter()
            .zip(resultats.eleves)
            .enumerate()
            .map(|(i, ((inscription_id, eleve, matricule, inscrit_le), r))| {
                let annuelle = annuelles.get(i).copied();
                let siennes: Vec<_> = absences_periode
                    .iter()
                    .filter(|a| a.inscription_id == *inscription_id)
                    .cloned()
                    .collect();
                // Une matière qu'aucun élève de la classe n'a eue sur la période (pas enseignée
                // à ce niveau, ou pas encore évaluée) ne s'imprime pas : elle encombrerait le
                // bulletin de tirets (réponse du 15 octobre 2026). Un élève seul sans note dans
                // une matière des autres la garde, « non noté » (Q-59).
                let lignes = r
                    .matieres
                    .iter()
                    .filter(|l| {
                        resultats
                            .moyennes_matieres
                            .iter()
                            .any(|m| m.matiere_id == l.matiere_id && m.moyenne.is_some())
                    })
                    .map(|l| LigneBulletin {
                        matiere_id: l.matiere_id,
                        matiere: libelle_matiere(l.matiere_id),
                        enseignant: enseignant_de(l.matiere_id),
                        note_de_classe: l.note_de_classe,
                        composition: l.composition,
                        moyenne: l.moyenne,
                        coefficient: l.coefficient,
                        points: l.points,
                        moyenne_classe: resultats
                            .moyennes_matieres
                            .iter()
                            .find(|m| m.matiere_id == l.matiere_id)
                            .and_then(|m| m.moyenne),
                        appreciation: appreciation(*inscription_id, &l.matiere_id.vers_chaine()),
                    })
                    .collect();
                (
                    *inscription_id,
                    ContenuBulletin {
                        eleve: eleve.clone(),
                        matricule: matricule.clone(),
                        classe: classe.libelle.clone(),
                        niveau: niveau_imprime.clone(),
                        cycle: niveau.cycle,
                        annee: annee.libelle.clone(),
                        periode: periode.libelle(),
                        bareme,
                        lignes,
                        total_points: r.total_points,
                        total_coefficients: r.total_coefficients,
                        moyenne: r.moyenne,
                        rang: r.rang,
                        mention: r.mention,
                        effectif,
                        ont_compose: resultats.ont_compose,
                        premier: resultats.resume.premier,
                        dernier: resultats.resume.dernier,
                        moyenne_classe: resultats.moyenne_classe,
                        absences: totaux(&siennes, periode.debut, periode.fin),
                        appreciation_generale: appreciation(*inscription_id, APPRECIATION_GENERALE),
                        moyenne_annuelle: annuelle.and_then(|a| a.moyenne),
                        rang_annuel: annuelle.and_then(|a| a.rang),
                        moyenne_annuelle_imprimee: annuelle.is_some() && annuelle_imprimee,
                        arrive_le: arrive_pendant(*inscrit_le, &periode).then_some(*inscrit_le),
                        sanctions_conseil: sanctions
                            .iter()
                            .filter(|(i, _)| i == inscription_id)
                            .map(|(_, texte)| texte.clone())
                            .collect(),
                        conduite: r.conduite.map(|note| ConduiteBulletin {
                            note,
                            coefficient: conduite.as_ref().map_or(1, |(c, _, _)| *c),
                            points: r.points_conduite,
                        }),
                    },
                )
            })
            .collect();
        Ok(Classement {
            evaluations_en_saisie: u32::try_from(
                liste
                    .iter()
                    .filter(|e| e.statut == StatutEvaluation::EnSaisie)
                    .count(),
            )
            .unwrap_or(u32::MAX),
            classe,
            periode,
            regle: regle_cycle,
            contenus,
        })
    }

    /// Bulletins enregistrés d'une classe : la dernière version de chaque élève.
    async fn derniers(
        &self,
        session: &Session,
        demande: &DemandeBulletins,
    ) -> Resultat<Vec<BulletinAffiche>> {
        let contexte = self.noyau.contexte(session.etablissement_id, Some(session));
        let mut cnx = self.connexion().await?;
        let tous =
            bulletins::de_la_classe(&mut cnx, demande.classe_id, demande.periode_id, &contexte)
                .await?;
        let mut affiches: Vec<BulletinAffiche> = Vec::new();
        for b in tous {
            match affiches
                .iter_mut()
                .find(|a| a.bulletin.inscription_id == b.inscription_id)
            {
                Some(a) => {
                    let ancien = std::mem::replace(&mut a.bulletin, b);
                    a.anterieures.push(VersionBulletin {
                        id: ancien.id,
                        version: ancien.version,
                        statut: ancien.statut,
                        publie_le: ancien.publie_le,
                    });
                }
                None => affiches.push(BulletinAffiche {
                    bulletin: b,
                    anterieures: Vec::new(),
                }),
            }
        }
        Ok(affiches)
    }

    /// Bulletins d'une classe pour une période, sans recalcul, dans l'ordre de la liste.
    pub async fn lire(
        &self,
        session: &Session,
        demande: &DemandeBulletins,
    ) -> Resultat<BulletinsClasse> {
        let classement = self.classer(session, demande).await?;
        let mut derniers = self.derniers(session, demande).await?;
        let ordre: Vec<Id<Inscription>> = classement.contenus.iter().map(|(i, _)| *i).collect();
        derniers.sort_by_key(|a| {
            ordre
                .iter()
                .position(|i| *i == a.bulletin.inscription_id)
                .unwrap_or(usize::MAX)
        });
        let permissions = self.permissions(session).await?;
        let portee = ServiceEnseignants::nouveau(self.noyau)
            .portee_notes(session)
            .await?;
        Ok(BulletinsClasse {
            classe: classement.classe.libelle.clone(),
            periode: classement.periode.libelle(),
            cycle: classement
                .contenus
                .first()
                .map_or(Cycle::SecondCycle, |(_, c)| c.cycle),
            bareme: classement.contenus.first().map_or(2000, |(_, c)| c.bareme),
            matieres_appreciables: classement
                .contenus
                .first()
                .map(|(_, c)| {
                    c.lignes
                        .iter()
                        .map(|l| l.matiere_id)
                        .filter(|m| portee.autorise(classement.classe.id, *m))
                        .collect()
                })
                .unwrap_or_default(),
            appreciation_generale: permissions.contains(&Permission::GRADES_VALIDATE),
            evaluations_en_saisie: classement.evaluations_en_saisie,
            bulletins: derniers,
        })
    }

    /// Calcule ou recalcule les bulletins non publiés d'une classe. Un bulletin validé dont le
    /// contenu change revient au calcul ; un bulletin publié ne bouge pas.
    pub async fn calculer(
        &self,
        session: &Session,
        demande: &DemandeBulletins,
    ) -> Resultat<BulletinsClasse> {
        annees::exiger_classe_modifiable(self.noyau, session, demande.classe_id).await?;
        let classement = self.classer(session, demande).await?;
        let derniers = self.derniers(session, demande).await?;
        let contexte = self.noyau.contexte(session.etablissement_id, Some(session));
        let mut unite = self
            .noyau
            .commencer(contexte.clone(), Durabilite::Critique)
            .await?;
        let mut calcules = 0_u32;
        for (inscription_id, contenu) in &classement.contenus {
            let dernier = derniers
                .iter()
                .find(|a| a.bulletin.inscription_id == *inscription_id)
                .map(|a| &a.bulletin);
            match dernier {
                None => {
                    let b = Bulletin {
                        id: nouvel_id(),
                        etablissement_id: session.etablissement_id,
                        inscription_id: *inscription_id,
                        periode_id: classement.periode.id,
                        classe_id: classement.classe.id,
                        version: 1,
                        statut: StatutBulletin::Calcule,
                        contenu: contenu.clone(),
                        regle_version: classement.regle.version,
                        motif_correction: String::new(),
                        calcule_le: contexte.maintenant,
                        publie_le: None,
                    };
                    bulletins::inserer(unite.connexion(), &b, &contexte).await?;
                    unite.auditer(
                        "bulletin",
                        &b.id.vers_chaine(),
                        Operation::Creation,
                        None,
                        Some(serde_json::to_value(&b)?),
                        "bulletins.calculer",
                    );
                    calcules += 1;
                }
                Some(avant) if !avant.statut.fige() => {
                    if avant.contenu == *contenu && avant.regle_version == classement.regle.version
                    {
                        continue;
                    }
                    let apres = Bulletin {
                        statut: StatutBulletin::Calcule,
                        contenu: contenu.clone(),
                        regle_version: classement.regle.version,
                        calcule_le: contexte.maintenant,
                        ..avant.clone()
                    };
                    bulletins::recrire(unite.connexion(), &apres, &contexte).await?;
                    unite.auditer(
                        "bulletin",
                        &apres.id.vers_chaine(),
                        Operation::Modification,
                        Some(serde_json::to_value(avant)?),
                        Some(serde_json::to_value(&apres)?),
                        "bulletins.calculer",
                    );
                    calcules += 1;
                }
                Some(_) => {}
            }
        }
        unite.emettre(
            TypeEvenement::ResultatCalcule,
            "classe",
            &classement.classe.id.vers_chaine(),
            serde_json::json!({ "periode_id": classement.periode.id, "bulletins": calcules,
                "regle_version": classement.regle.version }),
        );
        self.noyau.terminer(unite).await?;
        self.lire(session, demande).await
    }

    /// Écrit l'appréciation d'un élève dans une matière (son enseignant, ou la direction) ou
    /// l'appréciation générale (la direction), et la reporte sur son bulletin non publié.
    pub async fn apprecier(
        &self,
        session: &Session,
        demande: &DemandeAppreciation,
    ) -> Resultat<Bulletin> {
        let texte = demande.texte.trim();
        if texte.chars().count() > APPRECIATION_MAXIMUM {
            return Err(regle("appreciation.trop_longue"));
        }
        annees::exiger_inscription_modifiable(self.noyau, session, demande.inscription_id).await?;
        let contexte = self.noyau.contexte(session.etablissement_id, Some(session));
        let avant = {
            let mut cnx = self.connexion().await?;
            bulletins::de_l_inscription(&mut cnx, demande.inscription_id, &contexte)
                .await?
                .into_iter()
                .filter(|b| b.periode_id == demande.periode_id)
                .max_by_key(|b| b.version)
        }
        .ok_or(regle("appreciation.sans_bulletin"))?;
        if avant.statut.fige() {
            return Err(regle("bulletin.publie"));
        }
        let cle = match demande.matiere_id {
            Some(matiere) => {
                let portee = ServiceEnseignants::nouveau(self.noyau)
                    .portee_notes(session)
                    .await?;
                if !portee.autorise(avant.classe_id, matiere) {
                    return Err(ErreurService::PorteeRefusee {
                        ressource: format!("classe {}, matière {matiere}", avant.classe_id),
                    });
                }
                if !avant.contenu.lignes.iter().any(|l| l.matiere_id == matiere) {
                    return Err(regle("affectation.matiere_hors_programme"));
                }
                matiere.vers_chaine()
            }
            None => {
                if !self
                    .permissions(session)
                    .await?
                    .contains(&Permission::GRADES_VALIDATE)
                {
                    return Err(ErreurService::PorteeRefusee {
                        ressource: "appréciation générale".to_owned(),
                    });
                }
                APPRECIATION_GENERALE.to_owned()
            }
        };
        let mut contenu = avant.contenu.clone();
        match demande.matiere_id {
            Some(matiere) => {
                for l in contenu
                    .lignes
                    .iter_mut()
                    .filter(|l| l.matiere_id == matiere)
                {
                    texte.clone_into(&mut l.appreciation);
                }
            }
            None => texte.clone_into(&mut contenu.appreciation_generale),
        }
        let apres = Bulletin {
            contenu,
            ..avant.clone()
        };
        let mut unite = self
            .noyau
            .commencer(contexte.clone(), Durabilite::Critique)
            .await?;
        bulletins::ecrire_appreciation(
            unite.connexion(),
            demande.inscription_id,
            demande.periode_id,
            &cle,
            texte,
            &contexte,
        )
        .await?;
        bulletins::recrire(unite.connexion(), &apres, &contexte).await?;
        unite.auditer(
            "appreciation",
            &format!("{}:{}:{cle}", demande.inscription_id, demande.periode_id),
            Operation::Modification,
            None,
            Some(serde_json::json!({ "texte": texte })),
            "bulletins.apprecier",
        );
        unite.emettre(
            TypeEvenement::AppreciationSaisie,
            "bulletin",
            &apres.id.vers_chaine(),
            serde_json::json!({ "matiere": cle }),
        );
        self.noyau.terminer(unite).await?;
        Ok(apres)
    }

    async fn transiter(
        &self,
        session: &Session,
        bulletins_a_changer: &[Bulletin],
        statut: StatutBulletin,
        evenement: TypeEvenement,
        origine: &str,
    ) -> Resultat<()> {
        let contexte = self.noyau.contexte(session.etablissement_id, Some(session));
        let mut unite = self
            .noyau
            .commencer(contexte.clone(), Durabilite::Critique)
            .await?;
        for avant in bulletins_a_changer {
            let apres_statut = avant.statut.transiter(statut)?;
            bulletins::ecrire_statut(unite.connexion(), avant.id, apres_statut, &contexte).await?;
            let id = avant.id.vers_chaine();
            unite.auditer(
                "bulletin",
                &id,
                Operation::Transition,
                Some(serde_json::json!({ "statut": avant.statut })),
                Some(serde_json::json!({ "statut": apres_statut })),
                origine,
            );
            unite.emettre(
                evenement,
                "bulletin",
                &id,
                serde_json::json!({ "inscription_id": avant.inscription_id,
                    "periode_id": avant.periode_id, "version": avant.version }),
            );
            // L'avis « bulletin à retirer » naît dans la transaction de la publication.
            if apres_statut == StatutBulletin::Publie {
                crate::remises_documents::a_la_publication(unite.connexion(), avant, &contexte)
                    .await?;
                crate::messages_auto::avis_bulletin(unite.connexion(), avant, &contexte).await?;
            }
        }
        self.noyau.terminer(unite).await?;
        Ok(())
    }

    /// Valide les bulletins calculés d'une classe. Toutes les évaluations de la période doivent
    /// être validées : un bulletin ne se valide pas sur des notes encore en saisie.
    pub async fn valider(&self, session: &Session, demande: &DemandeBulletins) -> Resultat<u32> {
        annees::exiger_classe_modifiable(self.noyau, session, demande.classe_id).await?;
        let classement = self.classer(session, demande).await?;
        if classement.evaluations_en_saisie > 0 {
            return Err(regle("bulletin.evaluations_en_saisie"));
        }
        let a_valider: Vec<Bulletin> = self
            .derniers(session, demande)
            .await?
            .into_iter()
            .map(|a| a.bulletin)
            .filter(|b| b.statut == StatutBulletin::Calcule)
            .collect();
        // Un bulletin dont les notes ont changé depuis le calcul doit être recalculé.
        if a_valider.iter().any(|b| {
            classement
                .contenus
                .iter()
                .any(|(i, c)| *i == b.inscription_id && *c != b.contenu)
        }) {
            return Err(regle("bulletin.a_recalculer"));
        }
        self.transiter(
            session,
            &a_valider,
            StatutBulletin::Valide,
            TypeEvenement::BulletinValide,
            "bulletins.valider",
        )
        .await?;
        Ok(u32::try_from(a_valider.len()).unwrap_or(u32::MAX))
    }

    /// Publie les bulletins validés d'une classe ; la version publiée qu'une correction remplace
    /// est marquée remplacée. Rend les bulletins publiés.
    pub async fn publier(
        &self,
        session: &Session,
        demande: &DemandeBulletins,
    ) -> Resultat<Vec<Bulletin>> {
        annees::exiger_classe_modifiable(self.noyau, session, demande.classe_id).await?;
        let derniers = self.derniers(session, demande).await?;
        let a_publier: Vec<Bulletin> = derniers
            .iter()
            .map(|a| a.bulletin.clone())
            .filter(|b| b.statut == StatutBulletin::Valide)
            .collect();
        if a_publier.is_empty() {
            return Err(regle("bulletin.rien_a_publier"));
        }
        let contexte = self.noyau.contexte(session.etablissement_id, Some(session));
        let mut remplaces = Vec::new();
        {
            let mut cnx = self.connexion().await?;
            for b in &a_publier {
                remplaces.extend(
                    bulletins::de_l_inscription(&mut cnx, b.inscription_id, &contexte)
                        .await?
                        .into_iter()
                        .filter(|x| {
                            x.periode_id == b.periode_id
                                && x.id != b.id
                                && x.statut == StatutBulletin::Publie
                        }),
                );
            }
        }
        self.transiter(
            session,
            &a_publier,
            StatutBulletin::Publie,
            TypeEvenement::BulletinPublie,
            "bulletins.publier",
        )
        .await?;
        if !remplaces.is_empty() {
            self.transiter(
                session,
                &remplaces,
                StatutBulletin::Remplace,
                TypeEvenement::BulletinCorrige,
                "bulletins.publier",
            )
            .await?;
        }
        let mut cnx = self.connexion().await?;
        let mut publies = Vec::with_capacity(a_publier.len());
        for b in &a_publier {
            publies.push(bulletins::lire(&mut cnx, b.id, &contexte).await?);
        }
        Ok(publies)
    }

    /// Corrige un bulletin publié : une nouvelle version, recalculée avec les notes et les
    /// appréciations du jour, à valider puis publier ; l'ancienne reste en vigueur jusque-là.
    pub async fn corriger(
        &self,
        session: &Session,
        demande: &DemandeCorrection,
    ) -> Resultat<Bulletin> {
        let motif = demande.motif.trim();
        if motif.is_empty() {
            return Err(regle("bulletin.motif_obligatoire"));
        }
        let corrige = self.un(session, demande.id).await?;
        annees::exiger_inscription_modifiable(self.noyau, session, corrige.inscription_id).await?;
        let contexte = self.noyau.contexte(session.etablissement_id, Some(session));
        let publie = {
            let mut cnx = self.connexion().await?;
            bulletins::lire(&mut cnx, demande.id, &contexte).await?
        };
        let demande_classe = DemandeBulletins {
            classe_id: publie.classe_id,
            periode_id: publie.periode_id,
        };
        let derniers = self.derniers(session, &demande_classe).await?;
        let dernier = derniers
            .iter()
            .find(|a| a.bulletin.inscription_id == publie.inscription_id)
            .map(|a| &a.bulletin);
        if publie.statut != StatutBulletin::Publie || dernier.map(|d| d.id) != Some(publie.id) {
            return Err(regle("bulletin.non_corrigeable"));
        }
        let classement = self.classer(session, &demande_classe).await?;
        let contenu = classement
            .contenus
            .into_iter()
            .find(|(i, _)| *i == publie.inscription_id)
            .map(|(_, c)| c)
            .ok_or(regle("bulletin.eleve_hors_classe"))?;
        let b = Bulletin {
            id: nouvel_id(),
            version: publie.version + 1,
            statut: StatutBulletin::Calcule,
            contenu,
            regle_version: classement.regle.version,
            motif_correction: motif.to_owned(),
            calcule_le: contexte.maintenant,
            publie_le: None,
            ..publie.clone()
        };
        let mut unite = self
            .noyau
            .commencer(contexte.clone(), Durabilite::Critique)
            .await?;
        bulletins::inserer(unite.connexion(), &b, &contexte).await?;
        unite.auditer(
            "bulletin",
            &b.id.vers_chaine(),
            Operation::Creation,
            Some(serde_json::json!({ "remplace": publie.id, "version": publie.version })),
            Some(serde_json::to_value(&b)?),
            "bulletins.corriger",
        );
        self.noyau.terminer(unite).await?;
        Ok(b)
    }

    /// Bulletins à imprimer pour une classe : la dernière version de chaque élève.
    pub async fn a_imprimer(
        &self,
        session: &Session,
        demande: &DemandeBulletins,
    ) -> Resultat<Vec<Bulletin>> {
        let classe = self.lire(session, demande).await?;
        if classe.bulletins.is_empty() {
            return Err(regle("bulletin.aucun"));
        }
        Ok(classe.bulletins.into_iter().map(|a| a.bulletin).collect())
    }

    /// Élèves de la classe qui n'ont pas suivi cette période, arrivés après elle, et leur
    /// moyenne reprise d'une autre école (Q-28).
    pub async fn reprises(
        &self,
        session: &Session,
        demande: &DemandeBulletins,
    ) -> Resultat<ReprisesClasse> {
        let contexte = self.noyau.contexte(session.etablissement_id, Some(session));
        let (periode, bareme) = self
            .periode_de_classe(session, demande.classe_id, demande.periode_id)
            .await?;
        let eleves: Vec<_> = ServiceInscriptions::nouveau(self.noyau)
            .lister(session, periode.annee_id)
            .await?
            .into_iter()
            .filter(|l| {
                l.inscription.classe_id == demande.classe_id
                    && l.inscription.statut == StatutInscription::Validee
                    && non_suivie(l.inscription.date_inscription, &periode)
            })
            .collect();
        let ids: Vec<Id<Inscription>> = eleves.iter().map(|l| l.inscription.id).collect();
        let reprises = {
            let mut cnx = self.connexion().await?;
            bulletins::moyennes_reprises(&mut cnx, &ids, &contexte).await?
        };
        Ok(ReprisesClasse {
            bareme,
            lignes: eleves
                .into_iter()
                .map(|l| LigneReprise {
                    reprise: reprises
                        .iter()
                        .find(|r| {
                            r.inscription_id == l.inscription.id && r.periode_id == periode.id
                        })
                        .cloned(),
                    inscription_id: l.inscription.id,
                    eleve: l.eleve,
                    matricule: l.matricule,
                    inscrit_le: l.inscription.date_inscription,
                })
                .collect(),
        })
    }

    async fn periode_de_classe(
        &self,
        session: &Session,
        classe_id: Id<Classe>,
        periode_id: Id<Periode>,
    ) -> Resultat<(Periode, i32)> {
        let contexte = self.noyau.contexte(session.etablissement_id, Some(session));
        let mut cnx = self.connexion().await?;
        let classe = academique::classe(&mut cnx, classe_id, &contexte).await?;
        let niveau = academique::niveau(&mut cnx, classe.niveau_id, &contexte).await?;
        let bareme = bareme_du_cycle(niveau.cycle).ok_or(regle("bulletin.cycle_sans_notes"))?;
        let periode = pedagogie::periodes(&mut cnx, classe.annee_id, &contexte)
            .await?
            .into_iter()
            .find(|p| p.id == periode_id && p.niveau_id == classe.niveau_id)
            .ok_or(regle("evaluation.periode_invalide"))?;
        Ok((periode, bareme))
    }

    /// Enregistre, corrige ou retire (moyenne vide) la moyenne reprise d'un élève pour une
    /// période qu'il n'a pas suivie dans l'école, sur présentation du relevé officiel (Q-28).
    pub async fn reprendre(
        &self,
        session: &Session,
        demande: &DemandeReprise,
    ) -> Resultat<Option<MoyenneReprise>> {
        annees::exiger_classe_modifiable(self.noyau, session, demande.classe_id).await?;
        let contexte = self.noyau.contexte(session.etablissement_id, Some(session));
        let (periode, bareme) = self
            .periode_de_classe(session, demande.classe_id, demande.periode_id)
            .await?;
        let inscription = ServiceInscriptions::nouveau(self.noyau)
            .lister(session, periode.annee_id)
            .await?
            .into_iter()
            .find(|l| {
                l.inscription.id == demande.inscription_id
                    && l.inscription.classe_id == demande.classe_id
            })
            .ok_or(regle("moyenne_reprise.eleve_inconnu"))?
            .inscription;
        if !non_suivie(inscription.date_inscription, &periode) {
            return Err(regle("moyenne_reprise.periode_suivie"));
        }
        let apres = match lire_saisie(&demande.moyenne, bareme).map_err(regle)? {
            SaisieNote::Effacee => None,
            SaisieNote::Valeur(moyenne) => {
                let r = MoyenneReprise {
                    inscription_id: inscription.id,
                    periode_id: periode.id,
                    moyenne,
                    etablissement_origine: demande.etablissement_origine.trim().to_owned(),
                    piece: demande.piece.trim().to_owned(),
                };
                verifier_reprise(&r, bareme).map_err(regle)?;
                Some(r)
            }
            _ => return Err(regle("moyenne_reprise.illisible")),
        };
        let avant = {
            let mut cnx = self.connexion().await?;
            bulletins::moyennes_reprises(&mut cnx, &[inscription.id], &contexte)
                .await?
                .into_iter()
                .find(|r| r.periode_id == periode.id)
        };
        let mut unite = self
            .noyau
            .commencer(contexte.clone(), Durabilite::Critique)
            .await?;
        bulletins::ecrire_moyenne_reprise(
            unite.connexion(),
            inscription.id,
            periode.id,
            apres.as_ref(),
            &contexte,
        )
        .await?;
        let id = format!("{}:{}", inscription.id, periode.id);
        unite.auditer(
            "moyenne_reprise",
            &id,
            match (&avant, &apres) {
                (None, _) => Operation::Creation,
                (Some(_), None) => Operation::Suppression,
                (Some(_), Some(_)) => Operation::Modification,
            },
            avant.as_ref().map(serde_json::to_value).transpose()?,
            apres.as_ref().map(serde_json::to_value).transpose()?,
            "bulletins.reprendre",
        );
        unite.emettre(
            TypeEvenement::MoyenneReprise,
            "inscription",
            &inscription.id.vers_chaine(),
            serde_json::json!({ "periode_id": periode.id, "retiree": apres.is_none() }),
        );
        self.noyau.terminer(unite).await?;
        Ok(apres)
    }

    /// Ce que l'école veut voir imprimé sur les bulletins d'un cycle : `{ "colonne_coefficient":
    /// true, "rang": true, … }`. Une colonne propre aux bulletins trimestriels vaut faux au 1er
    /// cycle, qui ne l'imprime pas.
    pub async fn reglages_impression(
        &self,
        session: &Session,
        cycle: Cycle,
    ) -> Resultat<serde_json::Value> {
        let contexte = self.noyau.contexte(session.etablissement_id, Some(session));
        let mut cnx = self.connexion().await?;
        let mut afficher = serde_json::Map::new();
        for (suffixe, _, _) in REGLAGES_IMPRESSION {
            let nom = suffixe
                .strip_prefix("colonne.")
                .map(|n| format!("colonne_{n}"))
                .or_else(|| suffixe.strip_prefix("afficher.").map(str::to_owned))
                .unwrap_or_else(|| suffixe.to_owned());
            afficher.insert(nom, false.into());
        }
        for (cle, defaut) in reglages_impression(cycle) {
            let suffixe = cle.splitn(3, '.').nth(2).unwrap_or_default();
            let nom = suffixe
                .strip_prefix("colonne.")
                .map(|n| format!("colonne_{n}"))
                .or_else(|| suffixe.strip_prefix("afficher.").map(str::to_owned))
                .unwrap_or_else(|| suffixe.to_owned());
            afficher.insert(
                nom,
                parametre::booleen_param(&mut cnx, &cle, defaut, &contexte)
                    .await?
                    .into(),
            );
        }
        Ok(serde_json::Value::Object(afficher))
    }

    /// Un bulletin, dans n'importe quelle version.
    pub async fn un(&self, session: &Session, id: Id<Bulletin>) -> Resultat<Bulletin> {
        let contexte = self.noyau.contexte(session.etablissement_id, Some(session));
        let mut cnx = self.connexion().await?;
        Ok(bulletins::lire(&mut cnx, id, &contexte).await?)
    }
}

fn valeur(centiemes: Option<i32>) -> String {
    centiemes.map(ecrire).unwrap_or_default()
}

fn points(centiemes: Option<i64>) -> String {
    centiemes
        .and_then(|p| i32::try_from(p).ok())
        .map(ecrire)
        .unwrap_or_default()
}

fn rang(r: Option<Rang>) -> String {
    r.map(texte_rang).unwrap_or_default()
}

fn absences_imprimees(a: TotauxAbsences) -> serde_json::Value {
    serde_json::json!({
        "demi_journees": a.demi_journees,
        "justifiees": a.justifiees,
        "retards": a.retards,
    })
}

/// Données du modèle `bulletin` et QR de vérification de chaque bulletin : ce qui s'imprime,
/// déjà écrit (« 13,35 », « 3e ex »). Un bulletin non publié porte « Projet », une version
/// remplacée le dit.
pub fn donnees_impression(
    bulletins_a_imprimer: &[Bulletin],
    reglages: serde_json::Value,
) -> Resultat<(serde_json::Value, Vec<Piece>)> {
    let mut pieces = Vec::with_capacity(bulletins_a_imprimer.len());
    let mut liste = Vec::with_capacity(bulletins_a_imprimer.len());
    for (i, b) in bulletins_a_imprimer.iter().enumerate() {
        let reference = b.id.vers_chaine();
        let mut piece = qr::piece(&qr::reference(&reference))?;
        piece.nom = format!("qr-{i}.svg");
        let c = &b.contenu;
        let etat = match b.statut {
            StatutBulletin::Calcule | StatutBulletin::Valide => {
                "Projet — bulletin non publié".to_owned()
            }
            StatutBulletin::Publie if b.version > 1 => format!("Version corrigée n° {}", b.version),
            StatutBulletin::Publie => String::new(),
            StatutBulletin::Remplace => format!("Version n° {}, remplacée", b.version),
        };
        liste.push(serde_json::json!({
            "premier_cycle": c.cycle == Cycle::PremierCycle,
            "eleve": c.eleve,
            "matricule": c.matricule,
            "classe": c.classe,
            "niveau": c.niveau,
            "annee": c.annee,
            "periode": c.periode,
            "bareme": ecrire(c.bareme).trim_end_matches(",00").to_owned(),
            "lignes": c.lignes.iter().map(|l| serde_json::json!({
                "matiere": l.matiere,
                "enseignant": l.enseignant,
                "note_de_classe": valeur(l.note_de_classe),
                "composition": valeur(l.composition),
                "moyenne": valeur(l.moyenne),
                "coefficient": l.coefficient.to_string(),
                "points": points(l.points),
                "moyenne_classe": valeur(l.moyenne_classe),
                "appreciation": l.appreciation,
            })).collect::<Vec<_>>(),
            "total_points": points(Some(c.total_points)),
            "total_coefficients": c.total_coefficients.to_string(),
            "moyenne": valeur(c.moyenne),
            "rang": rang(c.rang),
            "mention": c.mention.clone().unwrap_or_default(),
            "effectif": c.effectif.to_string(),
            "ont_compose": c.ont_compose.to_string(),
            "premier": valeur(c.premier),
            "dernier": valeur(c.dernier),
            "moyenne_classe": valeur(c.moyenne_classe),
            "absences": absences_imprimees(c.absences),
            "appreciation_generale": c.appreciation_generale,
            "moyenne_annuelle": c.moyenne_annuelle.filter(|_| c.moyenne_annuelle_imprimee).map(ecrire),
            "rang_annuel": rang(c.rang_annuel),
            "arrive_le": c.arrive_le.map(crate::documents::date_civile_longue),
            "sanctions": c.sanctions_conseil,
            // Note de conduite : une ligne du tableau quand elle compte, du bilan sinon.
            "conduite": c.conduite.map(|k| serde_json::json!({
                "note": ecrire(k.note),
                "coefficient": k.coefficient.to_string(),
                "points": points(k.points),
                "comptee": k.points.is_some(),
            })),
            "etat": etat,
            "qr": piece.chemin(),
            "reference": reference,
        }));
        pieces.push(piece);
    }
    Ok((
        serde_json::json!({ "bulletins": liste, "afficher": reglages }),
        pieces,
    ))
}
