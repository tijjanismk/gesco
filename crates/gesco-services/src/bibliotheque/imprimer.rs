//! Imprimer et enregistrer en PDF (§11 de la spécification) : une leçon, un sujet, un corrigé,
//! la liste des faits à vérifier, un chapitre entier (ses contenus visibles dans l'ordre, non
//! retirés). Toujours la version visible, sauf le brouillon pour qui écrit (« Brouillon » en
//! filigrane). Deux formes : « Leçon complète » et « Sans les corrigés ».
//!
//! L'arbre de blocs de la leçon est traduit en balisage Typst, que le modèle `lecon` évalue ;
//! tout texte y est échappé, aucun caractère du texte n'est lu comme une commande. Une formule
//! que la conversion ne prend pas s'imprime telle qu'elle est écrite. Jamais de refus : si la
//! composition échoue, elle recommence sans les images, puis avec les formules telles qu'elles
//! sont écrites. Aucune note pour le maître, aucune mention de source ni de relecture.

use std::collections::HashMap;
use std::path::Path;

use base64::Engine as _;
use gesco_core::bibliotheque::{
    Alignement, Bloc, Enligne, ExamenSujet, Lecon, NatureContenu, SourceImage, places_du_catalogue,
};
use gesco_docs::Piece;
use gesco_docs::formules::latex_vers_typst;
use serde::{Deserialize, Serialize};

use super::{DemandeLectureContenu, LectureContenu, ServiceBibliotheque, VueContenu, regle};
use crate::documents::ServiceDocuments;
use crate::erreur::Resultat;
use crate::session::Session;

/// Modèle Typst des leçons.
const MODELE: &str = "lecon";

/// Forme d'une impression.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "snake_case")]
pub enum FormeImpression {
    /// La leçon entière.
    Complete,
    /// Sans la section des corrigés ni les réponses du quiz.
    SansCorriges,
}

/// Imprimer un contenu, ou ses faits à vérifier.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
pub struct DemandeImpressionContenu {
    /// Identifiant du contenu.
    pub contenu_id: String,
    /// Le brouillon (pour qui écrit) ; sinon la version visible.
    pub brouillon: bool,
    /// Forme.
    pub forme: FormeImpression,
    /// Chemin du PDF.
    pub chemin: String,
}

/// Imprimer un chapitre entier.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
pub struct DemandeImpressionChapitre {
    /// Identifiant du chapitre.
    pub chapitre_id: String,
    /// Forme.
    pub forme: FormeImpression,
    /// Chemin du PDF.
    pub chemin: String,
}

/// Une image à joindre à la composition.
struct ImageImprimee {
    cle: String,
    extension: &'static str,
    octets: Vec<u8>,
}

/// Un contenu prêt à imprimer.
struct ContenuImprime {
    titre: String,
    sous_titre: String,
    niveau: String,
    matiere: String,
    lecon: Lecon,
    images: Vec<ImageImprimee>,
}

/// Ce qu'on imprime : un ou plusieurs contenus, chacun sur une nouvelle page.
pub(crate) struct Impression {
    titre: String,
    brouillon: bool,
    contenus: Vec<ContenuImprime>,
}

/// Ce que la traduction garde.
#[derive(Clone, Copy)]
struct Options {
    images: bool,
    formules: bool,
}

/// Échappe un texte pour le balisage Typst : chaque signe de ponctuation ASCII est précédé d'une
/// barre oblique inverse ; un retour à la ligne devient une espace.
fn echapper(texte: &str, sortie: &mut String) {
    for c in texte.chars() {
        if c == '\n' || c == '\r' {
            sortie.push(' ');
        } else {
            if c.is_ascii_punctuation() {
                sortie.push('\\');
            }
            sortie.push(c);
        }
    }
}

/// Chaîne Typst entre guillemets.
fn chaine(texte: &str) -> String {
    let mut s = String::with_capacity(texte.len() + 2);
    s.push('"');
    for c in texte.chars() {
        match c {
            '\\' => s.push_str("\\\\"),
            '"' => s.push_str("\\\""),
            '\n' => s.push_str("\\n"),
            '\r' => {}
            autre => s.push(autre),
        }
    }
    s.push('"');
    s
}

