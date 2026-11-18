//! Import Excel de ce qui était payé avant GESCO (SPEC_REPRISE_AVANT_GESCO §5) : un modèle
//! prérempli avec les inscriptions validées, la colonne « Payé avant GESCO » remplie d'après le
//! cahier, puis un aperçu ligne par ligne avant toute écriture. L'écriture se fait en une
//! transaction, après une sauvegarde automatique (R-48), par le même chemin que l'écran
//! (`reprises::enregistrer_dans`, origine `import`). Une ligne en erreur n'empêche pas les autres.

use std::collections::HashMap;

use gesco_core::commun::{Id, Montant};
use gesco_core::core::{AnneeScolaire, Operation, TypeEvenement};
use gesco_core::finance::reprise::{OrigineReprise, lire_montant};
use gesco_core::scolarite::recherche::{cle_phonetique, normaliser};
use gesco_core::scolarite::{Classe, Eleve, Inscription, Personne, StatutInscription};
use gesco_db::depots::{academique, annee_scolaire, inscriptions, personnes};
use gesco_db::unite::Durabilite;
use gesco_db::{Contexte, nouvel_id};
use gesco_docs::tableur::{Liste, classeur_feuilles};
use serde::{Deserialize, Serialize};

use crate::erreur::{ErreurService, Resultat};
use crate::export::FichierEcrit;
use crate::import_eleves::FichierImporte;
use crate::noyau::Noyau;
use crate::reprises::{DemandeReprises, LigneRepriseAvantGesco, ServiceReprises, enregistrer_dans};
use crate::session::Session;

fn regle(cle: &'static str) -> ErreurService {
    ErreurService::Regle { cle }
}

/// Colonnes du modèle, dans l'ordre. « Payé avant GESCO » est exigée, avec le matricule ou bien
/// le nom, les prénoms et la classe.
pub const COLONNES_MODELE: [&str; 7] = [
    "Matricule",
    "Nom",
    "Prénoms",
    "Classe",
    "Dû de l'année",
    "Payé avant GESCO",
    "Note",
];

/// Mode d'emploi du modèle, une ligne par consigne.
const MODE_D_EMPLOI: [&str; 5] = [
    "Remplissez seulement la colonne « Payé avant GESCO », d'après le cahier ou les reçus papier.",
    "Un montant par élève, en francs, sans décimales : 75000, 75 000 ou 75 000 F.",
    "Laissez la case vide pour un élève dont vous ne voulez rien changer ; 0 annule ce qui avait été saisi.",
    "La note est facultative : la référence du cahier ou des reçus (« cahier p. 12 »).",
    "Gardez le matricule : c'est lui qui retrouve l'élève. Le dû de l'année est là pour lecture.",
];

/// Motif d'annulation d'une reprise mise à zéro par un classeur.
pub const MOTIF_ZERO_IMPORT: &str = "Montant 0 dans le classeur importé";

/// Classeur du modèle à écrire, prérempli avec les inscriptions validées de la sélection.
#[derive(Debug, Clone, Deserialize, specta::Type)]
pub struct DemandeModeleReprises {
    /// Chemin du classeur, extension `.xlsx` ; rendu par le dialogue du système.
    pub chemin: String,
    /// Année ; absente, l'année courante.
    #[serde(default)]
    pub annee_id: Option<Id<AnneeScolaire>>,
    /// Classe ; absente, toutes.
    #[serde(default)]
    pub classe_id: Option<Id<Classe>>,
}

/// Classeur rempli à analyser.
#[derive(Debug, Clone, Deserialize, specta::Type)]
pub struct DemandeAnalyseReprises {
    /// Chemin du fichier `.xlsx`, `.xls`, `.ods` ou `.csv`.
    pub chemin: String,
    /// Année dont les inscriptions sont visées ; absente, l'année courante.
    #[serde(default)]
    pub annee_id: Option<Id<AnneeScolaire>>,
}

