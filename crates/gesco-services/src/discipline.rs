//! Discipline (SPEC_VERSION_1_1 §1, étape 3, T-70) : le dossier de discipline d'un élève et le
//! registre de l'année. Avertissement, blâme, retenue et convocation des parents se notent avec
//! leur date, leur motif et qui les a décidés ; l'avertissement et le blâme du conseil de classe,
//! rattachés à une période, paraissent sur le bulletin. Une sanction saisie par erreur s'annule
//! avec un motif.

use gesco_core::commun::{DateCivile, Id};
use gesco_core::core::{AnneeScolaire, Operation, Permission, TypeEvenement};
use gesco_core::pedagogie::Periode;
use gesco_core::scolarite::discipline::{NatureSanction, Sanction, verifier_sanction};
use gesco_core::scolarite::{Inscription, StatutInscription};
use gesco_db::depots::{
    academique, annee_scolaire, discipline, inscriptions, pedagogie, personnes, role, utilisateur,
};
use gesco_db::unite::Durabilite;
use gesco_db::{Contexte, nouvel_id};
use serde::{Deserialize, Serialize};

use crate::erreur::{ErreurService, Resultat};
use crate::noyau::Noyau;
use crate::session::Session;

fn regle(cle: &'static str) -> ErreurService {
    ErreurService::Regle { cle }
}

/// Sanction affichée.
#[derive(Debug, Clone, Serialize, specta::Type)]
pub struct SanctionAffichee {
    /// Sanction.
    pub sanction: Sanction,
    /// Prénoms et nom de l'élève.
    pub eleve: String,
    /// Classe.
    pub classe: String,
    /// Période du conseil de classe, pour une sanction portée sur le bulletin.
    pub periode: Option<String>,
    /// Qui l'a saisie.
    pub saisie_par: String,
}

/// Période qu'un conseil de classe peut viser.
#[derive(Debug, Clone, Serialize, specta::Type)]
pub struct PeriodeConseil {
    /// Période.
    pub periode_id: Id<Periode>,
    /// Libellé : « 1er trimestre ».
    pub libelle: String,
}

/// Ce que peut faire le compte connecté.
#[derive(Debug, Clone, Copy, Serialize, specta::Type)]
pub struct DroitsDiscipline {
    /// Noter une sanction.
    pub sanctionner: bool,
    /// Annuler une sanction.
    pub annuler: bool,
}

/// Le dossier de discipline d'un élève pour son inscription.
#[derive(Debug, Clone, Serialize, specta::Type)]
pub struct DossierDiscipline {
    /// Inscription.
    pub inscription_id: Id<Inscription>,
    /// Prénoms et nom.
    pub eleve: String,
    /// Classe.
    pub classe: String,
    /// Sanctions, annulées comprises, par date.
    pub sanctions: Vec<SanctionAffichee>,
    /// Périodes de son niveau, pour le conseil de classe.
    pub periodes: Vec<PeriodeConseil>,
    /// Droits de qui consulte.
    pub droits: DroitsDiscipline,
}

/// Le registre de discipline de l'année courante.
#[derive(Debug, Clone, Serialize, specta::Type)]
pub struct RegistreDiscipline {
    /// Année.
    pub annee_id: Id<AnneeScolaire>,
    /// Sanctions de l'année, la plus récente d'abord.
    pub sanctions: Vec<SanctionAffichee>,
    /// Droits de qui consulte.
    pub droits: DroitsDiscipline,
}

/// Dossier d'un élève à lire.
#[derive(Debug, Deserialize, specta::Type)]
pub struct DemandeDossierDiscipline {
    /// Inscription.
    pub inscription_id: Id<Inscription>,
}

/// Sanction à noter.
#[derive(Debug, Deserialize, specta::Type)]
pub struct DemandeSanction {
    /// Inscription de l'élève.
    pub inscription_id: Id<Inscription>,
    /// Nature.
    pub nature: NatureSanction,
    /// Jour, `AAAA-MM-JJ` ; vide : aujourd'hui.
    pub date: String,
    /// Motif.
    pub motif: String,
    /// Qui l'a décidée.
    pub decidee_par: String,
    /// Période du conseil de classe, pour la porter sur le bulletin.
    pub periode_id: Option<Id<Periode>>,
}

/// Annulation motivée d'une sanction.
#[derive(Debug, Deserialize, specta::Type)]
pub struct DemandeAnnulationSanction {
    /// Sanction.
    pub id: Id<Sanction>,
    /// Motif.
    pub motif: String,
}

/// Service de la discipline.
pub struct ServiceDiscipline<'n> {
    noyau: &'n Noyau,
}

