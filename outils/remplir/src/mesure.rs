//! `gesco-remplir mesurer` — la mesure à plusieurs postes (SPEC_PHASE_11_A §6.5). Sur une école
//! d'essai déjà remplie, le poste principal allume son réseau ; dix postes simulés s'y relient
//! par le vrai canal chiffré, ouvrent chacun leur session, puis jouent ensemble : trois
//! inscrivent, trois encaissent, deux notent, deux lisent. Chaque commande est chronométrée du
//! départ de la demande à l'arrivée de la réponse, comme l'écran d'un poste relié l'attend.
//!
//! L'école mesurée est modifiée (comptes, élèves, versements, notes, licence d'essai) : lancer
//! sur une copie de l'école d'essai, jamais sur une école qui sert.

use std::collections::BTreeMap;
use std::net::{Ipv4Addr, SocketAddr};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};

use gesco_core::commun::DateCivile;
use gesco_core::core::Poste;
use gesco_core::core::licence::{self, Edition, Licence};
use gesco_reseau::relais::{self, Relais};
use gesco_services::session::{Jeton, PolitiqueSession};
use gesco_services::{Application, Horloge, HorlogeSysteme, OptionsApplication};
use serde_json::{Value, json};

use crate::{MOT_DE_PASSE_ESSAI, R, dire, empreinte, liste, texte, trousseau};

/// Clé privée de la licence d'essai que la mesure installe : elle ne vaut que pour l'application
/// ouverte par cet outil, qui reçoit la clé publique correspondante.
const GRAINE_MESURE: [u8; 32] = [7; 32];

/// Mot de passe des comptes créés pour la mesure.
const MOT_DE_PASSE_POSTE: &str = "mesure-dix-postes-2026";

/// Cible : une commande courante répond en moins d'une seconde dans 95 cas sur 100.
const CIBLE: Duration = Duration::from_secs(1);

/// Ce que fait un poste pendant la mesure.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Travail {
    Inscriptions,
    Versements,
    Notes,
    Lectures,
}

impl Travail {
    /// Dix postes : trois inscrivent, trois encaissent, deux notent, deux lisent.
    const fn du_poste(rang: usize) -> Self {
        match rang % 10 {
            0 | 4 | 7 => Self::Inscriptions,
            1 | 5 | 8 => Self::Versements,
            2 | 6 => Self::Notes,
            _ => Self::Lectures,
        }
    }

    const fn nom(self) -> &'static str {
        match self {
            Self::Inscriptions => "inscriptions",
            Self::Versements => "versements",
            Self::Notes => "notes",
            Self::Lectures => "lectures",
        }
    }
}

/// Réglages de la mesure.
#[derive(Debug, Clone, Copy)]
pub struct Reglages {
    /// Nombre de postes simulés.
    pub postes: usize,
    /// Nombre de tours de chaque poste.
    pub tours: usize,
    /// Pause entre deux commandes d'un poste, en millisecondes.
    pub pause_ms: u64,
}

/// Une commande chronométrée : son nom, sa durée, et si elle a abouti.
type Chrono = (String, Duration, bool);

/// Un poste simulé : son relais, sa session, ses chronos.
struct PosteSimule {
    rang: usize,
    relais: Relais,
    jeton: String,
    chronos: Vec<Chrono>,
    refus: Vec<String>,
    pause: Duration,
}

impl PosteSimule {
    /// Envoie une commande au poste principal et la chronomètre ; un refus est compté et gardé,
    /// il n'arrête pas la mesure.
    async fn commande(&mut self, nom: &str, entree: Value) -> Option<Value> {
        tokio::time::sleep(self.pause).await;
        let depart = Instant::now();
        let reponse = self.relais.executer(nom, Some(&self.jeton), entree).await;
        let duree = depart.elapsed();
        match reponse {
            Ok(valeur) => {
                self.chronos.push((nom.to_owned(), duree, true));
                Some(valeur)
            }
            Err(e) => {
                self.chronos.push((nom.to_owned(), duree, false));
                if self.refus.len() < 5 {
                    self.refus.push(format!("{nom} : {}", e.cle));
                }
                None
            }
        }
    }
}

