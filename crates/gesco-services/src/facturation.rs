//! Facture et échéancier d'une inscription (SPEC_PHASE_3 §4). La facture naît dans la
//! transaction qui valide l'inscription (R-17) ; la scolarité ne transmet à la finance que le
//! contexte tarifaire (§9.1 du blueprint), et la finance ne lit aucune table de la scolarité.

use gesco_core::commun::{DateCivile, Id, Montant};
use gesco_core::core::{
    AnneeScolaire, MachineAEtats, Operation, TypeDocumentNumerote, TypeEvenement,
};
use gesco_core::finance::facture::{ArriveeEnCours, Bornes, LigneCalculee, calculer};
use gesco_core::finance::tarif::JOURS_ECHEANCE;
use gesco_core::finance::{
    ContexteTarifaire, Echeance, Exigibilite, Facture, LigneFacture, LigneTarifaire, PlanPaiement,
    StatutEcheance, StatutFacture,
};
use gesco_core::scolarite::personne::millesime;
use gesco_core::scolarite::{Eleve, Inscription, StatutInscription};
use gesco_db::depots::{annee_scolaire, factures, parametre, tarifs};
use gesco_db::unite::Durabilite;
use gesco_db::{Contexte, UniteDeTravail, nouvel_id};
use serde::{Deserialize, Serialize};

use crate::effets::Effet;
use crate::erreur::{ErreurService, Resultat};
use crate::inscriptions::{ServiceInscriptions, jour};
use crate::noyau::Noyau;
use crate::numerotation;
use crate::session::Session;

fn regle(cle: &'static str) -> ErreurService {
    ErreurService::Regle { cle }
}

/// Paramètre d'établissement de Q-26.
pub const PARAMETRE_ARRIVEE: &str = "facturation.arrivee_en_cours";

/// Choix du parent à la validation : plan de paiement et frais facultatifs cochés.
#[derive(Debug, Clone, Default, Deserialize, specta::Type)]
pub struct ChoixFrais {
    /// Plan choisi ; absent, le plan par défaut de la grille.
    #[serde(default)]
    pub plan_paiement_id: Option<Id<PlanPaiement>>,
    /// Frais facultatifs cochés (cantine, transport…).
    #[serde(default)]
    pub frais_optionnels: Vec<Id<LigneTarifaire>>,
    /// Règle d'arrivée en cours d'année pour cet élève ; absente, celle de l'école.
    #[serde(default)]
    pub arrivee: Option<ArriveeEnCours>,
}

/// Échéance due, avec la part des remises accordées et ce qui reste à payer.
#[derive(Debug, Clone, Serialize, specta::Type)]
pub struct EcheanceAffichee {
    /// Échéance.
    pub echeance: Echeance,
    /// Part des remises accordées.
    pub remise: Montant,
    /// Dû après remise.
    pub net: Montant,
    /// Versements imputés.
    pub paye: Montant,
    /// Reste à payer : dû après remise, moins le payé.
    pub reste: Montant,
}

fn calcul(r: Result<Montant, gesco_core::commun::ErreurMontant>) -> Resultat<Montant> {
    r.map_err(|_| regle("facture.calcul_impossible"))
}

/// État des échéances dues d'une facture, par date : montant, remises, payé, reste. Tout se
/// recalcule depuis les échéances, les parts de remise et les imputations (R-20).
pub(crate) async fn etat_echeances(
    cnx: &mut gesco_db::Connexion,
    facture_id: Id<Facture>,
    contexte: &Contexte,
) -> Resultat<Vec<EcheanceAffichee>> {
    let parts = gesco_db::depots::remises::parts_de_la_facture(cnx, facture_id, contexte).await?;
    let imputations =
        gesco_db::depots::versements::imputations_de_la_facture(cnx, facture_id, contexte).await?;
    let mut etats = Vec::new();
    for echeance in factures::echeances(cnx, facture_id, contexte)
        .await?
        .into_iter()
        .filter(|e| e.statut == StatutEcheance::Due)
    {
        let remise = calcul(
            parts
                .iter()
                .filter(|(_, e, _)| *e == echeance.id)
                .try_fold(Montant::ZERO, |s, (_, _, m)| s.ajouter(*m)),
        )?;
        let paye = calcul(
            imputations
                .iter()
                .filter(|(_, e, _)| *e == echeance.id)
                .try_fold(Montant::ZERO, |s, (_, _, m)| s.ajouter(*m)),
        )?;
        let net = calcul(echeance.montant.soustraire(remise))?;
        etats.push(EcheanceAffichee {
            reste: calcul(net.soustraire(paye))?,
            net,
            paye,
            remise,
            echeance,
        });
    }
    Ok(etats)
}

