//! Prise en charge par l'État (SPEC_PRISE_EN_CHARGE_ETAT) : la scolarité d'un élève orienté par
//! l'État est due par l'État, pas par le parent ; les frais d'inscription ne sont dus par
//! personne ; le reste (cantine, tenue…) reste au parent. Le partage de chaque échéance entre
//! l'État et le parent, le placement de l'argent de l'État et le report du trop-payé du parent
//! **ne sont pas stockés** : ils se calculent à chaque lecture (R-20) par [`partager`], calcul
//! pur, sans base de données. Rien ne réécrit une facture, une ligne, une échéance ni une
//! imputation de versement.

use serde::{Deserialize, Serialize};

use crate::commun::{DateCivile, ErreurMontant, Horodatage, Id, Montant};
use crate::core::{AnneeScolaire, Etablissement, Utilisateur};
use crate::finance::facture::Echeance;
use crate::finance::reprise::PartReprise;
use crate::finance::tarif::TypeFrais;
use crate::scolarite::{Inscription, Niveau, Serie};
use crate::tresorerie::recette::Recette;

/// Longueur maximale de la référence d'orientation (numéro de décision, de place au DEF).
pub const REFERENCE_MAX: usize = 60;

/// Code stable de la catégorie de recette des versements de l'État.
pub const CATEGORIE_VERSEMENT_ETAT: &str = "versement_etat";

/// Payeur proposé d'un versement de l'État.
pub const PAYEUR_PAR_DEFAUT: &str = "État";

// ---------------------------------------------------------------------------------------------
// Modèle (§3.1, §3.2, §7.1)
// ---------------------------------------------------------------------------------------------

/// État d'une version de la prise en charge d'une inscription.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "snake_case")]
pub enum StatutPriseEnCharge {
    /// Case cochée : l'État doit la scolarité.
    EnCours,
    /// Case décochée : l'État ne doit plus que ce qu'il a déjà payé.
    Terminee,
    /// Version ancienne, remplacée par une autre ; jamais supprimée (R-14).
    Remplacee,
}

impl StatutPriseEnCharge {
    /// Nom persisté.
    pub const fn nom(&self) -> &'static str {
        match self {
            Self::EnCours => "en_cours",
            Self::Terminee => "terminee",
            Self::Remplacee => "remplacee",
        }
    }

    /// Lecture d'un nom persisté.
    pub fn depuis_nom(nom: &str) -> Option<Self> {
        [Self::EnCours, Self::Terminee, Self::Remplacee]
            .into_iter()
            .find(|s| s.nom() == nom)
    }
}

/// Une version de la prise en charge d'une inscription (§3.1).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
pub struct PriseEnChargeEtat {
    /// Identifiant technique.
    pub id: Id<PriseEnChargeEtat>,
    /// Établissement propriétaire.
    pub etablissement_id: Id<Etablissement>,
    /// Inscription de l'année : la finance référence l'inscription, jamais l'élève.
    pub inscription_id: Id<Inscription>,
    /// En cours, terminée ou remplacée.
    pub statut: StatutPriseEnCharge,
    /// Ce que l'État doit payer pour l'élève pour l'année, positif ou nul.
    pub montant_attendu: Montant,
    /// Ce que l'école renonce à attendre de l'État, entre zéro et le montant attendu.
    pub abandonne: Montant,
    /// Référence d'orientation, facultative.
    pub reference: String,
    /// Motif : obligatoire pour une version terminée et pour un abandon.
    pub motif: String,
    /// Version qu'elle remplace.
    pub remplace_id: Option<Id<PriseEnChargeEtat>>,
    /// Instant de la création.
    pub cree_le: Horodatage,
    /// Auteur.
    pub cree_par: Option<Id<Utilisateur>>,
    /// Instant de la dernière écriture (passage à `remplacee`) : l'écran le renvoie.
    pub mis_a_jour_le: Horodatage,
    /// Auteur de la dernière écriture.
    pub mis_a_jour_par: Option<Id<Utilisateur>>,
}

/// Montant attendu de l'État pour un niveau (et une série) d'une année (§3.2) ; vide, les frais
/// de l'école.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
pub struct MontantEtat {
    /// Identifiant technique.
    pub id: Id<MontantEtat>,
    /// Établissement.
    pub etablissement_id: Id<Etablissement>,
    /// Année.
    pub annee_id: Id<AnneeScolaire>,
    /// Niveau.
    pub niveau_id: Id<Niveau>,
    /// Série, facultative.
    pub serie_id: Option<Id<Serie>>,
    /// Montant ; absent, les frais de l'école.
    pub montant: Option<Montant>,
}

/// État d'un versement de l'État.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "snake_case")]
pub enum StatutVersementEtat {
    /// En vigueur.
    Valide,
    /// Annulé avec un motif ; jamais supprimé.
    Annule,
}

impl StatutVersementEtat {
    /// Nom persisté.
    pub const fn nom(&self) -> &'static str {
        match self {
            Self::Valide => "valide",
            Self::Annule => "annule",
        }
    }

    /// Lecture d'un nom persisté.
    pub fn depuis_nom(nom: &str) -> Option<Self> {
        [Self::Valide, Self::Annule]
            .into_iter()
            .find(|s| s.nom() == nom)
    }
}

/// Un versement de l'État (§7.1) : l'argent, porté par une recette de trésorerie, ou reçu avant
/// GESCO (déjà dans le solde d'ouverture, sans recette).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
pub struct VersementEtat {
    /// Identifiant technique.
    pub id: Id<VersementEtat>,
    /// Établissement.
    pub etablissement_id: Id<Etablissement>,
    /// Année scolaire concernée (une année précédente pour un arriéré).
    pub annee_id: Id<AnneeScolaire>,
    /// Jour de réception.
    pub date: DateCivile,
    /// Montant reçu, strictement positif.
    pub montant: Montant,
    /// Numéro de l'ordre de virement ou du chèque.
    pub reference: String,
    /// Libellé.
    pub libelle: String,
    /// Payeur (« État » par défaut).
    pub payeur: String,
    /// Recette qui porte l'argent ; absente exactement pour un versement reçu avant GESCO.
    pub recette_id: Option<Id<Recette>>,
    /// Reçu avant GESCO : déjà dans le solde d'ouverture du compte.
    pub avant_gesco: bool,
    /// Valide ou annulé.
    pub statut: StatutVersementEtat,
    /// Motif de l'annulation.
    pub motif_annulation: String,
    /// Instant de la saisie.
    pub cree_le: Horodatage,
    /// Auteur.
    pub cree_par: Option<Id<Utilisateur>>,
    /// Instant de la dernière écriture : l'écran le renvoie.
    pub mis_a_jour_le: Horodatage,
    /// Auteur de la dernière écriture.
    pub mis_a_jour_par: Option<Id<Utilisateur>>,
}

