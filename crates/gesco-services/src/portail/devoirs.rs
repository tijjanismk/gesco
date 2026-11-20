//! Devoirs au portail des élèves (SPEC_PHASE_12_LOT_5 §4 ; ADR « copies photographiées ») :
//! « Mes devoirs », le devoir ouvert, le rendu (un texte, ou jusqu'à cinq photos réduites par
//! le téléphone) et la photo d'un de ses rendus.
//!
//! Seul un élève entré avec son code voit ses devoirs : ceux donnés à sa classe de l'année en
//! cours, et rien d'un autre élève. Un rendu s'écrit tout de suite (pas de file) : l'élève sait
//! qu'il est arrivé. Après la date limite il est accepté et marqué « en retard » pour le maître ;
//! un devoir clos ne reçoit plus rien. Rendre de nouveau ajoute un rendu : le dernier compte, les
//! précédents restent. La note et le commentaire ne se voient qu'une fois les copies rendues.

use std::collections::HashMap;

use base64::Engine as _;
use gesco_core::bibliotheque::copies::{
    PHOTOS_MAX, PhotoRefusee, SortePhoto, TEXTE_RENDU_MAX, nettoyer_photo,
};
use gesco_core::bibliotheque::{Lecon, analyser};
use gesco_core::commun::Id;
use gesco_core::core::{Etablissement, ModuleEcole};
use gesco_db::depots::devoirs::{self as depot, LigneDevoir, LigneRendu, PhotoAEcrire};
use gesco_db::depots::pedagogie;
use gesco_db::unite::Durabilite;
use gesco_db::{Connexion, Contexte, ErreurBase, nouvel_id};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use super::entree::SessionEleve;
use super::lecture::{LecturePortail, ReponseLecture};
use crate::bibliotheque::ImageLecon;
use crate::devoirs::{EtatDevoir, FormeRendu, ServiceDevoirs};
use crate::erreur::Resultat;
use crate::noyau::Noyau;

/// Où en est un devoir pour l'élève : l'état se dit en mots sur le portail.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum EtatPourEleve {
    /// À rendre, avant la date limite.
    ARendre,
    /// Pas encore rendu, la date limite passée (on peut encore rendre).
    EnRetard,
    /// Rendu (le dernier rendu dit quand).
    Rendu,
    /// Les copies corrigées sont rendues : la note et le commentaire se voient.
    Corrige,
    /// Clos sans rendu de l'élève.
    Clos,
    /// À rendre sur papier au maître.
    Papier,
}

/// La note rendue à l'élève, avec le commentaire du maître.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct CorrectionEleve {
    /// Note en centièmes.
    pub note: i64,
    /// Barème en centièmes.
    pub bareme: i64,
    /// Commentaire du maître.
    pub commentaire: String,
}

/// Un devoir dans « Mes devoirs ».
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct DevoirDeLEleve {
    /// Identifiant.
    pub id: String,
    /// Titre.
    pub titre: String,
    /// Matière.
    pub matiere: String,
    /// Date et heure limites (millisecondes).
    pub date_limite: i64,
    /// Façon de rendre.
    pub forme_rendu: FormeRendu,
    /// Où il en est.
    pub etat: EtatPourEleve,
    /// Dernier rendu le.
    pub rendu_le: Option<i64>,
    /// Le dernier rendu est arrivé après la date limite.
    pub apres_limite: bool,
}

/// « Mes devoirs ».
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct MesDevoirs {
    /// L'élève est entré avec son code : sans code, il lit seulement qu'il faut entrer.
    pub avec_code: bool,
    /// Devoirs de sa classe, par date limite.
    pub devoirs: Vec<DevoirDeLEleve>,
}

/// Une photo d'un rendu.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct PhotoEleve {
    /// Ordre (1 à 5), pour `/eleve/api/rendu/{id}/photo/{ordre}`.
    pub ordre: i64,
    /// Place libérée après la clôture de l'année : la photo n'est plus là.
    pub retiree: bool,
}

/// Un rendu de l'élève.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct RenduEleve {
    /// Identifiant.
    pub id: String,
    /// Numéro.
    pub numero: i64,
    /// Rendu le.
    pub rendu_le: i64,
    /// Rendu après la date limite.
    pub apres_limite: bool,
    /// Rendu sur papier (noté par le maître).
    pub sur_papier: bool,
    /// Texte rendu.
    pub texte: String,
    /// Photos.
    pub photos: Vec<PhotoEleve>,
}

