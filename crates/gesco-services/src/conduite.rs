//! Note de conduite (porteur du projet, 24 octobre 2026) : au 2e cycle, au lycée et dans
//! l'enseignement technique, la direction ou le surveillant saisit la note de chaque élève pour
//! une période, ou GESCO la calcule d'après le registre de discipline, les exclusions temporaires
//! et les absences ; une note calculée se corrige avec un motif. Le calcul des bulletins la
//! reprend : elle compte dans la moyenne générale avec son coefficient, ou s'imprime à part.

use gesco_core::commun::Id;
use gesco_core::core::{ModuleEcole, Operation, TypeEvenement};
use gesco_core::pedagogie::assiduite::totaux;
use gesco_core::pedagogie::bulletin::non_suivie;
use gesco_core::pedagogie::conduite::{
    COEFFICIENT_DEFAUT, DecompteConduite, ModeConduite, NoteConduite, Retraits, cycle_note,
    lire_note, lire_points, note_automatique, parametres,
};
use gesco_core::pedagogie::regle::bareme_du_cycle;
use gesco_core::pedagogie::{Absence, Periode};
use gesco_core::scolarite::depart::{NatureExclusion, StatutExclusion};
use gesco_core::scolarite::discipline::{NatureSanction, Sanction};
use gesco_core::scolarite::{Classe, Inscription, StatutInscription};
use gesco_db::depots::{
    absences, academique, annee_scolaire, conduite, departs, discipline, inscriptions, parametre,
    pedagogie,
};
use gesco_db::unite::Durabilite;
use gesco_db::{Connexion, Contexte, nouvel_id};
use serde::{Deserialize, Serialize};

use crate::erreur::{ErreurService, Resultat};
use crate::inscriptions::ServiceInscriptions;
use crate::noyau::Noyau;
use crate::session::Session;

fn regle(cle: &'static str) -> ErreurService {
    ErreurService::Regle { cle }
}

/// Motif du retrait d'une note remplacée par une nouvelle saisie.
const MOTIF_REMPLACEE: &str = "Remplacée par une nouvelle note";

/// Période d'une classe, à choisir.
#[derive(Debug, Clone, Serialize, specta::Type)]
pub struct PeriodeConduite {
    /// Période.
    pub periode_id: Id<Periode>,
    /// « 1er trimestre ».
    pub libelle: String,
}

/// Classe qui a une note de conduite, avec ses périodes.
#[derive(Debug, Clone, Serialize, specta::Type)]
pub struct ClasseConduite {
    /// Classe.
    pub classe_id: Id<Classe>,
    /// Libellé.
    pub libelle: String,
    /// Périodes de son niveau, dans l'ordre.
    pub periodes: Vec<PeriodeConduite>,
}

/// Classes de l'année courante qui ont une note de conduite.
#[derive(Debug, Clone, Serialize, specta::Type)]
pub struct ClassesConduite {
    /// Saisie ou calculée.
    pub mode: ModeConduite,
    /// Classes du 2e cycle, du lycée et du technique.
    pub classes: Vec<ClasseConduite>,
}

/// Note de conduite d'une classe pour une période.
#[derive(Debug, Clone, Deserialize, specta::Type)]
pub struct DemandeConduite {
    /// Classe.
    pub classe_id: Id<Classe>,
    /// Période.
    pub periode_id: Id<Periode>,
}

/// Ligne d'un élève.
#[derive(Debug, Clone, Serialize, specta::Type)]
pub struct LigneConduite {
    /// Inscription.
    pub inscription_id: Id<Inscription>,
    /// Prénoms et nom.
    pub eleve: String,
    /// Matricule.
    pub matricule: String,
    /// Ce qui pèse sur sa conduite, en mode calculé.
    pub decompte: Option<DecompteConduite>,
    /// Note calculée, en mode calculé.
    pub calculee: Option<i32>,
    /// Note saisie : la note du mode saisi, ou la correction d'une note calculée.
    pub saisie: Option<NoteConduite>,
    /// Note retenue pour le bulletin.
    pub note: Option<i32>,
}

/// Notes de conduite d'une classe pour une période.
#[derive(Debug, Clone, Serialize, specta::Type)]
pub struct ConduiteClasse {
    /// Saisie ou calculée.
    pub mode: ModeConduite,
    /// Vrai si elle compte dans la moyenne générale.
    pub comptee: bool,
    /// Coefficient, quand elle compte.
    pub coefficient: i64,
    /// Barème du niveau, en centièmes.
    pub bareme: i32,
    /// Retraits du calcul, en centièmes.
    pub retraits: Retraits,
    /// Vrai si plus rien n'entre dans la période.
    pub verrouillee: bool,
    /// Élèves, dans l'ordre de la liste de classe.
    pub lignes: Vec<LigneConduite>,
}