/// Ce que tous les postes lisent une fois avant de jouer.
struct Terrain {
    annee_id: Value,
    aujourd_hui: DateCivile,
    /// Classes de l'année : (id, niveau, série).
    classes: Vec<(Value, Value, Value)>,
    /// Inscriptions validées : (inscription, élève, classe).
    inscrits: Vec<(Value, Value, Value)>,
    pedagogie: Value,
}

async fn sur_place(application: &Application, jeton: &Jeton, nom: &str, entree: Value) -> R<Value> {
    application
        .executer(nom, Some(jeton.clone()), entree)
        .await
        .map_err(|e| format!("{nom} : {e:?}"))
}

/// Un tour d'un poste qui inscrit : la fiche, le responsable, l'inscription, sa validation.
async fn tour_inscriptions(poste: &mut PosteSimule, terrain: &Terrain, tour: usize) {
    let Some((classe, _, _)) = terrain
        .classes
        .get((poste.rang * 5 + tour) % terrain.classes.len().max(1))
    else {
        return;
    };
    let numero = poste.rang * 1_000 + tour;
    let Some(fiche) = poste
        .commande(
            "eleve_creer",
            json!({ "identite": { "nom": "Mesure", "prenoms": format!("Élève {numero}"),
                "sexe": if tour.is_multiple_of(2) { "feminin" } else { "masculin" },
                "date_naissance": "2014-03-02", "date_naissance_estimee": false,
                "lieu_naissance": "Bamako", "nationalite": "Malienne", "adresse": "Bamako",
                "telephones": [] },
                "matricule_interne": "",
                "matricule_national": format!("ML-MESURE-{numero:06}"),
                "acte_nature": "acte", "acte_numero": "", "acte_date": "", "ecole_origine": "" }),
        )
        .await
    else {
        return;
    };
    let eleve_id = fiche["eleve"]["id"].clone();
    poste
        .commande(
            "responsable_rattacher",
            json!({ "eleve_id": eleve_id, "responsable_id": null,
                "saisie": { "identite": { "nom": "Mesure", "prenoms": format!("Parent {numero}"),
                    "sexe": "feminin", "date_naissance": "", "date_naissance_estimee": false,
                    "lieu_naissance": "", "nationalite": "", "adresse": "Bamako",
                    "telephones": [format!("79 {:02} {:02} {:02}", poste.rang, tour / 100, tour % 100)] },
                    "profession": "", "employeur": "" },
                "qualites": { "lien": "mere", "est_responsable_legal": true, "est_payeur": true,
                    "recoit_notifications": true, "vient_chercher": false, "ordre_priorite": 1 } }),
        )
        .await;
    let Some(inscription) = poste
        .commande(
            "inscription_creer",
            json!({ "eleve_id": eleve_id, "classe_id": classe, "date_inscription": "",
                "redoublant": false, "etablissement_origine": "" }),
        )
        .await
    else {
        return;
    };
    poste
        .commande(
            "inscription_valider",
            json!({ "id": inscription["inscription"]["id"] }),
        )
        .await;
}

/// Un tour d'un poste qui encaisse : ce que la famille doit, puis un versement et son reçu.
async fn tour_versements(poste: &mut PosteSimule, terrain: &Terrain, tour: usize, postes: usize) {
    // Chaque poste a ses familles : deux caissiers n'encaissent pas la même inscription.
    let miennes: Vec<&(Value, Value, Value)> = terrain
        .inscrits
        .iter()
        .enumerate()
        .filter(|(i, _)| i % postes.max(1) == poste.rang)
        .map(|(_, l)| l)
        .collect();
    let Some((inscription, _, _)) = miennes.get(tour % miennes.len().max(1)) else {
        return;
    };
    let Some(preparation) = poste
        .commande(
            "encaissement_preparer",
            json!({ "inscription_id": inscription, "montant": 0 }),
        )
        .await
    else {
        return;
    };
    let reste = preparation["reste"].as_i64().unwrap_or(0);
    if reste <= 0 {
        return;
    }
    poste
        .commande(
            "encaisser",
            json!({ "cle_idempotence": format!("mesure-{}-{tour}-{}", poste.rang, texte(inscription)),
                "inscription_id": inscription, "montant": reste.min(5_000), "mode": "especes",
                "reference": "", "payeur_id": preparation["payeurs"][0]["responsable_id"],
                "payeur_nom": "", "repartition": null }),
        )
        .await;
}