/// D'où vient une part d'un versement de l'État.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "snake_case")]
pub enum OriginePartEtat {
    /// Répartie à l'écran.
    Ecran,
    /// Importée d'un fichier.
    Import,
}

impl OriginePartEtat {
    /// Nom persisté.
    pub const fn nom(&self) -> &'static str {
        match self {
            Self::Ecran => "ecran",
            Self::Import => "import",
        }
    }

    /// Lecture d'un nom persisté.
    pub fn depuis_nom(nom: &str) -> Option<Self> {
        [Self::Ecran, Self::Import]
            .into_iter()
            .find(|o| o.nom() == nom)
    }
}

/// État d'une part d'un versement de l'État.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "snake_case")]
pub enum StatutPartEtat {
    /// En vigueur.
    Active,
    /// Annulée (répartition modifiée, versement annulé) ; jamais supprimée.
    Annulee,
}

impl StatutPartEtat {
    /// Nom persisté.
    pub const fn nom(&self) -> &'static str {
        match self {
            Self::Active => "active",
            Self::Annulee => "annulee",
        }
    }

    /// Lecture d'un nom persisté.
    pub fn depuis_nom(nom: &str) -> Option<Self> {
        [Self::Active, Self::Annulee]
            .into_iter()
            .find(|s| s.nom() == nom)
    }
}

/// Ce qu'un versement de l'État attribue à une inscription.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
pub struct PartVersementEtat {
    /// Identifiant technique.
    pub id: Id<PartVersementEtat>,
    /// Versement.
    pub versement_etat_id: Id<VersementEtat>,
    /// Inscription.
    pub inscription_id: Id<Inscription>,
    /// Montant, strictement positif.
    pub montant: Montant,
    /// Écran ou import.
    pub origine: OriginePartEtat,
    /// Active ou annulée.
    pub statut: StatutPartEtat,
}

/// Vérifie et nettoie la référence d'orientation.
pub fn verifier_reference(reference: &str) -> Result<String, &'static str> {
    let reference = reference.trim();
    if reference.chars().count() > REFERENCE_MAX {
        return Err("prise_en_charge.reference_trop_longue");
    }
    Ok(reference.to_owned())
}

// ---------------------------------------------------------------------------------------------
// Le partage d'une facture (§3.4)
// ---------------------------------------------------------------------------------------------

/// Nature d'une échéance pour le partage, d'après le type de frais de sa ligne de facture
/// (`LigneFacture.type_frais`) (§3.2).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "snake_case")]
pub enum NatureEcheance {
    /// Scolarité : à l'État pour un élève pris en charge.
    Scolarite,
    /// Frais d'inscription : dus par personne pour un élève dont la case est cochée.
    Inscription,
    /// Cantine, tenue, transport, examen, autre : au parent.
    Autre,
}

impl NatureEcheance {
    /// Nature d'après le type de frais de la ligne.
    pub const fn depuis_type_frais(type_frais: TypeFrais) -> Self {
        match type_frais {
            TypeFrais::Scolarite => Self::Scolarite,
            TypeFrais::Inscription => Self::Inscription,
            TypeFrais::Cantine
            | TypeFrais::Transport
            | TypeFrais::Tenue
            | TypeFrais::Examen
            | TypeFrais::Autre => Self::Autre,
        }
    }
}

/// Qui doit une échéance, selon la version courante de la prise en charge.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "snake_case")]
pub enum DebiteurEcheance {
    /// Le parent (échéance ordinaire).
    Parent,
    /// L'État : la scolarité d'un élève pris en charge (ou dont la prise en charge est
    /// terminée, jusqu'à ce que l'État a payé).
    Etat,
    /// Personne : l'inscription d'un élève dont la case est cochée, renoncée par l'école.
    NonDue,
}

impl DebiteurEcheance {
    /// Échéance « couverte » (à l'État ou non due).
    pub const fn couverte(&self) -> bool {
        matches!(self, Self::Etat | Self::NonDue)
    }
}

/// Version courante de la prise en charge, réduite à ce que le calcul lit.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "snake_case", tag = "statut")]
pub enum VersionCourante {
    /// Pas de prise en charge.
    #[default]
    Aucune,
    /// Case cochée.
    EnCours {
        /// Montant attendu de l'État.
        attendu: Montant,
        /// Abandonné par l'école.
        abandonne: Montant,
    },
    /// Case décochée.
    Terminee,
}

impl VersionCourante {
    /// Lue depuis la version courante d'une inscription (une version `remplacee` n'est jamais
    /// courante : elle compte comme aucune).
    pub fn depuis(version: Option<&PriseEnChargeEtat>) -> Self {
        match version.map(|v| (v.statut, v.montant_attendu, v.abandonne)) {
            Some((StatutPriseEnCharge::EnCours, attendu, abandonne)) => {
                Self::EnCours { attendu, abandonne }
            }
            Some((StatutPriseEnCharge::Terminee, ..)) => Self::Terminee,
            Some((StatutPriseEnCharge::Remplacee, ..)) | None => Self::Aucune,
        }
    }

    /// Qui doit une échéance de cette nature.
    pub const fn debiteur(&self, nature: NatureEcheance) -> DebiteurEcheance {
        match (self, nature) {
            (Self::EnCours { .. } | Self::Terminee, NatureEcheance::Scolarite) => {
                DebiteurEcheance::Etat
            }
            (Self::EnCours { .. }, NatureEcheance::Inscription) => DebiteurEcheance::NonDue,
            _ => DebiteurEcheance::Parent,
        }
    }
}

