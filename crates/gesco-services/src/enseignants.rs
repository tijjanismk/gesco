//! Enseignants et affectations (SPEC_PHASE_4 §3) : la direction enregistre les enseignants et
//! les affecte aux matières des classes ; la portée de saisie des notes en découle (§11.3 du
//! blueprint).

use gesco_core::commun::Id;
use gesco_core::core::{
    AnneeScolaire, Operation, Permission, StatutAnnee, TypeEvenement, Utilisateur,
};
use gesco_core::pedagogie::enseignant::matieres_a_confier;
use gesco_core::pedagogie::{
    AffectationEnseignement, Enseignant, Matiere, PorteeNotes, StatutEnseignant,
};
use gesco_core::scolarite::telephone::normaliser_liste;
use gesco_core::scolarite::{Classe, Personne};
use gesco_db::depots::{
    academique, annee_scolaire, enseignants, pedagogie, personnes, role, utilisateur,
};
use gesco_db::nouvel_id;
use gesco_db::unite::Durabilite;
use serde::{Deserialize, Serialize};

use crate::erreur::{ErreurService, Resultat};
use crate::noyau::Noyau;
use crate::session::Session;

fn regle(cle: &'static str) -> ErreurService {
    ErreurService::Regle { cle }
}

/// Enseignant affiché.
#[derive(Debug, Clone, Serialize, specta::Type)]
pub struct FicheEnseignant {
    /// Enseignant.
    pub enseignant: Enseignant,
    /// Nom.
    pub nom: String,
    /// Prénoms.
    pub prenoms: String,
    /// Téléphones, forme canonique.
    pub telephones: Vec<String>,
    /// Nom affiché du compte lié.
    pub compte: Option<String>,
}

/// Affectation affichée.
#[derive(Debug, Clone, Serialize, specta::Type)]
pub struct AffectationAffichee {
    /// Affectation.
    pub affectation: AffectationEnseignement,
    /// Libellé de la classe.
    pub classe: String,
    /// Libellé de la matière.
    pub matiere: String,
    /// Prénoms et nom de l'enseignant.
    pub enseignant: String,
}

/// Enseignant saisi par la direction ; sans `id`, une création.
#[derive(Debug, Clone, Deserialize, specta::Type)]
pub struct DemandeEnseignant {
    /// Enseignant modifié ; absent pour une création.
    pub id: Option<Id<Enseignant>>,
    /// Nom.
    pub nom: String,
    /// Prénoms.
    pub prenoms: String,
    /// Téléphones, tels que saisis.
    pub telephones: Vec<String>,
    /// Statut.
    pub statut: StatutEnseignant,
    /// Compte lié, pour qu'il saisisse lui-même.
    pub utilisateur_id: Option<Id<Utilisateur>>,
    /// Faux pour un enseignant parti.
    pub actif: bool,
}

/// Affectation demandée.
#[derive(Debug, Clone, Deserialize, specta::Type)]
pub struct DemandeAffectation {
    /// Classe.
    pub classe_id: Id<Classe>,
    /// Matière.
    pub matiere_id: Id<Matiere>,
    /// Enseignant.
    pub enseignant_id: Id<Enseignant>,
}

/// Affectation désignée.
#[derive(Debug, Clone, Deserialize, specta::Type)]
pub struct DemandeIdAffectation {
    /// Affectation.
    pub id: Id<AffectationEnseignement>,
}

/// Maître affecté à toute sa classe (T-25).
#[derive(Debug, Clone, Deserialize, specta::Type)]
pub struct DemandeMaitre {
    /// Classe.
    pub classe_id: Id<Classe>,
    /// Enseignant.
    pub enseignant_id: Id<Enseignant>,
}

/// Service des enseignants.
pub struct ServiceEnseignants<'n> {
    noyau: &'n Noyau,
}

impl<'n> ServiceEnseignants<'n> {
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

    /// Enseignants de l'établissement, avec leur identité et leur compte.
    pub async fn lister(&self, session: &Session) -> Resultat<Vec<FicheEnseignant>> {
        let contexte = self.noyau.contexte(session.etablissement_id, Some(session));
        let mut cnx = self.connexion().await?;
        let mut fiches = Vec::new();
        for e in enseignants::enseignants(&mut cnx, &contexte).await? {
            let p = personnes::personne(&mut cnx, e.personne_id, &contexte).await?;
            let compte = match e.utilisateur_id {
                Some(u) => utilisateur::lire(&mut cnx, u, &contexte)
                    .await
                    .ok()
                    .map(|u| u.nom_affiche),
                None => None,
            };
            fiches.push(FicheEnseignant {
                enseignant: e,
                nom: p.nom,
                prenoms: p.prenoms,
                telephones: p.telephones,
                compte,
            });
        }
        fiches.sort_by(|a, b| a.nom.cmp(&b.nom).then_with(|| a.prenoms.cmp(&b.prenoms)));
        Ok(fiches)
    }

