//! Convocations (SPEC_PHASE_5 §7, blueprint §8.5bis) : un responsable convoqué au sujet d'un
//! élève, un motif, un rendez-vous, le canal par lequel elle est partie et ce que l'école sait
//! de sa réception, puis sa suite. Confiée à l'élève, elle est « transmise, non confirmée » ; par
//! SMS, le message est préparé et attend la relecture d'une personne. Un billet s'imprime.

use gesco_core::commun::{DateCivile, Id};
use gesco_core::core::{Operation, Permission, TypeEvenement};
use gesco_core::notification::EtatMessage;
use gesco_core::scolarite::convocation::{
    AccuseConvocation, CanalConvocation, Convocation, SuiteConvocation, heure_lisible,
    verifier_convocation, verifier_suite,
};
use gesco_core::scolarite::discipline::{NatureSanction, Sanction};
use gesco_core::scolarite::{Inscription, Responsable};
use gesco_db::depots::{
    academique, discipline, file_sms, inscriptions, personnes, remises_documents, role, utilisateur,
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

/// Convocation, telle que l'écran la montre.
#[derive(Debug, Clone, Serialize, specta::Type)]
pub struct ConvocationAffichee {
    /// Convocation.
    pub convocation: Convocation,
    /// Prénoms et nom de l'élève.
    pub eleve: String,
    /// Classe.
    pub classe: String,
    /// Responsable convoqué ; vide : les responsables légaux.
    pub responsable: String,
    /// Qui l'a émise.
    pub emise_par: String,
}

/// Ce que peut faire le compte connecté.
#[derive(Debug, Clone, Copy, Serialize, specta::Type)]
pub struct DroitsConvocations {
    /// Convoquer, imprimer le billet.
    pub creer: bool,
    /// Noter la réception, reporter, annuler.
    pub modifier: bool,
    /// Noter la suite : honorée, non honorée.
    pub clore: bool,
}

/// Les convocations.
#[derive(Debug, Clone, Serialize, specta::Type)]
pub struct Convocations {
    /// Convocations, les rendez-vous les plus proches d'abord.
    pub liste: Vec<ConvocationAffichee>,
    /// Droits de qui consulte.
    pub droits: DroitsConvocations,
}

/// Convocations à lire.
#[derive(Debug, Deserialize, specta::Type)]
pub struct DemandeConvocations {
    /// Vrai : seulement celles en cours.
    pub en_cours: bool,
}

/// Convocation à émettre.
#[derive(Debug, Deserialize, specta::Type)]
pub struct DemandeConvocation {
    /// Inscription de l'élève.
    pub inscription_id: Id<Inscription>,
    /// Responsable convoqué ; aucun : les responsables légaux.
    pub responsable_id: Option<Id<Responsable>>,
    /// Motif.
    pub motif: String,
    /// Jour du rendez-vous, `AAAA-MM-JJ`.
    pub date: String,
    /// Heure, `HH:MM`.
    pub heure: String,
    /// Canal.
    pub canal: CanalConvocation,
    /// Sanction « convocation des parents » dont elle naît.
    pub sanction_id: Option<Id<Sanction>>,
}

/// Réception notée.
#[derive(Debug, Deserialize, specta::Type)]
pub struct DemandeAccuse {
    /// Convocation.
    pub id: Id<Convocation>,
    /// Réception.
    pub accuse: AccuseConvocation,
}

/// Rendez-vous déplacé.
#[derive(Debug, Deserialize, specta::Type)]
pub struct DemandeReport {
    /// Convocation.
    pub id: Id<Convocation>,
    /// Nouveau jour, `AAAA-MM-JJ`.
    pub date: String,
    /// Nouvelle heure, `HH:MM`.
    pub heure: String,
}

/// Suite définitive.
#[derive(Debug, Deserialize, specta::Type)]
pub struct DemandeClotureConvocation {
    /// Convocation.
    pub id: Id<Convocation>,
    /// Vrai : le responsable est venu.
    pub honoree: bool,
}

/// Annulation motivée.
#[derive(Debug, Deserialize, specta::Type)]
pub struct DemandeAnnulationConvocation {
    /// Convocation.
    pub id: Id<Convocation>,
    /// Motif.
    pub motif: String,
}

/// Une convocation.
#[derive(Debug, Deserialize, specta::Type)]
pub struct DemandeConvocationId {
    /// Convocation.
    pub id: Id<Convocation>,
}

/// Une convocation avec l'élève, sa classe, le responsable et qui l'a émise.
pub(crate) async fn afficher(
    cnx: &mut Connexion,
    convocation: Convocation,
    contexte: &Contexte,
) -> Resultat<ConvocationAffichee> {
    let inscription = inscriptions::lire(cnx, convocation.inscription_id, contexte).await?;
    let (_, eleve) = personnes::eleve(cnx, inscription.eleve_id, contexte).await?;
    let classe = academique::classe(cnx, inscription.classe_id, contexte).await?;
    let responsable = match convocation.responsable_id {
        Some(id) => personnes::responsable(cnx, id, contexte)
            .await
            .map(|(_, p)| p.nom_complet())
            .unwrap_or_default(),
        None => String::new(),
    };
    let emise_par = utilisateur::lire(cnx, convocation.emise_par, contexte)
        .await
        .map(|u| u.nom_affiche)
        .unwrap_or_default();
    Ok(ConvocationAffichee {
        eleve: eleve.nom_complet(),
        classe: classe.libelle,
        responsable,
        emise_par,
        convocation,
    })
}

/// Service des convocations.
pub struct ServiceConvocations<'n> {
    noyau: &'n Noyau,
}

