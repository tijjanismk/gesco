//! `gesco-remplir` — outil du porteur du projet, jamais livré aux écoles : remplit une base GESCO
//! de trois années de démonstration, 2024-2025 et 2025-2026 closes, 2026-2027 en cours au
//! 31 octobre 2026, une dizaine d'élèves par classe du 1er cycle, du 2e cycle et du lycée.
//!
//! ```text
//! gesco-remplir essai <dossier>   école d'essai neuve, premier démarrage compris ; elle s'ouvre
//!                                 ensuite comme les autres écoles d'essai (GESCO_DOSSIER_ESSAI)
//! gesco-remplir poste             base du poste (%APPDATA%\ml.gesco.poste), après le premier
//!                                 démarrage fait à l'écran ; demande l'identifiant et le mot de
//!                                 passe de l'administrateur
//! ```
//!
//! Tout passe par les commandes de l'application, comme l'écran : factures, échéances, bulletins,
//! décisions et audit restent ceux de GESCO. Une horloge réglée fait vivre chaque geste à sa
//! date, du 1er septembre 2024 au 31 octobre 2026, sans jamais reculer : le journal d'audit garde
//! l'ordre des dates. Seule exception, en mode `poste` : le premier démarrage, fait à l'écran le
//! jour même, précède dans le journal des gestes datés de 2024.
//!
//! GESCO doit être fermé pendant le remplissage. Pour le poste, lancer l'outil par l'Explorateur :
//! une console lancée depuis Claude voit un `%APPDATA%` virtualisé. Le mot de passe y est demandé
//! sans s'afficher.

// Les réponses se lisent comme l'écran les lit : `valeur["champ"]` sur une valeur JSON rend
// `null` quand le champ manque, sans jamais paniquer.
#![allow(clippy::indexing_slicing)]

use std::collections::{HashMap, HashSet};
use std::io::{BufRead, Write};
use std::path::PathBuf;
use std::sync::Arc;

use gesco_core::commun::DateCivile;
use gesco_core::core::Poste;
use gesco_services::cles::Trousseau;
use gesco_services::session::{Jeton, PolitiqueSession};
use gesco_services::{Application, Horloge, HorlogeFixe, OptionsApplication};
use serde_json::{Value, json};

#[path = "../../../crates/gesco-app/src/empreinte.rs"]
mod empreinte;
#[path = "../../../crates/gesco-app/src/trousseau.rs"]
mod trousseau;

type R<T> = Result<T, String>;

const USAGE: &str = "usage : gesco-remplir essai <dossier> | gesco-remplir poste";

/// Mot de passe de l'administrateur d'une école d'essai : noté dans son `LISEZ-MOI.txt`.
const MOT_DE_PASSE_ESSAI: &str = "demonstration-2026";

/// Années : libellé, rentrée, fin.
const ANNEES: [(&str, &str, &str); 3] = [
    ("2024-2025", "2024-10-01", "2025-07-31"),
    ("2025-2026", "2025-10-01", "2026-07-31"),
    ("2026-2027", "2026-10-01", "2027-07-31"),
];

/// L'année en cours s'arrête à ce jour.
const JOUR_FINAL: &str = "2026-10-31";

/// Jour où l'horloge de l'outil démarre : un mois avant la première rentrée.
const JOUR_DEPART: &str = "2024-09-01";

/// Classes de chaque année : code du niveau, code de la série, libellé.
const CLASSES: [(&str, Option<&str>, &str); 12] = [
    ("1", None, "1re A"),
    ("2", None, "2e A"),
    ("3", None, "3e A"),
    ("4", None, "4e A"),
    ("5", None, "5e A"),
    ("6", None, "6e A"),
    ("7", None, "7e A"),
    ("8", None, "8e A"),
    ("9", None, "9e A"),
    ("10", None, "10e A"),
    ("11", Some("S"), "11e S"),
    ("12", Some("TSE"), "12e TSE"),
];

/// Élèves d'une classe à la première année, et des 1re année de chaque rentrée.
const PAR_CLASSE: usize = 10;

const PRENOMS_FILLES: [&str; 20] = [
    "Aminata",
    "Fatoumata",
    "Awa",
    "Mariam",
    "Kadiatou",
    "Oumou",
    "Hawa",
    "Assitan",
    "Djeneba",
    "Rokia",
    "Salimata",
    "Nana",
    "Bintou",
    "Kadidia",
    "Aissata",
    "Fanta",
    "Mariame",
    "Sitan",
    "Korotoumou",
    "Ramata",
];
const PRENOMS_GARCONS: [&str; 20] = [
    "Moussa",
    "Mamadou",
    "Seydou",
    "Bakary",
    "Oumar",
    "Ibrahim",
    "Souleymane",
    "Adama",
    "Boubacar",
    "Modibo",
    "Lassana",
    "Drissa",
    "Yacouba",
    "Sidiki",
    "Amadou",
    "Cheick",
    "Issa",
    "Tiémoko",
    "Abdoulaye",
    "Siaka",
];
const NOMS: [&str; 20] = [
    "Traoré",
    "Diarra",
    "Coulibaly",
    "Keïta",
    "Konaté",
    "Sangaré",
    "Diallo",
    "Touré",
    "Sissoko",
    "Dembélé",
    "Camara",
    "Cissé",
    "Doumbia",
    "Kané",
    "Maïga",
    "Sidibé",
    "Samaké",
    "Fofana",
    "Bagayoko",
    "Kouyaté",
];

/// Ce qui distingue un élève d'une année à l'autre : son niveau scolaire, en centièmes sur 20,
/// et la façon dont sa famille paie (0 : tout, 1 : deux versements sur trois, 2 : un seul, 3 :
/// l'inscription seulement).
#[derive(Clone, Copy)]
struct Profil {
    aptitude: i64,
    payeur: u8,
}

struct Classe {
    id: Value,
    niveau_id: Value,
    serie_id: Value,
    code: &'static str,
    libelle: &'static str,
    premier_cycle: bool,
}

