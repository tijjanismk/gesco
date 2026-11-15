//! Le catalogue de la bibliothèque (§5.1 de la spécification) : niveau, série au lycée, matière,
//! partie, chapitre, contenus, dans l'ordre du programme, pour les seuls niveaux actifs de
//! l'école. Règles pures : le service lit la base et le paquet, ce module range.
//!
//! - Un contenu livré se range sous la matière de l'école que la correspondance des disciplines
//!   lui donne ; sans correspondance (ou matière éteinte), sous le libellé de référence du paquet.
//! - Un chapitre ou un contenu de l'école se range après la clé que l'école lui a donnée.
//! - Un contenu livré ne paraît que si son paquet est là et qu'il est encore livré, ou si l'école
//!   en a publié une version. Un contenu de l'école sans version publiée ne paraît que pour le
//!   personnel qui écrit. Aucun chapitre vide ne paraît, sauf un chapitre de l'école pour ce
//!   même personnel (il y ajoute ses leçons).

use std::collections::{HashMap, HashSet};

use serde::{Deserialize, Serialize};

use super::catalogue::{ChapitreBibliotheque, ContenuBibliotheque, OrigineBibliotheque};
use super::paquet::{DisciplinePaquet, ExamenSujet, NatureContenu};

/// Un niveau actif de l'école, dans l'ordre d'affichage.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NiveauReference {
    /// Identifiant du niveau.
    pub id: String,
    /// Code national (`7`, `12`…).
    pub code: String,
    /// Libellé de l'école.
    pub libelle: String,
}

/// Une série active d'un niveau de l'école.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SerieReference {
    /// Niveau de la série.
    pub niveau_id: String,
    /// Code (`TSE`…).
    pub code: String,
    /// Libellé de l'école.
    pub libelle: String,
}

/// Une matière de l'école, dans l'ordre de l'école.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MatiereReference {
    /// Identifiant de la matière.
    pub id: String,
    /// Code (`MATH`…).
    pub code: String,
    /// Libellé de l'école.
    pub libelle: String,
    /// Matière en service.
    pub active: bool,
}

/// Ce que l'école a fait d'un contenu, pour les marques du catalogue.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct EtatContenu {
    /// L'école a publié une version écrite (pas un retour à la version livrée).
    pub version_ecole: bool,
    /// Une version attend la validation.
    pub en_attente: bool,
    /// Un brouillon est en cours.
    pub brouillon: bool,
}

/// Tout ce que le catalogue range.
pub struct EntreesCatalogue<'a> {
    /// Niveaux actifs de l'école, dans l'ordre.
    pub niveaux: &'a [NiveauReference],
    /// Séries actives.
    pub series: &'a [SerieReference],
    /// Matières de l'école, dans l'ordre de l'école.
    pub matieres: &'a [MatiereReference],
    /// Libellés de référence des disciplines, dans l'ordre des paquets.
    pub disciplines: &'a [DisciplinePaquet],
    /// Correspondance : (discipline, code du niveau, matière de l'école).
    pub correspondances: &'a [(String, String, Option<String>)],
    /// Matières de l'utilisateur : (matière, code du niveau) de ses affectations de l'année.
    pub mes_matieres: &'a HashSet<(String, String)>,
    /// Chapitres de l'établissement.
    pub chapitres: &'a [ChapitreBibliotheque],
    /// Contenus de l'établissement.
    pub contenus: &'a [ContenuBibliotheque],
    /// État de chaque contenu qui a une version de l'école, par identifiant.
    pub etats: &'a HashMap<String, EtatContenu>,
    /// Codes des paquets lus sur ce poste.
    pub paquets_presents: &'a HashSet<String>,
    /// Vrai pour le personnel qui écrit (`library:edit`).
    pub voit_le_travail: bool,
}

/// Un niveau du catalogue.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
pub struct NiveauCatalogue {
    /// Code national.
    pub code: String,
    /// Libellé de l'école.
    pub libelle: String,
    /// Séries ; une seule, sans code, pour un niveau sans série.
    pub series: Vec<SerieCatalogue>,
}

