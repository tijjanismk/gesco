//! Administration de la phase 0.5 (spécification §12.1) : utilisateurs et rôles, paramètres
//! d'établissement, journal d'audit. Chaque mutation est auditée et, quand elle a un sens hors
//! du poste, publiée dans l'`outbox` (R-17).

use gesco_core::commun::{Horodatage, Id};
use gesco_core::core::{
    LigneAudit, Operation, Parametre, Permission, Role, RoleSysteme, TypeEvenement, Utilisateur,
    ValeurParametre,
};
use gesco_db::depots::{audit, parametre, role, utilisateur};
use gesco_db::unite::Durabilite;
use serde::{Deserialize, Serialize};

use crate::comptes::proteger_dernier_administrateur;
use crate::erreur::{ErreurService, Resultat};
use crate::noyau::Noyau;
use crate::session::Session;

/// Utilisateur et codes de ses rôles.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
pub struct FicheUtilisateur {
    /// Compte.
    pub utilisateur: Utilisateur,
    /// Codes des rôles attribués.
    pub roles: Vec<String>,
}

/// Rôle et permissions accordées.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, specta::Type)]
pub struct FicheRole {
    /// Rôle.
    pub role: Role,
    /// Permissions du rôle.
    pub permissions: Vec<Permission>,
}

/// Lignes par page du journal (SPEC_PHASE_1 §8).
pub const TAILLE_PAGE_JOURNAL: i64 = 200;

/// Recherche dans le journal d'audit ; un critère absent ne filtre pas.
#[derive(Debug, Clone, Default, Deserialize, specta::Type)]
pub struct RechercheJournal {
    /// Auteur des mutations.
    pub utilisateur_id: Option<Id<Utilisateur>>,
    /// Entité touchée.
    pub entite: Option<String>,
    /// Opération.
    pub operation: Option<Operation>,
    /// Début de la période, inclus.
    pub depuis: Option<Horodatage>,
    /// Fin de la période, exclue.
    pub jusqua: Option<Horodatage>,
    /// Page, à partir de 0.
    pub page: u32,
}

/// Auteur possible d'une ligne du journal, pour afficher son nom.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, specta::Type)]
pub struct Auteur {
    /// Compte.
    pub id: Id<Utilisateur>,
    /// Nom affiché.
    pub nom: String,
}

/// Une page du journal filtré.
#[derive(Debug, Clone, PartialEq, Serialize, specta::Type)]
pub struct PageJournal {
    /// Lignes, de la plus récente à la plus ancienne.
    pub lignes: Vec<LigneAudit>,
    /// Numéro de la page.
    pub page: u32,
    /// Vrai si une page suivante existe.
    pub suivante: bool,
    /// Entités présentes dans le journal.
    pub entites: Vec<String>,
    /// Comptes de l'établissement, pour nommer les auteurs.
    pub auteurs: Vec<Auteur>,
}

/// Résultat d'une vérification complète de la chaîne d'audit.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
pub struct VerificationAudit {
    /// Lignes vérifiées.
    pub lignes_verifiees: u32,
}

/// Service d'administration.
pub struct ServiceAdministration<'n> {
    noyau: &'n Noyau,
}

impl<'n> ServiceAdministration<'n> {
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

    /// Utilisateurs de l'établissement, avec leurs rôles.
    pub async fn utilisateurs(&self, session: &Session) -> Resultat<Vec<FicheUtilisateur>> {
        let contexte = self.noyau.contexte(session.etablissement_id, Some(session));
        let mut cnx = self.connexion().await?;
        let mut fiches = Vec::new();
        for compte in utilisateur::lister(&mut cnx, &contexte).await? {
            let roles = role::roles_de(&mut cnx, compte.id, &contexte)
                .await?
                .into_iter()
                .map(|r| r.code)
                .collect();
            fiches.push(FicheUtilisateur {
                utilisateur: compte,
                roles,
            });
        }
        Ok(fiches)
    }

