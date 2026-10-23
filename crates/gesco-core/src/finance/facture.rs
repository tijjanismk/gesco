//! Facture et échéancier d'une inscription (SPEC_PHASE_3 §4) : calcul pur, à partir de la
//! grille en vigueur, du plan choisi par le parent, des frais facultatifs cochés et de la date
//! d'inscription. Aucun montant n'est perdu ni inventé : chaque échéance est une part entière,
//! le reste d'une division va à la dernière (R-11).

use serde::{Deserialize, Serialize};

use crate::commun::{DateCivile, ErreurMontant, Id, ModeArrondi, Montant};
use crate::core::{AnneeScolaire, Etablissement, MachineAEtats};
use crate::finance::tarif::{
    EcheanceCalculee, Exigibilite, Grille, LigneTarifaire, PlanPaiement, TypeFrais,
    echeances_du_plan,
};
use crate::scolarite::{Inscription, Niveau, Serie};

/// Ce qu'on facture à un élève arrivé en cours d'année (Q-26, paramètre
/// `facturation.arrivee_en_cours`) ; chaque école choisit sa règle, et une autre peut être
/// choisie pour un élève à la validation (porteur du projet, 22 octobre 2026).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "snake_case")]
pub enum ArriveeEnCours {
    /// Frais d'inscription entiers ; les tranches du mois d'arrivée et des mois suivants, même
    /// si leur jour est passé (défaut) : inscrit le 14 octobre, l'élève paie la tranche du
    /// 5 octobre.
    EcheancesDuMois,
    /// Frais d'inscription entiers ; seules les tranches dont la date n'est pas passée.
    EcheancesRestantes,
    /// Au prorata des mois restants, réparti sur les tranches non échues.
    ProrataMensuel,
    /// Tout est dû.
    PleinTarif,
}

impl ArriveeEnCours {
    /// Nom persisté dans le paramètre.
    pub const fn nom(&self) -> &'static str {
        match self {
            Self::EcheancesDuMois => "echeances_du_mois",
            Self::EcheancesRestantes => "echeances_restantes",
            Self::ProrataMensuel => "prorata_mensuel",
            Self::PleinTarif => "plein_tarif",
        }
    }

    /// Lecture du paramètre.
    pub fn depuis_nom(nom: &str) -> Option<Self> {
        [
            Self::EcheancesDuMois,
            Self::EcheancesRestantes,
            Self::ProrataMensuel,
            Self::PleinTarif,
        ]
        .into_iter()
        .find(|r| r.nom() == nom)
    }

    /// Règle livrée, tant que l'école n'en a pas choisi une autre.
    pub const DEFAUT: Self = Self::EcheancesDuMois;
}

/// État d'une facture (§10.2 du blueprint, amendé par Q-51 : pas de brouillon). Réglée et soldée
/// se déduisent des imputations (étape 6) ; elles restent recalculables (R-20).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "snake_case")]
pub enum StatutFacture {
    /// Émise, rien de payé.
    Emise,
    /// Une partie est payée.
    PartiellementReglee,
    /// Plus rien n'est dû.
    Soldee,
    /// Annulée : inscription abandonnée sans paiement.
    Annulee,
}

impl MachineAEtats for StatutFacture {
    const ETATS: &'static [Self] = &[
        Self::Emise,
        Self::PartiellementReglee,
        Self::Soldee,
        Self::Annulee,
    ];
    const TRANSITIONS: &'static [(Self, Self)] = &[
        (Self::Emise, Self::PartiellementReglee),
        (Self::Emise, Self::Soldee),
        (Self::PartiellementReglee, Self::Soldee),
        // L'annulation d'un versement fait revenir en arrière (étape 7).
        (Self::PartiellementReglee, Self::Emise),
        (Self::Soldee, Self::PartiellementReglee),
        (Self::Soldee, Self::Emise),
        (Self::Emise, Self::Annulee),
    ];

    fn nom(&self) -> &'static str {
        match self {
            Self::Emise => "emise",
            Self::PartiellementReglee => "partiellement_reglee",
            Self::Soldee => "soldee",
            Self::Annulee => "annulee",
        }
    }
}

