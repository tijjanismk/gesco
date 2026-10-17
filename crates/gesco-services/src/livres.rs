//! Livres des comptes de trésorerie (SPEC_PHASE_8 §5, §6) : le solde de chaque compte, ses
//! lignes du mois, les transferts entre comptes, le rapprochement contre le relevé ; les salaires
//! et les avances payés qui deviennent des dépenses « Salaires », une seule fois.

use gesco_core::commun::{DateCivile, Id, Montant};
use gesco_core::core::{Operation, TypeEvenement};
use gesco_core::finance::{ModePaiement, MouvementCaisse, NatureMouvement};
use gesco_core::paie::paiement::{ObjetPaiement, SourcePaiement};
use gesco_core::rh::Mois;
use gesco_core::tresorerie::livre::{
    LigneLivre, OrigineLigne, Soldes, Transfert, compte_du_mode, soldes, verifier_transfert,
};
use gesco_core::tresorerie::{
    CATEGORIE_SALAIRES, CompteTresorerie, Depense, NatureCompte, StatutDepense,
};
use gesco_db::depots::{
    caisse, depenses, livres, paie, personnel, personnes, recettes, tresorerie,
};
use gesco_db::unite::Durabilite;
use gesco_db::{Contexte, UniteDeTravail, nouvel_id};
use serde::{Deserialize, Serialize};

use crate::effets::Effet;
use crate::erreur::{ErreurService, Resultat};
use crate::noyau::Noyau;
use crate::session::Session;

fn regle(cle: &'static str) -> ErreurService {
    ErreurService::Regle { cle }
}

fn calcul() -> ErreurService {
    regle("tresorerie.calcul_impossible")
}

/// Modes hors espèces, dont l'école choisit le compte.
pub const MODES_HORS_ESPECES: [ModePaiement; 5] = [
    ModePaiement::OrangeMoney,
    ModePaiement::Wave,
    ModePaiement::Moov,
    ModePaiement::Virement,
    ModePaiement::Cheque,
];

/// Soldes d'un compte.
#[derive(Debug, Clone, Serialize, specta::Type)]
pub struct SoldeCompte {
    /// Compte.
    pub compte_id: Id<CompteTresorerie>,
    /// Soldes à ce jour.
    pub soldes: Soldes,
}

/// Compte où arrive l'argent d'un mode.
#[derive(Debug, Clone, Serialize, specta::Type)]
pub struct ModeCompte {
    /// Mode.
    pub mode: ModePaiement,
    /// Compte ; absent quand l'école n'a pas de compte de cette nature.
    pub compte_id: Option<Id<CompteTresorerie>>,
}

/// Soldes des comptes et compte de chaque mode.
#[derive(Debug, Clone, Serialize, specta::Type)]
pub struct EtatComptes {
    /// Soldes de chaque compte.
    pub soldes: Vec<SoldeCompte>,
    /// Compte de chaque mode hors espèces.
    pub modes: Vec<ModeCompte>,
    /// Paiements du personnel qui attendent un compte pour leur mode.
    pub salaires_sans_compte: u32,
}

/// Livre d'un compte pour un mois.
#[derive(Debug, Clone, Serialize, specta::Type)]
pub struct LivreCompte {
    /// Compte.
    pub compte: CompteTresorerie,
    /// Mois.
    pub mois: Mois,
    /// Solde au premier jour du mois.
    pub report: Montant,
    /// Lignes du mois.
    pub lignes: Vec<LigneLivre>,
    /// Solde au dernier jour du mois.
    pub fin: Montant,
    /// Soldes à ce jour.
    pub soldes: Soldes,
}

/// Livre d'un compte à lire.
#[derive(Debug, Deserialize, specta::Type)]
pub struct DemandeLivre {
    /// Compte.
    pub compte_id: Id<CompteTresorerie>,
    /// Mois.
    pub mois: Mois,
}

