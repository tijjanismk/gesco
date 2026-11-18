//! Avancement des élèves au portail et suivi par leurs maîtres (SPEC_PHASE_12_LOT_4 §6).
//!
//! L'élève entré avec son code ouvre une leçon, puis appuie sur « J'ai fini cette leçon » :
//! ces gestes vont dans une **file en mémoire** ([`FileAvancement`]) que le poste principal
//! écrit d'un coup toutes les cinq secondes (ADR « charge du portail des élèves ») ; une classe
//! entière qui lit ne prend pas la base à chaque page. Ce qui attend encore dans la file se
//! compte déjà dans « Mon avancement » et dans le suivi du maître : les chiffres restent justes.
//!
//! Le suivi ne classe pas, ne relance pas, ne note pas (§9.5) : des nombres par leçon et, quand
//! la direction le permet, les leçons finies de chaque élève dans l'ordre alphabétique.

use std::collections::{HashMap, HashSet};
use std::sync::Mutex;

use gesco_core::bibliotheque::NatureContenu;
use gesco_core::bibliotheque::portail::GrainSuivi;
use gesco_core::core::ModuleEcole;
use gesco_db::depots::portail::{self as depot, AvancementAEcrire, LigneAvancement};
use gesco_db::nouvel_id;
use serde::{Deserialize, Serialize};

use super::entree::SessionEleve;
use super::lecture::{CatalogueEleve, LecturePortail};
use crate::erreur::{ErreurService, Resultat};
use crate::noyau::Noyau;
use crate::session::Session;

/// Dernières leçons ouvertes montrées à l'élève, au plus.
pub const RECENTES_MAX: usize = 5;

/// Gestes en attente au plus (ADR « charge du portail des élèves ») : au-delà, un geste nouveau
/// est oublié sans rien dire à l'élève ; ceux qui attendent déjà se fondent toujours.
pub const FILE_AVANCEMENT_MAX: usize = 10_000;

/// Geste de l'élève sur une leçon.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum GesteAvancement {
    /// La leçon s'est ouverte.
    Ouvert,
    /// « J'ai fini cette leçon ».
    Fini,
}

/// Ce que le téléphone envoie à `POST /eleve/api/avancement`.
#[derive(Debug, Clone, Deserialize)]
pub struct DemandeAvancement {
    /// Contenu lu.
    pub contenu_id: String,
    /// Empreinte de la version lue, telle que le contenu l'a donnée.
    pub version: String,
    /// Ouvert ou fini.
    pub geste: GesteAvancement,
}

/// Un geste qui attend d'être écrit.
#[derive(Debug, Clone, PartialEq, Eq)]
struct EnAttente {
    version: String,
    ouvert_le: i64,
    dernier_le: i64,
    fini_le: Option<i64>,
}

/// Clé d'un geste : élève, année, contenu.
type CleGeste = (String, String, String);

/// File des gestes d'avancement, vidée toutes les cinq secondes. Deux gestes sur la même leçon
/// se fondent : la première ouverture reste, la dernière avance, « fini » ne s'efface pas.
#[derive(Debug, Default)]
pub struct FileAvancement {
    gestes: Mutex<HashMap<CleGeste, EnAttente>>,
}

impl FileAvancement {
    /// Note un geste.
    pub fn noter(
        &self,
        (eleve_id, annee_id, contenu_id): (&str, &str, &str),
        version: &str,
        geste: GesteAvancement,
        maintenant: i64,
    ) {
        let Ok(mut gestes) = self.gestes.lock() else {
            return;
        };
        let fini = (geste == GesteAvancement::Fini).then_some(maintenant);
        let cle = (
            eleve_id.to_owned(),
            annee_id.to_owned(),
            contenu_id.to_owned(),
        );
        if gestes.len() >= FILE_AVANCEMENT_MAX && !gestes.contains_key(&cle) {
            return;
        }
        gestes
            .entry(cle)
            .and_modify(|e| {
                version.clone_into(&mut e.version);
                e.dernier_le = e.dernier_le.max(maintenant);
                e.fini_le = e.fini_le.or(fini);
            })
            .or_insert_with(|| EnAttente {
                version: version.to_owned(),
                ouvert_le: maintenant,
                dernier_le: maintenant,
                fini_le: fini,
            });
    }

