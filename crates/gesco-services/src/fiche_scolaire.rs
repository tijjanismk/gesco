//! Fiche scolaire de l'élève (SPEC_PHASE_9 §5.3, §9.4) : une page par élève, demandée par le
//! CAP, imprimée depuis la fiche de l'élève ou pour toute une classe. Ce que GESCO connaît est
//! rempli ; le reste (autorité qui a délivré la pièce, scolarité d'avant l'école…) laisse une
//! ligne à remplir à la main. Rien n'y dit qu'une donnée manque (règle 3), et la fiche sort
//! toujours, même pour une classe vide (règle 1).

use std::collections::HashMap;

use gesco_core::commun::Id;
use gesco_core::core::Examen;
use gesco_core::core::rapports::QUALITE_PAR_DEFAUT;
use gesco_core::scolarite::{Classe, Cycle, Eleve, Lien, NatureActe, Niveau, StatutInscription};
use gesco_db::depots::{academique, annee_scolaire, complements, examens, fiche, inscriptions};
use gesco_db::depots::{etablissement, personnes, rapports};
use serde::{Deserialize, Serialize};

use crate::documents::date_civile_longue;
use crate::erreur::{ErreurService, Resultat};
use crate::noyau::Noyau;
use crate::rapports::reponse_fiche;
use crate::session::Session;

/// Les fiches à imprimer : celle d'un élève, ou celles des élèves d'une classe.
#[derive(Debug, Clone, Deserialize, specta::Type)]
pub struct DemandeFicheScolaire {
    /// Élève.
    pub eleve_id: Option<Id<Eleve>>,
    /// Classe : une fiche par élève inscrit.
    pub classe_id: Option<Id<Classe>>,
}

/// Scolarité dans un cycle : une colonne par année d'études.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct ScolariteCycle {
    /// Intitulé du cycle.
    pub cycle: String,
    /// Années d'études du cycle.
    pub niveaux: Vec<String>,
    /// Années scolaires passées dans chaque année d'études.
    pub periodes: Vec<String>,
    /// École de chaque année d'études.
    pub ecoles: Vec<String>,
    /// Cercle de chaque année d'études.
    pub cercles: Vec<String>,
    /// Années d'études doublées.
    pub doublees: String,
}

/// Une fiche scolaire, prête à imprimer ; une chaîne vide laisse la ligne à remplir à la main.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct FicheScolaire {
    /// École.
    pub ecole: String,
    /// Commune de l'école.
    pub commune: String,
    /// Cercle de l'école.
    pub cercle: String,
    /// Région de l'école.
    pub region: String,
    /// Nom.
    pub nom: String,
    /// Prénoms.
    pub prenoms: String,
    /// Date de naissance, en toutes lettres.
    pub naissance: String,
    /// Lieu de naissance.
    pub lieu: String,
    /// Nature de la pièce d'état civil.
    pub piece: String,
    /// Numéro et date de la pièce.
    pub piece_numero: String,
    /// Autorité qui a délivré la pièce : à la main.
    pub delivree: String,
    /// Année de recrutement.
    pub recrutement: String,
    /// Matricule national, sinon celui de l'école.
    pub matricule: String,
    /// Père.
    pub pere: String,
    /// Profession du père.
    pub pere_profession: String,
    /// Mère.
    pub mere: String,
    /// Profession de la mère.
    pub mere_profession: String,
    /// Domicile.
    pub domicile: String,
    /// Tuteur : le premier contact, quand il n'est ni le père ni la mère.
    pub tuteur: String,
    /// Lien de parenté du tuteur.
    pub tuteur_lien: String,
    /// Adresse du tuteur.
    pub tuteur_adresse: String,
    /// Scolarité par cycle du fondamental.
    pub scolarite: Vec<ScolariteCycle>,
    /// École fréquentée avant celle-ci.
    pub origine: String,
    /// Session du DEF.
    pub def_date: String,
    /// Résultat au DEF.
    pub def_resultat: String,
    /// Qualité du signataire.
    pub signataire: String,
}

/// Ce que toutes les fiches d'une impression partagent.
struct Commun {
    ecole: String,
    commune: String,
    cercle: String,
    region: String,
    signataire: String,
    niveaux: Vec<Niveau>,
}

/// Service des fiches scolaires.
pub struct ServiceFicheScolaire<'n> {
    noyau: &'n Noyau,
}