/// Transfert d'un compte à un autre.
#[derive(Debug, Deserialize, specta::Type)]
pub struct DemandeTransfertCompte {
    /// Jour, `AAAA-MM-JJ` ; vide : aujourd'hui.
    pub date: String,
    /// Compte d'où l'argent sort.
    pub source_id: Id<CompteTresorerie>,
    /// Compte où il entre.
    pub destination_id: Id<CompteTresorerie>,
    /// Montant.
    pub montant: Montant,
    /// Référence.
    pub reference: String,
    /// Motif.
    pub motif: String,
}

/// Coche ou décoche une ligne du livre.
#[derive(Debug, Deserialize, specta::Type)]
pub struct DemandePointage {
    /// Compte.
    pub compte_id: Id<CompteTresorerie>,
    /// Mois du livre affiché.
    pub mois: Mois,
    /// Origine de la ligne.
    pub origine: OrigineLigne,
    /// Identifiant de l'écriture d'origine.
    pub origine_id: String,
    /// Vrai pour cocher.
    pub pointee: bool,
}

/// Compte choisi pour un mode.
#[derive(Debug, Deserialize, specta::Type)]
pub struct DemandeCompteDuMode {
    /// Mode hors espèces.
    pub mode: ModePaiement,
    /// Compte.
    pub compte_id: Id<CompteTresorerie>,
}

/// Toutes les lignes d'un compte depuis son ouverture, dans l'ordre des jours.
pub(crate) async fn lignes_du_compte(
    cnx: &mut gesco_db::Connexion,
    compte: &CompteTresorerie,
    comptes: &[CompteTresorerie],
    choix: &[(ModePaiement, Id<CompteTresorerie>)],
    contexte: &Contexte,
) -> Resultat<Vec<LigneLivre>> {
    let du_mode = |m: ModePaiement| compte_du_mode(m, comptes, choix) == Some(compte.id);
    let jour = |h| DateCivile::du_jour(h).ok_or(calcul());
    let mut lignes = Vec::new();
    for v in livres::versements_entrants(cnx, contexte).await? {
        if du_mode(v.mode) {
            lignes.push(LigneLivre {
                date: jour(v.le)?,
                origine: OrigineLigne::Versement,
                origine_id: v.id,
                libelle: format!("Reçu {} · {}", v.numero_recu, v.payeur),
                reference: v.reference,
                entree: v.montant,
                sortie: Montant::ZERO,
                pointee: false,
            });
        }
    }
    for m in caisse::mouvements(cnx, contexte).await? {
        if du_mode(m.mode) {
            let entree = m.nature.est_entree();
            lignes.push(LigneLivre {
                date: jour(m.le)?,
                origine: OrigineLigne::Mouvement,
                origine_id: m.id.vers_chaine(),
                libelle: m.motif,
                reference: String::new(),
                entree: if entree { m.montant } else { Montant::ZERO },
                sortie: if entree { Montant::ZERO } else { m.montant },
                pointee: false,
            });
        }
    }
    for d in depenses::payees_hors_session(cnx, contexte).await? {
        if d.compte_id == Some(compte.id) {
            lignes.push(LigneLivre {
                date: d.payee_le.and_then(DateCivile::du_jour).unwrap_or(d.date),
                origine: OrigineLigne::Depense,
                origine_id: d.id.vers_chaine(),
                libelle: format!("{} · {}", d.libelle, d.beneficiaire),
                reference: d.reference,
                entree: Montant::ZERO,
                sortie: d.montant,
                pointee: false,
            });
        }
    }
    // Côté caisse, un transfert ou une recette est un mouvement de la session : il est déjà
    // compté.
    if compte.nature != NatureCompte::Caisse {
        for r in recettes::recettes(cnx, contexte).await? {
            if r.annulee || r.compte_id != compte.id {
                continue;
            }
            lignes.push(LigneLivre {
                date: r.date,
                origine: OrigineLigne::Recette,
                origine_id: r.id.vers_chaine(),
                libelle: format!("{} · {}", r.libelle, r.payeur),
                reference: r.reference,
                entree: r.montant,
                sortie: Montant::ZERO,
                pointee: false,
            });
        }
        for t in livres::transferts(cnx, contexte).await? {
            let (entree, sortie) = if t.destination_id == compte.id {
                (t.montant, Montant::ZERO)
            } else if t.source_id == compte.id {
                (Montant::ZERO, t.montant)
            } else {
                continue;
            };
            lignes.push(LigneLivre {
                date: t.date,
                origine: OrigineLigne::Transfert,
                origine_id: t.id.vers_chaine(),
                libelle: t.motif,
                reference: t.reference,
                entree,
                sortie,
                pointee: false,
            });
        }
    }
    let pointees = livres::pointees(cnx, compte.id, contexte).await?;
    lignes.retain(|l| l.date >= compte.ouvert_le);
    for l in &mut lignes {
        l.pointee = pointees
            .iter()
            .any(|(o, id)| *o == l.origine && *id == l.origine_id);
    }
    lignes.sort_by_key(|l| l.date);
    Ok(lignes)
}

