//! Prise en charge par l'État (SPEC_PRISE_EN_CHARGE_ETAT §3, §5.3, §6.1, §8) : la direction coche,
//! modifie, abandonne et décoche la prise en charge d'un élève ou d'une sélection ; chaque geste
//! écrit une nouvelle version (l'ancienne passe `remplacee`), tient la case des rapports et
//! recalcule la facture, dans une seule transaction en durabilité critique, avec audit avant et
//! après et événement. Le partage de chaque facture entre l'État et le parent se calcule à la
//! lecture, dans `facturation::etat_facture` ; rien ne réécrit une facture, une ligne, une
//! échéance ni une imputation (R-20). Les versements de l'État (§7) : `versements`.

use std::collections::{HashMap, HashSet};

use gesco_core::commun::{DateCivile, Id, Montant};
use gesco_core::core::{
    AnneeScolaire, Etablissement, Operation, Permission, StatutAnnee, TypeEvenement,
};
use gesco_core::finance::facture::lignes_applicables;
use gesco_core::finance::prise_en_charge::{
    EntreePartage, MontantEtat, NatureEcheance, Partage, PriseEnChargeEtat, StatutPriseEnCharge,
    VersementEtat, VersionCourante, partager, verifier_reference,
};
use gesco_core::finance::{Facture, StatutFacture, TypeFrais};
use gesco_core::scolarite::{Classe, Cycle, Eleve, Inscription, Niveau, Serie};
use gesco_db::depots::fraicheur::Ligne;
use gesco_db::depots::{
    academique, annee_scolaire, factures, inscriptions, personnes, prise_en_charge, tarifs,
};
use gesco_db::unite::Durabilite;
use gesco_db::{Connexion, Contexte, UniteDeTravail, nouvel_id};
use serde::{Deserialize, Serialize};

use crate::effets::Effet;
use crate::erreur::{ErreurService, Resultat};
use crate::facturation::{EcheanceAffichee, etat_facture, partage_simule, recalculer_dans};
use crate::inscriptions::jour;
use crate::noyau::Noyau;
use crate::session::Session;

mod du_etat;
mod versements;

pub use du_etat::{
    ClasseDuEtat, DemandeDuEtat, DemandeProductionDuEtat, DuEtat, LigneDuEtat, SerieDuEtat,
    TITRE_DU_ETAT, TotauxDuEtat, VersementDuEtat,
};
pub(crate) use du_etat::{a_recevoir_de_l_etat, feuille_du_etat};
pub use versements::{
    ApercuVersementEtat, AvertissementPartEtat, DemandeAnnulationVersementEtat,
    DemandeApercuVersementEtat, DemandeRepartitionProposee, DemandeRepartitionVersementEtat,
    DemandeVersementEtat, DemandeVersementEtatId, DemandeVersementsEtat, DetailVersementEtat,
    EleveARepartir, LIBELLE_VERSEMENT_PAR_DEFAUT, ListeVersementsEtat, PartAffichee, PartSaisie,
    REFERENCE_VERSEMENT_MAX, RepartitionProposee, ReponseVersementEtat, VersementEtatAffiche,
};
pub(crate) use versements::{
    annuler_versement_de_la_recette_dans, lignes_versement, repartir_dans,
    repartition_proposee_dans,
};

fn regle(cle: &'static str) -> ErreurService {
    ErreurService::Regle { cle }
}

fn calcul(r: Result<Montant, gesco_core::commun::ErreurMontant>) -> Resultat<Montant> {
    r.map_err(|_| regle("facture.calcul_impossible"))
}

fn somme(mut montants: impl Iterator<Item = Montant>) -> Resultat<Montant> {
    calcul(montants.try_fold(Montant::ZERO, |s, m| s.ajouter(m)))
}

fn nombre(n: usize) -> u32 {
    u32::try_from(n).unwrap_or(u32::MAX)
}

/// Motif proposé au décochage (§3.7) ; c'est aussi celui de la grille « Pour les rapports », qui
/// ne demande pas de motif.
pub const MOTIF_DECOCHAGE_PROPOSE: &str = "Pas orienté par l'État";

/// Vrai pour un cycle où l'État prend des élèves en charge : le lycée et le technique.
pub(crate) const fn prise_en_charge_possible(cycle: Cycle) -> bool {
    matches!(cycle, Cycle::Lycee | Cycle::Technique)
}

/// Année close ou archivée : cocher et décocher y sont refusés ; abandonner et corriger le
/// montant attendu y restent permis (§3.7).
const fn annee_close(annee: &AnneeScolaire) -> bool {
    matches!(annee.statut, StatutAnnee::Cloturee | StatutAnnee::Archivee)
}

// ---------------------------------------------------------------------------------------------
// Demandes et réponses
// ---------------------------------------------------------------------------------------------

/// Une inscription visée par un geste, avec l'instant de lecture de sa ligne (modification
/// simultanée).
#[derive(Debug, Clone, Deserialize, specta::Type)]
pub struct CiblePriseEnCharge {
    /// Inscription.
    pub inscription_id: Id<Inscription>,
    /// Instant de lecture de la ligne, renvoyé tel quel.
    #[serde(default)]
    pub lue_le: Option<i64>,
}

/// Geste de la case, pour un élève ou une sélection (§3.7).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "snake_case")]
pub enum GesteCase {
    /// Cocher : version en cours, montant attendu donné ou proposé (§3.3).
    Cocher,
    /// Modifier le montant attendu ou la référence d'un élève coché.
    Modifier,
    /// Porter l'attendu de chaque élève coché au montant proposé aujourd'hui (« Appliquer aux
    /// élèves déjà cochés » après un changement du tableau des montants).
    AppliquerMontantPropose,
    /// Décocher, avec un motif.
    Decocher,
}

/// Cocher, modifier ou décocher la prise en charge d'un élève ou d'une sélection.
#[derive(Debug, Clone, Deserialize, specta::Type)]
pub struct DemandePriseEnCharge {
    /// Inscriptions visées.
    pub cibles: Vec<CiblePriseEnCharge>,
    /// Geste.
    pub geste: GesteCase,
    /// Montant attendu de l'État : en cochant, absent, le montant proposé de chaque élève ; en
    /// modifiant, absent, inchangé.
    #[serde(default)]
    pub montant_attendu: Option<Montant>,
    /// Référence d'orientation (60 caractères au plus) ; absente, inchangée.
    #[serde(default)]
    pub reference: Option<String>,
    /// Motif, obligatoire pour décocher.
    #[serde(default)]
    pub motif: String,
}

/// Ne plus attendre ce que l'État ne paiera pas (§3.6), ou défaire l'abandon.
#[derive(Debug, Clone, Deserialize, specta::Type)]
pub struct DemandeAbandonPriseEnCharge {
    /// Inscriptions visées.
    pub cibles: Vec<CiblePriseEnCharge>,
    /// Montant à ne plus attendre, pour chaque élève ; absent, ce que l'État doit encore à
    /// chacun. Jamais plus que ce qu'il doit encore.
    #[serde(default)]
    pub montant: Option<Montant>,
    /// Vrai pour défaire l'abandon : il revient à zéro.
    #[serde(default)]
    pub defaire: bool,
    /// Motif, obligatoire.
    pub motif: String,
}

/// Geste dont l'aperçu est demandé.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "snake_case")]
pub enum GestePriseEnCharge {
    /// Cocher.
    Cocher,
    /// Modifier le montant attendu.
    Modifier,
    /// Appliquer le montant proposé aux élèves cochés.
    AppliquerMontantPropose,
    /// Décocher.
    Decocher,
    /// Ne plus attendre.
    Abandonner,
    /// Défaire l'abandon.
    DefaireAbandon,
}

/// Ce que changerait un geste, sans rien écrire.
#[derive(Debug, Clone, Deserialize, specta::Type)]
pub struct DemandeApercuPriseEnCharge {
    /// Inscriptions visées.
    pub inscriptions: Vec<Id<Inscription>>,
    /// Geste.
    pub geste: GestePriseEnCharge,
    /// Montant attendu (cocher, modifier) ; absent, comme pour le geste.
    #[serde(default)]
    pub montant_attendu: Option<Montant>,
    /// Montant à ne plus attendre (abandonner) ; absent, ce que l'État doit encore.
    #[serde(default)]
    pub montant_abandon: Option<Montant>,
}

