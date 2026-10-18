//! Code d'accès des responsables et consultation par SMS entrant (SPEC_PHASE_5 §9, blueprint
//! §25.4 et §25.5). Au guichet, un code se remet en main propre sur un billet imprimé, une seule
//! fois : la base n'en garde que l'empreinte. Un parent écrit « SOLDE code », « ABS code »,
//! « NOTE code » à la puce de l'école et reçoit la réponse ; un numéro inconnu, un code faux
//! répété ou un quota dépassé ne vident pas le crédit. Chaque SMS reçu, servi ou refusé, reste au
//! journal et à l'audit : numéro, mot, élèves, réponse.

use gesco_core::commun::{DateCivile, Horodatage, Id, Montant};
use gesco_core::core::{Operation, TypeEvenement};
use gesco_core::finance::recouvrement::impaye;
use gesco_core::notification::consultation::{
    self, AbsencesEnfant, CodeAcces, EtatCodeAcces, Faits, IssueConsultation, MotConsultation,
    NoteEnfant, SmsRecu, SoldeEnfant,
};
use gesco_core::notification::{NatureMessage, gsm7};
use gesco_core::pedagogie::assiduite::NatureAbsence;
use gesco_core::pedagogie::{Periode, StatutBulletin, TypePeriode};
use gesco_core::scolarite::{Inscription, Responsable, StatutInscription, telephone};
use gesco_db::depots::{
    absences, academique, annee_scolaire, bulletins, consultation as depot, factures, file_sms,
    inscriptions, parametre, pedagogie, personnes, utilisateur,
};
use gesco_db::unite::Durabilite;
use gesco_db::{Connexion, Contexte, nouvel_id};
use gesco_sms::SmsEntrant;
use serde::{Deserialize, Serialize};

use crate::erreur::{ErreurService, Resultat};
use crate::file_sms::{MessageAFaire, PARAMETRE_PORT, PORT_SIMULE, canal_en_vigueur};
use crate::noyau::Noyau;
use crate::session::Session;

fn regle(cle: &'static str) -> ErreurService {
    ErreurService::Regle { cle }
}

/// Réglage qui ouvre la consultation par SMS.
pub const PARAMETRE_ACTIVE: &str = "sms.consultation_active";
/// Réglage du quota de consultations par numéro et par jour.
pub const PARAMETRE_QUOTA: &str = "sms.consultation_quota_jour";
/// Réglage qui ajoute le rang à la réponse NOTE.
pub const PARAMETRE_RANG: &str = "sms.consultation_rang";
/// Réglage du numéro de la puce de l'école, imprimé sur le billet.
pub const PARAMETRE_NUMERO_ECOLE: &str = "sms.numero_ecole";

/// SMS reçus listés au plus.
const LISTE_MAX: i64 = 200;

const JOUR_MS: i64 = 86_400_000;

/// Code d'accès d'un responsable, tel que le guichet le montre : jamais le code lui-même.
#[derive(Debug, Clone, Serialize, specta::Type)]
pub struct EtatCodeResponsable {
    /// Responsable.
    pub responsable_id: Id<Responsable>,
    /// Son code actif, s'il en a un.
    pub actif: Option<CodeAcces>,
    /// Révocation de son dernier code, s'il n'en a plus d'actif.
    pub revoque_le: Option<Horodatage>,
}

/// Code remis au guichet : à lire au parent et à imprimer, une seule fois.
#[derive(Debug, Clone, Serialize, specta::Type)]
pub struct CodeRemis {
    /// Code en groupes de trois, `K7P X3M`.
    pub code: String,
    /// Billet à remettre en main propre, PDF en base64 ; il n'est gardé nulle part.
    pub pdf: String,
    /// Le code du responsable après la remise.
    pub etat: EtatCodeResponsable,
}

/// Responsable dont on remet ou révoque le code.
#[derive(Debug, Deserialize, specta::Type)]
pub struct DemandeCodeAcces {
    /// Responsable.
    pub responsable_id: Id<Responsable>,
}

