//! Frais au changement de classe (décisions du porteur du 2026-11-13). Au changement, la
//! direction garde les frais actuels (par défaut) ou applique ceux de la nouvelle classe, l'écart
//! montré avant ; le même ajustement reste possible plus tard. Rien ne se réécrit ni ne
//! s'efface : une hausse est une ligne de facture en plus, une baisse des parts portées par les
//! échéances à venir, le trop-perçu un avoir de l'élève. Les versements restent ce qu'ils sont.

use gesco_core::commun::{DateCivile, Id, Montant};
use gesco_core::core::{Operation, Permission, TypeEvenement};
use gesco_core::finance::ajustement::{
    Ajustement, EcartTranche, TrancheDue, comparer, meme_tranche, porter_baisse, totaux,
    utiliser_avoir,
};
use gesco_core::finance::facture::{ArriveeEnCours, Bornes, calculer, lignes_applicables};
use gesco_core::finance::{
    Echeance, Exigibilite, Facture, LigneFacture, LigneTarifaire, StatutEcheance,
};
use gesco_core::scolarite::{Classe, Inscription};
use gesco_db::depots::{academique, ajustements, annee_scolaire, factures, inscriptions};
use gesco_db::depots::{parametre, tarifs};
use gesco_db::unite::Durabilite;
use gesco_db::{Connexion, Contexte, UniteDeTravail, nouvel_id};
use serde::{Deserialize, Serialize};

use crate::effets::Effet;
use crate::erreur::{ErreurService, Resultat};
use crate::facturation::{PARAMETRE_ARRIVEE, etat_echeances, recalculer_dans};
use crate::inscriptions::jour;
use crate::noyau::Noyau;
use crate::session::Session;

fn regle(cle: &'static str) -> ErreurService {
    ErreurService::Regle { cle }
}

fn calcul<T>(r: Result<T, gesco_core::commun::ErreurMontant>) -> Resultat<T> {
    r.map_err(|_| regle("facture.calcul_impossible"))
}

/// Écart demandé : celui d'un changement vers une classe, ou, sans classe, celui qui aligne la
/// facture sur la classe actuelle de l'élève.
#[derive(Debug, Deserialize, specta::Type)]
pub struct DemandeApercuAjustement {
    /// Inscription validée.
    pub inscription_id: Id<Inscription>,
    /// Classe visée ; absente : la classe actuelle.
    #[serde(default)]
    pub classe_id: Option<Id<Classe>>,
}

/// Inscription dont les frais s'alignent sur sa classe actuelle, ou dont l'avoir règle la facture.
#[derive(Debug, Deserialize, specta::Type)]
pub struct DemandeFraisInscription {
    /// Inscription.
    pub inscription_id: Id<Inscription>,
}

/// Ce que changerait l'application des frais de la nouvelle classe, sans rien écrire.
#[derive(Debug, Clone, Serialize, specta::Type)]
pub struct ApercuAjustement {
    /// Classe dont la facture porte les frais.
    pub classe_avant: String,
    /// Classe dont les frais s'appliqueraient.
    pub classe_apres: String,
    /// Vrai si l'inscription n'a pas encore de facture : rien à ajuster.
    pub sans_facture: bool,
    /// Tranches à venir qui changent.
    pub ecarts: Vec<EcartTranche>,
    /// Hausse totale.
    pub hausse: Montant,
    /// Baisse totale, avoir compris.
    pub baisse: Montant,
    /// Part de la baisse déjà payée, qui deviendrait un avoir.
    pub avoir: Montant,
}

/// Hausse d'une nature de frais : sa ligne de la grille et ses échéances (date, libellé, montant).
struct Hausse {
    ligne: LigneTarifaire,
    echeances: Vec<(DateCivile, String, Montant)>,
}

/// Ajustement calculé, prêt à écrire.
pub(crate) struct Prepare {
    facture: Facture,
    classe_avant: Classe,
    classe_apres: Classe,
    depuis: DateCivile,
    ecarts: Vec<EcartTranche>,
    hausses: Vec<Hausse>,
    parts: Vec<(Id<Echeance>, Montant)>,
    avoirs: Vec<(Id<Echeance>, Montant)>,
    lignes: usize,
}

impl Prepare {
    fn apercu(&self) -> Resultat<ApercuAjustement> {
        let (hausse, baisse) = calcul(totaux(&self.ecarts))?;
        let avoir = calcul(
            self.avoirs
                .iter()
                .try_fold(Montant::ZERO, |s, (_, m)| s.ajouter(*m)),
        )?;
        Ok(ApercuAjustement {
            classe_avant: self.classe_avant.libelle.clone(),
            classe_apres: self.classe_apres.libelle.clone(),
            sans_facture: false,
            ecarts: self.ecarts.clone(),
            hausse,
            baisse,
            avoir,
        })
    }
}