/// Ce qu'un geste changerait pour un élève.
#[derive(Debug, Clone, Serialize, specta::Type)]
pub struct ApercuEleve {
    /// Inscription.
    pub inscription_id: Id<Inscription>,
    /// Nom et prénoms.
    pub eleve: String,
    /// Classe.
    pub classe: String,
    /// Clé du refus du geste pour cet élève (classe hors du lycée et du technique, année close,
    /// case non cochée…) ; rien s'il est possible.
    pub refus: Option<String>,
    /// Vrai si le geste change quelque chose pour cet élève.
    pub change: bool,
    /// Pas encore de facture : le partage s'appliquera à sa naissance.
    pub sans_facture: bool,
    /// Montant attendu proposé (§3.3).
    pub montant_propose: Montant,
    /// Montant attendu avant le geste (zéro sans prise en charge en cours).
    pub attendu_avant: Montant,
    /// Montant attendu après.
    pub attendu_apres: Montant,
    /// Abandonné avant.
    pub abandonne_avant: Montant,
    /// Abandonné après.
    pub abandonne_apres: Montant,
    /// Dû par l'État avant (sur la facture et en complément).
    pub du_etat_avant: Montant,
    /// Dû par l'État après.
    pub du_etat_apres: Montant,
    /// Ce que l'État doit encore avant.
    pub doit_encore_etat_avant: Montant,
    /// Ce que l'État doit encore après.
    pub doit_encore_etat_apres: Montant,
    /// Reste du parent avant.
    pub reste_parent_avant: Montant,
    /// Reste du parent après.
    pub reste_parent_apres: Montant,
    /// Payé par le parent sur ce qui ne le regarde plus, après.
    pub trop_paye_apres: Montant,
    /// Trop-payé reporté sur ses autres frais, après.
    pub trop_paye_reporte_apres: Montant,
    /// À rembourser au parent avant.
    pub a_rembourser_avant: Montant,
    /// À rembourser au parent après.
    pub a_rembourser_apres: Montant,
    /// Argent déjà rendu au parent qu'il devra de nouveau, après (« dont déjà remboursé »).
    pub rendu_apres: Montant,
}

/// Aperçu d'un geste pour un élève ou une sélection (§6.1).
#[derive(Debug, Clone, Serialize, specta::Type)]
pub struct ApercuPriseEnCharge {
    /// Élèves, dans l'ordre demandé.
    pub eleves: Vec<ApercuEleve>,
    /// Élèves pour qui le geste change quelque chose.
    pub nombre: u32,
    /// Élèves pour qui le geste est refusé.
    pub refuses: u32,
    /// Ce que les parents ne doivent plus.
    pub parents_ne_doivent_plus: Montant,
    /// Ce que les parents devront en plus (décochage).
    pub parents_devront_en_plus: Montant,
    /// Dont l'argent déjà rendu aux parents qu'ils devront de nouveau.
    pub dont_deja_rembourse: Montant,
    /// Trop-payé des parents, après.
    pub trop_paye: Montant,
    /// Trop-payé reporté sur leurs autres frais, après.
    pub trop_paye_reporte: Montant,
    /// À rembourser aux parents, après.
    pub a_rembourser: Montant,
    /// Ce que l'État doit encore avant le geste.
    pub doit_encore_etat_avant: Montant,
    /// Ce que l'État doit encore après.
    pub doit_encore_etat_apres: Montant,
}

/// Sélection de la liste des élèves (§6.1).
#[derive(Debug, Clone, Default, Deserialize, specta::Type)]
pub struct DemandePrisesEnCharge {
    /// Année ; absente, l'année courante.
    #[serde(default)]
    pub annee_id: Option<Id<AnneeScolaire>>,
    /// Classe ; absente, toutes les classes du lycée et du technique.
    #[serde(default)]
    pub classe_id: Option<Id<Classe>>,
}

/// Une ligne de la liste : une inscription.
#[derive(Debug, Clone, Serialize, specta::Type)]
pub struct LignePriseEnCharge {
    /// Inscription.
    pub inscription_id: Id<Inscription>,
    /// Élève.
    pub eleve_id: Id<Eleve>,
    /// Classe.
    pub classe_id: Id<Classe>,
    /// Libellé de la classe.
    pub classe: String,
    /// Nom et prénoms.
    pub eleve: String,
    /// Matricule interne.
    pub matricule: String,
    /// Classe du lycée ou du technique : la case s'y coche.
    pub case_possible: bool,
    /// Case cochée.
    pub pris_en_charge: bool,
    /// Version courante (en cours ou terminée), s'il y en a une.
    pub statut: Option<StatutPriseEnCharge>,
    /// Référence d'orientation.
    pub reference: String,
    /// Montant attendu de l'État (zéro si la case n'est pas cochée).
    pub montant_attendu: Montant,
    /// Abandonné par l'école.
    pub abandonne: Montant,
    /// Montant que l'école proposerait aujourd'hui (§3.3).
    pub montant_propose: Montant,
    /// Dû par l'État : sur la facture et en complément.
    pub du_etat: Montant,
    /// Reçu de l'État pour l'élève.
    pub recu_etat: Montant,
    /// Reste dû par l'État.
    pub reste_etat: Montant,
    /// Dont échu.
    pub reste_etat_echu: Montant,
    /// Reçu de l'État au-delà de ce qu'il doit pour l'élève.
    pub recu_en_plus: Montant,
    /// Reste du parent.
    pub reste_parent: Montant,
    /// À rembourser au parent.
    pub a_rembourser: Montant,
    /// Pas encore de facture.
    pub sans_facture: bool,
    /// Instant de lecture de la ligne, renvoyé avec le geste.
    pub lue_le: i64,
}

/// Classe proposée au filtre.
#[derive(Debug, Clone, Serialize, specta::Type)]
pub struct ClassePriseEnCharge {
    /// Classe.
    pub id: Id<Classe>,
    /// Libellé.
    pub libelle: String,
}

/// Liste des élèves et totaux (§6.1).
#[derive(Debug, Clone, Serialize, specta::Type)]
pub struct ListePrisesEnCharge {
    /// Année montrée.
    pub annee_id: Id<AnneeScolaire>,
    /// Libellé de l'année.
    pub annee: String,
    /// Année close : cases grisées, montant et abandon modifiables.
    pub annee_close: bool,
    /// Classes du lycée et du technique de l'année.
    pub classes: Vec<ClassePriseEnCharge>,
    /// Lignes, par classe puis par nom.
    pub lignes: Vec<LignePriseEnCharge>,
    /// Élèves pris en charge.
    pub nombre: u32,
    /// Total attendu des élèves cochés.
    pub total_attendu: Montant,
    /// Total reçu de l'État.
    pub total_recu: Montant,
    /// Total dû par l'État.
    pub total_du_etat: Montant,
    /// Dont échu.
    pub total_du_etat_echu: Montant,
    /// Total reçu en plus.
    pub total_recu_en_plus: Montant,
    /// Total à rembourser aux parents.
    pub total_a_rembourser: Montant,
}

/// Inscription visée par une lecture.
#[derive(Debug, Clone, Deserialize, specta::Type)]
pub struct DemandeInscriptionPriseEnCharge {
    /// Inscription.
    pub inscription_id: Id<Inscription>,
}

/// Année du tableau des montants attendus.
#[derive(Debug, Clone, Default, Deserialize, specta::Type)]
pub struct DemandeMontantsEtat {
    /// Année ; absente, l'année courante.
    #[serde(default)]
    pub annee_id: Option<Id<AnneeScolaire>>,
}

/// Une ligne du tableau des montants attendus (§3.2).
#[derive(Debug, Clone, Serialize, specta::Type)]
pub struct LigneMontantEtat {
    /// Niveau.
    pub niveau_id: Id<Niveau>,
    /// Libellé du niveau.
    pub niveau: String,
    /// Série ou filière, s'il y en a.
    pub serie_id: Option<Id<Serie>>,
    /// Libellé de la série.
    pub serie: Option<String>,
    /// Montant réglé ; vide, les frais de l'école.
    pub montant: Option<Montant>,
    /// Scolarité de la grille en vigueur pour ce niveau et cette série.
    pub frais_ecole: Montant,
    /// Élèves cochés de ce niveau et de cette série.
    pub eleves_coches: u32,
    /// Élèves cochés dont l'attendu diffère du montant proposé aujourd'hui.
    pub eleves_a_changer: u32,
}

