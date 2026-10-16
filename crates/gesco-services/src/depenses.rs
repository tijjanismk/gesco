//! Dépenses (SPEC_PHASE_8 §3, T-61) : saisie, accord de la direction au-dessus du seuil, paiement
//! d'un compte (en espèces, une sortie de la session de caisse ouverte de qui paie), annulation
//! motivée, justificatif joint. Rien ne bloque une dépense sous le seuil : elle se saisit et se
//! paie d'un geste.

use std::path::Path;

use gesco_core::commun::{DateCivile, Id, Montant};
use gesco_core::core::{MachineAEtats, Operation, Permission, TypeEvenement};
use gesco_core::finance::{ModePaiement, MouvementCaisse, NatureMouvement};
use gesco_core::rh::Mois;
use gesco_core::rh::personnel::{TAILLE_MAX_PIECE, extension_piece};
use gesco_core::tresorerie::depense::{verifier_depense, verifier_paiement_depense};
use gesco_core::tresorerie::fournisseur::FactureFournisseur;
use gesco_core::tresorerie::{
    CategorieTresorerie, CompteTresorerie, Depense, NatureCompte, Sens, StatutDepense,
};
use gesco_db::depots::{caisse, depenses, role, tresorerie, utilisateur};
use gesco_db::unite::Durabilite;
use gesco_db::{Contexte, UniteDeTravail, nouvel_id};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::effets::Effet;
use crate::erreur::{ErreurService, Resultat};
use crate::noyau::Noyau;
use crate::session::Session;

fn regle(cle: &'static str) -> ErreurService {
    ErreurService::Regle { cle }
}

/// Justificatif joint, sans son contenu.
#[derive(Debug, Clone, Serialize, specta::Type)]
pub struct PieceJointe {
    /// Identifiant.
    pub id: String,
    /// Nom du fichier.
    pub nom_fichier: String,
}

/// Dépense affichée.
#[derive(Debug, Clone, Serialize, specta::Type)]
pub struct DepenseAffichee {
    /// Dépense.
    pub depense: Depense,
    /// Libellé de sa catégorie.
    pub categorie: String,
    /// Compte d'où elle est payée.
    pub compte: Option<String>,
    /// Qui l'a saisie.
    pub saisie_par: String,
    /// Justificatifs.
    pub pieces: Vec<PieceJointe>,
}

/// Ce que peut faire le compte connecté.
#[derive(Debug, Clone, Copy, Serialize, specta::Type)]
pub struct DroitsDepenses {
    /// Saisir une dépense.
    pub saisir: bool,
    /// Accorder ou refuser une dépense au-dessus du seuil.
    pub approuver: bool,
    /// Payer et annuler.
    pub payer: bool,
}

/// Les dépenses d'un mois et celles qui attendent.
#[derive(Debug, Clone, Serialize, specta::Type)]
pub struct Depenses {
    /// Mois.
    pub mois: Mois,
    /// Dépenses du mois, la plus récente d'abord.
    pub depenses: Vec<DepenseAffichee>,
    /// Dépenses qui attendent un accord ou leur paiement, tous mois confondus.
    pub en_attente: Vec<DepenseAffichee>,
    /// Total payé dans le mois, annulations exclues.
    pub total_paye: Montant,
    /// Seuil d'accord de l'école.
    pub seuil: Montant,
    /// Catégories de dépense actives.
    pub categories: Vec<CategorieTresorerie>,
    /// Comptes ouverts.
    pub comptes: Vec<CompteTresorerie>,
    /// Qui consulte a une caisse ouverte.
    pub caisse_ouverte: bool,
    /// Droits de qui consulte.
    pub droits: DroitsDepenses,
}

/// Paiement d'une dépense.
#[derive(Debug, Clone, Deserialize, specta::Type)]
pub struct PaiementDepense {
    /// Compte d'où elle sort.
    pub compte_id: Id<CompteTresorerie>,
    /// Mode.
    pub mode: ModePaiement,
    /// Référence, hors espèces.
    pub reference: String,
}

