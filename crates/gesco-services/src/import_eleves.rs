//! Import des élèves depuis un tableur (SPEC_PHASE_2 §10) : lecture du fichier en Rust,
//! correspondance des colonnes, aperçu ligne par ligne, application en une transaction après la
//! sauvegarde automatique (R-48). Les lignes en erreur ne s'importent jamais ; celles en
//! avertissement s'importent et restent à vérifier.

use std::collections::{HashMap, HashSet};
use std::path::Path;

use calamine::{Data, Reader};
use gesco_core::commun::DateCivile;
use gesco_core::core::{Operation, TypeEvenement};
use gesco_core::scolarite::import_eleves::{
    Champ, Decoupe, Disposition, EleveLu, ligne_vide, lire_ligne, reconnaitre,
    verifier_correspondance,
};
use gesco_core::scolarite::personne::{matricule_national, texte_propre};
use gesco_core::scolarite::recherche::{self, IdentiteComparee, doublon_probable};
use gesco_core::scolarite::{
    Classe, DecisionFinAnnee, Eleve, Inscription, Lien, Personne, Qualites, Responsable, Sexe,
    StatutInscription, TypeInscription, telephone,
};
use gesco_db::depots::{academique, annee_scolaire, inscriptions, personnes};
use gesco_db::nouvel_id;
use gesco_db::unite::Durabilite;
use serde::{Deserialize, Serialize};

use crate::eleves::{ServiceEleves, nouvelle_personne};
use crate::erreur::{ErreurService, Resultat};
use crate::inscriptions::jour;
use crate::noyau::Noyau;
use crate::session::Session;

fn regle(cle: &'static str) -> ErreurService {
    ErreurService::Regle { cle }
}

/// Taille maximale du fichier : 10 Mo (§10.2).
pub const TAILLE_MAX: u64 = 10 * 1024 * 1024;
/// Lignes au plus, toutes feuilles confondues (§10.2).
pub const LIGNES_MAX: usize = 5_000;
/// Lignes montrées par feuille dans l'analyse.
const APERCU_LIGNES: usize = 5;

/// Fichier déposé, lu et gardé en mémoire le temps de l'assistant.
#[derive(Debug, Clone)]
pub struct FichierImporte {
    /// Utilisateur qui l'a déposé : lui seul le voit.
    pub utilisateur_id: gesco_core::commun::Id<gesco_core::core::Utilisateur>,
    /// Nom du fichier.
    pub nom: String,
    /// Feuilles : nom, puis lignes (la première est l'en-tête).
    pub feuilles: Vec<(String, Vec<Vec<String>>)>,
}

/// Fichier à analyser : un chemin rendu par le dialogue du système.
#[derive(Debug, Deserialize, specta::Type)]
pub struct DemandeAnalyse {
    /// Chemin du fichier `.xlsx`, `.xls`, `.ods` ou `.csv`.
    pub chemin: String,
}

/// Feuille lue.
#[derive(Debug, Clone, Serialize, specta::Type)]
pub struct FeuilleAnalysee {
    /// Nom de la feuille.
    pub nom: String,
    /// Lignes de données, en-tête non compté.
    pub lignes: u32,
    /// En-têtes.
    pub entetes: Vec<String>,
    /// Premières lignes.
    pub apercu: Vec<Vec<String>>,
}

/// Résultat de l'analyse : ce que l'écran propose, corrigeable.
#[derive(Debug, Clone, Serialize, specta::Type)]
pub struct Analyse {
    /// Identifiant du fichier gardé en mémoire.
    pub fichier_id: String,
    /// Feuilles.
    pub feuilles: Vec<FeuilleAnalysee>,
    /// Correspondance proposée pour les colonnes de la première feuille.
    pub champs: Vec<Champ>,
    /// Disposition proposée.
    pub disposition: Disposition,
    /// Classes de l'année courante, libellés.
    pub classes: Vec<String>,
    /// L'école saisit ses matricules : la colonne matricule est exigée.
    pub matricule_manuel: bool,
}

