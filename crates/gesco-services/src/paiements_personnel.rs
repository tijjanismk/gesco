//! Avances et paiements du personnel (SPEC_PHASE_7 §4 et §7) : une avance se verse comme un
//! salaire et se rembourse sur les paies validées ; le net d'un bulletin validé se paie en une ou
//! plusieurs fois, en espèces de la caisse ou hors caisse. Rien ne bloque un paiement (T-58) :
//! un tiroir qui ne suffit pas avertit seulement ; seul le dépassement du reste dû est refusé.

use gesco_core::commun::{DateCivile, Id, Montant};
use gesco_core::core::{MachineAEtats, Operation, TypeEvenement};
use gesco_core::finance::{ModePaiement, MouvementCaisse, NatureMouvement, SessionCaisse};
use gesco_core::paie::avance::verifier_avance;
use gesco_core::paie::paiement::verifier_paiement;
use gesco_core::paie::{
    Avance, BulletinPaie, ObjetPaiement, PaiementPersonnel, SourcePaiement, StatutAvance,
    StatutPaie,
};
use gesco_core::rh::{MembrePersonnel, Mois, MoyenPaiement};
use gesco_db::depots::{caisse, paie, personnel, personnes};
use gesco_db::nouvel_id;
use gesco_db::unite::Durabilite;
use gesco_db::{Contexte, UniteDeTravail};
use serde::{Deserialize, Serialize};

use crate::caisse::ServiceCaisse;
use crate::effets::Effet;
use crate::erreur::{ErreurService, Resultat};
use crate::noyau::Noyau;
use crate::session::Session;

fn regle(cle: &'static str) -> ErreurService {
    ErreurService::Regle { cle }
}

/// Bulletin validé qui reste à payer, en tout ou en partie.
#[derive(Debug, Clone, Serialize, specta::Type)]
pub struct SalaireAPayer {
    /// Bulletin.
    pub bulletin_id: Id<BulletinPaie>,
    /// Numéro du bulletin.
    pub numero: String,
    /// Mois de la paie.
    pub mois: Mois,
    /// Membre.
    pub membre_id: Id<MembrePersonnel>,
    /// Prénoms et nom.
    pub membre: String,
    /// Net à payer.
    pub net: Montant,
    /// Déjà payé.
    pub paye: Montant,
    /// Reste à payer.
    pub reste: Montant,
    /// Mode proposé, d'après la fiche du membre.
    pub mode_propose: ModePaiement,
}

/// Avance affichée, avec ce qu'elle a déjà remboursé.
#[derive(Debug, Clone, Serialize, specta::Type)]
pub struct AvanceAffichee {
    /// Avance.
    pub avance: Avance,
    /// Prénoms et nom du membre.
    pub membre: String,
    /// Mensualité prévue.
    pub mensualite: Montant,
    /// Remboursé par les paies validées.
    pub rembourse: Montant,
    /// Reste dû.
    pub reste: Montant,
}

/// Paiement affiché.
#[derive(Debug, Clone, Serialize, specta::Type)]
pub struct PaiementAffiche {
    /// Paiement.
    pub paiement: PaiementPersonnel,
    /// Prénoms et nom du membre.
    pub membre: String,
    /// Mois du bulletin payé, pour un salaire.
    pub mois: Option<Mois>,
}

/// Ce que l'écran des paiements montre.
#[derive(Debug, Clone, Serialize, specta::Type)]
pub struct PaiementsPersonnel {
    /// Salaires encore dus, tous mois confondus, du plus ancien au plus récent.
    pub a_payer: Vec<SalaireAPayer>,
    /// Total encore dû.
    pub total_du: Montant,
    /// Avances, en cours d'abord.
    pub avances: Vec<AvanceAffichee>,
    /// Paiements, du plus récent au plus ancien.
    pub paiements: Vec<PaiementAffiche>,
    /// Membres encore présents, pour une avance : identifiant et nom.
    pub membres: Vec<(Id<MembrePersonnel>, String)>,
    /// Session de caisse ouverte de qui consulte, et ce qu'il y a dans le tiroir.
    pub caisse_ouverte: Option<Montant>,
}