/// Rattache à la trésorerie les salaires et les avances payés : chaque paiement devient une
/// dépense « Salaires » payée, une seule fois ; un paiement annulé annule la sienne. Un paiement
/// dont le mode n'a pas encore de compte attend que l'école l'ajoute.
pub(crate) async fn rattacher_salaires(unite: &mut UniteDeTravail<Effet>) -> Resultat<usize> {
    let contexte = unite.contexte().clone();
    let Some(salaires) = tresorerie::categories(unite.connexion(), &contexte)
        .await?
        .into_iter()
        .find(|c| c.code.as_deref() == Some(CATEGORIE_SALAIRES))
    else {
        return Ok(0);
    };
    let comptes = tresorerie::comptes(unite.connexion(), &contexte).await?;
    let choix = livres::comptes_des_modes(unite.connexion(), &contexte).await?;
    let caisse = comptes
        .iter()
        .find(|c| c.nature == NatureCompte::Caisse)
        .map(|c| c.id);
    let existantes = depenses::de_paiements(unite.connexion(), &contexte).await?;
    let mut faits = 0;
    for p in paie::paiements(unite.connexion(), &contexte).await? {
        let sienne = existantes.iter().find(|d| d.paiement_id == Some(p.id));
        match sienne {
            None if !p.annule => {
                let compte = if p.source == SourcePaiement::Caisse {
                    caisse
                } else {
                    compte_du_mode(p.mode, &comptes, &choix)
                };
                let Some(compte) = compte else { continue };
                let membre = personnel::membre(unite.connexion(), p.membre_id, &contexte).await?;
                let nom = personnes::personne(unite.connexion(), membre.personne_id, &contexte)
                    .await?
                    .nom_complet();
                let d = Depense {
                    id: nouvel_id(),
                    etablissement_id: p.etablissement_id,
                    date: DateCivile::du_jour(p.le).ok_or(calcul())?,
                    categorie_id: salaires.id,
                    libelle: match p.objet {
                        ObjetPaiement::Salaire => format!("Salaire de {nom}"),
                        ObjetPaiement::Avance => format!("Avance à {nom}"),
                    }
                    .chars()
                    .take(120)
                    .collect(),
                    beneficiaire: nom.chars().take(120).collect(),
                    montant: p.montant,
                    statut: StatutDepense::Payee,
                    saisie_par: p.par,
                    decidee_par: None,
                    compte_id: Some(compte),
                    mode: Some(p.mode),
                    reference: p.reference.clone(),
                    session_id: p.session_id,
                    payee_le: Some(p.le),
                    motif: String::new(),
                    facture_id: None,
                    paiement_id: Some(p.id),
                };
                depenses::inserer(unite.connexion(), &d, &contexte).await?;
                let id = d.id.vers_chaine();
                unite.auditer(
                    "depense",
                    &id,
                    Operation::Creation,
                    None,
                    Some(serde_json::to_value(&d)?),
                    "tresorerie.rattacher_salaires",
                );
                unite.emettre(
                    TypeEvenement::DepensePayee,
                    "depense",
                    &id,
                    serde_json::json!({ "montant": d.montant, "compte_id": compte,
                        "mode": d.mode, "paiement_id": p.id }),
                );
                faits += 1;
            }
            Some(d) if p.annule && d.statut == StatutDepense::Payee => {
                let apres = Depense {
                    statut: StatutDepense::Annulee,
                    motif: p.motif_annulation.clone(),
                    ..d.clone()
                };
                depenses::ecrire(unite.connexion(), &apres, &contexte).await?;
                let id = apres.id.vers_chaine();
                unite.auditer(
                    "depense",
                    &id,
                    Operation::Modification,
                    Some(serde_json::json!({ "statut": d.statut })),
                    Some(serde_json::json!({ "statut": apres.statut, "motif": apres.motif })),
                    "tresorerie.rattacher_salaires",
                );
                unite.emettre(
                    TypeEvenement::DepenseAnnulee,
                    "depense",
                    &id,
                    serde_json::json!({ "montant": apres.montant, "motif": apres.motif,
                        "paiement_id": p.id }),
                );
                faits += 1;
            }
            _ => {}
        }
    }
    Ok(faits)
}

