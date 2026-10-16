//! Authentification et session rapide (§6 de la spécification) : mot de passe à l'ouverture,
//! verrouillage progressif après échecs, code PIN pour rouvrir une session verrouillée,
//! bascule d'utilisateur depuis l'écran verrouillé.

use gesco_core::commun::Id;
use gesco_core::core::{Etablissement, Operation, TypeEvenement, Utilisateur};
use gesco_db::depots::utilisateur::Secrets;
use gesco_db::depots::{role, utilisateur};
use gesco_db::nouvel_id;
use gesco_db::unite::Durabilite;
use gesco_db::{Contexte, UniteDeTravail};

use crate::comptes::{empreinte_conforme, politique_mot_de_passe, seuil_verrouillage};
use crate::effets::Effet;
use crate::erreur::{ErreurService, MotifRefus, Resultat};
use crate::noyau::Noyau;
use crate::secrets::{MotDePasse, Pin, empreinte_pin, mot_de_passe_correspond, pin_correspond};
use crate::session::{ESSAIS_PIN, Jeton, Session};

/// Demande de création d'un utilisateur par un administrateur.
#[derive(Debug)]
pub struct NouvelUtilisateur<'a> {
    /// Identifiant de connexion, unique dans l'établissement.
    pub identifiant: &'a str,
    /// Nom affiché dans le bandeau et l'audit.
    pub nom_affiche: &'a str,
    /// Mot de passe initial, à changer à la première connexion.
    pub mot_de_passe_initial: MotDePasse<'a>,
    /// Codes des rôles attribués.
    pub roles: &'a [&'a str],
}

/// Service d'authentification.
pub struct ServiceAuthentification<'n> {
    noyau: &'n Noyau,
}

impl<'n> ServiceAuthentification<'n> {
    /// Service adossé au noyau.
    pub const fn nouveau(noyau: &'n Noyau) -> Self {
        Self { noyau }
    }

    /// Ouvre une session par identifiant et mot de passe.
    pub async fn connecter(
        &self,
        etablissement_id: Id<Etablissement>,
        identifiant: &str,
        mot_de_passe: MotDePasse<'_>,
    ) -> Resultat<Session> {
        let contexte = self.noyau.contexte(etablissement_id, None);
        let maintenant = contexte.maintenant;
        let mut unite = self
            .noyau
            .commencer(contexte.clone(), Durabilite::Critique)
            .await?;
        let Some(mut compte) =
            utilisateur::par_identifiant(unite.connexion(), identifiant, &contexte).await?
        else {
            unite.annuler().await?;
            return Err(ErreurService::Authentification(
                MotifRefus::IdentifiantsInvalides,
            ));
        };
        if !compte.actif {
            unite.annuler().await?;
            return Err(ErreurService::Authentification(MotifRefus::CompteInactif));
        }
        if !compte.peut_tenter_connexion(maintenant) {
            let jusqua = compte.verrouille_jusqua.map_or(0, |h| h.millis_epoque());
            unite.annuler().await?;
            return Err(ErreurService::Authentification(MotifRefus::Verrouille {
                jusqua,
            }));
        }
        let secrets = utilisateur::lire_secrets(unite.connexion(), compte.id, &contexte).await?;
        if !mot_de_passe_correspond(&mot_de_passe, &secrets.empreinte_mot_de_passe) {
            compte.echecs_consecutifs = compte.echecs_consecutifs.saturating_add(1);
            let seuil = seuil_verrouillage(unite.connexion(), &contexte).await?;
            compte.verrouille_jusqua =
                Utilisateur::duree_verrouillage_millis(compte.echecs_consecutifs, seuil)
                    .and_then(|d| maintenant.plus_millis(d).ok());
            utilisateur::ecrire_etat_connexion(unite.connexion(), &compte, &contexte).await?;
            Self::auditer_session(&mut unite, &compte, "connexion_refusee");
            self.noyau.terminer(unite).await?;
            return Err(ErreurService::Authentification(
                MotifRefus::IdentifiantsInvalides,
            ));
        }
        compte.echecs_consecutifs = 0;
        compte.verrouille_jusqua = None;
        compte.derniere_connexion = Some(maintenant);
        utilisateur::ecrire_etat_connexion(unite.connexion(), &compte, &contexte).await?;
        let roles = role::roles_de(unite.connexion(), compte.id, &contexte).await?;
        Self::auditer_session(&mut unite, &compte, "connexion");
        self.noyau.terminer(unite).await?;

        let session = Session {
            jeton: Jeton(nouvel_id::<Session>().vers_chaine()),
            utilisateur_id: compte.id,
            nom_affiche: compte.nom_affiche,
            etablissement_id,
            role_actif: roles.first().map(|r| r.code.clone()),
            ouverte_le: maintenant,
            derniere_activite: maintenant,
            verrouillee: false,
            pin_defini: secrets.empreinte_pin.is_some(),
        };
        self.noyau.sessions().ouvrir(session.clone());
        Ok(session)
    }