impl<'n> ServiceConvocations<'n> {
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

    fn aujourd_hui(&self) -> Resultat<DateCivile> {
        DateCivile::du_jour(self.noyau.horloge().maintenant())
            .ok_or(regle("convocation.date_invalide"))
    }

    /// Les convocations, toutes ou seulement celles en cours.
    pub async fn lister(
        &self,
        session: &Session,
        demande: &DemandeConvocations,
    ) -> Resultat<Convocations> {
        let contexte = self.noyau.contexte(session.etablissement_id, Some(session));
        let mut cnx = self.connexion().await?;
        let mut liste = Vec::new();
        for c in remises_documents::toutes(&mut cnx, &contexte).await? {
            if !demande.en_cours || c.en_cours() {
                liste.push(afficher(&mut cnx, c, &contexte).await?);
            }
        }
        let p = role::permissions_de(&mut cnx, session.utilisateur_id, &contexte).await?;
        Ok(Convocations {
            liste,
            droits: DroitsConvocations {
                creer: p.contains(&Permission::CONVOCATIONS_CREATE),
                modifier: p.contains(&Permission::CONVOCATIONS_UPDATE),
                clore: p.contains(&Permission::CONVOCATIONS_CLOSE),
            },
        })
    }

    /// Émet une convocation ; par SMS, le message est préparé et attend la relecture d'une
    /// personne.
    pub async fn creer(
        &self,
        session: &Session,
        demande: &DemandeConvocation,
    ) -> Resultat<ConvocationAffichee> {
        let date = DateCivile::analyser(&demande.date).ok_or(regle("convocation.date_invalide"))?;
        verifier_convocation(&demande.motif, &demande.heure, date, self.aujourd_hui()?)
            .map_err(regle)?;
        crate::annees::exiger_inscription_modifiable(self.noyau, session, demande.inscription_id)
            .await?;
        let contexte = self.noyau.contexte(session.etablissement_id, Some(session));
        let mut unite = self
            .noyau
            .commencer(contexte.clone(), Durabilite::Ordinaire)
            .await?;
        let inscription =
            inscriptions::lire(unite.connexion(), demande.inscription_id, &contexte).await?;
        if let Some(responsable_id) = demande.responsable_id
            && !personnes::liens_de_l_eleve(unite.connexion(), inscription.eleve_id, &contexte)
                .await?
                .iter()
                .any(|(_, id, _)| *id == responsable_id)
        {
            return Err(regle("convocation.responsable_inconnu"));
        }
        if let Some(sanction_id) = demande.sanction_id {
            let s = discipline::sanction(unite.connexion(), sanction_id, &contexte).await?;
            if s.inscription_id != inscription.id || s.nature != NatureSanction::ConvocationParents
            {
                return Err(regle("convocation.sanction_invalide"));
            }
        }
        if demande.canal == CanalConvocation::Sms
            && crate::file_sms::canal_en_vigueur(unite.connexion(), &contexte).await? == "aucun"
        {
            return Err(regle("convocation.sms_sans_canal"));
        }
        let convocation = Convocation {
            id: nouvel_id(),
            etablissement_id: session.etablissement_id,
            inscription_id: inscription.id,
            responsable_id: demande.responsable_id,
            sanction_id: demande.sanction_id,
            motif: demande.motif.trim().to_owned(),
            date,
            heure: demande.heure.clone(),
            canal: demande.canal,
            accuse: demande.canal.accuse_initial(),
            suite: SuiteConvocation::EnAttente,
            emise_le: contexte.maintenant,
            emise_par: session.utilisateur_id,
            annulee: false,
            motif_annulation: String::new(),
        };
        remises_documents::enregistrer_convocation(unite.connexion(), &convocation, &contexte)
            .await?;
        if convocation.canal == CanalConvocation::Sms
            && crate::messages_auto::preparer_convocation(
                unite.connexion(),
                &convocation,
                &contexte,
            )
            .await?
                == 0
        {
            return Err(regle("convocation.aucun_destinataire"));
        }
        let id = convocation.id.vers_chaine();
        unite.auditer(
            "convocation",
            &id,
            Operation::Creation,
            None,
            Some(serde_json::to_value(&convocation)?),
            "convocations.creer",
        );
        unite.emettre(
            TypeEvenement::ConvocationEmise,
            "convocation",
            &id,
            serde_json::json!({ "inscription_id": convocation.inscription_id, "canal": convocation.canal }),
        );
        let affichee = afficher(unite.connexion(), convocation, &contexte).await?;
        self.noyau.terminer(unite).await?;
        Ok(affichee)
    }