/// Tableau des montants attendus d'une année.
#[derive(Debug, Clone, Serialize, specta::Type)]
pub struct MontantsEtat {
    /// Année.
    pub annee_id: Id<AnneeScolaire>,
    /// Libellé de l'année.
    pub annee: String,
    /// Lignes : niveaux (et séries) du lycée et du technique de l'année.
    pub lignes: Vec<LigneMontantEtat>,
    /// Élèves cochés dont l'attendu diffère du montant proposé : « Appliquer aux élèves déjà
    /// cochés » (aperçu, puis geste `appliquer_montant_propose`).
    pub inscriptions_a_changer: Vec<Id<Inscription>>,
    /// Instant de lecture du tableau, renvoyé au réglage.
    pub lue_le: i64,
}

/// Un montant saisi dans le tableau.
#[derive(Debug, Clone, Deserialize, specta::Type)]
pub struct MontantEtatSaisi {
    /// Niveau.
    pub niveau_id: Id<Niveau>,
    /// Série, s'il y en a.
    #[serde(default)]
    pub serie_id: Option<Id<Serie>>,
    /// Montant ; vide, les frais de l'école.
    #[serde(default)]
    pub montant: Option<Montant>,
}

/// Réglage du tableau des montants attendus.
#[derive(Debug, Clone, Deserialize, specta::Type)]
pub struct DemandeReglageMontantsEtat {
    /// Année ; absente, l'année courante.
    #[serde(default)]
    pub annee_id: Option<Id<AnneeScolaire>>,
    /// Lignes saisies ; les autres ne changent pas.
    pub lignes: Vec<MontantEtatSaisi>,
    /// Instant de lecture du tableau.
    #[serde(default)]
    pub lue_le: Option<i64>,
}

// ---------------------------------------------------------------------------------------------
// Lecture d'une inscription et calculs
// ---------------------------------------------------------------------------------------------

/// Une inscription telle que les gestes la lisent.
struct Cible {
    inscription: Inscription,
    classe: Classe,
    cycle: Cycle,
    annee: AnneeScolaire,
    facture: Option<Facture>,
    courante: Option<PriseEnChargeEtat>,
}

impl Cible {
    fn en_cours(&self) -> Option<&PriseEnChargeEtat> {
        self.courante
            .as_ref()
            .filter(|v| v.statut == StatutPriseEnCharge::EnCours)
    }

    /// Lignes de fraîcheur : les versions de la prise en charge et la facture.
    fn lignes_fraicheur(&self) -> Vec<Ligne> {
        let mut lignes = vec![Ligne {
            table: "prise_en_charge_etat",
            cles: vec![("inscription_id", self.inscription.id.vers_chaine())],
        }];
        if let Some(f) = &self.facture {
            lignes.push(Ligne::par_id("facture", f.id.vers_chaine()));
        }
        lignes
    }
}

async fn lire_cible(
    cnx: &mut Connexion,
    inscription_id: Id<Inscription>,
    contexte: &Contexte,
) -> Resultat<Cible> {
    let inscription = inscriptions::lire(cnx, inscription_id, contexte).await?;
    let classe = academique::classe(cnx, inscription.classe_id, contexte).await?;
    let cycle = academique::niveau(cnx, classe.niveau_id, contexte)
        .await?
        .cycle;
    let annee = annee_scolaire::lire(cnx, inscription.annee_id, contexte).await?;
    let facture = factures::facture_vivante(cnx, inscription_id, contexte)
        .await?
        .filter(|f| f.statut != StatutFacture::Annulee);
    let courante = prise_en_charge::courante(cnx, inscription_id, contexte).await?;
    Ok(Cible {
        inscription,
        classe,
        cycle,
        annee,
        facture,
        courante,
    })
}

/// Partage de l'inscription avec une version donnée, sans rien écrire. Sans facture, la part de
/// l'État est tout entière un complément hors facture (aucune échéance encore).
async fn partage_avec(
    cnx: &mut Connexion,
    cible: &Cible,
    version: VersionCourante,
    contexte: &Contexte,
) -> Resultat<Partage> {
    match &cible.facture {
        Some(f) => partage_simule(cnx, f.id, version, contexte).await,
        None => {
            let paye_etat = prise_en_charge::paye_etat(cnx, cible.inscription.id, contexte).await?;
            partager(&EntreePartage {
                echeances: Vec::new(),
                version,
                paye_etat,
                rembourse: Montant::ZERO,
                reprise: Montant::ZERO,
            })
            .map_err(|_| regle("facture.calcul_impossible"))
        }
    }
}

/// Partage actuel et reste de l'État échu : les échéances dont la date est passée ou du jour,
/// et le complément avec la dernière tranche de l'État (§9.1).
async fn etat_actuel(
    cnx: &mut Connexion,
    cible: &Cible,
    aujourd_hui: DateCivile,
    contexte: &Contexte,
) -> Resultat<(Partage, Montant)> {
    let Some(facture) = &cible.facture else {
        let partage = partage_avec(
            cnx,
            cible,
            VersionCourante::depuis(cible.courante.as_ref()),
            contexte,
        )
        .await?;
        return Ok((partage, Montant::ZERO));
    };
    let etat = etat_facture(cnx, facture.id, contexte).await?;
    let echu = reste_etat_echu(&etat.echeances, &etat.partage, aujourd_hui)?;
    Ok((etat.partage, echu))
}

/// Ce que doit l'État pour une inscription (§9.1), d'après son partage actuel : dû (`E + K`),
/// reçu (`S`), reste dû (restes de l'État et complément non reçu), dont échu, reçu en plus. Un
/// seul calcul pour la liste des élèves, l'état de ce que doit l'État, la tuile du tableau de
/// bord et le budget.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ChiffresEtat {
    /// Dû par l'État : sur la facture et en complément.
    pub du: Montant,
    /// Reçu de l'État.
    pub recu: Montant,
    /// Reste dû par l'État.
    pub reste: Montant,
    /// Dont échu.
    pub echu: Montant,
    /// Reçu au-delà de ce que l'État doit.
    pub recu_en_plus: Montant,
}

/// Partage actuel d'une inscription et ce que l'État doit pour elle.
async fn chiffres_etat(
    cnx: &mut Connexion,
    cible: &Cible,
    aujourd_hui: DateCivile,
    contexte: &Contexte,
) -> Resultat<(Partage, ChiffresEtat)> {
    let (partage, echu) = etat_actuel(cnx, cible, aujourd_hui, contexte).await?;
    let chiffres = ChiffresEtat {
        du: calcul(partage.du_etat_total())?,
        recu: partage.recu_etat,
        reste: calcul(partage.doit_encore_etat())?,
        echu,
        recu_en_plus: partage.recu_en_plus,
    };
    Ok((partage, chiffres))
}

fn reste_etat_echu(
    echeances: &[EcheanceAffichee],
    partage: &Partage,
    aujourd_hui: DateCivile,
) -> Resultat<Montant> {
    let sur_facture = somme(
        echeances
            .iter()
            .filter(|e| e.echeance.date <= aujourd_hui)
            .map(|e| e.reste_etat),
    )?;
    let derniere = echeances
        .iter()
        .filter(|e| !e.part_etat.est_nul())
        .map(|e| e.echeance.date)
        .max();
    let complement = if derniere.is_some_and(|d| d <= aujourd_hui) {
        calcul(partage.complement.soustraire(partage.paye_complement))?.max(Montant::ZERO)
    } else {
        Montant::ZERO
    };
    calcul(sur_facture.ajouter(complement))
}

/// Scolarité de la grille en vigueur pour un niveau et une série (zéro sans grille).
async fn scolarite_de_la_grille(
    cnx: &mut Connexion,
    annee_id: Id<AnneeScolaire>,
    niveau_id: Id<Niveau>,
    serie_id: Option<Id<Serie>>,
    contexte: &Contexte,
) -> Resultat<Montant> {
    let Some(grille) = tarifs::grille_en_vigueur(cnx, annee_id, contexte).await? else {
        return Ok(Montant::ZERO);
    };
    let contenu = tarifs::contenu(cnx, grille.id, contexte).await?;
    somme(
        lignes_applicables(&contenu.lignes, niveau_id, serie_id)
            .into_iter()
            .filter(|l| l.type_frais == TypeFrais::Scolarite)
            .map(|l| l.montant),
    )
}

