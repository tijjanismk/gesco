//! Livre d'un compte de trésorerie (SPEC_PHASE_8 §5) : ses lignes d'entrée et de sortie, son solde
//! — le report d'ouverture, plus les entrées, moins les sorties — et son solde rapproché, celui
//! des seules lignes cochées contre le relevé. Les transferts entre comptes de l'école sortent de
//! l'un et entrent dans l'autre ; une erreur se corrige par le transfert inverse.

use serde::{Deserialize, Serialize};

use crate::commun::{DateCivile, ErreurMontant, Id, Montant};
use crate::core::{Etablissement, Utilisateur};
use crate::finance::{ModePaiement, SessionCaisse};
use crate::tresorerie::{CompteTresorerie, NatureCompte};

/// Longueur maximale du motif et de la référence d'un transfert.
pub const TEXTE_TRANSFERT_MAX: usize = 120;

/// D'où vient une ligne du livre.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "snake_case")]
pub enum OrigineLigne {
    /// Un versement d'un parent.
    Versement,
    /// Un mouvement de la caisse : salaire, avance, dépense, régularisation, transfert.
    Mouvement,
    /// Une dépense payée d'une banque ou d'un compte mobile money, salaires et avances compris.
    Depense,
    /// Un transfert, côté banque ou mobile money.
    Transfert,
    /// Une autre recette sur une banque ou un compte mobile money.
    Recette,
}

impl OrigineLigne {
    /// Toutes les origines.
    pub const TOUTES: [Self; 5] = [
        Self::Versement,
        Self::Mouvement,
        Self::Depense,
        Self::Transfert,
        Self::Recette,
    ];

    /// Nom persisté.
    pub const fn nom(&self) -> &'static str {
        match self {
            Self::Versement => "versement",
            Self::Mouvement => "mouvement",
            Self::Depense => "depense",
            Self::Transfert => "transfert",
            Self::Recette => "recette",
        }
    }

    /// Lecture d'un nom persisté.
    pub fn depuis_nom(nom: &str) -> Option<Self> {
        Self::TOUTES.into_iter().find(|o| o.nom() == nom)
    }
}

/// Une ligne du livre d'un compte.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, specta::Type)]
pub struct LigneLivre {
    /// Jour.
    pub date: DateCivile,
    /// Origine.
    pub origine: OrigineLigne,
    /// Identifiant de l'écriture d'origine.
    pub origine_id: String,
    /// Ce qui entre ou sort.
    pub libelle: String,
    /// Référence de la transaction.
    pub reference: String,
    /// Entrée.
    pub entree: Montant,
    /// Sortie.
    pub sortie: Montant,
    /// Cochée contre le relevé.
    pub pointee: bool,
}

/// Soldes d'un compte.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, specta::Type)]
pub struct Soldes {
    /// Le report d'ouverture, plus les entrées, moins les sorties.
    pub livre: Montant,
    /// Le report d'ouverture et les seules lignes cochées.
    pub rapproche: Montant,
    /// Lignes à cocher.
    pub a_pointer: u32,
}

/// Soldes d'un compte d'après ses lignes.
pub fn soldes(ouverture: Montant, lignes: &[LigneLivre]) -> Result<Soldes, ErreurMontant> {
    let mut livre = ouverture;
    let mut rapproche = ouverture;
    let mut a_pointer = 0u32;
    for l in lignes {
        let net = l.entree.soustraire(l.sortie)?;
        livre = livre.ajouter(net)?;
        if l.pointee {
            rapproche = rapproche.ajouter(net)?;
        } else {
            a_pointer = a_pointer.saturating_add(1);
        }
    }
    Ok(Soldes {
        livre,
        rapproche,
        a_pointer,
    })
}

/// Compte où arrive l'argent d'un mode : les espèces dans la caisse ; ailleurs, le compte que
/// l'école a choisi pour ce mode, à défaut le premier compte actif de la nature du mode.
pub fn compte_du_mode(
    mode: ModePaiement,
    comptes: &[CompteTresorerie],
    choix: &[(ModePaiement, Id<CompteTresorerie>)],
) -> Option<Id<CompteTresorerie>> {
    let nature = match mode {
        ModePaiement::Especes => NatureCompte::Caisse,
        ModePaiement::Virement | ModePaiement::Cheque => NatureCompte::Banque,
        ModePaiement::OrangeMoney | ModePaiement::Wave | ModePaiement::Moov => {
            NatureCompte::MobileMoney
        }
    };
    let actif = |id: Id<CompteTresorerie>| {
        comptes
            .iter()
            .any(|c| c.id == id && c.actif && c.nature == nature)
    };
    choix
        .iter()
        .find(|(m, id)| *m == mode && actif(*id))
        .map(|(_, id)| *id)
        .or_else(|| {
            comptes
                .iter()
                .find(|c| c.actif && c.nature == nature)
                .map(|c| c.id)
        })
}