/// Import d'un classeur déjà analysé.
#[derive(Debug, Clone, Deserialize, specta::Type)]
pub struct DemandeImportReprises {
    /// Fichier rendu par l'analyse.
    pub fichier_id: String,
    /// Année, la même qu'à l'analyse ; absente, l'année courante.
    #[serde(default)]
    pub annee_id: Option<Id<AnneeScolaire>>,
}

/// Sort d'une ligne du classeur.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, specta::Type)]
#[serde(rename_all = "snake_case")]
pub enum SortLigneReprise {
    /// Rien à faire : case vide ou montant inchangé.
    Ignoree,
    /// Sera enregistrée.
    Valide,
    /// Sera enregistrée ; à vérifier.
    Avertissement,
    /// Laissée de côté.
    Erreur,
}

/// Message d'une ligne : une clé du catalogue et ses montants.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, specta::Type)]
pub struct MessageLigneReprise {
    /// Clé du texte.
    pub cle: String,
    /// Premier montant cité.
    pub montant: Option<Montant>,
    /// Second montant cité (« remplace A par B »).
    pub autre: Option<Montant>,
}

/// Une ligne du classeur, telle que GESCO la comprend.
#[derive(Debug, Clone, Serialize, specta::Type)]
pub struct LigneImportReprise {
    /// Numéro de la ligne dans le fichier, en-tête compris.
    pub numero: u32,
    /// Matricule, tel qu'écrit.
    pub matricule: String,
    /// Nom, tel qu'écrit.
    pub nom: String,
    /// Prénoms, tels qu'écrits.
    pub prenoms: String,
    /// Classe, telle qu'écrite.
    pub classe: String,
    /// Payé avant GESCO, tel qu'écrit.
    pub saisi: String,
    /// Note, telle qu'écrite.
    pub note: String,
    /// Montant lu.
    pub montant: Option<Montant>,
    /// Élève retrouvé : prénoms et nom.
    pub eleve: String,
    /// Inscription retrouvée.
    pub inscription_id: Option<Id<Inscription>>,
    /// Sort de la ligne.
    pub sort: SortLigneReprise,
    /// Messages, dans l'ordre.
    pub messages: Vec<MessageLigneReprise>,
}

/// Aperçu du classeur, avant toute écriture.
#[derive(Debug, Clone, Serialize, specta::Type)]
pub struct ApercuReprises {
    /// Fichier déposé, à rendre à l'import.
    pub fichier_id: String,
    /// Nom du fichier.
    pub nom: String,
    /// Lignes, dans l'ordre du fichier.
    pub lignes: Vec<LigneImportReprise>,
    /// Lignes valides.
    pub valides: u32,
    /// Lignes en avertissement (enregistrées).
    pub avertissements: u32,
    /// Lignes en erreur (laissées de côté).
    pub erreurs: u32,
    /// Lignes ignorées.
    pub ignorees: u32,
    /// Total payé avant GESCO des lignes qui seront enregistrées.
    pub total: Montant,
}

/// Ce que l'import a fait.
#[derive(Debug, Clone, Serialize, specta::Type)]
pub struct RapportReprises {
    /// Reprises enregistrées ou remplacées.
    pub enregistrees: u32,
    /// Reprises annulées (montant 0).
    pub annulees: u32,
    /// Lignes ignorées (vides ou inchangées).
    pub ignorees: u32,
    /// Total enregistré.
    pub total: Montant,
    /// Lignes refusées, à corriger et réimporter.
    pub refusees: Vec<LigneImportReprise>,
}

/// Forme tolérante d'un nom : majuscules sans accents, puis clé phonétique de chaque mot.
fn tolerante(texte: &str) -> String {
    normaliser(texte)
        .split(' ')
        .map(cle_phonetique)
        .collect::<Vec<_>>()
        .join(" ")
}

/// Ce que l'analyse doit savoir d'un élève inscrit dans l'année.
struct Inscrit {
    inscription: Inscription,
    eleve: String,
    nom: String,
    prenoms: String,
    matricules: Vec<String>,
    classe: String,
}