/// Une formule : convertie, ou telle qu'elle est écrite.
fn formule(latex: &str, seule: bool, options: Options, sortie: &mut String) {
    let convertie = if options.formules {
        latex_vers_typst(latex).ok().map(|t| t.trim().to_owned())
    } else {
        None
    };
    match convertie {
        Some(t) if !t.is_empty() => {
            if seule {
                sortie.push_str("$ ");
                sortie.push_str(&t);
                sortie.push_str(" $");
            } else {
                sortie.push('$');
                sortie.push_str(&t);
                sortie.push('$');
            }
        }
        _ => {
            sortie.push_str("#raw(");
            sortie.push_str(&chaine(latex.trim()));
            sortie.push_str(");");
        }
    }
}

/// Traduit des éléments dans la ligne.
fn enlignes(
    elements: &[Enligne],
    images: &HashMap<String, String>,
    options: Options,
    sortie: &mut String,
) {
    for e in elements {
        match e {
            Enligne::Texte { texte } => echapper(texte, sortie),
            Enligne::Code { texte } => {
                sortie.push_str("#raw(");
                sortie.push_str(&chaine(texte));
                sortie.push_str(");");
            }
            Enligne::Gras { contenu } => {
                sortie.push_str("#strong[");
                enlignes(contenu, images, options, sortie);
                sortie.push_str("];");
            }
            Enligne::Italique { contenu } => {
                sortie.push_str("#emph[");
                enlignes(contenu, images, options, sortie);
                sortie.push_str("];");
            }
            Enligne::Formule { latex, seule } => formule(latex, *seule, options, sortie),
            Enligne::Image {
                source,
                description,
            } => {
                let cle = match source {
                    SourceImage::Paquet { nom } => nom,
                    SourceImage::Ecole { empreinte } => empreinte,
                };
                match images.get(cle).filter(|_| options.images) {
                    Some(chemin) => {
                        sortie.push_str("#box(image(");
                        sortie.push_str(&chaine(chemin));
                        sortie.push_str(", alt: ");
                        sortie.push_str(&chaine(description));
                        sortie.push_str("));");
                    }
                    None => echapper(description, sortie),
                }
            }
            Enligne::Video { titre, .. } => {
                sortie.push_str("#emph[Vidéo : ");
                echapper(titre, sortie);
                sortie.push_str("];");
            }
            Enligne::Saut => sortie.push_str("\\\n"),
        }
    }
}

/// Vrai si un paragraphe ne porte que des images (une figure) : on la centre.
fn est_figure(contenu: &[Enligne]) -> bool {
    contenu.iter().any(|e| matches!(e, Enligne::Image { .. }))
        && contenu.iter().all(|e| match e {
            Enligne::Image { .. } | Enligne::Saut => true,
            Enligne::Texte { texte } => texte.trim().is_empty(),
            _ => false,
        })
}

/// Traduit des blocs en balisage Typst.
fn blocs(liste: &[Bloc], images: &HashMap<String, String>, options: Options, sortie: &mut String) {
    for bloc in liste {
        match bloc {
            Bloc::Titre { niveau, contenu } => {
                // Le titre de la leçon est au niveau 1 ; ses titres viennent en dessous.
                for _ in 0..=(*niveau).clamp(1, 3) {
                    sortie.push('=');
                }
                sortie.push(' ');
                enlignes(contenu, images, options, sortie);
            }
            Bloc::Paragraphe { contenu } if est_figure(contenu) => {
                sortie.push_str("#align(center)[");
                enlignes(contenu, images, options, sortie);
                sortie.push_str("];");
            }
            Bloc::Paragraphe { contenu } => enlignes(contenu, images, options, sortie),
            Bloc::Formule { latex } => formule(latex, true, options, sortie),
            Bloc::Liste {
                numerotee,
                debut,
                elements,
            } => {
                if *numerotee {
                    sortie.push_str(&format!("#enum(start: {}", debut.unwrap_or(1)));
                } else {
                    sortie.push_str("#list(");
                }
                for (i, element) in elements.iter().enumerate() {
                    if i > 0 || *numerotee {
                        sortie.push_str(", ");
                    }
                    sortie.push('[');
                    blocs(element, images, options, sortie);
                    sortie.push(']');
                }
                sortie.push_str(");");
            }
            Bloc::Tableau {
                alignements,
                entete,
                lignes,
            } => {
                let colonnes = entete
                    .len()
                    .max(lignes.iter().map(Vec::len).max().unwrap_or(0))
                    .max(1);
                sortie.push_str("#table(columns: (");
                for _ in 0..colonnes {
                    sortie.push_str("1fr, ");
                }
                sortie.push_str("), align: (");
                for i in 0..colonnes {
                    sortie.push_str(match alignements.get(i) {
                        Some(Alignement::Centre) => "center, ",
                        Some(Alignement::Droite) => "right, ",
                        _ => "left, ",
                    });
                }
                sortie.push(')');
                if !entete.is_empty() {
                    sortie.push_str(", table.header(");
                    for cellule in entete {
                        sortie.push_str("[#strong[");
                        enlignes(cellule, images, options, sortie);
                        sortie.push_str("]], ");
                    }
                    sortie.push(')');
                }
                for ligne in lignes {
                    for i in 0..colonnes {
                        sortie.push_str(", [");
                        if let Some(cellule) = ligne.get(i) {
                            enlignes(cellule, images, options, sortie);
                        }
                        sortie.push(']');
                    }
                }
                sortie.push_str(");");
            }
            Bloc::Citation { blocs: dedans } => {
                sortie.push_str("#quote(block: true)[");
                blocs(dedans, images, options, sortie);
                sortie.push_str("];");
            }
            // Les notes pour le maître ne s'impriment jamais.
            Bloc::NoteMaitre { .. } => continue,
            Bloc::TexteBrut { texte } => {
                sortie.push_str("#raw(block: true, ");
                sortie.push_str(&chaine(texte));
                sortie.push_str(");");
            }
            Bloc::Filet => sortie.push_str("#line(length: 100%, stroke: 0.5pt);"),
        }
        sortie.push_str("\n\n");
    }
}

