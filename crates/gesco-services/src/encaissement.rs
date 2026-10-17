//! Encaissement (SPEC_PHASE_3 §7, UC-02) : un versement imputé sur les échéances, la plus
//! ancienne d'abord ou à la main, numéroté sans trou, dans la session ouverte du caissier. Une
//! clé tirée par l'écran rend l'encaissement rejouable sans doubler la recette (R-35).
//!
//! Annulation (§8) : le versement et ses imputations sont contre-passés, jamais supprimés. Si
//! sa session est close, la sortie de régularisation passe dans la session ouverte de qui
//! annule, et la direction en est avertie jusqu'au visa de cette session.

use gesco_core::commun::{Id, Montant};
use gesco_core::core::MachineAEtats;
use gesco_core::core::{Operation, TypeDocumentNumerote, TypeEvenement};
use gesco_core::finance::versement::{
    imputer_plus_ancienne, verifier_annulation, verifier_repartition, verifier_versement,
};
use gesco_core::finance::{
    Echeance, Facture, Imputation, ModePaiement, MouvementCaisse, SessionCaisse, StatutFacture,
    StatutSession, StatutVersement, Versement,
};
use gesco_core::scolarite::personne::millesime;
use gesco_core::scolarite::{Inscription, Responsable};
use gesco_db::depots::{annee_scolaire, caisse, factures, personnes, versements};
use gesco_db::unite::Durabilite;
use gesco_db::{Contexte, UniteDeTravail, nouvel_id};
use serde::{Deserialize, Serialize};

use crate::effets::Effet;
use crate::eleves::ServiceEleves;
use crate::erreur::{ErreurService, Resultat};
use crate::facturation::{EcheanceAffichee, etat_echeances, recalculer_dans};
use crate::inscriptions::ServiceInscriptions;
use crate::noyau::Noyau;
use crate::numerotation;
use crate::session::Session;

fn regle(cle: &'static str) -> ErreurService {
    ErreurService::Regle { cle }
}

/// Ce que la caisse doit savoir avant d'encaisser pour une inscription.
#[derive(Debug, Deserialize, specta::Type)]
pub struct DemandePreparation {
    /// Inscription.
    pub inscription_id: Id<Inscription>,
    /// Montant envisagé, pour proposer l'imputation ; absent : aucune proposition.
    pub montant: Option<Montant>,
}

/// Payeur proposé : un responsable payeur de l'élève.
#[derive(Debug, Clone, Serialize, specta::Type)]
pub struct PayeurPropose {
    /// Responsable.
    pub responsable_id: Id<Responsable>,
    /// Prénoms et nom.
    pub nom: String,
    /// Téléphones, forme canonique.
    pub telephones: Vec<String>,
}

/// Situation d'une inscription à la caisse.
#[derive(Debug, Clone, Serialize, specta::Type)]
pub struct PreparationEncaissement {
    /// Inscription.
    pub inscription_id: Id<Inscription>,
    /// Élève : prénoms et nom.
    pub eleve: String,
    /// Matricule.
    pub matricule: String,
    /// Classe.
    pub classe: String,
    /// Facture ; absente pour une inscription non validée.
    pub facture: Option<Facture>,
    /// Échéances dues avec leur reste, par date.
    pub echeances: Vec<EcheanceAffichee>,
    /// Reste dû total.
    pub reste: Montant,
    /// Reste échu à ce jour.
    pub reste_echu: Montant,
    /// Payeurs proposés, le premier par défaut.
    pub payeurs: Vec<PayeurPropose>,
    /// Imputation proposée pour le montant envisagé.
    pub imputation: Vec<Imputation>,
    /// Vrai si l'utilisateur a une caisse ouverte.
    pub caisse_ouverte: bool,
}