/// La leçon liée, quand l'élève la voit.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct LeconLieeEleve {
    /// Identifiant.
    pub id: String,
    /// Titre.
    pub titre: String,
}

/// Un devoir ouvert par l'élève.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct DevoirOuvertEleve {
    /// Identifiant.
    pub id: String,
    /// Titre.
    pub titre: String,
    /// Matière.
    pub matiere: String,
    /// Date et heure limites (millisecondes).
    pub date_limite: i64,
    /// Barème en centièmes.
    pub bareme: i64,
    /// Façon de rendre.
    pub forme_rendu: FormeRendu,
    /// Clos : plus de rendu.
    pub clos: bool,
    /// Où il en est.
    pub etat: EtatPourEleve,
    /// Consignes en arbre de blocs.
    pub lecon: Lecon,
    /// Images des consignes.
    pub images: Vec<ImageLecon>,
    /// Leçon liée.
    pub lecon_liee: Option<LeconLieeEleve>,
    /// Ses rendus, du premier au dernier.
    pub rendus: Vec<RenduEleve>,
    /// La note et le commentaire, une fois les copies rendues.
    pub correction: Option<CorrectionEleve>,
}

/// Ce que le téléphone envoie à `POST /eleve/api/rendu`.
#[derive(Debug, Clone, Deserialize)]
pub struct DemandeRendu {
    /// Devoir.
    pub devoir_id: String,
    /// Texte tapé.
    #[serde(default)]
    pub texte: String,
    /// Photos réduites, en base 64.
    #[serde(default)]
    pub photos: Vec<String>,
}

/// Pourquoi un rendu n'est pas reçu ; la page le dit en une phrase simple.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RefusRendu {
    /// Entré sans code.
    SansCode,
    /// Le devoir est clos.
    Clos,
    /// Le devoir se rend sur papier.
    Papier,
    /// Ni texte ni photo.
    Vide,
    /// Ce devoir ne se rend pas en texte.
    TexteNonPermis,
    /// Ce devoir ne se rend pas en photos.
    PhotosNonPermises,
    /// Plus de cinq photos.
    TropDePhotos,
    /// Un fichier qui n'est ni une photo JPEG ni PNG, ou abîmé.
    PasUnePhoto,
    /// Une photo de plus de 2 Mo.
    PhotoTropGrosse,
    /// Texte trop long.
    TexteLong,
}

/// Le rendu est arrivé.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct RenduFait {
    /// Numéro du rendu.
    pub numero: i64,
    /// Rendu le.
    pub rendu_le: i64,
    /// Rendu après la date limite.
    pub apres_limite: bool,
}

/// Une photo d'un rendu de l'élève : le contenu, ou rien quand la place a été libérée.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PhotoServie {
    /// Type MIME.
    pub type_mime: &'static str,
    /// Contenu ; `None` : retirée pour libérer de la place.
    pub octets: Option<Vec<u8>>,
}

/// Où en est un devoir pour l'élève, d'après son dernier rendu.
pub fn etat_pour_eleve(
    devoir: &LigneDevoir,
    dernier: Option<&LigneRendu>,
    maintenant: i64,
) -> EtatPourEleve {
    if devoir.copies_rendues_le.is_some() && dernier.is_some_and(|r| r.note.is_some()) {
        return EtatPourEleve::Corrige;
    }
    if dernier.is_some() {
        return EtatPourEleve::Rendu;
    }
    if FormeRendu::depuis_nom(&devoir.forme_rendu) == FormeRendu::Papier {
        return EtatPourEleve::Papier;
    }
    if EtatDevoir::depuis_nom(&devoir.etat) == EtatDevoir::Clos {
        return EtatPourEleve::Clos;
    }
    if maintenant > devoir.date_limite {
        EtatPourEleve::EnRetard
    } else {
        EtatPourEleve::ARendre
    }
}

/// La correction que l'élève voit : seulement les copies rendues, et sur son dernier rendu.
fn correction(devoir: &LigneDevoir, dernier: Option<&LigneRendu>) -> Option<CorrectionEleve> {
    devoir.copies_rendues_le?;
    let r = dernier?;
    Some(CorrectionEleve {
        note: r.note?,
        bareme: devoir.bareme,
        commentaire: r.commentaire.clone(),
    })
}

