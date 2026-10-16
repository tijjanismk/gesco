//! Binaire du poste (ADR-014) : la fenêtre Tauri et une seule commande, `executer`, qui remet
//! chaque appel du frontend à la frontière de `gesco-services`. La garde de permission y est
//! appliquée côté Rust (R-21) ; aucune autre commande n'est exposée au webview, hormis le
//! dialogue d'ouverture de fichier, qui ne lit rien et rend seulement un chemin.

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod empreinte;
mod trousseau;

use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use gesco_core::core::Poste;
use gesco_services::cles::Trousseau;
use gesco_services::session::{Jeton, PolitiqueSession};
use gesco_services::{
    Application, ErreurCommande, Horloge, HorlogeDecalee, HorlogeSysteme, OptionsApplication,
};
use tauri::Manager;

/// Point d'entrée unique du frontend (spécification §7.1, §12.2).
#[tauri::command]
async fn executer(
    application: tauri::State<'_, Arc<Application>>,
    nom: String,
    jeton: Option<String>,
    entree: serde_json::Value,
) -> Result<serde_json::Value, ErreurCommande> {
    application.executer(&nom, jeton.map(Jeton), entree).await
}

/// Nom du poste, porté par chaque écriture (R-13) : le nom de l'ordinateur.
fn nom_du_poste() -> String {
    std::env::var("COMPUTERNAME")
        .or_else(|_| std::env::var("HOSTNAME"))
        .unwrap_or_else(|_| "poste".to_owned())
}

/// Mode d'essai des compilations de développement : `GESCO_DOSSIER_ESSAI` désigne un espace de
/// données à part, et la clé du poste est un simple fichier de ce dossier. Les essais de bout en
/// bout (Playwright) rejouent ainsi un premier démarrage sans toucher à l'établissement du poste
/// ni au trousseau Windows, et une école d'essai se rouvre d'un lancement à l'autre. Une
/// compilation de publication l'ignore.
fn dossier_essai() -> Option<PathBuf> {
    if cfg!(debug_assertions) {
        std::env::var_os("GESCO_DOSSIER_ESSAI").map(PathBuf::from)
    } else {
        None
    }
}

/// Jour d'essai des compilations de développement : `GESCO_DATE_ESSAI` (`AAAA-MM-JJ`) fait vivre
/// GESCO ce jour-là, l'heure avançant normalement, pour que les essais de bout en bout ne
/// dépendent pas du jour réel. Une compilation de publication l'ignore.
fn horloge() -> Arc<dyn Horloge> {
    let jour = if cfg!(debug_assertions) {
        std::env::var("GESCO_DATE_ESSAI")
            .ok()
            .and_then(|t| gesco_core::commun::DateCivile::analyser(&t))
    } else {
        None
    };
    match jour {
        Some(jour) => Arc::new(HorlogeDecalee::au_jour(jour)),
        None => Arc::new(HorlogeSysteme),
    }
}

/// Sauvegarde planifiée et test de restauration mensuel, vérifiés chaque minute (§11.1, §11.5).
async fn taches_periodiques(application: Arc<Application>) {
    let mut minute = tokio::time::interval(Duration::from_secs(60));
    loop {
        minute.tick().await;
        application.taches_periodiques().await;
    }
}

fn main() -> tauri::Result<()> {
    let application = tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            let essai = dossier_essai();
            let (racine, trousseau): (PathBuf, Arc<dyn Trousseau>) = match essai {
                Some(dossier) => {
                    let cles = dossier.join("trousseau-essai");
                    (dossier, Arc::new(trousseau::TrousseauEssai::nouveau(cles)))
                }
                None => (app.path().app_data_dir()?, trousseau::du_systeme()?),
            };
            let poste = nom_du_poste();
            let options = OptionsApplication {
                racine,
                empreinte_machine: empreinte::du_poste(&poste),
                poste: Poste(poste),
                trousseau,
                horloge: horloge(),
                politique_session: PolitiqueSession::default(),
                version_applicative: env!("CARGO_PKG_VERSION").to_owned(),
                cle_licence: gesco_services::licence::cle_publique_editeur()?,
            };
            let application = Arc::new(tauri::async_runtime::block_on(Application::demarrer(
                options,
            ))?);
            app.manage(Arc::clone(&application));
            tauri::async_runtime::spawn(taches_periodiques(application));
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![executer])
        .build(tauri::generate_context!())?;
    application.run(|app, evenement| {
        if let tauri::RunEvent::ExitRequested { .. } = evenement {
            // Sauvegarde à la fermeture (§11.1). Son échec ne retient pas l'application : la
            // sauvegarde planifiée suivante le rattrapera.
            let etat = app.state::<Arc<Application>>();
            let _ = tauri::async_runtime::block_on(etat.fermer());
        }
    });
    Ok(())
}
