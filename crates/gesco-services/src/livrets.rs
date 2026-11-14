//! Livrets du jardin d'enfants (SPEC_PHASE_11_B §3.4, §5) : un livret est un bulletin, d'un
//! contenu d'une autre forme. Préparé, il fige ce qu'il imprime : pour chaque domaine, les
//! compétences observées au moins une fois avec leur niveau à chaque période déjà passée où
//! l'enfant était inscrit, le commentaire du domaine, l'appréciation générale, les absences si
//! l'école les relève, la décision de fin d'année au dernier livret. Rien ne bloque et rien ne
//! signale ce qui manque (RB-1, RB-2) : un enfant sans observation a son livret, sans mention.

use gesco_core::commun::Id;
use gesco_core::pedagogie::assiduite::totaux;
use gesco_core::pedagogie::bulletin::{arrive_pendant, non_suivie};
use gesco_core::pedagogie::jardin::cle_commentaire_domaine;
use gesco_core::pedagogie::{
    Bulletin, CompetenceLivret, ContenuBulletin, ContenuLivret, DomaineLivret, FormeBulletin,
    NiveauAcquisition, Observation, Periode, StatutBulletin,
};
use gesco_core::scolarite::{Classe, DecisionFinAnnee, Inscription, Niveau, StatutInscription};
use gesco_db::depots::bulletins::APPRECIATION_GENERALE;
use gesco_db::depots::{
    absences, academique, annee_scolaire, bulletins, inscriptions, jardin, pedagogie, personnes,
};
use gesco_docs::{Piece, qr};

use crate::erreur::{ErreurService, Resultat};
use crate::inscriptions::ServiceInscriptions;
use crate::jardin::ServiceJardin;
use crate::noyau::Noyau;
use crate::session::Session;

fn regle(cle: &'static str) -> ErreurService {
    ErreurService::Regle { cle }
}

/// Livrets préparés d'une classe pour une période, sans rien écrire.
pub(crate) struct Livrets {
    /// Période.
    pub periode: Periode,
    /// Contenu de chaque enfant qui a suivi la période, dans l'ordre de la liste.
    pub contenus: Vec<(Id<Inscription>, ContenuBulletin)>,
    /// Inscriptions de la classe, tous états.
    pub inscriptions: Vec<Id<Inscription>>,
}

/// Décision de fin d'année telle qu'elle s'imprime au livret ; rien tant qu'elle n'est pas posée.
fn decision(d: DecisionFinAnnee) -> Option<String> {
    match d {
        DecisionFinAnnee::NonStatue => None,
        DecisionFinAnnee::Admis => Some("Passe dans la section suivante".to_owned()),
        DecisionFinAnnee::Redouble => Some("Reste dans sa section".to_owned()),
        DecisionFinAnnee::Exclu => Some("Exclu".to_owned()),
        DecisionFinAnnee::Rattrapage => Some("Rattrapage".to_owned()),
    }
}