/// Une échéance due d'une facture, telle que le partage la lit.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EcheanceAPartager {
    /// Échéance.
    pub echeance_id: Id<Echeance>,
    /// Nature, d'après le type de frais de sa ligne.
    pub nature: NatureEcheance,
    /// Dû après remises et baisses, comme aujourd'hui : `net(e)`.
    pub net: Montant,
    /// Argent du parent imputé sur l'échéance : versements imputés et avoirs utilisés, **sans**
    /// la reprise avant GESCO (que le calcul place lui-même) : `m(e)`.
    pub paye_parent: Montant,
}

/// Entrée du partage d'une facture vivante.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct EntreePartage {
    /// Échéances dues, rangées par date puis par rang (plus ancienne d'abord).
    pub echeances: Vec<EcheanceAPartager>,
    /// Version courante de la prise en charge.
    pub version: VersionCourante,
    /// `S` : ce que l'État a payé pour l'inscription (parts actives des versements de l'État
    /// non annulés).
    pub paye_etat: Montant,
    /// `B` : remboursé à la famille (dépenses de remboursement payées, non annulées).
    pub rembourse: Montant,
    /// Montant de la reprise avant GESCO active (zéro sans reprise).
    pub reprise: Montant,
}

/// Une échéance partagée (§3.4, §4.1).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
pub struct EcheancePartagee {
    /// Échéance.
    pub echeance_id: Id<Echeance>,
    /// Nature.
    pub nature: NatureEcheance,
    /// Qui la doit.
    pub debiteur: DebiteurEcheance,
    /// Dû après remises et baisses.
    pub net: Montant,
    /// Part de l'État.
    pub part_etat: Montant,
    /// Ce que personne ne doit, à la charge de l'école.
    pub renonce: Montant,
    /// Part du parent : `net − part_etat − renonce`.
    pub part_parent: Montant,
    /// Payé par l'État sur sa part.
    pub paye_etat: Montant,
    /// Reste dû par l'État sur sa part.
    pub reste_etat: Montant,
    /// Argent du parent imputé (versements et avoirs), sans la reprise : `m(e)`.
    pub verse_parent: Montant,
    /// Trop-payé de l'échéance : `x(e) = max(0, m(e) − part_parent(e))`.
    pub trop_paye: Montant,
    /// Remboursé imputé sur ce trop-payé : `b1(e)`.
    pub trop_paye_rembourse: Montant,
    /// Argent rendu alors qu'il était dû, que le parent doit de nouveau : `b2(e)`.
    pub rendu: Montant,
    /// Argent du parent qui compte pour sa part : `c(e) = min(m(e), part_parent(e)) − b2(e)`.
    pub compte: Montant,
    /// Report du trop-payé : `t(e)`.
    pub report: Montant,
    /// Part de la reprise avant GESCO : `u(e)`.
    pub reprise: Montant,
    /// Payé du parent qui compte pour l'échéance : `c + t + u`.
    pub paye: Montant,
    /// Reste du parent : `part_parent − c − t − u`.
    pub reste: Montant,
}

/// Partage d'une facture : les échéances et les totaux (§3.4, §4.1, §9.1).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
pub struct Partage {
    /// Échéances, dans l'ordre reçu.
    pub echeances: Vec<EcheancePartagee>,
    /// `N` : somme des `net` des échéances à l'État.
    pub net_etat: Montant,
    /// `E` : ce que l'État doit sur la facture (somme des `part_etat`).
    pub du_etat: Montant,
    /// `K` : complément attendu de l'État, hors facture.
    pub complement: Montant,
    /// Total renoncé, à la charge de l'école.
    pub renonce: Montant,
    /// `S` : reçu de l'État pour l'inscription.
    pub recu_etat: Montant,
    /// Payé par l'État sur la facture (somme des `paye_etat`).
    pub paye_etat: Montant,
    /// Reste dû par l'État sur la facture (somme des `reste_etat`).
    pub reste_etat: Montant,
    /// Payé par l'État sur le complément, au plus `K`.
    pub paye_complement: Montant,
    /// Reçu de l'État au-delà de sa part et du complément.
    pub recu_en_plus: Montant,
    /// `X` : trop-payé du parent.
    pub trop_paye: Montant,
    /// Somme des `b1` : remboursé imputé sur le trop-payé.
    pub trop_paye_rembourse: Montant,
    /// Somme des `b2` : rendu alors que dû, que le parent doit de nouveau.
    pub rendu: Montant,
    /// `T` : trop-payé disponible pour le report.
    pub trop_paye_disponible: Montant,
    /// Somme des `t` : trop-payé reporté sur la part du parent.
    pub trop_paye_reporte: Montant,
    /// `B` : remboursé à la famille.
    pub rembourse: Montant,
    /// `B_reprise = min(B, reprise)`.
    pub rembourse_reprise: Montant,
    /// `B_tp = B − B_reprise`.
    pub rembourse_trop_paye: Montant,
    /// Montant de la reprise active.
    pub reprise: Montant,
    /// `U = reprise − B_reprise`.
    pub reprise_utilisable: Montant,
    /// Somme des `u` : reprise portée par les échéances.
    pub reprise_imputee: Montant,
    /// À rembourser au parent : `(T − somme des t) + (U − somme des u)`.
    pub a_rembourser: Montant,
    /// Remboursé qui n'a trouvé aucun argent du parent où s'imputer : toujours zéro tant que le
    /// remboursé ne dépasse pas l'argent du parent et la reprise (refus
    /// `versement.deja_rembourse`) ; gardé pour que le contrôle reste exact.
    pub rembourse_en_trop: Montant,
    /// Somme des `part_parent`.
    pub part_parent: Montant,
    /// Payé du parent qui compte (somme des `c + t + u`).
    pub paye_parent: Montant,
    /// Reste du parent (somme des `reste`).
    pub reste_parent: Montant,
}

impl Partage {
    /// Ce que l'État doit encore pour l'élève : reste sur la facture plus complément non reçu.
    pub fn doit_encore_etat(&self) -> Result<Montant, ErreurMontant> {
        self.reste_etat
            .ajouter(self.complement)?
            .soustraire(self.paye_complement)
    }