    /// Prend tout ce qui attend, prêt à écrire ; la file se vide.
    pub fn prendre(&self) -> Vec<AvancementAEcrire> {
        let Ok(mut gestes) = self.gestes.lock() else {
            return Vec::new();
        };
        gestes
            .drain()
            .map(|((eleve_id, annee_id, contenu_id), e)| AvancementAEcrire {
                id: nouvel_id::<()>().vers_chaine(),
                eleve_id,
                annee_id,
                contenu_id,
                version: e.version,
                ouvert_le: e.ouvert_le,
                dernier_le: e.dernier_le,
                fini_le: e.fini_le,
            })
            .collect()
    }

    /// Remet dans la file des gestes qui n'ont pas pu s'écrire ; ils repartent au passage
    /// suivant, fondus avec ce qui est arrivé entre-temps.
    pub fn remettre(&self, gestes: Vec<AvancementAEcrire>) {
        let Ok(mut file) = self.gestes.lock() else {
            return;
        };
        for g in gestes {
            file.entry((g.eleve_id, g.annee_id, g.contenu_id))
                .and_modify(|e| {
                    e.ouvert_le = e.ouvert_le.min(g.ouvert_le);
                    e.dernier_le = e.dernier_le.max(g.dernier_le);
                    e.fini_le = match (e.fini_le, g.fini_le) {
                        (Some(a), Some(b)) => Some(a.min(b)),
                        (a, b) => a.or(b),
                    };
                })
                .or_insert(EnAttente {
                    version: g.version,
                    ouvert_le: g.ouvert_le,
                    dernier_le: g.dernier_le,
                    fini_le: g.fini_le,
                });
        }
    }

    /// Gestes en attente pour une année, comme des lignes d'avancement ; `eleves` borne aux
    /// élèves voulus.
    pub fn en_attente(
        &self,
        annee_id: &str,
        eleves: &dyn Fn(&str) -> bool,
    ) -> Vec<LigneAvancement> {
        let Ok(file) = self.gestes.lock() else {
            return Vec::new();
        };
        file.iter()
            .filter(|((eleve, annee, _), _)| annee == annee_id && eleves(eleve))
            .map(|((eleve, _, contenu), e)| LigneAvancement {
                eleve_id: eleve.clone(),
                contenu_id: contenu.clone(),
                ouvert_le: e.ouvert_le,
                dernier_le: e.dernier_le,
                fini_le: e.fini_le,
            })
            .collect()
    }

    /// Nombre de gestes en attente.
    pub fn longueur(&self) -> usize {
        self.gestes.lock().map(|g| g.len()).unwrap_or(0)
    }
}

/// Fond les lignes écrites et celles qui attendent : une ligne par élève et contenu.
pub fn fondre(
    ecrites: Vec<LigneAvancement>,
    attente: Vec<LigneAvancement>,
) -> Vec<LigneAvancement> {
    let mut par_cle: HashMap<(String, String), LigneAvancement> = HashMap::new();
    for l in ecrites.into_iter().chain(attente) {
        par_cle
            .entry((l.eleve_id.clone(), l.contenu_id.clone()))
            .and_modify(|e| {
                e.ouvert_le = e.ouvert_le.min(l.ouvert_le);
                e.dernier_le = e.dernier_le.max(l.dernier_le);
                e.fini_le = e.fini_le.or(l.fini_le);
            })
            .or_insert(l);
    }
    let mut lignes: Vec<LigneAvancement> = par_cle.into_values().collect();
    lignes.sort_by(|a, b| {
        b.dernier_le
            .cmp(&a.dernier_le)
            .then_with(|| a.contenu_id.cmp(&b.contenu_id))
    });
    lignes
}

// ---------------------------------------------------------------------------------------------
// Ce que voit l'élève
// ---------------------------------------------------------------------------------------------

/// Une matière dans « Mon avancement ».
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct MatiereAvancement {
    /// Clé de la matière, comme au catalogue.
    pub cle: String,
    /// Libellé.
    pub libelle: String,
    /// Leçons finies, parmi les visibles.
    pub finies: u32,
    /// Leçons visibles.
    pub visibles: u32,
}

/// Une leçon ouverte récemment.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct LeconRecente {
    /// Contenu.
    pub id: String,
    /// Titre.
    pub titre: String,
    /// Libellé de sa matière.
    pub matiere: String,
}