    /// Crée ou modifie un enseignant.
    pub async fn enregistrer(
        &self,
        session: &Session,
        demande: &DemandeEnseignant,
    ) -> Resultat<FicheEnseignant> {
        let nom = demande.nom.trim().to_owned();
        let prenoms = demande.prenoms.trim().to_owned();
        if nom.is_empty() || prenoms.is_empty() {
            return Err(regle("enseignant.nom_obligatoire"));
        }
        let telephones = normaliser_liste(&demande.telephones).map_err(regle)?;
        let contexte = self.noyau.contexte(session.etablissement_id, Some(session));
        let (existants, compte) = {
            let mut cnx = self.connexion().await?;
            let existants = enseignants::enseignants(&mut cnx, &contexte).await?;
            let compte = match demande.utilisateur_id {
                Some(u) => Some(utilisateur::lire(&mut cnx, u, &contexte).await?.nom_affiche),
                None => None,
            };
            (existants, compte)
        };
        if let Some(u) = demande.utilisateur_id
            && existants
                .iter()
                .any(|e| e.utilisateur_id == Some(u) && Some(e.id) != demande.id)
        {
            return Err(regle("enseignant.compte_deja_lie"));
        }
        let avant = match demande.id {
            Some(id) => Some(
                existants
                    .iter()
                    .find(|e| e.id == id)
                    .cloned()
                    .ok_or(regle("enseignant.inconnu"))?,
            ),
            None => None,
        };
        let mut unite = self
            .noyau
            .commencer(contexte.clone(), Durabilite::Critique)
            .await?;
        let personne = match &avant {
            Some(a) => {
                let ancienne =
                    personnes::personne(unite.connexion(), a.personne_id, &contexte).await?;
                let p = Personne {
                    nom: nom.clone(),
                    prenoms: prenoms.clone(),
                    telephones: telephones.clone(),
                    ..ancienne
                };
                personnes::modifier_personne(unite.connexion(), &p, &contexte).await?;
                p
            }
            None => {
                let p = Personne {
                    id: nouvel_id(),
                    etablissement_id: session.etablissement_id,
                    nom: nom.clone(),
                    prenoms: prenoms.clone(),
                    sexe: None,
                    date_naissance: None,
                    date_naissance_estimee: false,
                    lieu_naissance: String::new(),
                    nationalite: String::new(),
                    adresse: String::new(),
                    telephones: telephones.clone(),
                };
                personnes::inserer_personne(unite.connexion(), &p, &contexte).await?;
                p
            }
        };
        let apres = Enseignant {
            id: avant.as_ref().map_or_else(nouvel_id, |a| a.id),
            etablissement_id: session.etablissement_id,
            personne_id: personne.id,
            statut: demande.statut,
            utilisateur_id: demande.utilisateur_id,
            actif: demande.actif,
        };
        match &avant {
            Some(_) => enseignants::ecrire_enseignant(unite.connexion(), &apres, &contexte).await?,
            None => {
                enseignants::inserer_enseignant(unite.connexion(), &apres, &contexte).await?;
                // Il entre aussi au registre du personnel (SPEC_PHASE_6 §2, Q-63).
                crate::personnel::rattacher_enseignant(
                    &mut unite,
                    &apres,
                    false,
                    "enseignants.enregistrer",
                )
                .await?;
            }
        }
        let id = apres.id.vers_chaine();
        unite.auditer(
            "enseignant",
            &id,
            if avant.is_some() {
                Operation::Modification
            } else {
                Operation::Creation
            },
            avant.as_ref().map(serde_json::to_value).transpose()?,
            Some(serde_json::json!({ "enseignant": apres, "nom": nom, "prenoms": prenoms })),
            "enseignants.enregistrer",
        );
        unite.emettre(
            TypeEvenement::EnseignantEnregistre,
            "enseignant",
            &id,
            serde_json::json!({ "statut": apres.statut, "compte": apres.utilisateur_id.is_some() }),
        );
        self.noyau.terminer(unite).await?;
        Ok(FicheEnseignant {
            enseignant: apres,
            nom,
            prenoms,
            telephones,
            compte,
        })
    }

