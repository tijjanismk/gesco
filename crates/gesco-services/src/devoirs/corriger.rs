//! Corriger un devoir (SPEC_PHASE_12_LOT_5 §5) : la liste des élèves de la classe avec leur état,
//! la copie de chacun (texte, photos, historique des rendus), la note sur le barème et le
//! commentaire, le rendu sur papier, « Rendre les copies corrigées » et la liste des notes à
//! imprimer.
//!
//! La note et le commentaire sont portés par le dernier rendu de l'élève : un rendu arrivé après
//! la correction arrive sans note (« à corriger »), l'ancienne note restant sur le rendu précédent.

use std::collections::HashMap;

use base64::Engine as _;
use gesco_core::bibliotheque::{Alignement, Bloc, Enligne, Lecon};
use gesco_core::core::Operation;
use gesco_core::pedagogie::SaisieNote;
use gesco_core::pedagogie::evaluation::lire_saisie;
use gesco_db::depots::devoirs::{self as depot, LigneDevoir, LigneRendu};
use gesco_db::depots::portail as depot_portail;
use gesco_db::unite::Durabilite;
use gesco_db::{ErreurBase, nouvel_id};
use serde::{Deserialize, Serialize};

use super::{DevoirAffiche, EtatDevoir, ServiceDevoirs, regle};
use crate::bibliotheque::Impression;
use crate::erreur::{ErreurService, Resultat};
use crate::session::Session;

/// Où en est un élève pour un devoir.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "snake_case")]
pub enum EtatCopie {
    /// Rendu par le portail avant la date limite.
    Rendu,
    /// Rendu par le portail après la date limite.
    EnRetard,
    /// Rendu sur papier, marqué par le maître.
    SurPapier,
    /// Rien n'est rendu.
    PasRendu,
}

/// Un élève de la classe, dans la liste des copies.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, specta::Type)]
pub struct LigneCopie {
    /// Élève.
    pub eleve_id: String,
    /// Nom et prénoms.
    pub nom: String,
    /// État de son dernier rendu.
    pub etat: EtatCopie,
    /// Dernier rendu le.
    pub rendu_le: Option<i64>,
    /// Nombre de rendus (le dernier compte).
    pub rendus: u32,
    /// Note du dernier rendu, en centièmes.
    pub note: Option<i32>,
    /// Commentaire du dernier rendu.
    pub commentaire: String,
    /// Rendu et pas encore noté.
    pub a_corriger: bool,
}

/// Les copies d'un devoir : un élève par ligne, par ordre alphabétique.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, specta::Type)]
pub struct CopiesDevoir {
    /// Devoir.
    pub devoir_id: String,
    /// Barème en centièmes.
    pub bareme: i32,
    /// Le compte peut noter, marquer un rendu sur papier, rendre les copies.
    pub peut_corriger: bool,
    /// Copies corrigées rendues aux élèves le.
    pub copies_rendues_le: Option<i64>,
    /// Élèves de la classe.
    pub lignes: Vec<LigneCopie>,
}

/// Un élève d'un devoir.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
pub struct DemandeCopie {
    /// Devoir.
    pub devoir_id: String,
    /// Élève.
    pub eleve_id: String,
}

/// Une photo d'un rendu, désignée.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, specta::Type)]
pub struct PhotoCopie {
    /// Rendu.
    pub rendu_id: String,
    /// Ordre dans le rendu (1 à 5).
    pub ordre: i32,
    /// Place libérée : la photo n'est plus là.
    pub retiree: bool,
}

/// Un rendu d'un élève, pour le maître.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, specta::Type)]
pub struct RenduAffiche {
    /// Identifiant.
    pub id: String,
    /// Numéro (le plus grand compte).
    pub numero: u32,
    /// Rendu sur papier, marqué par le maître.
    pub sur_papier: bool,
    /// Texte rendu.
    pub texte: String,
    /// Rendu le.
    pub rendu_le: i64,
    /// Après la date limite.
    pub en_retard: bool,
    /// Note en centièmes.
    pub note: Option<i32>,
    /// Commentaire.
    pub commentaire: String,
    /// Photos, dans l'ordre.
    pub photos: Vec<PhotoCopie>,
}

/// La copie d'un élève : ses rendus, du premier au dernier.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, specta::Type)]
pub struct CopieEleve {
    /// Élève.
    pub eleve_id: String,
    /// Nom et prénoms.
    pub nom: String,
    /// Rendus, du premier au dernier ; le dernier compte.
    pub rendus: Vec<RenduAffiche>,
}