/// SMS reçu, tel que l'écran des messages le montre.
#[derive(Debug, Clone, Serialize, specta::Type)]
pub struct SmsRecuAffiche {
    /// Le SMS, son code masqué.
    pub sms: SmsRecu,
    /// Responsable du numéro, s'il est connu.
    pub responsable: String,
    /// Texte de la réponse partie, s'il y en a une.
    pub reponse: String,
}

/// SMS reçus et état de la consultation.
#[derive(Debug, Clone, Serialize, specta::Type)]
pub struct SmsRecus {
    /// Les derniers SMS reçus, du plus récent au plus ancien.
    pub sms: Vec<SmsRecuAffiche>,
    /// Vrai si l'école a ouvert la consultation par SMS.
    pub consultation_active: bool,
    /// Vrai si le canal est simulé : un SMS reçu peut se simuler pour l'essai.
    pub simulation: bool,
}

/// SMS reçu à simuler sur le canal ou la clé simulés.
#[derive(Debug, Deserialize, specta::Type)]
pub struct DemandeSimulationSms {
    /// Numéro de l'expéditeur.
    pub numero: String,
    /// Texte.
    pub texte: String,
}

/// Où simuler un SMS reçu.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CanalDeSimulation {
    /// Le canal simulé.
    Canal,
    /// La clé simulée, derrière le port `simule`.
    Cle,
}

/// Enfant d'un responsable, inscrit cette année.
struct Enfant {
    prenom: String,
    inscription: Inscription,
}

/// Prénom d'usage : le premier des prénoms.
fn prenom(prenoms: &str) -> String {
    prenoms
        .split_whitespace()
        .next()
        .unwrap_or(prenoms)
        .to_owned()
}

/// Date écrite comme dans un SMS : `14/10/2026`.
fn date_courte(d: DateCivile) -> String {
    format!("{:02}/{:02}/{}", d.jour(), d.mois(), d.annee())
}

/// Période nommée dans une phrase : « du 1er trimestre », « d'octobre », « de novembre ».
fn periode_dans_la_phrase(p: &Periode) -> String {
    let libelle = p.libelle();
    match p.type_periode {
        TypePeriode::Mensuelle => {
            let mois = libelle.to_lowercase();
            if mois.starts_with(['a', 'o']) {
                format!("d'{mois}")
            } else {
                format!("de {mois}")
            }
        }
        TypePeriode::Trimestre | TypePeriode::Semestre => format!("du {libelle}"),
    }
}

/// Texte tenu dans trois SMS : au-delà, coupé.
fn tenir_en_trois_sms(texte: String) -> String {
    let texte = gsm7::translitterer(&texte);
    if gsm7::nombre_sms(&texte) <= gsm7::SMS_MAX {
        return texte;
    }
    let mut coupe: String = texte.chars().take(440).collect();
    coupe.push_str("...");
    coupe
}

/// Le code d'un responsable : actif, ou la date de sa dernière révocation.
pub(crate) async fn etat_code(
    cnx: &mut Connexion,
    responsable_id: Id<Responsable>,
    contexte: &Contexte,
) -> Resultat<EtatCodeResponsable> {
    let actif = depot::code_actif(cnx, responsable_id, contexte).await?;
    let revoque_le = if actif.is_none() {
        depot::dernier_revoque(cnx, responsable_id, contexte)
            .await?
            .and_then(|c| c.revoque_le)
    } else {
        None
    };
    Ok(EtatCodeResponsable {
        responsable_id,
        actif,
        revoque_le,
    })
}

/// Enfants d'un responsable inscrits cette année, inscription validée.
async fn enfants(
    cnx: &mut Connexion,
    responsable_id: Id<Responsable>,
    contexte: &Contexte,
) -> Resultat<Vec<Enfant>> {
    let Some(annee) = annee_scolaire::courante(cnx, contexte).await? else {
        return Ok(Vec::new());
    };
    let inscrites: Vec<Inscription> = inscriptions::de_l_annee(cnx, annee.id, contexte)
        .await?
        .into_iter()
        .filter(|i| i.statut == StatutInscription::Validee)
        .collect();
    let mut enfants = Vec::new();
    for (eleve_id, r, _) in personnes::liens(cnx, contexte).await? {
        if r != responsable_id {
            continue;
        }
        let Some(inscription) = inscrites.iter().find(|i| i.eleve_id == eleve_id) else {
            continue;
        };
        let (_, personne) = personnes::eleve(cnx, eleve_id, contexte).await?;
        enfants.push(Enfant {
            prenom: prenom(&personne.prenoms),
            inscription: inscription.clone(),
        });
    }
    Ok(enfants)
}