    /// Affectations d'une année, avec les libellés.
    pub async fn affectations(
        &self,
        session: &Session,
        annee_id: Id<AnneeScolaire>,
    ) -> Resultat<Vec<AffectationAffichee>> {
        let contexte = self.noyau.contexte(session.etablissement_id, Some(session));
        let fiches = self.lister(session).await?;
        let mut cnx = self.connexion().await?;
        let classes = academique::classes(&mut cnx, annee_id, &contexte).await?;
        let matieres = pedagogie::matieres(&mut cnx, &contexte).await?;
        Ok(enseignants::affectations(&mut cnx, annee_id, &contexte)
            .await?
            .into_iter()
            .map(|a| AffectationAffichee {
                classe: classes
                    .iter()
                    .find(|c| c.id == a.classe_id)
                    .map(|c| c.libelle.clone())
                    .unwrap_or_default(),
                matiere: matieres
                    .iter()
                    .find(|m| m.id == a.matiere_id)
                    .map(|m| m.libelle.clone())
                    .unwrap_or_default(),
                enseignant: fiches
                    .iter()
                    .find(|f| f.enseignant.id == a.enseignant_id)
                    .map(|f| format!("{} {}", f.prenoms, f.nom))
                    .unwrap_or_default(),
                affectation: a,
            })
            .collect())
    }

    /// Classe, matières de son programme, affectations existantes et enseignants, pour une
    /// affectation. Refuse une année close.
    async fn preparer(
        &self,
        session: &Session,
        classe_id: Id<Classe>,
    ) -> Resultat<(
        Classe,
        Vec<Id<Matiere>>,
        Vec<AffectationEnseignement>,
        Vec<Enseignant>,
    )> {
        let contexte = self.noyau.contexte(session.etablissement_id, Some(session));
        let mut cnx = self.connexion().await?;
        let classe = academique::classe(&mut cnx, classe_id, &contexte).await?;
        let annee = annee_scolaire::lire(&mut cnx, classe.annee_id, &contexte).await?;
        if matches!(annee.statut, StatutAnnee::Cloturee | StatutAnnee::Archivee) {
            return Err(regle("pedagogie.annee_close"));
        }
        let programme: Vec<Id<Matiere>> =
            pedagogie::programme(&mut cnx, classe.annee_id, &contexte)
                .await?
                .into_iter()
                .filter(|l| l.niveau_id == classe.niveau_id && l.serie_id == classe.serie_id)
                .map(|l| l.matiere_id)
                .collect();
        let affectations: Vec<AffectationEnseignement> =
            enseignants::affectations(&mut cnx, classe.annee_id, &contexte)
                .await?
                .into_iter()
                .filter(|a| a.classe_id == classe.id)
                .collect();
        let liste = enseignants::enseignants(&mut cnx, &contexte).await?;
        Ok((classe, programme, affectations, liste))
    }

    /// Affecte un enseignant à une matière d'une classe. `remplacer` confie la matière à lui seul
    /// (les autres affectations de la matière sont retirées) ; sinon il la partage (T-22).
    pub async fn affecter(
        &self,
        session: &Session,
        demande: &DemandeAffectation,
        remplacer: bool,
    ) -> Resultat<AffectationEnseignement> {
        let (classe, programme, existantes, liste) =
            self.preparer(session, demande.classe_id).await?;
        if !programme.contains(&demande.matiere_id) {
            return Err(regle("affectation.matiere_hors_programme"));
        }
        if !liste
            .iter()
            .any(|e| e.id == demande.enseignant_id && e.actif)
        {
            return Err(regle("affectation.enseignant_inactif"));
        }
        let deja = existantes.iter().any(|a| {
            a.matiere_id == demande.matiere_id && a.enseignant_id == demande.enseignant_id
        });
        if deja && !remplacer {
            return Err(regle("affectation.deja"));
        }
        let a = AffectationEnseignement {
            id: nouvel_id(),
            etablissement_id: session.etablissement_id,
            annee_id: classe.annee_id,
            classe_id: classe.id,
            matiere_id: demande.matiere_id,
            enseignant_id: demande.enseignant_id,
        };
        let avant: Vec<&AffectationEnseignement> = existantes
            .iter()
            .filter(|x| x.matiere_id == demande.matiere_id)
            .collect();
        let contexte = self.noyau.contexte(session.etablissement_id, Some(session));
        let mut unite = self
            .noyau
            .commencer(contexte.clone(), Durabilite::Critique)
            .await?;
        if remplacer {
            enseignants::retirer_affectations_de(
                unite.connexion(),
                classe.id,
                demande.matiere_id,
                &contexte,
            )
            .await?;
        }
        enseignants::inserer_affectation(unite.connexion(), &a, &contexte).await?;
        let id = classe.id.vers_chaine();
        unite.auditer(
            "affectation_enseignement",
            &id,
            Operation::Modification,
            Some(serde_json::to_value(&avant)?),
            Some(serde_json::to_value(&a)?),
            if remplacer {
                "enseignants.confier"
            } else {
                "enseignants.ajouter"
            },
        );
        unite.emettre(
            TypeEvenement::AffectationModifiee,
            "classe",
            &id,
            serde_json::json!({ "matiere_id": a.matiere_id, "enseignant_id": a.enseignant_id, "seul": remplacer }),
        );
        self.noyau.terminer(unite).await?;
        Ok(a)
    }

