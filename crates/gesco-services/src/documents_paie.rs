//! Documents de la paie (SPEC_PHASE_7 §8, T-57) : bulletins de paie, état de paie à signer pour
//! acquit, état des retenues et cotisations ; « Mes bulletins de paie » pour chacun. Le service
//! prépare les données ; le moteur documentaire compose le PDF.

use gesco_core::commun::{Id, Montant};
use gesco_core::paie::calcul::{LigneCalculee, SorteLigne};
use gesco_core::paie::{BulletinPaie, PaieMois, PaiementPersonnel, StatutPaie};
use gesco_core::rh::heures::ecrire_heures;
use gesco_core::rh::{MembrePersonnel, Mois, SituationFamille};
use gesco_db::Contexte;
use gesco_db::depots::{paie, personnel, personnes};
use serde::{Deserialize, Serialize};

use crate::erreur::{ErreurService, Resultat};
use crate::heures::mois_long;
use crate::noyau::Noyau;
use crate::session::Session;

fn regle(cle: &'static str) -> ErreurService {
    ErreurService::Regle { cle }
}

/// Bulletin de paie désigné.
#[derive(Debug, Deserialize, specta::Type)]
pub struct DemandeBulletinPaie {
    /// Bulletin.
    pub id: Id<BulletinPaie>,
}

/// Bulletin de paie d'un membre, dans « Mon compte ».
#[derive(Debug, Clone, Serialize, specta::Type)]
pub struct MonBulletinPaie {
    /// Bulletin.
    pub id: Id<BulletinPaie>,
    /// Mois.
    pub mois: Mois,
    /// Numéro.
    pub numero: String,
    /// Net à payer.
    pub net: Montant,
}

/// Taux en centièmes de pour cent : 306 → « 3,06 % », 360 → « 3,6 % ».
fn texte_taux(taux: i64) -> String {
    let entier = taux / 100;
    let reste = taux % 100;
    if reste == 0 {
        format!("{entier} %")
    } else {
        let decimales = format!("{reste:02}");
        format!("{entier},{} %", decimales.trim_end_matches('0'))
    }
}

/// Détail imprimé d'une ligne : jours, heures et taux, base, rappel, jour d'absence, reste.
fn detail(l: &LigneCalculee) -> String {
    let mut parts = Vec::new();
    if let Some((payes, sur)) = l.jours {
        parts.push(format!("{payes}/{sur} jours"));
    }
    if let Some(minutes) = l.minutes {
        parts.push(ecrire_heures(minutes));
    }
    if let Some(t) = l.taux {
        parts.push(texte_taux(t));
    }
    if let Some(base) = l.base {
        let mot = if l.minutes.is_some() { "à" } else { "base" };
        parts.push(format!("{mot} {}", base.formater()));
    }
    if let Some(m) = l.mois_rappel {
        parts.push(format!("rappel de {}", mois_long(m)));
    }
    if let Some(d) = l.date {
        parts.push(crate::documents::date_civile_longue(d));
    }
    if let Some(r) = l.reste {
        parts.push(if r.est_nul() {
            "avance soldée".to_owned()
        } else {
            format!("reste dû {}", r.formater())
        });
    }
    parts.join(" · ")
}

fn lignes(b: &BulletinPaie, sortes: &[SorteLigne]) -> Vec<serde_json::Value> {
    b.resultat
        .lignes
        .iter()
        .filter(|l| sortes.contains(&l.sorte))
        .map(|l| {
            serde_json::json!({
                "libelle": l.libelle,
                "detail": detail(l),
                "montant": l.montant.formater(),
            })
        })
        .collect()
}

fn situation(m: &MembrePersonnel) -> String {
    let etat = match m.situation_famille {
        Some(SituationFamille::Marie) => "Marié(e)",
        Some(SituationFamille::Celibataire) => "Célibataire",
        Some(SituationFamille::Veuf) => "Veuf ou veuve",
        Some(SituationFamille::Divorce) => "Divorcé(e)",
        None => "",
    };
    match m.enfants_a_charge {
        Some(n) if n > 0 && !etat.is_empty() => format!("{etat}, {n} enfant(s) à charge"),
        Some(n) if n > 0 => format!("{n} enfant(s) à charge"),
        _ => etat.to_owned(),
    }
}

fn paye_sur(paiements: &[PaiementPersonnel], b: &BulletinPaie) -> Montant {
    paiements
        .iter()
        .filter(|p| !p.annule && p.bulletin_id == Some(b.id))
        .fold(Montant::ZERO, |s, p| s.ajouter(p.montant).unwrap_or(s))
}

/// Service des documents de la paie.
pub struct ServiceDocumentsPaie<'n> {
    noyau: &'n Noyau,
}

impl<'n> ServiceDocumentsPaie<'n> {
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

