//! Tests du moteur documentaire (spécification §14, ligne « Documents ») : composition,
//! empreinte, version de modèle, variante d'établissement, refus d'un document incomplet, lot.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]

use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Instant;

use gesco_docs::{
    Annulation, Catalogue, ElementLot, ErreurDocument, FormatLogo, Logo, Moteur, Variante,
};
use serde_json::json;
use sha2::{Digest, Sha256};

/// PNG 4×4 uni, aux couleurs du jeton `primaire`.
const LOGO_PNG: &[u8] = &[
    0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a, 0x00, 0x00, 0x00, 0x0d, 0x49, 0x48, 0x44, 0x52,
    0x00, 0x00, 0x00, 0x04, 0x00, 0x00, 0x00, 0x04, 0x08, 0x02, 0x00, 0x00, 0x00, 0x26, 0x93, 0x09,
    0x29, 0x00, 0x00, 0x00, 0x10, 0x49, 0x44, 0x41, 0x54, 0x78, 0x9c, 0x63, 0x90, 0xf7, 0xde, 0x01,
    0x47, 0x0c, 0xc4, 0x71, 0x00, 0xcd, 0xf3, 0x12, 0x21, 0x9d, 0x33, 0xc3, 0xa1, 0x00, 0x00, 0x00,
    0x00, 0x49, 0x45, 0x4e, 0x44, 0xae, 0x42, 0x60, 0x82,
];

fn moteur() -> Moteur {
    Moteur::nouveau(Catalogue::integre().unwrap())
}

fn variante(nom: &str) -> Variante {
    Variante {
        nom: nom.to_owned(),
        adresse: "Hamdallaye ACI 2000, Bamako".to_owned(),
        telephones: vec!["+223 20 22 00 00".to_owned()],
        mentions: "Établissement privé agréé".to_owned(),
        logo: None,
        papier: gesco_docs::Papier::A4,
        largeurs: Default::default(),
    }
}

fn donnees_essai(titre: &str) -> serde_json::Value {
    json!({
        "titre": titre,
        "date": "4 octobre 2026",
        "auteur": "Administrateur",
        "lignes": [
            { "libelle": "Version du schéma", "valeur": "2" },
            { "libelle": "Poste", "valeur": "secrétariat" }
        ]
    })
}

#[test]
fn compose_un_pdf_avec_son_empreinte_et_sa_version() {
    let moteur = moteur();
    let document = moteur
        .composer("essai", &variante("Lycée Askia"), &donnees_essai("Essai"))
        .unwrap();
    assert!(document.pdf.starts_with(b"%PDF-"));
    assert_eq!(
        document.empreinte_sha256,
        hex::encode(Sha256::digest(&document.pdf))
    );
    assert_eq!(document.modele_id, "essai");
    assert_eq!(document.modele_version, 1);
}

#[test]
fn la_composition_est_reproductible() {
    let moteur = moteur();
    let a = moteur
        .composer("essai", &variante("Lycée Askia"), &donnees_essai("Essai"))
        .unwrap();
    let b = moteur
        .composer("essai", &variante("Lycée Askia"), &donnees_essai("Essai"))
        .unwrap();
    assert_eq!(a.empreinte_sha256, b.empreinte_sha256);
}

#[test]
fn une_variable_requise_absente_est_refusee() {
    let moteur = moteur();
    let mut donnees = donnees_essai("Essai");
    donnees.as_object_mut().unwrap().remove("auteur");
    let erreur = moteur
        .composer("essai", &variante("Lycée Askia"), &donnees)
        .unwrap_err();
    assert!(
        matches!(&erreur, ErreurDocument::VariableManquante { variable, .. } if variable == "auteur"),
        "{erreur:?}"
    );
    donnees
        .as_object_mut()
        .unwrap()
        .insert("auteur".to_owned(), serde_json::Value::Null);
    assert!(matches!(
        moteur.composer("essai", &variante("Lycée Askia"), &donnees),
        Err(ErreurDocument::VariableManquante { .. })
    ));
}

#[test]
fn une_variable_non_declaree_ou_reservee_est_refusee() {
    let moteur = moteur();
    let mut donnees = donnees_essai("Essai");
    donnees
        .as_object_mut()
        .unwrap()
        .insert("autuer".to_owned(), json!("faute de frappe"));
    assert!(matches!(
        moteur.composer("essai", &variante("Lycée Askia"), &donnees),
        Err(ErreurDocument::VariableInconnue { .. })
    ));
    let mut donnees = donnees_essai("Essai");
    donnees
        .as_object_mut()
        .unwrap()
        .insert("etablissement".to_owned(), json!({}));
    assert!(matches!(
        moteur.composer("essai", &variante("Lycée Askia"), &donnees),
        Err(ErreurDocument::VariableInconnue { .. })
    ));
}

#[test]
fn la_variante_change_le_document_sans_changer_le_modele() {
    let moteur = moteur();
    let a = moteur
        .composer("essai", &variante("Lycée Askia"), &donnees_essai("Essai"))
        .unwrap();
    let mut avec_logo = variante("Groupe scolaire Kankou Moussa");
    avec_logo.logo = Some(Logo {
        octets: LOGO_PNG.to_vec(),
        format: FormatLogo::Png,
    });
    let b = moteur
        .composer("essai", &avec_logo, &donnees_essai("Essai"))
        .unwrap();
    assert_ne!(a.empreinte_sha256, b.empreinte_sha256);
    assert_eq!(
        (a.modele_id.as_str(), a.modele_version),
        (b.modele_id.as_str(), b.modele_version)
    );
}