    /// Dû par l'État en tout : `E + K`.
    pub fn du_etat_total(&self) -> Result<Montant, ErreurMontant> {
        self.du_etat.ajouter(self.complement)
    }

    /// Parts de la reprise (`u(e)` non nuls), dans l'ordre des échéances : ce que
    /// `recalculer_dans` écrit dans `reprise_imputation`, à la place de `reprise::imputer`.
    pub fn parts_reprise(&self) -> Vec<PartReprise> {
        self.echeances
            .iter()
            .filter(|e| !e.reprise.est_nul())
            .map(|e| PartReprise {
                echeance_id: e.echeance_id,
                montant: e.reprise,
            })
            .collect()
    }
}

fn somme(montants: &[Montant]) -> Result<Montant, ErreurMontant> {
    montants
        .iter()
        .try_fold(Montant::ZERO, |s, m| s.ajouter(*m))
}

/// `a − b` pour chaque position.
fn moins(a: &[Montant], b: &[Montant]) -> Result<Vec<Montant>, ErreurMontant> {
    a.iter().zip(b).map(|(x, y)| x.soustraire(*y)).collect()
}

/// Place `montant` sur des capacités, en visitant les positions dans l'ordre donné (chaque
/// position une fois au plus) ; chaque part au plus sa capacité. Rend les parts (une par
/// capacité) et ce qui n'a pas trouvé de place.
fn placer(
    montant: Montant,
    capacites: &[Montant],
    ordre: impl IntoIterator<Item = usize>,
) -> Result<(Vec<Montant>, Montant), ErreurMontant> {
    let mut parts = vec![Montant::ZERO; capacites.len()];
    let mut a_placer = montant.max(Montant::ZERO);
    for i in ordre {
        if a_placer.est_nul() {
            break;
        }
        let (Some(capacite), Some(part)) = (capacites.get(i), parts.get_mut(i)) else {
            continue;
        };
        let p = capacite.soustraire(*part)?.max(Montant::ZERO).min(a_placer);
        *part = part.ajouter(p)?;
        a_placer = a_placer.soustraire(p)?;
    }
    Ok((parts, a_placer))
}

