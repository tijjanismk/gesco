//! Prise en charge par l'État (SPEC_PRISE_EN_CHARGE_ETAT). Le partage de chaque facture entre
//! l'État et le parent se calcule à la lecture, dans `facturation::etat_facture` ; rien ne
//! réécrit une facture, une ligne, une échéance ni une imputation (R-20). Les gestes de la
//! direction (cocher, modifier, abandonner, décocher) et les versements de l'État viennent avec
//! leurs services ; ce module porte en attendant les aides des essais, qu'aucune commande
//! n'atteint.

use gesco_core::commun::{Id, Montant};
use gesco_core::core::Etablissement;
use gesco_core::finance::prise_en_charge::{
    OriginePartEtat, PAYEUR_PAR_DEFAUT, PriseEnChargeEtat, StatutPriseEnCharge,
    StatutVersementEtat, VersementEtat, VersionCourante,
};
use gesco_core::scolarite::Inscription;
use gesco_db::depots::{inscriptions, prise_en_charge};
use gesco_db::unite::Durabilite;
use gesco_db::{Contexte, UniteDeTravail, nouvel_id};

use crate::effets::Effet;
use crate::erreur::{ErreurService, Resultat};
use crate::inscriptions::jour;
use crate::noyau::Noyau;

fn regle(cle: &'static str) -> ErreurService {
    ErreurService::Regle { cle }
}

/// Écrit une version courante de la prise en charge d'une inscription par le dépôt (la version
/// d'avant passe `remplacee`), tient la case des rapports et recalcule la facture, dans l'unité.
/// `Aucune` n'est pas une version : refusée.
pub(crate) async fn ecrire_version_dans(
    unite: &mut UniteDeTravail<Effet>,
    contexte: &Contexte,
    inscription_id: Id<Inscription>,
    version: VersionCourante,
    motif: &str,
) -> Resultat<PriseEnChargeEtat> {
    let (statut, montant_attendu, abandonne) = match version {
        VersionCourante::EnCours { attendu, abandonne } => {
            (StatutPriseEnCharge::EnCours, attendu, abandonne)
        }
        VersionCourante::Terminee => (StatutPriseEnCharge::Terminee, Montant::ZERO, Montant::ZERO),
        VersionCourante::Aucune => return Err(regle("facture.calcul_impossible")),
    };
    let ecrite = prise_en_charge::inserer_version(
        unite.connexion(),
        &PriseEnChargeEtat {
            id: nouvel_id(),
            etablissement_id: contexte.etablissement_id,
            inscription_id,
            statut,
            montant_attendu,
            abandonne,
            reference: String::new(),
            motif: motif.trim().to_owned(),
            remplace_id: None,
            cree_le: contexte.maintenant,
            cree_par: contexte.utilisateur_id,
            mis_a_jour_le: contexte.maintenant,
            mis_a_jour_par: contexte.utilisateur_id,
        },
        contexte,
    )
    .await?;
    prise_en_charge::ecrire_case(
        unite.connexion(),
        inscription_id,
        statut == StatutPriseEnCharge::EnCours,
        contexte,
    )
    .await?;
    crate::reprises::recalculer_inscription_dans(unite, contexte, inscription_id).await?;
    Ok(ecrite)
}

/// Enregistre un versement de l'État reçu avant GESCO (sans recette), entièrement attribué à
/// une inscription, et recalcule sa facture, dans l'unité.
pub(crate) async fn verser_dans(
    unite: &mut UniteDeTravail<Effet>,
    contexte: &Contexte,
    inscription_id: Id<Inscription>,
    montant: Montant,
) -> Resultat<VersementEtat> {
    let inscription = inscriptions::lire(unite.connexion(), inscription_id, contexte).await?;
    let versement = VersementEtat {
        id: nouvel_id(),
        etablissement_id: contexte.etablissement_id,
        annee_id: inscription.annee_id,
        date: jour(contexte.maintenant)?,
        montant,
        reference: String::new(),
        libelle: "Scolarité des élèves orientés".to_owned(),
        payeur: PAYEUR_PAR_DEFAUT.to_owned(),
        recette_id: None,
        avant_gesco: true,
        statut: StatutVersementEtat::Valide,
        motif_annulation: String::new(),
        cree_le: contexte.maintenant,
        cree_par: contexte.utilisateur_id,
        mis_a_jour_le: contexte.maintenant,
        mis_a_jour_par: contexte.utilisateur_id,
    };
    prise_en_charge::inserer_versement(unite.connexion(), &versement, contexte).await?;
    prise_en_charge::inserer_part(
        unite.connexion(),
        versement.id,
        inscription_id,
        montant,
        OriginePartEtat::Ecran,
        contexte,
    )
    .await?;
    crate::reprises::recalculer_inscription_dans(unite, contexte, inscription_id).await?;
    Ok(versement)
}

/// Aide des essais : écrit une version de la prise en charge dans sa propre transaction.
pub(crate) async fn regler_pour_essai(
    noyau: &Noyau,
    etablissement_id: Id<Etablissement>,
    inscription_id: Id<Inscription>,
    version: VersionCourante,
    motif: &str,
) -> Resultat<()> {
    let contexte = noyau.contexte(etablissement_id, None);
    let mut unite = noyau
        .commencer(contexte.clone(), Durabilite::Critique)
        .await?;
    ecrire_version_dans(&mut unite, &contexte, inscription_id, version, motif).await?;
    noyau.terminer(unite).await?;
    Ok(())
}

/// Aide des essais : un versement de l'État pour une inscription, dans sa propre transaction.
pub(crate) async fn verser_pour_essai(
    noyau: &Noyau,
    etablissement_id: Id<Etablissement>,
    inscription_id: Id<Inscription>,
    montant: Montant,
) -> Resultat<()> {
    let contexte = noyau.contexte(etablissement_id, None);
    let mut unite = noyau
        .commencer(contexte.clone(), Durabilite::Critique)
        .await?;
    verser_dans(&mut unite, &contexte, inscription_id, montant).await?;
    noyau.terminer(unite).await?;
    Ok(())
}
