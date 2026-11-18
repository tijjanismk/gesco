//! Répartition d'un versement de l'État par fichier (SPEC_PRISE_EN_CHARGE_ETAT §7.4), même façon
//! de faire que l'import des reprises : un modèle prérempli avec les élèves pris en charge de
//! l'année et une colonne « Montant versé » ; la lecture d'un fichier rempli (xlsx, xls, ods,
//! csv) ; un aperçu ligne par ligne avant toute écriture ; l'écriture des lignes valides et en
//! avertissement en une transaction, après une sauvegarde automatique (R-48), par le même chemin
//! que l'écran (`prise_en_charge::repartir_dans`, origine `import`).
//!
//! Le fichier donne la répartition **entière** du versement, comme une modification de la
//! répartition à l'écran (§7.3) : les parts qu'il donne remplacent les parts actives du
//! versement ; un élève absent du fichier, ou dont la case est vide, n'a plus de part de ce
//! versement. L'aperçu le dit et compte les parts qui seraient retirées. Le versement doit
//! exister : l'argent s'enregistre d'abord, la répartition se fait ensuite.

use std::collections::{HashMap, HashSet};

use gesco_core::commun::{Id, Montant};
use gesco_core::core::{AnneeScolaire, Operation, Permission, TypeEvenement};
use gesco_core::finance::prise_en_charge::{
    OriginePartEtat, PriseEnChargeEtat, StatutPriseEnCharge, StatutVersementEtat, VersementEtat,
};
use gesco_core::finance::reprise::lire_montant;
use gesco_core::scolarite::recherche::{cle_phonetique, normaliser};
use gesco_core::scolarite::{Classe, Eleve, Inscription, Personne, Serie};
use gesco_db::depots::{academique, inscriptions, personnes, prise_en_charge};
use gesco_db::nouvel_id;
use gesco_db::unite::Durabilite;
use gesco_docs::tableur::{Liste, classeur_feuilles};
use serde::{Deserialize, Serialize};

use crate::erreur::{ErreurService, Resultat};
use crate::export::FichierEcrit;
use crate::import_eleves::FichierImporte;
use crate::import_reprises::{MessageLigneReprise, SortLigneReprise};
use crate::noyau::Noyau;
use crate::prise_en_charge::{
    EleveARepartir, PartSaisie, lignes_versement, repartir_dans, repartition_proposee_dans,
};
use crate::session::Session;

fn regle(cle: &'static str) -> ErreurService {
    ErreurService::Regle { cle }
}

fn calcul(r: Result<Montant, gesco_core::commun::ErreurMontant>) -> Resultat<Montant> {
    r.map_err(|_| regle("facture.calcul_impossible"))
}

fn somme(mut montants: impl Iterator<Item = Montant>) -> Resultat<Montant> {
    calcul(montants.try_fold(Montant::ZERO, |s, m| s.ajouter(m)))
}

fn nombre(n: usize) -> u32 {
    u32::try_from(n).unwrap_or(u32::MAX)
}

/// Colonnes du modèle, dans l'ordre. « Montant versé » est exigée, avec le matricule, la
/// référence d'orientation, ou bien le nom et les prénoms.
pub const COLONNES_MODELE: [&str; 9] = [
    "Matricule",
    "Nom",
    "Prénoms",
    "Classe",
    "Série",
    "Référence",
    "Attendu",
    "Reste dû par l'État",
    COLONNE_MONTANT,
];

/// La colonne à remplir.
pub const COLONNE_MONTANT: &str = "Montant versé";

/// Mode d'emploi du modèle, une ligne par consigne.
const MODE_D_EMPLOI: [&str; 6] = [
    "Remplissez seulement la colonne « Montant versé » : ce que l'État a payé pour chaque élève dans ce versement, d'après l'avis de crédit ou la liste qui l'accompagne.",
    "Un montant par élève, en francs, sans décimales : 120000, 120 000, 120 000 F ou 120.000.",
    "Le fichier donne la répartition entière du versement : elle remplace celle déjà faite, et un élève dont la case est vide n'a pas de part de ce versement.",
    "Gardez le matricule : c'est lui qui retrouve l'élève ; à défaut, la référence d'orientation, puis le nom, les prénoms et la classe.",
    "Seuls les élèves cochés pris en charge par l'État reçoivent une part. L'attendu et le reste dû par l'État sont là pour lecture.",
    "Le total ne peut pas dépasser le montant reçu ; ce qui n'est pas réparti reste visible sur le versement, à répartir plus tard.",
];

