//! Sessions de caisse (SPEC_PHASE_3 §6) : ouverture avec le fonds, clôture avec le total compté
//! et l'écart motivé, visa de la direction ou du comptable, état de caisse imprimé. Une session
//! dure autant que le caissier le décide (T-21).

use gesco_core::commun::{Horodatage, Id, Montant};
use gesco_core::core::{MachineAEtats, Operation, Permission, TypeEvenement, Utilisateur};
use gesco_core::finance::caisse::{total_theorique, verifier_cloture, verifier_ouverture};
use gesco_core::finance::{ModePaiement, MouvementCaisse, SessionCaisse, StatutSession};
use gesco_db::depots::{caisse, role, utilisateur};
use gesco_db::nouvel_id;
use gesco_db::unite::Durabilite;
use serde::{Deserialize, Serialize};

use crate::documents::date_longue;
use crate::erreur::{ErreurService, Resultat};
use crate::noyau::Noyau;
use crate::session::Session;

fn regle(cle: &'static str) -> ErreurService {
    ErreurService::Regle { cle }
}

/// Ouverture d'une session.
#[derive(Debug, Deserialize, specta::Type)]
pub struct DemandeOuverture {
    /// Fonds de caisse initial, en espèces.
    pub fonds_initial: Montant,
}

/// Clôture de la session ouverte de l'utilisateur.
#[derive(Debug, Deserialize, specta::Type)]
pub struct DemandeCloture {
    /// Espèces comptées dans le tiroir.
    pub total_compte: Montant,
    /// Explication d'un écart ; obligatoire si le compté diffère du théorique.
    pub motif_ecart: String,
}

/// Visa d'une session clôturée.
#[derive(Debug, Deserialize, specta::Type)]
pub struct DemandeVisa {
    /// Session.
    pub id: Id<SessionCaisse>,
    /// Commentaire facultatif.
    pub commentaire: String,
}

/// Session désignée.
#[derive(Debug, Deserialize, specta::Type)]
pub struct DemandeSession {
    /// Session.
    pub id: Id<SessionCaisse>,
}

/// Total d'un mode de paiement.
#[derive(Debug, Clone, Serialize, specta::Type)]
pub struct TotalMode {
    /// Mode.
    pub mode: ModePaiement,
    /// Montant encaissé.
    pub montant: Montant,
}

/// Totaux d'une session.
#[derive(Debug, Clone, Serialize, specta::Type)]
pub struct TotauxSession {
    /// Fonds initial.
    pub fonds: Montant,
    /// Encaissements en espèces.
    pub entrees_especes: Montant,
    /// Sorties d'espèces (régularisations).
    pub sorties_especes: Montant,
    /// Espèces attendues dans le tiroir.
    pub theorique: Montant,
    /// Encaissements par mode, espèces comprises ; hors du tiroir pour les autres modes.
    pub par_mode: Vec<TotalMode>,
    /// Reçus émis pendant la session et comptés dans ses totaux.
    pub nombre_recus: u32,
    /// Sorties de régularisation passées dans la session, tous modes.
    pub regularisations: Vec<MouvementCaisse>,
}

/// Session affichée.
#[derive(Debug, Clone, Serialize, specta::Type)]
pub struct SessionAffichee {
    /// Session.
    pub session: SessionCaisse,
    /// Nom du caissier.
    pub caissier: String,
    /// Nom de qui a visé.
    pub vise_par: Option<String>,
    /// Totaux.
    pub totaux: TotauxSession,
}

/// Service des sessions de caisse.
pub struct ServiceCaisse<'n> {
    noyau: &'n Noyau,
}

impl<'n> ServiceCaisse<'n> {
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

