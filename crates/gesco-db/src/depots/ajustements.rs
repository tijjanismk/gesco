//! Ajustements de facture au changement de classe et avoirs des élèves (migration 0053). Rien ne
//! se supprime ni ne se réécrit (R-14) : une part d'ajustement se contre-passe, un avoir reste.

use gesco_core::commun::{DateCivile, Id, Montant};
use gesco_core::finance::{Ajustement, Avoir, Echeance, Facture, LigneFacture};
use gesco_core::scolarite::Eleve;
use sqlx::FromRow;

use crate::Connexion;
use crate::contexte::Contexte;
use crate::erreur::{ErreurBase, Resultat};
use crate::id::lire_id;

#[derive(FromRow)]
struct LigneAjustement {
    id: String,
    etablissement_id: String,
    facture_id: String,
    date_ajustement: String,
    hausse: i64,
    baisse: i64,
    avoir: i64,
    classe_avant_id: String,
    classe_apres_id: String,
    remplace_le: Option<i64>,
}

impl LigneAjustement {
    fn vers_entite(self) -> Resultat<Ajustement> {
        let t = "ajustement_facture";
        Ok(Ajustement {
            id: lire_id(t, "id", &self.id)?,
            etablissement_id: lire_id(t, "etablissement_id", &self.etablissement_id)?,
            facture_id: lire_id(t, "facture_id", &self.facture_id)?,
            date: DateCivile::analyser(&self.date_ajustement).ok_or_else(|| {
                ErreurBase::illisible(t, "date_ajustement", &self.date_ajustement)
            })?,
            hausse: Montant::depuis_francs(self.hausse),
            baisse: Montant::depuis_francs(self.baisse),
            avoir: Montant::depuis_francs(self.avoir),
            classe_avant_id: lire_id(t, "classe_avant_id", &self.classe_avant_id)?,
            classe_apres_id: lire_id(t, "classe_apres_id", &self.classe_apres_id)?,
            remplace: self.remplace_le.is_some(),
        })
    }
}

/// Enregistre un ajustement.
pub async fn inserer(cnx: &mut Connexion, a: &Ajustement, contexte: &Contexte) -> Resultat<()> {
    sqlx::query(
        "INSERT INTO ajustement_facture (id, etablissement_id, facture_id, date_ajustement, hausse, baisse,
            avoir, classe_avant_id, classe_apres_id, cree_le, cree_par, identifiant_poste)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
    )
    .bind(a.id.vers_chaine())
    .bind(contexte.etablissement_id.vers_chaine())
    .bind(a.facture_id.vers_chaine())
    .bind(a.date.to_string())
    .bind(a.hausse.francs())
    .bind(a.baisse.francs())
    .bind(a.avoir.francs())
    .bind(a.classe_avant_id.vers_chaine())
    .bind(a.classe_apres_id.vers_chaine())
    .bind(contexte.maintenant.millis_epoque())
    .bind(contexte.auteur())
    .bind(&contexte.poste.0)
    .execute(cnx)
    .await?;
    Ok(())
}

/// Ajustements d'une facture, du plus ancien au plus récent.
pub async fn de_la_facture(
    cnx: &mut Connexion,
    facture_id: Id<Facture>,
    contexte: &Contexte,
) -> Resultat<Vec<Ajustement>> {
    sqlx::query_as::<_, LigneAjustement>(
        "SELECT id, etablissement_id, facture_id, date_ajustement, hausse, baisse, avoir,
            classe_avant_id, classe_apres_id, remplace_le FROM ajustement_facture
         WHERE facture_id = ?1 AND etablissement_id = ?2 ORDER BY cree_le, rowid",
    )
    .bind(facture_id.vers_chaine())
    .bind(contexte.etablissement_id.vers_chaine())
    .fetch_all(cnx)
    .await?
    .into_iter()
    .map(LigneAjustement::vers_entite)
    .collect()
}