struct Annee {
    id: Value,
    libelle: &'static str,
    rentree: DateCivile,
    fin: DateCivile,
    classes: Vec<Classe>,
    pedagogie: Value,
}

struct Ecole {
    application: Application,
    horloge: Arc<HorlogeFixe>,
    identifiant: String,
    mot_de_passe: String,
    jeton: Option<Jeton>,
    graine: u64,
    compteur: u64,
    profils: HashMap<String, Profil>,
    /// Enseignants créés : clé (maître d'une classe, ou matière d'un cycle) vers leur id.
    enseignants: HashMap<String, Value>,
    /// Personnes des enseignants créés par l'outil ; vrai pour un maître du 1er cycle, payé
    /// moins qu'un professeur.
    personnes_enseignants: HashMap<String, bool>,
}

fn dire(texte: &str) {
    let mut sortie = std::io::stdout().lock();
    let _ = writeln!(sortie, "{texte}");
    let _ = sortie.flush();
}

fn lire_ligne(question: &str) -> R<String> {
    let mut sortie = std::io::stdout().lock();
    let _ = write!(sortie, "{question}");
    let _ = sortie.flush();
    drop(sortie);
    let mut ligne = String::new();
    std::io::stdin()
        .lock()
        .read_line(&mut ligne)
        .map_err(|e| e.to_string())?;
    Ok(ligne.trim_end_matches(['\r', '\n']).to_owned())
}

/// Lit un secret sans l'afficher : PowerShell le demande avec des astérisques et le rend sur sa
/// sortie, que seul l'outil lit. L'espace de travail interdit `unsafe`, donc pas d'appel direct
/// à la console de Windows, et l'outil reste sans dépendance de plus.
#[cfg(windows)]
fn lire_secret(question: &str) -> R<String> {
    let script = format!(
        "[Console]::OutputEncoding = [Text.Encoding]::UTF8; \
         $s = Read-Host '{}' -AsSecureString; \
         $p = [Runtime.InteropServices.Marshal]::SecureStringToBSTR($s); \
         try {{ [Console]::Out.Write([Runtime.InteropServices.Marshal]::PtrToStringBSTR($p)) }} \
         finally {{ [Runtime.InteropServices.Marshal]::ZeroFreeBSTR($p) }}",
        question.replace('\'', "''")
    );
    let sortie = std::process::Command::new("powershell")
        .args(["-NoProfile", "-Command", &script])
        .stdin(std::process::Stdio::inherit())
        .stderr(std::process::Stdio::inherit())
        .output()
        .map_err(|e| format!("PowerShell n'a pas pu demander le mot de passe : {e}"))?;
    if !sortie.status.success() {
        return Err("le mot de passe n'a pas pu être lu sans s'afficher".to_owned());
    }
    String::from_utf8(sortie.stdout).map_err(|e| e.to_string())
}

/// Hors de Windows, l'outil ne sert qu'aux essais : la saisie reste visible.
#[cfg(not(windows))]
fn lire_secret(question: &str) -> R<String> {
    lire_ligne(&format!("{question} : "))
}

fn date(texte: &str) -> R<DateCivile> {
    DateCivile::analyser(texte).ok_or_else(|| format!("date illisible : {texte}"))
}

fn decaler(jour: DateCivile, jours: i64) -> R<DateCivile> {
    jour.decaler(jours)
        .ok_or_else(|| "date hors calendrier".to_owned())
}

/// 8 h du matin ce jour-là, en millisecondes.
fn huit_heures(jour: DateCivile) -> i64 {
    jour.jours_depuis_epoque() * 86_400_000 + 8 * 3_600_000
}

fn texte(v: &Value) -> String {
    v.as_str().map_or_else(|| v.to_string(), str::to_owned)
}

fn liste(v: &Value) -> &[Value] {
    v.as_array().map_or(&[], Vec::as_slice)
}

impl Ecole {
    async fn appel(&self, nom: &str, entree: Value) -> R<Value> {
        self.application
            .executer(nom, self.jeton.clone(), entree)
            .await
            .map_err(|e| format!("{nom} : {e:?}"))
    }

    async fn connecter(&mut self) -> R<()> {
        self.jeton = None;
        let session = self
            .appel(
                "connexion",
                json!({ "identifiant": self.identifiant, "mot_de_passe": self.mot_de_passe }),
            )
            .await?;
        self.jeton =
            Some(serde_json::from_value(session["jeton"].clone()).map_err(|e| e.to_string())?);
        Ok(())
    }

    /// Avance l'horloge à 8 h le jour donné, puis rouvre une session : la précédente a expiré.
    /// Refuse de reculer : le journal d'audit garde l'ordre des dates.
    async fn aller_au(&mut self, jour: DateCivile) -> R<()> {
        let cible = huit_heures(jour);
        let maintenant = self.horloge.maintenant().millis_epoque();
        if cible < maintenant {
            return Err(format!("l'horloge reculerait au {jour}"));
        }
        self.horloge.avancer(cible - maintenant);
        self.connecter().await
    }

    /// Nombre pseudo-aléatoire reproductible, de 0 à `n` exclu.
    fn tirer(&mut self, n: u64) -> u64 {
        self.graine ^= self.graine << 13;
        self.graine ^= self.graine >> 7;
        self.graine ^= self.graine << 17;
        self.graine % n.max(1)
    }