/// Résultat d'un paiement : l'état mis à jour et les avertissements.
#[derive(Debug, Clone, Serialize, specta::Type)]
pub struct PaiementFait {
    /// État des paiements.
    pub etat: PaiementsPersonnel,
    /// Avertissements : clés des messages.
    pub avertissements: Vec<String>,
}

/// Paiement d'un salaire.
#[derive(Debug, Deserialize, specta::Type)]
pub struct DemandePaiementSalaire {
    /// Bulletin validé.
    pub bulletin_id: Id<BulletinPaie>,
    /// Montant ; le reste du bulletin au plus.
    pub montant: Montant,
    /// Mode.
    pub mode: ModePaiement,
    /// Référence, hors espèces.
    pub reference: String,
    /// Source.
    pub source: SourcePaiement,
}

/// Avance accordée et versée.
#[derive(Debug, Deserialize, specta::Type)]
pub struct DemandeAvance {
    /// Membre.
    pub membre_id: Id<MembrePersonnel>,
    /// Montant.
    pub montant: Montant,
    /// Nombre de mois de remboursement.
    pub mensualites: i64,
    /// Premier mois retenu.
    pub premier_mois: Mois,
    /// Motif.
    pub motif: String,
    /// Mode du versement.
    pub mode: ModePaiement,
    /// Référence, hors espèces.
    pub reference: String,
    /// Source du versement.
    pub source: SourcePaiement,
}

/// Annulation d'un paiement ou d'une avance, toujours motivée.
#[derive(Debug, Deserialize, specta::Type)]
pub struct DemandeAnnulationPaiement {
    /// Paiement.
    pub id: Id<PaiementPersonnel>,
    /// Motif.
    pub motif: String,
}

/// Avance désignée, à annuler avec un motif.
#[derive(Debug, Deserialize, specta::Type)]
pub struct DemandeAnnulationAvance {
    /// Avance.
    pub id: Id<Avance>,
    /// Motif.
    pub motif: String,
}

/// Mois dont le reste se paie d'un geste.
#[derive(Debug, Deserialize, specta::Type)]
pub struct DemandeResteDuMois {
    /// Mois.
    pub mois: Mois,
}

/// Service des paiements du personnel.
pub struct ServicePaiementsPersonnel<'n> {
    noyau: &'n Noyau,
}

impl<'n> ServicePaiementsPersonnel<'n> {
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