/// Montant attendu proposé (§3.3) : le montant réglé pour le niveau et la série de la classe ;
/// sinon le dû net de la scolarité de la facture (après remises et baisses) ; sans facture, la
/// scolarité de la grille en vigueur pour le niveau et la série.
async fn montant_propose(
    cnx: &mut Connexion,
    cible: &Cible,
    contexte: &Contexte,
) -> Resultat<Montant> {
    let classe = &cible.classe;
    if let Some(m) = prise_en_charge::montant_etat(
        cnx,
        cible.inscription.annee_id,
        classe.niveau_id,
        classe.serie_id,
        contexte,
    )
    .await?
    {
        return Ok(m);
    }
    match &cible.facture {
        Some(f) => {
            let etat = etat_facture(cnx, f.id, contexte).await?;
            somme(
                etat.partage
                    .echeances
                    .iter()
                    .filter(|e| e.nature == NatureEcheance::Scolarite)
                    .map(|e| e.net),
            )
        }
        None => {
            scolarite_de_la_grille(
                cnx,
                cible.inscription.annee_id,
                classe.niveau_id,
                classe.serie_id,
                contexte,
            )
            .await
        }
    }
}

/// Montant attendu que le service proposerait aujourd'hui pour une inscription (§3.3), sans rien
/// écrire : la réinscription s'en sert pour ne pas proposer zéro faute de grille.
pub(crate) async fn montant_propose_de(
    cnx: &mut Connexion,
    inscription_id: Id<Inscription>,
    contexte: &Contexte,
) -> Resultat<Montant> {
    let cible = lire_cible(cnx, inscription_id, contexte).await?;
    montant_propose(cnx, &cible, contexte).await
}

/// Propositions de l'aperçu d'un changement de classe (§5.3), sans rien faire : décocher si la
/// nouvelle classe n'est ni du lycée ni du technique ; le montant réglé pour la nouvelle classe
/// s'il diffère de l'attendu de l'élève. Rien pour un élève qui n'est pas coché.
pub(crate) async fn proposition_changement_classe(
    cnx: &mut Connexion,
    inscription: &Inscription,
    nouvelle: &Classe,
    contexte: &Contexte,
) -> Resultat<(bool, Option<Montant>)> {
    let Some(version) = prise_en_charge::courante(cnx, inscription.id, contexte)
        .await?
        .filter(|v| v.statut == StatutPriseEnCharge::EnCours)
    else {
        return Ok((false, None));
    };
    let cycle = academique::niveau(cnx, nouvelle.niveau_id, contexte)
        .await?
        .cycle;
    if !prise_en_charge_possible(cycle) {
        return Ok((true, None));
    }
    let montant = prise_en_charge::montant_etat(
        cnx,
        inscription.annee_id,
        nouvelle.niveau_id,
        nouvelle.serie_id,
        contexte,
    )
    .await?
    .filter(|m| *m != version.montant_attendu);
    Ok((false, montant))
}

// ---------------------------------------------------------------------------------------------
// Les gestes
// ---------------------------------------------------------------------------------------------

// Contextes du journal d'audit des gestes : étiquettes internes, jamais affichées, donc hors du
// catalogue des messages.
const AUDIT_COCHER: &str = "prise_en_charge.cocher";
const AUDIT_MODIFIER: &str = "prise_en_charge.modifier";
const AUDIT_DECOCHER: &str = "prise_en_charge.decocher";
const AUDIT_ABANDONNER: &str = "prise_en_charge.abandonner";

/// Geste d'écriture, avec ce qu'il porte.
#[derive(Debug, Clone)]
pub(crate) enum Geste {
    /// Cocher.
    Cocher {
        attendu: Option<Montant>,
        reference: Option<String>,
    },
    /// Modifier l'attendu ou la référence.
    Modifier {
        attendu: Option<Montant>,
        reference: Option<String>,
    },
    /// Porter l'attendu au montant proposé.
    AppliquerMontantPropose,
    /// Décocher.
    Decocher { motif: String },
    /// Ne plus attendre.
    Abandonner {
        montant: Option<Montant>,
        motif: String,
    },
    /// Défaire l'abandon.
    DefaireAbandon { motif: String },
}

impl Geste {
    const fn nom(&self) -> &'static str {
        match self {
            Self::Cocher { .. } => "cocher",
            Self::Modifier { .. } => "modifier",
            Self::AppliquerMontantPropose => "appliquer_montant_propose",
            Self::Decocher { .. } => "decocher",
            Self::Abandonner { .. } => "abandonner",
            Self::DefaireAbandon { .. } => "defaire_abandon",
        }
    }

    const fn contexte_ecran(&self) -> &'static str {
        match self {
            Self::Cocher { .. } => AUDIT_COCHER,
            Self::Modifier { .. } | Self::AppliquerMontantPropose => AUDIT_MODIFIER,
            Self::Decocher { .. } => AUDIT_DECOCHER,
            Self::Abandonner { .. } | Self::DefaireAbandon { .. } => AUDIT_ABANDONNER,
        }
    }

    const fn evenement(&self) -> TypeEvenement {
        match self {
            Self::Cocher { .. } => TypeEvenement::PriseEnChargeEtatCochee,
            Self::Decocher { .. } => TypeEvenement::PriseEnChargeEtatDecochee,
            Self::Modifier { .. }
            | Self::AppliquerMontantPropose
            | Self::Abandonner { .. }
            | Self::DefaireAbandon { .. } => TypeEvenement::PriseEnChargeEtatModifiee,
        }
    }
}

/// Version que le geste écrirait.
#[derive(Debug, Clone, PartialEq, Eq)]
struct NouvelleVersion {
    statut: StatutPriseEnCharge,
    montant_attendu: Montant,
    abandonne: Montant,
    reference: String,
    motif: String,
}

impl NouvelleVersion {
    const fn version(&self) -> VersionCourante {
        match self.statut {
            StatutPriseEnCharge::EnCours => VersionCourante::EnCours {
                attendu: self.montant_attendu,
                abandonne: self.abandonne,
            },
            StatutPriseEnCharge::Terminee | StatutPriseEnCharge::Remplacee => {
                VersionCourante::Terminee
            }
        }
    }
}

fn montant_saisi(m: Montant) -> Resultat<Montant> {
    if m.est_negatif() {
        return Err(regle("prise_en_charge.montant_invalide"));
    }
    Ok(m)
}

fn motif_exige(motif: &str) -> Resultat<String> {
    let motif = motif.trim();
    if motif.is_empty() {
        return Err(regle("prise_en_charge.motif_obligatoire"));
    }
    Ok(motif.to_owned())
}

/// Nouvelle version d'un élève coché dont l'attendu ou la référence changent : l'abandon reste
/// (borné par le nouvel attendu), avec son motif, que la base exige tant qu'il reste un abandon.
fn modifier_vers(
    v: &PriseEnChargeEtat,
    attendu: Montant,
    reference: String,
) -> Option<NouvelleVersion> {
    if attendu == v.montant_attendu && reference == v.reference {
        return None;
    }
    let abandonne = v.abandonne.min(attendu);
    Some(NouvelleVersion {
        statut: StatutPriseEnCharge::EnCours,
        montant_attendu: attendu,
        abandonne,
        reference,
        motif: if abandonne.est_nul() {
            String::new()
        } else {
            v.motif.clone()
        },
    })
}