/// État d'une échéance.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "snake_case")]
pub enum StatutEcheance {
    /// Due.
    Due,
    /// Annulée par contre-passation (changement de plan, abandon) ; jamais supprimée (R-14).
    Annulee,
}

impl StatutEcheance {
    /// Nom persisté.
    pub const fn nom(&self) -> &'static str {
        match self {
            Self::Due => "due",
            Self::Annulee => "annulee",
        }
    }

    /// Lecture d'un nom persisté.
    pub fn depuis_nom(nom: &str) -> Option<Self> {
        [Self::Due, Self::Annulee]
            .into_iter()
            .find(|s| s.nom() == nom)
    }
}

/// Facture d'une inscription.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
pub struct Facture {
    /// Identifiant technique.
    pub id: Id<Facture>,
    /// Établissement.
    pub etablissement_id: Id<Etablissement>,
    /// Inscription facturée.
    pub inscription_id: Id<Inscription>,
    /// Année.
    pub annee_id: Id<AnneeScolaire>,
    /// Grille qui a servi.
    pub grille_id: Id<Grille>,
    /// Plan choisi par le parent.
    pub plan_paiement_id: Id<PlanPaiement>,
    /// Numéro sans trou : `FAC-2026-000123` (R-46).
    pub numero: String,
    /// Date d'émission.
    pub date_emission: DateCivile,
    /// État.
    pub statut: StatutFacture,
    /// Total des échéances dues.
    pub total: Montant,
    /// Motif d'annulation, vide sinon.
    pub motif_annulation: String,
}

/// Ligne d'une facture : un frais de la grille.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
pub struct LigneFacture {
    /// Identifiant technique.
    pub id: Id<LigneFacture>,
    /// Ligne de la grille d'où elle vient.
    pub ligne_tarifaire_id: Id<LigneTarifaire>,
    /// Nature du frais.
    pub type_frais: TypeFrais,
    /// Libellé.
    pub libelle: String,
    /// Montant facturé : somme de ses échéances dues.
    pub montant: Montant,
}

/// Échéance d'une facture.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
pub struct Echeance {
    /// Identifiant technique.
    pub id: Id<Echeance>,
    /// Ligne de facture.
    pub ligne_facture_id: Id<LigneFacture>,
    /// Libellé : « Scolarité · 1re tranche ».
    pub libelle: String,
    /// Date d'échéance.
    pub date: DateCivile,
    /// Montant dû.
    pub montant: Montant,
    /// État.
    pub statut: StatutEcheance,
}

/// Ce que la scolarité dit d'une inscription à la finance (§9.1 du blueprint) : jamais plus.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ContexteTarifaire {
    /// Inscription.
    pub inscription_id: Id<Inscription>,
    /// Année.
    pub annee_id: Id<AnneeScolaire>,
    /// Niveau de la classe.
    pub niveau_id: Id<Niveau>,
    /// Série de la classe.
    pub serie_id: Option<Id<Serie>>,
    /// Date d'inscription.
    pub date_inscription: DateCivile,
}

/// Paramètres du calcul.
#[derive(Debug, Clone, Copy)]
pub struct Bornes {
    /// Premier jour de l'année.
    pub debut: DateCivile,
    /// Dernier jour de l'année.
    pub fin: DateCivile,
    /// Règle d'arrivée en cours d'année.
    pub arrivee: ArriveeEnCours,
    /// Multiple d'arrondi des montants calculés.
    pub multiple: i64,
}

/// Ligne calculée, avant enregistrement.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LigneCalculee {
    /// Ligne de la grille.
    pub ligne: LigneTarifaire,
    /// Échéances non nulles.
    pub echeances: Vec<EcheanceCalculee>,
}

impl LigneCalculee {
    /// Somme des échéances.
    pub fn montant(&self) -> Result<Montant, ErreurMontant> {
        self.echeances
            .iter()
            .try_fold(Montant::ZERO, |s, e| s.ajouter(e.montant))
    }
}

