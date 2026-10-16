//! Inscriptions (SPEC_PHASE_2 §5) : brouillon, validation, abandon, changement de classe.
//! La facture rejoindra la transaction de validation en phase 3 (§0.3).

use gesco_core::commun::{DateCivile, Id};
use gesco_core::core::{
    AnneeScolaire, MachineAEtats, Operation, Permission, StatutAnnee, TypeEvenement,
};
use gesco_core::finance::ContexteTarifaire;
use gesco_core::scolarite::inscription::{Dossier, verifier_changement, verifier_validation};
use gesco_core::scolarite::{
    Classe, DecisionFinAnnee, Eleve, Inscription, Lien, NatureActe, Sexe, StatutInscription,
    TypeInscription,
};
use gesco_db::depots::{academique, annee_scolaire, inscriptions, parametre, personnes, role};
use gesco_db::nouvel_id;
use gesco_db::unite::Durabilite;
use serde::{Deserialize, Serialize};

use crate::erreur::{ErreurService, Resultat};
use crate::facturation::{self, ChoixFrais};
use crate::noyau::Noyau;
use crate::session::Session;

fn regle(cle: &'static str) -> ErreurService {
    ErreurService::Regle { cle }
}

/// Paramètre d'établissement : jours après la rentrée au-delà desquels une inscription est
/// signalée tardive, sans être refusée (Q-31) ; 0 : jamais.
pub const PARAMETRE_ALERTE_TARDIVE: &str = "inscription.alerte_apres_jours";

/// Nouvelle inscription, en brouillon.
#[derive(Debug, Deserialize, specta::Type)]
pub struct DemandeInscription {
    /// Élève.
    pub eleve_id: Id<Eleve>,
    /// Classe de l'année courante.
    pub classe_id: Id<Classe>,
    /// Jour de l'inscription `AAAA-MM-JJ` ; vide : aujourd'hui.
    pub date_inscription: String,
    /// Redouble son niveau.
    pub redoublant: bool,
    /// École d'où vient l'élève, pour une arrivée par transfert.
    pub etablissement_origine: String,
}

/// Brouillon corrigé.
#[derive(Debug, Deserialize, specta::Type)]
pub struct DemandeModificationInscription {
    /// Inscription.
    pub id: Id<Inscription>,
    /// Champs du brouillon.
    pub champs: DemandeInscription,
}

/// Inscription désignée.
#[derive(Debug, Deserialize, specta::Type)]
pub struct DemandeIdInscription {
    /// Inscription.
    pub id: Id<Inscription>,
}

/// Validation d'un brouillon, avec le choix du parent pour sa facture (SPEC_PHASE_3 §4.1).
#[derive(Debug, Deserialize, specta::Type)]
pub struct DemandeValidation {
    /// Inscription.
    pub id: Id<Inscription>,
    /// Plan de paiement et frais facultatifs ; absents : plan par défaut, aucun frais facultatif.
    #[serde(default)]
    pub choix: ChoixFrais,
}

/// Abandon, avec son motif.
#[derive(Debug, Deserialize, specta::Type)]
pub struct DemandeAbandon {
    /// Inscription.
    pub id: Id<Inscription>,
    /// Motif, obligatoire.
    pub motif: String,
}

/// Changement de classe en cours d'année.
#[derive(Debug, Deserialize, specta::Type)]
pub struct DemandeChangementClasse {
    /// Inscription validée.
    pub id: Id<Inscription>,
    /// Nouvelle classe.
    pub classe_id: Id<Classe>,
    /// Motif, obligatoire hors du niveau et de la série.
    pub motif: String,
}

/// Inscriptions d'une année.
#[derive(Debug, Deserialize, specta::Type)]
pub struct DemandeListeInscriptions {
    /// Année scolaire.
    pub annee_id: Id<AnneeScolaire>,
}

/// Inscription et avertissements de son dernier changement.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, specta::Type)]
pub struct ResultatInscription {
    /// Inscription.
    pub inscription: Inscription,
    /// Clés des avertissements, qui n'empêchent rien.
    pub avertissements: Vec<String>,
}

/// Brouillon d'une classe que la validation en lot a refusé, avec la raison.
#[derive(Debug, Clone, Serialize, specta::Type)]
pub struct RefusValidation {
    /// Prénoms et nom.
    pub eleve: String,
    /// Clé de la raison.
    pub cle: String,
}

