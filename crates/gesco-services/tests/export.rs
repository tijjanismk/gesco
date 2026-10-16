//! Export Excel par la frontière (SPEC_PHASE_2 §11, R-41) : le classeur s'écrit où l'écran le
//! demande, se relit, et la commande refuse un nom qui n'est pas un classeur.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]

mod commun;

use calamine::{Data, Reader, Xlsx};
use commun::*;
use serde_json::json;

#[tokio::test]
async fn la_liste_affichee_devient_un_classeur_relisible() {
    let dossier = tempfile::tempdir().unwrap();
    let p = poste_vierge(dossier.path()).await;
    p.installer().await;
    let admin = p.connecter("admin", MOT_DE_PASSE).await;
    let chemin = dossier.path().join("inscriptions.xlsx");
    let demande = json!({
        "chemin": chemin.to_string_lossy(),
        "titre": "Inscriptions 2026/2027",
        "entetes": ["Classe", "Matricule", "Élève"],
        "lignes": [["7e A", "2026-00001", "Traoré Awa"], ["10CG", "L/2014/001", "Diallo Moussa"]],
    });

    assert_eq!(
        p.refus("exporter_xlsx", None, demande.clone()).await.cle,
        "session.invalide"
    );
    let ecrit = p.ok("exporter_xlsx", Some(&admin), demande.clone()).await;
    assert_eq!(ecrit["lignes"], 2);
    assert_eq!(ecrit["chemin"].as_str().unwrap(), chemin.to_string_lossy());

    let mut classeur = Xlsx::new(std::io::Cursor::new(std::fs::read(&chemin).unwrap())).unwrap();
    let plage = classeur.worksheet_range("Inscriptions 2026 2027").unwrap();
    let lignes: Vec<Vec<Data>> = plage.rows().map(<[Data]>::to_vec).collect();
    assert_eq!(lignes.len(), 3);
    assert_eq!(lignes[0][2], Data::String("Élève".to_owned()));
    assert_eq!(lignes[2][1], Data::String("L/2014/001".to_owned()));

    let mut mauvaise_extension = demande.clone();
    mauvaise_extension["chemin"] = json!(dossier.path().join("liste.csv").to_string_lossy());
    assert_eq!(
        p.refus("exporter_xlsx", Some(&admin), mauvaise_extension)
            .await
            .cle,
        "export.extension"
    );
    let mut de_travers = demande;
    de_travers["lignes"] = json!([["7e A", "2026-00001"]]);
    assert_eq!(
        p.refus("exporter_xlsx", Some(&admin), de_travers).await.cle,
        "export.liste_irreguliere"
    );
}