/// Encaissement d'un versement pour une inscription.
#[derive(Debug, Clone, Deserialize, specta::Type)]
pub struct DemandeEncaissement {
    /// Clé tirée par l'écran : un encaissement rejoué rend le même versement.
    pub cle_idempotence: String,
    /// Inscription.
    pub inscription_id: Id<Inscription>,
    /// Montant.
    pub montant: Montant,
    /// Mode.
    pub mode: ModePaiement,
    /// Référence de la transaction ; exigée hors espèces.
    pub reference: String,
    /// Responsable payeur ; absent : un tiers nommé par `payeur_nom`.
    pub payeur_id: Option<Id<Responsable>>,
    /// Nom du payeur tiers.
    pub payeur_nom: String,
    /// Répartition manuelle ; absente : la plus ancienne échéance d'abord.
    pub repartition: Option<Vec<Imputation>>,
}

/// Part d'un enfant dans un encaissement familial.
#[derive(Debug, Clone, Deserialize, specta::Type)]
pub struct PartEnfant {
    /// Inscription de l'enfant.
    pub inscription_id: Id<Inscription>,
    /// Montant pour cet enfant.
    pub montant: Montant,
}

/// Encaissement pour plusieurs enfants d'un même payeur (T-15) : un versement et un reçu par
/// enfant, dans une seule transaction.
#[derive(Debug, Clone, Deserialize, specta::Type)]
pub struct DemandeFamille {
    /// Clé de l'opération ; chaque enfant reçoit la sienne, dérivée.
    pub cle_idempotence: String,
    /// Mode.
    pub mode: ModePaiement,
    /// Référence.
    pub reference: String,
    /// Responsable payeur.
    pub payeur_id: Option<Id<Responsable>>,
    /// Nom du payeur tiers.
    pub payeur_nom: String,
    /// Parts, une par enfant.
    pub parts: Vec<PartEnfant>,
}

/// Résultat d'un encaissement : le versement, et vrai s'il vient d'être créé (faux pour une
/// clé déjà vue).
#[derive(Debug, Clone)]
pub struct Encaisse {
    /// Versement.
    pub versement: Versement,
    /// Créé par cet appel.
    pub nouveau: bool,
}

/// Annulation d'un versement, avec son motif.
#[derive(Debug, Deserialize, specta::Type)]
pub struct DemandeAnnulationVersement {
    /// Versement.
    pub id: Id<Versement>,
    /// Motif ; obligatoire, imprimé sur le reçu.
    pub motif: String,
}

/// Résultat d'une annulation : le versement annulé, et la sortie de régularisation passée si sa
/// session était close.
#[derive(Debug, Clone)]
pub struct Annule {
    /// Versement annulé.
    pub versement: Versement,
    /// Sortie de régularisation.
    pub regularisation: Option<MouvementCaisse>,
}

/// Service d'encaissement.
pub struct ServiceEncaissement<'n> {
    noyau: &'n Noyau,
}

struct Prepare {
    demande: DemandeEncaissement,
    facture: Facture,
    payeur_nom: String,
    imputations: Vec<Imputation>,
    exercice: String,
}

impl<'n> ServiceEncaissement<'n> {
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