/// Modèle à écrire, prérempli avec les élèves pris en charge de l'année.
#[derive(Debug, Clone, Deserialize, specta::Type)]
pub struct DemandeModeleRepartitionEtat {
    /// Chemin du classeur, extension `.xlsx` ; rendu par le dialogue du système.
    pub chemin: String,
    /// Année concernée, si aucun versement n'est donné ; absente, l'année courante.
    #[serde(default)]
    pub annee_id: Option<Id<AnneeScolaire>>,
    /// Versement dont la répartition actuelle remplit la colonne « Montant versé » ; absent, la
    /// colonne est vide.
    #[serde(default)]
    pub versement_id: Option<Id<VersementEtat>>,
}

/// Fichier rempli à analyser pour un versement.
#[derive(Debug, Clone, Deserialize, specta::Type)]
pub struct DemandeAnalyseRepartitionEtat {
    /// Chemin du fichier `.xlsx`, `.xls`, `.ods` ou `.csv`.
    pub chemin: String,
    /// Versement à répartir.
    pub versement_id: Id<VersementEtat>,
}

/// Import d'un fichier déjà analysé.
#[derive(Debug, Clone, Deserialize, specta::Type)]
pub struct DemandeImportRepartitionEtat {
    /// Fichier rendu par l'analyse.
    pub fichier_id: String,
    /// Versement à répartir, le même qu'à l'analyse.
    pub versement_id: Id<VersementEtat>,
    /// Instant de lecture du versement rendu par l'analyse : rien ne s'écrit si sa répartition a
    /// changé depuis.
    #[serde(default)]
    pub lue_le: Option<i64>,
}

