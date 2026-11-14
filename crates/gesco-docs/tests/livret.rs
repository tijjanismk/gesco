//! Modèle `livret` (SPEC_PHASE_11_B §5) : un livret complet sur deux trimestres, avec photo,
//! commentaire, appréciation, absences et décision, et un livret sans rien, se composent ; le
//! second n'imprime ni cadre de photo, ni domaine, ni ligne d'absences. Relecture à l'œil :
//! `cargo test -p gesco-docs --features apercu-svg --test livret` avec
//! `GESCO_LIVRET_SVG=<dossier>` y dépose chaque page en SVG.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]

use gesco_docs::{Catalogue, Moteur, Piece, Variante, qr};
use serde_json::{Value, json};

/// Petite image PNG, à la place d'une photo : le logo d'essai des scénarios de bout en bout.
const PNG: &[u8] = include_bytes!("../../../frontend/e2e/logo-essai.png");

fn variante() -> Variante {
    Variante {
        nom: "École fondamentale Askia".to_owned(),
        adresse: "Hamdallaye ACI 2000, Bamako".to_owned(),
        telephones: vec!["+223 20 22 00 00".to_owned()],
        mentions: String::new(),
        logo: None,
        papier: gesco_docs::Papier::A4,
        largeurs: Default::default(),
    }
}

fn n(libelle: &str, picto: &str) -> Value {
    json!({ "libelle": libelle, "picto": picto })
}

fn echelle() -> Value {
    json!([
        n("Acquis", "plein"),
        n("En cours", "demi"),
        n("Non acquis", "vide")
    ])
}

fn complet(qr: &Piece, photo: &Piece) -> Value {
    json!({
        "eleve": "Aminata Diarra", "naissance": "2 mars 2022", "classe": "MS A",
        "section": "Moyenne section", "annee": "2026-2027", "periode": "2e trimestre",
        "photo": photo.chemin(), "periodes": ["1er trimestre", "2e trimestre"], "echelle": echelle(),
        "domaines": [
            { "libelle": "Langage", "commentaire": "Parle volontiers devant le groupe.", "competences": [
                { "libelle": "Décrit une image ou un objet avec ses propres mots",
                  "niveaux": [n("En cours", "demi"), n("Acquis", "plein")] },
                { "libelle": "Reconnaît son prénom écrit", "niveaux": [Value::Null, n("Non acquis", "vide")] },
            ]},
            { "libelle": "Motricité", "commentaire": "", "competences": [
                { "libelle": "Saute à pieds joints", "niveaux": [n("Acquis", "plein"), n("Acquis", "plein")] },
            ]},
        ],
        "appreciation_generale": "Un bon trimestre, curieuse de tout.",
        "absences": 3, "decision": "Passe dans la section suivante", "etat": "",
        "qr": qr.chemin(), "reference": "01J9ZQ3W5N6K8T0V2X4Y6Z8A0B",
    })
}

fn sans_rien(qr: &Piece) -> Value {
    json!({
        "eleve": "Bakary Keïta", "naissance": null, "classe": "MS A", "section": "Moyenne section",
        "annee": "2026-2027", "periode": "1er trimestre", "photo": null, "periodes": ["1er trimestre"],
        "echelle": echelle(), "domaines": [], "appreciation_generale": "", "absences": null,
        "decision": null, "etat": "Projet — livret non publié", "qr": qr.chemin(),
        "reference": "01J9ZQ3W5N6K8T0V2X4Y6Z8A0C",
    })
}

#[test]
fn le_livret_se_compose_avec_ou_sans_ce_qui_existe() {
    let moteur = Moteur::nouveau(Catalogue::integre().unwrap());
    let qr = qr::piece(&qr::reference("01J9ZQ3W5N6K8T0V2X4Y6Z8A0B")).unwrap();
    let photo = Piece {
        nom: "photo-0.png".to_owned(),
        octets: PNG.to_vec(),
    };
    let donnees = json!({ "livrets": [complet(&qr, &photo), sans_rien(&qr)] });
    let pieces = [qr, photo];
    let document = moteur
        .composer_avec("livret", &variante(), &donnees, &pieces)
        .unwrap();
    assert!(document.pdf.starts_with(b"%PDF"));
    assert_eq!(document.modele_version, 1);
    #[cfg(feature = "apercu-svg")]
    if let Some(dossier) = std::env::var_os("GESCO_LIVRET_SVG") {
        let pages = moteur
            .composer_svg("livret", &variante(), &donnees, &pieces)
            .unwrap();
        assert_eq!(pages.len(), 2);
        for (i, page) in pages.iter().enumerate() {
            std::fs::write(
                std::path::Path::new(&dossier).join(format!("livret-{}.svg", i + 1)),
                page,
            )
            .unwrap();
        }
    }
}