    /// Situation de l'inscription à la caisse, et imputation proposée pour un montant.
    pub async fn preparer(
        &self,
        session: &Session,
        demande: &DemandePreparation,
    ) -> Resultat<PreparationEncaissement> {
        let contexte = self.noyau.contexte(session.etablissement_id, Some(session));
        let inscription = ServiceInscriptions::nouveau(self.noyau)
            .lire_publique(session, demande.inscription_id)
            .await?;
        let fiche = ServiceEleves::nouveau(self.noyau)
            .fiche(session, inscription.eleve_id)
            .await?;
        let classe = fiche
            .inscriptions
            .iter()
            .find(|i| i.inscription.id == inscription.id)
            .map(|i| i.classe.clone())
            .unwrap_or_default();
        let mut cnx = self.connexion().await?;
        let facture = factures::facture_vivante(&mut cnx, inscription.id, &contexte).await?;
        let echeances = match &facture {
            Some(f) => etat_echeances(&mut cnx, f.id, &contexte).await?,
            None => Vec::new(),
        };
        let aujourd_hui = crate::inscriptions::jour(contexte.maintenant)?;
        let somme = |filtre: &dyn Fn(&EcheanceAffichee) -> bool| {
            echeances
                .iter()
                .filter(|e| filtre(e))
                .try_fold(Montant::ZERO, |s, e| s.ajouter(e.reste))
                .map_err(|_| regle("versement.calcul_impossible"))
        };
        let reste = somme(&|_| true)?;
        let reste_echu = somme(&|e| e.echeance.date <= aujourd_hui)?;
        let imputation = match demande.montant {
            Some(m) if !m.est_nul() => {
                imputer_plus_ancienne(&restes(&echeances), m).unwrap_or_default()
            }
            _ => Vec::new(),
        };
        let mut payeurs: Vec<_> = fiche
            .responsables
            .iter()
            .filter(|r| r.qualites.est_payeur)
            .map(|r| PayeurPropose {
                responsable_id: r.responsable.id,
                nom: r.identite.nom_complet(),
                telephones: r.identite.telephones.clone(),
            })
            .collect();
        payeurs.extend(
            fiche
                .responsables
                .iter()
                .filter(|r| !r.qualites.est_payeur)
                .map(|r| PayeurPropose {
                    responsable_id: r.responsable.id,
                    nom: r.identite.nom_complet(),
                    telephones: r.identite.telephones.clone(),
                }),
        );
        let caisse_ouverte = caisse::ouverte_de(&mut cnx, session.utilisateur_id, &contexte)
            .await?
            .is_some();
        Ok(PreparationEncaissement {
            inscription_id: inscription.id,
            eleve: fiche.identite.nom_complet(),
            matricule: fiche.eleve.matricule_interne,
            classe,
            facture,
            echeances,
            reste,
            reste_echu,
            payeurs,
            imputation,
            caisse_ouverte,
        })
    }

    /// Vérifie un encaissement hors transaction : payeur, facture, imputation.
    async fn verifier(&self, session: &Session, demande: DemandeEncaissement) -> Resultat<Prepare> {
        let contexte = self.noyau.contexte(session.etablissement_id, Some(session));
        let mut cnx = self.connexion().await?;
        let payeur_nom = match demande.payeur_id {
            Some(id) => personnes::responsable(&mut cnx, id, &contexte)
                .await?
                .1
                .nom_complet(),
            None => demande.payeur_nom.trim().to_owned(),
        };
        verifier_versement(
            demande.montant,
            demande.mode,
            &demande.reference,
            &payeur_nom,
        )
        .map_err(regle)?;
        let facture = factures::facture_vivante(&mut cnx, demande.inscription_id, &contexte)
            .await?
            .ok_or(regle("facture.absente"))?;
        let etats = etat_echeances(&mut cnx, facture.id, &contexte).await?;
        let restes = restes(&etats);
        let imputations = match &demande.repartition {
            Some(r) => {
                verifier_repartition(&restes, r, demande.montant).map_err(regle)?;
                r.clone()
            }
            None => imputer_plus_ancienne(&restes, demande.montant).map_err(regle)?,
        };
        let annee = annee_scolaire::lire(&mut cnx, facture.annee_id, &contexte).await?;
        let exercice = millesime(&annee.libelle)
            .map(str::to_owned)
            .unwrap_or_else(|| facture.date_emission.annee().to_string());
        Ok(Prepare {
            demande,
            facture,
            payeur_nom,
            imputations,
            exercice,
        })
    }