/// Saisie ou correction de la note d'un élève.
#[derive(Debug, Clone, Deserialize, specta::Type)]
pub struct DemandeNoteConduite {
    /// Classe.
    pub classe_id: Id<Classe>,
    /// Période.
    pub periode_id: Id<Periode>,
    /// Élève.
    pub inscription_id: Id<Inscription>,
    /// Note telle que tapée : `15`, `14,5`.
    pub note: String,
    /// Motif : obligatoire pour corriger une note calculée.
    pub motif: String,
}

/// Retrait de la note saisie d'un élève.
#[derive(Debug, Clone, Deserialize, specta::Type)]
pub struct DemandeRetraitConduite {
    /// Classe.
    pub classe_id: Id<Classe>,
    /// Période.
    pub periode_id: Id<Periode>,
    /// Élève.
    pub inscription_id: Id<Inscription>,
    /// Motif, obligatoire.
    pub motif: String,
}

/// Réglages de la note de conduite de l'école.
struct Reglages {
    mode: ModeConduite,
    comptee: bool,
    coefficient: i64,
    retraits: Retraits,
}

async fn reglages(cnx: &mut Connexion, contexte: &Contexte) -> Resultat<Reglages> {
    let mode =
        parametre::texte(cnx, parametres::MODE, ModeConduite::DEFAUT.nom(), contexte).await?;
    let defaut = Retraits::par_defaut();
    let mut lus = Vec::with_capacity(7);
    for (cle, repli) in [
        (parametres::RETRAIT_AVERTISSEMENT, defaut.avertissement),
        (parametres::RETRAIT_BLAME, defaut.blame),
        (parametres::RETRAIT_RETENUE, defaut.retenue),
        (parametres::RETRAIT_CONVOCATION, defaut.convocation),
        (parametres::RETRAIT_EXCLUSION, defaut.exclusion),
        (parametres::RETRAIT_ABSENCE, defaut.absence),
        (parametres::RETRAIT_RETARD, defaut.retard),
    ] {
        let texte = parametre::texte(cnx, cle, "", contexte).await?;
        lus.push(lire_points(&texte).unwrap_or(repli));
    }
    let lu = |i: usize| lus.get(i).copied().unwrap_or(0);
    Ok(Reglages {
        mode: ModeConduite::depuis_nom(&mode).unwrap_or(ModeConduite::DEFAUT),
        comptee: parametre::booleen_param(cnx, parametres::COMPTEE, true, contexte).await?,
        coefficient: parametre::entier(cnx, parametres::COEFFICIENT, COEFFICIENT_DEFAUT, contexte)
            .await?,
        retraits: Retraits {
            avertissement: lu(0),
            blame: lu(1),
            retenue: lu(2),
            convocation: lu(3),
            exclusion: lu(4),
            absence: lu(5),
            retard: lu(6),
        },
    })
}

/// Ce qui pèse sur la conduite d'un élève pendant une période : les sanctions de la période (un
/// avertissement ou un blâme du conseil compte pour la période de son bulletin), les exclusions
/// temporaires qui y commencent, les absences non justifiées et les retards.
fn decompter(
    inscription_id: Id<Inscription>,
    periode: &Periode,
    sanctions: &[Sanction],
    exclusions: u32,
    absences: &[Absence],
) -> DecompteConduite {
    let dans = |s: &&Sanction| {
        s.inscription_id == inscription_id
            && match s.periode_id {
                Some(p) => p == periode.id,
                None => periode.debut <= s.date && s.date <= periode.fin,
            }
    };
    let nombre = |nature: NatureSanction| {
        u32::try_from(
            sanctions
                .iter()
                .filter(dans)
                .filter(|s| s.nature == nature)
                .count(),
        )
        .unwrap_or(u32::MAX)
    };
    let siennes: Vec<Absence> = absences
        .iter()
        .filter(|a| a.inscription_id == inscription_id)
        .cloned()
        .collect();
    let t = totaux(&siennes, periode.debut, periode.fin);
    DecompteConduite {
        avertissements: nombre(NatureSanction::Avertissement),
        blames: nombre(NatureSanction::Blame),
        retenues: nombre(NatureSanction::Retenue),
        convocations: nombre(NatureSanction::ConvocationParents),
        exclusions,
        absences: t.demi_journees.saturating_sub(t.justifiees),
        retards: t.retards,
    }
}

