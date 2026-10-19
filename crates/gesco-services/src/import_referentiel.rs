//! Import des classes et des matières depuis Excel, sans toucher au programme : un classeur à
//! deux feuilles, « Classes » (niveau, série, classe) et « Matières » (matière, code, nom court,
//! groupe), chacune facultative ; un CSV porte l'une ou l'autre, reconnue à ses colonnes. Le
//! modèle part de ce que l'école a déjà. L'analyse dit ce qu'elle comprend de chaque ligne sans
//! rien écrire ; l'import crée les classes de l'année en cours et les matières qui manquent,
//! chacune comme une saisie à l'écran. Ce qui existe déjà est laissé tel quel.

use gesco_core::pedagogie::GroupeMatiere;
use gesco_core::pedagogie::matiere::verifier_matiere;
use gesco_core::scolarite::academique::libelle_valide;
use gesco_core::scolarite::{Niveau, Serie};
use gesco_db::depots::{academique, annee_scolaire, pedagogie};
use serde::{Deserialize, Serialize};

use crate::erreur::{ErreurService, Resultat};
use crate::export::{Feuille, FichierEcrit};
use crate::import_programme::{
    code_nouveau, comparable, libelle_court, trouver_matiere, trouver_niveau,
};
use crate::noyau::Noyau;
use crate::pedagogie::{DemandeMatiere, ServicePedagogie, series_de};
use crate::referentiel::ServiceReferentiel;
use crate::session::Session;

fn regle(cle: &'static str) -> ErreurService {
    ErreurService::Regle { cle }
}

/// Colonnes de la feuille des classes.
pub const COLONNES_CLASSES: [&str; 3] = ["Niveau", "Série", "Classe"];
/// Colonnes de la feuille des matières.
pub const COLONNES_MATIERES: [&str; 4] = ["Matière", "Code", "Nom court", "Groupe"];

/// Ce qu'une ligne crée.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, specta::Type)]
#[serde(rename_all = "snake_case")]
pub enum NatureLigneReferentiel {
    /// Une classe de l'année en cours.
    Classe,
    /// Une matière.
    Matiere,
}

/// Une ligne du fichier, telle que GESCO la comprend.
#[derive(Debug, Clone, Serialize, specta::Type)]
pub struct LigneImportReferentiel {
    /// Classe ou matière.
    pub nature: NatureLigneReferentiel,
    /// Feuille du classeur.
    pub feuille: String,
    /// Numéro de la ligne dans sa feuille, en-tête compris.
    pub numero: u32,
    /// Nom de la classe ou de la matière.
    pub nom: String,
    /// Niveau et série d'une classe ; code et groupe d'une matière.
    pub detail: String,
    /// Vrai si elle existe déjà : elle est laissée telle quelle.
    pub deja: bool,
    /// Clés des erreurs de la ligne ; vide si elle est prête.
    pub erreurs: Vec<String>,
}

/// Ce que l'analyse a compris du fichier.
#[derive(Debug, Clone, Serialize, specta::Type)]
pub struct AnalyseImportReferentiel {
    /// Les lignes, feuille par feuille.
    pub lignes: Vec<LigneImportReferentiel>,
    /// Classes qui seront créées.
    pub classes_a_creer: u32,
    /// Matières qui seront créées.
    pub matieres_a_creer: u32,
}

/// Bilan de l'import.
#[derive(Debug, Clone, Serialize, specta::Type)]
pub struct BilanImportReferentiel {
    /// Classes créées.
    pub classes: u32,
    /// Matières créées.
    pub matieres: u32,
    /// Lignes laissées de côté : déjà là, ou à corriger.
    pub ignorees: u32,
}

/// Fichier à lire.
#[derive(Debug, Deserialize, specta::Type)]
pub struct DemandeImportReferentiel {
    /// Chemin du classeur ou du CSV.
    pub chemin: String,
}

/// Modèle à écrire.
#[derive(Debug, Deserialize, specta::Type)]
pub struct DemandeModeleReferentiel {
    /// Chemin du classeur à écrire.
    pub chemin: String,
}

/// Groupe d'une matière d'après le texte saisi ; vide : « Autre ».
fn lire_groupe(texte: &str) -> Option<GroupeMatiere> {
    match comparable(texte).as_str() {
        "" | "autre" | "autres" => Some(GroupeMatiere::Autre),
        "litteraire" | "lettres" | "litteraires" => Some(GroupeMatiere::Litteraire),
        "scientifique" | "sciences" | "scientifiques" => Some(GroupeMatiere::Scientifique),
        _ => None,
    }
}

fn nom_groupe(groupe: GroupeMatiere) -> &'static str {
    match groupe {
        GroupeMatiere::Litteraire => "Littéraire",
        GroupeMatiere::Scientifique => "Scientifique",
        GroupeMatiere::Autre => "Autre",
    }
}