/// Une ligne du fichier, telle que GESCO la comprend.
#[derive(Debug, Clone, Serialize, specta::Type)]
pub struct LigneImportRepartitionEtat {
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
    /// Référence d'orientation, telle qu'écrite.
    pub reference: String,
    /// Montant versé, tel qu'écrit.
    pub saisi: String,
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

/// Aperçu du fichier, avant toute écriture.
#[derive(Debug, Clone, Serialize, specta::Type)]
pub struct ApercuRepartitionEtat {
    /// Fichier déposé, à rendre à l'import.
    pub fichier_id: String,
    /// Nom du fichier.
    pub nom: String,
    /// Versement à répartir.
    pub versement_id: Id<VersementEtat>,
    /// Instant de lecture du versement, à rendre à l'import.
    pub lue_le: i64,
    /// Lignes, dans l'ordre du fichier.
    pub lignes: Vec<LigneImportRepartitionEtat>,
    /// Lignes valides.
    pub valides: u32,
    /// Lignes en avertissement (écrites).
    pub avertissements: u32,
    /// Lignes en erreur (laissées de côté).
    pub erreurs: u32,
    /// Lignes ignorées (case vide ou zéro).
    pub ignorees: u32,
    /// Montant reçu du versement.
    pub recu: Montant,
    /// Total des lignes qui seront écrites.
    pub reparti: Montant,
    /// Reçu moins réparti, s'il reste quelque chose.
    pub non_reparti: Montant,
    /// Réparti au-delà du reçu (refusé).
    pub depassement: Montant,
    /// Refus de l'import entier, s'il y en a un (`versement_etat.reparti_superieur`).
    pub refus: Option<String>,
    /// Élèves qui auront une part.
    pub nombre_eleves: u32,
    /// Parts actives du versement, que l'import remplace.
    pub parts_actuelles: u32,
    /// Total des parts actives du versement.
    pub montant_actuel: Montant,
    /// Élèves qui ont une part du versement et n'en auront plus.
    pub retirees: u32,
    /// Total des parts retirées.
    pub montant_retire: Montant,
}

/// Ce que l'import a fait.
#[derive(Debug, Clone, Serialize, specta::Type)]
pub struct RapportRepartitionEtat {
    /// Versement réparti.
    pub versement_id: Id<VersementEtat>,
    /// Faux si la répartition était déjà celle du fichier : rien n'a changé.
    pub change: bool,
    /// Élèves qui ont une part.
    pub nombre_eleves: u32,
    /// Total réparti.
    pub reparti: Montant,
    /// Reçu moins réparti.
    pub non_reparti: Montant,
    /// Lignes ignorées.
    pub ignorees: u32,
    /// Élèves dont la part a été retirée.
    pub retirees: u32,
    /// Lignes refusées, à corriger et réimporter.
    pub refusees: Vec<LigneImportRepartitionEtat>,
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
    serie: String,
    /// Référence d'orientation de la version courante, normalisée.
    reference: String,
    /// Version courante de la prise en charge, cochée ou décochée.
    version: Option<PriseEnChargeEtat>,
}

impl Inscrit {
    fn en_cours(&self) -> bool {
        self.version
            .as_ref()
            .is_some_and(|v| v.statut == StatutPriseEnCharge::EnCours)
    }
}

/// Données de l'année : le versement, les inscrits, la liste de répartition proposée.
struct Donnees {
    versement: Option<VersementEtat>,
    lue_le: i64,
    inscrits: Vec<Inscrit>,
    /// Élèves de la liste proposée (cochés, et ceux qui ont une part du versement), dans l'ordre
    /// des classes et des noms.
    proposes: Vec<EleveARepartir>,
}

impl Donnees {
    fn propose(&self, id: Id<Inscription>) -> Option<&EleveARepartir> {
        self.proposes.iter().find(|e| e.inscription_id == id)
    }

    fn inscrit(&self, id: Id<Inscription>) -> Option<&Inscrit> {
        self.inscrits.iter().find(|i| i.inscription.id == id)
    }
}

/// Colonnes repérées dans l'en-tête.
struct Colonnes {
    matricule: Option<usize>,
    nom: Option<usize>,
    prenoms: Option<usize>,
    classe: Option<usize>,
    reference: Option<usize>,
    montant: usize,
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
            reference: colonne("Référence"),
            montant: colonne(COLONNE_MONTANT)?,
        };
        let par_nom = colonnes.nom.is_some() && colonnes.prenoms.is_some();
        (colonnes.matricule.is_some() || colonnes.reference.is_some() || par_nom)
            .then_some(colonnes)
    }
}

fn message(cle: &str, montant: Option<Montant>, autre: Option<Montant>) -> MessageLigneReprise {
    MessageLigneReprise {
        cle: cle.to_owned(),
        montant,
        autre,
    }
}

fn compte(lignes: &[LigneImportRepartitionEtat], sort: SortLigneReprise) -> u32 {
    nombre(lignes.iter().filter(|l| l.sort == sort).count())
}

fn ecrite(l: &LigneImportRepartitionEtat) -> bool {
    matches!(
        l.sort,
        SortLigneReprise::Valide | SortLigneReprise::Avertissement
    )
}

/// Chiffres de l'aperçu et du rapport, d'après les lignes vérifiées.
struct Totaux {
    recu: Montant,
    reparti: Montant,
    non_reparti: Montant,
    depassement: Montant,
    nombre_eleves: u32,
    parts_actuelles: u32,
    montant_actuel: Montant,
    retirees: u32,
    montant_retire: Montant,
}

