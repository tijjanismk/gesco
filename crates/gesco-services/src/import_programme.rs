//! Import du programme depuis Excel : l'école remplit un classeur — niveau, série, matière,
//! coefficient, heures par semaine — et GESCO pose le programme de chaque niveau qu'il nomme. Le
//! modèle part du programme en vigueur de l'année, que l'école n'a qu'à corriger. L'analyse dit
//! ce qu'elle comprend de chaque ligne sans rien écrire ; l'import récrit le programme des seuls
//! niveaux du fichier, chacun comme une saisie à l'écran, et crée les matières qui manquent. Un
//! niveau dont une ligne est fausse est laissé tel quel, pour ne pas lui retirer de matière.

use std::collections::BTreeMap;

use gesco_core::commun::Id;
use gesco_core::pedagogie::{GroupeMatiere, Matiere, SaisieProgramme};
use gesco_core::scolarite::{Cycle, Niveau, Serie};
use gesco_db::depots::{academique, annee_scolaire, pedagogie};
use serde::{Deserialize, Serialize};

use crate::erreur::{ErreurService, Resultat};
use crate::export::{DemandeExport, FichierEcrit};
use crate::noyau::Noyau;
use crate::pedagogie::{DemandeMatiere, DemandeProgramme, ServicePedagogie, series_de};
use crate::session::Session;

fn regle(cle: &'static str) -> ErreurService {
    ErreurService::Regle { cle }
}

/// Colonnes du modèle, dans l'ordre.
pub const COLONNES_MODELE: [&str; 5] = [
    "Niveau",
    "Série",
    "Matière",
    "Coefficient",
    "Heures par semaine",
];

/// Une ligne du fichier, telle que GESCO la comprend.
#[derive(Debug, Clone, Serialize, specta::Type)]
pub struct LigneImportProgramme {
    /// Numéro de la ligne dans le fichier, en-tête compris.
    pub numero: u32,
    /// Niveau lu.
    pub niveau: String,
    /// Série lue.
    pub serie: String,
    /// Matière lue.
    pub matiere: String,
    /// Coefficient retenu.
    pub coefficient: Option<i64>,
    /// Minutes par semaine retenues.
    pub minutes_hebdo: Option<i64>,
    /// Vrai si la matière n'existe pas encore : l'import la crée.
    pub nouvelle_matiere: bool,
    /// Clés des erreurs de la ligne ; vide si elle est prête.
    pub erreurs: Vec<String>,
}

/// Ce que l'analyse a compris du fichier.
#[derive(Debug, Clone, Serialize, specta::Type)]
pub struct AnalyseImportProgramme {
    /// Les lignes, dans l'ordre du fichier.
    pub lignes: Vec<LigneImportProgramme>,
    /// Niveaux (et séries) dont le programme sera récrit.
    pub niveaux_prets: u32,
    /// Niveaux laissés tels quels à cause d'une ligne fausse.
    pub niveaux_ignores: u32,
    /// Matières qui seront créées.
    pub nouvelles_matieres: Vec<String>,
}

/// Bilan de l'import.
#[derive(Debug, Clone, Serialize, specta::Type)]
pub struct BilanImportProgramme {
    /// Niveaux dont le programme a été récrit.
    pub niveaux: u32,
    /// Matières créées.
    pub matieres_creees: u32,
    /// Niveaux laissés tels quels.
    pub niveaux_ignores: u32,
}

/// Fichier à lire, pour l'année en cours.
#[derive(Debug, Deserialize, specta::Type)]
pub struct DemandeImportProgramme {
    /// Chemin du classeur ou du CSV.
    pub chemin: String,
}

/// Modèle à écrire.
#[derive(Debug, Deserialize, specta::Type)]
pub struct DemandeModeleProgramme {
    /// Chemin du classeur à écrire.
    pub chemin: String,
}

/// Texte comparable : minuscules, sans accents ni espaces superflus.
fn comparable(texte: &str) -> String {
    texte
        .trim()
        .to_lowercase()
        .chars()
        .map(|c| match c {
            'é' | 'è' | 'ê' | 'ë' => 'e',
            'à' | 'â' => 'a',
            'î' | 'ï' => 'i',
            'ô' => 'o',
            'û' | 'ù' => 'u',
            'ç' => 'c',
            '’' => '\'',
            autre => autre,
        })
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

/// Le niveau d'un texte : son code (`7`, `CAP1`), son libellé (« 7e année »), ou une forme
/// courte (« 7e », « 7ème », « 1re », « CAP 1 »).
fn trouver_niveau<'a>(texte: &str, niveaux: &'a [Niveau]) -> Option<&'a Niveau> {
    let lu = comparable(texte);
    let compact: String = lu.chars().filter(|c| !c.is_whitespace()).collect();
    let chiffres: String = compact.chars().take_while(char::is_ascii_digit).collect();
    let suite = compact.get(chiffres.len()..).unwrap_or_default();
    let forme_courte = !chiffres.is_empty()
        && matches!(
            suite,
            "" | "e" | "eme" | "er" | "ere" | "re" | "eannee" | "emeannee" | "ereannee" | "reannee"
        );
    niveaux.iter().find(|n| {
        comparable(&n.code) == compact
            || comparable(&n.libelle) == lu
            || (forme_courte && n.code == chiffres)
    })
}