    /// Paie calculée d'un mois et ses bulletins non exclus, par nom de membre.
    async fn paie_et_bulletins(
        &self,
        cnx: &mut gesco_db::Connexion,
        mois: Mois,
        contexte: &Contexte,
    ) -> Resultat<(PaieMois, Vec<(MembrePersonnel, String, BulletinPaie)>)> {
        let paie_mois = paie::paie_du_mois(cnx, mois, contexte)
            .await?
            .ok_or(regle("paie.non_calculee"))?;
        let mut bulletins = Vec::new();
        for b in paie::bulletins_de_la_paie(cnx, paie_mois.id, contexte)
            .await?
            .into_iter()
            .filter(|b| !b.exclu)
        {
            let m = personnel::membre(cnx, b.membre_id, contexte).await?;
            let nom = personnes::personne(cnx, m.personne_id, contexte)
                .await?
                .nom_complet();
            bulletins.push((m, nom, b));
        }
        bulletins.sort_by(|a, b| a.1.cmp(&b.1));
        Ok((paie_mois, bulletins))
    }

    /// Données des bulletins de paie d'un mois, ou d'un seul.
    pub async fn donnees_bulletins(
        &self,
        session: &Session,
        mois: Mois,
        seul: Option<Id<BulletinPaie>>,
    ) -> Resultat<serde_json::Value> {
        let contexte = self.noyau.contexte(session.etablissement_id, Some(session));
        let mut cnx = self.connexion().await?;
        let (paie_mois, bulletins) = self.paie_et_bulletins(&mut cnx, mois, &contexte).await?;
        let categories = personnel::categories(&mut cnx, &contexte).await?;
        let paiements = paie::paiements(&mut cnx, &contexte).await?;
        let projet = paie_mois.statut != StatutPaie::Validee;
        let donnees: Vec<serde_json::Value> = bulletins
            .iter()
            .filter(|(_, _, b)| seul.is_none_or(|id| id == b.id))
            .map(|(m, nom, b)| {
                let r = &b.resultat;
                let paye = paye_sur(&paiements, b);
                serde_json::json!({
                    "mois": mois_long(mois),
                    "annee": mois.annee(),
                    "numero": b.numero.clone().unwrap_or_default(),
                    "projet": projet,
                    "membre": {
                        "nom": nom,
                        "matricule": m.matricule,
                        "fonction": m.fonction,
                        "categorie": categories.iter().find(|c| c.id == m.categorie_id)
                            .map(|c| c.libelle.clone()).unwrap_or_default(),
                        "inps": m.numero_inps,
                        "fonctionnaire": if m.fonctionnaire { m.numero_fonctionnaire.as_str() } else { "" },
                        "situation": situation(m),
                    },
                    "gains": lignes(b, &[SorteLigne::Base, SorteLigne::Heures, SorteLigne::Gain]),
                    "retenues": lignes(b, &[SorteLigne::Retenue, SorteLigne::Absence]),
                    "avances": lignes(b, &[SorteLigne::Avance]),
                    "patronales": lignes(b, &[SorteLigne::Patronale]),
                    "brut": r.brut.formater(),
                    "total_retenues": r.retenues.formater(),
                    "total_avances": r.avances.formater(),
                    "net": r.net.formater(),
                    "paye": paye.formater(),
                    "reste": r.net.soustraire(paye).unwrap_or(r.net).formater(),
                    "cumuls": {
                        "brut": r.cumuls.brut.formater(),
                        "retenues": r.cumuls.retenues.formater(),
                        "net": r.cumuls.net.formater(),
                    },
                    "avertissements": r.avertissements.iter().filter_map(|a| match a.as_str() {
                        "paie.avance_reportee" => Some("Une avance n'a pu être retenue en entier : le reste est reporté."),
                        _ => None,
                    }).collect::<Vec<_>>(),
                })
            })
            .collect();
        if donnees.is_empty() {
            return Err(regle("paie.aucun_bulletin"));
        }
        Ok(serde_json::json!({ "bulletins": donnees }))
    }

    /// Données de l'état de paie du mois, à signer pour acquit.
    pub async fn donnees_etat_paie(
        &self,
        session: &Session,
        mois: Mois,
    ) -> Resultat<serde_json::Value> {
        let contexte = self.noyau.contexte(session.etablissement_id, Some(session));
        let mut cnx = self.connexion().await?;
        let (paie_mois, bulletins) = self.paie_et_bulletins(&mut cnx, mois, &contexte).await?;
        let paiements = paie::paiements(&mut cnx, &contexte).await?;
        let mut totaux = [Montant::ZERO; 6];
        let mut ajouter = |i: usize, m: Montant| {
            if let Some(t) = totaux.get_mut(i) {
                *t = t.ajouter(m).unwrap_or(*t);
            }
        };
        let lignes: Vec<serde_json::Value> = bulletins
            .iter()
            .map(|(m, nom, b)| {
                let r = &b.resultat;
                let paye = paye_sur(&paiements, b);
                let reste = r.net.soustraire(paye).unwrap_or(r.net);
                for (i, v) in [r.brut, r.retenues, r.avances, r.net, paye, reste]
                    .into_iter()
                    .enumerate()
                {
                    ajouter(i, v);
                }
                serde_json::json!({
                    "numero": b.numero.clone().unwrap_or_default(),
                    "membre": nom,
                    "matricule": m.matricule,
                    "fonction": m.fonction,
                    "brut": r.brut.formater(),
                    "retenues": r.retenues.formater(),
                    "avances": r.avances.formater(),
                    "net": r.net.formater(),
                    "paye": paye.formater(),
                    "reste": reste.formater(),
                })
            })
            .collect();
        let [brut, retenues, avances, net, paye, reste] = totaux;
        Ok(serde_json::json!({
            "mois": mois_long(mois),
            "projet": paie_mois.statut != StatutPaie::Validee,
            "lignes": lignes,
            "totaux": {
                "brut": brut.formater(), "retenues": retenues.formater(),
                "avances": avances.formater(), "net": net.formater(),
                "paye": paye.formater(), "reste": reste.formater(),
            },
            "date": crate::documents::date_longue(self.noyau.horloge().maintenant()),
        }))
    }