    /// Totaux d'une session, recalculés (R-20) : les espèces vont au tiroir, les autres modes
    /// sont totalisés à part. Comptent les versements valides, et ceux annulés après la clôture
    /// de la session : leur sortie de régularisation passe dans une autre. Les sorties en
    /// espèces passées ici diminuent le tiroir.
    pub(crate) async fn totaux(&self, session: &SessionCaisse) -> Resultat<TotauxSession> {
        let contexte = self.noyau.contexte(session.etablissement_id, None);
        let (valides, regularisations): (Vec<_>, _) = {
            let mut cnx = self.connexion().await?;
            let ailleurs = caisse::regularises_ailleurs(&mut cnx, session.id, &contexte).await?;
            let valides =
                gesco_db::depots::versements::de_la_session(&mut cnx, session.id, &contexte)
                    .await?
                    .into_iter()
                    .filter(|v| {
                        v.statut == gesco_core::finance::StatutVersement::Valide
                            || ailleurs.contains(&v.id)
                    })
                    .collect();
            let mouvements =
                caisse::mouvements_de_la_session(&mut cnx, session.id, &contexte).await?;
            (valides, mouvements)
        };
        let mut par_mode = Vec::new();
        for mode in ModePaiement::TOUS {
            let montant = valides
                .iter()
                .filter(|v| v.mode == *mode)
                .try_fold(Montant::ZERO, |s, v| s.ajouter(v.montant))
                .map_err(|_| regle("caisse.calcul_impossible"))?;
            if !montant.est_nul() {
                par_mode.push(TotalMode {
                    mode: *mode,
                    montant,
                });
            }
        }
        // Les mouvements en espèces : les sorties (régularisations, salaires, avances) et les
        // entrées qui remettent un paiement du personnel annulé.
        let especes = |entree: bool| {
            regularisations
                .iter()
                .filter(|m| m.mode == ModePaiement::Especes && m.nature.est_entree() == entree)
                .try_fold(Montant::ZERO, |s, m| s.ajouter(m.montant))
                .map_err(|_| regle("caisse.calcul_impossible"))
        };
        let entrees_especes = par_mode
            .iter()
            .find(|t| t.mode == ModePaiement::Especes)
            .map_or(Montant::ZERO, |t| t.montant)
            .ajouter(especes(true)?)
            .map_err(|_| regle("caisse.calcul_impossible"))?;
        let sorties_especes = especes(false)?;
        Ok(TotauxSession {
            fonds: session.fonds_initial,
            theorique: total_theorique(session.fonds_initial, entrees_especes, sorties_especes)
                .map_err(|_| regle("caisse.calcul_impossible"))?,
            entrees_especes,
            sorties_especes,
            par_mode,
            nombre_recus: u32::try_from(valides.len()).unwrap_or(u32::MAX),
            regularisations,
        })
    }

    async fn nom(&self, session: &Session, id: Id<Utilisateur>) -> Resultat<String> {
        let contexte = self.noyau.contexte(session.etablissement_id, Some(session));
        let mut cnx = self.connexion().await?;
        Ok(utilisateur::lire(&mut cnx, id, &contexte)
            .await
            .map(|u| u.nom_affiche)
            .unwrap_or_default())
    }

    async fn affichee(&self, session: &Session, s: SessionCaisse) -> Resultat<SessionAffichee> {
        let caissier = self.nom(session, s.caissier_id).await?;
        let vise_par = match s.rapprochee_par {
            Some(id) => Some(self.nom(session, id).await?),
            None => None,
        };
        Ok(SessionAffichee {
            totaux: self.totaux(&s).await?,
            caissier,
            vise_par,
            session: s,
        })
    }

    async fn ouverte(&self, session: &Session) -> Resultat<Option<SessionCaisse>> {
        let contexte = self.noyau.contexte(session.etablissement_id, Some(session));
        let mut cnx = self.connexion().await?;
        Ok(caisse::ouverte_de(&mut cnx, session.utilisateur_id, &contexte).await?)
    }

    /// Session ouverte de l'utilisateur, avec ses totaux.
    pub async fn courante(&self, session: &Session) -> Resultat<Option<SessionAffichee>> {
        match self.ouverte(session).await? {
            Some(s) => Ok(Some(self.affichee(session, s).await?)),
            None => Ok(None),
        }
    }

    /// Ouvre une session avec son fonds initial ; une seule à la fois par caissier.
    pub async fn ouvrir(&self, session: &Session, fonds: Montant) -> Resultat<SessionAffichee> {
        verifier_ouverture(fonds).map_err(regle)?;
        if self.ouverte(session).await?.is_some() {
            return Err(regle("caisse.deja_ouverte"));
        }
        let contexte = self.noyau.contexte(session.etablissement_id, Some(session));
        let s = SessionCaisse {
            id: nouvel_id(),
            etablissement_id: session.etablissement_id,
            caissier_id: session.utilisateur_id,
            ouverte_le: contexte.maintenant,
            fonds_initial: fonds,
            statut: StatutSession::Ouverte,
            cloturee_le: None,
            total_theorique: None,
            total_compte: None,
            ecart: None,
            motif_ecart: String::new(),
            rapprochee_par: None,
            rapprochee_le: None,
            commentaire_visa: String::new(),
        };
        let mut unite = self
            .noyau
            .commencer(contexte.clone(), Durabilite::Critique)
            .await?;
        caisse::inserer(unite.connexion(), &s, &contexte).await?;
        let id = s.id.vers_chaine();
        unite.auditer(
            "session_caisse",
            &id,
            Operation::Creation,
            None,
            Some(serde_json::to_value(&s)?),
            "caisse.ouvrir",
        );
        unite.emettre(
            TypeEvenement::SessionCaisseOuverte,
            "session_caisse",
            &id,
            serde_json::json!({ "caissier_id": s.caissier_id, "fonds_initial": s.fonds_initial }),
        );
        self.noyau.terminer(unite).await?;
        self.affichee(session, s).await
    }