    /// Salaires à payer, avances et paiements.
    pub async fn lire(&self, session: &Session) -> Resultat<PaiementsPersonnel> {
        let contexte = self.noyau.contexte(session.etablissement_id, Some(session));
        let mut cnx = self.connexion().await?;
        let membres = personnel::membres(&mut cnx, &contexte).await?;
        let mut noms = Vec::with_capacity(membres.len());
        for m in &membres {
            let p = personnes::personne(&mut cnx, m.personne_id, &contexte).await?;
            noms.push((m.id, p.nom_complet()));
        }
        let nom = |id: Id<MembrePersonnel>| {
            noms.iter()
                .find(|(m, _)| *m == id)
                .map(|(_, n)| n.clone())
                .unwrap_or_default()
        };
        let paiements = paie::paiements(&mut cnx, &contexte).await?;
        let mut a_payer = Vec::new();
        let mut mois_des_bulletins = Vec::new();
        for p in paie::paies(&mut cnx, &contexte)
            .await?
            .into_iter()
            .filter(|p| p.statut == StatutPaie::Validee)
        {
            for b in paie::bulletins_de_la_paie(&mut cnx, p.id, &contexte)
                .await?
                .into_iter()
                .filter(|b| !b.exclu)
            {
                mois_des_bulletins.push((b.id, p.mois));
                let paye = paye_sur(&paiements, |x| x.bulletin_id == Some(b.id))?;
                let reste = b
                    .resultat
                    .net
                    .soustraire(paye)
                    .map_err(|_| regle("paiement.calcul_impossible"))?;
                if reste.francs() > 0 {
                    let membre = membres.iter().find(|m| m.id == b.membre_id);
                    a_payer.push(SalaireAPayer {
                        bulletin_id: b.id,
                        numero: b.numero.clone().unwrap_or_default(),
                        mois: p.mois,
                        membre_id: b.membre_id,
                        membre: nom(b.membre_id),
                        net: b.resultat.net,
                        paye,
                        reste,
                        mode_propose: membre.map_or(ModePaiement::Especes, mode_de),
                    });
                }
            }
        }
        let total_du = a_payer
            .iter()
            .try_fold(Montant::ZERO, |s, a| s.ajouter(a.reste))
            .map_err(|_| regle("paiement.calcul_impossible"))?;
        let rembourses = paie::rembourse_par_avance(&mut cnx, &contexte).await?;
        let mut avances: Vec<AvanceAffichee> = paie::avances(&mut cnx, &contexte)
            .await?
            .into_iter()
            .map(|a| {
                let rembourse = rembourses
                    .iter()
                    .find(|(id, _)| *id == a.id)
                    .map_or(Montant::ZERO, |(_, m)| *m);
                Ok(AvanceAffichee {
                    membre: nom(a.membre_id),
                    mensualite: a.mensualite(),
                    reste: a
                        .montant
                        .soustraire(rembourse)
                        .map_err(|_| regle("paiement.calcul_impossible"))?,
                    rembourse,
                    avance: a,
                })
            })
            .collect::<Resultat<_>>()?;
        avances.sort_by_key(|a| a.avance.statut != StatutAvance::EnCours);
        let mut affiches: Vec<PaiementAffiche> = paiements
            .into_iter()
            .map(|p| PaiementAffiche {
                membre: nom(p.membre_id),
                mois: p.bulletin_id.and_then(|b| {
                    mois_des_bulletins
                        .iter()
                        .find(|(id, _)| *id == b)
                        .map(|(_, m)| *m)
                }),
                paiement: p,
            })
            .collect();
        affiches.reverse();
        let ouverte = caisse::ouverte_de(&mut cnx, session.utilisateur_id, &contexte).await?;
        drop(cnx);
        let caisse_ouverte = match ouverte {
            Some(s) => Some(
                ServiceCaisse::nouveau(self.noyau)
                    .totaux(&s)
                    .await?
                    .theorique,
            ),
            None => None,
        };
        let presents = membres
            .iter()
            .filter(|m| m.parti_le.is_none())
            .map(|m| (m.id, nom(m.id)))
            .collect();
        Ok(PaiementsPersonnel {
            a_payer,
            total_du,
            avances,
            paiements: affiches,
            membres: presents,
            caisse_ouverte,
        })
    }