/// Une série du catalogue.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
pub struct SerieCatalogue {
    /// Code de la série ; aucun pour un niveau sans série.
    pub code: Option<String>,
    /// Libellé de la série.
    pub libelle: Option<String>,
    /// Matières.
    pub matieres: Vec<MatiereCatalogue>,
}

/// Une matière du catalogue.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
pub struct MatiereCatalogue {
    /// Clé de la matière : `m:<identifiant>` pour une matière de l'école, `d:<code>` pour une
    /// discipline de référence sans correspondance.
    pub cle: String,
    /// Libellé de l'école, sinon libellé de référence.
    pub libelle: String,
    /// Matière de l'école qui reçoit la discipline, s'il y en a une.
    pub matiere_id: Option<String>,
    /// Discipline de référence des contenus livrés rangés ici, s'il y en a.
    pub discipline_code: Option<String>,
    /// Une matière des affectations de l'utilisateur, dans ce niveau.
    pub mienne: bool,
    /// Parties, dans l'ordre du programme.
    pub parties: Vec<PartieCatalogue>,
}

/// Une partie (« Algèbre », « Examens »…).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
pub struct PartieCatalogue {
    /// Titre de la partie (vide : sans partie).
    pub titre: String,
    /// Chapitres.
    pub chapitres: Vec<ChapitreCatalogue>,
}

/// Un chapitre du catalogue.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
pub struct ChapitreCatalogue {
    /// Identifiant.
    pub id: String,
    /// Clé stable.
    pub cle: String,
    /// Titre.
    pub titre: String,
    /// Origine.
    pub origine: OrigineBibliotheque,
    /// Contenus, dans l'ordre.
    pub contenus: Vec<ContenuCatalogue>,
}

/// Un contenu du catalogue.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
pub struct ContenuCatalogue {
    /// Identifiant.
    pub id: String,
    /// Clé stable.
    pub cle: String,
    /// Titre.
    pub titre: String,
    /// Nature.
    pub nature: NatureContenu,
    /// Examen d'un sujet ou d'un corrigé.
    pub examen: Option<ExamenSujet>,
    /// Session d'un sujet ou d'un corrigé.
    pub session: Option<u16>,
    /// Origine.
    pub origine: OrigineBibliotheque,
    /// La version visible est celle de l'école (marque montrée au personnel qui écrit).
    pub version_ecole: bool,
    /// Une version attend la validation.
    pub en_attente: bool,
    /// Un brouillon est en cours.
    pub brouillon: bool,
    /// Le contenu lié (le corrigé d'un sujet, et l'inverse), s'il paraît aussi.
    pub lie_id: Option<String>,
}

/// Où se range une matière : une matière de l'école, ou une discipline de référence.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
enum CleMatiere {
    Ecole(String),
    Reference(String),
}

/// Range les éléments de l'école parmi les livrés : un élément sans clé de rangement va en tête,
/// un élément rangé après une clé inconnue va à la fin ; plusieurs éléments rangés au même
/// endroit gardent leur ordre. `livres` et `ecole` arrivent chacun dans leur ordre.
pub fn ranger<T>(
    livres: Vec<T>,
    ecole: Vec<T>,
    cle: impl Fn(&T) -> &str,
    apres: impl Fn(&T) -> Option<&str>,
) -> Vec<T> {
    // Chaque place garde l'ancre de l'élément de l'école posé là, pour poser le suivant après.
    let mut places: Vec<(T, Option<Option<String>>)> =
        livres.into_iter().map(|t| (t, None)).collect();
    for e in ecole {
        let ancre = apres(&e).map(str::to_owned);
        let mut pos = match &ancre {
            None => 0,
            Some(k) => match places.iter().position(|(t, _)| cle(t) == k) {
                Some(i) => i + 1,
                None => places.len(),
            },
        };
        while places
            .get(pos)
            .is_some_and(|p| p.1.as_ref() == Some(&ancre))
        {
            pos += 1;
        }
        places.insert(pos, (e, Some(ancre)));
    }
    places.into_iter().map(|(t, _)| t).collect()
}