    /// Ferme la session.
    pub async fn deconnecter(&self, jeton: &Jeton) -> Resultat<()> {
        let session = self
            .noyau
            .sessions()
            .fermer(jeton)
            .ok_or(ErreurService::Authentification(MotifRefus::SessionInvalide))?;
        let contexte = self
            .noyau
            .contexte(session.etablissement_id, Some(&session));
        let mut unite = self
            .noyau
            .commencer(contexte.clone(), Durabilite::Ordinaire)
            .await?;
        let compte =
            utilisateur::lire(unite.connexion(), session.utilisateur_id, &contexte).await?;
        Self::auditer_session(&mut unite, &compte, "deconnexion");
        self.noyau.terminer(unite).await?;
        Ok(())
    }

    /// Verrouille la session : écran verrouillé, PIN attendu.
    pub fn verrouiller(&self, jeton: &Jeton) -> Resultat<Session> {
        self.noyau
            .sessions()
            .verrouiller(jeton)
            .ok_or(ErreurService::Authentification(MotifRefus::SessionInvalide))
    }

    /// Rouvre une session verrouillée par le code PIN de son utilisateur.
    pub async fn deverrouiller_par_pin(&self, jeton: &Jeton, pin: Pin<'_>) -> Resultat<Session> {
        let session = self
            .noyau
            .sessions()
            .lire(jeton)
            .ok_or(ErreurService::Authentification(MotifRefus::SessionInvalide))?;
        let contexte = self
            .noyau
            .contexte(session.etablissement_id, Some(&session));
        let mut cnx = self
            .noyau
            .base()
            .pool()
            .acquire()
            .await
            .map_err(gesco_db::ErreurBase::from)?;
        let secrets =
            utilisateur::lire_secrets(&mut cnx, session.utilisateur_id, &contexte).await?;
        let Some(empreinte) = secrets.empreinte_pin else {
            return Err(ErreurService::Authentification(MotifRefus::PinNonDefini));
        };
        if !pin_correspond(&pin, &empreinte) {
            if self.noyau.sessions().noter_echec_pin(jeton) >= ESSAIS_PIN {
                self.noyau.sessions().fermer(jeton);
                return Err(ErreurService::Authentification(MotifRefus::PinEpuise));
            }
            return Err(ErreurService::Authentification(MotifRefus::PinInvalide));
        }
        self.noyau.sessions().effacer_echecs_pin(jeton);
        self.noyau
            .sessions()
            .deverrouiller(jeton, contexte.maintenant)
            .ok_or(ErreurService::Authentification(MotifRefus::SessionInvalide))
    }