/// Bilan de la validation en lot des brouillons d'une classe.
#[derive(Debug, Clone, Serialize, specta::Type)]
pub struct BilanValidation {
    /// Inscriptions validées, avec leur facture.
    pub validees: u32,
    /// Brouillons restés brouillons.
    pub refusees: Vec<RefusValidation>,
}

/// Ligne de la liste des inscriptions : le nécessaire pour reconnaître l'élève (R-24).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, specta::Type)]
pub struct LigneInscription {
    /// Inscription.
    pub inscription: Inscription,
    /// Matricule interne.
    pub matricule: String,
    /// Nom complet de l'élève.
    pub eleve: String,
    /// Libellé de la classe.
    pub classe: String,
}

/// Inscription affichée sur la fiche d'un élève.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, specta::Type)]
pub struct InscriptionAffichee {
    /// Inscription.
    pub inscription: Inscription,
    /// Libellé de la classe.
    pub classe: String,
    /// Libellé de l'année.
    pub annee: String,
}

pub(crate) fn jour(instant: gesco_core::commun::Horodatage) -> Resultat<DateCivile> {
    DateCivile::du_jour(instant).ok_or(regle("inscription.date_invalide"))
}

/// Paramètre d'établissement : imprimer les responsables sur la fiche d'inscription (T-08).
pub const PARAMETRE_FICHE_RESPONSABLES: &str = "fiche_inscription.afficher_responsables";

/// Données de la fiche d'inscription imprimée et ses pièces (photo).
pub struct DonneesFiche {
    /// Variables du modèle `fiche_inscription`.
    pub donnees: serde_json::Value,
    /// Photo de l'élève, s'il en a une.
    pub pieces: Vec<gesco_docs::Piece>,
}

/// Service des inscriptions.
pub struct ServiceInscriptions<'n> {
    noyau: &'n Noyau,
}

impl<'n> ServiceInscriptions<'n> {
    /// Service adossé au noyau.
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

    /// Inscriptions d'une année, par classe puis par nom.
    pub async fn lister(
        &self,
        session: &Session,
        annee_id: Id<AnneeScolaire>,
    ) -> Resultat<Vec<LigneInscription>> {
        let contexte = self.noyau.contexte(session.etablissement_id, Some(session));
        let mut cnx = self.connexion().await?;
        let eleves = personnes::eleves(&mut cnx, &contexte).await?;
        let classes = academique::classes(&mut cnx, annee_id, &contexte).await?;
        let mut lignes: Vec<LigneInscription> =
            inscriptions::de_l_annee(&mut cnx, annee_id, &contexte)
                .await?
                .into_iter()
                .filter_map(|inscription| {
                    let (eleve, identite) =
                        eleves.iter().find(|(e, _)| e.id == inscription.eleve_id)?;
                    let classe = classes.iter().find(|c| c.id == inscription.classe_id)?;
                    Some(LigneInscription {
                        matricule: eleve.matricule_interne.clone(),
                        eleve: identite.nom_complet(),
                        classe: classe.libelle.clone(),
                        inscription,
                    })
                })
                .collect();
        lignes.sort_by(|a, b| a.classe.cmp(&b.classe).then_with(|| a.eleve.cmp(&b.eleve)));
        Ok(lignes)
    }

    /// Inscriptions d'un élève, avec classe et année.
    pub async fn de_l_eleve(
        &self,
        session: &Session,
        eleve_id: Id<Eleve>,
    ) -> Resultat<Vec<InscriptionAffichee>> {
        let contexte = self.noyau.contexte(session.etablissement_id, Some(session));
        let mut cnx = self.connexion().await?;
        let mut affichees = Vec::new();
        for inscription in inscriptions::de_l_eleve(&mut cnx, eleve_id, &contexte).await? {
            let classe = academique::classe(&mut cnx, inscription.classe_id, &contexte).await?;
            let annee = annee_scolaire::lire(&mut cnx, inscription.annee_id, &contexte).await?;
            affichees.push(InscriptionAffichee {
                inscription,
                classe: classe.libelle,
                annee: annee.libelle,
            });
        }
        Ok(affichees)
    }

