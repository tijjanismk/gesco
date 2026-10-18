//! Remise des bulletins (SPEC_PHASE_5 §6, blueprint §8.5bis) : chaque bulletin publié naît « à
//! remettre » ; il se remet à un responsable, à l'élève, à une personne autorisée ou à un tiers,
//! et se signe ; resté à remettre quand la période suivante est publiée, il est « non retiré ».
//! Les bulletins non retirés à sept jours forment une liste, croisée avec les impayés de la
//! famille ; l'avis peut se renvoyer depuis elle.

use gesco_core::commun::{DateCivile, Id, Montant};
use gesco_core::core::{Operation, Permission, TypeEvenement};
use gesco_core::finance::recouvrement::impaye;
use gesco_core::pedagogie::Periode;
use gesco_core::pedagogie::bulletin::Bulletin;
use gesco_core::scolarite::remise_document::{
    RemisA, RemiseDocument, StatutRemiseDocument, attente, verifier_remise,
};
use gesco_core::scolarite::{Classe, Eleve, Responsable};
use gesco_db::depots::{
    academique, factures, inscriptions, pedagogie, personnes, remises_documents, role,
};
use gesco_db::unite::Durabilite;
use gesco_db::{Connexion, Contexte, nouvel_id};
use serde::{Deserialize, Serialize};

use crate::erreur::{ErreurService, Resultat};
use crate::noyau::Noyau;
use crate::session::Session;

fn regle(cle: &'static str) -> ErreurService {
    ErreurService::Regle { cle }
}

/// Responsable d'un élève, tel que la remise le propose.
#[derive(Debug, Clone, Serialize, specta::Type)]
pub struct ResponsableRemise {
    /// Responsable.
    pub id: Id<Responsable>,
    /// Prénoms et nom.
    pub nom: String,
    /// Lien : père, mère, tuteur…
    pub lien: String,
}

/// Remise d'un bulletin, telle que l'écran la montre.
#[derive(Debug, Clone, Serialize, specta::Type)]
pub struct LigneRemise {
    /// Remise.
    pub remise: RemiseDocument,
    /// Élève.
    pub eleve_id: Id<Eleve>,
    /// Prénoms et nom de l'élève.
    pub eleve: String,
    /// Classe.
    pub classe: String,
    /// Période : « 1er trimestre ».
    pub periode: String,
    /// Responsables de l'élève, à qui le bulletin peut être remis.
    pub responsables: Vec<ResponsableRemise>,
    /// Qui l'a reçu, en clair.
    pub recu_par: String,
    /// Jours écoulés depuis la publication.
    pub jours: i64,
    /// Vrai s'il est à signaler : non retiré, ou à remettre depuis sept jours.
    pub a_signaler: bool,
    /// Ce que la famille doit d'échu, s'il reste quelque chose.
    pub impaye_echu: Option<Montant>,
}

/// Ce que peut faire le compte connecté.
#[derive(Debug, Clone, Copy, Serialize, specta::Type)]
pub struct DroitsRemise {
    /// Remettre, faire signer, renvoyer l'avis.
    pub remettre: bool,
}

/// Remises d'une classe pour une période.
#[derive(Debug, Clone, Serialize, specta::Type)]
pub struct RemisesClasse {
    /// Une ligne par élève dont le bulletin est publié.
    pub lignes: Vec<LigneRemise>,
    /// Droits de qui consulte.
    pub droits: DroitsRemise,
}

/// Classe et période dont on lit les remises.
#[derive(Debug, Deserialize, specta::Type)]
pub struct DemandeRemisesClasse {
    /// Classe.
    pub classe_id: Id<Classe>,
    /// Période.
    pub periode_id: Id<Periode>,
}

/// Remise d'un bulletin.
#[derive(Debug, Deserialize, specta::Type)]
pub struct DemandeRemiseBulletin {
    /// Remise.
    pub id: Id<RemiseDocument>,
    /// À qui.
    pub remis_a: RemisA,
    /// Responsable qui le reçoit.
    pub responsable_id: Option<Id<Responsable>>,
    /// Nom de qui le retire, pour une personne autorisée ou un tiers.
    pub nom_retrait: String,
    /// Vrai s'il est signé sur place.
    pub signe: bool,
    /// Nom de qui signe.
    pub signataire: String,
}

