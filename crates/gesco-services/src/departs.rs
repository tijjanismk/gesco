//! Départs et exclusions (SPEC_PHASE_4 §9 ; réponse du 13 octobre 2026) : transfert vers une
//! autre école avec son certificat numéroté et le relevé des moyennes ; exclusion définitive,
//! révocable par décision motivée ; exclusion temporaire avec date de retour, levable avant son
//! terme. Chaque décision est une transaction auditée ; rien ne s'efface.

use gesco_core::commun::{DateCivile, Id, Montant};
use gesco_core::core::{MachineAEtats, Operation, TypeDocumentNumerote, TypeEvenement};
use gesco_core::pedagogie::bulletin::{non_suivie, texte_rang};
use gesco_core::pedagogie::calcul::ecrire;
use gesco_core::pedagogie::{Periode, StatutBulletin};
use gesco_core::scolarite::depart::{verifier_depart, verifier_exclusion};
use gesco_core::scolarite::{
    Cycle, Exclusion, Inscription, NatureExclusion, StatutExclusion, StatutInscription,
};
use gesco_db::depots::{
    academique, annee_scolaire, bulletins, departs, inscriptions, parametre, pedagogie, personnes,
};
use gesco_db::nouvel_id;
use gesco_db::unite::Durabilite;
use serde::{Deserialize, Serialize};

use crate::erreur::{ErreurService, Resultat};
use crate::facturation::{annuler_echeances_a_venir_dans, reste_echu, retablir_echeances_dans};
use crate::inscriptions::jour;
use crate::noyau::Noyau;
use crate::numerotation;
use crate::session::Session;

fn regle(cle: &'static str) -> ErreurService {
    ErreurService::Regle { cle }
}

/// Paramètre Q-33 : exiger un solde nul avant un transfert (faux par défaut).
pub const PARAMETRE_SOLDE_NUL: &str = "transfert.solde_nul_exige";

/// Transfert d'un élève vers une autre école.
#[derive(Debug, Clone, Deserialize, specta::Type)]
pub struct DemandeTransfert {
    /// Inscription.
    pub inscription_id: Id<Inscription>,
    /// Jour du départ.
    pub date_depart: DateCivile,
    /// École qui l'accueille.
    pub etablissement_destination: String,
    /// Motif : déménagement, affectation des parents…
    pub motif: String,
    /// Quand l'école exige un solde nul (Q-33) et qu'il reste un impayé : le motif pour lequel la
    /// direction passe outre ; vide sinon.
    pub passer_outre: String,
}

/// Transfert enregistré.
#[derive(Debug, Clone, Serialize, specta::Type)]
pub struct TransfertEnregistre {
    /// Inscription transférée.
    pub inscription: Inscription,
    /// Numéro du certificat de transfert.
    pub numero_certificat: String,
    /// Reste dû sur les échéances échues au jour du départ.
    pub reste_du: Montant,
    /// Échéances à venir annulées.
    pub echeances_annulees: u32,
    /// Clés des avertissements, qui n'empêchent rien.
    pub avertissements: Vec<String>,
}

/// Exclusion décidée par la direction.
#[derive(Debug, Clone, Deserialize, specta::Type)]
pub struct DemandeExclusion {
    /// Inscription.
    pub inscription_id: Id<Inscription>,
    /// Temporaire ou définitive.
    pub nature: NatureExclusion,
    /// Premier jour.
    pub du: DateCivile,
    /// Dernier jour d'une exclusion temporaire ; rien pour une définitive.
    pub au: Option<DateCivile>,
    /// Motif.
    pub motif: String,
    /// Décision : conseil de discipline, date…
    pub decision: String,
}

/// Levée d'une exclusion temporaire, ou réintégration après une exclusion définitive.
#[derive(Debug, Clone, Deserialize, specta::Type)]
pub struct DemandeLevee {
    /// Exclusion.
    pub id: Id<Exclusion>,
    /// Motif, obligatoire.
    pub motif: String,
}

/// Inscription désignée.
#[derive(Debug, Clone, Deserialize, specta::Type)]
pub struct DemandeDepart {
    /// Inscription.
    pub inscription_id: Id<Inscription>,
}

/// Service des départs et des exclusions.
pub struct ServiceDeparts<'n> {
    noyau: &'n Noyau,
}

