//! Le versement de l'État (SPEC_PRISE_EN_CHARGE_ETAT §7, §8, §10) : l'argent entre une fois en
//! trésorerie, par une recette créée du même code que `recette_saisir` (au compte de banque sans
//! session pour un virement ou un chèque ; dans la session ouverte de qui l'enregistre pour des
//! espèces), ou n'entre pas du tout s'il a été reçu avant GESCO (déjà dans le solde d'ouverture).
//! La répartition ne fait que dire ce que l'État a payé pour qui : des parts actives, remplacées
//! en bloc quand elle change, annulées avec le versement. Chaque geste recalcule les factures
//! touchées, dans une transaction en durabilité critique, avec audit avant et après et
//! événement. Une année close reçoit encore les versements de l'État : seul le côté de l'État
//! de ses factures change.

use std::collections::{HashMap, HashSet};

use gesco_core::commun::{DateCivile, Id, Montant};
use gesco_core::core::{AnneeScolaire, Operation, Permission, TypeEvenement};
use gesco_core::finance::ModePaiement;
use gesco_core::finance::prise_en_charge::{
    CATEGORIE_VERSEMENT_ETAT, EntreePartage, OriginePartEtat, PAYEUR_PAR_DEFAUT, PartVersementEtat,
    Partage, StatutPriseEnCharge, StatutVersementEtat, VersementEtat, VersionCourante, partager,
};
use gesco_core::scolarite::{Classe, Eleve, Inscription};
use gesco_core::tresorerie::CompteTresorerie;
use gesco_core::tresorerie::livre::compte_du_mode;
use gesco_db::depots::fraicheur::Ligne;
use gesco_db::depots::{inscriptions, livres, prise_en_charge, recettes, tresorerie};
use gesco_db::unite::Durabilite;
use gesco_db::{Connexion, Contexte, UniteDeTravail, nouvel_id};
use serde::{Deserialize, Serialize};

use super::{
    Cible, ServicePriseEnCharge, annee_close, annee_demandee, calcul, lire_cible, nombre,
    noms_des_eleves, regle, somme,
};
use crate::effets::Effet;
use crate::erreur::{ErreurService, Resultat};
use crate::facturation::partage_avec_paye_etat;
use crate::recettes::NouvelleRecette;
use crate::session::Session;

/// Libellé d'un versement de l'État laissé vide.
pub const LIBELLE_VERSEMENT_PAR_DEFAUT: &str = "Scolarité des élèves orientés";

/// Longueur permise de la référence d'un versement (numéro de l'ordre de virement, du chèque).
pub const REFERENCE_VERSEMENT_MAX: usize = 120;

// ---------------------------------------------------------------------------------------------
// Demandes et réponses
// ---------------------------------------------------------------------------------------------

/// Ce que l'État a payé pour un élève, dans une répartition.
#[derive(Debug, Clone, Deserialize, specta::Type)]
pub struct PartSaisie {
    /// Inscription de l'année du versement.
    pub inscription_id: Id<Inscription>,
    /// Montant ; zéro, la ligne est ignorée.
    pub montant: Montant,
}

/// « Nouveau versement de l'État » (§7.2, §7.7).
#[derive(Debug, Clone, Deserialize, specta::Type)]
pub struct DemandeVersementEtat {
    /// Année concernée ; absente, l'année courante (une année précédente pour un arriéré).
    #[serde(default)]
    pub annee_id: Option<Id<AnneeScolaire>>,
    /// Jour de réception, `AAAA-MM-JJ` ; vide, aujourd'hui. Ignoré reçu avant GESCO.
    #[serde(default)]
    pub date: String,
    /// Montant reçu.
    pub montant: Montant,
    /// Mode ; absent, le virement. Ignoré reçu avant GESCO.
    #[serde(default)]
    pub mode: Option<ModePaiement>,
    /// Compte où l'argent entre ; absent, le compte choisi pour le mode, à défaut le premier
    /// compte actif qui l'accepte. Ignoré reçu avant GESCO.
    #[serde(default)]
    pub compte_id: Option<Id<CompteTresorerie>>,
    /// Numéro de l'ordre de virement ou du chèque, exigé hors espèces.
    #[serde(default)]
    pub reference: String,
    /// Libellé ; vide, « Scolarité des élèves orientés ».
    #[serde(default)]
    pub libelle: String,
    /// Payeur ; vide, « État ».
    #[serde(default)]
    pub payeur: String,
    /// Reçu avant GESCO : déjà dans le solde d'ouverture, sans recette.
    #[serde(default)]
    pub avant_gesco: bool,
    /// Répartition ; vide, tout est non réparti.
    #[serde(default)]
    pub parts: Vec<PartSaisie>,
}

/// Modifier ou compléter la répartition d'un versement (§7.3) : les parts données remplacent
/// toutes les parts actives.
#[derive(Debug, Clone, Deserialize, specta::Type)]
pub struct DemandeRepartitionVersementEtat {
    /// Versement.
    pub versement_id: Id<VersementEtat>,
    /// La répartition entière, après le geste.
    pub parts: Vec<PartSaisie>,
    /// Instant de lecture du versement, renvoyé tel quel.
    #[serde(default)]
    pub lue_le: Option<i64>,
}

/// Annuler un versement de l'État (§7.3).
#[derive(Debug, Clone, Deserialize, specta::Type)]
pub struct DemandeAnnulationVersementEtat {
    /// Versement.
    pub versement_id: Id<VersementEtat>,
    /// Motif, obligatoire.
    pub motif: String,
    /// Instant de lecture du versement, renvoyé tel quel.
    #[serde(default)]
    pub lue_le: Option<i64>,
}

/// Année de la liste des versements.
#[derive(Debug, Clone, Default, Deserialize, specta::Type)]
pub struct DemandeVersementsEtat {
    /// Année ; absente, l'année courante.
    #[serde(default)]
    pub annee_id: Option<Id<AnneeScolaire>>,
}

/// Un versement désigné.
#[derive(Debug, Clone, Deserialize, specta::Type)]
pub struct DemandeVersementEtatId {
    /// Versement.
    pub versement_id: Id<VersementEtat>,
}

