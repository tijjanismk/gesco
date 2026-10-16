//! Éléments de paie de l'école (SPEC_PHASE_7 §3) : gains, retenues salariales, cotisations
//! patronales, chacun avec son calcul daté. Aucune règle fiscale ou sociale n'est écrite ici
//! (§8.7 du blueprint) : les retenues du Mali sont livrées comme des données, inactives, avec des
//! taux proposés que l'école vérifie (Q-79).

use serde::{Deserialize, Serialize};

use crate::commun::{Id, Montant};
use crate::core::Etablissement;
use crate::rh::{MembrePersonnel, Mois, NatureEngagement};

/// 100 %, en centièmes de pour cent : un taux de 3,06 % s'écrit 306.
pub const TAUX_MAX: i64 = 10_000;

/// Lit un taux saisi en pour cent : `3,06`, `3.6`, `37`, `3,06 %`. Deux décimales au plus, de 0
/// à 100 % ; rend des centièmes de pour cent. L'erreur est la clé du message.
pub fn lire_taux(texte: &str) -> Result<i64, &'static str> {
    let propre: String = texte
        .trim()
        .trim_end_matches('%')
        .chars()
        .filter(|c| !c.is_whitespace())
        .map(|c| if c == ',' { '.' } else { c })
        .collect();
    let (entier, decimales) = propre.split_once('.').unwrap_or((propre.as_str(), ""));
    if entier.is_empty()
        || decimales.len() > 2
        || !entier.chars().all(|c| c.is_ascii_digit())
        || !decimales.chars().all(|c| c.is_ascii_digit())
    {
        return Err("element.taux_invalide");
    }
    let entier: i64 = entier.parse().map_err(|_| "element.taux_invalide")?;
    let decimales: i64 = format!("{decimales:0<2}")
        .parse()
        .map_err(|_| "element.taux_invalide")?;
    let taux = entier
        .checked_mul(100)
        .and_then(|t| t.checked_add(decimales))
        .ok_or("element.taux_invalide")?;
    if taux > TAUX_MAX {
        return Err("element.taux_invalide");
    }
    Ok(taux)
}

/// Écrit un taux comme on le saisit : `3,06 %`, `37 %`.
pub fn ecrire_taux(taux: i64) -> String {
    let (entier, decimales) = (taux / 100, taux % 100);
    if decimales == 0 {
        format!("{entier}\u{a0}%")
    } else if decimales % 10 == 0 {
        format!("{entier},{}\u{a0}%", decimales / 10)
    } else {
        format!("{entier},{decimales:02}\u{a0}%")
    }
}

/// Nature d'un élément de paie.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "snake_case")]
pub enum NatureElement {
    /// S'ajoute au brut : salaire, prime, indemnité, heures, rappel.
    Gain,
    /// Se retire du brut, à la charge du membre : cotisation salariale, impôt, retenue.
    Retenue,
    /// À la charge de l'école, calculée pour les déclarations ; ne touche jamais le net.
    CotisationPatronale,
}

impl NatureElement {
    /// Les trois natures.
    pub const TOUTES: [Self; 3] = [Self::Gain, Self::Retenue, Self::CotisationPatronale];

    /// Nom persisté.
    pub const fn nom(&self) -> &'static str {
        match self {
            Self::Gain => "gain",
            Self::Retenue => "retenue",
            Self::CotisationPatronale => "cotisation_patronale",
        }
    }

    /// Lecture d'un nom persisté.
    pub fn depuis_nom(nom: &str) -> Option<Self> {
        Self::TOUTES.into_iter().find(|n| n.nom() == nom)
    }
}

/// TrancheBareme d'un barème progressif.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
pub struct TrancheBareme {
    /// Borne haute, comprise ; absente pour la dernière tranche.
    pub jusqua: Option<Montant>,
    /// Taux de la tranche, en centièmes de pour cent.
    pub taux: i64,
}