/// « Mon avancement » : par matière, leçons finies sur leçons visibles ; dernières leçons
/// ouvertes, la plus récente d'abord ; contenus finis.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct MonAvancement {
    /// Par matière, dans l'ordre du catalogue.
    pub matieres: Vec<MatiereAvancement>,
    /// Dernières leçons ouvertes.
    pub recentes: Vec<LeconRecente>,
    /// Contenus finis, parmi ceux que l'élève voit.
    pub finies: Vec<String>,
}

/// « Mon avancement » d'après le catalogue de l'élève (ce qu'il voit seulement) et ses lignes.
/// Un contenu qu'il ne voit plus (retiré) ne compte pas : les nombres restent justes.
pub fn mon_avancement(catalogue: &CatalogueEleve, lignes: &[LigneAvancement]) -> MonAvancement {
    let finis: HashSet<&str> = lignes
        .iter()
        .filter(|l| l.fini_le.is_some())
        .map(|l| l.contenu_id.as_str())
        .collect();
    let mut places: HashMap<&str, (&str, &str)> = HashMap::new();
    let mut matieres = Vec::new();
    let mut finies = Vec::new();
    for m in &catalogue.matieres {
        let mut finies_ici = 0_u32;
        for e in m
            .parties
            .iter()
            .flat_map(|p| p.chapitres.iter())
            .flat_map(|c| c.contenus.iter())
        {
            places.insert(e.id.as_str(), (e.titre.as_str(), m.libelle.as_str()));
            if finis.contains(e.id.as_str()) {
                finies.push(e.id.clone());
                if e.nature == NatureContenu::Lecon {
                    finies_ici = finies_ici.saturating_add(1);
                }
            }
        }
        matieres.push(MatiereAvancement {
            cle: m.cle.clone(),
            libelle: m.libelle.clone(),
            finies: finies_ici,
            visibles: m.lecons,
        });
    }
    let mut ouvertes: Vec<&LigneAvancement> = lignes.iter().collect();
    ouvertes.sort_by_key(|l| std::cmp::Reverse(l.dernier_le));
    let recentes = ouvertes
        .into_iter()
        .filter_map(|l| {
            places
                .get(l.contenu_id.as_str())
                .map(|(titre, matiere)| LeconRecente {
                    id: l.contenu_id.clone(),
                    titre: (*titre).to_owned(),
                    matiere: (*matiere).to_owned(),
                })
        })
        .take(RECENTES_MAX)
        .collect();
    MonAvancement {
        matieres,
        recentes,
        finies,
    }
}

// ---------------------------------------------------------------------------------------------
// Ce que voient le maître et la direction
// ---------------------------------------------------------------------------------------------

/// Ce que demande l'écran « Suivi des élèves » : une classe et une matière ; sans elles, la
/// première de la liste.
#[derive(Debug, Clone, Default, Serialize, Deserialize, specta::Type)]
pub struct DemandeSuivi {
    /// Classe voulue.
    #[serde(default)]
    pub classe_id: Option<String>,
    /// Matière voulue (clé du catalogue).
    #[serde(default)]
    pub matiere_cle: Option<String>,
}

/// Une classe que ce compte peut suivre.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
pub struct ClasseASuivre {
    /// Classe.
    pub id: String,
    /// Libellé.
    pub libelle: String,
}

/// Une matière de la classe que ce compte peut suivre.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
pub struct MatiereSuivie {
    /// Clé du catalogue.
    pub cle: String,
    /// Libellé.
    pub libelle: String,
}

/// Une leçon dans le suivi : combien d'élèves de la classe l'ont ouverte et finie.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
pub struct LeconSuivie {
    /// Contenu.
    pub id: String,
    /// Titre.
    pub titre: String,
    /// Titre de son chapitre.
    pub chapitre: String,
    /// Élèves qui l'ont ouverte.
    pub ouvertes: u32,
    /// Élèves qui l'ont finie.
    pub finies: u32,
}

/// Un élève dans le suivi (grain « par élève » seulement).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
pub struct EleveSuivi {
    /// Élève.
    pub eleve_id: String,
    /// Nom et prénoms.
    pub nom: String,
    /// Leçons de la matière ouvertes.
    pub ouvertes: u32,
    /// Leçons de la matière finies.
    pub finies: u32,
    /// Dernière visite au portail (millisecondes), toutes matières.
    pub derniere_visite: Option<i64>,
}