/// Réponse à SOLDE.
async fn solde(
    cnx: &mut Connexion,
    ecole: &str,
    enfants: &[Enfant],
    aujourd_hui: DateCivile,
    contexte: &Contexte,
) -> Resultat<String> {
    let mut lignes = Vec::new();
    let mut a_venir: Vec<(DateCivile, Montant)> = Vec::new();
    for enfant in enfants {
        let (mut reste, mut echu) = (Montant::ZERO, None);
        if let Some(facture) =
            factures::facture_vivante(cnx, enfant.inscription.id, contexte).await?
        {
            let echeances: Vec<(DateCivile, Montant)> =
                crate::facturation::etat_echeances(cnx, facture.id, contexte)
                    .await?
                    .iter()
                    .map(|e| (e.echeance.date, e.reste))
                    .collect();
            if let Ok(Some(i)) = impaye(&echeances, aujourd_hui) {
                echu = Some(i.echu);
            }
            for (date, r) in &echeances {
                reste = reste.ajouter(*r).unwrap_or(reste);
                if *date > aujourd_hui && r.francs() > 0 {
                    a_venir.push((*date, *r));
                }
            }
        }
        lignes.push(SoldeEnfant {
            prenom: enfant.prenom.clone(),
            reste: reste.formater(),
            echu: echu.filter(|e| e.francs() > 0).map(|e| e.formater()),
        });
    }
    let prochaine = a_venir.iter().map(|(d, _)| *d).min().map(|date| {
        let montant = a_venir
            .iter()
            .filter(|(d, _)| *d == date)
            .fold(Montant::ZERO, |s, (_, m)| s.ajouter(*m).unwrap_or(s));
        (date_courte(date), montant.formater())
    });
    Ok(consultation::reponse_solde(
        ecole,
        &lignes,
        prochaine.as_ref().map(|(d, m)| (d.as_str(), m.as_str())),
    ))
}

/// La période en cours d'un niveau : celle qui contient le jour, sinon la dernière commencée.
fn periode_en_cours(periodes: &[Periode], aujourd_hui: DateCivile) -> Option<&Periode> {
    periodes
        .iter()
        .find(|p| p.debut <= aujourd_hui && aujourd_hui <= p.fin)
        .or_else(|| {
            periodes
                .iter()
                .filter(|p| p.debut <= aujourd_hui)
                .max_by_key(|p| p.debut)
        })
}

/// Réponse à ABS.
async fn absences_enfants(
    cnx: &mut Connexion,
    ecole: &str,
    enfants: &[Enfant],
    aujourd_hui: DateCivile,
    contexte: &Contexte,
) -> Resultat<String> {
    let mut lignes = Vec::new();
    let mut libelle = String::new();
    for enfant in enfants {
        let classe = academique::classe(cnx, enfant.inscription.classe_id, contexte).await?;
        let periodes: Vec<Periode> =
            pedagogie::periodes(cnx, enfant.inscription.annee_id, contexte)
                .await?
                .into_iter()
                .filter(|p| p.niveau_id == classe.niveau_id)
                .collect();
        let (mut demi_journees, mut non_justifiees) = (0, 0);
        if let Some(p) = periode_en_cours(&periodes, aujourd_hui) {
            if libelle.is_empty() {
                libelle = periode_dans_la_phrase(p);
            }
            let fin = if p.fin < aujourd_hui {
                p.fin
            } else {
                aujourd_hui
            };
            for a in absences::de_la_classe(cnx, classe.id, p.debut, fin, contexte).await? {
                if a.inscription_id == enfant.inscription.id && a.nature == NatureAbsence::Absence {
                    demi_journees += 1;
                    if !a.justifiee {
                        non_justifiees += 1;
                    }
                }
            }
        }
        lignes.push(AbsencesEnfant {
            prenom: enfant.prenom.clone(),
            demi_journees,
            non_justifiees,
        });
    }
    if libelle.is_empty() {
        libelle = "de l'annee".to_owned();
    }
    Ok(consultation::reponse_absences(ecole, &libelle, &lignes))
}

