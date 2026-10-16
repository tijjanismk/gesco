//! Mise à niveau d'une base installée, à chaque ouverture : les rôles livrés reçoivent les
//! permissions qui manquent à leur composition (SPEC_PHASE_1 §5.6), l'établissement les
//! paramètres apparus depuis sa création (SPEC_PHASE_1 §6) et les niveaux et séries livrés
//! (SPEC_PHASE_2 §2.3). Rien n'est retiré ni écrasé.

use gesco_core::commun::Id;
use gesco_core::core::parametre::parametres_par_defaut;
use gesco_core::core::permission::{COMPOSITION_ROLES_LIVRES, RoleSysteme};
use gesco_core::core::{Etablissement, Operation, Permission, TypeEvenement};
use gesco_core::scolarite::Cycle;
use gesco_db::depots::{academique, annee_scolaire, enseignants, parametre, role};
use gesco_db::unite::Durabilite;

use crate::erreur::Resultat;
use crate::noyau::Noyau;

fn codes(permissions: &[Permission]) -> Vec<&'static str> {
    permissions.iter().map(Permission::code).collect()
}

/// Ajoute à chaque rôle livré en retard les permissions de sa composition qui lui manquent,
/// sans jamais en retirer : un rôle livré modifié par l'administrateur garde ses retraits. Le
/// système est l'auteur de l'audit et de l'événement. Rend le nombre de rôles complétés.
pub async fn completer_roles(
    noyau: &Noyau,
    etablissement_id: Id<Etablissement>,
) -> Resultat<usize> {
    let contexte = noyau.contexte(etablissement_id, None);
    let mut unite = noyau
        .commencer(contexte.clone(), Durabilite::Critique)
        .await?;
    let mut completes = 0;
    for systeme in RoleSysteme::TOUS {
        let Some(r) = role::par_code(unite.connexion(), systeme.code(), &contexte).await? else {
            continue;
        };
        if role::composition_livree(unite.connexion(), r.id, &contexte).await?
            >= COMPOSITION_ROLES_LIVRES
        {
            continue;
        }
        let avant = role::permissions_du_role(unite.connexion(), r.id, &contexte).await?;
        let ajoutees: Vec<Permission> = systeme
            .permissions_initiales()
            .into_iter()
            .filter(|p| !avant.contains(p))
            .collect();
        for permission in &ajoutees {
            role::accorder(unite.connexion(), r.id, *permission, &contexte).await?;
        }
        role::noter_composition_livree(
            unite.connexion(),
            r.id,
            COMPOSITION_ROLES_LIVRES,
            &contexte,
        )
        .await?;
        let apres = role::permissions_du_role(unite.connexion(), r.id, &contexte).await?;
        let id = r.id.vers_chaine();
        unite.auditer(
            "role",
            &id,
            Operation::Modification,
            Some(serde_json::json!({ "permissions": codes(&avant) })),
            Some(serde_json::json!({
                "permissions": codes(&apres),
                "composition_livree": COMPOSITION_ROLES_LIVRES,
            })),
            "mise_a_niveau.roles",
        );
        unite.emettre(
            TypeEvenement::RoleRecompose,
            "role",
            &id,
            serde_json::json!({
                "role": r.code,
                "ajoutees": codes(&ajoutees),
                "retirees": [],
                "composition_livree": COMPOSITION_ROLES_LIVRES,
            }),
        );
        completes += 1;
    }
    if completes == 0 {
        unite.annuler().await?;
    } else {
        noyau.terminer(unite).await?;
    }
    Ok(completes)
}

/// Ajoute les paramètres livrés qui manquent à l'établissement, avec leur valeur par défaut ;
/// un paramètre existant garde sa valeur. Rend le nombre de paramètres ajoutés.
pub async fn completer_parametres(
    noyau: &Noyau,
    etablissement_id: Id<Etablissement>,
) -> Resultat<usize> {
    let contexte = noyau.contexte(etablissement_id, None);
    let mut unite = noyau
        .commencer(contexte.clone(), Durabilite::Critique)
        .await?;
    let mut ajoutes = 0;
    for p in parametres_par_defaut(etablissement_id) {
        if parametre::lire(unite.connexion(), &p.cle, &contexte)
            .await?
            .is_some()
        {
            continue;
        }
        parametre::definir(unite.connexion(), &p, &contexte).await?;
        unite.auditer(
            "parametre",
            &p.cle,
            Operation::Creation,
            None,
            Some(serde_json::to_value(&p)?),
            "mise_a_niveau.parametres",
        );
        ajoutes += 1;
    }
    if ajoutes == 0 {
        unite.annuler().await?;
    } else {
        noyau.terminer(unite).await?;
    }
    Ok(ajoutes)
}