/// Transfert d'un compte de l'école à un autre : un dépôt de la caisse à la banque, un retrait,
/// un rechargement du compte mobile money.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
pub struct Transfert {
    /// Identifiant technique.
    pub id: Id<Transfert>,
    /// Établissement.
    pub etablissement_id: Id<Etablissement>,
    /// Jour.
    pub date: DateCivile,
    /// Compte d'où l'argent sort.
    pub source_id: Id<CompteTresorerie>,
    /// Compte où il entre.
    pub destination_id: Id<CompteTresorerie>,
    /// Montant.
    pub montant: Montant,
    /// Référence : bordereau de dépôt, numéro de chèque, transaction.
    pub reference: String,
    /// Motif.
    pub motif: String,
    /// Session de caisse touchée, quand la caisse est l'un des deux comptes.
    pub session_id: Option<Id<SessionCaisse>>,
    /// Qui l'a fait.
    pub fait_par: Id<Utilisateur>,
}

/// Vérifie un transfert : deux comptes actifs et distincts, un montant positif, une référence et
/// un motif.
pub fn verifier_transfert(
    source: &CompteTresorerie,
    destination: &CompteTresorerie,
    montant: Montant,
    reference: &str,
    motif: &str,
) -> Result<(), &'static str> {
    if source.id == destination.id {
        return Err("transfert_compte.meme_compte");
    }
    if !source.actif || !destination.actif {
        return Err("transfert_compte.compte_ferme");
    }
    if montant.francs() <= 0 {
        return Err("transfert_compte.montant_invalide");
    }
    for (texte, cle) in [
        (reference, "transfert_compte.reference_obligatoire"),
        (motif, "transfert_compte.motif_obligatoire"),
    ] {
        let t = texte.trim();
        if t.is_empty() || t.chars().count() > TEXTE_TRANSFERT_MAX {
            return Err(cle);
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn f(n: i64) -> Montant {
        Montant::depuis_francs(n)
    }

    fn jour(j: u8) -> DateCivile {
        DateCivile::nouvelle(2026, 10, j).unwrap_or_else(|| unreachable!())
    }

    fn compte(octet: u8, nature: NatureCompte) -> CompteTresorerie {
        CompteTresorerie {
            id: Id::depuis_octets([octet; 16]),
            etablissement_id: Id::depuis_octets([9; 16]),
            nature,
            libelle: format!("Compte {octet}"),
            organisme: String::new(),
            numero: String::new(),
            solde_ouverture: Montant::ZERO,
            ouvert_le: jour(1),
            actif: true,
        }
    }

    #[test]
    fn le_solde_rapproche_ne_compte_que_les_lignes_cochees() {
        let ligne = |entree, sortie, pointee| LigneLivre {
            date: jour(5),
            origine: OrigineLigne::Depense,
            origine_id: String::new(),
            libelle: String::new(),
            reference: String::new(),
            entree: f(entree),
            sortie: f(sortie),
            pointee,
        };
        let s = soldes(
            f(250_000),
            &[
                ligne(100_000, 0, true),
                ligne(0, 150_000, false),
                ligne(0, 2_500, true),
            ],
        );
        assert_eq!(
            s,
            Ok(Soldes {
                livre: f(197_500),
                rapproche: f(347_500),
                a_pointer: 1,
            })
        );
    }

    #[test]
    fn l_argent_d_un_mode_arrive_au_compte_choisi_sinon_au_premier_de_sa_nature() {
        let caisse = compte(1, NatureCompte::Caisse);
        let bdm = compte(2, NatureCompte::Banque);
        let bms = compte(3, NatureCompte::Banque);
        let orange = compte(4, NatureCompte::MobileMoney);
        let comptes = [caisse.clone(), bdm.clone(), bms.clone(), orange.clone()];
        assert_eq!(
            compte_du_mode(ModePaiement::Especes, &comptes, &[]),
            Some(caisse.id)
        );
        assert_eq!(
            compte_du_mode(ModePaiement::Cheque, &comptes, &[]),
            Some(bdm.id)
        );
        assert_eq!(
            compte_du_mode(
                ModePaiement::Cheque,
                &comptes,
                &[(ModePaiement::Cheque, bms.id)]
            ),
            Some(bms.id)
        );
        // Un choix qui ne convient pas au mode est ignoré.
        assert_eq!(
            compte_du_mode(
                ModePaiement::Wave,
                &comptes,
                &[(ModePaiement::Wave, bdm.id)]
            ),
            Some(orange.id)
        );
        assert_eq!(compte_du_mode(ModePaiement::Wave, &comptes[..3], &[]), None);
    }

    #[test]
    fn un_transfert_relie_deux_comptes_actifs_distincts() {
        let caisse = compte(1, NatureCompte::Caisse);
        let mut bdm = compte(2, NatureCompte::Banque);
        let ok = |s: &CompteTresorerie, d: &CompteTresorerie| {
            verifier_transfert(s, d, f(300_000), "BV-0042", "Dépôt de la recette")
        };
        assert_eq!(ok(&caisse, &bdm), Ok(()));
        assert_eq!(ok(&caisse, &caisse), Err("transfert_compte.meme_compte"));
        assert_eq!(
            verifier_transfert(&caisse, &bdm, f(300_000), " ", "Dépôt"),
            Err("transfert_compte.reference_obligatoire")
        );
        bdm.actif = false;
        assert_eq!(ok(&caisse, &bdm), Err("transfert_compte.compte_ferme"));
    }
}