/// Une photo vérifiée, sans métadonnée.
pub type PhotoNettoyee = (SortePhoto, Vec<u8>);

/// Vérifie le rendu et en tire les photos nettoyées.
pub fn verifier_rendu(
    devoir: &LigneDevoir,
    demande: &DemandeRendu,
) -> Result<(String, Vec<PhotoNettoyee>), RefusRendu> {
    let forme = FormeRendu::depuis_nom(&devoir.forme_rendu);
    if EtatDevoir::depuis_nom(&devoir.etat) == EtatDevoir::Clos {
        return Err(RefusRendu::Clos);
    }
    if forme == FormeRendu::Papier {
        return Err(RefusRendu::Papier);
    }
    let texte = demande.texte.trim();
    if texte.is_empty() && demande.photos.is_empty() {
        return Err(RefusRendu::Vide);
    }
    if !texte.is_empty() && !forme.accepte_texte() {
        return Err(RefusRendu::TexteNonPermis);
    }
    if texte.chars().count() > TEXTE_RENDU_MAX {
        return Err(RefusRendu::TexteLong);
    }
    if !demande.photos.is_empty() && !forme.accepte_photos() {
        return Err(RefusRendu::PhotosNonPermises);
    }
    if demande.photos.len() > PHOTOS_MAX {
        return Err(RefusRendu::TropDePhotos);
    }
    let mut photos = Vec::with_capacity(demande.photos.len());
    for p in &demande.photos {
        // Une adresse `data:` entière est acceptée aussi.
        let brut = p.split_once(";base64,").map_or(p.as_str(), |(_, b)| b);
        let octets = base64::engine::general_purpose::STANDARD
            .decode(brut.trim())
            .map_err(|_| RefusRendu::PasUnePhoto)?;
        photos.push(nettoyer_photo(&octets).map_err(|e| match e {
            PhotoRefusee::TropGrosse => RefusRendu::PhotoTropGrosse,
            PhotoRefusee::PasUnePhoto | PhotoRefusee::Illisible => RefusRendu::PasUnePhoto,
        })?);
    }
    Ok((texte.to_owned(), photos))
}

/// Empreinte SHA-256 en hexadécimal.
fn empreinte(octets: &[u8]) -> String {
    hex::encode(Sha256::digest(octets))
}

/// Les devoirs se lisent et se rendent : base ouverte et bibliothèque allumée (son interrupteur
/// coupe aussi les devoirs ; le serveur du portail se ferme déjà avec lui).
async fn module_actif(cnx: &mut Connexion, contexte: &Contexte) -> Resultat<bool> {
    crate::modules::actif(cnx, ModuleEcole::Bibliotheque, contexte).await
}

async fn connexion(noyau: &Noyau) -> Resultat<gesco_db::ConnexionEmpruntee> {
    Ok(noyau
        .base()
        .pool()
        .acquire()
        .await
        .map_err(ErreurBase::from)?)
}

/// Libellés des matières.
async fn matieres(cnx: &mut Connexion, contexte: &Contexte) -> Resultat<HashMap<String, String>> {
    Ok(pedagogie::matieres(cnx, contexte)
        .await?
        .into_iter()
        .map(|m| (m.id.vers_chaine(), m.libelle))
        .collect())
}

/// L'élève entré avec son code, sa classe et son année ; `None` sans code.
fn eleve(session: &SessionEleve) -> Option<(&str, &str, &str)> {
    Some((
        session.eleve_id.as_deref()?,
        session.classe_id.as_deref()?,
        session.annee_id.as_deref()?,
    ))
}

/// Le devoir, s'il est donné (ou clos) à la classe de l'élève cette année.
async fn devoir_de_l_eleve(
    cnx: &mut Connexion,
    devoir_id: &str,
    (classe_id, annee_id): (&str, &str),
    contexte: &Contexte,
) -> Resultat<Option<LigneDevoir>> {
    Ok(depot::lire(cnx, devoir_id, contexte).await?.filter(|d| {
        d.classe_id == classe_id
            && d.annee_id == annee_id
            && EtatDevoir::depuis_nom(&d.etat) != EtatDevoir::Brouillon
    }))
}

