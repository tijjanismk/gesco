//! Statistiques de l'école par la frontière (SPEC_PHASE_9 §9.1, §10, critères 1 à 4) : chaque
//! total est égal au décompte fait ici, à part, sur les élèves inscrits ; la date d'arrêt change
//! les chiffres ; une fiche sans sexe ou sans date de naissance compte dans le total sans entrer
//! dans une colonne ; une école vide a ses tableaux vides, sans refus.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]

use std::collections::BTreeMap;

use crate::commun::*;
use gesco_services::session::Jeton;
use serde_json::{Value, json};
use sqlx::Connection;

fn id_par(liste: &Value, champ: &str, valeur: &str) -> Value {
    liste
        .as_array()
        .unwrap()
        .iter()
        .find(|x| x[champ] == valeur)
        .unwrap_or_else(|| panic!("{champ} = {valeur} absent"))["id"]
        .clone()
}

/// Un élève de l'essai, tel qu'il est inscrit : la vérité contre laquelle on compte.
struct Eleve {
    prenoms: &'static str,
    classe: &'static str,
    niveau: &'static str,
    cycle: &'static str,
    sexe: Option<&'static str>,
    naissance: Option<i32>,
    redoublant: bool,
    inscrit_le: &'static str,
    /// Jour du départ, s'il part.
    parti_le: Option<&'static str>,
    /// Changement de classe : nouvelle classe et jour.
    change: Option<(&'static str, &'static str)>,
}

const fn eleve(
    prenoms: &'static str,
    classe: &'static str,
    sexe: Option<&'static str>,
    naissance: Option<i32>,
    redoublant: bool,
    inscrit_le: &'static str,
) -> Eleve {
    let (niveau, cycle) = if classe.as_bytes()[0] == b'1' {
        ("1re année", "premier_cycle")
    } else {
        ("7e année", "second_cycle")
    };
    Eleve {
        prenoms,
        classe,
        niveau,
        cycle,
        sexe,
        naissance,
        redoublant,
        inscrit_le,
        parti_le: None,
        change: None,
    }
}

/// Les élèves de l'essai. Cheick n'a pas de date de naissance, Issa pas de sexe (fiches reprises
/// d'un fichier) ; Kadi arrive après la rentrée ; Lamine part le 15 janvier ; Djeneba passe en
/// 7e B le 1er février.
fn eleves() -> Vec<Eleve> {
    let mut v = vec![
        eleve(
            "Awa",
            "1re A",
            Some("feminin"),
            Some(2019),
            false,
            "2026-10-01",
        ),
        eleve(
            "Bakary",
            "1re A",
            Some("masculin"),
            Some(2019),
            true,
            "2026-10-01",
        ),
        eleve(
            "Cheick",
            "1re A",
            Some("masculin"),
            None,
            false,
            "2026-10-01",
        ),
        eleve(
            "Djeneba",
            "7e A",
            Some("feminin"),
            Some(2013),
            false,
            "2026-10-01",
        ),
        eleve(
            "Fanta",
            "7e A",
            Some("feminin"),
            Some(2012),
            true,
            "2026-10-01",
        ),
        eleve("Issa", "7e A", None, Some(2013), false, "2026-10-01"),
        eleve(
            "Kadi",
            "7e A",
            Some("feminin"),
            Some(2013),
            false,
            "2026-11-02",
        ),
        eleve(
            "Lamine",
            "7e B",
            Some("masculin"),
            Some(2012),
            false,
            "2026-10-01",
        ),
    ];
    v[3].change = Some(("7e B", "2027-02-01"));
    v[7].parti_le = Some("2027-01-15");
    v
}

/// Ce que l'essai compte de son côté, à une date : par classe, par sexe, par âge.
struct Decompte {
    total: u64,
    garcons: u64,
    filles: u64,
    par_classe: BTreeMap<String, u64>,
    par_niveau: BTreeMap<String, u64>,
    par_cycle: BTreeMap<String, u64>,
    par_age: BTreeMap<String, u64>,
    sans_naissance: u64,
    sans_sexe: u64,
    redoublants: u64,
}