    /// Classe de l'année courante et ouverte, ou de l'année suivante en préparation (T-66 : la
    /// réinscription se valide avant la rentrée), et jour d'inscription compris dans l'année.
    async fn verifier_classe_et_date(
        &self,
        session: &Session,
        classe_id: Id<Classe>,
        date_saisie: &str,
    ) -> Resultat<(Classe, AnneeScolaire, DateCivile)> {
        let contexte = self.noyau.contexte(session.etablissement_id, Some(session));
        let mut cnx = self.connexion().await?;
        let classe = academique::classe(&mut cnx, classe_id, &contexte).await?;
        let annee = annee_scolaire::lire(&mut cnx, classe.annee_id, &contexte).await?;
        let ouverte = annee.statut == StatutAnnee::Active && annee.est_courante;
        if !ouverte && annee.statut != StatutAnnee::Preparation {
            return Err(regle("inscription.annee_non_active"));
        }
        // Une inscription préparée avant la rentrée compte depuis la rentrée.
        let date = if date_saisie.trim().is_empty() {
            jour(contexte.maintenant)?.max(jour(annee.debut)?)
        } else {
            DateCivile::analyser(date_saisie).ok_or(regle("inscription.date_invalide"))?
        };
        if date < jour(annee.debut)? || date > jour(annee.fin)? {
            return Err(regle("inscription.date_hors_annee"));
        }
        Ok((classe, annee, date))
    }

    async fn nature(
        &self,
        session: &Session,
        eleve_id: Id<Eleve>,
        annee_id: Id<AnneeScolaire>,
        origine: &str,
    ) -> Resultat<TypeInscription> {
        let contexte = self.noyau.contexte(session.etablissement_id, Some(session));
        let mut cnx = self.connexion().await?;
        let deja = inscriptions::de_l_eleve(&mut cnx, eleve_id, &contexte)
            .await?
            .iter()
            .any(|i| {
                i.annee_id != annee_id
                    && i.statut != StatutInscription::Brouillon
                    && i.statut != StatutInscription::Abandonnee
            });
        Ok(TypeInscription::deduire(deja, origine))
    }