/// Une classe prête à créer.
struct ClasseLue {
    niveau: Niveau,
    serie: Option<Serie>,
    libelle: String,
}

/// Une matière prête à créer.
struct MatiereLue {
    code: String,
    libelle: String,
    libelle_court: String,
    groupe: GroupeMatiere,
}

struct Lue {
    ligne: LigneImportReferentiel,
    classe: Option<ClasseLue>,
    matiere: Option<MatiereLue>,
}

/// Service de l'import des classes et des matières.
pub struct ServiceImportReferentiel<'n> {
    noyau: &'n Noyau,
}

impl<'n> ServiceImportReferentiel<'n> {
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

    /// Le modèle, prérempli des classes de l'année en cours et des matières de l'école.
    pub async fn ecrire_modele(
        &self,
        session: &Session,
        demande: DemandeModeleReferentiel,
    ) -> Resultat<FichierEcrit> {
        let contexte = self.noyau.contexte(session.etablissement_id, Some(session));
        let mut cnx = self.connexion().await?;
        let niveaux = academique::niveaux(&mut cnx, &contexte).await?;
        let series = academique::series(&mut cnx, &contexte).await?;
        let mut classes = match annee_scolaire::courante(&mut cnx, &contexte).await? {
            Some(annee) => academique::classes(&mut cnx, annee.id, &contexte).await?,
            None => Vec::new(),
        };
        let matieres = pedagogie::matieres(&mut cnx, &contexte).await?;
        drop(cnx);
        let rang = |id| niveaux.iter().find(|n| n.id == id).map_or(0, |n| n.ordre);
        classes
            .sort_by(|a, b| (rang(a.niveau_id), &a.libelle).cmp(&(rang(b.niveau_id), &b.libelle)));
        let lignes_classes = classes
            .iter()
            .map(|c| {
                vec![
                    niveaux
                        .iter()
                        .find(|n| n.id == c.niveau_id)
                        .map(|n| n.libelle.clone())
                        .unwrap_or_default(),
                    c.serie_id
                        .and_then(|s| series.iter().find(|x| x.id == s))
                        .map(|s| s.code.clone())
                        .unwrap_or_default(),
                    c.libelle.clone(),
                ]
            })
            .collect();
        let lignes_matieres = matieres
            .iter()
            .filter(|m| m.active)
            .map(|m| {
                vec![
                    m.libelle.clone(),
                    m.code.clone(),
                    m.libelle_court.clone(),
                    nom_groupe(m.groupe).to_owned(),
                ]
            })
            .collect();
        crate::export::exporter_xlsx_feuilles(
            demande.chemin,
            "Classes et matières".to_owned(),
            vec![
                Feuille {
                    titre: "Classes".to_owned(),
                    entetes: COLONNES_CLASSES.iter().map(|c| (*c).to_owned()).collect(),
                    lignes: lignes_classes,
                },
                Feuille {
                    titre: "Matières".to_owned(),
                    entetes: COLONNES_MATIERES.iter().map(|c| (*c).to_owned()).collect(),
                    lignes: lignes_matieres,
                },
            ],
        )
        .await
    }