/// Nouvelle dépense, payée aussitôt si elle n'attend pas d'accord et qu'un paiement est donné.
#[derive(Debug, Deserialize, specta::Type)]
pub struct DemandeDepense {
    /// Jour, `AAAA-MM-JJ` ; vide : aujourd'hui.
    pub date: String,
    /// Catégorie.
    pub categorie_id: Id<CategorieTresorerie>,
    /// Libellé.
    pub libelle: String,
    /// Bénéficiaire.
    pub beneficiaire: String,
    /// Montant.
    pub montant: Montant,
    /// Paiement immédiat.
    pub paiement: Option<PaiementDepense>,
    /// Facture de fournisseur qu'elle paie : son montant ne dépasse pas ce qui en reste.
    #[serde(default)]
    pub facture_id: Option<Id<FactureFournisseur>>,
}

/// Accord ou refus de la direction.
#[derive(Debug, Deserialize, specta::Type)]
pub struct DemandeDecisionDepense {
    /// Dépense.
    pub id: Id<Depense>,
    /// Vrai pour accorder.
    pub accorder: bool,
    /// Motif d'un refus.
    pub motif: String,
}

/// Paiement d'une dépense approuvée.
#[derive(Debug, Deserialize, specta::Type)]
pub struct DemandePayerDepense {
    /// Dépense.
    pub id: Id<Depense>,
    /// Paiement.
    pub paiement: PaiementDepense,
}

/// Annulation motivée.
#[derive(Debug, Deserialize, specta::Type)]
pub struct DemandeAnnulationDepense {
    /// Dépense.
    pub id: Id<Depense>,
    /// Motif.
    pub motif: String,
}

/// Justificatif lu sur le disque.
#[derive(Debug, Deserialize, specta::Type)]
pub struct DemandePieceDepense {
    /// Dépense.
    pub id: Id<Depense>,
    /// Chemin du fichier.
    pub chemin: String,
}

/// Justificatif à écrire où l'on veut.
#[derive(Debug, Deserialize, specta::Type)]
pub struct DemandePieceDepenseSous {
    /// Justificatif.
    pub piece_id: String,
    /// Chemin choisi.
    pub chemin: String,
}

/// Mois des dépenses à lire.
#[derive(Debug, Deserialize, specta::Type)]
pub struct DemandeMoisDepenses {
    /// Mois.
    pub mois: Mois,
}

/// Service des dépenses.
pub struct ServiceDepenses<'n> {
    noyau: &'n Noyau,
}

impl<'n> ServiceDepenses<'n> {
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

    async fn droits(&self, session: &Session) -> Resultat<DroitsDepenses> {
        let contexte = self.noyau.contexte(session.etablissement_id, Some(session));
        let mut cnx = self.connexion().await?;
        let p = role::permissions_de(&mut cnx, session.utilisateur_id, &contexte).await?;
        Ok(DroitsDepenses {
            saisir: p.contains(&Permission::EXPENSES_CREATE),
            approuver: p.contains(&Permission::EXPENSES_APPROVE),
            payer: p.contains(&Permission::EXPENSES_PAY),
        })
    }

    /// Droits de qui consulte, qui doit avoir un droit sur les dépenses ou la lecture de la
    /// trésorerie.
    pub(crate) async fn droits_lecture(&self, session: &Session) -> Resultat<DroitsDepenses> {
        let droits = self.droits(session).await?;
        let contexte = self.noyau.contexte(session.etablissement_id, Some(session));
        let mut cnx = self.connexion().await?;
        let lit = droits.saisir
            || droits.approuver
            || droits.payer
            || role::permissions_de(&mut cnx, session.utilisateur_id, &contexte)
                .await?
                .contains(&Permission::TREASURY_READ);
        if !lit {
            return Err(regle("depense.sans_droit"));
        }
        Ok(droits)
    }