/// Version que le geste écrirait pour l'inscription ; `None` si rien ne change ; le refus nommé
/// sinon. Ne lit que la base.
async fn planifier(
    cnx: &mut Connexion,
    cible: &Cible,
    geste: &Geste,
    contexte: &Contexte,
) -> Resultat<Option<NouvelleVersion>> {
    match geste {
        Geste::Cocher { attendu, reference } => {
            if !prise_en_charge_possible(cible.cycle) {
                return Err(regle("prise_en_charge.classe_non_concernee"));
            }
            if annee_close(&cible.annee) {
                return Err(regle("prise_en_charge.annee_close"));
            }
            if cible.en_cours().is_some() {
                return Ok(None);
            }
            let montant_attendu = match attendu {
                Some(m) => montant_saisi(*m)?,
                None => montant_propose(cnx, cible, contexte).await?,
            };
            let reference = match reference {
                Some(r) => verifier_reference(r).map_err(regle)?,
                None => cible
                    .courante
                    .as_ref()
                    .map(|v| v.reference.clone())
                    .unwrap_or_default(),
            };
            Ok(Some(NouvelleVersion {
                statut: StatutPriseEnCharge::EnCours,
                montant_attendu,
                abandonne: Montant::ZERO,
                reference,
                motif: String::new(),
            }))
        }
        Geste::Modifier { attendu, reference } => {
            let v = cible
                .en_cours()
                .ok_or(regle("prise_en_charge.non_cochee"))?;
            let attendu = attendu
                .map(montant_saisi)
                .transpose()?
                .unwrap_or(v.montant_attendu);
            let reference = match reference {
                Some(r) => verifier_reference(r).map_err(regle)?,
                None => v.reference.clone(),
            };
            Ok(modifier_vers(v, attendu, reference))
        }
        Geste::AppliquerMontantPropose => {
            let v = cible
                .en_cours()
                .ok_or(regle("prise_en_charge.non_cochee"))?;
            let attendu = montant_propose(cnx, cible, contexte).await?;
            Ok(modifier_vers(v, attendu, v.reference.clone()))
        }
        Geste::Decocher { motif } => {
            let motif = motif_exige(motif)?;
            if annee_close(&cible.annee) {
                return Err(regle("prise_en_charge.annee_close"));
            }
            let Some(v) = cible.en_cours() else {
                return Ok(None);
            };
            Ok(Some(NouvelleVersion {
                statut: StatutPriseEnCharge::Terminee,
                montant_attendu: v.montant_attendu,
                abandonne: Montant::ZERO,
                reference: v.reference.clone(),
                motif,
            }))
        }
        Geste::Abandonner { montant, motif } => {
            let motif = motif_exige(motif)?;
            let v = cible
                .en_cours()
                .ok_or(regle("prise_en_charge.non_cochee"))?;
            let doit = calcul(
                partage_avec(cnx, cible, VersionCourante::depuis(Some(v)), contexte)
                    .await?
                    .doit_encore_etat(),
            )?;
            let montant = match montant {
                Some(m) => montant_saisi(*m)?,
                None => doit,
            };
            if montant > doit {
                return Err(regle("prise_en_charge.abandon_superieur"));
            }
            if montant.est_nul() {
                return Ok(None);
            }
            Ok(Some(NouvelleVersion {
                statut: StatutPriseEnCharge::EnCours,
                montant_attendu: v.montant_attendu,
                abandonne: calcul(v.abandonne.ajouter(montant))?.min(v.montant_attendu),
                reference: v.reference.clone(),
                motif,
            }))
        }
        Geste::DefaireAbandon { motif } => {
            let motif = motif_exige(motif)?;
            let v = cible
                .en_cours()
                .ok_or(regle("prise_en_charge.non_cochee"))?;
            if v.abandonne.est_nul() {
                return Ok(None);
            }
            Ok(Some(NouvelleVersion {
                statut: StatutPriseEnCharge::EnCours,
                montant_attendu: v.montant_attendu,
                abandonne: Montant::ZERO,
                reference: v.reference.clone(),
                motif,
            }))
        }
    }
}

/// Écrit la nouvelle version (l'ancienne passe `remplacee`), la case des rapports, recalcule la
/// facture, audite avant et après et émet l'événement, dans l'unité.
async fn ecrire_dans(
    unite: &mut UniteDeTravail<Effet>,
    contexte: &Contexte,
    cible: &Cible,
    nouvelle: NouvelleVersion,
    geste: &Geste,
) -> Resultat<PriseEnChargeEtat> {
    let inscription_id = cible.inscription.id;
    let ecrite = prise_en_charge::inserer_version(
        unite.connexion(),
        &PriseEnChargeEtat {
            id: nouvel_id(),
            etablissement_id: contexte.etablissement_id,
            inscription_id,
            statut: nouvelle.statut,
            montant_attendu: nouvelle.montant_attendu,
            abandonne: nouvelle.abandonne,
            reference: nouvelle.reference,
            motif: nouvelle.motif,
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
        ecrite.statut == StatutPriseEnCharge::EnCours,
        contexte,
    )
    .await?;
    if let Some(facture) = &cible.facture {
        recalculer_dans(unite, contexte, facture).await?;
    }
    let avant = cible.courante.as_ref();
    let id = ecrite.id.vers_chaine();
    unite.auditer(
        "prise_en_charge_etat",
        &id,
        if avant.is_some() {
            Operation::Modification
        } else {
            Operation::Creation
        },
        avant.map(serde_json::to_value).transpose()?,
        Some(serde_json::to_value(&ecrite)?),
        geste.contexte_ecran(),
    );
    unite.emettre(
        geste.evenement(),
        "prise_en_charge_etat",
        &id,
        serde_json::json!({
            "inscription_id": inscription_id,
            "geste": geste.nom(),
            "statut": ecrite.statut.nom(),
            "montant_attendu": ecrite.montant_attendu,
            "abandonne": ecrite.abandonne,
            "ancien_montant_attendu": avant.map(|a| a.montant_attendu),
            "ancien_abandonne": avant.map(|a| a.abandonne),
            "reference": ecrite.reference,
            "motif": ecrite.motif,
        }),
    );
    Ok(ecrite)
}

/// Applique un geste à une ou plusieurs inscriptions dans l'unité ouverte : fraîcheur de chaque
/// ligne d'abord, puis les écritures ; un audit par élève, et un pour le lot quand la sélection
/// compte plusieurs élèves. Un refus pour un élève arrête toute la sélection. Rend les
/// inscriptions visées, sans doublon, dans l'ordre demandé.
pub(crate) async fn regler_dans(
    unite: &mut UniteDeTravail<Effet>,
    contexte: &Contexte,
    cibles: &[CiblePriseEnCharge],
    geste: &Geste,
) -> Resultat<Vec<Id<Inscription>>> {
    let mut vues = HashSet::new();
    let mut lues = Vec::new();
    for c in cibles {
        if !vues.insert(c.inscription_id) {
            continue;
        }
        let cible = lire_cible(unite.connexion(), c.inscription_id, contexte).await?;
        if let Some(lue_le) = c.lue_le {
            crate::fraicheur::verifier(unite.connexion(), &cible.lignes_fraicheur(), lue_le)
                .await?;
        }
        lues.push(cible);
    }
    let mut changees = Vec::new();
    for cible in &lues {
        if let Some(nouvelle) = planifier(unite.connexion(), cible, geste, contexte).await? {
            ecrire_dans(unite, contexte, cible, nouvelle, geste).await?;
            changees.push(cible.inscription.id);
        }
    }
    if lues.len() > 1 && !changees.is_empty() {
        let lot: Id<PriseEnChargeEtat> = nouvel_id();
        unite.auditer(
            "prise_en_charge_lot",
            &lot.vers_chaine(),
            Operation::Modification,
            None,
            Some(serde_json::json!({
                "geste": geste.nom(),
                "nombre": changees.len(),
                "inscriptions": changees,
            })),
            geste.contexte_ecran(),
        );
    }
    Ok(lues.into_iter().map(|c| c.inscription.id).collect())
}

/// Écrit une version courante de la prise en charge d'une inscription par le dépôt (la version
/// d'avant passe `remplacee`), tient la case des rapports et recalcule la facture, dans l'unité,
/// sans règle ni audit : réservé aux aides des essais. `Aucune` n'est pas une version : refusée.
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
/// une inscription, par le geste du service (`versements::enregistrer_dans`, sans session ni
/// permission), dans l'unité : réservé aux aides des essais.
pub(crate) async fn verser_dans(
    unite: &mut UniteDeTravail<Effet>,
    contexte: &Contexte,
    inscription_id: Id<Inscription>,
    montant: Montant,
) -> Resultat<VersementEtat> {
    let inscription = inscriptions::lire(unite.connexion(), inscription_id, contexte).await?;
    versements::enregistrer_dans(
        unite,
        contexte,
        None,
        &DemandeVersementEtat {
            annee_id: Some(inscription.annee_id),
            date: String::new(),
            montant,
            mode: None,
            compte_id: None,
            reference: String::new(),
            libelle: String::new(),
            payeur: String::new(),
            avant_gesco: true,
            parts: vec![PartSaisie {
                inscription_id,
                montant,
            }],
        },
    )
    .await
}

// ---------------------------------------------------------------------------------------------
// Le service
// ---------------------------------------------------------------------------------------------

/// Service de la prise en charge par l'État : gestes de la direction, aperçu, liste, historique
/// et tableau des montants attendus. Chaque méthode vérifie sa permission (§8).
pub struct ServicePriseEnCharge<'n> {
    noyau: &'n Noyau,
}