/// Partage une facture entre l'État et le parent (§3.4), sans rien écrire.
///
/// Ordres de placement : la part de l'État de la plus ancienne échéance à la plus récente ; le
/// renoncé (écart et abandon) de la plus récente à la plus ancienne ; l'argent de l'État de la
/// plus ancienne à la plus récente, puis le complément, puis le reçu en plus ; le remboursé du
/// trop-payé d'abord sur les trop-payés (`b1`, plus ancienne d'abord), puis sur l'argent du parent
/// qui compte (`b2`), échéances couvertes d'abord, plus ancienne d'abord, puis les autres ; le
/// report du trop-payé (`t`) puis la reprise (`u`), plus ancienne d'abord.
///
/// Les montants d'entrée négatifs comptent pour zéro. Sans prise en charge, avec un argent du
/// parent jamais supérieur au dû de l'échéance et un remboursé jamais supérieur à la reprise, le
/// résultat est exactement celui d'avant (`etat_echeances` et `reprise::imputer`).
pub fn partager(entree: &EntreePartage) -> Result<Partage, ErreurMontant> {
    let zero = Montant::ZERO;
    let positif = |m: Montant| m.max(zero);
    let version = entree.version;
    let debiteurs: Vec<DebiteurEcheance> = entree
        .echeances
        .iter()
        .map(|e| version.debiteur(e.nature))
        .collect();
    let nets: Vec<Montant> = entree.echeances.iter().map(|e| positif(e.net)).collect();
    let verses: Vec<Montant> = entree
        .echeances
        .iter()
        .map(|e| positif(e.paye_parent))
        .collect();
    let recu_etat = positif(entree.paye_etat);
    let rembourse = positif(entree.rembourse);
    let reprise = positif(entree.reprise);
    let nb = nets.len();
    let plus_ancienne = || 0..nb;

    // Ce que l'État doit sur la facture (E) et hors facture (K).
    let net_etat_par_echeance: Vec<Montant> = nets
        .iter()
        .zip(&debiteurs)
        .map(|(n, d)| {
            if *d == DebiteurEcheance::Etat {
                *n
            } else {
                zero
            }
        })
        .collect();
    let net_etat = somme(&net_etat_par_echeance)?;
    let (du_etat, complement) = match version {
        VersionCourante::Aucune => (zero, zero),
        VersionCourante::EnCours { attendu, abandonne } => {
            let a = positif(positif(attendu).soustraire(positif(abandonne))?);
            let e = a.min(net_etat);
            (e, a.soustraire(e)?)
        }
        VersionCourante::Terminee => (recu_etat.min(net_etat), zero),
    };

    // Part de l'État, de la plus ancienne à la plus récente.
    let (part_etat, _) = placer(du_etat, &net_etat_par_echeance, plus_ancienne())?;

    // Renoncé : l'écart et l'abandon, de la plus récente à la plus ancienne ; l'inscription non
    // due, tout son dû. Seulement pour une version en cours.
    let renonce: Vec<Montant> = if matches!(version, VersionCourante::EnCours { .. }) {
        let disponible = moins(&net_etat_par_echeance, &part_etat)?;
        let (sur_etat, _) = placer(net_etat.soustraire(du_etat)?, &disponible, (0..nb).rev())?;
        sur_etat
            .iter()
            .zip(&debiteurs)
            .zip(&nets)
            .map(|((r, d), n)| {
                if *d == DebiteurEcheance::NonDue {
                    *n
                } else {
                    *r
                }
            })
            .collect()
    } else {
        vec![zero; nb]
    };
    let part_parent = moins(&moins(&nets, &part_etat)?, &renonce)?;

    // Côté État : S sur les parts de l'État, puis le complément, puis le reçu en plus.
    let (paye_etat, surplus) = placer(recu_etat, &part_etat, plus_ancienne())?;
    let reste_etat = moins(&part_etat, &paye_etat)?;
    let paye_complement = surplus.min(complement);
    let recu_en_plus = surplus.soustraire(paye_complement)?;

    // Côté parent.
    let trop_paye: Vec<Montant> = verses
        .iter()
        .zip(&part_parent)
        .map(|(m, p)| m.soustraire(*p).map(positif))
        .collect::<Result<_, _>>()?;
    let total_trop_paye = somme(&trop_paye)?;
    let rembourse_reprise = rembourse.min(reprise);
    let rembourse_trop_paye = rembourse.soustraire(rembourse_reprise)?;
    let (b1, apres_b1) = placer(rembourse_trop_paye, &trop_paye, plus_ancienne())?;
    let compte_brut: Vec<Montant> = verses
        .iter()
        .zip(&part_parent)
        .map(|(m, p)| (*m).min(*p))
        .collect();
    let ordre_b2: Vec<usize> = (0..nb)
        .filter(|i| debiteurs.get(*i).is_some_and(DebiteurEcheance::couverte))
        .chain((0..nb).filter(|i| !debiteurs.get(*i).is_some_and(DebiteurEcheance::couverte)))
        .collect();
    let (b2, rembourse_en_trop) = placer(apres_b1, &compte_brut, ordre_b2)?;
    let compte = moins(&compte_brut, &b2)?;
    let trop_paye_disponible = total_trop_paye.soustraire(somme(&b1)?)?;
    let reprise_utilisable = reprise.soustraire(rembourse_reprise)?;
    let r0 = moins(&part_parent, &compte)?;
    let (report, report_en_trop) = placer(trop_paye_disponible, &r0, plus_ancienne())?;
    let apres_report = moins(&r0, &report)?;
    let (part_reprise, reprise_en_trop) =
        placer(reprise_utilisable, &apres_report, plus_ancienne())?;
    let reste = moins(&apres_report, &part_reprise)?;
    let a_rembourser = report_en_trop.ajouter(reprise_en_trop)?;

    let mut echeances = Vec::with_capacity(nb);
    for (i, e) in entree.echeances.iter().enumerate() {
        let a = |v: &[Montant]| v.get(i).copied().unwrap_or(zero);
        let (c, t, u) = (a(&compte), a(&report), a(&part_reprise));
        echeances.push(EcheancePartagee {
            echeance_id: e.echeance_id,
            nature: e.nature,
            debiteur: debiteurs
                .get(i)
                .copied()
                .unwrap_or(DebiteurEcheance::Parent),
            net: a(&nets),
            part_etat: a(&part_etat),
            renonce: a(&renonce),
            part_parent: a(&part_parent),
            paye_etat: a(&paye_etat),
            reste_etat: a(&reste_etat),
            verse_parent: a(&verses),
            trop_paye: a(&trop_paye),
            trop_paye_rembourse: a(&b1),
            rendu: a(&b2),
            compte: c,
            report: t,
            reprise: u,
            paye: c.ajouter(t)?.ajouter(u)?,
            reste: a(&reste),
        });
    }
    let total = |f: fn(&EcheancePartagee) -> Montant| {
        echeances
            .iter()
            .try_fold(zero, |s: Montant, e| s.ajouter(f(e)))
    };
    Ok(Partage {
        net_etat,
        du_etat,
        complement,
        renonce: total(|e| e.renonce)?,
        recu_etat,
        paye_etat: total(|e| e.paye_etat)?,
        reste_etat: total(|e| e.reste_etat)?,
        paye_complement,
        recu_en_plus,
        trop_paye: total_trop_paye,
        trop_paye_rembourse: total(|e| e.trop_paye_rembourse)?,
        rendu: total(|e| e.rendu)?,
        trop_paye_disponible,
        trop_paye_reporte: total(|e| e.report)?,
        rembourse,
        rembourse_reprise,
        rembourse_trop_paye,
        reprise,
        reprise_utilisable,
        reprise_imputee: total(|e| e.reprise)?,
        a_rembourser,
        rembourse_en_trop,
        part_parent: total(|e| e.part_parent)?,
        paye_parent: total(|e| e.paye)?,
        reste_parent: total(|e| e.reste)?,
        echeances,
    })
}

#[cfg(test)]
#[allow(clippy::indexing_slicing, clippy::unwrap_used)]
mod tests {
    use super::*;
    use crate::finance::reprise;
    use proptest::prelude::*;

    fn f(n: i64) -> Montant {
        Montant::depuis_francs(n)
    }

    fn id(n: usize) -> Id<Echeance> {
        Id::depuis_octets([u8::try_from(n % 250).unwrap() + 1; 16])
    }

    fn echeance(n: usize, nature: NatureEcheance, net: i64, paye: i64) -> EcheanceAPartager {
        EcheanceAPartager {
            echeance_id: id(n),
            nature,
            net: f(net),
            paye_parent: f(paye),
        }
    }

    use NatureEcheance::{Autre, Inscription, Scolarite};

    /// L'exemple du §12 (essai 2) : inscription 15 000 F au 1er octobre ; scolarité 120 000 F en
    /// trois tranches (48 000 F au 15 octobre, 36 000 F au 15 janvier, 36 000 F au 15 avril) ;
    /// cantine 30 000 F au 15 octobre ; le parent a payé l'inscription et la 1re tranche.
    /// Rangées par date puis rang : inscription, 1re tranche, cantine, 2e, 3e.
    fn exemple(avec_cantine: bool, paye: bool) -> Vec<EcheanceAPartager> {
        let p = |m: i64| if paye { m } else { 0 };
        let mut e = vec![
            echeance(0, Inscription, 15_000, p(15_000)),
            echeance(1, Scolarite, 48_000, p(48_000)),
        ];
        if avec_cantine {
            e.push(echeance(2, Autre, 30_000, 0));
        }
        e.push(echeance(3, Scolarite, 36_000, 0));
        e.push(echeance(4, Scolarite, 36_000, 0));
        e
    }

    fn en_cours(attendu: i64, abandonne: i64) -> VersionCourante {
        VersionCourante::EnCours {
            attendu: f(attendu),
            abandonne: f(abandonne),
        }
    }