    /// Modifie le nom affiché ou l'état d'un compte. On ne désactive ni son propre compte, ni le
    /// dernier administrateur actif ; un compte désactivé perd aussitôt ses sessions.
    pub async fn modifier_utilisateur(
        &self,
        session: &Session,
        id: Id<Utilisateur>,
        nom_affiche: &str,
        actif: bool,
    ) -> Resultat<Utilisateur> {
        if nom_affiche.trim().is_empty() {
            return Err(ErreurService::Regle {
                cle: "utilisateur.nom_obligatoire",
            });
        }
        if id == session.utilisateur_id && !actif {
            return Err(ErreurService::Regle {
                cle: "utilisateur.desactivation_de_soi",
            });
        }
        let contexte = self.noyau.contexte(session.etablissement_id, Some(session));
        let mut unite = self
            .noyau
            .commencer(contexte.clone(), Durabilite::Critique)
            .await?;
        let avant = utilisateur::lire(unite.connexion(), id, &contexte).await?;
        if avant.actif && !actif {
            proteger_dernier_administrateur(unite.connexion(), id, &contexte).await?;
        }
        let mut apres = avant.clone();
        nom_affiche.trim().clone_into(&mut apres.nom_affiche);
        apres.actif = actif;
        utilisateur::ecrire_etat_connexion(unite.connexion(), &apres, &contexte).await?;
        let id_texte = id.vers_chaine();
        unite.auditer(
            "utilisateur",
            &id_texte,
            Operation::Modification,
            Some(serde_json::to_value(&avant)?),
            Some(serde_json::to_value(&apres)?),
            "administration.utilisateur_modifier",
        );
        if avant.actif != actif {
            let fermees = if actif {
                0
            } else {
                self.noyau.sessions().fermer_de(id).len()
            };
            unite.emettre(
                if actif {
                    TypeEvenement::CompteReactive
                } else {
                    TypeEvenement::CompteDesactive
                },
                "utilisateur",
                &id_texte,
                serde_json::json!({
                    "identifiant": apres.identifiant,
                    "nom_affiche": apres.nom_affiche,
                    "sessions_fermees": fermees,
                }),
            );
        }
        self.noyau.terminer(unite).await?;
        Ok(apres)
    }

    /// Rôles de l'établissement, avec leurs permissions.
    pub async fn roles(&self, session: &Session) -> Resultat<Vec<FicheRole>> {
        let contexte = self.noyau.contexte(session.etablissement_id, Some(session));
        let mut cnx = self.connexion().await?;
        let mut fiches = Vec::new();
        for r in role::lister(&mut cnx, &contexte).await? {
            let permissions = role::permissions_du_role(&mut cnx, r.id, &contexte).await?;
            fiches.push(FicheRole {
                role: r,
                permissions,
            });
        }
        Ok(fiches)
    }

    /// Attribue ou retire un rôle. Personne ne modifie ses propres rôles ; on n'attribue qu'un
    /// rôle dont on détient toutes les permissions ; le dernier administrateur actif garde le sien.
    pub async fn attribuer_role(
        &self,
        session: &Session,
        utilisateur_id: Id<Utilisateur>,
        code_role: &str,
        attribue: bool,
    ) -> Resultat<()> {
        if utilisateur_id == session.utilisateur_id {
            return Err(ErreurService::Regle {
                cle: "role.modification_de_soi",
            });
        }
        let contexte = self.noyau.contexte(session.etablissement_id, Some(session));
        let mut unite = self
            .noyau
            .commencer(contexte.clone(), Durabilite::Critique)
            .await?;
        let r = role::par_code(unite.connexion(), code_role, &contexte)
            .await?
            .ok_or(ErreurService::Regle {
                cle: "role.inconnu",
            })?;
        let id_texte = utilisateur_id.vers_chaine();
        if attribue {
            let detenues =
                role::permissions_de(unite.connexion(), session.utilisateur_id, &contexte).await?;
            let exigees = role::permissions_du_role(unite.connexion(), r.id, &contexte).await?;
            if exigees.iter().any(|p| !detenues.contains(p)) {
                return Err(ErreurService::Regle {
                    cle: "role.permission_non_detenue",
                });
            }
            role::attribuer(unite.connexion(), utilisateur_id, r.id, &contexte).await?;
            unite.emettre(
                TypeEvenement::RoleAttribue,
                "utilisateur",
                &id_texte,
                serde_json::json!({ "role": r.code }),
            );
        } else {
            if r.code == RoleSysteme::Administrateur.code() {
                proteger_dernier_administrateur(unite.connexion(), utilisateur_id, &contexte)
                    .await?;
            }
            role::desattribuer(unite.connexion(), utilisateur_id, r.id, &contexte).await?;
            unite.emettre(
                TypeEvenement::RoleRetire,
                "utilisateur",
                &id_texte,
                serde_json::json!({ "role": r.code }),
            );
        }
        unite.auditer(
            "utilisateur_role",
            &id_texte,
            if attribue {
                Operation::Creation
            } else {
                Operation::Suppression
            },
            None,
            Some(serde_json::json!({ "role": r.code, "attribue": attribue })),
            "administration.role_attribuer",
        );
        self.noyau.terminer(unite).await?;
        Ok(())
    }

    /// Paramètres de l'établissement.
    pub async fn parametres(&self, session: &Session) -> Resultat<Vec<Parametre>> {
        let contexte = self.noyau.contexte(session.etablissement_id, Some(session));
        let mut cnx = self.connexion().await?;
        Ok(parametre::lister(&mut cnx, &contexte).await?)
    }