    fn choisir<'a>(&mut self, mots: &[&'a str]) -> &'a str {
        let i = usize::try_from(self.tirer(mots.len() as u64)).unwrap_or(0);
        mots.get(i).copied().unwrap_or("")
    }

    fn profil(&self, eleve_id: &str) -> Profil {
        self.profils.get(eleve_id).copied().unwrap_or(Profil {
            aptitude: 1_200,
            payeur: 0,
        })
    }

    /// Grille de frais de l'année, en vigueur : 10 000 F d'inscription, et une scolarité qui
    /// croît avec le cycle, payable selon les plans livrés.
    async fn poser_grille(&self, annee_id: &Value) -> R<()> {
        let referentiel = self.appel("niveaux_lister", Value::Null).await?;
        let grille = self
            .appel(
                "grille_creer",
                json!({ "annee_id": annee_id, "copie_de": null }),
            )
            .await?;
        let mut lignes = Vec::new();
        for n in liste(&referentiel["niveaux"]) {
            let scolarite = match n["cycle"].as_str() {
                Some("jardin") => 60_000,
                Some("premier_cycle") => 75_000,
                Some("second_cycle") => 105_000,
                Some("lycee") => 135_000,
                _ => 150_000,
            };
            lignes.push(json!({ "niveau_id": n["id"], "serie_id": null, "type_frais": "inscription",
                "libelle": "Frais d'inscription", "montant": 10_000, "obligatoire": true, "exigibilite": "une_fois" }));
            lignes.push(json!({ "niveau_id": n["id"], "serie_id": null, "type_frais": "scolarite",
                "libelle": "Scolarité", "montant": scolarite, "obligatoire": true, "exigibilite": "selon_plan" }));
        }
        let plans: Vec<Value> = liste(&grille["contenu"]["plans"])
            .iter()
            .map(|p| json!({ "libelle": p["libelle"], "par_defaut": p["par_defaut"], "tranches": p["tranches"] }))
            .collect();
        self.appel(
            "grille_modifier",
            json!({ "id": grille["grille"]["id"], "libelle": grille["grille"]["libelle"], "lignes": lignes, "plans": plans }),
        )
        .await?;
        self.appel(
            "grille_mettre_en_vigueur",
            json!({ "id": grille["grille"]["id"] }),
        )
        .await?;
        Ok(())
    }

    /// Les classes de l'année, créées ou reprises de la précédente, dans l'ordre de `CLASSES`.
    async fn classes(&self, annee_id: &Value, pedagogie: &Value, creer: bool) -> R<Vec<Classe>> {
        let existantes = self
            .appel("classes_lister", json!({ "annee_id": annee_id }))
            .await?;
        let mut classes = Vec::new();
        for (code, serie, libelle) in CLASSES {
            let niveau = liste(&pedagogie["niveaux"])
                .iter()
                .find(|n| n["code"] == code)
                .ok_or_else(|| format!("niveau {code} absent du référentiel"))?;
            let serie_id = match serie {
                Some(s) => liste(&pedagogie["series"])
                    .iter()
                    .find(|x| x["niveau_id"] == niveau["id"] && x["code"] == s)
                    .map(|x| x["id"].clone())
                    .ok_or_else(|| format!("série {s} absente"))?,
                None => Value::Null,
            };
            let deja = liste(&existantes)
                .iter()
                .find(|c| c["classe"]["libelle"] == libelle)
                .map(|c| c["classe"]["id"].clone());
            let id = match deja {
                Some(id) => id,
                None if creer => {
                    self.appel(
                        "classe_creer",
                        json!({ "annee_id": annee_id, "niveau_id": niveau["id"], "serie_id": serie_id, "libelle": libelle }),
                    )
                    .await?["id"]
                        .clone()
                }
                None => return Err(format!("classe {libelle} non reprise")),
            };
            classes.push(Classe {
                id,
                niveau_id: niveau["id"].clone(),
                serie_id,
                code,
                libelle,
                premier_cycle: niveau["cycle"] == "premier_cycle",
            });
        }
        Ok(classes)
    }

    /// Un élève neuf, son père ou sa mère payeur, et son inscription dans la classe ; rend l'id
    /// de l'inscription.
    async fn nouvel_eleve(&mut self, classe: &Classe, rentree: DateCivile) -> R<Value> {
        self.compteur += 1;
        let fille = self.tirer(2) == 0;
        let prenoms = if fille {
            self.choisir(&PRENOMS_FILLES)
        } else {
            self.choisir(&PRENOMS_GARCONS)
        };
        let nom = self.choisir(&NOMS);
        let rang: i32 = classe.code.parse().unwrap_or(1);
        let annee_naissance = rentree.annee() - 5 - rang;
        let naissance = format!(
            "{annee_naissance}-{:02}-{:02}",
            1 + self.tirer(12),
            1 + self.tirer(28)
        );
        let fiche = self
            .appel(
                "eleve_creer",
                json!({ "identite": { "nom": nom, "prenoms": prenoms,
                    "sexe": if fille { "feminin" } else { "masculin" },
                    "date_naissance": naissance, "date_naissance_estimee": false,
                    "lieu_naissance": "Bamako", "nationalite": "Malienne", "adresse": "Bamako",
                    "telephones": [] },
                    // Exigé dès la 9e et au lycée, où chacun finit par monter.
                    "matricule_interne": "",
                    "matricule_national": format!("ML-{}-{:06}", rentree.annee(), self.compteur),
                    "acte_nature": "acte",
                    "acte_numero": "", "acte_date": "", "ecole_origine": "" }),
            )
            .await?;
        let eleve_id = fiche["eleve"]["id"].clone();
        let pere = self.tirer(2) == 0;
        let parent = if pere {
            self.choisir(&PRENOMS_GARCONS)
        } else {
            self.choisir(&PRENOMS_FILLES)
        };
        let telephone = format!(
            "7{} {:02} {:02} {:02}",
            self.tirer(10),
            self.compteur % 100,
            self.tirer(100),
            self.tirer(100)
        );
        self.appel(
            "responsable_rattacher",
            json!({ "eleve_id": eleve_id, "responsable_id": null,
                "saisie": { "identite": { "nom": nom, "prenoms": parent,
                    "sexe": if pere { "masculin" } else { "feminin" },
                    "date_naissance": "", "date_naissance_estimee": false, "lieu_naissance": "",
                    "nationalite": "", "adresse": "Bamako", "telephones": [telephone] },
                    "profession": "", "employeur": "" },
                "qualites": { "lien": if pere { "pere" } else { "mere" }, "est_responsable_legal": true,
                    "est_payeur": true, "recoit_notifications": true, "vient_chercher": false,
                    "ordre_priorite": 1 } }),
        )
        .await?;
        // Un élève sur dix peine, un sur cinq brille ; une famille sur quatre paie en retard.
        let aptitude = match self.tirer(10) {
            0 => 650 + i64::try_from(self.tirer(250)).unwrap_or(0),
            1 | 2 => 1_450 + i64::try_from(self.tirer(300)).unwrap_or(0),
            _ => 950 + i64::try_from(self.tirer(500)).unwrap_or(0),
        };
        let payeur = match self.tirer(20) {
            0 => 3,
            1 | 2 => 2,
            3..=5 => 1,
            _ => 0,
        };
        self.profils
            .insert(texte(&eleve_id), Profil { aptitude, payeur });
        self.inscrire(&eleve_id, &classe.id, false).await
    }

    /// L'enseignant d'une clé, créé à son premier usage ; sa fiche du personnel suit.
    async fn enseignant(&mut self, cle: &str, maitre: bool) -> R<Value> {
        if let Some(id) = self.enseignants.get(cle) {
            return Ok(id.clone());
        }
        self.compteur += 1;
        let homme = self.tirer(3) != 0;
        let prenoms = if homme {
            self.choisir(&PRENOMS_GARCONS)
        } else {
            self.choisir(&PRENOMS_FILLES)
        };
        let nom = self.choisir(&NOMS);
        let telephone = format!(
            "6{} {:02} {:02} {:02}",
            self.tirer(10),
            self.compteur % 100,
            self.tirer(100),
            self.tirer(100)
        );
        let fiche = self
            .appel(
                "enseignant_enregistrer",
                json!({ "id": null, "nom": nom, "prenoms": prenoms, "telephones": [telephone],
                    "statut": "titulaire", "utilisateur_id": null, "actif": true }),
            )
            .await?;
        let id = fiche["enseignant"]["id"].clone();
        self.personnes_enseignants
            .insert(texte(&fiche["enseignant"]["personne_id"]), maitre);
        self.enseignants.insert(cle.to_owned(), id.clone());
        Ok(id)
    }

    /// Un maître par classe du 1er cycle ; au 2e cycle et au lycée, un professeur par matière,
    /// l'un pour le collège (7e à 9e), l'autre pour le lycée. Une affectation qui existe déjà
    /// est laissée telle quelle ; tout autre refus arrête l'outil et s'affiche.
    async fn affecter(&mut self, annee_id: &Value, pedagogie: &Value, classes: &[Classe]) -> R<()> {
        let mut affectations = 0;
        for classe in classes {
            if classe.premier_cycle {
                let maitre = self
                    .enseignant(&format!("maitre-{}", classe.code), true)
                    .await?;
                self.appel(
                    "maitre_affecter",
                    json!({ "classe_id": classe.id, "enseignant_id": maitre }),
                )
                .await?;
                affectations += 1;
                continue;
            }
            let groupe = if matches!(classe.code, "7" | "8" | "9") {
                "college"
            } else {
                "lycee"
            };
            let matieres: Vec<(Value, String)> = liste(&pedagogie["programme"])
                .iter()
                .filter(|l| {
                    l["annee_id"] == *annee_id
                        && l["niveau_id"] == classe.niveau_id
                        && (l["serie_id"].is_null() || l["serie_id"] == classe.serie_id)
                })
                .map(|l| {
                    let code = liste(&pedagogie["matieres"])
                        .iter()
                        .find(|m| m["id"] == l["matiere_id"])
                        .map_or_else(String::new, |m| texte(&m["code"]));
                    (l["matiere_id"].clone(), code)
                })
                .collect();
            for (matiere, code) in matieres {
                let professeur = self.enseignant(&format!("{groupe}-{code}"), false).await?;
                match self
                    .appel(
                        "affectation_ajouter",
                        json!({ "classe_id": classe.id, "matiere_id": matiere, "enseignant_id": professeur }),
                    )
                    .await
                {
                    Ok(_) => affectations += 1,
                    Err(e) if e.contains("affectation.deja") => {}
                    Err(e) => return Err(e),
                }
            }
        }
        dire(&format!(
            "  {} enseignants, {affectations} affectations",
            self.enseignants.len()
        ));
        Ok(())
    }

    /// Un contrat à durée indéterminée, payé au mois, pour chaque enseignant que l'outil a créé ;
    /// un refus arrête l'outil et s'affiche.
    async fn engager(&mut self, debut: DateCivile) -> R<()> {
        let registre = self.appel("personnel_lister", Value::Null).await?;
        let mut engages = 0;
        for m in liste(&registre) {
            let Some(maitre) = self
                .personnes_enseignants
                .get(&texte(&m["membre"]["personne_id"]))
            else {
                continue;
            };
            let montant = if *maitre { 85_000 } else { 125_000 };
            self.appel(
                "engagement_conclure",
                json!({ "membre_id": m["membre"]["id"], "nature": "duree_indeterminee",
                    "debut": debut.to_string(), "fin": null, "fonction": "",
                    "remuneration": { "type": "mensuelle", "montant": montant }, "note": "",
                    "remplace": null, "renouvelle": null }),
            )
            .await?;
            engages += 1;
        }
        dire(&format!("  {engages} contrats d'enseignants"));
        Ok(())
    }

    /// La paie d'un mois : calculée, validée, puis tout le reste payé.
    async fn paie(&mut self, mois: &str, jour: DateCivile) -> R<()> {
        self.aller_au(jour).await?;
        let demande = json!({ "mois": mois });
        self.appel("paie_calculer", demande.clone()).await?;
        self.appel("paie_valider", demande.clone()).await?;
        self.appel("paie_payer_reste", demande).await?;
        Ok(())
    }

    async fn inscrire(&self, eleve_id: &Value, classe_id: &Value, redoublant: bool) -> R<Value> {
        Ok(self
            .appel(
                "inscription_creer",
                json!({ "eleve_id": eleve_id, "classe_id": classe_id, "date_inscription": "",
                    "redoublant": redoublant, "etablissement_origine": "" }),
            )
            .await?["inscription"]["id"]
            .clone())
    }

    /// Inscriptions validées de l'année : (inscription, élève, classe).
    async fn inscrits(&self, annee: &Annee) -> R<Vec<(Value, String, Value)>> {
        let lignes = self
            .appel("inscriptions_lister", json!({ "annee_id": annee.id }))
            .await?;
        Ok(liste(&lignes)
            .iter()
            .filter(|l| l["inscription"]["statut"] == "validee")
            .map(|l| {
                (
                    l["inscription"]["id"].clone(),
                    texte(&l["inscription"]["eleve_id"]),
                    l["inscription"]["classe_id"].clone(),
                )
            })
            .collect())
    }

    /// La première année : créée, ouverte, ses classes et ses élèves.
    async fn premiere_annee(
        &mut self,
        libelle: &'static str,
        rentree: DateCivile,
        fin: DateCivile,
    ) -> R<Annee> {
        self.aller_au(decaler(rentree, -29)?).await?;
        let annee = self.creer_annee(libelle, rentree, fin).await?;
        self.appel("annee_activer", json!({ "id": annee.id }))
            .await?;
        self.poser_grille(&annee.id).await?;
        let classes = self.classes(&annee.id, &annee.pedagogie, true).await?;
        self.affecter(&annee.id, &annee.pedagogie, &classes).await?;
        self.engager(rentree).await?;
        self.aller_au(decaler(rentree, -21)?).await?;
        for classe in &classes {
            for _ in 0..PAR_CLASSE {
                let inscription = self.nouvel_eleve(classe, rentree).await?;
                self.appel("inscription_valider", json!({ "id": inscription }))
                    .await?;
            }
            dire(&format!("  {libelle} : {} inscrite", classe.libelle));
        }
        Ok(Annee { classes, ..annee })
    }

    async fn creer_annee(
        &self,
        libelle: &'static str,
        rentree: DateCivile,
        fin: DateCivile,
    ) -> R<Annee> {
        let debut = rentree.jours_depuis_epoque() * 86_400_000;
        let fin_millis = fin.jours_depuis_epoque() * 86_400_000;
        let cree = self
            .appel(
                "annee_creer",
                json!({ "libelle": libelle, "debut": debut, "fin": fin_millis }),
            )
            .await?;
        let pedagogie = self
            .appel("pedagogie_lire", json!({ "id": cree["id"] }))
            .await?;
        Ok(Annee {
            id: cree["id"].clone(),
            libelle,
            rentree,
            fin,
            classes: Vec::new(),
            pedagogie,
        })
    }

    /// L'année suivante : classes reprises, élèves admis ou redoublants réinscrits, nouveaux
    /// élèves de 1re année, puis la rentrée. Entre les deux années, les paies d'août et de
    /// septembre : les contrats sont payés douze mois.
    async fn annee_suivante(
        &mut self,
        decisions: &HashMap<String, (String, &'static str)>,
        libelle: &'static str,
        rentree: DateCivile,
        fin: DateCivile,
    ) -> R<Annee> {
        self.aller_au(decaler(rentree, -60)?).await?;
        let annee = self.creer_annee(libelle, rentree, fin).await?;
        self.appel("classes_reprendre", json!({ "id": annee.id }))
            .await?;
        self.poser_grille(&annee.id).await?;
        let classes = self.classes(&annee.id, &annee.pedagogie, true).await?;
        self.affecter(&annee.id, &annee.pedagogie, &classes).await?;
        let bilan = self
            .appel("reinscriptions_preparer", json!({ "id": annee.id }))
            .await?;
        dire(&format!(
            "  {libelle} : {} réinscriptions préparées, {} sortants",
            bilan["preparees"], bilan["sortants"]
        ));
        // Ceux que la préparation n'a pas placés (changement de série) : inscrits à la main.
        let deja: HashSet<String> = liste(
            &self
                .appel("inscriptions_lister", json!({ "annee_id": annee.id }))
                .await?,
        )
        .iter()
        .map(|l| texte(&l["inscription"]["eleve_id"]))
        .collect();
        let suivant = |code: &str| -> Option<&'static str> {
            let i = CLASSES.iter().position(|(c, _, _)| *c == code)?;
            CLASSES.get(i + 1).map(|(c, _, _)| *c)
        };
        for (eleve_id, (decision, code)) in decisions {
            if deja.contains(eleve_id) {
                continue;
            }
            let cible = match decision.as_str() {
                "admis" => suivant(code),
                "redouble" => Some(*code),
                _ => None,
            };
            if let Some(classe) = cible.and_then(|c| classes.iter().find(|k| k.code == c)) {
                self.inscrire(&json!(eleve_id), &classe.id, decision == "redouble")
                    .await?;
            }
        }
        let an = rentree.annee();
        self.paie(&format!("{an}-08"), date(&format!("{an}-08-28"))?)
            .await?;
        self.aller_au(decaler(rentree, -21)?).await?;
        if let Some(premiere) = classes.first() {
            for _ in 0..PAR_CLASSE {
                self.nouvel_eleve(premiere, rentree).await?;
            }
        }
        self.aller_au(decaler(rentree, -14)?).await?;
        for classe in &classes {
            self.appel("inscriptions_valider_classe", json!({ "id": classe.id }))
                .await?;
        }
        self.aller_au(decaler(rentree, -6)?).await?;
        self.appel("annee_activer", json!({ "id": annee.id }))
            .await?;
        self.paie(&format!("{an}-09"), date(&format!("{an}-09-28"))?)
            .await?;
        Ok(Annee { classes, ..annee })
    }

    /// Un tour de versements : chaque famille paie selon son habitude, en espèces, dans une
    /// caisse ouverte puis clôturée sans écart.
    async fn versements(&mut self, annee: &Annee, tour: u8, jour: DateCivile) -> R<()> {
        self.aller_au(jour).await?;
        self.appel("caisse_ouvrir", json!({ "fonds_initial": 0 }))
            .await?;
        let mut recus = 0;
        for (inscription, eleve_id, _) in self.inscrits(annee).await? {
            let profil = self.profil(&eleve_id);
            let paie = match tour {
                1 => true,
                2 => profil.payeur <= 1,
                _ => profil.payeur == 0,
            };
            if !paie {
                continue;
            }
            let preparation = self
                .appel(
                    "encaissement_preparer",
                    json!({ "inscription_id": inscription, "montant": 0 }),
                )
                .await?;
            let reste = preparation["reste"].as_i64().unwrap_or(0);
            if reste <= 0 {
                continue;
            }
            let part = if tour == 1 && profil.payeur == 3 {
                10_000
            } else {
                reste / i64::from(4 - tour)
            };
            let montant = ((part + 4) / 5 * 5).min(reste);
            self.appel(
                "encaisser",
                json!({ "cle_idempotence": format!("demonstration-{}-{tour}-{}", annee.libelle, texte(&inscription)),
                    "inscription_id": inscription, "montant": montant, "mode": "especes",
                    "reference": "", "payeur_id": preparation["payeurs"][0]["responsable_id"],
                    "payeur_nom": "", "repartition": null }),
            )
            .await?;
            recus += 1;
        }
        let session = self.appel("caisse_session_courante", Value::Null).await?;
        self.appel(
            "caisse_cloturer",
            json!({ "total_compte": session["totaux"]["theorique"], "motif_ecart": "" }),
        )
        .await?;
        dire(&format!("  {} : {recus} reçus le {jour}", annee.libelle));
        Ok(())
    }

    /// Un devoir ou une composition dans chaque matière au programme de chaque classe, notée
    /// et validée.
    async fn evaluations(
        &mut self,
        annee: &Annee,
        rang: i64,
        composition: bool,
        jour: DateCivile,
    ) -> R<()> {
        self.aller_au(jour).await?;
        let inscrits = self.inscrits(annee).await?;
        let mut nombre = 0;
        for classe in &annee.classes {
            if !composition && classe.premier_cycle {
                continue;
            }
            let Some(periode) = liste(&annee.pedagogie["periodes"])
                .iter()
                .find(|p| {
                    p["periode"]["niveau_id"] == classe.niveau_id && p["periode"]["rang"] == rang
                })
                .map(|p| p["periode"].clone())
            else {
                continue;
            };
            // Les trimestres du 1er cycle n'ont pas les dates du 2e : l'évaluation se date dans
            // celui de sa classe, au plus tard aujourd'hui.
            let debut = date(&texte(&periode["debut"]))?;
            let fin = date(&texte(&periode["fin"]))?;
            let jour_classe = DateCivile::depuis_jours_epoque(
                jour.jours_depuis_epoque()
                    .min(fin.jours_depuis_epoque())
                    .max(debut.jours_depuis_epoque()),
            )
            .unwrap_or(jour);
            let periode = periode["id"].clone();
            let matieres: Vec<Value> = liste(&annee.pedagogie["programme"])
                .iter()
                .filter(|l| {
                    l["niveau_id"] == classe.niveau_id
                        && (l["serie_id"].is_null() || l["serie_id"] == classe.serie_id)
                })
                .map(|l| l["matiere_id"].clone())
                .collect();
            let eleves: Vec<&(Value, String, Value)> = inscrits
                .iter()
                .filter(|(_, _, c)| *c == classe.id)
                .collect();
            for matiere in matieres {
                let evaluation = self
                    .appel(
                        "evaluation_creer",
                        json!({ "classe_id": classe.id, "periode_id": periode, "matiere_id": matiere,
                            "type_evaluation": if composition { "composition" } else { "devoir" },
                            "libelle": if composition { "Composition" } else { "Devoir" },
                            "date": jour_classe.to_string(), "bareme": null }),
                    )
                    .await?;
                let bareme = evaluation["bareme"].as_i64().unwrap_or(2_000);
                for (inscription, eleve_id, _) in &eleves {
                    let ecart = i64::try_from(self.tirer(700)).unwrap_or(0) - 350;
                    let sur_vingt = (self.profil(eleve_id).aptitude + ecart).clamp(0, 2_000);
                    let note = sur_vingt * bareme / 2_000 / 50 * 50;
                    let saisie = if note % 100 == 0 {
                        format!("{}", note / 100)
                    } else {
                        format!("{},5", note / 100)
                    };
                    self.appel(
                        "note_saisir",
                        json!({ "evaluation_id": evaluation["id"], "inscription_id": inscription, "saisie": saisie }),
                    )
                    .await?;
                }
                self.appel("evaluation_valider", json!({ "id": evaluation["id"] }))
                    .await?;
                nombre += 1;
            }
        }
        dire(&format!(
            "  {} : {nombre} {} le {jour}",
            annee.libelle,
            if composition {
                "compositions"
            } else {
                "devoirs"
            }
        ));
        Ok(())
    }

    /// Bulletins de la période : calculés, validés, publiés.
    async fn bulletins(&mut self, annee: &Annee, rang: i64, jour: DateCivile) -> R<()> {
        self.aller_au(jour).await?;
        for classe in &annee.classes {
            let Some(periode) = liste(&annee.pedagogie["periodes"])
                .iter()
                .find(|p| {
                    p["periode"]["niveau_id"] == classe.niveau_id && p["periode"]["rang"] == rang
                })
                .map(|p| p["periode"]["id"].clone())
            else {
                continue;
            };
            let demande = json!({ "classe_id": classe.id, "periode_id": periode });
            self.appel("resultats_calculer", demande.clone()).await?;
            self.appel("bulletins_valider", demande.clone()).await?;
            self.appel("bulletins_publier", demande).await?;
        }
        dire(&format!(
            "  {} : bulletins du trimestre {rang} publiés",
            annee.libelle
        ));
        Ok(())
    }

    /// L'année vécue jusqu'à sa fin, ou jusqu'au jour final : trois tours de versements, un
    /// devoir et une composition par trimestre, les bulletins.
    async fn vivre(&mut self, annee: &Annee, limite: DateCivile) -> R<()> {
        enum Geste {
            Versements(u8),
            Devoirs(i64),
            Compositions(i64),
            Bulletins(i64),
            Paie(String),
        }
        let mut gestes: Vec<(DateCivile, Geste)> = vec![
            (decaler(annee.rentree, 2)?, Geste::Versements(1)),
            (
                date(&format!("{}-01-06", annee.rentree.annee() + 1))?,
                Geste::Versements(2),
            ),
            (
                date(&format!("{}-04-03", annee.rentree.annee() + 1))?,
                Geste::Versements(3),
            ),
        ];
        let reference = annee
            .classes
            .iter()
            .find(|c| !c.premier_cycle)
            .map(|c| c.niveau_id.clone())
            .unwrap_or(Value::Null);
        for p in liste(&annee.pedagogie["periodes"]) {
            let periode = &p["periode"];
            if periode["niveau_id"] != reference {
                continue;
            }
            let rang = periode["rang"].as_i64().unwrap_or(1);
            let debut = date(&texte(&periode["debut"]))?;
            let fin = date(&texte(&periode["fin"]))?;
            gestes.push((decaler(debut, 20)?, Geste::Devoirs(rang)));
            gestes.push((decaler(fin, -12)?, Geste::Compositions(rang)));
            gestes.push((decaler(fin, -4)?, Geste::Bulletins(rang)));
        }
        // La paie de chaque mois de classe, le 28, pour les années closes seulement : celle de
        // l'année en cours reste à faire à l'écran (porteur du projet, 1er novembre 2026).
        let close = annee.fin.jours_depuis_epoque() < limite.jours_depuis_epoque();
        for rang in (0..10).filter(|_| close) {
            let (an, mois) = if rang < 3 {
                (annee.rentree.annee(), 10 + rang)
            } else {
                (annee.rentree.annee() + 1, rang - 2)
            };
            gestes.push((
                date(&format!("{an}-{mois:02}-28"))?,
                Geste::Paie(format!("{an}-{mois:02}")),
            ));
        }
        gestes.sort_by_key(|(jour, _)| jour.jours_depuis_epoque());
        for (jour, geste) in gestes {
            if jour.jours_depuis_epoque() > limite.jours_depuis_epoque() {
                continue;
            }
            match geste {
                Geste::Versements(tour) => self.versements(annee, tour, jour).await?,
                Geste::Devoirs(rang) => self.evaluations(annee, rang, false, jour).await?,
                Geste::Compositions(rang) => self.evaluations(annee, rang, true, jour).await?,
                Geste::Bulletins(rang) => self.bulletins(annee, rang, jour).await?,
                Geste::Paie(mois) => self.paie(&mois, jour).await?,
            }
        }
        Ok(())
    }

    /// Décisions de fin d'année validées, puis clôture, les deux derniers jours de l'année : après
    /// la paie de juillet, dernier geste de l'année. Rend la décision de chaque élève avec le
    /// niveau de sa classe.
    async fn clore(&mut self, annee: &Annee) -> R<HashMap<String, (String, &'static str)>> {
        self.aller_au(decaler(annee.fin, -2)?).await?;
        let inscriptions: HashMap<String, String> = liste(
            &self
                .appel("inscriptions_lister", json!({ "annee_id": annee.id }))
                .await?,
        )
        .iter()
        .map(|l| {
            (
                texte(&l["inscription"]["id"]),
                texte(&l["inscription"]["eleve_id"]),
            )
        })
        .collect();
        let mut decisions = HashMap::new();
        for classe in &annee.classes {
            let demande = json!({ "classe_id": classe.id });
            let mut lues = self
                .appel("decisions_valider_classe", demande.clone())
                .await?;
            let attente: Vec<Value> = liste(&lues["lignes"])
                .iter()
                .filter(|l| {
                    l["decision"] != "admis"
                        && l["decision"] != "redouble"
                        && l["decision"] != "exclu"
                })
                .map(|l| l["inscription_id"].clone())
                .collect();
            for inscription in &attente {
                self.appel(
                    "decision_poser",
                    json!({ "inscription_id": inscription, "decision": "admis" }),
                )
                .await?;
            }
            if !attente.is_empty() {
                lues = self.appel("decisions_classe_lire", demande).await?;
            }
            for l in liste(&lues["lignes"]) {
                if let Some(eleve) = inscriptions.get(&texte(&l["inscription_id"])) {
                    decisions.insert(eleve.clone(), (texte(&l["decision"]), classe.code));
                }
            }
        }
        self.aller_au(decaler(annee.fin, -1)?).await?;
        self.appel("annee_cloturer", json!({ "id": annee.id }))
            .await?;
        let redoublants = decisions.values().filter(|(d, _)| d == "redouble").count();
        dire(&format!(
            "  {} clôturée : {} décisions, dont {redoublants} redoublements",
            annee.libelle,
            decisions.len()
        ));
        Ok(decisions)
    }
}