    fn partage(
        echeances: Vec<EcheanceAPartager>,
        version: VersionCourante,
        s: i64,
        b: i64,
        reprise: i64,
    ) -> Partage {
        partager(&EntreePartage {
            echeances,
            version,
            paye_etat: f(s),
            rembourse: f(b),
            reprise: f(reprise),
        })
        .unwrap()
    }

    fn colonne(p: &Partage, g: fn(&EcheancePartagee) -> Montant) -> Vec<i64> {
        p.echeances.iter().map(|e| g(e).francs()).collect()
    }

    #[test]
    fn la_nature_suit_le_type_de_frais() {
        assert_eq!(
            NatureEcheance::depuis_type_frais(TypeFrais::Scolarite),
            Scolarite
        );
        assert_eq!(
            NatureEcheance::depuis_type_frais(TypeFrais::Inscription),
            Inscription
        );
        for t in [
            TypeFrais::Cantine,
            TypeFrais::Transport,
            TypeFrais::Tenue,
            TypeFrais::Examen,
            TypeFrais::Autre,
        ] {
            assert_eq!(NatureEcheance::depuis_type_frais(t), Autre);
        }
    }

    #[test]
    fn les_noms_persistes_se_relisent() {
        for s in [
            StatutPriseEnCharge::EnCours,
            StatutPriseEnCharge::Terminee,
            StatutPriseEnCharge::Remplacee,
        ] {
            assert_eq!(StatutPriseEnCharge::depuis_nom(s.nom()), Some(s));
        }
        for s in [StatutVersementEtat::Valide, StatutVersementEtat::Annule] {
            assert_eq!(StatutVersementEtat::depuis_nom(s.nom()), Some(s));
        }
        for s in [StatutPartEtat::Active, StatutPartEtat::Annulee] {
            assert_eq!(StatutPartEtat::depuis_nom(s.nom()), Some(s));
        }
        for o in [OriginePartEtat::Ecran, OriginePartEtat::Import] {
            assert_eq!(OriginePartEtat::depuis_nom(o.nom()), Some(o));
        }
        assert_eq!(StatutPriseEnCharge::depuis_nom("cochee"), None);
        assert!(verifier_reference(&"x".repeat(61)).is_err());
        assert_eq!(
            verifier_reference(" DEF-2026-118 ").unwrap(),
            "DEF-2026-118"
        );
    }

    /// Essai 2 du §12 : case cochée après les paiements du parent, puis versement de l'État.
    #[test]
    fn essai_2_partage_trop_paye_reporte_et_a_rembourser() {
        let p = partage(exemple(true, true), en_cours(120_000, 0), 0, 0, 0);
        assert_eq!(p.du_etat, f(120_000));
        assert_eq!(p.complement, Montant::ZERO);
        assert_eq!(colonne(&p, |e| e.part_etat), [0, 48_000, 0, 36_000, 36_000]);
        assert_eq!(colonne(&p, |e| e.renonce), [15_000, 0, 0, 0, 0]);
        assert_eq!(colonne(&p, |e| e.part_parent), [0, 0, 30_000, 0, 0]);
        assert_eq!(p.trop_paye, f(63_000));
        assert_eq!(p.trop_paye_reporte, f(30_000));
        assert_eq!(colonne(&p, |e| e.report), [0, 0, 30_000, 0, 0]);
        assert_eq!(p.a_rembourser, f(33_000));
        assert_eq!(p.reste_parent, Montant::ZERO);
        assert_eq!(p.reste_etat, f(120_000));
        assert_eq!(p.doit_encore_etat().unwrap(), f(120_000));

        // Versement de l'État de 120 000 F : l'État ne doit plus rien.
        let p = partage(exemple(true, true), en_cours(120_000, 0), 120_000, 0, 0);
        assert_eq!(p.reste_etat, Montant::ZERO);
        assert_eq!(p.recu_en_plus, Montant::ZERO);
        assert_eq!(p.reste_parent, Montant::ZERO);
        assert_eq!(p.doit_encore_etat().unwrap(), Montant::ZERO);

        // Même élève coché dès l'inscription : seule la cantine reste au parent.
        let p = partage(exemple(true, false), en_cours(120_000, 0), 0, 0, 0);
        assert_eq!(p.reste_parent, f(30_000));
        assert_eq!(colonne(&p, |e| e.reste), [0, 0, 30_000, 0, 0]);
        assert_eq!(p.a_rembourser, Montant::ZERO);
    }

    /// Essai 3 du §12 : attendu plus bas que la scolarité, puis abandon.
    #[test]
    fn essai_3_ecart_puis_abandon_a_la_charge_de_l_ecole() {
        let p = partage(exemple(true, false), en_cours(100_000, 0), 0, 0, 0);
        assert_eq!(colonne(&p, |e| e.part_etat), [0, 48_000, 0, 36_000, 16_000]);
        assert_eq!(colonne(&p, |e| e.renonce), [15_000, 0, 0, 0, 20_000]);
        assert_eq!(p.reste_parent, f(30_000));

        let p = partage(exemple(true, false), en_cours(100_000, 36_000), 0, 0, 0);
        assert_eq!(p.du_etat, f(64_000));
        assert_eq!(colonne(&p, |e| e.part_etat), [0, 48_000, 0, 16_000, 0]);
        assert_eq!(colonne(&p, |e| e.renonce), [15_000, 0, 0, 20_000, 36_000]);
        assert_eq!(p.renonce, f(71_000));
        // 56 000 F renoncés sur la scolarité, 15 000 F sur l'inscription.
        assert_eq!(p.reste_parent, f(30_000));
    }