impl<'n> ServiceDeparts<'n> {
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

    async fn inscription(&self, session: &Session, id: Id<Inscription>) -> Resultat<Inscription> {
        let contexte = self.noyau.contexte(session.etablissement_id, Some(session));
        let mut cnx = self.connexion().await?;
        Ok(inscriptions::lire(&mut cnx, id, &contexte).await?)
    }

    /// Transfère un élève vers une autre école (§9) : inscription transférée, échéances à venir
    /// annulées (Q-34), certificat numéroté. Un impayé échu avertit ; si l'école exige un solde
    /// nul (Q-33), il refuse, sauf motif de la direction pour passer outre.
    pub async fn transferer(
        &self,
        session: &Session,
        demande: &DemandeTransfert,
    ) -> Resultat<TransfertEnregistre> {
        let avant = self.inscription(session, demande.inscription_id).await?;
        let motif = demande.motif.trim();
        if motif.is_empty() {
            return Err(regle("inscription.motif_obligatoire"));
        }
        verifier_depart(
            demande.date_depart,
            avant.date_inscription,
            &demande.etablissement_destination,
        )
        .map_err(regle)?;
        let statut = avant.statut.transiter(StatutInscription::Transferee)?;
        let contexte = self.noyau.contexte(session.etablissement_id, Some(session));
        let (reste_du, exige) = {
            let mut cnx = self.connexion().await?;
            (
                reste_echu(&mut cnx, avant.id, demande.date_depart, &contexte).await?,
                parametre::booleen_param(&mut cnx, PARAMETRE_SOLDE_NUL, false, &contexte).await?,
            )
        };
        let mut avertissements = Vec::new();
        if !reste_du.est_nul() {
            if exige && demande.passer_outre.trim().is_empty() {
                return Err(regle("transfert.solde_non_nul"));
            }
            avertissements.push("transfert.impaye".to_owned());
        }
        let apres = Inscription {
            statut,
            date_depart: Some(demande.date_depart),
            etablissement_destination: demande.etablissement_destination.trim().to_owned(),
            motif_depart: motif.to_owned(),
            ..avant.clone()
        };
        let exercice = self.exercice(session, &avant).await?;
        let mut unite = self
            .noyau
            .commencer(contexte.clone(), Durabilite::Critique)
            .await?;
        departs::ecrire_depart(unite.connexion(), &apres, &contexte).await?;
        let annulees = annuler_echeances_a_venir_dans(
            &mut unite,
            &contexte,
            avant.id,
            demande.date_depart,
            "departs.transferer",
        )
        .await?;
        let numero = numerotation::attribuer(
            &mut unite,
            TypeDocumentNumerote::CertificatTransfert,
            &exercice,
        )
        .await?;
        departs::inserer_certificat(unite.connexion(), avant.id, &numero.numero, &contexte).await?;
        let id = avant.id.vers_chaine();
        unite.auditer(
            "inscription",
            &id,
            Operation::Transition,
            Some(serde_json::to_value(&avant)?),
            Some(serde_json::to_value(&apres)?),
            "departs.transferer",
        );
        unite.emettre(
            TypeEvenement::InscriptionTransferee,
            "inscription",
            &id,
            serde_json::json!({ "date_depart": demande.date_depart, "destination": apres.etablissement_destination,
                "certificat": numero.numero, "reste_du": reste_du, "passer_outre": demande.passer_outre.trim() }),
        );
        self.noyau.terminer(unite).await?;
        Ok(TransfertEnregistre {
            inscription: apres,
            numero_certificat: numero.numero,
            reste_du,
            echeances_annulees: u32::try_from(annulees.len()).unwrap_or(u32::MAX),
            avertissements,
        })
    }

    /// Exercice de numérotation : l'année civile de la rentrée de l'inscription.
    async fn exercice(&self, session: &Session, inscription: &Inscription) -> Resultat<String> {
        let contexte = self.noyau.contexte(session.etablissement_id, Some(session));
        let mut cnx = self.connexion().await?;
        let annee = annee_scolaire::lire(&mut cnx, inscription.annee_id, &contexte).await?;
        Ok(jour(annee.debut)?.annee().to_string())
    }