async fn lancer() -> R<()> {
    let arguments: Vec<String> = std::env::args().skip(1).collect();
    let mode = arguments.first().map(String::as_str);
    let (racine, trousseau, essai): (PathBuf, Arc<dyn Trousseau>, bool) = match mode {
        Some("essai") => {
            let dossier = PathBuf::from(arguments.get(1).ok_or(USAGE)?);
            let cles = dossier.join("trousseau-essai");
            (
                dossier,
                Arc::new(trousseau::TrousseauEssai::nouveau(cles)),
                true,
            )
        }
        Some("poste") => {
            let appdata = std::env::var_os("APPDATA").ok_or("APPDATA absent")?;
            let trousseau = trousseau::du_systeme().map_err(|e| format!("{e:?}"))?;
            (
                PathBuf::from(appdata).join("ml.gesco.poste"),
                trousseau,
                false,
            )
        }
        _ => return Err(USAGE.to_owned()),
    };
    dire(&format!("Base : {}", racine.display()));
    let horloge = Arc::new(HorlogeFixe::a(
        gesco_core::commun::Horodatage::depuis_millis_epoque(huit_heures(date(JOUR_DEPART)?)),
    ));
    let poste = std::env::var("COMPUTERNAME").unwrap_or_else(|_| "poste".to_owned());
    let application = Application::demarrer(OptionsApplication {
        racine: racine.clone(),
        empreinte_machine: empreinte::du_poste(&poste),
        poste: Poste(poste),
        trousseau,
        horloge: horloge.clone(),
        politique_session: PolitiqueSession::default(),
        version_applicative: env!("CARGO_PKG_VERSION").to_owned(),
        cle_licence: gesco_services::licence::cle_publique_editeur()
            .map_err(|e| format!("{e:?}"))?,
    })
    .await
    .map_err(|e| format!("ouverture de la base : {e:?}"))?;
    let mut ecole = Ecole {
        application,
        horloge,
        identifiant: String::new(),
        mot_de_passe: String::new(),
        jeton: None,
        graine: 0x2024_2025_2026_2027,
        compteur: 0,
        profils: HashMap::new(),
        enseignants: HashMap::new(),
        personnes_enseignants: HashMap::new(),
    };
    let phase = ecole.appel("etat_demarrage", Value::Null).await?["phase"].clone();
    match (phase.as_str(), essai) {
        (Some("premier_demarrage"), true) => {
            ecole
                .appel(
                    "premier_demarrage",
                    json!({
                        "organisation": "Groupe scolaire de démonstration",
                        "etablissement_nom": "École de démonstration Askia",
                        "statut": "prive", "code_academie": "AE-BKO-RD", "code_cap": "CAP-01",
                        "adresse": "Hamdallaye ACI 2000, Bamako", "telephones": ["+223 20 22 00 00"],
                        "administrateur_identifiant": "admin",
                        "administrateur_nom": "Administrateur de démonstration",
                        "administrateur_mot_de_passe": MOT_DE_PASSE_ESSAI,
                        "cycles": ["premier_cycle", "second_cycle", "lycee"],
                    }),
                )
                .await?;
            std::fs::write(
                racine.join("LISEZ-MOI.txt"),
                format!(
                    "École de démonstration remplie par gesco-remplir.\r\nIdentifiant : admin\r\nMot de passe : {MOT_DE_PASSE_ESSAI}\r\n"
                ),
            )
            .map_err(|e| e.to_string())?;
            ecole.identifiant = "admin".to_owned();
            ecole.mot_de_passe = MOT_DE_PASSE_ESSAI.to_owned();
        }
        (Some("pret"), true) => {
            ecole.identifiant = "admin".to_owned();
            ecole.mot_de_passe = MOT_DE_PASSE_ESSAI.to_owned();
        }
        (Some("pret"), false) => {
            ecole.identifiant = lire_ligne("Identifiant de l'administrateur : ")?;
            ecole.mot_de_passe = lire_secret("Mot de passe")?;
        }
        (autre, _) => {
            return Err(format!(
                "la base n'est pas prête (phase {autre:?}) : faites d'abord le premier démarrage dans GESCO"
            ));
        }
    }
    ecole.connecter().await?;
    if !liste(&ecole.appel("annees_lister", Value::Null).await?).is_empty() {
        return Err("la base a déjà des années scolaires : rien n'est ajouté".to_owned());
    }

    let limite = date(JOUR_FINAL)?;
    let mut precedente: Option<HashMap<String, (String, &'static str)>> = None;
    for (libelle, rentree, fin) in ANNEES {
        dire(&format!("Année {libelle}"));
        let (rentree, fin) = (date(rentree)?, date(fin)?);
        let annee = match &precedente {
            None => ecole.premiere_annee(libelle, rentree, fin).await?,
            Some(decisions) => {
                ecole
                    .annee_suivante(decisions, libelle, rentree, fin)
                    .await?
            }
        };
        ecole.vivre(&annee, limite).await?;
        if annee.fin.jours_depuis_epoque() < limite.jours_depuis_epoque() {
            precedente = Some(ecole.clore(&annee).await?);
        }
    }
    ecole.aller_au(limite).await?;
    ecole
        .application
        .fermer()
        .await
        .map_err(|e| format!("{e:?}"))?;
    dire("Terminé : 2024-2025 et 2025-2026 closes, 2026-2027 en cours au 31 octobre 2026.");
    Ok(())
}

/// Pile du fil qui remplit. Celle du fil principal de Windows, 1 Mo, déborde dès l'ouverture de
/// la base quand l'outil est compilé sans optimisation (`cargo run -p gesco-remplir`).
const PILE: usize = 64 * 1024 * 1024;

/// Le remplissage sur son propre fil, à grande pile.
fn remplir() -> R<()> {
    let fil = std::thread::Builder::new()
        .stack_size(PILE)
        .spawn(|| {
            tokio::runtime::Builder::new_multi_thread()
                .enable_all()
                .build()
                .map_err(|e| e.to_string())?
                .block_on(lancer())
        })
        .map_err(|e| e.to_string())?;
    fil.join()
        .unwrap_or_else(|_| Err("le remplissage s'est interrompu".to_owned()))
}

fn main() {
    let resultat = remplir();
    if let Err(e) = &resultat {
        let _ = writeln!(std::io::stderr(), "Échec : {e}");
    }
    let _ = lire_ligne("Appuyez sur Entrée pour fermer cette fenêtre.");
    if resultat.is_err() {
        std::process::exit(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn les_classes_se_suivent_de_la_1re_a_la_12e() {
        let codes: Vec<&str> = CLASSES.iter().map(|(c, _, _)| *c).collect();
        assert_eq!(codes.len(), 12);
        assert_eq!(codes.first(), Some(&"1"));
        assert_eq!(codes.last(), Some(&"12"));
    }

    #[test]
    fn les_dates_des_annees_se_lisent() {
        for (_, rentree, fin) in ANNEES {
            assert!(date(rentree).is_ok() && date(fin).is_ok());
        }
        assert!(date(JOUR_FINAL).is_ok());
    }
}
