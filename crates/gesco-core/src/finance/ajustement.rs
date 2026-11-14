//! Ajustement des frais au changement de classe (décision du porteur du 2026-11-13) : la
//! direction garde les frais actuels ou applique ceux de la nouvelle classe. Rien ne se réécrit :
//! sur les échéances à venir, l'écart par tranche devient une hausse (une ligne de facture en
//! plus) ou une baisse (des parts portées par les échéances, comme une remise) ; ce que la baisse
//! ne peut pas porter parce que c'est déjà payé devient un avoir de l'élève.

use serde::{Deserialize, Serialize};

use crate::commun::{DateCivile, ErreurMontant, Id, Montant};
use crate::core::Etablissement;
use crate::finance::facture::{Echeance, Facture};
use crate::finance::tarif::TypeFrais;
use crate::scolarite::Classe;

/// Tranche d'un frais payé selon le plan : ce qui est dû, à quelle date.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TrancheDue {
    /// Nature du frais.
    pub type_frais: TypeFrais,
    /// Date d'échéance.
    pub date: DateCivile,
    /// Libellé : « Scolarité · 2e tranche ».
    pub libelle: String,
    /// Montant dû.
    pub montant: Montant,
}

/// Écart d'une tranche à venir entre les frais actuels et ceux de la nouvelle classe.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, specta::Type)]
pub struct EcartTranche {
    /// Nature du frais.
    pub type_frais: TypeFrais,
    /// Date d'échéance.
    pub date: DateCivile,
    /// Libellé de la tranche.
    pub libelle: String,
    /// Dû aujourd'hui.
    pub avant: Montant,
    /// Dû aux frais de la nouvelle classe.
    pub apres: Montant,
}

impl EcartTranche {
    /// Écart signé : positif pour une hausse, négatif pour une baisse.
    pub fn ecart(&self) -> Result<Montant, ErreurMontant> {
        self.apres.soustraire(self.avant)
    }
}

/// Mois d'une tranche : une échéance déplacée à un autre jour du mois reste la même tranche.
pub fn meme_tranche(a: DateCivile, b: DateCivile) -> bool {
    a.annee() == b.annee() && a.mois() == b.mois()
}

/// Compare, tranche par tranche (nature du frais et mois), ce qui est dû aujourd'hui et ce que
/// les frais de la nouvelle classe demanderaient, pour les tranches dont l'échéance est au jour
/// du changement ou après : le passé reste au tarif de l'ancienne classe. La date d'une tranche
/// est celle de l'échéance actuelle, qui garde le jour choisi pour l'élève. Seules les tranches
/// qui changent sont rendues, par date puis par nature du frais.
pub fn comparer(
    avant: &[TrancheDue],
    apres: &[TrancheDue],
    depuis: DateCivile,
) -> Result<Vec<EcartTranche>, ErreurMontant> {
    let mut cles: Vec<(DateCivile, TypeFrais)> = Vec::new();
    for t in avant.iter().chain(apres) {
        let date = avant
            .iter()
            .find(|a| a.type_frais == t.type_frais && meme_tranche(a.date, t.date))
            .map_or(t.date, |a| a.date);
        if date >= depuis && !cles.contains(&(date, t.type_frais)) {
            cles.push((date, t.type_frais));
        }
    }
    cles.sort_by_key(|(date, frais)| (*date, TypeFrais::TOUS.iter().position(|f| f == frais)));
    let somme = |tranches: &[TrancheDue], date: DateCivile, frais: TypeFrais| {
        tranches
            .iter()
            .filter(|t| t.type_frais == frais && meme_tranche(t.date, date))
            .try_fold(Montant::ZERO, |s, t| s.ajouter(t.montant))
    };
    let mut ecarts = Vec::new();
    for (date, type_frais) in cles {
        let du_avant = somme(avant, date, type_frais)?;
        let du_apres = somme(apres, date, type_frais)?;
        if du_avant == du_apres {
            continue;
        }
        let libelle = apres
            .iter()
            .chain(avant)
            .find(|t| t.type_frais == type_frais && meme_tranche(t.date, date))
            .map(|t| t.libelle.clone())
            .unwrap_or_default();
        ecarts.push(EcartTranche {
            type_frais,
            date,
            libelle,
            avant: du_avant,
            apres: du_apres,
        });
    }
    Ok(ecarts)
}