    /// Paie tout ou partie du net d'un bulletin validé.
    pub async fn payer_salaire(
        &self,
        session: &Session,
        demande: &DemandePaiementSalaire,
    ) -> Resultat<PaiementFait> {
        let etat = self.lire(session).await?;
        let du = etat
            .a_payer
            .iter()
            .find(|a| a.bulletin_id == demande.bulletin_id)
            .ok_or(regle("paiement.rien_a_payer"))?;
        verifier_paiement(
            demande.montant,
            du.reste,
            demande.mode,
            &demande.reference,
            demande.source,
        )
        .map_err(regle)?;
        let contexte = self.noyau.contexte(session.etablissement_id, Some(session));
        let mut unite = self
            .noyau
            .commencer(contexte.clone(), Durabilite::Critique)
            .await?;
        let mut avertissements = Vec::new();
        let paiement = PaiementPersonnel {
            id: nouvel_id(),
            etablissement_id: session.etablissement_id,
            membre_id: du.membre_id,
            objet: ObjetPaiement::Salaire,
            bulletin_id: Some(du.bulletin_id),
            avance_id: None,
            montant: demande.montant,
            le: contexte.maintenant,
            mode: demande.mode,
            reference: demande.reference.trim().to_owned(),
            source: demande.source,
            session_id: None,
            par: session.utilisateur_id,
            annule: false,
            motif_annulation: String::new(),
        };
        let motif = format!("Salaire de {} · {}", du.membre, du.numero);
        let paiement = self
            .enregistrer(
                &mut unite,
                session,
                paiement,
                &motif,
                &etat,
                &mut avertissements,
            )
            .await?;
        unite.emettre(
            TypeEvenement::SalairePaye,
            "paiement_personnel",
            &paiement.id.vers_chaine(),
            serde_json::json!({
                "montant": paiement.montant, "mode": paiement.mode, "source": paiement.source,
                "membre_id": paiement.membre_id, "mois": du.mois,
            }),
        );
        self.noyau.terminer(unite).await?;
        Ok(PaiementFait {
            etat: self.lire(session).await?,
            avertissements,
        })
    }

    /// Paie d'un geste le reste de chaque bulletin validé du mois, selon le mode de chacun : les
    /// espèces sortent de la caisse ouverte de qui paie tant que le tiroir les couvre, hors caisse
    /// au-delà ou sans caisse ouverte, ce que GESCO signale.
    pub async fn payer_reste_du_mois(
        &self,
        session: &Session,
        demande: &DemandeResteDuMois,
    ) -> Resultat<PaiementFait> {
        let etat = self.lire(session).await?;
        let dus: Vec<&SalaireAPayer> = etat
            .a_payer
            .iter()
            .filter(|a| a.mois == demande.mois)
            .collect();
        if dus.is_empty() {
            return Err(regle("paiement.rien_a_payer"));
        }
        let contexte = self.noyau.contexte(session.etablissement_id, Some(session));
        let mut unite = self
            .noyau
            .commencer(contexte.clone(), Durabilite::Critique)
            .await?;
        let mut avertissements = Vec::new();
        let mut tiroir = etat.caisse_ouverte.map(|t| t.francs());
        for du in dus {
            let source = match tiroir {
                Some(t) if du.mode_propose == ModePaiement::Especes && t >= du.reste.francs() => {
                    tiroir = Some(t - du.reste.francs());
                    SourcePaiement::Caisse
                }
                Some(_) if du.mode_propose == ModePaiement::Especes => {
                    avertissements.push("paiement.reste_hors_caisse".to_owned());
                    SourcePaiement::HorsCaisse
                }
                _ => SourcePaiement::HorsCaisse,
            };
            let paiement = PaiementPersonnel {
                id: nouvel_id(),
                etablissement_id: session.etablissement_id,
                membre_id: du.membre_id,
                objet: ObjetPaiement::Salaire,
                bulletin_id: Some(du.bulletin_id),
                avance_id: None,
                montant: du.reste,
                le: contexte.maintenant,
                mode: du.mode_propose,
                reference: if du.mode_propose.exige_reference() {
                    du.numero.clone()
                } else {
                    String::new()
                },
                source,
                session_id: None,
                par: session.utilisateur_id,
                annule: false,
                motif_annulation: String::new(),
            };
            let motif = format!("Salaire de {} · {}", du.membre, du.numero);
            let paiement = self
                .enregistrer(
                    &mut unite,
                    session,
                    paiement,
                    &motif,
                    &etat,
                    &mut avertissements,
                )
                .await?;
            unite.emettre(
                TypeEvenement::SalairePaye,
                "paiement_personnel",
                &paiement.id.vers_chaine(),
                serde_json::json!({
                    "montant": paiement.montant, "mode": paiement.mode, "source": paiement.source,
                    "membre_id": paiement.membre_id, "mois": du.mois,
                }),
            );
        }
        self.noyau.terminer(unite).await?;
        avertissements.dedup();
        Ok(PaiementFait {
            etat: self.lire(session).await?,
            avertissements,
        })
    }