/// Recalcule le total et l'état d'une facture d'après ses échéances dues et ses imputations
/// (R-20), et l'écrit s'il a changé. Rend la facture à jour.
pub(crate) async fn recalculer_dans(
    unite: &mut UniteDeTravail<Effet>,
    contexte: &Contexte,
    facture: &Facture,
) -> Resultat<Facture> {
    let etats = etat_echeances(unite.connexion(), facture.id, contexte).await?;
    let somme = |f: fn(&EcheanceAffichee) -> Montant| {
        calcul(etats.iter().try_fold(Montant::ZERO, |s, e| s.ajouter(f(e))))
    };
    let total = somme(|e| e.echeance.montant)?;
    let paye = somme(|e| e.paye)?;
    let reste = somme(|e| e.reste)?;
    let statut = gesco_core::finance::versement::statut_d_apres(paye, reste);
    if statut == facture.statut && total == facture.total {
        return Ok(facture.clone());
    }
    let apres = Facture {
        statut: if statut == facture.statut {
            statut
        } else {
            facture.statut.transiter(statut)?
        },
        total,
        ..facture.clone()
    };
    factures::ecrire_facture(unite.connexion(), &apres, contexte).await?;
    Ok(apres)
}

/// Facture avec ses lignes, ses échéances dues et ce qui reste à payer.
#[derive(Debug, Clone, Serialize, specta::Type)]
pub struct FactureAffichee {
    /// Facture.
    pub facture: Facture,
    /// Libellé du plan choisi.
    pub plan: String,
    /// Lignes.
    pub lignes: Vec<LigneFacture>,
    /// Échéances dues, par date, nettes de remise.
    pub echeances: Vec<EcheanceAffichee>,
    /// Total des remises accordées.
    pub remise: Montant,
    /// Versements imputés.
    pub paye: Montant,
    /// Reste à payer : total, moins les remises, moins le payé.
    pub reste: Montant,
}

/// Plan proposé au changement.
#[derive(Debug, Clone, Serialize, specta::Type)]
pub struct PlanPropose {
    /// Plan.
    pub id: Id<PlanPaiement>,
    /// Libellé.
    pub libelle: String,
}

/// Situation financière d'une inscription (SPEC_PHASE_3 §4.4).
#[derive(Debug, Clone, Serialize, specta::Type)]
pub struct SituationInscription {
    /// Inscription.
    pub inscription: Inscription,
    /// Année.
    pub annee: String,
    /// Classe.
    pub classe: String,
    /// Facture non annulée, s'il y en a une.
    pub facture: Option<FactureAffichee>,
    /// Plans de la grille de la facture, pour en changer.
    pub plans: Vec<PlanPropose>,
    /// Remises de l'inscription, toutes.
    pub remises: Vec<gesco_core::finance::Remise>,
    /// Versements de l'inscription, annulés compris, du plus ancien au plus récent.
    pub versements: Vec<gesco_core::finance::Versement>,
}

/// Changement de plan d'une inscription.
#[derive(Debug, Deserialize, specta::Type)]
pub struct DemandeChangementPlan {
    /// Inscription.
    pub inscription_id: Id<Inscription>,
    /// Nouveau plan, de la grille de la facture.
    pub plan_paiement_id: Id<PlanPaiement>,
}

/// Jour d'échéance propre à un élève : un parent payé le 10 paie le 10.
#[derive(Debug, Clone, Deserialize, specta::Type)]
pub struct DemandeJourEcheance {
    /// Inscription.
    pub inscription_id: Id<Inscription>,
    /// Jour du mois, du 1er au 28.
    pub jour: u8,
}

/// Inscription validée sans facture, et ce que le rattrapage lui facturerait.
#[derive(Debug, Clone, Serialize, specta::Type)]
pub struct LigneRattrapage {
    /// Inscription.
    pub inscription_id: Id<Inscription>,
    /// Élève.
    pub eleve: String,
    /// Classe.
    pub classe: String,
    /// Total facturé avec le plan par défaut et sans frais facultatif ; absent en cas d'erreur.
    pub total: Option<Montant>,
    /// Clé de l'erreur qui empêche de facturer.
    pub erreur: Option<String>,
}

/// Résultat du rattrapage.
#[derive(Debug, Clone, Serialize, specta::Type)]
pub struct RapportRattrapage {
    /// Factures créées.
    pub facturees: u32,
    /// Inscriptions laissées sans facture (niveau absent de la grille).
    pub ignorees: u32,
}

struct Calcul {
    lignes: Vec<LigneCalculee>,
    plan: PlanPaiement,
    grille_id: Id<gesco_core::finance::Grille>,
}