/// Le suivi d'une classe dans une matière.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
pub struct SuiviClasse {
    /// Le suivi est allumé (et l'e-learning aussi) ; éteint, rien n'est montré.
    pub actif: bool,
    /// `eleve` ou `classe`.
    pub grain: String,
    /// Classes que ce compte peut suivre.
    pub classes: Vec<ClasseASuivre>,
    /// Classe montrée.
    pub classe_id: Option<String>,
    /// Libellé de la classe montrée.
    pub classe: String,
    /// Élèves inscrits dans la classe.
    pub eleves: u32,
    /// Matières de la classe que ce compte peut suivre.
    pub matieres: Vec<MatiereSuivie>,
    /// Matière montrée.
    pub matiere_cle: Option<String>,
    /// Leçons de la matière, dans l'ordre du programme.
    pub lecons: Vec<LeconSuivie>,
    /// Élèves, dans l'ordre alphabétique ; vide au grain « par classe ».
    pub par_eleve: Vec<EleveSuivi>,
}

/// Le suivi demandé, et ce qu'il faut pour l'imprimer.
pub struct SuiviLu {
    /// Ce que l'écran montre.
    pub suivi: SuiviClasse,
    /// Libellé de la matière montrée.
    pub matiere: String,
}

/// Session « de classe » pour lire ce que voient les élèves de cette classe : sans élève, avec
/// la classe et l'année, pour que les ajouts réservés à la classe comptent.
fn session_de_classe(c: &depot::ClasseSuivie) -> SessionEleve {
    SessionEleve {
        eleve_id: None,
        prenom: None,
        classe_id: Some(c.classe_id.clone()),
        classe: Some(c.libelle.clone()),
        niveau_id: c.niveau_id.clone(),
        niveau: c.niveau.clone(),
        serie_id: c.serie_id.clone(),
        annee_id: Some(c.annee_id.clone()),
        ephemere: false,
        duree_ms: 0,
        derniere_activite: 0,
    }
}