/// L'évaluation qu'un poste qui note remplit : la sienne, dans sa classe.
struct Evaluation {
    id: Value,
    classe: Value,
    periode: Value,
    eleves: Vec<Value>,
}

/// Crée l'évaluation d'un poste qui note, dans la période où tombe aujourd'hui.
async fn ouvrir_evaluation(poste: &mut PosteSimule, terrain: &Terrain) -> Option<Evaluation> {
    let jour = terrain.aujourd_hui.jours_depuis_epoque();
    for decalage in 0..terrain.classes.len() {
        let (classe, niveau, serie) = terrain
            .classes
            .get((poste.rang + decalage) % terrain.classes.len())?;
        let Some(periode) = liste(&terrain.pedagogie["periodes"])
            .iter()
            .map(|p| &p["periode"])
            .find(|p| {
                let borne = |champ: &str| {
                    DateCivile::analyser(&texte(&p[champ])).map(|d| d.jours_depuis_epoque())
                };
                p["niveau_id"] == *niveau
                    && borne("debut").is_some_and(|d| d <= jour)
                    && borne("fin").is_some_and(|f| f >= jour)
            })
        else {
            continue;
        };
        let Some(matiere) = liste(&terrain.pedagogie["programme"])
            .iter()
            .find(|l| {
                l["niveau_id"] == *niveau && (l["serie_id"].is_null() || l["serie_id"] == *serie)
            })
            .map(|l| l["matiere_id"].clone())
        else {
            continue;
        };
        let evaluation = poste
            .commande(
                "evaluation_creer",
                json!({ "classe_id": classe, "periode_id": periode["id"], "matiere_id": matiere,
                    "type_evaluation": "composition",
                    "libelle": format!("Composition du poste {}", poste.rang + 1),
                    "date": terrain.aujourd_hui.to_string(), "bareme": null }),
            )
            .await?;
        return Some(Evaluation {
            id: evaluation["id"].clone(),
            classe: classe.clone(),
            periode: periode["id"].clone(),
            eleves: terrain
                .inscrits
                .iter()
                .filter(|(_, _, c)| c == classe)
                .map(|(i, _, _)| i.clone())
                .collect(),
        });
    }
    None
}

/// Un tour d'un poste qui note : une note au clavier ; tous les dix tours, la liste des
/// évaluations de la classe et la grille.
async fn tour_notes(poste: &mut PosteSimule, evaluation: &Evaluation, tour: usize) {
    if let Some(inscription) = evaluation.eleves.get(tour % evaluation.eleves.len().max(1)) {
        poste
            .commande(
                "note_saisir",
                json!({ "evaluation_id": evaluation.id, "inscription_id": inscription,
                    // De 3 à 8 : dans le barème du 1er cycle (sur 10) comme des autres (sur 20).
                    "saisie": format!("{}", 3 + tour % 6) }),
            )
            .await;
    }
    if tour.is_multiple_of(10) {
        poste
            .commande(
                "evaluations_lister",
                json!({ "classe_id": evaluation.classe, "periode_id": evaluation.periode }),
            )
            .await;
        poste
            .commande("notes_grille", json!({ "id": evaluation.id }))
            .await;
    }
}

/// Un tour d'un poste qui lit : une recherche, une fiche, puis à tour de rôle les listes
/// lourdes (inscriptions de l'année, impayés, tableau de bord).
async fn tour_lectures(poste: &mut PosteSimule, terrain: &Terrain, tour: usize) {
    const DEBUTS: [&str; 6] = ["Tra", "Dia", "Cou", "Keï", "Awa", "Mou"];
    let debut = DEBUTS.get(tour % DEBUTS.len()).copied().unwrap_or("Tra");
    poste
        .commande("eleves_rechercher", json!({ "texte": debut }))
        .await;
    if let Some((_, eleve, _)) = terrain
        .inscrits
        .get((poste.rang * 31 + tour * 7) % terrain.inscrits.len().max(1))
    {
        poste.commande("eleve_lire", json!({ "id": eleve })).await;
    }
    match tour % 3 {
        0 => {
            poste
                .commande(
                    "inscriptions_lister",
                    json!({ "annee_id": terrain.annee_id }),
                )
                .await;
        }
        1 => {
            poste.commande("impayes_lister", Value::Null).await;
        }
        _ => {
            poste.commande("tableau_de_bord_lire", Value::Null).await;
        }
    }
}