/// Marque remplacé un ajustement dont les effets sont contre-passés : un changement de plan
/// recalcule les échéances et applique l'ajustement de nouveau.
pub async fn remplacer(
    cnx: &mut Connexion,
    id: Id<Ajustement>,
    contexte: &Contexte,
) -> Resultat<()> {
    sqlx::query(
        "UPDATE ajustement_facture SET remplace_le = ?1
         WHERE id = ?2 AND etablissement_id = ?3 AND remplace_le IS NULL",
    )
    .bind(contexte.maintenant.millis_epoque())
    .bind(id.vers_chaine())
    .bind(contexte.etablissement_id.vers_chaine())
    .execute(cnx)
    .await?;
    Ok(())
}

/// Rattache à son ajustement une ligne de facture née d'une hausse.
pub async fn marquer_ligne(
    cnx: &mut Connexion,
    ligne_id: Id<LigneFacture>,
    ajustement_id: Id<Ajustement>,
    contexte: &Contexte,
) -> Resultat<()> {
    sqlx::query(
        "UPDATE ligne_facture SET ajustement_id = ?1 WHERE id = ?2 AND etablissement_id = ?3",
    )
    .bind(ajustement_id.vers_chaine())
    .bind(ligne_id.vers_chaine())
    .bind(contexte.etablissement_id.vers_chaine())
    .execute(cnx)
    .await?;
    Ok(())
}

/// Lignes d'une facture nées d'une hausse.
pub async fn lignes_d_ajustement(
    cnx: &mut Connexion,
    facture_id: Id<Facture>,
    contexte: &Contexte,
) -> Resultat<Vec<Id<LigneFacture>>> {
    let ids: Vec<(String,)> = sqlx::query_as(
        "SELECT id FROM ligne_facture
         WHERE facture_id = ?1 AND etablissement_id = ?2 AND ajustement_id IS NOT NULL",
    )
    .bind(facture_id.vers_chaine())
    .bind(contexte.etablissement_id.vers_chaine())
    .fetch_all(cnx)
    .await?;
    ids.into_iter()
        .map(|(id,)| lire_id("ligne_facture", "id", &id))
        .collect()
}

/// Enregistre la part d'une baisse sur une échéance.
pub async fn inserer_part(
    cnx: &mut Connexion,
    ajustement_id: Id<Ajustement>,
    echeance_id: Id<Echeance>,
    montant: Montant,
    contexte: &Contexte,
) -> Resultat<()> {
    sqlx::query(
        "INSERT INTO ajustement_echeance (id, etablissement_id, ajustement_id, echeance_id, montant, statut,
            cree_le, cree_par, mis_a_jour_le, mis_a_jour_par, identifiant_poste)
         VALUES (?1, ?2, ?3, ?4, ?5, 'active', ?6, ?7, ?6, ?7, ?8)",
    )
    .bind(crate::nouvel_id::<()>().vers_chaine())
    .bind(contexte.etablissement_id.vers_chaine())
    .bind(ajustement_id.vers_chaine())
    .bind(echeance_id.vers_chaine())
    .bind(montant.francs())
    .bind(contexte.maintenant.millis_epoque())
    .bind(contexte.auteur())
    .bind(&contexte.poste.0)
    .execute(cnx)
    .await?;
    Ok(())
}

/// Parts actives des baisses sur les échéances d'une facture : (ajustement, échéance, montant).
pub async fn parts_de_la_facture(
    cnx: &mut Connexion,
    facture_id: Id<Facture>,
    contexte: &Contexte,
) -> Resultat<Vec<(Id<Ajustement>, Id<Echeance>, Montant)>> {
    let lignes: Vec<(String, String, i64)> = sqlx::query_as(
        "SELECT p.ajustement_id, p.echeance_id, p.montant FROM ajustement_echeance p
         JOIN echeance e ON e.id = p.echeance_id
         WHERE e.facture_id = ?1 AND p.etablissement_id = ?2 AND p.statut = 'active'",
    )
    .bind(facture_id.vers_chaine())
    .bind(contexte.etablissement_id.vers_chaine())
    .fetch_all(cnx)
    .await?;
    lignes
        .into_iter()
        .map(|(a, e, m)| {
            Ok((
                lire_id("ajustement_echeance", "ajustement_id", &a)?,
                lire_id("ajustement_echeance", "echeance_id", &e)?,
                Montant::depuis_francs(m),
            ))
        })
        .collect()
}