#[test]
fn certificat_et_numero_annule_se_composent() {
    let moteur = moteur();
    let mots: Vec<String> = (1..=12).map(|i| format!("mot{i}")).collect();
    let certificat = moteur
        .composer(
            "certificat_cle_secours",
            &variante("Lycée Askia"),
            &json!({
                "date": "4 octobre 2026",
                "enveloppe_id": "01K6PZ3T9H5V2QW8E4R7Y6U1IO",
                "mots": mots,
                "emis_par": "Administrateur",
            }),
        )
        .unwrap();
    assert!(certificat.pdf.starts_with(b"%PDF-"));
    let annule = moteur
        .composer(
            "numero_annule",
            &variante("Lycée Askia"),
            &json!({
                "numero": "REC-2026-000147",
                "type_document": "recu",
                "exercice": "2026",
                "motif": "erreur de saisie",
                "date": "4 octobre 2026",
            }),
        )
        .unwrap();
    assert_eq!(annule.modele_id, "numero_annule");
}

#[test]
fn un_modele_sur_mesure_remplace_le_modele_de_base() {
    let dossier = tempfile::tempdir().unwrap();
    let propre = dossier.path().join("essai");
    std::fs::create_dir_all(&propre).unwrap();
    std::fs::write(
        propre.join("modele.json"),
        r#"{ "id": "essai", "version": 7, "libelle": "Essai sur mesure",
             "variables": [ { "nom": "titre", "requise": true } ] }"#,
    )
    .unwrap();
    std::fs::write(
        propre.join("modele.typ"),
        "#let d = json(\"/donnees.json\")\n#set text(font: \"Charis SIL\")\n= #d.titre\n",
    )
    .unwrap();
    let mut catalogue = Catalogue::integre().unwrap();
    assert_eq!(catalogue.surcharger_depuis(dossier.path()).unwrap(), 1);
    let moteur = Moteur::nouveau(catalogue);
    let document = moteur
        .composer(
            "essai",
            &variante("Lycée Askia"),
            &json!({ "titre": "Sur mesure" }),
        )
        .unwrap();
    assert_eq!(document.modele_version, 7);
}

fn elements(nombre: usize) -> Vec<ElementLot> {
    (0..nombre)
        .map(|i| ElementLot {
            cle: format!("eleve-{i:04}"),
            donnees: donnees_essai(&format!("Bulletin d'essai n° {i}")),
        })
        .collect()
}

#[test]
fn un_lot_refuse_tout_si_un_element_est_incomplet() {
    let moteur = moteur();
    let dossier = tempfile::tempdir().unwrap();
    let mut lot = elements(3);
    lot[2].donnees = json!({ "titre": "incomplet" });
    let erreur = moteur
        .composer_lot(
            "essai",
            &variante("Lycée Askia"),
            &lot,
            dossier.path(),
            &Annulation::default(),
            &|_| {},
        )
        .unwrap_err();
    assert!(matches!(erreur, ErreurDocument::VariableManquante { .. }));
    assert_eq!(std::fs::read_dir(dossier.path()).unwrap().count(), 0);
}

#[test]
fn un_lot_annule_reprend_sans_recomposer_ce_qui_est_fait() {
    let moteur = moteur();
    let dossier = tempfile::tempdir().unwrap();
    let lot = elements(12);
    let annulation = Annulation::default();
    let premier = moteur
        .composer_lot(
            "essai",
            &variante("Lycée Askia"),
            &lot,
            dossier.path(),
            &annulation,
            &|p| {
                if p.traites >= 4 {
                    annulation.annuler();
                }
            },
        )
        .unwrap();
    assert!(premier.annule);
    assert!(premier.documents.len() >= 4 && premier.documents.len() < 12);

    let vus = AtomicUsize::new(0);
    let second = moteur
        .composer_lot(
            "essai",
            &variante("Lycée Askia"),
            &lot,
            dossier.path(),
            &Annulation::default(),
            &|p| {
                vus.fetch_max(p.traites, Ordering::SeqCst);
            },
        )
        .unwrap();
    assert!(!second.annule);
    assert_eq!(second.documents.len(), 12);
    assert_eq!(vus.load(Ordering::SeqCst), 12);
    let repris = second.documents.iter().filter(|d| d.repris).count();
    assert_eq!(repris, premier.documents.len());
    for (avant, apres) in premier.documents.iter().zip(&second.documents) {
        assert_eq!(avant.empreinte_sha256, apres.empreinte_sha256);
    }
}

/// Cible de la spécification §10.2 : 400 documents en moins de deux minutes. Mesurée en
/// `--release`, seul profil représentatif ; la CI la lance avec `cargo test --release`.
#[test]
#[cfg_attr(debug_assertions, ignore = "cible de performance mesurée en --release")]
fn lot_de_400_documents_en_moins_de_deux_minutes() {
    let moteur = moteur();
    let dossier = tempfile::tempdir().unwrap();
    let lot = elements(400);
    let debut = Instant::now();
    let bilan = moteur
        .composer_lot(
            "essai",
            &variante("Lycée Askia"),
            &lot,
            dossier.path(),
            &Annulation::default(),
            &|_| {},
        )
        .unwrap();
    let duree = debut.elapsed();
    assert_eq!(bilan.documents.len(), 400);
    assert!(bilan.documents.iter().all(|d| !d.repris && d.taille > 0));
    assert!(duree.as_secs() < 120, "lot de 400 en {duree:?}");
}
