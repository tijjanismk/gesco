//! Lire la bibliothèque (§5 de la spécification) : le catalogue, une leçon, un sujet ou un
//! corrigé en arbre de blocs avec ses images, la recherche plein texte. Rien ici n'écrit.
//!
//! Ce qui s'affiche par défaut est la version visible : la version publiée par l'école, sinon le
//! texte livré. Le personnel qui écrit (`library:edit`) voit aussi les notes pour le maître, et
//! peut basculer sur la version livrée et sur le brouillon en cours.

use std::collections::{HashMap, HashSet};

use base64::Engine as _;
use gesco_core::bibliotheque::paquet::nom_fichier_image;
use gesco_core::bibliotheque::{
    ContenuBibliotheque, EntreesCatalogue, EtatContenu, ExamenSujet, Lecon, MatiereReference,
    NatureContenu, NiveauCatalogue, NiveauReference, OrigineBibliotheque, SerieReference,
    SourceImage, analyser, construire_catalogue, requete_recherche,
};
use gesco_core::core::Permission;
use gesco_db::ErreurBase;
use gesco_db::depots::bibliotheque as depot;
use gesco_db::depots::{academique, pedagogie};
use serde::{Deserialize, Serialize};

use super::ServiceBibliotheque;
use crate::erreur::Resultat;
use crate::session::Session;

/// Résultats d'une recherche, au plus.
const RESULTATS_MAX: u32 = 60;

/// Le catalogue tel que l'écran le montre.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
pub struct CatalogueBibliotheque {
    /// Niveaux de l'école qui ont des contenus.
    pub niveaux: Vec<NiveauCatalogue>,
    /// L'utilisateur écrit (`library:edit`) : il voit les marques et les notes pour le maître.
    pub peut_ecrire: bool,
    /// L'utilisateur valide (`library:publish`).
    pub peut_valider: bool,
    /// Compte d'un maître qui ne valide pas : l'écran s'ouvre sur « Mes matières ».
    pub maitre: bool,
}

/// Quelle version d'un contenu lire.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "snake_case")]
pub enum VueContenu {
    /// La version que voient les élèves.
    Visible,
    /// Le texte livré par la bibliothèque.
    Livree,
    /// Le brouillon en cours.
    Brouillon,
}

/// Lire un contenu.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
pub struct DemandeLectureContenu {
    /// Identifiant du contenu.
    pub contenu_id: String,
    /// Version demandée ; aucune : la version visible.
    pub vue: Option<VueContenu>,
}

/// Une image appelée par la leçon, prête à afficher.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
pub struct ImageLecon {
    /// Nom du fichier du paquet, ou empreinte d'une image de l'école.
    pub cle: String,
    /// Type MIME.
    pub type_mime: String,
    /// Contenu en base 64.
    pub donnees: String,
}

/// Le contenu lié (le corrigé d'un sujet, et l'inverse).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
pub struct ContenuLie {
    /// Identifiant.
    pub id: String,
    /// Titre.
    pub titre: String,
    /// Nature.
    pub nature: NatureContenu,
}

/// Un contenu ouvert.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
pub struct LectureContenu {
    /// Identifiant.
    pub id: String,
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
    /// Phrase de présentation d'un document.
    pub presentation: String,
    /// Version montrée.
    pub vue: VueContenu,
    /// Versions que l'utilisateur peut montrer.
    pub vues: Vec<VueContenu>,
    /// La version visible est celle de l'école.
    pub version_ecole: bool,
    /// Le texte, en arbre de blocs ; vide pour un contenu sans texte.
    pub lecon: Lecon,
    /// Images appelées par le texte.
    pub images: Vec<ImageLecon>,
    /// Contenu lié.
    pub lie: Option<ContenuLie>,
}

/// Chercher dans la bibliothèque.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
pub struct DemandeRechercheBibliotheque {
    /// Ce que l'utilisateur a tapé.
    pub texte: String,
}