/// Liste de répartition proposée (§7.2, étape 2 du formulaire).
#[derive(Debug, Clone, Default, Deserialize, specta::Type)]
pub struct DemandeRepartitionProposee {
    /// Année concernée ; absente, celle du versement, à défaut l'année courante.
    #[serde(default)]
    pub annee_id: Option<Id<AnneeScolaire>>,
    /// Versement dont on modifie la répartition ; absent pour un nouveau versement.
    #[serde(default)]
    pub versement_id: Option<Id<VersementEtat>>,
}

/// Pied et avertissements d'une répartition, sans rien écrire.
#[derive(Debug, Clone, Deserialize, specta::Type)]
pub struct DemandeApercuVersementEtat {
    /// Versement dont on modifie la répartition ; absent pour un nouveau versement.
    #[serde(default)]
    pub versement_id: Option<Id<VersementEtat>>,
    /// Année concernée d'un nouveau versement ; absente, l'année courante.
    #[serde(default)]
    pub annee_id: Option<Id<AnneeScolaire>>,
    /// Montant reçu (celui du versement quand il existe).
    #[serde(default)]
    pub montant: Option<Montant>,
    /// Répartition proposée.
    #[serde(default)]
    pub parts: Vec<PartSaisie>,
}

/// Un versement de l'État tel que la liste le montre (§6.2).
#[derive(Debug, Clone, Serialize, specta::Type)]
pub struct VersementEtatAffiche {
    /// Le versement.
    pub versement: VersementEtat,
    /// Date affichée : la réception, ou l'arrêt avant GESCO de l'année pour un versement reçu
    /// avant GESCO.
    pub date_affichee: DateCivile,
    /// Compte de la recette (aucun reçu avant GESCO).
    pub compte_id: Option<Id<CompteTresorerie>>,
    /// Libellé du compte.
    pub compte: String,
    /// Mode de la recette.
    pub mode: Option<ModePaiement>,
    /// Somme des parts actives.
    pub reparti: Montant,
    /// Reçu, pas encore attribué (zéro pour un versement annulé).
    pub non_reparti: Montant,
    /// Élèves de la répartition.
    pub nombre_eleves: u32,
    /// Instant de lecture, renvoyé avec la répartition ou l'annulation.
    pub lue_le: i64,
}

/// Versements de l'État d'une année (§6.2).
#[derive(Debug, Clone, Serialize, specta::Type)]
pub struct ListeVersementsEtat {
    /// Année.
    pub annee_id: Id<AnneeScolaire>,
    /// Libellé de l'année.
    pub annee: String,
    /// Année close : les versements s'y enregistrent et s'y répartissent encore.
    pub annee_close: bool,
    /// Versements, valides et annulés, par date de réception.
    pub versements: Vec<VersementEtatAffiche>,
    /// Versements valides.
    pub nombre: u32,
    /// Total reçu (versements valides).
    pub total_recu: Montant,
    /// Total réparti.
    pub total_reparti: Montant,
    /// Total non réparti.
    pub total_non_reparti: Montant,
}

/// Une part de la répartition d'un versement.
#[derive(Debug, Clone, Serialize, specta::Type)]
pub struct PartAffichee {
    /// Inscription.
    pub inscription_id: Id<Inscription>,
    /// Nom et prénoms.
    pub eleve: String,
    /// Matricule interne.
    pub matricule: String,
    /// Classe.
    pub classe: String,
    /// Montant.
    pub montant: Montant,
    /// Écran ou fichier.
    pub origine: OriginePartEtat,
}

/// Un versement de l'État avec sa répartition.
#[derive(Debug, Clone, Serialize, specta::Type)]
pub struct DetailVersementEtat {
    /// Le versement.
    pub versement: VersementEtatAffiche,
    /// Libellé de l'année concernée.
    pub annee: String,
    /// Année concernée close.
    pub annee_close: bool,
    /// Parts actives, dans l'ordre de leur saisie.
    pub parts: Vec<PartAffichee>,
}

/// Part au-delà de ce que l'État doit pour l'élève (§7.2) : acceptée, la différence est « reçu
/// en plus » ; l'attendu peut être porté à la somme reçue (`prise_en_charge_regler`, geste
/// `modifier`, montant `attendu_porte`).
#[derive(Debug, Clone, Serialize, specta::Type)]
pub struct AvertissementPartEtat {
    /// Inscription.
    pub inscription_id: Id<Inscription>,
    /// Nom et prénoms.
    pub eleve: String,
    /// Reçu de l'État au-delà de ce qu'il doit pour l'élève.
    pub en_plus: Montant,
    /// Montant attendu actuel (aucun pour un élève décoché).
    pub attendu: Option<Montant>,
    /// Attendu qui égale la somme reçue (aucun pour un élève décoché).
    pub attendu_porte: Option<Montant>,
}

/// Réponse d'un enregistrement ou d'une répartition.
#[derive(Debug, Clone, Serialize, specta::Type)]
pub struct ReponseVersementEtat {
    /// Le versement à jour.
    pub detail: DetailVersementEtat,
    /// Élèves qui ont reçu plus que ce que l'État leur doit.
    pub avertissements: Vec<AvertissementPartEtat>,
}

/// Un élève de la liste de répartition.
#[derive(Debug, Clone, Serialize, specta::Type)]
pub struct EleveARepartir {
    /// Inscription.
    pub inscription_id: Id<Inscription>,
    /// Élève.
    pub eleve_id: Id<Eleve>,
    /// Nom et prénoms.
    pub eleve: String,
    /// Matricule interne.
    pub matricule: String,
    /// Classe.
    pub classe_id: Id<Classe>,
    /// Libellé de la classe.
    pub classe: String,
    /// Référence d'orientation.
    pub reference: String,
    /// Case cochée (faux pour un élève décoché qui a une part de ce versement).
    pub pris_en_charge: bool,
    /// Montant attendu de l'État.
    pub attendu: Montant,
    /// Déjà reçu de l'État, hors de ce versement.
    pub deja_recu: Montant,
    /// Reste dû par l'État, hors de ce versement.
    pub reste_du_etat: Montant,
    /// Part de ce versement aujourd'hui.
    pub part: Montant,
    /// Montant prérempli : la part de ce versement, sinon le reste dû par l'État.
    pub propose: Montant,
}