impl<'n> ServicePriseEnCharge<'n> {
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

    /// Refuse la session qui n'a pas la permission.
    async fn exiger(&self, session: &Session, permission: Permission) -> Resultat<()> {
        self.exiger_l_une(session, permission, &[]).await
    }

    /// Refuse la session qui n'a ni la permission ni aucune des autres données.
    async fn exiger_l_une(
        &self,
        session: &Session,
        permission: Permission,
        autres: &[Permission],
    ) -> Resultat<()> {
        let contexte = self.noyau.contexte(session.etablissement_id, Some(session));
        let droits = self
            .noyau
            .droits_de(session.utilisateur_id, &contexte)
            .await?;
        if !droits.permissions.contains(&permission)
            && !autres.iter().any(|p| droits.permissions.contains(p))
        {
            return Err(ErreurService::permission(permission));
        }
        Ok(())
    }

    /// Cocher, modifier ou décocher, pour un élève ou une sélection, dans une transaction (§3.7).
    /// Rend les lignes à jour des élèves visés.
    pub async fn regler(
        &self,
        session: &Session,
        demande: &DemandePriseEnCharge,
    ) -> Resultat<Vec<LignePriseEnCharge>> {
        let geste = match demande.geste {
            GesteCase::Cocher => Geste::Cocher {
                attendu: demande.montant_attendu,
                reference: demande.reference.clone(),
            },
            GesteCase::Modifier => Geste::Modifier {
                attendu: demande.montant_attendu,
                reference: demande.reference.clone(),
            },
            GesteCase::AppliquerMontantPropose => Geste::AppliquerMontantPropose,
            GesteCase::Decocher => Geste::Decocher {
                motif: demande.motif.clone(),
            },
        };
        self.executer(session, &demande.cibles, &geste).await
    }

    /// Ne plus attendre ce que l'État ne paiera pas, ou défaire l'abandon (§3.6), pour un élève
    /// ou une sélection.
    pub async fn abandonner(
        &self,
        session: &Session,
        demande: &DemandeAbandonPriseEnCharge,
    ) -> Resultat<Vec<LignePriseEnCharge>> {
        let geste = if demande.defaire {
            Geste::DefaireAbandon {
                motif: demande.motif.clone(),
            }
        } else {
            Geste::Abandonner {
                montant: demande.montant,
                motif: demande.motif.clone(),
            }
        };
        self.executer(session, &demande.cibles, &geste).await
    }

    async fn executer(
        &self,
        session: &Session,
        cibles: &[CiblePriseEnCharge],
        geste: &Geste,
    ) -> Resultat<Vec<LignePriseEnCharge>> {
        self.exiger(session, Permission::INVOICES_DISCOUNT).await?;
        let contexte = self.noyau.contexte(session.etablissement_id, Some(session));
        let mut unite = self
            .noyau
            .commencer(contexte.clone(), Durabilite::Critique)
            .await?;
        let ids = regler_dans(&mut unite, &contexte, cibles, geste).await?;
        self.noyau.terminer(unite).await?;
        self.lignes_de(session, &ids).await
    }

    /// Ce que changerait un geste pour un élève ou une sélection, sans rien écrire (§6.1).
    pub async fn apercu(
        &self,
        session: &Session,
        demande: &DemandeApercuPriseEnCharge,
    ) -> Resultat<ApercuPriseEnCharge> {
        self.exiger(session, Permission::INVOICES_DISCOUNT).await?;
        let contexte = self.noyau.contexte(session.etablissement_id, Some(session));
        let mut cnx = self.connexion().await?;
        let motif = MOTIF_DECOCHAGE_PROPOSE.to_owned();
        let geste = match demande.geste {
            GestePriseEnCharge::Cocher => Geste::Cocher {
                attendu: demande.montant_attendu,
                reference: None,
            },
            GestePriseEnCharge::Modifier => Geste::Modifier {
                attendu: demande.montant_attendu,
                reference: None,
            },
            GestePriseEnCharge::AppliquerMontantPropose => Geste::AppliquerMontantPropose,
            GestePriseEnCharge::Decocher => Geste::Decocher { motif },
            GestePriseEnCharge::Abandonner => Geste::Abandonner {
                montant: demande.montant_abandon,
                motif,
            },
            GestePriseEnCharge::DefaireAbandon => Geste::DefaireAbandon { motif },
        };
        let noms = noms_des_eleves(&mut cnx, &contexte).await?;
        let mut vues = HashSet::new();
        let mut eleves = Vec::new();
        for id in &demande.inscriptions {
            if !vues.insert(*id) {
                continue;
            }
            let cible = lire_cible(&mut cnx, *id, &contexte).await?;
            eleves.push(apercu_eleve(&mut cnx, &cible, &geste, &noms, &contexte).await?);
        }
        let total = |f: fn(&ApercuEleve) -> Montant| somme(eleves.iter().map(f));
        let ecart = |f: fn(&ApercuEleve) -> Result<Montant, gesco_core::commun::ErreurMontant>| {
            somme(
                eleves
                    .iter()
                    .map(|e| f(e).unwrap_or(Montant::ZERO).max(Montant::ZERO)),
            )
        };
        Ok(ApercuPriseEnCharge {
            nombre: nombre(eleves.iter().filter(|e| e.change).count()),
            refuses: nombre(eleves.iter().filter(|e| e.refus.is_some()).count()),
            parents_ne_doivent_plus: ecart(|e| {
                e.reste_parent_avant.soustraire(e.reste_parent_apres)
            })?,
            parents_devront_en_plus: ecart(|e| {
                e.reste_parent_apres.soustraire(e.reste_parent_avant)
            })?,
            dont_deja_rembourse: total(|e| e.rendu_apres)?,
            trop_paye: total(|e| e.trop_paye_apres)?,
            trop_paye_reporte: total(|e| e.trop_paye_reporte_apres)?,
            a_rembourser: total(|e| e.a_rembourser_apres)?,
            doit_encore_etat_avant: total(|e| e.doit_encore_etat_avant)?,
            doit_encore_etat_apres: total(|e| e.doit_encore_etat_apres)?,
            eleves,
        })
    }