/// Réponse à NOTE : le dernier bulletin publié de chaque enfant.
async fn notes(
    cnx: &mut Connexion,
    ecole: &str,
    enfants: &[Enfant],
    contexte: &Contexte,
) -> Resultat<String> {
    let avec_rang = parametre::booleen_param(cnx, PARAMETRE_RANG, false, contexte).await?;
    let mut lignes = Vec::new();
    for enfant in enfants {
        let publie = bulletins::de_l_inscription(cnx, enfant.inscription.id, contexte)
            .await?
            .into_iter()
            .filter(|b| b.statut == StatutBulletin::Publie)
            .max_by_key(|b| b.publie_le.map(|h| h.millis_epoque()));
        lignes.push(match publie {
            Some(b) => NoteEnfant {
                prenom: enfant.prenom.clone(),
                periode: Some(b.contenu.periode.clone()),
                moyenne: b.contenu.moyenne,
                bareme: b.contenu.bareme,
                rang: if avec_rang {
                    b.contenu.rang.map(|r| (r.rang, b.contenu.effectif))
                } else {
                    None
                },
            },
            None => NoteEnfant {
                prenom: enfant.prenom.clone(),
                periode: None,
                moyenne: None,
                bareme: 2_000,
                rang: None,
            },
        });
    }
    Ok(consultation::reponse_notes(ecole, &lignes))
}

