//! Reprise avant GESCO (SPEC_REPRISE_AVANT_GESCO §3) : ce que les parents ont payé avant
//! l'installation. Un seul montant par inscription, qui règle des échéances comme un avoir, sans
//! versement, sans reçu, sans caisse. Ce qui dépasse ce que l'école facture est à rembourser au
//! parent, par une dépense liée à l'inscription (§3.6 bis). Tout se recalcule (R-20).

use serde::{Deserialize, Serialize};

use crate::commun::{DateCivile, ErreurMontant, Horodatage, Id, Montant};
use crate::core::{Etablissement, Utilisateur};
use crate::finance::facture::Echeance;
use crate::scolarite::Inscription;

/// Longueur maximale de la note (« cahier p. 12 »).
pub const NOTE_MAX: usize = 120;

/// Code stable de la catégorie de dépense des remboursements aux parents.
pub const CATEGORIE_REMBOURSEMENT: &str = "remboursement_parent";

/// D'où vient une reprise.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "snake_case")]
pub enum OrigineReprise {
    /// Saisie à l'écran.
    Ecran,
    /// Importée d'un classeur.
    Import,
}

impl OrigineReprise {
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

/// État d'une reprise ou d'une de ses parts.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "snake_case")]
pub enum StatutReprise {
    /// En vigueur.
    Active,
    /// Annulée ou remplacée ; jamais supprimée (R-14).
    Annulee,
}

impl StatutReprise {
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

/// Ce qu'un parent avait payé avant GESCO, pour une inscription.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
pub struct RepriseAvantGesco {
    /// Identifiant technique.
    pub id: Id<RepriseAvantGesco>,
    /// Établissement propriétaire.
    pub etablissement_id: Id<Etablissement>,
    /// Inscription : la finance référence l'inscription, jamais l'élève.
    pub inscription_id: Id<Inscription>,
    /// Montant, strictement positif.
    pub montant: Montant,
    /// Note libre : la référence du cahier ou des reçus papier.
    pub note: String,
    /// Écran ou import.
    pub origine: OrigineReprise,
    /// Active ou annulée.
    pub statut: StatutReprise,
    /// Motif d'annulation (« remplacée » pour une modification).
    pub motif_annulation: String,
    /// Reprise qui la remplace, pour une modification.
    pub remplacee_par: Option<Id<RepriseAvantGesco>>,
    /// Instant de la saisie.
    pub cree_le: Horodatage,
    /// Auteur de la saisie.
    pub cree_par: Option<Id<Utilisateur>>,
    /// Instant de la dernière modification (annulation) : l'écran le renvoie.
    pub mis_a_jour_le: Horodatage,
    /// Auteur de l'annulation.
    pub mis_a_jour_par: Option<Id<Utilisateur>>,
}

/// Part d'une reprise portée par une échéance.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
pub struct PartReprise {
    /// Échéance.
    pub echeance_id: Id<Echeance>,
    /// Montant porté.
    pub montant: Montant,
}

/// Chiffres d'une reprise, recalculés à chaque fois (§3.6 bis).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize, specta::Type)]
pub struct ChiffresReprise {
    /// Montant de la reprise active (zéro sans reprise).
    pub montant: Montant,
    /// Remboursé au parent : dépenses payées et non annulées liées à l'inscription.
    pub rembourse: Montant,
    /// Montant moins remboursé, jamais négatif.
    pub utilisable: Montant,
    /// Porté par les échéances.
    pub impute: Montant,
    /// Excédent qui reste à rendre au parent : utilisable moins imputé.
    pub a_rembourser: Montant,
}

/// Vérifie et nettoie la note.
pub fn verifier_note(note: &str) -> Result<String, &'static str> {
    let note = note.trim();
    if note.chars().count() > NOTE_MAX {
        return Err("reprise.note_trop_longue");
    }
    Ok(note.to_owned())
}

/// Lit un montant saisi : « 75000 », « 75 000 », « 75 000 F », « 75.000 ». Pas de décimales
/// ni de signe. Vide : `Some(0)` (la case vidée annule la reprise).
pub fn lire_montant(texte: &str) -> Option<i64> {
    let mut sans_franc = texte.trim().to_lowercase();
    for suffixe in ["fcfa", "cfa", "f"] {
        if let Some(reste) = sans_franc.strip_suffix(suffixe) {
            sans_franc = reste.trim_end().to_owned();
            break;
        }
    }
    let chiffres: String = sans_franc
        .chars()
        .filter(|c| !matches!(c, ' ' | '\u{a0}' | '\u{202f}' | '.'))
        .collect();
    if chiffres.is_empty() {
        return Some(0);
    }
    if !chiffres.chars().all(|c| c.is_ascii_digit()) {
        return None;
    }
    chiffres.parse::<i64>().ok()
}

/// Parts de la reprise : l'utilisable remplit les restes (calculés sans la reprise), donnés dans
/// l'ordre des dates puis des rangs, plus ancienne d'abord ; jamais plus que la somme des restes
/// (le surplus est à rembourser). C'est `imputer_plus_ancienne`, borné au lieu de refuser.
pub fn imputer(
    restes_sans_reprise: &[(Id<Echeance>, Montant)],
    utilisable: Montant,
) -> Result<Vec<PartReprise>, ErreurMontant> {
    let mut a_imputer = utilisable.max(Montant::ZERO);
    let mut parts = Vec::new();
    for (echeance_id, reste) in restes_sans_reprise {
        if a_imputer.est_nul() {
            break;
        }
        let part = (*reste).max(Montant::ZERO).min(a_imputer);
        if !part.est_nul() {
            parts.push(PartReprise {
                echeance_id: *echeance_id,
                montant: part,
            });
            a_imputer = a_imputer.soustraire(part)?;
        }
    }
    Ok(parts)
}

/// Chiffres d'une inscription : montant de la reprise active, remboursé, et parts calculées.
pub fn chiffres(
    montant: Montant,
    rembourse: Montant,
    parts: &[PartReprise],
) -> Result<ChiffresReprise, ErreurMontant> {
    let utilisable = montant.soustraire(rembourse)?.max(Montant::ZERO);
    let impute = parts
        .iter()
        .try_fold(Montant::ZERO, |s, p| s.ajouter(p.montant))?;
    Ok(ChiffresReprise {
        montant,
        rembourse,
        utilisable,
        impute,
        a_rembourser: utilisable.soustraire(impute)?.max(Montant::ZERO),
    })
}

/// Plus petit montant qu'une reprise peut prendre (§3.3) : jamais sous ce qui a déjà été
/// remboursé (`reprise.deja_remboursee`) ; après un changement de classe qui a fait naître des
/// avoirs, jamais sous ce qui garde ces avoirs couverts (`reprise.avoir_a_couvrir`) : payé total
/// au moins égal au dû net plus les avoirs nés. Rend le minimum et la règle qui le fixe.
pub fn minimum(
    rembourse: Montant,
    du_net: Montant,
    avoirs_nes: Montant,
    paye_hors_reprise: Montant,
) -> Result<(Montant, Option<&'static str>), ErreurMontant> {
    let pour_les_avoirs = if avoirs_nes.est_nul() {
        Montant::ZERO
    } else {
        du_net
            .ajouter(avoirs_nes)?
            .soustraire(paye_hors_reprise)?
            .max(Montant::ZERO)
    };
    if rembourse.est_nul() && pour_les_avoirs.est_nul() {
        return Ok((Montant::ZERO, None));
    }
    if pour_les_avoirs > rembourse {
        Ok((pour_les_avoirs, Some("reprise.avoir_a_couvrir")))
    } else {
        Ok((rembourse, Some("reprise.deja_remboursee")))
    }
}

/// Date d'arrêt avant GESCO (§3.2), jamais stockée : la veille du jour de la première session de
/// caisse ouverte dans l'année ; sans caisse, la veille d'aujourd'hui (« à ce jour »). Rend la
/// date et vrai si elle vient d'une caisse.
pub fn date_d_arret(
    premiere_caisse: Option<DateCivile>,
    aujourd_hui: DateCivile,
) -> (DateCivile, bool) {
    match premiere_caisse {
        Some(jour) => (jour.decaler(-1).unwrap_or(jour), true),
        None => (aujourd_hui.decaler(-1).unwrap_or(aujourd_hui), false),
    }
}

#[cfg(test)]
#[allow(clippy::indexing_slicing, clippy::unwrap_used)]
mod tests {
    use super::*;