fn decompter(liste: &[Eleve], jour: &str) -> Decompte {
    let mut d = Decompte {
        total: 0,
        garcons: 0,
        filles: 0,
        par_classe: BTreeMap::new(),
        par_niveau: BTreeMap::new(),
        par_cycle: BTreeMap::new(),
        par_age: BTreeMap::new(),
        sans_naissance: 0,
        sans_sexe: 0,
        redoublants: 0,
    };
    // Les dates sont au format AAAA-MM-JJ : l'ordre des textes est celui des jours.
    for e in liste
        .iter()
        .filter(|e| e.inscrit_le <= jour && e.parti_le.is_none_or(|p| p > jour))
    {
        d.total += 1;
        match e.sexe {
            Some("masculin") => d.garcons += 1,
            Some(_) => d.filles += 1,
            None => d.sans_sexe += 1,
        }
        let classe = match e.change {
            Some((nouvelle, depuis)) if depuis <= jour => nouvelle,
            _ => e.classe,
        };
        *d.par_classe.entry(classe.to_owned()).or_default() += 1;
        *d.par_niveau.entry(e.niveau.to_owned()).or_default() += 1;
        *d.par_cycle.entry(e.cycle.to_owned()).or_default() += 1;
        match e.naissance {
            Some(n) => *d.par_age.entry((2026 - n).to_string()).or_default() += 1,
            None => d.sans_naissance += 1,
        }
        if e.redoublant {
            d.redoublants += 1;
        }
    }
    d
}

fn tableau<'a>(onglet: &'a Value, code: &str) -> &'a Value {
    onglet["tableaux"]
        .as_array()
        .unwrap()
        .iter()
        .find(|t| t["code"] == code)
        .unwrap_or_else(|| panic!("tableau {code} absent"))
}

/// Nombre d'une case : colonne 0 garçons, 1 filles, 2 total.
fn case(ligne: &Value, colonne: usize) -> u64 {
    ligne["cellules"][colonne]["nombre"].as_u64().unwrap()
}

/// Totaux par libellé de ligne (ou par clé quand la ligne n'a pas de libellé).
fn lignes(t: &Value) -> BTreeMap<String, u64> {
    t["lignes"]
        .as_array()
        .unwrap()
        .iter()
        .map(|l| {
            let nom = if l["libelle"] == "" {
                l["cle"].as_str().unwrap().to_owned()
            } else {
                l["libelle"].as_str().unwrap().to_owned()
            };
            (nom, case(l, 2))
        })
        .filter(|(_, n)| *n > 0)
        .collect()
}

fn hors(t: &Value, cle: &str) -> u64 {
    t["hors"]
        .as_array()
        .unwrap()
        .iter()
        .find(|h| h["cle"] == cle)
        .map_or(0, |h| h["membres"].as_array().unwrap().len() as u64)
}

async fn lire(p: &Poste_, jeton: &Jeton, jour: &str) -> Value {
    p.ok("statistiques_lire", Some(jeton), json!({ "date": jour }))
        .await
}

