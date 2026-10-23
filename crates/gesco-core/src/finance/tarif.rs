//! Grille de frais d'une année et plans de paiement (SPEC_PHASE_3 §3, blueprint §8.8). Règles
//! pures : une grille se vérifie, un plan se découpe en échéances, une grille se recopie d'une
//! année à l'autre. Aucune lecture de la scolarité : la grille ne connaît que des identifiants
//! de niveau et de série (§9.1 du blueprint).

use serde::{Deserialize, Serialize};

use crate::commun::{DateCivile, Id, Montant};
use crate::core::{AnneeScolaire, Etablissement, MachineAEtats};
use crate::scolarite::{Niveau, Serie};

/// Nombre maximal de plans de paiement d'une grille (T-12).
pub const PLANS_MAX: usize = 4;
/// Nombre maximal de tranches d'un plan : un mensuel de douze mois.
pub const TRANCHES_MAX: usize = 12;

/// État d'une grille de frais.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "snake_case")]
pub enum EtatGrille {
    /// En préparation : se modifie librement, ne facture rien.
    Brouillon,
    /// Sert aux factures des inscriptions validées.
    EnVigueur,
    /// Remplacée par une version plus récente ; garde les factures qu'elle a produites.
    Remplacee,
}

impl MachineAEtats for EtatGrille {
    const ETATS: &'static [Self] = &[Self::Brouillon, Self::EnVigueur, Self::Remplacee];
    const TRANSITIONS: &'static [(Self, Self)] = &[
        (Self::Brouillon, Self::EnVigueur),
        (Self::EnVigueur, Self::Remplacee),
    ];

    fn nom(&self) -> &'static str {
        match self {
            Self::Brouillon => "brouillon",
            Self::EnVigueur => "en_vigueur",
            Self::Remplacee => "remplacee",
        }
    }
}

/// Nature d'un frais (blueprint §8.8, T-11).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "snake_case")]
pub enum TypeFrais {
    /// Frais d'inscription, dus à l'inscription.
    Inscription,
    /// Scolarité, payée selon le plan choisi par le parent.
    Scolarite,
    /// Cantine.
    Cantine,
    /// Transport.
    Transport,
    /// Tenue.
    Tenue,
    /// Frais d'examen.
    Examen,
    /// Autre frais.
    Autre,
}

impl TypeFrais {
    /// Tous les types, dans l'ordre d'affichage.
    pub const TOUS: &'static [Self] = &[
        Self::Inscription,
        Self::Scolarite,
        Self::Cantine,
        Self::Transport,
        Self::Tenue,
        Self::Examen,
        Self::Autre,
    ];

    /// Nom persisté.
    pub const fn nom(&self) -> &'static str {
        match self {
            Self::Inscription => "inscription",
            Self::Scolarite => "scolarite",
            Self::Cantine => "cantine",
            Self::Transport => "transport",
            Self::Tenue => "tenue",
            Self::Examen => "examen",
            Self::Autre => "autre",
        }
    }

    /// Lecture d'un nom persisté.
    pub fn depuis_nom(nom: &str) -> Option<Self> {
        Self::TOUS.iter().copied().find(|t| t.nom() == nom)
    }
}

/// Quand un frais est dû.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "snake_case")]
pub enum Exigibilite {
    /// En une fois, à la date de l'inscription.
    UneFois,
    /// Selon les tranches du plan choisi par le parent.
    SelonPlan,
}

impl Exigibilite {
    /// Nom persisté.
    pub const fn nom(&self) -> &'static str {
        match self {
            Self::UneFois => "une_fois",
            Self::SelonPlan => "selon_plan",
        }
    }

    /// Lecture d'un nom persisté.
    pub fn depuis_nom(nom: &str) -> Option<Self> {
        [Self::UneFois, Self::SelonPlan]
            .into_iter()
            .find(|e| e.nom() == nom)
    }
}

