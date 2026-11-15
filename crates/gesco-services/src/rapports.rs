//! Rapports de l'école (SPEC_PHASE_9 §3, §4, §5, §8.6, §9.3) : le rapport succinct de rentrée,
//! le rapport de rentrée et le rapport de fermeture, pour le fondamental. La direction rédige les textes et règle
//! la date d'arrêt et la signature ; les tableaux se calculent à la date d'arrêt et ne se
//! retouchent pas (règle 9). Un rapport sort toujours, même vide (règle 1). Arrêté, il est figé
//! avec son document ; rouvert, il passe à sa version suivante (règle 10).

use gesco_core::commun::{DateCivile, Id};
use gesco_core::core::complements::Fonctionnement;
use gesco_core::core::fiche::SEPARATEUR;
use gesco_core::core::rapports::{
    QUALITE_PAR_DEFAUT, SectionModele, section_connue, verifier_reglage, verifier_texte,
};
use gesco_core::core::{AnneeScolaire, EtatRapport, Operation, StatutEtablissement, TypeRapport};
use gesco_db::depots::document::{self, Document};
use gesco_db::depots::rapports::{self, EffectifProbable, LigneRapport, NouveauRapport};
use gesco_db::depots::{annee_scolaire, complements, etablissement, fiche};
use gesco_db::unite::Durabilite;
use serde::{Deserialize, Serialize};

use crate::documents::date_civile_longue;
use crate::erreur::{ErreurService, Resultat};
use crate::fraicheur::{self, Ligne};
use crate::noyau::Noyau;
use crate::rapports_tableaux::{
    self, Annexes, AnnexesFermeture, EffectifProbableNiveau, FONDAMENTAL, TableauRapport,
};
use crate::session::Session;
use crate::statistiques::{AnneeStatistiques, DemandeStatistiques, ServiceStatistiques};

fn regle(cle: &'static str) -> ErreurService {
    ErreurService::Regle { cle }
}

/// Seul périmètre des rapports de rentrée : le fondamental, premier et second cycles ensemble.
const PERIMETRE: &str = "fondamental";

/// Les rapports d'une année.
#[derive(Debug, Clone, Default, Deserialize, specta::Type)]
pub struct DemandeRapports {
    /// Année ; l'année courante si rien n'est donné.
    pub annee_id: Option<Id<AnneeScolaire>>,
}

/// Un rapport de la liste : préparé ou pas encore.
#[derive(Debug, Clone, Serialize, specta::Type)]
pub struct ResumeRapport {
    /// Type.
    pub type_rapport: TypeRapport,
    /// Identifiant, s'il est préparé.
    pub id: Option<String>,
    /// État, s'il est préparé.
    pub etat: Option<EtatRapport>,
    /// Version, s'il est préparé.
    pub version: Option<i64>,
    /// Date d'arrêt, s'il est préparé.
    pub date_arret: Option<DateCivile>,
}

/// Rapports d'une année.
#[derive(Debug, Clone, Serialize, specta::Type)]
pub struct ListeRapports {
    /// Années qu'on peut choisir, de la plus récente à la plus ancienne.
    pub annees: Vec<AnneeStatistiques>,
    /// Année retenue.
    pub annee_id: Option<Id<AnneeScolaire>>,
    /// Un résumé par type de rapport.
    pub rapports: Vec<ResumeRapport>,
}

/// Préparer le rapport d'un type pour une année : il est créé s'il n'existe pas.
#[derive(Debug, Clone, Deserialize, specta::Type)]
pub struct DemandePreparationRapport {
    /// Année.
    pub annee_id: Id<AnneeScolaire>,
    /// Type.
    pub type_rapport: TypeRapport,
}

/// Désigne un rapport.
#[derive(Debug, Clone, Deserialize, specta::Type)]
pub struct DemandeRapport {
    /// RapportEcole.
    pub id: String,
}

/// Réglages d'un rapport en préparation.
#[derive(Debug, Clone, Deserialize, specta::Type)]
pub struct DemandeReglageRapport {
    /// RapportEcole.
    pub id: String,
    /// Date d'arrêt des chiffres.
    pub date_arret: DateCivile,
    /// Lieu de « Fait à ».
    pub lieu: String,
    /// Date de signature.
    pub date_signature: Option<DateCivile>,
    /// Qualité du signataire.
    pub qualite_signataire: String,
}

/// Texte d'une section, avec l'instant lu avec lui.
#[derive(Debug, Clone, Deserialize, specta::Type)]
pub struct DemandeTexteRapport {
    /// RapportEcole.
    pub id: String,
    /// Code de la section.
    pub section: String,
    /// Texte.
    pub texte: String,
    /// Instant de dernière modification lu avec la section.
    pub lue_le: i64,
}

/// Effectifs probables à la rentrée d'un niveau, saisis dans un rapport de fermeture : une case
/// vide reprend le nombre proposé.
#[derive(Debug, Clone, Deserialize, specta::Type)]
pub struct DemandeEffectifProbable {
    /// RapportEcole.
    pub id: String,
    /// Niveau.
    pub niveau_id: String,
    /// Garçons ; rien pour garder le nombre proposé.
    pub garcons: Option<u32>,
    /// Filles ; rien pour garder le nombre proposé.
    pub filles: Option<u32>,
}

