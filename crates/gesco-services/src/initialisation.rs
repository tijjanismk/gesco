//! Premier démarrage : la base vient d'être créée, aucun utilisateur n'existe. On pose
//! l'organisation, l'établissement, les paramètres par défaut, les rôles livrés et le compte
//! administrateur, dans une seule transaction (R-17).

use gesco_core::commun::Id;
use gesco_core::core::parametre::parametres_par_defaut;
use gesco_core::core::permission::{COMPOSITION_ROLES_LIVRES, RoleSysteme};
use gesco_core::core::{Etablissement, Operation, Organisation, Role, TypeEvenement, Utilisateur};
use gesco_core::scolarite::Cycle;
use gesco_db::depots::utilisateur::Secrets;
use gesco_db::depots::{etablissement, parametre, role, utilisateur};
use gesco_db::nouvel_id;
use gesco_db::unite::Durabilite;

use crate::erreur::{ErreurService, Resultat};
use crate::noyau::Noyau;
use crate::secrets::MotDePasse;

/// Données du premier démarrage.
#[derive(Debug)]
pub struct PremierDemarrage<'a> {
    /// Nom de l'organisation propriétaire.
    pub organisation: &'a str,
    /// Établissement, avec son identifiant déjà tiré.
    pub etablissement: Etablissement,
    /// Identifiant de connexion de l'administrateur.
    pub administrateur_identifiant: &'a str,
    /// Nom affiché de l'administrateur.
    pub administrateur_nom: &'a str,
    /// Mot de passe initial ; le changement est exigé à la première connexion.
    pub administrateur_mot_de_passe: MotDePasse<'a>,
    /// Cycles enseignés : leurs niveaux sont actifs, les autres désactivés.
    pub cycles: &'a [Cycle],
}

/// Résultat du premier démarrage.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EtablissementInitialise {
    /// Établissement créé.
    pub etablissement_id: Id<Etablissement>,
    /// Administrateur créé.
    pub administrateur_id: Id<Utilisateur>,
    /// Rôles livrés, par code.
    pub roles: Vec<String>,
}

/// Service du premier démarrage.
pub struct ServiceInitialisation<'n> {
    noyau: &'n Noyau,
}

impl<'n> ServiceInitialisation<'n> {
    /// Service adossé au noyau.
    pub const fn nouveau(noyau: &'n Noyau) -> Self {
        Self { noyau }
    }

    /// `true` si aucun utilisateur n'existe encore dans l'établissement : l'écran de premier
    /// démarrage ne s'affiche que dans ce cas.
    pub async fn est_vierge(&self, etablissement_id: Id<Etablissement>) -> Resultat<bool> {
        let contexte = self.noyau.contexte(etablissement_id, None);
        let mut cnx = self
            .noyau
            .base()
            .pool()
            .acquire()
            .await
            .map_err(gesco_db::ErreurBase::from)?;
        Ok(utilisateur::compter(&mut cnx, &contexte).await? == 0)
    }

    /// Exécute le premier démarrage. Refusé si l'établissement compte déjà un utilisateur.
    pub async fn executer(
        &self,
        demarrage: PremierDemarrage<'_>,
    ) -> Resultat<EtablissementInitialise> {
        let etablissement_id = demarrage.etablissement.id;
        if !self.est_vierge(etablissement_id).await? {
            return Err(ErreurService::Regle {
                cle: "premier_demarrage.deja_effectue",
            });
        }
        crate::referentiel::verifier_cycles(demarrage.cycles)?;
        // Les paramètres n'existent pas encore : la politique par défaut s'applique.
        let empreinte = crate::comptes::empreinte_conforme(
            &gesco_core::core::PolitiqueMotDePasse::default(),
            &demarrage.administrateur_mot_de_passe,
            demarrage.administrateur_identifiant,
        )?;
        let contexte = self.noyau.contexte(etablissement_id, None);
        let mut unite = self
            .noyau
            .commencer(contexte.clone(), Durabilite::Critique)
            .await?;

        let organisation = Organisation {
            id: demarrage.etablissement.organisation_id,
            nom: demarrage.organisation.to_owned(),
        };
        etablissement::inserer_organisation(unite.connexion(), &organisation, &contexte).await?;
        etablissement::inserer(unite.connexion(), &demarrage.etablissement, &contexte).await?;
        unite.auditer(
            "etablissement",
            &etablissement_id.vers_chaine(),
            Operation::Creation,
            None,
            Some(serde_json::to_value(&demarrage.etablissement)?),
            "premier_demarrage",
        );
        parametre::definir_si_absent(
            unite.connexion(),
            &parametres_par_defaut(etablissement_id),
            &contexte,
        )
        .await?;

        crate::referentiel::poser_referentiel(
            &mut unite,
            etablissement_id,
            demarrage.cycles,
            "premier_demarrage",
        )
        .await?;

        let mut codes = Vec::with_capacity(RoleSysteme::TOUS.len());
        let mut role_administrateur: Option<Id<Role>> = None;
        for systeme in RoleSysteme::TOUS {
            let role = Role {
                id: nouvel_id(),
                etablissement_id,
                code: systeme.code().to_owned(),
                libelle: systeme.libelle().to_owned(),
                systeme: true,
            };
            role::inserer(unite.connexion(), &role, &contexte).await?;
            for permission in systeme.permissions_initiales() {
                role::accorder(unite.connexion(), role.id, permission, &contexte).await?;
            }
            role::noter_composition_livree(
                unite.connexion(),
                role.id,
                COMPOSITION_ROLES_LIVRES,
                &contexte,
            )
            .await?;
            if *systeme == RoleSysteme::Administrateur {
                role_administrateur = Some(role.id);
            }
            codes.push(role.code);
        }
        let role_administrateur = role_administrateur.ok_or(ErreurService::Regle {
            cle: "premier_demarrage.role_administrateur_absent",
        })?;

        let administrateur = Utilisateur {
            id: nouvel_id(),
            etablissement_id,
            identifiant: demarrage.administrateur_identifiant.to_owned(),
            nom_affiche: demarrage.administrateur_nom.to_owned(),
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
        utilisateur::inserer(unite.connexion(), &administrateur, &secrets, &contexte).await?;
        role::attribuer(
            unite.connexion(),
            administrateur.id,
            role_administrateur,
            &contexte,
        )
        .await?;
        let id_texte = administrateur.id.vers_chaine();
        unite.auditer(
            "utilisateur",
            &id_texte,
            Operation::Creation,
            None,
            Some(serde_json::to_value(&administrateur)?),
            "premier_demarrage",
        );
        unite.emettre(
            TypeEvenement::UtilisateurCree,
            "utilisateur",
            &id_texte,
            serde_json::json!({
                "identifiant": administrateur.identifiant,
                "nom_affiche": administrateur.nom_affiche,
            }),
        );
        unite.emettre(
            TypeEvenement::RoleAttribue,
            "utilisateur",
            &id_texte,
            serde_json::json!({ "role": RoleSysteme::Administrateur.code() }),
        );
        self.noyau.terminer(unite).await?;
        Ok(EtablissementInitialise {
            etablissement_id,
            administrateur_id: administrateur.id,
            roles: codes,
        })
    }
}