    /// Exclut un élève (réponse du 13 octobre 2026) : temporairement, il reste inscrit jusqu'à sa
    /// date de retour ; définitivement, il quitte l'école et ses échéances à venir s'annulent,
    /// gardées pour être rétablies s'il est réintégré.
    pub async fn exclure(
        &self,
        session: &Session,
        demande: &DemandeExclusion,
    ) -> Resultat<Exclusion> {
        let avant = self.inscription(session, demande.inscription_id).await?;
        if avant.statut != StatutInscription::Validee {
            return Err(regle("inscription.non_validee"));
        }
        verifier_exclusion(
            demande.nature,
            demande.du,
            demande.au,
            &demande.motif,
            avant.date_inscription,
        )
        .map_err(regle)?;
        let contexte = self.noyau.contexte(session.etablissement_id, Some(session));
        let mut exclusion = Exclusion {
            id: nouvel_id(),
            etablissement_id: session.etablissement_id,
            inscription_id: avant.id,
            nature: demande.nature,
            du: demande.du,
            au: demande.au,
            motif: demande.motif.trim().to_owned(),
            decision: demande.decision.trim().to_owned(),
            statut: StatutExclusion::EnCours,
            leve_le: None,
            motif_levee: String::new(),
            echeances_annulees: Vec::new(),
        };
        let mut unite = self
            .noyau
            .commencer(contexte.clone(), Durabilite::Critique)
            .await?;
        if demande.nature == NatureExclusion::Definitive {
            let apres = Inscription {
                statut: avant.statut.transiter(StatutInscription::Radiee)?,
                date_depart: Some(demande.du),
                motif_depart: exclusion.motif.clone(),
                ..avant.clone()
            };
            departs::ecrire_depart(unite.connexion(), &apres, &contexte).await?;
            exclusion.echeances_annulees = annuler_echeances_a_venir_dans(
                &mut unite,
                &contexte,
                avant.id,
                demande.du,
                "departs.exclure",
            )
            .await?;
            unite.auditer(
                "inscription",
                &avant.id.vers_chaine(),
                Operation::Transition,
                Some(serde_json::to_value(&avant)?),
                Some(serde_json::to_value(&apres)?),
                "departs.exclure",
            );
        }
        departs::inserer_exclusion(unite.connexion(), &exclusion, &contexte).await?;
        let id = exclusion.id.vers_chaine();
        unite.auditer(
            "exclusion",
            &id,
            Operation::Creation,
            None,
            Some(serde_json::to_value(&exclusion)?),
            "departs.exclure",
        );
        unite.emettre(
            TypeEvenement::EleveExclu,
            "inscription",
            &avant.id.vers_chaine(),
            serde_json::json!({ "exclusion_id": exclusion.id, "nature": exclusion.nature,
                "du": exclusion.du, "au": exclusion.au }),
        );
        self.noyau.terminer(unite).await?;
        Ok(exclusion)
    }

