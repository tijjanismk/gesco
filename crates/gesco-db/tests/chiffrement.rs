//! Chiffrement de la base par SQLite3 Multiple Ciphers (assertion 2 de la phase 0.5, décision du
//! porteur du projet du 10 octobre 2026) : le fichier n'est jamais lisible sans la clé, une base
//! en clair se chiffre à sa première ouverture avec clé, l'instantané de sauvegarde reste chiffré.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::path::Path;

use gesco_db::base::est_en_clair;
use gesco_db::{Base, OptionsBase};

const CLE: &str = "8f3a1c0d9e7b6a5f4e3d2c1b0a99887766554433221100ffeeddccbbaa998877";
const AUTRE_CLE: &str = "0000000000000000000000000000000000000000000000000000000000000001";

async fn ecrire_temoin(base: &Base) {
    sqlx::query("CREATE TABLE temoin (nom TEXT NOT NULL)")
        .execute(base.pool_critique())
        .await
        .unwrap();
    sqlx::query("INSERT INTO temoin (nom) VALUES ('Awa Traoré, 7e A')")
        .execute(base.pool_critique())
        .await
        .unwrap();
}

async fn lire_temoin(base: &Base) -> String {
    sqlx::query_scalar("SELECT nom FROM temoin")
        .fetch_one(base.pool())
        .await
        .unwrap()
}

fn contient(chemin: &Path, motif: &[u8]) -> bool {
    std::fs::read(chemin)
        .unwrap()
        .windows(motif.len())
        .any(|w| w == motif)
}

#[tokio::test]
async fn une_base_nait_chiffree_et_ne_s_ouvre_qu_avec_sa_cle() {
    let dossier = tempfile::tempdir().unwrap();
    let chemin = dossier.path().join("gesco.db");
    let base = Base::ouvrir(OptionsBase::fichier(&chemin).avec_cle_hex(CLE))
        .await
        .unwrap();
    ecrire_temoin(&base).await;
    base.consolider_wal().await.unwrap();
    base.fermer().await;

    assert!(!est_en_clair(&chemin).unwrap());
    assert!(!contient(&chemin, "Awa Traoré".as_bytes()));
    assert!(!contient(&chemin, b"CREATE TABLE"));

    let base = Base::ouvrir(OptionsBase::fichier(&chemin).avec_cle_hex(CLE))
        .await
        .unwrap();
    assert_eq!(lire_temoin(&base).await, "Awa Traoré, 7e A");
    base.fermer().await;

    // Mauvaise clé ou pas de clé : la base ne s'ouvre pas.
    assert!(
        Base::ouvrir(OptionsBase::fichier(&chemin).avec_cle_hex(AUTRE_CLE))
            .await
            .is_err()
    );
    assert!(Base::ouvrir(OptionsBase::fichier(&chemin)).await.is_err());
}

#[tokio::test]
async fn une_base_en_clair_se_chiffre_a_sa_premiere_ouverture_avec_cle() {
    let dossier = tempfile::tempdir().unwrap();
    let chemin = dossier.path().join("gesco.db");
    let base = Base::ouvrir(OptionsBase::fichier(&chemin)).await.unwrap();
    ecrire_temoin(&base).await;
    base.fermer().await;
    assert!(est_en_clair(&chemin).unwrap());

    let base = Base::ouvrir(OptionsBase::fichier(&chemin).avec_cle_hex(CLE))
        .await
        .unwrap();
    assert_eq!(lire_temoin(&base).await, "Awa Traoré, 7e A");
    assert!(base.version_schema().await.unwrap() > 0);
    base.consolider_wal().await.unwrap();
    base.fermer().await;
    assert!(!est_en_clair(&chemin).unwrap());
    assert!(!contient(&chemin, "Awa Traoré".as_bytes()));
}

#[tokio::test]
async fn l_instantane_de_sauvegarde_reste_chiffre_avec_la_meme_cle() {
    let dossier = tempfile::tempdir().unwrap();
    let chemin = dossier.path().join("gesco.db");
    let copie = dossier.path().join("instantane.db");
    let base = Base::ouvrir(OptionsBase::fichier(&chemin).avec_cle_hex(CLE))
        .await
        .unwrap();
    ecrire_temoin(&base).await;
    base.instantane(&copie).await.unwrap();
    base.fermer().await;

    assert!(!est_en_clair(&copie).unwrap());
    assert!(!contient(&copie, "Awa Traoré".as_bytes()));
    let restauree = Base::ouvrir_sans_migrer(OptionsBase::fichier(&copie).avec_cle_hex(CLE))
        .await
        .unwrap();
    assert_eq!(lire_temoin(&restauree).await, "Awa Traoré, 7e A");
    restauree.fermer().await;
}

#[test]
fn un_fichier_absent_ou_vide_n_est_pas_en_clair() {
    let dossier = tempfile::tempdir().unwrap();
    let absent = dossier.path().join("absent.db");
    assert!(!est_en_clair(&absent).unwrap());
    let vide = dossier.path().join("vide.db");
    std::fs::write(&vide, b"").unwrap();
    assert!(!est_en_clair(&vide).unwrap());
}