/// Le travail d'un poste, du premier au dernier tour.
async fn jouer(mut poste: PosteSimule, terrain: Arc<Terrain>, reglages: Reglages) -> PosteSimule {
    let travail = Travail::du_poste(poste.rang);
    let evaluation = if travail == Travail::Notes {
        ouvrir_evaluation(&mut poste, &terrain).await
    } else {
        None
    };
    if travail == Travail::Versements {
        for _ in 0..3 {
            if poste
                .commande("caisse_ouvrir", json!({ "fonds_initial": 0 }))
                .await
                .is_some()
            {
                break;
            }
        }
    }
    for tour in 0..reglages.tours {
        match travail {
            Travail::Inscriptions => tour_inscriptions(&mut poste, &terrain, tour).await,
            Travail::Versements => {
                tour_versements(&mut poste, &terrain, tour, reglages.postes).await;
            }
            Travail::Notes => {
                if let Some(e) = &evaluation {
                    tour_notes(&mut poste, e, tour).await;
                }
            }
            Travail::Lectures => tour_lectures(&mut poste, &terrain, tour).await,
        }
    }
    poste
}

/// Valeur au rang donné (de 0 à 100) d'une liste de durées triée.
fn centile(triees: &[Duration], rang: usize) -> Duration {
    let dernier = triees.len().saturating_sub(1);
    triees
        .get((triees.len() * rang / 100).min(dernier))
        .copied()
        .unwrap_or_default()
}

fn millis(duree: Duration) -> String {
    format!("{:>6} ms", duree.as_millis())
}

/// Le compte rendu : par commande, puis l'ensemble, et la cible.
fn rendre_compte(postes: &[PosteSimule], duree: Duration, a_vide: &mut [Duration]) {
    let mut par_commande: BTreeMap<&str, Vec<Duration>> = BTreeMap::new();
    let mut toutes: Vec<Duration> = Vec::new();
    let mut refusees = 0_usize;
    for poste in postes {
        for (nom, d, aboutie) in &poste.chronos {
            if *aboutie {
                par_commande.entry(nom).or_default().push(*d);
                toutes.push(*d);
            } else {
                refusees += 1;
            }
        }
    }
    dire(&format!(
        "{:<26} {:>6} {:>9} {:>9} {:>9}",
        "commande", "nombre", "médiane", "95 %", "maximum"
    ));
    for (nom, durees) in &mut par_commande {
        durees.sort();
        dire(&format!(
            "{nom:<26} {:>6} {} {} {}",
            durees.len(),
            millis(centile(durees, 50)),
            millis(centile(durees, 95)),
            millis(centile(durees, 100)),
        ));
    }
    toutes.sort();
    a_vide.sort();
    let sous_la_cible = toutes.iter().filter(|d| **d < CIBLE).count();
    dire(&format!(
        "{:<26} {:>6} {} {} {}",
        "ensemble",
        toutes.len(),
        millis(centile(&toutes, 50)),
        millis(centile(&toutes, 95)),
        millis(centile(&toutes, 100)),
    ));
    dire(&format!(
        "Aller-retour à vide (canal chiffré ouvert, état lu, canal fermé) : médiane {}, 95 % {}",
        millis(centile(a_vide, 50)).trim(),
        millis(centile(a_vide, 95)).trim(),
    ));
    dire(&format!(
        "{} commandes en {:.1} s ({:.1} par seconde), {refusees} refusées.",
        toutes.len(),
        duree.as_secs_f64(),
        f64::from(u32::try_from(toutes.len()).unwrap_or(u32::MAX)) / duree.as_secs_f64().max(0.001),
    ));
    let part = sous_la_cible * 1_000 / toutes.len().max(1);
    dire(&format!(
        "Sous une seconde : {sous_la_cible} sur {} ({},{} %). Cible de 95 % : {}.",
        toutes.len(),
        part / 10,
        part % 10,
        if part >= 950 { "tenue" } else { "manquée" },
    ));
    for poste in postes {
        for refus in &poste.refus {
            dire(&format!(
                "  refus au poste {} ({}) — {refus}",
                poste.rang + 1,
                Travail::du_poste(poste.rang).nom()
            ));
        }
    }
}