/// Réglages de l'assistant, valables pour la validation et l'application.
#[derive(Debug, Clone, Deserialize, specta::Type)]
pub struct Reglages {
    /// Fichier analysé.
    pub fichier_id: String,
    /// Correspondance des colonnes.
    pub champs: Vec<Champ>,
    /// Où lire la classe.
    pub disposition: Disposition,
    /// Règle de découpe d'une colonne « nom et prénoms ».
    pub decoupe: Decoupe,
}

/// Sort d'une ligne.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, specta::Type)]
#[serde(rename_all = "snake_case")]
pub enum EtatLigne {
    /// S'importe telle quelle.
    Valide,
    /// S'importe, à vérifier ensuite.
    Avertissement,
    /// Ne s'importe pas.
    Erreur,
}

/// Ligne de l'aperçu.
#[derive(Debug, Clone, Serialize, specta::Type)]
pub struct LigneApercu {
    /// Feuille.
    pub feuille: String,
    /// Numéro de la ligne dans la feuille, en-tête compris (1).
    pub numero: u32,
    /// Nom lu.
    pub nom: String,
    /// Prénoms lus.
    pub prenoms: String,
    /// Sexe lu.
    pub sexe: Option<Sexe>,
    /// Date de naissance lue.
    pub date_naissance: Option<DateCivile>,
    /// Classe lue.
    pub classe: String,
    /// Matricule interne lu.
    pub matricule_interne: String,
    /// Téléphone du responsable, tel qu'écrit.
    pub telephone: String,
    /// Sort.
    pub etat: EtatLigne,
    /// Clés des motifs, erreurs puis avertissements.
    pub motifs: Vec<String>,
    /// Fiche existante probablement identique : « matricule · nom prénoms ».
    pub doublon_de: Option<String>,
}

/// Aperçu complet.
#[derive(Debug, Clone, Serialize, specta::Type)]
pub struct Apercu {
    /// Lignes, dans l'ordre du fichier.
    pub lignes: Vec<LigneApercu>,
    /// Lignes valides.
    pub valides: u32,
    /// Lignes en avertissement.
    pub avertissements: u32,
    /// Lignes en erreur.
    pub erreurs: u32,
    /// Classes citées par le fichier et absentes de l'année : à créer avant d'importer.
    pub classes_absentes: Vec<String>,
}

/// Rapport d'application.
#[derive(Debug, Clone, Serialize, specta::Type)]
pub struct Rapport {
    /// Élèves créés, avec leur brouillon d'inscription.
    pub importes: u32,
    /// Responsables créés.
    pub responsables_crees: u32,
    /// Responsables retrouvés par leur téléphone et liés.
    pub responsables_retrouves: u32,
    /// Lignes en erreur, laissées de côté.
    pub ignorees: u32,
}

/// Lit un fichier déposé : tableur par `calamine`, CSV lu ici. Rien n'est lu en JavaScript.
pub fn lire_fichier(chemin: &str) -> Resultat<Vec<(String, Vec<Vec<String>>)>> {
    let chemin = Path::new(chemin);
    let taille = std::fs::metadata(chemin)?.len();
    if taille > TAILLE_MAX {
        return Err(regle("import.fichier_trop_grand"));
    }
    let extension = chemin
        .extension()
        .and_then(|e| e.to_str())
        .map(str::to_ascii_lowercase)
        .unwrap_or_default();
    let feuilles = match extension.as_str() {
        "csv" | "txt" => {
            let nom = chemin
                .file_stem()
                .and_then(|n| n.to_str())
                .unwrap_or("CSV")
                .to_owned();
            vec![(nom, lire_csv(&std::fs::read(chemin)?))]
        }
        "xlsx" | "xlsm" | "xls" | "ods" => {
            let mut classeur =
                calamine::open_workbook_auto(chemin).map_err(|e| ErreurService::Regle {
                    cle: match e {
                        calamine::Error::Io(_) => "erreur.fichier",
                        _ => "import.fichier_illisible",
                    },
                })?;
            classeur
                .worksheets()
                .into_iter()
                .map(|(nom, plage)| {
                    let lignes = plage
                        .rows()
                        .map(|cellules| cellules.iter().map(cellule).collect())
                        .collect();
                    (nom, lignes)
                })
                .collect()
        }
        _ => return Err(regle("import.format_inconnu")),
    };
    let feuilles: Vec<(String, Vec<Vec<String>>)> = feuilles
        .into_iter()
        .map(|(nom, lignes)| (nom, retirer_lignes_vides(lignes)))
        .filter(|(_, lignes)| lignes.len() > 1)
        .collect();
    if feuilles.is_empty() {
        return Err(regle("import.fichier_vide"));
    }
    if feuilles.iter().map(|(_, l)| l.len() - 1).sum::<usize>() > LIGNES_MAX {
        return Err(regle("import.trop_de_lignes"));
    }
    Ok(feuilles)
}