/// Données de l'année : inscrits et lignes de l'écran « Payé avant GESCO ».
struct Annee {
    inscrits: Vec<Inscrit>,
    lignes: HashMap<Id<Inscription>, LigneRepriseAvantGesco>,
}

/// Colonnes repérées dans l'en-tête.
struct Colonnes {
    matricule: Option<usize>,
    nom: Option<usize>,
    prenoms: Option<usize>,
    classe: Option<usize>,
    montant: usize,
    note: Option<usize>,
}

impl Colonnes {
    fn depuis(entete: &[String]) -> Option<Self> {
        let formes: Vec<String> = entete.iter().map(|c| normaliser(c)).collect();
        let colonne = |nom: &str| formes.iter().position(|e| *e == normaliser(nom));
        let colonnes = Self {
            matricule: colonne("Matricule"),
            nom: colonne("Nom"),
            prenoms: colonne("Prénoms"),
            classe: colonne("Classe"),
            montant: colonne("Payé avant GESCO")?,
            note: colonne("Note"),
        };
        let par_nom = colonnes.nom.is_some() && colonnes.prenoms.is_some();
        (colonnes.matricule.is_some() || par_nom).then_some(colonnes)
    }
}

fn message(cle: &str, montant: Option<Montant>, autre: Option<Montant>) -> MessageLigneReprise {
    MessageLigneReprise {
        cle: cle.to_owned(),
        montant,
        autre,
    }
}

fn compte(lignes: &[LigneImportReprise], sort: SortLigneReprise) -> u32 {
    u32::try_from(lignes.iter().filter(|l| l.sort == sort).count()).unwrap_or(u32::MAX)
}

/// Service de l'import des reprises.
pub struct ServiceImportReprises<'n> {
    noyau: &'n Noyau,
}

impl<'n> ServiceImportReprises<'n> {
    /// Service sur le noyau ouvert.
    pub const fn nouveau(noyau: &'n Noyau) -> Self {
        Self { noyau }
    }

    async fn annee(
        &self,
        cnx: &mut gesco_db::Connexion,
        contexte: &Contexte,
        annee_id: Option<Id<AnneeScolaire>>,
    ) -> Resultat<AnneeScolaire> {
        match annee_id {
            Some(id) => Ok(annee_scolaire::lire(cnx, id, contexte).await?),
            None => annee_scolaire::courante(cnx, contexte)
                .await?
                .ok_or(regle("eleve.annee_absente")),
        }
    }

    /// Inscrits de l'année, avec leurs noms, matricules et classe, et les lignes de l'écran.
    async fn donnees(
        &self,
        session: &Session,
        annee_id: Option<Id<AnneeScolaire>>,
    ) -> Resultat<Annee> {
        let contexte = self.noyau.contexte(session.etablissement_id, Some(session));
        let mut cnx = self
            .noyau
            .base()
            .pool()
            .acquire()
            .await
            .map_err(gesco_db::ErreurBase::from)?;
        let annee = self.annee(&mut cnx, &contexte, annee_id).await?;
        let classes: HashMap<Id<Classe>, String> =
            academique::classes(&mut cnx, annee.id, &contexte)
                .await?
                .into_iter()
                .map(|c| (c.id, c.libelle))
                .collect();
        let eleves: HashMap<Id<Eleve>, (Eleve, Personne)> = personnes::eleves(&mut cnx, &contexte)
            .await?
            .into_iter()
            .map(|(e, p)| (e.id, (e, p)))
            .collect();
        let inscrits = inscriptions::de_l_annee(&mut cnx, annee.id, &contexte)
            .await?
            .into_iter()
            .filter_map(|inscription| {
                let (eleve, personne) = eleves.get(&inscription.eleve_id)?;
                let mut matricules = vec![normaliser(&eleve.matricule_interne)];
                if let Some(national) = eleve.matricule_national.as_deref() {
                    matricules.push(normaliser(national));
                }
                matricules.retain(|m| !m.is_empty());
                Some(Inscrit {
                    eleve: personne.nom_complet(),
                    nom: personne.nom.clone(),
                    prenoms: personne.prenoms.clone(),
                    matricules,
                    classe: classes
                        .get(&inscription.classe_id)
                        .cloned()
                        .unwrap_or_default(),
                    inscription,
                })
            })
            .collect();
        drop(cnx);
        let liste = ServiceReprises::nouveau(self.noyau)
            .lister(
                session,
                &DemandeReprises {
                    annee_id: Some(annee.id),
                    classe_id: None,
                },
            )
            .await?;
        Ok(Annee {
            inscrits,
            lignes: liste
                .lignes
                .into_iter()
                .map(|l| (l.inscription_id, l))
                .collect(),
        })
    }