/// Calcule la facture d'une inscription dans la transaction ouverte.
async fn calculer_dans(
    unite: &mut UniteDeTravail<Effet>,
    contexte: &Contexte,
    tarif: &ContexteTarifaire,
    choix: &ChoixFrais,
) -> Resultat<(Calcul, AnneeScolaire)> {
    let cnx = unite.connexion();
    let annee = annee_scolaire::lire(cnx, tarif.annee_id, contexte).await?;
    let grille = tarifs::grille_en_vigueur(cnx, tarif.annee_id, contexte)
        .await?
        .ok_or(regle("facture.grille_absente"))?;
    let contenu = tarifs::contenu(cnx, grille.id, contexte).await?;
    let plan = match choix.plan_paiement_id {
        Some(id) => contenu.plans.iter().find(|p| p.id == id),
        None => contenu.plans.iter().find(|p| p.par_defaut),
    }
    .cloned()
    .ok_or(regle("facture.plan_inconnu"))?;
    let arrivee = match choix.arrivee {
        Some(regle_eleve) => regle_eleve,
        None => ArriveeEnCours::depuis_nom(
            &parametre::texte(
                cnx,
                PARAMETRE_ARRIVEE,
                ArriveeEnCours::DEFAUT.nom(),
                contexte,
            )
            .await?,
        )
        .unwrap_or(ArriveeEnCours::DEFAUT),
    };
    let multiple = parametre::entier(cnx, "montant.multiple_arrondi", 5, contexte).await?;
    let bornes = Bornes {
        debut: jour(annee.debut)?,
        fin: jour(annee.fin)?,
        arrivee,
        multiple,
    };
    let lignes = calculer(
        &contenu.lignes,
        tarif,
        &choix.frais_optionnels,
        &plan,
        &bornes,
    )
    .map_err(regle)?;
    Ok((
        Calcul {
            lignes,
            plan,
            grille_id: grille.id,
        },
        annee,
    ))
}

fn total(lignes: &[LigneCalculee]) -> Resultat<Montant> {
    let mut somme = Montant::ZERO;
    for l in lignes {
        somme = somme
            .ajouter(
                l.montant()
                    .map_err(|_| regle("facture.calcul_impossible"))?,
            )
            .map_err(|_| regle("facture.calcul_impossible"))?;
    }
    Ok(somme)
}

/// Crée la facture et l'échéancier d'une inscription, dans la transaction ouverte (R-17).
pub(crate) async fn facturer_dans(
    unite: &mut UniteDeTravail<Effet>,
    contexte: &Contexte,
    tarif: &ContexteTarifaire,
    choix: &ChoixFrais,
) -> Resultat<Facture> {
    if factures::facture_vivante(unite.connexion(), tarif.inscription_id, contexte)
        .await?
        .is_some()
    {
        return Err(regle("facture.deja_emise"));
    }
    let (calcul, annee) = calculer_dans(unite, contexte, tarif, choix).await?;
    let exercice = millesime(&annee.libelle)
        .map(str::to_owned)
        .unwrap_or_else(|| tarif.date_inscription.annee().to_string());
    let numero = numerotation::attribuer(unite, TypeDocumentNumerote::Facture, &exercice).await?;
    let facture = Facture {
        id: nouvel_id(),
        etablissement_id: contexte.etablissement_id,
        inscription_id: tarif.inscription_id,
        annee_id: tarif.annee_id,
        grille_id: calcul.grille_id,
        plan_paiement_id: calcul.plan.id,
        numero: numero.numero,
        date_emission: jour(contexte.maintenant)?,
        statut: StatutFacture::Emise,
        total: total(&calcul.lignes)?,
        motif_annulation: String::new(),
    };
    factures::inserer_facture(unite.connexion(), &facture, contexte).await?;
    let mut rang = 0;
    let mut enregistrees = Vec::new();
    for (ordre, calculee) in calcul.lignes.iter().enumerate() {
        let ligne = LigneFacture {
            id: nouvel_id(),
            ligne_tarifaire_id: calculee.ligne.id,
            type_frais: calculee.ligne.type_frais,
            libelle: calculee.ligne.libelle.clone(),
            montant: calculee
                .montant()
                .map_err(|_| regle("facture.calcul_impossible"))?,
        };
        factures::inserer_ligne(unite.connexion(), facture.id, ordre, &ligne, contexte).await?;
        for e in &calculee.echeances {
            rang += 1;
            let echeance = Echeance {
                id: nouvel_id(),
                ligne_facture_id: ligne.id,
                libelle: e.libelle.clone(),
                date: e.date,
                montant: e.montant,
                statut: StatutEcheance::Due,
            };
            factures::inserer_echeance(unite.connexion(), facture.id, rang, &echeance, contexte)
                .await?;
            enregistrees.push(echeance);
        }
    }
    crate::remises::reappliquer_dans(unite, contexte, &facture).await?;
    let id = facture.id.vers_chaine();
    unite.auditer(
        "facture",
        &id,
        Operation::Creation,
        None,
        Some(serde_json::json!({ "facture": facture, "echeances": enregistrees })),
        "facturation.facturer",
    );
    unite.emettre(
        TypeEvenement::FactureEmise,
        "facture",
        &id,
        serde_json::json!({
            "inscription_id": facture.inscription_id,
            "numero": facture.numero,
            "total": facture.total,
        }),
    );
    Ok(facture)
}