    /// Lève une exclusion temporaire avant son terme, ou réintègre l'élève exclu définitivement :
    /// il revient dans sa classe et ses échéances annulées sont rétablies.
    pub async fn lever(&self, session: &Session, demande: &DemandeLevee) -> Resultat<Exclusion> {
        let motif = demande.motif.trim();
        if motif.is_empty() {
            return Err(regle("exclusion.motif_obligatoire"));
        }
        let contexte = self.noyau.contexte(session.etablissement_id, Some(session));
        let avant = {
            let mut cnx = self.connexion().await?;
            departs::exclusion(&mut cnx, demande.id, &contexte).await?
        };
        let aujourd_hui = jour(contexte.maintenant)?;
        let statut = match avant.nature {
            NatureExclusion::Temporaire => {
                if avant.au.is_some_and(|au| au < aujourd_hui) {
                    return Err(regle("exclusion.deja_terminee"));
                }
                StatutExclusion::Levee
            }
            NatureExclusion::Definitive => StatutExclusion::Revoquee,
        };
        let apres = Exclusion {
            statut: avant.statut.transiter(statut)?,
            leve_le: Some(aujourd_hui),
            motif_levee: motif.to_owned(),
            ..avant.clone()
        };
        let inscription = self.inscription(session, avant.inscription_id).await?;
        let mut unite = self
            .noyau
            .commencer(contexte.clone(), Durabilite::Critique)
            .await?;
        if avant.nature == NatureExclusion::Definitive {
            let reintegree = Inscription {
                statut: inscription.statut.transiter(StatutInscription::Validee)?,
                date_depart: None,
                motif_depart: String::new(),
                ..inscription.clone()
            };
            departs::ecrire_depart(unite.connexion(), &reintegree, &contexte).await?;
            retablir_echeances_dans(
                &mut unite,
                &contexte,
                inscription.id,
                &avant.echeances_annulees,
                "departs.reintegrer",
            )
            .await?;
            unite.auditer(
                "inscription",
                &inscription.id.vers_chaine(),
                Operation::Transition,
                Some(serde_json::to_value(&inscription)?),
                Some(serde_json::to_value(&reintegree)?),
                "departs.reintegrer",
            );
        }
        departs::ecrire_levee(unite.connexion(), &apres, &contexte).await?;
        let id = apres.id.vers_chaine();
        unite.auditer(
            "exclusion",
            &id,
            Operation::Transition,
            Some(serde_json::to_value(&avant)?),
            Some(serde_json::to_value(&apres)?),
            "departs.lever",
        );
        unite.emettre(
            TypeEvenement::ExclusionLevee,
            "inscription",
            &avant.inscription_id.vers_chaine(),
            serde_json::json!({ "exclusion_id": avant.id, "statut": apres.statut, "motif": motif }),
        );
        self.noyau.terminer(unite).await?;
        Ok(apres)
    }

    /// Exclusions d'un élève.
    pub async fn exclusions(
        &self,
        session: &Session,
        inscription_id: Id<Inscription>,
    ) -> Resultat<Vec<Exclusion>> {
        let contexte = self.noyau.contexte(session.etablissement_id, Some(session));
        let mut cnx = self.connexion().await?;
        Ok(departs::exclusions(&mut cnx, inscription_id, &contexte).await?)
    }

    /// Données du certificat de transfert d'un élève transféré.
    pub async fn donnees_certificat(
        &self,
        session: &Session,
        inscription_id: Id<Inscription>,
    ) -> Resultat<serde_json::Value> {
        let contexte = self.noyau.contexte(session.etablissement_id, Some(session));
        let mut cnx = self.connexion().await?;
        let i = inscriptions::lire(&mut cnx, inscription_id, &contexte).await?;
        if i.statut != StatutInscription::Transferee {
            return Err(regle("transfert.non_transferee"));
        }
        let numero = departs::certificat(&mut cnx, i.id, &contexte)
            .await?
            .ok_or(regle("transfert.non_transferee"))?;
        let entete = Entete::lire(&mut cnx, &i, &contexte).await?;
        Ok(serde_json::json!({
            "numero": numero,
            "eleve": entete.eleve,
            "naissance": entete.naissance,
            "matricule": entete.matricule,
            "matricule_national": entete.matricule_national,
            "classe": entete.classe,
            "annee": entete.annee,
            "inscrit_le": crate::documents::date_civile_longue(i.date_inscription),
            "date_depart": i.date_depart.map(crate::documents::date_civile_longue).unwrap_or_default(),
            "destination": i.etablissement_destination,
            "date": crate::documents::date_longue(contexte.maintenant),
        }))
    }