/// Range le catalogue.
pub fn construire_catalogue(e: &EntreesCatalogue<'_>) -> Vec<NiveauCatalogue> {
    let cles_contenus: HashMap<&str, &ContenuBibliotheque> =
        e.contenus.iter().map(|c| (c.cle.as_str(), c)).collect();
    let paraissent: HashSet<&str> = e
        .contenus
        .iter()
        .filter(|c| contenu_parait(e, c))
        .map(|c| c.cle.as_str())
        .collect();

    // Contenus par chapitre, rangés.
    let mut par_chapitre: HashMap<String, Vec<&ContenuBibliotheque>> = HashMap::new();
    for c in e
        .contenus
        .iter()
        .filter(|c| paraissent.contains(c.cle.as_str()))
    {
        par_chapitre
            .entry(c.chapitre_id.vers_chaine())
            .or_default()
            .push(c);
    }
    let par_chapitre: HashMap<String, Vec<ContenuCatalogue>> = par_chapitre
        .into_iter()
        .map(|(chapitre, liste)| {
            let (mut livres, mut ecole): (Vec<_>, Vec<_>) = liste
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
            let contenus = ranges
                .into_iter()
                .map(|c| {
                    let etat = e
                        .etats
                        .get(&c.id.vers_chaine())
                        .copied()
                        .unwrap_or_default();
                    ContenuCatalogue {
                        id: c.id.vers_chaine(),
                        cle: c.cle.clone(),
                        titre: c.titre.clone(),
                        nature: c.nature,
                        examen: c.examen,
                        session: c.session,
                        origine: c.origine,
                        version_ecole: etat.version_ecole,
                        en_attente: etat.en_attente,
                        brouillon: etat.brouillon,
                        lie_id: c
                            .lie_cle
                            .as_deref()
                            .filter(|k| paraissent.contains(k))
                            .and_then(|k| cles_contenus.get(k))
                            .map(|l| l.id.vers_chaine()),
                    }
                })
                .collect();
            (chapitre, contenus)
        })
        .collect();

    let matieres: HashMap<&str, &MatiereReference> =
        e.matieres.iter().map(|m| (m.id.as_str(), m)).collect();
    let mut niveaux = Vec::new();
    for niveau in e.niveaux {
        // Chapitres du niveau, rangés dans l'ordre du programme.
        let (mut livres, mut ecole): (Vec<&ChapitreBibliotheque>, Vec<&ChapitreBibliotheque>) = e
            .chapitres
            .iter()
            .filter(|c| match c.origine {
                OrigineBibliotheque::Livre => c.niveau_code == niveau.code,
                OrigineBibliotheque::Ecole => c.niveau_id.as_deref() == Some(niveau.id.as_str()),
            })
            .partition(|c| c.origine == OrigineBibliotheque::Livre);
        livres.sort_by_key(|c| c.ordre);
        ecole.sort_by_key(|c| c.ordre);
        let chapitres = ranger(
            livres,
            ecole,
            |c| c.cle.as_str(),
            |c| c.apres_cle.as_deref(),
        );

        let series_ecole: Vec<&SerieReference> = e
            .series
            .iter()
            .filter(|s| s.niveau_id == niveau.id)
            .collect();
        let groupes: Vec<(Option<&SerieReference>, Vec<&ChapitreBibliotheque>)> = if series_ecole
            .is_empty()
        {
            vec![(None, chapitres)]
        } else {
            series_ecole
                .iter()
                .map(|s| {
                    let du_groupe = chapitres
                        .iter()
                        .copied()
                        .filter(|c| c.series.is_empty() || c.series.iter().any(|x| x == &s.code))
                        .collect();
                    (Some(*s), du_groupe)
                })
                .collect()
        };

        let mut series = Vec::new();
        for (serie, chapitres) in groupes {
            let matieres_serie = ranger_matieres(e, &matieres, niveau, &chapitres, &par_chapitre);
            if !matieres_serie.is_empty() {
                series.push(SerieCatalogue {
                    code: serie.map(|s| s.code.clone()),
                    libelle: serie.map(|s| s.libelle.clone()),
                    matieres: matieres_serie,
                });
            }
        }
        if !series.is_empty() {
            niveaux.push(NiveauCatalogue {
                code: niveau.code.clone(),
                libelle: niveau.libelle.clone(),
                series,
            });
        }
    }
    niveaux
}