/// Prépare le contenu des livrets d'une classe du jardin pour une période.
pub(crate) async fn preparer(
    noyau: &Noyau,
    session: &Session,
    classe: &Classe,
    niveau: &Niveau,
    periode_id: Id<Periode>,
) -> Resultat<Livrets> {
    let contexte = noyau.contexte(session.etablissement_id, Some(session));
    let liste = ServiceJardin::nouveau(noyau).lire(session).await?;
    let lignes_classe = ServiceInscriptions::nouveau(noyau)
        .de_la_classe(session, classe)
        .await?;
    let mut cnx = noyau
        .base()
        .pool()
        .acquire()
        .await
        .map_err(gesco_db::ErreurBase::from)?;
    let annee = annee_scolaire::lire(&mut cnx, classe.annee_id, &contexte).await?;
    let niveaux = academique::niveaux(&mut cnx, &contexte).await?;
    let mut periodes: Vec<Periode> = pedagogie::periodes(&mut cnx, classe.annee_id, &contexte)
        .await?
        .into_iter()
        .filter(|p| p.niveau_id == classe.niveau_id)
        .collect();
    periodes.sort_by_key(|p| p.rang);
    let periode = periodes
        .iter()
        .find(|p| p.id == periode_id)
        .cloned()
        .ok_or(regle("evaluation.periode_invalide"))?;
    let derniere = periodes.last().is_some_and(|p| p.id == periode.id);
    // Les périodes déjà passées, celle du livret comprise (T-111).
    let passees: Vec<Periode> = periodes
        .iter()
        .filter(|p| p.rang <= periode.rang)
        .cloned()
        .collect();
    let mut observations: Vec<Observation> = Vec::new();
    for p in &passees {
        observations.extend(jardin::observations_de_la_periode(&mut cnx, p.id, &contexte).await?);
    }
    let appreciations = bulletins::appreciations(&mut cnx, periode.id, &contexte).await?;
    let absences_periode =
        absences::de_la_classe(&mut cnx, classe.id, periode.debut, periode.fin, &contexte).await?;
    let absences_relevees =
        absences::jours_releves(&mut cnx, classe.id, periode.debut, periode.fin, &contexte).await?
            > 0
            || !absences_periode.is_empty();
    let arrivees = inscriptions::arrivees_dans_la_classe(&mut cnx, classe.id, &contexte).await?;
    let identites = personnes::eleves_de_la_classe(&mut cnx, classe.id, &contexte).await?;

    let inscriptions_classe: Vec<Id<Inscription>> = lignes_classe
        .iter()
        .filter(|l| l.inscription.classe_id == classe.id)
        .map(|l| l.inscription.id)
        .collect();
    let eleves: Vec<_> = lignes_classe
        .into_iter()
        .filter(|l| {
            l.inscription.classe_id == classe.id
                && l.inscription.statut == StatutInscription::Validee
        })
        .map(|l| {
            let depuis = arrivees
                .iter()
                .find(|(i, _)| *i == l.inscription.id)
                .map_or(l.inscription.date_inscription, |(_, d)| *d);
            (l, depuis)
        })
        .filter(|(_, depuis)| !non_suivie(*depuis, &periode))
        .collect();
    let effectif = u32::try_from(eleves.len()).unwrap_or(u32::MAX);
    let section_de = |n: Id<Niveau>| {
        niveaux
            .iter()
            .find(|x| x.id == n)
            .map(|x| x.libelle.clone())
            .unwrap_or_default()
    };

    let mut contenus = Vec::with_capacity(eleves.len());
    for (ligne, depuis) in eleves {
        let inscription = &ligne.inscription;
        // Une colonne par période passée où l'enfant était inscrit dans l'école (RB-8) ; ses
        // observations d'une autre section le suivent (RB-7).
        let colonnes: Vec<&Periode> = passees
            .iter()
            .filter(|p| !non_suivie(inscription.date_inscription, p))
            .collect();
        let niveau_de = |c: Id<gesco_core::pedagogie::Competence>, p: Id<Periode>| {
            observations
                .iter()
                .find(|o| {
                    o.inscription_id == inscription.id && o.competence_id == c && o.periode_id == p
                })
                .and_then(|o| o.niveau)
        };
        let mut domaines = Vec::new();
        for domaine in liste.domaines.iter().filter(|d| d.retire_le.is_none()) {
            let mut siennes: Vec<_> = liste
                .competences
                .iter()
                .filter(|c| c.domaine_id == domaine.id && c.retiree_le.is_none())
                .filter(|c| colonnes.iter().any(|p| niveau_de(c.id, p.id).is_some()))
                .collect();
            if siennes.is_empty() {
                continue;
            }
            // Celles de sa section d'abord, puis celles observées dans une autre.
            siennes.sort_by_key(|c| (c.niveau_id != classe.niveau_id, c.ordre));
            let commentaire = if liste.commentaire_domaine {
                appreciations
                    .iter()
                    .find(|a| {
                        a.inscription_id == inscription.id
                            && a.matiere_cle == cle_commentaire_domaine(domaine.id)
                    })
                    .map(|a| a.texte.clone())
                    .unwrap_or_default()
            } else {
                String::new()
            };
            domaines.push(DomaineLivret {
                libelle: domaine.libelle.clone(),
                competences: siennes
                    .iter()
                    .map(|c| CompetenceLivret {
                        libelle: c.libelle.clone(),
                        section: (c.niveau_id != classe.niveau_id).then(|| section_de(c.niveau_id)),
                        niveaux: colonnes.iter().map(|p| niveau_de(c.id, p.id)).collect(),
                    })
                    .collect(),
                commentaire,
            });
        }
        let siennes_absences: Vec<_> = absences_periode
            .iter()
            .filter(|a| a.inscription_id == inscription.id)
            .cloned()
            .collect();
        let date_naissance = identites
            .iter()
            .find(|(e, _)| e.id == inscription.eleve_id)
            .and_then(|(_, p)| p.date_naissance);
        contenus.push((
            inscription.id,
            ContenuBulletin {
                eleve: ligne.eleve.clone(),
                matricule: ligne.matricule.clone(),
                classe: classe.libelle.clone(),
                niveau: niveau.libelle.clone(),
                cycle: niveau.cycle,
                annee: annee.libelle.clone(),
                periode: periode.libelle(),
                bareme: 0,
                lignes: Vec::new(),
                total_points: 0,
                total_coefficients: 0,
                moyenne: None,
                rang: None,
                mention: None,
                effectif,
                ont_compose: 0,
                premier: None,
                dernier: None,
                moyenne_classe: None,
                absences: totaux(&siennes_absences, periode.debut, periode.fin),
                appreciation_generale: appreciations
                    .iter()
                    .find(|a| {
                        a.inscription_id == inscription.id && a.matiere_cle == APPRECIATION_GENERALE
                    })
                    .map(|a| a.texte.clone())
                    .unwrap_or_default(),
                moyenne_annuelle: None,
                rang_annuel: None,
                moyenne_annuelle_imprimee: false,
                arrive_le: arrive_pendant(depuis, &periode).then_some(depuis),
                sanctions_conseil: Vec::new(),
                conduite: None,
                forme: FormeBulletin::Livret,
                livret: Some(ContenuLivret {
                    date_naissance,
                    periodes: colonnes.iter().map(|p| p.libelle()).collect(),
                    echelle: liste.echelle.clone(),
                    domaines,
                    absences_relevees,
                    decision: if derniere {
                        decision(inscription.decision_fin_annee)
                    } else {
                        None
                    },
                }),
            },
        ));
    }
    Ok(Livrets {
        periode,
        contenus,
        inscriptions: inscriptions_classe,
    })
}