impl Totaux {
    fn depuis(lignes: &[LigneImportRepartitionEtat], donnees: &Donnees) -> Resultat<Self> {
        let recu = donnees
            .versement
            .as_ref()
            .map_or(Montant::ZERO, |v| v.montant);
        let reparti = somme(
            lignes
                .iter()
                .filter(|l| ecrite(l))
                .filter_map(|l| l.montant),
        )?;
        let ecrites: HashSet<Id<Inscription>> = lignes
            .iter()
            .filter(|l| ecrite(l))
            .filter_map(|l| l.inscription_id)
            .collect();
        let actuelles: Vec<&EleveARepartir> = donnees
            .proposes
            .iter()
            .filter(|e| !e.part.est_nul())
            .collect();
        let retirees: Vec<&&EleveARepartir> = actuelles
            .iter()
            .filter(|e| !ecrites.contains(&e.inscription_id))
            .collect();
        Ok(Self {
            recu,
            reparti,
            non_reparti: calcul(recu.soustraire(reparti))?.max(Montant::ZERO),
            depassement: calcul(reparti.soustraire(recu))?.max(Montant::ZERO),
            nombre_eleves: nombre(ecrites.len()),
            parts_actuelles: nombre(actuelles.len()),
            montant_actuel: somme(actuelles.iter().map(|e| e.part))?,
            retirees: nombre(retirees.len()),
            montant_retire: somme(retirees.iter().map(|e| e.part))?,
        })
    }

    /// Refus de l'import entier : réparti au-delà du reçu (§7.2).
    fn refus(&self) -> Option<&'static str> {
        (self.reparti > self.recu).then_some("versement_etat.reparti_superieur")
    }
}

/// Service de la répartition par fichier. Permission `treasury:manage`.
pub struct ServiceImportRepartitionEtat<'n> {
    noyau: &'n Noyau,
}

impl<'n> ServiceImportRepartitionEtat<'n> {
    /// Service sur le noyau ouvert.
    pub const fn nouveau(noyau: &'n Noyau) -> Self {
        Self { noyau }
    }

    /// Refuse la session qui n'a pas `treasury:manage` (la frontière le vérifie déjà).
    async fn exiger(&self, session: &Session) -> Resultat<()> {
        let contexte = self.noyau.contexte(session.etablissement_id, Some(session));
        let droits = self
            .noyau
            .droits_de(session.utilisateur_id, &contexte)
            .await?;
        if !droits.permissions.contains(&Permission::TREASURY_MANAGE) {
            return Err(ErreurService::permission(Permission::TREASURY_MANAGE));
        }
        Ok(())
    }

    /// Le versement, les inscrits de son année et la liste de répartition proposée.
    async fn donnees(
        &self,
        session: &Session,
        versement_id: Option<Id<VersementEtat>>,
        annee_id: Option<Id<AnneeScolaire>>,
    ) -> Resultat<Donnees> {
        let contexte = self.noyau.contexte(session.etablissement_id, Some(session));
        let mut cnx = self
            .noyau
            .base()
            .pool()
            .acquire()
            .await
            .map_err(gesco_db::ErreurBase::from)?;
        let proposee =
            repartition_proposee_dans(&mut cnx, versement_id, annee_id, &contexte).await?;
        let (versement, lue_le) = match versement_id {
            Some(id) => {
                let versement = prise_en_charge::versement(&mut cnx, id, &contexte)
                    .await?
                    .ok_or(regle("versement_etat.introuvable"))?;
                let lue_le = crate::fraicheur::instant(&mut cnx, &lignes_versement(id)).await?;
                (Some(versement), lue_le)
            }
            None => (None, 0),
        };
        let series: HashMap<Id<Serie>, String> = academique::series(&mut cnx, &contexte)
            .await?
            .into_iter()
            .map(|s| (s.id, s.libelle))
            .collect();
        let classes: HashMap<Id<Classe>, (String, String)> =
            academique::classes(&mut cnx, proposee.annee_id, &contexte)
                .await?
                .into_iter()
                .map(|c| {
                    let serie = c
                        .serie_id
                        .and_then(|s| series.get(&s).cloned())
                        .unwrap_or_default();
                    (c.id, (c.libelle, serie))
                })
                .collect();
        let eleves: HashMap<Id<Eleve>, (Eleve, Personne)> = personnes::eleves(&mut cnx, &contexte)
            .await?
            .into_iter()
            .map(|(e, p)| (e.id, (e, p)))
            .collect();
        let mut versions: HashMap<Id<Inscription>, PriseEnChargeEtat> =
            prise_en_charge::courantes_de_l_annee(&mut cnx, proposee.annee_id, &contexte)
                .await?
                .into_iter()
                .map(|v| (v.inscription_id, v))
                .collect();
        let inscrits = inscriptions::de_l_annee(&mut cnx, proposee.annee_id, &contexte)
            .await?
            .into_iter()
            .filter_map(|inscription| {
                let (eleve, personne) = eleves.get(&inscription.eleve_id)?;
                let mut matricules = vec![normaliser(&eleve.matricule_interne)];
                if let Some(national) = eleve.matricule_national.as_deref() {
                    matricules.push(normaliser(national));
                }
                matricules.retain(|m| !m.is_empty());
                let (classe, serie) = classes
                    .get(&inscription.classe_id)
                    .cloned()
                    .unwrap_or_default();
                let version = versions.remove(&inscription.id);
                Some(Inscrit {
                    eleve: personne.nom_complet(),
                    nom: personne.nom.clone(),
                    prenoms: personne.prenoms.clone(),
                    matricules,
                    classe,
                    serie,
                    reference: version
                        .as_ref()
                        .map(|v| normaliser(&v.reference))
                        .unwrap_or_default(),
                    version,
                    inscription,
                })
            })
            .collect();
        Ok(Donnees {
            versement,
            lue_le,
            inscrits,
            proposes: proposee.eleves,
        })
    }