/// Liste de répartition proposée (§7.2) : les élèves pris en charge de l'année, et ceux qui ont
/// une part du versement.
#[derive(Debug, Clone, Serialize, specta::Type)]
pub struct RepartitionProposee {
    /// Année.
    pub annee_id: Id<AnneeScolaire>,
    /// Libellé de l'année.
    pub annee: String,
    /// Année close.
    pub annee_close: bool,
    /// Versement dont on modifie la répartition.
    pub versement_id: Option<Id<VersementEtat>>,
    /// Montant reçu du versement.
    pub montant: Option<Montant>,
    /// Élèves, par classe puis par nom.
    pub eleves: Vec<EleveARepartir>,
    /// Total du reste dû par l'État, hors de ce versement.
    pub total_reste_du_etat: Montant,
}

/// Pied d'une répartition et avertissements.
#[derive(Debug, Clone, Serialize, specta::Type)]
pub struct ApercuVersementEtat {
    /// Montant reçu.
    pub montant: Montant,
    /// Réparti.
    pub reparti: Montant,
    /// Non réparti.
    pub non_reparti: Montant,
    /// Réparti au-delà du reçu.
    pub depassement: Montant,
    /// Élèves de la répartition.
    pub nombre_eleves: u32,
    /// Clé du refus de l'enregistrement, s'il serait refusé.
    pub refus: Option<String>,
    /// Élèves qui recevraient plus que ce que l'État leur doit.
    pub avertissements: Vec<AvertissementPartEtat>,
}

// ---------------------------------------------------------------------------------------------
// Règles et écritures, dans une unité ouverte
// ---------------------------------------------------------------------------------------------

fn montant_recu(m: Montant) -> Resultat<Montant> {
    if m.francs() <= 0 {
        return Err(regle("versement_etat.montant_invalide"));
    }
    Ok(m)
}

fn reference_versement(reference: &str) -> Resultat<String> {
    let reference = reference.trim();
    if reference.chars().count() > REFERENCE_VERSEMENT_MAX {
        return Err(regle("versement_etat.reference_trop_longue"));
    }
    Ok(reference.to_owned())
}

fn ou_par_defaut(texte: &str, defaut: &str) -> String {
    let texte = texte.trim();
    if texte.is_empty() {
        defaut.to_owned()
    } else {
        texte.to_owned()
    }
}

/// Lignes de fraîcheur d'un versement : sa ligne et ses parts.
pub(crate) fn lignes_versement(id: Id<VersementEtat>) -> Vec<Ligne> {
    vec![
        Ligne::par_id("versement_etat", id.vers_chaine()),
        Ligne {
            table: "versement_etat_part",
            cles: vec![("versement_etat_id", id.vers_chaine())],
        },
    ]
}

/// Parts actives regroupées par inscription.
fn par_inscription(
    parts: &[(Id<Inscription>, Montant)],
) -> Resultat<HashMap<Id<Inscription>, Montant>> {
    let mut total: HashMap<Id<Inscription>, Montant> = HashMap::new();
    for (inscription, montant) in parts {
        let m = total.entry(*inscription).or_insert(Montant::ZERO);
        *m = calcul(m.ajouter(*montant))?;
    }
    Ok(total)
}

/// Vérifie une répartition saisie pour un versement de l'année : montants positifs (un zéro est
/// ignoré), un élève une fois, une inscription de l'année concernée qui est ou a été prise en
/// charge (version courante en cours ou terminée). Ne vérifie pas le total (voir
/// `verifier_total`). Rend les parts à écrire, dans l'ordre saisi.
async fn verifier_parts(
    cnx: &mut Connexion,
    annee_id: Id<AnneeScolaire>,
    saisies: &[PartSaisie],
    contexte: &Contexte,
) -> Resultat<Vec<(Id<Inscription>, Montant)>> {
    let mut vues = HashSet::new();
    let mut parts = Vec::new();
    for saisie in saisies {
        if saisie.montant.est_negatif() {
            return Err(regle("versement_etat.part_invalide"));
        }
        if saisie.montant.est_nul() {
            continue;
        }
        if !vues.insert(saisie.inscription_id) {
            return Err(regle("versement_etat.eleve_en_double"));
        }
        let inscription = inscriptions::lire(cnx, saisie.inscription_id, contexte).await?;
        if inscription.annee_id != annee_id {
            return Err(regle("versement_etat.autre_annee"));
        }
        if prise_en_charge::courante(cnx, inscription.id, contexte)
            .await?
            .is_none()
        {
            return Err(regle("versement_etat.eleve_non_pris_en_charge"));
        }
        parts.push((inscription.id, saisie.montant));
    }
    Ok(parts)
}

/// Réparti, et refus s'il dépasse le reçu (§7.2).
fn verifier_total(montant: Montant, parts: &[(Id<Inscription>, Montant)]) -> Resultat<Montant> {
    let reparti = somme(parts.iter().map(|(_, m)| *m))?;
    if reparti > montant {
        return Err(regle("versement_etat.reparti_superieur"));
    }
    Ok(reparti)
}

/// Écrit les parts d'un versement et recalcule la facture de chaque inscription touchée
/// (anciennes et nouvelles parts), dans l'unité.
async fn ecrire_parts_dans(
    unite: &mut UniteDeTravail<Effet>,
    contexte: &Contexte,
    versement_id: Id<VersementEtat>,
    parts: &[(Id<Inscription>, Montant)],
    origine: OriginePartEtat,
    anciennes: &[(Id<Inscription>, Montant)],
) -> Resultat<()> {
    for (inscription_id, montant) in parts {
        prise_en_charge::inserer_part(
            unite.connexion(),
            versement_id,
            *inscription_id,
            *montant,
            origine,
            contexte,
        )
        .await?;
    }
    recalculer_dans(
        unite,
        contexte,
        anciennes.iter().chain(parts).map(|(i, _)| *i),
    )
    .await
}