/// Texte d'une cellule : les nombres entiers sans décimale, les dates en `AAAA-MM-JJ`.
fn cellule(valeur: &Data) -> String {
    match valeur {
        Data::Empty => String::new(),
        Data::String(s) | Data::DateTimeIso(s) | Data::DurationIso(s) => s.trim().to_owned(),
        Data::Int(i) => i.to_string(),
        Data::Float(f) if f.fract() == 0.0 && f.abs() < 1e15 => format!("{f:.0}"),
        Data::Float(f) => f.to_string(),
        Data::Bool(b) => b.to_string(),
        Data::DateTime(d) => date_excel(d.as_f64())
            .map(|d| d.to_string())
            .unwrap_or_else(|| d.as_f64().to_string()),
        Data::Error(e) => format!("#{e:?}"),
    }
}

/// Date d'un numéro de série Excel (jours depuis le 30 décembre 1899) : le 1er janvier 1970
/// est le jour 25 569. Le calendrier civil se retrouve par l'algorithme des ères de 400 ans.
fn date_excel(serie: f64) -> Option<DateCivile> {
    if !serie.is_finite() || serie < 1.0 || serie > 2_958_465.0 {
        return None;
    }
    // Les bornes ci-dessus garantissent une conversion exacte.
    #[allow(clippy::cast_possible_truncation)]
    let jours = serie.floor() as i64 - 25_569 + 719_468;
    let ere = jours.div_euclid(146_097);
    let jour_ere = jours.rem_euclid(146_097);
    let annee_ere = (jour_ere - jour_ere / 1_460 + jour_ere / 36_524 - jour_ere / 146_096) / 365;
    let annee = annee_ere + ere * 400;
    let jour_annee = jour_ere - (365 * annee_ere + annee_ere / 4 - annee_ere / 100);
    let mois_decale = (5 * jour_annee + 2) / 153;
    let jour = jour_annee - (153 * mois_decale + 2) / 5 + 1;
    let mois = if mois_decale < 10 {
        mois_decale + 3
    } else {
        mois_decale - 9
    };
    let annee = if mois <= 2 { annee + 1 } else { annee };
    DateCivile::nouvelle(
        i32::try_from(annee).ok()?,
        u8::try_from(mois).ok()?,
        u8::try_from(jour).ok()?,
    )
}

fn retirer_lignes_vides(lignes: Vec<Vec<String>>) -> Vec<Vec<String>> {
    let mut lignes: Vec<Vec<String>> = lignes.into_iter().filter(|l| !ligne_vide(l)).collect();
    // Les colonnes vides à droite de l'en-tête ne portent rien.
    if let Some(entete) = lignes.first_mut() {
        while entete.last().is_some_and(|c| c.trim().is_empty()) {
            entete.pop();
        }
    }
    lignes
}