    /// Données de la fiche d'inscription d'une inscription validée (SPEC_PHASE_2 §9).
    pub async fn donnees_fiche(
        &self,
        session: &Session,
        id: Id<Inscription>,
    ) -> Resultat<DonneesFiche> {
        let contexte = self.noyau.contexte(session.etablissement_id, Some(session));
        let mut cnx = self.connexion().await?;
        let inscription = inscriptions::lire(&mut cnx, id, &contexte).await?;
        if inscription.statut != StatutInscription::Validee {
            return Err(regle("inscription.fiche_non_validee"));
        }
        let (eleve, identite) = personnes::eleve(&mut cnx, inscription.eleve_id, &contexte).await?;
        let classe = academique::classe(&mut cnx, inscription.classe_id, &contexte).await?;
        let niveau = academique::niveau(&mut cnx, classe.niveau_id, &contexte).await?;
        // Dans l'enseignement technique, la fiche porte l'année et la filière (SPEC_PHASE_6 §11).
        let classe_imprimee = if niveau.cycle.a_filieres() {
            format!(
                "{} ({})",
                classe.libelle,
                crate::referentiel::niveau_imprime(&mut cnx, &niveau, &classe, &contexte).await?
            )
        } else {
            classe.libelle.clone()
        };
        let annee = annee_scolaire::lire(&mut cnx, inscription.annee_id, &contexte).await?;
        let afficher =
            parametre::booleen_param(&mut cnx, PARAMETRE_FICHE_RESPONSABLES, true, &contexte)
                .await?;
        let mut responsables = Vec::new();
        if afficher {
            for (_, responsable_id, q) in
                personnes::liens_de_l_eleve(&mut cnx, eleve.id, &contexte).await?
            {
                let (_, p) = personnes::responsable(&mut cnx, responsable_id, &contexte).await?;
                let qualites: Vec<&str> = [
                    (q.est_responsable_legal, "responsable légal"),
                    (q.est_payeur, "payeur"),
                    (q.vient_chercher, "vient chercher l'enfant"),
                ]
                .into_iter()
                .filter_map(|(oui, libelle)| oui.then_some(libelle))
                .collect();
                responsables.push(serde_json::json!({
                    "nom": p.nom_complet(),
                    "lien": match q.lien {
                        Lien::Pere => "Père",
                        Lien::Mere => "Mère",
                        Lien::Tuteur => "Tuteur",
                        Lien::Autre => "Autre",
                    },
                    "telephones": p
                        .telephones
                        .iter()
                        .map(|t| gesco_core::scolarite::telephone::lisible(t))
                        .collect::<Vec<_>>()
                        .join(" · "),
                    "qualites": qualites.join(", "),
                }));
            }
        }
        let photo = personnes::photo(&mut cnx, identite.id, &contexte)
            .await?
            .map(|(octets, format)| gesco_docs::Piece {
                nom: format!("photo.{}", if format == "png" { "png" } else { "jpg" }),
                octets,
            });
        let naissance = identite.date_naissance.map(|d| {
            let texte = crate::documents::date_civile_longue(d);
            if identite.date_naissance_estimee {
                format!("{texte} (date estimée)")
            } else {
                texte
            }
        });
        let acte = eleve.acte_nature.map(|n| {
            let nature = match n {
                NatureActe::Acte => "Acte de naissance",
                NatureActe::JugementSuppletif => "Jugement supplétif",
            };
            [nature, eleve.acte_numero.as_str()]
                .into_iter()
                .filter(|x| !x.is_empty())
                .collect::<Vec<_>>()
                .join(" n° ")
        });
        let donnees = serde_json::json!({
            "annee": annee.libelle,
            "classe": classe_imprimee,
            "date": crate::documents::date_civile_longue(inscription.date_inscription),
            "nature": match inscription.type_inscription {
                TypeInscription::Nouvelle => "Nouvelle inscription",
                TypeInscription::Reinscription => "Réinscription",
                TypeInscription::TransfertEntrant => "Arrivée d'une autre école",
            },
            "eleve": {
                "nom": identite.nom,
                "prenoms": identite.prenoms,
                "matricule": eleve.matricule_interne,
                "matricule_national": eleve.matricule_national,
                "sexe": match identite.sexe {
                    Some(Sexe::Masculin) => "Masculin",
                    Some(Sexe::Feminin) => "Féminin",
                    None => "",
                },
                "date_naissance": naissance,
                "lieu_naissance": identite.lieu_naissance,
                "nationalite": identite.nationalite,
                "acte": acte,
                "ecole_origine": eleve.ecole_origine,
            },
            "responsables": afficher.then_some(responsables),
            "photo": photo.as_ref().map(gesco_docs::Piece::chemin),
        });
        Ok(DonneesFiche {
            donnees,
            pieces: photo.into_iter().collect(),
        })
    }

    /// Crée une inscription en brouillon dans l'année courante.
    pub async fn creer(
        &self,
        session: &Session,
        demande: &DemandeInscription,
    ) -> Resultat<ResultatInscription> {
        let contexte = self.noyau.contexte(session.etablissement_id, Some(session));
        let (classe, annee, date) = self
            .verifier_classe_et_date(session, demande.classe_id, &demande.date_inscription)
            .await?;
        {
            let mut cnx = self.connexion().await?;
            let (eleve, _) = personnes::eleve(&mut cnx, demande.eleve_id, &contexte).await?;
            if eleve.fusionne_dans.is_some() {
                return Err(regle("eleve.absorbe"));
            }
            if inscriptions::de_l_eleve(&mut cnx, demande.eleve_id, &contexte)
                .await?
                .iter()
                .any(|i| i.annee_id == annee.id && i.statut.est_vivante())
            {
                return Err(regle("inscription.deja_inscrit"));
            }
        }
        let origine = demande.etablissement_origine.trim().to_owned();
        let inscription = Inscription {
            id: nouvel_id(),
            etablissement_id: session.etablissement_id,
            annee_id: annee.id,
            eleve_id: demande.eleve_id,
            classe_id: classe.id,
            date_inscription: date,
            type_inscription: self
                .nature(session, demande.eleve_id, annee.id, &origine)
                .await?,
            statut: StatutInscription::Brouillon,
            redoublant: demande.redoublant,
            decision_fin_annee: DecisionFinAnnee::NonStatue,
            classe_precedente_id: None,
            etablissement_origine: origine,
            motif_abandon: String::new(),
            date_depart: None,
            etablissement_destination: String::new(),
            motif_depart: String::new(),
        };
        let mut unite = self
            .noyau
            .commencer(contexte.clone(), Durabilite::Critique)
            .await?;
        inscriptions::inserer(unite.connexion(), &inscription, &contexte).await?;
        let id = inscription.id.vers_chaine();
        unite.auditer(
            "inscription",
            &id,
            Operation::Creation,
            None,
            Some(serde_json::to_value(&inscription)?),
            "inscriptions.creer",
        );
        unite.emettre(
            TypeEvenement::InscriptionPreparee,
            "inscription",
            &id,
            serde_json::json!({
                "eleve_id": inscription.eleve_id,
                "classe_id": inscription.classe_id,
                "annee_id": inscription.annee_id,
            }),
        );
        self.noyau.terminer(unite).await?;
        Ok(ResultatInscription {
            inscription,
            avertissements: Vec::new(),
        })
    }