    /// Données du relevé des moyennes des périodes suivies (§9) : par période, la moyenne de
    /// chaque matière et la moyenne générale, avec le rang sauf pour la période du départ ; une
    /// moyenne reprise d'une autre école porte sa source.
    pub async fn donnees_releve(
        &self,
        session: &Session,
        inscription_id: Id<Inscription>,
    ) -> Resultat<serde_json::Value> {
        let contexte = self.noyau.contexte(session.etablissement_id, Some(session));
        let mut cnx = self.connexion().await?;
        let i = inscriptions::lire(&mut cnx, inscription_id, &contexte).await?;
        let classe = academique::classe(&mut cnx, i.classe_id, &contexte).await?;
        let mut periodes: Vec<Periode> = pedagogie::periodes(&mut cnx, i.annee_id, &contexte)
            .await?
            .into_iter()
            .filter(|p| p.niveau_id == classe.niveau_id)
            .collect();
        periodes.sort_by_key(|p| p.rang);
        let siens = bulletins::de_l_inscription(&mut cnx, i.id, &contexte).await?;
        let reprises = bulletins::moyennes_reprises(&mut cnx, &[i.id], &contexte).await?;
        let entete = Entete::lire(&mut cnx, &i, &contexte).await?;
        let depart = i.date_depart;
        let mut lignes = Vec::new();
        for p in &periodes {
            if depart.is_some_and(|d| d < p.debut) {
                break;
            }
            let en_cours = depart.is_some_and(|d| p.debut <= d && d <= p.fin);
            if non_suivie(i.date_inscription, p) {
                if let Some(r) = reprises.iter().find(|r| r.periode_id == p.id) {
                    lignes.push(serde_json::json!({
                        "periode": p.libelle(), "moyenne": ecrire(r.moyenne), "rang": "",
                        "matieres": [], "source": format!("Reprise du relevé de {} ({})", r.etablissement_origine, r.piece),
                    }));
                }
                continue;
            }
            // La version publiée en vigueur, ou à défaut la dernière calculée.
            let b = siens
                .iter()
                .filter(|b| b.periode_id == p.id && b.statut != StatutBulletin::Remplace)
                .max_by_key(|b| (b.statut == StatutBulletin::Publie, b.version));
            let Some(b) = b else { continue };
            lignes.push(serde_json::json!({
                "periode": p.libelle(),
                "moyenne": b.contenu.moyenne.map(ecrire).unwrap_or_default(),
                "rang": if en_cours { String::new() } else { b.contenu.rang.map(texte_rang).unwrap_or_default() },
                "matieres": b.contenu.lignes.iter().filter(|l| l.moyenne.is_some()).map(|l| serde_json::json!({
                    "matiere": l.matiere, "moyenne": l.moyenne.map(ecrire).unwrap_or_default(),
                })).collect::<Vec<_>>(),
                "source": if b.statut == StatutBulletin::Publie { String::new() } else { "Bulletin non publié".to_owned() },
            }));
        }
        Ok(serde_json::json!({
            "eleve": entete.eleve,
            "naissance": entete.naissance,
            "matricule": entete.matricule,
            "matricule_national": entete.matricule_national,
            "classe": entete.classe,
            "annee": entete.annee,
            "bareme": if entete.cycle == Cycle::PremierCycle { "10" } else { "20" },
            "periodes": lignes,
            "date": crate::documents::date_longue(contexte.maintenant),
        }))
    }
}

/// Ce qui figure en tête du certificat et du relevé.
struct Entete {
    eleve: String,
    naissance: String,
    matricule: String,
    matricule_national: Option<String>,
    classe: String,
    annee: String,
    cycle: Cycle,
}

impl Entete {
    async fn lire(
        cnx: &mut gesco_db::Connexion,
        i: &Inscription,
        contexte: &gesco_db::Contexte,
    ) -> Resultat<Self> {
        let (eleve, identite) = personnes::eleve(cnx, i.eleve_id, contexte).await?;
        let classe = academique::classe(cnx, i.classe_id, contexte).await?;
        let niveau = academique::niveau(cnx, classe.niveau_id, contexte).await?;
        let annee = annee_scolaire::lire(cnx, i.annee_id, contexte).await?;
        let naissance = match (identite.date_naissance, identite.lieu_naissance.trim()) {
            (Some(d), "") => crate::documents::date_civile_longue(d),
            (Some(d), lieu) => format!("{} à {lieu}", crate::documents::date_civile_longue(d)),
            (None, lieu) => lieu.to_owned(),
        };
        Ok(Self {
            eleve: identite.nom_complet(),
            naissance,
            matricule: eleve.matricule_interne,
            // Au lycée, le matricule national figure sur les documents de transfert (Q-32) ; dans
            // l'enseignement technique, quand l'élève en a un (Q-70).
            matricule_national: eleve
                .matricule_national
                .filter(|_| matches!(niveau.cycle, Cycle::Lycee | Cycle::Technique)),
            classe: format!(
                "{} ({})",
                classe.libelle,
                crate::referentiel::niveau_imprime(cnx, &niveau, &classe, contexte).await?
            ),
            annee: annee.libelle,
            cycle: niveau.cycle,
        })
    }
}