/// Grille de frais d'une année.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
pub struct Grille {
    /// Identifiant technique.
    pub id: Id<Grille>,
    /// Établissement.
    pub etablissement_id: Id<Etablissement>,
    /// Année scolaire.
    pub annee_id: Id<AnneeScolaire>,
    /// Libellé : « Frais 2026-2027 ».
    pub libelle: String,
    /// Version dans l'année : 1, puis 2 si la première est remplacée.
    pub version: i64,
    /// État.
    pub etat: EtatGrille,
}

/// Ligne de frais d'une grille.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
pub struct LigneTarifaire {
    /// Identifiant technique.
    pub id: Id<LigneTarifaire>,
    /// Niveau concerné.
    pub niveau_id: Id<Niveau>,
    /// Série, au lycée ; absente : tout le niveau.
    pub serie_id: Option<Id<Serie>>,
    /// Nature du frais.
    pub type_frais: TypeFrais,
    /// Libellé imprimé sur la facture et le reçu.
    pub libelle: String,
    /// Montant annuel, en francs.
    pub montant: Montant,
    /// Faux pour un frais coché à l'inscription (cantine, transport…).
    pub obligatoire: bool,
    /// Quand le frais est dû.
    pub exigibilite: Exigibilite,
}

/// Tranche d'un plan de paiement.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
pub struct Tranche {
    /// Libellé : « 1re tranche », « Octobre ».
    pub libelle: String,
    /// Date d'échéance.
    pub echeance: DateCivile,
    /// Part du montant, en pourcentage entier.
    pub part: i64,
}

/// Plan de paiement que le parent peut choisir (T-12).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
pub struct PlanPaiement {
    /// Identifiant technique.
    pub id: Id<PlanPaiement>,
    /// Libellé : « Trois tranches ».
    pub libelle: String,
    /// Plan appliqué quand le parent n'en choisit pas.
    pub par_defaut: bool,
    /// Tranches, dans l'ordre des échéances.
    pub tranches: Vec<Tranche>,
}

/// Contenu d'une grille : ses lignes et ses plans.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
pub struct ContenuGrille {
    /// Lignes de frais.
    pub lignes: Vec<LigneTarifaire>,
    /// Plans de paiement.
    pub plans: Vec<PlanPaiement>,
}

/// Échéance calculée d'un frais, avant toute facture.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EcheanceCalculee {
    /// Libellé.
    pub libelle: String,
    /// Date.
    pub date: DateCivile,
    /// Montant dû.
    pub montant: Montant,
}

/// Vérifie une tranche, un plan, puis l'ensemble des plans d'une grille : de un à quatre plans,
/// un seul par défaut, des parts entières positives dont la somme vaut 100, des dates croissantes.
pub fn verifier_plans(plans: &[PlanPaiement]) -> Result<(), &'static str> {
    if plans.is_empty() || plans.len() > PLANS_MAX {
        return Err("frais.nombre_de_plans");
    }
    if plans.iter().filter(|p| p.par_defaut).count() != 1 {
        return Err("frais.plan_par_defaut");
    }
    for plan in plans {
        if plan.libelle.trim().is_empty() {
            return Err("frais.plan_sans_libelle");
        }
        if plan.tranches.is_empty() || plan.tranches.len() > TRANCHES_MAX {
            return Err("frais.nombre_de_tranches");
        }
        if plan
            .tranches
            .iter()
            .any(|t| t.part <= 0 || t.libelle.trim().is_empty())
        {
            return Err("frais.tranche_invalide");
        }
        if plan.tranches.iter().map(|t| t.part).sum::<i64>() != 100 {
            return Err("frais.parts_differentes_de_cent");
        }
        if plan.tranches.windows(2).any(|p| match p {
            [a, b] => a.echeance > b.echeance,
            _ => false,
        }) {
            return Err("frais.tranches_desordonnees");
        }
    }
    let mut libelles: Vec<String> = plans
        .iter()
        .map(|p| p.libelle.trim().to_lowercase())
        .collect();
    libelles.sort();
    libelles.dedup();
    if libelles.len() != plans.len() {
        return Err("frais.plan_en_double");
    }
    Ok(())
}