    async fn lire(&self, session: &Session, id: Id<Inscription>) -> Resultat<Inscription> {
        let contexte = self.noyau.contexte(session.etablissement_id, Some(session));
        let mut cnx = self.connexion().await?;
        Ok(inscriptions::lire(&mut cnx, id, &contexte).await?)
    }

    /// Corrige un brouillon.
    pub async fn modifier(
        &self,
        session: &Session,
        id: Id<Inscription>,
        demande: &DemandeInscription,
    ) -> Resultat<ResultatInscription> {
        let avant = self.lire(session, id).await?;
        if avant.statut != StatutInscription::Brouillon {
            return Err(regle("inscription.non_modifiable"));
        }
        let (classe, annee, date) = self
            .verifier_classe_et_date(session, demande.classe_id, &demande.date_inscription)
            .await?;
        if annee.id != avant.annee_id {
            return Err(regle("inscription.autre_annee"));
        }
        let origine = demande.etablissement_origine.trim().to_owned();
        let apres = Inscription {
            classe_id: classe.id,
            date_inscription: date,
            redoublant: demande.redoublant,
            type_inscription: self
                .nature(session, avant.eleve_id, annee.id, &origine)
                .await?,
            etablissement_origine: origine,
            ..avant.clone()
        };
        let contexte = self.noyau.contexte(session.etablissement_id, Some(session));
        let mut unite = self
            .noyau
            .commencer(contexte.clone(), Durabilite::Critique)
            .await?;
        inscriptions::modifier_brouillon(unite.connexion(), &apres, &contexte).await?;
        unite.auditer(
            "inscription",
            &id.vers_chaine(),
            Operation::Modification,
            Some(serde_json::to_value(&avant)?),
            Some(serde_json::to_value(&apres)?),
            "inscriptions.modifier",
        );
        self.noyau.terminer(unite).await?;
        Ok(ResultatInscription {
            inscription: apres,
            avertissements: Vec::new(),
        })
    }

    /// Valide un brouillon si les conditions du §5.3 sont réunies.
    /// Lit une inscription de l'établissement.
    pub async fn lire_publique(
        &self,
        session: &Session,
        id: Id<Inscription>,
    ) -> Resultat<Inscription> {
        self.lire(session, id).await
    }

    /// Ce que la scolarité dit d'une inscription à la finance (§9.1 du blueprint) : année,
    /// niveau et série de sa classe, date d'inscription.
    pub async fn contexte_tarifaire(
        &self,
        session: &Session,
        id: Id<Inscription>,
    ) -> Resultat<ContexteTarifaire> {
        let inscription = self.lire(session, id).await?;
        let contexte = self.noyau.contexte(session.etablissement_id, Some(session));
        let mut cnx = self.connexion().await?;
        let classe = academique::classe(&mut cnx, inscription.classe_id, &contexte).await?;
        Ok(ContexteTarifaire {
            inscription_id: inscription.id,
            annee_id: inscription.annee_id,
            niveau_id: classe.niveau_id,
            serie_id: classe.serie_id,
            date_inscription: inscription.date_inscription,
        })
    }