/// Reçoit un SMS : le journal, l'issue, la réponse mise en file s'il y en a une. Appelé par la
/// file à chacun de ses passages, sans session.
pub(crate) async fn recevoir(
    noyau: &Noyau,
    entrant: &SmsEntrant,
    contexte: &Contexte,
) -> Resultat<IssueConsultation> {
    let mut cnx = noyau
        .base()
        .pool()
        .acquire()
        .await
        .map_err(gesco_db::ErreurBase::from)?;
    let maintenant = contexte.maintenant;
    let active = parametre::booleen_param(&mut cnx, PARAMETRE_ACTIVE, false, contexte).await?;
    let numero = telephone::normaliser(&entrant.numero)
        .unwrap_or_else(|_| entrant.numero.trim().chars().take(24).collect());
    let demande = consultation::lire_demande(&entrant.texte);
    // Les responsables de ce numéro ; un numéro qui ne répond à aucun ne coûte rien.
    let mut candidats: Vec<Id<Responsable>> = Vec::new();
    if active {
        for (r, p) in personnes::responsables(&mut cnx, contexte).await? {
            if p.telephones.contains(&numero) {
                candidats.push(r.id);
            }
        }
    }
    let debut_du_jour =
        Horodatage::depuis_millis_epoque(maintenant.millis_epoque().div_euclid(JOUR_MS) * JOUR_MS);
    let compte = depot::compte_depuis(&mut cnx, &numero, debut_du_jour, contexte).await?;
    let quota = parametre::entier(
        &mut cnx,
        PARAMETRE_QUOTA,
        consultation::QUOTA_DEFAUT,
        contexte,
    )
    .await?;
    let mut faits = Faits {
        active,
        numero_connu: !candidats.is_empty(),
        compte,
        quota: u32::try_from(quota.max(0)).unwrap_or(u32::MAX),
        mot: demande.as_ref().map(|d| d.mot),
        code_juste: true,
    };
    // Le code ne se vérifie que si la demande peut être servie : une empreinte coûte du temps.
    let mut code_servi: Option<CodeAcces> = None;
    if consultation::decider(&faits) == IssueConsultation::Servie {
        let code = demande.as_ref().map(|d| d.code.clone()).unwrap_or_default();
        if consultation::code_bien_forme(&code) {
            for id in &candidats {
                if let Some(c) = depot::code_actif(&mut cnx, *id, contexte).await?
                    && crate::secrets::code_acces_correspond(&code, &c.empreinte)
                {
                    code_servi = Some(c);
                    break;
                }
            }
        }
        faits.code_juste = code_servi.is_some();
    }
    let issue = consultation::decider(&faits);
    let responsable_id = code_servi
        .as_ref()
        .map(|c| c.responsable_id)
        .or_else(|| candidats.first().copied());
    let ecole = crate::file_sms::signature(&mut cnx, contexte).await?;
    let aujourd_hui = DateCivile::du_jour(maintenant).ok_or(regle("sms.date_invalide"))?;
    let mut eleves: Vec<String> = Vec::new();
    let reponse = match (issue, &code_servi, demande.as_ref().map(|d| d.mot)) {
        (IssueConsultation::Servie, Some(c), Some(mot)) => {
            // Chaque réponse se calcule sur le tas : les dépôts qu'elle traverse sont profonds.
            let enfants = Box::pin(enfants(&mut cnx, c.responsable_id, contexte)).await?;
            eleves = enfants.iter().map(|e| e.prenom.clone()).collect();
            Some(match mot {
                MotConsultation::Solde => {
                    Box::pin(solde(&mut cnx, &ecole, &enfants, aujourd_hui, contexte)).await?
                }
                MotConsultation::Abs => {
                    Box::pin(absences_enfants(
                        &mut cnx,
                        &ecole,
                        &enfants,
                        aujourd_hui,
                        contexte,
                    ))
                    .await?
                }
                MotConsultation::Note => {
                    Box::pin(notes(&mut cnx, &ecole, &enfants, contexte)).await?
                }
                MotConsultation::Aide => consultation::reponse_aide(&ecole),
            })
        }
        (IssueConsultation::Aide, _, _) => Some(consultation::reponse_aide(&ecole)),
        (IssueConsultation::CodeIncorrect, _, _) => {
            Some(consultation::reponse_code_incorrect(&ecole))
        }
        _ => None,
    };
    drop(cnx);

    let id: Id<SmsRecu> = nouvel_id();
    let mut unite = noyau
        .commencer(contexte.clone(), Durabilite::Ordinaire)
        .await?;
    let mut reponse_id = None;
    // Une réponse ne part que vers un numéro qui peut en recevoir une.
    if let Some(texte) = &reponse
        && numero.starts_with('+')
    {
        let message = crate::file_sms::mettre_en_file(
            unite.connexion(),
            MessageAFaire {
                nature: NatureMessage::Consultation,
                cle_fait: format!("consultation:{}", id.vers_chaine()),
                responsable_id,
                eleve_id: None,
                numero: numero.clone(),
                texte: tenir_en_trois_sms(texte.clone()),
                attente_ms: 0,
                expire_le: None,
            },
            contexte,
        )
        .await?;
        reponse_id = message.map(|m| m.id);
    }
    let sms = SmsRecu {
        id,
        etablissement_id: contexte.etablissement_id,
        numero: numero.clone(),
        texte: consultation::texte_a_garder(&entrant.texte, demande.as_ref()),
        mot: demande.as_ref().map(|d| d.mot),
        issue,
        responsable_id,
        reponse_id,
        recu_le: maintenant,
    };
    depot::inserer_sms_recu(unite.connexion(), &sms, contexte).await?;
    if let Some(c) = &code_servi {
        depot::noter_utilisation(unite.connexion(), c.id, contexte).await?;
    }
    let id_texte = id.vers_chaine();
    unite.auditer(
        "sms_recu",
        &id_texte,
        Operation::Creation,
        None,
        Some(serde_json::json!({
            "numero": sms.numero,
            "mot": sms.mot,
            "issue": sms.issue,
            "responsable_id": sms.responsable_id,
            "eleves": eleves,
            "reponse": reponse,
        })),
        "consultation.recevoir",
    );
    unite.emettre(
        TypeEvenement::SmsRecu,
        "sms_recu",
        &id_texte,
        serde_json::json!({ "numero": sms.numero, "mot": sms.mot, "issue": sms.issue }),
    );
    noyau.terminer(unite).await?;
    Ok(issue)
}