/// CSV tel qu'Excel l'écrit en français : point-virgule, guillemets doublés, marque d'ordre
/// d'octets ; la virgule et la tabulation sont acceptées, le Windows-1252 aussi.
pub fn lire_csv(octets: &[u8]) -> Vec<Vec<String>> {
    let texte = match std::str::from_utf8(octets) {
        Ok(t) => t.to_owned(),
        Err(_) => octets.iter().map(|&b| char::from(b)).collect(),
    };
    let texte = texte.strip_prefix('\u{feff}').unwrap_or(&texte);
    let premiere = texte.lines().next().unwrap_or("");
    let separateur = [';', '\t', ',']
        .into_iter()
        .max_by_key(|s| premiere.matches(*s).count())
        .unwrap_or(';');
    let mut lignes = Vec::new();
    let mut ligne = Vec::new();
    let mut champ = String::new();
    let mut entre_guillemets = false;
    let mut chars = texte.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '"' if entre_guillemets && chars.peek() == Some(&'"') => {
                champ.push('"');
                chars.next();
            }
            '"' => entre_guillemets = !entre_guillemets,
            c if c == separateur && !entre_guillemets => ligne.push(std::mem::take(&mut champ)),
            '\r' if !entre_guillemets => {}
            '\n' if !entre_guillemets => {
                ligne.push(std::mem::take(&mut champ));
                lignes.push(std::mem::take(&mut ligne));
            }
            c => champ.push(c),
        }
    }
    if !champ.is_empty() || !ligne.is_empty() {
        ligne.push(champ);
        lignes.push(ligne);
    }
    lignes
}

/// Service d'import.
pub struct ServiceImport<'n> {
    noyau: &'n Noyau,
}

/// Élève prêt à être inséré.
struct Prepare {
    lu: EleveLu,
    classe: Classe,
}

impl<'n> ServiceImport<'n> {
    /// Service sur le noyau ouvert.
    pub const fn nouveau(noyau: &'n Noyau) -> Self {
        Self { noyau }
    }

    async fn connexion(&self) -> Resultat<gesco_db::ConnexionEmpruntee> {
        Ok(self
            .noyau
            .base()
            .pool()
            .acquire()
            .await
            .map_err(gesco_db::ErreurBase::from)?)
    }

    async fn classes_courantes(&self, session: &Session) -> Resultat<Vec<Classe>> {
        let contexte = self.noyau.contexte(session.etablissement_id, Some(session));
        let mut cnx = self.connexion().await?;
        let Some(annee) = annee_scolaire::courante(&mut cnx, &contexte).await? else {
            return Err(regle("eleve.annee_absente"));
        };
        Ok(academique::classes(&mut cnx, annee.id, &contexte).await?)
    }

    /// Analyse un fichier lu : feuilles, correspondance et disposition proposées.
    pub async fn analyser(
        &self,
        session: &Session,
        fichier_id: &str,
        fichier: &FichierImporte,
    ) -> Resultat<Analyse> {
        let classes = self.classes_courantes(session).await?;
        let libelles: Vec<String> = classes.iter().map(|c| c.libelle.clone()).collect();
        let matricule_manuel = ServiceEleves::nouveau(self.noyau)
            .reglages(session)
            .await?
            .matricule_manuel;
        let feuilles: Vec<FeuilleAnalysee> = fichier
            .feuilles
            .iter()
            .map(|(nom, lignes)| FeuilleAnalysee {
                nom: nom.clone(),
                lignes: u32::try_from(lignes.len().saturating_sub(1)).unwrap_or(u32::MAX),
                entetes: lignes.first().cloned().unwrap_or_default(),
                apercu: lignes.iter().skip(1).take(APERCU_LIGNES).cloned().collect(),
            })
            .collect();
        let champs: Vec<Champ> = feuilles
            .first()
            .map(|f| f.entetes.iter().map(|e| reconnaitre(e)).collect())
            .unwrap_or_default();
        // Une feuille par classe quand plusieurs feuilles portent des noms de classes de
        // l'année, ou quand aucune colonne ne porte la classe.
        let feuilles_classes = fichier
            .feuilles
            .iter()
            .filter(|(nom, _)| classe_de(&classes, nom).is_some())
            .count();
        let disposition = if (fichier.feuilles.len() > 1 && feuilles_classes > 0)
            || !champs.contains(&Champ::Classe)
        {
            Disposition::FeuilleParClasse
        } else {
            Disposition::ColonneClasse
        };
        Ok(Analyse {
            fichier_id: fichier_id.to_owned(),
            feuilles,
            champs,
            disposition,
            classes: libelles,
            matricule_manuel,
        })
    }