/// Lit ce que tous les postes ont besoin de connaître avant de jouer.
async fn lire_terrain(
    application: &Application,
    admin: &Jeton,
    horloge: &dyn Horloge,
) -> R<Terrain> {
    let annees = sur_place(application, admin, "annees_lister", Value::Null).await?;
    let annee_id = liste(&annees)
        .iter()
        .find(|a| a["est_courante"] == true)
        .map(|a| a["id"].clone())
        .ok_or("aucune année en cours : l'école d'essai est-elle remplie ?")?;
    let classes = sur_place(
        application,
        admin,
        "classes_lister",
        json!({ "annee_id": annee_id }),
    )
    .await?;
    let inscriptions = sur_place(
        application,
        admin,
        "inscriptions_lister",
        json!({ "annee_id": annee_id }),
    )
    .await?;
    let pedagogie = sur_place(
        application,
        admin,
        "pedagogie_lire",
        json!({ "id": annee_id }),
    )
    .await?;
    Ok(Terrain {
        aujourd_hui: DateCivile::du_jour(horloge.maintenant()).ok_or("date du jour illisible")?,
        classes: liste(&classes)
            .iter()
            .map(|c| {
                (
                    c["classe"]["id"].clone(),
                    c["classe"]["niveau_id"].clone(),
                    c["classe"]["serie_id"].clone(),
                )
            })
            .collect(),
        inscrits: liste(&inscriptions)
            .iter()
            .filter(|l| l["inscription"]["statut"] == "validee")
            .map(|l| {
                (
                    l["inscription"]["id"].clone(),
                    l["inscription"]["eleve_id"].clone(),
                    l["inscription"]["classe_id"].clone(),
                )
            })
            .collect(),
        annee_id,
        pedagogie,
    })
}

/// Relie un poste simulé comme son écran le ferait : aperçu, code, relais, session.
async fn relier(
    application: &Application,
    admin: &Jeton,
    adresse: &str,
    version: &str,
    rang: usize,
    pause: Duration,
) -> R<PosteSimule> {
    let nom = format!("Poste {:02}", rang + 1);
    let identifiant = format!("mesure{:02}", rang + 1);
    sur_place(
        application,
        admin,
        "utilisateur_creer",
        json!({ "identifiant": identifiant, "nom_affiche": format!("Mesure {:02}", rang + 1),
            "mot_de_passe_initial": MOT_DE_PASSE_POSTE, "roles": ["administrateur"] }),
    )
    .await?;
    let ouvert = sur_place(application, admin, "poste_appairage_ouvrir", Value::Null).await?;
    let code = texte(&ouvert["appairage"]["code"]).replace(' ', "");
    let apercu = relais::apercevoir(adresse)
        .await
        .map_err(|e| format!("aperçu du poste principal : {}", e.cle))?;
    let (liaison, _) = relais::appairer(&apercu, version, &code, &nom)
        .await
        .map_err(|e| format!("appairage de {nom} : {}", e.cle))?;
    let relais = Relais::nouveau(liaison, version).map_err(|e| e.cle)?;
    let session = relais
        .executer(
            "connexion",
            None,
            json!({ "identifiant": identifiant, "mot_de_passe": MOT_DE_PASSE_POSTE }),
        )
        .await
        .map_err(|e| format!("connexion de {nom} : {}", e.cle))?;
    Ok(PosteSimule {
        rang,
        relais,
        jeton: texte(&session["jeton"]),
        chronos: Vec::new(),
        refus: Vec::new(),
        pause,
    })
}

