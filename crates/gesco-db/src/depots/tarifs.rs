//! Grilles de frais et plans de paiement (SPEC_PHASE_3 §3).

use gesco_core::commun::{DateCivile, Id, Montant};
use gesco_core::core::{AnneeScolaire, MachineAEtats};
use gesco_core::finance::{
    ContenuGrille, EtatGrille, Exigibilite, Grille, LigneTarifaire, PlanPaiement, Tranche,
    TypeFrais,
};
use sqlx::FromRow;

use crate::Connexion;
use crate::contexte::Contexte;
use crate::depots::booleen;
use crate::erreur::{ErreurBase, Resultat};
use crate::id::lire_id;

#[derive(FromRow)]
struct LigneGrille {
    id: String,
    etablissement_id: String,
    annee_id: String,
    libelle: String,
    version: i64,
    etat: String,
}

impl LigneGrille {
    fn vers_entite(self) -> Resultat<Grille> {
        Ok(Grille {
            id: lire_id("grille_tarifaire", "id", &self.id)?,
            etablissement_id: lire_id(
                "grille_tarifaire",
                "etablissement_id",
                &self.etablissement_id,
            )?,
            annee_id: lire_id("grille_tarifaire", "annee_id", &self.annee_id)?,
            etat: EtatGrille::depuis_nom(&self.etat)
                .ok_or_else(|| ErreurBase::illisible("grille_tarifaire", "etat", &self.etat))?,
            libelle: self.libelle,
            version: self.version,
        })
    }
}

macro_rules! selectionner_grille {
    ($suite:literal) => {
        concat!(
            "SELECT id, etablissement_id, annee_id, libelle, version, etat FROM grille_tarifaire ",
            $suite
        )
    };
}

/// Crée une grille dans l'établissement du contexte.
pub async fn inserer_grille(
    cnx: &mut Connexion,
    grille: &Grille,
    contexte: &Contexte,
) -> Resultat<()> {
    if grille.etablissement_id != contexte.etablissement_id {
        return Err(ErreurBase::Cloisonnement {
            entite: "grille_tarifaire",
            id: grille.id.vers_chaine(),
        });
    }
    sqlx::query(
        "INSERT INTO grille_tarifaire (id, etablissement_id, annee_id, libelle, version, etat,
            cree_le, cree_par, mis_a_jour_le, mis_a_jour_par, identifiant_poste)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?7, ?8, ?9)",
    )
    .bind(grille.id.vers_chaine())
    .bind(grille.etablissement_id.vers_chaine())
    .bind(grille.annee_id.vers_chaine())
    .bind(&grille.libelle)
    .bind(grille.version)
    .bind(grille.etat.nom())
    .bind(contexte.maintenant.millis_epoque())
    .bind(contexte.auteur())
    .bind(&contexte.poste.0)
    .execute(cnx)
    .await?;
    Ok(())
}

/// Grilles d'une année, de la plus récente à la plus ancienne.
pub async fn grilles(
    cnx: &mut Connexion,
    annee_id: Id<AnneeScolaire>,
    contexte: &Contexte,
) -> Resultat<Vec<Grille>> {
    sqlx::query_as::<_, LigneGrille>(selectionner_grille!(
        "WHERE etablissement_id = ?1 AND annee_id = ?2 ORDER BY version DESC"
    ))
    .bind(contexte.etablissement_id.vers_chaine())
    .bind(annee_id.vers_chaine())
    .fetch_all(cnx)
    .await?
    .into_iter()
    .map(LigneGrille::vers_entite)
    .collect()
}

/// Lit une grille de l'établissement du contexte.
pub async fn grille(cnx: &mut Connexion, id: Id<Grille>, contexte: &Contexte) -> Resultat<Grille> {
    sqlx::query_as::<_, LigneGrille>(selectionner_grille!(
        "WHERE id = ?1 AND etablissement_id = ?2"
    ))
    .bind(id.vers_chaine())
    .bind(contexte.etablissement_id.vers_chaine())
    .fetch_optional(cnx)
    .await?
    .ok_or_else(|| ErreurBase::Introuvable {
        entite: "grille_tarifaire",
        id: id.vers_chaine(),
    })?
    .vers_entite()
}

/// Grille en vigueur d'une année, s'il y en a une.
pub async fn grille_en_vigueur(
    cnx: &mut Connexion,
    annee_id: Id<AnneeScolaire>,
    contexte: &Contexte,
) -> Resultat<Option<Grille>> {
    sqlx::query_as::<_, LigneGrille>(selectionner_grille!(
        "WHERE etablissement_id = ?1 AND annee_id = ?2 AND etat = 'en_vigueur'"
    ))
    .bind(contexte.etablissement_id.vers_chaine())
    .bind(annee_id.vers_chaine())
    .fetch_optional(cnx)
    .await?
    .map(LigneGrille::vers_entite)
    .transpose()
}