    /// Accorde une avance et la verse.
    pub async fn accorder_avance(
        &self,
        session: &Session,
        demande: &DemandeAvance,
    ) -> Resultat<PaiementFait> {
        verifier_avance(demande.montant, demande.mensualites, &demande.motif).map_err(regle)?;
        verifier_paiement(
            demande.montant,
            demande.montant,
            demande.mode,
            &demande.reference,
            demande.source,
        )
        .map_err(regle)?;
        let etat = self.lire(session).await?;
        let contexte = self.noyau.contexte(session.etablissement_id, Some(session));
        let aujourd_hui =
            DateCivile::du_jour(contexte.maintenant).ok_or(regle("paiement.calcul_impossible"))?;
        let mut unite = self
            .noyau
            .commencer(contexte.clone(), Durabilite::Critique)
            .await?;
        let membre = personnel::membre(unite.connexion(), demande.membre_id, &contexte).await?;
        let personne =
            personnes::personne(unite.connexion(), membre.personne_id, &contexte).await?;
        let avance = Avance {
            id: nouvel_id(),
            etablissement_id: session.etablissement_id,
            membre_id: membre.id,
            montant: demande.montant,
            mensualites: demande.mensualites,
            premier_mois: demande.premier_mois,
            accordee_le: aujourd_hui,
            motif: demande.motif.trim().to_owned(),
            statut: StatutAvance::EnCours,
            motif_annulation: String::new(),
        };
        paie::inserer_avance(unite.connexion(), &avance, &contexte).await?;
        unite.auditer(
            "avance_salaire",
            &avance.id.vers_chaine(),
            Operation::Creation,
            None,
            Some(serde_json::to_value(&avance)?),
            "paie.avance",
        );
        let mut avertissements = Vec::new();
        let paiement = PaiementPersonnel {
            id: nouvel_id(),
            etablissement_id: session.etablissement_id,
            membre_id: membre.id,
            objet: ObjetPaiement::Avance,
            bulletin_id: None,
            avance_id: Some(avance.id),
            montant: demande.montant,
            le: contexte.maintenant,
            mode: demande.mode,
            reference: demande.reference.trim().to_owned(),
            source: demande.source,
            session_id: None,
            par: session.utilisateur_id,
            annule: false,
            motif_annulation: String::new(),
        };
        let motif = format!("Avance à {} : {}", personne.nom_complet(), avance.motif);
        self.enregistrer(
            &mut unite,
            session,
            paiement,
            &motif,
            &etat,
            &mut avertissements,
        )
        .await?;
        unite.emettre(
            TypeEvenement::AvanceAccordee,
            "avance_salaire",
            &avance.id.vers_chaine(),
            serde_json::json!({
                "membre_id": avance.membre_id, "montant": avance.montant,
                "mensualites": avance.mensualites, "premier_mois": avance.premier_mois,
            }),
        );
        self.noyau.terminer(unite).await?;
        Ok(PaiementFait {
            etat: self.lire(session).await?,
            avertissements,
        })
    }

    /// Annule un paiement saisi par erreur : sorti de la caisse, une entrée le remet dans la
    /// session ouverte de qui annule. Le bulletin redevient à payer d'autant.
    pub async fn annuler_paiement(
        &self,
        session: &Session,
        demande: &DemandeAnnulationPaiement,
    ) -> Resultat<PaiementFait> {
        let motif = demande.motif.trim();
        if motif.is_empty() {
            return Err(regle("paiement.motif_obligatoire"));
        }
        let contexte = self.noyau.contexte(session.etablissement_id, Some(session));
        let mut unite = self
            .noyau
            .commencer(contexte.clone(), Durabilite::Critique)
            .await?;
        let paiement = paie::paiement(unite.connexion(), demande.id, &contexte).await?;
        if paiement.objet == ObjetPaiement::Avance {
            return Err(regle("paiement.avance_a_annuler"));
        }
        annuler_dans(&mut unite, session, &paiement, motif, &contexte).await?;
        self.noyau.terminer(unite).await?;
        Ok(PaiementFait {
            etat: self.lire(session).await?,
            avertissements: Vec::new(),
        })
    }