    /// Clôture la session ouverte de l'utilisateur avec le total compté ; un écart exige un
    /// motif. La session ne se rouvrira pas.
    pub async fn cloturer(
        &self,
        session: &Session,
        demande: &DemandeCloture,
    ) -> Resultat<SessionAffichee> {
        let avant = self
            .ouverte(session)
            .await?
            .ok_or(regle("caisse.session_fermee"))?;
        let totaux = self.totaux(&avant).await?;
        let ecart = verifier_cloture(totaux.theorique, demande.total_compte, &demande.motif_ecart)
            .map_err(regle)?;
        let contexte = self.noyau.contexte(session.etablissement_id, Some(session));
        let apres = SessionCaisse {
            statut: avant.statut.transiter(StatutSession::Cloturee)?,
            cloturee_le: Some(contexte.maintenant),
            total_theorique: Some(totaux.theorique),
            total_compte: Some(demande.total_compte),
            ecart: Some(ecart),
            motif_ecart: demande.motif_ecart.trim().to_owned(),
            ..avant.clone()
        };
        let mut unite = self
            .noyau
            .commencer(contexte.clone(), Durabilite::Critique)
            .await?;
        caisse::ecrire(unite.connexion(), &apres, &contexte).await?;
        let id = apres.id.vers_chaine();
        unite.auditer(
            "session_caisse",
            &id,
            Operation::Transition,
            Some(serde_json::to_value(&avant)?),
            Some(serde_json::to_value(&apres)?),
            "caisse.cloturer",
        );
        unite.emettre(
            TypeEvenement::SessionCaisseCloturee,
            "session_caisse",
            &id,
            serde_json::json!({
                "caissier_id": apres.caissier_id,
                "total_theorique": totaux.theorique,
                "total_compte": demande.total_compte,
                "ecart": ecart,
            }),
        );
        self.noyau.terminer(unite).await?;
        self.affichee(session, apres).await
    }

    /// Vise une session clôturée (§10.9 du blueprint).
    pub async fn rapprocher(
        &self,
        session: &Session,
        demande: &DemandeVisa,
    ) -> Resultat<SessionAffichee> {
        let contexte = self.noyau.contexte(session.etablissement_id, Some(session));
        let avant = {
            let mut cnx = self.connexion().await?;
            caisse::lire(&mut cnx, demande.id, &contexte).await?
        };
        let apres = SessionCaisse {
            statut: avant.statut.transiter(StatutSession::Rapprochee)?,
            rapprochee_par: Some(session.utilisateur_id),
            rapprochee_le: Some(contexte.maintenant),
            commentaire_visa: demande.commentaire.trim().to_owned(),
            ..avant.clone()
        };
        let mut unite = self
            .noyau
            .commencer(contexte.clone(), Durabilite::Critique)
            .await?;
        caisse::ecrire(unite.connexion(), &apres, &contexte).await?;
        let id = apres.id.vers_chaine();
        unite.auditer(
            "session_caisse",
            &id,
            Operation::Transition,
            Some(serde_json::to_value(&avant)?),
            Some(serde_json::to_value(&apres)?),
            "caisse.rapprocher",
        );
        unite.emettre(
            TypeEvenement::SessionCaisseRapprochee,
            "session_caisse",
            &id,
            serde_json::json!({ "ecart": apres.ecart }),
        );
        self.noyau.terminer(unite).await?;
        self.affichee(session, apres).await
    }

    /// Vrai si des régularisations attendent un visa et que l'utilisateur vise les caisses : la
    /// direction en est avertie (SPEC_PHASE_3 §8).
    pub async fn regularisations_a_viser(&self, session: &Session) -> Resultat<bool> {
        let contexte = self.noyau.contexte(session.etablissement_id, Some(session));
        let mut cnx = self.connexion().await?;
        if !role::permissions_de(&mut cnx, session.utilisateur_id, &contexte)
            .await?
            .contains(&Permission::CASHBOX_RECONCILE)
        {
            return Ok(false);
        }
        Ok(caisse::regularisations_a_viser(&mut cnx, &contexte).await? > 0)
    }