/// Reporte sur une échéance recréée (jour changé, élève réintégré) les parts de baisse de celle
/// qu'elle remplace : contre-passées sur l'ancienne, enregistrées à nouveau sur la nouvelle.
pub async fn reporter_parts(
    cnx: &mut Connexion,
    ancienne: Id<Echeance>,
    nouvelle: Id<Echeance>,
    contexte: &Contexte,
) -> Resultat<()> {
    let parts: Vec<(String, i64)> = sqlx::query_as(
        "SELECT ajustement_id, montant FROM ajustement_echeance
         WHERE echeance_id = ?1 AND etablissement_id = ?2 AND statut = 'active'",
    )
    .bind(ancienne.vers_chaine())
    .bind(contexte.etablissement_id.vers_chaine())
    .fetch_all(&mut *cnx)
    .await?;
    if parts.is_empty() {
        return Ok(());
    }
    sqlx::query(
        "UPDATE ajustement_echeance SET statut = 'annulee', mis_a_jour_le = ?1, mis_a_jour_par = ?2,
            identifiant_poste = ?3
         WHERE echeance_id = ?4 AND etablissement_id = ?5 AND statut = 'active'",
    )
    .bind(contexte.maintenant.millis_epoque())
    .bind(contexte.auteur())
    .bind(&contexte.poste.0)
    .bind(ancienne.vers_chaine())
    .bind(contexte.etablissement_id.vers_chaine())
    .execute(&mut *cnx)
    .await?;
    for (ajustement, montant) in parts {
        let ajustement = lire_id("ajustement_echeance", "ajustement_id", &ajustement)?;
        inserer_part(
            cnx,
            ajustement,
            nouvelle,
            Montant::depuis_francs(montant),
            contexte,
        )
        .await?;
    }
    Ok(())
}

/// Enregistre un avoir né d'un ajustement.
pub async fn inserer_avoir(
    cnx: &mut Connexion,
    ajustement_id: Id<Ajustement>,
    echeance_id: Id<Echeance>,
    montant: Montant,
    contexte: &Contexte,
) -> Resultat<()> {
    sqlx::query(
        "INSERT INTO avoir (id, etablissement_id, ajustement_id, echeance_id, montant, cree_le, cree_par,
            identifiant_poste)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
    )
    .bind(crate::nouvel_id::<()>().vers_chaine())
    .bind(contexte.etablissement_id.vers_chaine())
    .bind(ajustement_id.vers_chaine())
    .bind(echeance_id.vers_chaine())
    .bind(montant.francs())
    .bind(contexte.maintenant.millis_epoque())
    .bind(contexte.auteur())
    .bind(&contexte.poste.0)
    .execute(cnx)
    .await?;
    Ok(())
}

#[derive(FromRow)]
struct LigneAvoir {
    id: String,
    ajustement_id: String,
    echeance_id: String,
    montant: i64,
    utilise: i64,
}