    /// Écrit le modèle prérempli : une feuille des inscriptions validées de la sélection, une
    /// feuille « Mode d'emploi ».
    pub async fn ecrire_modele(
        &self,
        session: &Session,
        demande: &DemandeModeleReprises,
    ) -> Resultat<FichierEcrit> {
        if !std::path::Path::new(&demande.chemin)
            .extension()
            .is_some_and(|e| e.eq_ignore_ascii_case("xlsx"))
        {
            return Err(regle("export.extension"));
        }
        let donnees = self.donnees(session, demande.annee_id).await?;
        let mut inscrits: Vec<&Inscrit> = donnees
            .inscrits
            .iter()
            .filter(|i| {
                demande
                    .classe_id
                    .is_none_or(|c| c == i.inscription.classe_id)
            })
            .filter(|i| donnees.lignes.contains_key(&i.inscription.id))
            .collect();
        inscrits
            .sort_by(|a, b| (&a.classe, &a.nom, &a.prenoms).cmp(&(&b.classe, &b.nom, &b.prenoms)));
        let lignes: Vec<Vec<String>> = inscrits
            .iter()
            .filter_map(|i| {
                let ligne = donnees.lignes.get(&i.inscription.id)?;
                Some(vec![
                    ligne.matricule.clone(),
                    i.nom.clone(),
                    i.prenoms.clone(),
                    i.classe.clone(),
                    ligne.du.francs().to_string(),
                    if ligne.reprise.est_nul() {
                        String::new()
                    } else {
                        ligne.reprise.francs().to_string()
                    },
                    ligne.note.clone(),
                ])
            })
            .collect();
        let entetes: Vec<String> = COLONNES_MODELE.iter().map(|c| (*c).to_owned()).collect();
        let consignes: Vec<Vec<String>> = MODE_D_EMPLOI
            .iter()
            .map(|c| vec![(*c).to_owned()])
            .collect();
        let entete_consignes = vec!["Mode d'emploi".to_owned()];
        let nombre = u32::try_from(lignes.len()).map_err(|_| regle("export.liste_trop_grande"))?;
        let chemin = demande.chemin.clone();
        let octets = tokio::task::spawn_blocking(move || {
            classeur_feuilles(
                "Payé avant GESCO",
                &[
                    Liste {
                        titre: "Payé avant GESCO",
                        entetes: &entetes,
                        lignes: &lignes,
                    },
                    Liste {
                        titre: "Mode d'emploi",
                        entetes: &entete_consignes,
                        lignes: &consignes,
                    },
                ],
            )
        })
        .await
        .map_err(|e| ErreurService::Fichier(std::io::Error::other(e)))??;
        tokio::fs::write(&chemin, octets).await?;
        Ok(FichierEcrit {
            chemin,
            lignes: nombre,
        })
    }