/// Une photo d'une copie demandée par le maître.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
pub struct DemandePhotoCopie {
    /// Devoir.
    pub devoir_id: String,
    /// Rendu.
    pub rendu_id: String,
    /// Ordre dans le rendu.
    pub ordre: i32,
}

/// Une photo servie : absente quand la place a été libérée.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, specta::Type)]
pub struct PhotoCopieServie {
    /// Type MIME.
    pub type_mime: String,
    /// Contenu en base64 ; `None` : photo retirée pour libérer de la place.
    pub donnees: Option<String>,
}

/// La note et le commentaire du maître pour un élève.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
pub struct DemandeNoteCopie {
    /// Devoir.
    pub devoir_id: String,
    /// Élève.
    pub eleve_id: String,
    /// Note tapée (« 13,5 ») ; vide : pas de note.
    pub note: String,
    /// Commentaire.
    pub commentaire: String,
}

/// Commentaire le plus long, en caractères.
pub const COMMENTAIRE_MAX: usize = 2_000;

fn nom_eleve(nom: &str, prenoms: &str) -> String {
    format!("{nom} {prenoms}").trim().to_owned()
}

/// État d'un élève d'après son dernier rendu.
pub(crate) fn etat_copie(dernier: Option<&LigneRendu>) -> EtatCopie {
    match dernier {
        None => EtatCopie::PasRendu,
        Some(r) if r.origine == "papier" => EtatCopie::SurPapier,
        Some(r) if r.en_retard => EtatCopie::EnRetard,
        Some(_) => EtatCopie::Rendu,
    }
}

/// Note du maître lue comme dans la grille des notes : un nombre sur le barème, au centième ;
/// vide, pas de note. Les lettres de la grille (absent, dispensé) n'ont pas de sens ici.
pub(crate) fn lire_note(texte: &str, bareme: i32) -> Resultat<Option<i64>> {
    match lire_saisie(texte, bareme).map_err(regle)? {
        SaisieNote::Valeur(v) => Ok(Some(i64::from(v))),
        SaisieNote::Effacee => Ok(None),
        SaisieNote::Absent | SaisieNote::AbsentJustifie | SaisieNote::Dispense => {
            Err(regle("devoir.note_lettre"))
        }
    }
}

/// Points lisibles : « 13,5 », « 20 ».
pub(crate) fn points(centiemes: i64) -> String {
    let entier = centiemes / 100;
    let reste = centiemes % 100;
    if reste == 0 {
        format!("{entier}")
    } else if reste % 10 == 0 {
        format!("{entier},{}", reste / 10)
    } else {
        format!("{entier},{reste:02}")
    }
}

/// Le dernier rendu de chaque élève (plus grand numéro), et leur nombre.
fn derniers(rendus: &[LigneRendu]) -> HashMap<&str, (&LigneRendu, u32)> {
    let mut par_eleve: HashMap<&str, (&LigneRendu, u32)> = HashMap::new();
    for r in rendus {
        let entree = par_eleve.entry(r.eleve_id.as_str()).or_insert((r, 0));
        entree.1 += 1;
        if r.numero > entree.0.numero {
            entree.0 = r;
        }
    }
    par_eleve
}

fn note_i32(note: Option<i64>) -> Option<i32> {
    note.map(|n| i32::try_from(n).unwrap_or(i32::MAX))
}