    /// Enregistre un versement vérifié dans la transaction ouverte.
    async fn inscrire_dans(
        unite: &mut UniteDeTravail<Effet>,
        contexte: &Contexte,
        session: &Session,
        caisse_id: Id<SessionCaisse>,
        prepare: Prepare,
    ) -> Resultat<Versement> {
        let numero =
            numerotation::attribuer(unite, TypeDocumentNumerote::Recu, &prepare.exercice).await?;
        let versement = Versement {
            id: nouvel_id(),
            etablissement_id: session.etablissement_id,
            inscription_id: prepare.demande.inscription_id,
            facture_id: prepare.facture.id,
            session_id: caisse_id,
            numero_recu: numero.numero,
            encaisse_le: contexte.maintenant,
            montant: prepare.demande.montant,
            mode: prepare.demande.mode,
            reference: prepare.demande.reference.trim().to_owned(),
            payeur_id: prepare.demande.payeur_id,
            payeur_nom: prepare.payeur_nom,
            encaisse_par: session.utilisateur_id,
            cle_idempotence: prepare.demande.cle_idempotence.clone(),
            statut: StatutVersement::Valide,
            motif_annulation: String::new(),
        };
        versements::inserer(unite.connexion(), &versement, contexte).await?;
        for imputation in &prepare.imputations {
            versements::inserer_imputation(unite.connexion(), versement.id, imputation, contexte)
                .await?;
        }
        let facture = recalculer_dans(unite, contexte, &prepare.facture).await?;
        let id = versement.id.vers_chaine();
        unite.auditer(
            "versement",
            &id,
            Operation::Creation,
            None,
            Some(serde_json::json!({ "versement": versement, "imputations": prepare.imputations })),
            "encaissement.encaisser",
        );
        unite.emettre(
            TypeEvenement::VersementEncaisse,
            "versement",
            &id,
            serde_json::json!({
                "inscription_id": versement.inscription_id,
                "numero_recu": versement.numero_recu,
                "montant": versement.montant,
                "mode": versement.mode,
                "facture_soldee": facture.statut == StatutFacture::Soldee,
            }),
        );
        Ok(versement)
    }

    async fn deja_vu(&self, session: &Session, cle: &str) -> Resultat<Option<Versement>> {
        if cle.trim().len() < 8 {
            return Err(regle("versement.cle_invalide"));
        }
        let contexte = self.noyau.contexte(session.etablissement_id, Some(session));
        let mut cnx = self.connexion().await?;
        Ok(versements::par_cle(&mut cnx, cle, &contexte).await?)
    }

    async fn caisse_ouverte(&self, session: &Session) -> Resultat<Id<SessionCaisse>> {
        let contexte = self.noyau.contexte(session.etablissement_id, Some(session));
        let mut cnx = self.connexion().await?;
        caisse::ouverte_de(&mut cnx, session.utilisateur_id, &contexte)
            .await?
            .map(|s| s.id)
            .ok_or(regle("caisse.session_fermee"))
    }

    /// Encaisse un versement : session ouverte, imputation, reçu numéroté, état de la facture,
    /// le tout dans une transaction (R-17). Une clé déjà vue rend le versement d'origine.
    pub async fn encaisser(
        &self,
        session: &Session,
        demande: DemandeEncaissement,
    ) -> Resultat<Encaisse> {
        if let Some(versement) = self.deja_vu(session, &demande.cle_idempotence).await? {
            if versement.inscription_id != demande.inscription_id {
                return Err(regle("versement.cle_reutilisee"));
            }
            return Ok(Encaisse {
                versement,
                nouveau: false,
            });
        }
        let caisse_id = self.caisse_ouverte(session).await?;
        let prepare = self.verifier(session, demande).await?;
        let contexte = self.noyau.contexte(session.etablissement_id, Some(session));
        let mut unite = self
            .noyau
            .commencer(contexte.clone(), Durabilite::Critique)
            .await?;
        let versement =
            Self::inscrire_dans(&mut unite, &contexte, session, caisse_id, prepare).await?;
        self.noyau.terminer(unite).await?;
        Ok(Encaisse {
            versement,
            nouveau: true,
        })
    }