    /// Retire une affectation.
    pub async fn retirer(
        &self,
        session: &Session,
        id: Id<AffectationEnseignement>,
    ) -> Resultat<()> {
        let contexte = self.noyau.contexte(session.etablissement_id, Some(session));
        let annees = {
            let mut cnx = self.connexion().await?;
            annee_scolaire::lister(&mut cnx, &contexte).await?
        };
        let mut trouvee = None;
        {
            let mut cnx = self.connexion().await?;
            for annee in &annees {
                if let Some(a) = enseignants::affectations(&mut cnx, annee.id, &contexte)
                    .await?
                    .into_iter()
                    .find(|a| a.id == id)
                {
                    if matches!(annee.statut, StatutAnnee::Cloturee | StatutAnnee::Archivee) {
                        return Err(regle("pedagogie.annee_close"));
                    }
                    trouvee = Some(a);
                    break;
                }
            }
        }
        let a = trouvee.ok_or(regle("affectation.inconnue"))?;
        let mut unite = self
            .noyau
            .commencer(contexte.clone(), Durabilite::Critique)
            .await?;
        enseignants::retirer_affectation(unite.connexion(), a.id, &contexte).await?;
        let cle = a.classe_id.vers_chaine();
        unite.auditer(
            "affectation_enseignement",
            &cle,
            Operation::Suppression,
            Some(serde_json::to_value(&a)?),
            None,
            "enseignants.retirer",
        );
        unite.emettre(
            TypeEvenement::AffectationModifiee,
            "classe",
            &cle,
            serde_json::json!({ "matiere_id": a.matiere_id, "enseignant_id": a.enseignant_id, "retiree": true }),
        );
        self.noyau.terminer(unite).await?;
        Ok(())
    }

    /// Affecte un maître à toute sa classe : il reçoit chaque matière du programme que personne
    /// n'enseigne encore (T-25). Rend le nombre de matières reçues.
    pub async fn affecter_maitre(
        &self,
        session: &Session,
        demande: &DemandeMaitre,
    ) -> Resultat<usize> {
        let (classe, programme, existantes, liste) =
            self.preparer(session, demande.classe_id).await?;
        if !liste
            .iter()
            .any(|e| e.id == demande.enseignant_id && e.actif)
        {
            return Err(regle("affectation.enseignant_inactif"));
        }
        let a_confier = matieres_a_confier(&programme, &existantes);
        if a_confier.is_empty() {
            return Err(regle("affectation.rien_a_confier"));
        }
        let contexte = self.noyau.contexte(session.etablissement_id, Some(session));
        let mut unite = self
            .noyau
            .commencer(contexte.clone(), Durabilite::Critique)
            .await?;
        let mut creees = Vec::new();
        for matiere_id in &a_confier {
            let a = AffectationEnseignement {
                id: nouvel_id(),
                etablissement_id: session.etablissement_id,
                annee_id: classe.annee_id,
                classe_id: classe.id,
                matiere_id: *matiere_id,
                enseignant_id: demande.enseignant_id,
            };
            enseignants::inserer_affectation(unite.connexion(), &a, &contexte).await?;
            creees.push(a);
        }
        let id = classe.id.vers_chaine();
        unite.auditer(
            "affectation_enseignement",
            &id,
            Operation::Creation,
            None,
            Some(serde_json::to_value(&creees)?),
            "enseignants.maitre",
        );
        unite.emettre(
            TypeEvenement::AffectationModifiee,
            "classe",
            &id,
            serde_json::json!({ "enseignant_id": demande.enseignant_id, "matieres": creees.len() }),
        );
        self.noyau.terminer(unite).await?;
        Ok(creees.len())
    }

    /// Portée de saisie des notes de l'utilisateur (§11.3) : la direction, qui valide les notes,
    /// partout ; un compte lié à un enseignant, dans ses affectations ; sinon nulle part.
    pub async fn portee_notes(&self, session: &Session) -> Resultat<PorteeNotes> {
        let contexte = self.noyau.contexte(session.etablissement_id, Some(session));
        let mut cnx = self.connexion().await?;
        let valide = role::permissions_de(&mut cnx, session.utilisateur_id, &contexte)
            .await?
            .contains(&Permission::GRADES_VALIDATE);
        let affectations =
            match enseignants::enseignant_du_compte(&mut cnx, session.utilisateur_id, &contexte)
                .await?
            {
                Some(e) => enseignants::affectations_de(&mut cnx, e.id, &contexte).await?,
                None => Vec::new(),
            };
        Ok(PorteeNotes::de(valide, &affectations))
    }
}