/// Numéro de la prochaine version dans l'année.
pub async fn prochaine_version(
    cnx: &mut Connexion,
    annee_id: Id<AnneeScolaire>,
    contexte: &Contexte,
) -> Resultat<i64> {
    let max: Option<i64> = sqlx::query_scalar(
        "SELECT MAX(version) FROM grille_tarifaire WHERE etablissement_id = ?1 AND annee_id = ?2",
    )
    .bind(contexte.etablissement_id.vers_chaine())
    .bind(annee_id.vers_chaine())
    .fetch_one(cnx)
    .await?;
    Ok(max.unwrap_or(0) + 1)
}

/// Récrit le libellé et l'état d'une grille.
pub async fn ecrire_grille(
    cnx: &mut Connexion,
    grille: &Grille,
    contexte: &Contexte,
) -> Resultat<()> {
    sqlx::query(
        "UPDATE grille_tarifaire SET libelle = ?1, etat = ?2, mis_a_jour_le = ?3, mis_a_jour_par = ?4, identifiant_poste = ?5
         WHERE id = ?6 AND etablissement_id = ?7",
    )
    .bind(&grille.libelle)
    .bind(grille.etat.nom())
    .bind(contexte.maintenant.millis_epoque())
    .bind(contexte.auteur())
    .bind(&contexte.poste.0)
    .bind(grille.id.vers_chaine())
    .bind(contexte.etablissement_id.vers_chaine())
    .execute(cnx)
    .await?;
    Ok(())
}

#[derive(FromRow)]
struct LigneLigneTarifaire {
    id: String,
    niveau_id: String,
    serie_id: Option<String>,
    type_frais: String,
    libelle: String,
    montant: i64,
    obligatoire: i64,
    exigibilite: String,
}

#[derive(FromRow)]
struct LignePlan {
    id: String,
    libelle: String,
    par_defaut: i64,
}

#[derive(FromRow)]
struct LigneTranche {
    libelle: String,
    echeance: String,
    part: i64,
}

/// Lignes et plans d'une grille, dans leur ordre de saisie.
pub async fn contenu(
    cnx: &mut Connexion,
    grille_id: Id<Grille>,
    contexte: &Contexte,
) -> Resultat<ContenuGrille> {
    let illisible = |colonne: &'static str, valeur: &str| {
        ErreurBase::illisible("ligne_tarifaire", colonne, valeur)
    };
    let lignes = sqlx::query_as::<_, LigneLigneTarifaire>(
        "SELECT id, niveau_id, serie_id, type_frais, libelle, montant, obligatoire, exigibilite
         FROM ligne_tarifaire WHERE grille_id = ?1 AND etablissement_id = ?2 AND supprime_le IS NULL
         ORDER BY ordre",
    )
    .bind(grille_id.vers_chaine())
    .bind(contexte.etablissement_id.vers_chaine())
    .fetch_all(&mut *cnx)
    .await?
    .into_iter()
    .map(|l| {
        Ok(LigneTarifaire {
            id: lire_id("ligne_tarifaire", "id", &l.id)?,
            niveau_id: lire_id("ligne_tarifaire", "niveau_id", &l.niveau_id)?,
            serie_id: l
                .serie_id
                .as_deref()
                .map(|s| lire_id("ligne_tarifaire", "serie_id", s))
                .transpose()?,
            type_frais: TypeFrais::depuis_nom(&l.type_frais)
                .ok_or_else(|| illisible("type_frais", &l.type_frais))?,
            exigibilite: Exigibilite::depuis_nom(&l.exigibilite)
                .ok_or_else(|| illisible("exigibilite", &l.exigibilite))?,
            libelle: l.libelle,
            montant: Montant::depuis_francs(l.montant),
            obligatoire: booleen(l.obligatoire),
        })
    })
    .collect::<Resultat<Vec<_>>>()?;
    let entetes = sqlx::query_as::<_, LignePlan>(
        "SELECT id, libelle, par_defaut FROM plan_paiement
         WHERE grille_id = ?1 AND etablissement_id = ?2 AND supprime_le IS NULL ORDER BY ordre",
    )
    .bind(grille_id.vers_chaine())
    .bind(contexte.etablissement_id.vers_chaine())
    .fetch_all(&mut *cnx)
    .await?;
    let mut plans = Vec::with_capacity(entetes.len());
    for p in entetes {
        let tranches = sqlx::query_as::<_, LigneTranche>(
            "SELECT libelle, echeance, part FROM tranche WHERE plan_id = ?1 AND etablissement_id = ?2 ORDER BY rang",
        )
        .bind(&p.id)
        .bind(contexte.etablissement_id.vers_chaine())
        .fetch_all(&mut *cnx)
        .await?
        .into_iter()
        .map(|t| {
            Ok(Tranche {
                echeance: DateCivile::analyser(&t.echeance)
                    .ok_or_else(|| ErreurBase::illisible("tranche", "echeance", &t.echeance))?,
                libelle: t.libelle,
                part: t.part,
            })
        })
        .collect::<Resultat<Vec<_>>>()?;
        plans.push(PlanPaiement {
            id: lire_id("plan_paiement", "id", &p.id)?,
            libelle: p.libelle,
            par_defaut: booleen(p.par_defaut),
            tranches,
        });
    }
    Ok(ContenuGrille { lignes, plans })
}