/// Service des livres de comptes.
pub struct ServiceLivres<'n> {
    noyau: &'n Noyau,
}

impl<'n> ServiceLivres<'n> {
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

    /// Soldes de chaque compte et compte de chaque mode.
    pub async fn etat(&self, session: &Session) -> Resultat<EtatComptes> {
        let contexte = self.noyau.contexte(session.etablissement_id, Some(session));
        let mut cnx = self.connexion().await?;
        let comptes = tresorerie::comptes(&mut cnx, &contexte).await?;
        let choix = livres::comptes_des_modes(&mut cnx, &contexte).await?;
        let mut liste = Vec::with_capacity(comptes.len());
        for c in &comptes {
            let lignes = lignes_du_compte(&mut cnx, c, &comptes, &choix, &contexte).await?;
            liste.push(SoldeCompte {
                compte_id: c.id,
                soldes: soldes(c.solde_ouverture, &lignes).map_err(|_| calcul())?,
            });
        }
        let rattaches = depenses::de_paiements(&mut cnx, &contexte).await?;
        let salaires_sans_compte = paie::paiements(&mut cnx, &contexte)
            .await?
            .iter()
            .filter(|p| !p.annule && !rattaches.iter().any(|d| d.paiement_id == Some(p.id)))
            .count();
        Ok(EtatComptes {
            soldes: liste,
            modes: MODES_HORS_ESPECES
                .into_iter()
                .map(|mode| ModeCompte {
                    mode,
                    compte_id: compte_du_mode(mode, &comptes, &choix),
                })
                .collect(),
            salaires_sans_compte: u32::try_from(salaires_sans_compte).unwrap_or(u32::MAX),
        })
    }

    /// Livre d'un compte pour un mois.
    pub async fn livre(&self, session: &Session, demande: &DemandeLivre) -> Resultat<LivreCompte> {
        let contexte = self.noyau.contexte(session.etablissement_id, Some(session));
        let mut cnx = self.connexion().await?;
        let comptes = tresorerie::comptes(&mut cnx, &contexte).await?;
        let compte = comptes
            .iter()
            .find(|c| c.id == demande.compte_id)
            .cloned()
            .ok_or(regle("tresorerie.compte_inconnu"))?;
        let choix = livres::comptes_des_modes(&mut cnx, &contexte).await?;
        let lignes = lignes_du_compte(&mut cnx, &compte, &comptes, &choix, &contexte).await?;
        let (Some(debut), Some(fin)) = (demande.mois.premier_jour(), demande.mois.dernier_jour())
        else {
            return Err(regle("tresorerie.date_invalide"));
        };
        let avant: Vec<LigneLivre> = lignes.iter().filter(|l| l.date < debut).cloned().collect();
        let du_mois: Vec<LigneLivre> = lignes
            .iter()
            .filter(|l| l.date >= debut && l.date <= fin)
            .cloned()
            .collect();
        let report = soldes(compte.solde_ouverture, &avant)
            .map_err(|_| calcul())?
            .livre;
        let fin = soldes(report, &du_mois).map_err(|_| calcul())?.livre;
        Ok(LivreCompte {
            soldes: soldes(compte.solde_ouverture, &lignes).map_err(|_| calcul())?,
            compte,
            mois: demande.mois,
            report,
            lignes: du_mois,
            fin,
        })
    }

