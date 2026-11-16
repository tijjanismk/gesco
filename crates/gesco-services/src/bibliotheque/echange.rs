//! Échanger des leçons avec une autre école (SPEC_PHASE_12_LOT_2 §6 ; ADR « échange de leçons
//! entre écoles ») : exporter un chapitre, une matière d'un niveau ou une sélection dans un
//! fichier `.gles`, importer un tel fichier en brouillons.
//!
//! Ne part que la version visible de chaque contenu (jamais un brouillon), avec ses images, ses
//! sons et les fichiers d'un document ; une leçon livrée part seulement si l'école en a publié sa
//! version. À l'arrivée, rien n'est visible des élèves : une version d'une leçon que l'école a
//! devient un brouillon de cette leçon, ou une « version reçue » si un brouillon existe déjà ; une
//! leçon nouvelle devient une leçon de l'école en brouillon, sous un chapitre du même titre
//! (créé au besoin, à la fin de sa partie) ; une discipline sans matière se range sous un chapitre
//! livré de cette discipline, c'est-à-dire sous son libellé de référence. Un texte déjà dans
//! l'historique du contenu est ignoré : le même fichier importé deux fois n'ajoute rien.

use std::collections::{BTreeMap, HashMap, HashSet};
use std::path::Path;

use gesco_core::bibliotheque::paquet::empreinte;
use gesco_core::bibliotheque::{
    ChapitreBibliotheque, ChapitreEchange, ContenuBibliotheque, EchangeLu, ErreurEchange,
    EtatVersion, ExamenSujet, FichierRecu, JointEchange, LeconEchange, ManifesteEchange,
    NatureContenu, OrigineBibliotheque, SourceImage, TAILLE_ECHANGE_MAX, analyser, cle_recue,
    fabriquer_echange, lire_echange, nom_fichier_echange, nom_texte_echange, places_du_catalogue,
    preparer_fichier_recu, ranger, remplacer_empreintes,
};
use gesco_core::core::{Operation, TypeEvenement};
use gesco_db::Connexion;
use gesco_db::contexte::Contexte;
use gesco_db::depots::bibliotheque::{self as depot, Publication};
use gesco_db::depots::{academique, pedagogie};
use gesco_db::unite::Durabilite;
use serde::{Deserialize, Serialize};

use super::{ServiceBibliotheque, regle, reindexer_contenu};
use crate::erreur::Resultat;
use crate::session::Session;

/// Ce que l'on exporte.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum SelectionExport {
    /// Un chapitre du catalogue.
    Chapitre {
        /// Clé du chapitre.
        chapitre_cle: String,
    },
    /// Une matière d'un niveau du catalogue.
    Matiere {
        /// Code du niveau.
        niveau_code: String,
        /// Clé de la matière au catalogue (`m:<identifiant>` ou `d:<code>`).
        matiere_cle: String,
    },
    /// Des contenus choisis.
    Contenus {
        /// Identifiants des contenus.
        contenu_ids: Vec<String>,
    },
}

/// Voir ce qui partira.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
pub struct DemandeExportApercu {
    /// Ce que l'on exporte.
    pub selection: SelectionExport,
}

/// Une leçon qui partira.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
pub struct LeconExportee {
    /// Identifiant du contenu.
    pub contenu_id: String,
    /// Titre.
    pub titre: String,
    /// Nature.
    pub nature: NatureContenu,
    /// Titre du chapitre.
    pub chapitre: String,
    /// Version de l'école d'une leçon livrée.
    pub version_d_une_livree: bool,
}

/// Ce qui partira, et le nom proposé pour le fichier.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
pub struct ApercuExport {
    /// Leçons, dans l'ordre du catalogue.
    pub lecons: Vec<LeconExportee>,
    /// Nom proposé : « Leçons — Classe — Matière.gles ».
    pub nom_propose: String,
}

/// Exporter dans un fichier.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
pub struct DemandeExportLecons {
    /// Ce que l'on exporte.
    pub selection: SelectionExport,
    /// Chemin du fichier `.gles` à écrire.
    pub chemin: String,
}

/// Un export fait.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
pub struct ExportFait {
    /// Leçons parties.
    pub lecons: u32,
    /// Images, sons et fichiers de documents partis avec elles.
    pub fichiers: u32,
}

/// Lire ou importer un fichier de leçons.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
pub struct DemandeImportLecons {
    /// Chemin du fichier `.gles` (sur un poste relié, son contenu voyage à sa place).
    pub chemin: String,
}

/// Où une leçon reçue se rangera.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "snake_case")]
pub enum ArriveeLecon {
    /// Brouillon d'une leçon que l'école a déjà.
    Brouillon,
    /// L'école a déjà un brouillon : version reçue, dans l'historique.
    VersionRecue,
    /// Leçon nouvelle de l'école, en brouillon.
    Nouvelle,
    /// Ce texte est déjà dans l'historique : rien n'est ajouté.
    DejaLa,
    /// Aucune place : ni ce niveau ni cette matière dans l'école.
    SansPlace,
}

/// Une leçon du fichier et où elle se rangera.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
pub struct LeconArrivee {
    /// Titre.
    pub titre: String,
    /// Nature.
    pub nature: NatureContenu,
    /// Niveau de l'école (libellé), sinon le code reçu.
    pub niveau: String,
    /// Matière où elle se range (libellé de l'école, sinon de référence).
    pub matiere: String,
    /// Titre du chapitre.
    pub chapitre: String,
    /// Le chapitre sera créé.
    pub chapitre_nouveau: bool,
    /// Ce qui arrivera.
    pub arrivee: ArriveeLecon,
}

/// Ce qu'un fichier apporte, avant de confirmer.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
pub struct ApercuImport {
    /// Nom du fichier.
    pub nom_fichier: String,
    /// Date de fabrication écrite dans le fichier.
    pub date: String,
    /// Leçons, dans l'ordre du fichier.
    pub lecons: Vec<LeconArrivee>,
    /// Ce fichier a déjà été importé : quand, millisecondes depuis l'époque.
    pub deja_importe_le: Option<i64>,
}

/// Un import tel que les réglages le montrent.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
pub struct ImportAffiche {
    /// Identifiant.
    pub id: String,
    /// Nom du fichier.
    pub nom_fichier: String,
    /// Importé le, millisecondes depuis l'époque.
    pub importe_le: i64,
    /// Nom de qui l'a importé.
    pub importe_par: String,
    /// Leçons reçues.
    pub lecons: u32,
    /// Leçons déjà là, ignorées.
    pub ignorees: u32,
    /// Leçons reçues encore à publier.
    pub a_publier: u32,
}