/// Vrai si le contenu paraît au catalogue.
fn contenu_parait(e: &EntreesCatalogue<'_>, c: &ContenuBibliotheque) -> bool {
    let etat = e
        .etats
        .get(&c.id.vers_chaine())
        .copied()
        .unwrap_or_default();
    if etat.version_ecole {
        return true;
    }
    match c.origine {
        OrigineBibliotheque::Livre => {
            !c.plus_livre
                && c.paquet_code
                    .as_deref()
                    .is_some_and(|p| e.paquets_presents.contains(p))
        }
        OrigineBibliotheque::Ecole => e.voit_le_travail,
    }
}

/// Matières d'un groupe de chapitres, chacune avec ses parties dans l'ordre des chapitres.
fn ranger_matieres(
    e: &EntreesCatalogue<'_>,
    matieres: &HashMap<&str, &MatiereReference>,
    niveau: &NiveauReference,
    chapitres: &[&ChapitreBibliotheque],
    par_chapitre: &HashMap<String, Vec<ContenuCatalogue>>,
) -> Vec<MatiereCatalogue> {
    let mut ordre_cles: Vec<CleMatiere> = Vec::new();
    let mut parties: HashMap<CleMatiere, Vec<PartieCatalogue>> = HashMap::new();
    let mut disciplines: HashMap<CleMatiere, String> = HashMap::new();
    for c in chapitres {
        let contenus = par_chapitre
            .get(&c.id.vers_chaine())
            .cloned()
            .unwrap_or_default();
        let garde_vide = c.origine == OrigineBibliotheque::Ecole && e.voit_le_travail;
        if contenus.is_empty() && !garde_vide {
            continue;
        }
        let cle = match c.origine {
            OrigineBibliotheque::Livre => {
                let discipline = c.discipline_code.clone().unwrap_or_default();
                let matiere = e
                    .correspondances
                    .iter()
                    .find(|(d, n, _)| d == &discipline && n == &niveau.code)
                    .and_then(|(_, _, m)| m.as_deref())
                    .filter(|m| matieres.get(m).is_some_and(|x| x.active));
                let cle = match matiere {
                    Some(m) => CleMatiere::Ecole(m.to_owned()),
                    None => CleMatiere::Reference(discipline.clone()),
                };
                disciplines.entry(cle.clone()).or_insert(discipline);
                cle
            }
            OrigineBibliotheque::Ecole => {
                CleMatiere::Ecole(c.matiere_id.clone().unwrap_or_default())
            }
        };
        if !ordre_cles.contains(&cle) {
            ordre_cles.push(cle.clone());
        }
        let liste = parties.entry(cle).or_default();
        let chapitre = ChapitreCatalogue {
            id: c.id.vers_chaine(),
            cle: c.cle.clone(),
            titre: c.titre.clone(),
            origine: c.origine,
            contenus,
        };
        match liste.iter_mut().find(|p| p.titre == c.partie) {
            Some(p) => p.chapitres.push(chapitre),
            None => liste.push(PartieCatalogue {
                titre: c.partie.clone(),
                chapitres: vec![chapitre],
            }),
        }
    }
    // Matières de l'école dans l'ordre de l'école, puis disciplines de référence dans l'ordre
    // des paquets.
    let rang = |cle: &CleMatiere| -> (usize, usize) {
        match cle {
            CleMatiere::Ecole(id) => (
                0,
                e.matieres
                    .iter()
                    .position(|m| &m.id == id)
                    .unwrap_or(usize::MAX),
            ),
            CleMatiere::Reference(code) => (
                1,
                e.disciplines
                    .iter()
                    .position(|d| &d.code == code)
                    .unwrap_or(usize::MAX),
            ),
        }
    };
    ordre_cles.sort_by_key(|c| rang(c));
    ordre_cles
        .into_iter()
        .map(|cle| {
            let parties = parties.remove(&cle).unwrap_or_default();
            let discipline_code = disciplines.get(&cle).cloned();
            match cle {
                CleMatiere::Ecole(id) => MatiereCatalogue {
                    cle: format!("m:{id}"),
                    libelle: matieres
                        .get(id.as_str())
                        .map(|m| m.libelle.clone())
                        .unwrap_or_default(),
                    mienne: e.mes_matieres.contains(&(id.clone(), niveau.code.clone())),
                    matiere_id: Some(id),
                    discipline_code,
                    parties,
                },
                CleMatiere::Reference(code) => MatiereCatalogue {
                    cle: format!("d:{code}"),
                    libelle: e
                        .disciplines
                        .iter()
                        .find(|d| d.code == code)
                        .map(|d| d.libelle.clone())
                        .unwrap_or_else(|| code.clone()),
                    mienne: false,
                    matiere_id: None,
                    discipline_code: Some(code),
                    parties,
                },
            }
        })
        .collect()
}