impl ServiceDevoirs<'_> {
    /// Le devoir permis, déjà donné, que ce compte peut corriger ; l'année de sa classe doit
    /// être ouverte.
    async fn devoir_a_corriger(&self, session: &Session, id: &str) -> Resultat<LigneDevoir> {
        self.exiger_module(session).await?;
        let (devoir, portee) = self.devoir_permis(session, id).await?;
        if !portee.peut_donner {
            return Err(ErreurService::PorteeRefusee {
                ressource: format!("devoir {id}"),
            });
        }
        if EtatDevoir::depuis_nom(&devoir.etat) == EtatDevoir::Brouillon {
            return Err(regle("devoir.pas_donne"));
        }
        let classe = gesco_core::commun::Id::analyser(&devoir.classe_id).map_err(|_| {
            ErreurService::from(ErreurBase::Introuvable {
                entite: "classe",
                id: devoir.classe_id.clone(),
            })
        })?;
        crate::annees::exiger_classe_modifiable(self.noyau, session, classe).await?;
        Ok(devoir)
    }

    /// Les élèves de la classe du devoir et leur copie.
    pub async fn copies(&self, session: &Session, id: &str) -> Resultat<CopiesDevoir> {
        self.exiger_module(session).await?;
        let (devoir, portee) = self.devoir_permis(session, id).await?;
        let contexte = self.contexte(session);
        let mut cnx = self.connexion().await?;
        let eleves =
            depot_portail::eleves_de_la_classe(&mut cnx, &devoir.classe_id, &contexte).await?;
        let rendus = depot::rendus_du_devoir(&mut cnx, &devoir.id, &contexte).await?;
        let derniers = derniers(&rendus);
        let lignes = eleves
            .into_iter()
            .map(|e| {
                let dernier = derniers.get(e.eleve_id.as_str());
                let rendu = dernier.map(|(r, _)| *r);
                LigneCopie {
                    nom: nom_eleve(&e.nom, &e.prenoms),
                    etat: etat_copie(rendu),
                    rendu_le: rendu.map(|r| r.rendu_le),
                    rendus: dernier.map_or(0, |(_, n)| *n),
                    note: note_i32(rendu.and_then(|r| r.note)),
                    commentaire: rendu.map(|r| r.commentaire.clone()).unwrap_or_default(),
                    a_corriger: rendu.is_some_and(|r| r.note.is_none()),
                    eleve_id: e.eleve_id,
                }
            })
            .collect();
        Ok(CopiesDevoir {
            devoir_id: devoir.id,
            bareme: i32::try_from(devoir.bareme).unwrap_or(super::BAREME_DEFAUT),
            peut_corriger: portee.peut_donner
                && EtatDevoir::depuis_nom(&devoir.etat) != EtatDevoir::Brouillon,
            copies_rendues_le: devoir.copies_rendues_le,
            lignes,
        })
    }

    /// L'élève, s'il est de la classe du devoir : (identifiant, nom).
    async fn eleve_du_devoir(
        &self,
        session: &Session,
        devoir: &LigneDevoir,
        eleve_id: &str,
    ) -> Resultat<String> {
        let contexte = self.contexte(session);
        let mut cnx = self.connexion().await?;
        depot_portail::eleves_de_la_classe(&mut cnx, &devoir.classe_id, &contexte)
            .await?
            .into_iter()
            .find(|e| e.eleve_id == eleve_id)
            .map(|e| nom_eleve(&e.nom, &e.prenoms))
            .ok_or_else(|| regle("devoir.eleve_hors_classe"))
    }

    /// La copie d'un élève : ses rendus et leurs photos.
    pub async fn copie(&self, session: &Session, demande: &DemandeCopie) -> Resultat<CopieEleve> {
        self.exiger_module(session).await?;
        let (devoir, _) = self.devoir_permis(session, &demande.devoir_id).await?;
        let nom = self
            .eleve_du_devoir(session, &devoir, &demande.eleve_id)
            .await?;
        let contexte = self.contexte(session);
        let mut cnx = self.connexion().await?;
        let rendus =
            depot::rendus_de_l_eleve(&mut cnx, &demande.eleve_id, Some(&devoir.id), &contexte)
                .await?;
        let photos =
            depot::photos_du_devoir(&mut cnx, &devoir.id, Some(&demande.eleve_id), &contexte)
                .await?;
        let rendus = rendus
            .into_iter()
            .map(|r| RenduAffiche {
                photos: photos
                    .iter()
                    .filter(|(rendu, ..)| rendu == &r.id)
                    .map(|(rendu, ordre, _, libere)| PhotoCopie {
                        rendu_id: rendu.clone(),
                        ordre: i32::try_from(*ordre).unwrap_or(0),
                        retiree: *libere,
                    })
                    .collect(),
                numero: u32::try_from(r.numero).unwrap_or(0),
                sur_papier: r.origine == "papier",
                note: note_i32(r.note),
                id: r.id,
                texte: r.texte,
                rendu_le: r.rendu_le,
                en_retard: r.en_retard,
                commentaire: r.commentaire,
            })
            .collect();
        Ok(CopieEleve {
            eleve_id: demande.eleve_id.clone(),
            nom,
            rendus,
        })
    }

    /// Une photo d'une copie, pour le maître ou la direction qui voit ce devoir.
    pub async fn photo_copie(
        &self,
        session: &Session,
        demande: &DemandePhotoCopie,
    ) -> Resultat<PhotoCopieServie> {
        self.exiger_module(session).await?;
        let (devoir, _) = self.devoir_permis(session, &demande.devoir_id).await?;
        let contexte = self.contexte(session);
        let mut cnx = self.connexion().await?;
        let introuvable = || {
            ErreurService::from(ErreurBase::Introuvable {
                entite: "photo_rendu",
                id: format!("{}/{}", demande.rendu_id, demande.ordre),
            })
        };
        let du_devoir = depot::rendus_du_devoir(&mut cnx, &devoir.id, &contexte)
            .await?
            .iter()
            .any(|r| r.id == demande.rendu_id);
        if !du_devoir {
            return Err(introuvable());
        }
        let (sorte, contenu) = depot::photo_du_rendu(
            &mut cnx,
            &demande.rendu_id,
            i64::from(demande.ordre),
            None,
            &contexte,
        )
        .await?
        .ok_or_else(introuvable)?;
        Ok(PhotoCopieServie {
            type_mime: if sorte == "png" {
                "image/png"
            } else {
                "image/jpeg"
            }
            .to_owned(),
            donnees: contenu.map(|c| base64::engine::general_purpose::STANDARD.encode(c)),
        })
    }

    /// Note et commentaire du dernier rendu d'un élève. Une note changée après « Rendre les
    /// copies corrigées » est visible aussitôt de l'élève ; chaque note va au journal.
    pub async fn noter(
        &self,
        session: &Session,
        demande: &DemandeNoteCopie,
    ) -> Resultat<CopiesDevoir> {
        let devoir = self.devoir_a_corriger(session, &demande.devoir_id).await?;
        let bareme = i32::try_from(devoir.bareme).unwrap_or(super::BAREME_DEFAUT);
        let note = lire_note(&demande.note, bareme)?;
        let commentaire = demande.commentaire.trim();
        if commentaire.chars().count() > COMMENTAIRE_MAX {
            return Err(regle("devoir.commentaire_long"));
        }
        self.eleve_du_devoir(session, &devoir, &demande.eleve_id)
            .await?;
        let contexte = self.contexte(session);
        let mut cnx = self.connexion().await?;
        let rendus =
            depot::rendus_de_l_eleve(&mut cnx, &demande.eleve_id, Some(&devoir.id), &contexte)
                .await?;
        drop(cnx);
        let Some(dernier) = rendus.into_iter().max_by_key(|r| r.numero) else {
            return Err(regle("devoir.pas_de_rendu"));
        };
        if dernier.note == note && dernier.commentaire == commentaire {
            return self.copies(session, &devoir.id).await;
        }
        let mut unite = self
            .noyau
            .commencer(contexte.clone(), Durabilite::Critique)
            .await?;
        depot::noter_rendu(
            unite.connexion(),
            &dernier.id,
            (note, commentaire),
            &contexte,
        )
        .await?;
        unite.auditer(
            "rendu_devoir",
            &dernier.id,
            Operation::Modification,
            Some(serde_json::json!({
                "note": dernier.note,
                "commentaire": dernier.commentaire,
            })),
            Some(serde_json::json!({
                "note": note,
                "commentaire": commentaire,
                "copies_rendues": devoir.copies_rendues_le.is_some(),
            })),
            "devoirs.noter",
        );
        self.noyau.terminer(unite).await?;
        self.copies(session, &devoir.id).await
    }

    /// « Rendu sur papier » : un rendu de l'élève, sans texte ni photo, que le maître note
    /// ensuite. Rien de nouveau si son dernier rendu est déjà sur papier.
    pub async fn rendu_papier(
        &self,
        session: &Session,
        demande: &DemandeCopie,
    ) -> Resultat<CopiesDevoir> {
        let devoir = self.devoir_a_corriger(session, &demande.devoir_id).await?;
        self.eleve_du_devoir(session, &devoir, &demande.eleve_id)
            .await?;
        let contexte = self.contexte(session);
        let mut cnx = self.connexion().await?;
        let deja =
            depot::rendus_de_l_eleve(&mut cnx, &demande.eleve_id, Some(&devoir.id), &contexte)
                .await?
                .into_iter()
                .max_by_key(|r| r.numero)
                .is_some_and(|r| r.origine == "papier");
        drop(cnx);
        if deja {
            return self.copies(session, &devoir.id).await;
        }
        let mut unite = self
            .noyau
            .commencer(contexte.clone(), Durabilite::Critique)
            .await?;
        let numero =
            depot::prochain_numero(unite.connexion(), &devoir.id, &demande.eleve_id).await?;
        let rendu = LigneRendu {
            id: nouvel_id::<()>().vers_chaine(),
            devoir_id: devoir.id.clone(),
            eleve_id: demande.eleve_id.clone(),
            numero,
            origine: "papier".to_owned(),
            texte: String::new(),
            rendu_le: contexte.maintenant.millis_epoque(),
            // La copie a été remise en main propre : le maître juge du retard.
            en_retard: false,
            note: None,
            commentaire: String::new(),
            note_le: None,
            note_par: None,
        };
        depot::inserer_rendu(unite.connexion(), &rendu, &[], &contexte).await?;
        unite.auditer(
            "rendu_devoir",
            &rendu.id,
            Operation::Creation,
            None,
            Some(serde_json::json!({
                "devoir_id": rendu.devoir_id,
                "eleve_id": rendu.eleve_id,
                "numero": rendu.numero,
                "origine": rendu.origine,
            })),
            "devoirs.papier",
        );
        self.noyau.terminer(unite).await?;
        self.copies(session, &devoir.id).await
    }

    /// « Rendre les copies corrigées » : notes et commentaires deviennent visibles des élèves.
    pub async fn rendre_copies(&self, session: &Session, id: &str) -> Resultat<DevoirAffiche> {
        let devoir = self.devoir_a_corriger(session, id).await?;
        if devoir.copies_rendues_le.is_none() {
            let contexte = self.contexte(session);
            let mut unite = self
                .noyau
                .commencer(contexte.clone(), Durabilite::Critique)
                .await?;
            depot::marquer_copies_rendues(unite.connexion(), &devoir.id, &contexte).await?;
            unite.auditer(
                "devoir",
                &devoir.id,
                Operation::Transition,
                Some(serde_json::json!({ "copies_rendues_le": null })),
                Some(serde_json::json!({
                    "copies_rendues_le": contexte.maintenant.millis_epoque(),
                })),
                "devoirs.rendre_copies",
            );
            self.noyau.terminer(unite).await?;
        }
        self.lire(session, id).await
    }

    /// Liste des notes de la classe pour ce devoir, à imprimer.
    pub(crate) async fn liste_des_notes(
        &self,
        session: &Session,
        id: &str,
    ) -> Resultat<Impression> {
        let affiche = self.lire(session, id).await?;
        let copies = self.copies(session, id).await?;
        let lecon = tableau_des_notes(&copies);
        let sous_titre = format!(
            "Liste des notes · {} · à rendre le {}",
            super::feuille::sur(i64::from(copies.bareme)),
            super::date_limite_longue(affiche.date_limite)
        );
        Ok(Impression::feuille_de_devoir(
            &affiche.titre,
            &sous_titre,
            (&affiche.classe, &affiche.matiere),
            &lecon,
            Vec::new(),
        ))
    }
}