    /// Les dépenses d'un mois et celles qui attendent ; il faut un droit sur les dépenses ou la
    /// lecture de la trésorerie.
    pub async fn lire(&self, session: &Session, mois: Mois) -> Resultat<Depenses> {
        let droits = self.droits_lecture(session).await?;
        let contexte = self.noyau.contexte(session.etablissement_id, Some(session));
        let mut cnx = self.connexion().await?;
        let (Some(debut), Some(fin)) = (mois.premier_jour(), mois.dernier_jour()) else {
            return Err(regle("depense.date_invalide"));
        };
        let categories = tresorerie::categories(&mut cnx, &contexte).await?;
        let comptes = tresorerie::comptes(&mut cnx, &contexte).await?;
        let du_mois = depenses::depenses_entre(&mut cnx, debut, fin, &contexte).await?;
        let attente = depenses::en_attente(&mut cnx, &contexte).await?;
        let mut affichees = Vec::with_capacity(du_mois.len());
        for d in du_mois {
            affichees.push(afficher(&mut cnx, d, &categories, &comptes, &contexte).await?);
        }
        let mut en_attente = Vec::with_capacity(attente.len());
        for d in attente {
            en_attente.push(afficher(&mut cnx, d, &categories, &comptes, &contexte).await?);
        }
        let total_paye = affichees
            .iter()
            .filter(|d| d.depense.statut == StatutDepense::Payee)
            .try_fold(Montant::ZERO, |s, d| s.ajouter(d.depense.montant))
            .map_err(|_| regle("depense.calcul_impossible"))?;
        let caisse_ouverte = caisse::ouverte_de(&mut cnx, session.utilisateur_id, &contexte)
            .await?
            .is_some();
        let seuil = crate::tresorerie::reglages(&mut cnx, &contexte)
            .await?
            .seuil_accord;
        Ok(Depenses {
            mois,
            depenses: affichees,
            en_attente,
            total_paye,
            seuil,
            categories: categories
                .into_iter()
                .filter(|c| c.sens == Sens::Depense && c.active)
                .collect(),
            comptes: comptes.into_iter().filter(|c| c.actif).collect(),
            caisse_ouverte,
            droits,
        })
    }

    /// Saisit une dépense : sous le seuil, approuvée, et payée aussitôt si un paiement est donné
    /// et que le compte connecté peut payer ; au-dessus, elle attend l'accord.
    pub async fn saisir(&self, session: &Session, demande: &DemandeDepense) -> Resultat<Depenses> {
        verifier_depense(&demande.libelle, &demande.beneficiaire, demande.montant)
            .map_err(regle)?;
        let droits = self.droits(session).await?;
        let contexte = self.noyau.contexte(session.etablissement_id, Some(session));
        let date = if demande.date.trim().is_empty() {
            DateCivile::du_jour(contexte.maintenant).ok_or(regle("depense.date_invalide"))?
        } else {
            DateCivile::analyser(&demande.date).ok_or(regle("depense.date_invalide"))?
        };
        let mut unite = self
            .noyau
            .commencer(contexte.clone(), Durabilite::Critique)
            .await?;
        let categorie = tresorerie::categories(unite.connexion(), &contexte)
            .await?
            .into_iter()
            .find(|c| c.id == demande.categorie_id && c.sens == Sens::Depense && c.active)
            .ok_or(regle("depense.categorie_invalide"))?;
        if let Some(facture_id) = demande.facture_id {
            let facture =
                crate::fournisseurs::facture(unite.connexion(), facture_id, &contexte).await?;
            if facture.annulee {
                return Err(regle("fournisseur.facture_annulee"));
            }
            let reglements = depenses::de_factures(unite.connexion(), &contexte).await?;
            if demande.montant > crate::fournisseurs::disponible(&facture, &reglements)? {
                return Err(regle("fournisseur.au_dela_du_reste"));
            }
        }
        let seuil = crate::tresorerie::reglages(unite.connexion(), &contexte)
            .await?
            .seuil_accord;
        let depense = Depense {
            id: nouvel_id(),
            etablissement_id: session.etablissement_id,
            date,
            categorie_id: categorie.id,
            libelle: demande.libelle.trim().to_owned(),
            beneficiaire: demande.beneficiaire.trim().to_owned(),
            montant: demande.montant,
            statut: StatutDepense::initial(demande.montant, seuil),
            saisie_par: session.utilisateur_id,
            decidee_par: None,
            compte_id: None,
            mode: None,
            reference: String::new(),
            session_id: None,
            payee_le: None,
            motif: String::new(),
            facture_id: demande.facture_id,
            paiement_id: None,
        };
        depenses::inserer(unite.connexion(), &depense, &contexte).await?;
        let id = depense.id.vers_chaine();
        unite.auditer(
            "depense",
            &id,
            Operation::Creation,
            None,
            Some(serde_json::to_value(&depense)?),
            "depenses.saisir",
        );
        unite.emettre(
            TypeEvenement::DepenseSaisie,
            "depense",
            &id,
            serde_json::json!({ "montant": depense.montant, "statut": depense.statut,
                "categorie": categorie.libelle }),
        );
        if let Some(paiement) = &demande.paiement
            && depense.statut == StatutDepense::Approuvee
        {
            if !droits.payer {
                return Err(regle("depense.sans_droit"));
            }
            payer_dans(&mut unite, session, depense, paiement, &contexte).await?;
        }
        self.noyau.terminer(unite).await?;
        self.lire(session, Mois::de(date)).await
    }