/// La matière d'un texte : son code, son libellé ou son libellé court.
fn trouver_matiere<'a>(texte: &str, matieres: &'a [Matiere]) -> Option<&'a Matiere> {
    let lu = comparable(texte);
    matieres.iter().find(|m| {
        comparable(&m.code) == lu
            || comparable(&m.libelle) == lu
            || comparable(&m.libelle_court) == lu
    })
}

/// Heures par semaine : « 3 », « 1,5 », « 1h30 », « 1 h 30 », « 90 min ». Rien si la cellule est
/// vide ; une erreur si elle est illisible.
fn lire_minutes(texte: &str) -> Result<Option<i64>, ()> {
    let lu = comparable(texte).replace(' ', "");
    if lu.is_empty() {
        return Ok(None);
    }
    if let Some(minutes) = lu.strip_suffix("min").or_else(|| lu.strip_suffix("mn")) {
        return minutes.parse::<i64>().map(Some).map_err(|_| ());
    }
    if let Some((heures, minutes)) = lu.split_once('h') {
        let h = heures.parse::<i64>().map_err(|_| ())?;
        let m = if minutes.is_empty() {
            0
        } else {
            minutes.parse::<i64>().map_err(|_| ())?
        };
        return Ok(Some(h * 60 + m));
    }
    let decimal: f64 = lu.replace(',', ".").parse().map_err(|_| ())?;
    if !decimal.is_finite() || decimal < 0.0 {
        return Err(());
    }
    // Arrondi à la minute : « 1,5 » vaut 90 minutes.
    Ok(Some((decimal * 60.0).round() as i64))
}

/// Code d'une matière nouvelle : les lettres de son libellé, sans accents, cinq au plus, suivies
/// d'un chiffre s'il est déjà pris.
fn code_nouveau(libelle: &str, pris: &[String]) -> String {
    let lettres: String = comparable(libelle)
        .chars()
        .filter(char::is_ascii_alphanumeric)
        .map(|c| c.to_ascii_uppercase())
        .take(5)
        .collect();
    let base = if lettres.is_empty() {
        "MAT".to_owned()
    } else {
        lettres
    };
    if !pris.contains(&base) {
        return base;
    }
    (2..)
        .map(|n| format!("{base}{n}"))
        .find(|c| !pris.contains(c))
        .unwrap_or(base)
}

/// Libellé court d'une matière nouvelle : son libellé, coupé à seize caractères.
fn libelle_court(libelle: &str) -> String {
    libelle.trim().chars().take(16).collect()
}

/// Un niveau, et sa série au lycée : ce dont un programme se récrit d'un coup.
type Groupe = (Id<Niveau>, Option<Id<Serie>>);

/// Une ligne lue, avec ce qu'elle désigne.
struct Lue {
    ligne: LigneImportProgramme,
    groupe: Option<Groupe>,
    matiere_id: Option<Id<Matiere>>,
}

/// Service de l'import du programme.
pub struct ServiceImportProgramme<'n> {
    noyau: &'n Noyau,
}