fn cellule(texte: impl Into<String>) -> Vec<Enligne> {
    vec![Enligne::Texte {
        texte: texte.into(),
    }]
}

/// Mot de l'état, pour la liste imprimée.
fn mot_etat(etat: EtatCopie) -> &'static str {
    match etat {
        EtatCopie::Rendu => "Rendu",
        EtatCopie::EnRetard => "En retard",
        EtatCopie::SurPapier => "Sur papier",
        EtatCopie::PasRendu => "Pas rendu",
    }
}

/// Le tableau imprimé : numéro, élève, état, note, commentaire.
fn tableau_des_notes(copies: &CopiesDevoir) -> Lecon {
    let lignes = copies
        .lignes
        .iter()
        .enumerate()
        .map(|(i, l)| {
            vec![
                cellule((i + 1).to_string()),
                cellule(l.nom.clone()),
                cellule(mot_etat(l.etat)),
                cellule(l.note.map(|n| points(i64::from(n))).unwrap_or_default()),
                cellule(l.commentaire.clone()),
            ]
        })
        .collect();
    let notes: Vec<i64> = copies
        .lignes
        .iter()
        .filter_map(|l| l.note.map(i64::from))
        .collect();
    let mut blocs = vec![Bloc::Tableau {
        alignements: vec![
            Alignement::Droite,
            Alignement::Gauche,
            Alignement::Gauche,
            Alignement::Droite,
            Alignement::Gauche,
        ],
        entete: vec![
            cellule("N°"),
            cellule("Élève"),
            cellule("État"),
            cellule("Note"),
            cellule("Commentaire"),
        ],
        lignes,
    }];
    let resume = if notes.is_empty() {
        format!("{} élèves, aucune note.", copies.lignes.len())
    } else {
        let somme: i64 = notes.iter().sum();
        let nombre = i64::try_from(notes.len()).unwrap_or(1);
        format!(
            "{} élèves, {} notés ; moyenne {}.",
            copies.lignes.len(),
            notes.len(),
            points((somme + nombre / 2) / nombre)
        )
    };
    blocs.push(Bloc::Paragraphe {
        contenu: cellule(resume),
    });
    Lecon { blocs }
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::panic, clippy::indexing_slicing)]
mod tests {
    use super::*;