    /// Une ligne par inscription des classes du lycée et du technique de l'année (et de toute
    /// inscription qui a une prise en charge), avec ce que doivent l'État et le parent ; totaux.
    /// La donnée de l'onglet « Élèves » (§6.1). Lue avec `invoices:issue`, ou avec
    /// `invoices:discount` : qui coche lit la liste (le Directeur n'a pas `invoices:issue`).
    pub async fn lister(
        &self,
        session: &Session,
        demande: &DemandePrisesEnCharge,
    ) -> Resultat<ListePrisesEnCharge> {
        self.exiger_l_une(
            session,
            Permission::INVOICES_ISSUE,
            &[Permission::INVOICES_DISCOUNT],
        )
        .await?;
        let contexte = self.noyau.contexte(session.etablissement_id, Some(session));
        let mut cnx = self.connexion().await?;
        let annee = annee_demandee(&mut cnx, demande.annee_id, &contexte).await?;
        let cycles: HashMap<Id<Niveau>, Cycle> = academique::niveaux(&mut cnx, &contexte)
            .await?
            .into_iter()
            .map(|n| (n.id, n.cycle))
            .collect();
        let classes = academique::classes(&mut cnx, annee.id, &contexte).await?;
        let concernees: HashSet<Id<Classe>> = classes
            .iter()
            .filter(|c| {
                cycles
                    .get(&c.niveau_id)
                    .is_some_and(|cy| prise_en_charge_possible(*cy))
            })
            .map(|c| c.id)
            .collect();
        let avec_version: HashSet<Id<Inscription>> =
            prise_en_charge::courantes_de_l_annee(&mut cnx, annee.id, &contexte)
                .await?
                .into_iter()
                .map(|v| v.inscription_id)
                .collect();
        let selection: Vec<Id<Inscription>> =
            inscriptions::de_l_annee(&mut cnx, annee.id, &contexte)
                .await?
                .into_iter()
                .filter(|i| concernees.contains(&i.classe_id) || avec_version.contains(&i.id))
                .filter(|i| demande.classe_id.is_none_or(|c| c == i.classe_id))
                .map(|i| i.id)
                .collect();
        let noms = noms_des_eleves(&mut cnx, &contexte).await?;
        let aujourd_hui = jour(contexte.maintenant)?;
        let mut lignes = Vec::with_capacity(selection.len());
        for id in selection {
            let cible = lire_cible(&mut cnx, id, &contexte).await?;
            lignes.push(ligne(&mut cnx, &cible, &noms, aujourd_hui, &contexte).await?);
        }
        lignes.sort_by(|a, b| (&a.classe, &a.eleve).cmp(&(&b.classe, &b.eleve)));
        let total = |f: fn(&LignePriseEnCharge) -> Montant| somme(lignes.iter().map(f));
        let mut classes: Vec<ClassePriseEnCharge> = classes
            .into_iter()
            .filter(|c| concernees.contains(&c.id))
            .map(|c| ClassePriseEnCharge {
                id: c.id,
                libelle: c.libelle,
            })
            .collect();
        classes.sort_by(|a, b| a.libelle.cmp(&b.libelle));
        Ok(ListePrisesEnCharge {
            annee_id: annee.id,
            annee: annee.libelle.clone(),
            annee_close: annee_close(&annee),
            classes,
            nombre: nombre(lignes.iter().filter(|l| l.pris_en_charge).count()),
            total_attendu: somme(
                lignes
                    .iter()
                    .filter(|l| l.pris_en_charge)
                    .map(|l| l.montant_attendu),
            )?,
            total_recu: total(|l| l.recu_etat)?,
            total_du_etat: total(|l| l.reste_etat)?,
            total_du_etat_echu: total(|l| l.reste_etat_echu)?,
            total_recu_en_plus: total(|l| l.recu_en_plus)?,
            total_a_rembourser: total(|l| l.a_rembourser)?,
            lignes,
        })
    }

    /// Lignes de quelques inscriptions, telles que la liste les montre après un geste.
    async fn lignes_de(
        &self,
        session: &Session,
        ids: &[Id<Inscription>],
    ) -> Resultat<Vec<LignePriseEnCharge>> {
        let contexte = self.noyau.contexte(session.etablissement_id, Some(session));
        let mut cnx = self.connexion().await?;
        let noms = noms_des_eleves(&mut cnx, &contexte).await?;
        let aujourd_hui = jour(contexte.maintenant)?;
        let mut lignes = Vec::with_capacity(ids.len());
        for id in ids {
            let cible = lire_cible(&mut cnx, *id, &contexte).await?;
            lignes.push(ligne(&mut cnx, &cible, &noms, aujourd_hui, &contexte).await?);
        }
        Ok(lignes)
    }

    /// Versions de la prise en charge d'une inscription, la plus récente d'abord.
    pub async fn historique(
        &self,
        session: &Session,
        inscription_id: Id<Inscription>,
    ) -> Resultat<Vec<PriseEnChargeEtat>> {
        self.exiger(session, Permission::STUDENTS_READ).await?;
        let contexte = self.noyau.contexte(session.etablissement_id, Some(session));
        let mut cnx = self.connexion().await?;
        Ok(prise_en_charge::historique(&mut cnx, inscription_id, &contexte).await?)
    }

    /// Tableau des montants attendus de l'État d'une année (§3.2) : niveaux et séries du lycée
    /// et du technique qui ont une classe dans l'année, et toute ligne déjà réglée. Se lit avec la
    /// lecture des élèves ou avec les frais (le Comptable règle les frais sans lire les fiches).
    pub async fn montants_lire(
        &self,
        session: &Session,
        demande: &DemandeMontantsEtat,
    ) -> Resultat<MontantsEtat> {
        self.exiger_l_une(
            session,
            Permission::STUDENTS_READ,
            &[Permission::FEES_MANAGE],
        )
        .await?;
        let contexte = self.noyau.contexte(session.etablissement_id, Some(session));
        let mut cnx = self.connexion().await?;
        let annee = annee_demandee(&mut cnx, demande.annee_id, &contexte).await?;
        montants_de_l_annee(&mut cnx, &annee, &contexte).await
    }

    /// Règle des lignes du tableau des montants attendus (§3.2). Les élèves déjà cochés ne
    /// changent pas : le tableau rendu dit lesquels proposer de changer.
    pub async fn montants_regler(
        &self,
        session: &Session,
        demande: &DemandeReglageMontantsEtat,
    ) -> Resultat<MontantsEtat> {
        self.exiger(session, Permission::FEES_MANAGE).await?;
        let contexte = self.noyau.contexte(session.etablissement_id, Some(session));
        let mut unite = self
            .noyau
            .commencer(contexte.clone(), Durabilite::Critique)
            .await?;
        let annee = annee_demandee(unite.connexion(), demande.annee_id, &contexte).await?;
        if let Some(lue_le) = demande.lue_le {
            crate::fraicheur::verifier(unite.connexion(), &lignes_montants(&annee), lue_le).await?;
        }
        let existants =
            prise_en_charge::montants_etat(unite.connexion(), annee.id, &contexte).await?;
        let mut change = false;
        for saisi in &demande.lignes {
            let niveau = academique::niveau(unite.connexion(), saisi.niveau_id, &contexte).await?;
            if !prise_en_charge_possible(niveau.cycle) {
                return Err(regle("prise_en_charge.niveau_non_concerne"));
            }
            if let Some(serie_id) = saisi.serie_id {
                let serie = academique::serie(unite.connexion(), serie_id, &contexte).await?;
                if serie.niveau_id != niveau.id {
                    return Err(regle("prise_en_charge.niveau_non_concerne"));
                }
            }
            let montant = saisi.montant.map(montant_saisi).transpose()?;
            let avant = existants
                .iter()
                .find(|m| m.niveau_id == saisi.niveau_id && m.serie_id == saisi.serie_id);
            if avant.map(|m| m.montant) == Some(montant) || (avant.is_none() && montant.is_none()) {
                continue;
            }
            let ligne = MontantEtat {
                id: avant.map_or_else(nouvel_id, |m| m.id),
                etablissement_id: contexte.etablissement_id,
                annee_id: annee.id,
                niveau_id: saisi.niveau_id,
                serie_id: saisi.serie_id,
                montant,
            };
            prise_en_charge::regler_montant_etat(unite.connexion(), &ligne, &contexte).await?;
            unite.auditer(
                "montant_etat",
                &ligne.id.vers_chaine(),
                if avant.is_some() {
                    Operation::Modification
                } else {
                    Operation::Creation
                },
                avant.map(serde_json::to_value).transpose()?,
                Some(serde_json::to_value(&ligne)?),
                "prise_en_charge.montants_regler",
            );
            change = true;
        }
        if change {
            self.noyau.terminer(unite).await?;
        } else {
            unite.annuler().await?;
        }
        let mut cnx = self.connexion().await?;
        montants_de_l_annee(&mut cnx, &annee, &contexte).await
    }
}

/// Année demandée, ou l'année courante.
async fn annee_demandee(
    cnx: &mut Connexion,
    annee_id: Option<Id<AnneeScolaire>>,
    contexte: &Contexte,
) -> Resultat<AnneeScolaire> {
    match annee_id {
        Some(id) => Ok(annee_scolaire::lire(cnx, id, contexte).await?),
        None => annee_scolaire::courante(cnx, contexte)
            .await?
            .ok_or(regle("eleve.annee_absente")),
    }
}

/// Nom et matricule de chaque élève.
async fn noms_des_eleves(
    cnx: &mut Connexion,
    contexte: &Contexte,
) -> Resultat<HashMap<Id<Eleve>, (String, String)>> {
    Ok(personnes::eleves(cnx, contexte)
        .await?
        .into_iter()
        .map(|(e, p)| (e.id, (p.nom_complet(), e.matricule_interne)))
        .collect())
}