    /// Accorde ou refuse une dépense au-dessus du seuil.
    pub async fn decider(
        &self,
        session: &Session,
        demande: &DemandeDecisionDepense,
    ) -> Resultat<Depenses> {
        let motif = demande.motif.trim();
        if !demande.accorder && motif.is_empty() {
            return Err(regle("depense.motif_obligatoire"));
        }
        let contexte = self.noyau.contexte(session.etablissement_id, Some(session));
        let mut unite = self
            .noyau
            .commencer(contexte.clone(), Durabilite::Ordinaire)
            .await?;
        let avant = depenses::depense(unite.connexion(), demande.id, &contexte).await?;
        let statut = if demande.accorder {
            StatutDepense::Approuvee
        } else {
            StatutDepense::Refusee
        };
        let apres = Depense {
            statut: avant.statut.transiter(statut)?,
            decidee_par: Some(session.utilisateur_id),
            motif: if demande.accorder {
                String::new()
            } else {
                motif.to_owned()
            },
            ..avant.clone()
        };
        depenses::ecrire(unite.connexion(), &apres, &contexte).await?;
        let id = apres.id.vers_chaine();
        unite.auditer(
            "depense",
            &id,
            Operation::Modification,
            Some(serde_json::json!({ "statut": avant.statut })),
            Some(serde_json::json!({ "statut": apres.statut, "motif": apres.motif })),
            "depenses.decider",
        );
        unite.emettre(
            TypeEvenement::DepenseDecidee,
            "depense",
            &id,
            serde_json::json!({ "accordee": demande.accorder, "montant": apres.montant }),
        );
        self.noyau.terminer(unite).await?;
        self.lire(session, Mois::de(apres.date)).await
    }

    /// Paie une dépense approuvée.
    pub async fn payer(
        &self,
        session: &Session,
        demande: &DemandePayerDepense,
    ) -> Resultat<Depenses> {
        let contexte = self.noyau.contexte(session.etablissement_id, Some(session));
        let mut unite = self
            .noyau
            .commencer(contexte.clone(), Durabilite::Critique)
            .await?;
        let depense = depenses::depense(unite.connexion(), demande.id, &contexte).await?;
        let date = depense.date;
        payer_dans(&mut unite, session, depense, &demande.paiement, &contexte).await?;
        self.noyau.terminer(unite).await?;
        self.lire(session, Mois::de(date)).await
    }