/// Avoirs d'un élève, toutes années, du plus ancien au plus récent, avec ce qui en est utilisé.
pub async fn avoirs_de_l_eleve(
    cnx: &mut Connexion,
    eleve_id: Id<Eleve>,
    contexte: &Contexte,
) -> Resultat<Vec<Avoir>> {
    sqlx::query_as::<_, LigneAvoir>(
        "SELECT a.id, a.ajustement_id, a.echeance_id, a.montant,
            (SELECT COALESCE(SUM(u.montant), 0) FROM avoir_imputation u WHERE u.avoir_id = a.id) AS utilise
         FROM avoir a
            JOIN ajustement_facture j ON j.id = a.ajustement_id
            JOIN facture f ON f.id = j.facture_id
            JOIN inscription i ON i.id = f.inscription_id
         WHERE i.eleve_id = ?1 AND a.etablissement_id = ?2
         ORDER BY a.cree_le, a.rowid",
    )
    .bind(eleve_id.vers_chaine())
    .bind(contexte.etablissement_id.vers_chaine())
    .fetch_all(cnx)
    .await?
    .into_iter()
    .map(|l| {
        let t = "avoir";
        Ok(Avoir {
            id: lire_id(t, "id", &l.id)?,
            ajustement_id: lire_id(t, "ajustement_id", &l.ajustement_id)?,
            echeance_id: lire_id(t, "echeance_id", &l.echeance_id)?,
            montant: Montant::depuis_francs(l.montant),
            utilise: Montant::depuis_francs(l.utilise),
        })
    })
    .collect()
}

/// Avoirs nés sur les échéances d'une facture : (échéance, montant).
pub async fn avoirs_de_la_facture(
    cnx: &mut Connexion,
    facture_id: Id<Facture>,
    contexte: &Contexte,
) -> Resultat<Vec<(Id<Echeance>, Montant)>> {
    let lignes: Vec<(String, i64)> = sqlx::query_as(
        "SELECT a.echeance_id, a.montant FROM avoir a JOIN echeance e ON e.id = a.echeance_id
         WHERE e.facture_id = ?1 AND a.etablissement_id = ?2",
    )
    .bind(facture_id.vers_chaine())
    .bind(contexte.etablissement_id.vers_chaine())
    .fetch_all(cnx)
    .await?;
    lignes
        .into_iter()
        .map(|(e, m)| {
            Ok((
                lire_id("avoir", "echeance_id", &e)?,
                Montant::depuis_francs(m),
            ))
        })
        .collect()
}

/// Enregistre l'usage d'un avoir sur une échéance.
pub async fn imputer_avoir(
    cnx: &mut Connexion,
    avoir_id: Id<Avoir>,
    echeance_id: Id<Echeance>,
    montant: Montant,
    contexte: &Contexte,
) -> Resultat<()> {
    sqlx::query(
        "INSERT INTO avoir_imputation (id, etablissement_id, avoir_id, echeance_id, montant, cree_le,
            cree_par, identifiant_poste)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
    )
    .bind(crate::nouvel_id::<()>().vers_chaine())
    .bind(contexte.etablissement_id.vers_chaine())
    .bind(avoir_id.vers_chaine())
    .bind(echeance_id.vers_chaine())
    .bind(montant.francs())
    .bind(contexte.maintenant.millis_epoque())
    .bind(contexte.auteur())
    .bind(&contexte.poste.0)
    .execute(cnx)
    .await?;
    Ok(())
}

/// Avoirs utilisés sur les échéances d'une facture : (échéance, montant).
pub async fn avoirs_utilises_sur_la_facture(
    cnx: &mut Connexion,
    facture_id: Id<Facture>,
    contexte: &Contexte,
) -> Resultat<Vec<(Id<Echeance>, Montant)>> {
    let lignes: Vec<(String, i64)> = sqlx::query_as(
        "SELECT u.echeance_id, u.montant FROM avoir_imputation u JOIN echeance e ON e.id = u.echeance_id
         WHERE e.facture_id = ?1 AND u.etablissement_id = ?2",
    )
    .bind(facture_id.vers_chaine())
    .bind(contexte.etablissement_id.vers_chaine())
    .fetch_all(cnx)
    .await?;
    lignes
        .into_iter()
        .map(|(e, m)| {
            Ok((
                lire_id("avoir_imputation", "echeance_id", &e)?,
                Montant::depuis_francs(m),
            ))
        })
        .collect()
}
