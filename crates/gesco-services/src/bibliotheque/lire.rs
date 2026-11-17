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
    CibleRetrait, ContenuBibliotheque, EntreesCatalogue, EtatContenu, ExamenSujet, GrainRetrait,
    Lecon, MatiereReference, NatureContenu, NiveauCatalogue, NiveauReference, OrigineBibliotheque,
    PlaceContenu, SerieReference, SourceImage, analyser, construire_catalogue, places_du_catalogue,
    requete_recherche,
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
    /// L'utilisateur gère la bibliothèque (`library:manage`) : il retire et remet.
    pub peut_gerer: bool,
    /// Compte d'un maître qui ne valide pas : l'écran s'ouvre sur « Mes matières ».
    pub maitre: bool,
    /// L'utilisateur écrit dans toutes les matières ; sinon, dans celles marquées « mienne ».
    pub ecrit_partout: bool,
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
    /// La version soumise, en attente de validation.
    Soumise,
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
    /// L'utilisateur peut écrire ce contenu (ses matières pour un maître).
    pub peut_modifier: bool,
    /// Faits à vérifier joints par la bibliothèque, pour le seul personnel qui écrit.
    pub faits: Option<Lecon>,
    /// Vidéos appelées par le texte, et leur présence sur le poste principal.
    pub videos: Vec<super::VideoLecon>,
    /// Sons appelés par le texte, et leur présence dans la base.
    pub sons: Vec<super::SonLecon>,
    /// Classes de l'année auxquelles ce contenu de l'école est réservé (vide : tout le niveau).
    pub classes_seulement: Vec<String>,
    /// Fichiers d'un document de l'école, dans l'ordre.
    pub fichiers: Vec<super::FichierDocument>,
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

/// Le catalogue rangé, avec ce qui a servi à le ranger.
pub(crate) struct CatalogueRange {
    pub(crate) niveaux: Vec<NiveauCatalogue>,
    pub(crate) contenus: Vec<ContenuBibliotheque>,
    pub(crate) chapitres: Vec<gesco_core::bibliotheque::ChapitreBibliotheque>,
    pub(super) a_des_matieres: bool,
}