    fn f(n: i64) -> Montant {
        Montant::depuis_francs(n)
    }

    fn id(n: u8) -> Id<Echeance> {
        Id::depuis_octets([n; 16])
    }

    #[test]
    fn la_reprise_remplit_la_plus_ancienne_d_abord_et_garde_l_excedent() {
        let restes = [(id(1), f(25_000)), (id(2), f(50_000)), (id(3), f(75_000))];
        let parts = imputer(&restes, f(60_000)).unwrap();
        assert_eq!(parts.len(), 2);
        assert_eq!(parts[0].montant, f(25_000));
        assert_eq!(parts[1].montant, f(35_000));
        let c = chiffres(f(60_000), Montant::ZERO, &parts).unwrap();
        assert_eq!(c.a_rembourser, Montant::ZERO);

        let parts = imputer(&restes, f(200_000)).unwrap();
        let c = chiffres(f(200_000), Montant::ZERO, &parts).unwrap();
        assert_eq!(c.impute, f(150_000));
        assert_eq!(c.a_rembourser, f(50_000));
    }

    #[test]
    fn le_rembourse_reduit_l_utilisable() {
        let restes = [(id(1), f(185_000))];
        let utilisable = f(200_000).soustraire(f(10_000)).unwrap();
        let parts = imputer(&restes, utilisable).unwrap();
        let c = chiffres(f(200_000), f(10_000), &parts).unwrap();
        assert_eq!(c.utilisable, f(190_000));
        assert_eq!(c.impute, f(185_000));
        assert_eq!(c.a_rembourser, f(5_000));
        // Imputé + à rembourser + remboursé = montant.
        assert_eq!(
            c.impute
                .ajouter(c.a_rembourser)
                .unwrap()
                .ajouter(c.rembourse)
                .unwrap(),
            c.montant
        );
    }