/// Balisage Typst d'une leçon.
fn balisage(lecon: &Lecon, images: &HashMap<String, String>, options: Options) -> String {
    let mut sortie = String::new();
    blocs(&lecon.blocs, images, options, &mut sortie);
    sortie
}

/// Ce qui suit le titre : « Sujet du DEF 2024 », « Corrigé du BAC 2023 ».
fn sous_titre(lecture: &LectureContenu) -> String {
    let examen = match lecture.examen {
        Some(ExamenSujet::Def) => "DEF",
        Some(ExamenSujet::Bac) => "BAC",
        None => "",
    };
    let session = lecture.session.map(|s| format!(" {s}")).unwrap_or_default();
    match (lecture.nature, examen.is_empty()) {
        (NatureContenu::Sujet, false) => format!("Sujet du {examen}{session}"),
        (NatureContenu::Corrige, false) => format!("Corrigé du {examen}{session}"),
        (NatureContenu::Corrige, true) => "Corrigé".to_owned(),
        _ => String::new(),
    }
}

/// Extension d'une image d'après son type.
fn extension(type_mime: &str) -> &'static str {
    match type_mime {
        "image/png" => "png",
        "image/jpeg" => "jpg",
        _ => "svg",
    }
}

impl Impression {
    /// Données et pièces du modèle, avec ou sans images et formules converties.
    fn donnees(&self, options: Options) -> (serde_json::Value, Vec<Piece>) {
        let mut pieces = Vec::new();
        let mut documents = Vec::new();
        for (n, c) in self.contenus.iter().enumerate() {
            let mut chemins = HashMap::new();
            if options.images {
                for (i, image) in c.images.iter().enumerate() {
                    let piece = Piece {
                        nom: format!("image-{n}-{i}.{}", image.extension),
                        octets: image.octets.clone(),
                    };
                    chemins.insert(image.cle.clone(), piece.chemin());
                    pieces.push(piece);
                }
            }
            documents.push(serde_json::json!({
                "titre": c.titre,
                "sous_titre": c.sous_titre,
                "niveau": c.niveau,
                "matiere": c.matiere,
                "corps": balisage(&c.lecon, &chemins, options),
            }));
        }
        (
            serde_json::json!({
                "titre": self.titre,
                "brouillon": self.brouillon,
                "documents": documents,
            }),
            pieces,
        )
    }