    /// Données de l'état des retenues et cotisations du mois.
    pub async fn donnees_etat_retenues(
        &self,
        session: &Session,
        mois: Mois,
    ) -> Resultat<serde_json::Value> {
        let contexte = self.noyau.contexte(session.etablissement_id, Some(session));
        let mut cnx = self.connexion().await?;
        let (paie_mois, bulletins) = self.paie_et_bulletins(&mut cnx, mois, &contexte).await?;
        let elements = paie::elements(&mut cnx, &contexte).await?;
        let mut sortie = Vec::new();
        for element in &elements {
            let mut total = Montant::ZERO;
            let mut nature = "";
            let lignes: Vec<serde_json::Value> = bulletins
                .iter()
                .flat_map(|(_, nom, b)| {
                    b.resultat
                        .lignes
                        .iter()
                        .filter(|l| {
                            l.element_id == Some(element.id)
                                && matches!(l.sorte, SorteLigne::Retenue | SorteLigne::Patronale)
                        })
                        .map(move |l| (nom, l))
                })
                .map(|(nom, l)| {
                    total = total.ajouter(l.montant).unwrap_or(total);
                    nature = if l.sorte == SorteLigne::Patronale {
                        "part de l'école"
                    } else {
                        "retenue sur le salaire"
                    };
                    serde_json::json!({
                        "membre": nom,
                        "assiette": l.base.map(|b| b.formater()).unwrap_or_default(),
                        "taux": l.taux.map(texte_taux).unwrap_or_default(),
                        "montant": l.montant.formater(),
                    })
                })
                .collect();
            if !lignes.is_empty() {
                sortie.push(serde_json::json!({
                    "libelle": element.libelle,
                    "nature": nature,
                    "lignes": lignes,
                    "total": total.formater(),
                }));
            }
        }
        Ok(serde_json::json!({
            "mois": mois_long(mois),
            "projet": paie_mois.statut != StatutPaie::Validee,
            "elements": sortie,
            "date": crate::documents::date_longue(self.noyau.horloge().maintenant()),
        }))
    }

    /// Mois de la paie d'un bulletin.
    pub async fn mois_du_bulletin(
        &self,
        session: &Session,
        id: Id<BulletinPaie>,
    ) -> Resultat<(Mois, Id<MembrePersonnel>)> {
        let contexte = self.noyau.contexte(session.etablissement_id, Some(session));
        let mut cnx = self.connexion().await?;
        for p in paie::paies(&mut cnx, &contexte).await? {
            if let Some(b) = paie::bulletins_de_la_paie(&mut cnx, p.id, &contexte)
                .await?
                .into_iter()
                .find(|b| b.id == id)
            {
                return Ok((p.mois, b.membre_id));
            }
        }
        Err(regle("paie.aucun_bulletin"))
    }

    /// Bulletins validés de qui est connecté, s'il est membre du personnel.
    pub async fn mes_bulletins(&self, session: &Session) -> Resultat<Vec<MonBulletinPaie>> {
        let contexte = self.noyau.contexte(session.etablissement_id, Some(session));
        let mut cnx = self.connexion().await?;
        let Some(membre) =
            personnel::membre_du_compte(&mut cnx, session.utilisateur_id, &contexte).await?
        else {
            return Ok(Vec::new());
        };
        let mut siens: Vec<MonBulletinPaie> =
            paie::bulletins_valides_du_membre(&mut cnx, membre.id, &contexte)
                .await?
                .into_iter()
                .map(|(mois, b)| MonBulletinPaie {
                    id: b.id,
                    mois,
                    numero: b.numero.unwrap_or_default(),
                    net: b.resultat.net,
                })
                .collect();
        siens.reverse();
        Ok(siens)
    }

    /// Données d'un des bulletins validés de qui est connecté ; celui d'un autre est refusé.
    pub async fn donnees_mon_bulletin(
        &self,
        session: &Session,
        id: Id<BulletinPaie>,
    ) -> Resultat<(Mois, serde_json::Value)> {
        let mois = self
            .mes_bulletins(session)
            .await?
            .into_iter()
            .find(|b| b.id == id)
            .map(|b| b.mois)
            .ok_or(regle("paie.bulletin_d_un_autre"))?;
        Ok((mois, self.donnees_bulletins(session, mois, Some(id)).await?))
    }
}