    /// Écrit le modèle prérempli : une feuille des élèves pris en charge de l'année (et de ceux
    /// qui ont une part du versement donné), une feuille « Mode d'emploi ».
    pub async fn ecrire_modele(
        &self,
        session: &Session,
        demande: &DemandeModeleRepartitionEtat,
    ) -> Resultat<FichierEcrit> {
        self.exiger(session).await?;
        if !std::path::Path::new(&demande.chemin)
            .extension()
            .is_some_and(|e| e.eq_ignore_ascii_case("xlsx"))
        {
            return Err(regle("export.extension"));
        }
        let donnees = self
            .donnees(session, demande.versement_id, demande.annee_id)
            .await?;
        let lignes: Vec<Vec<String>> = donnees
            .proposes
            .iter()
            .map(|e| {
                let inscrit = donnees.inscrit(e.inscription_id);
                vec![
                    e.matricule.clone(),
                    inscrit.map(|i| i.nom.clone()).unwrap_or_default(),
                    inscrit
                        .map(|i| i.prenoms.clone())
                        .unwrap_or_else(|| e.eleve.clone()),
                    e.classe.clone(),
                    inscrit.map(|i| i.serie.clone()).unwrap_or_default(),
                    e.reference.clone(),
                    e.attendu.francs().to_string(),
                    e.reste_du_etat.francs().to_string(),
                    if e.part.est_nul() {
                        String::new()
                    } else {
                        e.part.francs().to_string()
                    },
                ]
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
                "Répartition du versement de l'État",
                &[
                    Liste {
                        titre: "Répartition",
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

    /// Ce que GESCO comprend du fichier, sans rien écrire (§7.4).
    pub async fn analyser(
        &self,
        session: &Session,
        fichier_id: &str,
        fichier: &FichierImporte,
        versement_id: Id<VersementEtat>,
    ) -> Resultat<ApercuRepartitionEtat> {
        self.exiger(session).await?;
        let donnees = self.donnees(session, Some(versement_id), None).await?;
        if donnees
            .versement
            .as_ref()
            .is_some_and(|v| v.statut == StatutVersementEtat::Annule)
        {
            return Err(regle("versement_etat.annule"));
        }
        let lignes = analyser_lignes(fichier, &donnees)?;
        let totaux = Totaux::depuis(&lignes, &donnees)?;
        Ok(ApercuRepartitionEtat {
            fichier_id: fichier_id.to_owned(),
            nom: fichier.nom.clone(),
            versement_id,
            lue_le: donnees.lue_le,
            valides: compte(&lignes, SortLigneReprise::Valide),
            avertissements: compte(&lignes, SortLigneReprise::Avertissement),
            erreurs: compte(&lignes, SortLigneReprise::Erreur),
            ignorees: compte(&lignes, SortLigneReprise::Ignoree),
            refus: totaux.refus().map(str::to_owned),
            recu: totaux.recu,
            reparti: totaux.reparti,
            non_reparti: totaux.non_reparti,
            depassement: totaux.depassement,
            nombre_eleves: totaux.nombre_eleves,
            parts_actuelles: totaux.parts_actuelles,
            montant_actuel: totaux.montant_actuel,
            retirees: totaux.retirees,
            montant_retire: totaux.montant_retire,
            lignes,
        })
    }

    /// Écrit la répartition des lignes valides et en avertissement en une transaction : elle
    /// remplace les parts actives du versement (origine `import`), les factures touchées se
    /// recalculent, puis un audit et `ImportRepartitionEtatApplique` pour le fichier. Refus du
    /// tout si le réparti dépasse le reçu. La sauvegarde automatique est prise avant, par la
    /// frontière (R-48).
    pub async fn importer(
        &self,
        session: &Session,
        fichier: &FichierImporte,
        demande: &DemandeImportRepartitionEtat,
    ) -> Resultat<RapportRepartitionEtat> {
        self.exiger(session).await?;
        let donnees = self
            .donnees(session, Some(demande.versement_id), None)
            .await?;
        let lignes = analyser_lignes(fichier, &donnees)?;
        let totaux = Totaux::depuis(&lignes, &donnees)?;
        if let Some(cle) = totaux.refus() {
            return Err(regle(cle));
        }
        let contexte = self.noyau.contexte(session.etablissement_id, Some(session));
        let mut unite = self
            .noyau
            .commencer(contexte.clone(), Durabilite::Critique)
            .await?;
        if let Some(lue_le) = demande.lue_le {
            crate::fraicheur::verifier(
                unite.connexion(),
                &lignes_versement(demande.versement_id),
                lue_le,
            )
            .await?;
        }
        let versement =
            prise_en_charge::versement(unite.connexion(), demande.versement_id, &contexte)
                .await?
                .ok_or(regle("versement_etat.introuvable"))?;
        let saisies: Vec<PartSaisie> = lignes
            .iter()
            .filter(|l| ecrite(l))
            .filter_map(|l| {
                Some(PartSaisie {
                    inscription_id: l.inscription_id?,
                    montant: l.montant?,
                })
            })
            .collect();
        let change = repartir_dans(
            &mut unite,
            &contexte,
            &versement,
            &saisies,
            OriginePartEtat::Import,
            "versement_etat.importer",
        )
        .await?;
        let mut ignorees = 0u32;
        let mut refusees = Vec::new();
        for ligne in lignes {
            match ligne.sort {
                SortLigneReprise::Ignoree => ignorees += 1,
                SortLigneReprise::Erreur => refusees.push(ligne),
                SortLigneReprise::Valide | SortLigneReprise::Avertissement => {}
            }
        }
        let rapport = RapportRepartitionEtat {
            versement_id: versement.id,
            change,
            nombre_eleves: totaux.nombre_eleves,
            reparti: totaux.reparti,
            non_reparti: totaux.non_reparti,
            ignorees,
            retirees: totaux.retirees,
            refusees,
        };
        let id = nouvel_id::<RapportRepartitionEtat>().vers_chaine();
        let resume = serde_json::json!({
            "fichier": fichier.nom,
            "versement_id": versement.id,
            "annee_id": versement.annee_id,
            "change": rapport.change,
            "nombre_eleves": rapport.nombre_eleves,
            "reparti": rapport.reparti,
            "non_reparti": rapport.non_reparti,
            "ignorees": rapport.ignorees,
            "retirees": rapport.retirees,
            "refusees": rapport.refusees.len(),
        });
        unite.auditer(
            "import_repartition_etat",
            &id,
            Operation::Creation,
            None,
            Some(resume.clone()),
            "versement_etat.importer",
        );
        unite.emettre(
            TypeEvenement::ImportRepartitionEtatApplique,
            "import_repartition_etat",
            &id,
            resume,
        );
        self.noyau.terminer(unite).await?;
        Ok(rapport)
    }
}

/// Feuille à lire : la première dont l'en-tête porte « Montant versé », sinon la première.
fn feuille(fichier: &FichierImporte) -> Option<&Vec<Vec<String>>> {
    fichier
        .feuilles
        .iter()
        .find(|(_, lignes)| {
            lignes.first().is_some_and(|e| {
                e.iter()
                    .any(|c| normaliser(c) == normaliser(COLONNE_MONTANT))
            })
        })
        .or_else(|| fichier.feuilles.first())
        .map(|(_, lignes)| lignes)
}

/// Lit et vérifie chaque ligne du fichier (§7.4).
fn analyser_lignes(
    fichier: &FichierImporte,
    donnees: &Donnees,
) -> Resultat<Vec<LigneImportRepartitionEtat>> {
    let lignes = feuille(fichier).ok_or(regle("import.fichier_vide"))?;
    let entete = lignes.first().ok_or(regle("import.fichier_vide"))?;
    let colonnes = Colonnes::depuis(entete).ok_or(regle("import_repartition_etat.colonnes"))?;
    let mut sortie: Vec<LigneImportRepartitionEtat> = Vec::new();
    for (i, l) in lignes.iter().enumerate().skip(1) {
        // Une ligne entièrement vide se saute sans rien dire.
        if l.iter().all(|c| c.trim().is_empty()) {
            continue;
        }
        let cellule = |c: Option<usize>| {
            c.and_then(|c| l.get(c))
                .map(|t| t.trim().to_owned())
                .unwrap_or_default()
        };
        let mut ligne = LigneImportRepartitionEtat {
            numero: u32::try_from(i + 1).unwrap_or(u32::MAX),
            matricule: cellule(colonnes.matricule),
            nom: cellule(colonnes.nom),
            prenoms: cellule(colonnes.prenoms),
            classe: cellule(colonnes.classe),
            reference: cellule(colonnes.reference),
            saisi: cellule(Some(colonnes.montant)),
            montant: None,
            eleve: String::new(),
            inscription_id: None,
            sort: SortLigneReprise::Valide,
            messages: Vec::new(),
        };
        verifier_ligne(&mut ligne, donnees);
        sortie.push(ligne);
    }
    // Un élève deux fois dans le fichier : erreur sur chacune de ses lignes qui donne un montant.
    let mut vues: HashMap<Id<Inscription>, usize> = HashMap::new();
    for l in sortie
        .iter()
        .filter(|l| l.sort != SortLigneReprise::Ignoree)
    {
        if let Some(id) = l.inscription_id {
            *vues.entry(id).or_default() += 1;
        }
    }
    for l in &mut sortie {
        if l.sort != SortLigneReprise::Ignoree
            && l.inscription_id
                .is_some_and(|id| vues.get(&id).copied().unwrap_or(0) > 1)
        {
            refuser(l, "import_repartition_etat.deux_fois", None, None);
        }
    }
    Ok(sortie)
}

fn avertir(
    ligne: &mut LigneImportRepartitionEtat,
    cle: &str,
    montant: Option<Montant>,
    autre: Option<Montant>,
) {
    ligne.messages.push(message(cle, montant, autre));
    ligne.sort = ligne.sort.max(SortLigneReprise::Avertissement);
}

fn refuser(
    ligne: &mut LigneImportRepartitionEtat,
    cle: &str,
    montant: Option<Montant>,
    autre: Option<Montant>,
) {
    ligne.messages.push(message(cle, montant, autre));
    ligne.sort = SortLigneReprise::Erreur;
}

/// Recherche de l'élève d'une ligne.
enum Trouve<'d> {
    Un(&'d Inscrit),
    ParLeNom(&'d Inscrit),
    Plusieurs,
    Aucun,
}

/// Retrouve l'élève : par le matricule, à défaut par la référence d'orientation, à défaut par
/// le nom, les prénoms et la classe.
fn retrouver<'d>(ligne: &LigneImportRepartitionEtat, donnees: &'d Donnees) -> Trouve<'d> {
    if !ligne.matricule.is_empty() {
        let matricule = normaliser(&ligne.matricule);
        // Un élève inscrit deux fois dans l'année : l'inscription qui a une prise en charge.
        if let Some(i) = donnees
            .inscrits
            .iter()
            .filter(|i| i.matricules.contains(&matricule))
            .max_by_key(|i| (i.en_cours(), i.version.is_some()))
        {
            return Trouve::Un(i);
        }
    }
    if !ligne.reference.is_empty() {
        let reference = normaliser(&ligne.reference);
        let trouves: Vec<&Inscrit> = donnees
            .inscrits
            .iter()
            .filter(|i| !i.reference.is_empty() && i.reference == reference)
            .collect();
        match trouves.as_slice() {
            [] => {}
            [un] => return Trouve::Un(un),
            _ => return Trouve::Plusieurs,
        }
    }
    if ligne.nom.is_empty() && ligne.prenoms.is_empty() {
        return Trouve::Aucun;
    }
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
        [] => Trouve::Aucun,
        [un] => Trouve::ParLeNom(un),
        _ => Trouve::Plusieurs,
    }
}

/// Retrouve l'élève et son inscription, lit le montant, et donne à la ligne son sort.
fn verifier_ligne(ligne: &mut LigneImportRepartitionEtat, donnees: &Donnees) {
    // Case vide : pas de part de ce versement pour cet élève.
    if ligne.saisi.is_empty() {
        ligne.sort = SortLigneReprise::Ignoree;
        return;
    }
    let inscrit = match retrouver(ligne, donnees) {
        Trouve::Un(i) => i,
        Trouve::ParLeNom(i) => {
            avertir(
                ligne,
                "import_repartition_etat.retrouve_par_le_nom",
                None,
                None,
            );
            i
        }
        Trouve::Plusieurs => {
            refuser(ligne, "import_repartition_etat.plusieurs", None, None);
            return;
        }
        Trouve::Aucun => {
            refuser(ligne, "import_repartition_etat.introuvable", None, None);
            return;
        }
    };
    ligne.eleve = inscrit.eleve.clone();
    ligne.inscription_id = Some(inscrit.inscription.id);
    let Some(montant) = lire_montant(&ligne.saisi).map(Montant::depuis_francs) else {
        refuser(
            ligne,
            "import_repartition_etat.montant_illisible",
            None,
            None,
        );
        return;
    };
    // Zéro : comme une case vide, pas de part.
    if montant.est_nul() {
        ligne.sort = SortLigneReprise::Ignoree;
        ligne
            .messages
            .push(message("import_repartition_etat.zero", None, None));
        return;
    }
    ligne.montant = Some(montant);
    let propose = donnees.propose(inscrit.inscription.id);
    if !inscrit.en_cours() {
        // Un élève décoché qui a déjà une part de ce versement la garde (l'État l'a payé pour
        // lui) ; un élève jamais coché, ou décoché sans part de ce versement, est refusé.
        let a_une_part = propose.is_some_and(|e| !e.part.est_nul());
        if inscrit.version.is_some() && a_une_part {
            avertir(ligne, "import_repartition_etat.decoche", None, None);
        } else {
            refuser(ligne, "import_repartition_etat.non_coche", None, None);
        }
        // Décoché, l'État ne doit plus rien pour lui : sa part couvre ce que doit le parent,
        // rien à comparer à un reste de l'État.
        return;
    }
    let reste = propose.map_or(Montant::ZERO, |e| e.reste_du_etat);
    if montant > reste {
        let en_plus = montant.soustraire(reste).unwrap_or(Montant::ZERO);
        avertir(
            ligne,
            "import_repartition_etat.plus_que_le_reste",
            Some(en_plus),
            Some(reste),
        );
    }
}