    /// Compose le PDF. Si la composition échoue, recommence sans les images, puis avec les
    /// formules telles qu'elles sont écrites : une leçon s'imprime toujours. Chaque reprise est
    /// notée par `noter` (journal technique), pour qu'une erreur du modèle ne passe pas inaperçue.
    pub(crate) async fn composer(
        &self,
        documents: &ServiceDocuments<'_>,
        session: &Session,
        noter: &(dyn Fn(&str) + Sync),
    ) -> Resultat<Vec<u8>> {
        let essais = [
            Options {
                images: true,
                formules: true,
            },
            Options {
                images: false,
                formules: true,
            },
            Options {
                images: false,
                formules: false,
            },
        ];
        let mut derniere = None;
        for options in essais {
            let (donnees, pieces) = self.donnees(options);
            match documents
                .composer_en_memoire_avec(session.etablissement_id, MODELE, donnees, pieces)
                .await
            {
                Ok(pdf) => return Ok(pdf),
                Err(e) => {
                    noter(&format!(
                        "bibliotheque impression reprise (images {}, formules {}) : {e:?}",
                        options.images, options.formules
                    ));
                    derniere = Some(e);
                }
            }
        }
        Err(derniere.unwrap_or(regle("bibliotheque.impression_impossible")))
    }
}

impl ServiceBibliotheque<'_> {
    /// Un contenu lu, prêt à imprimer.
    fn imprime(
        lecture: LectureContenu,
        niveau: &str,
        matiere: &str,
        forme: FormeImpression,
    ) -> ContenuImprime {
        let mut lecon = lecture.lecon.pour_les_eleves();
        if forme == FormeImpression::SansCorriges {
            lecon = lecon.sans_les_corriges();
        }
        let sous_titre = sous_titre(&lecture);
        let images = lecture
            .images
            .into_iter()
            .filter_map(|i| {
                let octets = base64::engine::general_purpose::STANDARD
                    .decode(&i.donnees)
                    .ok()?;
                Some(ImageImprimee {
                    extension: extension(&i.type_mime),
                    cle: i.cle,
                    octets,
                })
            })
            .collect();
        ContenuImprime {
            titre: lecture.titre,
            sous_titre,
            niveau: niveau.to_owned(),
            matiere: matiere.to_owned(),
            lecon,
            images,
        }
    }

    /// Niveau et matière d'un contenu au catalogue (vides s'il n'y paraît pas).
    async fn place_de(&self, session: &Session, contenu_id: &str) -> Resultat<(String, String)> {
        let (peut_ecrire, _) = self.permissions(session).await?;
        let range = self.ranger_catalogue(session, peut_ecrire, true).await?;
        Ok(places_du_catalogue(&range.niveaux, &range.contenus)
            .into_iter()
            .find(|p| p.contenu_id == contenu_id)
            .map(|p| (p.niveau, p.matiere))
            .unwrap_or_default())
    }

    /// Ce qu'on imprime d'un contenu : la version visible, ou le brouillon pour qui écrit.
    pub(crate) async fn impression_contenu(
        &self,
        session: &Session,
        demande: &DemandeImpressionContenu,
    ) -> Resultat<Impression> {
        let (niveau, matiere) = self.place_de(session, &demande.contenu_id).await?;
        let lecture = self
            .lire(
                session,
                &DemandeLectureContenu {
                    contenu_id: demande.contenu_id.clone(),
                    vue: Some(if demande.brouillon {
                        VueContenu::Brouillon
                    } else {
                        VueContenu::Visible
                    }),
                },
            )
            .await?;
        let brouillon = lecture.vue == VueContenu::Brouillon;
        let titre = lecture.titre.clone();
        Ok(Impression {
            titre,
            brouillon,
            contenus: vec![Self::imprime(lecture, &niveau, &matiere, demande.forme)],
        })
    }

    /// La liste des faits à vérifier d'un contenu, pour qui écrit.
    pub(crate) async fn impression_faits(
        &self,
        session: &Session,
        demande: &DemandeImpressionContenu,
    ) -> Resultat<Impression> {
        let (niveau, matiere) = self.place_de(session, &demande.contenu_id).await?;
        let lecture = self
            .lire(
                session,
                &DemandeLectureContenu {
                    contenu_id: demande.contenu_id.clone(),
                    vue: None,
                },
            )
            .await?;
        let faits = lecture.faits.clone().unwrap_or_default();
        let titre = format!("Faits à vérifier — {}", lecture.titre);
        Ok(Impression {
            titre: titre.clone(),
            brouillon: false,
            contenus: vec![ContenuImprime {
                titre,
                sous_titre: String::new(),
                niveau,
                matiere,
                lecon: faits,
                images: Vec::new(),
            }],
        })
    }

    /// Ce qu'on imprime d'un chapitre : ses contenus visibles, dans l'ordre du catalogue, sans
    /// ceux qui sont retirés ; sans les documents ; sans les corrigés dans la forme « Sans les
    /// corrigés ».
    pub(crate) async fn impression_chapitre(
        &self,
        session: &Session,
        demande: &DemandeImpressionChapitre,
    ) -> Resultat<Impression> {
        let (peut_ecrire, _) = self.permissions(session).await?;
        let range = self.ranger_catalogue(session, peut_ecrire, true).await?;
        let mut trouve = None;
        'niveaux: for n in &range.niveaux {
            for s in &n.series {
                for m in &s.matieres {
                    for p in &m.parties {
                        if let Some(c) = p.chapitres.iter().find(|c| c.id == demande.chapitre_id) {
                            trouve = Some((n.libelle.clone(), m.libelle.clone(), c.clone()));
                            break 'niveaux;
                        }
                    }
                }
            }
        }
        let (niveau, matiere, chapitre) =
            trouve.ok_or(regle("bibliotheque.chapitre_introuvable"))?;
        let mut contenus = Vec::new();
        for c in &chapitre.contenus {
            if c.nature == NatureContenu::Document
                || (demande.forme == FormeImpression::SansCorriges
                    && c.nature == NatureContenu::Corrige)
            {
                continue;
            }
            let lecture = self
                .lire(
                    session,
                    &DemandeLectureContenu {
                        contenu_id: c.id.clone(),
                        vue: Some(VueContenu::Visible),
                    },
                )
                .await?;
            if lecture.lecon.blocs.is_empty() {
                continue;
            }
            contenus.push(Self::imprime(lecture, &niveau, &matiere, demande.forme));
        }
        Ok(Impression {
            titre: chapitre.titre,
            brouillon: false,
            contenus,
        })
    }
}