/// Requête plein texte d'après ce que l'utilisateur tape : chaque mot devient un préfixe entre
/// guillemets, tous les mots sont exigés. Rien de ce qui est tapé n'est lu comme un opérateur.
/// Vide si aucun mot.
pub fn requete_recherche(texte: &str) -> String {
    texte
        .split(|c: char| !c.is_alphanumeric())
        .filter(|m| !m.is_empty())
        .take(12)
        .map(|m| format!("\"{m}\"*"))
        .collect::<Vec<_>>()
        .join(" ")
}

#[cfg(test)]
#[allow(clippy::indexing_slicing)]
mod tests {
    use super::*;
    use crate::commun::{Horodatage, Id};
    use std::sync::atomic::{AtomicU64, Ordering};

    fn id<T>() -> Id<T> {
        static COMPTEUR: AtomicU64 = AtomicU64::new(1);
        Id::nouveau(
            Horodatage::depuis_millis_epoque(1_700_000_000_000),
            u128::from(COMPTEUR.fetch_add(1, Ordering::Relaxed)),
        )
    }

    fn chapitre(
        cle: &str,
        niveau: &str,
        discipline: &str,
        partie: &str,
        ordre: i64,
    ) -> ChapitreBibliotheque {
        ChapitreBibliotheque {
            id: id(),
            cle: cle.into(),
            origine: OrigineBibliotheque::Livre,
            paquet_code: Some("p".into()),
            niveau_code: niveau.into(),
            discipline_code: Some(discipline.into()),
            niveau_id: None,
            matiere_id: None,
            partie: partie.into(),
            titre: cle.into(),
            ordre,
            apres_cle: None,
            a_confirmer: false,
            series: Vec::new(),
            plus_livre: false,
        }
    }

    fn contenu(cle: &str, chapitre: &ChapitreBibliotheque, ordre: i64) -> ContenuBibliotheque {
        ContenuBibliotheque {
            id: id(),
            cle: cle.into(),
            origine: OrigineBibliotheque::Livre,
            nature: NatureContenu::Lecon,
            chapitre_id: chapitre.id,
            titre: cle.into(),
            presentation: String::new(),
            ordre,
            apres_cle: None,
            langue: "fr".into(),
            lie_cle: None,
            examen: None,
            serie_code: None,
            session: None,
            paquet_code: Some("p".into()),
            fichier: Some(format!("{cle}.md")),
            empreinte: Some("0".repeat(64)),
            faits: None,
            plus_livre: false,
        }
    }

    struct Monde {
        niveaux: Vec<NiveauReference>,
        series: Vec<SerieReference>,
        matieres: Vec<MatiereReference>,
        disciplines: Vec<DisciplinePaquet>,
        correspondances: Vec<(String, String, Option<String>)>,
        mes: HashSet<(String, String)>,
        chapitres: Vec<ChapitreBibliotheque>,
        contenus: Vec<ContenuBibliotheque>,
        etats: HashMap<String, EtatContenu>,
        presents: HashSet<String>,
    }

    impl Monde {
        fn nouveau() -> Self {
            Self {
                niveaux: vec![
                    NiveauReference {
                        id: "n9".into(),
                        code: "9".into(),
                        libelle: "9e année".into(),
                    },
                    NiveauReference {
                        id: "n12".into(),
                        code: "12".into(),
                        libelle: "Terminale".into(),
                    },
                ],
                series: vec![
                    SerieReference {
                        niveau_id: "n12".into(),
                        code: "TSE".into(),
                        libelle: "TSE".into(),
                    },
                    SerieReference {
                        niveau_id: "n12".into(),
                        code: "TSExp".into(),
                        libelle: "TSExp".into(),
                    },
                ],
                matieres: vec![MatiereReference {
                    id: "m-math".into(),
                    code: "MATH".into(),
                    libelle: "Maths".into(),
                    active: true,
                }],
                disciplines: vec![
                    DisciplinePaquet {
                        code: "MATH".into(),
                        libelle: "Mathématiques".into(),
                    },
                    DisciplinePaquet {
                        code: "SVT".into(),
                        libelle: "Biologie".into(),
                    },
                ],
                correspondances: vec![("MATH".into(), "9".into(), Some("m-math".into()))],
                mes: HashSet::new(),
                chapitres: Vec::new(),
                contenus: Vec::new(),
                etats: HashMap::new(),
                presents: ["p".to_owned()].into(),
            }
        }

