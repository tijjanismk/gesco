//! Binaire du poste (ADR-014) : la fenêtre Tauri et la commande `executer`, qui remet chaque
//! appel du frontend à la frontière de `gesco-services` — sur place, ou par le poste principal
//! quand ce poste est relié (SPEC_PHASE_11_A §5). La garde de permission y est appliquée côté
//! Rust (R-21). S'y ajoutent les commandes du rôle du poste (rejoindre ou quitter le poste
//! principal, état de la liaison, pare-feu de Windows), qui ne touchent à aucune donnée de
//! l'école, et le dialogue d'ouverture de fichier, qui ne lit rien et rend seulement un chemin.

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod empreinte;
mod pare_feu;
mod poste;
mod trousseau;

use std::path::PathBuf;
use std::sync::Arc;

use gesco_services::cles::Trousseau;
use gesco_services::{ErreurCommande, Horloge, HorlogeDecalee, HorlogeSysteme};
use tauri::{Emitter, Manager};

use crate::pare_feu::{EtatPareFeu, PareFeu};
use crate::poste::{ApercuAffiche, Demarrage, EtatPoste, InfoPoste};
use crate::trousseau::CoffreLiaison;

/// Événement envoyé à l'écran quand on ferme le poste principal alors que des postes reliés
/// ont une session ouverte : l'écran demande confirmation.
const EVENEMENT_FERMETURE: &str = "fermeture-demandee";

/// Point d'entrée unique du frontend vers les données de l'école (spécification §7.1, §12.2).
#[tauri::command]
async fn executer(
    poste: tauri::State<'_, Arc<EtatPoste>>,
    nom: String,
    jeton: Option<String>,
    entree: serde_json::Value,
) -> Result<serde_json::Value, ErreurCommande> {
    poste.executer(&nom, jeton, entree).await
}

/// Rôle du poste, connu de l'écran avant toute session.
#[tauri::command]
async fn poste_lire(poste: tauri::State<'_, Arc<EtatPoste>>) -> Result<InfoPoste, ErreurCommande> {
    Ok(poste.info().await)
}

/// Version et empreinte du poste principal à cette adresse, avant de s'y relier.
#[tauri::command]
async fn poste_principal_apercevoir(
    poste: tauri::State<'_, Arc<EtatPoste>>,
    adresse: String,
) -> Result<ApercuAffiche, ErreurCommande> {
    poste.apercevoir(&adresse).await
}

/// Relie ce poste au poste principal aperçu, avec le code qu'il affiche.
#[tauri::command]
async fn poste_rejoindre(
    poste: tauri::State<'_, Arc<EtatPoste>>,
    code: String,
    nom: String,
) -> Result<InfoPoste, ErreurCommande> {
    poste.rejoindre(&code, &nom).await
}

/// Quitte le poste principal : le poste revient au premier lancement.
#[tauri::command]
async fn poste_quitter(
    poste: tauri::State<'_, Arc<EtatPoste>>,
) -> Result<InfoPoste, ErreurCommande> {
    poste.quitter().await
}

/// Vérifie que le poste principal répond : le poste relié réessaie de lui-même après une coupure.
#[tauri::command]
async fn liaison_verifier(poste: tauri::State<'_, Arc<EtatPoste>>) -> Result<(), ErreurCommande> {
    poste.verifier_liaison().await
}

/// État du pare-feu de Windows pour le port du réseau de l'école, lu sans aucun droit.
#[tauri::command]
async fn pare_feu_lire(
    poste: tauri::State<'_, Arc<EtatPoste>>,
    port: u16,
) -> Result<EtatPareFeu, ErreurCommande> {
    poste.pare_feu_lire(port).await
}

/// Ouvre le pare-feu de Windows aux postes reliés, sur les réseaux privés : Windows demande son
/// accord à la personne, GESCO n'ayant pas les droits d'administrateur.
#[tauri::command]
async fn pare_feu_ouvrir(
    poste: tauri::State<'_, Arc<EtatPoste>>,
    port: u16,
) -> Result<EtatPareFeu, ErreurCommande> {
    poste.pare_feu_ouvrir(port).await
}