    fn rendu(numero: i64, origine: &str, en_retard: bool, note: Option<i64>) -> LigneRendu {
        LigneRendu {
            id: format!("r{numero}"),
            devoir_id: "d".into(),
            eleve_id: "e".into(),
            numero,
            origine: origine.into(),
            texte: String::new(),
            rendu_le: 0,
            en_retard,
            note,
            commentaire: String::new(),
            note_le: None,
            note_par: None,
        }
    }

    #[test]
    fn l_etat_d_une_copie_suit_le_dernier_rendu() {
        assert_eq!(etat_copie(None), EtatCopie::PasRendu);
        assert_eq!(
            etat_copie(Some(&rendu(1, "portail", false, None))),
            EtatCopie::Rendu
        );
        assert_eq!(
            etat_copie(Some(&rendu(1, "portail", true, None))),
            EtatCopie::EnRetard
        );
        assert_eq!(
            etat_copie(Some(&rendu(2, "papier", false, Some(1200)))),
            EtatCopie::SurPapier
        );
        let deux = [
            rendu(1, "portail", false, Some(1500)),
            rendu(2, "portail", true, None),
        ];
        let d = derniers(&deux);
        let (dernier, nombre) = d.get("e").copied().expect("élève");
        assert_eq!((dernier.numero, nombre), (2, 2));
        assert!(
            dernier.note.is_none(),
            "le rendu de nouveau arrive sans note"
        );
    }