/// Le suivi d'une classe, pour ce compte : un compte de maître ne voit que les classes et les
/// matières de ses affectations ; la direction voit tout. `file` apporte les gestes qui
/// attendent encore d'être écrits.
pub async fn lire(
    noyau: &Noyau,
    lecture: &LecturePortail,
    file: &FileAvancement,
    session: &Session,
    demande: &DemandeSuivi,
) -> Resultat<SuiviLu> {
    let contexte = noyau.contexte(session.etablissement_id, Some(session));
    let maitre = noyau
        .droits_de(session.utilisateur_id, &contexte)
        .await?
        .maitre;
    let mut cnx = noyau
        .base()
        .pool()
        .acquire()
        .await
        .map_err(gesco_db::ErreurBase::from)?;
    let reglages = super::ServicePortail::lire_reglages(&mut cnx, &contexte).await?;
    let module = crate::modules::actif(&mut cnx, ModuleEcole::Bibliotheque, &contexte).await?;
    let actif = reglages.suivi && module;
    let grain = GrainSuivi::depuis_nom(&reglages.grain).unwrap_or_default();

    // Classes et matières permises.
    let mut classes = depot::classes_suivies(&mut cnx, &contexte).await?;
    let permises: Option<HashMap<String, HashSet<String>>> = if maitre {
        let mut p: HashMap<String, HashSet<String>> = HashMap::new();
        for (classe, matiere) in depot::affectations_du_compte(
            &mut cnx,
            &session.utilisateur_id.vers_chaine(),
            &contexte,
        )
        .await?
        {
            p.entry(classe).or_default().insert(format!("m:{matiere}"));
        }
        classes.retain(|c| p.contains_key(&c.classe_id));
        Some(p)
    } else {
        None
    };
    let choisie = match &demande.classe_id {
        Some(id) => Some(classes.iter().find(|c| &c.classe_id == id).ok_or_else(|| {
            ErreurService::PorteeRefusee {
                ressource: format!("classe {id}"),
            }
        })?),
        None => classes.first(),
    };
    let liste: Vec<ClasseASuivre> = classes
        .iter()
        .map(|c| ClasseASuivre {
            id: c.classe_id.clone(),
            libelle: c.libelle.clone(),
        })
        .collect();
    let mut suivi = SuiviClasse {
        actif,
        grain: grain.nom().to_owned(),
        classes: liste,
        classe_id: None,
        classe: String::new(),
        eleves: 0,
        matieres: Vec::new(),
        matiere_cle: None,
        lecons: Vec::new(),
        par_eleve: Vec::new(),
    };
    let Some(classe) = choisie.cloned() else {
        return Ok(SuiviLu {
            suivi,
            matiere: String::new(),
        });
    };
    suivi.classe_id = Some(classe.classe_id.clone());
    suivi.classe = classe.libelle.clone();

    // Ce que voient les élèves de la classe, matière par matière.
    let catalogue = lecture.catalogue(&session_de_classe(&classe));
    let siennes = permises
        .as_ref()
        .and_then(|p| p.get(&classe.classe_id))
        .cloned();
    let matieres: Vec<_> = catalogue
        .matieres
        .iter()
        .filter(|m| siennes.as_ref().is_none_or(|s| s.contains(&m.cle)))
        .collect();
    suivi.matieres = matieres
        .iter()
        .map(|m| MatiereSuivie {
            cle: m.cle.clone(),
            libelle: m.libelle.clone(),
        })
        .collect();
    let matiere = match &demande.matiere_cle {
        Some(cle) => matieres.iter().find(|m| &m.cle == cle).copied(),
        None => None,
    }
    .or_else(|| matieres.first().copied());

    let eleves = depot::eleves_de_la_classe(&mut cnx, &classe.classe_id, &contexte).await?;
    suivi.eleves = u32::try_from(eleves.len()).unwrap_or(u32::MAX);
    let Some(matiere) = matiere else {
        return Ok(SuiviLu {
            suivi,
            matiere: String::new(),
        });
    };
    suivi.matiere_cle = Some(matiere.cle.clone());
    // Éteint, ce qui a été écrit reste mais ne se montre plus.
    if !actif {
        return Ok(SuiviLu {
            suivi,
            matiere: matiere.libelle.clone(),
        });
    }

    let de_la_classe: HashSet<&str> = eleves.iter().map(|e| e.eleve_id.as_str()).collect();
    let ecrites =
        depot::avancement_de_la_classe(&mut cnx, &classe.classe_id, &classe.annee_id, &contexte)
            .await?;
    let attente = file.en_attente(&classe.annee_id, &|e| de_la_classe.contains(e));
    let lignes: Vec<LigneAvancement> = fondre(ecrites, attente)
        .into_iter()
        .filter(|l| de_la_classe.contains(l.eleve_id.as_str()))
        .collect();

    let lecons: Vec<(String, String, String)> = matiere
        .parties
        .iter()
        .flat_map(|p| p.chapitres.iter())
        .flat_map(|c| {
            c.contenus
                .iter()
                .filter(|e| e.nature == NatureContenu::Lecon)
                .map(move |e| (e.id.clone(), e.titre.clone(), c.titre.clone()))
        })
        .collect();
    let de_la_matiere: HashSet<&str> = lecons.iter().map(|(id, _, _)| id.as_str()).collect();
    let mut par_contenu: HashMap<&str, (u32, u32)> = HashMap::new();
    let mut par_eleve: HashMap<&str, (u32, u32)> = HashMap::new();
    let mut visites: HashMap<&str, i64> = HashMap::new();
    for l in &lignes {
        let v = visites.entry(l.eleve_id.as_str()).or_insert(l.dernier_le);
        *v = (*v).max(l.dernier_le);
        if !de_la_matiere.contains(l.contenu_id.as_str()) {
            continue;
        }
        let fini = u32::from(l.fini_le.is_some());
        let c = par_contenu.entry(l.contenu_id.as_str()).or_default();
        c.0 = c.0.saturating_add(1);
        c.1 = c.1.saturating_add(fini);
        let e = par_eleve.entry(l.eleve_id.as_str()).or_default();
        e.0 = e.0.saturating_add(1);
        e.1 = e.1.saturating_add(fini);
    }
    suivi.lecons = lecons
        .iter()
        .map(|(id, titre, chapitre)| {
            let (ouvertes, finies) = par_contenu.get(id.as_str()).copied().unwrap_or_default();
            LeconSuivie {
                id: id.clone(),
                titre: titre.clone(),
                chapitre: chapitre.clone(),
                ouvertes,
                finies,
            }
        })
        .collect();
    if grain == GrainSuivi::Eleve {
        suivi.par_eleve = eleves
            .iter()
            .map(|e| {
                let (ouvertes, finies) = par_eleve
                    .get(e.eleve_id.as_str())
                    .copied()
                    .unwrap_or_default();
                EleveSuivi {
                    eleve_id: e.eleve_id.clone(),
                    nom: format!("{} {}", e.nom, e.prenoms).trim().to_owned(),
                    ouvertes,
                    finies,
                    derniere_visite: visites.get(e.eleve_id.as_str()).copied(),
                }
            })
            .collect();
    }
    Ok(SuiviLu {
        suivi,
        matiere: matiere.libelle.clone(),
    })
}