impl<'n> ServiceImportProgramme<'n> {
    /// Service sur le noyau.
    pub fn nouveau(noyau: &'n Noyau) -> Self {
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

    /// Le modèle, prérempli du programme en vigueur de l'année en cours.
    pub async fn ecrire_modele(
        &self,
        session: &Session,
        demande: DemandeModeleProgramme,
    ) -> Resultat<FichierEcrit> {
        let contexte = self.noyau.contexte(session.etablissement_id, Some(session));
        let mut cnx = self.connexion().await?;
        let annee = annee_scolaire::courante(&mut cnx, &contexte)
            .await?
            .ok_or(regle("import_programme.annee_absente"))?;
        let niveaux = academique::niveaux(&mut cnx, &contexte).await?;
        let series = academique::series(&mut cnx, &contexte).await?;
        let matieres = pedagogie::matieres(&mut cnx, &contexte).await?;
        let mut programme = pedagogie::programme(&mut cnx, annee.id, &contexte).await?;
        drop(cnx);
        // Les niveaux que l'école n'a pas activés ne chargent pas le modèle.
        programme.retain(|l| niveaux.iter().any(|n| n.id == l.niveau_id && n.actif));
        let rang = |id: Id<Niveau>| niveaux.iter().find(|n| n.id == id).map_or(0, |n| n.ordre);
        programme.sort_by_key(|l| {
            (
                rang(l.niveau_id),
                l.serie_id.map(|s| s.vers_chaine()),
                l.ordre,
            )
        });
        let lignes = programme
            .iter()
            .map(|l| {
                let niveau = niveaux
                    .iter()
                    .find(|n| n.id == l.niveau_id)
                    .map(|n| n.libelle.clone())
                    .unwrap_or_default();
                let serie = l
                    .serie_id
                    .and_then(|s| series.iter().find(|x| x.id == s))
                    .map(|s| s.code.clone())
                    .unwrap_or_default();
                let matiere = matieres
                    .iter()
                    .find(|m| m.id == l.matiere_id)
                    .map(|m| m.libelle.clone())
                    .unwrap_or_default();
                let heures = l.minutes_hebdo.map_or_else(String::new, |m| {
                    if m % 60 == 0 {
                        (m / 60).to_string()
                    } else {
                        format!("{}h{:02}", m / 60, m % 60)
                    }
                });
                vec![
                    niveau,
                    serie,
                    matiere,
                    l.coefficient.map(|c| c.to_string()).unwrap_or_default(),
                    heures,
                ]
            })
            .collect();
        crate::export::exporter_xlsx(DemandeExport {
            chemin: demande.chemin,
            titre: "Programme".to_owned(),
            entetes: COLONNES_MODELE.iter().map(|c| (*c).to_owned()).collect(),
            lignes,
        })
        .await
    }

    async fn lire(&self, session: &Session, chemin: &str) -> Resultat<Vec<Lue>> {
        let contexte = self.noyau.contexte(session.etablissement_id, Some(session));
        let lignes = crate::import_eleves::lire_fichier(chemin)?
            .into_iter()
            .next()
            .map(|(_, l)| l)
            .ok_or(regle("import_programme.fichier_vide"))?;
        let entete: Vec<String> = lignes
            .first()
            .map(|l| l.iter().map(|c| comparable(c)).collect())
            .unwrap_or_default();
        let colonne = |nom: &str| entete.iter().position(|e| *e == comparable(nom));
        let (Some(c_niveau), Some(c_matiere)) = (colonne("Niveau"), colonne("Matière")) else {
            return Err(regle("import_programme.colonnes"));
        };
        let c_serie = colonne("Série");
        let c_coefficient = colonne("Coefficient");
        let c_heures = colonne("Heures par semaine").or_else(|| colonne("Heures"));
        let mut cnx = self.connexion().await?;
        let niveaux = academique::niveaux(&mut cnx, &contexte).await?;
        let series = academique::series(&mut cnx, &contexte).await?;
        let matieres = pedagogie::matieres(&mut cnx, &contexte).await?;
        drop(cnx);
        let mut lues: Vec<Lue> = Vec::new();
        for (i, l) in lignes.iter().enumerate().skip(1) {
            let cellule = |c: Option<usize>| {
                c.and_then(|c| l.get(c))
                    .map(|t| t.trim().to_owned())
                    .unwrap_or_default()
            };
            let (niveau_lu, matiere_lue) = (cellule(Some(c_niveau)), cellule(Some(c_matiere)));
            if niveau_lu.is_empty() && matiere_lue.is_empty() {
                continue;
            }
            let serie_lue = cellule(c_serie);
            let mut erreurs = Vec::new();
            let niveau = trouver_niveau(&niveau_lu, &niveaux);
            if niveau.is_none() {
                erreurs.push("import_programme.niveau_inconnu".to_owned());
            }
            if niveau.is_some_and(|n| n.cycle == Cycle::Jardin) {
                erreurs.push("programme.jardin_sans_notes".to_owned());
            }
            // La série, au lycée : elle doit être une de celles du niveau.
            let mut serie_id = None;
            if let Some(n) = niveau {
                let attendues: Vec<Serie> = series_de(n, &series).into_iter().flatten().collect();
                if attendues.is_empty() {
                    if !serie_lue.is_empty() {
                        erreurs.push("import_programme.serie_inutile".to_owned());
                    }
                } else {
                    let lue = comparable(&serie_lue);
                    match attendues
                        .iter()
                        .find(|s| comparable(&s.code) == lue || comparable(&s.libelle) == lue)
                    {
                        Some(s) => serie_id = Some(s.id),
                        None => erreurs.push("import_programme.serie_inconnue".to_owned()),
                    }
                }
            }
            let matiere = trouver_matiere(&matiere_lue, &matieres);
            if matiere_lue.is_empty() {
                erreurs.push("import_programme.matiere_vide".to_owned());
            } else if matiere.is_none() && matiere_lue.chars().count() > 60 {
                erreurs.push("matiere.libelle_invalide".to_owned());
            }
            let texte_coefficient = cellule(c_coefficient);
            let coefficient = if texte_coefficient.is_empty() {
                None
            } else {
                match texte_coefficient.replace(',', ".").parse::<f64>() {
                    Ok(c) if c.fract() == 0.0 && (1.0..=20.0).contains(&c) => Some(c as i64),
                    _ => {
                        erreurs.push("import_programme.coefficient_illisible".to_owned());
                        None
                    }
                }
            };
            if coefficient.is_some() && niveau.is_some_and(|n| n.cycle == Cycle::PremierCycle) {
                erreurs.push("import_programme.coefficient_premier_cycle".to_owned());
            }
            let minutes_hebdo = match lire_minutes(&cellule(c_heures)) {
                Ok(Some(m)) if !(0..=50 * 60).contains(&m) => {
                    erreurs.push("import_programme.heures_illisibles".to_owned());
                    None
                }
                Ok(m) => m,
                Err(()) => {
                    erreurs.push("import_programme.heures_illisibles".to_owned());
                    None
                }
            };
            let groupe = niveau.map(|n| (n.id, serie_id));
            // Une matière deux fois au même niveau.
            let cle_matiere = matiere.map_or_else(|| comparable(&matiere_lue), |m| m.code.clone());
            if groupe.is_some()
                && lues.iter().any(|x| {
                    x.groupe == groupe
                        && x.matiere_id.map_or_else(
                            || comparable(&x.ligne.matiere),
                            |id| {
                                matieres
                                    .iter()
                                    .find(|m| m.id == id)
                                    .map(|m| m.code.clone())
                                    .unwrap_or_default()
                            },
                        ) == cle_matiere
                })
            {
                erreurs.push("import_programme.matiere_en_double".to_owned());
            }
            lues.push(Lue {
                ligne: LigneImportProgramme {
                    numero: u32::try_from(i + 1).unwrap_or(u32::MAX),
                    niveau: niveau_lu,
                    serie: serie_lue,
                    matiere: matiere.map_or(matiere_lue, |m| m.libelle.clone()),
                    coefficient,
                    minutes_hebdo,
                    nouvelle_matiere: matiere.is_none(),
                    erreurs,
                },
                groupe,
                matiere_id: matiere.map(|m| m.id),
            });
        }
        if lues.is_empty() {
            return Err(regle("import_programme.fichier_vide"));
        }
        Ok(lues)
    }

    /// Niveaux prêts et niveaux à laisser : un niveau est prêt si toutes ses lignes le sont.
    fn groupes(lues: &[Lue]) -> (Vec<Groupe>, u32) {
        let mut etats: BTreeMap<String, (Groupe, bool)> = BTreeMap::new();
        let mut sans_niveau = false;
        for l in lues {
            match l.groupe {
                Some(g) => {
                    let cle = format!(
                        "{}{}",
                        g.0.vers_chaine(),
                        g.1.map(|s| s.vers_chaine()).unwrap_or_default()
                    );
                    let pret = l.ligne.erreurs.is_empty();
                    etats
                        .entry(cle)
                        .and_modify(|e| e.1 &= pret)
                        .or_insert((g, pret));
                }
                None => sans_niveau = true,
            }
        }
        let prets: Vec<_> = etats.values().filter(|e| e.1).map(|e| e.0).collect();
        let ignores = etats.values().filter(|e| !e.1).count() + usize::from(sans_niveau);
        (prets, u32::try_from(ignores).unwrap_or(u32::MAX))
    }

    /// Ce que GESCO comprend du fichier, sans rien enregistrer.
    pub async fn analyser(
        &self,
        session: &Session,
        demande: &DemandeImportProgramme,
    ) -> Resultat<AnalyseImportProgramme> {
        let lues = self.lire(session, &demande.chemin).await?;
        let (prets, niveaux_ignores) = Self::groupes(&lues);
        let mut nouvelles_matieres: Vec<String> = Vec::new();
        for l in &lues {
            if l.ligne.nouvelle_matiere
                && l.ligne.erreurs.is_empty()
                && l.groupe.is_some_and(|g| prets.contains(&g))
                && !nouvelles_matieres.contains(&l.ligne.matiere)
            {
                nouvelles_matieres.push(l.ligne.matiere.clone());
            }
        }
        Ok(AnalyseImportProgramme {
            niveaux_prets: u32::try_from(prets.len()).unwrap_or(u32::MAX),
            niveaux_ignores,
            nouvelles_matieres,
            lignes: lues.into_iter().map(|l| l.ligne).collect(),
        })
    }

    /// Crée les matières qui manquent, puis récrit le programme de chaque niveau prêt de l'année
    /// en cours ; chaque niveau s'enregistre et se trace comme une saisie à l'écran.
    pub async fn appliquer(
        &self,
        session: &Session,
        demande: &DemandeImportProgramme,
    ) -> Resultat<BilanImportProgramme> {
        let contexte = self.noyau.contexte(session.etablissement_id, Some(session));
        let annee = {
            let mut cnx = self.connexion().await?;
            annee_scolaire::courante(&mut cnx, &contexte)
                .await?
                .ok_or(regle("import_programme.annee_absente"))?
        };
        let mut lues = self.lire(session, &demande.chemin).await?;
        let (prets, niveaux_ignores) = Self::groupes(&lues);
        let pedagogie_service = ServicePedagogie::nouveau(self.noyau);
        let mut pris: Vec<String> = {
            let mut cnx = self.connexion().await?;
            pedagogie::matieres(&mut cnx, &contexte)
                .await?
                .into_iter()
                .map(|m| m.code)
                .collect()
        };
        let mut creees: Vec<(String, Id<Matiere>)> = Vec::new();
        for l in lues
            .iter_mut()
            .filter(|l| l.matiere_id.is_none() && l.groupe.is_some_and(|g| prets.contains(&g)))
        {
            let lu = comparable(&l.ligne.matiere);
            if let Some((_, id)) = creees.iter().find(|(c, _)| *c == lu) {
                l.matiere_id = Some(*id);
                continue;
            }
            let code = code_nouveau(&l.ligne.matiere, &pris);
            let matiere = pedagogie_service
                .enregistrer_matiere(
                    session,
                    &DemandeMatiere {
                        id: None,
                        code: code.clone(),
                        libelle: l.ligne.matiere.trim().to_owned(),
                        libelle_court: libelle_court(&l.ligne.matiere),
                        groupe: GroupeMatiere::Autre,
                        active: true,
                    },
                )
                .await?;
            pris.push(code);
            creees.push((lu, matiere.id));
            l.matiere_id = Some(matiere.id);
        }
        for (niveau_id, serie_id) in &prets {
            let lignes: Vec<SaisieProgramme> = lues
                .iter()
                .filter(|l| l.groupe == Some((*niveau_id, *serie_id)))
                .filter_map(|l| {
                    Some(SaisieProgramme {
                        matiere_id: l.matiere_id?,
                        coefficient: l.ligne.coefficient,
                        minutes_hebdo: l.ligne.minutes_hebdo,
                    })
                })
                .collect();
            pedagogie_service
                .enregistrer_programme(
                    session,
                    &DemandeProgramme {
                        annee_id: annee.id,
                        niveau_id: *niveau_id,
                        serie_id: *serie_id,
                        lignes,
                    },
                )
                .await?;
        }
        Ok(BilanImportProgramme {
            niveaux: u32::try_from(prets.len()).unwrap_or(u32::MAX),
            matieres_creees: u32::try_from(creees.len()).unwrap_or(u32::MAX),
            niveaux_ignores,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn heures_lues_sous_toutes_leurs_formes() {
        assert_eq!(lire_minutes(""), Ok(None));
        assert_eq!(lire_minutes("3"), Ok(Some(180)));
        assert_eq!(lire_minutes("1,5"), Ok(Some(90)));
        assert_eq!(lire_minutes("1h30"), Ok(Some(90)));
        assert_eq!(lire_minutes("1 h 30"), Ok(Some(90)));
        assert_eq!(lire_minutes("2h"), Ok(Some(120)));
        assert_eq!(lire_minutes("45 min"), Ok(Some(45)));
        assert_eq!(lire_minutes("beaucoup"), Err(()));
    }

    #[test]
    fn codes_des_matieres_nouvelles() {
        let pris = vec!["INFOR".to_owned()];
        assert_eq!(code_nouveau("Bambara", &pris), "BAMBA");
        assert_eq!(code_nouveau("Informatique", &pris), "INFOR2");
        assert_eq!(code_nouveau("Éducation islamique", &[]), "EDUCA");
        assert_eq!(
            libelle_court("Éducation artistique et culturelle"),
            "Éducation artist"
        );
    }
}