async fn bornes(cnx: &mut Connexion, facture: &Facture, contexte: &Contexte) -> Resultat<Bornes> {
    let annee = annee_scolaire::lire(cnx, facture.annee_id, contexte).await?;
    let arrivee = ArriveeEnCours::depuis_nom(
        &parametre::texte(
            cnx,
            PARAMETRE_ARRIVEE,
            ArriveeEnCours::DEFAUT.nom(),
            contexte,
        )
        .await?,
    )
    .unwrap_or(ArriveeEnCours::DEFAUT);
    let multiple = parametre::entier(cnx, "montant.multiple_arrondi", 5, contexte).await?;
    Ok(Bornes {
        debut: jour(annee.debut)?,
        fin: jour(annee.fin)?,
        arrivee,
        multiple,
    })
}

/// Calcule, sans rien écrire, l'ajustement qui porterait la facture de l'inscription aux frais
/// de la classe `apres`, pour les tranches dont l'échéance est au jour `depuis` ou après. Aucun
/// ajustement sans facture.
pub(crate) async fn preparer(
    cnx: &mut Connexion,
    contexte: &Contexte,
    inscription: &Inscription,
    avant: &Classe,
    apres: &Classe,
    depuis: DateCivile,
) -> Resultat<Option<Prepare>> {
    let Some(facture) = factures::facture_vivante(cnx, inscription.id, contexte).await? else {
        return Ok(None);
    };
    let lignes = factures::lignes(cnx, facture.id, contexte).await?;
    let d_ajustement = ajustements::lignes_d_ajustement(cnx, facture.id, contexte).await?;
    let nes = ajustements::avoirs_de_la_facture(cnx, facture.id, contexte).await?;
    let etats = etat_echeances(cnx, facture.id, contexte).await?;
    let contenu = tarifs::contenu(cnx, facture.grille_id, contexte).await?;
    let plan = contenu
        .plans
        .iter()
        .find(|p| p.id == facture.plan_paiement_id)
        .cloned()
        .ok_or(regle("facture.plan_inconnu"))?;
    let selon_plan = |l: &LigneFacture| {
        d_ajustement.contains(&l.id)
            || contenu
                .lignes
                .iter()
                .any(|g| g.id == l.ligne_tarifaire_id && g.exigibilite == Exigibilite::SelonPlan)
    };
    let ligne_de = |e: &Echeance| lignes.iter().find(|l| l.id == e.ligne_facture_id);

    // Ce qui est dû aujourd'hui, tranche par tranche : le montant, moins les baisses déjà
    // portées et les avoirs déjà nés sur l'échéance. Les remises restent celles accordées.
    let mut dues = Vec::new();
    for e in &etats {
        let Some(ligne) = ligne_de(&e.echeance).filter(|l| selon_plan(l)) else {
            continue;
        };
        let ne = calcul(
            nes.iter()
                .filter(|(id, _)| *id == e.echeance.id)
                .try_fold(Montant::ZERO, |s, (_, m)| s.ajouter(*m)),
        )?;
        dues.push(TrancheDue {
            type_frais: ligne.type_frais,
            date: e.echeance.date,
            libelle: e.echeance.libelle.clone(),
            montant: calcul(
                e.echeance
                    .montant
                    .soustraire(e.ajustement)
                    .and_then(|m| m.soustraire(ne)),
            )?,
        });
    }

    // Ce que demanderaient les frais de la nouvelle classe : ceux de la grille en vigueur, qui a
    // pu changer de version depuis la facture, avec le même plan (retrouvé par son libellé) et le
    // même jour d'inscription ; un frais facultatif choisi l'est encore s'il existe pour la
    // nouvelle classe.
    let cible = match tarifs::grille_en_vigueur(cnx, facture.annee_id, contexte).await? {
        Some(g) if g.id != facture.grille_id => tarifs::contenu(cnx, g.id, contexte).await?,
        _ => contenu.clone(),
    };
    let plan = cible
        .plans
        .iter()
        .find(|p| p.id == plan.id)
        .or_else(|| cible.plans.iter().find(|p| p.libelle == plan.libelle))
        .or_else(|| cible.plans.iter().find(|p| p.par_defaut))
        .cloned()
        .ok_or(regle("facture.plan_inconnu"))?;
    let applicables = lignes_applicables(&cible.lignes, apres.niveau_id, apres.serie_id);
    let options: Vec<Id<LigneTarifaire>> = lignes
        .iter()
        .filter(|l| !d_ajustement.contains(&l.id))
        .filter_map(|l| {
            contenu
                .lignes
                .iter()
                .find(|g| g.id == l.ligne_tarifaire_id && !g.obligatoire)
        })
        .filter_map(|g| {
            applicables
                .iter()
                .find(|n| !n.obligatoire && n.type_frais == g.type_frais)
                .map(|n| n.id)
        })
        .collect();
    let tarif = gesco_core::finance::ContexteTarifaire {
        inscription_id: inscription.id,
        annee_id: inscription.annee_id,
        niveau_id: apres.niveau_id,
        serie_id: apres.serie_id,
        date_inscription: inscription.date_inscription,
    };
    let bornes = bornes(cnx, &facture, contexte).await?;
    let calculees = calculer(&cible.lignes, &tarif, &options, &plan, &bornes).map_err(|cle| {
        regle(if cle == "facture.grille_absente" {
            "ajustement.frais_absents"
        } else {
            cle
        })
    })?;
    let attendues: Vec<TrancheDue> = calculees
        .iter()
        .filter(|c| c.ligne.exigibilite == Exigibilite::SelonPlan)
        .flat_map(|c| {
            c.echeances.iter().map(|e| TrancheDue {
                type_frais: c.ligne.type_frais,
                date: e.date,
                libelle: e.libelle.clone(),
                montant: e.montant,
            })
        })
        .collect();
    let ecarts = calcul(comparer(&dues, &attendues, depuis))?;

    let mut hausses: Vec<Hausse> = Vec::new();
    let mut parts = Vec::new();
    let mut avoirs = Vec::new();
    for ecart in &ecarts {
        let signe = calcul(ecart.ecart())?;
        if signe.est_negatif() {
            let restes: Vec<(Id<Echeance>, Montant)> = etats
                .iter()
                .filter(|e| {
                    meme_tranche(e.echeance.date, ecart.date)
                        && ligne_de(&e.echeance)
                            .is_some_and(|l| l.type_frais == ecart.type_frais && selon_plan(l))
                })
                .map(|e| (e.echeance.id, e.reste))
                .collect();
            let (p, a) = calcul(porter_baisse(&restes, calcul(signe.oppose())?))?;
            parts.extend(p);
            avoirs.extend(a);
        } else {
            let ligne = calculees
                .iter()
                .find(|c| c.ligne.type_frais == ecart.type_frais)
                .map(|c| c.ligne.clone())
                .ok_or(regle("facture.calcul_impossible"))?;
            let echeance = (ecart.date, ecart.libelle.clone(), signe);
            match hausses
                .iter_mut()
                .find(|h| h.ligne.type_frais == ecart.type_frais)
            {
                Some(h) => h.echeances.push(echeance),
                None => hausses.push(Hausse {
                    ligne,
                    echeances: vec![echeance],
                }),
            }
        }
    }
    Ok(Some(Prepare {
        facture,
        classe_avant: avant.clone(),
        classe_apres: apres.clone(),
        depuis,
        ecarts,
        hausses,
        parts,
        avoirs,
        lignes: lignes.len(),
    }))
}