    /// Valide d'un coup les brouillons d'une classe, chacun dans sa propre transaction et avec le
    /// plan de paiement par défaut : après un import, des centaines d'élèves (SPEC_PHASE_4 §13,
    /// Q-62). Un brouillon refusé reste brouillon, avec sa raison.
    pub async fn valider_classe(
        &self,
        session: &Session,
        classe_id: Id<Classe>,
    ) -> Resultat<BilanValidation> {
        let contexte = self.noyau.contexte(session.etablissement_id, Some(session));
        let classe = {
            let mut cnx = self.connexion().await?;
            academique::classe(&mut cnx, classe_id, &contexte).await?
        };
        let brouillons: Vec<LigneInscription> = self
            .lister(session, classe.annee_id)
            .await?
            .into_iter()
            .filter(|l| {
                l.inscription.classe_id == classe.id
                    && l.inscription.statut == StatutInscription::Brouillon
            })
            .collect();
        let mut bilan = BilanValidation {
            validees: 0,
            refusees: Vec::new(),
        };
        for l in brouillons {
            match self
                .valider(
                    session,
                    l.inscription.id,
                    &crate::facturation::ChoixFrais::default(),
                )
                .await
            {
                Ok(_) => bilan.validees += 1,
                Err(ErreurService::Regle { cle }) => bilan.refusees.push(RefusValidation {
                    eleve: l.eleve,
                    cle: cle.to_owned(),
                }),
                Err(autre) => return Err(autre),
            }
        }
        Ok(bilan)
    }

    /// Valide un brouillon : conditions du §5.3 de la phase 2, puis facture et échéancier dans
    /// la même transaction (R-17, SPEC_PHASE_3 §4.1).
    pub async fn valider(
        &self,
        session: &Session,
        id: Id<Inscription>,
        choix: &ChoixFrais,
    ) -> Resultat<ResultatInscription> {
        let avant = self.lire(session, id).await?;
        avant.statut.transiter(StatutInscription::Validee)?;
        let contexte = self.noyau.contexte(session.etablissement_id, Some(session));
        let avertissements = {
            let mut cnx = self.connexion().await?;
            let (eleve, identite) = personnes::eleve(&mut cnx, avant.eleve_id, &contexte).await?;
            if eleve.fusionne_dans.is_some() {
                return Err(regle("eleve.absorbe"));
            }
            let a_photo = personnes::photo(&mut cnx, identite.id, &contexte)
                .await?
                .is_some();
            let mut responsables = Vec::new();
            for (_, responsable_id, qualites) in
                personnes::liens_de_l_eleve(&mut cnx, eleve.id, &contexte).await?
            {
                let (_, personne) =
                    personnes::responsable(&mut cnx, responsable_id, &contexte).await?;
                responsables.push((qualites, !personne.telephones.is_empty()));
            }
            let classe = academique::classe(&mut cnx, avant.classe_id, &contexte).await?;
            let niveau = academique::niveau(&mut cnx, classe.niveau_id, &contexte).await?;
            let serie_active = match classe.serie_id {
                Some(s) => academique::serie(&mut cnx, s, &contexte).await?.actif,
                None => true,
            };
            let annee = annee_scolaire::lire(&mut cnx, avant.annee_id, &contexte).await?;
            let alerte =
                parametre::entier(&mut cnx, PARAMETRE_ALERTE_TARDIVE, 0, &contexte).await?;
            verifier_validation(&Dossier {
                identite: &identite,
                eleve: &eleve,
                a_photo,
                niveau: &niveau,
                serie_active,
                responsables: &responsables,
                jours_depuis_rentree: avant.date_inscription.jours_depuis_epoque()
                    - jour(annee.debut)?.jours_depuis_epoque(),
                alerte_apres_jours: alerte,
            })
            .map_err(regle)?
        };
        let apres = Inscription {
            statut: StatutInscription::Validee,
            ..avant.clone()
        };
        let tarif = self.contexte_tarifaire(session, id).await?;
        let mut unite = self
            .noyau
            .commencer(contexte.clone(), Durabilite::Critique)
            .await?;
        inscriptions::ecrire_statut(
            unite.connexion(),
            id,
            StatutInscription::Validee,
            "",
            &contexte,
        )
        .await?;
        facturation::facturer_dans(&mut unite, &contexte, &tarif, choix).await?;
        let id_texte = id.vers_chaine();
        unite.auditer(
            "inscription",
            &id_texte,
            Operation::Transition,
            Some(serde_json::to_value(&avant)?),
            Some(serde_json::to_value(&apres)?),
            "inscriptions.valider",
        );
        unite.emettre(
            TypeEvenement::InscriptionValidee,
            "inscription",
            &id_texte,
            serde_json::json!({
                "eleve_id": apres.eleve_id,
                "classe_id": apres.classe_id,
                "annee_id": apres.annee_id,
                "type": apres.type_inscription.nom(),
            }),
        );
        self.noyau.terminer(unite).await?;
        Ok(ResultatInscription {
            inscription: apres,
            avertissements: avertissements.into_iter().map(str::to_owned).collect(),
        })
    }