/// Recalcule la facture de chaque inscription, une fois chacune.
async fn recalculer_dans(
    unite: &mut UniteDeTravail<Effet>,
    contexte: &Contexte,
    inscriptions: impl Iterator<Item = Id<Inscription>>,
) -> Resultat<()> {
    let mut vues = HashSet::new();
    for inscription_id in inscriptions {
        if vues.insert(inscription_id) {
            crate::reprises::recalculer_inscription_dans(unite, contexte, inscription_id).await?;
        }
    }
    Ok(())
}

fn parts_json(parts: &[(Id<Inscription>, Montant)]) -> serde_json::Value {
    serde_json::Value::Array(
        parts
            .iter()
            .map(|(i, m)| serde_json::json!({ "inscription_id": i, "montant": m }))
            .collect(),
    )
}

fn actives(parts: &[PartVersementEtat]) -> Vec<(Id<Inscription>, Montant)> {
    parts
        .iter()
        .map(|p| (p.inscription_id, p.montant))
        .collect()
}

/// Enregistre un versement de l'État dans l'unité (§7.2, §7.6, §7.7) : la recette par le code de
/// `recette_saisir` (sauf reçu avant GESCO), le versement, ses parts, le recalcul de chaque
/// facture touchée, l'audit et `VersementEtatEnregistre`. `session` est celle de qui enregistre ;
/// sans session, seul un versement reçu avant GESCO s'enregistre (aides des essais).
pub(crate) async fn enregistrer_dans(
    unite: &mut UniteDeTravail<Effet>,
    contexte: &Contexte,
    session: Option<&Session>,
    demande: &DemandeVersementEtat,
) -> Resultat<VersementEtat> {
    let montant = montant_recu(demande.montant)?;
    let annee = annee_demandee(unite.connexion(), demande.annee_id, contexte).await?;
    let reference = reference_versement(&demande.reference)?;
    let libelle = ou_par_defaut(&demande.libelle, LIBELLE_VERSEMENT_PAR_DEFAUT);
    let payeur = ou_par_defaut(&demande.payeur, PAYEUR_PAR_DEFAUT);
    let parts = verifier_parts(unite.connexion(), annee.id, &demande.parts, contexte).await?;
    let reparti = verifier_total(montant, &parts)?;
    let (date, recette_id) = if demande.avant_gesco {
        let (arret, _) =
            crate::reprises::arret_de_l_annee(unite.connexion(), &annee, contexte).await?;
        (arret, None)
    } else {
        let session = session.ok_or(regle("versement_etat.session_absente"))?;
        let date = if demande.date.trim().is_empty() {
            crate::inscriptions::jour(contexte.maintenant)?
        } else {
            DateCivile::analyser(demande.date.trim())
                .ok_or(regle("versement_etat.date_invalide"))?
        };
        let categorie = tresorerie::categories(unite.connexion(), contexte)
            .await?
            .into_iter()
            .find(|c| c.code.as_deref() == Some(CATEGORIE_VERSEMENT_ETAT))
            .ok_or(regle("recette.categorie_invalide"))?;
        let mode = demande.mode.unwrap_or(ModePaiement::Virement);
        let compte_id = match demande.compte_id {
            Some(id) => id,
            None => {
                let comptes = tresorerie::comptes(unite.connexion(), contexte).await?;
                let choix = livres::comptes_des_modes(unite.connexion(), contexte).await?;
                compte_du_mode(mode, &comptes, &choix).ok_or(regle("tresorerie.compte_inconnu"))?
            }
        };
        let recette = crate::recettes::inserer_dans(
            unite,
            contexte,
            session,
            &NouvelleRecette {
                date,
                categorie: &categorie,
                libelle: &libelle,
                payeur: &payeur,
                montant,
                compte_id,
                mode,
                reference: &reference,
            },
        )
        .await?;
        (date, Some(recette.id))
    };
    let versement = VersementEtat {
        id: nouvel_id(),
        etablissement_id: contexte.etablissement_id,
        annee_id: annee.id,
        date,
        montant,
        reference,
        libelle,
        payeur,
        recette_id,
        avant_gesco: demande.avant_gesco,
        statut: StatutVersementEtat::Valide,
        motif_annulation: String::new(),
        cree_le: contexte.maintenant,
        cree_par: contexte.utilisateur_id,
        mis_a_jour_le: contexte.maintenant,
        mis_a_jour_par: contexte.utilisateur_id,
    };
    prise_en_charge::inserer_versement(unite.connexion(), &versement, contexte).await?;
    ecrire_parts_dans(
        unite,
        contexte,
        versement.id,
        &parts,
        OriginePartEtat::Ecran,
        &[],
    )
    .await?;
    let id = versement.id.vers_chaine();
    unite.auditer(
        "versement_etat",
        &id,
        Operation::Creation,
        None,
        Some(serde_json::json!({ "versement": versement, "parts": parts_json(&parts) })),
        "versement_etat.enregistrer",
    );
    unite.emettre(
        TypeEvenement::VersementEtatEnregistre,
        "versement_etat",
        &id,
        serde_json::json!({
            "annee_id": versement.annee_id,
            "montant": versement.montant,
            "reparti": reparti,
            "nombre_eleves": parts.len(),
            "avant_gesco": versement.avant_gesco,
            "recette_id": versement.recette_id,
        }),
    );
    Ok(versement)
}