/// Service du code d'accès et des SMS reçus.
pub struct ServiceConsultation<'n> {
    noyau: &'n Noyau,
}

impl<'n> ServiceConsultation<'n> {
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

    /// Remet un nouveau code à un responsable ; l'ancien est révoqué. Rend le code en clair et
    /// les données du billet : ni l'un ni l'autre ne sont gardés.
    pub async fn remettre(
        &self,
        session: &Session,
        demande: &DemandeCodeAcces,
    ) -> Resultat<(String, serde_json::Value, EtatCodeResponsable)> {
        let contexte = self.noyau.contexte(session.etablissement_id, Some(session));
        let code = consultation::code_depuis_octets(&crate::alea::octets::<
            { consultation::LONGUEUR_CODE },
        >()?);
        let empreinte = crate::secrets::empreinte_code_acces(&code)?;
        let mut unite = self
            .noyau
            .commencer(contexte.clone(), Durabilite::Ordinaire)
            .await?;
        if !crate::file_sms::module_actif(unite.connexion(), &contexte).await? {
            return Err(regle("sms.module_inactif"));
        }
        let (_, personne) =
            personnes::responsable(unite.connexion(), demande.responsable_id, &contexte).await?;
        let ancien =
            depot::code_actif(unite.connexion(), demande.responsable_id, &contexte).await?;
        if let Some(a) = &ancien {
            depot::revoquer_code(unite.connexion(), a.id, &contexte).await?;
            unite.auditer(
                "code_acces",
                &a.id.vers_chaine(),
                Operation::Modification,
                Some(serde_json::json!({ "etat": EtatCodeAcces::Actif })),
                Some(serde_json::json!({ "etat": EtatCodeAcces::Revoque, "motif": "remplace" })),
                "consultation.remettre",
            );
        }
        let nouveau = CodeAcces {
            id: nouvel_id(),
            etablissement_id: contexte.etablissement_id,
            responsable_id: demande.responsable_id,
            empreinte,
            etat: EtatCodeAcces::Actif,
            cree_le: contexte.maintenant,
            revoque_le: None,
            derniere_utilisation: None,
        };
        depot::inserer_code(unite.connexion(), &nouveau, &contexte).await?;
        let id = nouveau.id.vers_chaine();
        unite.auditer(
            "code_acces",
            &id,
            Operation::Creation,
            None,
            Some(serde_json::to_value(&nouveau)?),
            "consultation.remettre",
        );
        unite.emettre(
            TypeEvenement::CodeAccesRemis,
            "responsable",
            &demande.responsable_id.vers_chaine(),
            serde_json::json!({ "code_acces_id": id, "remplace": ancien.map(|a| a.id) }),
        );
        let numero_ecole =
            parametre::texte(unite.connexion(), PARAMETRE_NUMERO_ECOLE, "", &contexte).await?;
        let remis_par = utilisateur::lire(unite.connexion(), session.utilisateur_id, &contexte)
            .await
            .map(|u| u.nom_affiche)
            .unwrap_or_default();
        let enfants: Vec<String> = enfants(unite.connexion(), demande.responsable_id, &contexte)
            .await?
            .into_iter()
            .map(|e| e.prenom)
            .collect();
        let etat = etat_code(unite.connexion(), demande.responsable_id, &contexte).await?;
        self.noyau.terminer(unite).await?;
        let lisible = consultation::code_lisible(&code);
        let donnees = serde_json::json!({
            "responsable": personne.nom_complet(),
            "enfants": enfants.join(", "),
            "code": lisible,
            "numero_ecole": telephone::normaliser(&numero_ecole)
                .map(|n| telephone::lisible(&n))
                .unwrap_or_default(),
            "remis_le": crate::documents::date_longue(contexte.maintenant),
            "remis_par": remis_par,
        });
        Ok((lisible, donnees, etat))
    }