    async fn modifier(
        &self,
        session: &Session,
        id: Id<Convocation>,
        changer: impl FnOnce(&Convocation) -> Result<Convocation, &'static str>,
        evenement: TypeEvenement,
        operation: &'static str,
    ) -> Resultat<ConvocationAffichee> {
        let contexte = self.noyau.contexte(session.etablissement_id, Some(session));
        let mut unite = self
            .noyau
            .commencer(contexte.clone(), Durabilite::Ordinaire)
            .await?;
        let avant = remises_documents::convocation(unite.connexion(), id, &contexte).await?;
        let apres = changer(&avant).map_err(regle)?;
        remises_documents::enregistrer_convocation(unite.connexion(), &apres, &contexte).await?;
        if apres.annulee {
            // Le SMS préparé qui n'est pas parti s'annule avec la convocation.
            for mut m in file_sms::par_fait(
                unite.connexion(),
                &format!("convocation:{}", apres.id),
                &contexte,
            )
            .await?
            {
                if m.etat.annulable() {
                    m.etat = EtatMessage::Annule;
                    m.motif_annulation = "Convocation annulée".to_owned();
                    file_sms::mettre_a_jour(unite.connexion(), &m, &contexte).await?;
                }
            }
        }
        let cle = apres.id.vers_chaine();
        unite.auditer(
            "convocation",
            &cle,
            Operation::Modification,
            Some(serde_json::json!({ "accuse": avant.accuse, "suite": avant.suite, "date": avant.date, "heure": avant.heure })),
            Some(serde_json::json!({ "accuse": apres.accuse, "suite": apres.suite, "date": apres.date, "heure": apres.heure, "annulee": apres.annulee })),
            operation,
        );
        unite.emettre(
            evenement,
            "convocation",
            &cle,
            serde_json::json!({ "suite": apres.suite, "accuse": apres.accuse }),
        );
        let affichee = afficher(unite.connexion(), apres, &contexte).await?;
        self.noyau.terminer(unite).await?;
        Ok(affichee)
    }