/// Signature d'un bulletin déjà remis.
#[derive(Debug, Deserialize, specta::Type)]
pub struct DemandeSignatureBulletin {
    /// Remise.
    pub id: Id<RemiseDocument>,
    /// Nom de qui signe.
    pub signataire: String,
}

/// Une remise.
#[derive(Debug, Deserialize, specta::Type)]
pub struct DemandeRemiseId {
    /// Remise.
    pub id: Id<RemiseDocument>,
}

/// Remise neuve ou mise à jour à la publication d'un bulletin, dans sa transaction : un
/// bulletin corrigé et republié est de nouveau à remettre ; les bulletins des périodes
/// précédentes encore à remettre deviennent non retirés.
pub(crate) async fn a_la_publication(
    cnx: &mut Connexion,
    bulletin: &Bulletin,
    contexte: &Contexte,
) -> Resultat<()> {
    let existante = remises_documents::de_la_periode_de(
        cnx,
        bulletin.inscription_id,
        bulletin.periode_id,
        contexte,
    )
    .await?;
    let remise = match existante {
        Some(r) if r.bulletin_id == bulletin.id => r,
        Some(r) => RemiseDocument {
            bulletin_id: bulletin.id,
            statut: StatutRemiseDocument::ARemettre,
            remis_a: None,
            responsable_id: None,
            nom_retrait: String::new(),
            signataire: String::new(),
            remis_le: None,
            signe_le: None,
            remis_par: None,
            publie_le: contexte.maintenant,
            ..r
        },
        None => RemiseDocument {
            id: nouvel_id(),
            etablissement_id: contexte.etablissement_id,
            bulletin_id: bulletin.id,
            inscription_id: bulletin.inscription_id,
            periode_id: bulletin.periode_id,
            statut: StatutRemiseDocument::ARemettre,
            remis_a: None,
            responsable_id: None,
            nom_retrait: String::new(),
            signataire: String::new(),
            remis_le: None,
            signe_le: None,
            remis_par: None,
            publie_le: contexte.maintenant,
        },
    };
    remises_documents::enregistrer(cnx, &remise, contexte).await?;
    for mut ancienne in
        remises_documents::de_l_inscription(cnx, bulletin.inscription_id, contexte).await?
    {
        if ancienne.id != remise.id && ancienne.statut == StatutRemiseDocument::ARemettre {
            ancienne.statut = StatutRemiseDocument::NonRetire;
            remises_documents::enregistrer(cnx, &ancienne, contexte).await?;
        }
    }
    Ok(())
}

/// Une remise avec l'élève, sa classe, la période, ses responsables et l'impayé de la famille.
pub(crate) async fn afficher(
    cnx: &mut Connexion,
    remise: RemiseDocument,
    contexte: &Contexte,
) -> Resultat<LigneRemise> {
    let inscription = inscriptions::lire(cnx, remise.inscription_id, contexte).await?;
    let (_, personne) = personnes::eleve(cnx, inscription.eleve_id, contexte).await?;
    let classe = academique::classe(cnx, inscription.classe_id, contexte).await?;
    let periode = pedagogie::periodes(cnx, inscription.annee_id, contexte)
        .await?
        .into_iter()
        .find(|p| p.id == remise.periode_id)
        .map(|p| p.libelle())
        .unwrap_or_default();
    let mut responsables = Vec::new();
    for (_, id, qualites) in
        personnes::liens_de_l_eleve(cnx, inscription.eleve_id, contexte).await?
    {
        let (_, p) = personnes::responsable(cnx, id, contexte).await?;
        responsables.push(ResponsableRemise {
            id,
            nom: p.nom_complet(),
            lien: qualites.lien.nom().to_owned(),
        });
    }
    let recu_par = match remise.remis_a {
        Some(RemisA::Responsable) => remise
            .responsable_id
            .and_then(|id| responsables.iter().find(|r| r.id == id))
            .map(|r| r.nom.clone())
            .unwrap_or_default(),
        Some(RemisA::Eleve) => personne.nom_complet(),
        Some(RemisA::PersonneAutorisee | RemisA::Tiers) => remise.nom_retrait.clone(),
        None => String::new(),
    };
    let impaye_echu = match (
        factures::facture_vivante(cnx, inscription.id, contexte).await?,
        DateCivile::du_jour(contexte.maintenant),
    ) {
        (Some(facture), Some(aujourd_hui)) => {
            let echeances: Vec<_> = crate::facturation::etat_echeances(cnx, facture.id, contexte)
                .await?
                .iter()
                .map(|e| (e.echeance.date, e.reste))
                .collect();
            impaye(&echeances, aujourd_hui)
                .ok()
                .flatten()
                .map(|i| i.echu)
        }
        _ => None,
    };
    let (jours, a_signaler) = attente(&remise, contexte.maintenant);
    Ok(LigneRemise {
        eleve_id: inscription.eleve_id,
        eleve: personne.nom_complet(),
        classe: classe.libelle,
        periode,
        responsables,
        recu_par,
        jours,
        a_signaler,
        impaye_echu,
        remise,
    })
}