/// Totaux d'un écart : hausse et baisse, toutes deux positives.
pub fn totaux(ecarts: &[EcartTranche]) -> Result<(Montant, Montant), ErreurMontant> {
    let mut hausse = Montant::ZERO;
    let mut baisse = Montant::ZERO;
    for e in ecarts {
        let ecart = e.ecart()?;
        if ecart.est_negatif() {
            baisse = baisse.ajouter(ecart.oppose()?)?;
        } else {
            hausse = hausse.ajouter(ecart)?;
        }
    }
    Ok((hausse, baisse))
}

/// Montants portés par des échéances : (échéance, montant).
pub type Parts = Vec<(Id<Echeance>, Montant)>;

/// Porte une baisse sur les échéances d'une tranche, données avec leur reste à payer : chacune
/// en prend autant que son reste ; ce qui ne trouve plus de reste, parce que c'est déjà payé,
/// devient un avoir, rattaché à l'échéance qui l'a fait naître. Rend les parts et les avoirs.
pub fn porter_baisse(
    restes: &[(Id<Echeance>, Montant)],
    baisse: Montant,
) -> Result<(Parts, Parts), ErreurMontant> {
    let mut a_porter = baisse;
    let mut parts = Vec::new();
    for (id, reste) in restes.iter().rev() {
        if a_porter.est_nul() {
            break;
        }
        let part = (*reste).max(Montant::ZERO).min(a_porter);
        if !part.est_nul() {
            parts.push((*id, part));
            a_porter = a_porter.soustraire(part)?;
        }
    }
    let avoirs = match (a_porter.est_nul(), restes.last()) {
        (false, Some((id, _))) => vec![(*id, a_porter)],
        _ => Vec::new(),
    };
    Ok((parts, avoirs))
}

/// Ajustement enregistré sur une facture.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
pub struct Ajustement {
    /// Identifiant technique.
    pub id: Id<Ajustement>,
    /// Établissement propriétaire.
    pub etablissement_id: Id<Etablissement>,
    /// Facture ajustée.
    pub facture_id: Id<Facture>,
    /// Jour de l'ajustement.
    pub date: DateCivile,
    /// Hausse : la ligne de facture en plus.
    pub hausse: Montant,
    /// Baisse, avoir compris.
    pub baisse: Montant,
    /// Part de la baisse déjà payée : l'avoir né de l'ajustement.
    pub avoir: Montant,
    /// Classe quittée.
    pub classe_avant_id: Id<Classe>,
    /// Classe rejointe.
    pub classe_apres_id: Id<Classe>,
    /// Vrai si un changement de plan l'a appliqué de nouveau : il reste dans l'historique.
    pub remplace: bool,
}

/// Avoir d'un élève : un trop-perçu, jamais effacé, qui règle ses échéances sans encaissement.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
pub struct Avoir {
    /// Identifiant technique.
    pub id: Id<Avoir>,
    /// Ajustement d'où il vient.
    pub ajustement_id: Id<Ajustement>,
    /// Échéance payée qui l'a fait naître.
    pub echeance_id: Id<Echeance>,
    /// Montant de l'avoir.
    pub montant: Montant,
    /// Déjà utilisé.
    pub utilise: Montant,
}

impl Avoir {
    /// Ce qui reste à utiliser.
    pub fn solde(&self) -> Result<Montant, ErreurMontant> {
        self.montant.soustraire(self.utilise)
    }
}

/// Règle des échéances avec un avoir : la plus ancienne d'abord, jamais plus que son reste.
/// Rend les parts imputées.
pub fn utiliser_avoir(
    restes: &[(Id<Echeance>, Montant)],
    solde: Montant,
) -> Result<Vec<(Id<Echeance>, Montant)>, ErreurMontant> {
    let mut a_imputer = solde;
    let mut parts = Vec::new();
    for (id, reste) in restes {
        if a_imputer.est_nul() {
            break;
        }
        let part = (*reste).max(Montant::ZERO).min(a_imputer);
        if !part.est_nul() {
            parts.push((*id, part));
            a_imputer = a_imputer.soustraire(part)?;
        }
    }
    Ok(parts)
}

#[cfg(test)]
#[allow(clippy::indexing_slicing)]
mod tests {
    use super::*;

    fn jour(m: u8, j: u8) -> DateCivile {
        let annee = if m >= 9 { 2026 } else { 2027 };
        DateCivile::nouvelle(annee, m, j).unwrap_or_else(|| unreachable!())
    }

    fn tranche(frais: TypeFrais, date: DateCivile, montant: i64) -> TrancheDue {
        TrancheDue {
            type_frais: frais,
            date,
            libelle: format!("{} {date}", frais.nom()),
            montant: Montant::depuis_francs(montant),
        }
    }