/// Une ligne par élève : son décompte et sa note calculée en mode calculé, sa note saisie.
type Calcul = (
    Id<Inscription>,
    Option<DecompteConduite>,
    Option<i32>,
    Option<NoteConduite>,
);

async fn calculer(
    cnx: &mut Connexion,
    reglages: &Reglages,
    classe: &Classe,
    periode: &Periode,
    bareme: i32,
    eleves: &[Id<Inscription>],
    contexte: &Contexte,
) -> Resultat<Vec<Calcul>> {
    let saisies = conduite::de_la_periode(cnx, periode.id, contexte).await?;
    let saisie = |i: Id<Inscription>| saisies.iter().find(|n| n.inscription_id == i).cloned();
    if reglages.mode == ModeConduite::Manuelle {
        return Ok(eleves
            .iter()
            .map(|i| (*i, None, None, saisie(*i)))
            .collect());
    }
    let sanctions = discipline::de_la_classe(cnx, classe.id, contexte).await?;
    let absences_classe =
        absences::de_la_classe(cnx, classe.id, periode.debut, periode.fin, contexte).await?;
    let mut lignes = Vec::with_capacity(eleves.len());
    for inscription_id in eleves {
        let exclusions = departs::exclusions(cnx, *inscription_id, contexte)
            .await?
            .iter()
            .filter(|e| {
                e.nature == NatureExclusion::Temporaire
                    && e.statut != StatutExclusion::Revoquee
                    && periode.debut <= e.du
                    && e.du <= periode.fin
            })
            .count();
        let d = decompter(
            *inscription_id,
            periode,
            &sanctions,
            u32::try_from(exclusions).unwrap_or(u32::MAX),
            &absences_classe,
        );
        lignes.push((
            *inscription_id,
            Some(d),
            Some(note_automatique(bareme, &reglages.retraits, &d)),
            saisie(*inscription_id),
        ));
    }
    Ok(lignes)
}

/// Notes de conduite d'une classe pour le calcul de ses bulletins : rien si l'école n'a pas
/// activé la note de conduite ou si le cycle n'en a pas ; sinon le coefficient (rien quand la
/// note s'imprime à part) et la note retenue de chaque élève.
pub(crate) async fn pour_bulletins(
    cnx: &mut Connexion,
    classe: &Classe,
    periode: &Periode,
    cycle: gesco_core::scolarite::Cycle,
    eleves: &[Id<Inscription>],
    contexte: &Contexte,
) -> Resultat<Option<(i64, bool, Vec<(Id<Inscription>, i32)>)>> {
    let Some(bareme) = bareme_du_cycle(cycle).filter(|_| cycle_note(cycle)) else {
        return Ok(None);
    };
    if !crate::modules::actif(cnx, ModuleEcole::Conduite, contexte).await? {
        return Ok(None);
    }
    let r = reglages(cnx, contexte).await?;
    let notes = calculer(cnx, &r, classe, periode, bareme, eleves, contexte)
        .await?
        .into_iter()
        .filter_map(|(i, _, calculee, saisie)| saisie.map(|s| s.note).or(calculee).map(|n| (i, n)))
        .collect();
    Ok(Some((r.coefficient, r.comptee, notes)))
}

/// Service de la note de conduite.
pub struct ServiceConduite<'n> {
    noyau: &'n Noyau,
}

impl<'n> ServiceConduite<'n> {
    /// Service sur le noyau.
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

    /// Refuse tout tant que l'école n'a pas activé la note de conduite.
    async fn exiger_active(cnx: &mut Connexion, contexte: &Contexte) -> Resultat<()> {
        if crate::modules::actif(cnx, ModuleEcole::Conduite, contexte).await? {
            Ok(())
        } else {
            Err(regle("conduite.inactive"))
        }
    }

    /// Classes de l'année courante qui ont une note de conduite, avec leurs périodes.
    pub async fn classes(&self, session: &Session) -> Resultat<ClassesConduite> {
        let contexte = self.noyau.contexte(session.etablissement_id, Some(session));
        let mut cnx = self.connexion().await?;
        Self::exiger_active(&mut cnx, &contexte).await?;
        let r = reglages(&mut cnx, &contexte).await?;
        let Some(annee) = annee_scolaire::courante(&mut cnx, &contexte).await? else {
            return Ok(ClassesConduite {
                mode: r.mode,
                classes: Vec::new(),
            });
        };
        let niveaux = academique::niveaux(&mut cnx, &contexte).await?;
        let mut periodes = pedagogie::periodes(&mut cnx, annee.id, &contexte).await?;
        periodes.sort_by_key(|p| p.rang);
        let classes = academique::classes(&mut cnx, annee.id, &contexte)
            .await?
            .into_iter()
            .filter(|c| {
                niveaux
                    .iter()
                    .any(|n| n.id == c.niveau_id && cycle_note(n.cycle))
            })
            .map(|c| ClasseConduite {
                periodes: periodes
                    .iter()
                    .filter(|p| p.niveau_id == c.niveau_id)
                    .map(|p| PeriodeConduite {
                        periode_id: p.id,
                        libelle: p.libelle(),
                    })
                    .collect(),
                classe_id: c.id,
                libelle: c.libelle,
            })
            .collect();
        Ok(ClassesConduite {
            mode: r.mode,
            classes,
        })
    }