/// Vérifie l'onglet Élèves contre le décompte de l'essai.
fn verifier(stats: &Value, d: &Decompte, jour: &str) {
    let onglet = &stats["eleves"];
    assert_eq!(stats["date"], jour);
    for code in [
        "eleves_par_classe",
        "eleves_par_niveau",
        "eleves_par_cycle",
        "eleves_par_annee_naissance",
        "eleves_par_age",
    ] {
        let t = tableau(onglet, code);
        let total = &t["total"];
        assert_eq!(case(total, 2), d.total, "{code} au {jour}");
        assert_eq!(case(total, 0), d.garcons, "{code} au {jour}");
        assert_eq!(case(total, 1), d.filles, "{code} au {jour}");
        assert_eq!(hors(t, "statistiques.hors.eleves_sans_sexe"), d.sans_sexe);
        // Le total compte les fiches sans sexe : il dépasse la somme des colonnes.
        assert_eq!(
            case(total, 2),
            case(total, 0) + case(total, 1) + d.sans_sexe
        );
    }
    let par_classe = tableau(onglet, "eleves_par_classe");
    assert_eq!(lignes(par_classe), d.par_classe, "classes au {jour}");
    // Toutes les classes de l'année paraissent, même vides.
    assert_eq!(par_classe["lignes"].as_array().unwrap().len(), 3);
    assert_eq!(
        lignes(tableau(onglet, "eleves_par_niveau")),
        d.par_niveau,
        "niveaux au {jour}"
    );
    let cycles: BTreeMap<String, u64> = d
        .par_cycle
        .iter()
        .map(|(c, n)| (format!("niveaux.cycle.{c}"), *n))
        .collect();
    assert_eq!(lignes(tableau(onglet, "eleves_par_cycle")), cycles);
    let ages = tableau(onglet, "eleves_par_age");
    assert_eq!(lignes(ages), d.par_age, "âges au {jour}");
    assert_eq!(
        hors(ages, "statistiques.hors.eleves_sans_naissance"),
        d.sans_naissance
    );
    let redoublants = tableau(onglet, "redoublants_par_niveau");
    assert_eq!(case(&redoublants["total"], 2), d.redoublants);
    // Chaque case désigne les fiches qu'elle compte.
    for l in par_classe["lignes"].as_array().unwrap() {
        assert_eq!(
            l["cellules"][2]["membres"].as_array().unwrap().len() as u64,
            case(l, 2)
        );
    }
}