/// Annule la facture d'une inscription abandonnée, dans la transaction de l'abandon
/// (SPEC_PHASE_3 §4.3) : facture annulée, échéances contre-passées. Les versements n'existant
/// pas avant l'étape 6, rien n'est encore acquis.
pub(crate) async fn annuler_pour_abandon_dans(
    unite: &mut UniteDeTravail<Effet>,
    contexte: &Contexte,
    inscription_id: Id<Inscription>,
    motif: &str,
) -> Resultat<()> {
    let Some(avant) =
        factures::facture_vivante(unite.connexion(), inscription_id, contexte).await?
    else {
        return Ok(());
    };
    let etats = etat_echeances(unite.connexion(), avant.id, contexte).await?;
    if etats.iter().any(|e| !e.paye.est_nul()) {
        // Des versements existent : le payé reste acquis, les échéances à venir et sans
        // paiement s'annulent par contre-passation (SPEC_PHASE_3 §4.3, comme Q-34).
        let aujourd_hui = jour(contexte.maintenant)?;
        for e in &etats {
            if e.paye.est_nul() && e.echeance.date > aujourd_hui {
                factures::annuler_echeance(unite.connexion(), e.echeance.id, contexte).await?;
            }
        }
        let apres = recalculer_dans(unite, contexte, &avant).await?;
        unite.auditer(
            "facture",
            &avant.id.vers_chaine(),
            Operation::Modification,
            Some(serde_json::to_value(&avant)?),
            Some(serde_json::to_value(&apres)?),
            "facturation.abandon_avec_paiements",
        );
        return Ok(());
    }
    let apres = Facture {
        statut: avant.statut.transiter(StatutFacture::Annulee)?,
        motif_annulation: motif.to_owned(),
        ..avant.clone()
    };
    for e in factures::echeances(unite.connexion(), avant.id, contexte).await? {
        if e.statut == StatutEcheance::Due {
            factures::annuler_echeance(unite.connexion(), e.id, contexte).await?;
        }
    }
    factures::ecrire_facture(unite.connexion(), &apres, contexte).await?;
    let id = apres.id.vers_chaine();
    unite.auditer(
        "facture",
        &id,
        Operation::Annulation,
        Some(serde_json::to_value(&avant)?),
        Some(serde_json::to_value(&apres)?),
        "facturation.annuler_pour_abandon",
    );
    unite.emettre(
        TypeEvenement::FactureAnnulee,
        "facture",
        &id,
        serde_json::json!({ "inscription_id": inscription_id, "motif": motif }),
    );
    Ok(())
}

/// Départ d'un élève (transfert, exclusion définitive ; SPEC_PHASE_4 §9, Q-34) : les échéances
/// sans paiement dont la date suit le départ s'annulent par contre-passation ; les échues restent
/// dues. Rend les échéances annulées, pour les rétablir si l'élève est réintégré.
pub(crate) async fn annuler_echeances_a_venir_dans(
    unite: &mut UniteDeTravail<Effet>,
    contexte: &Contexte,
    inscription_id: Id<Inscription>,
    depart: DateCivile,
    origine: &str,
) -> Resultat<Vec<Id<Echeance>>> {
    let Some(avant) =
        factures::facture_vivante(unite.connexion(), inscription_id, contexte).await?
    else {
        return Ok(Vec::new());
    };
    let mut annulees = Vec::new();
    for e in etat_echeances(unite.connexion(), avant.id, contexte).await? {
        if e.paye.est_nul() && e.echeance.date > depart {
            factures::annuler_echeance(unite.connexion(), e.echeance.id, contexte).await?;
            annulees.push(e.echeance.id);
        }
    }
    if annulees.is_empty() {
        return Ok(annulees);
    }
    let apres = recalculer_dans(unite, contexte, &avant).await?;
    unite.auditer(
        "facture",
        &avant.id.vers_chaine(),
        Operation::Modification,
        Some(serde_json::json!({ "facture": avant })),
        Some(serde_json::json!({ "facture": apres, "echeances_annulees": annulees })),
        origine,
    );
    Ok(annulees)
}