    /// Encaisse pour plusieurs enfants d'un même payeur : un versement et un reçu par enfant,
    /// dans une seule transaction (T-15). Rejouée, l'opération rend les mêmes versements.
    pub async fn encaisser_famille(
        &self,
        session: &Session,
        demande: DemandeFamille,
    ) -> Resultat<Vec<Encaisse>> {
        if demande.parts.is_empty() {
            return Err(regle("versement.famille_vide"));
        }
        let cle = |i: usize| format!("{}-{}", demande.cle_idempotence, i + 1);
        let mut deja = Vec::new();
        for i in 0..demande.parts.len() {
            if let Some(v) = self.deja_vu(session, &cle(i)).await? {
                deja.push(Encaisse {
                    versement: v,
                    nouveau: false,
                });
            }
        }
        if !deja.is_empty() {
            return Ok(deja);
        }
        let caisse_id = self.caisse_ouverte(session).await?;
        let mut prepares = Vec::new();
        for (i, part) in demande.parts.iter().enumerate() {
            if demande
                .parts
                .iter()
                .take(i)
                .any(|p| p.inscription_id == part.inscription_id)
            {
                return Err(regle("versement.enfant_en_double"));
            }
            prepares.push(
                self.verifier(
                    session,
                    DemandeEncaissement {
                        cle_idempotence: cle(i),
                        inscription_id: part.inscription_id,
                        montant: part.montant,
                        mode: demande.mode,
                        reference: demande.reference.clone(),
                        payeur_id: demande.payeur_id,
                        payeur_nom: demande.payeur_nom.clone(),
                        repartition: None,
                    },
                )
                .await?,
            );
        }
        let contexte = self.noyau.contexte(session.etablissement_id, Some(session));
        let mut unite = self
            .noyau
            .commencer(contexte.clone(), Durabilite::Critique)
            .await?;
        let mut faits = Vec::new();
        for prepare in prepares {
            faits.push(Encaisse {
                versement: Self::inscrire_dans(&mut unite, &contexte, session, caisse_id, prepare)
                    .await?,
                nouveau: true,
            });
        }
        self.noyau.terminer(unite).await?;
        Ok(faits)
    }

    /// Annule un versement (`payments:cancel`, §10.3 du blueprint) : motif obligatoire,
    /// imputations contre-passées, état de la facture recalculé. Si la session du versement est
    /// close, une sortie de régularisation du même montant passe dans la session ouverte de qui
    /// annule ; elle est exigée. Le reçu reste, marqué annulé.
    pub async fn annuler(
        &self,
        session: &Session,
        demande: &DemandeAnnulationVersement,
    ) -> Resultat<Annule> {
        let contexte = self.noyau.contexte(session.etablissement_id, Some(session));
        let (avant, caisse_du_versement, facture, ouverte) = {
            let mut cnx = self.connexion().await?;
            let v = versements::lire(&mut cnx, demande.id, &contexte).await?;
            verifier_annulation(&v, &demande.motif).map_err(regle)?;
            let s = caisse::lire(&mut cnx, v.session_id, &contexte).await?;
            let f = factures::factures_de_l_inscription(&mut cnx, v.inscription_id, &contexte)
                .await?
                .into_iter()
                .find(|f| f.id == v.facture_id)
                .ok_or(regle("facture.absente"))?;
            let ouverte = caisse::ouverte_de(&mut cnx, session.utilisateur_id, &contexte).await?;
            (v, s, f, ouverte)
        };
        let motif = demande.motif.trim().to_owned();
        let regularisation = if caisse_du_versement.statut == StatutSession::Ouverte {
            None
        } else {
            let ouverte = ouverte.ok_or(regle("versement.caisse_requise"))?;
            if avant.mode == ModePaiement::Especes {
                let tiroir = crate::caisse::ServiceCaisse::nouveau(self.noyau)
                    .totaux(&ouverte)
                    .await?
                    .theorique;
                if tiroir < avant.montant {
                    return Err(regle("versement.especes_insuffisantes"));
                }
            }
            Some(MouvementCaisse {
                id: nouvel_id(),
                etablissement_id: session.etablissement_id,
                session_id: ouverte.id,
                montant: avant.montant,
                mode: avant.mode,
                motif: format!("Annulation du reçu {} : {motif}", avant.numero_recu),
                versement_id: Some(avant.id),
                le: contexte.maintenant,
                par: session.utilisateur_id,
                nature: gesco_core::finance::NatureMouvement::SortieRegularisation,
                paiement_id: None,
                depense_id: None,
                transfert_id: None,
                recette_id: None,
            })
        };
        let apres = Versement {
            statut: avant.statut.transiter(StatutVersement::Annule)?,
            motif_annulation: motif.clone(),
            ..avant.clone()
        };
        let mut unite = self
            .noyau
            .commencer(contexte.clone(), Durabilite::Critique)
            .await?;
        if !versements::annuler(unite.connexion(), avant.id, &motif, &contexte).await? {
            return Err(regle("versement.deja_annule"));
        }
        versements::annuler_imputations(unite.connexion(), avant.id, &contexte).await?;
        let facture = recalculer_dans(&mut unite, &contexte, &facture).await?;
        let id = avant.id.vers_chaine();
        unite.auditer(
            "versement",
            &id,
            Operation::Annulation,
            Some(serde_json::to_value(&avant)?),
            Some(serde_json::to_value(&apres)?),
            "encaissement.annuler",
        );
        if let Some(m) = &regularisation {
            caisse::inserer_mouvement(unite.connexion(), m, &contexte).await?;
            unite.auditer(
                "mouvement_caisse",
                &m.id.vers_chaine(),
                Operation::Creation,
                None,
                Some(serde_json::to_value(m)?),
                "encaissement.regulariser",
            );
        }
        unite.emettre(
            TypeEvenement::VersementAnnule,
            "versement",
            &id,
            serde_json::json!({
                "inscription_id": avant.inscription_id,
                "numero_recu": avant.numero_recu,
                "montant": avant.montant,
                "motif": motif,
                "statut_facture": facture.statut,
                "regularisation": regularisation.as_ref().map(|m| m.session_id),
            }),
        );
        self.noyau.terminer(unite).await?;
        Ok(Annule {
            versement: apres,
            regularisation,
        })
    }
}