    /// Rouvre une session verrouillée par le mot de passe de son utilisateur, quand il n'a pas
    /// de PIN. Un mot de passe faux compte comme un échec de connexion (verrouillage progressif).
    pub async fn deverrouiller_par_mot_de_passe(
        &self,
        jeton: &Jeton,
        mot_de_passe: MotDePasse<'_>,
    ) -> Resultat<Session> {
        let session = self
            .noyau
            .sessions()
            .lire(jeton)
            .ok_or(ErreurService::Authentification(MotifRefus::SessionInvalide))?;
        let contexte = self
            .noyau
            .contexte(session.etablissement_id, Some(&session));
        let maintenant = contexte.maintenant;
        let mut unite = self
            .noyau
            .commencer(contexte.clone(), Durabilite::Critique)
            .await?;
        let mut compte =
            utilisateur::lire(unite.connexion(), session.utilisateur_id, &contexte).await?;
        if !compte.peut_tenter_connexion(maintenant) {
            let jusqua = compte.verrouille_jusqua.map_or(0, |h| h.millis_epoque());
            unite.annuler().await?;
            return Err(ErreurService::Authentification(MotifRefus::Verrouille {
                jusqua,
            }));
        }
        let secrets = utilisateur::lire_secrets(unite.connexion(), compte.id, &contexte).await?;
        if !mot_de_passe_correspond(&mot_de_passe, &secrets.empreinte_mot_de_passe) {
            compte.echecs_consecutifs = compte.echecs_consecutifs.saturating_add(1);
            let seuil = seuil_verrouillage(unite.connexion(), &contexte).await?;
            compte.verrouille_jusqua =
                Utilisateur::duree_verrouillage_millis(compte.echecs_consecutifs, seuil)
                    .and_then(|d| maintenant.plus_millis(d).ok());
            utilisateur::ecrire_etat_connexion(unite.connexion(), &compte, &contexte).await?;
            Self::auditer_session(&mut unite, &compte, "deverrouillage_refuse");
            self.noyau.terminer(unite).await?;
            return Err(ErreurService::Authentification(
                MotifRefus::IdentifiantsInvalides,
            ));
        }
        if compte.echecs_consecutifs > 0 {
            compte.echecs_consecutifs = 0;
            compte.verrouille_jusqua = None;
            utilisateur::ecrire_etat_connexion(unite.connexion(), &compte, &contexte).await?;
            Self::auditer_session(&mut unite, &compte, "deverrouillage");
            self.noyau.terminer(unite).await?;
        } else {
            unite.annuler().await?;
        }
        self.noyau
            .sessions()
            .deverrouiller(jeton, maintenant)
            .ok_or(ErreurService::Authentification(MotifRefus::SessionInvalide))
    }

    /// Définit ou remplace le code PIN de la session courante. Le mot de passe est redemandé :
    /// le PIN ne se change pas depuis une session simplement déverrouillée.
    pub async fn definir_pin(
        &self,
        session: &Session,
        mot_de_passe: MotDePasse<'_>,
        pin: Pin<'_>,
    ) -> Resultat<()> {
        let contexte = self.noyau.contexte(session.etablissement_id, Some(session));
        let mut unite = self
            .noyau
            .commencer(contexte.clone(), Durabilite::Critique)
            .await?;
        let secrets =
            utilisateur::lire_secrets(unite.connexion(), session.utilisateur_id, &contexte).await?;
        if !mot_de_passe_correspond(&mot_de_passe, &secrets.empreinte_mot_de_passe) {
            unite.annuler().await?;
            return Err(ErreurService::Authentification(
                MotifRefus::IdentifiantsInvalides,
            ));
        }
        let nouveaux = Secrets {
            empreinte_mot_de_passe: secrets.empreinte_mot_de_passe,
            empreinte_pin: Some(empreinte_pin(&pin)?),
        };
        utilisateur::ecrire_secrets(
            unite.connexion(),
            session.utilisateur_id,
            &nouveaux,
            &contexte,
        )
        .await?;
        unite.auditer(
            "utilisateur",
            &session.utilisateur_id.vers_chaine(),
            Operation::Modification,
            None,
            Some(serde_json::json!({ "pin": "defini" })),
            "session.definir_pin",
        );
        self.noyau.terminer(unite).await?;
        self.noyau
            .sessions()
            .marquer_pin_defini(session.utilisateur_id);
        Ok(())
    }

