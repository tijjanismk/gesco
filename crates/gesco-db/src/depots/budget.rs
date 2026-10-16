//! Journal de trésorerie et budget (SPEC_PHASE_8 §7) : ce qui est entré par type de frais, ce qui
//! est sorti par catégorie, le budget prévu de l'année.

use gesco_core::commun::{Horodatage, Id, Montant};
use gesco_core::core::AnneeScolaire;
use gesco_core::tresorerie::CategorieTresorerie;

use crate::Connexion;
use crate::contexte::Contexte;
use crate::erreur::Resultat;
use crate::id::lire_id;

/// Recettes encaissées entre deux instants, par type de frais ; `None` pour la part d'un
/// versement qu'aucune échéance n'a reçue.
pub async fn recettes_entre(
    cnx: &mut Connexion,
    debut: Horodatage,
    fin: Horodatage,
    contexte: &Contexte,
) -> Resultat<Vec<(Option<String>, Montant)>> {
    let imputees: Vec<(String, i64)> = sqlx::query_as(
        "SELECT lf.type_frais, SUM(i.montant) FROM imputation_versement i
            JOIN versement v ON v.id = i.versement_id
            JOIN echeance e ON e.id = i.echeance_id
            JOIN ligne_facture lf ON lf.id = e.ligne_facture_id
         WHERE v.etablissement_id = ?1 AND v.statut = 'valide' AND i.statut = 'active'
            AND v.encaisse_le >= ?2 AND v.encaisse_le < ?3
         GROUP BY lf.type_frais",
    )
    .bind(contexte.etablissement_id.vers_chaine())
    .bind(debut.millis_epoque())
    .bind(fin.millis_epoque())
    .fetch_all(&mut *cnx)
    .await?;
    let (total,): (i64,) = sqlx::query_as(
        "SELECT COALESCE(SUM(montant), 0) FROM versement
         WHERE etablissement_id = ?1 AND statut = 'valide' AND encaisse_le >= ?2
            AND encaisse_le < ?3",
    )
    .bind(contexte.etablissement_id.vers_chaine())
    .bind(debut.millis_epoque())
    .bind(fin.millis_epoque())
    .fetch_one(&mut *cnx)
    .await?;
    let impute: i64 = imputees.iter().map(|(_, m)| *m).sum();
    let mut lignes: Vec<(Option<String>, Montant)> = imputees
        .into_iter()
        .map(|(t, m)| (Some(t), Montant::depuis_francs(m)))
        .collect();
    if total > impute {
        lignes.push((None, Montant::depuis_francs(total - impute)));
    }
    Ok(lignes)
}

/// Dépenses payées entre deux instants, par catégorie.
pub async fn depenses_entre(
    cnx: &mut Connexion,
    debut: Horodatage,
    fin: Horodatage,
    contexte: &Contexte,
) -> Resultat<Vec<(Id<CategorieTresorerie>, Montant)>> {
    let lignes: Vec<(String, i64)> = sqlx::query_as(
        "SELECT categorie_id, SUM(montant) FROM depense
         WHERE etablissement_id = ?1 AND statut = 'payee' AND payee_le >= ?2 AND payee_le < ?3
         GROUP BY categorie_id",
    )
    .bind(contexte.etablissement_id.vers_chaine())
    .bind(debut.millis_epoque())
    .bind(fin.millis_epoque())
    .fetch_all(cnx)
    .await?;
    lignes
        .into_iter()
        .map(|(c, m)| {
            Ok((
                lire_id("depense", "categorie_id", &c)?,
                Montant::depuis_francs(m),
            ))
        })
        .collect()
}

/// Budget prévu d'une année : catégorie et montant.
pub async fn budget(
    cnx: &mut Connexion,
    annee_id: Id<AnneeScolaire>,
    contexte: &Contexte,
) -> Resultat<Vec<(Id<CategorieTresorerie>, Montant)>> {
    let lignes: Vec<(String, i64)> = sqlx::query_as(
        "SELECT categorie_id, montant FROM budget_tresorerie
         WHERE etablissement_id = ?1 AND annee_id = ?2",
    )
    .bind(contexte.etablissement_id.vers_chaine())
    .bind(annee_id.vers_chaine())
    .fetch_all(cnx)
    .await?;
    lignes
        .into_iter()
        .map(|(c, m)| {
            Ok((
                lire_id("budget_tresorerie", "categorie_id", &c)?,
                Montant::depuis_francs(m),
            ))
        })
        .collect()
}

/// Prévoit le montant d'une catégorie pour une année.
pub async fn prevoir(
    cnx: &mut Connexion,
    id_si_nouveau: &str,
    annee_id: Id<AnneeScolaire>,
    categorie_id: Id<CategorieTresorerie>,
    montant: Montant,
    contexte: &Contexte,
) -> Resultat<()> {
    sqlx::query(
        "INSERT INTO budget_tresorerie (id, etablissement_id, annee_id, categorie_id, montant,
            cree_le, cree_par, mis_a_jour_le, mis_a_jour_par, identifiant_poste)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?6, ?7, ?8)
         ON CONFLICT (annee_id, categorie_id) DO UPDATE SET montant = excluded.montant,
            mis_a_jour_le = excluded.mis_a_jour_le, mis_a_jour_par = excluded.mis_a_jour_par,
            identifiant_poste = excluded.identifiant_poste",
    )
    .bind(id_si_nouveau)
    .bind(contexte.etablissement_id.vers_chaine())
    .bind(annee_id.vers_chaine())
    .bind(categorie_id.vers_chaine())
    .bind(montant.francs())
    .bind(contexte.maintenant.millis_epoque())
    .bind(contexte.auteur())
    .bind(&contexte.poste.0)
    .execute(cnx)
    .await?;
    Ok(())
}