/// Ligne de la liste d'une inscription.
async fn ligne(
    cnx: &mut Connexion,
    cible: &Cible,
    noms: &HashMap<Id<Eleve>, (String, String)>,
    aujourd_hui: DateCivile,
    contexte: &Contexte,
) -> Resultat<LignePriseEnCharge> {
    let (partage, etat) = chiffres_etat(cnx, cible, aujourd_hui, contexte).await?;
    let montant_propose = montant_propose(cnx, cible, contexte).await?;
    let lue_le = crate::fraicheur::instant(cnx, &cible.lignes_fraicheur()).await?;
    let (eleve, matricule) = noms
        .get(&cible.inscription.eleve_id)
        .cloned()
        .unwrap_or_default();
    let en_cours = cible.en_cours();
    Ok(LignePriseEnCharge {
        inscription_id: cible.inscription.id,
        eleve_id: cible.inscription.eleve_id,
        classe_id: cible.classe.id,
        classe: cible.classe.libelle.clone(),
        eleve,
        matricule,
        case_possible: prise_en_charge_possible(cible.cycle),
        pris_en_charge: en_cours.is_some(),
        statut: cible.courante.as_ref().map(|v| v.statut),
        reference: cible
            .courante
            .as_ref()
            .map(|v| v.reference.clone())
            .unwrap_or_default(),
        montant_attendu: en_cours.map_or(Montant::ZERO, |v| v.montant_attendu),
        abandonne: en_cours.map_or(Montant::ZERO, |v| v.abandonne),
        montant_propose,
        du_etat: etat.du,
        recu_etat: etat.recu,
        reste_etat: etat.reste,
        reste_etat_echu: etat.echu,
        recu_en_plus: etat.recu_en_plus,
        reste_parent: partage.reste_parent,
        a_rembourser: partage.a_rembourser,
        sans_facture: cible.facture.is_none(),
        lue_le,
    })
}

/// Aperçu d'un geste pour une inscription.
async fn apercu_eleve(
    cnx: &mut Connexion,
    cible: &Cible,
    geste: &Geste,
    noms: &HashMap<Id<Eleve>, (String, String)>,
    contexte: &Contexte,
) -> Resultat<ApercuEleve> {
    let version_avant = VersionCourante::depuis(cible.courante.as_ref());
    let avant = partage_avec(cnx, cible, version_avant, contexte).await?;
    let (refus, nouvelle) = match planifier(cnx, cible, geste, contexte).await {
        Ok(n) => (None, n),
        Err(ErreurService::Regle { cle }) => (Some(cle.to_owned()), None),
        Err(e) => return Err(e),
    };
    let apres = match &nouvelle {
        Some(n) => partage_avec(cnx, cible, n.version(), contexte).await?,
        None => avant.clone(),
    };
    let en_cours = cible.en_cours();
    let attendu_avant = en_cours.map_or(Montant::ZERO, |v| v.montant_attendu);
    let abandonne_avant = en_cours.map_or(Montant::ZERO, |v| v.abandonne);
    let (attendu_apres, abandonne_apres) = match &nouvelle {
        Some(n) if n.statut == StatutPriseEnCharge::EnCours => (n.montant_attendu, n.abandonne),
        Some(_) => (Montant::ZERO, Montant::ZERO),
        None => (attendu_avant, abandonne_avant),
    };
    Ok(ApercuEleve {
        inscription_id: cible.inscription.id,
        eleve: noms
            .get(&cible.inscription.eleve_id)
            .map(|(n, _)| n.clone())
            .unwrap_or_default(),
        classe: cible.classe.libelle.clone(),
        refus,
        change: nouvelle.is_some(),
        sans_facture: cible.facture.is_none(),
        montant_propose: montant_propose(cnx, cible, contexte).await?,
        attendu_avant,
        attendu_apres,
        abandonne_avant,
        abandonne_apres,
        du_etat_avant: calcul(avant.du_etat_total())?,
        du_etat_apres: calcul(apres.du_etat_total())?,
        doit_encore_etat_avant: calcul(avant.doit_encore_etat())?,
        doit_encore_etat_apres: calcul(apres.doit_encore_etat())?,
        reste_parent_avant: avant.reste_parent,
        reste_parent_apres: apres.reste_parent,
        trop_paye_apres: apres.trop_paye,
        trop_paye_reporte_apres: apres.trop_paye_reporte,
        a_rembourser_avant: avant.a_rembourser,
        a_rembourser_apres: apres.a_rembourser,
        rendu_apres: apres.rendu,
    })
}

/// Lignes de fraîcheur du tableau des montants d'une année.
fn lignes_montants(annee: &AnneeScolaire) -> Vec<Ligne> {
    vec![Ligne {
        table: "montant_etat",
        cles: vec![("annee_id", annee.id.vers_chaine())],
    }]
}

/// Ligne du tableau des montants : un niveau et, s'il y en a, une série.
type CleMontant = (Id<Niveau>, Option<Id<Serie>>);

/// Tableau des montants d'une année, avec les élèves cochés à changer.
async fn montants_de_l_annee(
    cnx: &mut Connexion,
    annee: &AnneeScolaire,
    contexte: &Contexte,
) -> Resultat<MontantsEtat> {
    let niveaux: HashMap<Id<Niveau>, Niveau> = academique::niveaux(cnx, contexte)
        .await?
        .into_iter()
        .map(|n| (n.id, n))
        .collect();
    let series: HashMap<Id<Serie>, Serie> = academique::series(cnx, contexte)
        .await?
        .into_iter()
        .map(|s| (s.id, s))
        .collect();
    let reglees = prise_en_charge::montants_etat(cnx, annee.id, contexte).await?;
    let mut cles: Vec<CleMontant> = Vec::new();
    for c in academique::classes(cnx, annee.id, contexte).await? {
        let concerne = niveaux
            .get(&c.niveau_id)
            .is_some_and(|n| prise_en_charge_possible(n.cycle));
        if concerne && !cles.contains(&(c.niveau_id, c.serie_id)) {
            cles.push((c.niveau_id, c.serie_id));
        }
    }
    for m in &reglees {
        if !cles.contains(&(m.niveau_id, m.serie_id)) {
            cles.push((m.niveau_id, m.serie_id));
        }
    }
    let ordre = |(n, s): &CleMontant| {
        (
            niveaux.get(n).map_or(i64::MAX, |n| n.ordre),
            s.and_then(|s| series.get(&s)).map(|s| s.libelle.clone()),
        )
    };
    cles.sort_by_key(ordre);

    // Élèves cochés : leur ligne (niveau et série de leur classe) et s'il faut les changer.
    let mut coches: HashMap<CleMontant, (u32, u32)> = HashMap::new();
    let mut a_changer = Vec::new();
    for v in prise_en_charge::courantes_de_l_annee(cnx, annee.id, contexte).await? {
        if v.statut != StatutPriseEnCharge::EnCours {
            continue;
        }
        let cible = lire_cible(cnx, v.inscription_id, contexte).await?;
        let propose = montant_propose(cnx, &cible, contexte).await?;
        let compte = coches
            .entry((cible.classe.niveau_id, cible.classe.serie_id))
            .or_default();
        compte.0 += 1;
        if propose != v.montant_attendu {
            compte.1 += 1;
            a_changer.push(v.inscription_id);
        }
    }
    let mut lignes = Vec::with_capacity(cles.len());
    for (niveau_id, serie_id) in cles {
        let (eleves_coches, eleves_a_changer) = coches
            .get(&(niveau_id, serie_id))
            .copied()
            .unwrap_or_default();
        lignes.push(LigneMontantEtat {
            niveau_id,
            niveau: niveaux
                .get(&niveau_id)
                .map(|n| n.libelle.clone())
                .unwrap_or_default(),
            serie_id,
            serie: serie_id
                .and_then(|s| series.get(&s))
                .map(|s| s.libelle.clone()),
            montant: reglees
                .iter()
                .find(|m| m.niveau_id == niveau_id && m.serie_id == serie_id)
                .and_then(|m| m.montant),
            frais_ecole: scolarite_de_la_grille(cnx, annee.id, niveau_id, serie_id, contexte)
                .await?,
            eleves_coches,
            eleves_a_changer,
        });
    }
    Ok(MontantsEtat {
        annee_id: annee.id,
        annee: annee.libelle.clone(),
        lignes,
        inscriptions_a_changer: a_changer,
        lue_le: crate::fraicheur::instant(cnx, &lignes_montants(annee)).await?,
    })
}

// ---------------------------------------------------------------------------------------------
// Aides des essais
// ---------------------------------------------------------------------------------------------

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