    /// Transfère de l'argent d'un compte de l'école à un autre ; la caisse touchée est la
    /// session ouverte de qui transfère.
    pub async fn transferer(
        &self,
        session: &Session,
        demande: &DemandeTransfertCompte,
    ) -> Resultat<EtatComptes> {
        let contexte = self.noyau.contexte(session.etablissement_id, Some(session));
        let date = if demande.date.trim().is_empty() {
            DateCivile::du_jour(contexte.maintenant).ok_or(regle("tresorerie.date_invalide"))?
        } else {
            DateCivile::analyser(&demande.date).ok_or(regle("tresorerie.date_invalide"))?
        };
        let mut unite = self
            .noyau
            .commencer(contexte.clone(), Durabilite::Critique)
            .await?;
        let comptes = tresorerie::comptes(unite.connexion(), &contexte).await?;
        let trouver = |id| {
            comptes
                .iter()
                .find(|c| c.id == id)
                .cloned()
                .ok_or(regle("tresorerie.compte_inconnu"))
        };
        let (source, destination) = (
            trouver(demande.source_id)?,
            trouver(demande.destination_id)?,
        );
        verifier_transfert(
            &source,
            &destination,
            demande.montant,
            &demande.reference,
            &demande.motif,
        )
        .map_err(regle)?;
        let caisse_touchee =
            source.nature == NatureCompte::Caisse || destination.nature == NatureCompte::Caisse;
        let session_id = if caisse_touchee {
            Some(
                caisse::ouverte_de(unite.connexion(), session.utilisateur_id, &contexte)
                    .await?
                    .ok_or(regle("transfert_compte.caisse_fermee"))?
                    .id,
            )
        } else {
            None
        };
        let t = Transfert {
            id: nouvel_id(),
            etablissement_id: session.etablissement_id,
            date,
            source_id: source.id,
            destination_id: destination.id,
            montant: demande.montant,
            reference: demande.reference.trim().to_owned(),
            motif: demande.motif.trim().to_owned(),
            session_id,
            fait_par: session.utilisateur_id,
        };
        livres::inserer_transfert(unite.connexion(), &t, &contexte).await?;
        if let Some(session_id) = session_id {
            let sortie = source.nature == NatureCompte::Caisse;
            let mouvement = MouvementCaisse {
                id: nouvel_id(),
                etablissement_id: t.etablissement_id,
                session_id,
                montant: t.montant,
                mode: ModePaiement::Especes,
                motif: format!(
                    "{} · {} · {}",
                    t.motif,
                    if sortie {
                        &destination.libelle
                    } else {
                        &source.libelle
                    },
                    t.reference
                ),
                versement_id: None,
                le: contexte.maintenant,
                par: session.utilisateur_id,
                nature: if sortie {
                    NatureMouvement::SortieTransfert
                } else {
                    NatureMouvement::EntreeTransfert
                },
                paiement_id: None,
                depense_id: None,
                transfert_id: Some(t.id),
                recette_id: None,
            };
            caisse::inserer_mouvement(unite.connexion(), &mouvement, &contexte).await?;
        }
        let id = t.id.vers_chaine();
        unite.auditer(
            "transfert_tresorerie",
            &id,
            Operation::Creation,
            None,
            Some(serde_json::to_value(&t)?),
            "tresorerie.transferer",
        );
        unite.emettre(
            TypeEvenement::TransfertTresorerieEnregistre,
            "transfert_tresorerie",
            &id,
            serde_json::json!({ "montant": t.montant, "source": source.libelle,
                "destination": destination.libelle }),
        );
        self.noyau.terminer(unite).await?;
        self.etat(session).await
    }