/// « Mes devoirs » : les devoirs donnés à la classe de l'élève, par date limite.
pub async fn mes_devoirs(
    noyau: &Noyau,
    etablissement_id: Id<Etablissement>,
    session: &SessionEleve,
) -> Resultat<ReponseLecture<MesDevoirs>> {
    let contexte = noyau.contexte(etablissement_id, None);
    let mut cnx = connexion(noyau).await?;
    if !module_actif(&mut cnx, &contexte).await? {
        return Ok(ReponseLecture::Ferme);
    }
    let Some((eleve_id, classe_id, annee_id)) = eleve(session) else {
        return Ok(ReponseLecture::Lu(MesDevoirs {
            avec_code: false,
            devoirs: Vec::new(),
        }));
    };
    let maintenant = contexte.maintenant.millis_epoque();
    let libelles = matieres(&mut cnx, &contexte).await?;
    let mut derniers: HashMap<String, LigneRendu> = HashMap::new();
    for r in depot::rendus_de_l_eleve(&mut cnx, eleve_id, None, &contexte).await? {
        // Par numéro croissant : le dernier reste.
        derniers.insert(r.devoir_id.clone(), r);
    }
    let devoirs = depot::donnes_a_la_classe(&mut cnx, classe_id, annee_id, &contexte)
        .await?
        .into_iter()
        .map(|d| {
            let dernier = derniers.get(&d.id);
            DevoirDeLEleve {
                etat: etat_pour_eleve(&d, dernier, maintenant),
                rendu_le: dernier.map(|r| r.rendu_le),
                apres_limite: dernier.is_some_and(|r| r.en_retard),
                matiere: libelles.get(&d.matiere_id).cloned().unwrap_or_default(),
                forme_rendu: FormeRendu::depuis_nom(&d.forme_rendu),
                id: d.id,
                titre: d.titre,
                date_limite: d.date_limite,
            }
        })
        .collect();
    Ok(ReponseLecture::Lu(MesDevoirs {
        avec_code: true,
        devoirs,
    }))
}

/// Un devoir ouvert par l'élève, avec ses rendus et, les copies rendues, sa note.
pub async fn devoir(
    noyau: &Noyau,
    etablissement_id: Id<Etablissement>,
    (session, lecture): (&SessionEleve, &LecturePortail),
    devoir_id: &str,
) -> Resultat<ReponseLecture<DevoirOuvertEleve>> {
    let contexte = noyau.contexte(etablissement_id, None);
    let mut cnx = connexion(noyau).await?;
    if !module_actif(&mut cnx, &contexte).await? {
        return Ok(ReponseLecture::Ferme);
    }
    let Some((eleve_id, classe_id, annee_id)) = eleve(session) else {
        return Ok(ReponseLecture::Introuvable);
    };
    let Some(d) = devoir_de_l_eleve(&mut cnx, devoir_id, (classe_id, annee_id), &contexte).await?
    else {
        return Ok(ReponseLecture::Introuvable);
    };
    let rendus = depot::rendus_de_l_eleve(&mut cnx, eleve_id, Some(&d.id), &contexte).await?;
    let photos = depot::photos_du_devoir(&mut cnx, &d.id, Some(eleve_id), &contexte).await?;
    let lecon = analyser(&d.consignes).pour_les_eleves();
    let images = ServiceDevoirs::images(&mut cnx, &lecon, &contexte).await?;
    let matiere = matieres(&mut cnx, &contexte)
        .await?
        .remove(&d.matiere_id)
        .unwrap_or_default();
    let maintenant = contexte.maintenant.millis_epoque();
    let dernier = rendus.last();
    let lecon_liee = d
        .contenu_id
        .as_deref()
        .and_then(|c| lecture.contenu(c, session))
        .map(|c| LeconLieeEleve {
            id: c.id,
            titre: c.titre,
        });
    Ok(ReponseLecture::Lu(DevoirOuvertEleve {
        etat: etat_pour_eleve(&d, dernier, maintenant),
        correction: correction(&d, dernier),
        clos: EtatDevoir::depuis_nom(&d.etat) == EtatDevoir::Clos,
        forme_rendu: FormeRendu::depuis_nom(&d.forme_rendu),
        rendus: rendus
            .iter()
            .map(|r| RenduEleve {
                id: r.id.clone(),
                numero: r.numero,
                rendu_le: r.rendu_le,
                apres_limite: r.en_retard,
                sur_papier: r.origine == "papier",
                texte: r.texte.clone(),
                photos: photos
                    .iter()
                    .filter(|(rendu, ..)| *rendu == r.id)
                    .map(|(_, ordre, _, libere)| PhotoEleve {
                        ordre: *ordre,
                        retiree: *libere,
                    })
                    .collect(),
            })
            .collect(),
        lecon,
        images,
        lecon_liee,
        matiere,
        id: d.id,
        titre: d.titre,
        date_limite: d.date_limite,
        bareme: d.bareme,
    }))
}