    /// Révoque le code d'un responsable : il n'ouvre plus rien.
    pub async fn revoquer(
        &self,
        session: &Session,
        demande: &DemandeCodeAcces,
    ) -> Resultat<EtatCodeResponsable> {
        let contexte = self.noyau.contexte(session.etablissement_id, Some(session));
        let mut unite = self
            .noyau
            .commencer(contexte.clone(), Durabilite::Ordinaire)
            .await?;
        personnes::responsable(unite.connexion(), demande.responsable_id, &contexte).await?;
        let actif = depot::code_actif(unite.connexion(), demande.responsable_id, &contexte)
            .await?
            .ok_or(regle("code_acces.aucun"))?;
        depot::revoquer_code(unite.connexion(), actif.id, &contexte).await?;
        let id = actif.id.vers_chaine();
        unite.auditer(
            "code_acces",
            &id,
            Operation::Modification,
            Some(serde_json::json!({ "etat": EtatCodeAcces::Actif })),
            Some(serde_json::json!({ "etat": EtatCodeAcces::Revoque, "motif": "revoque" })),
            "consultation.revoquer",
        );
        unite.emettre(
            TypeEvenement::CodeAccesRevoque,
            "responsable",
            &demande.responsable_id.vers_chaine(),
            serde_json::json!({ "code_acces_id": id }),
        );
        let etat = etat_code(unite.connexion(), demande.responsable_id, &contexte).await?;
        self.noyau.terminer(unite).await?;
        Ok(etat)
    }

    /// Les SMS reçus, avec le responsable du numéro et la réponse partie.
    pub async fn recus(&self, session: &Session) -> Resultat<SmsRecus> {
        let contexte = self.noyau.contexte(session.etablissement_id, Some(session));
        let mut cnx = self.connexion().await?;
        let mut sms = Vec::new();
        for s in depot::sms_recus(&mut cnx, LISTE_MAX, &contexte).await? {
            let responsable = match s.responsable_id {
                Some(id) => personnes::responsable(&mut cnx, id, &contexte)
                    .await
                    .map(|(_, p)| p.nom_complet())
                    .unwrap_or_default(),
                None => String::new(),
            };
            let reponse = match s.reponse_id {
                Some(id) => file_sms::message(&mut cnx, id, &contexte)
                    .await
                    .map(|m| m.texte)
                    .unwrap_or_default(),
                None => String::new(),
            };
            sms.push(SmsRecuAffiche {
                sms: s,
                responsable,
                reponse,
            });
        }
        let consultation_active =
            parametre::booleen_param(&mut cnx, PARAMETRE_ACTIVE, false, &contexte).await?;
        let simulation = Self::simulation(&mut cnx, &contexte).await?.is_some();
        Ok(SmsRecus {
            sms,
            consultation_active,
            simulation,
        })
    }

    async fn simulation(
        cnx: &mut Connexion,
        contexte: &Contexte,
    ) -> Resultat<Option<CanalDeSimulation>> {
        let canal = canal_en_vigueur(cnx, contexte).await?;
        let port = parametre::texte(cnx, PARAMETRE_PORT, "", contexte).await?;
        Ok(match canal.as_str() {
            "simule" => Some(CanalDeSimulation::Canal),
            "modem" if port == PORT_SIMULE => Some(CanalDeSimulation::Cle),
            _ => None,
        })
    }

    /// Où simuler un SMS reçu, et de quel numéro canonique : le canal simulé, ou la clé simulée ;
    /// refusé sur une vraie clé.
    pub async fn canal_de_simulation(
        &self,
        session: &Session,
        demande: &DemandeSimulationSms,
    ) -> Resultat<(CanalDeSimulation, String)> {
        let contexte = self.noyau.contexte(session.etablissement_id, Some(session));
        if demande.texte.trim().is_empty() || demande.texte.chars().count() > 160 {
            return Err(regle("sms.simulation_texte"));
        }
        let numero = telephone::normaliser(&demande.numero).map_err(regle)?;
        let mut cnx = self.connexion().await?;
        let canal = Self::simulation(&mut cnx, &contexte)
            .await?
            .ok_or(regle("sms.simulation_impossible"))?;
        Ok((canal, numero))
    }
}
