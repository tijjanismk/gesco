//! Catalogue des commandes vu du réseau local (SPEC_PHASE_11_A §3.3 et §3.4) : toute commande
//! dont l'entrée porte un chemin est déclarée, soit comme lisant ou écrivant un fichier, soit
//! comme réservée au poste principal. Sans cette déclaration, un poste relié enverrait au poste
//! principal un chemin de son propre disque.
//!
//! L'essai lit le contrat généré (`frontend/src/types/gesco.ts`), que `types_typescript` tient
//! égal aux types Rust.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]

use std::collections::{HashMap, HashSet};
use std::path::PathBuf;

use gesco_services::commandes::{COMMANDES, COMMANDES_A_FICHIER, Commande, Fichier, Lieu};

/// Types générés : pour chacun, les noms de ses champs.
pub(crate) fn champs_des_types(texte: &str) -> HashMap<String, HashSet<String>> {
    let mut types = HashMap::new();
    let mut courant: Option<(String, HashSet<String>)> = None;
    for ligne in texte.lines() {
        if let Some(suite) = ligne.strip_prefix("export type ") {
            let nom = suite
                .split(|c: char| !c.is_alphanumeric() && c != '_')
                .next()
                .unwrap()
                .to_owned();
            courant = ligne.ends_with('{').then(|| (nom, HashSet::new()));
        } else if ligne.starts_with("};") {
            if let Some((nom, champs)) = courant.take() {
                types.insert(nom, champs);
            }
        } else if let Some((_, champs)) = courant.as_mut()
            && let Some(champ) = ligne.strip_prefix('\t')
            && !champ.starts_with(['/', ' ', '\t'])
            && let Some((nom, _)) = champ.split_once(": ")
        {
            champs.insert(nom.trim_end_matches('?').to_owned());
        }
    }
    types
}

/// Type d'entrée de chaque commande, tel que le contrat généré l'écrit.
pub(crate) fn entrees_du_contrat(texte: &str) -> HashMap<String, String> {
    texte
        .lines()
        .filter_map(|ligne| {
            let (nom, signature) = ligne.strip_prefix('\t')?.split_once(": Signature<")?;
            let (entree, _) = signature.split_once(", ")?;
            Some((nom.to_owned(), entree.trim_end_matches("[]").to_owned()))
        })
        .collect()
}

pub(crate) fn contrat_genere() -> String {
    let chemin =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../frontend/src/types/gesco.ts");
    std::fs::read_to_string(chemin).unwrap()
}

#[test]
fn toute_commande_qui_recoit_un_chemin_est_declaree() {
    let texte = contrat_genere();
    let types = champs_des_types(&texte);
    let entrees = entrees_du_contrat(&texte);
    assert_eq!(entrees.len(), COMMANDES.len(), "contrat généré incomplet");
    let vide = HashSet::new();
    for commande in COMMANDES {
        let entree = &entrees[commande.nom];
        let champs = types.get(entree).unwrap_or(&vide);
        match commande.fichier() {
            Some(Fichier::Lu(champ) | Fichier::Ecrit(champ)) => assert!(
                champs.contains(champ),
                "{} déclare le champ « {champ} », absent de {entree}",
                commande.nom
            ),
            None => assert!(
                !champs.contains("chemin") || commande.lieu() == Lieu::PostePrincipal,
                "{} reçoit un chemin ({entree}) sans être déclarée dans COMMANDES_A_FICHIER \
                 ni réservée au poste principal",
                commande.nom
            ),
        }
    }
}

#[test]
fn les_declarations_de_fichier_disent_ce_que_la_commande_fait() {
    // Les commandes qui rendent le chemin du fichier écrit sont celles qui écrivent.
    let texte = contrat_genere();
    let ecrivent: HashSet<&str> = texte
        .lines()
        .filter_map(|ligne| {
            let (nom, signature) = ligne.strip_prefix('\t')?.split_once(": Signature<")?;
            signature.ends_with(", FichierEcrit>,").then_some(nom)
        })
        .collect();
    assert!(!ecrivent.is_empty());
    for nom in ecrivent {
        let fichier = Commande::par_nom(nom).and_then(Commande::fichier);
        assert!(
            matches!(fichier, Some(Fichier::Ecrit(_))),
            "{nom} rend un fichier écrit mais n'est pas déclarée comme écrivant"
        );
    }
    // Aucune commande n'est déclarée deux fois.
    let noms: HashSet<&str> = COMMANDES_A_FICHIER.iter().map(|(nom, _)| *nom).collect();
    assert_eq!(noms.len(), COMMANDES_A_FICHIER.len());
}