    /// Annule une dépense approuvée ou payée, avec un motif ; payée en espèces de la caisse, elle
    /// y rentre par une entrée de régularisation dans la session ouverte de qui annule.
    pub async fn annuler(
        &self,
        session: &Session,
        demande: &DemandeAnnulationDepense,
    ) -> Resultat<Depenses> {
        let motif = demande.motif.trim();
        if motif.is_empty() {
            return Err(regle("depense.motif_obligatoire"));
        }
        let contexte = self.noyau.contexte(session.etablissement_id, Some(session));
        let mut unite = self
            .noyau
            .commencer(contexte.clone(), Durabilite::Critique)
            .await?;
        let avant = depenses::depense(unite.connexion(), demande.id, &contexte).await?;
        if avant.paiement_id.is_some() {
            return Err(regle("depense.nee_d_un_paiement"));
        }
        let apres = Depense {
            statut: avant.statut.transiter(StatutDepense::Annulee)?,
            motif: motif.to_owned(),
            ..avant.clone()
        };
        if avant.statut == StatutDepense::Payee && avant.session_id.is_some() {
            let ouverte = caisse::ouverte_de(unite.connexion(), session.utilisateur_id, &contexte)
                .await?
                .ok_or(regle("depense.caisse_fermee_annulation"))?;
            let entree = MouvementCaisse {
                id: nouvel_id(),
                etablissement_id: session.etablissement_id,
                session_id: ouverte.id,
                montant: avant.montant,
                mode: ModePaiement::Especes,
                motif: format!("Dépense annulée : {motif}"),
                versement_id: None,
                le: contexte.maintenant,
                par: session.utilisateur_id,
                nature: NatureMouvement::EntreeRegularisation,
                paiement_id: None,
                depense_id: Some(avant.id),
                transfert_id: None,
            };
            caisse::inserer_mouvement(unite.connexion(), &entree, &contexte).await?;
        }
        depenses::ecrire(unite.connexion(), &apres, &contexte).await?;
        let id = apres.id.vers_chaine();
        unite.auditer(
            "depense",
            &id,
            Operation::Modification,
            Some(serde_json::json!({ "statut": avant.statut })),
            Some(serde_json::json!({ "statut": apres.statut, "motif": motif })),
            "depenses.annuler",
        );
        unite.emettre(
            TypeEvenement::DepenseAnnulee,
            "depense",
            &id,
            serde_json::json!({ "montant": apres.montant, "motif": motif }),
        );
        self.noyau.terminer(unite).await?;
        self.lire(session, Mois::de(apres.date)).await
    }

    /// Joint un justificatif lu sur le disque : PDF ou image, 2 Mo au plus.
    pub async fn joindre(
        &self,
        session: &Session,
        demande: &DemandePieceDepense,
    ) -> Resultat<Depenses> {
        let chemin = Path::new(&demande.chemin);
        let nom_fichier = chemin
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or_default()
            .to_owned();
        if extension_piece(&nom_fichier).is_none() {
            return Err(regle("depense.piece_format"));
        }
        let taille = tokio::fs::metadata(chemin).await?.len();
        if taille == 0 || usize::try_from(taille).map_or(true, |t| t > TAILLE_MAX_PIECE) {
            return Err(regle("depense.piece_trop_lourde"));
        }
        let contenu = tokio::fs::read(chemin).await?;
        let empreinte: String = Sha256::digest(&contenu)
            .iter()
            .map(|o| format!("{o:02x}"))
            .collect();
        let contexte = self.noyau.contexte(session.etablissement_id, Some(session));
        let mut unite = self
            .noyau
            .commencer(contexte.clone(), Durabilite::Critique)
            .await?;
        let depense = depenses::depense(unite.connexion(), demande.id, &contexte).await?;
        let id: Id<PieceJointe> = nouvel_id();
        let id = id.vers_chaine();
        depenses::inserer_piece(
            unite.connexion(),
            &id,
            depense.id,
            &nom_fichier,
            &contenu,
            &empreinte,
            &contexte,
        )
        .await?;
        unite.auditer(
            "piece_depense",
            &id,
            Operation::Creation,
            None,
            Some(
                serde_json::json!({ "depense_id": depense.id, "nom_fichier": nom_fichier,
                "empreinte_sha256": empreinte }),
            ),
            "depenses.joindre",
        );
        self.noyau.terminer(unite).await?;
        self.lire(session, Mois::de(depense.date)).await
    }