/// La mesure, de l'ouverture de l'école au compte rendu.
pub async fn lancer(racine: PathBuf, reglages: Reglages) -> R<()> {
    dire(&format!("Base : {}", racine.display()));
    let horloge = Arc::new(HorlogeSysteme);
    let poste = std::env::var("COMPUTERNAME").unwrap_or_else(|_| "poste".to_owned());
    let empreinte_machine = empreinte::du_poste(&poste);
    let version = env!("CARGO_PKG_VERSION");
    let application = Arc::new(
        Application::demarrer(OptionsApplication {
            racine: racine.clone(),
            empreinte_machine: empreinte_machine.clone(),
            poste: Poste(poste),
            trousseau: Arc::new(trousseau::TrousseauEssai::nouveau(
                racine.join("trousseau-essai"),
            )),
            horloge: horloge.clone(),
            politique_session: PolitiqueSession::default(),
            version_applicative: version.to_owned(),
            cle_licence: licence::cle_publique(&GRAINE_MESURE),
        })
        .await
        .map_err(|e| format!("ouverture de la base : {e:?}"))?,
    );
    let session = application
        .executer(
            "connexion",
            None,
            json!({ "identifiant": "admin", "mot_de_passe": MOT_DE_PASSE_ESSAI }),
        )
        .await
        .map_err(|e| format!("connexion : {e:?}"))?;
    let admin: Jeton =
        serde_json::from_value(session["jeton"].clone()).map_err(|e| e.to_string())?;

    // Une licence d'essai qui couvre les postes de la mesure, puis le réseau allumé.
    let contenu = licence::signer(
        &Licence {
            licence_id: "LIC-MESURE".to_owned(),
            etablissement: "École de démonstration".to_owned(),
            edition: Edition::Essentielle,
            modules: vec!["postes:illimites".to_owned()],
            empreinte_machine,
            emise_le: horloge.maintenant(),
            echeance: None,
        },
        &GRAINE_MESURE,
    )
    .map_err(|e| format!("{e:?}"))?;
    sur_place(
        &application,
        &admin,
        "licence_installer",
        json!({ "contenu": contenu }),
    )
    .await?;
    sur_place(&application, &admin, "reseau_activer", Value::Null).await?;
    let mut serveur = gesco_reseau::superviser(Arc::clone(&application), |_port| {
        SocketAddr::from((Ipv4Addr::LOCALHOST, 0))
    });
    let adresse = serveur
        .wait_for(Option::is_some)
        .await
        .map_err(|e| e.to_string())?
        .ok_or("le serveur n'écoute pas")?
        .to_string();

    let terrain = Arc::new(lire_terrain(&application, &admin, horloge.as_ref()).await?);
    dire(&format!(
        "École : {} classes, {} inscriptions validées dans l'année en cours.",
        terrain.classes.len(),
        terrain.inscrits.len()
    ));
    let pause = Duration::from_millis(reglages.pause_ms);
    let mut postes = Vec::with_capacity(reglages.postes);
    for rang in 0..reglages.postes {
        postes.push(relier(&application, &admin, &adresse, version, rang, pause).await?);
    }
    dire(&format!(
        "{} postes reliés ; {} tours chacun, {} ms entre deux commandes.",
        postes.len(),
        reglages.tours,
        reglages.pause_ms
    ));

    // L'aller-retour à vide : ce que coûte le canal lui-même, ouvert à chaque commande.
    let mut a_vide = Vec::new();
    if let Some(premier) = postes.first() {
        for _ in 0..30 {
            let depart = Instant::now();
            premier.relais.verifier().await.map_err(|e| e.cle)?;
            a_vide.push(depart.elapsed());
        }
    }

    let depart = Instant::now();
    let taches: Vec<_> = postes
        .into_iter()
        .map(|p| tokio::spawn(jouer(p, Arc::clone(&terrain), reglages)))
        .collect();
    let mut finis = Vec::with_capacity(taches.len());
    for tache in taches {
        finis.push(tache.await.map_err(|e| e.to_string())?);
    }
    rendre_compte(&finis, depart.elapsed(), &mut a_vide);

    sur_place(&application, &admin, "reseau_eteindre", Value::Null).await?;
    application.fermer().await.map_err(|e| format!("{e:?}"))?;
    Ok(())
}
