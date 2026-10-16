//! Aides communes aux tests de bout en bout par la frontière : un poste de test, son horloge et
//! son trousseau en mémoire, et les appels tels que le frontend les fait.

// Chaque fichier de test est une crate à part : une aide qui n'y sert pas y paraît morte.
#![allow(
    dead_code,
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]

use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use base64::Engine;
use gesco_core::commun::Horodatage;
use gesco_core::core::Poste;
use gesco_core::core::licence;
use gesco_services::cles::{Trousseau, TrousseauMemoire};
use gesco_services::session::{Jeton, PolitiqueSession};
use gesco_services::{Application, ErreurCommande, HorlogeFixe, OptionsApplication};
use serde_json::{Value, json};

pub const DEBUT: i64 = 1_791_101_730_250;
pub const MOT_DE_PASSE: &str = "premier-mot-de-passe";
/// Clé privée de l'éditeur pour les tests ; la vraie ne quitte pas le poste de l'éditeur.
pub const GRAINE_EDITEUR: [u8; 32] = [42; 32];

/// Bornes d'années scolaires utilisées par les scénarios.
pub const RENTREE_2026: i64 = 1_790_812_800_000;
pub const FIN_2027: i64 = 1_816_992_000_000;
pub const RENTREE_2027: i64 = 1_822_348_800_000;
pub const FIN_2028: i64 = 1_848_614_400_000;

/// Image PNG de 4 × 4 pixels, en base64.
pub const PNG_ESSAI: &str = "iVBORw0KGgoAAAANSUhEUgAAAAQAAAAECAIAAAAmkwkpAAAAEElEQVR4nGOQc1gPRwzEcQCsUxDRqgYFyQAAAABJRU5ErkJggg==";

/// Connexion directe à la base chiffrée d'un poste de test, avec sa clé de données : pour imiter
/// une base ancienne ou abîmée, ce que l'application ne sait pas faire d'elle-même.
pub async fn base_brute(dossier: &Path, trousseau: Arc<dyn Trousseau>) -> sqlx::SqliteConnection {
    use sqlx::Connection;
    let espace = gesco_services::espace::EspaceDonnees::nouveau(dossier);
    let cles = gesco_services::cles::EspaceCles::nouveau(espace.cles(), trousseau);
    let gesco_services::cles::EtatCles::Ouvertes(cle) = cles.preparer().unwrap() else {
        panic!("clés de données verrouillées");
    };
    let options = sqlx::sqlite::SqliteConnectOptions::new()
        .filename(espace.base())
        .pragma("key", format!("'{}'", cle.en_hexadecimal().as_str()));
    sqlx::SqliteConnection::connect_with(&options)
        .await
        .unwrap()
}

pub struct Poste_ {
    pub application: Application,
    pub horloge: Arc<HorlogeFixe>,
    pub trousseau: Arc<dyn Trousseau>,
    pub racine: PathBuf,
}

pub async fn demarrer(
    racine: &Path,
    trousseau: Arc<dyn Trousseau>,
    horloge: Arc<HorlogeFixe>,
) -> Poste_ {
    let application = Application::demarrer(OptionsApplication {
        racine: racine.to_path_buf(),
        poste: Poste("secretariat".to_owned()),
        empreinte_machine: "machine-test".to_owned(),
        trousseau: Arc::clone(&trousseau),
        horloge: horloge.clone(),
        politique_session: PolitiqueSession::default(),
        version_applicative: "0.1.0".to_owned(),
        cle_licence: licence::cle_publique(&GRAINE_EDITEUR),
    })
    .await
    .unwrap();
    Poste_ {
        application,
        horloge,
        trousseau,
        racine: racine.to_path_buf(),
    }
}

pub async fn poste_vierge(racine: &Path) -> Poste_ {
    demarrer(
        racine,
        Arc::new(TrousseauMemoire::default()),
        Arc::new(HorlogeFixe::a(Horodatage::depuis_millis_epoque(DEBUT))),
    )
    .await
}

impl Poste_ {
    pub async fn appeler(
        &self,
        nom: &str,
        jeton: Option<&Jeton>,
        entree: Value,
    ) -> Result<Value, ErreurCommande> {
        self.application.executer(nom, jeton.cloned(), entree).await
    }

    pub async fn ok(&self, nom: &str, jeton: Option<&Jeton>, entree: Value) -> Value {
        self.appeler(nom, jeton, entree)
            .await
            .unwrap_or_else(|e| panic!("{nom} : {e:?}"))
    }

    pub async fn refus(&self, nom: &str, jeton: Option<&Jeton>, entree: Value) -> ErreurCommande {
        self.appeler(nom, jeton, entree).await.expect_err(nom)
    }