#[tokio::test]
async fn les_effectifs_a_une_date_egalent_le_decompte_des_inscriptions() {
    let dossier = tempfile::tempdir().unwrap();
    let p = poste_vierge(dossier.path()).await;
    p.installer().await;
    let admin = p.connecter("admin", MOT_DE_PASSE).await;
    let annee = p
        .ok(
            "annee_creer",
            Some(&admin),
            json!({ "libelle": "2026-2027", "debut": RENTREE_2026, "fin": FIN_2027 }),
        )
        .await;
    p.ok("annee_activer", Some(&admin), json!({ "id": annee["id"] }))
        .await;
    poser_grille(&p, &admin, &annee["id"]).await;
    let ped = p
        .ok("pedagogie_lire", Some(&admin), json!({ "id": annee["id"] }))
        .await;
    let niveau = |code: &str| id_par(&ped["niveaux"], "code", code);
    let matiere = |code: &str| id_par(&ped["matieres"], "code", code);
    let mut classes: BTreeMap<&str, Value> = BTreeMap::new();
    for (code, libelle) in [("1", "1re A"), ("7", "7e A"), ("7", "7e B")] {
        classes.insert(
            libelle,
            p.ok(
                "classe_creer",
                Some(&admin),
                json!({ "annee_id": annee["id"], "niveau_id": niveau(code), "serie_id": null, "libelle": libelle }),
            )
            .await["id"]
                .clone(),
        );
    }

    let liste = eleves();
    let mut inscriptions: BTreeMap<&str, Value> = BTreeMap::new();
    for e in &liste {
        let fiche = p
            .ok(
                "eleve_creer",
                Some(&admin),
                json!({ "identite": { "nom": "Traoré", "prenoms": e.prenoms,
                    "sexe": e.sexe.unwrap_or("masculin"),
                    "date_naissance": format!("{}-03-02", e.naissance.unwrap_or(2019)),
                    "date_naissance_estimee": false, "lieu_naissance": "Bamako",
                    "nationalite": "Malienne", "adresse": "", "telephones": [] },
                    "matricule_interne": "", "matricule_national": "", "acte_nature": "acte",
                    "acte_numero": "", "acte_date": "", "ecole_origine": "" }),
            )
            .await;
        p.ok(
            "responsable_rattacher",
            Some(&admin),
            json!({ "eleve_id": fiche["eleve"]["id"], "responsable_id": null,
                "saisie": { "identite": { "nom": "Coulibaly", "prenoms": format!("Mère de {}", e.prenoms),
                    "sexe": "feminin", "date_naissance": "", "date_naissance_estimee": false,
                    "lieu_naissance": "", "nationalite": "", "adresse": "", "telephones": ["76 12 34 56"] },
                    "profession": "", "employeur": "" },
                "qualites": { "lien": "mere", "est_responsable_legal": true, "est_payeur": true,
                    "recoit_notifications": true, "vient_chercher": false, "ordre_priorite": 1 } }),
        )
        .await;
        let inscription = p
            .ok(
                "inscription_creer",
                Some(&admin),
                json!({ "eleve_id": fiche["eleve"]["id"], "classe_id": classes[e.classe],
                    "date_inscription": e.inscrit_le, "redoublant": e.redoublant,
                    "etablissement_origine": "" }),
            )
            .await["inscription"]["id"]
            .clone();
        p.ok(
            "inscription_valider",
            Some(&admin),
            json!({ "id": inscription, "choix": {} }),
        )
        .await;
        inscriptions.insert(e.prenoms, inscription);
    }
    // Un brouillon ne compte jamais.
    let mariam = p
        .ok(
            "eleve_creer",
            Some(&admin),
            json!({ "identite": { "nom": "Sidibé", "prenoms": "Mariam", "sexe": "feminin",
                "date_naissance": "2013-05-05", "date_naissance_estimee": false,
                "lieu_naissance": "Kati", "nationalite": "Malienne", "adresse": "", "telephones": [] },
                "matricule_interne": "", "matricule_national": "", "acte_nature": "acte",
                "acte_numero": "", "acte_date": "", "ecole_origine": "" }),
        )
        .await;
    p.ok(
        "inscription_creer",
        Some(&admin),
        json!({ "eleve_id": mariam["eleve"]["id"], "classe_id": classes["7e B"],
            "date_inscription": "2026-10-01", "redoublant": false, "etablissement_origine": "" }),
    )
    .await;

    // Des fiches reprises d'un fichier, sans date de naissance ou sans sexe : la base les
    // accepte ainsi, l'écran de saisie non.
    let mut cnx = base_brute(dossier.path(), p.trousseau.clone()).await;
    sqlx::query("UPDATE personne SET date_naissance = NULL WHERE prenoms = 'Cheick'")
        .execute(&mut cnx)
        .await
        .unwrap();
    sqlx::query("UPDATE personne SET sexe = NULL WHERE prenoms = 'Issa'")
        .execute(&mut cnx)
        .await
        .unwrap();
    cnx.close().await.unwrap();

    // Le personnel : deux maîtres, dont une dans les deux cycles ; une secrétaire ; un agent
    // qui n'arrive qu'en mars.
    let enseignant = |nom: &str, prenoms: &str| {
        json!({ "id": null, "nom": nom, "prenoms": prenoms, "telephones": ["76 11 22 33"],
            "statut": "titulaire", "utilisateur_id": null, "actif": true })
    };
    let moussa = p
        .ok(
            "enseignant_enregistrer",
            Some(&admin),
            enseignant("Diarra", "Moussa"),
        )
        .await["enseignant"]["id"]
        .clone();
    let aminata = p
        .ok(
            "enseignant_enregistrer",
            Some(&admin),
            enseignant("Koné", "Aminata"),
        )
        .await["enseignant"]["id"]
        .clone();
    p.ok(
        "maitre_affecter",
        Some(&admin),
        json!({ "classe_id": classes["1re A"], "enseignant_id": moussa }),
    )
    .await;
    p.ok(
        "affectation_confier",
        Some(&admin),
        json!({ "classe_id": classes["1re A"], "matiere_id": matiere("DESS"), "enseignant_id": aminata }),
    )
    .await;
    p.ok(
        "affectation_confier",
        Some(&admin),
        json!({ "classe_id": classes["7e A"], "matiere_id": matiere("MATH"), "enseignant_id": aminata }),
    )
    .await;
    let seance = |classe: &Value, jour: u8, debut: u16, fin: u16, matiere: &Value| {
        json!({ "id": null, "classe_id": classe, "jour": jour, "debut": debut, "fin": fin,
            "matiere_id": matiere, "enseignant_id": aminata, "salle": "" })
    };
    p.ok(
        "seance_placer",
        Some(&admin),
        seance(&classes["1re A"], 1, 8 * 60, 9 * 60, &matiere("DESS")),
    )
    .await;
    p.ok(
        "seance_placer",
        Some(&admin),
        seance(&classes["7e A"], 2, 8 * 60, 10 * 60, &matiere("MATH")),
    )
    .await;
    let referentiel = p
        .ok("personnel_referentiel_lire", Some(&admin), Value::Null)
        .await;
    let administration = id_par(&referentiel["categories"], "code", "administration");
    let membre = |prenoms: &str, arrive: &str| {
        json!({
            "id": null, "nom": "Coulibaly", "prenoms": prenoms, "sexe": "feminin",
            "date_naissance": null, "telephones": ["76 11 22 33"], "adresse": "",
            "matricule": "", "categorie_id": administration, "fonction": "Secrétaire",
            "arrive_le": arrive, "fonctionnaire": true, "numero_fonctionnaire": "123",
            "numero_inps": "", "prevenir_nom": "", "prevenir_telephone": "",
            "paiement": { "mode": "especes" },
            "diplomes": [{ "intitule": "BT secrétariat", "delivre_par": "", "annee": 2010 },
                         { "intitule": "DUT", "delivre_par": "", "annee": 2014 }],
            "utilisateur_id": null,
        })
    };
    p.ok(
        "membre_enregistrer",
        Some(&admin),
        membre("Oumou", "2024-10-01"),
    )
    .await;
    p.ok(
        "membre_enregistrer",
        Some(&admin),
        membre("Rokia", "2027-03-01"),
    )
    .await;

    // Le 1er février 2027 : Lamine est parti le 15 janvier, Djeneba change de classe.
    p.horloge.avancer(120 * 86_400_000);
    let admin = p.connecter("admin", MOT_DE_PASSE).await;
    p.ok(
        "inscription_transferer",
        Some(&admin),
        json!({ "inscription_id": inscriptions["Lamine"], "date_depart": "2027-01-15",
            "etablissement_destination": "École de Kati", "motif": "Déménagement",
            "passer_outre": "" }),
    )
    .await;
    p.ok(
        "inscription_changer_classe",
        Some(&admin),
        json!({ "id": inscriptions["Djeneba"], "classe_id": classes["7e B"], "motif": "" }),
    )
    .await;

    // Chaque total égale le décompte de l'essai, à chaque date.
    for jour in [
        "2026-10-01",
        "2026-11-01",
        "2026-11-02",
        "2027-01-10",
        "2027-01-15",
        "2027-01-20",
        "2027-02-10",
    ] {
        let stats = lire(&p, &admin, jour).await;
        verifier(&stats, &decompter(&liste, jour), jour);
    }

    // Lamine compte au 10 janvier, ne compte plus au 20 ; il est alors dans les départs, avec
    // sa classe et son motif.
    let au_10 = lire(&p, &admin, "2027-01-10").await;
    let au_20 = lire(&p, &admin, "2027-01-20").await;
    let septb = |s: &Value| {
        lignes(tableau(&s["eleves"], "eleves_par_classe"))
            .get("7e B")
            .copied()
    };
    assert_eq!(septb(&au_10), Some(1));
    assert_eq!(septb(&au_20), None);
    assert!(
        tableau(&au_10["eleves"], "departs_depuis_rentree")["lignes"]
            .as_array()
            .unwrap()
            .is_empty()
    );
    let departs = tableau(&au_20["eleves"], "departs_depuis_rentree");
    assert_eq!(
        lignes(departs),
        BTreeMap::from([("statistiques.depart.transferee".to_owned(), 1)])
    );
    let lamine = departs["lignes"][0]["cellules"][2]["membres"][0]
        .as_u64()
        .unwrap() as usize;
    let fiche = &au_20["eleves"]["fiches"][lamine];
    assert_eq!(fiche["nom"], "Traoré Lamine");
    assert_eq!(fiche["detail"], "7e B");
    assert_eq!(fiche["motif"], "Déménagement");
    // Kadi est entrée après la rentrée.
    assert_eq!(
        lignes(tableau(&au_20["eleves"], "entrees_depuis_rentree")),
        BTreeMap::from([("inscriptions.type.nouvelle".to_owned(), 1)])
    );

    // Par défaut, la date est aujourd'hui ; un seul cycle se choisit.
    let defaut = p.ok("statistiques_lire", Some(&admin), json!({})).await;
    assert_eq!(defaut["date"], "2027-02-01");
    assert_eq!(defaut["cycles"], json!(["premier_cycle", "second_cycle"]));
    let second = p
        .ok(
            "statistiques_lire",
            Some(&admin),
            json!({ "date": "2027-02-10", "cycle": "second_cycle" }),
        )
        .await;
    let d = decompter(&liste, "2027-02-10");
    assert_eq!(
        case(&tableau(&second["eleves"], "eleves_par_classe")["total"], 2),
        d.par_cycle["second_cycle"]
    );

    // Le personnel compte sur sa fiche : Rokia n'est pas encore arrivée ; les maîtres n'ont pas de
    // sexe sur leur fiche et comptent au total.
    let personnel = &defaut["personnel"];
    let categories = tableau(personnel, "personnel_par_categorie");
    assert_eq!(case(&categories["total"], 2), 3);
    assert_eq!(case(&categories["total"], 1), 1);
    assert_eq!(hors(categories, "statistiques.hors.personnel_sans_sexe"), 2);
    let situation = tableau(personnel, "personnel_par_situation");
    assert_eq!(
        lignes(situation),
        BTreeMap::from([
            ("statistiques.fonctionnaire".to_owned(), 1),
            ("statistiques.non_fonctionnaire".to_owned(), 2),
        ])
    );
    // Aminata enseigne dans les deux cycles : elle compte dans chacun, une fois au total.
    let cycles = tableau(personnel, "enseignants_par_cycle");
    assert_eq!(
        lignes(cycles),
        BTreeMap::from([
            ("niveaux.cycle.premier_cycle".to_owned(), 2),
            ("niveaux.cycle.second_cycle".to_owned(), 1),
        ])
    );
    assert_eq!(case(&cycles["total"], 2), 2);
    let disciplines = lignes(tableau(personnel, "enseignants_par_discipline"));
    assert_eq!(disciplines["Dessin"], 1);
    // Les mathématiques : le maître de 1re A et elle en 7e A.
    assert_eq!(disciplines["Mathématiques"], 2);
    let heures = tableau(personnel, "heures_par_discipline");
    assert_eq!(heures["total"]["cellules"][0]["nombre"], 180);
    assert_eq!(heures["lignes"].as_array().unwrap().len(), 2);
    // La liste nominative : une case vide quand la fiche ne porte rien, jamais une mention.
    let nominative = tableau(personnel, "liste_nominative");
    let oumou = nominative["lignes"]
        .as_array()
        .unwrap()
        .iter()
        .find(|l| l["libelle"] == "Coulibaly Oumou")
        .unwrap();
    assert_eq!(oumou["cellules"][4]["texte"], "oui");
    assert_eq!(oumou["cellules"][5]["texte"], "DUT");
    assert_eq!(oumou["cellules"][6]["texte"], "2024-10-01");
    assert_eq!(oumou["cellules"][1]["texte"], Value::Null);

    // Rien ne s'écrit : une lecture de plus ne change pas une ligne du journal d'audit.
    let journal = json!({ "utilisateur_id": null, "entite": null, "operation": null,
        "depuis": null, "jusqua": null, "page": 0 });
    let audit_avant = p.ok("audit_lister", Some(&admin), journal.clone()).await;
    lire(&p, &admin, "2027-01-10").await;
    assert_eq!(
        p.ok("audit_lister", Some(&admin), journal.clone()).await,
        audit_avant
    );

    // L'export : une feuille par tableau, écrit où l'écran le demande.
    let chemin = dossier.path().join("statistiques.xlsx");
    let ecrit = p
        .ok(
            "statistiques_exporter",
            Some(&admin),
            json!({ "chemin": chemin.to_string_lossy(), "titre": "Élèves",
                "feuilles": [
                    { "titre": "Par classe", "entetes": ["Classe", "Garçons", "Filles", "Total"],
                      "lignes": [["1re A", "2", "1", "3"]] },
                    { "titre": "Par âge", "entetes": ["Âge", "Garçons", "Filles", "Total"],
                      "lignes": [["7", "1", "1", "2"], ["13", "0", "2", "3"]] } ] }),
        )
        .await;
    assert_eq!(ecrit["lignes"], 3);
    assert!(chemin.exists());
}