    /// Lit chaque ligne selon les réglages et vérifie ce que le noyau exige : identité
    /// complète, classe de l'année, matricules uniques dans le fichier et dans la base,
    /// doublons probables signalés.
    async fn preparer(
        &self,
        session: &Session,
        fichier: &FichierImporte,
        reglages: &Reglages,
    ) -> Resultat<(Vec<(LigneApercu, Option<Prepare>)>, Vec<String>)> {
        verifier_correspondance(&reglages.champs, reglages.disposition).map_err(regle)?;
        let contexte = self.noyau.contexte(session.etablissement_id, Some(session));
        let classes = self.classes_courantes(session).await?;
        let matricule_manuel = ServiceEleves::nouveau(self.noyau)
            .reglages(session)
            .await?
            .matricule_manuel;
        let existants = {
            let mut cnx = self.connexion().await?;
            personnes::eleves(&mut cnx, &contexte).await?
        };
        let matricules_pris: HashSet<String> = existants
            .iter()
            .map(|(e, _)| recherche::normaliser(&e.matricule_interne))
            .collect();
        let nationaux_pris: HashSet<String> = existants
            .iter()
            .filter_map(|(e, _)| e.matricule_national.clone())
            .collect();
        let mut vus_fichier: HashMap<String, (String, u32)> = HashMap::new();
        let mut matricules_fichier: HashSet<String> = HashSet::new();
        let mut nationaux_fichier: HashSet<String> = HashSet::new();
        let mut classes_absentes: Vec<String> = Vec::new();
        let mut resultat = Vec::new();
        for (feuille, lignes) in &fichier.feuilles {
            let classe_de_feuille =
                (reglages.disposition == Disposition::FeuilleParClasse).then_some(feuille.as_str());
            for (index, cellules) in lignes.iter().enumerate().skip(1) {
                let numero = u32::try_from(index + 1).unwrap_or(u32::MAX);
                let mut lu = lire_ligne(
                    &reglages.champs,
                    cellules,
                    reglages.decoupe,
                    classe_de_feuille,
                );
                let classe = classe_de(&classes, &lu.classe).cloned();
                if classe.is_none() && !lu.classe.is_empty() {
                    lu.erreurs.push("import.classe_absente");
                    if !classes_absentes.contains(&lu.classe) {
                        classes_absentes.push(lu.classe.clone());
                    }
                }
                if matricule_manuel {
                    let m = recherche::normaliser(&lu.matricule_interne);
                    if m.is_empty() {
                        lu.erreurs.push("import.matricule_manquant");
                    } else if matricules_pris.contains(&m) {
                        lu.erreurs.push("eleve.matricule_pris");
                    } else if !matricules_fichier.insert(m) {
                        lu.erreurs.push("import.matricule_double");
                    }
                } else {
                    lu.matricule_interne.clear();
                }
                match matricule_national(&lu.matricule_national) {
                    Err(cle) => lu.erreurs.push(cle),
                    Ok(Some(n)) => {
                        if nationaux_pris.contains(&n) {
                            lu.erreurs.push("eleve.matricule_national_pris");
                        } else if !nationaux_fichier.insert(n) {
                            lu.erreurs.push("import.matricule_national_double");
                        }
                    }
                    Ok(None) => {}
                }
                if !lu.responsable_telephone.trim().is_empty()
                    && telephone::normaliser(&lu.responsable_telephone).is_err()
                {
                    lu.avertissements.push("import.telephone_illisible");
                }
                let mut doublon_de = None;
                let date = lu.date_naissance;
                let cle_fichier = format!(
                    "{}|{}|{}",
                    recherche::normaliser(&lu.nom),
                    recherche::normaliser(&lu.prenoms),
                    date.map(|d| d.jours_depuis_epoque()).unwrap_or_default()
                );
                if !lu.nom.is_empty() {
                    if let Some((f, n)) = vus_fichier.get(&cle_fichier) {
                        lu.avertissements.push("import.doublon_fichier");
                        doublon_de = Some(format!("{f} · {n}"));
                    } else {
                        vus_fichier.insert(cle_fichier, (feuille.clone(), numero));
                    }
                }
                if doublon_de.is_none() {
                    let saisie = IdentiteComparee {
                        nom: &lu.nom,
                        prenoms: &lu.prenoms,
                        date_naissance: date,
                    };
                    if let Some((e, p)) = existants.iter().find(|(_, p)| {
                        doublon_probable(
                            saisie,
                            IdentiteComparee {
                                nom: &p.nom,
                                prenoms: &p.prenoms,
                                date_naissance: p.date_naissance,
                            },
                        )
                    }) {
                        lu.avertissements.push("import.doublon_base");
                        doublon_de = Some(format!("{} · {}", e.matricule_interne, p.nom_complet()));
                    }
                }
                let etat = if !lu.erreurs.is_empty() {
                    EtatLigne::Erreur
                } else if !lu.avertissements.is_empty() {
                    EtatLigne::Avertissement
                } else {
                    EtatLigne::Valide
                };
                let apercu = LigneApercu {
                    feuille: feuille.clone(),
                    numero,
                    nom: lu.nom.clone(),
                    prenoms: lu.prenoms.clone(),
                    sexe: lu.sexe,
                    date_naissance: lu.date_naissance,
                    classe: lu.classe.clone(),
                    matricule_interne: lu.matricule_interne.clone(),
                    telephone: lu.responsable_telephone.clone(),
                    etat,
                    motifs: lu
                        .erreurs
                        .iter()
                        .chain(lu.avertissements.iter())
                        .map(|c| (*c).to_owned())
                        .collect(),
                    doublon_de,
                };
                let prepare = match (etat, classe) {
                    (EtatLigne::Erreur, _) | (_, None) => None,
                    (_, Some(classe)) => Some(Prepare { lu, classe }),
                };
                resultat.push((apercu, prepare));
            }
        }
        Ok((resultat, classes_absentes))
    }