/// Pose les niveaux et séries livrés qui manquent à l'établissement. Rend le nombre de lignes
/// ajoutées.
pub async fn completer_referentiel(
    noyau: &Noyau,
    etablissement_id: Id<Etablissement>,
) -> Resultat<usize> {
    let contexte = noyau.contexte(etablissement_id, None);
    let mut unite = noyau
        .commencer(contexte.clone(), Durabilite::Critique)
        .await?;
    // Une base installée ne dit pas ses cycles : l'enseignement général est posé actif, l'école
    // désactive ensuite ce qu'elle n'enseigne pas ; les années du CAP et du BT arrivent fermées,
    // l'école ouvre le cycle technique si elle l'enseigne (SPEC_PHASE_6 §9).
    let ajoutes = crate::referentiel::poser_referentiel(
        &mut unite,
        etablissement_id,
        &Cycle::GENERAUX,
        "mise_a_niveau.referentiel",
    )
    .await?;
    if ajoutes == 0 {
        unite.annuler().await?;
    } else {
        noyau.terminer(unite).await?;
    }
    Ok(ajoutes)
}

/// Pose, pour chaque année ni clôturée ni archivée, le référentiel pédagogique qui lui manque
/// (SPEC_PHASE_4 §2) : les bases installées avant la phase 4 le reçoivent à leur ouverture. Rend
/// le nombre de lignes ajoutées.
pub async fn completer_pedagogie(
    noyau: &Noyau,
    etablissement_id: Id<Etablissement>,
) -> Resultat<usize> {
    let contexte = noyau.contexte(etablissement_id, None);
    let mut unite = noyau
        .commencer(contexte.clone(), Durabilite::Critique)
        .await?;
    let annees = gesco_db::depots::annee_scolaire::lister(unite.connexion(), &contexte).await?;
    let mut ajoutes = 0;
    for annee in annees.iter().filter(|a| {
        !matches!(
            a.statut,
            gesco_core::core::StatutAnnee::Cloturee | gesco_core::core::StatutAnnee::Archivee
        )
    }) {
        ajoutes +=
            crate::pedagogie::poser_annee(&mut unite, annee, "mise_a_niveau.pedagogie").await?;
    }
    if ajoutes == 0 {
        unite.annuler().await?;
    } else {
        noyau.terminer(unite).await?;
    }
    Ok(ajoutes)
}

/// Fait entrer au registre du personnel chaque enseignant qui n'y est pas (SPEC_PHASE_6 §2,
/// Q-63) : *maître* s'il a été affecté au 1er cycle, *enseignant* sinon ; ses affectations ne
/// bougent pas. Pose aussi les catégories et fonctions livrées. Rend le nombre de lignes ajoutées.
pub async fn completer_personnel(
    noyau: &Noyau,
    etablissement_id: Id<Etablissement>,
) -> Resultat<usize> {
    let contexte = noyau.contexte(etablissement_id, None);
    let mut unite = noyau
        .commencer(contexte.clone(), Durabilite::Critique)
        .await?;
    let mut ajoutes =
        crate::personnel::poser_personnel(&mut unite, etablissement_id, "mise_a_niveau.personnel")
            .await?;
    let niveaux = academique::niveaux(unite.connexion(), &contexte).await?;
    let mut au_premier_cycle = Vec::new();
    for annee in annee_scolaire::lister(unite.connexion(), &contexte).await? {
        let classes = academique::classes(unite.connexion(), annee.id, &contexte).await?;
        for a in enseignants::affectations(unite.connexion(), annee.id, &contexte).await? {
            let premier = classes
                .iter()
                .find(|c| c.id == a.classe_id)
                .and_then(|c| niveaux.iter().find(|n| n.id == c.niveau_id))
                .is_some_and(|n| n.cycle == Cycle::PremierCycle);
            if premier {
                au_premier_cycle.push(a.enseignant_id);
            }
        }
    }
    for e in enseignants::enseignants(unite.connexion(), &contexte).await? {
        if crate::personnel::rattacher_enseignant(
            &mut unite,
            &e,
            au_premier_cycle.contains(&e.id),
            "mise_a_niveau.personnel",
        )
        .await?
        .is_some()
        {
            ajoutes += 1;
        }
    }
    if ajoutes == 0 {
        unite.annuler().await?;
    } else {
        noyau.terminer(unite).await?;
    }
    Ok(ajoutes)
}