/// Vérifie les lignes : libellé, montant positif ou nul, et pas deux lignes du même type pour le
/// même niveau et la même série, ce qui facturerait deux fois.
pub fn verifier_lignes(lignes: &[LigneTarifaire]) -> Result<(), &'static str> {
    for (i, ligne) in lignes.iter().enumerate() {
        if ligne.libelle.trim().is_empty() {
            return Err("frais.ligne_sans_libelle");
        }
        if ligne.montant.est_negatif() {
            return Err("frais.montant_negatif");
        }
        if ligne.type_frais == TypeFrais::Scolarite && ligne.exigibilite != Exigibilite::SelonPlan {
            return Err("frais.scolarite_selon_plan");
        }
        if lignes.iter().skip(i + 1).any(|autre| {
            autre.niveau_id == ligne.niveau_id
                && autre.serie_id == ligne.serie_id
                && autre.type_frais == ligne.type_frais
                && autre
                    .libelle
                    .trim()
                    .eq_ignore_ascii_case(ligne.libelle.trim())
        }) {
            return Err("frais.ligne_en_double");
        }
    }
    Ok(())
}

/// Découpe un montant selon les tranches d'un plan : chaque tranche reçoit sa part entière, la
/// dernière le reste des divisions, de sorte que la somme égale exactement le montant.
pub fn echeances_du_plan(
    montant: Montant,
    plan: &PlanPaiement,
) -> Result<Vec<EcheanceCalculee>, crate::commun::ErreurMontant> {
    let mut echeances = Vec::with_capacity(plan.tranches.len());
    let mut reparti = Montant::ZERO;
    let derniere = plan.tranches.len().saturating_sub(1);
    for (i, tranche) in plan.tranches.iter().enumerate() {
        let part = if i == derniere {
            montant.soustraire(reparti)?
        } else {
            montant.pourcentage(tranche.part)?.quotient
        };
        reparti = reparti.ajouter(part)?;
        echeances.push(EcheanceCalculee {
            libelle: tranche.libelle.clone(),
            date: tranche.echeance,
            montant: part,
        });
    }
    Ok(echeances)
}

/// Le même jour, un an plus tard ; le 29 février devient le 28.
pub fn un_an_plus_tard(date: DateCivile) -> Option<DateCivile> {
    let annee = date.annee().checked_add(1)?;
    DateCivile::nouvelle(annee, date.mois(), date.jour())
        .or_else(|| DateCivile::nouvelle(annee, date.mois(), 28))
}

/// Jour du mois des échéances livrées, tant que l'école n'en a pas choisi un autre : le 5, en
/// début de mois (décision du porteur du projet du 22 octobre 2026).
pub const JOUR_ECHEANCE_DEFAUT: u8 = 5;

/// Bornes du jour d'échéance : du 1er au 28, qui existe dans tous les mois.
pub const JOURS_ECHEANCE: std::ops::RangeInclusive<u8> = 1..=28;