/// Date courte d'un instant (heure du Mali, qui est celle de Greenwich) : `18/11/2026`.
fn date_courte(millis: i64) -> String {
    let c = gesco_core::commun::Horodatage::depuis_millis_epoque(millis).civil_utc();
    format!("{:02}/{:02}/{}", c.jour, c.mois, c.annee)
}

/// Données du modèle d'impression `suivi_eleves`.
pub fn donnees_impression(lu: &SuiviLu, date: &str) -> serde_json::Value {
    let s = &lu.suivi;
    serde_json::json!({
        "classe": s.classe,
        "matiere": lu.matiere,
        "date": date,
        "eleves": s.eleves,
        "par_eleve": s.grain == GrainSuivi::Eleve.nom(),
        "lecons": s.lecons.iter().map(|l| serde_json::json!({
            "chapitre": l.chapitre,
            "titre": l.titre,
            "ouvertes": l.ouvertes,
            "finies": l.finies,
        })).collect::<Vec<_>>(),
        "liste": s.par_eleve.iter().map(|e| serde_json::json!({
            "nom": e.nom,
            "ouvertes": e.ouvertes,
            "finies": e.finies,
            "visite": e.derniere_visite.map(date_courte).unwrap_or_default(),
        })).collect::<Vec<_>>(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ligne(eleve: &str, contenu: &str, dernier: i64, fini: Option<i64>) -> LigneAvancement {
        LigneAvancement {
            eleve_id: eleve.to_owned(),
            contenu_id: contenu.to_owned(),
            ouvert_le: dernier,
            dernier_le: dernier,
            fini_le: fini,
        }
    }

    #[test]
    fn deux_gestes_sur_la_meme_lecon_se_fondent() {
        let file = FileAvancement::default();
        file.noter(("e1", "a1", "c1"), "v1", GesteAvancement::Ouvert, 10);
        file.noter(("e1", "a1", "c1"), "v2", GesteAvancement::Fini, 20);
        file.noter(("e1", "a1", "c1"), "v2", GesteAvancement::Ouvert, 30);
        assert_eq!(file.longueur(), 1);
        let pris = file.prendre();
        assert_eq!(file.longueur(), 0);
        assert_eq!(pris.len(), 1);
        assert_eq!(
            pris.first()
                .map(|g| (g.ouvert_le, g.dernier_le, g.fini_le, g.version.as_str())),
            Some((10, 30, Some(20), "v2"))
        );
        // Un échec d'écriture les remet, fondus avec ce qui est arrivé entre-temps.
        file.noter(("e1", "a1", "c1"), "v3", GesteAvancement::Ouvert, 40);
        file.remettre(pris);
        let repris = file.prendre();
        assert_eq!(repris.len(), 1);
        assert_eq!(
            repris
                .first()
                .map(|g| (g.ouvert_le, g.dernier_le, g.fini_le)),
            Some((10, 40, Some(20)))
        );
    }

    #[test]
    fn l_attente_compte_deja() {
        let file = FileAvancement::default();
        file.noter(("e1", "a1", "c2"), "v", GesteAvancement::Fini, 50);
        file.noter(("e2", "a1", "c2"), "v", GesteAvancement::Ouvert, 60);
        file.noter(("e1", "a0", "c2"), "v", GesteAvancement::Ouvert, 70);
        let attente = file.en_attente("a1", &|e| e == "e1");
        assert_eq!(attente.len(), 1);
        let lignes = fondre(vec![ligne("e1", "c1", 5, None)], attente);
        assert_eq!(lignes.len(), 2);
        assert_eq!(
            lignes.first().map(|l| (l.contenu_id.as_str(), l.fini_le)),
            Some(("c2", Some(50)))
        );
    }

    #[test]
    fn une_lecon_finie_le_reste() {
        let lignes = fondre(
            vec![ligne("e1", "c1", 5, Some(5))],
            vec![ligne("e1", "c1", 9, None)],
        );
        assert_eq!(lignes.len(), 1);
        assert_eq!(
            lignes.first().map(|l| (l.dernier_le, l.fini_le)),
            Some((9, Some(5)))
        );
    }
}