    /// Change le mot de passe de la session courante et lève l'obligation de changement.
    pub async fn changer_mot_de_passe(
        &self,
        session: &Session,
        ancien: MotDePasse<'_>,
        nouveau: MotDePasse<'_>,
    ) -> Resultat<()> {
        let contexte = self.noyau.contexte(session.etablissement_id, Some(session));
        let mut unite = self
            .noyau
            .commencer(contexte.clone(), Durabilite::Critique)
            .await?;
        let secrets =
            utilisateur::lire_secrets(unite.connexion(), session.utilisateur_id, &contexte).await?;
        if !mot_de_passe_correspond(&ancien, &secrets.empreinte_mot_de_passe) {
            unite.annuler().await?;
            return Err(ErreurService::Authentification(
                MotifRefus::IdentifiantsInvalides,
            ));
        }
        let compte_avant =
            utilisateur::lire(unite.connexion(), session.utilisateur_id, &contexte).await?;
        let politique = politique_mot_de_passe(unite.connexion(), &contexte).await?;
        let nouveaux = Secrets {
            empreinte_mot_de_passe: empreinte_conforme(
                &politique,
                &nouveau,
                &compte_avant.identifiant,
            )?,
            empreinte_pin: secrets.empreinte_pin,
        };
        utilisateur::ecrire_secrets(
            unite.connexion(),
            session.utilisateur_id,
            &nouveaux,
            &contexte,
        )
        .await?;
        let mut compte =
            utilisateur::lire(unite.connexion(), session.utilisateur_id, &contexte).await?;
        compte.doit_changer_mot_de_passe = false;
        utilisateur::ecrire_etat_connexion(unite.connexion(), &compte, &contexte).await?;
        unite.auditer(
            "utilisateur",
            &compte.id.vers_chaine(),
            Operation::Modification,
            None,
            Some(serde_json::json!({ "mot_de_passe": "change" })),
            "session.changer_mot_de_passe",
        );
        self.noyau.terminer(unite).await?;
        Ok(())
    }

    /// Crée un utilisateur et lui attribue ses rôles. La garde a déjà vérifié
    /// `core:users:create` ; le contexte porte l'administrateur qui agit.
    pub async fn creer_utilisateur(
        &self,
        contexte: &Contexte,
        nouveau: NouvelUtilisateur<'_>,
    ) -> Resultat<Utilisateur> {
        let mut unite = self
            .noyau
            .commencer(contexte.clone(), Durabilite::Critique)
            .await?;
        let politique = politique_mot_de_passe(unite.connexion(), contexte).await?;
        let empreinte = empreinte_conforme(
            &politique,
            &nouveau.mot_de_passe_initial,
            nouveau.identifiant,
        )?;
        if utilisateur::par_identifiant(unite.connexion(), nouveau.identifiant, contexte)
            .await?
            .is_some()
        {
            unite.annuler().await?;
            return Err(ErreurService::Regle {
                cle: "utilisateur.identifiant_deja_pris",
            });
        }
        let compte = Utilisateur {
            id: nouvel_id(),
            etablissement_id: contexte.etablissement_id,
            identifiant: nouveau.identifiant.to_owned(),
            nom_affiche: nouveau.nom_affiche.to_owned(),
            actif: true,
            doit_changer_mot_de_passe: true,
            derniere_connexion: None,
            echecs_consecutifs: 0,
            verrouille_jusqua: None,
        };
        let secrets = Secrets {
            empreinte_mot_de_passe: empreinte,
            empreinte_pin: None,
        };
        utilisateur::inserer(unite.connexion(), &compte, &secrets, contexte).await?;
        let id_texte = compte.id.vers_chaine();
        for code in nouveau.roles {
            let role = role::par_code(unite.connexion(), code, contexte)
                .await?
                .ok_or(ErreurService::Regle {
                    cle: "role.inconnu",
                })?;
            role::attribuer(unite.connexion(), compte.id, role.id, contexte).await?;
            unite.emettre(
                TypeEvenement::RoleAttribue,
                "utilisateur",
                &id_texte,
                serde_json::json!({ "role": role.code }),
            );
        }
        unite.auditer(
            "utilisateur",
            &id_texte,
            Operation::Creation,
            None,
            Some(serde_json::to_value(&compte)?),
            "utilisateurs.creer",
        );
        unite.emettre(
            TypeEvenement::UtilisateurCree,
            "utilisateur",
            &id_texte,
            serde_json::json!({ "identifiant": compte.identifiant, "nom_affiche": compte.nom_affiche }),
        );
        self.noyau.terminer(unite).await?;
        Ok(compte)
    }

    fn auditer_session(unite: &mut UniteDeTravail<Effet>, compte: &Utilisateur, evenement: &str) {
        unite.auditer(
            "utilisateur",
            &compte.id.vers_chaine(),
            Operation::Session,
            None,
            Some(serde_json::json!({
                "evenement": evenement,
                "echecs_consecutifs": compte.echecs_consecutifs,
            })),
            "session",
        );
    }
}