    /// Aperçu ligne par ligne.
    pub async fn valider(
        &self,
        session: &Session,
        fichier: &FichierImporte,
        reglages: &Reglages,
    ) -> Resultat<Apercu> {
        let (lignes, classes_absentes) = self.preparer(session, fichier, reglages).await?;
        let compte = |etat| {
            u32::try_from(lignes.iter().filter(|(l, _)| l.etat == etat).count()).unwrap_or(u32::MAX)
        };
        Ok(Apercu {
            valides: compte(EtatLigne::Valide),
            avertissements: compte(EtatLigne::Avertissement),
            erreurs: compte(EtatLigne::Erreur),
            lignes: lignes.into_iter().map(|(l, _)| l).collect(),
            classes_absentes,
        })
    }

    /// Applique les lignes valides et en avertissement en une transaction : fiche, responsable
    /// (retrouvé par son téléphone ou créé), brouillon d'inscription dans la classe. La
    /// sauvegarde automatique est prise par l'appelant avant (R-48).
    pub async fn appliquer(
        &self,
        session: &Session,
        fichier_id: &str,
        fichier: &FichierImporte,
        reglages: &Reglages,
    ) -> Resultat<Rapport> {
        let (lignes, _) = self.preparer(session, fichier, reglages).await?;
        let contexte = self.noyau.contexte(session.etablissement_id, Some(session));
        let service_eleves = ServiceEleves::nouveau(self.noyau);
        let insertion = service_eleves.preparer_insertion(session).await?;
        let annee = {
            let mut cnx = self.connexion().await?;
            annee_scolaire::courante(&mut cnx, &contexte)
                .await?
                .ok_or(regle("eleve.annee_absente"))?
        };
        let aujourd_hui = jour(contexte.maintenant)?;
        let mut responsables_par_telephone: HashMap<String, Responsable> = {
            let mut cnx = self.connexion().await?;
            personnes::responsables(&mut cnx, &contexte)
                .await?
                .into_iter()
                .flat_map(|(r, p)| p.telephones.into_iter().map(move |t| (t, r.clone())))
                .collect()
        };
        let mut unite = self
            .noyau
            .commencer(contexte.clone(), Durabilite::Critique)
            .await?;
        let mut rapport = Rapport {
            importes: 0,
            responsables_crees: 0,
            responsables_retrouves: 0,
            ignorees: 0,
        };
        let texte = |t: &str| texte_propre(t, "personne.texte_trop_long").map_err(regle);
        for (_, prepare) in lignes {
            let Some(Prepare { lu, classe }) = prepare else {
                rapport.ignorees += 1;
                continue;
            };
            let personne = Personne {
                nom: texte(&lu.nom)?,
                prenoms: texte(&lu.prenoms)?,
                sexe: lu.sexe,
                date_naissance: lu.date_naissance,
                date_naissance_estimee: lu.date_estimee,
                lieu_naissance: texte(&lu.lieu_naissance)?,
                adresse: texte(&lu.adresse)?,
                ..nouvelle_personne(session)
            };
            personne
                .verifier_eleve(contexte.maintenant)
                .map_err(regle)?;
            let brouillon = Eleve {
                matricule_national: matricule_national(&lu.matricule_national).map_err(regle)?,
                ..ServiceEleves::eleve_vierge(session, &personne)
            };
            let saisi = insertion
                .manuel
                .then(|| lu.matricule_interne.trim().to_owned());
            let eleve = ServiceEleves::inserer_dans(
                &mut unite,
                &contexte,
                &insertion,
                &personne,
                brouillon,
                saisi,
                "import.appliquer",
            )
            .await?;
            if let Ok(numero) = telephone::normaliser(&lu.responsable_telephone) {
                let responsable = match responsables_par_telephone.get(&numero) {
                    Some(r) => {
                        rapport.responsables_retrouves += 1;
                        r.clone()
                    }
                    None => {
                        let (nom, prenoms) = nom_du_responsable(&lu);
                        let personne_responsable = Personne {
                            nom: texte(&nom)?,
                            prenoms: texte(&prenoms)?,
                            telephones: vec![numero.clone()],
                            ..nouvelle_personne(session)
                        };
                        let responsable = Responsable {
                            id: nouvel_id(),
                            etablissement_id: session.etablissement_id,
                            personne_id: personne_responsable.id,
                            profession: String::new(),
                            employeur: String::new(),
                        };
                        personnes::inserer_personne(
                            unite.connexion(),
                            &personne_responsable,
                            &contexte,
                        )
                        .await?;
                        personnes::inserer_responsable(unite.connexion(), &responsable, &contexte)
                            .await?;
                        unite.auditer(
                            "responsable",
                            &responsable.id.vers_chaine(),
                            Operation::Creation,
                            None,
                            Some(serde_json::json!({ "responsable": responsable, "identite": personne_responsable })),
                            "import.appliquer",
                        );
                        responsables_par_telephone.insert(numero, responsable.clone());
                        rapport.responsables_crees += 1;
                        responsable
                    }
                };
                let qualites = Qualites {
                    lien: Lien::Tuteur,
                    est_responsable_legal: true,
                    est_payeur: true,
                    recoit_notifications: true,
                    vient_chercher: false,
                    ordre_priorite: 1,
                };
                personnes::lier(
                    unite.connexion(),
                    eleve.id,
                    responsable.id,
                    &qualites,
                    &contexte,
                )
                .await?;
                unite.emettre(
                    TypeEvenement::ResponsableRattache,
                    "eleve",
                    &eleve.id.vers_chaine(),
                    serde_json::json!({ "responsable_id": responsable.id, "lien": qualites.lien }),
                );
            }
            let inscription = Inscription {
                id: nouvel_id(),
                etablissement_id: session.etablissement_id,
                annee_id: annee.id,
                eleve_id: eleve.id,
                classe_id: classe.id,
                date_inscription: aujourd_hui,
                type_inscription: TypeInscription::deduire(false, ""),
                statut: StatutInscription::Brouillon,
                redoublant: false,
                decision_fin_annee: DecisionFinAnnee::NonStatue,
                classe_precedente_id: None,
                etablissement_origine: String::new(),
                motif_abandon: String::new(),
                date_depart: None,
                etablissement_destination: String::new(),
                motif_depart: String::new(),
            };
            inscriptions::inserer(unite.connexion(), &inscription, &contexte).await?;
            let id = inscription.id.vers_chaine();
            unite.auditer(
                "inscription",
                &id,
                Operation::Creation,
                None,
                Some(serde_json::to_value(&inscription)?),
                "import.appliquer",
            );
            unite.emettre(
                TypeEvenement::InscriptionPreparee,
                "inscription",
                &id,
                serde_json::json!({
                    "eleve_id": inscription.eleve_id,
                    "classe_id": inscription.classe_id,
                    "annee_id": inscription.annee_id,
                }),
            );
            rapport.importes += 1;
        }
        unite.auditer(
            "import",
            fichier_id,
            Operation::Creation,
            None,
            Some(serde_json::json!({ "fichier": fichier.nom, "rapport": rapport })),
            "import.appliquer",
        );
        unite.emettre(
            TypeEvenement::ImportElevesApplique,
            "import",
            fichier_id,
            serde_json::json!({ "fichier": fichier.nom, "importes": rapport.importes }),
        );
        self.noyau.terminer(unite).await?;
        Ok(rapport)
    }
}