/// Résumé d'un import.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
pub struct ImportFait {
    /// Identifiant de l'import.
    pub import_id: String,
    /// Leçons reçues en brouillon.
    pub brouillons: u32,
    /// Versions reçues ajoutées à l'historique.
    pub versions_recues: u32,
    /// Leçons déjà là, ignorées.
    pub ignorees: u32,
    /// Leçons sans place dans l'école.
    pub sans_place: u32,
    /// Chapitres créés.
    pub chapitres: u32,
    /// Imports de l'école, les plus récents d'abord.
    pub imports: Vec<ImportAffiche>,
}

/// Publier les leçons reçues d'un import.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
pub struct DemandeImportPublier {
    /// Identifiant de l'import.
    pub import_id: String,
}

/// Leçons publiées d'un geste.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
pub struct PublicationImport {
    /// Versions publiées.
    pub publiees: u32,
    /// Imports de l'école, les plus récents d'abord.
    pub imports: Vec<ImportAffiche>,
}

/// Le chapitre où se range une leçon nouvelle.
#[derive(Debug, Clone, PartialEq, Eq)]
enum PlaceChapitre {
    /// Un chapitre de l'école ou livré qui existe.
    Existant { id: String },
    /// Un chapitre de l'école à créer (une seule fois par import, par sa clé).
    ACreer {
        cle: String,
        niveau_id: String,
        niveau_code: String,
        matiere_id: String,
        partie: String,
        titre: String,
        apres_cle: Option<String>,
        series: Vec<String>,
    },
}

/// Ce que l'import fera d'une leçon.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Cible {
    /// Un contenu de l'école, livré ou non ; vrai s'il a déjà un brouillon ou une version
    /// soumise.
    Existant {
        contenu: Box<ContenuBibliotheque>,
        occupe: bool,
    },
    /// Un contenu de l'école à créer sous cette clé.
    Nouveau {
        cle: String,
        place: Box<PlaceChapitre>,
    },
    /// Texte déjà dans l'historique.
    DejaLa,
    /// Aucune place.
    SansPlace,
}

/// Une leçon du fichier, préparée : texte final, cible et ce que l'aperçu en dit.
struct LeconPlanifiee {
    rang: usize,
    texte: String,
    cible: Cible,
    niveau: String,
    matiere: String,
    chapitre: String,
}

/// Les fichiers du fichier d'échange, reconnus et nettoyés, par empreinte d'origine ; et les
/// empreintes changées par le nettoyage d'un SVG.
struct FichiersPrepares {
    prets: HashMap<String, FichierRecu>,
    remplacements: BTreeMap<String, String>,
}

fn preparer_fichiers(lu: &EchangeLu) -> FichiersPrepares {
    let mut prets = HashMap::new();
    let mut remplacements = BTreeMap::new();
    for (nom, octets) in &lu.fichiers {
        let Some(origine) = nom.strip_prefix("fichiers/") else {
            continue;
        };
        if let Some(recu) = preparer_fichier_recu(octets) {
            remplacements.insert(origine.to_owned(), empreinte(&recu.octets));
            prets.insert(origine.to_owned(), recu);
        }
    }
    FichiersPrepares {
        prets,
        remplacements,
    }
}

/// Garde d'un nom de fichier proposé ce que le système accepte.
fn nom_de_fichier(texte: &str) -> String {
    texte
        .chars()
        .map(|c| match c {
            '\\' | '/' | ':' | '*' | '?' | '"' | '<' | '>' | '|' => '-',
            c if c.is_control() => ' ',
            c => c,
        })
        .collect::<String>()
        .trim()
        .to_owned()
}

fn nombre(n: usize) -> u32 {
    u32::try_from(n).unwrap_or(u32::MAX)
}

fn import_affiche(l: depot::LigneImport) -> ImportAffiche {
    let n = |x: i64| u32::try_from(x).unwrap_or(0);
    ImportAffiche {
        id: l.id,
        nom_fichier: l.nom_fichier,
        importe_le: l.importe_le,
        importe_par: l.importe_par,
        lecons: n(l.nombre_lecons),
        ignorees: n(l.nombre_ignorees),
        a_publier: n(l.a_publier),
    }
}

/// Un contenu qui partira, avec ce que le fichier dira de lui.
struct ContenuExporte {
    contenu: ContenuBibliotheque,
    texte: String,
    chapitre: ChapitreEchange,
    livre: bool,
}