/// Lignes de la grille qui s'appliquent à une classe : celles du niveau, sans série ou de sa série.
pub fn lignes_applicables(
    lignes: &[LigneTarifaire],
    niveau_id: Id<Niveau>,
    serie_id: Option<Id<Serie>>,
) -> Vec<&LigneTarifaire> {
    lignes
        .iter()
        .filter(|l| l.niveau_id == niveau_id && (l.serie_id.is_none() || l.serie_id == serie_id))
        .collect()
}

fn mois(date: DateCivile) -> i64 {
    i64::from(date.annee()) * 12 + i64::from(date.mois())
}

/// Mois restants de l'année à la date d'inscription, mois d'inscription compris, et mois de
/// l'année : `(restants, total)`.
pub fn mois_restants(date: DateCivile, debut: DateCivile, fin: DateCivile) -> (i64, i64) {
    let total = (mois(fin) - mois(debut) + 1).max(1);
    let restants = (mois(fin) - mois(date.max(debut)) + 1).clamp(0, total);
    (restants, total)
}

/// Échéances d'un frais payé selon le plan, d'après la règle d'arrivée en cours d'année.
fn selon_plan(
    ligne: &LigneTarifaire,
    plan: &PlanPaiement,
    date: DateCivile,
    bornes: &Bornes,
) -> Result<Vec<EcheanceCalculee>, ErreurMontant> {
    let completes = echeances_du_plan(ligne.montant, plan)?;
    let a_venir = |e: &&EcheanceCalculee| e.date >= date;
    Ok(match bornes.arrivee {
        ArriveeEnCours::PleinTarif => completes,
        ArriveeEnCours::EcheancesDuMois => completes
            .into_iter()
            .filter(|e| mois(e.date) >= mois(date))
            .collect(),
        ArriveeEnCours::EcheancesRestantes => completes.iter().filter(a_venir).cloned().collect(),
        ArriveeEnCours::ProrataMensuel => {
            let (restants, total) = mois_restants(date, bornes.debut, bornes.fin);
            if restants == total {
                return Ok(completes);
            }
            let prorata = ligne
                .montant
                .part(restants, total)?
                .quotient
                .arrondir_commercial(bornes.multiple, ModeArrondi::AuPlusProche)?
                .min(ligne.montant);
            let tranches: Vec<(EcheanceCalculee, i64)> = plan
                .tranches
                .iter()
                .zip(completes)
                .filter(|(_, e)| e.date >= date)
                .map(|(t, e)| (e, t.part))
                .collect();
            if tranches.is_empty() {
                return Ok(vec![EcheanceCalculee {
                    libelle: "Solde".to_owned(),
                    date,
                    montant: prorata,
                }]);
            }
            let somme_parts: i64 = tranches.iter().map(|(_, p)| p).sum();
            let derniere = tranches.len() - 1;
            let mut reparti = Montant::ZERO;
            let mut sortie = Vec::with_capacity(tranches.len());
            for (i, (e, part)) in tranches.into_iter().enumerate() {
                let montant = if i == derniere {
                    prorata.soustraire(reparti)?
                } else {
                    prorata.part(part, somme_parts)?.quotient
                };
                reparti = reparti.ajouter(montant)?;
                sortie.push(EcheanceCalculee { montant, ..e });
            }
            sortie
        }
    })
}