/// Effectif probable le plus grand qu'on puisse saisir pour un niveau.
const EFFECTIF_MAX: u32 = 100_000;

/// Une ligne de l'en-tête administratif.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
pub struct LigneEntete {
    /// Rubrique.
    pub libelle: String,
    /// Valeur ; vide quand elle n'est pas connue.
    pub valeur: String,
}

/// Une section rédigée, telle qu'imprimée.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
pub struct SectionImprimee {
    /// Titre.
    pub titre: String,
    /// Texte.
    pub texte: String,
}

/// Contenu d'un rapport : ce qui s'imprime. Figé à l'arrêt.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
pub struct ContenuRapport {
    /// Titre.
    pub titre: String,
    /// Année scolaire.
    pub annee: String,
    /// Date d'arrêt des chiffres, en toutes lettres.
    pub arret: String,
    /// En-tête administratif.
    pub entete: Vec<LigneEntete>,
    /// Sections rédigées.
    pub sections: Vec<SectionImprimee>,
    /// Tableaux.
    pub tableaux: Vec<TableauRapport>,
    /// « Fait à … le … ».
    pub fait_a: String,
    /// Qualité du signataire.
    pub qualite: String,
}

/// Une section rédigée à l'écran.
#[derive(Debug, Clone, Serialize, specta::Type)]
pub struct SectionRapport {
    /// Code.
    pub code: String,
    /// Titre.
    pub titre: String,
    /// Texte.
    pub texte: String,
    /// Instant de dernière modification ; zéro si rien n'est encore écrit.
    pub lue_le: i64,
    /// Texte de la même section dans le rapport de l'an dernier, s'il y en a un.
    pub an_dernier: Option<String>,
    /// Chiffres du jour proposés à insérer (section aidée).
    pub propose: Option<String>,
}

/// Un rapport ouvert.
#[derive(Debug, Clone, Serialize, specta::Type)]
pub struct RapportEcole {
    /// Identifiant.
    pub id: String,
    /// Année.
    pub annee_id: String,
    /// Type.
    pub type_rapport: TypeRapport,
    /// État.
    pub etat: EtatRapport,
    /// Version.
    pub version: i64,
    /// Date d'arrêt des chiffres.
    pub date_arret: DateCivile,
    /// Lieu de « Fait à ».
    pub lieu: String,
    /// Date de signature.
    pub date_signature: Option<DateCivile>,
    /// Qualité du signataire.
    pub qualite_signataire: String,
    /// Sections rédigées, dans l'ordre du modèle.
    pub sections: Vec<SectionRapport>,
    /// Contenu imprimé : calculé à la date d'arrêt, ou figé si le rapport est arrêté.
    pub contenu: ContenuRapport,
    /// Effectifs probables à la rentrée, proposés et saisis, par niveau : rapport de fermeture
    /// en préparation seulement.
    pub effectifs_probables: Vec<EffectifProbableNiveau>,
    /// Documents imprimés aux arrêts successifs, du plus ancien au plus récent.
    pub documents: Vec<Document>,
}

/// Service des rapports.
pub struct ServiceRapports<'n> {
    noyau: &'n Noyau,
}