/// Remplace la répartition d'un versement valide (§7.3) : les parts actives passent `annulee`,
/// les nouvelles naissent avec leur origine ; la recette ne bouge pas ; factures touchées
/// recalculées ; audit avant et après ; `VersementEtatReparti`. Rien n'est écrit si la
/// répartition ne change pas. Rend vrai si elle a changé.
pub(crate) async fn repartir_dans(
    unite: &mut UniteDeTravail<Effet>,
    contexte: &Contexte,
    versement: &VersementEtat,
    saisies: &[PartSaisie],
    origine: OriginePartEtat,
    contexte_ecran: &str,
) -> Resultat<bool> {
    if versement.statut == StatutVersementEtat::Annule {
        return Err(regle("versement_etat.annule"));
    }
    let nouvelles =
        verifier_parts(unite.connexion(), versement.annee_id, saisies, contexte).await?;
    let reparti = verifier_total(versement.montant, &nouvelles)?;
    let anciennes = actives(
        &prise_en_charge::parts_du_versement(unite.connexion(), versement.id, contexte).await?,
    );
    if par_inscription(&anciennes)? == par_inscription(&nouvelles)? {
        return Ok(false);
    }
    prise_en_charge::annuler_parts(unite.connexion(), versement.id, contexte).await?;
    ecrire_parts_dans(
        unite,
        contexte,
        versement.id,
        &nouvelles,
        origine,
        &anciennes,
    )
    .await?;
    let id = versement.id.vers_chaine();
    unite.auditer(
        "versement_etat",
        &id,
        Operation::Modification,
        Some(serde_json::json!({ "parts": parts_json(&anciennes) })),
        Some(serde_json::json!({ "parts": parts_json(&nouvelles) })),
        contexte_ecran,
    );
    unite.emettre(
        TypeEvenement::VersementEtatReparti,
        "versement_etat",
        &id,
        serde_json::json!({
            "annee_id": versement.annee_id,
            "montant": versement.montant,
            "reparti": reparti,
            "nombre_eleves": nouvelles.len(),
            "ancien_nombre_eleves": anciennes.len(),
            "origine": origine.nom(),
        }),
    );
    Ok(true)
}

/// Annule un versement valide dans l'unité, sans toucher à sa recette (que le geste des recettes
/// annule) : le versement passe `annule` avec son motif, ses parts `annulee`, les factures
/// touchées se recalculent ; audit avant et après ; `VersementEtatAnnule`.
async fn annuler_seul_dans(
    unite: &mut UniteDeTravail<Effet>,
    contexte: &Contexte,
    versement: &VersementEtat,
    motif: &str,
    contexte_ecran: &str,
) -> Resultat<()> {
    let anciennes = actives(
        &prise_en_charge::parts_du_versement(unite.connexion(), versement.id, contexte).await?,
    );
    prise_en_charge::annuler_versement(unite.connexion(), versement.id, motif, contexte).await?;
    recalculer_dans(unite, contexte, anciennes.iter().map(|(i, _)| *i)).await?;
    let id = versement.id.vers_chaine();
    unite.auditer(
        "versement_etat",
        &id,
        Operation::Modification,
        Some(serde_json::json!({ "statut": versement.statut.nom(), "parts": parts_json(&anciennes) })),
        Some(serde_json::json!({ "statut": StatutVersementEtat::Annule.nom(), "motif": motif,
            "parts": [] })),
        contexte_ecran,
    );
    unite.emettre(
        TypeEvenement::VersementEtatAnnule,
        "versement_etat",
        &id,
        serde_json::json!({
            "annee_id": versement.annee_id,
            "montant": versement.montant,
            "recette_id": versement.recette_id,
            "avant_gesco": versement.avant_gesco,
            "motif": motif,
        }),
    );
    Ok(())
}

/// Annulation d'une recette depuis la trésorerie (§7.3) : si elle porte un versement de l'État
/// valide, il est annulé de la même façon, dans la même unité (la recette l'est déjà par le
/// geste des recettes).
pub(crate) async fn annuler_versement_de_la_recette_dans(
    unite: &mut UniteDeTravail<Effet>,
    contexte: &Contexte,
    recette_id: Id<gesco_core::tresorerie::recette::Recette>,
    motif: &str,
) -> Resultat<()> {
    let Some(versement) =
        prise_en_charge::versement_de_la_recette(unite.connexion(), recette_id, contexte).await?
    else {
        return Ok(());
    };
    if versement.statut == StatutVersementEtat::Annule {
        return Ok(());
    }
    annuler_seul_dans(
        unite,
        contexte,
        &versement,
        motif.trim(),
        "recettes.annuler",
    )
    .await
}

// ---------------------------------------------------------------------------------------------
// Lectures
// ---------------------------------------------------------------------------------------------

/// Partage d'une inscription si l'État avait payé `paye_etat` pour elle, sans rien écrire.
async fn partage_avec_recu(
    cnx: &mut Connexion,
    cible: &Cible,
    paye_etat: Montant,
    contexte: &Contexte,
) -> Resultat<Partage> {
    match &cible.facture {
        Some(f) => partage_avec_paye_etat(cnx, f.id, paye_etat, contexte).await,
        None => partager(&EntreePartage {
            echeances: Vec::new(),
            version: VersionCourante::depuis(cible.courante.as_ref()),
            paye_etat,
            rembourse: Montant::ZERO,
            reprise: Montant::ZERO,
        })
        .map_err(|_| regle("facture.calcul_impossible")),
    }
}

/// Avertissement d'un élève dont l'État a payé plus que ce qu'il doit, d'après son partage.
fn avertissement(
    cible: &Cible,
    partage: &Partage,
    noms: &HashMap<Id<Eleve>, (String, String)>,
) -> Resultat<Option<AvertissementPartEtat>> {
    if partage.recu_en_plus.est_nul() {
        return Ok(None);
    }
    let attendu = cible.en_cours().map(|v| v.montant_attendu);
    Ok(Some(AvertissementPartEtat {
        inscription_id: cible.inscription.id,
        eleve: noms
            .get(&cible.inscription.eleve_id)
            .map(|(n, _)| n.clone())
            .unwrap_or_default(),
        en_plus: partage.recu_en_plus,
        attendu,
        attendu_porte: attendu
            .map(|a| calcul(a.ajouter(partage.recu_en_plus)))
            .transpose()?,
    }))
}