/// Rétablit, à la réintégration d'un élève exclu, les échéances que son exclusion avait annulées :
/// chacune revient en échéance due, sous un nouvel identifiant, l'ancienne restant annulée (R-14).
pub(crate) async fn retablir_echeances_dans(
    unite: &mut UniteDeTravail<Effet>,
    contexte: &Contexte,
    inscription_id: Id<Inscription>,
    annulees: &[Id<Echeance>],
    origine: &str,
) -> Resultat<()> {
    if annulees.is_empty() {
        return Ok(());
    }
    let Some(facture) =
        factures::facture_vivante(unite.connexion(), inscription_id, contexte).await?
    else {
        return Ok(());
    };
    let toutes = factures::echeances(unite.connexion(), facture.id, contexte).await?;
    let mut rang = factures::nombre_d_echeances(unite.connexion(), facture.id, contexte).await?;
    let mut retablies = Vec::new();
    for e in toutes.iter().filter(|e| annulees.contains(&e.id)) {
        rang += 1;
        let copie = Echeance {
            id: nouvel_id(),
            statut: StatutEcheance::Due,
            ..e.clone()
        };
        factures::inserer_echeance(unite.connexion(), facture.id, rang, &copie, contexte).await?;
        retablies.push(copie);
    }
    crate::remises::reappliquer_dans(unite, contexte, &facture).await?;
    let apres = recalculer_dans(unite, contexte, &facture).await?;
    unite.auditer(
        "facture",
        &facture.id.vers_chaine(),
        Operation::Modification,
        Some(serde_json::json!({ "facture": facture, "echeances_annulees": annulees })),
        Some(serde_json::json!({ "facture": apres, "echeances_retablies": retablies })),
        origine,
    );
    Ok(())
}

/// Reste dû par un élève sur ses échéances échues à une date (Q-33).
pub(crate) async fn reste_echu(
    cnx: &mut gesco_db::Connexion,
    inscription_id: Id<Inscription>,
    au: DateCivile,
    contexte: &Contexte,
) -> Resultat<Montant> {
    let Some(facture) = factures::facture_vivante(cnx, inscription_id, contexte).await? else {
        return Ok(Montant::ZERO);
    };
    let etats = etat_echeances(cnx, facture.id, contexte).await?;
    calcul(
        etats
            .iter()
            .filter(|e| e.echeance.date <= au)
            .try_fold(Montant::ZERO, |s, e| s.ajouter(e.reste)),
    )
}

/// Service de facturation.
pub struct ServiceFacturation<'n> {
    noyau: &'n Noyau,
}

impl<'n> ServiceFacturation<'n> {
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

    async fn affichee(&self, session: &Session, facture: Facture) -> Resultat<FactureAffichee> {
        let contexte = self.noyau.contexte(session.etablissement_id, Some(session));
        let mut cnx = self.connexion().await?;
        let lignes = factures::lignes(&mut cnx, facture.id, &contexte).await?;
        let echeances = etat_echeances(&mut cnx, facture.id, &contexte).await?;
        let somme = |f: fn(&EcheanceAffichee) -> Montant| {
            calcul(
                echeances
                    .iter()
                    .try_fold(Montant::ZERO, |s, e| s.ajouter(f(e))),
            )
        };
        let remise_totale = somme(|e| e.remise)?;
        let paye = somme(|e| e.paye)?;
        let reste = somme(|e| e.reste)?;
        let plan = tarifs::contenu(&mut cnx, facture.grille_id, &contexte)
            .await?
            .plans
            .into_iter()
            .find(|p| p.id == facture.plan_paiement_id)
            .map(|p| p.libelle)
            .unwrap_or_default();
        Ok(FactureAffichee {
            plan,
            lignes,
            echeances,
            paye,
            reste,
            remise: remise_totale,
            facture,
        })
    }

    /// Situation financière d'un élève : une entrée par inscription, la plus récente d'abord.
    pub async fn situation(
        &self,
        session: &Session,
        eleve_id: Id<Eleve>,
    ) -> Resultat<Vec<SituationInscription>> {
        let contexte = self.noyau.contexte(session.etablissement_id, Some(session));
        let mut situations = Vec::new();
        for affichee in ServiceInscriptions::nouveau(self.noyau)
            .de_l_eleve(session, eleve_id)
            .await?
        {
            let facture = {
                let mut cnx = self.connexion().await?;
                factures::facture_vivante(&mut cnx, affichee.inscription.id, &contexte).await?
            };
            let (facture, plans) = match facture {
                None => (None, Vec::new()),
                Some(f) => {
                    let plans = {
                        let mut cnx = self.connexion().await?;
                        tarifs::contenu(&mut cnx, f.grille_id, &contexte)
                            .await?
                            .plans
                            .into_iter()
                            .map(|p| PlanPropose {
                                id: p.id,
                                libelle: p.libelle,
                            })
                            .collect()
                    };
                    (Some(self.affichee(session, f).await?), plans)
                }
            };
            let remises = crate::remises::ServiceRemises::nouveau(self.noyau)
                .de_l_inscription(session, affichee.inscription.id)
                .await?;
            let versements = {
                let mut cnx = self.connexion().await?;
                gesco_db::depots::versements::de_l_inscription(
                    &mut cnx,
                    affichee.inscription.id,
                    &contexte,
                )
                .await?
            };
            situations.push(SituationInscription {
                versements,
                remises,
                inscription: affichee.inscription,
                annee: affichee.annee,
                classe: affichee.classe,
                facture,
                plans,
            });
        }
        Ok(situations)
    }