/// Réduction pour charges de famille d'un impôt, en pourcentage de l'impôt.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
pub struct ReductionFamille {
    /// Pour un membre marié, en centièmes de pour cent.
    pub marie: i64,
    /// Par enfant à charge, en centièmes de pour cent.
    pub par_enfant: i64,
    /// Plafond de la réduction, en centièmes de pour cent.
    pub plafond: i64,
}

/// Calcul d'un élément, dans une version datée.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(tag = "mode", rename_all = "snake_case")]
pub enum Calcul {
    /// Un montant par mois.
    Fixe {
        /// Montant mensuel.
        montant: Montant,
    },
    /// Un pourcentage de l'assiette, plafonnée ou non.
    Taux {
        /// Taux, en centièmes de pour cent.
        taux: i64,
        /// Plafond de l'assiette, par mois.
        plafond: Option<Montant>,
    },
    /// Un barème progressif de l'assiette, mensuel ou annuel, avec ses réductions.
    Bareme {
        /// Tranches, par bornes croissantes, la dernière sans borne.
        tranches: Vec<TrancheBareme>,
        /// Bornes annuelles : l'assiette du mois × 12, l'impôt trouvé ÷ 12.
        annuel: bool,
        /// Réduction en points : ce pourcentage de l'assiette retranché de l'impôt (0 : aucune).
        reduction_points: i64,
        /// Réduction pour charges de famille.
        famille: Option<ReductionFamille>,
    },
    /// Le montant se saisit dans la paie du mois.
    Saisi,
}

/// Élément de paie de l'école.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
pub struct ElementPaie {
    /// Identifiant technique.
    pub id: Id<ElementPaie>,
    /// Établissement.
    pub etablissement_id: Id<Etablissement>,
    /// Code stable d'un élément livré ; absent pour un élément créé par l'école.
    pub code: Option<String>,
    /// Libellé imprimé sur le bulletin.
    pub libelle: String,
    /// Nature.
    pub nature: NatureElement,
    /// Natures d'engagement auxquelles il s'applique.
    pub natures: Vec<NatureEngagement>,
    /// Vrai s'il s'applique aussi aux fonctionnaires de l'État.
    pub fonctionnaires: bool,
    /// Ordre sur le bulletin et dans le calcul.
    pub ordre: i64,
    /// Faux pour un élément que l'école ne pratique pas, ou qu'elle a retiré.
    pub actif: bool,
}

impl ElementPaie {
    /// Vrai si l'élément s'applique à un engagement de cette nature, d'un fonctionnaire ou non.
    pub fn s_applique(&self, nature: NatureEngagement, fonctionnaire: bool) -> bool {
        self.actif && self.natures.contains(&nature) && (self.fonctionnaires || !fonctionnaire)
    }
}

/// Version datée du calcul d'un élément : une paie applique celle en vigueur dans son mois.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
pub struct VersionElement {
    /// Identifiant technique.
    pub id: Id<VersionElement>,
    /// Établissement.
    pub etablissement_id: Id<Etablissement>,
    /// Élément.
    pub element_id: Id<ElementPaie>,
    /// Premier mois d'application.
    pub effet: Mois,
    /// Calcul.
    pub calcul: Calcul,
    /// Retenues salariales retranchées du brut pour former l'assiette d'un taux ou d'un barème.
    pub assiette_deduit: Vec<Id<ElementPaie>>,
}

/// Version en vigueur dans un mois : la dernière dont l'effet n'est pas après lui.
pub fn version_en_vigueur(versions: &[VersionElement], mois: Mois) -> Option<&VersionElement> {
    versions
        .iter()
        .filter(|v| v.effet <= mois)
        .max_by_key(|v| v.effet)
}

/// Gain ou retenue attribué à un membre, avec son montant propre, chaque mois de sa période.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
pub struct Attribution {
    /// Identifiant technique.
    pub id: Id<Attribution>,
    /// Établissement.
    pub etablissement_id: Id<Etablissement>,
    /// Membre.
    pub membre_id: Id<MembrePersonnel>,
    /// Élément.
    pub element_id: Id<ElementPaie>,
    /// Montant mensuel.
    pub montant: Montant,
    /// Premier mois.
    pub debut: Mois,
    /// Dernier mois ; absent tant qu'elle dure.
    pub fin: Option<Mois>,
}