    /// Sessions de l'établissement, les plus récentes d'abord.
    pub async fn lister(&self, session: &Session) -> Resultat<Vec<SessionAffichee>> {
        let contexte = self.noyau.contexte(session.etablissement_id, Some(session));
        let sessions = {
            let mut cnx = self.connexion().await?;
            caisse::lister(&mut cnx, 500, &contexte).await?
        };
        let mut affichees = Vec::with_capacity(sessions.len());
        for s in sessions {
            affichees.push(self.affichee(session, s).await?);
        }
        Ok(affichees)
    }

    /// Données de l'état de caisse d'une session (§6.2) : le caissier imprime le sien, le
    /// contrôleur (`cashbox:reconcile`) toutes.
    pub async fn donnees_etat(
        &self,
        session: &Session,
        id: Id<SessionCaisse>,
    ) -> Resultat<serde_json::Value> {
        let contexte = self.noyau.contexte(session.etablissement_id, Some(session));
        let s = {
            let mut cnx = self.connexion().await?;
            let s = caisse::lire(&mut cnx, id, &contexte).await?;
            if s.caissier_id != session.utilisateur_id
                && !role::permissions_de(&mut cnx, session.utilisateur_id, &contexte)
                    .await?
                    .contains(&Permission::CASHBOX_RECONCILE)
            {
                return Err(ErreurService::permission(Permission::CASHBOX_RECONCILE));
            }
            s
        };
        let a = self.affichee(session, s).await?;
        let francs = |m: Montant| m.formater();
        let date = |h: Option<Horodatage>| h.map(date_longue).unwrap_or_default();
        Ok(serde_json::json!({
            "caissier": a.caissier,
            "ouverture": date_longue(a.session.ouverte_le),
            "cloture": date(a.session.cloturee_le),
            "statut": match a.session.statut {
                StatutSession::Ouverte => "Ouverte",
                StatutSession::Cloturee => "Clôturée",
                StatutSession::Rapprochee => "Visée",
            },
            "fonds": francs(a.totaux.fonds),
            "entrees_especes": francs(a.totaux.entrees_especes),
            "sorties_especes": francs(a.totaux.sorties_especes),
            "theorique": francs(a.totaux.theorique),
            "compte": a.session.total_compte.map(francs).unwrap_or_default(),
            "ecart": a.session.ecart.map(francs).unwrap_or_default(),
            "motif_ecart": a.session.motif_ecart,
            "modes": a.totaux.par_mode.iter().map(|t| serde_json::json!({
                "mode": libelle_mode(t.mode),
                "montant": francs(t.montant),
            })).collect::<Vec<_>>(),
            "nombre_recus": a.totaux.nombre_recus,
            "sorties": a.totaux.regularisations.iter().map(|m| serde_json::json!({
                "nature": libelle_nature(m.nature),
                "motif": m.motif,
                "mode": libelle_mode(m.mode),
                "montant": francs(m.montant),
            })).collect::<Vec<_>>(),
            "vise_par": a.vise_par.unwrap_or_default(),
            "visa": date(a.session.rapprochee_le),
            "commentaire_visa": a.session.commentaire_visa,
        }))
    }
}

/// Libellé imprimé de la nature d'un mouvement de caisse.
pub fn libelle_nature(nature: gesco_core::finance::NatureMouvement) -> &'static str {
    use gesco_core::finance::NatureMouvement as N;
    match nature {
        N::SortieRegularisation => "Régularisation",
        N::SortieSalaire => "Salaire",
        N::SortieAvance => "Avance",
        N::SortieDepense => "Dépense",
        N::SortieTransfert => "Dépôt",
        N::EntreeRegularisation => "Remis en caisse",
        N::EntreeTransfert => "Retrait",
        N::EntreeRecette => "Recette",
    }
}

/// Libellé imprimé d'un mode de paiement.
pub fn libelle_mode(mode: ModePaiement) -> &'static str {
    match mode {
        ModePaiement::Especes => "Espèces",
        ModePaiement::OrangeMoney => "Orange Money",
        ModePaiement::Wave => "Wave",
        ModePaiement::Moov => "Moov Money",
        ModePaiement::Cheque => "Chèque",
        ModePaiement::Virement => "Virement",
    }
}