    /// Note ce que l'école sait de la réception.
    pub async fn accuser(
        &self,
        session: &Session,
        demande: &DemandeAccuse,
    ) -> Resultat<ConvocationAffichee> {
        let accuse = demande.accuse;
        self.modifier(
            session,
            demande.id,
            |c| {
                if c.annulee {
                    return Err("convocation.close");
                }
                Ok(Convocation {
                    accuse,
                    ..c.clone()
                })
            },
            TypeEvenement::ConvocationAccusee,
            "convocations.accuser",
        )
        .await
    }

    /// Déplace le rendez-vous.
    pub async fn reporter(
        &self,
        session: &Session,
        demande: &DemandeReport,
    ) -> Resultat<ConvocationAffichee> {
        let date = DateCivile::analyser(&demande.date).ok_or(regle("convocation.date_invalide"))?;
        let aujourd_hui = self.aujourd_hui()?;
        let heure = demande.heure.clone();
        self.modifier(
            session,
            demande.id,
            |c| {
                verifier_suite(c, SuiteConvocation::Reportee)?;
                verifier_convocation(&c.motif, &heure, date, aujourd_hui)?;
                Ok(Convocation {
                    date,
                    heure,
                    suite: SuiteConvocation::Reportee,
                    ..c.clone()
                })
            },
            TypeEvenement::ConvocationReportee,
            "convocations.reporter",
        )
        .await
    }

    /// Note la suite définitive : honorée ou non.
    pub async fn clore(
        &self,
        session: &Session,
        demande: &DemandeClotureConvocation,
    ) -> Resultat<ConvocationAffichee> {
        let suite = if demande.honoree {
            SuiteConvocation::Honoree
        } else {
            SuiteConvocation::NonHonoree
        };
        self.modifier(
            session,
            demande.id,
            |c| {
                verifier_suite(c, suite)?;
                let accuse = if demande.honoree {
                    AccuseConvocation::Confirme
                } else {
                    c.accuse
                };
                Ok(Convocation {
                    suite,
                    accuse,
                    ..c.clone()
                })
            },
            TypeEvenement::ConvocationCloturee,
            "convocations.clore",
        )
        .await
    }

    /// Annule une convocation en cours, avec un motif ; son SMS préparé s'annule aussi.
    pub async fn annuler(
        &self,
        session: &Session,
        demande: &DemandeAnnulationConvocation,
    ) -> Resultat<ConvocationAffichee> {
        let motif = demande.motif.trim().to_owned();
        if motif.is_empty() {
            return Err(regle("convocation.motif_annulation_obligatoire"));
        }
        self.modifier(
            session,
            demande.id,
            |c| {
                if !c.en_cours() {
                    return Err("convocation.close");
                }
                Ok(Convocation {
                    annulee: true,
                    motif_annulation: motif,
                    ..c.clone()
                })
            },
            TypeEvenement::ConvocationAnnulee,
            "convocations.annuler",
        )
        .await
    }

    /// Données du billet de convocation imprimé.
    pub async fn donnees_billet(
        &self,
        session: &Session,
        demande: &DemandeConvocationId,
    ) -> Resultat<serde_json::Value> {
        let contexte = self.noyau.contexte(session.etablissement_id, Some(session));
        let mut cnx = self.connexion().await?;
        let c = remises_documents::convocation(&mut cnx, demande.id, &contexte).await?;
        let affichee = afficher(&mut cnx, c, &contexte).await?;
        let c = &affichee.convocation;
        let jour = gesco_core::commun::Horodatage::depuis_millis_epoque(
            c.date.jours_depuis_epoque() * 86_400_000,
        );
        Ok(serde_json::json!({
            "eleve": affichee.eleve,
            "classe": affichee.classe,
            "responsable": if affichee.responsable.is_empty() { "Les parents".to_owned() } else { affichee.responsable.clone() },
            "motif": c.motif,
            "date": crate::documents::date_longue(jour),
            "heure": heure_lisible(&c.heure),
            "emise_le": crate::documents::date_longue(c.emise_le),
            "emise_par": affichee.emise_par,
        }))
    }
}