impl Attribution {
    /// Vrai si l'attribution court ce mois.
    pub fn couvre(&self, mois: Mois) -> bool {
        self.debut <= mois && self.fin.is_none_or(|f| mois <= f)
    }
}

/// Longueur maximale d'un libellé d'élément.
pub const LIBELLE_MAX: usize = 60;

/// Vérifie un élément ; l'erreur est la clé du message.
pub fn verifier_element(e: &ElementPaie) -> Result<(), &'static str> {
    let libelle = e.libelle.trim();
    if libelle.is_empty() || libelle.chars().count() > LIBELLE_MAX {
        return Err("element.libelle_invalide");
    }
    if e.natures.is_empty() {
        return Err("element.aucune_nature");
    }
    Ok(())
}

fn taux_valide(taux: i64) -> bool {
    (0..=TAUX_MAX).contains(&taux)
}

/// Vérifie le calcul d'une version pour un élément de cette nature.
pub fn verifier_calcul(nature: NatureElement, calcul: &Calcul) -> Result<(), &'static str> {
    match calcul {
        Calcul::Fixe { montant } => {
            if montant.francs() <= 0 {
                return Err("element.montant_invalide");
            }
        }
        Calcul::Taux { taux, plafond } => {
            if !taux_valide(*taux) {
                return Err("element.taux_invalide");
            }
            if plafond.is_some_and(|p| p.francs() <= 0) {
                return Err("element.plafond_invalide");
            }
        }
        Calcul::Bareme {
            tranches,
            reduction_points,
            famille,
            ..
        } => {
            if nature != NatureElement::Retenue {
                return Err("element.bareme_retenue");
            }
            let derniere = tranches.len().saturating_sub(1);
            let mut precedente: Option<Montant> = None;
            for (i, t) in tranches.iter().enumerate() {
                if !taux_valide(t.taux) {
                    return Err("element.tranches_invalides");
                }
                match t.jusqua {
                    None if i != derniere => return Err("element.tranches_invalides"),
                    Some(_) if i == derniere => return Err("element.tranches_invalides"),
                    Some(b) => {
                        if b.francs() <= precedente.map_or(0, |p| p.francs()) {
                            return Err("element.tranches_invalides");
                        }
                        precedente = Some(b);
                    }
                    None => {}
                }
            }
            if tranches.is_empty() {
                return Err("element.tranches_invalides");
            }
            if !taux_valide(*reduction_points) {
                return Err("element.reduction_invalide");
            }
            if famille.is_some_and(|f| {
                !taux_valide(f.marie) || !taux_valide(f.par_enfant) || !taux_valide(f.plafond)
            }) {
                return Err("element.reduction_invalide");
            }
        }
        Calcul::Saisi => {
            if nature == NatureElement::CotisationPatronale {
                return Err("element.cotisation_calculee");
            }
        }
    }
    Ok(())
}

/// Vérifie une attribution à un membre.
pub fn verifier_attribution(element: &ElementPaie, a: &Attribution) -> Result<(), &'static str> {
    if element.nature == NatureElement::CotisationPatronale {
        return Err("attribution.cotisation");
    }
    if a.montant.francs() <= 0 {
        return Err("attribution.montant_invalide");
    }
    if a.fin.is_some_and(|f| f < a.debut) {
        return Err("attribution.fin_avant_debut");
    }
    Ok(())
}

/// Élément livré avec GESCO.
#[derive(Debug, Clone, Copy)]
pub struct ElementLivre {
    /// Code stable.
    pub code: &'static str,
    /// Libellé.
    pub libelle: &'static str,
    /// Nature.
    pub nature: NatureElement,
    /// Actif dès l'installation.
    pub actif: bool,
    /// S'applique aussi aux fonctionnaires.
    pub fonctionnaires: bool,
    /// Premier mois de la version livrée, `AAAA-MM`.
    pub effet: &'static str,
    /// Calcul proposé.
    pub calcul: CalculLivre,
    /// Codes des retenues retranchées de l'assiette.
    pub assiette_deduit: &'static [&'static str],
}