    /// Essai 3 bis du §12 : attendu plus haut que la scolarité, complément hors facture.
    #[test]
    fn essai_3_bis_complement_hors_facture() {
        let scolarite = || {
            vec![
                echeance(1, Scolarite, 40_000, 0),
                echeance(3, Scolarite, 30_000, 0),
                echeance(4, Scolarite, 30_000, 0),
            ]
        };
        let p = partage(scolarite(), en_cours(126_000, 0), 0, 0, 0);
        assert_eq!(p.du_etat, f(100_000));
        assert_eq!(p.complement, f(26_000));
        assert_eq!(p.du_etat_total().unwrap(), f(126_000));
        assert_eq!(p.doit_encore_etat().unwrap(), f(126_000));
        assert_eq!(p.renonce, Montant::ZERO);

        let p = partage(scolarite(), en_cours(126_000, 0), 126_000, 0, 0);
        assert_eq!(p.reste_etat, Montant::ZERO);
        assert_eq!(p.paye_complement, f(26_000));
        assert_eq!(p.recu_en_plus, Montant::ZERO);
        assert_eq!(p.doit_encore_etat().unwrap(), Montant::ZERO);

        let p = partage(scolarite(), en_cours(126_000, 0), 110_000, 0, 0);
        assert_eq!(p.reste_etat, Montant::ZERO);
        assert_eq!(p.paye_complement, f(10_000));
        assert_eq!(p.doit_encore_etat().unwrap(), f(16_000));

        let p = partage(scolarite(), en_cours(126_000, 16_000), 110_000, 0, 0);
        assert_eq!(p.complement, f(10_000));
        assert_eq!(p.doit_encore_etat().unwrap(), Montant::ZERO);
        assert_eq!(p.recu_en_plus, Montant::ZERO);
    }

    /// Essai 4 du §12 : case décochée après le remboursement de 33 000 F.
    #[test]
    fn essai_4_decocher_apres_un_remboursement() {
        let p = partage(exemple(true, true), VersionCourante::Terminee, 0, 33_000, 0);
        assert_eq!(p.du_etat, Montant::ZERO);
        assert_eq!(p.doit_encore_etat().unwrap(), Montant::ZERO);
        assert_eq!(p.reste_parent, f(135_000));
        assert_eq!(colonne(&p, |e| e.rendu), [0, 33_000, 0, 0, 0]);
        assert_eq!(
            colonne(&p, |e| e.reste),
            [0, 33_000, 30_000, 36_000, 36_000]
        );
        assert_eq!(p.a_rembourser, Montant::ZERO);

        // Puis un versement de l'État de 60 000 F pour l'élève.
        let p = partage(
            exemple(true, true),
            VersionCourante::Terminee,
            60_000,
            33_000,
            0,
        );
        assert_eq!(p.du_etat, f(60_000));
        assert_eq!(colonne(&p, |e| e.part_etat), [0, 48_000, 0, 12_000, 0]);
        assert_eq!(p.reste_etat, Montant::ZERO);
        assert_eq!(p.reste_parent, f(75_000));
        assert_eq!(colonne(&p, |e| e.reste), [0, 0, 15_000, 24_000, 36_000]);
        assert_eq!(p.a_rembourser, Montant::ZERO);
    }

    /// Essai 4 bis du §12 : école sans cantine.
    #[test]
    fn essai_4_bis_ecole_sans_cantine() {
        let p = partage(exemple(false, true), en_cours(120_000, 0), 0, 0, 0);
        assert_eq!(p.trop_paye, f(63_000));
        assert_eq!(p.trop_paye_reporte, Montant::ZERO);
        assert_eq!(p.a_rembourser, f(63_000));
        assert_eq!(p.reste_parent, Montant::ZERO);
    }

    #[test]
    fn la_reprise_se_place_sur_la_part_du_parent_apres_le_report() {
        // Coché, reprise de 50 000 F : le trop-payé (63 000 F) couvre la cantine, la reprise
        // n'a plus rien à porter ; tout est à rembourser.
        let p = partage(exemple(true, true), en_cours(120_000, 0), 0, 0, 50_000);
        assert_eq!(p.reprise_imputee, Montant::ZERO);
        assert_eq!(p.a_rembourser, f(83_000));
        // Remboursé 40 000 F : d'abord sur la reprise.
        let p = partage(exemple(true, true), en_cours(120_000, 0), 0, 40_000, 50_000);
        assert_eq!(p.rembourse_reprise, f(40_000));
        assert_eq!(p.rembourse_trop_paye, Montant::ZERO);
        assert_eq!(p.a_rembourser, f(43_000));
        assert!(p.parts_reprise().is_empty());
    }

    /// Le calcul d'avant cette spécification : reste = net − versé − reprise, la reprise
    /// remplissant les restes sans elle par `reprise::imputer`.
    fn calcul_d_avant(
        echeances: &[EcheanceAPartager],
        montant_reprise: Montant,
        rembourse: Montant,
    ) -> (Vec<Montant>, Vec<Montant>, Montant) {
        let restes: Vec<(Id<Echeance>, Montant)> = echeances
            .iter()
            .map(|e| (e.echeance_id, e.net.soustraire(e.paye_parent).unwrap()))
            .collect();
        let utilisable = montant_reprise
            .soustraire(rembourse)
            .unwrap()
            .max(Montant::ZERO);
        let parts = reprise::imputer(&restes, utilisable).unwrap();
        let chiffres = reprise::chiffres(montant_reprise, rembourse, &parts).unwrap();
        let u: Vec<Montant> = echeances
            .iter()
            .map(|e| {
                parts
                    .iter()
                    .filter(|p| p.echeance_id == e.echeance_id)
                    .fold(Montant::ZERO, |s, p| s.ajouter(p.montant).unwrap())
            })
            .collect();
        let reste = restes
            .iter()
            .zip(&u)
            .map(|((_, r), u)| r.soustraire(*u).unwrap())
            .collect();
        (u, reste, chiffres.a_rembourser)
    }

    fn nature(n: u8) -> NatureEcheance {
        match n % 3 {
            0 => Scolarite,
            1 => Inscription,
            _ => Autre,
        }
    }

    fn version(choix: u8, attendu: i64, abandon_pour_cent: i64) -> VersionCourante {
        match choix % 3 {
            0 => VersionCourante::Aucune,
            1 => en_cours(attendu, attendu * abandon_pour_cent / 100),
            _ => VersionCourante::Terminee,
        }
    }