    /// Modifie un paramètre existant, modifiable et de même type (§3.8 : rien en dur, mais rien
    /// d'inventé non plus).
    pub async fn definir_parametre(
        &self,
        session: &Session,
        cle: &str,
        valeur: ValeurParametre,
    ) -> Resultat<Parametre> {
        let contexte = self.noyau.contexte(session.etablissement_id, Some(session));
        let mut unite = self
            .noyau
            .commencer(contexte.clone(), Durabilite::Ordinaire)
            .await?;
        let avant = parametre::lire(unite.connexion(), cle, &contexte)
            .await?
            .ok_or(ErreurService::Regle {
                cle: "parametre.inconnu",
            })?;
        if !avant.modifiable_par_utilisateur {
            return Err(ErreurService::Regle {
                cle: "parametre.non_modifiable",
            });
        }
        if std::mem::discriminant(&avant.valeur) != std::mem::discriminant(&valeur) {
            return Err(ErreurService::Regle {
                cle: "parametre.type_incompatible",
            });
        }
        if gesco_core::core::parametre::hors_bornes(cle, &valeur) {
            return Err(ErreurService::Regle {
                cle: "parametre.hors_bornes",
            });
        }
        let apres = Parametre {
            valeur,
            ..avant.clone()
        };
        parametre::definir(unite.connexion(), &apres, &contexte).await?;
        unite.auditer(
            "parametre",
            cle,
            Operation::Modification,
            Some(serde_json::to_value(&avant)?),
            Some(serde_json::to_value(&apres)?),
            "administration.parametre_definir",
        );
        self.noyau.terminer(unite).await?;
        Ok(apres)
    }

    /// Dernières lignes du journal d'audit, la plus récente d'abord.
    pub async fn journal(
        &self,
        session: &Session,
        recherche: &RechercheJournal,
    ) -> Resultat<PageJournal> {
        let montants = crate::engagements::voit_les_montants(self.noyau, session).await?;
        let contexte = self.noyau.contexte(session.etablissement_id, Some(session));
        let mut cnx = self.connexion().await?;
        let filtre = audit::FiltreAudit {
            utilisateur_id: recherche.utilisateur_id.map(|id| id.vers_chaine()),
            entite: recherche.entite.clone().filter(|e| !e.is_empty()),
            operation: recherche.operation.map(|o| o.nom()),
            depuis: recherche.depuis.map(|h| h.millis_epoque()),
            jusqua: recherche.jusqua.map(|h| h.millis_epoque()),
        };
        let mut lignes = audit::rechercher(
            &mut cnx,
            &filtre,
            TAILLE_PAGE_JOURNAL,
            i64::from(recherche.page) * TAILLE_PAGE_JOURNAL,
            &contexte,
        )
        .await?;
        let suivante = lignes.len() > usize::try_from(TAILLE_PAGE_JOURNAL).unwrap_or(usize::MAX);
        lignes.truncate(usize::try_from(TAILLE_PAGE_JOURNAL).unwrap_or(usize::MAX));
        // La rémunération d'un engagement se lit avec `contracts:read` (SPEC_PHASE_6 §7), au
        // journal comme sur la fiche.
        if !montants {
            let masquee = serde_json::to_value(gesco_core::rh::Remuneration::Masquee)?;
            for l in lignes.iter_mut().filter(|l| l.entite == "engagement") {
                for etat in [&mut l.avant, &mut l.apres].into_iter().flatten() {
                    if let Some(r) = etat.get_mut("remuneration") {
                        r.clone_from(&masquee);
                    }
                }
            }
        }
        let auteurs: Vec<Auteur> = utilisateur::lister(&mut cnx, &contexte)
            .await?
            .into_iter()
            .map(|u| Auteur {
                id: u.id,
                nom: u.nom_affiche,
            })
            .collect();
        Ok(PageJournal {
            lignes,
            page: recherche.page,
            suivante,
            entites: audit::entites(&mut cnx, &contexte).await?,
            auteurs,
        })
    }

    /// Recalcule toute la chaîne d'audit (§5.2) ; la première rupture est une erreur.
    pub async fn verifier_journal(&self, session: &Session) -> Resultat<VerificationAudit> {
        let contexte = self.noyau.contexte(session.etablissement_id, Some(session));
        let mut cnx = self.connexion().await?;
        let lignes = audit::verifier_chaine(&mut cnx, None, &contexte).await?;
        Ok(VerificationAudit {
            lignes_verifiees: u32::try_from(lignes).unwrap_or(u32::MAX),
        })
    }
}