/// Écrit un ajustement préparé dans la transaction ouverte : la ligne de la hausse et ses
/// échéances, les parts de la baisse, les avoirs ; la facture se recalcule. Rien sans écart.
pub(crate) async fn appliquer_dans(
    unite: &mut UniteDeTravail<Effet>,
    contexte: &Contexte,
    prepare: &Prepare,
) -> Resultat<Option<Ajustement>> {
    if prepare.ecarts.is_empty() {
        return Ok(None);
    }
    let apercu = prepare.apercu()?;
    let ajustement = Ajustement {
        id: nouvel_id(),
        etablissement_id: contexte.etablissement_id,
        facture_id: prepare.facture.id,
        date: prepare.depuis,
        hausse: apercu.hausse,
        baisse: apercu.baisse,
        avoir: apercu.avoir,
        classe_avant_id: prepare.classe_avant.id,
        classe_apres_id: prepare.classe_apres.id,
    };
    ajustements::inserer(unite.connexion(), &ajustement, contexte).await?;
    let mut ordre = prepare.lignes;
    let mut rang =
        factures::nombre_d_echeances(unite.connexion(), prepare.facture.id, contexte).await?;
    for h in &prepare.hausses {
        let ligne = LigneFacture {
            id: nouvel_id(),
            ligne_tarifaire_id: h.ligne.id,
            type_frais: h.ligne.type_frais,
            libelle: format!(
                "{} · ajustement {} → {}",
                h.ligne.libelle, prepare.classe_avant.libelle, prepare.classe_apres.libelle
            ),
            montant: calcul(
                h.echeances
                    .iter()
                    .try_fold(Montant::ZERO, |s, (_, _, m)| s.ajouter(*m)),
            )?,
        };
        factures::inserer_ligne(
            unite.connexion(),
            prepare.facture.id,
            ordre,
            &ligne,
            contexte,
        )
        .await?;
        ordre += 1;
        ajustements::marquer_ligne(unite.connexion(), ligne.id, ajustement.id, contexte).await?;
        for (date, libelle, montant) in &h.echeances {
            rang += 1;
            let echeance = Echeance {
                id: nouvel_id(),
                ligne_facture_id: ligne.id,
                libelle: format!("{libelle} · ajustement"),
                date: *date,
                montant: *montant,
                statut: StatutEcheance::Due,
            };
            factures::inserer_echeance(
                unite.connexion(),
                prepare.facture.id,
                rang,
                &echeance,
                contexte,
            )
            .await?;
        }
    }
    for (echeance, montant) in &prepare.parts {
        ajustements::inserer_part(
            unite.connexion(),
            ajustement.id,
            *echeance,
            *montant,
            contexte,
        )
        .await?;
    }
    for (echeance, montant) in &prepare.avoirs {
        ajustements::inserer_avoir(
            unite.connexion(),
            ajustement.id,
            *echeance,
            *montant,
            contexte,
        )
        .await?;
    }
    let apres = recalculer_dans(unite, contexte, &prepare.facture).await?;
    let id = prepare.facture.id.vers_chaine();
    unite.auditer(
        "facture",
        &id,
        Operation::Modification,
        Some(serde_json::to_value(&prepare.facture)?),
        Some(
            serde_json::json!({ "facture": apres, "ajustement": ajustement,
            "ecarts": prepare.ecarts }),
        ),
        "ajustements.appliquer",
    );
    unite.emettre(
        TypeEvenement::FactureAjustee,
        "facture",
        &id,
        serde_json::json!({
            "inscription_id": prepare.facture.inscription_id,
            "hausse": ajustement.hausse,
            "baisse": ajustement.baisse,
            "avoir": ajustement.avoir,
        }),
    );
    Ok(Some(ajustement))
}