    pub async fn phase(&self) -> String {
        self.ok("etat_demarrage", None, Value::Null).await["phase"]
            .as_str()
            .unwrap()
            .to_owned()
    }

    /// Premier démarrage ; rend les douze mots de la phrase de secours.
    pub async fn installer(&self) -> String {
        let r = self
            .ok(
                "premier_demarrage",
                None,
                json!({
                    "organisation": "Groupe scolaire Askia",
                    "etablissement_nom": "Lycée Askia Mohamed",
                    "statut": "prive",
                    "code_academie": "AE-BKO-RD",
                    "code_cap": "CAP-01",
                    "adresse": "Hamdallaye ACI 2000, Bamako",
                    "telephones": ["+223 20 22 00 00"],
                    "administrateur_identifiant": "admin",
                    "administrateur_nom": "Awa Traoré",
                    "administrateur_mot_de_passe": MOT_DE_PASSE,
                    "cycles": ["jardin", "premier_cycle", "second_cycle", "lycee"],
                }),
            )
            .await;
        let mots: Vec<String> = serde_json::from_value(r["phrase"]["mots"].clone()).unwrap();
        assert_eq!(mots.len(), 12);
        let pdf = base64::engine::general_purpose::STANDARD
            .decode(r["phrase"]["certificat_pdf"].as_str().unwrap())
            .unwrap();
        assert!(pdf.starts_with(b"%PDF-"));
        mots.join(" ")
    }

    pub async fn connecter(&self, identifiant: &str, mot_de_passe: &str) -> Jeton {
        let s = self
            .ok(
                "connexion",
                None,
                json!({ "identifiant": identifiant, "mot_de_passe": mot_de_passe }),
            )
            .await;
        serde_json::from_value(s["jeton"].clone()).unwrap()
    }

    /// Sauvegarde manuelle ; rend une copie de l'archive locale, à l'abri de la rotation.
    pub async fn sauvegarder(&self, jeton: &Jeton, copie: &Path) -> PathBuf {
        self.horloge.avancer(60_000);
        let lignes = self.ok("sauvegarde_creer", Some(jeton), Value::Null).await;
        let archive = PathBuf::from(lignes[0]["destination"].as_str().unwrap());
        assert_eq!(lignes[0]["resultat"], "reussie");
        std::fs::create_dir_all(copie).unwrap();
        let cible = copie.join(archive.file_name().unwrap());
        std::fs::copy(&archive, &cible).unwrap();
        cible
    }
}

pub fn avertissements(valeur: &Value) -> HashSet<String> {
    valeur
        .as_array()
        .unwrap()
        .iter()
        .map(|a| a["cle"].as_str().unwrap().to_owned())
        .collect()
}

/// Grille de frais en vigueur pour l'année : pour chaque niveau, 10 000 F d'inscription et
/// 100 000 F de scolarité selon les plans livrés. Sans elle, aucune inscription ne se valide
/// (SPEC_PHASE_3 Q-49).
pub async fn poser_grille(p: &Poste_, admin: &Jeton, annee_id: &Value) -> Value {
    let referentiel = p.ok("niveaux_lister", Some(admin), Value::Null).await;
    let grille = p
        .ok(
            "grille_creer",
            Some(admin),
            json!({ "annee_id": annee_id, "copie_de": null }),
        )
        .await;
    let mut lignes = Vec::new();
    for n in referentiel["niveaux"].as_array().unwrap() {
        lignes.push(json!({ "niveau_id": n["id"], "serie_id": null, "type_frais": "inscription",
            "libelle": "Frais d'inscription", "montant": 10_000, "obligatoire": true, "exigibilite": "une_fois" }));
        lignes.push(json!({ "niveau_id": n["id"], "serie_id": null, "type_frais": "scolarite",
            "libelle": "Scolarité", "montant": 100_000, "obligatoire": true, "exigibilite": "selon_plan" }));
    }
    let plans: Vec<Value> = grille["contenu"]["plans"]
        .as_array()
        .unwrap()
        .iter()
        .map(|p| json!({ "libelle": p["libelle"], "par_defaut": p["par_defaut"], "tranches": p["tranches"] }))
        .collect();
    p.ok(
        "grille_modifier",
        Some(admin),
        json!({ "id": grille["grille"]["id"], "libelle": grille["grille"]["libelle"], "lignes": lignes, "plans": plans }),
    )
    .await;
    p.ok(
        "grille_mettre_en_vigueur",
        Some(admin),
        json!({ "id": grille["grille"]["id"] }),
    )
    .await
}