impl<'n> ServiceDiscipline<'n> {
    /// Service sur le noyau.
    pub fn nouveau(noyau: &'n Noyau) -> Self {
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

    async fn droits(
        cnx: &mut gesco_db::Connexion,
        session: &Session,
        contexte: &Contexte,
    ) -> Resultat<DroitsDiscipline> {
        let p = role::permissions_de(cnx, session.utilisateur_id, contexte).await?;
        Ok(DroitsDiscipline {
            sanctionner: p.contains(&Permission::SANCTIONS_CREATE),
            annuler: p.contains(&Permission::SANCTIONS_CANCEL),
        })
    }

    /// Une sanction avec l'élève, sa classe, la période et qui l'a saisie.
    async fn afficher(
        cnx: &mut gesco_db::Connexion,
        s: Sanction,
        periodes: &[Periode],
        contexte: &Contexte,
    ) -> Resultat<SanctionAffichee> {
        let inscription = inscriptions::lire(cnx, s.inscription_id, contexte).await?;
        let (_, personne) = personnes::eleve(cnx, inscription.eleve_id, contexte).await?;
        let classe = academique::classe(cnx, inscription.classe_id, contexte).await?;
        let saisie_par = utilisateur::lire(cnx, s.saisie_par, contexte)
            .await
            .map(|u| u.nom_affiche)
            .unwrap_or_default();
        Ok(SanctionAffichee {
            eleve: personne.nom_complet(),
            classe: classe.libelle,
            periode: s
                .periode_id
                .and_then(|id| periodes.iter().find(|p| p.id == id))
                .map(Periode::libelle),
            saisie_par,
            sanction: s,
        })
    }

    /// Le registre de discipline de l'année courante.
    pub async fn registre(&self, session: &Session) -> Resultat<RegistreDiscipline> {
        let contexte = self.noyau.contexte(session.etablissement_id, Some(session));
        let mut cnx = self.connexion().await?;
        let annee = annee_scolaire::courante(&mut cnx, &contexte)
            .await?
            .ok_or(regle("sanction.sans_annee"))?;
        let periodes = pedagogie::periodes(&mut cnx, annee.id, &contexte).await?;
        let mut sanctions = Vec::new();
        for s in discipline::sanctions_de_l_annee(&mut cnx, annee.id, &contexte).await? {
            sanctions.push(Self::afficher(&mut cnx, s, &periodes, &contexte).await?);
        }
        Ok(RegistreDiscipline {
            annee_id: annee.id,
            sanctions,
            droits: Self::droits(&mut cnx, session, &contexte).await?,
        })
    }

    /// Le dossier de discipline d'un élève pour une inscription.
    pub async fn dossier(
        &self,
        session: &Session,
        demande: &DemandeDossierDiscipline,
    ) -> Resultat<DossierDiscipline> {
        let contexte = self.noyau.contexte(session.etablissement_id, Some(session));
        let mut cnx = self.connexion().await?;
        let inscription = inscriptions::lire(&mut cnx, demande.inscription_id, &contexte).await?;
        let (_, personne) = personnes::eleve(&mut cnx, inscription.eleve_id, &contexte).await?;
        let classe = academique::classe(&mut cnx, inscription.classe_id, &contexte).await?;
        let periodes: Vec<Periode> = pedagogie::periodes(&mut cnx, inscription.annee_id, &contexte)
            .await?
            .into_iter()
            .filter(|p| p.niveau_id == classe.niveau_id)
            .collect();
        let mut sanctions = Vec::new();
        for s in discipline::sanctions_de(&mut cnx, inscription.id, &contexte).await? {
            sanctions.push(Self::afficher(&mut cnx, s, &periodes, &contexte).await?);
        }
        Ok(DossierDiscipline {
            inscription_id: inscription.id,
            eleve: personne.nom_complet(),
            classe: classe.libelle,
            sanctions,
            periodes: periodes
                .iter()
                .map(|p| PeriodeConseil {
                    periode_id: p.id,
                    libelle: p.libelle(),
                })
                .collect(),
            droits: Self::droits(&mut cnx, session, &contexte).await?,
        })
    }

    /// Note une sanction au dossier d'un élève inscrit.
    pub async fn sanctionner(
        &self,
        session: &Session,
        demande: &DemandeSanction,
    ) -> Resultat<DossierDiscipline> {
        verifier_sanction(
            demande.nature,
            &demande.motif,
            &demande.decidee_par,
            demande.periode_id.is_some(),
        )
        .map_err(regle)?;
        let contexte = self.noyau.contexte(session.etablissement_id, Some(session));
        let date = if demande.date.trim().is_empty() {
            DateCivile::du_jour(contexte.maintenant).ok_or(regle("sanction.date_invalide"))?
        } else {
            DateCivile::analyser(&demande.date).ok_or(regle("sanction.date_invalide"))?
        };
        crate::annees::exiger_inscription_modifiable(self.noyau, session, demande.inscription_id)
            .await?;
        let mut unite = self
            .noyau
            .commencer(contexte.clone(), Durabilite::Ordinaire)
            .await?;
        let inscription =
            inscriptions::lire(unite.connexion(), demande.inscription_id, &contexte).await?;
        if inscription.statut != StatutInscription::Validee {
            return Err(regle("sanction.eleve_non_inscrit"));
        }
        if let Some(periode_id) = demande.periode_id {
            let classe =
                academique::classe(unite.connexion(), inscription.classe_id, &contexte).await?;
            pedagogie::periodes(unite.connexion(), inscription.annee_id, &contexte)
                .await?
                .iter()
                .find(|p| p.id == periode_id && p.niveau_id == classe.niveau_id)
                .ok_or(regle("sanction.periode_inconnue"))?;
        }
        let sanction = Sanction {
            id: nouvel_id(),
            etablissement_id: session.etablissement_id,
            inscription_id: inscription.id,
            nature: demande.nature,
            date,
            motif: demande.motif.trim().to_owned(),
            decidee_par: demande.decidee_par.trim().to_owned(),
            periode_id: demande.periode_id,
            saisie_par: session.utilisateur_id,
            annulee: false,
            motif_annulation: String::new(),
        };
        discipline::inserer(unite.connexion(), &sanction, &contexte).await?;
        let id = sanction.id.vers_chaine();
        unite.auditer(
            "sanction",
            &id,
            Operation::Creation,
            None,
            Some(serde_json::to_value(&sanction)?),
            "discipline.sanctionner",
        );
        unite.emettre(
            TypeEvenement::SanctionNotee,
            "sanction",
            &id,
            serde_json::json!({ "nature": sanction.nature, "inscription_id": inscription.id }),
        );
        self.noyau.terminer(unite).await?;
        self.dossier(
            session,
            &DemandeDossierDiscipline {
                inscription_id: inscription.id,
            },
        )
        .await
    }

    /// Annule une sanction saisie par erreur, avec un motif.
    pub async fn annuler(
        &self,
        session: &Session,
        demande: &DemandeAnnulationSanction,
    ) -> Resultat<DossierDiscipline> {
        let motif = demande.motif.trim();
        if motif.is_empty() {
            return Err(regle("sanction.motif_obligatoire"));
        }
        let contexte = self.noyau.contexte(session.etablissement_id, Some(session));
        let sanction = {
            let mut cnx = self.connexion().await?;
            discipline::sanction(&mut cnx, demande.id, &contexte).await?
        };
        if sanction.annulee {
            return Err(regle("sanction.deja_annulee"));
        }
        crate::annees::exiger_inscription_modifiable(self.noyau, session, sanction.inscription_id)
            .await?;
        let mut unite = self
            .noyau
            .commencer(contexte.clone(), Durabilite::Ordinaire)
            .await?;
        let inscription =
            inscriptions::lire(unite.connexion(), sanction.inscription_id, &contexte).await?;
        discipline::annuler(unite.connexion(), sanction.id, motif, &contexte).await?;
        let id = sanction.id.vers_chaine();
        unite.auditer(
            "sanction",
            &id,
            Operation::Modification,
            Some(serde_json::json!({ "annulee": false })),
            Some(serde_json::json!({ "annulee": true, "motif": motif })),
            "discipline.annuler",
        );
        unite.emettre(
            TypeEvenement::SanctionAnnulee,
            "sanction",
            &id,
            serde_json::json!({ "nature": sanction.nature, "motif": motif }),
        );
        self.noyau.terminer(unite).await?;
        self.dossier(
            session,
            &DemandeDossierDiscipline {
                inscription_id: inscription.id,
            },
        )
        .await
    }
}

/// Les sanctions du conseil de classe d'une période, par inscription, telles que le bulletin
/// les imprime : « Avertissement : travail insuffisant ».
pub(crate) async fn du_conseil_imprimees(
    cnx: &mut gesco_db::Connexion,
    periode_id: Id<Periode>,
    contexte: &Contexte,
) -> Resultat<Vec<(Id<Inscription>, String)>> {
    Ok(discipline::du_conseil(cnx, periode_id, contexte)
        .await?
        .into_iter()
        .map(|s| {
            (
                s.inscription_id,
                format!("{} : {}", s.nature.libelle(), s.motif),
            )
        })
        .collect())
}