    /// Annule une avance qui n'a encore rien remboursé, et son versement.
    pub async fn annuler_avance(
        &self,
        session: &Session,
        demande: &DemandeAnnulationAvance,
    ) -> Resultat<PaiementFait> {
        let motif = demande.motif.trim();
        if motif.is_empty() {
            return Err(regle("paiement.motif_obligatoire"));
        }
        let contexte = self.noyau.contexte(session.etablissement_id, Some(session));
        let mut unite = self
            .noyau
            .commencer(contexte.clone(), Durabilite::Critique)
            .await?;
        let avant = paie::avance(unite.connexion(), demande.id, &contexte).await?;
        let rembourse = paie::rembourse_par_avance(unite.connexion(), &contexte)
            .await?
            .iter()
            .any(|(id, m)| *id == avant.id && m.francs() > 0);
        if rembourse {
            return Err(regle("avance.deja_remboursee"));
        }
        let apres = Avance {
            statut: avant.statut.transiter(StatutAvance::Annulee)?,
            motif_annulation: motif.to_owned(),
            ..avant.clone()
        };
        paie::ecrire_statut_avance(unite.connexion(), &apres, &contexte).await?;
        unite.auditer(
            "avance_salaire",
            &apres.id.vers_chaine(),
            Operation::Modification,
            Some(serde_json::to_value(&avant)?),
            Some(serde_json::to_value(&apres)?),
            "paie.annuler_avance",
        );
        for p in paie::paiements(unite.connexion(), &contexte)
            .await?
            .into_iter()
            .filter(|p| p.avance_id == Some(avant.id) && !p.annule)
        {
            annuler_dans(&mut unite, session, &p, motif, &contexte).await?;
        }
        self.noyau.terminer(unite).await?;
        Ok(PaiementFait {
            etat: self.lire(session).await?,
            avertissements: Vec::new(),
        })
    }

    /// Enregistre un paiement : de la caisse, il sort de la session ouverte de qui paie et
    /// avertit si le tiroir ne suffit pas ; sans session, des espèces se paient hors caisse.
    async fn enregistrer(
        &self,
        unite: &mut UniteDeTravail<Effet>,
        session: &Session,
        mut paiement: PaiementPersonnel,
        motif: &str,
        etat: &PaiementsPersonnel,
        avertissements: &mut Vec<String>,
    ) -> Resultat<PaiementPersonnel> {
        let contexte = unite.contexte().clone();
        let ouverte: Option<SessionCaisse> =
            caisse::ouverte_de(unite.connexion(), session.utilisateur_id, &contexte).await?;
        if paiement.source == SourcePaiement::Caisse {
            let Some(ouverte) = ouverte else {
                return Err(regle("paiement.caisse_fermee"));
            };
            paiement.session_id = Some(ouverte.id);
            if etat
                .caisse_ouverte
                .is_some_and(|tiroir| tiroir.francs() < paiement.montant.francs())
            {
                avertissements.push("paiement.tiroir_insuffisant".to_owned());
            }
        } else if paiement.mode == ModePaiement::Especes && ouverte.is_none() {
            avertissements.push("paiement.hors_caisse_sans_session".to_owned());
        }
        paie::inserer_paiement(unite.connexion(), &paiement, &contexte).await?;
        unite.auditer(
            "paiement_personnel",
            &paiement.id.vers_chaine(),
            Operation::Creation,
            None,
            Some(serde_json::to_value(&paiement)?),
            "paie.payer",
        );
        if let Some(session_id) = paiement.session_id {
            let mouvement = MouvementCaisse {
                id: nouvel_id(),
                etablissement_id: paiement.etablissement_id,
                session_id,
                montant: paiement.montant,
                mode: paiement.mode,
                motif: motif.to_owned(),
                versement_id: None,
                le: paiement.le,
                par: paiement.par,
                nature: if paiement.objet == ObjetPaiement::Avance {
                    NatureMouvement::SortieAvance
                } else {
                    NatureMouvement::SortieSalaire
                },
                paiement_id: Some(paiement.id),
                depense_id: None,
                transfert_id: None,
                recette_id: None,
            };
            caisse::inserer_mouvement(unite.connexion(), &mouvement, &contexte).await?;
        }
        crate::livres::rattacher_salaires(unite).await?;
        Ok(paiement)
    }
}