    /// Change le plan de paiement d'une inscription (§3.1 bis) : les échéances payées selon le
    /// plan sont contre-passées et recréées selon le nouveau, dans une seule transaction. Refusé
    /// dès qu'un versement est imputé sur elles (étape 6).
    pub async fn changer_plan(
        &self,
        session: &Session,
        demande: &DemandeChangementPlan,
    ) -> Resultat<FactureAffichee> {
        let contexte = self.noyau.contexte(session.etablissement_id, Some(session));
        let tarif = ServiceInscriptions::nouveau(self.noyau)
            .contexte_tarifaire(session, demande.inscription_id)
            .await?;
        let (avant, lignes_avant, echeances_avant, contenu) = {
            let mut cnx = self.connexion().await?;
            let avant = factures::facture_vivante(&mut cnx, demande.inscription_id, &contexte)
                .await?
                .ok_or(regle("facture.absente"))?;
            let lignes = factures::lignes(&mut cnx, avant.id, &contexte).await?;
            let echeances = factures::echeances(&mut cnx, avant.id, &contexte).await?;
            let contenu = tarifs::contenu(&mut cnx, avant.grille_id, &contexte).await?;
            (avant, lignes, echeances, contenu)
        };
        if avant.plan_paiement_id == demande.plan_paiement_id {
            return self.affichee(session, avant).await;
        }
        let plan = contenu
            .plans
            .iter()
            .find(|p| p.id == demande.plan_paiement_id)
            .cloned()
            .ok_or(regle("facture.plan_inconnu"))?;
        {
            let lignes_selon_plan: Vec<_> = lignes_avant
                .iter()
                .filter(|l| {
                    contenu.lignes.iter().any(|g| {
                        g.id == l.ligne_tarifaire_id && g.exigibilite == Exigibilite::SelonPlan
                    })
                })
                .map(|l| l.id)
                .collect();
            let mut cnx = self.connexion().await?;
            if etat_echeances(&mut cnx, avant.id, &contexte)
                .await?
                .iter()
                .any(|e| {
                    lignes_selon_plan.contains(&e.echeance.ligne_facture_id) && !e.paye.est_nul()
                })
            {
                return Err(regle("echeancier.plan_verrouille"));
            }
        }
        let options: Vec<Id<LigneTarifaire>> = lignes_avant
            .iter()
            .filter(|l| {
                contenu
                    .lignes
                    .iter()
                    .any(|g| g.id == l.ligne_tarifaire_id && !g.obligatoire)
            })
            .map(|l| l.ligne_tarifaire_id)
            .collect();
        let selon_plan: Vec<Id<LigneTarifaire>> = contenu
            .lignes
            .iter()
            .filter(|g| g.exigibilite == Exigibilite::SelonPlan)
            .map(|g| g.id)
            .collect();
        let mut unite = self
            .noyau
            .commencer(contexte.clone(), Durabilite::Critique)
            .await?;
        let annee = annee_scolaire::lire(unite.connexion(), tarif.annee_id, &contexte).await?;
        let arrivee = ArriveeEnCours::depuis_nom(
            &parametre::texte(
                unite.connexion(),
                PARAMETRE_ARRIVEE,
                ArriveeEnCours::DEFAUT.nom(),
                &contexte,
            )
            .await?,
        )
        .unwrap_or(ArriveeEnCours::DEFAUT);
        let multiple =
            parametre::entier(unite.connexion(), "montant.multiple_arrondi", 5, &contexte).await?;
        let bornes = Bornes {
            debut: jour(annee.debut)?,
            fin: jour(annee.fin)?,
            arrivee,
            multiple,
        };
        let calcul = calculer(&contenu.lignes, &tarif, &options, &plan, &bornes).map_err(regle)?;
        let mut rang = factures::nombre_d_echeances(unite.connexion(), avant.id, &contexte).await?;
        let mut total = Montant::ZERO;
        for ligne in &lignes_avant {
            let echeances_ligne: Vec<&Echeance> = echeances_avant
                .iter()
                .filter(|e| e.ligne_facture_id == ligne.id && e.statut == StatutEcheance::Due)
                .collect();
            let montant = if selon_plan.contains(&ligne.ligne_tarifaire_id) {
                for e in &echeances_ligne {
                    factures::annuler_echeance(unite.connexion(), e.id, &contexte).await?;
                }
                let nouvelles = calcul
                    .iter()
                    .find(|c| c.ligne.id == ligne.ligne_tarifaire_id)
                    .map(|c| c.echeances.clone())
                    .unwrap_or_default();
                let mut somme = Montant::ZERO;
                for e in nouvelles {
                    rang += 1;
                    let echeance = Echeance {
                        id: nouvel_id(),
                        ligne_facture_id: ligne.id,
                        libelle: e.libelle,
                        date: e.date,
                        montant: e.montant,
                        statut: StatutEcheance::Due,
                    };
                    factures::inserer_echeance(
                        unite.connexion(),
                        avant.id,
                        rang,
                        &echeance,
                        &contexte,
                    )
                    .await?;
                    somme = somme
                        .ajouter(e.montant)
                        .map_err(|_| regle("facture.calcul_impossible"))?;
                }
                factures::ecrire_montant_ligne(unite.connexion(), ligne.id, somme, &contexte)
                    .await?;
                somme
            } else {
                ligne.montant
            };
            total = total
                .ajouter(montant)
                .map_err(|_| regle("facture.calcul_impossible"))?;
        }
        let apres = Facture {
            plan_paiement_id: plan.id,
            total,
            ..avant.clone()
        };
        factures::ecrire_facture(unite.connexion(), &apres, &contexte).await?;
        crate::remises::reappliquer_dans(&mut unite, &contexte, &apres).await?;
        let id = apres.id.vers_chaine();
        unite.auditer(
            "facture",
            &id,
            Operation::Modification,
            Some(serde_json::to_value(&avant)?),
            Some(serde_json::to_value(&apres)?),
            "facturation.changer_plan",
        );
        unite.emettre(
            TypeEvenement::PlanPaiementChange,
            "facture",
            &id,
            serde_json::json!({
                "inscription_id": apres.inscription_id,
                "plan_paiement_id": apres.plan_paiement_id,
                "total": apres.total,
            }),
        );
        self.noyau.terminer(unite).await?;
        self.affichee(session, apres).await
    }