    /// La classe, sa période et son barème, vérifiés.
    async fn classe_et_periode(
        cnx: &mut Connexion,
        demande: &DemandeConduite,
        contexte: &Contexte,
    ) -> Resultat<(Classe, Periode, i32)> {
        let classe = academique::classe(cnx, demande.classe_id, contexte).await?;
        let niveau = academique::niveau(cnx, classe.niveau_id, contexte).await?;
        let bareme = bareme_du_cycle(niveau.cycle)
            .filter(|_| cycle_note(niveau.cycle))
            .ok_or(regle("conduite.cycle_sans_conduite"))?;
        let periode = pedagogie::periodes(cnx, classe.annee_id, contexte)
            .await?
            .into_iter()
            .find(|p| p.id == demande.periode_id && p.niveau_id == classe.niveau_id)
            .ok_or(regle("evaluation.periode_invalide"))?;
        Ok((classe, periode, bareme))
    }

    /// Les notes de conduite d'une classe pour une période.
    pub async fn lire(
        &self,
        session: &Session,
        demande: &DemandeConduite,
    ) -> Resultat<ConduiteClasse> {
        let contexte = self.noyau.contexte(session.etablissement_id, Some(session));
        let mut cnx = self.connexion().await?;
        Self::exiger_active(&mut cnx, &contexte).await?;
        let (classe, periode, bareme) =
            Self::classe_et_periode(&mut cnx, demande, &contexte).await?;
        let r = reglages(&mut cnx, &contexte).await?;
        drop(cnx);
        // Les élèves qui ont suivi la période, dans l'ordre de la liste de classe.
        let eleves: Vec<(Id<Inscription>, String, String)> =
            ServiceInscriptions::nouveau(self.noyau)
                .lister(session, classe.annee_id)
                .await?
                .into_iter()
                .filter(|l| {
                    l.inscription.classe_id == classe.id
                        && l.inscription.statut == StatutInscription::Validee
                        && !non_suivie(l.inscription.date_inscription, &periode)
                })
                .map(|l| (l.inscription.id, l.eleve, l.matricule))
                .collect();
        let ids: Vec<Id<Inscription>> = eleves.iter().map(|(i, _, _)| *i).collect();
        let mut cnx = self.connexion().await?;
        let calculs = calculer(&mut cnx, &r, &classe, &periode, bareme, &ids, &contexte).await?;
        let lignes = eleves
            .into_iter()
            .zip(calculs)
            .map(
                |((inscription_id, eleve, matricule), (_, decompte, calculee, saisie))| {
                    LigneConduite {
                        note: saisie.as_ref().map(|s| s.note).or(calculee),
                        inscription_id,
                        eleve,
                        matricule,
                        decompte,
                        calculee,
                        saisie,
                    }
                },
            )
            .collect();
        Ok(ConduiteClasse {
            mode: r.mode,
            comptee: r.comptee,
            coefficient: r.coefficient,
            bareme,
            retraits: r.retraits,
            verrouillee: periode.verrouillee,
            lignes,
        })
    }

    /// Vérifie qu'une note peut s'écrire : note de conduite active, année et période ouvertes,
    /// élève de la classe qui a suivi la période.
    async fn exiger_modifiable(
        &self,
        session: &Session,
        demande: &DemandeConduite,
        inscription_id: Id<Inscription>,
    ) -> Resultat<(ModeConduite, i32)> {
        crate::annees::exiger_classe_modifiable(self.noyau, session, demande.classe_id).await?;
        let contexte = self.noyau.contexte(session.etablissement_id, Some(session));
        let mut cnx = self.connexion().await?;
        Self::exiger_active(&mut cnx, &contexte).await?;
        let (classe, periode, bareme) =
            Self::classe_et_periode(&mut cnx, demande, &contexte).await?;
        if periode.verrouillee {
            return Err(regle("periode.verrouillee"));
        }
        let inscription = inscriptions::lire(&mut cnx, inscription_id, &contexte).await?;
        if inscription.classe_id != classe.id
            || inscription.statut != StatutInscription::Validee
            || non_suivie(inscription.date_inscription, &periode)
        {
            return Err(regle("conduite.eleve_hors_classe"));
        }
        let r = reglages(&mut cnx, &contexte).await?;
        Ok((r.mode, bareme))
    }