/// Plans livrés avec une grille neuve (SPEC_PHASE_3 §3.1) : comptant, trois tranches (par
/// défaut) et mensuel, datés depuis l'année de la rentrée, chaque échéance au jour du mois que
/// l'école a choisi. L'école les modifie ou les retire. Le noyau ne tire aucun identifiant
/// lui-même (R-03) : l'appelant les fournit.
pub fn plans_livres(
    annee_rentree: i32,
    jour: u8,
    mut nouvel_id: impl FnMut() -> Id<PlanPaiement>,
) -> Vec<PlanPaiement> {
    let jour = if JOURS_ECHEANCE.contains(&jour) {
        jour
    } else {
        JOUR_ECHEANCE_DEFAUT
    };
    let date = |annee: i32, mois: u8| DateCivile::nouvelle(annee, mois, jour);
    let tranche = |libelle: &str, echeance: Option<DateCivile>, part: i64| {
        echeance.map(|echeance| Tranche {
            libelle: libelle.to_owned(),
            echeance,
            part,
        })
    };
    let suivante = annee_rentree + 1;
    let mois = [
        ("Octobre", annee_rentree, 10),
        ("Novembre", annee_rentree, 11),
        ("Décembre", annee_rentree, 12),
        ("Janvier", suivante, 1),
        ("Février", suivante, 2),
        ("Mars", suivante, 3),
        ("Avril", suivante, 4),
        ("Mai", suivante, 5),
        ("Juin", suivante, 6),
    ];
    // Neuf mois : 11 % chacun, 12 % le dernier pour faire 100.
    let mensuel: Vec<Tranche> = mois
        .iter()
        .enumerate()
        .filter_map(|(i, (libelle, annee, m))| {
            tranche(libelle, date(*annee, *m), if i == 8 { 12 } else { 11 })
        })
        .collect();
    let mut plan = |libelle: &str, par_defaut: bool, tranches: Vec<Tranche>| PlanPaiement {
        id: nouvel_id(),
        libelle: libelle.to_owned(),
        par_defaut,
        tranches,
    };
    vec![
        plan(
            "Comptant",
            false,
            tranche("Paiement unique", date(annee_rentree, 10), 100)
                .into_iter()
                .collect(),
        ),
        plan(
            "Trois tranches",
            true,
            [
                tranche("1re tranche", date(annee_rentree, 10), 40),
                tranche("2e tranche", date(suivante, 1), 30),
                tranche("3e tranche", date(suivante, 4), 30),
            ]
            .into_iter()
            .flatten()
            .collect(),
        ),
        plan("Mensuel", false, mensuel),
    ]
}

#[cfg(test)]
#[allow(clippy::indexing_slicing)]
mod tests {
    use super::*;
    use crate::core::transition::verifier_exhaustivement;

    fn f(v: i64) -> Montant {
        Montant::depuis_francs(v)
    }

    fn id<T>(n: u128) -> Id<T> {
        Id::depuis_ulid(ulid::Ulid::from(n))
    }

    fn plans_livres(annee: i32) -> Vec<PlanPaiement> {
        let mut n = 100;
        super::plans_livres(annee, JOUR_ECHEANCE_DEFAUT, || {
            n += 1;
            id(n)
        })
    }

    fn jour(a: i32, m: u8, j: u8) -> DateCivile {
        DateCivile::nouvelle(a, m, j).unwrap_or_else(|| unreachable!())
    }

    fn ligne(niveau: Id<Niveau>, type_frais: TypeFrais, montant: i64) -> LigneTarifaire {
        LigneTarifaire {
            id: id(u128::from(montant.unsigned_abs()) + 1_000_000),
            niveau_id: niveau,
            serie_id: None,
            type_frais,
            libelle: type_frais.nom().to_owned(),
            montant: f(montant),
            obligatoire: true,
            exigibilite: if type_frais == TypeFrais::Scolarite {
                Exigibilite::SelonPlan
            } else {
                Exigibilite::UneFois
            },
        }
    }

    #[test]
    fn etats_de_la_grille() {
        verifier_exhaustivement::<EtatGrille>();
    }

    #[test]
    fn les_plans_livres_sont_valides() {
        let plans = plans_livres(2026);
        assert_eq!(verifier_plans(&plans), Ok(()));
        assert_eq!(plans.len(), 3);
        assert_eq!(plans[1].libelle, "Trois tranches");
        assert!(plans[1].par_defaut);
        assert_eq!(plans[1].tranches[1].echeance, jour(2027, 1, 5));
        assert_eq!(plans[2].tranches.len(), 9);
        assert_eq!(plans[2].tranches[8].echeance, jour(2027, 6, 5));
    }