/// Date et heure d'un encaissement : « 9 octobre 2026 à 10 h 42 ».
fn date_et_heure(instant: gesco_core::commun::Horodatage) -> String {
    let civil = instant.civil_utc();
    format!(
        "{} à {} h {:02}",
        crate::documents::date_longue(instant),
        civil.heure,
        civil.minute
    )
}

impl ServiceEncaissement<'_> {
    /// Données du reçu d'un versement (§7.4). `duplicata` : réimpression.
    pub async fn donnees_recu(
        &self,
        session: &Session,
        id: Id<Versement>,
        duplicata: bool,
    ) -> Resultat<serde_json::Value> {
        let contexte = self.noyau.contexte(session.etablissement_id, Some(session));
        let mut cnx = self.connexion().await?;
        let v = versements::lire(&mut cnx, id, &contexte).await?;
        let facture = factures::factures_de_l_inscription(&mut cnx, v.inscription_id, &contexte)
            .await?
            .into_iter()
            .find(|f| f.id == v.facture_id)
            .ok_or(regle("facture.absente"))?;
        let echeances = factures::echeances(&mut cnx, facture.id, &contexte).await?;
        let reste = etat_echeances(&mut cnx, facture.id, &contexte)
            .await?
            .iter()
            .try_fold(Montant::ZERO, |s, e| s.ajouter(e.reste))
            .map_err(|_| regle("versement.calcul_impossible"))?;
        let imputations = versements::imputations_du_versement(&mut cnx, v.id, &contexte).await?;
        let annee = annee_scolaire::lire(&mut cnx, facture.annee_id, &contexte).await?;
        let caissier = gesco_db::depots::utilisateur::lire(&mut cnx, v.encaisse_par, &contexte)
            .await
            .map(|u| u.nom_affiche)
            .unwrap_or_default();
        drop(cnx);
        let inscription = ServiceInscriptions::nouveau(self.noyau)
            .lire_publique(session, v.inscription_id)
            .await?;
        let fiche = ServiceEleves::nouveau(self.noyau)
            .fiche(session, inscription.eleve_id)
            .await?;
        let classe = fiche
            .inscriptions
            .iter()
            .find(|i| i.inscription.id == inscription.id)
            .map(|i| i.classe.clone())
            .unwrap_or_default();
        Ok(serde_json::json!({
            "numero": v.numero_recu,
            "date": date_et_heure(v.encaisse_le),
            "eleve": fiche.identite.nom_complet(),
            "matricule": fiche.eleve.matricule_interne,
            "classe": classe,
            "annee": annee.libelle,
            "payeur": v.payeur_nom,
            "montant": v.montant.formater(),
            "montant_lettres": v.montant.en_lettres().map_err(|_| regle("versement.calcul_impossible"))?,
            "mode": crate::caisse::libelle_mode(v.mode),
            "reference_paiement": v.reference,
            "imputations": imputations.iter().map(|i| serde_json::json!({
                "libelle": echeances.iter().find(|e| e.id == i.echeance_id).map(|e| e.libelle.clone()).unwrap_or_default(),
                "montant": i.montant.formater(),
            })).collect::<Vec<_>>(),
            "reste": reste.formater(),
            "caissier": caissier,
            "annule": v.statut == StatutVersement::Annule,
            "motif_annulation": v.motif_annulation,
            "duplicata": duplicata,
        }))
    }

    /// Données du relevé de compte d'un élève (§4.4).
    pub async fn donnees_releve(
        &self,
        session: &Session,
        eleve_id: Id<gesco_core::scolarite::Eleve>,
    ) -> Resultat<serde_json::Value> {
        let contexte = self.noyau.contexte(session.etablissement_id, Some(session));
        let fiche = ServiceEleves::nouveau(self.noyau)
            .fiche(session, eleve_id)
            .await?;
        let situations = crate::facturation::ServiceFacturation::nouveau(self.noyau)
            .situation(session, eleve_id)
            .await?;
        let date_civile =
            |d: gesco_core::commun::DateCivile| crate::documents::date_civile_longue(d);
        let inscriptions: Vec<serde_json::Value> = situations
            .iter()
            .map(|s| {
                serde_json::json!({
                    "annee": s.annee,
                    "classe": s.classe,
                    "facture": s.facture.as_ref().map(|f| serde_json::json!({
                        "numero": f.facture.numero,
                        "plan": f.plan,
                        "total": f.facture.total.formater(),
                        "remise": f.remise.formater(),
                        "paye": f.paye.formater(),
                        "reste": f.reste.formater(),
                        "echeances": f.echeances.iter().map(|e| serde_json::json!({
                            "libelle": e.echeance.libelle,
                            "date": date_civile(e.echeance.date),
                            "montant": e.echeance.montant.formater(),
                            "remise": e.remise.formater(),
                            "paye": e.paye.formater(),
                            "reste": e.reste.formater(),
                        })).collect::<Vec<_>>(),
                    })),
                    "versements": s.versements.iter().map(|v| serde_json::json!({
                        "numero": if v.statut == StatutVersement::Annule { format!("{} (annulé)", v.numero_recu) } else { v.numero_recu.clone() },
                        "date": crate::documents::date_longue(v.encaisse_le),
                        "mode": crate::caisse::libelle_mode(v.mode),
                        "montant": v.montant.formater(),
                    })).collect::<Vec<_>>(),
                })
            })
            .collect();
        Ok(serde_json::json!({
            "eleve": fiche.identite.nom_complet(),
            "matricule": fiche.eleve.matricule_interne,
            "date": crate::documents::date_longue(contexte.maintenant),
            "inscriptions": inscriptions,
        }))
    }
}

fn restes(etats: &[EcheanceAffichee]) -> Vec<(Id<Echeance>, Montant)> {
    etats.iter().map(|e| (e.echeance.id, e.reste)).collect()
}
