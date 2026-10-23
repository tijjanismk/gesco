//! Cycle de vie des comptes (SPEC_PHASE_1 §4) : réinitialisation du mot de passe par un
//! administrateur, déverrouillage après échecs, et garde-fous qui protègent le dernier
//! administrateur actif.

use gesco_core::commun::Id;
use gesco_core::core::permission::RoleSysteme;
use gesco_core::core::{Etablissement, Operation, PolitiqueMotDePasse, TypeEvenement, Utilisateur};
use gesco_db::depots::utilisateur::Secrets;
use gesco_db::depots::{parametre, role, utilisateur};
use gesco_db::unite::Durabilite;
use gesco_db::{Connexion, Contexte};

use crate::erreur::{ErreurService, Resultat};
use crate::noyau::Noyau;
use crate::secrets::{MotDePasse, empreinte_mot_de_passe};
use crate::session::Session;

fn regle(cle: &'static str) -> ErreurService {
    ErreurService::Regle { cle }
}

/// Politique de mot de passe de l'établissement, lue dans ses paramètres (SPEC_PHASE_1 §6).
pub(crate) async fn politique_mot_de_passe(
    cnx: &mut Connexion,
    contexte: &Contexte,
) -> Resultat<PolitiqueMotDePasse> {
    let defaut = PolitiqueMotDePasse::default();
    let longueur = parametre::entier(
        &mut *cnx,
        "mot_de_passe.longueur_min",
        i64::try_from(defaut.longueur_min).unwrap_or(i64::MAX),
        contexte,
    )
    .await?;
    Ok(PolitiqueMotDePasse {
        longueur_min: usize::try_from(longueur).unwrap_or(defaut.longueur_min),
        refuser_identifiant: parametre::booleen_param(
            &mut *cnx,
            "mot_de_passe.refuser_identifiant",
            defaut.refuser_identifiant,
            contexte,
        )
        .await?,
        refuser_courants: parametre::booleen_param(
            cnx,
            "mot_de_passe.refuser_courants",
            defaut.refuser_courants,
            contexte,
        )
        .await?,
    })
}

/// Applique la politique et rend l'empreinte du mot de passe accepté.
pub(crate) fn empreinte_conforme(
    politique: &PolitiqueMotDePasse,
    mot_de_passe: &MotDePasse<'_>,
    identifiant: &str,
) -> Resultat<String> {
    politique
        .verifier(mot_de_passe.0, identifiant)
        .map_err(regle)?;
    empreinte_mot_de_passe(mot_de_passe)
}

/// Seuil d'échecs de connexion avant verrouillage (paramètre, 3 par défaut).
pub(crate) async fn seuil_verrouillage(cnx: &mut Connexion, contexte: &Contexte) -> Resultat<u32> {
    let seuil = parametre::entier(cnx, "connexion.echecs_avant_verrouillage", 3, contexte).await?;
    Ok(u32::try_from(seuil).unwrap_or(3).clamp(3, 10))
}

/// Vrai si l'utilisateur porte le rôle Administrateur.
pub(crate) async fn est_administrateur(
    cnx: &mut Connexion,
    utilisateur_id: Id<Utilisateur>,
    contexte: &Contexte,
) -> Resultat<bool> {
    Ok(role::roles_de(cnx, utilisateur_id, contexte)
        .await?
        .iter()
        .any(|r| r.code == RoleSysteme::Administrateur.code()))
}

/// Refuse une opération qui laisserait l'établissement sans administrateur actif : désactiver
/// le dernier, ou lui retirer son rôle (SPEC_PHASE_1 §4.2).
pub(crate) async fn proteger_dernier_administrateur(
    cnx: &mut Connexion,
    cible: Id<Utilisateur>,
    contexte: &Contexte,
) -> Resultat<()> {
    let Some(administrateur) =
        role::par_code(&mut *cnx, RoleSysteme::Administrateur.code(), contexte).await?
    else {
        return Ok(());
    };
    let actifs = role::porteurs_actifs_ids(cnx, administrateur.id, contexte).await?;
    if actifs == [cible] {
        return Err(regle("utilisateur.dernier_administrateur"));
    }
    Ok(())
}

/// Origine au journal d'audit d'un mot de passe d'administrateur remplacé par la phrase de
/// secours.
pub const ORIGINE_REINITIALISATION_PAR_PHRASE: &str = "comptes.reinitialiser_par_phrase";

/// Jours pendant lesquels le tableau de bord signale cette réinitialisation.
pub const JOURS_SIGNALEMENT_REINITIALISATION: i64 = 7;

/// Service des comptes.
pub struct ServiceComptes<'n> {
    noyau: &'n Noyau,
}

impl<'n> ServiceComptes<'n> {
    /// Service adossé au noyau.
    pub const fn nouveau(noyau: &'n Noyau) -> Self {
        Self { noyau }
    }