impl<'n> ServiceRapports<'n> {
    /// Service sur le noyau ouvert.
    pub const fn nouveau(noyau: &'n Noyau) -> Self {
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

    /// Les rapports de l'année : un par type, préparé ou non.
    pub async fn lister(
        &self,
        session: &Session,
        demande: &DemandeRapports,
    ) -> Resultat<ListeRapports> {
        let contexte = self.noyau.contexte(session.etablissement_id, Some(session));
        let mut cnx = self.connexion().await?;
        let mut annees = annee_scolaire::lister(&mut cnx, &contexte).await?;
        annees.sort_by_key(|a| std::cmp::Reverse(a.debut.millis_epoque()));
        let choisie = match demande.annee_id {
            Some(id) => annees.iter().find(|a| a.id == id),
            None => annees
                .iter()
                .find(|a| a.est_courante)
                .or_else(|| annees.first()),
        };
        let existants = match choisie {
            Some(a) => rapports::de_l_annee(&mut cnx, &a.id.vers_chaine(), &contexte).await?,
            None => Vec::new(),
        };
        let rapports = TypeRapport::TOUS
            .iter()
            .map(|t| {
                let r = existants
                    .iter()
                    .find(|r| r.type_rapport == t.nom() && r.perimetre == PERIMETRE);
                ResumeRapport {
                    type_rapport: *t,
                    id: r.map(|r| r.id.clone()),
                    etat: r.map(|r| EtatRapport::depuis_nom(&r.etat)),
                    version: r.map(|r| r.version),
                    date_arret: r.and_then(|r| DateCivile::analyser(&r.date_arret)),
                }
            })
            .collect();
        Ok(ListeRapports {
            annee_id: choisie.map(|a| a.id),
            annees: annees
                .iter()
                .map(|a| AnneeStatistiques {
                    id: a.id,
                    libelle: a.libelle.clone(),
                })
                .collect(),
            rapports,
        })
    }

    /// Prépare le rapport d'un type pour l'année : le crée s'il n'existe pas, avec la date
    /// d'arrêt proposée (quinze jours après la rentrée), le lieu et la qualité du signataire du
    /// dernier rapport réglé ; rend le rapport.
    pub async fn preparer(
        &self,
        session: &Session,
        demande: DemandePreparationRapport,
    ) -> Resultat<RapportEcole> {
        let contexte = self.noyau.contexte(session.etablissement_id, Some(session));
        let mut unite = self
            .noyau
            .commencer(contexte.clone(), Durabilite::Ordinaire)
            .await?;
        let annee = annee_scolaire::lire(unite.connexion(), demande.annee_id, &contexte).await?;
        let annee_id = annee.id.vers_chaine();
        let existant = rapports::du_type(
            unite.connexion(),
            &annee_id,
            demande.type_rapport.nom(),
            PERIMETRE,
            &contexte,
        )
        .await?;
        let id = if let Some(r) = existant {
            unite.annuler().await?;
            r.id
        } else {
            let rentree =
                DateCivile::du_jour(annee.debut).ok_or(regle("commande.entree_invalide"))?;
            let fin = DateCivile::du_jour(annee.fin).unwrap_or(rentree);
            let date_arret = demande
                .type_rapport
                .date_arret_proposee(rentree, fin)
                .to_string();
            let dernier = rapports::dernier_regle(unite.connexion(), &contexte).await?;
            let lieu = dernier.as_ref().map(|r| r.lieu.clone()).unwrap_or_default();
            let qualite = dernier
                .as_ref()
                .map(|r| r.qualite_signataire.clone())
                .filter(|q| !q.is_empty())
                .unwrap_or_else(|| QUALITE_PAR_DEFAUT.to_owned());
            let id = rapports::inserer(
                unite.connexion(),
                NouveauRapport {
                    annee_id: &annee_id,
                    type_rapport: demande.type_rapport.nom(),
                    perimetre: PERIMETRE,
                    date_arret: &date_arret,
                    lieu: &lieu,
                    qualite_signataire: &qualite,
                },
                &contexte,
            )
            .await?;
            unite.auditer(
                "rapport_ecole",
                &id,
                Operation::Creation,
                None,
                Some(serde_json::json!({
                    "annee_id": annee_id, "type_rapport": demande.type_rapport.nom(),
                    "perimetre": PERIMETRE, "date_arret": date_arret,
                })),
                "rapports.preparer",
            );
            self.noyau.terminer(unite).await?;
            id
        };
        self.lire(session, &DemandeRapport { id }).await
    }

    /// Un rapport, avec ses textes, son contenu et ses documents.
    pub async fn lire(
        &self,
        session: &Session,
        demande: &DemandeRapport,
    ) -> Resultat<RapportEcole> {
        let contexte = self.noyau.contexte(session.etablissement_id, Some(session));
        let mut cnx = self.connexion().await?;
        let ligne = rapports::lire(&mut cnx, &demande.id, &contexte).await?;
        let type_rapport = TypeRapport::depuis_nom(&ligne.type_rapport)
            .ok_or(regle("commande.entree_invalide"))?;
        let textes = rapports::textes(&mut cnx, &ligne.id, &contexte).await?;
        let documents =
            document::par_entite(&mut cnx, "rapport_ecole", &ligne.id, &contexte).await?;
        let an_dernier = self.textes_an_dernier(&mut cnx, &ligne, &contexte).await?;
        drop(cnx);
        let etat = EtatRapport::depuis_nom(&ligne.etat);
        let fige: Option<ContenuRapport> = match (&etat, &ligne.contenu_fige) {
            (EtatRapport::Arrete, Some(json)) => serde_json::from_str(json).ok(),
            _ => None,
        };
        let calcul = match fige {
            Some(c) => Calcul {
                contenu: c,
                propositions: Vec::new(),
                effectifs_probables: Vec::new(),
            },
            None => self.calculer(session, &ligne, type_rapport).await?,
        };
        let Calcul {
            contenu,
            propositions,
            effectifs_probables,
        } = calcul;
        let sections = type_rapport
            .sections()
            .iter()
            .map(|s: &SectionModele| {
                let t = textes.iter().find(|t| t.section == s.code);
                SectionRapport {
                    code: s.code.to_owned(),
                    titre: s.titre.to_owned(),
                    texte: t.map(|t| t.texte.clone()).unwrap_or_default(),
                    lue_le: t.map_or(0, |t| t.mis_a_jour_le),
                    an_dernier: an_dernier
                        .iter()
                        .find(|x| x.section == s.code && !x.texte.is_empty())
                        .map(|x| x.texte.clone()),
                    propose: if s.aidee {
                        propositions
                            .iter()
                            .find(|(code, _)| *code == s.code)
                            .map(|(_, p)| p.clone())
                    } else {
                        None
                    },
                }
            })
            .collect();
        Ok(RapportEcole {
            id: ligne.id,
            annee_id: ligne.annee_id,
            type_rapport,
            etat,
            version: ligne.version,
            date_arret: DateCivile::analyser(&ligne.date_arret)
                .ok_or(regle("commande.entree_invalide"))?,
            lieu: ligne.lieu,
            date_signature: ligne
                .date_signature
                .as_deref()
                .and_then(DateCivile::analyser),
            qualite_signataire: ligne.qualite_signataire,
            sections,
            contenu,
            effectifs_probables,
            documents,
        })
    }

    /// Textes du rapport du même type pour l'année d'avant, s'il existe.
    async fn textes_an_dernier(
        &self,
        cnx: &mut gesco_db::Connexion,
        ligne: &LigneRapport,
        contexte: &gesco_db::Contexte,
    ) -> Resultat<Vec<rapports::TexteRapport>> {
        let mut annees = annee_scolaire::lister(cnx, contexte).await?;
        annees.sort_by_key(|a| std::cmp::Reverse(a.debut.millis_epoque()));
        let Some(k) = annees
            .iter()
            .position(|a| a.id.vers_chaine() == ligne.annee_id)
        else {
            return Ok(Vec::new());
        };
        let Some(avant) = annees.get(k + 1) else {
            return Ok(Vec::new());
        };
        match rapports::du_type(
            cnx,
            &avant.id.vers_chaine(),
            &ligne.type_rapport,
            &ligne.perimetre,
            contexte,
        )
        .await?
        {
            Some(r) => Ok(rapports::textes(cnx, &r.id, contexte).await?),
            None => Ok(Vec::new()),
        }
    }

    /// Contenu calculé à la date d'arrêt, phrases proposées pour les sections aidées et, pour le
    /// rapport de fermeture, la grille des effectifs probables.
    async fn calculer(
        &self,
        session: &Session,
        ligne: &LigneRapport,
        type_rapport: TypeRapport,
    ) -> Resultat<Calcul> {
        let annee_id: Id<AnneeScolaire> =
            Id::analyser(&ligne.annee_id).map_err(|_| regle("commande.entree_invalide"))?;
        let date_arret =
            DateCivile::analyser(&ligne.date_arret).ok_or(regle("commande.entree_invalide"))?;
        let d = ServiceStatistiques::nouveau(self.noyau)
            .charger(
                session,
                &DemandeStatistiques {
                    annee_id: Some(annee_id),
                    date: Some(date_arret),
                    cycle: None,
                },
            )
            .await?;
        let contexte = self.noyau.contexte(session.etablissement_id, Some(session));
        let mut cnx = self.connexion().await?;
        let ecole = etablissement::lire(&mut cnx, session.etablissement_id).await?;
        let rubriques = fiche::rubriques(&mut cnx, &contexte).await?;
        let valeurs = fiche::valeurs(&mut cnx, &contexte).await?;
        let classes = complements::classes(&mut cnx, &ligne.annee_id, &contexte).await?;
        let textes = rapports::textes(&mut cnx, &ligne.id, &contexte).await?;
        let probables = rapports::effectifs_probables(&mut cnx, &ligne.id, &contexte).await?;
        drop(cnx);
        let reponse = |code: &str| reponse_fiche(&rubriques, &valeurs, &ligne.annee_id, code);
        let annee = d
            .choisie
            .as_ref()
            .map(|a| a.libelle.clone())
            .unwrap_or_default();
        let entete = match type_rapport {
            TypeRapport::SuccinctRentree => {
                let cycles: Vec<_> = FONDAMENTAL
                    .iter()
                    .filter(|c| d.cycles.contains(c))
                    .collect();
                let cycles = match cycles.as_slice() {
                    [_, _] => "Cycle complet",
                    [c] if **c == gesco_core::scolarite::Cycle::PremierCycle => "1er cycle",
                    [_] => "2e cycle",
                    _ => "",
                };
                vec![
                    ("Année scolaire", annee.clone()),
                    ("École", ecole.nom.clone()),
                    ("CAP", ecole.code_cap.clone()),
                    ("Académie", ecole.code_academie.clone()),
                    ("Village ou quartier", reponse("quartier")),
                    ("Commune", reponse("commune")),
                    ("Cercle", reponse("cercle")),
                    ("Cycles", cycles.to_owned()),
                    (
                        "Statut administratif",
                        statut_administratif(ecole.statut, &reponse("confession")),
                    ),
                    ("Type d'enseignement", reponse("type_enseignement")),
                    ("Langue", reponse("langue")),
                    ("Mode de recrutement", reponse("mode_recrutement")),
                ]
            }
            TypeRapport::Rentree => vec![
                ("Région", reponse("region")),
                ("Académie", ecole.code_academie.clone()),
                ("CAP", ecole.code_cap.clone()),
                ("Commune", reponse("commune")),
                ("École", ecole.nom.clone()),
                ("Année scolaire", annee.clone()),
            ],
            TypeRapport::Fermeture => vec![
                ("Région", reponse("region")),
                ("Académie", ecole.code_academie.clone()),
                ("Cercle", reponse("cercle")),
                ("CAP", ecole.code_cap.clone()),
                ("Commune", reponse("commune")),
                ("École", ecole.nom.clone()),
                ("Année scolaire", annee.clone()),
                ("Statut de l'école", statut_administratif(ecole.statut, "")),
                ("Type de programme", reponse("type_enseignement")),
                (
                    "Type de l'école",
                    type_ecole(&reponse("type_ecole"), &classes),
                ),
                ("Milieu", reponse("milieu")),
            ],
        }
        .into_iter()
        .map(|(libelle, valeur)| LigneEntete {
            libelle: libelle.to_owned(),
            valeur,
        })
        .collect();
        let tableaux = match type_rapport {
            TypeRapport::SuccinctRentree => rapports_tableaux::succinct(&d),
            TypeRapport::Rentree => rapports_tableaux::rentree(
                &d,
                &Annexes {
                    classes: &classes,
                    defauts: crate::complements::valeurs_par_defaut(&rubriques, &valeurs),
                },
            ),
            TypeRapport::Fermeture => rapports_tableaux::fermeture(
                &d,
                &AnnexesFermeture {
                    probables: &probables,
                },
            ),
        };
        let sections = type_rapport
            .sections()
            .iter()
            .map(|s| SectionImprimee {
                titre: s.titre.to_owned(),
                texte: textes
                    .iter()
                    .find(|t| t.section == s.code)
                    .map(|t| t.texte.clone())
                    .unwrap_or_default(),
            })
            .collect();
        let propositions = match type_rapport {
            TypeRapport::SuccinctRentree => Vec::new(),
            TypeRapport::Rentree => vec![(
                "introduction",
                introduction_proposee(
                    &d,
                    date_arret,
                    &reponse("commune"),
                    &reponse("cercle"),
                    &reponse("superficie"),
                ),
            )],
            TypeRapport::Fermeture => propositions_fermeture(&d),
        };
        let le = ligne
            .date_signature
            .as_deref()
            .and_then(DateCivile::analyser)
            .map(date_civile_longue);
        let fait_a = match (ligne.lieu.is_empty(), le) {
            (false, Some(le)) => format!("Fait à {}, le {le}", ligne.lieu),
            (false, None) => format!("Fait à {}, le", ligne.lieu),
            (true, Some(le)) => format!("Fait à …, le {le}"),
            (true, None) => String::new(),
        };
        let effectifs_probables = match type_rapport {
            TypeRapport::Fermeture => rapports_tableaux::effectifs_probables(&d, &probables),
            TypeRapport::SuccinctRentree | TypeRapport::Rentree => Vec::new(),
        };
        Ok(Calcul {
            contenu: ContenuRapport {
                titre: type_rapport.titre().to_owned(),
                annee,
                arret: date_civile_longue(date_arret),
                entete,
                sections,
                tableaux,
                fait_a,
                qualite: ligne.qualite_signataire.clone(),
            },
            propositions,
            effectifs_probables,
        })
    }

    /// Règle les effectifs probables d'un niveau dans un rapport de fermeture en préparation.
    pub async fn regler_effectif(
        &self,
        session: &Session,
        demande: DemandeEffectifProbable,
    ) -> Resultat<RapportEcole> {
        if demande.garcons.is_some_and(|n| n > EFFECTIF_MAX)
            || demande.filles.is_some_and(|n| n > EFFECTIF_MAX)
        {
            return Err(regle("rapports.effectif_trop_grand"));
        }
        let contexte = self.noyau.contexte(session.etablissement_id, Some(session));
        let mut unite = self
            .noyau
            .commencer(contexte.clone(), Durabilite::Ordinaire)
            .await?;
        let ligne = rapports::lire(unite.connexion(), &demande.id, &contexte).await?;
        if EtatRapport::depuis_nom(&ligne.etat) == EtatRapport::Arrete {
            return Err(regle("rapports.arrete"));
        }
        if TypeRapport::depuis_nom(&ligne.type_rapport) != Some(TypeRapport::Fermeture) {
            return Err(regle("commande.entree_invalide"));
        }
        let effectif = EffectifProbable {
            niveau_id: demande.niveau_id.clone(),
            garcons: demande.garcons.map(i64::from),
            filles: demande.filles.map(i64::from),
        };
        let avant = rapports::ecrire_effectif_probable(
            unite.connexion(),
            &demande.id,
            &effectif,
            &contexte,
        )
        .await?;
        unite.auditer(
            "rapport_ecole",
            &demande.id,
            Operation::Modification,
            Some(serde_json::json!({
                "niveau_id": demande.niveau_id,
                "garcons": avant.as_ref().and_then(|a| a.garcons),
                "filles": avant.as_ref().and_then(|a| a.filles),
            })),
            Some(serde_json::json!({
                "niveau_id": demande.niveau_id,
                "garcons": effectif.garcons,
                "filles": effectif.filles,
            })),
            "rapports.regler_effectif",
        );
        self.noyau.terminer(unite).await?;
        self.lire(session, &DemandeRapport { id: demande.id }).await
    }

    /// Règle la date d'arrêt et la signature d'un rapport en préparation.
    pub async fn regler(
        &self,
        session: &Session,
        demande: DemandeReglageRapport,
    ) -> Resultat<RapportEcole> {
        let lieu = verifier_reglage(&demande.lieu).map_err(regle)?;
        let qualite = verifier_reglage(&demande.qualite_signataire).map_err(regle)?;
        let contexte = self.noyau.contexte(session.etablissement_id, Some(session));
        let mut unite = self
            .noyau
            .commencer(contexte.clone(), Durabilite::Ordinaire)
            .await?;
        let avant = rapports::lire(unite.connexion(), &demande.id, &contexte).await?;
        if EtatRapport::depuis_nom(&avant.etat) == EtatRapport::Arrete {
            return Err(regle("rapports.arrete"));
        }
        let date_arret = demande.date_arret.to_string();
        let signature = demande.date_signature.map(|d| d.to_string());
        rapports::regler(
            unite.connexion(),
            &demande.id,
            &date_arret,
            &lieu,
            signature.as_deref(),
            &qualite,
            &contexte,
        )
        .await?;
        unite.auditer(
            "rapport_ecole",
            &demande.id,
            Operation::Modification,
            Some(serde_json::json!({
                "date_arret": avant.date_arret, "lieu": avant.lieu,
                "date_signature": avant.date_signature, "qualite_signataire": avant.qualite_signataire,
            })),
            Some(serde_json::json!({
                "date_arret": date_arret, "lieu": lieu,
                "date_signature": signature, "qualite_signataire": qualite,
            })),
            "rapports.regler",
        );
        self.noyau.terminer(unite).await?;
        self.lire(session, &DemandeRapport { id: demande.id }).await
    }

    /// Enregistre le texte d'une section, si personne ne l'a changé depuis qu'il a été lu.
    pub async fn enregistrer_texte(
        &self,
        session: &Session,
        demande: DemandeTexteRapport,
    ) -> Resultat<RapportEcole> {
        let texte = verifier_texte(&demande.texte).map_err(regle)?;
        let contexte = self.noyau.contexte(session.etablissement_id, Some(session));
        let mut unite = self
            .noyau
            .commencer(contexte.clone(), Durabilite::Ordinaire)
            .await?;
        let ligne = rapports::lire(unite.connexion(), &demande.id, &contexte).await?;
        if EtatRapport::depuis_nom(&ligne.etat) == EtatRapport::Arrete {
            return Err(regle("rapports.arrete"));
        }
        let type_rapport = TypeRapport::depuis_nom(&ligne.type_rapport)
            .ok_or(regle("commande.entree_invalide"))?;
        if !section_connue(type_rapport, &demande.section) {
            return Err(regle("rapports.section_inconnue"));
        }
        fraicheur::verifier(
            unite.connexion(),
            &[Ligne {
                table: "texte_rapport",
                cles: vec![
                    ("rapport_id", demande.id.clone()),
                    ("section", demande.section.clone()),
                ],
            }],
            demande.lue_le,
        )
        .await?;
        let avant = rapports::ecrire_texte(
            unite.connexion(),
            &demande.id,
            &demande.section,
            &texte,
            &contexte,
        )
        .await?;
        unite.auditer(
            "rapport_ecole",
            &demande.id,
            Operation::Modification,
            Some(serde_json::json!({ "section": demande.section, "texte": avant })),
            Some(serde_json::json!({ "section": demande.section, "texte": texte })),
            "rapports.enregistrer_texte",
        );
        self.noyau.terminer(unite).await?;
        self.lire(session, &DemandeRapport { id: demande.id }).await
    }

    /// Contenu à imprimer et vrai si le rapport est déjà arrêté (son document fait foi).
    pub async fn a_imprimer(
        &self,
        session: &Session,
        demande: &DemandeRapport,
    ) -> Resultat<(ContenuRapport, Option<String>)> {
        let r = self.lire(session, demande).await?;
        let document = match r.etat {
            EtatRapport::Arrete => r.documents.last().map(|d| d.chemin.clone()),
            EtatRapport::Preparation => None,
        };
        Ok((r.contenu, document))
    }

    /// Marque le rapport arrêté, avec le document imprimé et le contenu figé.
    pub async fn marquer_arrete(
        &self,
        session: &Session,
        id: &str,
        document_id: &str,
        contenu: &ContenuRapport,
    ) -> Resultat<RapportEcole> {
        let contexte = self.noyau.contexte(session.etablissement_id, Some(session));
        let mut unite = self
            .noyau
            .commencer(contexte.clone(), Durabilite::Critique)
            .await?;
        let json = serde_json::to_string(contenu)?;
        if !rapports::arreter(unite.connexion(), id, document_id, &json, &contexte).await? {
            return Err(regle("rapports.deja_arrete"));
        }
        unite.auditer(
            "rapport_ecole",
            id,
            Operation::Modification,
            Some(serde_json::json!({ "etat": "preparation" })),
            Some(serde_json::json!({ "etat": "arrete", "document_id": document_id })),
            "rapports.arreter",
        );
        self.noyau.terminer(unite).await?;
        self.lire(session, &DemandeRapport { id: id.to_owned() })
            .await
    }

    /// Rouvre un rapport arrêté : il passe à sa version suivante ; l'ancien document reste.
    pub async fn rouvrir(
        &self,
        session: &Session,
        demande: &DemandeRapport,
    ) -> Resultat<RapportEcole> {
        let contexte = self.noyau.contexte(session.etablissement_id, Some(session));
        let mut unite = self
            .noyau
            .commencer(contexte.clone(), Durabilite::Critique)
            .await?;
        let avant = rapports::lire(unite.connexion(), &demande.id, &contexte).await?;
        if !rapports::rouvrir(unite.connexion(), &demande.id, &contexte).await? {
            return Err(regle("rapports.pas_arrete"));
        }
        unite.auditer(
            "rapport_ecole",
            &demande.id,
            Operation::Modification,
            Some(serde_json::json!({
                "etat": "arrete", "version": avant.version, "document_id": avant.document_id,
            })),
            Some(serde_json::json!({ "etat": "preparation", "version": avant.version + 1 })),
            "rapports.rouvrir",
        );
        self.noyau.terminer(unite).await?;
        self.lire(session, demande).await
    }
}

/// Réponse de la fiche de l'établissement : celle de l'année pour une rubrique annuelle, sinon
/// la permanente ; vide si rien n'est répondu.
pub(crate) fn reponse_fiche(
    rubriques: &[gesco_core::core::RubriqueFiche],
    valeurs: &[fiche::ValeurFiche],
    annee_id: &str,
    code: &str,
) -> String {
    let Some(r) = rubriques.iter().find(|r| r.code.as_deref() == Some(code)) else {
        return String::new();
    };
    let de = |annee: Option<&str>| {
        valeurs
            .iter()
            .find(|v| {
                v.rubrique_id == r.id && v.annee_id.map(|a| a.vers_chaine()).as_deref() == annee
            })
            .and_then(|v| v.valeur.clone())
    };
    de(Some(annee_id))
        .or_else(|| de(None))
        .unwrap_or_default()
        .replace(SEPARATEUR, ", ")
}

/// Ce que le calcul d'un rapport rend.
struct Calcul {
    contenu: ContenuRapport,
    propositions: Vec<(&'static str, String)>,
    effectifs_probables: Vec<EffectifProbableNiveau>,
}

/// Un nombre suivi du nom au singulier ou au pluriel.
fn nombre_de(n: usize, singulier: &str, pluriel: &str) -> String {
    format!("{n} {}", if n == 1 { singulier } else { pluriel })
}

/// Joint une liste en français : « a », « a et b », « a, b et c ».
fn joindre(parties: &[String]) -> String {
    match parties {
        [] => String::new(),
        [seule] => seule.clone(),
        [debut @ .., derniere] => format!("{} et {derniere}", debut.join(", ")),
    }
}

fn du_cycle(c: gesco_core::scolarite::Cycle) -> &'static str {
    if c == gesco_core::scolarite::Cycle::PremierCycle {
        "au premier cycle"
    } else {
        "au second cycle"
    }
}

/// Phrase proposée pour l'introduction du rapport de rentrée : les chiffres du jour.
fn introduction_proposee(
    d: &crate::statistiques::DonneesStat,
    date_arret: DateCivile,
    commune: &str,
    cercle: &str,
    superficie: &str,
) -> String {
    let (enseignants, eleves, filles) = rapports_tableaux::chiffres_du_jour(d);
    let mut propose = format!(
        "Au {}, l'école compte {enseignants} {} et {eleves} {}, dont {filles} {}.",
        date_civile_longue(date_arret),
        if enseignants == 1 {
            "enseignant"
        } else {
            "enseignants"
        },
        if eleves == 1 { "élève" } else { "élèves" },
        if filles == 1 { "fille" } else { "filles" },
    );
    match (commune.is_empty(), cercle.is_empty()) {
        (false, false) => {
            propose.push_str(&format!(
                " Elle est située dans la commune de {commune}, cercle de {cercle}."
            ));
        }
        (false, true) => {
            propose.push_str(&format!(" Elle est située dans la commune de {commune}."))
        }
        (true, false) => propose.push_str(&format!(" Elle est située dans le cercle de {cercle}.")),
        (true, true) => {}
    }
    if !superficie.is_empty() {
        propose.push_str(&format!(" Le domaine scolaire couvre {superficie} m²."));
    }
    propose
}

/// Phrases proposées pour le rapport de fermeture : état des lieux (personnel et effectifs par
/// cycle), besoins relevés à l'inventaire, taux d'admission en classe supérieure. Rien n'est
/// proposé pour une partie qui n'a pas de chiffres.
fn propositions_fermeture(d: &crate::statistiques::DonneesStat) -> Vec<(&'static str, String)> {
    let (cycles, besoins) = rapports_tableaux::chiffres_fermeture(d);
    let mut propositions = Vec::new();
    let mut etat = Vec::new();
    let enseignants: Vec<String> = cycles
        .iter()
        .filter(|c| c.enseignants > 0)
        .map(|c| {
            format!(
                "{} {}",
                nombre_de(c.enseignants, "enseignant", "enseignants"),
                du_cycle(c.cycle)
            )
        })
        .collect();
    if !enseignants.is_empty() {
        etat.push(format!("Personnel : {}.", joindre(&enseignants)));
    }
    for c in cycles.iter().filter(|c| c.total > 0) {
        let mut phrase = format!(
            "L'école compte {} {}, dont {} et {}",
            nombre_de(c.total, "élève", "élèves"),
            du_cycle(c.cycle),
            nombre_de(c.filles, "fille", "filles"),
            nombre_de(c.garcons, "garçon", "garçons"),
        );
        let niveaux: Vec<String> = c
            .niveaux
            .iter()
            .map(|(libelle, n)| format!("{n} en {libelle}"))
            .collect();
        if !niveaux.is_empty() {
            phrase.push_str(" : ");
            phrase.push_str(&joindre(&niveaux));
        }
        phrase.push('.');
        etat.push(phrase);
    }
    if !etat.is_empty() {
        propositions.push(("etat_des_lieux", etat.join(" ")));
    }
    if !besoins.is_empty() {
        let liste: Vec<String> = besoins
            .iter()
            .map(|(designation, n)| format!("{designation} : {n}"))
            .collect();
        propositions.push((
            "besoins",
            format!("Besoins relevés à l'inventaire : {}.", liste.join(" ; ")),
        ));
    }
    let taux: Vec<String> = cycles
        .iter()
        .filter(|c| c.decides > 0)
        .filter_map(|c| {
            gesco_core::core::examens::taux(
                u32::try_from(c.admis).unwrap_or(u32::MAX),
                u32::try_from(c.decides).unwrap_or(u32::MAX),
            )
            .map(|t| format!("{t} {}", du_cycle(c.cycle)))
        })
        .collect();
    if !taux.is_empty() {
        propositions.push((
            "resultats",
            format!(
                "Taux d'admission en classe supérieure : {}.",
                joindre(&taux)
            ),
        ));
    }
    propositions
}

/// Type de l'école : la réponse de la fiche ; sinon, double division et double vacation
/// d'après le fonctionnement saisi des classes.
fn type_ecole(
    reponse: &str,
    classes: &[gesco_db::depots::complements::ClasseComplements],
) -> String {
    if !reponse.is_empty() {
        return reponse.to_owned();
    }
    let a = |f: Fonctionnement| {
        classes.iter().any(|c| {
            c.fonctionnement
                .as_deref()
                .and_then(Fonctionnement::depuis_nom)
                == Some(f)
        })
    };
    [
        (Fonctionnement::DoubleDivision, "Double division"),
        (Fonctionnement::DoubleVacation, "Double vacation"),
    ]
    .into_iter()
    .filter(|(f, _)| a(*f))
    .map(|(_, t)| t)
    .collect::<Vec<_>>()
    .join(", ")
}

/// Statut administratif tel que le rapport succinct le nomme.
fn statut_administratif(statut: StatutEtablissement, confession: &str) -> String {
    match statut {
        StatutEtablissement::Public => "Publique".to_owned(),
        StatutEtablissement::Communautaire => "Communautaire".to_owned(),
        StatutEtablissement::Medersa => "Médersa".to_owned(),
        StatutEtablissement::Prive => {
            if confession.is_empty() {
                "Privée".to_owned()
            } else {
                format!("Privée {}", confession.to_lowercase())
            }
        }
    }
}

/// Données du modèle d'impression `rapport_ecole`.
pub fn donnees_impression(contenu: &ContenuRapport) -> serde_json::Value {
    serde_json::json!({
        "titre": contenu.titre,
        "annee": contenu.annee,
        "arret": contenu.arret,
        "entete": contenu.entete,
        "sections": contenu.sections,
        "tableaux": contenu.tableaux,
        "fait_a": contenu.fait_a,
        "qualite": contenu.qualite,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn les_listes_se_joignent_en_francais() {
        assert_eq!(joindre(&[]), "");
        assert_eq!(joindre(&["a".to_owned()]), "a");
        assert_eq!(
            joindre(&["a".to_owned(), "b".to_owned(), "c".to_owned()]),
            "a, b et c"
        );
        assert_eq!(nombre_de(1, "élève", "élèves"), "1 élève");
        assert_eq!(nombre_de(0, "fille", "filles"), "0 filles");
    }

    #[test]
    fn le_type_de_l_ecole_vient_de_la_fiche_sinon_des_classes() {
        let classe = |f: Option<&str>| gesco_db::depots::complements::ClasseComplements {
            id: "c".to_owned(),
            libelle: "6e A".to_owned(),
            cycle: "premier_cycle".to_owned(),
            fonctionnement: f.map(str::to_owned),
            pedagogie: None,
        };
        assert_eq!(type_ecole("Classe unique", &[]), "Classe unique");
        assert_eq!(
            type_ecole(
                "",
                &[
                    classe(Some("double_vacation")),
                    classe(Some("double_division"))
                ]
            ),
            "Double division, Double vacation"
        );
        assert_eq!(type_ecole("", &[classe(None), classe(Some("simple"))]), "");
    }

    #[test]
    fn le_statut_administratif_suit_le_statut_et_la_confession() {
        assert_eq!(
            statut_administratif(StatutEtablissement::Public, ""),
            "Publique"
        );
        assert_eq!(
            statut_administratif(StatutEtablissement::Prive, "Catholique"),
            "Privée catholique"
        );
        assert_eq!(
            statut_administratif(StatutEtablissement::Prive, "Laïque"),
            "Privée laïque"
        );
        assert_eq!(
            statut_administratif(StatutEtablissement::Prive, ""),
            "Privée"
        );
        assert_eq!(
            statut_administratif(StatutEtablissement::Medersa, ""),
            "Médersa"
        );
    }
}