/// Calcul d'un élément livré, en données constantes.
#[derive(Debug, Clone, Copy)]
pub enum CalculLivre {
    /// Saisi chaque mois.
    Saisi,
    /// Un taux du brut, sans plafond.
    Taux(i64),
    /// Un barème annuel : (borne haute en francs, taux), la dernière sans borne ; réduction en
    /// points ; réduction pour charges de famille (marié, par enfant, plafond).
    BaremeAnnuel(&'static [(Option<i64>, i64)], i64, (i64, i64, i64)),
}

impl CalculLivre {
    /// Le calcul du noyau.
    pub fn calcul(&self) -> Calcul {
        match self {
            Self::Saisi => Calcul::Saisi,
            Self::Taux(taux) => Calcul::Taux {
                taux: *taux,
                plafond: None,
            },
            Self::BaremeAnnuel(tranches, points, (marie, par_enfant, plafond)) => Calcul::Bareme {
                tranches: tranches
                    .iter()
                    .map(|(borne, taux)| TrancheBareme {
                        jusqua: borne.map(Montant::depuis_francs),
                        taux: *taux,
                    })
                    .collect(),
                annuel: true,
                reduction_points: *points,
                famille: Some(ReductionFamille {
                    marie: *marie,
                    par_enfant: *par_enfant,
                    plafond: *plafond,
                }),
            },
        }
    }
}

/// Natures d'engagement payées : toutes sauf le bénévolat.
pub const NATURES_PAYEES: [NatureEngagement; 4] = [
    NatureEngagement::DureeIndeterminee,
    NatureEngagement::DureeDeterminee,
    NatureEngagement::Vacation,
    NatureEngagement::Stage,
];

/// Barème annuel de l'impôt sur les traitements et salaires (Code général des impôts du Mali,
/// art. 10, loi 2016-010), proposé à vérifier par l'école.
const BAREME_ITS: &[(Option<i64>, i64)] = &[
    (Some(330_000), 0),
    (Some(578_400), 500),
    (Some(1_176_400), 1_200),
    (Some(1_789_733), 1_800),
    (Some(2_384_195), 2_600),
    (Some(3_494_130), 3_100),
    (None, 3_700),
];

/// Élément livré : `(actif, fonctionnaires)` disent s'il est actif à l'installation et s'il
/// s'applique aussi aux fonctionnaires.
const fn livre(
    code: &'static str,
    libelle: &'static str,
    nature: NatureElement,
    (actif, fonctionnaires): (bool, bool),
    effet: &'static str,
    calcul: CalculLivre,
    assiette_deduit: &'static [&'static str],
) -> ElementLivre {
    ElementLivre {
        code,
        libelle,
        nature,
        actif,
        fonctionnaires,
        effet,
        calcul,
        assiette_deduit,
    }
}