/// Un contenu trouvé.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
pub struct ResultatRechercheBibliotheque {
    /// Identifiant du contenu.
    pub contenu_id: String,
    /// Titre.
    pub titre: String,
    /// Nature.
    pub nature: NatureContenu,
    /// Niveau (libellé de l'école).
    pub niveau: String,
    /// Matière (libellé de l'école ou de référence).
    pub matiere: String,
    /// Chapitre.
    pub chapitre: String,
    /// Extrait : les mots trouvés sont entre les caractères `\u{2}` et `\u{3}`.
    pub extrait: String,
}

/// Où un contenu paraît au catalogue.
struct Place {
    rang: usize,
    niveau: String,
    matiere: String,
    chapitre: String,
    nature: NatureContenu,
    titre: String,
}

impl ServiceBibliotheque<'_> {
    async fn permissions(&self, session: &Session) -> Resultat<(bool, bool)> {
        let contexte = self.noyau.contexte(session.etablissement_id, Some(session));
        let mut cnx = self.connexion().await?;
        let permissions = self
            .noyau
            .permissions_de(&mut cnx, session.utilisateur_id, &contexte)
            .await?;
        Ok((
            permissions.contains(&Permission::LIBRARY_EDIT),
            permissions.contains(&Permission::LIBRARY_PUBLISH),
        ))
    }

    /// Le catalogue rangé pour l'utilisateur.
    async fn niveaux_du_catalogue(
        &self,
        session: &Session,
        voit_le_travail: bool,
    ) -> Resultat<(Vec<NiveauCatalogue>, bool)> {
        let contexte = self.noyau.contexte(session.etablissement_id, Some(session));
        let mut cnx = self.connexion().await?;
        let niveaux: Vec<NiveauReference> = academique::niveaux(&mut cnx, &contexte)
            .await?
            .into_iter()
            .filter(|n| n.actif)
            .map(|n| NiveauReference {
                id: n.id.vers_chaine(),
                code: n.code,
                libelle: n.libelle,
            })
            .collect();
        let series: Vec<SerieReference> = academique::series(&mut cnx, &contexte)
            .await?
            .into_iter()
            .filter(|s| s.actif)
            .map(|s| SerieReference {
                niveau_id: s.niveau_id.vers_chaine(),
                code: s.code,
                libelle: s.libelle,
            })
            .collect();
        let matieres: Vec<MatiereReference> = pedagogie::matieres(&mut cnx, &contexte)
            .await?
            .into_iter()
            .map(|m| MatiereReference {
                id: m.id.vers_chaine(),
                code: m.code,
                libelle: m.libelle,
                active: m.active,
            })
            .collect();
        let disciplines = depot::paquets(&mut cnx, &contexte)
            .await?
            .into_iter()
            .filter(|p| p.actif)
            .flat_map(|p| p.disciplines)
            .collect::<Vec<_>>();
        let correspondances = depot::correspondances(&mut cnx, &contexte).await?;
        let mes_matieres: HashSet<(String, String)> =
            depot::matieres_du_compte(&mut cnx, &session.utilisateur_id.vers_chaine(), &contexte)
                .await?
                .into_iter()
                .collect();
        let chapitres = depot::chapitres(&mut cnx, &contexte).await?;
        let contenus = depot::contenus(&mut cnx, &contexte).await?;
        let mut etats: HashMap<String, EtatContenu> = HashMap::new();
        for (id, etat, retour) in depot::etats_versions(&mut cnx, &contexte).await? {
            let e = etats.entry(id).or_default();
            match etat.as_str() {
                "publiee" => e.version_ecole = !retour,
                "soumise" => e.en_attente = true,
                "brouillon" => e.brouillon = true,
                _ => {}
            }
        }
        let paquets_presents: HashSet<String> = self.paquets.tous().into_keys().collect();
        let catalogue = construire_catalogue(&EntreesCatalogue {
            niveaux: &niveaux,
            series: &series,
            matieres: &matieres,
            disciplines: &disciplines,
            correspondances: &correspondances,
            mes_matieres: &mes_matieres,
            chapitres: &chapitres,
            contenus: &contenus,
            etats: &etats,
            paquets_presents: &paquets_presents,
            voit_le_travail,
        });
        Ok((catalogue, !mes_matieres.is_empty()))
    }

    /// Le catalogue de la bibliothèque.
    pub async fn catalogue(&self, session: &Session) -> Resultat<CatalogueBibliotheque> {
        let (peut_ecrire, peut_valider) = self.permissions(session).await?;
        let (niveaux, a_des_matieres) = self.niveaux_du_catalogue(session, peut_ecrire).await?;
        Ok(CatalogueBibliotheque {
            niveaux,
            peut_ecrire,
            peut_valider,
            maitre: a_des_matieres && !peut_valider,
        })
    }

    /// Ouvre un contenu.
    pub async fn lire(
        &self,
        session: &Session,
        demande: &DemandeLectureContenu,
    ) -> Resultat<LectureContenu> {
        let (peut_ecrire, _) = self.permissions(session).await?;
        let contexte = self.noyau.contexte(session.etablissement_id, Some(session));
        let mut cnx = self.connexion().await?;
        let contenu = depot::contenu(&mut cnx, &demande.contenu_id, &contexte)
            .await?
            .ok_or_else(|| ErreurBase::Introuvable {
                entite: "contenu",
                id: demande.contenu_id.clone(),
            })?;
        let publiee =
            depot::version_dans_etat(&mut cnx, &demande.contenu_id, "publiee", &contexte).await?;
        let brouillon = if peut_ecrire {
            depot::version_dans_etat(&mut cnx, &demande.contenu_id, "brouillon", &contexte)
                .await?
                .filter(|(_, retour)| !retour)
                .map(|(texte, _)| texte)
        } else {
            None
        };
        let livre = self.texte_livre(&contenu);
        let version_ecole = matches!(publiee, Some((_, false)));
        let visible: Option<String> = match &publiee {
            Some((texte, false)) => Some(texte.clone()),
            _ if contenu.plus_livre => None,
            _ => livre.clone(),
        };

        let mut vues = vec![VueContenu::Visible];
        if peut_ecrire && livre.is_some() && version_ecole {
            vues.push(VueContenu::Livree);
        }
        if brouillon.is_some() {
            vues.push(VueContenu::Brouillon);
        }
        let vue = demande
            .vue
            .filter(|v| vues.contains(v))
            .unwrap_or(VueContenu::Visible);
        let texte = match vue {
            VueContenu::Visible => visible,
            VueContenu::Livree => livre,
            VueContenu::Brouillon => brouillon,
        }
        .unwrap_or_default();
        let mut lecon = analyser(&texte);
        if !peut_ecrire {
            lecon = lecon.pour_les_eleves();
        }

        let mut images = Vec::new();
        let mut empreintes = Vec::new();
        let paquet = contenu
            .paquet_code
            .as_deref()
            .and_then(|p| self.paquets.paquet(p));
        for source in lecon.images() {
            match source {
                SourceImage::Paquet { nom } => {
                    let Some(paquet) = &paquet else { continue };
                    let fichier = nom_fichier_image(&contenu.cle, nom);
                    if let Some(octets) = paquet.fichiers.get(&fichier) {
                        images.push(ImageLecon {
                            cle: nom.clone(),
                            type_mime: type_mime(nom).to_owned(),
                            donnees: base64::engine::general_purpose::STANDARD.encode(octets),
                        });
                    }
                }
                SourceImage::Ecole { empreinte } => empreintes.push(empreinte.clone()),
            }
        }
        empreintes.sort();
        empreintes.dedup();
        for (empreinte, type_mime, octets) in
            depot::images_par_empreinte(&mut cnx, &empreintes, &contexte).await?
        {
            images.push(ImageLecon {
                cle: empreinte,
                type_mime,
                donnees: base64::engine::general_purpose::STANDARD.encode(octets),
            });
        }
        images.sort_by(|a, b| a.cle.cmp(&b.cle));
        images.dedup_by(|a, b| a.cle == b.cle);

        let lie = match contenu.lie_cle.as_deref() {
            Some(cle) => depot::contenus(&mut cnx, &contexte)
                .await?
                .into_iter()
                .find(|c| c.cle == cle)
                .map(|c| ContenuLie {
                    id: c.id.vers_chaine(),
                    titre: c.titre,
                    nature: c.nature,
                }),
            None => None,
        };

        Ok(LectureContenu {
            id: contenu.id.vers_chaine(),
            titre: contenu.titre,
            nature: contenu.nature,
            examen: contenu.examen,
            session: contenu.session,
            origine: contenu.origine,
            presentation: contenu.presentation,
            vue,
            vues,
            version_ecole,
            lecon,
            images,
            lie,
        })
    }

    /// Texte livré d'un contenu, si son paquet est chargé sur ce poste.
    fn texte_livre(&self, contenu: &ContenuBibliotheque) -> Option<String> {
        let paquet = self.paquets.paquet(contenu.paquet_code.as_deref()?)?;
        paquet.texte(contenu.fichier.as_deref()?).map(str::to_owned)
    }

    /// Cherche dans les titres et le texte des versions visibles, sur ce que le catalogue montre.
    pub async fn rechercher(
        &self,
        session: &Session,
        demande: &DemandeRechercheBibliotheque,
    ) -> Resultat<Vec<ResultatRechercheBibliotheque>> {
        let requete = requete_recherche(&demande.texte);
        if requete.is_empty() {
            return Ok(Vec::new());
        }
        let (peut_ecrire, _) = self.permissions(session).await?;
        let (niveaux, _) = self.niveaux_du_catalogue(session, peut_ecrire).await?;
        let places = places_du_catalogue(&niveaux);
        let trouves = {
            let mut cnx = self.connexion().await?;
            // Plus de lignes que l'on n'en montre : une part peut viser ce que le catalogue ne
            // montre pas (niveau absent de l'école).
            depot::rechercher(&mut cnx, &requete, RESULTATS_MAX * 4).await?
        };
        let mut resultats: Vec<(usize, usize, ResultatRechercheBibliotheque)> = trouves
            .into_iter()
            .enumerate()
            .filter_map(|(pertinence, (id, extrait))| {
                let place = places.get(&id)?;
                Some((
                    place.rang,
                    pertinence,
                    ResultatRechercheBibliotheque {
                        contenu_id: id,
                        titre: place.titre.clone(),
                        nature: place.nature,
                        niveau: place.niveau.clone(),
                        matiere: place.matiere.clone(),
                        chapitre: place.chapitre.clone(),
                        extrait,
                    },
                ))
            })
            .take(RESULTATS_MAX as usize)
            .collect();
        // Groupés par niveau et matière (l'ordre du catalogue), les plus pertinents d'abord dans
        // chaque groupe.
        resultats.sort_by_key(|(rang, pertinence, _)| (*rang, *pertinence));
        Ok(resultats.into_iter().map(|(_, _, r)| r).collect())
    }
}