    async fn lire(&self, session: &Session, chemin: &str) -> Resultat<Vec<Lue>> {
        let contexte = self.noyau.contexte(session.etablissement_id, Some(session));
        let feuilles = crate::import_eleves::lire_fichier(chemin)?;
        let mut cnx = self.connexion().await?;
        let niveaux = academique::niveaux(&mut cnx, &contexte).await?;
        let series = academique::series(&mut cnx, &contexte).await?;
        let classes = match annee_scolaire::courante(&mut cnx, &contexte).await? {
            Some(annee) => academique::classes(&mut cnx, annee.id, &contexte).await?,
            None => Vec::new(),
        };
        let matieres = pedagogie::matieres(&mut cnx, &contexte).await?;
        drop(cnx);
        let mut lues: Vec<Lue> = Vec::new();
        let mut reconnue = false;
        for (nom_feuille, lignes) in &feuilles {
            let entete: Vec<String> = lignes
                .first()
                .map(|l| l.iter().map(|c| comparable(c)).collect())
                .unwrap_or_default();
            let colonne = |nom: &str| entete.iter().position(|e| *e == comparable(nom));
            let cellule = |l: &Vec<String>, c: Option<usize>| {
                c.and_then(|c| l.get(c))
                    .map(|t| t.trim().to_owned())
                    .unwrap_or_default()
            };
            if let (Some(c_niveau), Some(c_classe)) = (colonne("Niveau"), colonne("Classe")) {
                reconnue = true;
                let c_serie = colonne("Série");
                for (i, l) in lignes.iter().enumerate().skip(1) {
                    let (niveau_lu, nom) = (cellule(l, Some(c_niveau)), cellule(l, Some(c_classe)));
                    if niveau_lu.is_empty() && nom.is_empty() {
                        continue;
                    }
                    let serie_lue = cellule(l, c_serie);
                    let mut erreurs = Vec::new();
                    let niveau = trouver_niveau(&niveau_lu, &niveaux);
                    if niveau.is_none() {
                        erreurs.push("import_programme.niveau_inconnu".to_owned());
                    }
                    let mut serie = None;
                    if let Some(n) = niveau {
                        let attendues: Vec<Serie> =
                            series_de(n, &series).into_iter().flatten().collect();
                        if attendues.is_empty() {
                            if !serie_lue.is_empty() {
                                erreurs.push("import_programme.serie_inutile".to_owned());
                            }
                        } else {
                            let lue = comparable(&serie_lue);
                            serie = attendues
                                .iter()
                                .find(|s| {
                                    comparable(&s.code) == lue || comparable(&s.libelle) == lue
                                })
                                .cloned();
                            if serie.is_none() {
                                erreurs.push("import_programme.serie_inconnue".to_owned());
                            }
                        }
                        // Un niveau d'un cycle que l'école n'a pas ouvert ne reçoit pas de classe.
                        if !n.actif {
                            erreurs.push("classe.niveau_inactif".to_owned());
                        }
                    }
                    let libelle = match libelle_valide(&nom, "classe.libelle_invalide") {
                        Ok(l) => l,
                        Err(cle) => {
                            erreurs.push(cle.to_owned());
                            nom.clone()
                        }
                    };
                    let deja = classes
                        .iter()
                        .any(|c| comparable(&c.libelle) == comparable(&libelle));
                    if !deja
                        && lues.iter().any(|x| {
                            x.ligne.nature == NatureLigneReferentiel::Classe
                                && comparable(&x.ligne.nom) == comparable(&libelle)
                        })
                    {
                        erreurs.push("import_referentiel.en_double".to_owned());
                    }
                    let detail = match &serie {
                        Some(s) => format!("{niveau_lu} · {}", s.code),
                        None => niveau_lu.clone(),
                    };
                    let classe =
                        niveau
                            .filter(|_| erreurs.is_empty() && !deja)
                            .map(|n| ClasseLue {
                                niveau: n.clone(),
                                serie: serie.clone(),
                                libelle: libelle.clone(),
                            });
                    lues.push(Lue {
                        ligne: LigneImportReferentiel {
                            nature: NatureLigneReferentiel::Classe,
                            feuille: nom_feuille.clone(),
                            numero: u32::try_from(i + 1).unwrap_or(u32::MAX),
                            nom: libelle,
                            detail,
                            deja,
                            erreurs,
                        },
                        classe,
                        matiere: None,
                    });
                }
            } else if let (Some(c_matiere), None) = (colonne("Matière"), colonne("Niveau")) {
                reconnue = true;
                let c_code = colonne("Code");
                let c_court = colonne("Nom court").or_else(|| colonne("Libellé court"));
                let c_groupe = colonne("Groupe");
                for (i, l) in lignes.iter().enumerate().skip(1) {
                    let libelle = cellule(l, Some(c_matiere));
                    let code_lu = cellule(l, c_code).to_uppercase();
                    if libelle.is_empty() && code_lu.is_empty() {
                        continue;
                    }
                    let mut erreurs = Vec::new();
                    let existante = trouver_matiere(&libelle, &matieres).or_else(|| {
                        matieres
                            .iter()
                            .find(|m| !code_lu.is_empty() && m.code == code_lu)
                    });
                    let deja = existante.is_some();
                    let groupe = lire_groupe(&cellule(l, c_groupe));
                    if groupe.is_none() {
                        erreurs.push("import_referentiel.groupe_inconnu".to_owned());
                    }
                    // Code donné, ou tiré du nom ; il ne doit pas être celui d'une autre matière,
                    // ni d'une ligne précédente du fichier.
                    let pris: Vec<String> = matieres
                        .iter()
                        .map(|m| m.code.clone())
                        .chain(
                            lues.iter()
                                .filter_map(|x| x.matiere.as_ref().map(|m| m.code.clone())),
                        )
                        .collect();
                    let code = if code_lu.is_empty() {
                        code_nouveau(&libelle, &pris)
                    } else {
                        code_lu.clone()
                    };
                    let court = {
                        let c = cellule(l, c_court);
                        if c.is_empty() {
                            libelle_court(&libelle)
                        } else {
                            c
                        }
                    };
                    if !deja {
                        if let Err(cle) = verifier_matiere(&code, &libelle, &court) {
                            erreurs.push(cle.to_owned());
                        }
                        if !code_lu.is_empty() && pris.contains(&code_lu) {
                            erreurs.push("matiere.code_pris".to_owned());
                        }
                        if lues.iter().any(|x| {
                            x.ligne.nature == NatureLigneReferentiel::Matiere
                                && comparable(&x.ligne.nom) == comparable(&libelle)
                        }) {
                            erreurs.push("import_referentiel.en_double".to_owned());
                        }
                    }
                    let groupe = groupe.unwrap_or(GroupeMatiere::Autre);
                    let matiere = (!deja && erreurs.is_empty()).then(|| MatiereLue {
                        code: code.clone(),
                        libelle: libelle.trim().to_owned(),
                        libelle_court: court.clone(),
                        groupe,
                    });
                    lues.push(Lue {
                        ligne: LigneImportReferentiel {
                            nature: NatureLigneReferentiel::Matiere,
                            feuille: nom_feuille.clone(),
                            numero: u32::try_from(i + 1).unwrap_or(u32::MAX),
                            nom: existante.map_or_else(|| libelle.clone(), |m| m.libelle.clone()),
                            detail: format!(
                                "{} · {}",
                                existante.map_or(code, |m| m.code.clone()),
                                nom_groupe(existante.map_or(groupe, |m| m.groupe))
                            ),
                            deja,
                            erreurs,
                        },
                        classe: None,
                        matiere,
                    });
                }
            }
        }
        if !reconnue {
            return Err(regle("import_referentiel.colonnes"));
        }
        if lues.is_empty() {
            return Err(regle("import_programme.fichier_vide"));
        }
        Ok(lues)
    }