/// Écrit le PDF au chemin demandé.
pub(crate) fn ecrire_pdf(chemin: &str, pdf: &[u8]) -> Resultat<()> {
    crate::espace::ecrire_atomiquement(Path::new(chemin), pdf)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use gesco_core::bibliotheque::analyser;

    use super::*;

    const TOUT: Options = Options {
        images: true,
        formules: true,
    };

    #[test]
    fn le_texte_est_echappe_et_ne_devient_jamais_une_commande() {
        let lecon = analyser("Un #set et $ 2 * 3 = 6 $ ; voir [x](https://exemple.org) // fin");
        let typst = balisage(&lecon, &HashMap::new(), TOUT);
        assert!(typst.contains("Un \\#set"));
        assert!(typst.contains("\\/\\/ fin"));
        assert!(!typst.contains("https:"));
    }

    #[test]
    fn une_formule_convertie_devient_des_mathematiques_sinon_du_texte_brut() {
        let lecon = analyser("Soit $\\frac{1}{2}$ et $\\commandeinconnue{x}$.");
        let typst = balisage(&lecon, &HashMap::new(), TOUT);
        assert!(typst.contains('$'));
        assert!(typst.contains("#raw(\"\\\\commandeinconnue{x}\");"));
        let sans = balisage(
            &lecon,
            &HashMap::new(),
            Options {
                images: true,
                formules: false,
            },
        );
        assert!(!sans.contains('$'));
    }

    #[test]
    fn titres_listes_tableaux_et_videos_se_traduisent() {
        let video = "a".repeat(64);
        let lecon = analyser(&format!(
            "## Partie\n\n- un\n- deux\n\n| a | b |\n|:-:|--:|\n| 1 | 2 |\n\n![Ma vidéo](video:{video})"
        ));
        let typst = balisage(&lecon, &HashMap::new(), TOUT);
        assert!(typst.contains("== Partie"));
        assert!(typst.contains("#list(["));
        assert!(typst.contains("align: (center, right, )"));
        assert!(typst.contains("Vidéo : Ma vidéo"));
    }

    #[test]
    fn une_image_absente_laisse_sa_description_et_une_note_pour_le_maitre_ne_s_imprime_pas() {
        let lecon = analyser(
            "![Le triangle](figure-1.svg)\n\n> Note pour le maître : à montrer au tableau.",
        );
        let mut images = HashMap::new();
        let typst = balisage(&lecon, &images, TOUT);
        assert!(typst.contains("Le triangle"));
        assert!(!typst.contains("tableau"));
        images.insert(
            "figure-1.svg".to_owned(),
            "/pieces/image-0-0.svg".to_owned(),
        );
        let typst = balisage(&lecon, &images, TOUT);
        assert!(typst.contains("#align(center)[#box(image(\"/pieces/image-0-0.svg\""));
    }
}