    /// Remplace le mot de passe d'un compte par un mot de passe initial : changement exigé à la
    /// connexion suivante, PIN effacé, compteur d'échecs remis à zéro, sessions fermées. Seul un
    /// administrateur réinitialise le mot de passe d'un administrateur ; personne ne réinitialise
    /// le sien, qui se change dans « Mon compte ».
    pub async fn reinitialiser_mot_de_passe(
        &self,
        session: &Session,
        id: Id<Utilisateur>,
        mot_de_passe_initial: MotDePasse<'_>,
    ) -> Resultat<Utilisateur> {
        if id == session.utilisateur_id {
            return Err(regle("utilisateur.reinitialisation_de_soi"));
        }
        let contexte = self.noyau.contexte(session.etablissement_id, Some(session));
        let mut unite = self
            .noyau
            .commencer(contexte.clone(), Durabilite::Critique)
            .await?;
        if est_administrateur(unite.connexion(), id, &contexte).await?
            && !est_administrateur(unite.connexion(), session.utilisateur_id, &contexte).await?
        {
            return Err(regle("utilisateur.reinitialisation_administrateur"));
        }
        let avant = utilisateur::lire(unite.connexion(), id, &contexte).await?;
        let politique = politique_mot_de_passe(unite.connexion(), &contexte).await?;
        let empreinte = empreinte_conforme(&politique, &mot_de_passe_initial, &avant.identifiant)?;
        utilisateur::ecrire_secrets(
            unite.connexion(),
            id,
            &Secrets {
                empreinte_mot_de_passe: empreinte,
                empreinte_pin: None,
            },
            &contexte,
        )
        .await?;
        let mut apres = avant.clone();
        apres.doit_changer_mot_de_passe = true;
        apres.echecs_consecutifs = 0;
        apres.verrouille_jusqua = None;
        utilisateur::ecrire_etat_connexion(unite.connexion(), &apres, &contexte).await?;
        let fermees = self.noyau.sessions().fermer_de(id).len();
        let id_texte = id.vers_chaine();
        unite.auditer(
            "utilisateur",
            &id_texte,
            Operation::Modification,
            Some(serde_json::to_value(&avant)?),
            Some(serde_json::json!({
                "compte": apres,
                "pin_efface": true,
                "sessions_fermees": fermees,
            })),
            "comptes.reinitialiser_mot_de_passe",
        );
        unite.emettre(
            TypeEvenement::MotDePasseReinitialise,
            "utilisateur",
            &id_texte,
            serde_json::json!({
                "identifiant": apres.identifiant,
                "nom_affiche": apres.nom_affiche,
                "sessions_fermees": fermees,
            }),
        );
        self.noyau.terminer(unite).await?;
        Ok(apres)
    }

    /// Remplace le mot de passe d'un administrateur actif, sans session : l'appelant a déjà
    /// vérifié la phrase de secours, qui prouve qu'on détient le secret de l'école. Le compte est
    /// déverrouillé, son PIN effacé, ses sessions fermées ; le tableau de bord le signale sept
    /// jours (porteur du projet, 23 octobre 2026).
    pub async fn reinitialiser_par_phrase(
        &self,
        etablissement_id: Id<Etablissement>,
        identifiant: &str,
        nouveau: MotDePasse<'_>,
    ) -> Resultat<Utilisateur> {
        let contexte = self.noyau.contexte(etablissement_id, None);
        let mut unite = self
            .noyau
            .commencer(contexte.clone(), Durabilite::Critique)
            .await?;
        let avant = utilisateur::par_identifiant(unite.connexion(), identifiant.trim(), &contexte)
            .await?
            .filter(|u| u.actif)
            .ok_or(regle("secours.administrateur_inconnu"))?;
        if !est_administrateur(unite.connexion(), avant.id, &contexte).await? {
            return Err(regle("secours.administrateur_inconnu"));
        }
        let politique = politique_mot_de_passe(unite.connexion(), &contexte).await?;
        let empreinte = empreinte_conforme(&politique, &nouveau, &avant.identifiant)?;
        utilisateur::ecrire_secrets(
            unite.connexion(),
            avant.id,
            &Secrets {
                empreinte_mot_de_passe: empreinte,
                empreinte_pin: None,
            },
            &contexte,
        )
        .await?;
        let mut apres = avant.clone();
        apres.doit_changer_mot_de_passe = false;
        apres.echecs_consecutifs = 0;
        apres.verrouille_jusqua = None;
        utilisateur::ecrire_etat_connexion(unite.connexion(), &apres, &contexte).await?;
        let fermees = self.noyau.sessions().fermer_de(avant.id).len();
        let id_texte = avant.id.vers_chaine();
        unite.auditer(
            "utilisateur",
            &id_texte,
            Operation::Modification,
            Some(serde_json::to_value(&avant)?),
            Some(serde_json::json!({
                "compte": apres,
                "pin_efface": true,
                "sessions_fermees": fermees,
            })),
            ORIGINE_REINITIALISATION_PAR_PHRASE,
        );
        unite.emettre(
            TypeEvenement::MotDePasseReinitialise,
            "utilisateur",
            &id_texte,
            serde_json::json!({
                "identifiant": apres.identifiant,
                "nom_affiche": apres.nom_affiche,
                "sessions_fermees": fermees,
                "par_phrase_de_secours": true,
            }),
        );
        self.noyau.terminer(unite).await?;
        Ok(apres)
    }

    /// Lève le verrouillage temporaire dû aux échecs de connexion.
    pub async fn deverrouiller(
        &self,
        session: &Session,
        id: Id<Utilisateur>,
    ) -> Resultat<Utilisateur> {
        let contexte = self.noyau.contexte(session.etablissement_id, Some(session));
        let mut unite = self
            .noyau
            .commencer(contexte.clone(), Durabilite::Critique)
            .await?;
        let avant = utilisateur::lire(unite.connexion(), id, &contexte).await?;
        let mut apres = avant.clone();
        apres.echecs_consecutifs = 0;
        apres.verrouille_jusqua = None;
        utilisateur::ecrire_etat_connexion(unite.connexion(), &apres, &contexte).await?;
        unite.auditer(
            "utilisateur",
            &id.vers_chaine(),
            Operation::Modification,
            Some(serde_json::to_value(&avant)?),
            Some(serde_json::to_value(&apres)?),
            "comptes.deverrouiller",
        );
        self.noyau.terminer(unite).await?;
        Ok(apres)
    }
}