/// Calcule les lignes et les échéances d'une facture. Refuse une classe dont le niveau n'a
/// aucune ligne dans la grille (Q-49) et un frais facultatif qui ne la concerne pas. Les
/// échéances nulles sont omises : un frais à zéro ne figure jamais dans les impayés.
pub fn calculer(
    lignes: &[LigneTarifaire],
    contexte: &ContexteTarifaire,
    options: &[Id<LigneTarifaire>],
    plan: &PlanPaiement,
    bornes: &Bornes,
) -> Result<Vec<LigneCalculee>, &'static str> {
    let applicables = lignes_applicables(lignes, contexte.niveau_id, contexte.serie_id);
    if applicables.is_empty() {
        return Err("facture.grille_absente");
    }
    if options
        .iter()
        .any(|o| !applicables.iter().any(|l| l.id == *o && !l.obligatoire))
    {
        return Err("facture.option_inconnue");
    }
    let date = contexte.date_inscription;
    let mut retenues: Vec<&LigneTarifaire> = applicables
        .into_iter()
        .filter(|l| l.obligatoire || options.contains(&l.id))
        .collect();
    // Les frais d'inscription d'abord : à date égale, un versement les règle en premier, quel que
    // soit l'ordre des lignes dans la grille.
    retenues.sort_by_key(|l| l.type_frais != TypeFrais::Inscription);
    retenues
        .into_iter()
        .map(|ligne| {
            let echeances = match ligne.exigibilite {
                Exigibilite::UneFois => vec![EcheanceCalculee {
                    libelle: ligne.libelle.clone(),
                    date,
                    montant: ligne.montant,
                }],
                Exigibilite::SelonPlan => selon_plan(ligne, plan, date, bornes)
                    .map_err(|_| "facture.calcul_impossible")?
                    .into_iter()
                    .map(|e| EcheanceCalculee {
                        libelle: format!("{} · {}", ligne.libelle, e.libelle),
                        ..e
                    })
                    .collect(),
            };
            Ok(LigneCalculee {
                ligne: ligne.clone(),
                echeances: echeances
                    .into_iter()
                    .filter(|e| !e.montant.est_nul())
                    .collect(),
            })
        })
        .collect()
}

#[cfg(test)]
#[allow(clippy::indexing_slicing)]
mod tests {
    use super::*;
    use crate::core::transition::verifier_exhaustivement;
    use crate::finance::tarif::plans_livres;

    fn id<T>(n: u128) -> Id<T> {
        Id::depuis_ulid(ulid::Ulid::from(n))
    }

    fn f(v: i64) -> Montant {
        Montant::depuis_francs(v)
    }

    fn jour(a: i32, m: u8, j: u8) -> DateCivile {
        DateCivile::nouvelle(a, m, j).unwrap_or_else(|| unreachable!())
    }

    fn ligne(n: u128, type_frais: TypeFrais, montant: i64, obligatoire: bool) -> LigneTarifaire {
        LigneTarifaire {
            id: id(n),
            niveau_id: id(7),
            serie_id: None,
            type_frais,
            libelle: match type_frais {
                TypeFrais::Inscription => "Inscription",
                TypeFrais::Scolarite => "Scolarité",
                TypeFrais::Cantine => "Cantine",
                _ => "Autre",
            }
            .to_owned(),
            montant: f(montant),
            obligatoire,
            exigibilite: if type_frais == TypeFrais::Scolarite || type_frais == TypeFrais::Cantine {
                Exigibilite::SelonPlan
            } else {
                Exigibilite::UneFois
            },
        }
    }

    fn grille() -> Vec<LigneTarifaire> {
        vec![
            ligne(1, TypeFrais::Inscription, 10_000, true),
            ligne(2, TypeFrais::Scolarite, 150_000, true),
            ligne(3, TypeFrais::Cantine, 45_000, false),
        ]
    }

    fn trois_tranches() -> PlanPaiement {
        let mut n = 100;
        plans_livres(2026, crate::finance::tarif::JOUR_ECHEANCE_DEFAUT, || {
            n += 1;
            id(n)
        })
        .into_iter()
        .find(|p| p.par_defaut)
        .unwrap_or_else(|| unreachable!())
    }

    fn contexte(date: DateCivile) -> ContexteTarifaire {
        ContexteTarifaire {
            inscription_id: id(50),
            annee_id: id(60),
            niveau_id: id(7),
            serie_id: None,
            date_inscription: date,
        }
    }

    fn bornes(arrivee: ArriveeEnCours) -> Bornes {
        Bornes {
            debut: jour(2026, 10, 1),
            fin: jour(2027, 6, 30),
            arrivee,
            multiple: 5,
        }
    }

    fn total(calcul: &[LigneCalculee]) -> i64 {
        calcul
            .iter()
            .map(|l| l.montant().map(|m| m.francs()).unwrap_or(-1))
            .sum()
    }

    #[test]
    fn etats_de_la_facture() {
        verifier_exhaustivement::<StatutFacture>();
    }