    #[test]
    fn la_note_se_lit_comme_dans_la_grille() {
        assert_eq!(lire_note("13,5", 2000).ok(), Some(Some(1350)));
        assert_eq!(lire_note("", 2000).ok(), Some(None));
        assert!(matches!(
            lire_note("21", 2000),
            Err(ErreurService::Regle {
                cle: "note.hors_bareme"
            })
        ));
        assert!(matches!(
            lire_note("abs", 2000),
            Err(ErreurService::Regle {
                cle: "devoir.note_lettre"
            })
        ));
        assert_eq!(points(1350), "13,5");
        assert_eq!(points(1325), "13,25");
        assert_eq!(points(2000), "20");
    }

    #[test]
    fn la_liste_des_notes_donne_la_moyenne_des_notes_seules() {
        let ligne = |nom: &str, etat, note| LigneCopie {
            eleve_id: nom.into(),
            nom: nom.into(),
            etat,
            rendu_le: None,
            rendus: 0,
            note,
            commentaire: String::new(),
            a_corriger: false,
        };
        let copies = CopiesDevoir {
            devoir_id: "d".into(),
            bareme: 2000,
            peut_corriger: true,
            copies_rendues_le: None,
            lignes: vec![
                ligne("A", EtatCopie::Rendu, Some(1200)),
                ligne("B", EtatCopie::SurPapier, Some(1500)),
                ligne("C", EtatCopie::PasRendu, None),
            ],
        };
        let lecon = tableau_des_notes(&copies);
        let Some(Bloc::Tableau { lignes, .. }) = lecon.blocs.first() else {
            panic!("tableau attendu");
        };
        assert_eq!(lignes.len(), 3);
        assert_eq!(lignes[2][2], cellule("Pas rendu"));
        assert_eq!(
            lecon.blocs.get(1),
            Some(&Bloc::Paragraphe {
                contenu: cellule("3 élèves, 2 notés ; moyenne 13,5.")
            })
        );
    }
}