/// Classe de l'année dont le libellé correspond, à la casse et aux accents près.
fn classe_de<'c>(classes: &'c [Classe], libelle: &str) -> Option<&'c Classe> {
    let cherche = recherche::normaliser(libelle);
    if cherche.is_empty() {
        return None;
    }
    classes
        .iter()
        .find(|c| recherche::normaliser(&c.libelle) == cherche)
}

/// Nom et prénoms du responsable : la colonne du fichier, découpée nom d'abord ; sans elle, le
/// nom de l'élève et la mention « Parent », à corriger sur la fiche.
fn nom_du_responsable(lu: &EleveLu) -> (String, String) {
    let (nom, prenoms) =
        gesco_core::scolarite::import_eleves::decouper(&lu.responsable_nom, Decoupe::NomDAbord);
    if nom.is_empty() {
        return (lu.nom.clone(), "Parent".to_owned());
    }
    if prenoms.is_empty() {
        return (nom, "Parent".to_owned());
    }
    (nom, prenoms)
}

#[cfg(test)]
#[allow(clippy::indexing_slicing)]
mod tests {
    use super::*;

    #[test]
    fn le_csv_d_excel_se_lit() {
        let octets = "\u{feff}Nom;Prénoms;\"Né(e) le\"\r\nTRAORE;Awa;10/05/2014\r\n\"DIALLO; dit Jallo\";Moussa;\"vers 2013\"\r\n".as_bytes();
        let lignes = lire_csv(octets);
        assert_eq!(lignes.len(), 3);
        assert_eq!(lignes[0], vec!["Nom", "Prénoms", "Né(e) le"]);
        assert_eq!(lignes[2], vec!["DIALLO; dit Jallo", "Moussa", "vers 2013"]);
        let virgules = lire_csv(b"a,b\n1,2");
        assert_eq!(virgules, vec![vec!["a", "b"], vec!["1", "2"]]);
        let latin = lire_csv(b"Pr\xe9noms\n");
        assert_eq!(latin[0][0], "Prénoms");
    }

    #[test]
    fn les_dates_excel_se_convertissent() {
        let jour = |a, m, j| DateCivile::nouvelle(a, m, j);
        assert_eq!(date_excel(25_569.0), jour(1970, 1, 1));
        assert_eq!(date_excel(44_927.0), jour(2023, 1, 1));
        assert_eq!(date_excel(41_769.5), jour(2014, 5, 10));
        assert_eq!(date_excel(36_585.0), jour(2000, 2, 29));
        assert_eq!(date_excel(0.0), None);
        assert_eq!(
            cellule(&Data::DateTime(calamine::ExcelDateTime::new(
                44_927.0,
                calamine::ExcelDateTimeType::DateTime,
                false
            ))),
            "2023-01-01"
        );
    }

    #[test]
    fn les_cellules_du_tableur_deviennent_du_texte() {
        assert_eq!(cellule(&Data::Float(2014.0)), "2014");
        assert_eq!(cellule(&Data::Float(11.5)), "11.5");
        assert_eq!(cellule(&Data::Int(7)), "7");
        assert_eq!(cellule(&Data::String("  Awa ".to_owned())), "Awa");
        assert_eq!(cellule(&Data::Empty), "");
    }
}