/// Service des frais au changement de classe.
pub struct ServiceAjustements<'n> {
    noyau: &'n Noyau,
}

impl<'n> ServiceAjustements<'n> {
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

    /// Inscription et classes de l'ajustement : celle dont la facture porte les frais (la classe
    /// quittée lors du dernier changement, sinon l'actuelle) et celle visée.
    async fn classes(
        &self,
        cnx: &mut Connexion,
        contexte: &Contexte,
        inscription_id: Id<Inscription>,
        visee: Option<Id<Classe>>,
    ) -> Resultat<(Inscription, Classe, Classe)> {
        let inscription = inscriptions::lire(cnx, inscription_id, contexte).await?;
        let (avant, apres) = match visee {
            Some(c) if c != inscription.classe_id => (inscription.classe_id, c),
            _ => (
                inscription
                    .classe_precedente_id
                    .unwrap_or(inscription.classe_id),
                inscription.classe_id,
            ),
        };
        let avant = academique::classe(cnx, avant, contexte).await?;
        let apres = academique::classe(cnx, apres, contexte).await?;
        Ok((inscription, avant, apres))
    }

    /// Écart entre les frais de la facture et ceux d'une classe, sans rien écrire.
    pub async fn apercu(
        &self,
        session: &Session,
        demande: &DemandeApercuAjustement,
    ) -> Resultat<ApercuAjustement> {
        let contexte = self.noyau.contexte(session.etablissement_id, Some(session));
        let mut cnx = self.connexion().await?;
        let (inscription, avant, apres) = self
            .classes(
                &mut cnx,
                &contexte,
                demande.inscription_id,
                demande.classe_id,
            )
            .await?;
        let depuis = jour(contexte.maintenant)?;
        match preparer(&mut cnx, &contexte, &inscription, &avant, &apres, depuis).await? {
            Some(p) => p.apercu(),
            None => Ok(ApercuAjustement {
                classe_avant: avant.libelle,
                classe_apres: apres.libelle,
                sans_facture: true,
                ecarts: Vec::new(),
                hausse: Montant::ZERO,
                baisse: Montant::ZERO,
                avoir: Montant::ZERO,
            }),
        }
    }