/// Remplace le contenu d'une grille : l'ancien est supprimé logiquement, le nouveau inséré. Le
/// service n'appelle cette fonction que sur un brouillon.
pub async fn remplacer_contenu(
    cnx: &mut Connexion,
    grille_id: Id<Grille>,
    contenu: &ContenuGrille,
    contexte: &Contexte,
) -> Resultat<()> {
    let maintenant = contexte.maintenant.millis_epoque();
    let etablissement = contexte.etablissement_id.vers_chaine();
    for requete in [
        "UPDATE ligne_tarifaire SET supprime_le = ?1, supprime_par = ?2
         WHERE grille_id = ?3 AND etablissement_id = ?4 AND supprime_le IS NULL",
        "UPDATE plan_paiement SET supprime_le = ?1, supprime_par = ?2
         WHERE grille_id = ?3 AND etablissement_id = ?4 AND supprime_le IS NULL",
    ] {
        sqlx::query(requete)
            .bind(maintenant)
            .bind(contexte.auteur())
            .bind(grille_id.vers_chaine())
            .bind(&etablissement)
            .execute(&mut *cnx)
            .await?;
    }
    for (ordre, ligne) in contenu.lignes.iter().enumerate() {
        sqlx::query(
            "INSERT INTO ligne_tarifaire (id, etablissement_id, grille_id, ordre, niveau_id, serie_id, type_frais,
                libelle, montant, obligatoire, exigibilite, cree_le, cree_par, identifiant_poste)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14)",
        )
        .bind(ligne.id.vers_chaine())
        .bind(&etablissement)
        .bind(grille_id.vers_chaine())
        .bind(i64::try_from(ordre).unwrap_or(i64::MAX))
        .bind(ligne.niveau_id.vers_chaine())
        .bind(ligne.serie_id.map(|s| s.vers_chaine()))
        .bind(ligne.type_frais.nom())
        .bind(&ligne.libelle)
        .bind(ligne.montant.francs())
        .bind(i64::from(ligne.obligatoire))
        .bind(ligne.exigibilite.nom())
        .bind(maintenant)
        .bind(contexte.auteur())
        .bind(&contexte.poste.0)
        .execute(&mut *cnx)
        .await?;
    }
    for (ordre, plan) in contenu.plans.iter().enumerate() {
        sqlx::query(
            "INSERT INTO plan_paiement (id, etablissement_id, grille_id, ordre, libelle, par_defaut,
                cree_le, cree_par, identifiant_poste)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
        )
        .bind(plan.id.vers_chaine())
        .bind(&etablissement)
        .bind(grille_id.vers_chaine())
        .bind(i64::try_from(ordre).unwrap_or(i64::MAX))
        .bind(&plan.libelle)
        .bind(i64::from(plan.par_defaut))
        .bind(maintenant)
        .bind(contexte.auteur())
        .bind(&contexte.poste.0)
        .execute(&mut *cnx)
        .await?;
        for (rang, tranche) in plan.tranches.iter().enumerate() {
            sqlx::query(
                "INSERT INTO tranche (plan_id, rang, etablissement_id, libelle, echeance, part)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            )
            .bind(plan.id.vers_chaine())
            .bind(i64::try_from(rang + 1).unwrap_or(i64::MAX))
            .bind(&etablissement)
            .bind(&tranche.libelle)
            .bind(tranche.echeance.to_string())
            .bind(tranche.part)
            .execute(&mut *cnx)
            .await?;
        }
    }
    Ok(())
}