    /// Met au jour choisi les échéances de l'élève qui suivent le plan et que rien n'a encore
    /// payé, chacune dans son mois ; une échéance déjà payée, même en partie, garde sa date.
    /// Chaque échéance décalée est contre-passée et recréée, et les remises se réappliquent.
    pub async fn changer_jour(
        &self,
        session: &Session,
        demande: &DemandeJourEcheance,
    ) -> Resultat<FactureAffichee> {
        if !JOURS_ECHEANCE.contains(&demande.jour) {
            return Err(regle("echeancier.jour_invalide"));
        }
        let contexte = self.noyau.contexte(session.etablissement_id, Some(session));
        let (facture, lignes, etats, contenu) = {
            let mut cnx = self.connexion().await?;
            let facture = factures::facture_vivante(&mut cnx, demande.inscription_id, &contexte)
                .await?
                .ok_or(regle("facture.absente"))?;
            let lignes = factures::lignes(&mut cnx, facture.id, &contexte).await?;
            let etats = etat_echeances(&mut cnx, facture.id, &contexte).await?;
            let contenu = tarifs::contenu(&mut cnx, facture.grille_id, &contexte).await?;
            (facture, lignes, etats, contenu)
        };
        let selon_plan: Vec<Id<LigneFacture>> = lignes
            .iter()
            .filter(|l| {
                contenu.lignes.iter().any(|g| {
                    g.id == l.ligne_tarifaire_id && g.exigibilite == Exigibilite::SelonPlan
                })
            })
            .map(|l| l.id)
            .collect();
        let a_decaler: Vec<(Echeance, DateCivile)> = etats
            .into_iter()
            .filter(|e| {
                e.echeance.statut == StatutEcheance::Due
                    && e.paye.est_nul()
                    && selon_plan.contains(&e.echeance.ligne_facture_id)
                    && e.echeance.date.jour() != demande.jour
            })
            .map(|e| {
                let date = DateCivile::nouvelle(
                    e.echeance.date.annee(),
                    e.echeance.date.mois(),
                    demande.jour,
                )
                .ok_or(regle("echeancier.jour_invalide"))?;
                Ok((e.echeance, date))
            })
            .collect::<Resultat<_>>()?;
        if a_decaler.is_empty() {
            return self.affichee(session, facture).await;
        }
        let mut unite = self
            .noyau
            .commencer(contexte.clone(), Durabilite::Critique)
            .await?;
        let mut rang =
            factures::nombre_d_echeances(unite.connexion(), facture.id, &contexte).await?;
        let mut decalees = Vec::new();
        for (ancienne, date) in a_decaler {
            factures::annuler_echeance(unite.connexion(), ancienne.id, &contexte).await?;
            rang += 1;
            let nouvelle = Echeance {
                id: nouvel_id(),
                date,
                ..ancienne.clone()
            };
            factures::inserer_echeance(unite.connexion(), facture.id, rang, &nouvelle, &contexte)
                .await?;
            decalees.push(serde_json::json!({
                "libelle": ancienne.libelle, "avant": ancienne.date, "apres": date,
            }));
        }
        crate::remises::reappliquer_dans(&mut unite, &contexte, &facture).await?;
        let id = facture.id.vers_chaine();
        unite.auditer(
            "facture",
            &id,
            Operation::Modification,
            None,
            Some(serde_json::json!({ "jour": demande.jour, "echeances": decalees })),
            "facturation.changer_jour",
        );
        unite.emettre(
            TypeEvenement::EcheancierDecale,
            "facture",
            &id,
            serde_json::json!({
                "inscription_id": facture.inscription_id,
                "jour": demande.jour,
                "echeances": decalees.len(),
            }),
        );
        self.noyau.terminer(unite).await?;
        self.affichee(session, facture).await
    }