    /// Ce que GESCO comprend du fichier, sans rien enregistrer.
    pub async fn analyser(
        &self,
        session: &Session,
        demande: &DemandeImportReferentiel,
    ) -> Resultat<AnalyseImportReferentiel> {
        let lues = self.lire(session, &demande.chemin).await?;
        let compter = |f: fn(&Lue) -> bool| {
            u32::try_from(lues.iter().filter(|l| f(l)).count()).unwrap_or(u32::MAX)
        };
        Ok(AnalyseImportReferentiel {
            classes_a_creer: compter(|l| l.classe.is_some()),
            matieres_a_creer: compter(|l| l.matiere.is_some()),
            lignes: lues.into_iter().map(|l| l.ligne).collect(),
        })
    }

    /// Crée les matières puis les classes prêtes ; chacune s'enregistre et se trace comme une
    /// saisie à l'écran.
    pub async fn appliquer(
        &self,
        session: &Session,
        demande: &DemandeImportReferentiel,
    ) -> Resultat<BilanImportReferentiel> {
        let contexte = self.noyau.contexte(session.etablissement_id, Some(session));
        let lues = self.lire(session, &demande.chemin).await?;
        let pedagogie_service = ServicePedagogie::nouveau(self.noyau);
        let referentiel = ServiceReferentiel::nouveau(self.noyau);
        let (mut classes, mut matieres, mut ignorees) = (0u32, 0u32, 0u32);
        for l in &lues {
            let Some(m) = &l.matiere else { continue };
            pedagogie_service
                .enregistrer_matiere(
                    session,
                    &DemandeMatiere {
                        id: None,
                        code: m.code.clone(),
                        libelle: m.libelle.clone(),
                        libelle_court: m.libelle_court.clone(),
                        groupe: m.groupe,
                        active: true,
                    },
                )
                .await?;
            matieres += 1;
        }
        if lues.iter().any(|l| l.classe.is_some()) {
            let annee = {
                let mut cnx = self.connexion().await?;
                annee_scolaire::courante(&mut cnx, &contexte)
                    .await?
                    .ok_or(regle("import_programme.annee_absente"))?
            };
            for l in &lues {
                let Some(c) = &l.classe else { continue };
                referentiel
                    .creer_classe(
                        session,
                        annee.id,
                        c.niveau.id,
                        c.serie.as_ref().map(|s| s.id),
                        &c.libelle,
                    )
                    .await?;
                classes += 1;
            }
        }
        for l in &lues {
            if l.classe.is_none() && l.matiere.is_none() {
                ignorees += 1;
            }
        }
        Ok(BilanImportReferentiel {
            classes,
            matieres,
            ignorees,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn groupes_lus() {
        assert_eq!(lire_groupe(""), Some(GroupeMatiere::Autre));
        assert_eq!(lire_groupe("Littéraire"), Some(GroupeMatiere::Litteraire));
        assert_eq!(lire_groupe("sciences"), Some(GroupeMatiere::Scientifique));
        assert_eq!(lire_groupe("sport"), None);
    }
}