    /// Le contrôle du §3.4, l'égalité des parts et l'absence de montant négatif.
    fn verifier(entree: &EntreePartage, p: &Partage) -> Result<(), TestCaseError> {
        let z = Montant::ZERO;
        let s = |v: &[i64]| v.iter().sum::<i64>();
        for e in &p.echeances {
            prop_assert_eq!(
                e.net.francs(),
                e.part_etat.francs() + e.renonce.francs() + e.part_parent.francs()
            );
            for m in [
                e.part_etat,
                e.renonce,
                e.part_parent,
                e.paye_etat,
                e.reste_etat,
                e.trop_paye,
                e.trop_paye_rembourse,
                e.rendu,
                e.compte,
                e.report,
                e.reprise,
                e.paye,
                e.reste,
            ] {
                prop_assert!(m >= z, "négatif : {:?}", e);
            }
            prop_assert_eq!(
                e.reste.francs(),
                e.part_parent.francs() - e.compte.francs() - e.report.francs() - e.reprise.francs()
            );
            prop_assert!(e.paye_etat <= e.part_etat);
            if matches!(entree.version, VersionCourante::EnCours { .. }) && e.debiteur.couverte() {
                prop_assert_eq!(e.part_parent, z);
            }
            if e.debiteur == DebiteurEcheance::Parent {
                prop_assert_eq!(e.part_parent, e.net);
            }
        }
        for m in [
            p.du_etat,
            p.complement,
            p.renonce,
            p.paye_complement,
            p.recu_en_plus,
            p.trop_paye_disponible,
            p.reprise_utilisable,
            p.a_rembourser,
            p.rembourse_en_trop,
        ] {
            prop_assert!(m >= z);
        }
        prop_assert!(p.paye_complement <= p.complement);
        prop_assert_eq!(s(&colonne(p, |e| e.part_etat)), p.du_etat.francs());
        // Somme des payés de l'État + complément payé + reçu en plus = S.
        prop_assert_eq!(
            p.paye_etat.francs() + p.paye_complement.francs() + p.recu_en_plus.francs(),
            entree.paye_etat.francs()
        );
        // Versements + avoirs + reprise − remboursé = c + t + u + à rembourser.
        prop_assert_eq!(p.rembourse_en_trop, z);
        let verses: i64 = entree
            .echeances
            .iter()
            .map(|e| e.paye_parent.francs())
            .sum();
        prop_assert_eq!(
            verses + entree.reprise.francs() - entree.rembourse.francs(),
            s(&colonne(p, |e| e.compte))
                + p.trop_paye_reporte.francs()
                + p.reprise_imputee.francs()
                + p.a_rembourser.francs()
        );
        Ok(())
    }

    proptest! {
        /// Essai 5 du §12 (propriété du partage), sur le calcul pur.
        #[test]
        fn essai_5_propriete_du_partage(
            lignes in prop::collection::vec((0u8..3, 0i64..150_000, 0i64..=100), 0..9),
            choix in 0u8..3,
            attendu in 0i64..400_000,
            abandon in 0i64..=100,
            s in 0i64..450_000,
            reprise_montant in 0i64..200_000,
            fraction_rembourse in 0i64..=100,
        ) {
            let echeances: Vec<EcheanceAPartager> = lignes
                .iter()
                .enumerate()
                .map(|(i, (n, net, pc))| echeance(i, nature(*n), *net, net * pc / 100))
                .collect();
            let verses: i64 = echeances.iter().map(|e| e.paye_parent.francs()).sum();
            // Le remboursé ne dépasse jamais l'argent du parent et la reprise
            // (`versement.deja_rembourse`).
            let b = (verses + reprise_montant) * fraction_rembourse / 100;
            let entree = EntreePartage {
                echeances,
                version: version(choix, attendu, abandon),
                paye_etat: f(s),
                rembourse: f(b),
                reprise: f(reprise_montant),
            };
            let p = partager(&entree);
            prop_assert!(p.is_ok());
            if let Ok(p) = p {
                verifier(&entree, &p)?;
            }
        }

        /// Essai 5, dernière clause : sans prise en charge (ni trop-payé, ni remboursé au-delà
        /// de la reprise), chiffres identiques à ceux d'avant (assertion 10).
        #[test]
        fn essai_5_sans_prise_en_charge_rien_ne_change(
            lignes in prop::collection::vec((0u8..3, 0i64..150_000, 0i64..=100), 0..9),
            reprise_montant in 0i64..300_000,
            fraction_rembourse in 0i64..=100,
            s in 0i64..100_000,
        ) {
            let echeances: Vec<EcheanceAPartager> = lignes
                .iter()
                .enumerate()
                .map(|(i, (n, net, pc))| echeance(i, nature(*n), *net, net * pc / 100))
                .collect();
            let b = reprise_montant * fraction_rembourse / 100;
            let (u, reste, a_rembourser) = calcul_d_avant(&echeances, f(reprise_montant), f(b));
            let entree = EntreePartage {
                echeances: echeances.clone(),
                version: VersionCourante::Aucune,
                // Un versement de l'État pour un élève jamais coché : reçu en plus, sans effet
                // sur le parent.
                paye_etat: f(s),
                rembourse: f(b),
                reprise: f(reprise_montant),
            };
            let p = partager(&entree);
            prop_assert!(p.is_ok());
            if let Ok(p) = p {
                verifier(&entree, &p)?;
                prop_assert_eq!(p.recu_en_plus, f(s));
                prop_assert_eq!(p.a_rembourser, a_rembourser);
                for ((e, a), (u, r)) in p.echeances.iter().zip(&echeances).zip(u.iter().zip(&reste)) {
                    prop_assert_eq!(e.net, a.net);
                    prop_assert_eq!(e.part_parent, a.net);
                    prop_assert_eq!(e.reprise, *u);
                    prop_assert_eq!(e.reste, *r);
                    prop_assert_eq!(e.paye, a.paye_parent.ajouter(*u).unwrap());
                    prop_assert_eq!(e.part_etat, Montant::ZERO);
                }
                let parts = p.parts_reprise();
                let restes: Vec<(Id<Echeance>, Montant)> = echeances
                    .iter()
                    .map(|e| (e.echeance_id, e.net.soustraire(e.paye_parent).unwrap()))
                    .collect();
                let utilisable = f(reprise_montant - b);
                prop_assert_eq!(parts, reprise::imputer(&restes, utilisable).unwrap());
            }
        }
    }
}