/// Annule un paiement dans l'unité : sorti de la caisse, une entrée de régularisation le remet
/// dans la session ouverte de qui annule.
async fn annuler_dans(
    unite: &mut UniteDeTravail<Effet>,
    session: &Session,
    paiement: &PaiementPersonnel,
    motif: &str,
    contexte: &Contexte,
) -> Resultat<()> {
    if paiement.annule {
        return Err(regle("paiement.deja_annule"));
    }
    if paiement.source == SourcePaiement::Caisse {
        let ouverte = caisse::ouverte_de(unite.connexion(), session.utilisateur_id, contexte)
            .await?
            .ok_or(regle("paiement.caisse_fermee_annulation"))?;
        let entree = MouvementCaisse {
            id: nouvel_id(),
            etablissement_id: paiement.etablissement_id,
            session_id: ouverte.id,
            montant: paiement.montant,
            mode: paiement.mode,
            motif: format!("Paiement annulé : {motif}"),
            versement_id: None,
            le: contexte.maintenant,
            par: session.utilisateur_id,
            nature: NatureMouvement::EntreeRegularisation,
            paiement_id: Some(paiement.id),
            depense_id: None,
            transfert_id: None,
            recette_id: None,
        };
        caisse::inserer_mouvement(unite.connexion(), &entree, contexte).await?;
    }
    paie::annuler_paiement(unite.connexion(), paiement.id, motif, contexte).await?;
    let id = paiement.id.vers_chaine();
    unite.auditer(
        "paiement_personnel",
        &id,
        Operation::Modification,
        Some(serde_json::json!({ "annule": false })),
        Some(serde_json::json!({ "annule": true, "motif_annulation": motif })),
        "paie.annuler_paiement",
    );
    unite.emettre(
        TypeEvenement::PaiementPersonnelAnnule,
        "paiement_personnel",
        &id,
        serde_json::json!({ "montant": paiement.montant, "motif": motif }),
    );
    crate::livres::rattacher_salaires(unite).await?;
    Ok(())
}

/// Ce qui a déjà été payé sur un bulletin ou une avance, paiements annulés exclus.
fn paye_sur(
    paiements: &[PaiementPersonnel],
    filtre: impl Fn(&PaiementPersonnel) -> bool,
) -> Resultat<Montant> {
    paiements
        .iter()
        .filter(|p| !p.annule && filtre(p))
        .try_fold(Montant::ZERO, |s, p| s.ajouter(p.montant))
        .map_err(|_| regle("paiement.calcul_impossible"))
}

/// Mode de paiement d'après la fiche du membre : un numéro mobile money se paie par Orange
/// Money, le mode le plus courant, que la direction change au besoin.
fn mode_de(m: &MembrePersonnel) -> ModePaiement {
    match m.paiement {
        MoyenPaiement::Especes => ModePaiement::Especes,
        MoyenPaiement::MobileMoney { .. } => ModePaiement::OrangeMoney,
        MoyenPaiement::Banque { .. } => ModePaiement::Virement,
    }
}