/// Une inscription comptée dans la scolarité : pas un brouillon, pas une inscription annulée
/// avant d'avoir servi.
const fn compte(statut: StatutInscription) -> bool {
    !matches!(statut, StatutInscription::Brouillon)
}

fn titre_cycle(c: Cycle) -> &'static str {
    if c == Cycle::PremierCycle {
        "Premier cycle"
    } else {
        "Second cycle"
    }
}

impl<'n> ServiceFicheScolaire<'n> {
    /// Service sur le noyau ouvert.
    pub const fn nouveau(noyau: &'n Noyau) -> Self {
        Self { noyau }
    }

    /// Les fiches demandées, dans l'ordre alphabétique pour une classe ; une fiche vierge quand
    /// la classe n'a aucun élève.
    pub async fn fiches(
        &self,
        session: &Session,
        demande: &DemandeFicheScolaire,
    ) -> Resultat<Vec<FicheScolaire>> {
        let contexte = self.noyau.contexte(session.etablissement_id, Some(session));
        let mut cnx = self
            .noyau
            .base()
            .pool()
            .acquire()
            .await
            .map_err(gesco_db::ErreurBase::from)?;
        let mut eleves: Vec<Id<Eleve>> = Vec::new();
        if let Some(id) = demande.eleve_id {
            eleves.push(id);
        }
        if let Some(classe_id) = demande.classe_id {
            for i in inscriptions::de_la_classe(&mut cnx, classe_id, &contexte).await? {
                if compte(i.statut) && !eleves.contains(&i.eleve_id) {
                    eleves.push(i.eleve_id);
                }
            }
        }
        if demande.eleve_id.is_none() && demande.classe_id.is_none() {
            return Err(ErreurService::Regle {
                cle: "commande.entree_invalide",
            });
        }
        let ecole = etablissement::lire(&mut cnx, session.etablissement_id).await?;
        let rubriques = fiche::rubriques(&mut cnx, &contexte).await?;
        let valeurs = fiche::valeurs(&mut cnx, &contexte).await?;
        let reponse = |code: &str| reponse_fiche(&rubriques, &valeurs, "", code);
        let signataire = rapports::dernier_regle(&mut cnx, &contexte)
            .await?
            .map(|r| r.qualite_signataire)
            .filter(|q| !q.is_empty())
            .unwrap_or_else(|| QUALITE_PAR_DEFAUT.to_owned());
        let mut niveaux = academique::niveaux(&mut cnx, &contexte).await?;
        niveaux.retain(|n| matches!(n.cycle, Cycle::PremierCycle | Cycle::SecondCycle));
        niveaux.sort_by_key(|n| n.ordre);
        let commun = Commun {
            ecole: ecole.nom.clone(),
            commune: reponse("commune"),
            cercle: reponse("cercle"),
            region: reponse("region"),
            signataire,
            niveaux,
        };
        let annees_recrutement = complements::annees_recrutement(&mut cnx, &contexte).await?;
        let mut fiches = Vec::new();
        for eleve_id in eleves {
            fiches.push(
                self.fiche(&mut cnx, &contexte, &commun, &annees_recrutement, eleve_id)
                    .await?,
            );
        }
        fiches.sort_by(|a, b| (&a.nom, &a.prenoms).cmp(&(&b.nom, &b.prenoms)));
        if fiches.is_empty() {
            fiches.push(FicheScolaire {
                scolarite: scolarite(&commun, &[]),
                ..vierge(&commun)
            });
        }
        Ok(fiches)
    }