/// Ferme le poste principal malgré les postes connectés : la personne vient de le confirmer.
#[tauri::command]
fn fermer_quand_meme(fenetre: tauri::Window, poste: tauri::State<'_, Arc<EtatPoste>>) {
    poste.confirmer_fermeture();
    let _ = fenetre.destroy();
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
/// dépendent pas du jour réel ; `GESCO_HEURE_ESSAI` (de 0 à 23) fixe en plus l'heure de départ,
/// pour qu'ils ne dépendent pas non plus de l'heure réelle. Une compilation de publication les
/// ignore.
fn horloge() -> Arc<dyn Horloge> {
    let jour = if cfg!(debug_assertions) {
        std::env::var("GESCO_DATE_ESSAI")
            .ok()
            .and_then(|t| gesco_core::commun::DateCivile::analyser(&t))
    } else {
        None
    };
    let heure = std::env::var("GESCO_HEURE_ESSAI")
        .ok()
        .and_then(|t| t.parse::<u8>().ok())
        .filter(|h| *h < 24);
    match (jour, heure) {
        (Some(jour), Some(heure)) => Arc::new(HorlogeDecalee::au_jour_a(jour, heure)),
        (Some(jour), None) => Arc::new(HorlogeDecalee::au_jour(jour)),
        (None, _) => Arc::new(HorlogeSysteme),
    }
}

fn main() -> tauri::Result<()> {
    let application = tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            let essai = dossier_essai();
            let en_essai = essai.is_some();
            let pare_feu = Arc::new(PareFeu::du_poste(essai.clone()));
            let (racine, trousseau, coffre): (PathBuf, Arc<dyn Trousseau>, Arc<dyn CoffreLiaison>) =
                match essai {
                    Some(dossier) => {
                        let cles = dossier.join("trousseau-essai");
                        let coffre = Arc::new(trousseau::CoffreEssai::nouveau(&cles));
                        (
                            dossier,
                            Arc::new(trousseau::TrousseauEssai::nouveau(cles)),
                            coffre,
                        )
                    }
                    None => (
                        app.path().app_data_dir()?,
                        trousseau::du_systeme()?,
                        trousseau::coffre_du_systeme(),
                    ),
                };
            // Une panique, où qu'elle survienne (tâche de fond comprise), laisse au journal
            // technique son message et sa ligne : la compilation de publication n'a pas de console.
            let journal = racine.clone();
            let defaut = std::panic::take_hook();
            std::panic::set_hook(Box::new(move |info| {
                gesco_services::journal_technique::noter(
                    &journal,
                    HorlogeSysteme.maintenant(),
                    &format!("panique {info}"),
                );
                defaut(info);
            }));
            let demarrage = Demarrage {
                racine,
                en_essai,
                nom_ordinateur: nom_du_poste(),
                version: env!("CARGO_PKG_VERSION").to_owned(),
                trousseau,
                coffre,
                horloge: horloge(),
                cle_licence: gesco_services::licence::cle_publique_editeur()?,
                pare_feu,
            };
            let poste = Arc::new(tauri::async_runtime::block_on(EtatPoste::demarrer(
                demarrage,
            ))?);
            app.manage(poste);
            Ok(())
        })
        .on_window_event(|fenetre, evenement| {
            // Fermer le poste principal coupe le travail des postes reliés : s'il en reste de
            // connectés, l'écran le dit et demande confirmation (SPEC_PHASE_11_A §5).
            if let tauri::WindowEvent::CloseRequested { api, .. } = evenement {
                let poste = fenetre.state::<Arc<EtatPoste>>();
                let connectes = tauri::async_runtime::block_on(poste.postes_a_prevenir());
                if connectes > 0 {
                    api.prevent_close();
                    let _ = fenetre.emit(EVENEMENT_FERMETURE, connectes);
                }
            }
        })
        .invoke_handler(tauri::generate_handler![
            executer,
            poste_lire,
            poste_principal_apercevoir,
            poste_rejoindre,
            poste_quitter,
            liaison_verifier,
            pare_feu_lire,
            pare_feu_ouvrir,
            fermer_quand_meme
        ])
        .build(tauri::generate_context!())?;
    application.run(|app, evenement| {
        if let tauri::RunEvent::ExitRequested { .. } = evenement {
            let poste = app.state::<Arc<EtatPoste>>();
            tauri::async_runtime::block_on(poste.fermer());
        }
    });
    Ok(())
}