#[tokio::test]
async fn une_ecole_vide_a_ses_tableaux_vides_et_le_droit_est_tenu() {
    let dossier = tempfile::tempdir().unwrap();
    let p = poste_vierge(dossier.path()).await;
    p.installer().await;
    let admin = p.connecter("admin", MOT_DE_PASSE).await;

    // Ni année, ni élève, ni personnel : des tableaux vides, aucun refus.
    let vide = p.ok("statistiques_lire", Some(&admin), json!({})).await;
    assert_eq!(vide["annee_id"], Value::Null);
    assert!(vide["annees"].as_array().unwrap().is_empty());
    for onglet in ["eleves", "personnel"] {
        for t in vide[onglet]["tableaux"].as_array().unwrap() {
            assert!(t["lignes"].as_array().unwrap().is_empty(), "{}", t["code"]);
            assert!(t["hors"].as_array().unwrap().is_empty(), "{}", t["code"]);
            if !t["total"].is_null() {
                assert_eq!(t["total"]["cellules"][2]["nombre"], 0, "{}", t["code"]);
            }
        }
    }

    // Une année sans élève : ses classes paraissent, à zéro.
    let annee = p
        .ok(
            "annee_creer",
            Some(&admin),
            json!({ "libelle": "2026-2027", "debut": RENTREE_2026, "fin": FIN_2027 }),
        )
        .await;
    p.ok("annee_activer", Some(&admin), json!({ "id": annee["id"] }))
        .await;
    let ped = p
        .ok("pedagogie_lire", Some(&admin), json!({ "id": annee["id"] }))
        .await;
    p.ok(
        "classe_creer",
        Some(&admin),
        json!({ "annee_id": annee["id"], "niveau_id": id_par(&ped["niveaux"], "code", "1"),
            "serie_id": null, "libelle": "1re A" }),
    )
    .await;
    let stats = p.ok("statistiques_lire", Some(&admin), json!({})).await;
    assert_eq!(stats["annee_id"], annee["id"]);
    assert_eq!(stats["rentree"], "2026-10-01");
    let classes = tableau(&stats["eleves"], "eleves_par_classe");
    assert_eq!(classes["lignes"].as_array().unwrap().len(), 1);
    assert_eq!(case(&classes["lignes"][0], 2), 0);

    // La caisse n'a pas le droit ; la direction et le secrétariat l'ont.
    for (identifiant, role) in [("caissier", "caissier"), ("secretaire", "secretaire")] {
        p.ok(
            "utilisateur_creer",
            Some(&admin),
            json!({ "identifiant": identifiant, "nom_affiche": identifiant,
                "mot_de_passe_initial": "mot-de-passe-2026", "roles": [role] }),
        )
        .await;
    }
    let caissier = p.connecter("caissier", "mot-de-passe-2026").await;
    assert_eq!(
        p.refus("statistiques_lire", Some(&caissier), json!({}))
            .await
            .cle,
        "erreur.permission_refusee"
    );
    let secretaire = p.connecter("secretaire", "mot-de-passe-2026").await;
    p.ok("statistiques_lire", Some(&secretaire), json!({}))
        .await;
}
