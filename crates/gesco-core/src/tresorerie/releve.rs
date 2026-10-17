//! Import des relevés (SPEC_VERSION_1_1 §1, étape 7, T-72) : un relevé d'Orange Money, de Wave,
//! de Moov ou de la banque, en Excel ou en CSV. L'école dit une fois, pour chaque compte, les
//! colonnes de la date, du montant (ou du débit et du crédit) et de la référence. GESCO coche les
//! lignes du livre qu'il retrouve : d'abord par la référence et le montant, puis par le montant à
//! trois jours près ; une ligne du relevé pour une ligne du livre.

use serde::{Deserialize, Serialize};

use crate::commun::{DateCivile, Montant};
use crate::tresorerie::livre::LigneLivre;

/// Écart de jours toléré entre le relevé et le livre.
pub const ECART_JOURS_MAX: i64 = 3;

/// Colonnes d'un relevé, comptées à partir de zéro dans la ligne d'en-tête.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
pub struct FormatReleve {
    /// Colonne de la date.
    pub date: u32,
    /// Colonne du montant signé (positif pour une entrée), si le relevé n'a qu'une colonne.
    pub montant: Option<u32>,
    /// Colonne du débit (une sortie), si le relevé en a deux.
    pub debit: Option<u32>,
    /// Colonne du crédit (une entrée), si le relevé en a deux.
    pub credit: Option<u32>,
    /// Colonne de la référence de la transaction.
    pub reference: Option<u32>,
}

/// Ligne lue d'un relevé.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
pub struct LigneDeReleve {
    /// Jour.
    pub date: DateCivile,
    /// Montant signé : positif pour une entrée sur le compte, négatif pour une sortie.
    pub montant: Montant,
    /// Référence de la transaction.
    pub reference: String,
}

/// Lit une date : `AAAA-MM-JJ`, `JJ/MM/AAAA`, `JJ-MM-AAAA` ou `JJ.MM.AAAA`, suivie ou non d'une
/// heure.
pub fn lire_date(texte: &str) -> Option<DateCivile> {
    let jour = texte.split_whitespace().next()?;
    let jour = jour.split('T').next()?;
    if let Some(d) = DateCivile::analyser(jour) {
        return Some(d);
    }
    let morceaux: Vec<&str> = jour.split(['/', '-', '.']).collect();
    let [j, m, a] = morceaux.as_slice() else {
        return None;
    };
    let annee: i32 = a.parse().ok()?;
    let annee = if annee < 100 { 2000 + annee } else { annee };
    DateCivile::nouvelle(annee, m.parse().ok()?, j.parse().ok()?)
}

/// Lit un montant en francs : `12 500`, `12 500,00`, `-12500`, `12.500 F`, `1,250.00` ;
/// arrondi au franc.
pub fn lire_montant(texte: &str) -> Option<i64> {
    let garde: String = texte
        .chars()
        .filter(|c| c.is_ascii_digit() || matches!(c, '-' | ',' | '.'))
        .collect();
    if !garde.chars().any(|c| c.is_ascii_digit()) {
        return None;
    }
    let negatif = garde.starts_with('-') || texte.trim().starts_with('(');
    let chiffres = garde.trim_start_matches('-');
    // Le dernier séparateur suivi d'un ou deux chiffres est la virgule décimale ; les autres
    // séparent les milliers.
    let decimale = chiffres
        .rfind([',', '.'])
        .filter(|&i| (1..=2).contains(&chiffres.len().saturating_sub(i + 1)));
    let (entiers, centimes) = match decimale {
        Some(i) => (chiffres.get(..i)?, chiffres.get(i + 1..)?),
        None => (chiffres, ""),
    };
    let entiers: String = entiers.chars().filter(char::is_ascii_digit).collect();
    let mut francs: i64 = if entiers.is_empty() {
        0
    } else {
        entiers.parse().ok()?
    };
    if centimes.starts_with(|c: char| ('5'..='9').contains(&c)) {
        francs = francs.checked_add(1)?;
    }
    Some(if negatif { -francs } else { francs })
}

/// Lignes d'un relevé d'après son format ; la première ligne est l'en-tête. Rend les lignes lues
/// et le nombre de lignes laissées de côté (sans date ou sans montant lisibles).
pub fn lire_lignes(lignes: &[Vec<String>], format: &FormatReleve) -> (Vec<LigneDeReleve>, u32) {
    let cellule = |l: &Vec<String>, c: Option<u32>| {
        c.and_then(|c| l.get(usize::try_from(c).ok()?))
            .map(String::as_str)
            .unwrap_or("")
            .to_owned()
    };
    let mut lues = Vec::new();
    let mut ignorees = 0u32;
    for l in lignes.iter().skip(1) {
        let date = lire_date(&cellule(l, Some(format.date)));
        let montant = match format.montant {
            Some(c) => lire_montant(&cellule(l, Some(c))),
            None => {
                let credit = lire_montant(&cellule(l, format.credit)).unwrap_or(0).abs();
                let debit = lire_montant(&cellule(l, format.debit)).unwrap_or(0).abs();
                (credit != 0 || debit != 0).then(|| credit - debit)
            }
        };
        match (date, montant) {
            (Some(date), Some(m)) if m != 0 => lues.push(LigneDeReleve {
                date,
                montant: Montant::depuis_francs(m),
                reference: cellule(l, format.reference).trim().to_owned(),
            }),
            _ => ignorees = ignorees.saturating_add(1),
        }
    }
    (lues, ignorees)
}