    /// Applique à la facture les frais de la classe actuelle de l'élève, pour les tranches à
    /// venir ; rend ce qui a été appliqué.
    pub async fn ajuster(
        &self,
        session: &Session,
        demande: &DemandeFraisInscription,
    ) -> Resultat<ApercuAjustement> {
        let contexte = self.noyau.contexte(session.etablissement_id, Some(session));
        let mut unite = self
            .noyau
            .commencer(contexte.clone(), Durabilite::Critique)
            .await?;
        let (inscription, avant, apres) = self
            .classes(unite.connexion(), &contexte, demande.inscription_id, None)
            .await?;
        let depuis = jour(contexte.maintenant)?;
        let prepare = preparer(
            unite.connexion(),
            &contexte,
            &inscription,
            &avant,
            &apres,
            depuis,
        )
        .await?
        .ok_or(regle("facture.absente"))?;
        let apercu = prepare.apercu()?;
        appliquer_dans(&mut unite, &contexte, &prepare).await?;
        self.noyau.terminer(unite).await?;
        Ok(apercu)
    }

    /// Règle les échéances de la facture de l'inscription avec les avoirs de l'élève, la plus
    /// ancienne d'abord, sans encaissement : la recette a déjà été comptée quand l'argent est
    /// entré. Rend le montant utilisé.
    pub async fn utiliser_avoirs(
        &self,
        session: &Session,
        demande: &DemandeFraisInscription,
    ) -> Resultat<Montant> {
        let contexte = self.noyau.contexte(session.etablissement_id, Some(session));
        let mut unite = self
            .noyau
            .commencer(contexte.clone(), Durabilite::Critique)
            .await?;
        let inscription =
            inscriptions::lire(unite.connexion(), demande.inscription_id, &contexte).await?;
        let facture = factures::facture_vivante(unite.connexion(), inscription.id, &contexte)
            .await?
            .ok_or(regle("facture.absente"))?;
        let mut restes: Vec<(Id<Echeance>, DateCivile, Montant)> =
            etat_echeances(unite.connexion(), facture.id, &contexte)
                .await?
                .into_iter()
                .filter(|e| !e.reste.est_nul())
                .map(|e| (e.echeance.id, e.echeance.date, e.reste))
                .collect();
        restes.sort_by_key(|(_, date, _)| *date);
        let avoirs =
            ajustements::avoirs_de_l_eleve(unite.connexion(), inscription.eleve_id, &contexte)
                .await?;
        let mut utilise = Montant::ZERO;
        let mut usages = Vec::new();
        for avoir in &avoirs {
            let solde = calcul(avoir.solde())?;
            if solde.est_nul() || solde.est_negatif() {
                continue;
            }
            let a_regler: Vec<(Id<Echeance>, Montant)> =
                restes.iter().map(|(id, _, r)| (*id, *r)).collect();
            for (echeance, montant) in calcul(utiliser_avoir(&a_regler, solde))? {
                ajustements::imputer_avoir(
                    unite.connexion(),
                    avoir.id,
                    echeance,
                    montant,
                    &contexte,
                )
                .await?;
                if let Some(r) = restes.iter_mut().find(|(id, _, _)| *id == echeance) {
                    r.2 = calcul(r.2.soustraire(montant))?;
                }
                utilise = calcul(utilise.ajouter(montant))?;
                usages.push(serde_json::json!({ "avoir": avoir.id, "echeance": echeance,
                    "montant": montant }));
            }
        }
        if utilise.est_nul() {
            return Ok(utilise);
        }
        let apres = recalculer_dans(&mut unite, &contexte, &facture).await?;
        let id = facture.id.vers_chaine();
        unite.auditer(
            "facture",
            &id,
            Operation::Modification,
            Some(serde_json::to_value(&facture)?),
            Some(serde_json::json!({ "facture": apres, "avoirs_utilises": usages })),
            "ajustements.utiliser_avoirs",
        );
        unite.emettre(
            TypeEvenement::AvoirUtilise,
            "facture",
            &id,
            serde_json::json!({ "inscription_id": inscription.id, "montant": utilise }),
        );
        self.noyau.terminer(unite).await?;
        Ok(utilise)
    }
}

/// Vrai si la session peut décider d'appliquer les frais de la nouvelle classe : la direction.
pub(crate) fn decide_les_frais(permissions: &[Permission]) -> bool {
    permissions.contains(&Permission::ENROLLMENTS_VALIDATE)
}