    #[test]
    fn inscription_a_la_rentree_tout_selon_le_plan() {
        let calcul = calculer(
            &grille(),
            &contexte(jour(2026, 9, 20)),
            &[],
            &trois_tranches(),
            &bornes(ArriveeEnCours::EcheancesRestantes),
        )
        .unwrap_or_default();
        assert_eq!(calcul.len(), 2, "la cantine n'est pas cochée");
        assert_eq!(calcul[0].echeances[0].date, jour(2026, 9, 20));
        assert_eq!(calcul[0].echeances[0].montant, f(10_000));
        let scolarite = &calcul[1].echeances;
        assert_eq!(
            scolarite.iter().map(|e| e.montant).collect::<Vec<_>>(),
            vec![f(60_000), f(45_000), f(45_000)]
        );
        assert_eq!(scolarite[1].libelle, "Scolarité · 2e tranche");
        assert_eq!(total(&calcul), 160_000);
    }

    #[test]
    fn la_tranche_du_mois_d_arrivee_compte_selon_la_regle_de_l_ecole() {
        // Inscrit le 14 octobre, la 1re tranche datée du 5 octobre.
        let scolarite = |regle| {
            calculer(
                &grille(),
                &contexte(jour(2026, 10, 14)),
                &[],
                &trois_tranches(),
                &bornes(regle),
            )
            .unwrap_or_default()
            .get(1)
            .map(|l| l.echeances.iter().map(|e| e.montant).collect::<Vec<_>>())
            .unwrap_or_default()
        };
        assert_eq!(
            scolarite(ArriveeEnCours::EcheancesDuMois),
            vec![f(60_000), f(45_000), f(45_000)]
        );
        assert_eq!(
            scolarite(ArriveeEnCours::EcheancesRestantes),
            vec![f(45_000), f(45_000)]
        );
    }

    #[test]
    fn un_frais_facultatif_coche_s_ajoute() {
        let calcul = calculer(
            &grille(),
            &contexte(jour(2026, 9, 20)),
            &[id(3)],
            &trois_tranches(),
            &bornes(ArriveeEnCours::PleinTarif),
        )
        .unwrap_or_default();
        assert_eq!(calcul.len(), 3);
        assert_eq!(total(&calcul), 205_000);
        assert_eq!(
            calculer(
                &grille(),
                &contexte(jour(2026, 9, 20)),
                &[id(2)],
                &trois_tranches(),
                &bornes(ArriveeEnCours::PleinTarif),
            ),
            Err("facture.option_inconnue"),
            "un frais obligatoire ne se coche pas"
        );
    }

    #[test]
    fn arrivee_en_cours_d_annee_selon_les_trois_regles() {
        // Inscrit le 1er décembre : la tranche d'octobre est échue.
        let date = jour(2026, 12, 1);
        let restantes = calculer(
            &grille(),
            &contexte(date),
            &[],
            &trois_tranches(),
            &bornes(ArriveeEnCours::EcheancesRestantes),
        )
        .unwrap_or_default();
        assert_eq!(
            restantes[1]
                .echeances
                .iter()
                .map(|e| e.montant)
                .collect::<Vec<_>>(),
            vec![f(45_000), f(45_000)]
        );
        assert_eq!(
            total(&restantes),
            100_000,
            "inscription entière + deux tranches"
        );

        // Prorata : décembre à juin, 7 mois sur 9 ; 116 666 arrondi à 116 665, réparti 30/30.
        let prorata = calculer(
            &grille(),
            &contexte(date),
            &[],
            &trois_tranches(),
            &bornes(ArriveeEnCours::ProrataMensuel),
        )
        .unwrap_or_default();
        assert_eq!(
            prorata[1]
                .echeances
                .iter()
                .map(|e| e.montant)
                .collect::<Vec<_>>(),
            vec![f(58_332), f(58_333)]
        );
        assert_eq!(prorata[1].montant(), Ok(f(116_665)));

        let plein = calculer(
            &grille(),
            &contexte(date),
            &[],
            &trois_tranches(),
            &bornes(ArriveeEnCours::PleinTarif),
        )
        .unwrap_or_default();
        assert_eq!(total(&plein), 160_000);
        assert_eq!(
            plein[1].echeances[0].date,
            jour(2026, 10, 5),
            "échue, donc due tout de suite"
        );
    }