fn normaliser(reference: &str) -> String {
    reference
        .chars()
        .filter(|c| c.is_alphanumeric())
        .flat_map(char::to_uppercase)
        .collect()
}

/// Rapproche un relevé du livre : rend les couples (ligne du relevé, ligne du livre) retrouvés,
/// une ligne pour une ligne, parmi les lignes du livre pas encore cochées. D'abord par la
/// référence et le montant, puis par le montant à trois jours près, au plus proche.
pub fn rapprocher(livre: &[LigneLivre], releve: &[LigneDeReleve]) -> Vec<(usize, usize)> {
    let net = |l: &LigneLivre| l.entree.francs() - l.sortie.francs();
    let mut prises = vec![false; livre.len()];
    let mut couples = Vec::new();
    let mut restent: Vec<usize> = Vec::new();
    for (i, r) in releve.iter().enumerate() {
        let reference = normaliser(&r.reference);
        let trouvee = (!reference.is_empty())
            .then(|| {
                livre.iter().enumerate().position(|(j, l)| {
                    !l.pointee
                        && !prises.get(j).copied().unwrap_or(true)
                        && net(l) == r.montant.francs()
                        && normaliser(&l.reference) == reference
                })
            })
            .flatten();
        match trouvee {
            Some(j) => {
                if let Some(p) = prises.get_mut(j) {
                    *p = true;
                }
                couples.push((i, j));
            }
            None => restent.push(i),
        }
    }
    for i in restent {
        let Some(r) = releve.get(i) else { continue };
        let meilleure = livre
            .iter()
            .enumerate()
            .filter(|(j, l)| {
                !l.pointee
                    && !prises.get(*j).copied().unwrap_or(true)
                    && net(l) == r.montant.francs()
                    && l.date.jours_jusqua(r.date).abs() <= ECART_JOURS_MAX
            })
            .min_by_key(|(_, l)| l.date.jours_jusqua(r.date).abs())
            .map(|(j, _)| j);
        if let Some(j) = meilleure {
            if let Some(p) = prises.get_mut(j) {
                *p = true;
            }
            couples.push((i, j));
        }
    }
    couples.sort_unstable();
    couples
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tresorerie::livre::OrigineLigne;

    fn jour(j: u8) -> DateCivile {
        DateCivile::nouvelle(2026, 10, j).unwrap_or_else(|| unreachable!())
    }

    #[test]
    fn les_dates_et_les_montants_d_un_releve_se_lisent() {
        assert_eq!(lire_date("12/10/2026"), Some(jour(12)));
        assert_eq!(lire_date("2026-10-12 08:14:02"), Some(jour(12)));
        assert_eq!(lire_date("12.10.26"), Some(jour(12)));
        assert_eq!(lire_date("hier"), None);
        assert_eq!(lire_montant("12 500"), Some(12_500));
        assert_eq!(lire_montant("12 500,00 F"), Some(12_500));
        assert_eq!(lire_montant("-25000"), Some(-25_000));
        assert_eq!(lire_montant("1,250.50"), Some(1_251));
        assert_eq!(lire_montant("12.500"), Some(12_500));
        assert_eq!(lire_montant(""), None);
    }

    #[test]
    fn le_releve_se_rapproche_par_reference_puis_par_montant_et_date() {
        let ligne = |j: u8, entree: i64, sortie: i64, reference: &str| LigneLivre {
            date: jour(j),
            origine: OrigineLigne::Transfert,
            origine_id: String::new(),
            libelle: String::new(),
            reference: reference.to_owned(),
            entree: Montant::depuis_francs(entree),
            sortie: Montant::depuis_francs(sortie),
            pointee: false,
        };
        let livre = [
            ligne(5, 50_000, 0, "BV-0042"),
            ligne(6, 0, 150_000, "VIR-118"),
            ligne(9, 25_000, 0, ""),
            ligne(20, 25_000, 0, ""),
        ];
        let lue = |j: u8, montant: i64, reference: &str| LigneDeReleve {
            date: jour(j),
            montant: Montant::depuis_francs(montant),
            reference: reference.to_owned(),
        };
        let releve = [
            lue(7, -150_000, "vir 118"),
            lue(5, 50_000, "BV0042"),
            lue(10, 25_000, "OM-77"),
            lue(1, 999, ""),
        ];
        // La référence retrouve le virement et le dépôt, le montant la recette du 9 (pas celle
        // du 20, trop loin) ; la ligne de 999 F reste.
        assert_eq!(rapprocher(&livre, &releve), vec![(0, 1), (1, 0), (2, 2)]);
        let format = FormatReleve {
            date: 0,
            montant: None,
            debit: Some(1),
            credit: Some(2),
            reference: Some(3),
        };
        let lignes = vec![
            vec![
                "Date".into(),
                "Débit".into(),
                "Crédit".into(),
                "Réf.".into(),
            ],
            vec![
                "07/10/2026".into(),
                "150 000".into(),
                String::new(),
                "VIR-118".into(),
            ],
            vec!["Solde".into(), String::new(), String::new(), String::new()],
        ];
        let (lues, ignorees) = lire_lignes(&lignes, &format);
        assert_eq!(ignorees, 1);
        assert_eq!(lues, vec![lue(7, -150_000, "VIR-118")]);
    }
}