    /// Coche ou décoche une ligne du livre d'un compte contre le relevé.
    pub async fn pointer(
        &self,
        session: &Session,
        demande: &DemandePointage,
    ) -> Resultat<LivreCompte> {
        let contexte = self.noyau.contexte(session.etablissement_id, Some(session));
        let mut unite = self
            .noyau
            .commencer(contexte.clone(), Durabilite::Ordinaire)
            .await?;
        let comptes = tresorerie::comptes(unite.connexion(), &contexte).await?;
        let compte = comptes
            .iter()
            .find(|c| c.id == demande.compte_id)
            .cloned()
            .ok_or(regle("tresorerie.compte_inconnu"))?;
        let choix = livres::comptes_des_modes(unite.connexion(), &contexte).await?;
        let lignes =
            lignes_du_compte(unite.connexion(), &compte, &comptes, &choix, &contexte).await?;
        if !lignes
            .iter()
            .any(|l| l.origine == demande.origine && l.origine_id == demande.origine_id)
        {
            return Err(regle("tresorerie.ligne_inconnue"));
        }
        let id: Id<LigneLivre> = nouvel_id();
        livres::pointer(
            unite.connexion(),
            &id.vers_chaine(),
            compte.id,
            demande.origine,
            &demande.origine_id,
            demande.pointee,
            &contexte,
        )
        .await?;
        let cle = format!("{}:{}", demande.origine.nom(), demande.origine_id);
        unite.auditer(
            "pointage_tresorerie",
            &cle,
            Operation::Modification,
            None,
            Some(serde_json::json!({ "compte_id": compte.id, "pointee": demande.pointee })),
            "tresorerie.pointer",
        );
        unite.emettre(
            TypeEvenement::LigneTresoreriePointee,
            "compte_tresorerie",
            &compte.id.vers_chaine(),
            serde_json::json!({ "origine": demande.origine, "origine_id": demande.origine_id,
                "pointee": demande.pointee }),
        );
        self.noyau.terminer(unite).await?;
        self.livre(
            session,
            &DemandeLivre {
                compte_id: compte.id,
                mois: demande.mois,
            },
        )
        .await
    }

    /// Choisit le compte où arrive l'argent d'un mode ; les salaires qui l'attendaient s'y
    /// rattachent.
    pub async fn choisir_compte_du_mode(
        &self,
        session: &Session,
        demande: &DemandeCompteDuMode,
    ) -> Resultat<EtatComptes> {
        let contexte = self.noyau.contexte(session.etablissement_id, Some(session));
        let mut unite = self
            .noyau
            .commencer(contexte.clone(), Durabilite::Ordinaire)
            .await?;
        let comptes = tresorerie::comptes(unite.connexion(), &contexte).await?;
        if demande.mode == ModePaiement::Especes
            || compte_du_mode(demande.mode, &comptes, &[(demande.mode, demande.compte_id)])
                != Some(demande.compte_id)
        {
            return Err(regle("tresorerie.compte_du_mode_invalide"));
        }
        livres::choisir_compte_du_mode(
            unite.connexion(),
            demande.mode,
            demande.compte_id,
            &contexte,
        )
        .await?;
        let id = demande.compte_id.vers_chaine();
        unite.auditer(
            "compte_du_mode",
            demande.mode.nom(),
            Operation::Modification,
            None,
            Some(serde_json::json!({ "mode": demande.mode, "compte_id": demande.compte_id })),
            "tresorerie.compte_du_mode",
        );
        unite.emettre(
            TypeEvenement::CompteTresorerieModifie,
            "compte_tresorerie",
            &id,
            serde_json::json!({ "mode": demande.mode }),
        );
        rattacher_salaires(&mut unite).await?;
        self.noyau.terminer(unite).await?;
        self.etat(session).await
    }
}