    /// Ce que GESCO comprend du classeur, sans rien écrire (§5.3).
    pub async fn analyser(
        &self,
        session: &Session,
        fichier_id: &str,
        fichier: &FichierImporte,
        annee_id: Option<Id<AnneeScolaire>>,
    ) -> Resultat<ApercuReprises> {
        let donnees = self.donnees(session, annee_id).await?;
        let lignes = analyser_lignes(fichier, &donnees)?;
        let total = lignes
            .iter()
            .filter(|l| {
                matches!(
                    l.sort,
                    SortLigneReprise::Valide | SortLigneReprise::Avertissement
                )
            })
            .filter_map(|l| l.montant)
            .try_fold(Montant::ZERO, |s, m| s.ajouter(m))
            .map_err(|_| regle("facture.calcul_impossible"))?;
        Ok(ApercuReprises {
            fichier_id: fichier_id.to_owned(),
            nom: fichier.nom.clone(),
            valides: compte(&lignes, SortLigneReprise::Valide),
            avertissements: compte(&lignes, SortLigneReprise::Avertissement),
            erreurs: compte(&lignes, SortLigneReprise::Erreur),
            ignorees: compte(&lignes, SortLigneReprise::Ignoree),
            total,
            lignes,
        })
    }

    /// Écrit les lignes valides et en avertissement en une transaction : chaque reprise auditée
    /// et son événement émis, puis un audit et `ImportReprisesApplique` pour le fichier. La
    /// sauvegarde automatique est prise avant, par la frontière (R-48).
    pub async fn importer(
        &self,
        session: &Session,
        fichier: &FichierImporte,
        annee_id: Option<Id<AnneeScolaire>>,
    ) -> Resultat<RapportReprises> {
        let donnees = self.donnees(session, annee_id).await?;
        let lignes = analyser_lignes(fichier, &donnees)?;
        let contexte = self.noyau.contexte(session.etablissement_id, Some(session));
        let mut unite = self
            .noyau
            .commencer(contexte.clone(), Durabilite::Critique)
            .await?;
        let (mut enregistrees, mut annulees, mut ignorees) = (0u32, 0u32, 0u32);
        let mut total = Montant::ZERO;
        let mut refusees = Vec::new();
        for mut ligne in lignes {
            match ligne.sort {
                SortLigneReprise::Ignoree => {
                    ignorees += 1;
                    continue;
                }
                SortLigneReprise::Erreur => {
                    refusees.push(ligne);
                    continue;
                }
                SortLigneReprise::Valide | SortLigneReprise::Avertissement => {}
            }
            let (Some(inscription_id), Some(montant)) = (ligne.inscription_id, ligne.montant)
            else {
                refusees.push(ligne);
                continue;
            };
            let note = if ligne.note.trim().is_empty() {
                donnees
                    .lignes
                    .get(&inscription_id)
                    .map(|l| l.note.clone())
                    .unwrap_or_default()
            } else {
                ligne.note.clone()
            };
            match enregistrer_dans(
                &mut unite,
                &contexte,
                inscription_id,
                montant,
                &note,
                MOTIF_ZERO_IMPORT,
                OrigineReprise::Import,
            )
            .await
            {
                Ok(false) => ignorees += 1,
                Ok(true) if montant.est_nul() => annulees += 1,
                Ok(true) => {
                    enregistrees += 1;
                    total = total
                        .ajouter(montant)
                        .map_err(|_| regle("facture.calcul_impossible"))?;
                }
                // Les règles se vérifient avant toute écriture de la ligne : la refuser laisse
                // les autres passer.
                Err(ErreurService::Regle { cle }) => {
                    ligne.sort = SortLigneReprise::Erreur;
                    ligne.messages.push(message(cle, None, None));
                    refusees.push(ligne);
                }
                Err(e) => return Err(e),
            }
        }
        let rapport = RapportReprises {
            enregistrees,
            annulees,
            ignorees,
            total,
            refusees,
        };
        let id = nouvel_id::<RapportReprises>().vers_chaine();
        let resume = serde_json::json!({
            "fichier": fichier.nom,
            "enregistrees": rapport.enregistrees,
            "annulees": rapport.annulees,
            "ignorees": rapport.ignorees,
            "refusees": rapport.refusees.len(),
            "total": rapport.total,
        });
        unite.auditer(
            "import_reprises",
            &id,
            Operation::Creation,
            None,
            Some(resume.clone()),
            "reprises.importer",
        );
        unite.emettre(
            TypeEvenement::ImportReprisesApplique,
            "import_reprises",
            &id,
            resume,
        );
        self.noyau.terminer(unite).await?;
        Ok(rapport)
    }
}