/// Où paraît chaque contenu du catalogue ; un contenu sous plusieurs séries garde la première.
fn places_du_catalogue(niveaux: &[NiveauCatalogue]) -> HashMap<String, Place> {
    let mut places = HashMap::new();
    let mut rang = 0;
    for n in niveaux {
        for s in &n.series {
            for m in &s.matieres {
                rang += 1;
                for p in &m.parties {
                    for c in &p.chapitres {
                        for x in &c.contenus {
                            places.entry(x.id.clone()).or_insert_with(|| Place {
                                rang,
                                niveau: n.libelle.clone(),
                                matiere: m.libelle.clone(),
                                chapitre: c.titre.clone(),
                                nature: x.nature,
                                titre: x.titre.clone(),
                            });
                        }
                    }
                }
            }
        }
    }
    places
}

/// Type MIME d'une image du paquet d'après son nom.
fn type_mime(nom: &str) -> &'static str {
    let bas = nom.to_ascii_lowercase();
    if bas.ends_with(".png") {
        "image/png"
    } else if bas.ends_with(".jpg") || bas.ends_with(".jpeg") {
        "image/jpeg"
    } else {
        "image/svg+xml"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn le_type_d_une_image_vient_de_son_nom() {
        assert_eq!(type_mime("figure-1.svg"), "image/svg+xml");
        assert_eq!(type_mime("photo.JPG"), "image/jpeg");
        assert_eq!(type_mime("carte.png"), "image/png");
    }
}