impl ServiceBibliotheque<'_> {
    /// Les contenus de la sélection qui partiront, dans l'ordre du catalogue : leur version
    /// visible de l'école, dans la portée de l'utilisateur. Rend aussi le nom proposé.
    async fn a_exporter(
        &self,
        session: &Session,
        selection: &SelectionExport,
    ) -> Resultat<(Vec<ContenuExporte>, String)> {
        let portee = self.portee(session).await?;
        let range = self.ranger_catalogue(session, true, true).await?;
        let contexte = self.noyau.contexte(session.etablissement_id, Some(session));
        let mut cnx = self.connexion().await?;
        let places = places_du_catalogue(&range.niveaux, &range.contenus);
        let mut vus = HashSet::new();
        let choisies: Vec<_> = places
            .into_iter()
            .filter(|p| match selection {
                SelectionExport::Chapitre { chapitre_cle } => &p.chapitre_cle == chapitre_cle,
                SelectionExport::Matiere {
                    niveau_code,
                    matiere_cle,
                } => &p.niveau_code == niveau_code && &p.matiere_cle == matiere_cle,
                SelectionExport::Contenus { contenu_ids } => contenu_ids.contains(&p.contenu_id),
            })
            .filter(|p| vus.insert(p.contenu_id.clone()))
            .collect();

        let disciplines: HashMap<String, String> = depot::paquets(&mut cnx, &contexte)
            .await?
            .into_iter()
            .filter(|p| p.actif)
            .flat_map(|p| p.disciplines)
            .map(|d| (d.code, d.libelle))
            .collect();
        let matieres = pedagogie::matieres(&mut cnx, &contexte).await?;
        let correspondances = depot::correspondances(&mut cnx, &contexte).await?;
        let chapitres: HashMap<String, &ChapitreBibliotheque> = range
            .chapitres
            .iter()
            .map(|c| (c.id.vers_chaine(), c))
            .collect();
        let contenus: HashMap<String, &ContenuBibliotheque> = range
            .contenus
            .iter()
            .map(|c| (c.id.vers_chaine(), c))
            .collect();

        let mut sortie = Vec::new();
        for place in &choisies {
            let Some(contenu) = contenus.get(&place.contenu_id) else {
                continue;
            };
            let Some(chapitre) = chapitres.get(&contenu.chapitre_id.vers_chaine()) else {
                continue;
            };
            if !portee.permet_chapitre(chapitre) {
                continue;
            }
            // Seule la version publiée de l'école part ; jamais un brouillon, jamais le livré.
            let Some((texte, false)) =
                depot::version_dans_etat(&mut cnx, &place.contenu_id, "publiee", &contexte).await?
            else {
                continue;
            };
            let (discipline, libelle) = match chapitre.origine {
                OrigineBibliotheque::Livre => {
                    let code = chapitre.discipline_code.clone().unwrap_or_default();
                    let libelle = disciplines.get(&code).cloned().unwrap_or(code.clone());
                    (code, libelle)
                }
                OrigineBibliotheque::Ecole => {
                    let matiere_id = chapitre.matiere_id.clone().unwrap_or_default();
                    let matiere = matieres.iter().find(|m| m.id.vers_chaine() == matiere_id);
                    // La discipline de référence que l'école range sous cette matière, sinon
                    // le code de la matière.
                    let code = correspondances
                        .iter()
                        .find(|(_, n, m)| {
                            n == &chapitre.niveau_code && m.as_deref() == Some(matiere_id.as_str())
                        })
                        .map(|(d, _, _)| d.clone())
                        .or_else(|| matiere.map(|m| m.code.clone()))
                        .unwrap_or_default();
                    let libelle = disciplines
                        .get(&code)
                        .cloned()
                        .or_else(|| matiere.map(|m| m.libelle.clone()))
                        .unwrap_or(code.clone());
                    (code, libelle)
                }
            };
            sortie.push(ContenuExporte {
                livre: contenu.origine == OrigineBibliotheque::Livre,
                texte,
                chapitre: ChapitreEchange {
                    cle: chapitre.cle.clone(),
                    livre: chapitre.origine == OrigineBibliotheque::Livre,
                    niveau: chapitre.niveau_code.clone(),
                    discipline,
                    discipline_libelle: libelle,
                    series: chapitre.series.clone(),
                    partie: chapitre.partie.clone(),
                    titre: chapitre.titre.clone(),
                },
                contenu: (*contenu).clone(),
            });
        }

        let niveaux: HashSet<&str> = choisies.iter().map(|p| p.niveau.as_str()).collect();
        let matieres_choisies: HashSet<&str> =
            choisies.iter().map(|p| p.matiere.as_str()).collect();
        let nom = match (
            niveaux.iter().next(),
            matieres_choisies.iter().next(),
            niveaux.len(),
            matieres_choisies.len(),
        ) {
            (Some(n), Some(m), 1, 1) => format!("Leçons — {n} — {m}"),
            (Some(n), _, 1, _) => format!("Leçons — {n}"),
            _ => "Leçons choisies".to_owned(),
        };
        Ok((sortie, format!("{}.gles", nom_de_fichier(&nom))))
    }

    /// Ce qui partira, avant d'enregistrer.
    pub async fn export_apercu(
        &self,
        session: &Session,
        demande: &DemandeExportApercu,
    ) -> Resultat<ApercuExport> {
        let (contenus, nom_propose) = self.a_exporter(session, &demande.selection).await?;
        Ok(ApercuExport {
            lecons: contenus
                .into_iter()
                .map(|c| LeconExportee {
                    contenu_id: c.contenu.id.vers_chaine(),
                    titre: c.contenu.titre,
                    nature: c.contenu.nature,
                    chapitre: c.chapitre.titre,
                    version_d_une_livree: c.livre,
                })
                .collect(),
            nom_propose,
        })
    }

    /// Exporte la sélection dans un fichier `.gles` ; le geste va au journal.
    pub async fn exporter_lecons(
        &self,
        session: &Session,
        demande: &DemandeExportLecons,
    ) -> Resultat<ExportFait> {
        let (contenus, _) = self.a_exporter(session, &demande.selection).await?;
        if contenus.is_empty() {
            return Err(regle("bibliotheque.export_vide"));
        }
        let contexte = self.noyau.contexte(session.etablissement_id, Some(session));
        let mut cnx = self.connexion().await?;
        let mut fichiers: Vec<(String, Vec<u8>)> = Vec::new();
        let mut deja: HashSet<String> = HashSet::new();
        let mut noms = BTreeMap::new();
        let mut lecons = Vec::new();
        for (rang, c) in contenus.iter().enumerate() {
            let lecon = analyser(&c.texte);
            let mut appels: Vec<String> = lecon
                .images()
                .into_iter()
                .filter_map(|s| match s {
                    SourceImage::Ecole { empreinte } => Some(empreinte.clone()),
                    SourceImage::Paquet { .. } => None,
                })
                .collect();
            appels.extend(lecon.sons().into_iter().map(str::to_owned));
            let mut joints = Vec::new();
            if c.contenu.nature == NatureContenu::Document {
                for f in
                    depot::fichiers_du_contenu(&mut cnx, &c.contenu.id.vers_chaine(), &contexte)
                        .await?
                {
                    joints.push(JointEchange {
                        empreinte: f.empreinte_sha256.clone(),
                        nom: f.nom_fichier,
                    });
                }
            }
            let mut absents = HashSet::new();
            for e in appels
                .iter()
                .chain(joints.iter().map(|j| &j.empreinte))
                .cloned()
                .collect::<Vec<_>>()
            {
                if deja.contains(&e) || absents.contains(&e) {
                    continue;
                }
                match depot::fichier_par_empreinte(&mut cnx, &e, &contexte).await? {
                    Some((nom, _, octets)) => {
                        noms.insert(e.clone(), nom);
                        fichiers.push((nom_fichier_echange(&e), octets));
                        deja.insert(e);
                    }
                    // Une image ou un son absent : la leçon part sans lui.
                    None => {
                        absents.insert(e);
                    }
                }
            }
            joints.retain(|j| deja.contains(&j.empreinte));
            fichiers.push((nom_texte_echange(rang), c.texte.clone().into_bytes()));
            lecons.push(LeconEchange {
                cle: c.contenu.cle.clone(),
                livre: c.livre,
                nature: c.contenu.nature,
                titre: c.contenu.titre.clone(),
                presentation: c.contenu.presentation.clone(),
                examen: c.contenu.examen,
                serie: c.contenu.serie_code.clone(),
                session: c.contenu.session,
                lie: c.contenu.lie_cle.clone(),
                chapitre: c.chapitre.clone(),
                texte: nom_texte_echange(rang),
                joints,
            });
        }
        drop(cnx);
        let nombre_fichiers = nombre(deja.len());
        let civil = contexte.maintenant.civil_utc();
        let manifeste = ManifesteEchange {
            date: format!("{:04}-{:02}-{:02}", civil.annee, civil.mois, civil.jour),
            gesco: self.version_gesco.to_owned(),
            lecons,
            noms,
            fichiers: Vec::new(),
        };
        let octets = fabriquer_echange(&manifeste, &fichiers)
            .map_err(|_| regle("bibliotheque.export_impossible"))?;
        crate::espace::ecrire_atomiquement(Path::new(&demande.chemin), &octets)?;

        let mut unite = self
            .noyau
            .commencer(contexte.clone(), Durabilite::Critique)
            .await?;
        unite.auditer(
            "etablissement",
            &session.etablissement_id.vers_chaine(),
            Operation::Acces,
            None,
            Some(serde_json::json!({
                "selection": demande.selection,
                "lecons": manifeste.lecons.len(),
                "cles": manifeste.lecons.iter().map(|l| l.cle.clone()).collect::<Vec<_>>(),
                "fichiers": nombre_fichiers,
                "empreinte": empreinte(&octets),
            })),
            "bibliotheque.exporter",
        );
        self.noyau.terminer(unite).await?;
        Ok(ExportFait {
            lecons: nombre(manifeste.lecons.len()),
            fichiers: nombre_fichiers,
        })
    }

    /// Lit et vérifie un fichier de leçons.
    fn lire_fichier_lecons(chemin: &Path) -> Resultat<(EchangeLu, String, String)> {
        if std::fs::metadata(chemin)?.len() > TAILLE_ECHANGE_MAX {
            return Err(regle("bibliotheque.echange_abime"));
        }
        let octets = std::fs::read(chemin)?;
        let lu = lire_echange(&octets).map_err(|e| match e {
            ErreurEchange::FormatInconnu => regle("bibliotheque.echange_trop_recent"),
            ErreurEchange::Abime | ErreurEchange::Fabrication(_) => {
                regle("bibliotheque.echange_abime")
            }
        })?;
        let nom = chemin
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default();
        Ok((lu, empreinte(&octets), nom))
    }

    /// Range chaque leçon du fichier : où elle ira, et si elle est déjà là.
    async fn planifier(
        cnx: &mut Connexion,
        lu: &EchangeLu,
        prepares: &FichiersPrepares,
        contexte: &Contexte,
    ) -> Resultat<Vec<LeconPlanifiee>> {
        let niveaux: Vec<_> = academique::niveaux(cnx, contexte)
            .await?
            .into_iter()
            .filter(|n| n.actif)
            .collect();
        let series = academique::series(cnx, contexte).await?;
        let matieres: Vec<_> = pedagogie::matieres(cnx, contexte)
            .await?
            .into_iter()
            .filter(|m| m.active)
            .collect();
        let correspondances = depot::correspondances(cnx, contexte).await?;
        let disciplines: HashMap<String, String> = depot::paquets(cnx, contexte)
            .await?
            .into_iter()
            .filter(|p| p.actif)
            .flat_map(|p| p.disciplines)
            .map(|d| (d.code, d.libelle))
            .collect();
        let chapitres = depot::chapitres(cnx, contexte).await?;
        let contenus = depot::contenus(cnx, contexte).await?;
        let par_cle: HashMap<&str, &ContenuBibliotheque> =
            contenus.iter().map(|c| (c.cle.as_str(), c)).collect();
        let correspondance = |discipline: &str, niveau: &str| {
            correspondances
                .iter()
                .find(|(d, n, _)| d.eq_ignore_ascii_case(discipline) && n == niveau)
                .map(|(_, _, m)| m.clone())
        };
        // Matière d'un chapitre de l'école (sinon celle que la correspondance donne à un
        // chapitre livré).
        let matiere_du_chapitre = |c: &ChapitreBibliotheque| -> Option<String> {
            match c.origine {
                OrigineBibliotheque::Ecole => c.matiere_id.clone(),
                OrigineBibliotheque::Livre => correspondance(
                    c.discipline_code.as_deref().unwrap_or_default(),
                    &c.niveau_code,
                )
                .flatten(),
            }
        };
        // Chapitres à créer pendant cet import, par (niveau, matière, titre) : leur clé.
        let mut a_creer: HashMap<(String, String, String), String> = HashMap::new();

        let mut plan = Vec::new();
        for (rang, l) in lu.manifeste.lecons.iter().enumerate() {
            let texte = remplacer_empreintes(lu.texte(l), &prepares.remplacements);
            let empreinte_texte = empreinte(texte.as_bytes());
            let niveau = niveaux.iter().find(|n| n.code == l.chapitre.niveau);
            let niveau_libelle = niveau.map_or(l.chapitre.niveau.clone(), |n| n.libelle.clone());
            let libelle_reference = disciplines
                .get(&l.chapitre.discipline)
                .cloned()
                .unwrap_or_else(|| l.chapitre.discipline_libelle.clone());

            // Un contenu que l'école a déjà : par sa clé, sinon par la clé sous laquelle un
            // import précédent l'a rangé.
            let existant = par_cle
                .get(l.cle.as_str())
                .or_else(|| par_cle.get(cle_recue(&l.cle).as_str()))
                .copied();
            if let Some(c) = existant {
                let id = c.id.vers_chaine();
                let versions = depot::versions(cnx, &id, contexte).await?;
                let recues = depot::versions_recues(cnx, &id, contexte).await?;
                let connue = c.empreinte.as_deref() == Some(empreinte_texte.as_str())
                    || versions
                        .iter()
                        .any(|v| v.empreinte_sha256 == empreinte_texte)
                    || recues.iter().any(|r| r.empreinte_sha256 == empreinte_texte);
                let occupe = versions
                    .iter()
                    .any(|v| v.etat == "brouillon" || v.etat == "soumise");
                let chapitre = chapitres.iter().find(|x| x.id == c.chapitre_id);
                let matiere = chapitre
                    .and_then(matiere_du_chapitre)
                    .and_then(|m| matieres.iter().find(|x| x.id.vers_chaine() == m))
                    .map_or(libelle_reference.clone(), |m| m.libelle.clone());
                plan.push(LeconPlanifiee {
                    rang,
                    texte,
                    cible: if connue {
                        Cible::DejaLa
                    } else {
                        Cible::Existant {
                            contenu: Box::new(c.clone()),
                            occupe,
                        }
                    },
                    niveau: niveau_libelle,
                    matiere,
                    chapitre: chapitre.map_or(l.chapitre.titre.clone(), |x| x.titre.clone()),
                });
                continue;
            }

            // Une leçon nouvelle : son chapitre.
            let cle = cle_recue(&l.cle);
            let livre_connu = chapitres
                .iter()
                .find(|c| l.chapitre.livre && c.cle == l.chapitre.cle);
            let matiere_id: Option<String> = niveau.and_then(|n| {
                match correspondance(&l.chapitre.discipline, &n.code) {
                    // La direction a rangé cette discipline sous une matière, ou l'a laissée
                    // sous son libellé de référence.
                    Some(m) => m.filter(|m| matieres.iter().any(|x| x.id.vers_chaine() == *m)),
                    None => matieres
                        .iter()
                        .find(|m| m.code.eq_ignore_ascii_case(&l.chapitre.discipline))
                        .or_else(|| {
                            matieres.iter().find(|m| {
                                m.libelle.trim().to_lowercase()
                                    == l.chapitre.discipline_libelle.trim().to_lowercase()
                            })
                        })
                        .map(|m| m.id.vers_chaine()),
                }
            });
            let matiere_libelle = matiere_id
                .as_deref()
                .and_then(|m| matieres.iter().find(|x| x.id.vers_chaine() == m))
                .map_or(libelle_reference.clone(), |m| m.libelle.clone());
            let place = match (livre_connu, niveau, &matiere_id) {
                (Some(c), _, _) => Some((
                    PlaceChapitre::Existant {
                        id: c.id.vers_chaine(),
                    },
                    c.titre.clone(),
                )),
                (None, Some(n), Some(m)) => {
                    let niveau_id = n.id.vers_chaine();
                    let titre = l.chapitre.titre.trim().to_owned();
                    let existe = chapitres.iter().find(|c| {
                        c.origine == OrigineBibliotheque::Ecole
                            && c.niveau_id.as_deref() == Some(niveau_id.as_str())
                            && c.matiere_id.as_deref() == Some(m.as_str())
                            && c.titre.trim() == titre
                    });
                    let cle_creee = a_creer.get(&(niveau_id.clone(), m.clone(), titre.clone()));
                    match (existe, cle_creee) {
                        (Some(c), _) => Some((
                            PlaceChapitre::Existant {
                                id: c.id.vers_chaine(),
                            },
                            c.titre.clone(),
                        )),
                        (None, Some(k)) => Some((
                            PlaceChapitre::ACreer {
                                cle: k.clone(),
                                niveau_id,
                                niveau_code: n.code.clone(),
                                matiere_id: m.clone(),
                                partie: String::new(),
                                titre: titre.clone(),
                                apres_cle: None,
                                series: Vec::new(),
                            },
                            titre,
                        )),
                        (None, None) => {
                            // À la fin de sa partie dans la matière du niveau, sinon à la fin de
                            // la matière.
                            let dans_la_matiere: Vec<&ChapitreBibliotheque> = chapitres
                                .iter()
                                .filter(|c| {
                                    c.niveau_code == n.code
                                        && matiere_du_chapitre(c).as_deref() == Some(m.as_str())
                                })
                                .collect();
                            let (mut livres, mut ecole): (Vec<_>, Vec<_>) = dans_la_matiere
                                .into_iter()
                                .partition(|c| c.origine == OrigineBibliotheque::Livre);
                            livres.sort_by_key(|c| c.ordre);
                            ecole.sort_by_key(|c| c.ordre);
                            let ranges = ranger(
                                livres,
                                ecole,
                                |c| c.cle.as_str(),
                                |c| c.apres_cle.as_deref(),
                            );
                            let partie = l.chapitre.partie.trim().to_owned();
                            let apres_cle = ranges
                                .iter()
                                .rev()
                                .find(|c| c.partie == partie)
                                .or_else(|| ranges.last())
                                .map(|c| c.cle.clone());
                            let series_du_niveau: Vec<&str> = series
                                .iter()
                                .filter(|s| s.actif && s.niveau_id == n.id)
                                .map(|s| s.code.as_str())
                                .collect();
                            let mut series_gardees: Vec<String> = l
                                .chapitre
                                .series
                                .iter()
                                .filter(|s| series_du_niveau.contains(&s.as_str()))
                                .cloned()
                                .collect();
                            series_gardees.sort();
                            series_gardees.dedup();
                            // La clé du chapitre de l'autre école, si elle est libre ici : un
                            // second import le retrouve aussi par elle.
                            let cle_chapitre = if l.chapitre.cle.starts_with("ecole/")
                                && !chapitres.iter().any(|c| c.cle == l.chapitre.cle)
                                && !a_creer.values().any(|k| k == &l.chapitre.cle)
                            {
                                l.chapitre.cle.clone()
                            } else {
                                format!(
                                    "ecole/{}",
                                    gesco_db::nouvel_id::<ChapitreBibliotheque>().vers_chaine()
                                )
                            };
                            a_creer.insert(
                                (niveau_id.clone(), m.clone(), titre.clone()),
                                cle_chapitre.clone(),
                            );
                            Some((
                                PlaceChapitre::ACreer {
                                    cle: cle_chapitre,
                                    niveau_id,
                                    niveau_code: n.code.clone(),
                                    matiere_id: m.clone(),
                                    partie,
                                    titre: titre.clone(),
                                    apres_cle,
                                    series: series_gardees,
                                },
                                titre,
                            ))
                        }
                    }
                }
                // Sans matière : un chapitre livré de cette discipline et de ce niveau, rangé
                // sous son libellé de référence ; celui du même titre d'abord.
                (None, _, None) => {
                    let livres: Vec<&ChapitreBibliotheque> = chapitres
                        .iter()
                        .filter(|c| {
                            c.origine == OrigineBibliotheque::Livre
                                && !c.plus_livre
                                && c.niveau_code == l.chapitre.niveau
                                && c.discipline_code
                                    .as_deref()
                                    .is_some_and(|d| d.eq_ignore_ascii_case(&l.chapitre.discipline))
                        })
                        .collect();
                    livres
                        .iter()
                        .find(|c| c.titre.trim() == l.chapitre.titre.trim())
                        .or_else(|| livres.iter().max_by_key(|c| c.ordre))
                        .map(|c| {
                            (
                                PlaceChapitre::Existant {
                                    id: c.id.vers_chaine(),
                                },
                                c.titre.clone(),
                            )
                        })
                }
                (None, None, Some(_)) => None,
            };
            let (cible, chapitre) = match place {
                Some((place, titre)) => (
                    Cible::Nouveau {
                        cle,
                        place: Box::new(place),
                    },
                    titre,
                ),
                None => (Cible::SansPlace, l.chapitre.titre.clone()),
            };
            plan.push(LeconPlanifiee {
                rang,
                texte,
                cible,
                niveau: niveau_libelle,
                matiere: matiere_libelle,
                chapitre,
            });
        }
        Ok(plan)
    }

    /// Ce qu'un fichier apporte et où cela se rangera, avant de confirmer.
    pub async fn import_apercu(
        &self,
        session: &Session,
        demande: &DemandeImportLecons,
    ) -> Resultat<ApercuImport> {
        let (lu, empreinte_fichier, nom_fichier) =
            Self::lire_fichier_lecons(Path::new(&demande.chemin))?;
        let prepares = preparer_fichiers(&lu);
        let contexte = self.noyau.contexte(session.etablissement_id, Some(session));
        let mut cnx = self.connexion().await?;
        let plan = Self::planifier(&mut cnx, &lu, &prepares, &contexte).await?;
        let deja_importe_le =
            depot::import_par_empreinte(&mut cnx, &empreinte_fichier, &contexte).await?;
        let lecons = plan
            .into_iter()
            .filter_map(|p| {
                let l = lu.manifeste.lecons.get(p.rang)?;
                Some(LeconArrivee {
                    titre: l.titre.clone(),
                    nature: l.nature,
                    niveau: p.niveau,
                    matiere: p.matiere,
                    chapitre: p.chapitre,
                    chapitre_nouveau: matches!(
                        &p.cible,
                        Cible::Nouveau { place, .. } if matches!(**place, PlaceChapitre::ACreer { .. })
                    ),
                    arrivee: match p.cible {
                        Cible::Existant { occupe: false, .. } => ArriveeLecon::Brouillon,
                        Cible::Existant { occupe: true, .. } => ArriveeLecon::VersionRecue,
                        Cible::Nouveau { .. } => ArriveeLecon::Nouvelle,
                        Cible::DejaLa => ArriveeLecon::DejaLa,
                        Cible::SansPlace => ArriveeLecon::SansPlace,
                    },
                })
            })
            .collect();
        Ok(ApercuImport {
            nom_fichier,
            date: lu.manifeste.date.clone(),
            lecons,
            deja_importe_le,
        })
    }

    /// Range dans l'unité les images, sons et fichiers qu'un texte reçu appelle ; rend les
    /// identifiants des fichiers joints d'un document, dans l'ordre.
    async fn ranger_fichiers_recus(
        unite: &mut gesco_db::UniteDeTravail<crate::effets::Effet>,
        contexte: &Contexte,
        lu: &EchangeLu,
        prepares: &FichiersPrepares,
        texte: &str,
        joints: &[JointEchange],
    ) -> Resultat<Vec<String>> {
        let lecon = analyser(texte);
        let mut appels: Vec<String> = lecon
            .images()
            .into_iter()
            .filter_map(|s| match s {
                SourceImage::Ecole { empreinte } => Some(empreinte.clone()),
                SourceImage::Paquet { .. } => None,
            })
            .collect();
        appels.extend(lecon.sons().into_iter().map(str::to_owned));
        // Les appels du texte portent déjà l'empreinte nettoyée : retrouver l'origine.
        let origines: HashMap<&str, &str> = prepares
            .remplacements
            .iter()
            .map(|(avant, apres)| (apres.as_str(), avant.as_str()))
            .collect();
        for e in &appels {
            let origine = origines.get(e.as_str()).copied().unwrap_or(e.as_str());
            if let Some(recu) = prepares.prets.get(origine) {
                let nom = lu
                    .manifeste
                    .noms
                    .get(origine)
                    .cloned()
                    .unwrap_or_else(|| format!("fichier.{}", recu.sorte.extension()));
                Self::enregistrer_dans(unite, contexte, recu.sorte, &nom, &recu.octets).await?;
            }
        }
        let mut ids = Vec::new();
        for j in joints {
            if let Some(recu) = prepares.prets.get(&j.empreinte)
                && (recu.sorte.est_image()
                    || recu.sorte == gesco_core::bibliotheque::SorteFichier::Pdf)
            {
                let (id, _) =
                    Self::enregistrer_dans(unite, contexte, recu.sorte, &j.nom, &recu.octets)
                        .await?;
                ids.push(id);
            }
        }
        Ok(ids)
    }

    /// Joint à un document les fichiers reçus qu'il n'a pas encore.
    async fn joindre_recus(
        unite: &mut gesco_db::UniteDeTravail<crate::effets::Effet>,
        contexte: &Contexte,
        contenu_id: &str,
        fichier_ids: &[String],
    ) -> Resultat<()> {
        // Même retiré depuis, un fichier déjà joint n'est pas remis par un import.
        let deja: HashSet<String> =
            depot::fichiers_joints_ids(unite.connexion(), contenu_id, contexte)
                .await?
                .into_iter()
                .collect();
        for fichier_id in fichier_ids {
            if deja.contains(fichier_id) {
                continue;
            }
            let lien =
                depot::joindre_fichier(unite.connexion(), contenu_id, fichier_id, contexte).await?;
            unite.auditer(
                "fichier_contenu",
                &lien,
                Operation::Creation,
                None,
                Some(serde_json::json!({ "contenu_id": contenu_id, "fichier_id": fichier_id })),
                "bibliotheque.importer",
            );
        }
        Ok(())
    }

    /// Importe un fichier de leçons : chaque leçon arrive en brouillon, au nom de qui importe,
    /// ou en version reçue ; rien n'est visible des élèves avant publication.
    pub async fn importer_lecons(
        &self,
        session: &Session,
        demande: &DemandeImportLecons,
    ) -> Resultat<ImportFait> {
        let (lu, empreinte_fichier, nom_fichier) =
            Self::lire_fichier_lecons(Path::new(&demande.chemin))?;
        let prepares = preparer_fichiers(&lu);
        let contexte = self.noyau.contexte(session.etablissement_id, Some(session));
        let moi = session.utilisateur_id.vers_chaine();
        let mut unite = self
            .noyau
            .commencer(contexte.clone(), Durabilite::Critique)
            .await?;
        let plan = Self::planifier(unite.connexion(), &lu, &prepares, &contexte).await?;
        let import_id = depot::inserer_import(
            unite.connexion(),
            &empreinte_fichier,
            &nom_fichier,
            &moi,
            &contexte,
        )
        .await?;

        let (mut brouillons, mut versions_recues, mut ignorees, mut sans_place) = (0, 0, 0, 0);
        let mut chapitres_crees: HashMap<String, String> = HashMap::new();
        // Dernier contenu rangé par cet import dans chaque chapitre : le suivant se range après.
        let mut derniers: HashMap<String, String> = HashMap::new();
        for p in &plan {
            let Some(l) = lu.manifeste.lecons.get(p.rang) else {
                continue;
            };
            match &p.cible {
                Cible::DejaLa => ignorees += 1,
                Cible::SansPlace => sans_place += 1,
                Cible::Existant { contenu, occupe } => {
                    let contenu_id = contenu.id.vers_chaine();
                    let joints = Self::ranger_fichiers_recus(
                        &mut unite, &contexte, &lu, &prepares, &p.texte, &l.joints,
                    )
                    .await?;
                    if contenu.nature == NatureContenu::Document
                        && contenu.origine == OrigineBibliotheque::Ecole
                    {
                        Self::joindre_recus(&mut unite, &contexte, &contenu_id, &joints).await?;
                    }
                    let empreinte_texte = empreinte(p.texte.as_bytes());
                    if *occupe {
                        let id = depot::inserer_version_recue(
                            unite.connexion(),
                            &contenu_id,
                            &p.texte,
                            &empreinte_texte,
                            &import_id,
                            &contexte,
                        )
                        .await?;
                        unite.auditer(
                            "version_recue",
                            &id,
                            Operation::Creation,
                            None,
                            Some(serde_json::json!({
                                "contenu_id": contenu_id,
                                "import_id": import_id,
                                "texte": p.texte,
                            })),
                            "bibliotheque.importer",
                        );
                        versions_recues += 1;
                    } else {
                        let versions =
                            depot::versions(unite.connexion(), &contenu_id, &contexte).await?;
                        let (depart_version, depart_livre) = match versions
                            .iter()
                            .find(|v| v.etat == "publiee" && v.retour_livre == 0)
                        {
                            Some(v) => (Some(v.id.clone()), v.depart_livre_sha256.clone()),
                            None => (None, contenu.empreinte.clone()),
                        };
                        let version_id = self
                            .creer_brouillon(
                                &mut unite,
                                session,
                                &contenu_id,
                                &p.texte,
                                false,
                                depart_version.as_deref(),
                                depart_livre.as_deref(),
                                "bibliotheque.importer",
                            )
                            .await?;
                        depot::poser_import(unite.connexion(), &version_id, &import_id, &contexte)
                            .await?;
                        brouillons += 1;
                    }
                }
                Cible::Nouveau { cle, place } => {
                    let chapitre_id = match place.as_ref() {
                        PlaceChapitre::Existant { id } => id.clone(),
                        PlaceChapitre::ACreer {
                            cle: cle_chapitre,
                            niveau_id,
                            niveau_code,
                            matiere_id,
                            partie,
                            titre,
                            apres_cle,
                            series,
                        } => match chapitres_crees.get(cle_chapitre) {
                            Some(id) => id.clone(),
                            None => {
                                let id = Self::creer_chapitre_recu(
                                    &mut unite,
                                    &depot::NouveauChapitre {
                                        cle: cle_chapitre,
                                        niveau_id,
                                        niveau_code,
                                        matiere_id,
                                        partie,
                                        titre,
                                        apres_cle: apres_cle.as_deref(),
                                        series,
                                    },
                                    &contexte,
                                )
                                .await?;
                                chapitres_crees.insert(cle_chapitre.clone(), id.clone());
                                id
                            }
                        },
                    };
                    let apres_cle = match derniers.get(&chapitre_id) {
                        Some(k) => Some(k.clone()),
                        None => {
                            Self::dernier_du_chapitre(unite.connexion(), &chapitre_id, &contexte)
                                .await?
                        }
                    };
                    let contenu_id = depot::inserer_contenu_ecole(
                        unite.connexion(),
                        &depot::NouveauContenu {
                            cle,
                            nature: l.nature,
                            chapitre_id: &chapitre_id,
                            titre: l.titre.trim(),
                            presentation: l.presentation.trim(),
                            apres_cle: apres_cle.as_deref(),
                            lie_cle: l.lie.as_deref(),
                            examen: l.examen,
                            serie_code: l.serie.as_deref(),
                            session: l.session,
                        },
                        &contexte,
                    )
                    .await?;
                    derniers.insert(chapitre_id.clone(), cle.clone());
                    let valeur = serde_json::json!({
                        "cle": cle,
                        "nature": l.nature.nom(),
                        "chapitre_id": chapitre_id,
                        "titre": l.titre.trim(),
                        "presentation": l.presentation.trim(),
                        "examen": l.examen.map(ExamenSujet::sigle),
                        "serie_code": l.serie,
                        "session": l.session,
                        "import_id": import_id,
                    });
                    unite.auditer(
                        "contenu",
                        &contenu_id,
                        Operation::Creation,
                        None,
                        Some(valeur.clone()),
                        "bibliotheque.importer",
                    );
                    unite.emettre(
                        TypeEvenement::ContenuBibliothequeEnregistre,
                        "contenu",
                        &contenu_id,
                        valeur,
                    );
                    let joints = Self::ranger_fichiers_recus(
                        &mut unite, &contexte, &lu, &prepares, &p.texte, &l.joints,
                    )
                    .await?;
                    Self::joindre_recus(&mut unite, &contexte, &contenu_id, &joints).await?;
                    let version_id = self
                        .creer_brouillon(
                            &mut unite,
                            session,
                            &contenu_id,
                            &p.texte,
                            false,
                            None,
                            None,
                            "bibliotheque.importer",
                        )
                        .await?;
                    depot::poser_import(unite.connexion(), &version_id, &import_id, &contexte)
                        .await?;
                    brouillons += 1;
                }
            }
        }
        let n = |x: u32| i64::from(x);
        depot::compter_import(
            unite.connexion(),
            &import_id,
            n(brouillons + versions_recues),
            n(ignorees),
            &contexte,
        )
        .await?;
        unite.auditer(
            "import_lecons",
            &import_id,
            Operation::Creation,
            None,
            Some(serde_json::json!({
                "nom_fichier": nom_fichier,
                "empreinte": empreinte_fichier,
                "date": lu.manifeste.date,
                "lecons": lu.manifeste.lecons.len(),
                "brouillons": brouillons,
                "versions_recues": versions_recues,
                "ignorees": ignorees,
                "sans_place": sans_place,
                "chapitres": chapitres_crees.len(),
            })),
            "bibliotheque.importer",
        );
        self.noyau.terminer(unite).await?;
        Ok(ImportFait {
            import_id,
            brouillons,
            versions_recues,
            ignorees,
            sans_place,
            chapitres: nombre(chapitres_crees.len()),
            imports: self.imports(session).await?,
        })
    }

    /// Écrit un chapitre reçu, avec son audit et son événement.
    async fn creer_chapitre_recu(
        unite: &mut gesco_db::UniteDeTravail<crate::effets::Effet>,
        c: &depot::NouveauChapitre<'_>,
        contexte: &Contexte,
    ) -> Resultat<String> {
        let id = depot::inserer_chapitre_ecole(unite.connexion(), c, contexte).await?;
        let valeur = serde_json::json!({
            "cle": c.cle,
            "niveau_id": c.niveau_id,
            "matiere_id": c.matiere_id,
            "series": c.series,
            "partie": c.partie,
            "titre": c.titre,
            "apres_cle": c.apres_cle,
        });
        unite.auditer(
            "chapitre_programme",
            &id,
            Operation::Creation,
            None,
            Some(valeur.clone()),
            "bibliotheque.importer",
        );
        unite.emettre(
            TypeEvenement::ChapitreBibliothequeEnregistre,
            "chapitre_programme",
            &id,
            valeur,
        );
        Ok(id)
    }

    /// Clé du dernier contenu d'un chapitre, dans l'ordre où le catalogue le range.
    async fn dernier_du_chapitre(
        cnx: &mut Connexion,
        chapitre_id: &str,
        contexte: &Contexte,
    ) -> Resultat<Option<String>> {
        let contenus = depot::contenus(cnx, contexte).await?;
        let (mut livres, mut ecole): (Vec<_>, Vec<_>) = contenus
            .into_iter()
            .filter(|c| c.chapitre_id.vers_chaine() == chapitre_id)
            .partition(|c| c.origine == OrigineBibliotheque::Livre);
        livres.sort_by_key(|c| c.ordre);
        ecole.sort_by_key(|c| c.ordre);
        Ok(ranger(
            livres,
            ecole,
            |c| c.cle.as_str(),
            |c| c.apres_cle.as_deref(),
        )
        .last()
        .map(|c| c.cle.clone()))
    }

    /// Les imports de l'école, les plus récents d'abord.
    pub async fn imports(&self, session: &Session) -> Resultat<Vec<ImportAffiche>> {
        let contexte = self.noyau.contexte(session.etablissement_id, Some(session));
        let mut cnx = self.connexion().await?;
        Ok(depot::imports(&mut cnx, &contexte)
            .await?
            .into_iter()
            .map(import_affiche)
            .collect())
    }

    /// « Publier les leçons reçues de cet import » : publie chaque brouillon ou version soumise
    /// encore ouverte née de cet import ; les versions reçues restent dans l'historique.
    pub async fn publier_import(
        &self,
        session: &Session,
        demande: &DemandeImportPublier,
    ) -> Resultat<PublicationImport> {
        let contexte = self.noyau.contexte(session.etablissement_id, Some(session));
        let paquets = self.paquets.tous();
        let mut unite = self
            .noyau
            .commencer(contexte.clone(), Durabilite::Critique)
            .await?;
        let a_publier =
            depot::versions_de_l_import(unite.connexion(), &demande.import_id, &contexte).await?;
        let mut publiees = 0;
        for (contenu_id, version_id) in &a_publier {
            let Some(contenu) = depot::contenu(unite.connexion(), contenu_id, &contexte).await?
            else {
                continue;
            };
            let Publication::Faite { remplacee } =
                depot::publier_version(unite.connexion(), contenu_id, version_id, &contexte)
                    .await?
            else {
                continue;
            };
            if let Some(ancienne) = &remplacee {
                unite.auditer(
                    "version_contenu",
                    ancienne,
                    Operation::Modification,
                    Some(serde_json::json!({ "etat": EtatVersion::Publiee.nom() })),
                    Some(serde_json::json!({ "etat": EtatVersion::Remplacee.nom() })),
                    "bibliotheque.import_publier",
                );
            }
            unite.auditer(
                "version_contenu",
                version_id,
                Operation::Modification,
                None,
                Some(serde_json::json!({
                    "etat": EtatVersion::Publiee.nom(),
                    "remplace": remplacee,
                    "import_id": demande.import_id,
                })),
                "bibliotheque.import_publier",
            );
            unite.emettre(
                TypeEvenement::VersionContenuModifiee,
                "version_contenu",
                version_id,
                serde_json::json!({
                    "contenu_id": contenu_id,
                    "etat": EtatVersion::Publiee.nom(),
                }),
            );
            reindexer_contenu(unite.connexion(), &contexte, &contenu, &paquets).await?;
            publiees += 1;
        }
        self.noyau.terminer(unite).await?;
        Ok(PublicationImport {
            publiees,
            imports: self.imports(session).await?,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn un_nom_propose_ne_garde_que_ce_que_le_systeme_accepte() {
        assert_eq!(
            nom_de_fichier("Leçons — 9e A/B — Maths: algèbre?"),
            "Leçons — 9e A-B — Maths- algèbre-"
        );
    }
}