    /// Saisit la note d'un élève, ou corrige sa note calculée ; une note déjà saisie est
    /// remplacée.
    pub async fn noter(
        &self,
        session: &Session,
        demande: &DemandeNoteConduite,
    ) -> Resultat<ConduiteClasse> {
        let classe_periode = DemandeConduite {
            classe_id: demande.classe_id,
            periode_id: demande.periode_id,
        };
        let (mode, bareme) = self
            .exiger_modifiable(session, &classe_periode, demande.inscription_id)
            .await?;
        let note = lire_note(&demande.note, bareme, &demande.motif, mode).map_err(regle)?;
        let contexte = self.noyau.contexte(session.etablissement_id, Some(session));
        let mut unite = self
            .noyau
            .commencer(contexte.clone(), Durabilite::Ordinaire)
            .await?;
        let avant = conduite::en_vigueur(
            unite.connexion(),
            demande.inscription_id,
            demande.periode_id,
            &contexte,
        )
        .await?;
        if let Some(ancienne) = &avant {
            conduite::annuler(unite.connexion(), ancienne.id, MOTIF_REMPLACEE, &contexte).await?;
            unite.auditer(
                "note_conduite",
                &ancienne.id.vers_chaine(),
                Operation::Modification,
                Some(serde_json::json!({ "annulee": false })),
                Some(serde_json::json!({ "annulee": true, "motif": MOTIF_REMPLACEE })),
                "conduite.noter",
            );
        }
        let nouvelle = NoteConduite {
            id: nouvel_id(),
            etablissement_id: session.etablissement_id,
            inscription_id: demande.inscription_id,
            periode_id: demande.periode_id,
            note,
            motif: demande.motif.trim().to_owned(),
            saisie_par: session.utilisateur_id,
            annulee: false,
            motif_annulation: String::new(),
        };
        conduite::inserer(unite.connexion(), &nouvelle, &contexte).await?;
        let id = nouvelle.id.vers_chaine();
        unite.auditer(
            "note_conduite",
            &id,
            Operation::Creation,
            avant.map(serde_json::to_value).transpose()?,
            Some(serde_json::to_value(&nouvelle)?),
            "conduite.noter",
        );
        unite.emettre(
            TypeEvenement::NoteConduiteEnregistree,
            "note_conduite",
            &id,
            serde_json::json!({ "inscription_id": nouvelle.inscription_id,
                "periode_id": nouvelle.periode_id, "note": nouvelle.note }),
        );
        self.noyau.terminer(unite).await?;
        self.lire(session, &classe_periode).await
    }

    /// Retire la note saisie d'un élève, avec un motif : en mode calculé, il retrouve sa note
    /// calculée.
    pub async fn retirer(
        &self,
        session: &Session,
        demande: &DemandeRetraitConduite,
    ) -> Resultat<ConduiteClasse> {
        let motif = demande.motif.trim();
        if motif.is_empty() {
            return Err(regle("conduite.motif_obligatoire"));
        }
        let classe_periode = DemandeConduite {
            classe_id: demande.classe_id,
            periode_id: demande.periode_id,
        };
        self.exiger_modifiable(session, &classe_periode, demande.inscription_id)
            .await?;
        let contexte = self.noyau.contexte(session.etablissement_id, Some(session));
        let mut unite = self
            .noyau
            .commencer(contexte.clone(), Durabilite::Ordinaire)
            .await?;
        let note = conduite::en_vigueur(
            unite.connexion(),
            demande.inscription_id,
            demande.periode_id,
            &contexte,
        )
        .await?
        .ok_or(regle("conduite.aucune_note"))?;
        conduite::annuler(unite.connexion(), note.id, motif, &contexte).await?;
        let id = note.id.vers_chaine();
        unite.auditer(
            "note_conduite",
            &id,
            Operation::Modification,
            Some(serde_json::json!({ "annulee": false })),
            Some(serde_json::json!({ "annulee": true, "motif": motif })),
            "conduite.retirer",
        );
        unite.emettre(
            TypeEvenement::NoteConduiteRetiree,
            "note_conduite",
            &id,
            serde_json::json!({ "inscription_id": note.inscription_id,
                "periode_id": note.periode_id, "motif": motif }),
        );
        self.noyau.terminer(unite).await?;
        self.lire(session, &classe_periode).await
    }
}