/// Service de la remise des documents.
pub struct ServiceRemisesBulletins<'n> {
    noyau: &'n Noyau,
}

impl<'n> ServiceRemisesBulletins<'n> {
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

    async fn droits(
        cnx: &mut Connexion,
        session: &Session,
        contexte: &Contexte,
    ) -> Resultat<DroitsRemise> {
        let p = role::permissions_de(cnx, session.utilisateur_id, contexte).await?;
        Ok(DroitsRemise {
            remettre: p.contains(&Permission::DOCUMENTS_DELIVER),
        })
    }

    /// Les remises d'une classe pour une période, par élève.
    pub async fn classe(
        &self,
        session: &Session,
        demande: &DemandeRemisesClasse,
    ) -> Resultat<RemisesClasse> {
        let contexte = self.noyau.contexte(session.etablissement_id, Some(session));
        let mut cnx = self.connexion().await?;
        let mut lignes = Vec::new();
        for r in remises_documents::de_la_periode(&mut cnx, demande.periode_id, &contexte).await? {
            if inscriptions::lire(&mut cnx, r.inscription_id, &contexte)
                .await?
                .classe_id
                == demande.classe_id
            {
                lignes.push(afficher(&mut cnx, r, &contexte).await?);
            }
        }
        lignes.sort_by(|a, b| a.eleve.cmp(&b.eleve));
        Ok(RemisesClasse {
            lignes,
            droits: Self::droits(&mut cnx, session, &contexte).await?,
        })
    }

    /// Les bulletins à signaler : non retirés, ou à remettre depuis sept jours, les plus anciens
    /// d'abord, avec l'impayé de la famille.
    pub async fn non_retires(&self, session: &Session) -> Resultat<RemisesClasse> {
        let contexte = self.noyau.contexte(session.etablissement_id, Some(session));
        let mut cnx = self.connexion().await?;
        let mut lignes = Vec::new();
        for r in remises_documents::en_attente(&mut cnx, &contexte).await? {
            if attente(&r, contexte.maintenant).1 {
                lignes.push(afficher(&mut cnx, r, &contexte).await?);
            }
        }
        Ok(RemisesClasse {
            lignes,
            droits: Self::droits(&mut cnx, session, &contexte).await?,
        })
    }

    async fn ecrire(
        &self,
        session: &Session,
        avant: &RemiseDocument,
        apres: &RemiseDocument,
        evenement: TypeEvenement,
        operation: &'static str,
    ) -> Resultat<LigneRemise> {
        let contexte = self.noyau.contexte(session.etablissement_id, Some(session));
        let mut unite = self
            .noyau
            .commencer(contexte.clone(), Durabilite::Ordinaire)
            .await?;
        remises_documents::enregistrer(unite.connexion(), apres, &contexte).await?;
        let id = apres.id.vers_chaine();
        unite.auditer(
            "remise_document",
            &id,
            Operation::Modification,
            Some(serde_json::to_value(avant)?),
            Some(serde_json::to_value(apres)?),
            operation,
        );
        unite.emettre(
            evenement,
            "remise_document",
            &id,
            serde_json::json!({ "inscription_id": apres.inscription_id, "statut": apres.statut }),
        );
        let ligne = afficher(unite.connexion(), apres.clone(), &contexte).await?;
        self.noyau.terminer(unite).await?;
        Ok(ligne)
    }