fn pictogramme(n: NiveauAcquisition) -> &'static str {
    match n {
        NiveauAcquisition::Acquis => "plein",
        NiveauAcquisition::EnCours => "demi",
        NiveauAcquisition::NonAcquis => "vide",
    }
}

/// Données du modèle `livret` : ce qui s'imprime, déjà écrit, avec la photo de chaque enfant
/// qui en a une et le QR de vérification. Un livret non publié porte « Projet ».
pub(crate) async fn donnees_impression(
    noyau: &Noyau,
    session: &Session,
    livrets: &[Bulletin],
) -> Resultat<(serde_json::Value, Vec<Piece>)> {
    let contexte = noyau.contexte(session.etablissement_id, Some(session));
    let mut cnx = noyau
        .base()
        .pool()
        .acquire()
        .await
        .map_err(gesco_db::ErreurBase::from)?;
    let mut pieces = Vec::new();
    let mut liste = Vec::with_capacity(livrets.len());
    for (i, b) in livrets.iter().enumerate() {
        let reference = b.id.vers_chaine();
        let mut code = qr::piece(&qr::reference(&reference))?;
        code.nom = format!("qr-{i}.svg");
        let inscription = inscriptions::lire(&mut cnx, b.inscription_id, &contexte).await?;
        let (_, identite) = personnes::eleve(&mut cnx, inscription.eleve_id, &contexte).await?;
        // La photo si la fiche en a une ; sinon rien à sa place (§5).
        let photo = personnes::photo(&mut cnx, identite.id, &contexte)
            .await?
            .map(|(octets, format)| Piece {
                nom: format!("photo-{i}.{}", if format == "png" { "png" } else { "jpg" }),
                octets,
            });
        let c = &b.contenu;
        let vide = ContenuLivret {
            date_naissance: None,
            periodes: Vec::new(),
            echelle: Vec::new(),
            domaines: Vec::new(),
            absences_relevees: false,
            decision: None,
        };
        let l = c.livret.as_ref().unwrap_or(&vide);
        let etat = match b.statut {
            StatutBulletin::Calcule | StatutBulletin::Valide => {
                "Projet — livret non publié".to_owned()
            }
            StatutBulletin::Publie if b.version > 1 => format!("Version corrigée n° {}", b.version),
            StatutBulletin::Publie => String::new(),
            StatutBulletin::Remplace => format!("Version n° {}, remplacée", b.version),
        };
        let libelle_niveau = |n: NiveauAcquisition| {
            l.echelle
                .get(match n {
                    NiveauAcquisition::Acquis => 0,
                    NiveauAcquisition::EnCours => 1,
                    NiveauAcquisition::NonAcquis => 2,
                })
                .cloned()
                .unwrap_or_else(|| n.libelle_livre().to_owned())
        };
        liste.push(serde_json::json!({
            "eleve": c.eleve,
            "naissance": l.date_naissance.map(crate::documents::date_civile_longue),
            "classe": c.classe,
            "section": c.niveau,
            "annee": c.annee,
            "periode": c.periode,
            "photo": photo.as_ref().map(Piece::chemin),
            "periodes": l.periodes,
            "echelle": NiveauAcquisition::TOUS.iter().map(|n| serde_json::json!({
                "libelle": libelle_niveau(*n),
                "picto": pictogramme(*n),
            })).collect::<Vec<_>>(),
            "domaines": l.domaines.iter().map(|d| serde_json::json!({
                "libelle": d.libelle,
                "commentaire": d.commentaire,
                "competences": d.competences.iter().map(|k| serde_json::json!({
                    "libelle": match &k.section {
                        Some(s) => format!("{} ({s})", k.libelle),
                        None => k.libelle.clone(),
                    },
                    "niveaux": k.niveaux.iter().map(|n| n.map(|n| serde_json::json!({
                        "libelle": libelle_niveau(n),
                        "picto": pictogramme(n),
                    }))).collect::<Vec<_>>(),
                })).collect::<Vec<_>>(),
            })).collect::<Vec<_>>(),
            "appreciation_generale": c.appreciation_generale,
            "absences": l.absences_relevees.then_some(c.absences.demi_journees),
            "decision": l.decision,
            "etat": etat,
            "qr": code.chemin(),
            "reference": reference,
        }));
        pieces.push(code);
        pieces.extend(photo);
    }
    Ok((serde_json::json!({ "livrets": liste }), pieces))
}