    async fn fiche(
        &self,
        cnx: &mut gesco_db::Connexion,
        contexte: &gesco_db::Contexte,
        commun: &Commun,
        annees_recrutement: &HashMap<String, i64>,
        eleve_id: Id<Eleve>,
    ) -> Resultat<FicheScolaire> {
        let (eleve, identite) = personnes::eleve(cnx, eleve_id, contexte).await?;
        let mut liens = personnes::liens_de_l_eleve(cnx, eleve.id, contexte).await?;
        liens.sort_by_key(|(_, _, q)| q.ordre_priorite);
        let mut fiche = vierge(commun);
        fiche.nom = identite.nom.clone();
        fiche.prenoms = identite.prenoms.clone();
        fiche.naissance = identite
            .date_naissance
            .map(date_civile_longue)
            .unwrap_or_default();
        fiche.lieu = identite.lieu_naissance.clone();
        fiche.piece = match eleve.acte_nature {
            Some(NatureActe::Acte) => "Acte de naissance".to_owned(),
            Some(NatureActe::JugementSuppletif) => "Jugement supplétif".to_owned(),
            None => String::new(),
        };
        fiche.piece_numero = match eleve.acte_date {
            Some(d) if !eleve.acte_numero.is_empty() => {
                format!("{} du {}", eleve.acte_numero, date_civile_longue(d))
            }
            _ => eleve.acte_numero.clone(),
        };
        fiche.recrutement = annees_recrutement
            .get(&eleve.id.vers_chaine())
            .map(ToString::to_string)
            .unwrap_or_default();
        fiche.matricule = eleve
            .matricule_national
            .clone()
            .filter(|m| !m.trim().is_empty())
            .unwrap_or_else(|| eleve.matricule_interne.clone());
        let mut adresse_parent = String::new();
        let mut tuteur_vu = false;
        for (k, (_, responsable_id, q)) in liens.iter().enumerate() {
            let (responsable, p) = personnes::responsable(cnx, *responsable_id, contexte).await?;
            match q.lien {
                Lien::Pere if fiche.pere.is_empty() => {
                    fiche.pere = p.nom_complet();
                    fiche.pere_profession = responsable.profession.clone();
                }
                Lien::Mere if fiche.mere.is_empty() => {
                    fiche.mere = p.nom_complet();
                    fiche.mere_profession = responsable.profession.clone();
                }
                _ => {}
            }
            if matches!(q.lien, Lien::Pere | Lien::Mere) && adresse_parent.is_empty() {
                adresse_parent = p.adresse.clone();
            }
            // Le tuteur : le premier contact, quand il n'est ni le père ni la mère.
            if k == 0 && matches!(q.lien, Lien::Tuteur | Lien::Autre) && !tuteur_vu {
                tuteur_vu = true;
                fiche.tuteur = p.nom_complet();
                fiche.tuteur_lien = if matches!(q.lien, Lien::Tuteur) {
                    "Tuteur".to_owned()
                } else {
                    String::new()
                };
                fiche.tuteur_adresse = p.adresse.clone();
            }
        }
        fiche.domicile = if identite.adresse.trim().is_empty() {
            adresse_parent
        } else {
            identite.adresse.clone()
        };

        // Scolarité dans l'école : une ligne par inscription comptée.
        let mut passages: Vec<Passage> = Vec::new();
        let mut origine = eleve.ecole_origine.trim().to_owned();
        let mut annees: HashMap<String, String> = HashMap::new();
        let mut toutes = inscriptions::de_l_eleve(cnx, eleve.id, contexte).await?;
        toutes.reverse();
        for i in toutes.iter().filter(|i| compte(i.statut)) {
            if origine.is_empty() {
                origine = i.etablissement_origine.trim().to_owned();
            }
            let classe = academique::classe(cnx, i.classe_id, contexte).await?;
            let annee_id = i.annee_id.vers_chaine();
            if !annees.contains_key(&annee_id) {
                let a = annee_scolaire::lire(cnx, i.annee_id, contexte).await?;
                annees.insert(annee_id.clone(), a.libelle);
            }
            let resultat_def = if commun
                .niveaux
                .iter()
                .any(|n| n.id == classe.niveau_id && n.cycle == Cycle::SecondCycle)
            {
                examens::resultats(cnx, i.annee_id, contexte)
                    .await?
                    .into_iter()
                    .find(|r| r.inscription_id == i.id.vers_chaine() && r.examen == Examen::Def)
            } else {
                None
            };
            if let Some(r) = resultat_def {
                let texte = match (r.inscrit, r.present, r.admis) {
                    (_, _, Some(true)) => "Admis",
                    (_, Some(false), _) => "Absent",
                    (_, Some(true), Some(false)) => "Non admis",
                    _ => "",
                };
                if !texte.is_empty() {
                    fiche.def_resultat = if r.mention.trim().is_empty() {
                        texte.to_owned()
                    } else {
                        format!("{texte}, mention {}", r.mention.trim())
                    };
                    fiche.def_date = annees.get(&annee_id).cloned().unwrap_or_default();
                }
            }
            passages.push(Passage {
                niveau_id: classe.niveau_id,
                annee: annees.get(&annee_id).cloned().unwrap_or_default(),
                redoublant: i.redoublant,
            });
        }
        fiche.scolarite = scolarite(commun, &passages);
        fiche.origine = origine;
        Ok(fiche)
    }
}

/// Une année passée dans l'école.
struct Passage {
    niveau_id: Id<Niveau>,
    annee: String,
    redoublant: bool,
}