/// Solde d'un compte, au tableau de bord.
#[derive(Debug, Clone, Serialize, specta::Type)]
pub struct SoldeDuCompte {
    /// Compte.
    pub compte: String,
    /// Solde du livre.
    pub solde: Montant,
}

/// Bloc « Trésorerie » du tableau de bord : les soldes, les dépenses du mois, ce qui attend.
#[derive(Debug, Clone, Serialize, specta::Type)]
pub struct BlocTresorerie {
    /// Solde de chaque compte ouvert.
    pub soldes: Vec<SoldeDuCompte>,
    /// Dépenses payées dans le mois.
    pub depenses_du_mois: Montant,
    /// Dépenses qui attendent l'accord de la direction.
    pub a_approuver: u32,
    /// Ce que l'école doit à ses fournisseurs.
    pub du_fournisseurs: Montant,
    /// Factures de fournisseurs dont l'échéance est passée.
    pub factures_echues: u32,
    /// Catégories de dépense au-delà de leur budget.
    pub budget_depasse: u32,
}

/// Le bloc « Trésorerie » du tableau de bord.
pub(crate) async fn bloc(noyau: &Noyau, session: &Session) -> Resultat<BlocTresorerie> {
    let etat = ServiceLivres::nouveau(noyau).etat(session).await?;
    let contexte = noyau.contexte(session.etablissement_id, Some(session));
    let mut cnx = noyau
        .base()
        .pool()
        .acquire()
        .await
        .map_err(gesco_db::ErreurBase::from)?;
    let comptes = tresorerie::comptes(&mut cnx, &contexte).await?;
    let aujourd_hui = DateCivile::du_jour(contexte.maintenant).ok_or(calcul())?;
    let mois = Mois::de(aujourd_hui);
    let (Some(debut), Some(fin)) = (mois.premier_jour(), mois.dernier_jour()) else {
        return Err(calcul());
    };
    let depenses_du_mois = depenses::depenses_entre(&mut cnx, debut, fin, &contexte)
        .await?
        .iter()
        .filter(|d| d.statut == StatutDepense::Payee)
        .try_fold(Montant::ZERO, |s, d| s.ajouter(d.montant))
        .map_err(|_| calcul())?;
    let a_approuver = depenses::en_attente(&mut cnx, &contexte)
        .await?
        .iter()
        .filter(|d| d.statut == StatutDepense::AApprouver)
        .count();
    drop(cnx);
    let fournisseurs = crate::fournisseurs::ServiceFournisseurs::nouveau(noyau)
        .lire(session)
        .await?;
    let budget = crate::journal_tresorerie::ServiceJournal::nouveau(noyau)
        .budget(
            session,
            &crate::journal_tresorerie::DemandeBudget { annee_id: None },
        )
        .await
        .map_or(0, |b| b.depassements);
    Ok(BlocTresorerie {
        soldes: comptes
            .iter()
            .filter(|c| c.actif)
            .filter_map(|c| {
                etat.soldes
                    .iter()
                    .find(|s| s.compte_id == c.id)
                    .map(|s| SoldeDuCompte {
                        compte: c.libelle.clone(),
                        solde: s.soldes.livre,
                    })
            })
            .collect(),
        depenses_du_mois,
        a_approuver: u32::try_from(a_approuver).unwrap_or(u32::MAX),
        du_fournisseurs: fournisseurs.total_du,
        factures_echues: u32::try_from(
            fournisseurs.factures.iter().filter(|f| f.en_retard).count(),
        )
        .unwrap_or(u32::MAX),
        budget_depasse: budget,
    })
}