/// Le versement tel que la liste le montre.
pub(super) async fn affiche(
    cnx: &mut Connexion,
    versement: VersementEtat,
    comptes: &[CompteTresorerie],
    arret: Option<DateCivile>,
    contexte: &Contexte,
) -> Resultat<(VersementEtatAffiche, Vec<PartVersementEtat>)> {
    let parts = prise_en_charge::parts_du_versement(cnx, versement.id, contexte).await?;
    let reparti = somme(parts.iter().map(|p| p.montant))?;
    let non_reparti = if versement.statut == StatutVersementEtat::Valide {
        calcul(versement.montant.soustraire(reparti))?.max(Montant::ZERO)
    } else {
        Montant::ZERO
    };
    let (compte_id, mode) = match versement.recette_id {
        Some(id) => {
            let r = recettes::recette(cnx, id, contexte).await?;
            (Some(r.compte_id), Some(r.mode))
        }
        None => (None, None),
    };
    let compte = compte_id
        .and_then(|id| comptes.iter().find(|c| c.id == id))
        .map(|c| c.libelle.clone())
        .unwrap_or_default();
    let eleves: HashSet<Id<Inscription>> = parts.iter().map(|p| p.inscription_id).collect();
    let lue_le = crate::fraicheur::instant(cnx, &lignes_versement(versement.id)).await?;
    let date_affichee = match (versement.avant_gesco, arret) {
        (true, Some(arret)) => arret,
        _ => versement.date,
    };
    Ok((
        VersementEtatAffiche {
            date_affichee,
            compte_id,
            compte,
            mode,
            reparti,
            non_reparti,
            nombre_eleves: nombre(eleves.len()),
            lue_le,
            versement,
        },
        parts,
    ))
}

/// Un versement et sa répartition.
async fn detail(
    cnx: &mut Connexion,
    id: Id<VersementEtat>,
    contexte: &Contexte,
) -> Resultat<DetailVersementEtat> {
    let versement = prise_en_charge::versement(cnx, id, contexte)
        .await?
        .ok_or(regle("versement_etat.introuvable"))?;
    let annee = gesco_db::depots::annee_scolaire::lire(cnx, versement.annee_id, contexte).await?;
    let arret = if versement.avant_gesco {
        Some(
            crate::reprises::arret_de_l_annee(cnx, &annee, contexte)
                .await?
                .0,
        )
    } else {
        None
    };
    let comptes = tresorerie::comptes(cnx, contexte).await?;
    let (versement, parts) = affiche(cnx, versement, &comptes, arret, contexte).await?;
    let noms = noms_des_eleves(cnx, contexte).await?;
    let mut affichees = Vec::with_capacity(parts.len());
    for p in parts {
        let cible = lire_cible(cnx, p.inscription_id, contexte).await?;
        let (eleve, matricule) = noms
            .get(&cible.inscription.eleve_id)
            .cloned()
            .unwrap_or_default();
        affichees.push(PartAffichee {
            inscription_id: p.inscription_id,
            eleve,
            matricule,
            classe: cible.classe.libelle.clone(),
            montant: p.montant,
            origine: p.origine,
        });
    }
    Ok(DetailVersementEtat {
        versement,
        annee: annee.libelle.clone(),
        annee_close: annee_close(&annee),
        parts: affichees,
    })
}

/// Avertissements des inscriptions d'une répartition écrite, d'après leur partage actuel.
async fn avertissements_ecrits(
    cnx: &mut Connexion,
    inscriptions: &[Id<Inscription>],
    contexte: &Contexte,
) -> Resultat<Vec<AvertissementPartEtat>> {
    let noms = noms_des_eleves(cnx, contexte).await?;
    let mut avertissements = Vec::new();
    for id in inscriptions {
        let cible = lire_cible(cnx, *id, contexte).await?;
        let paye = prise_en_charge::paye_etat(cnx, *id, contexte).await?;
        let partage = partage_avec_recu(cnx, &cible, paye, contexte).await?;
        if let Some(a) = avertissement(&cible, &partage, &noms)? {
            avertissements.push(a);
        }
    }
    Ok(avertissements)
}

/// La liste de répartition proposée, sans vérifier de permission : élèves pris en charge de
/// l'année concernée (celle du versement s'il est donné) et ceux qui ont une part du versement,
/// avec attendu, déjà reçu et reste dû par l'État hors de ce versement. Sert aussi l'import de la
/// répartition (modèle et analyse).
pub(crate) async fn repartition_proposee_dans(
    cnx: &mut Connexion,
    versement_id: Option<Id<VersementEtat>>,
    annee_id: Option<Id<AnneeScolaire>>,
    contexte: &Contexte,
) -> Resultat<RepartitionProposee> {
    let versement = match versement_id {
        Some(id) => Some(
            prise_en_charge::versement(cnx, id, contexte)
                .await?
                .ok_or(regle("versement_etat.introuvable"))?,
        ),
        None => None,
    };
    let annee_id = versement.as_ref().map(|v| v.annee_id).or(annee_id);
    let annee = annee_demandee(cnx, annee_id, contexte).await?;
    let parts = match &versement {
        Some(v) => par_inscription(&actives(
            &prise_en_charge::parts_du_versement(cnx, v.id, contexte).await?,
        ))?,
        None => HashMap::new(),
    };
    let mut selection: Vec<Id<Inscription>> =
        prise_en_charge::courantes_de_l_annee(cnx, annee.id, contexte)
            .await?
            .into_iter()
            .filter(|v| {
                v.statut == StatutPriseEnCharge::EnCours || parts.contains_key(&v.inscription_id)
            })
            .map(|v| v.inscription_id)
            .collect();
    for id in parts.keys() {
        if !selection.contains(id) {
            selection.push(*id);
        }
    }
    let noms = noms_des_eleves(cnx, contexte).await?;
    let mut eleves = Vec::with_capacity(selection.len());
    for id in selection {
        let cible = lire_cible(cnx, id, contexte).await?;
        let part = parts.get(&id).copied().unwrap_or(Montant::ZERO);
        let paye = prise_en_charge::paye_etat(cnx, id, contexte).await?;
        let deja_recu = calcul(paye.soustraire(part))?.max(Montant::ZERO);
        let partage = partage_avec_recu(cnx, &cible, deja_recu, contexte).await?;
        let reste_du_etat = calcul(partage.doit_encore_etat())?;
        let (eleve, matricule) = noms
            .get(&cible.inscription.eleve_id)
            .cloned()
            .unwrap_or_default();
        let en_cours = cible.en_cours();
        eleves.push(EleveARepartir {
            inscription_id: id,
            eleve_id: cible.inscription.eleve_id,
            eleve,
            matricule,
            classe_id: cible.classe.id,
            classe: cible.classe.libelle.clone(),
            reference: cible
                .courante
                .as_ref()
                .map(|v| v.reference.clone())
                .unwrap_or_default(),
            pris_en_charge: en_cours.is_some(),
            attendu: en_cours.map_or(Montant::ZERO, |v| v.montant_attendu),
            deja_recu,
            reste_du_etat,
            part,
            propose: if part.est_nul() { reste_du_etat } else { part },
        });
    }
    eleves.sort_by(|a, b| (&a.classe, &a.eleve).cmp(&(&b.classe, &b.eleve)));
    Ok(RepartitionProposee {
        annee_id: annee.id,
        annee: annee.libelle.clone(),
        annee_close: annee_close(&annee),
        versement_id: versement.as_ref().map(|v| v.id),
        montant: versement.as_ref().map(|v| v.montant),
        total_reste_du_etat: somme(eleves.iter().map(|e| e.reste_du_etat))?,
        eleves,
    })
}