fn vierge(commun: &Commun) -> FicheScolaire {
    FicheScolaire {
        ecole: commun.ecole.clone(),
        commune: commun.commune.clone(),
        cercle: commun.cercle.clone(),
        region: commun.region.clone(),
        signataire: commun.signataire.clone(),
        ..FicheScolaire::default()
    }
}

/// Scolarité par cycle du fondamental : pour chaque année d'études, les années scolaires passées
/// dans l'école, son nom et son cercle ; les années d'études faites deux fois ou plus, ou
/// marquées redoublées, sont « doublées ». Un niveau sans passage reste à remplir à la main.
fn scolarite(commun: &Commun, passages: &[Passage]) -> Vec<ScolariteCycle> {
    [Cycle::PremierCycle, Cycle::SecondCycle]
        .into_iter()
        .filter_map(|cycle| {
            let niveaux: Vec<&Niveau> = commun
                .niveaux
                .iter()
                .filter(|n| n.cycle == cycle && n.actif)
                .collect();
            if niveaux.is_empty() {
                return None;
            }
            let mut c = ScolariteCycle {
                cycle: titre_cycle(cycle).to_owned(),
                ..ScolariteCycle::default()
            };
            let mut doublees = Vec::new();
            for n in niveaux {
                let du_niveau: Vec<&Passage> =
                    passages.iter().filter(|p| p.niveau_id == n.id).collect();
                let vu = !du_niveau.is_empty();
                c.niveaux.push(n.libelle.clone());
                c.periodes.push(
                    du_niveau
                        .iter()
                        .map(|p| p.annee.clone())
                        .collect::<Vec<_>>()
                        .join(", "),
                );
                c.ecoles.push(if vu {
                    commun.ecole.clone()
                } else {
                    String::new()
                });
                c.cercles.push(if vu {
                    commun.cercle.clone()
                } else {
                    String::new()
                });
                if du_niveau.len() > 1 || du_niveau.iter().any(|p| p.redoublant) {
                    doublees.push(n.libelle.clone());
                }
            }
            c.doublees = doublees.join(", ");
            Some(c)
        })
        .collect()
}

/// Données du modèle d'impression `fiche_scolaire`.
pub fn donnees_impression(fiches: &[FicheScolaire]) -> serde_json::Value {
    serde_json::json!({ "fiches": fiches })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn niveau(libelle: &str, cycle: Cycle, ordre: i64) -> Niveau {
        Niveau {
            id: gesco_db::nouvel_id(),
            etablissement_id: gesco_db::nouvel_id(),
            code: libelle.to_owned(),
            libelle: libelle.to_owned(),
            cycle,
            ordre,
            mode_evaluation: gesco_core::scolarite::ModeEvaluation::Notes,
            diplome_terminal: gesco_core::scolarite::DiplomeTerminal::Aucun,
            actif: true,
        }
    }

    #[test]
    fn une_annee_faite_deux_fois_est_doublee() {
        let n1 = niveau("1re année", Cycle::PremierCycle, 1);
        let n2 = niveau("2e année", Cycle::PremierCycle, 2);
        let n7 = niveau("7e année", Cycle::SecondCycle, 7);
        let commun = Commun {
            ecole: "École d'essai".to_owned(),
            commune: String::new(),
            cercle: "Cercle d'essai".to_owned(),
            region: String::new(),
            signataire: QUALITE_PAR_DEFAUT.to_owned(),
            niveaux: vec![n1.clone(), n2.clone(), n7],
        };
        let passages = [
            Passage {
                niveau_id: n1.id,
                annee: "2023-2024".to_owned(),
                redoublant: false,
            },
            Passage {
                niveau_id: n1.id,
                annee: "2024-2025".to_owned(),
                redoublant: true,
            },
            Passage {
                niveau_id: n2.id,
                annee: "2025-2026".to_owned(),
                redoublant: false,
            },
        ];
        let s = scolarite(&commun, &passages);
        assert_eq!(s.len(), 2);
        let premier = s.first().cloned().unwrap_or_default();
        assert_eq!(premier.periodes, ["2023-2024, 2024-2025", "2025-2026"]);
        assert_eq!(premier.doublees, "1re année");
        assert_eq!(premier.cercles, ["Cercle d'essai", "Cercle d'essai"]);
        let second = s.get(1).cloned().unwrap_or_default();
        assert_eq!(second.periodes, [""]);
        assert_eq!(second.ecoles, [""]);
        assert!(second.doublees.is_empty());
    }
}