/// Rend un devoir : un rendu de plus, écrit tout de suite ; le dernier compte.
pub async fn rendre(
    noyau: &Noyau,
    etablissement_id: Id<Etablissement>,
    session: &SessionEleve,
    demande: &DemandeRendu,
) -> Resultat<ReponseLecture<Result<RenduFait, RefusRendu>>> {
    let contexte = noyau.contexte(etablissement_id, None);
    let mut cnx = connexion(noyau).await?;
    if !module_actif(&mut cnx, &contexte).await? {
        return Ok(ReponseLecture::Ferme);
    }
    let Some((eleve_id, classe_id, annee_id)) = eleve(session) else {
        return Ok(ReponseLecture::Lu(Err(RefusRendu::SansCode)));
    };
    let Some(d) = devoir_de_l_eleve(
        &mut cnx,
        &demande.devoir_id,
        (classe_id, annee_id),
        &contexte,
    )
    .await?
    else {
        return Ok(ReponseLecture::Introuvable);
    };
    drop(cnx);
    let (texte, photos) = match verifier_rendu(&d, demande) {
        Ok(v) => v,
        Err(refus) => return Ok(ReponseLecture::Lu(Err(refus))),
    };
    let maintenant = contexte.maintenant.millis_epoque();
    let photos: Vec<PhotoAEcrire> = photos
        .into_iter()
        .zip(1_i64..)
        .map(|((sorte, contenu), ordre)| PhotoAEcrire {
            ordre,
            empreinte: empreinte(&contenu),
            contenu,
            sorte: sorte.nom().to_owned(),
        })
        .collect();
    let mut unite = noyau
        .commencer(contexte.clone(), Durabilite::Critique)
        .await?;
    let numero = depot::prochain_numero(unite.connexion(), &d.id, eleve_id).await?;
    let rendu = LigneRendu {
        id: nouvel_id::<()>().vers_chaine(),
        devoir_id: d.id.clone(),
        eleve_id: eleve_id.to_owned(),
        numero,
        origine: "portail".to_owned(),
        texte,
        rendu_le: maintenant,
        en_retard: maintenant > d.date_limite,
        note: None,
        commentaire: String::new(),
        note_le: None,
        note_par: None,
    };
    depot::inserer_rendu(unite.connexion(), &rendu, &photos, &contexte).await?;
    noyau.terminer(unite).await?;
    Ok(ReponseLecture::Lu(Ok(RenduFait {
        numero,
        rendu_le: rendu.rendu_le,
        apres_limite: rendu.en_retard,
    })))
}