    /// Écrit un justificatif au chemin choisi, pour l'ouvrir ou l'imprimer hors de GESCO.
    pub async fn enregistrer_piece_sous(
        &self,
        session: &Session,
        demande: &DemandePieceDepenseSous,
    ) -> Resultat<()> {
        let contexte = self.noyau.contexte(session.etablissement_id, Some(session));
        let (_, contenu) = {
            let mut cnx = self.connexion().await?;
            depenses::contenu_piece(&mut cnx, &demande.piece_id, &contexte).await?
        };
        tokio::fs::write(&demande.chemin, contenu).await?;
        Ok(())
    }
}

/// Paie une dépense approuvée d'un compte : en espèces de la caisse, une sortie de la session
/// ouverte de qui paie.
async fn payer_dans(
    unite: &mut UniteDeTravail<Effet>,
    session: &Session,
    avant: Depense,
    paiement: &PaiementDepense,
    contexte: &Contexte,
) -> Resultat<()> {
    let compte = tresorerie::comptes(unite.connexion(), contexte)
        .await?
        .into_iter()
        .find(|c| c.id == paiement.compte_id)
        .ok_or(regle("depense.compte_inconnu"))?;
    verifier_paiement_depense(&compte, paiement.mode, &paiement.reference).map_err(regle)?;
    let session_id = if compte.nature == NatureCompte::Caisse {
        let ouverte = caisse::ouverte_de(unite.connexion(), session.utilisateur_id, contexte)
            .await?
            .ok_or(regle("depense.caisse_fermee"))?;
        Some(ouverte.id)
    } else {
        None
    };
    let apres = Depense {
        statut: avant.statut.transiter(StatutDepense::Payee)?,
        compte_id: Some(compte.id),
        mode: Some(paiement.mode),
        reference: paiement.reference.trim().to_owned(),
        session_id,
        payee_le: Some(contexte.maintenant),
        ..avant.clone()
    };
    depenses::ecrire(unite.connexion(), &apres, contexte).await?;
    if let Some(session_id) = session_id {
        let sortie = MouvementCaisse {
            id: nouvel_id(),
            etablissement_id: apres.etablissement_id,
            session_id,
            montant: apres.montant,
            mode: ModePaiement::Especes,
            motif: format!("{} · {}", apres.libelle, apres.beneficiaire),
            versement_id: None,
            le: contexte.maintenant,
            par: session.utilisateur_id,
            nature: NatureMouvement::SortieDepense,
            paiement_id: None,
            depense_id: Some(apres.id),
            transfert_id: None,
        };
        caisse::inserer_mouvement(unite.connexion(), &sortie, contexte).await?;
    }
    let id = apres.id.vers_chaine();
    unite.auditer(
        "depense",
        &id,
        Operation::Modification,
        Some(serde_json::json!({ "statut": avant.statut })),
        Some(
            serde_json::json!({ "statut": apres.statut, "compte": compte.libelle,
            "mode": apres.mode, "reference": apres.reference }),
        ),
        "depenses.payer",
    );
    unite.emettre(
        TypeEvenement::DepensePayee,
        "depense",
        &id,
        serde_json::json!({ "montant": apres.montant, "compte_id": compte.id, "mode": apres.mode }),
    );
    Ok(())
}

async fn afficher(
    cnx: &mut gesco_db::Connexion,
    d: Depense,
    categories: &[CategorieTresorerie],
    comptes: &[CompteTresorerie],
    contexte: &Contexte,
) -> Resultat<DepenseAffichee> {
    let saisie_par = utilisateur::lire(cnx, d.saisie_par, contexte)
        .await
        .map(|u| u.nom_affiche)
        .unwrap_or_default();
    let pieces = depenses::pieces(cnx, d.id, contexte)
        .await?
        .into_iter()
        .map(|p| PieceJointe {
            id: p.id,
            nom_fichier: p.nom_fichier,
        })
        .collect();
    Ok(DepenseAffichee {
        categorie: categories
            .iter()
            .find(|c| c.id == d.categorie_id)
            .map(|c| c.libelle.clone())
            .unwrap_or_default(),
        compte: d
            .compte_id
            .and_then(|id| comptes.iter().find(|c| c.id == id))
            .map(|c| c.libelle.clone()),
        saisie_par,
        pieces,
        depense: d,
    })
}