// ---------------------------------------------------------------------------------------------
// Le service
// ---------------------------------------------------------------------------------------------

impl ServicePriseEnCharge<'_> {
    /// Versements de l'État d'une année (§6.2), avec réparti, non réparti et nombre d'élèves.
    /// Permission `treasury:read`.
    pub async fn versements_lister(
        &self,
        session: &Session,
        demande: &DemandeVersementsEtat,
    ) -> Resultat<ListeVersementsEtat> {
        self.exiger(session, Permission::TREASURY_READ).await?;
        let contexte = self.noyau.contexte(session.etablissement_id, Some(session));
        let mut cnx = self.connexion().await?;
        let annee = annee_demandee(&mut cnx, demande.annee_id, &contexte).await?;
        let versements =
            prise_en_charge::versements_de_l_annee(&mut cnx, annee.id, &contexte).await?;
        let arret = if versements.iter().any(|v| v.avant_gesco) {
            Some(
                crate::reprises::arret_de_l_annee(&mut cnx, &annee, &contexte)
                    .await?
                    .0,
            )
        } else {
            None
        };
        let comptes = tresorerie::comptes(&mut cnx, &contexte).await?;
        let mut lignes = Vec::with_capacity(versements.len());
        for v in versements {
            lignes.push(affiche(&mut cnx, v, &comptes, arret, &contexte).await?.0);
        }
        let valides: Vec<&VersementEtatAffiche> = lignes
            .iter()
            .filter(|l| l.versement.statut == StatutVersementEtat::Valide)
            .collect();
        Ok(ListeVersementsEtat {
            annee_id: annee.id,
            annee: annee.libelle.clone(),
            annee_close: annee_close(&annee),
            nombre: nombre(valides.len()),
            total_recu: somme(valides.iter().map(|l| l.versement.montant))?,
            total_reparti: somme(valides.iter().map(|l| l.reparti))?,
            total_non_reparti: somme(valides.iter().map(|l| l.non_reparti))?,
            versements: lignes,
        })
    }

    /// Un versement de l'État et sa répartition. Permission `treasury:read`.
    pub async fn versement_lire(
        &self,
        session: &Session,
        versement_id: Id<VersementEtat>,
    ) -> Resultat<DetailVersementEtat> {
        self.exiger(session, Permission::TREASURY_READ).await?;
        let contexte = self.noyau.contexte(session.etablissement_id, Some(session));
        let mut cnx = self.connexion().await?;
        detail(&mut cnx, versement_id, &contexte).await
    }

    /// La liste de répartition proposée (§7.2) : élèves pris en charge de l'année concernée avec
    /// attendu, déjà reçu et reste dû par l'État (hors du versement qu'on modifie), et ceux qui
    /// ont une part de ce versement. Permission `treasury:read`.
    pub async fn versement_repartition_proposee(
        &self,
        session: &Session,
        demande: &DemandeRepartitionProposee,
    ) -> Resultat<RepartitionProposee> {
        self.exiger(session, Permission::TREASURY_READ).await?;
        let contexte = self.noyau.contexte(session.etablissement_id, Some(session));
        let mut cnx = self.connexion().await?;
        repartition_proposee_dans(&mut cnx, demande.versement_id, demande.annee_id, &contexte).await
    }

    /// Pied d'une répartition (reçu, réparti, non réparti), refus éventuel et avertissements,
    /// sans rien écrire. Permission `treasury:manage`.
    pub async fn versement_apercu(
        &self,
        session: &Session,
        demande: &DemandeApercuVersementEtat,
    ) -> Resultat<ApercuVersementEtat> {
        self.exiger(session, Permission::TREASURY_MANAGE).await?;
        let contexte = self.noyau.contexte(session.etablissement_id, Some(session));
        let mut cnx = self.connexion().await?;
        let versement = match demande.versement_id {
            Some(id) => Some(
                prise_en_charge::versement(&mut cnx, id, &contexte)
                    .await?
                    .ok_or(regle("versement_etat.introuvable"))?,
            ),
            None => None,
        };
        let montant = versement
            .as_ref()
            .map(|v| v.montant)
            .or(demande.montant)
            .unwrap_or(Montant::ZERO);
        let annee_id = versement.as_ref().map(|v| v.annee_id).or(demande.annee_id);
        let annee = annee_demandee(&mut cnx, annee_id, &contexte).await?;
        let reparti = somme(demande.parts.iter().map(|p| p.montant.max(Montant::ZERO)))?;
        let nombre_eleves = demande
            .parts
            .iter()
            .filter(|p| !p.montant.est_nul() && !p.montant.est_negatif())
            .map(|p| p.inscription_id)
            .collect::<HashSet<_>>()
            .len();
        let mut refus = None;
        let mut avertissements = Vec::new();
        let verifiees = match montant_recu(montant) {
            Err(ErreurService::Regle { cle }) => {
                refus = Some(cle.to_owned());
                None
            }
            Err(e) => return Err(e),
            Ok(_) => match verifier_parts(&mut cnx, annee.id, &demande.parts, &contexte).await {
                Ok(parts) => Some(parts),
                Err(ErreurService::Regle { cle }) => {
                    refus = Some(cle.to_owned());
                    None
                }
                Err(e) => return Err(e),
            },
        };
        if let Some(parts) = &verifiees {
            if reparti > montant {
                refus = Some("versement_etat.reparti_superieur".to_owned());
            }
            let anciennes = match &versement {
                Some(v) => par_inscription(&actives(
                    &prise_en_charge::parts_du_versement(&mut cnx, v.id, &contexte).await?,
                ))?,
                None => HashMap::new(),
            };
            let noms = noms_des_eleves(&mut cnx, &contexte).await?;
            for (id, part) in parts {
                let cible = lire_cible(&mut cnx, *id, &contexte).await?;
                let paye = prise_en_charge::paye_etat(&mut cnx, *id, &contexte).await?;
                let ancienne = anciennes.get(id).copied().unwrap_or(Montant::ZERO);
                let recu = calcul(paye.soustraire(ancienne).and_then(|m| m.ajouter(*part)))?
                    .max(Montant::ZERO);
                let partage = partage_avec_recu(&mut cnx, &cible, recu, &contexte).await?;
                if let Some(a) = avertissement(&cible, &partage, &noms)? {
                    avertissements.push(a);
                }
            }
        }
        Ok(ApercuVersementEtat {
            montant,
            reparti,
            non_reparti: calcul(montant.soustraire(reparti))?.max(Montant::ZERO),
            depassement: calcul(reparti.soustraire(montant))?.max(Montant::ZERO),
            nombre_eleves: nombre(nombre_eleves),
            refus,
            avertissements,
        })
    }

    /// « Nouveau versement de l'État » (§7.2, §7.7), en une transaction : la recette (sauf reçu
    /// avant GESCO), le versement, ses parts, le recalcul de chaque facture touchée, l'audit et
    /// l'événement. Permission `treasury:manage`, plus `invoices:discount` reçu avant GESCO.
    /// Permis sur une année close.
    pub async fn versement_enregistrer(
        &self,
        session: &Session,
        demande: &DemandeVersementEtat,
    ) -> Resultat<ReponseVersementEtat> {
        self.exiger(session, Permission::TREASURY_MANAGE).await?;
        if demande.avant_gesco {
            self.exiger(session, Permission::INVOICES_DISCOUNT).await?;
        }
        let contexte = self.noyau.contexte(session.etablissement_id, Some(session));
        let mut unite = self
            .noyau
            .commencer(contexte.clone(), Durabilite::Critique)
            .await?;
        let versement = enregistrer_dans(&mut unite, &contexte, Some(session), demande).await?;
        self.noyau.terminer(unite).await?;
        self.reponse(&contexte, versement.id).await
    }

    /// Modifier ou compléter la répartition d'un versement (§7.3), en une transaction : les
    /// parts données remplacent les parts actives ; la recette ne bouge pas. Permission
    /// `treasury:manage`. Permis sur une année close.
    pub async fn versement_repartir(
        &self,
        session: &Session,
        demande: &DemandeRepartitionVersementEtat,
    ) -> Resultat<ReponseVersementEtat> {
        self.exiger(session, Permission::TREASURY_MANAGE).await?;
        let contexte = self.noyau.contexte(session.etablissement_id, Some(session));
        let mut unite = self
            .noyau
            .commencer(contexte.clone(), Durabilite::Critique)
            .await?;
        let versement =
            prise_en_charge::versement(unite.connexion(), demande.versement_id, &contexte)
                .await?
                .ok_or(regle("versement_etat.introuvable"))?;
        if let Some(lue_le) = demande.lue_le {
            crate::fraicheur::verifier(unite.connexion(), &lignes_versement(versement.id), lue_le)
                .await?;
        }
        let change = repartir_dans(
            &mut unite,
            &contexte,
            &versement,
            &demande.parts,
            OriginePartEtat::Ecran,
            "versement_etat.repartir",
        )
        .await?;
        if change {
            self.noyau.terminer(unite).await?;
        } else {
            unite.annuler().await?;
        }
        self.reponse(&contexte, versement.id).await
    }

    /// Annule un versement de l'État (§7.3), motif obligatoire : le versement `annule`, ses parts
    /// `annulee`, sa recette annulée par le geste des recettes (une recette en espèces ressort
    /// par la session ouverte de qui annule), les factures recalculées. Permission
    /// `treasury:manage`.
    pub async fn versement_annuler(
        &self,
        session: &Session,
        demande: &DemandeAnnulationVersementEtat,
    ) -> Resultat<DetailVersementEtat> {
        self.exiger(session, Permission::TREASURY_MANAGE).await?;
        let motif = demande.motif.trim();
        if motif.is_empty() {
            return Err(regle("versement_etat.motif_obligatoire"));
        }
        let contexte = self.noyau.contexte(session.etablissement_id, Some(session));
        let mut unite = self
            .noyau
            .commencer(contexte.clone(), Durabilite::Critique)
            .await?;
        let versement =
            prise_en_charge::versement(unite.connexion(), demande.versement_id, &contexte)
                .await?
                .ok_or(regle("versement_etat.introuvable"))?;
        if versement.statut == StatutVersementEtat::Annule {
            return Err(regle("versement_etat.annule"));
        }
        if let Some(lue_le) = demande.lue_le {
            crate::fraicheur::verifier(unite.connexion(), &lignes_versement(versement.id), lue_le)
                .await?;
        }
        if let Some(recette_id) = versement.recette_id {
            crate::recettes::annuler_dans(&mut unite, &contexte, session, recette_id, motif)
                .await?;
        }
        annuler_seul_dans(
            &mut unite,
            &contexte,
            &versement,
            motif,
            "versement_etat.annuler",
        )
        .await?;
        self.noyau.terminer(unite).await?;
        let mut cnx = self.connexion().await?;
        detail(&mut cnx, versement.id, &contexte).await
    }

    /// Le versement à jour et les avertissements de sa répartition.
    async fn reponse(
        &self,
        contexte: &Contexte,
        id: Id<VersementEtat>,
    ) -> Resultat<ReponseVersementEtat> {
        let mut cnx = self.connexion().await?;
        let detail = detail(&mut cnx, id, contexte).await?;
        let inscriptions: Vec<Id<Inscription>> =
            detail.parts.iter().map(|p| p.inscription_id).collect();
        let avertissements = avertissements_ecrits(&mut cnx, &inscriptions, contexte).await?;
        Ok(ReponseVersementEtat {
            detail,
            avertissements,
        })
    }
}