    #[test]
    fn apres_la_derniere_tranche_le_prorata_fait_un_solde() {
        let date = jour(2027, 5, 2);
        let prorata = calculer(
            &grille(),
            &contexte(date),
            &[],
            &trois_tranches(),
            &bornes(ArriveeEnCours::ProrataMensuel),
        )
        .unwrap_or_default();
        // Mai et juin : 2 mois sur 9, 33 333 arrondi à 33 335.
        assert_eq!(prorata[1].echeances.len(), 1);
        assert_eq!(prorata[1].echeances[0].libelle, "Scolarité · Solde");
        assert_eq!(prorata[1].echeances[0].montant, f(33_335));
        let restantes = calculer(
            &grille(),
            &contexte(date),
            &[],
            &trois_tranches(),
            &bornes(ArriveeEnCours::EcheancesRestantes),
        )
        .unwrap_or_default();
        assert_eq!(
            restantes[1].echeances,
            Vec::new(),
            "toutes les tranches sont échues"
        );
    }

    #[test]
    fn un_niveau_absent_de_la_grille_est_refuse_un_tarif_nul_ne_l_est_pas() {
        let autre = ContexteTarifaire {
            niveau_id: id(8),
            ..contexte(jour(2026, 9, 20))
        };
        assert_eq!(
            calculer(
                &grille(),
                &autre,
                &[],
                &trois_tranches(),
                &bornes(ArriveeEnCours::PleinTarif)
            ),
            Err("facture.grille_absente")
        );
        let gratuit = vec![LigneTarifaire {
            niveau_id: id(8),
            ..ligne(9, TypeFrais::Scolarite, 0, true)
        }];
        let calcul = calculer(
            &gratuit,
            &autre,
            &[],
            &trois_tranches(),
            &bornes(ArriveeEnCours::PleinTarif),
        )
        .unwrap_or_default();
        assert_eq!(calcul.len(), 1);
        assert!(calcul[0].echeances.is_empty());
    }

    #[test]
    fn une_ligne_de_serie_ne_vaut_que_pour_sa_serie() {
        let serie_s: Id<Serie> = id(70);
        let mut lignes = grille();
        lignes.push(LigneTarifaire {
            serie_id: Some(serie_s),
            ..ligne(4, TypeFrais::Examen, 5_000, true)
        });
        let sans = calculer(
            &lignes,
            &contexte(jour(2026, 9, 20)),
            &[],
            &trois_tranches(),
            &bornes(ArriveeEnCours::PleinTarif),
        )
        .unwrap_or_default();
        assert_eq!(sans.len(), 2);
        let avec = calculer(
            &lignes,
            &ContexteTarifaire {
                serie_id: Some(serie_s),
                ..contexte(jour(2026, 9, 20))
            },
            &[],
            &trois_tranches(),
            &bornes(ArriveeEnCours::PleinTarif),
        )
        .unwrap_or_default();
        assert_eq!(avec.len(), 3);
    }

    #[test]
    fn les_frais_d_inscription_passent_en_premier() {
        // La grille de l'école met la scolarité avant l'inscription ; l'échéance due le jour de
        // l'inscription reste la première réglée.
        let mut lignes = grille();
        lignes.reverse();
        let calcul = calculer(
            &lignes,
            &contexte(jour(2026, 10, 15)),
            &[],
            &trois_tranches(),
            &bornes(ArriveeEnCours::PleinTarif),
        )
        .unwrap_or_default();
        assert_eq!(
            calcul.first().map(|c| c.ligne.type_frais),
            Some(TypeFrais::Inscription)
        );
    }

    #[test]
    fn mois_restants_de_l_annee() {
        let (debut, fin) = (jour(2026, 10, 1), jour(2027, 6, 30));
        assert_eq!(mois_restants(jour(2026, 9, 1), debut, fin), (9, 9));
        assert_eq!(mois_restants(jour(2026, 12, 31), debut, fin), (7, 9));
        assert_eq!(mois_restants(jour(2027, 6, 30), debut, fin), (1, 9));
        assert_eq!(mois_restants(jour(2027, 8, 1), debut, fin), (0, 9));
    }
}