    /// Inscriptions validées de l'année courante qui n'ont pas de facture (Q-50).
    async fn sans_facture(
        &self,
        session: &Session,
    ) -> Resultat<Vec<(gesco_core::scolarite::Inscription, String, String)>> {
        let contexte = self.noyau.contexte(session.etablissement_id, Some(session));
        let Some(annee) = ({
            let mut cnx = self.connexion().await?;
            annee_scolaire::courante(&mut cnx, &contexte).await?
        }) else {
            return Ok(Vec::new());
        };
        let facturees = {
            let mut cnx = self.connexion().await?;
            factures::inscriptions_facturees(&mut cnx, annee.id, &contexte).await?
        };
        Ok(ServiceInscriptions::nouveau(self.noyau)
            .lister(session, annee.id)
            .await?
            .into_iter()
            .filter(|l| {
                l.inscription.statut == StatutInscription::Validee
                    && !facturees.contains(&l.inscription.id)
            })
            .map(|l| (l.inscription, l.eleve, l.classe))
            .collect())
    }

    /// Vrai s'il reste des inscriptions validées sans facture : avertissement de la direction.
    pub async fn rattrapage_en_attente(&self, session: &Session) -> Resultat<bool> {
        Ok(!self.sans_facture(session).await?.is_empty())
    }

    /// Ce que le rattrapage facturerait, sans rien écrire : plan par défaut, aucun frais
    /// facultatif, règle d'arrivée de l'école.
    pub async fn rattrapage_apercu(&self, session: &Session) -> Resultat<Vec<LigneRattrapage>> {
        let contexte = self.noyau.contexte(session.etablissement_id, Some(session));
        let mut lignes = Vec::new();
        for (inscription, eleve, classe) in self.sans_facture(session).await? {
            let tarif = ServiceInscriptions::nouveau(self.noyau)
                .contexte_tarifaire(session, inscription.id)
                .await?;
            // Calcul dans une transaction jamais validée : rien n'est écrit.
            let mut unite = self
                .noyau
                .commencer(contexte.clone(), Durabilite::Ordinaire)
                .await?;
            let calcul = calculer_dans(&mut unite, &contexte, &tarif, &ChoixFrais::default()).await;
            unite.annuler().await?;
            let (total, erreur) = match calcul {
                Ok((c, _)) => (Some(total(&c.lignes)?), None),
                Err(ErreurService::Regle { cle }) => (None, Some(cle.to_owned())),
                Err(autre) => return Err(autre),
            };
            lignes.push(LigneRattrapage {
                inscription_id: inscription.id,
                eleve,
                classe,
                total,
                erreur,
            });
        }
        Ok(lignes)
    }

    /// Facture en une transaction toutes les inscriptions validées qui peuvent l'être ; la
    /// sauvegarde est prise par l'appelant avant (R-48).
    pub async fn rattraper(&self, session: &Session) -> Resultat<RapportRattrapage> {
        let contexte = self.noyau.contexte(session.etablissement_id, Some(session));
        let mut tarifs_a_facturer = Vec::new();
        let mut ignorees = 0;
        for ligne in self.rattrapage_apercu(session).await? {
            if ligne.erreur.is_some() {
                ignorees += 1;
                continue;
            }
            tarifs_a_facturer.push(
                ServiceInscriptions::nouveau(self.noyau)
                    .contexte_tarifaire(session, ligne.inscription_id)
                    .await?,
            );
        }
        let mut unite = self
            .noyau
            .commencer(contexte.clone(), Durabilite::Critique)
            .await?;
        let mut facturees = 0;
        for tarif in &tarifs_a_facturer {
            facturer_dans(&mut unite, &contexte, tarif, &ChoixFrais::default()).await?;
            facturees += 1;
        }
        self.noyau.terminer(unite).await?;
        Ok(RapportRattrapage {
            facturees,
            ignorees,
        })
    }
}