/// Feuille à lire : la première dont l'en-tête porte « Payé avant GESCO », sinon la première.
fn feuille(fichier: &FichierImporte) -> Option<&Vec<Vec<String>>> {
    fichier
        .feuilles
        .iter()
        .find(|(_, lignes)| {
            lignes.first().is_some_and(|e| {
                e.iter()
                    .any(|c| normaliser(c) == normaliser("Payé avant GESCO"))
            })
        })
        .or_else(|| fichier.feuilles.first())
        .map(|(_, lignes)| lignes)
}

/// Lit et vérifie chaque ligne du classeur (§5.3).
fn analyser_lignes(fichier: &FichierImporte, donnees: &Annee) -> Resultat<Vec<LigneImportReprise>> {
    let lignes = feuille(fichier).ok_or(regle("import.fichier_vide"))?;
    let entete = lignes.first().ok_or(regle("import.fichier_vide"))?;
    let colonnes = Colonnes::depuis(entete).ok_or(regle("import_reprises.colonnes"))?;
    let mut sortie: Vec<LigneImportReprise> = Vec::new();
    for (i, l) in lignes.iter().enumerate().skip(1) {
        let cellule = |c: Option<usize>| {
            c.and_then(|c| l.get(c))
                .map(|t| t.trim().to_owned())
                .unwrap_or_default()
        };
        let mut ligne = LigneImportReprise {
            numero: u32::try_from(i + 1).unwrap_or(u32::MAX),
            matricule: cellule(colonnes.matricule),
            nom: cellule(colonnes.nom),
            prenoms: cellule(colonnes.prenoms),
            classe: cellule(colonnes.classe),
            saisi: cellule(Some(colonnes.montant)),
            note: cellule(colonnes.note),
            montant: None,
            eleve: String::new(),
            inscription_id: None,
            sort: SortLigneReprise::Valide,
            messages: Vec::new(),
        };
        // Une ligne entièrement vide se saute sans rien dire.
        if l.iter().all(|c| c.trim().is_empty()) {
            continue;
        }
        verifier_ligne(&mut ligne, donnees);
        sortie.push(ligne);
    }
    // Un élève deux fois dans le fichier : erreur sur ses deux lignes.
    let mut vues: HashMap<Id<Inscription>, usize> = HashMap::new();
    for id in sortie.iter().filter_map(|l| l.inscription_id) {
        *vues.entry(id).or_default() += 1;
    }
    for l in &mut sortie {
        if l.inscription_id
            .is_some_and(|id| vues.get(&id).copied().unwrap_or(0) > 1)
        {
            l.sort = SortLigneReprise::Erreur;
            l.messages
                .push(message("import_reprises.deux_fois", None, None));
        }
    }
    Ok(sortie)
}

fn avertir(
    ligne: &mut LigneImportReprise,
    cle: &str,
    montant: Option<Montant>,
    autre: Option<Montant>,
) {
    ligne.messages.push(message(cle, montant, autre));
    ligne.sort = ligne.sort.max(SortLigneReprise::Avertissement);
}

fn refuser(
    ligne: &mut LigneImportReprise,
    cle: &str,
    montant: Option<Montant>,
    autre: Option<Montant>,
) {
    ligne.messages.push(message(cle, montant, autre));
    ligne.sort = SortLigneReprise::Erreur;
}