/// Une photo d'un des rendus de l'élève ; jamais celle d'un autre.
pub async fn photo(
    noyau: &Noyau,
    etablissement_id: Id<Etablissement>,
    session: &SessionEleve,
    (rendu_id, ordre): (&str, i64),
) -> Resultat<ReponseLecture<PhotoServie>> {
    let contexte = noyau.contexte(etablissement_id, None);
    let mut cnx = connexion(noyau).await?;
    if !module_actif(&mut cnx, &contexte).await? {
        return Ok(ReponseLecture::Ferme);
    }
    let Some((eleve_id, ..)) = eleve(session) else {
        return Ok(ReponseLecture::Introuvable);
    };
    Ok(
        match depot::photo_du_rendu(&mut cnx, rendu_id, ordre, Some(eleve_id), &contexte).await? {
            Some((sorte, octets)) => ReponseLecture::Lu(PhotoServie {
                type_mime: SortePhoto::depuis_nom(&sorte).type_mime(),
                octets,
            }),
            None => ReponseLecture::Introuvable,
        },
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn devoir(etat: &str, forme: &str) -> LigneDevoir {
        LigneDevoir {
            id: "d".to_owned(),
            annee_id: "a".to_owned(),
            classe_id: "c".to_owned(),
            matiere_id: "m".to_owned(),
            auteur_id: "u".to_owned(),
            titre: "Devoir".to_owned(),
            consignes: String::new(),
            contenu_id: None,
            date_limite: 1_000,
            bareme: 2_000,
            forme_rendu: forme.to_owned(),
            etat: etat.to_owned(),
            donne_le: Some(1),
            clos_le: None,
            copies_rendues_le: None,
            reporte_le: None,
            copie_de: None,
            cree_le: 1,
            mis_a_jour_le: 1,
        }
    }

    fn rendu(note: Option<i64>) -> LigneRendu {
        LigneRendu {
            id: "r".to_owned(),
            devoir_id: "d".to_owned(),
            eleve_id: "e".to_owned(),
            numero: 1,
            origine: "portail".to_owned(),
            texte: "x".to_owned(),
            rendu_le: 900,
            en_retard: false,
            note,
            commentaire: "Bien.".to_owned(),
            note_le: None,
            note_par: None,
        }
    }

    #[test]
    fn l_etat_se_dit_pour_l_eleve() {
        let d = devoir("donne", "texte_ou_photos");
        assert_eq!(etat_pour_eleve(&d, None, 500), EtatPourEleve::ARendre);
        assert_eq!(etat_pour_eleve(&d, None, 2_000), EtatPourEleve::EnRetard);
        assert_eq!(
            etat_pour_eleve(&d, Some(&rendu(None)), 2_000),
            EtatPourEleve::Rendu
        );
        // Noté, mais les copies ne sont pas rendues : l'élève ne voit pas encore la note.
        assert_eq!(
            etat_pour_eleve(&d, Some(&rendu(Some(1_500))), 2_000),
            EtatPourEleve::Rendu
        );
        assert_eq!(correction(&d, Some(&rendu(Some(1_500)))), None);
        let mut rendues = d.clone();
        rendues.copies_rendues_le = Some(3_000);
        assert_eq!(
            etat_pour_eleve(&rendues, Some(&rendu(Some(1_500))), 4_000),
            EtatPourEleve::Corrige
        );
        assert_eq!(
            correction(&rendues, Some(&rendu(Some(1_500)))).map(|c| (c.note, c.bareme)),
            Some((1_500, 2_000))
        );
        assert_eq!(
            etat_pour_eleve(&devoir("clos", "texte"), None, 500),
            EtatPourEleve::Clos
        );
        assert_eq!(
            etat_pour_eleve(&devoir("donne", "papier"), None, 500),
            EtatPourEleve::Papier
        );
    }

    #[test]
    fn un_rendu_est_verifie_avant_d_entrer() {
        let demande = |texte: &str, photos: Vec<String>| DemandeRendu {
            devoir_id: "d".to_owned(),
            texte: texte.to_owned(),
            photos,
        };
        let d = devoir("donne", "texte_ou_photos");
        assert_eq!(
            verifier_rendu(&d, &demande("  ", vec![])).err(),
            Some(RefusRendu::Vide)
        );
        assert_eq!(
            verifier_rendu(&d, &demande(" Ma réponse ", vec![])).map(|(t, p)| (t, p.len())),
            Ok(("Ma réponse".to_owned(), 0))
        );
        assert_eq!(
            verifier_rendu(&devoir("clos", "texte"), &demande("x", vec![])).err(),
            Some(RefusRendu::Clos)
        );
        assert_eq!(
            verifier_rendu(&devoir("donne", "papier"), &demande("x", vec![])).err(),
            Some(RefusRendu::Papier)
        );
        assert_eq!(
            verifier_rendu(&devoir("donne", "photos"), &demande("x", vec![])).err(),
            Some(RefusRendu::TexteNonPermis)
        );
        let pdf = base64::engine::general_purpose::STANDARD.encode(b"%PDF-1.7");
        assert_eq!(
            verifier_rendu(&d, &demande("", vec![pdf.clone()])).err(),
            Some(RefusRendu::PasUnePhoto)
        );
        assert_eq!(
            verifier_rendu(&devoir("donne", "texte"), &demande("", vec![pdf.clone()])).err(),
            Some(RefusRendu::PhotosNonPermises)
        );
        assert_eq!(
            verifier_rendu(&d, &demande("", vec![pdf; PHOTOS_MAX + 1])).err(),
            Some(RefusRendu::TropDePhotos)
        );
        assert_eq!(
            verifier_rendu(&d, &demande(&"a".repeat(TEXTE_RENDU_MAX + 1), vec![])).err(),
            Some(RefusRendu::TexteLong)
        );
    }
}