    /// Remet un bulletin : à un responsable de l'élève, à l'élève, à une personne autorisée ou à
    /// un tiers nommé ; signé sur place ou non.
    pub async fn remettre(
        &self,
        session: &Session,
        demande: &DemandeRemiseBulletin,
    ) -> Resultat<LigneRemise> {
        verifier_remise(
            demande.remis_a,
            demande.responsable_id.is_some(),
            &demande.nom_retrait,
            demande.signe,
            &demande.signataire,
        )
        .map_err(regle)?;
        let contexte = self.noyau.contexte(session.etablissement_id, Some(session));
        let avant = {
            let mut cnx = self.connexion().await?;
            let r = remises_documents::remise(&mut cnx, demande.id, &contexte).await?;
            if let (RemisA::Responsable, Some(responsable_id)) =
                (demande.remis_a, demande.responsable_id)
            {
                let eleve = inscriptions::lire(&mut cnx, r.inscription_id, &contexte)
                    .await?
                    .eleve_id;
                if !personnes::liens_de_l_eleve(&mut cnx, eleve, &contexte)
                    .await?
                    .iter()
                    .any(|(_, id, _)| *id == responsable_id)
                {
                    return Err(regle("remise.responsable_inconnu"));
                }
            }
            r
        };
        if !avant.statut.en_attente() {
            return Err(regle("remise.deja_remis"));
        }
        let maintenant = self.noyau.horloge().maintenant();
        let apres = RemiseDocument {
            statut: if demande.signe {
                StatutRemiseDocument::Signe
            } else {
                StatutRemiseDocument::Remis
            },
            remis_a: Some(demande.remis_a),
            responsable_id: (demande.remis_a == RemisA::Responsable)
                .then_some(demande.responsable_id)
                .flatten(),
            nom_retrait: demande.nom_retrait.trim().to_owned(),
            signataire: demande.signataire.trim().to_owned(),
            remis_le: Some(maintenant),
            signe_le: demande.signe.then_some(maintenant),
            remis_par: Some(session.utilisateur_id),
            ..avant.clone()
        };
        self.ecrire(
            session,
            &avant,
            &apres,
            TypeEvenement::DocumentRemis,
            "remises.remettre",
        )
        .await
    }

    /// Fait signer un bulletin déjà remis.
    pub async fn signer(
        &self,
        session: &Session,
        demande: &DemandeSignatureBulletin,
    ) -> Resultat<LigneRemise> {
        let signataire = demande.signataire.trim();
        if signataire.is_empty() {
            return Err(regle("remise.signataire_exige"));
        }
        let contexte = self.noyau.contexte(session.etablissement_id, Some(session));
        let avant = {
            let mut cnx = self.connexion().await?;
            remises_documents::remise(&mut cnx, demande.id, &contexte).await?
        };
        if avant.statut != StatutRemiseDocument::Remis {
            return Err(regle("remise.pas_remis"));
        }
        let apres = RemiseDocument {
            statut: StatutRemiseDocument::Signe,
            signataire: signataire.to_owned(),
            signe_le: Some(self.noyau.horloge().maintenant()),
            ..avant.clone()
        };
        self.ecrire(
            session,
            &avant,
            &apres,
            TypeEvenement::DocumentSigne,
            "remises.signer",
        )
        .await
    }

    /// Renvoie l'avis « bulletin à retirer » d'un bulletin pas encore remis ; rend le nombre de
    /// messages mis en file.
    pub async fn renvoyer_avis(
        &self,
        session: &Session,
        demande: &DemandeRemiseId,
    ) -> Resultat<u32> {
        let contexte = self.noyau.contexte(session.etablissement_id, Some(session));
        let mut unite = self
            .noyau
            .commencer(contexte.clone(), Durabilite::Ordinaire)
            .await?;
        let r = remises_documents::remise(unite.connexion(), demande.id, &contexte).await?;
        if !r.statut.en_attente() {
            return Err(regle("remise.deja_remis"));
        }
        let mis = crate::messages_auto::rappel_bulletin(unite.connexion(), &r, &contexte).await?;
        if mis == 0 {
            return Err(regle("remise.aucun_destinataire"));
        }
        let id = r.id.vers_chaine();
        unite.emettre(
            TypeEvenement::AvisBulletinRenvoye,
            "remise_document",
            &id,
            serde_json::json!({ "messages": mis }),
        );
        self.noyau.terminer(unite).await?;
        Ok(mis)
    }
}