        fn catalogue(&self, voit_le_travail: bool) -> Vec<NiveauCatalogue> {
            construire_catalogue(&EntreesCatalogue {
                niveaux: &self.niveaux,
                series: &self.series,
                matieres: &self.matieres,
                disciplines: &self.disciplines,
                correspondances: &self.correspondances,
                mes_matieres: &self.mes,
                chapitres: &self.chapitres,
                contenus: &self.contenus,
                etats: &self.etats,
                paquets_presents: &self.presents,
                voit_le_travail,
            })
        }
    }

    #[test]
    fn un_contenu_livre_se_range_sous_la_matiere_de_l_ecole_ou_sous_la_reference() {
        let mut m = Monde::nouveau();
        let c1 = chapitre("9/math/racine", "9", "MATH", "Algèbre", 1);
        let c2 = chapitre("9/svt/cellule", "9", "SVT", "Biologie", 2);
        m.contenus.push(contenu("9/math/racine/l1", &c1, 1));
        m.contenus.push(contenu("9/svt/cellule/l1", &c2, 1));
        m.chapitres = vec![c1, c2];
        let cat = m.catalogue(false);
        assert_eq!(cat.len(), 1);
        let matieres = &cat[0].series[0].matieres;
        assert_eq!(matieres[0].libelle, "Maths");
        assert_eq!(matieres[0].cle, "m:m-math");
        assert_eq!(matieres[1].libelle, "Biologie");
        assert_eq!(matieres[1].cle, "d:SVT");
    }

    #[test]
    fn un_niveau_que_l_ecole_n_a_pas_ne_montre_rien_et_un_chapitre_vide_ne_parait_pas() {
        let mut m = Monde::nouveau();
        let c1 = chapitre("7/math/a", "7", "MATH", "", 1);
        let c2 = chapitre("9/math/vide", "9", "MATH", "", 1);
        m.contenus.push(contenu("7/math/a/l1", &c1, 1));
        m.chapitres = vec![c1, c2];
        assert!(m.catalogue(true).is_empty());
    }

    #[test]
    fn un_chapitre_du_lycee_parait_sous_chaque_serie_qu_il_vise() {
        let mut m = Monde::nouveau();
        let mut c1 = chapitre("12/math/complexes", "12", "MATH", "", 1);
        c1.series = vec!["TSE".into()];
        let c2 = chapitre("12/math/suites", "12", "MATH", "", 2);
        m.contenus.push(contenu("12/math/complexes/l1", &c1, 1));
        m.contenus.push(contenu("12/math/suites/l1", &c2, 1));
        m.chapitres = vec![c1, c2];
        let cat = m.catalogue(false);
        let series = &cat[0].series;
        assert_eq!(series.len(), 2);
        let titres = |i: usize| -> Vec<String> {
            series[i].matieres[0].parties[0]
                .chapitres
                .iter()
                .map(|c| c.titre.clone())
                .collect()
        };
        assert_eq!(titres(0), ["12/math/complexes", "12/math/suites"]);
        assert_eq!(titres(1), ["12/math/suites"]);
    }