impl ServiceBibliotheque<'_> {
    pub(super) async fn permissions(&self, session: &Session) -> Resultat<(bool, bool)> {
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

    /// Le catalogue rangé pour l'utilisateur ; sans les retraits, pour dire ce qu'un retrait
    /// cache.
    pub(super) async fn ranger_catalogue(
        &self,
        session: &Session,
        voit_le_travail: bool,
        avec_retraits: bool,
    ) -> Resultat<CatalogueRange> {
        let contexte = self.noyau.contexte(session.etablissement_id, Some(session));
        self.ranger_catalogue_dans(
            &contexte,
            Some(session.utilisateur_id.vers_chaine()),
            voit_le_travail,
            avec_retraits,
        )
        .await
    }

    /// Le catalogue que voient les élèves (portail, lot 4) : versions visibles seulement,
    /// retraits appliqués, sans compte du personnel.
    pub(crate) async fn catalogue_des_eleves(
        &self,
        etablissement_id: gesco_core::commun::Id<gesco_core::core::Etablissement>,
    ) -> Resultat<CatalogueRange> {
        let contexte = self.noyau.contexte(etablissement_id, None);
        self.ranger_catalogue_dans(&contexte, None, false, true)
            .await
    }

    async fn ranger_catalogue_dans(
        &self,
        contexte: &gesco_db::contexte::Contexte,
        utilisateur_id: Option<String>,
        voit_le_travail: bool,
        avec_retraits: bool,
    ) -> Resultat<CatalogueRange> {
        let contexte = contexte.clone();
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
        let mes_matieres: HashSet<(String, String)> = match &utilisateur_id {
            Some(u) => depot::matieres_du_compte(&mut cnx, u, &contexte)
                .await?
                .into_iter()
                .collect(),
            None => HashSet::new(),
        };
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
        let retraits: Vec<CibleRetrait> = if avec_retraits {
            depot::retraits(&mut cnx, true, &contexte)
                .await?
                .into_iter()
                .filter_map(|r| {
                    Some(CibleRetrait {
                        grain: GrainRetrait::depuis_nom(&r.grain)?,
                        cible: r.cible,
                        niveau_code: r.niveau_code,
                        serie_code: r.serie_code,
                    })
                })
                .collect()
        } else {
            Vec::new()
        };
        let niveaux = construire_catalogue(&EntreesCatalogue {
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
            retraits: &retraits,
        });
        Ok(CatalogueRange {
            niveaux,
            contenus,
            chapitres,
            a_des_matieres: !mes_matieres.is_empty(),
        })
    }

    /// Le catalogue de la bibliothèque.
    pub async fn catalogue(&self, session: &Session) -> Resultat<CatalogueBibliotheque> {
        let (peut_ecrire, peut_valider) = self.permissions(session).await?;
        let peut_gerer = self.peut_gerer(session).await?;
        let mut range = self.ranger_catalogue(session, peut_ecrire, true).await?;
        let ecrit_partout = peut_ecrire && self.portee(session).await?.partout();
        // La marque « 9e A seulement » : le personnel voit tout, avec la marque.
        let marques = {
            let contexte = self.noyau.contexte(session.etablissement_id, Some(session));
            let mut cnx = self.connexion().await?;
            Self::marques_classes(&mut cnx, &contexte).await?
        };
        if !marques.is_empty() {
            for chapitre in range
                .niveaux
                .iter_mut()
                .flat_map(|n| n.series.iter_mut())
                .flat_map(|s| s.matieres.iter_mut())
                .flat_map(|m| m.parties.iter_mut())
                .flat_map(|p| p.chapitres.iter_mut())
            {
                if let Some(m) = marques.get(&chapitre.id) {
                    chapitre.classes_seulement.clone_from(m);
                }
                for contenu in &mut chapitre.contenus {
                    if let Some(m) = marques.get(&contenu.id) {
                        contenu.classes_seulement.clone_from(m);
                    }
                }
            }
        }
        Ok(CatalogueBibliotheque {
            niveaux: range.niveaux,
            peut_ecrire,
            peut_valider,
            peut_gerer,
            maitre: range.a_des_matieres && !peut_valider,
            ecrit_partout,
        })
    }

    /// Vrai si l'utilisateur gère la bibliothèque (`library:manage`).
    pub(super) async fn peut_gerer(&self, session: &Session) -> Resultat<bool> {
        let contexte = self.noyau.contexte(session.etablissement_id, Some(session));
        let mut cnx = self.connexion().await?;
        Ok(self
            .noyau
            .permissions_de(&mut cnx, session.utilisateur_id, &contexte)
            .await?
            .contains(&Permission::LIBRARY_MANAGE))
    }

    /// Ouvre un contenu.
    pub async fn lire(
        &self,
        session: &Session,
        demande: &DemandeLectureContenu,
    ) -> Resultat<LectureContenu> {
        let (peut_ecrire, peut_valider) = self.permissions(session).await?;
        let portee = if peut_ecrire {
            Some(self.portee(session).await?)
        } else {
            None
        };
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
        let peut_modifier = match &portee {
            Some(p) => Self::peut_modifier(&mut cnx, p, &contenu, &contexte).await?,
            None => false,
        };
        // Une version soumise se montre à qui valide et à qui écrit ce contenu ; pas à un maître
        // hors de ses matières.
        let soumise = if peut_valider || peut_modifier {
            depot::version_dans_etat(&mut cnx, &demande.contenu_id, "soumise", &contexte)
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
        if soumise.is_some() {
            vues.push(VueContenu::Soumise);
        }
        let vue = demande
            .vue
            .filter(|v| vues.contains(v))
            .unwrap_or(VueContenu::Visible);
        let texte = match vue {
            VueContenu::Visible => visible,
            VueContenu::Livree => livre,
            VueContenu::Brouillon => brouillon,
            VueContenu::Soumise => soumise,
        }
        .unwrap_or_default();
        let mut lecon = analyser(&texte);
        if !peut_ecrire {
            lecon = lecon.pour_les_eleves();
        }

        let images = self
            .images_de(&mut cnx, Some(&contenu), &lecon, &contexte)
            .await?;

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

        let faits = if peut_ecrire {
            self.faits_livres(&contenu).map(|f| analyser(&f))
        } else {
            None
        };
        let empreintes: Vec<String> = lecon.videos().into_iter().map(str::to_owned).collect();
        let videos = self.videos_de(&mut cnx, &empreintes, &contexte).await?;
        let empreintes: Vec<String> = lecon.sons().into_iter().map(str::to_owned).collect();
        let sons = self.sons_de(&mut cnx, &empreintes, &contexte).await?;
        let marques = Self::marques_classes(&mut cnx, &contexte).await?;
        let classes_seulement = marques
            .get(&demande.contenu_id)
            .or_else(|| marques.get(&contenu.chapitre_id.vers_chaine()))
            .cloned()
            .unwrap_or_default();
        let fichiers = if contenu.nature == NatureContenu::Document {
            depot::fichiers_du_contenu(&mut cnx, &demande.contenu_id, &contexte)
                .await?
                .into_iter()
                .map(|f| super::FichierDocument {
                    lien_id: f.lien_id,
                    empreinte: f.empreinte_sha256,
                    nom: f.nom_fichier,
                    type_mime: f.type_mime,
                    taille: f.taille,
                })
                .collect()
        } else {
            Vec::new()
        };
        Ok(LectureContenu {
            faits,
            videos,
            sons,
            classes_seulement,
            fichiers,
            peut_modifier,
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

    /// Images appelées par une leçon : celles du paquet du contenu, et celles de l'école par leur
    /// empreinte. Une image introuvable est passée (la leçon s'ouvre sans elle).
    pub(super) async fn images_de(
        &self,
        cnx: &mut gesco_db::Connexion,
        contenu: Option<&ContenuBibliotheque>,
        lecon: &Lecon,
        contexte: &gesco_db::contexte::Contexte,
    ) -> Resultat<Vec<ImageLecon>> {
        let mut images = Vec::new();
        let mut empreintes = Vec::new();
        let paquet = contenu
            .and_then(|c| c.paquet_code.as_deref())
            .and_then(|p| self.paquets.paquet(p));
        for source in lecon.images() {
            match source {
                SourceImage::Paquet { nom } => {
                    let (Some(paquet), Some(contenu)) = (&paquet, contenu) else {
                        continue;
                    };
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
            depot::images_par_empreinte(cnx, &empreintes, contexte).await?
        {
            images.push(ImageLecon {
                cle: empreinte,
                type_mime,
                donnees: base64::engine::general_purpose::STANDARD.encode(octets),
            });
        }
        images.sort_by(|a, b| a.cle.cmp(&b.cle));
        images.dedup_by(|a, b| a.cle == b.cle);
        Ok(images)
    }

    /// Texte livré d'un contenu, si son paquet est chargé sur ce poste.
    /// Faits à vérifier que le paquet joint au contenu.
    fn faits_livres(&self, contenu: &ContenuBibliotheque) -> Option<String> {
        let paquet = self.paquets.paquet(contenu.paquet_code.as_deref()?)?;
        paquet
            .texte(contenu.faits.as_deref()?)
            .filter(|f| !f.trim().is_empty())
            .map(str::to_owned)
    }

    pub(crate) fn texte_livre(&self, contenu: &ContenuBibliotheque) -> Option<String> {
        let paquet = self.paquets.paquet(contenu.paquet_code.as_deref()?)?;
        paquet.texte(contenu.fichier.as_deref()?).map(str::to_owned)
    }

    /// Octets d'une image du paquet appelée par un contenu livré (`nom` tel que la leçon
    /// l'écrit) ; aucun si le paquet n'est pas lu sur ce poste.
    pub(crate) fn image_du_paquet(
        &self,
        paquet_code: &str,
        contenu_cle: &str,
        nom: &str,
    ) -> Option<Vec<u8>> {
        let paquet = self.paquets.paquet(paquet_code)?;
        paquet
            .fichiers
            .get(&nom_fichier_image(contenu_cle, nom))
            .cloned()
    }

    /// Dossier des vidéos de l'école sur ce poste.
    pub(crate) fn dossier_videos(&self) -> std::path::PathBuf {
        self.espace.videos()
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
        let range = self.ranger_catalogue(session, peut_ecrire, true).await?;
        // Chaque contenu garde sa première place ; le rang est celui du groupe niveau, série et
        // matière dans le catalogue.
        let mut groupes: Vec<(String, Option<String>, String)> = Vec::new();
        let mut places: HashMap<String, (usize, PlaceContenu)> = HashMap::new();
        for place in places_du_catalogue(&range.niveaux, &range.contenus) {
            let groupe = (
                place.niveau_code.clone(),
                place.serie_code.clone(),
                place.matiere_cle.clone(),
            );
            let rang = match groupes.iter().position(|g| g == &groupe) {
                Some(i) => i,
                None => {
                    groupes.push(groupe);
                    groupes.len() - 1
                }
            };
            places
                .entry(place.contenu_id.clone())
                .or_insert((rang, place));
        }
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
                let (rang, place) = places.get(&id)?;
                Some((
                    *rang,
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

/// Type MIME d'une image du paquet d'après son nom.
pub(crate) fn type_mime(nom: &str) -> &'static str {
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