    /// Abandonne un brouillon ou une inscription validée, avec son motif.
    pub async fn abandonner(
        &self,
        session: &Session,
        id: Id<Inscription>,
        motif: &str,
    ) -> Resultat<ResultatInscription> {
        let motif = motif.trim();
        if motif.is_empty() {
            return Err(regle("inscription.motif_obligatoire"));
        }
        let avant = self.lire(session, id).await?;
        avant.statut.transiter(StatutInscription::Abandonnee)?;
        let apres = Inscription {
            statut: StatutInscription::Abandonnee,
            motif_abandon: motif.to_owned(),
            ..avant.clone()
        };
        let contexte = self.noyau.contexte(session.etablissement_id, Some(session));
        let mut unite = self
            .noyau
            .commencer(contexte.clone(), Durabilite::Critique)
            .await?;
        inscriptions::ecrire_statut(
            unite.connexion(),
            id,
            StatutInscription::Abandonnee,
            motif,
            &contexte,
        )
        .await?;
        if avant.statut == StatutInscription::Validee {
            facturation::annuler_pour_abandon_dans(&mut unite, &contexte, id, motif).await?;
        }
        let id_texte = id.vers_chaine();
        unite.auditer(
            "inscription",
            &id_texte,
            Operation::Transition,
            Some(serde_json::to_value(&avant)?),
            Some(serde_json::to_value(&apres)?),
            "inscriptions.abandonner",
        );
        unite.emettre(
            TypeEvenement::InscriptionAbandonnee,
            "inscription",
            &id_texte,
            serde_json::json!({ "motif": motif, "depuis": avant.statut.nom() }),
        );
        self.noyau.terminer(unite).await?;
        Ok(ResultatInscription {
            inscription: apres,
            avertissements: Vec::new(),
        })
    }

    /// Change de classe une inscription validée (Q-35) : même niveau et même série, sauf décision
    /// de la direction (`enrollments:validate`), motivée.
    pub async fn changer_classe(
        &self,
        session: &Session,
        id: Id<Inscription>,
        classe_id: Id<Classe>,
        motif: &str,
    ) -> Resultat<ResultatInscription> {
        let avant = self.lire(session, id).await?;
        if avant.statut != StatutInscription::Validee {
            return Err(regle("inscription.non_validee"));
        }
        let contexte = self.noyau.contexte(session.etablissement_id, Some(session));
        {
            let mut cnx = self.connexion().await?;
            let ancienne = academique::classe(&mut cnx, avant.classe_id, &contexte).await?;
            let nouvelle = academique::classe(&mut cnx, classe_id, &contexte).await?;
            let direction = role::permissions_de(&mut cnx, session.utilisateur_id, &contexte)
                .await?
                .contains(&Permission::ENROLLMENTS_VALIDATE);
            verifier_changement(&ancienne, &nouvelle, direction, motif).map_err(regle)?;
        }
        let apres = Inscription {
            classe_id,
            classe_precedente_id: Some(avant.classe_id),
            ..avant.clone()
        };
        let mut unite = self
            .noyau
            .commencer(contexte.clone(), Durabilite::Critique)
            .await?;
        inscriptions::changer_classe(unite.connexion(), id, classe_id, avant.classe_id, &contexte)
            .await?;
        let id_texte = id.vers_chaine();
        let trace = serde_json::json!({ "inscription": apres, "motif": motif.trim() });
        unite.auditer(
            "inscription",
            &id_texte,
            Operation::Modification,
            Some(serde_json::to_value(&avant)?),
            Some(trace),
            "inscriptions.changer_classe",
        );
        unite.emettre(
            TypeEvenement::ClasseChangee,
            "inscription",
            &id_texte,
            serde_json::json!({
                "ancienne": avant.classe_id,
                "nouvelle": classe_id,
                "motif": motif.trim(),
            }),
        );
        self.noyau.terminer(unite).await?;
        Ok(ResultatInscription {
            inscription: apres,
            avertissements: Vec::new(),
        })
    }
}