    #[test]
    fn les_echeances_livrees_tombent_au_jour_choisi() {
        let mut n = 0;
        let le_28 = super::plans_livres(2026, 28, || {
            n += 1;
            id(n)
        });
        assert!(
            le_28
                .iter()
                .flat_map(|p| &p.tranches)
                .all(|t| t.echeance.jour() == 28)
        );
        // Hors du 1er au 28, le jour livré.
        let hors = super::plans_livres(2026, 31, || {
            n += 1;
            id(n)
        });
        assert_eq!(hors[2].tranches[0].echeance, jour(2026, 10, 5));
    }

    #[test]
    fn un_plan_bancal_est_refuse() {
        let mut plans = plans_livres(2026);
        plans[0].par_defaut = true;
        assert_eq!(verifier_plans(&plans), Err("frais.plan_par_defaut"));
        let mut plans = plans_livres(2026);
        plans[1].tranches[0].part = 50;
        assert_eq!(
            verifier_plans(&plans),
            Err("frais.parts_differentes_de_cent")
        );
        let mut plans = plans_livres(2026);
        plans[1].tranches.swap(0, 1);
        assert_eq!(verifier_plans(&plans), Err("frais.tranches_desordonnees"));
        let mut plans = plans_livres(2026);
        plans[2].libelle = "comptant".to_owned();
        assert_eq!(verifier_plans(&plans), Err("frais.plan_en_double"));
        let mut cinq = plans_livres(2026);
        cinq.extend(plans_livres(2026).into_iter().filter(|p| !p.par_defaut));
        assert_eq!(verifier_plans(&cinq), Err("frais.nombre_de_plans"));
        assert_eq!(verifier_plans(&[]), Err("frais.nombre_de_plans"));
    }

    #[test]
    fn un_plan_decoupe_le_montant_sans_perdre_un_franc() {
        let plans = plans_livres(2026);
        let trois = echeances_du_plan(f(125_005), &plans[1]).unwrap_or_default();
        assert_eq!(
            trois.iter().map(|e| e.montant).collect::<Vec<_>>(),
            vec![f(50_002), f(37_501), f(37_502)]
        );
        let mensuel = echeances_du_plan(f(150_000), &plans[2]).unwrap_or_default();
        assert_eq!(mensuel.len(), 9);
        assert_eq!(
            mensuel.iter().map(|e| e.montant.francs()).sum::<i64>(),
            150_000
        );
        assert_eq!(mensuel[0].montant, f(16_500));
        assert_eq!(mensuel[8].montant, f(18_000));
        assert_eq!(
            echeances_du_plan(Montant::ZERO, &plans[0]).unwrap_or_default()[0].montant,
            Montant::ZERO
        );
    }

    #[test]
    fn des_lignes_bancales_sont_refusees() {
        let sept: Id<Niveau> = id(7);
        let bonnes = [
            ligne(sept, TypeFrais::Inscription, 10_000),
            ligne(sept, TypeFrais::Scolarite, 150_000),
            ligne(sept, TypeFrais::Cantine, 0),
        ];
        assert_eq!(verifier_lignes(&bonnes), Ok(()));
        let mut negative = bonnes.clone();
        negative[0].montant = f(-1);
        assert_eq!(verifier_lignes(&negative), Err("frais.montant_negatif"));
        let mut double = bonnes.to_vec();
        double.push(ligne(sept, TypeFrais::Scolarite, 1));
        assert_eq!(verifier_lignes(&double), Err("frais.ligne_en_double"));
        let mut une_fois = bonnes.clone();
        une_fois[1].exigibilite = Exigibilite::UneFois;
        assert_eq!(
            verifier_lignes(&une_fois),
            Err("frais.scolarite_selon_plan")
        );
        let mut sans = bonnes;
        sans[2].libelle = "  ".to_owned();
        assert_eq!(verifier_lignes(&sans), Err("frais.ligne_sans_libelle"));
    }

    #[test]
    fn une_date_se_reporte_d_un_an() {
        assert_eq!(
            un_an_plus_tard(jour(2026, 10, 15)),
            Some(jour(2027, 10, 15))
        );
        assert_eq!(un_an_plus_tard(jour(2028, 2, 29)), Some(jour(2029, 2, 28)));
    }
}