    #[test]
    fn sans_paquet_ni_version_de_l_ecole_un_contenu_livre_ne_parait_pas() {
        let mut m = Monde::nouveau();
        let c1 = chapitre("9/math/racine", "9", "MATH", "", 1);
        let l1 = contenu("9/math/racine/l1", &c1, 1);
        let l2 = contenu("9/math/racine/l2", &c1, 2);
        m.etats.insert(
            l2.id.vers_chaine(),
            EtatContenu {
                version_ecole: true,
                ..EtatContenu::default()
            },
        );
        m.contenus = vec![l1, l2];
        m.chapitres = vec![c1];
        m.presents.clear();
        let cat = m.catalogue(false);
        let contenus = &cat[0].series[0].matieres[0].parties[0].chapitres[0].contenus;
        assert_eq!(contenus.len(), 1);
        assert_eq!(contenus[0].cle, "9/math/racine/l2");
        assert!(contenus[0].version_ecole);
    }

    #[test]
    fn un_contenu_plus_livre_ne_reste_que_s_il_a_une_version_de_l_ecole() {
        let mut m = Monde::nouveau();
        let c1 = chapitre("9/math/racine", "9", "MATH", "", 1);
        let mut l1 = contenu("9/math/racine/l1", &c1, 1);
        l1.plus_livre = true;
        m.contenus = vec![l1];
        m.chapitres = vec![c1];
        assert!(m.catalogue(true).is_empty());
    }

    #[test]
    fn la_mienne_suit_les_affectations_du_niveau() {
        let mut m = Monde::nouveau();
        let c1 = chapitre("9/math/racine", "9", "MATH", "", 1);
        m.contenus.push(contenu("9/math/racine/l1", &c1, 1));
        m.chapitres = vec![c1];
        m.mes.insert(("m-math".into(), "9".into()));
        assert!(m.catalogue(false)[0].series[0].matieres[0].mienne);
        m.mes.clear();
        m.mes.insert(("m-math".into(), "8".into()));
        assert!(!m.catalogue(false)[0].series[0].matieres[0].mienne);
    }

    #[test]
    fn un_sujet_renvoie_a_son_corrige() {
        let mut m = Monde::nouveau();
        let c1 = chapitre("9/math/examens", "9", "MATH", "Examens", 1);
        let mut s = contenu("9/math/def-2024", &c1, 1);
        s.nature = NatureContenu::Sujet;
        s.lie_cle = Some("9/math/def-2024-corrige".into());
        let mut k = contenu("9/math/def-2024-corrige", &c1, 2);
        k.nature = NatureContenu::Corrige;
        k.lie_cle = Some("9/math/def-2024".into());
        let id_k = k.id.vers_chaine();
        m.contenus = vec![s, k];
        m.chapitres = vec![c1];
        let cat = m.catalogue(false);
        let contenus = &cat[0].series[0].matieres[0].parties[0].chapitres[0].contenus;
        assert_eq!(contenus[0].lie_id.as_deref(), Some(id_k.as_str()));
    }

    #[test]
    fn un_chapitre_de_l_ecole_vide_ne_parait_que_pour_qui_ecrit() {
        let mut m = Monde::nouveau();
        let mut c1 = chapitre("ecole/1", "9", "MATH", "Algèbre", 1);
        c1.origine = OrigineBibliotheque::Ecole;
        c1.niveau_id = Some("n9".into());
        c1.matiere_id = Some("m-math".into());
        c1.discipline_code = None;
        m.chapitres = vec![c1];
        assert!(m.catalogue(false).is_empty());
        assert_eq!(m.catalogue(true)[0].series[0].matieres[0].cle, "m:m-math");
    }

    #[test]
    fn ranger_pose_l_ecole_apres_sa_cle_en_tete_ou_a_la_fin() {
        let livres = vec![("a", None), ("b", None), ("c", None)];
        let ecole = vec![
            ("x", Some("a")),
            ("y", None),
            ("z", Some("a")),
            ("w", Some("inconnue")),
        ];
        let ranges = ranger(livres, ecole, |t| t.0, |t| t.1);
        let cles: Vec<&str> = ranges.iter().map(|t| t.0).collect();
        assert_eq!(cles, ["y", "a", "x", "z", "b", "c", "w"]);
    }

    #[test]
    fn la_requete_ne_garde_que_des_mots() {
        assert_eq!(
            requete_recherche("racine carrée"),
            "\"racine\"* \"carrée\"*"
        );
        assert_eq!(requete_recherche("\"a\" OR b*"), "\"a\"* \"OR\"* \"b\"*");
        assert_eq!(requete_recherche("  ;; "), "");
    }
}