    #[test]
    fn le_minimum_suit_le_rembourse_puis_les_avoirs() {
        assert_eq!(
            minimum(Montant::ZERO, f(100_000), Montant::ZERO, Montant::ZERO).unwrap(),
            (Montant::ZERO, None)
        );
        assert_eq!(
            minimum(f(15_000), f(100_000), Montant::ZERO, Montant::ZERO).unwrap(),
            (f(15_000), Some("reprise.deja_remboursee"))
        );
        assert_eq!(
            minimum(Montant::ZERO, f(80_000), f(20_000), f(30_000)).unwrap(),
            (f(70_000), Some("reprise.avoir_a_couvrir"))
        );
    }

    #[test]
    fn les_montants_saisis_se_lisent() {
        assert_eq!(lire_montant("75000"), Some(75_000));
        assert_eq!(lire_montant("75 000"), Some(75_000));
        assert_eq!(lire_montant("75 000 F"), Some(75_000));
        assert_eq!(lire_montant("75.000"), Some(75_000));
        assert_eq!(lire_montant(""), Some(0));
        assert_eq!(lire_montant("-5"), None);
        assert_eq!(lire_montant("12,5"), None);
    }

    #[test]
    fn la_date_d_arret_est_la_veille_de_la_premiere_caisse() {
        let jour = |j| DateCivile::nouvelle(2026, 11, j).unwrap();
        assert_eq!(date_d_arret(Some(jour(18)), jour(25)), (jour(17), true));
        assert_eq!(date_d_arret(None, jour(25)), (jour(24), false));
    }
}