    fn id(n: u128) -> Id<Echeance> {
        Id::depuis_ulid(ulid::Ulid::from(n))
    }

    #[test]
    fn seules_les_tranches_a_venir_qui_changent_comptent() {
        let avant = [
            tranche(TypeFrais::Scolarite, jour(10, 5), 50_000),
            tranche(TypeFrais::Scolarite, jour(1, 5), 50_000),
            tranche(TypeFrais::Scolarite, jour(4, 5), 50_000),
            tranche(TypeFrais::Cantine, jour(4, 5), 10_000),
        ];
        let apres = [
            tranche(TypeFrais::Scolarite, jour(10, 5), 40_000),
            tranche(TypeFrais::Scolarite, jour(1, 5), 40_000),
            tranche(TypeFrais::Scolarite, jour(4, 5), 40_000),
            tranche(TypeFrais::Cantine, jour(4, 5), 10_000),
        ];
        let ecarts = comparer(&avant, &apres, jour(1, 5)).unwrap_or_default();
        assert_eq!(ecarts.len(), 2);
        assert_eq!(ecarts[0].date, jour(1, 5));
        assert_eq!(ecarts[0].ecart(), Ok(Montant::depuis_francs(-10_000)));
        assert_eq!(
            totaux(&ecarts),
            Ok((Montant::ZERO, Montant::depuis_francs(20_000)))
        );
        // Mêmes frais : aucun écart.
        assert_eq!(comparer(&avant, &avant, jour(9, 1)), Ok(Vec::new()));
        // Échéances de l'élève au 20 du mois : la tranche reste celle du mois, à sa date.
        let au_vingt = [tranche(TypeFrais::Scolarite, jour(4, 20), 50_000)];
        let ecarts = comparer(&au_vingt, &apres[2..3], jour(4, 10)).unwrap_or_default();
        assert_eq!(ecarts.len(), 1);
        assert_eq!(ecarts[0].date, jour(4, 20));
        assert_eq!(ecarts[0].ecart(), Ok(Montant::depuis_francs(-10_000)));
    }

    #[test]
    fn une_tranche_nouvelle_ou_disparue_compte_en_entier() {
        let avant = [tranche(TypeFrais::Scolarite, jour(4, 5), 50_000)];
        let apres = [
            tranche(TypeFrais::Scolarite, jour(4, 5), 60_000),
            tranche(TypeFrais::Examen, jour(4, 5), 7_500),
        ];
        let ecarts = comparer(&avant, &apres, jour(2, 1)).unwrap_or_default();
        assert_eq!(
            totaux(&ecarts),
            Ok((Montant::depuis_francs(17_500), Montant::ZERO))
        );
        let inverse = comparer(&apres, &avant, jour(2, 1)).unwrap_or_default();
        assert_eq!(
            totaux(&inverse),
            Ok((Montant::ZERO, Montant::depuis_francs(17_500)))
        );
    }

    #[test]
    fn la_baisse_deja_payee_devient_un_avoir() {
        let restes = [
            (id(1), Montant::depuis_francs(30_000)),
            (id(2), Montant::depuis_francs(5_000)),
        ];
        let (parts, avoirs) =
            porter_baisse(&restes, Montant::depuis_francs(20_000)).unwrap_or_default();
        assert_eq!(
            parts,
            vec![
                (id(2), Montant::depuis_francs(5_000)),
                (id(1), Montant::depuis_francs(15_000))
            ]
        );
        assert!(avoirs.is_empty());
        let payee = [(id(3), Montant::ZERO)];
        let (parts, avoirs) =
            porter_baisse(&payee, Montant::depuis_francs(10_000)).unwrap_or_default();
        assert!(parts.is_empty());
        assert_eq!(avoirs, vec![(id(3), Montant::depuis_francs(10_000))]);
    }

    #[test]
    fn un_avoir_regle_les_plus_anciennes_d_abord() {
        let restes = [
            (id(1), Montant::depuis_francs(3_000)),
            (id(2), Montant::depuis_francs(8_000)),
        ];
        assert_eq!(
            utiliser_avoir(&restes, Montant::depuis_francs(5_000)),
            Ok(vec![
                (id(1), Montant::depuis_francs(3_000)),
                (id(2), Montant::depuis_francs(2_000))
            ])
        );
        assert_eq!(
            utiliser_avoir(&restes, Montant::depuis_francs(50_000))
                .map(|p| p.iter().map(|(_, m)| m.francs()).sum::<i64>()),
            Ok(11_000)
        );
    }
}