/// Retrouve l'élève et son inscription, lit le montant, et donne à la ligne son sort.
fn verifier_ligne(ligne: &mut LigneImportReprise, donnees: &Annee) {
    // Case vide : rien à faire pour cet élève.
    if ligne.saisi.is_empty() {
        ligne.sort = SortLigneReprise::Ignoree;
        return;
    }
    let inscrit = if ligne.matricule.is_empty() {
        let nom = (normaliser(&ligne.nom), normaliser(&ligne.prenoms));
        let classe = normaliser(&ligne.classe);
        let dans_la_classe = |i: &&Inscrit| classe.is_empty() || normaliser(&i.classe) == classe;
        let mut trouves: Vec<&Inscrit> = donnees
            .inscrits
            .iter()
            .filter(dans_la_classe)
            .filter(|i| (normaliser(&i.nom), normaliser(&i.prenoms)) == nom)
            .collect();
        if trouves.is_empty() {
            let tolerant = (tolerante(&ligne.nom), tolerante(&ligne.prenoms));
            trouves = donnees
                .inscrits
                .iter()
                .filter(dans_la_classe)
                .filter(|i| (tolerante(&i.nom), tolerante(&i.prenoms)) == tolerant)
                .collect();
        }
        match trouves.as_slice() {
            [] => {
                refuser(ligne, "import_reprises.introuvable", None, None);
                return;
            }
            [un] => {
                avertir(ligne, "import_reprises.retrouve_par_le_nom", None, None);
                *un
            }
            _ => {
                refuser(ligne, "import_reprises.plusieurs", None, None);
                return;
            }
        }
    } else {
        let matricule = normaliser(&ligne.matricule);
        // Un élève inscrit deux fois dans l'année : son inscription validée d'abord.
        match donnees
            .inscrits
            .iter()
            .filter(|i| i.matricules.contains(&matricule))
            .max_by_key(|i| donnees.lignes.contains_key(&i.inscription.id))
        {
            Some(i) => i,
            None => {
                refuser(ligne, "import_reprises.introuvable", None, None);
                return;
            }
        }
    };
    ligne.eleve = inscrit.eleve.clone();
    ligne.inscription_id = Some(inscrit.inscription.id);
    let Some(montant) = lire_montant(&ligne.saisi).map(Montant::depuis_francs) else {
        refuser(ligne, "import_reprises.montant_illisible", None, None);
        return;
    };
    ligne.montant = Some(montant);
    let Some(ecran) = donnees
        .lignes
        .get(&inscrit.inscription.id)
        .filter(|_| inscrit.inscription.statut != StatutInscription::Brouillon)
    else {
        refuser(ligne, "import_reprises.pas_validee", None, None);
        return;
    };
    let note_changee = !ligne.note.is_empty() && ligne.note != ecran.note;
    if montant == ecran.reprise && !note_changee {
        ligne.sort = SortLigneReprise::Ignoree;
        ligne
            .messages
            .push(message("import_reprises.inchange", None, None));
        return;
    }
    if montant.est_nul() && ecran.reprise.est_nul() {
        ligne.sort = SortLigneReprise::Ignoree;
        ligne
            .messages
            .push(message("import_reprises.inchange", None, None));
        return;
    }
    if montant < ecran.minimum {
        if ecran.rembourse >= ecran.minimum {
            refuser(
                ligne,
                "import_reprises.deja_rembourse",
                Some(ecran.rembourse),
                Some(ecran.minimum),
            );
        } else {
            refuser(
                ligne,
                "import_reprises.avoir_a_couvrir",
                Some(ecran.minimum),
                None,
            );
        }
        return;
    }
    if montant.est_nul() {
        avertir(ligne, "import_reprises.annule", Some(ecran.reprise), None);
        return;
    }
    if !ecran.reprise.est_nul() && montant != ecran.reprise {
        avertir(
            ligne,
            "import_reprises.remplace",
            Some(ecran.reprise),
            Some(montant),
        );
    }
    // Ce qui dépasse ce que l'élève doit encore (hors reprise) est à rendre au parent.
    let doit = (ecran.du.francs() - ecran.paye_gesco.francs()).max(0);
    let utilisable = (montant.francs() - ecran.rembourse.francs()).max(0);
    let excedent = utilisable - utilisable.min(doit);
    if excedent > 0 {
        avertir(
            ligne,
            "import_reprises.a_rembourser",
            Some(Montant::depuis_francs(excedent)),
            None,
        );
    }
}