/// Éléments livrés, dans l'ordre du bulletin (SPEC_PHASE_7 §3) : les saisis, actifs ; les
/// retenues et cotisations du Mali, inactives, aux taux proposés (CLEISS au 1er janvier 2025 ;
/// Code général des impôts, art. 10 ; réduction de deux points, loi 2008-002).
pub const ELEMENTS_LIVRES: &[ElementLivre] = {
    use CalculLivre::{BaremeAnnuel, Saisi, Taux};
    use NatureElement::{CotisationPatronale, Gain, Retenue};
    &[
        livre("PRIME", "Prime", Gain, (true, true), "2000-01", Saisi, &[]),
        livre(
            "HSUP",
            "Heures supplémentaires",
            Gain,
            (true, true),
            "2000-01",
            Saisi,
            &[],
        ),
        livre(
            "REGUL",
            "Régularisation",
            Gain,
            (true, true),
            "2000-01",
            Saisi,
            &[],
        ),
        livre(
            "RETENUE",
            "Retenue diverse",
            Retenue,
            (true, true),
            "2000-01",
            Saisi,
            &[],
        ),
        livre(
            "INPS_RET",
            "Retraite INPS",
            Retenue,
            (false, false),
            "2025-01",
            Taux(360),
            &[],
        ),
        livre(
            "AMO",
            "AMO",
            Retenue,
            (false, false),
            "2025-01",
            Taux(306),
            &[],
        ),
        livre(
            "ITS",
            "Impôt sur les traitements et salaires",
            Retenue,
            (false, true),
            "2015-07",
            BaremeAnnuel(BAREME_ITS, 200, (1_000, 250, 3_500)),
            &["INPS_RET"],
        ),
        livre(
            "INPS_RET_PAT",
            "Retraite INPS (part patronale)",
            CotisationPatronale,
            (false, false),
            "2025-01",
            Taux(540),
            &[],
        ),
        livre(
            "INPS_PF",
            "Prestations familiales (INPS)",
            CotisationPatronale,
            (false, false),
            "2025-01",
            Taux(800),
            &[],
        ),
        livre(
            "INPS_AT",
            "Accidents du travail (INPS)",
            CotisationPatronale,
            (false, false),
            "2025-01",
            Taux(100),
            &[],
        ),
        livre(
            "AMO_PAT",
            "AMO (part patronale)",
            CotisationPatronale,
            (false, false),
            "2025-01",
            Taux(350),
            &[],
        ),
        livre(
            "ANPE",
            "Contribution à l'ANPE",
            CotisationPatronale,
            (false, false),
            "2025-01",
            Taux(100),
            &[],
        ),
    ]
};

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    fn mois(t: &str) -> Mois {
        Mois::analyser(t).unwrap_or_else(|| unreachable!())
    }

    fn element(nature: NatureElement) -> ElementPaie {
        ElementPaie {
            id: Id::depuis_octets([1; 16]),
            etablissement_id: Id::depuis_octets([2; 16]),
            code: None,
            libelle: "Prime de transport".to_owned(),
            nature,
            natures: NATURES_PAYEES.to_vec(),
            fonctionnaires: true,
            ordre: 1,
            actif: true,
        }
    }

    #[test]
    fn un_taux_se_lit_et_s_ecrit() {
        assert_eq!(lire_taux("3,06"), Ok(306));
        assert_eq!(lire_taux(" 3.6 % "), Ok(360));
        assert_eq!(lire_taux("37"), Ok(3_700));
        assert_eq!(lire_taux("0"), Ok(0));
        assert_eq!(lire_taux("100"), Ok(10_000));
        assert_eq!(lire_taux("100,01"), Err("element.taux_invalide"));
        assert_eq!(lire_taux("3,061"), Err("element.taux_invalide"));
        assert_eq!(lire_taux("-2"), Err("element.taux_invalide"));
        assert_eq!(lire_taux(""), Err("element.taux_invalide"));
        assert_eq!(ecrire_taux(306), "3,06\u{a0}%");
        assert_eq!(ecrire_taux(360), "3,6\u{a0}%");
        assert_eq!(ecrire_taux(3_700), "37\u{a0}%");
    }

    #[test]
    fn un_calcul_se_controle_selon_la_nature() {
        use NatureElement::{CotisationPatronale, Gain, Retenue};
        let f = |v| Montant::depuis_francs(v);
        assert_eq!(
            verifier_calcul(Gain, &Calcul::Fixe { montant: f(0) }),
            Err("element.montant_invalide")
        );
        assert_eq!(
            verifier_calcul(
                Retenue,
                &Calcul::Taux {
                    taux: 10_001,
                    plafond: None
                }
            ),
            Err("element.taux_invalide")
        );
        assert_eq!(
            verifier_calcul(CotisationPatronale, &Calcul::Saisi),
            Err("element.cotisation_calculee")
        );
        let bareme = |tranches: Vec<TrancheBareme>| Calcul::Bareme {
            tranches,
            annuel: false,
            reduction_points: 0,
            famille: None,
        };
        let t = |b: Option<i64>, taux| TrancheBareme {
            jusqua: b.map(f),
            taux,
        };
        assert_eq!(
            verifier_calcul(Gain, &bareme(vec![t(None, 100)])),
            Err("element.bareme_retenue")
        );
        assert_eq!(
            verifier_calcul(Retenue, &bareme(vec![t(Some(100), 0), t(None, 500)])),
            Ok(())
        );
        for mauvaises in [
            vec![],
            vec![t(Some(100), 0)],
            vec![t(None, 0), t(None, 500)],
            vec![t(Some(100), 0), t(Some(100), 5), t(None, 500)],
        ] {
            assert_eq!(
                verifier_calcul(Retenue, &bareme(mauvaises)),
                Err("element.tranches_invalides")
            );
        }
    }

    #[test]
    fn la_version_en_vigueur_est_la_derniere_avant_le_mois() {
        let v = |effet: &str, taux| VersionElement {
            id: Id::depuis_octets([3; 16]),
            etablissement_id: Id::depuis_octets([2; 16]),
            element_id: Id::depuis_octets([1; 16]),
            effet: mois(effet),
            calcul: Calcul::Taux {
                taux,
                plafond: None,
            },
            assiette_deduit: vec![],
        };
        let versions = [v("2025-01", 360), v("2027-01", 400)];
        assert_eq!(version_en_vigueur(&versions, mois("2024-12")), None);
        assert_eq!(
            version_en_vigueur(&versions, mois("2026-10")).map(|v| v.effet),
            Some(mois("2025-01"))
        );
        assert_eq!(
            version_en_vigueur(&versions, mois("2027-01")).map(|v| v.effet),
            Some(mois("2027-01"))
        );
    }

    #[test]
    fn un_element_s_applique_selon_l_engagement() {
        let mut e = element(NatureElement::Retenue);
        e.fonctionnaires = false;
        assert!(e.s_applique(NatureEngagement::Vacation, false));
        assert!(!e.s_applique(NatureEngagement::Vacation, true));
        assert!(!e.s_applique(NatureEngagement::Benevole, false));
        e.actif = false;
        assert!(!e.s_applique(NatureEngagement::DureeIndeterminee, false));
    }

    #[test]
    fn une_attribution_se_controle() {
        let a = Attribution {
            id: Id::depuis_octets([4; 16]),
            etablissement_id: Id::depuis_octets([2; 16]),
            membre_id: Id::depuis_octets([5; 16]),
            element_id: Id::depuis_octets([1; 16]),
            montant: Montant::depuis_francs(10_000),
            debut: mois("2026-10"),
            fin: Some(mois("2026-12")),
        };
        assert_eq!(
            verifier_attribution(&element(NatureElement::Gain), &a),
            Ok(())
        );
        assert!(a.couvre(mois("2026-11")));
        assert!(!a.couvre(mois("2027-01")));
        assert_eq!(
            verifier_attribution(&element(NatureElement::CotisationPatronale), &a),
            Err("attribution.cotisation")
        );
        let mut b = a.clone();
        b.fin = Some(mois("2026-09"));
        assert_eq!(
            verifier_attribution(&element(NatureElement::Gain), &b),
            Err("attribution.fin_avant_debut")
        );
    }

    #[test]
    fn les_elements_livres_sont_valides() {
        let codes: HashSet<&str> = ELEMENTS_LIVRES.iter().map(|e| e.code).collect();
        assert_eq!(codes.len(), ELEMENTS_LIVRES.len());
        for e in ELEMENTS_LIVRES {
            assert_eq!(
                verifier_calcul(e.nature, &e.calcul.calcul()),
                Ok(()),
                "{}",
                e.code
            );
            assert!(Mois::analyser(e.effet).is_some(), "{}", e.code);
            for d in e.assiette_deduit {
                assert!(codes.contains(d), "{} : {d}", e.code);
            }
            // Les retenues du Mali arrivent inactives : l'école les vérifie et les active.
            assert_eq!(
                e.actif,
                matches!(e.calcul, CalculLivre::Saisi),
                "{}",
                e.code
            );
        }
    }
}
